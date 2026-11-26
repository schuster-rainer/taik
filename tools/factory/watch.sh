#!/bin/sh
# tools/factory/watch.sh NAME [MINUTES]: waits until job-NAME is accepted, asks, or stops; prints its rounds.
J="$HOME/.local/state/taik-factory/jobs/job-$1"; limit=$(( ${2:-9} * 20 )); i=0; seen=""
while [ $i -lt $limit ]; do
  last=$(ls "$J"/round-[0-9][0-9].run 2>/dev/null | tail -1)
  [ -n "$last" ] && grep -q '^decision=accepted' "$last" && break
  msg=$(ls "$J"/message-*.md 2>/dev/null | tail -1)
  case "$msg" in *agent.md) [ "$msg" != "$seen" ] && break;; esac
  /bin/sleep 3; i=$((i+1))
done
for r in "$J"/round-[0-9][0-9].run; do [ -e "$r" ] || continue; echo "== $(basename $r)"; grep -E '^(decision|checks|files|worker_failure)=' "$r" | cut -c1-240; done
msg=$(ls "$J"/message-*.md 2>/dev/null | tail -1); case "$msg" in *agent.md) echo "ASKS: $(tail -n +3 "$msg")";; esac
