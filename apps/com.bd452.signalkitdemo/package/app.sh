#!/bin/sh

cd "$(dirname "$0")" || exit 1

if [ -f /lib/ld-linux-armhf.so.3 ]; then
    PLAT=kindlehf
else
    PLAT=kindlepw2
fi

BIN="./bin/${PLAT}/signalkit-demo"
if [ ! -x "$BIN" ]; then
    echo "signalkit-demo binary not found for ${PLAT} at ${BIN}." >&2
    exit 1
fi

# The binary links FBInk statically and reads the touchscreen itself.
exec "$BIN"
