# Board view: every waiting thing as one ticket

Add a ticket board to the left column of the Factory view. The board is a third section next to the work history and the proposals. It shows what waits for the user, what runs and what was decided, and it is the place where the user answers a ticket.

## Why

Today the left column lists jobs and proposals separately, and only the proposals carry a button. The user has to read the history to find out what waits for them. A board puts every waiting thing in one list, ordered by what needs an answer, and keeps the details in the right pane.

## What the board shows

- A lead line with the number of tickets that wait for the user, and a button that jumps to the first one.
- Three columns: `Wartet auf Antwort`, `Läuft`, `Entschieden`. Each column scrolls on its own. The page does not scroll.
- A ticket has an id, a title, one or two flags and a meta line.
- Flags: `Antwort` and `Frage` wait for the user. `Entscheidung` waits for an approval. `In Arbeit`, `Warteschlange`, `Angenommen` and `Abgelehnt` are state only and stay quiet.

## Where the tickets come from

Every ticket is built from the snapshot that the view already receives. No new data source and no new command.

| Column | Source | Flag |
|---|---|---|
| Wartet auf Antwort | a proposal without `approved` | `Entscheidung` |
| Wartet auf Antwort | a software job with `sourceActive` or `sourceQueued` | `Antwort` |
| Wartet auf Antwort | the latest round of a pending job whose decision is `rejected` | `Frage` |
| Läuft | a job with `sourceActive` | `In Arbeit` |
| Läuft | a pending job that waits for its next round | `Warteschlange` |
| Entschieden | a job whose last round is accepted and not advancing | `Angenommen` |
| Entschieden | a job whose last round is rejected | `Abgelehnt` |

A job appears in exactly one column: the first row of the table that matches it, top to bottom.

## What a ticket does

- A click on a ticket selects its job in the history. The right pane then shows that job. For a proposal ticket, the click selects the job named in `origin`, when there is one.
- The `Freigeben` button on a proposal ticket sends `approveProposal`, as the proposals list does today.
- A comment field sits under the open ticket. It sends the text with `commentTicket`, a new message handler that carries `{name, text}`.
- The proposals list stays. The board does not replace it in this job.

## Shape

- The board is a section in `#taik-activity`, with the same raised surface, radius and shadow tokens as the other sections.
- The CSS is scoped to `#taik-live`, as all other rules are.
- The host gains the `commentTicket` handler beside `approveProposal`. It writes the text to `STATE/inbox/<name>.md` and creates `STATE/inbox` when it is missing. The Factory reads that folder in a later job.
- The board renders from the same `render()` pass as the rest of the view. A live update keeps the selected job and the open ticket.

## Scope

- One file: `tools/factory-view.html`.
- The host change is one handler in `tools/factory-view.swift`.
- No change to the flow, the workpiece, the history or the proposals list.
- Keep the fragment contract: one `<style>` at the start, exactly one `/*FACTORY_DATA*/null` placeholder and one `window.updateFactory` entry point.
