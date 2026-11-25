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

// One message of the conversation that a job keeps in its folder.
private struct Message: Codable {
    let author: String
    let round: Int
    let text: String
}

private struct Job: Codable {
    let id: String
    let goal: String?
    let preview: String?
    let previous: String?
    let cost: Double
    let draft: String?
    let agent: String?
    let brief: String?
    let messages: [Message]
    let changes: [String: Change]
    let analyses: [String: String]
    let dialogs: [String: [[String: String]]]
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

// What a round changed against the state it started from.
private struct Change: Codable {
    let file: String
    let added: Int
    let removed: Int
    let fresh: Bool
    let rejected: Bool
    let lines: [String]
    let more: Int

    private static var cache: [String: Change] = [:]

    // A patch of several files, as an agent session leaves it.
    static func fromPatch(_ patch: String, files: String, rejected: Bool, stamp: String) -> Change {
        if let known = cache[stamp] { return known }
        var lines: [String] = [], more = 0, added = 0, removed = 0, names: [String] = []
        func push(_ line: String) { if lines.count < 240 { lines.append(line) } else { more += 1 } }
        for raw in patch.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = String(raw.prefix(400))
            if line.hasPrefix("+++ ") {
                let name = String(line.dropFirst(4)).replacingOccurrences(of: "b/", with: "", options: .anchored)
                if name != "/dev/null" { names.append(name); push("···"); push("  " + name) }
            } else if line.hasPrefix("diff --git") || line.hasPrefix("index ") || line.hasPrefix("--- ") || line.hasPrefix("new file") || line.hasPrefix("deleted file") {
                continue
            } else if line.hasPrefix("@@") { push("···") }
            else if line.hasPrefix("+") { added += 1; push("+ " + line.dropFirst()) }
            else if line.hasPrefix("-") { removed += 1; push("- " + line.dropFirst()) }
            else if line.hasPrefix(" ") { push("  " + line.dropFirst()) }
        }
        if lines.first == "···" { lines.removeFirst() }
        let label = names.isEmpty ? "keine Datei" : names.count <= 3 ? names.joined(separator: ", ") : "\(names.count) Dateien"
        let change = Change(file: label, added: added, removed: removed, fresh: false, rejected: rejected, lines: lines, more: more)
        if cache.count > 400 { cache.removeAll() }
        cache[stamp] = change
        return change
    }

    static func between(_ old: String?, _ new: String, file: String, rejected: Bool, stamp: String) -> Change {
        if let known = cache[stamp] { return known }
        let before = (old ?? "").split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        let after = new.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        var marks: [(Character, String)] = []
        if old == nil {
            marks = after.map { ("+", $0) }
        } else {
            let difference = after.difference(from: before)
            var removedAt = Set<Int>(), insertedAt = Set<Int>()
            for step in difference {
                switch step {
                case .remove(let offset, _, _): removedAt.insert(offset)
                case .insert(let offset, _, _): insertedAt.insert(offset)
                }
            }
            var i = 0, j = 0
            while i < before.count || j < after.count {
                if i < before.count, removedAt.contains(i) { marks.append(("-", before[i])); i += 1 }
                else if j < after.count, insertedAt.contains(j) { marks.append(("+", after[j])); j += 1 }
                else if i < before.count, j < after.count { marks.append((" ", after[j])); i += 1; j += 1 }
                else { break }
            }
        }
        let added = marks.filter { $0.0 == "+" }.count, removed = marks.filter { $0.0 == "-" }.count
        // The changed lines with two lines around them, as far as a ticket can carry.
        var keep = [Bool](repeating: false, count: marks.count)
        for (index, mark) in marks.enumerated() where mark.0 != " " {
            for near in max(0, index - 2)...min(marks.count - 1, index + 2) { keep[near] = true }
        }
        var lines: [String] = [], gap = false, more = 0
        for (index, mark) in marks.enumerated() {
            guard keep[index] else { gap = true; continue }
            if lines.count >= 240 { more += 1; continue }
            if gap, !lines.isEmpty { lines.append("···") }
            gap = false
            lines.append("\(mark.0) " + String(mark.1.prefix(400)))
        }
        let change = Change(file: file, added: added, removed: removed, fresh: old == nil, rejected: rejected, lines: lines, more: more)
        if cache.count > 400 { cache.removeAll() }
        cache[stamp] = change
        return change
    }
}

