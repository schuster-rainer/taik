#!/bin/sh
# tools/factory/land.sh NAME [BRIEF.md] ["COMMIT SUBJECT"]: lands an accepted session job of this
# repository from a terminal. The work is done by .taik/land.sh, which the app runs for "Übernehmen".
exec "$(dirname "$0")/../../.taik/land.sh" "$@"
