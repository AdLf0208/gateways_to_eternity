# Gateways to Eternity for Horizon

A Horizon port of **Gateways to Eternity** by Brennan Ward (`Shadows_of_Fire`), MIT.
One gate pearl opens a gateway, the gateway runs its wave list, and each wave pays
out when its mobs die.

**This is slice 1: one gateway, its full wave list, and its rewards.** It ships as one
flat SDK package (`gateways.hmod`) carrying the code and the content: `api = "0.2"`,
`src/lib.rs` is the one `impl mod_api::Hooks`, `src/support.rs` is the glue and
`src/runtime.rs` is the pure state machine with its tests.

## What works

- `gateways:gate_pearl`, made by the emerald-grove recipe (8 flowers around an
  ender eye), carrying its gateway in `minecraft:custom_data`.
- Using a pearl on a block opens `gateways:emerald_grove` there: the pearl's
  placement follows upstream (`GatePearlItem.useOn` — the clicked block's top,
  lifted while the box collides), and a second gateway in the same box is
  refused.
- The gateway runs the jar's four waves in order (zombies, spiders, a mixed wave,
  a named zombie villager), each after its `setup_time`, with the wave's
  `max_wave_time`, the 200-tick no-player grace period, dead-mob accounting,
  out-of-leash re-placement, and rewards split into stacks of four that drop from
  the gateway.
- Wave rewards and the completion summons (six of each animal) are paid once.
- `/gateways list` and `/gateways open <x> <y> <z> <type>` (level 2), upstream's
  `/open_gateway`.

## What it does not do (yet)

- **No gateway entity**: no visible portal, boss bar, particles or sound — the
  gateway is a server-side record, and the wave mobs are what you see.
- An open gateway does not survive a world reload.
- Wave attribute modifiers, gear sets and `entity_loot` rewards are not applied;
  failure explosions, lives, and the other seven gateway definitions are not
  carried.

`notes.md` has the full list, the deviations, the migration's hook table and every
trap this port learned.

## Building

The SDK is fetched into `.horizon/sdk/` by the tool (gitignored, never packed):

```sh
cargo test                                     # the pure logic: 18 runtime + 7 glue tests
cargo clippy --all-targets -- -D warnings     # 0 warnings
cargo fmt --check                              # not --all: that checks the fetched SDK
rm -f Cargo.lock && cargo horizon-mod build    # -> target/horizon-mod/gateways.hmod
cargo horizon-mod check gateways.hmod          # the registry's own check
cargo horizon-mod fits gateways.hmod --build nightly
```

`Cargo.lock` is packed and changes the artifact: remove it right before the build.
The in-game leg needs a game the launcher built with this package
(`horizon-launcher native-build .`), which is not installed in this environment —
`tools/e2e.sh` is kept for that run and no 0.2.0 in-game result is claimed.

## Licence

MIT (see `LICENSE`, and `LICENSE-NOTICE.md` for where it was read and what
travels from upstream).
