import AppKit
import WebKit

final class Check: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKScriptMessageHandler {
    var window: NSWindow!
    var browser: WKWebView!
    var evaluated = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.userContentController.add(self, name: "check")
        browser = WKWebView(frame: NSRect(x: 0, y: 0, width: 1300, height: 760), configuration: configuration)
        browser.navigationDelegate = self
        window = NSWindow(contentRect: browser.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = browser
        window.orderBack(nil)
        let data = FileHandle.standardInput.readDataToEndOfFile()
        guard let page = String(data: data, encoding: .utf8), !page.isEmpty else {
            FileHandle.standardError.write(Data("Provide a Factory viewer snapshot on stdin.\n".utf8))
            exit(2)
        }
        browser.loadHTMLString("<!doctype html><meta charset=utf-8>" + page, baseURL: nil)
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) {
            FileHandle.standardError.write(Data("Timed out waiting for the Floorplanner appearance and network checks.\n".utf8))
            exit(1)
        }
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                 decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void) {
        let url = navigationAction.request.url?.absoluteString
        decisionHandler(url == "about:blank" || url == "about:srcdoc" ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard !evaluated else { return }
        evaluated = true
        webView.evaluateJavaScript(#"""
        const viewport = document.querySelector('#taik-result-viewport');
        const preview = viewport.shadowRoot.querySelector('iframe');
        const current = preview.srcdoc;
        function chooseJob(id) {
          const picker = document.querySelector('#taik-live-job');
          picker.value = id;
          picker.dispatchEvent(new Event('change', {bubbles:true}));
        }
        chooseJob('job-floorplanner');
        const code = document.querySelector('#taik-job-content .syntax-tag');
        if (!code || code.getBoundingClientRect().top < preview.getBoundingClientRect().bottom) {
          throw new Error('Highlighted job content is missing below the Floorplanner');
        }
        if (document.querySelector('#taik-live-job option[value=job-auftragsannahme]')) {
          chooseJob('job-auftragsannahme');
          const jobViewport = document.querySelector('#taik-job-viewport');
          if (jobViewport.hidden || !jobViewport.shadowRoot.querySelector('iframe').srcdoc.includes('Auftrag einreichen')) {
            throw new Error('Selected job did not open in the lower viewport');
          }
        }
        if (viewport.shadowRoot.querySelector('iframe') !== preview || preview.srcdoc !== current ||
            viewport.hidden || document.querySelector('#taik-preview button') ||
            preview.getBoundingClientRect().left < document.querySelector('#taik-activity').getBoundingClientRect().right ||
            !preview.srcdoc.includes('<canvas') ||
            !preview.srcdoc.includes('Rückgängig')) {
          throw new Error('Floorplanner canvas is missing from the right pane');
        }
        const appearance = document.querySelector('#taik-appearance');
        let phase = 0, networkBlocked = false, position;
        const expected = [
          ['light', 'rgb(248, 250, 252)', '#f3f4ee', '#f7f5e9'],
          ['dark', 'rgb(16, 25, 35)', '#192625', '#2e433d'],
          ['light', 'rgb(248, 250, 252)', '#f3f4ee', '#f7f5e9']
        ];
        function setAppearance() {
          const [mode, background] = expected[phase];
          appearance.value = mode;
          appearance.dispatchEvent(new Event('change', {bubbles:true}));
          if (getComputedStyle(document.body).backgroundColor !== background) {
            window.webkit.messageHandlers.check.postMessage('FAIL: viewer background did not follow ' + mode);
          }
        }
        window.addEventListener('message', event => {
          if (event.source !== preview.contentWindow) return;
          if (event.data === 'offline') networkBlocked = true;
          const sample = event.data?.sample;
          if (sample && phase < expected.length) {
            const [mode, , paper, floor] = expected[phase];
            if (sample.mode !== mode) return;
            if (sample.paper !== paper || sample.floor !== floor || !sample.painted ||
                !sample.hasUndo || (position && sample.position !== position)) {
              window.webkit.messageHandlers.check.postMessage('FAIL: Floorplanner appearance or edit state ' + JSON.stringify(sample));
              return;
            }
            position = sample.position;
            phase += 1;
            if (phase < expected.length) setAppearance();
          }
          if (phase === expected.length && networkBlocked && preview.srcdoc === current + extra) {
            window.webkit.messageHandlers.check.postMessage('PASS: Light, dark and light updated the viewer and Floorplanner canvas in place, preserved an edit, and blocked network access.');
          }
        });
        const extra = `<script>
          const canvas = document.querySelector('canvas');
          const undo = document.querySelector('#undo');
          for (const button of document.querySelectorAll('button[data-dx]')) {
            button.click();
            if (!undo.disabled) break;
          }
          function sample() {
            const style = getComputedStyle(document.documentElement);
            const floor = style.getPropertyValue('--map-floor').trim();
            const paper = style.getPropertyValue('--paper').trim();
            const rgb = [1,3,5].map(index => Number.parseInt(floor.slice(index,index+2),16));
            const pixels = canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
            const painted = pixels.some((r,i) => i % 4 === 0 && r === rgb[0] && pixels[i+1] === rgb[1] && pixels[i+2] === rgb[2] && pixels[i+3] === 255);
            parent.postMessage({sample:{mode:document.documentElement.dataset.appearance, paper, floor, painted,
              hasUndo:!undo.disabled, position:document.querySelector('#details').textContent}}, '*');
          }
          const observer = new MutationObserver(() => requestAnimationFrame(sample));
          observer.observe(document.documentElement,{attributes:true,attributeFilter:['data-appearance']});
          requestAnimationFrame(sample);
          document.addEventListener('securitypolicyviolation', e => {
            if (e.effectiveDirective === 'connect-src') parent.postMessage('offline', '*');
          });
          fetch('https://example.invalid/probe').catch(() => {});
        <\/script>`;
        let probeStarted = false;
        function runProbe() {
          if (probeStarted) return;
          probeStarted = true;
          setAppearance();
          preview.srcdoc = current + extra;
        }
        preview.addEventListener('load', runProbe, {once:true});
        setTimeout(runProbe, 120);
        """#) { _, error in
            if let error { print(error); exit(1) }
        }
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.frameInfo.isMainFrame else { return }
        print(message.body)
        exit(String(describing: message.body).hasPrefix("PASS:") ? 0 : 1)
    }
}

let app = NSApplication.shared
let check = Check()
app.delegate = check
withExtendedLifetime(check) { app.run() }
