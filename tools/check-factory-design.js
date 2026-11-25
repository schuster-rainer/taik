const find = selector => document.querySelector(selector);
const frame = selector => find(selector)?.shadowRoot?.querySelector('iframe');
const report = (state, observations, detail) => window.webkit.messageHandlers.factoryDesignGate.postMessage({state, observations, detail});
const requireFact = (condition, reason) => { if (!condition) throw new Error(reason); };
const rendered = el => !!el && el.getBoundingClientRect().height > 0 && getComputedStyle(el).visibility !== 'hidden';
const delayFrame = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
const select = (selector, value) => {
  const el = find(selector);
  requireFact(!!el, 'Missing selector: ' + selector);
  el.value = value;
  el.dispatchEvent(new Event('change', {bubbles: true}));
};
const evidence = '[design-gate] Prüfbefund im Live-Entwurf';
const draft = '<html><!-- ' + evidence + ' --><body>Lokaler Prüfentwurf</body></html>';

// Contract of the workpiece view: one pane shows the workpiece of the selected job. While a round
// runs it shows the live draft, marked as unverified, and the accepted result afterwards.
function renderFacts(mode) {
  const activity = find('#taik-activity'), history = find('[aria-label="Auftragsverlauf"]');
  const preview = find('#taik-preview'), flow = find('#taik-flow');
  const result = frame('#taik-result-viewport');
  for (const [el, label] of [[history,'history'], [preview,'preview'], [flow,'flow'], [result,'accepted result iframe']]) {
    requireFact(rendered(el), label + ' is not rendered');
  }
  for (const gone of ['#taik-job-content', '#taik-job-observation', '#taik-job-viewport']) {
    requireFact(!find(gone), 'The removed pane element still exists: ' + gone);
  }
  requireFact(!!history.querySelector('.rounds .round'), 'The visible history contains no completed rounds');
  requireFact(result.srcdoc.includes('<canvas'), 'The accepted result no longer contains the Floorplanner canvas');
  requireFact(result.getAttribute('sandbox') === 'allow-scripts', 'The accepted result lost its preview isolation');
  requireFact(flow.querySelectorAll('.flow-node').length === 5, 'The flow does not show its five stations');
  const rect = el => el.getBoundingClientRect().toJSON();
  const facts = {
    mode, viewport: innerWidth, scrollWidth: document.documentElement.scrollWidth,
    historyY: Math.round(rect(history).top), historyCount: history.querySelectorAll('.rounds .round').length,
    openDetails: history.querySelectorAll('details[open]').length,
    result: rect(result), flow: rect(flow)
  };
  requireFact(facts.result.top >= facts.flow.bottom - 1, 'The workpiece overlaps the flow');
  if (mode === 'desktop') {
    requireFact(rect(preview).left >= rect(activity).right - 1, 'Desktop right pane is not beside history/activity');
    requireFact(facts.result.bottom <= innerHeight + 1, 'The workpiece leaves the window');
    requireFact(facts.result.height >= innerHeight * 0.45, 'The workpiece does not use the freed height: ' + Math.round(facts.result.height));
  } else {
    requireFact(rect(preview).top >= rect(activity).bottom - 1, 'Narrow layout does not stack activity and preview');
    if (facts.scrollWidth > facts.viewport + 1) {
      const outside = [...document.querySelectorAll('#taik-live *')]
        .filter(el => rect(el).right > innerWidth + 1)
        .slice(0, 8)
        .map(el => [el.tagName, el.id || el.className, Math.round(rect(el).right), el.textContent.trim().slice(0, 60)]);
      throw new Error('Narrow layout overflows: ' + JSON.stringify({viewport:innerWidth, width:facts.scrollWidth, outside}));
    }
  }
  return facts;
}

