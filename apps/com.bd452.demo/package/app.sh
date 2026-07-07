#!/bin/sh

KPM_PKG_ROOT=/mnt/us/kmc/kpm/packages
FBINK_PKG=com.bd452.fbink

if [ -f /lib/ld-linux-armhf.so.3 ]; then
    PLAT=kindlehf
else
    PLAT=kindlepw2
fi

FBINK="${KPM_PKG_ROOT}/${FBINK_PKG}/bin/${PLAT}/fbink"
if [ ! -x "$FBINK" ]; then
    echo "fbink not found at ${FBINK}." >&2
    echo "Install the ${FBINK_PKG} package first." >&2
    exit 1
fi

"$FBINK" -q -c
"$FBINK" -q -mM "demo"
"$FBINK" -q -m -y 4 "touch anywhere to close"
