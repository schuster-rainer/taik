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

// A job is reached through its ticket, the way a person does it: find the group that holds it, click it.
async function chooseTicket(id) {
  for (const tab of ['open', 'run', 'decision']) {
    find('button[data-board-tab="' + tab + '"]')?.click();
    await delayFrame();
    const ticket = find('#taik-board button[data-ticket="' + id + '"]');
    if (ticket && rendered(ticket)) {
      ticket.click();
      await delayFrame();
      requireFact(find('#taik-ticket')?.dataset.job === id, 'A click on the ticket did not open it: ' + id);
      return;
    }
  }
  throw new Error('No ticket for ' + id);
}

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
  // The flow is the workflow of the chosen ticket: six stations, each with who carries it out and its state.
  const stations = [...flow.querySelectorAll('.flow-node')].map(node => node.querySelector('.flow-name')?.textContent.trim());
  requireFact(JSON.stringify(stations) === JSON.stringify(['Wunsch', 'Plan des Orchestrators', 'Gate', 'Freigabe', 'Arbeit', 'Urteil']), 'The flow does not show the six stations of a ticket: ' + stations.join(', '));
  requireFact(flow.querySelectorAll('.flow-node.active').length <= 1, 'More than one station of the flow is active');
  // The orchestrator comes first in the left column, then the tickets, then the ticket.
  const order = ['#taik-orchestrator', '#taik-board', '#taik-ticket'].map(id => find(id)?.getBoundingClientRect().top ?? NaN);
  requireFact(order[0] < order[1] && order[1] < order[2], 'The left column is not ordered orchestrator, tickets, ticket: ' + order.join(', '));
  const rect = el => el.getBoundingClientRect().toJSON();
  const facts = {
    mode, viewport: innerWidth, scrollWidth: document.documentElement.scrollWidth,
    historyY: Math.round(rect(history).top), historyCount: history.querySelectorAll('.rounds .round').length,
    openDetails: history.querySelectorAll('details[open]').length,
    result: rect(result), flow: rect(flow)
  };
  requireFact(facts.result.top >= facts.flow.bottom - 1, 'The workpiece overlaps the flow');
  // One ticket holds the job: description, conversation, history. There is no second job list.
  const ticket = find('#taik-ticket');
  requireFact(rendered(ticket), 'The ticket is not rendered');
  requireFact(!find('#taik-live-job'), 'The separate job list still exists');
  requireFact(ticket.contains(history), 'The history is not part of the ticket');
  requireFact((ticket.querySelector('.ticket-brief')?.textContent.trim().length ?? 0) > 20, 'The ticket lacks its description');
  const cut = [...ticket.querySelectorAll('.ticket-title, .thread-message, .round-checks, .round h3')]
    .filter(el => rendered(el) && el.scrollWidth > el.clientWidth + 1).map(el => el.className + ': ' + el.textContent.trim().slice(0, 40));
  requireFact(!cut.length, 'Text in the ticket is cut off: ' + cut.join(' | '));
  requireFact(rect(ticket).right <= rect(activity).right + 1, 'The ticket leaves its column');
  // Ticket board: one group at a time, every ticket as wide as its list, compact enough to leave the history in view.
  const board = find('#taik-board');
  if (board) {
    const shown = [...board.querySelectorAll('.board-column')].filter(rendered);
    requireFact(shown.length === 1, 'The board shows ' + shown.length + ' groups instead of one');
    const tickets = [...shown[0].querySelectorAll('.ticket')];
    const widths = tickets.map(ticket => Math.round(rect(ticket).width));
    requireFact(new Set(widths).size <= 1, 'Tickets differ in width: ' + widths.join(','));
    if (tickets.length) {
      const list = shown[0].querySelector('.board-list');
      requireFact(widths[0] >= list.clientWidth - 16, 'A ticket does not span its list: ' + widths[0] + ' of ' + list.clientWidth);
      requireFact(tickets.every(ticket => rect(ticket).right <= rect(board).right + 1), 'A ticket leaves the board');
    }
    requireFact(![...board.querySelectorAll('.ticket .title')].some(title => /^\s*#/.test(title.textContent)), 'A ticket title starts with a raw heading mark');
    requireFact(board.querySelectorAll('button[data-board-tab]').length === 3, 'The board lacks its three tabs');
    facts.board = { height: Math.round(rect(board).height), tickets: tickets.length, ticketWidth: widths[0] ?? 0 };
    if (mode === 'desktop') {
      requireFact(facts.board.height <= innerHeight * 0.5, 'The board takes more than half the window: ' + facts.board.height);
    }
  }
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
    await chooseTicket('job-floorplanner');
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
    // Conversation: a question of the worker puts the job among the waiting tickets, and its ticket tells it in full.
    {
      const asked = structuredClone(snapshot);
      const asking = asked.jobs.find(entry => entry.id === 'job-floorplanner');
      const question = 'Gate-Frage: welche Breite soll der Gang zwischen den Regalen haben, und gilt sie auch vor den Kassen?';
      asking.messages = [{ author: 'agent', round: asking.rounds.length, text: question }];
      window.updateFactory(asked);
      await delayFrame();
      requireFact(!!find('#taik-board-open button[data-ticket="job-floorplanner"]'), 'A job whose worker asked is not among the waiting tickets');
      const ticket = find('#taik-ticket');
      requireFact(rendered(ticket) && ticket.textContent.includes(question), 'The ticket does not show the whole question of the worker');
      const message = ticket.querySelector('.thread-message');
      requireFact(message.scrollWidth <= message.clientWidth + 1 && message.scrollHeight <= message.clientHeight + 1, 'A message is cut off');
      requireFact(ticket.querySelectorAll('.rounds .round').length === asking.rounds.length, 'The ticket does not hold its rounds');
      requireFact(rendered(ticket.querySelector('textarea')), 'The ticket has no reply field');
      window.updateFactory(snapshot);
      await delayFrame();
      requireFact(!find('#taik-ticket').textContent.includes(question), 'The question stayed after the snapshot changed');
    }
    // A proposal of the Factory is a ticket too: a click shows it with approval and comment.
    {
      const proposed = structuredClone(snapshot);
      proposed.proposals = [{ name: 'probe-vorschlag', intent: 'Einen Vorschlag als Ticket zeigen', origin: 'job-floorplanner' }];
      window.updateFactory(proposed);
      await delayFrame();
      find('button[data-board-tab="open"]').click();
      await delayFrame();
      find('#taik-board button[data-ticket="proposal:probe-vorschlag"]').click();
      await delayFrame();
      const ticket = find('#taik-ticket');
      requireFact(ticket?.dataset.proposal === 'probe-vorschlag', 'A click on a proposal did not open it as the ticket');
      requireFact(rendered(ticket.querySelector('button[data-proposal="probe-vorschlag"]')) && rendered(ticket.querySelector('textarea')), 'The proposal ticket lacks approval or comment');
      requireFact(find('#taik-board button[data-ticket="proposal:probe-vorschlag"]').getAttribute('aria-current') === 'true', 'The shown proposal is not marked in the list');
      window.updateFactory(snapshot);
      await delayFrame();
      await chooseTicket('job-floorplanner');
    }
    // A request of the user: the field keeps what is typed, and the request shows where it stands.
    {
      const field = find('.new-ticket textarea');
      requireFact(rendered(field) && rendered(find('#taik-new-ticket')), 'There is no field to open a ticket');
      field.value = 'Ein Rechner für Brotteig';
      field.dispatchEvent(new Event('input', { bubbles: true }));
      const requested = structuredClone(snapshot);
      requested.proposals = [
        { name: 'ticket-901', intent: 'Ein Rechner für Brotteig', origin: '', brief: 'Ein Rechner für Brotteig\n\nMit Hydration.', messages: [], state: 'deriving' },
        { name: 'ticket-902', intent: 'Unklar', origin: '', brief: 'Unklar', messages: [], state: 'declined', note: 'Der Wunsch nennt nichts Prüfbares.' },
        { name: 'ticket-903', intent: 'Fertig abgeleitet', origin: 'requests', brief: 'Fertig abgeleitet\n\nVerlangt: ein Titel.', messages: [{ author: 'user', round: 0, text: 'Bitte in Blau.' }], gate: 'test -f hello.txt || exit 1\necho PASS', plan: { kind: 'sitzung', repo: 'taik', gate: 'savant', work: 'standard', reason: 'Es müssen bestehende Dateien geändert werden.' } },
      ];
      window.updateFactory(requested);
      await delayFrame();
      requireFact(find('.new-ticket textarea').value === 'Ein Rechner für Brotteig', 'A live update emptied the new ticket');
      find('button[data-board-tab="run"]').click();
      await delayFrame();
      find('#taik-board button[data-ticket="proposal:ticket-901"]').click();
      await delayFrame();
      requireFact(find('#taik-ticket').textContent.includes('Mit Hydration.') && !find('#taik-ticket button[data-proposal]'), 'A request being derived shows its text and cannot be approved');
      find('button[data-board-tab="decision"]').click();
      await delayFrame();
      find('#taik-board button[data-ticket="proposal:ticket-902"]').click();
      await delayFrame();
      requireFact(find('#taik-ticket').textContent.includes('Der Wunsch nennt nichts Prüfbares.'), 'A declined request does not tell its reason');
      find('button[data-board-tab="open"]').click();
      await delayFrame();
      find('#taik-board button[data-ticket="proposal:ticket-903"]').click();
      await delayFrame();
      const ticket = find('#taik-ticket');
      requireFact(ticket.textContent.includes('Verlangt: ein Titel.') && ticket.textContent.includes('Bitte in Blau.'), 'A proposal lacks its description or what was said about it');
      requireFact(rendered(ticket.querySelector('button[data-proposal="ticket-903"]')), 'A finished proposal cannot be approved');
      const gateText = [...ticket.querySelectorAll('details')].find(details => /^Gate \(2 Zeilen\)/.test(details.querySelector('summary')?.textContent.trim() ?? ''));
      requireFact(gateText && !gateText.open, 'A proposal does not offer its gate, closed at first');
      gateText.open = true;
      await delayFrame();
      requireFact(gateText.querySelector('pre')?.textContent.includes('test -f hello.txt') && gateText.scrollWidth <= gateText.clientWidth + 1, 'The gate of a proposal is not shown in full');
      const plans = find('#taik-orchestrator .plans');
      requireFact(rendered(plans) && plans.textContent.includes('Sitzung auf taik · Gate @savant · Arbeit @standard') && plans.textContent.includes('Der Orchestrator plant.'), 'The orchestrator does not show its plans');
      requireFact(ticket.textContent.includes('Es müssen bestehende Dateien geändert werden.'), 'The proposal does not show the plan of the orchestrator');
      requireFact(plans.scrollWidth <= plans.clientWidth + 1, 'A plan leaves the orchestrator card');
      const clipped = [...plans.querySelectorAll('.plan-title, .why, .plan-reason')].filter(el => el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1);
      requireFact(!clipped.length, 'A plan is cut off: ' + clipped.map(el => el.textContent.slice(0, 30)).join(' | '));
      find('.new-ticket textarea').value = '';
      find('.new-ticket textarea').dispatchEvent(new Event('input', { bubbles: true }));
      window.updateFactory(snapshot);
      await delayFrame();
      await chooseTicket('job-floorplanner');
    }
    // The stock: old jobs are on the board, can be searched and opened.
    {
      const stocked = structuredClone(snapshot);
      stocked.index = [...stocked.jobs.map(job => ({ id: job.id, title: job.goal ?? '', decision: 'accepted', rounds: job.rounds.length })),
        { id: 'job-uralt', title: 'Ein Auftrag von früher', decision: 'accepted', rounds: 2 },
        { id: 'job-verworfen', title: 'Ein verworfener Versuch', decision: 'rejected', rounds: 4 }];
      window.updateFactory(stocked);
      await delayFrame();
      find('button[data-board-tab="decision"]').click();
      await delayFrame();
      requireFact(!!find('#taik-board button[data-ticket="job-uralt"]') && !!find('#taik-board button[data-ticket="job-verworfen"]'), 'The stock is not on the board');
      const search = find('#taik-board-search');
      search.focus();
      search.value = 'früher';
      search.dispatchEvent(new Event('input', { bubbles: true }));
      await delayFrame();
      requireFact(document.activeElement?.id === 'taik-board-search' && find('#taik-board-search').value === 'früher', 'Searching loses the field');
      const shown = [...document.querySelectorAll('#taik-board-decision button[data-ticket]')].map(button => button.dataset.ticket);
      requireFact(shown.length === 1 && shown[0] === 'job-uralt', 'The search does not narrow the tickets: ' + shown.join(','));
      find('#taik-board button[data-ticket="job-uralt"]').click();
      await delayFrame();
      requireFact(find('#taik-ticket')?.dataset.job === 'job-uralt' && find('#taik-ticket').textContent.includes('Ein Auftrag von früher'), 'A job of the stock does not open');
      // The app delivers the job in full.
      const old = structuredClone(stocked.jobs.find(job => job.id === 'job-floorplanner'));
      old.id = 'job-uralt';
      old.goal = 'Ein Auftrag von früher';
      stocked.jobs.push(old);
      window.updateFactory(stocked);
      await delayFrame();
      requireFact(find('#taik-ticket')?.dataset.job === 'job-uralt' && find('#taik-ticket .rounds .round'), 'A loaded job of the stock does not show its history');
      const field = find('#taik-board-search');
      field.value = '';
      field.dispatchEvent(new Event('input', { bubbles: true }));
      document.activeElement?.blur();
      window.updateFactory(snapshot);
      await delayFrame();
      await chooseTicket('job-floorplanner');
    }
    // Top down: the ticket says where the job stands, each round says what it changed, the lines are behind it.
    {
      const told = structuredClone(snapshot);
      const job = told.jobs.find(entry => entry.id === 'job-floorplanner');
      const last = job.rounds.at(-1);
      last.fields.decision = 'rejected';
      last.fields.checks = 'FAIL: der Grundriss hat keine Tür';
      job.analyses = { [last.name.replace(/\.run$/, '')]: 'Verlangt: Der Grundriss braucht eine Tür.\nUrsache: Der Kandidat hat sie ausgelassen.' };
      last.fields.summary = 'Ich habe die Tür ergänzt und die README angepasst.';
      job.dialogs = { [last.name.replace(/\.run$/, '')]: [{ who: 'agent', text: 'Ich lese zuerst den Grundriss.' }, { tool: 'Edit', target: 'plan/grundriss.html' }] };
      job.changes = { [last.name.replace(/\.run$/, '')]: { file: 'round-99.rejected.txt', added: 2, removed: 1, fresh: false, rejected: true,
        lines: ['  <main>', '- <p>alt</p>', '+ <p>neu</p>', '+ <p>mehr</p>', '···', '  </main>'], more: 0 } };
      window.updateFactory(told);
      await delayFrame();
      const state = find('#taik-ticket .ticket-state');
      requireFact(state.querySelectorAll('.analysis').length === 2 && state.textContent.includes('Der Kandidat hat sie ausgelassen.'), 'The ticket does not explain the rejection');
      requireFact(rendered(state) && state.textContent.includes('FAIL: der Grundriss hat keine Tür') && state.textContent.includes('abgelehnt'), 'The ticket does not say where the job stands');
      requireFact(state.getBoundingClientRect().top < find('#taik-ticket .rounds').getBoundingClientRect().top, 'The state is not above the history');
      const change = find('#taik-ticket details.round-change');
      requireFact(rendered(change) && change.querySelector('summary').textContent.includes('round-99.rejected.txt · +2 −1 Zeilen'), 'A round does not say what it changed');
      change.open = true;
      await delayFrame();
      requireFact(change.querySelectorAll('.diff-add').length === 2 && change.querySelectorAll('.diff-del').length === 1, 'The changed lines are missing');
      requireFact(change.scrollWidth <= change.clientWidth + 1, 'The changed lines leave the ticket');
      const dialog = find('#taik-ticket details.round-dialog');
      requireFact(rendered(dialog) && find('#taik-ticket .round-summary')?.textContent.includes('Tür ergänzt'), 'A session round lacks its summary or dialog');
      dialog.open = true;
      await delayFrame();
      requireFact(dialog.textContent.includes('Ich lese zuerst den Grundriss.') && dialog.textContent.includes('plan/grundriss.html'), 'The dialog does not show what the agent said and did');
      requireFact(dialog.scrollWidth <= dialog.clientWidth + 1, 'The dialog leaves the ticket');
      window.updateFactory(snapshot);
      await delayFrame();
    }
    // Closing: a waiting ticket offers it, and a closed job is decided and no longer waits.
    {
      const waiting = structuredClone(snapshot);
      const job = waiting.jobs.find(entry => entry.id === 'job-floorplanner');
      job.rounds.at(-1).fields.decision = 'rejected';
      job.rounds.at(-1).fields.checks = 'FAIL: keine Tür';
      window.updateFactory(waiting);
      await delayFrame();
      await chooseTicket('job-floorplanner');
      const close = find('#taik-ticket button[data-ticket-end]');
      requireFact(rendered(close), 'A waiting ticket cannot be closed');
      const before = Number(find('#taik-board-waiting').textContent);
      job.closed = 'Wird nicht mehr gebraucht.';
      window.updateFactory(waiting);
      await delayFrame();
      requireFact(Number(find('#taik-board-waiting').textContent) === before - 1, 'A closed job still waits for the user');
      requireFact(!!find('#taik-board-decision button[data-ticket="job-floorplanner"]'), 'A closed job is not among the decided tickets');
      requireFact(find('#taik-ticket').textContent.includes('Wird nicht mehr gebraucht.') && !find('#taik-ticket textarea'), 'A closed ticket does not show its reason or still takes replies');
      window.updateFactory(snapshot);
      await delayFrame();
      await chooseTicket('job-floorplanner');
    }
    // A job tells who planned, judged and worked, and where its result is.
    {
      const planned = structuredClone(snapshot);
      const job = planned.jobs.find(entry => entry.id === 'job-floorplanner');
      job.plan = { kind: 'sitzung', repo: 'taik', gate: 'pruefer', work: 'standard', reason: 'Bestehende Dateien werden geändert.' };
      job.adopted = 'taik/floorplanner';
      window.updateFactory(planned);
      await delayFrame();
      await chooseTicket('job-floorplanner');
      const actors = Object.fromEntries([...document.querySelectorAll('#taik-flow .flow-node')].map(node =>
        [node.querySelector('.flow-name')?.textContent.trim(), node.textContent]));
      requireFact(actors.Gate?.includes('@pruefer') && actors.Arbeit?.includes('Standard') && !actors.Arbeit.includes('@standard'), 'The flow does not name who wrote the gate and who worked: ' + JSON.stringify(actors));
      const ticket = find('#taik-ticket');
      requireFact(ticket.textContent.includes('Sitzung auf taik · Gate @pruefer · Arbeit @standard') && ticket.textContent.includes('Bestehende Dateien werden geändert.'), 'The ticket of a job lacks its plan');
      const branch = [...ticket.querySelectorAll('code')].find(code => code.textContent.trim() === 'taik/floorplanner');
      requireFact(branch && branch.scrollWidth <= branch.clientWidth + 1, 'The ticket does not show the branch of its result');
      window.updateFactory(snapshot);
      await delayFrame();
      await chooseTicket('job-floorplanner');
    }
    // Every button sends the message the app expects. The app's side is tested by tools/check-app-handlers.sh.
    {
      const sent = [];
      // WebKit protects window.webkit itself. A handler of the page's own name can be put on its list.
      const names = ['newTicket', 'approveProposal', 'declineTicket', 'commentTicket', 'closeTicket', 'landJob', 'setJobAgent', 'assignWorker'];
      const handlers = window.webkit?.messageHandlers;
      requireFact(!!handlers, 'This WebKit view has no message handlers to record on');
      const kept = {};
      for (const name of names) {
        kept[name] = Object.getOwnPropertyDescriptor(handlers, name);
        Object.defineProperty(handlers, name, { value: { postMessage: body => sent.push([name, body]) }, configurable: true, writable: true });
      }
      requireFact(handlers.newTicket && typeof handlers.newTicket.postMessage === 'function', 'The messages of the page cannot be recorded in this view');
      const type = (field, text) => { field.value = text; field.dispatchEvent(new Event('input', { bubbles: true })); };
      const last = () => JSON.stringify(sent.at(-1));
      const acting = structuredClone(snapshot);
      const job = acting.jobs.find(entry => entry.id === 'job-floorplanner');
      job.rounds.at(-1).fields.decision = 'rejected';
      job.rounds.at(-1).fields.checks = 'FAIL: keine Tür';
      const done = structuredClone(job);
      done.id = 'job-fertig'; done.goal = 'Fertige Sitzung'; done.rounds.at(-1).fields.decision = 'accepted'; done.landable = true;
      acting.jobs.push(done);
      acting.proposals = [
        { name: 'ticket-801', intent: 'Ein Vorschlag', origin: 'requests', brief: 'Ein Vorschlag', messages: [], plan: { kind: 'dokument', repo: '', gate: 'pruefer', work: 'standard', reason: 'Ein Dokument.' } },
        { name: 'ticket-802', intent: 'Eine Frage', origin: '', brief: 'Eine Frage', messages: [{ author: 'agent', round: 0, text: 'Was genau?' }], state: 'asks' },
      ];
      window.updateFactory(acting);
      await delayFrame();

      type(find('.new-ticket textarea'), 'Ein neuer Wunsch');
      find('#taik-new-ticket').click();
      requireFact(last() === JSON.stringify(['newTicket', { text: 'Ein neuer Wunsch' }]), 'The orchestrator field sends ' + last());

      const open = async key => { find('button[data-board-tab="open"]').click(); await delayFrame(); find('#taik-board button[data-ticket="' + key + '"]').click(); await delayFrame(); };
      await open('proposal:ticket-801');
      const choice = find('#taik-ticket select[data-assign="ticket-801"]');
      requireFact(rendered(choice) && choice.value === 'standard', 'A planned proposal does not offer the choice of its worker');
      requireFact(![...choice.options].some(option => option.value === 'pruefer'), 'The author of the gate can be chosen for the work');
      const other = [...choice.options].map(option => option.value).find(value => value !== 'standard');
      if (other) {
        choice.value = other;
        choice.dispatchEvent(new Event('change', { bubbles: true }));
        requireFact(last() === JSON.stringify(['assignWorker', { name: 'ticket-801', agent: other }]), 'Choosing a worker sends ' + last());
      }
      find('#taik-ticket button[data-proposal="ticket-801"]').click();
      requireFact(last() === JSON.stringify(['approveProposal', 'ticket-801']), 'Freigeben sends ' + last());
      type(find('#taik-ticket textarea'), 'Lieber nicht');
      find('#taik-ticket button[data-ticket-decline]').click();
      requireFact(last() === JSON.stringify(['declineTicket', { name: 'ticket-801', text: 'Lieber nicht' }]), 'Ablehnen sends ' + last());

      await open('proposal:ticket-802');
      type(find('#taik-ticket textarea'), 'Die Farbe');
      find('#taik-ticket button[data-ticket-send]').click();
      requireFact(last() === JSON.stringify(['commentTicket', { name: 'ticket-802', text: 'Die Farbe' }]), 'An answer to the orchestrator sends ' + last());

      await open('job-floorplanner');
      type(find('#taik-ticket textarea'), 'Bitte mit Tür');
      find('#taik-ticket button[data-ticket-send]').click();
      requireFact(last() === JSON.stringify(['commentTicket', { name: 'job-floorplanner', text: 'Bitte mit Tür' }]), 'A reply to a job sends ' + last());
      window.updateFactory(acting);
      await delayFrame();
      type(find('#taik-ticket textarea'), 'Wird nicht gebraucht');
      find('#taik-ticket button[data-ticket-end]').click();
      requireFact(last() === JSON.stringify(['closeTicket', { job: 'job-floorplanner', text: 'Wird nicht gebraucht' }]), 'Schließen sends ' + last());

      await chooseTicket('job-fertig');
      find('#taik-ticket button[data-land]').click();
      requireFact(last() === JSON.stringify(['landJob', 'job-fertig']), 'Übernehmen sends ' + last());

      for (const name of names) { if (kept[name]) Object.defineProperty(handlers, name, kept[name]); else delete handlers[name]; }
      window.updateFactory(snapshot);
      await delayFrame();
      await chooseTicket('job-floorplanner');
    }
    // Crew: a new agent is typed while the snapshot refreshes. The draft and the focus survive.
    const crew = find('#taik-crew');
    if (crew) {
      requireFact(rendered(crew), 'The crew is not rendered');
      requireFact(crew.getBoundingClientRect().right <= find('#taik-activity').getBoundingClientRect().right + 1, 'The crew leaves its column');
      find('button[data-agent-new]').click();
      await delayFrame();
      const field = find('#taik-agent-name');
      requireFact(rendered(field) && rendered(find('#taik-agent-harness')) && rendered(find('#taik-agent-persona')), 'The agent form did not open');
      field.focus();
      field.value = 'gate-agent';
      field.dispatchEvent(new Event('input', {bubbles: true}));
      const refreshed = structuredClone(snapshot);
      refreshed.spent = (refreshed.spent ?? 0) + 0.01;
      window.updateFactory(refreshed);
      await delayFrame();
      requireFact(find('#taik-agent-name')?.value === 'gate-agent', 'A snapshot update lost the agent draft');
      requireFact(document.activeElement?.id === 'taik-agent-name', 'A snapshot update took the focus from the agent form');
      find('button[data-agent-cancel]').click();
      await delayFrame();
      requireFact(!find('#taik-agent-name'), 'The agent form did not close');
      window.updateFactory(snapshot);
      await delayFrame();
    }
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
    requireFact(find('#taik-ticket')?.dataset.job === 'job-floorplanner', 'A live update dislodged the selected ticket');
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
