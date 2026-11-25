import AppKit
import WebKit

private let defaultState = FileManager.default.homeDirectoryForCurrentUser
    .appendingPathComponent(".local/state/taik-factory", isDirectory: true)

private struct Round: Codable {
    let name: String
    let fields: [String: String]
    let detail: String

    init?(file: URL) {
        guard let info = try? file.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
              info.isRegularFile == true, info.isSymbolicLink != true,
              let size = info.fileSize, size <= 8_192,
              let text = try? String(contentsOf: file, encoding: .utf8), text.hasSuffix("end\n") else { return nil }
        var fields: [String: String] = [:]
        for line in text.split(separator: "\n") {
            let pair = line.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
            if pair.count == 2 { fields[String(pair[0])] = String(pair[1]) }
        }
        guard let decision = fields["decision"], ["accepted", "rejected"].contains(decision) else { return nil }
        name = file.lastPathComponent
        self.fields = fields
        detail = text
    }
}

private struct Job: Codable {
    let id: String
    let goal: String?
    let preview: String?
    let previous: String?
    let cost: Double
    let draft: String?
    let baseline: Round
    let rounds: [Round]
    let progress: [String: [ProgressEvent]]
    let sourceQueued: Bool
    let sourceActive: Bool
    let sourceStartedAt: Double?

    var pending: Bool {
        if baseline.fields["kind"] == "software" { return sourceActive || sourceQueued }
        guard let last = rounds.last else { return true }
        return last.fields["advance"] == "true" && rounds.count < 20
    }
}

private struct ProgressEvent: Codable {
    let elapsed: UInt64
    let kind: String
    let detail: String

    static func read(_ url: URL) -> [Self] {
        guard let info = try? url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
              info.isRegularFile == true, info.isSymbolicLink != true,
              let size = info.fileSize, size <= 32_768,
              let contents = try? String(contentsOf: url, encoding: .utf8) else { return [] }
        return contents.split(separator: "\n", omittingEmptySubsequences: false).dropLast().compactMap { line in
            let fields = line.split(separator: "\t", maxSplits: 2, omittingEmptySubsequences: false)
            guard fields.count == 3, let elapsed = UInt64(fields[0]) else { return nil }
            return Self(elapsed: elapsed, kind: String(fields[1]), detail: String(fields[2]))
        }
    }
}

private struct Proposal: Codable {
    let name: String
    let intent: String
    let origin: String
}

private struct Agent: Codable {
    let at: Int
    let session: String
    let event: String
    let tool: String
    let target: String
    let cwd: String
}

private struct Snapshot: Codable {
    let parallel: Int
    let agents: [Agent]
    let proposals: [Proposal]
    let spent: Double
    let jobs: [Job]
    let pending: Int
    let service: String
    let pid: Int32?
}

private final class FactoryMonitor {
    private let state: URL
    private let onChange: (String) -> Void
    private var directories: [URL: DispatchSourceFileSystemObject] = [:]
    private var serviceExit: DispatchSourceProcess?
    private var servicePID: Int32?
    private var lastSnapshot: String?

    init(state: URL, onChange: @escaping (String) -> Void) {
        self.state = state
        self.onChange = onChange
    }

