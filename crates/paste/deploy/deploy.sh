#!/bin/sh
# Builds hspaste for a Raspberry Pi and installs it there: deploy.sh [user@host]
# The previous binary is kept and put back if the new one does not start.
#
# The engine, game data and icons are compiled into the binary, so the site keeps the
# numbers of the build it runs until this script installs a new one. Stored builds are
# share codes and need no migration. When the app starts writing a share-code format
# the running site does not know, run this before releasing the app.
set -eu

host=${1:-${HSPASTE_HOST:-zium@raspberrypi.local}}
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)
target=aarch64-unknown-linux-musl

cd "$repo"
cargo test -p hsplanner-paste
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C linker-flavor=ld.lld -C link-self-contained=yes" \
    cargo build --release -p hsplanner-paste --target "$target"

# The binary travels on the connection that installs it, so the password is asked once.
ssh "$host" '
set -eu
cat > /tmp/hspaste
healthy() {
    for attempt in 1 2 3 4 5 6 7 8 9 10; do
        curl -fsS -m 2 http://127.0.0.1:8787/healthz >/dev/null 2>&1 && return 0
        sleep 1
    done
    return 1
}
if [ -f /usr/local/bin/hspaste ]; then
    sudo cp -p /usr/local/bin/hspaste /usr/local/bin/hspaste.previous
fi
sudo install -m 755 /tmp/hspaste /usr/local/bin/hspaste
rm /tmp/hspaste
sudo systemctl restart hspaste
if healthy; then
    echo "hspaste is up with the new build"
    exit 0
fi
echo "the new build did not start:" >&2
sudo journalctl -u hspaste -n 20 --no-pager >&2
if [ -f /usr/local/bin/hspaste.previous ]; then
    sudo install -m 755 /usr/local/bin/hspaste.previous /usr/local/bin/hspaste
    sudo systemctl restart hspaste
    healthy && echo "the previous build is running again" >&2
fi
exit 1
' < "target/$target/release/hspaste"
