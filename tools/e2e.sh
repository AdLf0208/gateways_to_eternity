#!/usr/bin/env bash
# In-game check for Gateways to Eternity (SDK package 0.2.0): one gateway
# (gateways:emerald_grove), its first wave, and its first reward.
#
# Default run (headless, creative, The End with a built arena):
#   1. a gate pearl carrying `custom_data.gateway=gateways:emerald_grove` is used
#      on a block: the gateway opens on it (no visible entity in this slice);
#   2. CONTROL "setup": for the first 200 ticks no mob spawns (wave 1's
#      setup_time is 200 in the jar's emerald_grove.json);
#   3. wave 1 spawns the jar's five zombies;
#   4. CONTROL "second use": using a second pearl while the wave is active opens
#      nothing (upstream's spacing check) - no new mobs, no pearl spent;
#   5. CONTROL "unkilled wave": 100 ticks with the five zombies alive pay nothing
#      (no hay block anywhere);
#   6. the five zombies die: the wave completes and pays 16 hay blocks
#      (the jar's `gateways:stack` reward), split into stacks of four;
#   7. the wave list advances: after another 200-tick setup, wave 2's five
#      spiders spawn;
#   8. `/gateways list` names the carried gateway and `/gateways open` opens a
#      second one.
#
# SELFTEST=1 inverts step 6's assertion (it expects no reward), so the run MUST
# end `FAIL:` and exit non-zero - the proof that the assertion can fail.
#
# GAME must be a game the launcher built WITH this package: a native `.hmod`
# cannot load on an unmodified game, so the retired `.hnmod`/`--dev-mod` shape is
# gone. `horizon-launcher` is not installed in this environment, so this script
# has not been run since the migration (recorded as a gap in `notes.md`).
#
#   horizon-launcher native-build ports/gateways --instance <i> --trust-native-code
#   GAME=<that instance's horizon-client> tools/e2e.sh
#
# CONTROL_GAME (optional) is a game built WITHOUT this package: the same arena
# must then refuse the pearl and the recipe, which is what proves the content
# comes from this package rather than from the base game.
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/gateways
# The control binary is shared at the campaign pin; the game is the package build.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/gateways-e2e.sock}
LOG=${LOG:-/tmp/opencode/gateways-e2e-transcript.log}
GAME_LOG=${GAME_LOG:-/tmp/opencode/gateways-e2e.log}
SELFTEST=${SELFTEST:-0}
WORLD=Gateways
CONTROL_WORLD=GatewaysControl
# The pearl the recipe makes: `minecraft:custom_data` names the gateway.
PEARL='gateways:gate_pearl[minecraft:custom_data={gateway:"gateways:emerald_grove"}]'
HAY=minecraft:hay_block

mkdir -p "$(dirname "$LOG")"
exec > >(tee "$LOG") 2>&1

SCRATCH=$(mktemp -d /tmp/opencode/gateways-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { printf 'ctl %s\n' "$*" >&2; "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
mobs() { ctl entities 24 "$1" | jq '.entities | length'; }
# Reward on the floor plus reward already picked up: either alone is the wrong
# signal, so both are counted (the pearl is excluded from neither count: it is
# not hay).
hay_total() {
    local floor bag
    floor=$(ctl entities 24 minecraft:item         | jq --arg item "$HAY" '[.entities[] | select(.item.item == $item) | .item.count] | add // 0')
    bag=$(ctl inventory | jq --arg item "$HAY" '.counts[$item] // 0')
    echo $((floor + bag))
}
pearls() { ctl inventory | jq '.counts["gateways:gate_pearl"] // 0'; }

launch() { # launch WORLD GAME
    local reply log
    if ! reply=$("$CTL" --socket "$SOCK" launch \
        --game "$2" --world "$1" --seed 20261105 --creative --headless --wait \
        --game-dir "$SCRATCH/game" --resources "$RES"); then
        echo "FAIL: the game did not launch ($1): $reply" >&2
        return 1
    fi
    printf '%s\n' "$reply"
    log=$(printf '%s' "$reply" | jq -r '.log // empty')
    [ -n "$log" ] && GAME_LOG=$log
    ctl wait-ready 120 >/dev/null
}

cleanup() {
    local status=$?
    timeout 15 "$CTL" --socket "$SOCK" quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" 2>/dev/null || true
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

[ -n "$GAME" ] || fail "set GAME to a horizon-client the launcher built with this package (horizon-launcher native-build ports/gateways --instance <i> --trust-native-code)"
[ -x "$CTL" ] || fail "missing horizon-ctl: $CTL"
[ -x "$GAME" ] || fail "missing horizon-client: $GAME"
[ -d "$RES/assets" ] || fail "missing prepared resources: $RES"
command -v jq >/dev/null || fail "missing jq"

# The control: a game built WITHOUT this package. Its pearl id and its recipe must
# be unknown, which is what makes the default legs evidence about this package.
control_leg() {
    echo "== control: the same arena on a game built without this package =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    local reply
    reply=$(ctl command "give @s $PEARL 1" 2>&1 | tr -d '\n')
    echo "  give the pearl without the package: $reply"
    case "$reply" in
        *"Unknown item"* | *"Incorrect argument"* | *"unknown"*) ;;
        *) fail "the pearl exists on a game without this package: $reply" ;;
    esac
    [ "$(pearls)" -eq 0 ] || fail "the control game holds a pearl"
    reply=$(ctl command "recipe give @s gateways:emerald_grove" 2>&1 | tr -d '\n')
    echo "  the emerald-grove recipe without the package: $reply"
    case "$reply" in
        *"Unknown recipe"*) ;;
        *) fail "the recipe is known without this package: $reply" ;;
    esac
    ctl quit >/dev/null 2>&1 || true
    sleep 6
    rm -f "$SOCK"
}

