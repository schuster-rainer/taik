#!/bin/sh
# tools/factory/wish.sh FILE: sends a request to the orchestrator and waits for its proposal. Prints the ticket name and the plan.
state="$HOME/.local/state/taik-factory"
name=$("$state/taik-seed" request "$state" < "$1") || exit 1
i=0
while [ $i -lt 200 ]; do
  [ -d "$state/proposals/$name" ] && break
  [ -e "$state/requests/$name.failed" ] && break
  grep -q '^outcome=stop' "$state/requests/reflection-$name.run" 2>/dev/null && break
  /bin/sleep 3; i=$((i+1))
done
echo "ticket=$name"
grep -v repo_path "$state/proposals/$name/plan.txt" 2>/dev/null
cat "$state/requests/$name.failed" 2>/dev/null
grep '^outcome=' "$state/requests/reflection-$name.run" 2>/dev/null | cut -c1-300
ls "$state/proposals/$name" 2>/dev/null | tr '\n' ' '