private struct Proposal: Codable {
    let name: String
    let intent: String
    let origin: String
    var brief: String = ""
    var messages: [Message] = []
    // Set for a request of the user without a proposal: deriving, declined or failed.
    var state: String? = nil
    var note: String? = nil
}

private struct Agent: Codable {
    let at: Int
    let session: String
    let event: String
    let tool: String
    let target: String
    let cwd: String
}

// One light line per job of the stock, so that every job can be found.
private struct StockEntry: Codable {
    let id: String
    let title: String
    let decision: String
    let rounds: Int
}

private struct Snapshot: Codable {
    let index: [StockEntry]
    let crew: String?
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
            watchPath(state.appendingPathComponent("requests", isDirectory: true))
            watchPath(state.appendingPathComponent("agents", isDirectory: true))
            watchPath(state.appendingPathComponent("crew", isDirectory: true))
        }
        let jobURLs = latestJobs(in: queue)
        if watch { for url in jobURLs { watchPath(url) } }
        let workFilesByJob = Dictionary(uniqueKeysWithValues: jobURLs.map { ($0, workFiles(in: $0)) })
        if watch {
            let workFiles = workFilesByJob.values.flatMap { $0 }
            for url in workFiles { watchPath(url) }
            let retained = Set(jobURLs + workFiles + [state, queue, state.appendingPathComponent("service-error.log"),
                state.appendingPathComponent("proposals", isDirectory: true),
                state.appendingPathComponent("requests", isDirectory: true),
                state.appendingPathComponent("agents", isDirectory: true),
                state.appendingPathComponent("crew", isDirectory: true)])
            for url in Array(directories.keys) where !retained.contains(url) { stopWatching(url) }
        }
        let jobs = jobURLs.compactMap { readJob($0, workFilesByJob[$0] ?? []) }
        let pid = runningService()
        if watch { watchServiceExit(pid) }
        let snapshot = Snapshot(
            index: stock(in: queue), crew: crewOverview(), parallel: parallelSlots(), agents: recentAgents(), proposals: waitingProposals(), spent: recentWorkerCost(in: queue), jobs: jobs, pending: jobs.filter(\.pending).count,
            service: state == defaultState ? (pid == nil ? "offline" : "running") : "unmanaged", pid: pid
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        guard let data = try? encoder.encode(snapshot), let json = String(data: data, encoding: .utf8),
              json != lastSnapshot else { return }
        lastSnapshot = json
        onChange(json)
    }

    // The overview that `taik-seed crew` wrote: harnesses, sign-in state and the defined agents.
    private func crewOverview() -> String? {
        let url = state.appendingPathComponent("crew", isDirectory: true).appendingPathComponent("overview.json")
        guard let data = try? Data(contentsOf: url), data.count < 262_144 else { return nil }
        return String(data: data, encoding: .utf8)
    }

    // The description of a job: its brief as it was queued.
    // The work order of a job: a document job or an agent session.
    private func briefURL(of job: URL) -> URL {
        let session = job.appendingPathComponent("session.brief.md")
        return FileManager.default.fileExists(atPath: session.path) ? session : job.appendingPathComponent("submission.brief.md")
    }

    // What was said and done in the rounds of an agent session, as the Factory noted it while it happened.
    private func roundDialogs(_ rounds: Int, in directory: URL) -> [String: [[String: String]]] {
        var dialogs: [String: [[String: String]]] = [:]
        for number in 1...max(1, rounds + 1) {
            let stem = String(format: "round-%02d", number)
            guard let data = try? Data(contentsOf: directory.appendingPathComponent(stem + ".dialog.jsonl")), data.count <= 2_000_000,
                  let text = String(data: data, encoding: .utf8) else { continue }
            let entries = text.split(separator: "\n").compactMap { line -> [String: String]? in
                guard let entry = try? JSONSerialization.jsonObject(with: Data(line.utf8)) as? [String: String] else { return nil }
                return entry.mapValues { String($0.prefix(4000)) }
            }
            if !entries.isEmpty { dialogs[stem] = Array(entries.suffix(300)) }
        }
        return dialogs
    }

    private func jobBrief(in job: URL) -> String? {
        for name in ["submission.brief.md", "session.brief.md", "round-01.brief.md"] {
            if let data = try? Data(contentsOf: job.appendingPathComponent(name)), data.count <= 8192,
               let text = String(data: data, encoding: .utf8) { return text }
        }
        return nil
    }

    // The conversation of a job, oldest first, as the Factory wrote it into the job folder.
    private func jobMessages(in job: URL) -> [Message] {
        var messages: [Message] = []
        for sequence in 1...200 {
            var found: Message?
            for author in ["user", "agent"] {
                let url = job.appendingPathComponent(String(format: "message-%03d.", sequence) + author + ".md")
                guard let data = try? Data(contentsOf: url), data.count <= 8300,
                      let text = String(data: data, encoding: .utf8),
                      let split = text.range(of: "\n\n"),
                      let round = Int(text[..<split.lowerBound].replacingOccurrences(of: "round=", with: "")) else { continue }
                found = Message(author: author, round: round,
                    text: String(text[split.upperBound...]).trimmingCharacters(in: .whitespacesAndNewlines))
            }
            guard let message = found else { break }
            messages.append(message)
        }
        return messages
    }

    // The agent a job names for its next round.
    private func jobAgent(in job: URL) -> String? {
        guard let text = try? String(contentsOf: job.appendingPathComponent("agent"), encoding: .utf8) else { return nil }
        let name = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return name.range(of: #"^[a-z0-9-]{1,32}$"#, options: .regularExpression) != nil ? name : nil
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
            return Proposal(name: url.lastPathComponent, intent: String(intent.prefix(300)), origin: origin,
                brief: String(brief.prefix(8192)), messages: jobMessages(in: url))
        } + openRequests()
    }

    // Requests of the user that have no proposal: still being derived, or answered without one during the last day.
    private func openRequests() -> [Proposal] {
        let directory = state.appendingPathComponent("requests", isDirectory: true)
        let entries = (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: [.contentModificationDateKey],
            options: [.skipsHiddenFiles])) ?? []
        let field: (String, String) -> String = { text, key in
            text.split(separator: "\n").first { $0.hasPrefix(key + "=") }.map { String($0.dropFirst(key.count + 1)) } ?? ""
        }
        return entries.filter { $0.pathExtension == "run" && $0.lastPathComponent.hasPrefix("ticket-") }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }.compactMap { url in
            let name = url.deletingPathExtension().lastPathComponent
            guard !FileManager.default.fileExists(atPath: state.appendingPathComponent("proposals/" + name).path),
                  let record = try? String(contentsOf: url, encoding: .utf8) else { return nil }
            let text = [field(record, "intent"), field(record, "observation")].filter { !$0.isEmpty }.joined(separator: "\n\n")
            var request = Proposal(name: name, intent: String(field(record, "intent").prefix(300)), origin: "", brief: text, state: "deriving")
            let answer = directory.appendingPathComponent("reflection-" + name + ".run")
            let failure = directory.appendingPathComponent(name + ".failed")
            for (file, state) in [(answer, "declined"), (failure, "failed")] {
                guard let told = try? String(contentsOf: file, encoding: .utf8) else { continue }
                let changed = (try? file.resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate ?? .distantPast
                if Date().timeIntervalSince(changed) > 86_400 { return nil }
                request.state = state
                request.note = state == "declined" ? String(field(told, "outcome").prefix(600)) : String(told.prefix(600))
            }
            return request
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

    // The job the user opened from the stock. It is loaded in full beside the latest ones.
    var shownJob = ""

    private func stock(in queue: URL) -> [StockEntry] {
        let entries = (try? FileManager.default.contentsOfDirectory(at: queue, includingPropertiesForKeys: nil, options: [.skipsHiddenFiles])) ?? []
        return entries.filter { $0.lastPathComponent.hasPrefix("job-") }.sorted { $0.lastPathComponent < $1.lastPathComponent }.compactMap { url in
            guard FileManager.default.fileExists(atPath: url.appendingPathComponent("request.run").path)
                || FileManager.default.fileExists(atPath: url.appendingPathComponent("baseline.run").path) else { return nil }
            var last: Round?
            var rounds = 0
            for number in 1...99 {
                guard let round = Round(file: url.appendingPathComponent(String(format: "round-%02d.run", number))) else { break }
                last = round
                rounds = number
            }
            let brief = (try? String(contentsOf: briefURL(of: url), encoding: .utf8))?
                .split(separator: "\n").first.map(String.init)
            let title = brief ?? last?.fields["intent"] ?? ""
            return StockEntry(id: url.lastPathComponent, title: String(title.prefix(300)), decision: last?.fields["decision"] ?? "", rounds: rounds)
        }
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
            index < 12 || url.lastPathComponent == "job-floorplanner" || url.lastPathComponent == shownJob || waits(url) ? url : nil
        }
    }

    // A job that waits is always loaded in full, however old it is: waiting is not a decision.
    // It waits when its last round was rejected, when its agent asked last, or when a round is still due.
    private func waits(_ url: URL) -> Bool {
        var last: Round?
        for number in 1...99 {
            guard let round = Round(file: url.appendingPathComponent(String(format: "round-%02d.run", number))) else { break }
            last = round
        }
        if last?.fields["decision"] == "rejected" || last?.fields["advance"] == "true" { return true }
        if jobMessages(in: url).last?.author == "agent" { return true }
        return last == nil && FileManager.default.fileExists(atPath: briefURL(of: url).path)
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
        let submission = briefURL(of: url)
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
            cost: (cost * 10000).rounded() / 10000, draft: draft, agent: jobAgent(in: url), brief: jobBrief(in: url), messages: jobMessages(in: url), changes: roundChanges(rounds, in: url), analyses: roundAnalyses(rounds, in: url), dialogs: roundDialogs(rounds.count, in: url), baseline: baseline, rounds: rounds, progress: progress,
            sourceQueued: queued, sourceActive: active, sourceStartedAt: startedAt)
    }

    // The short analysis the Factory wrote for a rejected round.
    private func roundAnalyses(_ rounds: [Round], in directory: URL) -> [String: String] {
        var analyses: [String: String] = [:]
        for round in rounds where round.fields["decision"] == "rejected" {
            let stem = String(round.name.dropLast(4))
            if let data = try? Data(contentsOf: directory.appendingPathComponent(stem + ".analysis.md")), data.count <= 1600,
               let text = String(data: data, encoding: .utf8) { analyses[stem] = text }
        }
        return analyses
    }

    // What each round changed: its artifact, or its rejected candidate, against the state accepted before it.
    private func roundChanges(_ rounds: [Round], in directory: URL) -> [String: Change] {
        func text(_ name: String) -> (String, String)? {
            let url = directory.appendingPathComponent(name)
            guard let values = try? url.resourceValues(forKeys: [.fileSizeKey, .contentModificationDateKey, .isRegularFileKey]),
                  values.isRegularFile == true, let size = values.fileSize, size <= 98_304,
                  let data = try? Data(contentsOf: url), let content = String(data: data, encoding: .utf8) else { return nil }
            return (content, "\(url.path):\(size):\(values.contentModificationDate?.timeIntervalSince1970 ?? 0)")
        }
        var base = text("submission.seed.html")
        var changes: [String: Change] = [:]
        for round in rounds {
            let stem = String(round.name.dropLast(4))
            if round.fields["decision"] == "accepted", let artifact = round.fields["artifact"], artifact.hasSuffix(".html"),
               let new = text(artifact) {
                // A release keeps what the artifact replaced. That is the state this round started from.
                if base == nil { base = text("rollback." + stem + ".html") }
                changes[stem] = Change.between(base?.0, new.0, file: artifact, rejected: false, stamp: (base?.1 ?? "") + "|" + new.1)
                base = new
            } else if let commit = round.fields["commit"], !commit.isEmpty, let patch = text(stem + ".diff") {
                changes[stem] = Change.fromPatch(patch.0, files: round.fields["files"] ?? "", rejected: round.fields["decision"] == "rejected", stamp: patch.1)
            } else if round.fields["decision"] == "rejected", let new = text(stem + ".rejected.txt") {
                changes[stem] = Change.between(base?.0, new.0, file: stem + ".rejected.txt", rejected: true, stamp: (base?.1 ?? "") + "|" + new.1)
            }
        }
        return changes
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
        configuration.userContentController.add(self, name: "newTicket")
        configuration.userContentController.add(self, name: "showJob")
        configuration.userContentController.add(self, name: "setParallel")
        for name in ["saveAgent", "deleteAgent", "setJobAgent", "refreshCrew", "attention"] {
            configuration.userContentController.add(self, name: name)
        }
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
        refreshCrew()
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

    // Asks the installed Factory which harnesses are here and signed in. It writes STATE/crew/overview.json,
    // which the monitor watches. The service's own settings are passed on so that both see the same accounts.
    private func refreshCrew() {
        let seed = state.appendingPathComponent("taik-seed")
        guard FileManager.default.isExecutableFile(atPath: seed.path) else { return }
        let probe = Process()
        probe.executableURL = seed
        probe.arguments = ["crew", state.path]
        var environment = ProcessInfo.processInfo.environment
        let plist = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents/local.taik.factory.plist")
        if let service = NSDictionary(contentsOf: plist)?["EnvironmentVariables"] as? [String: String] {
            for (key, value) in service where key.hasPrefix("TAIK_") || key == "PATH" { environment[key] = value }
        }
        probe.environment = environment
        probe.standardOutput = FileHandle.nullDevice
        probe.standardError = FileHandle.nullDevice
        try? probe.run()
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.frameInfo.isMainFrame else { return }
        if message.name == "setAppearance", let value = message.body as? String,
           let appearance = ViewerAppearance(rawValue: value) {
            UserDefaults.standard.set(appearance.rawValue, forKey: ViewerAppearance.preference)
            window.appearance = appearance.native
            return
        }
        // Agents decide which account a round spends. Every value is checked against a fixed form,
        // and files are written only into STATE/crew or as the one file `agent` of a known job.
        let agentName = #"^[a-z0-9-]{1,32}$"#
        let crew = state.appendingPathComponent("crew", isDirectory: true)
        if message.name == "refreshCrew" {
            refreshCrew()
            return
        }
        if message.name == "saveAgent", let body = message.body as? [String: String],
           let name = body["name"], name.range(of: agentName, options: .regularExpression) != nil,
           let harness = body["harness"], ["claude", "codex", "savant", "grok", "pi"].contains(harness),
           let model = body["model"], model.range(of: #"^[A-Za-z0-9._:/-]{0,80}$"#, options: .regularExpression) != nil,
           let effort = body["effort"], ["", "off", "minimal", "low", "medium", "high", "xhigh", "max"].contains(effort) {
            let persona = (body["persona"] ?? "").components(separatedBy: .newlines).joined(separator: " ")
                .trimmingCharacters(in: .whitespaces)
            guard persona.utf8.count <= 600 else { return }
            try? FileManager.default.createDirectory(at: crew, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            let text = "harness=\(harness)\nmodel=\(model)\neffort=\(effort)\npersona=\(persona)\nend\n"
            try? text.write(to: crew.appendingPathComponent(name + ".agent"), atomically: true, encoding: .utf8)
            refreshCrew()
            return
        }
        if message.name == "deleteAgent", let name = message.body as? String,
           name.range(of: agentName, options: .regularExpression) != nil {
            try? FileManager.default.removeItem(at: crew.appendingPathComponent(name + ".agent"))
            refreshCrew()
            return
        }
        if message.name == "setJobAgent", let body = message.body as? [String: String],
           let job = body["job"], job.range(of: #"^job-[A-Za-z0-9-]{1,60}$"#, options: .regularExpression) != nil,
           let agent = body["agent"] {
            let folder = state.appendingPathComponent("jobs", isDirectory: true).appendingPathComponent(job, isDirectory: true)
            var isFolder: ObjCBool = false
            guard FileManager.default.fileExists(atPath: folder.path, isDirectory: &isFolder), isFolder.boolValue else { return }
            let choice = folder.appendingPathComponent("agent")
            if agent.isEmpty {
                try? FileManager.default.removeItem(at: choice)
            } else if agent.range(of: agentName, options: .regularExpression) != nil,
                      FileManager.default.fileExists(atPath: crew.appendingPathComponent(agent + ".agent").path) {
                try? (agent + "\n").write(to: choice, atomically: true, encoding: .utf8)
            }
            monitor.refresh(watch: false)
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
        if message.name == "showJob", let name = message.body as? String,
           name.range(of: #"^job-[A-Za-z0-9-]{1,60}$"#, options: .regularExpression) != nil {
            monitor.shownJob = name
            monitor.refresh(watch: true)
            return
        }
        if message.name == "attention" {
            // A ticket started to wait for the user: a sound and the Dock, no notification service.
            NSSound(named: "Glass")?.play()
            NSApp.requestUserAttention(.informationalRequest)
            return
        }
        if message.name == "commentTicket", let body = message.body as? [String: String],
           let name = body["name"], name.range(of: #"^job-[A-Za-z0-9-]{1,60}$"#, options: .regularExpression) != nil,
           let text = body["text"], !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, text.utf8.count <= 8000 {
            // A reply to a job belongs into the job. The installed Factory writes it there and wakes the service.
            let reply = Process()
            reply.executableURL = state.appendingPathComponent("taik-seed")
            reply.arguments = ["reply", state.path, name]
            let input = Pipe()
            reply.standardInput = input
            reply.standardOutput = FileHandle.nullDevice
            reply.standardError = FileHandle.nullDevice
            if (try? reply.run()) != nil {
                input.fileHandleForWriting.write(Data(text.utf8))
                try? input.fileHandleForWriting.close()
            }
            return
        }
        if message.name == "commentTicket" || message.name == "newTicket", let body = message.body as? [String: String],
           let text = body["text"], !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
           text.utf8.count <= (message.name == "newTicket" ? 4000 : 8000) {
            // A comment on a waiting proposal stays with the proposal. A new ticket is a request the service derives a proposal from.
            var arguments = ["request", state.path]
            if message.name == "commentTicket" {
                guard let name = body["name"], name.range(of: #"^[a-z0-9-]{1,32}$"#, options: .regularExpression) != nil else { return }
                arguments = ["reply", state.path, name]
            }
            let call = Process()
            call.executableURL = state.appendingPathComponent("taik-seed")
            call.arguments = arguments
            let input = Pipe()
            call.standardInput = input
            call.standardOutput = FileHandle.nullDevice
            call.standardError = FileHandle.nullDevice
            if (try? call.run()) != nil {
                input.fileHandleForWriting.write(Data(text.utf8))
                try? input.fileHandleForWriting.close()
            }
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