    func refresh(watch: Bool) {
        let queue = state.appendingPathComponent("jobs", isDirectory: true).resolvingSymlinksInPath()
        if watch {
            watchPath(state)
            watchPath(queue)
            watchPath(state.appendingPathComponent("service-error.log"))
            watchPath(state.appendingPathComponent("proposals", isDirectory: true))
            watchPath(state.appendingPathComponent("agents", isDirectory: true))
        }
        let jobURLs = latestJobs(in: queue)
        if watch { for url in jobURLs { watchPath(url) } }
        let workFilesByJob = Dictionary(uniqueKeysWithValues: jobURLs.map { ($0, workFiles(in: $0)) })
        if watch {
            let workFiles = workFilesByJob.values.flatMap { $0 }
            for url in workFiles { watchPath(url) }
            let retained = Set(jobURLs + workFiles + [state, queue, state.appendingPathComponent("service-error.log"),
                state.appendingPathComponent("proposals", isDirectory: true),
                state.appendingPathComponent("agents", isDirectory: true)])
            for url in Array(directories.keys) where !retained.contains(url) { stopWatching(url) }
        }
        let jobs = jobURLs.compactMap { readJob($0, workFilesByJob[$0] ?? []) }
        let pid = runningService()
        if watch { watchServiceExit(pid) }
        let snapshot = Snapshot(
            parallel: parallelSlots(), agents: recentAgents(), proposals: waitingProposals(), spent: recentWorkerCost(in: queue), jobs: jobs, pending: jobs.filter(\.pending).count,
            service: state == defaultState ? (pid == nil ? "offline" : "running") : "unmanaged", pid: pid
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        guard let data = try? encoder.encode(snapshot), let json = String(data: data, encoding: .utf8),
              json != lastSnapshot else { return }
        lastSnapshot = json
        onChange(json)
    }

    // Slots for software jobs: STATE/parallel as the viewer set it, otherwise the service default of three.
    private func parallelSlots() -> Int {
        let text = try? String(contentsOf: state.appendingPathComponent("parallel"), encoding: .utf8)
        return min(8, max(1, text.flatMap { Int($0.trimmingCharacters(in: .whitespacesAndNewlines)) } ?? 3))
    }

    // Agent sessions that reported through tools/agent-hook.sh during the last ten minutes.
    private func recentAgents() -> [Agent] {
        let directory = state.appendingPathComponent("agents", isDirectory: true)
        let entries = (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles])) ?? []
        let now = Int(Date().timeIntervalSince1970)
        return entries.filter { $0.pathExtension == "json" }.compactMap { url in
            guard let data = try? Data(contentsOf: url), data.count < 4096,
                  let agent = try? JSONDecoder().decode(Agent.self, from: data), now - agent.at < 600 else { return nil }
            return agent
        }.sorted { $0.at > $1.at }
    }

    // Proposals the Factory wrote that nobody has approved yet.
    private func waitingProposals() -> [Proposal] {
        let directory = state.appendingPathComponent("proposals", isDirectory: true)
        let entries = (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles])) ?? []
        return entries.sorted { $0.lastPathComponent < $1.lastPathComponent }.compactMap { url in
            guard !FileManager.default.fileExists(atPath: url.appendingPathComponent("approved.txt").path),
                  let brief = try? String(contentsOf: url.appendingPathComponent("brief.md"), encoding: .utf8),
                  let intent = brief.split(separator: "\n").first else { return nil }
            let origin = (try? String(contentsOf: url.appendingPathComponent("origin.txt"), encoding: .utf8))?
                .split(separator: "\n").first { $0.hasPrefix("derived_from=") }
                .map { URL(fileURLWithPath: String($0.dropFirst("derived_from=".count))).deletingLastPathComponent().lastPathComponent } ?? ""
            return Proposal(name: url.lastPathComponent, intent: String(intent.prefix(300)), origin: origin)
        }
    }

    // Cost at list price that worker calls recorded during the last 24 hours, over the whole queue.
    private func recentWorkerCost(in queue: URL) -> Double {
        let keys: [URLResourceKey] = [.contentModificationDateKey]
        let jobs = (try? FileManager.default.contentsOfDirectory(at: queue, includingPropertiesForKeys: nil)) ?? []
        var total = 0.0
        for job in jobs {
            let files = (try? FileManager.default.contentsOfDirectory(at: job, includingPropertiesForKeys: keys)) ?? []
            for file in files where file.pathExtension == "run" {
                guard let modified = try? file.resourceValues(forKeys: Set(keys)).contentModificationDate,
                      modified.timeIntervalSinceNow > -86400,
                      let text = try? String(contentsOf: file, encoding: .utf8) else { continue }
                for line in text.split(separator: "\n") where line.hasPrefix("worker_cost_usd=") {
                    total += Double(line.dropFirst("worker_cost_usd=".count)) ?? 0
                }
            }
        }
        return (total * 1000).rounded() / 1000
    }

    private func latestJobs(in queue: URL) -> [URL] {
        // A job folder changes whenever a round adds a file, so this orders by latest activity.
        let keys: Set<URLResourceKey> = [.isDirectoryKey, .isSymbolicLinkKey, .contentModificationDateKey]
        let entries = (try? FileManager.default.contentsOfDirectory(
            at: queue, includingPropertiesForKeys: Array(keys), options: [.skipsHiddenFiles]
        )) ?? []
        let dated = entries.compactMap { url -> (URL, Date)? in
            guard url.lastPathComponent.hasPrefix("job-"),
                  let values = try? url.resourceValues(forKeys: keys),
                  values.isDirectory == true, values.isSymbolicLink != true else { return nil }
            return (url, values.contentModificationDate ?? .distantPast)
        }
        let ordered = dated.sorted { first, second in
            if first.1 == second.1 { return first.0.lastPathComponent > second.0.lastPathComponent }
            return first.1 > second.1
        }.map(\.0)
        return ordered.enumerated().compactMap { index, url in
            index < 12 || url.lastPathComponent == "job-floorplanner" ? url : nil
        }
    }

    private func workFiles(in url: URL) -> [URL] {
        let entries = (try? FileManager.default.contentsOfDirectory(at: url, includingPropertiesForKeys: nil)) ?? []
        return entries.filter {
            let basename = $0.deletingPathExtension().lastPathComponent
            return ["progress", "draft"].contains($0.pathExtension) && basename.hasPrefix("round-")
                && (1...20).contains(Int(basename.dropFirst(6)) ?? 0)
        }
    }

    private func readJob(_ url: URL, _ workFiles: [URL]) -> Job? {
        guard let baseline = Round(file: url.appendingPathComponent("baseline.run"))
            ?? Round(file: url.appendingPathComponent("request.run")),
              baseline.fields["decision"] == "accepted" else { return nil }
        var rounds: [Round] = []
        for index in 1...20 {
            let name = String(format: "round-%02d.run", index)
            guard let round = Round(file: url.appendingPathComponent(name)) else { break }
            rounds.append(round)
        }
        let progress = Dictionary(uniqueKeysWithValues: workFiles.filter { $0.pathExtension == "progress" }.map {
            ($0.deletingPathExtension().lastPathComponent, ProgressEvent.read($0))
        })
        let next = String(format: "round-%02d", rounds.count + 1)
        let active = baseline.fields["kind"] == "software" && progress[next] != nil
            && isLocked(url.appendingPathComponent("source.lock"))
        let submission = url.appendingPathComponent("submission.brief.md")
        let hasNextRound = rounds.isEmpty || rounds.last?.fields["advance"] == "true"
        let queued = !active && hasNextRound && FileManager.default.fileExists(atPath: submission.path)
        let startedAt = active ? (try? url.appendingPathComponent(next + ".brief.md")
            .resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate?.timeIntervalSince1970 : nil
        let requestIntent = baseline.name == "request.run" ? baseline.fields["intent"] : nil
        let goal = briefIntent(at: submission)
            ?? briefIntent(at: url.appendingPathComponent("round-01.brief.md"))
            ?? requestIntent ?? rounds.first?.fields["intent"]
        let preview = rounds.last(where: { $0.fields["decision"] == "accepted" }).flatMap {
            readPreview(for: $0, in: url)
        }
        // The accepted state before the latest one, for a diff of the result.
        let accepted = rounds.filter { $0.fields["decision"] == "accepted" }
        let previous = accepted.count > 1 ? readPreview(for: accepted[accepted.count - 2], in: url)
            : accepted.isEmpty ? nil : (readSource(at: url.appendingPathComponent("rollback.factory-view.html"))
                ?? readSource(at: url.appendingPathComponent("submission.seed.html")))
        // Everything this job spent on workers, including reflections, at list price.
        var cost = 0.0
        for file in (try? FileManager.default.contentsOfDirectory(at: url, includingPropertiesForKeys: nil)) ?? []
        where file.pathExtension == "run" {
            guard let text = try? String(contentsOf: file, encoding: .utf8) else { continue }
            for line in text.split(separator: "\n") where line.hasPrefix("worker_cost_usd=") {
                cost += Double(line.dropFirst("worker_cost_usd=".count)) ?? 0
            }
        }
        let draft = active ? readSource(at: url.appendingPathComponent(next + ".draft")) : nil
        return Job(id: url.lastPathComponent, goal: goal, preview: preview, previous: previous,
            cost: (cost * 10000).rounded() / 10000, draft: draft, baseline: baseline, rounds: rounds, progress: progress,
            sourceQueued: queued, sourceActive: active, sourceStartedAt: startedAt)
    }

    private func readPreview(for round: Round, in directory: URL) -> String? {
        let name = String(round.name.dropLast(4)) + ".html"
        guard round.fields["artifact"] == name else { return nil }
        return readSource(at: directory.appendingPathComponent(name))
    }

    private func readSource(at url: URL) -> String? {
        guard let info = try? url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
              info.isRegularFile == true, info.isSymbolicLink != true,
              let size = info.fileSize, size <= 96 * 1024 else { return nil }
        guard let text = try? String(contentsOf: url, encoding: .utf8), !text.isEmpty else { return nil }
        return text
    }

    private func briefIntent(at url: URL) -> String? {
        guard let info = try? url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
              info.isRegularFile == true, info.isSymbolicLink != true,
              let size = info.fileSize, size <= 8_192,
              let text = try? String(contentsOf: url, encoding: .utf8),
              let firstLine = text.split(omittingEmptySubsequences: false, whereSeparator: \.isNewline).first else { return nil }
        let intent = firstLine.trimmingCharacters(in: .whitespaces)
        return intent.isEmpty ? nil : intent
    }

    private func isLocked(_ url: URL) -> Bool {
        let fd = open(url.path, O_RDONLY | O_NONBLOCK | O_NOFOLLOW)
        guard fd >= 0 else { return false }
        defer { _ = close(fd) }
        if flock(fd, LOCK_SH | LOCK_NB) == 0 { return false }
        return errno == EWOULDBLOCK
    }

    private func watchPath(_ url: URL) {
        guard directories[url] == nil else { return }
        let descriptor = open(url.path, O_EVTONLY | O_CLOEXEC)
        guard descriptor >= 0 else { return }
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor, eventMask: [.write, .delete, .rename, .revoke], queue: .main
        )
        source.setEventHandler { [weak self, weak source] in
            guard let self, let source else { return }
            if !source.data.intersection([.delete, .rename, .revoke]).isEmpty { self.stopWatching(url) }
            self.refresh(watch: true)
        }
        source.setCancelHandler { _ = close(descriptor) }
        directories[url] = source
        source.resume()
    }

    private func stopWatching(_ url: URL) {
        directories.removeValue(forKey: url)?.cancel()
    }

    private func runningService() -> Int32? {
        guard state == defaultState else { return nil }
        let command = Process()
        command.executableURL = URL(fileURLWithPath: "/bin/launchctl")
        command.arguments = ["print", "gui/\(getuid())/local.taik.factory"]
        let pipe = Pipe()
        command.standardOutput = pipe
        command.standardError = FileHandle.nullDevice
        guard (try? command.run()) != nil else { return nil }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        command.waitUntilExit()
        guard command.terminationStatus == 0, let text = String(data: data, encoding: .utf8) else { return nil }
        for line in text.split(separator: "\n") {
            let field = line.trimmingCharacters(in: .whitespaces)
            if field.hasPrefix("pid = ") { return Int32(field.dropFirst(6)) }
        }
        return nil
    }

    private func watchServiceExit(_ pid: Int32?) {
        guard pid != servicePID else { return }
        serviceExit?.cancel()
        serviceExit = nil
        servicePID = pid
        guard let pid else { return }
        let source = DispatchSource.makeProcessSource(identifier: pid, eventMask: .exit, queue: .main)
        source.setEventHandler { [weak self] in
            self?.servicePID = nil
            self?.serviceExit?.cancel()
            self?.serviceExit = nil
            self?.refresh(watch: true)
        }
        serviceExit = source
        source.resume()
    }
}

