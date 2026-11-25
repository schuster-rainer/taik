import AppKit
import WebKit

final class SnapshotCapture: NSObject, NSApplicationDelegate, WKNavigationDelegate {
    let output: URL
    let markup: String
    private var window: NSWindow!
    private var viewer: WKWebView!
    private var scheduled = false

    init(output: URL, markup: String) {
        self.output = output
        self.markup = markup
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let frame = NSRect(x: 0, y: 0, width: 1440, height: 900)
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        viewer = WKWebView(frame: frame, configuration: configuration)
        viewer.navigationDelegate = self
        window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = viewer
        window.orderBack(nil)
        viewer.loadHTMLString("<!doctype html><html lang=de data-appearance=dark><meta charset=utf-8>" + markup, baseURL: nil)
        DispatchQueue.main.asyncAfter(deadline: .now() + 10) { exit(1) }
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                 decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void) {
        let url = navigationAction.request.url?.absoluteString
        decisionHandler(url == "about:blank" || url == "about:srcdoc" ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard !scheduled else { return }
        scheduled = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) {
            webView.takeSnapshot(with: nil) { image, error in
                guard let bytes = image?.tiffRepresentation,
                      let bitmap = NSBitmapImageRep(data: bytes),
                      let png = bitmap.representation(using: .png, properties: [:]) else {
                    if let error { print(error) }
                    exit(1)
                }
                do { try png.write(to: self.output, options: [.atomic]); exit(0) }
                catch { print(error); exit(1) }
            }
        }
    }
}

guard CommandLine.arguments.count == 2,
      let markup = String(data: FileHandle.standardInput.readDataToEndOfFile(), encoding: .utf8),
      !markup.isEmpty else {
    print("usage: factory-view --snapshot | swift capture-factory-view.swift OUTPUT.png")
    exit(2)
}
let app = NSApplication.shared
let capture = SnapshotCapture(output: URL(fileURLWithPath: CommandLine.arguments[1]), markup: markup)
app.delegate = capture
withExtendedLifetime(capture) { app.run() }