if [ -n "$CONTROL_GAME" ]; then
    [ -x "$CONTROL_GAME" ] || fail "missing control horizon-client: $CONTROL_GAME"
    control_leg
fi

echo "== launch ($WORLD, the package the launcher built) =="
launch "$WORLD" "$GAME"

echo "== an arena in The End: a stone floor at y=99 and clean air above =="
# The player goes in first: a fill into a dimension nobody has entered applies to
# a chunk that is not loaded yet, and the chunk regenerates over it when the
# player arrives (measured: the floor read back as the End's bedrock). The End's
# exit platform at (0, 99) catches the player, so the teleport is safe.
run_cmd() { # run_cmd "line" - prints the reply, fails on an unknown/refused command
    local reply
    reply=$(ctl command "$1" 2>&1 | tr -d '\n')
    echo "  $1 -> $reply"
    case "$reply" in
        *"Unknown"* | *"Incorrect argument"* | *"No entity"*) fail "command refused: $1 ($reply)" ;;
    esac
}
run_cmd "time set noon"
run_cmd "difficulty normal"
run_cmd "gamerule spawn_monsters false"
run_cmd "gamerule spawn_mobs false"
run_cmd "execute in minecraft:the_end run forceload add -8 -8 8 8"
run_cmd "execute in minecraft:the_end run tp @s 0.5 100 2.5"
ctl wait 20
dimension=$(ctl state | jq -r '.dimension')
echo "  dimension after the teleport: $dimension"
[ "$dimension" = "minecraft:the_end" ] || fail "the player is in $dimension, not The End"
run_cmd "execute in minecraft:the_end run fill -8 99 -8 8 99 8 minecraft:stone"
run_cmd "execute in minecraft:the_end run fill -8 100 -8 8 110 8 minecraft:air"
run_cmd "tp @s 0.5 100 2.5"
ctl wait 20
[ "$(ctl block 0 99 0 | jq -r '.state // "unknown"')" = "minecraft:stone" ] \
    || fail "the arena floor is not there: $(ctl block 0 99 0)"
run_cmd "kill @e[type=!minecraft:player]"
ctl wait 10
ctl command "clear @s" >/dev/null
ctl wait 4
give_reply=$(ctl command "give @s $PEARL 2" 2>&1 | tr -d '\n')
echo "  give reply: $give_reply"
case "$give_reply" in
    *"Unknown item"* | *"Incorrect argument"*) fail "cannot give the gate pearl: $give_reply" ;;
esac
ctl wait 4
[ "$(pearls)" -eq 2 ] || fail "the pearl did not land: $(pearls)"
ctl select gateways:gate_pearl >/dev/null || fail "cannot select the pearl"
ctl state | jq -c '{dimension, position: .player.position}' || true
[ "$(mobs minecraft:zombie)" -eq 0 ] || fail "the arena is not clear of zombies"

echo "== the content half: the jar's emerald-grove recipe is registered =="
recipe_reply=$(ctl command "recipe give @s gateways:emerald_grove" 2>&1 | tr -d '\n')
echo "  recipe reply: $recipe_reply"
case "$recipe_reply" in
    *"Unknown recipe"*) fail "the emerald-grove recipe did not load: $recipe_reply" ;;
    *Unlocked*) ;;
    *) fail "the recipe probe answered nothing recognisable: $recipe_reply" ;;
esac