async function runDesignGate(requireOpen) {
  try {
    const snapshot = window.designGateSnapshot;
    requireFact(snapshot?.jobs.some(job => job.id === 'job-floorplanner'), 'Snapshot lacks the verified Floorplanner control');
    select('#taik-live-job','job-floorplanner');
    await delayFrame();
    const history = () => find('[aria-label="Auftragsverlauf"]');
    const defaultOpen = history()?.querySelectorAll('details[open]').length ?? 0;
    if (requireOpen) requireFact(defaultOpen > 0, 'The selected history has no open work detail by default');
    const details = history()?.querySelector('details[data-round]');
    requireFact(!!details, 'History has no expandable persisted work detail');
    const selectedDetail = details.dataset.round;
    details.open = true;
    await delayFrame();
    const result = frame('#taik-result-viewport');
    requireFact(rendered(result), 'Floorplanner is not shown as the workpiece');
    const original = result.srcdoc;
    const modes = [...document.querySelectorAll('button[data-workpiece-mode]')].map(button => button.dataset.workpieceMode);
    requireFact(['view', 'source', 'diff'].every(mode => modes.includes(mode)), 'The workpiece lacks its three modes: ' + modes.join(','));

    // A round starts: the live draft takes the pane and is marked as unverified.
    const active = structuredClone(snapshot);
    active.pending = Math.max(1, active.pending);
    const job = active.jobs.find(entry => entry.id === 'job-floorplanner');
    job.sourceActive = true;
    job.draft = draft;
    const key = 'round-' + String(job.rounds.length + 1).padStart(2, '0');
    job.progress[key] = [{elapsed: 100, kind:'start', detail:evidence}];
    window.updateFactory(active);
    await delayFrame();
    requireFact([...history().querySelectorAll('details[open]')].some(el => el.dataset.round === selectedDetail),
      'A live update closed the selected history detail');
    requireFact(find('#taik-live-job').value === 'job-floorplanner', 'A live update dislodged the selected job');
    const draftFrame = frame('#taik-draft-viewport');
    requireFact(rendered(draftFrame) && draftFrame.srcdoc.includes(evidence), 'The live draft is not shown as the workpiece');
    requireFact(draftFrame.getAttribute('sandbox') === '', 'The live draft may run scripts');
    requireFact(!rendered(result), 'The accepted result is shown beside the live draft');
    requireFact(frame('#taik-result-viewport') === result && result.srcdoc === original, 'A draft update reloaded the accepted result');
    requireFact(/ungeprüft/i.test(find('#taik-preview').textContent), 'The live draft is not marked as unverified');
    requireFact(find('#taik-activity').textContent.includes(evidence), 'The history does not show the live Factory event');
    requireFact(find('#taik-flow .flow-node.active'), 'No station of the flow is active during a round');
    find('button[data-workpiece-mode="source"]').click();
    await delayFrame();
    const source = find('#taik-result-source');
    requireFact(rendered(source) && source.textContent.includes(evidence), 'Source mode does not show the draft text');
    requireFact(!rendered(draftFrame), 'Source mode still shows the rendered draft');
    job.progress[key].push({elapsed: 200, kind: 'checks', detail: 'Independent gate update'});
    window.updateFactory(active);
    await delayFrame();
    requireFact(find('#taik-activity').textContent.includes('Independent gate update'), 'The history stopped updating while the draft was unchanged');
    requireFact(rendered(find('#taik-result-source')), 'A live update left source mode');

    // The round is over: the accepted result returns, untouched.
    find('button[data-workpiece-mode="view"]').click();
    window.updateFactory(snapshot);
    await delayFrame();
    requireFact(frame('#taik-result-viewport') === result && result.srcdoc === original && rendered(result),
      'The accepted result did not return unchanged after the round');
    requireFact(!rendered(frame('#taik-draft-viewport')), 'The draft stayed after the round');
    requireFact(!find('#taik-flow .flow-node.active'), 'A station stays active after the round');
    const desktop = renderFacts('desktop');
    desktop.defaultOpenDetails = defaultOpen;
    await verifyCanvasAndTheme(result, original, desktop);
    report('desktop', desktop);
  } catch (error) {
    report('fail', undefined, String(error));
  }
}

function runNarrowDesignGate() {
  try {
    report('pass', renderFacts('narrow'));
  } catch (error) {
    report('fail', undefined, String(error));
  }
}

function verifyCanvasAndTheme(result, original, facts) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('Floorplanner theme or interaction observation did not arrive')), 5500);
    const token = '[factory-design-gate canvas]';
    const modes = ['light','dark','light'];
    let phase = 0, position, lightBackground, darkBackground;
    const receive = event => {
      const sample = event.data;
      if (event.source !== result.contentWindow || sample?.token !== token || sample.mode !== modes[phase]) return;
      try {
        requireFact(sample.canvas && sample.moved, 'Floorplanner Canvas did not paint or move a fixture');
        requireFact(!position || position === sample.position, 'Theme switch changed the edited fixture position');
        requireFact(document.documentElement.dataset.appearance === sample.mode, 'Preview appearance did not follow viewer appearance');
        position = sample.position;
        const background = getComputedStyle(document.body).backgroundColor;
        if (phase === 0) lightBackground = background;
        if (phase === 1) {
          darkBackground = background;
          requireFact(lightBackground !== darkBackground, 'Viewer uses the same surface in light and dark modes');
        }
        phase += 1;
        if (phase < modes.length) {
          select('#taik-appearance', modes[phase]);
        } else {
          requireFact(background === lightBackground, 'Returning to light did not restore the light surface');
          window.removeEventListener('message', receive);
          clearTimeout(timeout);
          facts.verifiedCanvasThemes = modes;
          facts.lightSurface = lightBackground;
          facts.darkSurface = darkBackground;
          resolve();
        }
      } catch (error) {
        clearTimeout(timeout);
        reject(error);
      }
    };
    window.addEventListener('message', receive);
    const instrument = `<script>
      const gCanvas = document.querySelector('canvas');
      const gUndo = document.querySelector('#undo');
      if (gUndo) for (const button of document.querySelectorAll('button[data-dx]')) {
        if (!gUndo.disabled) break;
        button.click();
      }
      function gSample() {
        const pixels = gCanvas?.getContext('2d')?.getImageData(0,0,gCanvas.width,gCanvas.height)?.data;
        const painted = pixels?.some((_, index) => index % 4 === 3 && pixels[index] === 255);
        parent.postMessage({token:${JSON.stringify(token)}, mode:document.documentElement.dataset.appearance,
          canvas:painted, moved:!!gUndo && !gUndo.disabled, position:document.querySelector('#details')?.textContent}, '*');
      }
      new MutationObserver(() => requestAnimationFrame(gSample)).observe(document.documentElement,
        {attributes:true,attributeFilter:['data-appearance']});
      requestAnimationFrame(gSample);
    <\/script>`;
    select('#taik-appearance','light');
    result.srcdoc = original + instrument;
  });
}
