#!/bin/sh

cd "$(dirname "$0")" || exit 1

lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null
./app.sh
EXIT_CODE=$?
lipc-set-prop com.lab126.pillow disableEnablePillow enable 2>/dev/null
# The framework does not notice that we drew on its framebuffer; poke appmgrd
# so the home booklet repaints instead of leaving a blank (or stale) screen.
lipc-set-prop com.lab126.appmgrd start app://com.lab126.booklet.home 2>/dev/null

exit "$EXIT_CODE"
