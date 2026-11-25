#!/bin/sh
# Claude Code hook. Writes what this agent session is doing into the Factory state, one small file per session.
# No model call, no network. It must never block or fail the agent.
state="${TAIK_STATE:-$HOME/.local/state/taik-factory}"
[ -d "$state" ] || exit 0
mkdir -p "$state/agents" 2>/dev/null || exit 0
next=$(mktemp "$state/agents/.next.XXXXXX" 2>/dev/null) || exit 0
if /usr/bin/env jq -c --arg at "$(date +%s)" '{
      at: ($at | tonumber),
      session: (.session_id // "unknown"),
      event: (.hook_event_name // ""),
      tool: (.tool_name // ""),
      target: ((.tool_input.file_path // .tool_input.description // .tool_input.pattern // .tool_input.skill // "") | tostring | .[0:160]),
      cwd: (.cwd // "")
    }' > "$next" 2>/dev/null; then
  session=$(jq -r '.session' "$next" 2>/dev/null | tr -cd 'A-Za-z0-9-' | cut -c1-64)
  mv "$next" "$state/agents/${session:-unknown}.json" 2>/dev/null
else
  rm -f "$next"
fi
exit 0
