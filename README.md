# Gateways to Eternity — a Horizon port

**Gateways to Eternity** is a Minecraft mod. A minecraft mod about spawning mobs via portals. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Shadows-of-Fire/GatewaysToEternity](https://github.com/Shadows-of-Fire/GatewaysToEternity) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Gateways to Eternity ([source](https://github.com/Shadows-of-Fire/GatewaysToEternity)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `gateways.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What works

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

## Not yet ported

- The other seven gateway definitions (basic blaze/slime/enderman,
- Attribute/effect/gear modifiers on wave mobs (attribute API), `entity_loot`
- The gateway entity itself: renderer, particles, boss bar, sounds, and a
- The pearl's tooltip (the wave/reward/summary lines of `NormalGateClient`) and
- JEI/Jade compat, the `gates_defeated` statistic and the advancement trigger.
- **No in-game run for 0.2.0** (trap 15): the behaviour's evidence is the 0.1.0 e2e
- **The optional `CONTROL_GAME=` leg is unexercised** — it needs a second launcher build
- **The pearl's Part is attached to a *content* item id** (`Registrar::item_key(

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `gateways.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Gateways to Eternity was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Shadows-of-Fire/GatewaysToEternity

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run gateways` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/gateways/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
