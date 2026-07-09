#!/usr/bin/env bash
# Copy the latest signalkit-demo binary to a jailbroken Kindle over SSH and
# launch it in the foreground app slot.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_BIN="$REPO_ROOT/apps/com.bd452.signalkitdemo/package/bin/kindlehf/signalkit-demo"
FALLBACK_BIN="$REPO_ROOT/rust/target-kindle/armv7-unknown-linux-gnueabihf/release/signalkit-demo"

BIN="${BIN:-$DEFAULT_BIN}"
KINDLE="${KINDLE:-${KINDLE_HOST:-}}"
KINDLE_USER="${KINDLE_USER:-root}"
KINDLE_PASSWORD="${KINDLE_PASSWORD:-kindle}"
REMOTE_BIN="${REMOTE_BIN:-/mnt/us/signalkit-demo}"

usage() {
    cat >&2 <<EOF
Usage:
  KINDLE=root@192.168.1.231 $0
  KINDLE_HOST=192.168.1.231 KINDLE_PASSWORD=kindle $0
  BIN=rust/target-kindle/armv7-unknown-linux-gnueabihf/release/signalkit-demo $0

Build first, for example:
  ./scripts/build-in-container.sh apps/com.bd452.signalkitdemo/build.sh
EOF
}

if [[ ! -x "$BIN" && -x "$FALLBACK_BIN" ]]; then
    BIN="$FALLBACK_BIN"
fi

if [[ ! -x "$BIN" ]]; then
    echo "error: demo binary not found or not executable: $BIN" >&2
    usage
    exit 1
fi

ssh_base=(
    ssh
    -o StrictHostKeyChecking=no
    -o UserKnownHostsFile=/dev/null
    -o ConnectTimeout=3
    -o NumberOfPasswordPrompts=1
)
scp_base=(
    scp
    -o StrictHostKeyChecking=no
    -o UserKnownHostsFile=/dev/null
    -o ConnectTimeout=3
    -o NumberOfPasswordPrompts=1
)

if command -v sshpass >/dev/null 2>&1 && [[ -n "$KINDLE_PASSWORD" ]]; then
    ssh_cmd=(sshpass -p "$KINDLE_PASSWORD" "${ssh_base[@]}")
    scp_cmd=(sshpass -p "$KINDLE_PASSWORD" "${scp_base[@]}")
else
    ssh_cmd=("${ssh_base[@]}")
    scp_cmd=("${scp_base[@]}")
fi

target_from_host() {
    local host=$1
    if [[ "$host" == *@* ]]; then
        printf '%s\n' "$host"
    else
        printf '%s@%s\n' "$KINDLE_USER" "$host"
    fi
}

can_ssh() {
    local target=$1
    "${ssh_cmd[@]}" "$target" 'true' >/dev/null 2>&1
}

discover_kindle() {
    local ip target
    while IFS= read -r ip; do
        [[ -z "$ip" ]] && continue
        target="$(target_from_host "$ip")"
        echo "==> Trying $target" >&2
        if can_ssh "$target"; then
            printf '%s\n' "$target"
            return 0
        fi
    done < <(arp -an 2>/dev/null | sed -n 's/.*(\([^)]*\)).*/\1/p' | sort -u)
    return 1
}

if [[ -n "$KINDLE" ]]; then
    TARGET="$(target_from_host "$KINDLE")"
else
    echo "==> KINDLE not set; trying SSH against current ARP table" >&2
    if ! TARGET="$(discover_kindle)"; then
        echo "error: could not find a Kindle via ARP. Set KINDLE=root@<ip>." >&2
        usage
        exit 1
    fi
fi

echo "==> Using $TARGET"
echo "==> Stopping old demo"
"${ssh_cmd[@]}" "$TARGET" 'killall signalkit-demo 2>/dev/null || true; sleep 1'

echo "==> Copying $BIN to $TARGET:$REMOTE_BIN"
"${scp_cmd[@]}" "$BIN" "$TARGET:$REMOTE_BIN"

echo "==> Launching demo"
"${ssh_cmd[@]}" "$TARGET" "
chmod +x '$REMOTE_BIN'
lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null || true
cd /mnt/us
nohup '$REMOTE_BIN' >/mnt/us/signalkit-demo.out 2>/mnt/us/signalkit-demo.err &
sleep 2
ps | grep signalkit-demo | grep -v grep || true
if [ -s /mnt/us/signalkit-demo.err ]; then
    echo '--- /mnt/us/signalkit-demo.err ---'
    cat /mnt/us/signalkit-demo.err
fi
"
