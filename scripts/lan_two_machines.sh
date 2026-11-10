#!/bin/sh
# Physical two-machine LAN check for the headless lockstep client.
#
# Machine A (the host):
#   ./scripts/lan_two_machines.sh host
# Machine B (the joiner), with A's LAN address:
#   ./scripts/lan_two_machines.sh join 192.168.1.20
#
# Both machines must have the same StarCraft-derived fixtures/assets.  The
# script runs `sc_lan`, which exchanges only commands and prints the state
# hash after every turn; the two final hashes must match.
#
# Environment overrides:
#   SC_LAN_PORT   TCP/UDP port (default 7788)
#   SC_LAN_CHK    scenario CHK (default engine-rust/assets/maps/(2)Boxer.chk)
#   SC_LAN_VF4    terrain flags (default engine-rust/assets/TileSet/badlands.vf4)
#   SC_LAN_TICKS  turns to play (default 1200)
#   SC_LAN_BIN    built binary (default engine-rust/target/release/sc_lan)

set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN=${SC_LAN_BIN:-"$ROOT/engine-rust/target/release/sc_lan"}
CHK=${SC_LAN_CHK:-"$ROOT/engine-rust/assets/maps/(2)Boxer.chk"}
VF4=${SC_LAN_VF4:-"$ROOT/engine-rust/assets/TileSet/badlands.vf4"}
TICKS=${SC_LAN_TICKS:-1200}
PORT=${SC_LAN_PORT:-7788}

if [ ! -x "$BIN" ]; then
    echo "build it first: (cd engine-rust && cargo build --release --bin sc_lan)" >&2
    exit 2
fi
if [ ! -f "$CHK" ] || [ ! -f "$VF4" ]; then
    echo "missing map or tileset: $CHK / $VF4" >&2
    exit 2
fi

case "${1:-}" in
    host)
        echo "host: waiting on port $PORT (join from the other machine)"
        exec "$BIN" --host --chk "$CHK" --vf4 "$VF4" \
            --races terran,terran --ticks "$TICKS" --port "$PORT"
        ;;
    join)
        [ $# -ge 2 ] || { echo "usage: $0 join HOST [PORT]" >&2; exit 2; }
        HOST=$2
        [ $# -ge 3 ] && PORT=$3
        exec "$BIN" --join "$HOST:$PORT" --chk "$CHK" --vf4 "$VF4" \
            --race terran --ticks "$TICKS"
        ;;
    *)
        echo "usage: $0 host | join HOST [PORT]" >&2
        exit 2
        ;;
esac
