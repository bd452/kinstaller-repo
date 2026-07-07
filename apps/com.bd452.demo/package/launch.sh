#!/bin/sh

cd "$(dirname "$0")" || exit 1

lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null
./app.sh
EXIT_CODE=$?
lipc-set-prop com.lab126.pillow disableEnablePillow enable 2>/dev/null

exit "$EXIT_CODE"