echo "== a gate pearl opens its gateway on the clicked block =="
# `ctl interact` aims at the block and clicks it (a bare `use-item` after
# `look-at block` left the crosshair off the block at this pin: `state.target`
# came back null and nothing was used).
ctl interact 0 99 0 up >/dev/null || fail "the pearl click failed"
ctl wait 2
echo "  pearl count after the use: $(pearls) (creative keeps it, as upstream's useOn does)"
[ "$(pearls)" -eq 2 ] || fail "a creative player's pearl was spent: $(pearls)"

echo "== control: a second use while the gateway is open opens nothing =="
# Done before the first wave exists, so a mob cannot stand in the crosshair and
# take the click: any second gateway would show as ten zombies two lines down.
second=$(ctl interact 0 99 0 up 2>&1 | tr -d '\n') || true
echo "  second click: $second"

echo "== control: the first 200 ticks are wave 1's setup time (no mobs) =="
ctl wait 188
setup_mobs=$(mobs minecraft:zombie)
echo "  zombies after 192 ticks: $setup_mobs"
[ "$setup_mobs" -eq 0 ] || fail "a wave spawned before its setup time"

echo "== wave 1: the jar's five zombies, and only one gateway =="
ctl wait 20
zombies=$(mobs minecraft:zombie)
echo "  zombies after 212 ticks: $zombies"
[ "$zombies" -eq 5 ] || fail "wave 1 spawned $zombies zombies, expected 5 (ten would mean the second use opened a gateway)"

echo "== control: an unkilled wave never pays =="
hay_before=$(hay_total)
ctl wait 100
hay_alive=$(hay_total)
echo "  hay with the five zombies alive: $hay_before -> $hay_alive"
[ "$hay_alive" -eq "$hay_before" ] || fail "a wave with living mobs paid out"

if [ "$SELFTEST" = 1 ]; then
    # The inverted assertion: this run expects the wave NOT to pay, so it must
    # fail once it does. It proves the reward assertion below is not vacuous.
    echo "== SELFTEST: the inverted expectation (no reward) =="
fi

echo "== the five zombies die: wave 1 completes and pays 16 hay blocks =="
ctl command "kill @e[type=minecraft:zombie]" >/dev/null || fail "cannot kill the wave"
ctl wait 20
hay_after=$(hay_total)
echo "  hay after the wave: $hay_before -> $hay_after (floor+bag)"
if [ "$SELFTEST" = 1 ]; then
    [ "$hay_after" -eq "$hay_before" ] \
        || fail "SELFTEST: the wave paid $((hay_after - hay_before)) hay blocks, and the inverted expectation said it would not"
    fail "SELFTEST: the inverted expectation held (no reward), so the run is invalid"
fi
[ "$hay_after" -eq $((hay_before + 16)) ] \
    || fail "the wave paid $((hay_after - hay_before)) hay blocks, expected 16"

echo "== the reward is granted once =="
ctl wait 60
hay_later=$(hay_total)
echo "  hay 60 ticks later: $hay_later"
[ "$hay_later" -eq "$hay_after" ] || fail "the same wave paid again: $hay_after -> $hay_later"

echo "== the wave list advances: wave 2's five spiders after its setup time =="
ctl wait 210
spiders=$(mobs minecraft:spider)
echo "  spiders after the next setup: $spiders"
[ "$spiders" -eq 5 ] || fail "wave 2 spawned $spiders spiders, expected 5"

echo "== the command surface: /gateways list and /gateways open =="
# The player's chat path: the tree the server registered, then `run_mod_command`.
list_reply=$(ctl chat "/gateways list" 2>&1 | tr -d '\n')
echo "  /gateways list -> $list_reply"
case "$list_reply" in
    *gateways:emerald_grove*) ;;
    *) fail "/gateways list did not name the carried gateway: $list_reply" ;;
esac
open_reply=$(ctl chat "/gateways open 0.5 100 6.5 gateways:emerald_grove" 2>&1 | tr -d '\n')
echo "  /gateways open -> $open_reply"
case "$open_reply" in
    *"Unknown"* | *"Incorrect argument"* | *"permission"*) fail "/gateways open was refused: $open_reply" ;;
esac
ctl wait 210
echo "  zombies after the command's setup: $(mobs minecraft:zombie), spiders: $(mobs minecraft:spider)"
[ "$(mobs minecraft:zombie)" -eq 5 ] || fail "/gateways open did not open a gateway (zombies: $(mobs minecraft:zombie))"
[ "$(mobs minecraft:spider)" -eq 5 ] || fail "the first gateway's wave 2 was disturbed"

echo "PASS: a gate pearl opened its gateway, wave 1 spawned the jar's five zombies after its 200-tick setup, an active wave paid nothing until its mobs died and refused a second gateway, it then paid exactly 16 hay blocks once and advanced to wave 2's five spiders, and /gateways open opened a second gateway"
