# Similarity search in the ticket store

Give the Factory a small SQLite ticket store and a similarity search over it, so a new ticket can find the earlier tickets that are close to it.

## Where the tickets come from

The view writes a comment to `STATE/inbox/<name>.md`. This job reads that folder. Each ticket is one Markdown file: the file name is the ticket name, the first line is the title, the rest is the text. A ticket also carries its flag and its time, both taken from the file.

## The store

- One SQLite database at `STATE/tickets.sqlite`.
- Access through the `sqlite3` command line tool at `/usr/bin/sqlite3` (version 3.43 here, with FTS5 and JSON built in). The Rust crate `rusqlite` is not in the offline cargo cache, so no new crate is added. The gate runs offline.
- The binary starts `sqlite3` with `-json` and reads the result as JSON with `serde_json`, which is already a dependency.
- Text goes into the database as a bound parameter, never as part of the SQL string. The binary passes the statements through `sqlite3`'s `readfile()` of a temporary SQL file or an equivalent quoting that a test with quotes and a newline proves.
- The schema is created when the file is missing: a `tickets` table with the name, title, text, flag and time, and an FTS5 index over the title and the text.
- The store is a convenience, not a source of truth: `STATE/inbox` holds the tickets and the store can be rebuilt from it at any time.

## The search

`taik-seed similar STATE QUERY [--limit N]` prints the closest tickets as one line each: rank, name, title and score. The limit defaults to 5.

- The query is matched against the FTS5 index first. The words are joined with OR, and each hit gets a BM25 rank from FTS5.
- Ties and near-ties are broken by token overlap, so a ticket that shares rare words with the query comes first.
- A query with no FTS5 hit falls back to the overlap, over all tickets.
- The output is stable: the same store and the same query print the same order.
- An empty query, a missing store or a limit of zero is a clear error on stderr with exit code 2. A query with no match prints nothing and exits 0.

## Ingest

The store is filled from `STATE/inbox` at the start of each search, and by `taik-seed similar STATE --ingest`. A ticket that is already stored is skipped when its text is unchanged and replaced when it changed. A file that disappeared from the inbox stays in the store.

## Tests

- A temporary store with four tickets, searched for a word that two of them share: both come back, the closer one first.
- A query that matches nothing prints nothing and exits 0.
- A missing store is created by `--ingest`, and the second ingest changes no row.
- The JSON from `sqlite3` with a text that holds quotes, a newline and a German umlaut is stored and found again.
- A store that is deleted is rebuilt from the inbox, and the search prints the same order as before.
- `cargo test --offline --locked` and `cargo clippy --offline --locked --all-targets -- -D warnings` stay clean.