private func scriptSafe(_ json: String) -> String {
    json.replacingOccurrences(of: "<", with: "\\u003c")
        .replacingOccurrences(of: "\u{2028}", with: "\\u2028")
        .replacingOccurrences(of: "\u{2029}", with: "\\u2029")
}

private enum ViewerAppearance: String {
    case system, light, dark

    static let preference = "appearance"

    var native: NSAppearance? {
        switch self {
        case .system: nil
        case .light: NSAppearance(named: .aqua)
        case .dark: NSAppearance(named: .darkAqua)
        }
    }
}

private final class FactoryWindow: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKScriptMessageHandler {
    private let state: URL
    private let fragment: String
    private var window: NSWindow!
    private var webView: WKWebView!
    private var monitor: FactoryMonitor!
    private var snapshot = "null"
    private var loaded = false

    init(state: URL, fragment: String) {
        self.state = state
        self.fragment = fragment
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1330, height: 790),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false
        )
        window.title = "TAIK Factory"
        window.minSize = NSSize(width: 375, height: 320)
        let appearance = UserDefaults.standard.string(forKey: ViewerAppearance.preference)
            .flatMap(ViewerAppearance.init(rawValue:)) ?? .system
        window.appearance = appearance.native
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.userContentController.add(self, name: "openArtifact")
        configuration.userContentController.add(self, name: "setAppearance")
        configuration.userContentController.add(self, name: "approveProposal")
        configuration.userContentController.add(self, name: "commentTicket")
        configuration.userContentController.add(self, name: "setParallel")
        webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = self
        window.contentView = webView
        installMenu()
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        monitor = FactoryMonitor(state: state) { [weak self] json in
            self?.snapshot = json
            self?.renderChanges()
        }
        monitor.refresh(watch: true)
        let content = fragment.replacingOccurrences(of: "/*FACTORY_DATA*/null", with: scriptSafe(snapshot))
        webView.loadHTMLString("""
        <!doctype html><html lang="de" data-appearance="\(appearance.rawValue)"><meta charset="utf-8">
        <meta name="viewport" content="width=device-width,initial-scale=1">
        <title>TAIK Factory</title>\(content)</html>
        """, baseURL: nil)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        loaded = true
        renderChanges()
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                 decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void) {
        let url = navigationAction.request.url?.absoluteString
        decisionHandler(url == "about:blank" || url == "about:srcdoc" ? .allow : .cancel)
    }

    private func renderChanges() {
        guard loaded else { return }
        webView.evaluateJavaScript("window.updateFactory(\(scriptSafe(snapshot)))")
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.frameInfo.isMainFrame else { return }
        if message.name == "setAppearance", let value = message.body as? String,
           let appearance = ViewerAppearance(rawValue: value) {
            UserDefaults.standard.set(appearance.rawValue, forKey: ViewerAppearance.preference)
            window.appearance = appearance.native
            return
        }
        if message.name == "setParallel", let slots = message.body as? Int, (1...8).contains(slots) {
            try? "\(slots)\n".write(to: state.appendingPathComponent("parallel"), atomically: true, encoding: .utf8)
            monitor.refresh(watch: false)
            return
        }
        if message.name == "approveProposal", let name = message.body as? String,
           name.range(of: #"^[a-z0-9-]{1,32}$"#, options: .regularExpression) != nil {
            // The user's click is the approval. The installed Factory queues the job.
            let approval = Process()
            approval.executableURL = state.appendingPathComponent("taik-seed")
            approval.arguments = ["approve", state.path, name]
            try? approval.run()
            return
        }
        if message.name == "commentTicket", let body = message.body as? [String: String],
           let name = body["name"], name.range(of: #"^[a-z0-9-]{1,32}$"#, options: .regularExpression) != nil,
           let text = body["text"], !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, text.count <= 8000 {
            // The user's comment is the reply to a ticket. The Factory reads STATE/inbox.
            let inbox = state.appendingPathComponent("inbox", isDirectory: true)
            try? FileManager.default.createDirectory(at: inbox, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            // One file per comment, never appended: a reply is a complete record, and nothing reads it half-written.
            let stem = name + "-" + ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "")
            let entry = inbox.appendingPathComponent(stem + ".md")
            let record = "name: \(name)\nat: \(ISO8601DateFormatter().string(from: Date()))\n\n\(text)\n"
            try? record.write(to: entry, atomically: true, encoding: .utf8)
            return
        }
        guard message.name == "openArtifact", let body = message.body as? [String: String],
              let job = body["job"], job.hasPrefix("job-"), !job.contains("/"),
              let name = body["name"], name.range(of: #"^round-[0-9]{2}(\.html|-taik-seed)$"#, options: .regularExpression) != nil else { return }
        let directory = state.appendingPathComponent("jobs", isDirectory: true).appendingPathComponent(job, isDirectory: true)
        let artifact = directory.appendingPathComponent(name)
        let record = directory.appendingPathComponent(String(name.prefix(8)) + ".run")
        guard let round = Round(file: record), round.fields["decision"] == "accepted", round.fields["artifact"] == name else { return }
        if artifact.pathExtension == "html" { NSWorkspace.shared.open(artifact) }
        else { NSWorkspace.shared.activateFileViewerSelecting([artifact]) }
    }

    private func installMenu() {
        let menu = NSMenu()
        let app = NSMenuItem()
        app.submenu = NSMenu()
        app.submenu?.addItem(withTitle: "Beenden", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        menu.addItem(app)
        NSApp.mainMenu = menu
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

let arguments = CommandLine.arguments
let state = arguments.firstIndex(of: "--state").flatMap { arguments.indices.contains($0 + 1) ? arguments[$0 + 1] : nil }
    .map { URL(fileURLWithPath: $0, isDirectory: true).standardizedFileURL } ?? defaultState
let fragmentPath = Bundle.main.url(forResource: "factory-view", withExtension: "html")
    ?? URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("factory-view.html")

if arguments.contains("--stream") {
    let monitor = FactoryMonitor(state: state) { json in print(json); fflush(stdout) }
    monitor.refresh(watch: true)
    withExtendedLifetime(monitor) { dispatchMain() }
} else {
    let fragment = try String(contentsOf: fragmentPath, encoding: .utf8)
    if arguments.contains("--snapshot") {
        FactoryMonitor(state: state) { json in
            print(fragment.replacingOccurrences(of: "/*FACTORY_DATA*/null", with: scriptSafe(json)))
        }.refresh(watch: false)
    } else {
        let app = NSApplication.shared
        let controller = FactoryWindow(state: state, fragment: fragment)
        app.delegate = controller
        app.setActivationPolicy(.regular)
        withExtendedLifetime(controller) { app.run() }
    }
}
