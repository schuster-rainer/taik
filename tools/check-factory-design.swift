import AppKit
import WebKit

final class FactoryDesignGate: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKScriptMessageHandler {
    var window: NSWindow!
    var view: WKWebView!
    var started = false
    var narrow = false
    var captures = false
    let arguments = CommandLine.arguments
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()

    func applicationDidFinishLaunching(_ notification: Notification) {
        let data = FileHandle.standardInput.readDataToEndOfFile()
        let marker = "window.updateFactory("
        guard var fragment = String(data: data, encoding: .utf8),
              let entry = fragment.range(of: marker),
              let javascript = try? String(contentsOf: root.appendingPathComponent("check-factory-design.js"), encoding: .utf8) else {
            finish(false, "Expected a Factory snapshot on stdin and check-factory-design.js beside the Swift gate.")
            return
        }
        fragment.insert(contentsOf: "window.designGateSnapshot = ", at: entry.upperBound)
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .nonPersistent()
        config.userContentController.add(self, name: "factoryDesignGate")
        view = WKWebView(frame: NSRect(x: 0, y: 0, width: 1300, height: 820), configuration: config)
        window = NSWindow(contentRect: view.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = view
        window.orderBack(nil)
        view.navigationDelegate = self
        let page = "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width, initial-scale=1\">"
            + fragment + "<script>" + javascript + "</script>"
        view.loadHTMLString(page, baseURL: nil)
        DispatchQueue.main.asyncAfter(deadline: .now() + 12) { [weak self] in
            self?.finish(false, "Timed out waiting for WebKit layout, Canvas, or snapshot evidence.")
        }
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                 decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void) {
        let url = navigationAction.request.url?.absoluteString
        decisionHandler(url == "about:blank" || url == "about:srcdoc" ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard !started else { return }
        started = true
        if arguments.contains("--capture") {
            // Let nested srcdoc frames paint before taking the untouched snapshot.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                self.captureIfRequested { self.startChecks() }
            }
        } else {
            startChecks()
        }
    }

    private func startChecks() {
        if captures && arguments.contains("--capture-only") {
            finish(true, "Captured the unmodified Factory snapshot.")
        } else {
            evaluate("void runDesignGate(\(arguments.contains("--require-open")))")
        }
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.frameInfo.isMainFrame, let body = message.body as? [String: Any],
              let state = body["state"] as? String else { return }
        if let observations = body["observations"],
           let json = try? JSONSerialization.data(withJSONObject: observations, options: [.sortedKeys]),
           let line = String(data: json, encoding: .utf8) {
            print(line)
        }
        switch state {
        case "desktop":
            guard !narrow else { return }
            narrow = true
            window.setContentSize(NSSize(width: 420, height: 820))
            evaluate("requestAnimationFrame(() => requestAnimationFrame(runNarrowDesignGate))")
        case "pass":
            finish(true, "Desktop and narrow history/viewport, live draft, Canvas interaction and theme checks passed.")
        case "fail":
            finish(false, body["detail"] as? String ?? "Unknown WebKit check failure.")
        default:
            break
        }
    }

    private func evaluate(_ script: String) {
        view.evaluateJavaScript(script) { [weak self] _, error in
            if let error { self?.finish(false, String(describing: error)) }
        }
    }

    private func captureIfRequested(_ continuation: @escaping () -> Void) {
        guard !captures, let i = arguments.firstIndex(of: "--capture"),
              arguments.indices.contains(i + 1) else { continuation(); return }
        captures = true
        let path = arguments[i + 1]
        view.takeSnapshot(with: nil) { [weak self] image, error in
            guard let image, let tiff = image.tiffRepresentation,
                  let representation = NSBitmapImageRep(data: tiff),
                  let png = representation.representation(using: .png, properties: [:]) else {
                self?.finish(false, "Could not capture WebKit snapshot: \(String(describing: error))")
                return
            }
            do {
                try png.write(to: URL(fileURLWithPath: path), options: .atomic)
                continuation()
            } catch {
                self?.finish(false, "Could not save WebKit snapshot: \(error)")
            }
        }
    }

    private func finish(_ passed: Bool, _ detail: String) {
        print("\(passed ? "PASS" : "FAIL"): \(detail)")
        exit(passed ? 0 : 1)
    }
}

let app = NSApplication.shared
let gate = FactoryDesignGate()
app.delegate = gate
withExtendedLifetime(gate) { app.run() }
