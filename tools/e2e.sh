#!/usr/bin/env bash
# In-game check for QuarryPlus slice 1: the Quarry mines a bounded 3 x 3
# shaft with FE and collects the drops into its nine-slot inventory.
#
# The Overworld arena is built at y=96 (floor) with the test cubes at y=97..99
# and the quarries at y=100, away from the terrain. Every mod command is typed
# with `ctl chat "/cmd"` because `ctl command` runs the console path and
# bypasses the native `perform_command` hook.
#
# Default run:
#  A  (0,100,0),  1,000 FE, 3 x 3 x 3 stone cube -> the 27 blocks become air
#     and the inventory holds 27 cobblestone;
#  B  (8,100,0),  0 FE, same cube                  -> the cube stays, inventory
#     empty (no-energy control);
#  C  (16,100,0), 1,000 FE, no blocks              -> mined 0 of 27, FE intact
#     (no-target control);
#  D  (24,100,0), 1,000 FE, one stone inside and one outside the area -> the
#     inside one is mined and the outside one stays (out-of-area control).
#
# Default run (EMBEDDED=0): the game loads `content/` as a dev mod.
# EMBEDDED=1: the game is launched with **only the native bundle** (no
# `--dev-mod`) and the content comes from the bundle's embedded copy
# (`mixins/quarryplus/data/…`, the AE2 pattern). The run also checks the file the
# helper wrote. CONTROL=1 (default with EMBEDDED) first launches the same binary
# with `.horizon-cache` unusable, where the ids must be unknown.
#
# SELFTEST=1 flips the A assertion (it must FAIL), proving the scenario can
# fail.
set -euo pipefail
export LC_ALL=C

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/quarryplus
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/quarryplus-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=QuarryPlusE2E
CONTROL_WORLD=QuarryPlusControl
# EMBEDDED=1 launches with no --dev-mod: the delivered .hnmod carries its content.
EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
DEV_MOD=()
[ "$EMBEDDED" = 1 ] || DEV_MOD=(--dev-mod "$PORT/content")

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/quarryplus-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

# The `/quarryplus info` line for a quarry.
info() { # info X Y Z
    say "quarryplus info $1 $2 $3"
    ctl wait 3 >/dev/null
    ctl messages 10 | jq -r '.chat[]' | grep -m1 'Quarry:' || true
}
expect_info() { # expect_info X Y Z TEXT
    local line
    line=$(info "$1" "$2" "$3")
    echo "  info($1,$2,$3): $line"
    case "$line" in
        *"$4"*) ;;
        *) fail "expected /quarryplus info to say '$4', got '$line'" ;;
    esac
}

# The block name at a position.
block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# The total count of one item in the quarry's nine slots.
container_count() { # container_count X Y Z ITEM
    ctl container "$1" "$2" "$3" \
        | jq -r --arg item "$4" '[.slots[] | select(. != null and .item == $item) | .count] | add // 0'
}

cleanup() {
    rc=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$rc"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    fi
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --headless --wait \
        --resources "$RES" "${DEV_MOD[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

overworld() { ctl command "execute in minecraft:overworld run $*"; }

arena() {
    ctl command "execute in minecraft:overworld run tp @s 0.5 101 3.5"
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5
        if [ "$(ctl block 0 96 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the Overworld arena chunk never loaded"
    overworld "fill -8 96 -8 32 96 8 minecraft:stone"
    overworld "fill -8 97 -8 32 110 8 minecraft:air"
    overworld "fill -1 97 -1 1 99 1 minecraft:stone"     # A's cube
    overworld "fill 7 97 -1 9 99 1 minecraft:stone"      # B's cube
    overworld "setblock 25 97 0 minecraft:stone"         # D inside the area
    overworld "setblock 27 97 0 minecraft:stone"         # D outside the area
    overworld "kill @e[type=!player]"
    overworld "tp @s 0.5 97 3.5"
    ctl wait 20
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. Both ids must then be unknown; if they resolve, the run proves
# nothing about the embedded file.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch "$CONTROL_WORLD"
    arena
    ctl command "item replace entity @s hotbar.0 with quarryplus:quarry 1" >/dev/null || true
    ctl wait 4
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "quarryplus:quarry")] | length == 0' >/dev/null \
        || fail "the control game has the quarry item, so the content did not come from the embedded file"
    ctl command "execute in minecraft:overworld run setblock 40 100 0 quarryplus:quarry" >/dev/null || true
    ctl wait 4
    local name
    name=$(block_name 40 100 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no quarry item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$SCRATCH/.horizon-cache"
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/quarryplus-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/quarryplus.hmod" \
        || fail "the embedded content differs from content/quarryplus.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/quarryplus.hmod"
}

echo "== launch (Overworld arena, content + native bundle) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
arena

echo "== place the quarries and set each area to 3 x 3 x 3 =="
for x in 0 8 16 24; do
    overworld "setblock $x 100 0 quarryplus:quarry"
done
ctl wait 4
for x in 0 8 16 24; do
    say "quarryplus area $x 100 0 1 3"
done
ctl wait 4
expect_info 0 100 0 "mined 0 of 27"
expect_info 8 100 0 "mined 0 of 27"
expect_info 16 100 0 "mined 0 of 27"
expect_info 24 100 0 "mined 0 of 27"

echo "== A: 1,000 FE mines the stone cube and collects the drops =="
say "quarryplus energy 1000 0 100 0"
ctl wait 4
# 27 stone x 37 FE = 999 FE; 27 blocks x 4 ticks = 108 ticks.
ctl wait 160
a_air=$(block_name 0 99 0)
a_low=$(block_name 0 97 0)
[ "$a_air" = "minecraft:air" ] || fail "the top of A's cube is not air (got '$a_air')"
[ "$a_low" = "minecraft:air" ] || fail "the bottom of A's cube is not air (got '$a_low')"
echo "  cube 0,99,0 -> $a_air; 0,97,0 -> $a_low"
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting A's inventory is empty)"
    expect_info 0 100 0 "inventory empty"
else
    expect_info 0 100 0 "mined 27 of 27"
    a_count=$(container_count 0 100 0 minecraft:cobblestone)
    [ "$a_count" = 27 ] || fail "A's inventory holds $a_count cobblestone, expected 27"
    echo "  inventory: minecraft:cobblestone x$a_count"
fi

echo "== B: no FE leaves the cube untouched (control) =="
expect_info 8 100 0 "mined 0 of 27"
expect_info 8 100 0 "0/10000 FE"
b_block=$(block_name 8 99 0)
[ "$b_block" = "minecraft:stone" ] || fail "the no-energy cube changed (got '$b_block')"
b_count=$(container_count 8 100 0 minecraft:cobblestone)
[ "$b_count" = 0 ] || fail "the no-energy quarry collected $b_count items"
echo "  cube 8,99,0 still $b_block; inventory 0"

echo "== C: no target blocks mines nothing and keeps the FE (control) =="
say "quarryplus energy 1000 16 100 0"
ctl wait 4
ctl wait 40
expect_info 16 100 0 "mined 0 of 27"
expect_info 16 100 0 "1000/10000 FE"
c_count=$(container_count 16 100 0 minecraft:cobblestone)
[ "$c_count" = 0 ] || fail "the empty quarry collected $c_count items"
c_block=$(block_name 16 97 0)
[ "$c_block" = "minecraft:air" ] || fail "the empty area is not air (got '$c_block')"

echo "== D: a block outside the area stays (control) =="
say "quarryplus energy 1000 24 100 0"
ctl wait 4
ctl wait 40
d_in=$(block_name 25 97 0)
d_out=$(block_name 27 97 0)
[ "$d_in" = "minecraft:air" ] || fail "the inside block was not mined (got '$d_in')"
[ "$d_out" = "minecraft:stone" ] || fail "the outside block was mined (got '$d_out')"
expect_info 24 100 0 "mined 1 of 27"
d_count=$(container_count 24 100 0 minecraft:cobblestone)
[ "$d_count" = 1 ] || fail "D collected $d_count items, expected 1"
echo "  inside 25,97,0 -> $d_in; outside 27,97,0 -> $d_out; inventory 1"

echo "PASS: 1,000 FE mined the 3x3x3 stone cube into 27 cobblestone in the quarry's inventory, no FE mined nothing, an empty area mined nothing and kept its FE, and a block outside the area stayed"
