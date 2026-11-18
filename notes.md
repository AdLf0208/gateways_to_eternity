# Gateways to Eternity — port notes

> **Current state: the SDK package, version 0.2.0 (2026-11-18).** The retired
> `.hnmod` container is history; read
> [`## Migration to the SDK format`](#migration-to-the-sdk-format-2026-11-18-sdk-0b14f8a)
> at the end of this file for the file map, the helper→hook table, this port's own
> numbered traps, the measured ladder and the changelog. Everything above it
> describes the 0.1.0 slice, which is still the port's behaviour.

## Identity

| | |
|---|---|
| Mod | **Gateways to Eternity** 1.21.1-**5.1.0** (`Shadows_of_Fire`) |
| Class / tier | `content` / **deep** (ATM10 row 209; an entity-driven combat encounter) |
| Slot | core, mechanics, `P0` |
| Licence | **MIT** — read from the jar (`META-INF/neoforge.mods.toml`, `license="MIT License"`) and from the upstream `LICENSE` ("Copyright (c) 2020 Brennan Ward") → **rank 0**, permissive |
| Pin | SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` (2026-11-18, `package_format` 6, `content_api` "0.2") + mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; the 0.1.0 bundle's campaign pin was Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` + mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` |
| Source read | `Shadows-of-Fire/GatewaysToEternity`, default branch commit `73aa0726c998214a180df1e2981ef1752ddb56e9` (6.0.2, MC **26.1.2**). The pack ships 1.21.1-5.1.0 and the repo has no 5.1.0 tag, so the 26.1.2 source was read — closer to the 26.3 target than 1.21.1 (the `ironjetpacks` / `silentgems` precedent). The clone was deleted after `FINDINGS.md`; the jar stays in `licence-cache/` |

## What the slice is (and is not)

Upstream's runtime object is an **entity**: `NormalGatewayEntity` ticks itself,
spawns waves, and pays rewards. A content mod at this pin can declare an entity
type from a template but cannot give it a tick body, so the gateway is carried as
a **server-side gateway record** (`mixins/gateways/support.rs`, the `Live` list)
driven from `level_tick_post` (API 23), plus the pearl's `use_item_on` (API 13)
and `/gateways open` (API 8). The state machine, the wave tables, the reward
tables and the rules are the upstream ones, so the *behaviour* is the mod's; the
**entity, its renderer, the boss bar, the particles and the soundtrack are gaps**.

The slice carries **one gateway**, `gateways:emerald_grove`, with its complete
four-wave list from the jar (`data/gateways/gateways/emerald_grove.json`):

| wave | mobs | setup / max | wave reward |
|---:|---|---|---|
| 1 | 5 × zombie | 200 / 1000 | 16 × hay_block |
| 2 | 5 × spider | 200 / 1000 | 16 cactus + 16 sugar cane + 16 bamboo |
| 3 | 5 × zombie + 2 × spider | 200 / 1000 | 32 carrot / potato / apple / beetroot |
| 4 | 1 × zombie_villager "Necrotic Farmer" + 3 zombie + 2 spider | 200 / 1200 | 16 of each sapling |

Completion summons 6 cows, 6 chickens, 6 sheep and 6 pigs; the failure penalty is
an explosion, which is **not** carried (gap).

The runtime is pure and unit-tested (18 tests, also run as
`game::gateways::tests` inside the staged game): the tick order mirrors
`GatewayEntity.tick` (wave timer → nearby-player grace → dead-mob pruning →
out-of-leash every 20 ticks → reward drip every 4 ticks → wave/complete
transitions), `Wave.spawnRewards`' "split at four", `NormalGatewayEntity`'s
completion gate, `SpawnAlgorithms.open_field` and `GatePearlItem.useOn`'s
placement lift.

## Hooks used (all stamped at the pin; the 0.2.0 place is in the migration table)

The 0.1.0 bundle used the hook points below. In 0.2.0 they are typed `Hooks`
methods and one item Part — see *Helper → hook table* in the migration section; the
right-hand column here is the API the 0.1.0 injection named.

| Hook | API | Why |
|---|---|---|
| `content_root` | AE2 pattern | write the embedded `.hmod` to `.horizon-cache/` and append it to the dev list, so one file installs the content |
| `use_item_on` | 13 | a gate pearl opens its gateway on the clicked block |
| `level_tick_post` | 23 | one tick per level, with the level in place: `native_spawn_mob`, `native_mobs`, `native_summon`, `native_teleport_mob`, `native_remove_mob` all need it |
| `register_commands` | 8 | `/gateways open <x> <y> <z> <type>` and `/gateways list`, at level 2 (`GAMEMASTERS`, upstream's `/open_gateway`) |
| `perform_command` | 8 | run them |

Helpers used: `native_spawn_mob`/`native_mobs` (API 37), `native_summon`,
`native_teleport_mob`, `native_remove_mob`, `native_block_collision`,
`native_block`, `native_world_height`, `native_server_players`, `native_tell`,
`native_feedback`.

## Content

The carried content now sits at the **package root**: `assets/gateways/` and
`data/gateways/` (the 0.1.0 `content/` is the evidence copy under
`../_shared/evidence/gateways/retired-hnmod-0.1.0/content/`, and every carried file is
byte-identical — `evidence/gateways/sdk-content-carry.log`).

`assets/gateways/` + `data/gateways/` carry the item `gateways:gate_pearl` (template
`minecraft:emerald` — an inert item — rarity uncommon), the emerald-grove recipe
(8 × `#minecraft:flowers` around an ender eye) whose **result component**
`minecraft:custom_data={gateway:"gateways:emerald_grove"}` is what binds a pearl
to its gateway, the 26.3 item definition and model, the upstream texture
`gate_pearl_small.png`, upstream's `en_us.json`, and the creative tab. The 0.1.0
bundle embedded the built `.hmod` at `mixins/gateways/data/`; in 0.2.0 the content
travels in the same package, so there is nothing to embed.

The jar's `gateways:gate_recipe` type cannot be declared by a content mod, so the
four upstream recipes are carried as **vanilla `crafting_shaped` recipes with a
result component**; one (emerald grove) is shipped. The other three gateway
pearls are gaps.

## Upstream files read

From the 26.1.2 source (`73aa0726…`, clone deleted after these notes):
`entity/GatewayEntity.java`, `entity/NormalGatewayEntity.java`,
`gate/ Gateway.java`, `gate/normal/NormalGateway.java`, `gate/Wave.java`,
`gate/StandardWaveEntity.java`, `gate/Reward.java`, `gate/GateRules.java`,
`gate/SpawnAlgorithms.java`, `item/GatePearlItem.java`,
`command/GatewayCommand.java`, `GatewayEvents.java`, `GatewayObjects.java`,
`gate/GatewayRegistry.java`. From the jar: `META-INF/neoforge.mods.toml`,
`data/gateways/gateways/*.json`, `data/gateways/recipe/*.json`,
`assets/gateways/lang/en_us.json`, the item models and textures,
`assets/gateways/sounds.json`. The behaviour extracted from each is in
[`FINDINGS.md`](FINDINGS.md).

## Upstream defects fixed

None found that the slice carries. Two observations are recorded as *findings*,
not fixes:

- `GateRules.requires_nearby_player` is declared and shown in tooltips but **never
  read** by `GatewayEntity.tick`, which always fails after a 200-tick grace
  period without a nearby player. The port implements the tick's behaviour (the
  rule is inert upstream); it is not "fixed", because that would invent a
  different failure rule.
- The jar's census lists "4 items"; the artefact has **one** item id
  (`gateways:gate_pearl`) and three size textures/model files. The artefact wins
  (see `FINDINGS.md`).

## Deviations (deliberate, recorded)

- **No gateway entity.** The gateway is a server-side record: no visible portal,
  no boss bar, no particles, no loop sound. The wave mobs and the rewards are the
  observable behaviour, and they are the slice.
- **Memory only.** A world reload closes every open gateway (upstream saves the
  entity). Recorded gap; a persistence file would be the next slice.
- **Attribute modifiers not applied.** The pin has no attribute API, so the
  waves' `+% max health` / armour / attack-damage modifiers, the gear set on the
  "Necrotic Farmer" (`finalize_spawn: false`) and `follow_range_boost` are not
  applied. Mobs spawn with vanilla stats. (`native_spawn_mob` can set a name and
  an absolute health only.)
- **Failure effects.** The gateway closes, its mobs are removed and the summoner
  is told the reason; the `explosion` penalty and the lightning are not carried.
- **Lives.** `playerDied` → a lost life → `OUT_OF_LIVES` is not implemented (no
  death hook was wired in this slice); `lives` is -1 (infinite) for this gateway
  anyway.
- **`entity_loot` rewards** (a mob's loot table rolled N times) are not carried;
  this gateway's waves and completion use `stack` / `stack_list` / `summon`.
- **The pearl keeps its size texture.** The three size models are selected by a
  custom item property at this pin's content API, so only the small texture is
  carried.
- Messages are **plain English text** (`native_tell`/`native_feedback`), not
  translatable components; the lang file travels for names and tooltips.

## Gaps (next slices)

1. The other seven gateway definitions (basic blaze/slime/enderman,
   `overworldian_nights`, `hellish_fortress`, the endless pair) and their
   recipes.
2. Attribute/effect/gear modifiers on wave mobs (attribute API), `entity_loot`
   rewards, failure explosions, lives, persistence.
3. The gateway entity itself: renderer, particles, boss bar, sounds, and a
   visible portal (needs a custom entity type with a tick body, or a display
   entity driven from the record).
4. The pearl's tooltip (the wave/reward/summary lines of `NormalGateClient`) and
   the size-driven pearl model.
5. JEI/Jade compat, the `gates_defeated` statistic and the advancement trigger.

## Verification

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

| Check | Command | Result |
|---|---|---|
| fmt | `cargo fmt -- --check` | exit 0 |
| clippy | `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| runtime tests | `cargo test --locked --offline` | `18 passed; 0 failed` |
| stage | `tools/horizon.py stage` | pinned export + plan + apply, exit 0 |
| check | `tools/horizon.py check` | `cargo_exit: 0`, 0 warnings |
| embedded tests | `tools/horizon.py test` | `18 passed; 0 failed` (`game::gateways::tests`) |
| build | `tools/horizon.py build` | `cargo_exit: 0`, fast client, 0 warnings |
| e2e default | `tools/e2e.sh` (embedded, no `--dev-mod`, unusable-cache control) | **PASS**, `EXIT:0` — `ports/_shared/evidence/gateways/e2e-default.log` |
| e2e selftest | `SELFTEST=1 tools/e2e.sh` | **FAIL**, `EXIT:1` (the inverted reward assertion) — `e2e-selftest.log` |
| checker | `check-e2e-logs.py gateways --strict` | `logs=2 ok=2`, exit 0 |
| packager | `tools/package.sh` | `.hmod` 5,513 B, `.hnmod` 25,682 B |
| registry check | `cargo horizon-mod check gateways.hnmod --kind native-mod` | exit 0 |

**These rows are the 0.1.0 `.hnmod` ladder and are historical**: the tools that
produced them (`tools/horizon.py`, `tools/package.sh`) are retired and the e2e legs ran
on a game built from the sources with the content embedded in the bundle. The 0.2.0
ladder is in the migration section below; the in-game legs were **not** re-run (no
`horizon-launcher` here).

Test-order note: the runtime tests were written first and watched fail (a
first red run: 5 of 18 failed on wrong expectations, fixed in the tests, not the
code), and the suite was then proved **load-bearing** by two mutations —
dropping `self.active = false` on wave completion (4 tests fail, including
`the_reward_is_granted_once`) and paying a wave with living mobs (2 tests fail,
including `an_unkilled_wave_never_pays`) — each restored and re-run green.

In-game harness facts found here (worth keeping):

- `ctl use-item` after `look-at block` left the crosshair off the block
  (`state.target` null, the click did nothing); **`ctl interact X Y Z FACE`** aims
  and clicks, and its reply names the block it hit.
- A `fill` into a dimension **nobody has entered** applies to a chunk that is not
  loaded, and the chunk regenerates over it when the player arrives (the floor
  read back as the End's bedrock). Teleport first, then fill.
- 26.3's gamerules are `spawn_monsters` / `spawn_mobs` (not `doMobSpawning`).
- A creative player's pearl is **not** consumed — upstream's
  `if (!player.isCreative()) stack.shrink(1)`, and the port follows it.
- `/gateways` needs the `register_commands` hook (the `NativeCommand` usages) for
  the **server** tree; the legacy `commands` hook (`&mut Vec<&'static str>`) only
  colours the chat box, so without both the client says "Unknown command".
- A command's answer belongs in `native_feedback` (the source's feedback): a
  `native_tell` line does not appear in `ctl command`'s reply.

## Migration to the SDK format (2026-11-18, SDK `0b14f8a`)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70; PR #17992 cleans up the old files)
retired `.hnmod`: the plan, the injection points and the per-hook helper files are gone,
and a native mod is one flat `.hmod` carrying `horizon-mod.toml` (`api = "0.2"`),
`Cargo.toml`, `src/lib.rs` (one `impl Hooks`), `src/runtime.rs`, `assets/` and `data/`.
Pins for this migration: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**
(2026-11-18, `package_format` 6, `content_api` "0.2"), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78, for Minecraft 26.3.
The SDK is cached in-tree at `../_shared/sdk/0b14f8a/` and fetched into `.horizon/sdk/`
by any `cargo horizon-mod` command (gitignored, never packed).

**Discrepancy recorded — the artefact wins.** An earlier campaign note claimed this port
had already been migrated to the SDK format. The disk said otherwise: `ports/gateways`
held `mixins/gateways/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, six per-hook
helpers), `compatibility.json`, `rust-toolchain.toml`, `content/` and a 25,682 B
`gateways.hnmod`, and there was **no `horizon-mod.toml`**. The artefact is the evidence,
so the migration was done in this job and **0.2.0 is this port's first SDK package and a
first publication** (there is no closed PR to supersede).

**Done in this order:** the retired layout moved to
`../_shared/evidence/gateways/retired-hnmod-0.1.0/` **before** the first
`cargo horizon-mod build` (the tool refuses a root holding retired package paths), the
content moved to the package root, `src/runtime.rs` was copied with `cp` and proved with
`cmp` + `sha256sum`, `cargo test` ran on the new crate **before any glue existed**
(18/18 green), and only then were `src/lib.rs` and `src/support.rs` written — the glue's
seven tests first red on `todo!()`, then green.

### File map

| Old `.hnmod` layout | New SDK package | Where the old one lives |
|---|---|---|
| `horizon-mixin.toml` (`[mod]`, `[compatibility]`, 2 `[[module]]`, 6 `[[injection]]`) | `horizon-mod.toml` (id/name/version/api/side/capabilities + `[native]`) | `retired-hnmod-0.1.0/` |
| `Cargo.toml` (`[lib] path = "mixins/gateways/runtime.rs"`, `[workspace]`) | `Cargo.toml` (`[lib]` implied at `src/lib.rs`, one path dependency: `horizon-mod-api`) | `retired-hnmod-0.1.0/` |
| `mixins/gateways/runtime.rs` (module + crate lib) | `src/runtime.rs` — **byte-identical** (`95ba3e97…fca6c`), `pub mod runtime;` at the crate root | `retired-hnmod-0.1.0/mixins/gateways/` |
| `mixins/gateways/support.rs` (`gateways_state`, 427 lines) | `src/support.rs` (the glue: the open gateways, the tick, the pearl's Part, the commands) | same |
| six per-hook helper `.rs` files (`embedded_content.rs`, `use_item_on.rs`, `level_tick_post.rs`, `register.rs`, `commands.rs`, `perform_command.rs`) | methods of `impl Hooks`, one item Part, or a `Registrar` call; no per-hook files | same |
| `content/` (`horizon-mod.toml`, `assets/`, `data/`, `gateways.hmod` 5,513 B) | package root: `horizon-mod.toml`, `assets/`, `data/` — **every carried file byte-identical** (`sdk-content-carry.log`) | `retired-hnmod-0.1.0/content/` |
| `mixins/gateways/data/gateways-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: code and content are one package, so there is nothing to embed | `retired-hnmod-0.1.0/mixins/gateways/` |
| `compatibility.json`, `rust-toolchain.toml` | **gone**: `[native]` is the pin and the launcher's toolchain is the one that builds | `retired-hnmod-0.1.0/` |
| `tools/horizon.py` (stage/check/test/build/ctl), `tools/package.py`, `tools/package.sh`, `tools/add-notices.py` | **gone**: no staging, no `.hnmod` packaging | `retired-hnmod-0.1.0/tools/` |
| `gateways.hnmod` (25,682 B, embeds the content) | `gateways.hmod` (33,298 B, native + content) | `retired-hnmod-0.1.0/gateways.hnmod` |
| `Cargo.lock` (160 B) | not kept: it is packed and changes the artifact (trap 5) | not kept |
| `target/` (staged tree, ladder logs) | deleted (disposable); the 0.1.0 `check`/`test`/`build` logs are in `retired-hnmod-0.1.0/target-logs/` | same |

**Kept and updated:** `tools/e2e.sh` (the scenario and its assertions are
format-independent; the launch block now needs a launcher-built game, see below),
`README.md` (rewritten for the SDK package — it is packed), `notes.md`, `FINDINGS.md`,
`VALIDATION.md`, `LICENSE`, `LICENSE-NOTICE.md`.

### Manifest mapping

- identity: `id = "gateways"` (also the resource namespace), `name`, `version = "0.2.0"`
  (the port's version, not upstream's 5.1.0), `authors = ["Shadows_of_Fire", "AdLf0208"]`.
- `api = "0.2"` (exact), `side = "both"` (an integrated server belongs to the client
  installation, so a single-player world and a LAN host need it),
  `capabilities = ["registries"]` (the schema's one value; this package really does
  declare an item, its recipe and a creative tab).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's mixins revision. Both pins are
  **40-hex**: a short one reads as missing (trap 2).
- `license` is not a manifest field (`Cargo.toml` carries `license = "MIT"`); the licence
  text and the notice travel in the package as `LICENSE` / `LICENSE-NOTICE.md`.
- `[mod] priority = 100` and the plan's ordering are gone; this slice needs no ordering
  (`#[listener(priority = …)]` is the new lever).

### Helper → hook table (every `[[module]]` and every `[[injection]]`)

| 0.1.0 plan entry | 0.2.0 place | Mapping notes |
|---|---|---|
| `[[module]] id = "runtime"`, `source = "runtime.rs"` (`crate::game::gateways`) | `src/runtime.rs`, `pub mod runtime;` | byte-identical file; the module name is gone, the path is the crate's |
| `[[module]] id = "state"`, `source = "support.rs"` (`crate::game::gateways_state`) | `src/support.rs`, `pub mod support;` | the same glue, reading hosts instead of `Game` |
| `[[injection]] embedded-content` (`content_root`, HEAD, cancellable) | **gone** | one flat `.hmod` carries `assets/` and `data/`; there is no bundle to write out |
| `[[injection]] use-item-on` (`use_item_on` 13, HEAD) | `ItemPart::use_on` on `support::Pearl`, attached with `Registrar::item_part` in `Hooks::register` | upstream's `GatePearlItem.useOn` is `ItemStack.useOn`, which is what `ItemPart::use_on` is; the Part is attached to the pearl, so no hook tests the item id |
| `[[injection]] level-tick` (`level_tick_post` 23, HEAD) | `Hooks::level_tick_post(host: HostSide, event: &mut LevelTickPostEvent)` | `event.level` is the `LevelRef` in place (`HostSide::Server`); the client side returns |
| `[[injection]] register-commands` (`register_commands` 8, HEAD) | `Hooks::register_commands` → `support::declare_commands` | two `Command::op(…, 2)` usages, the same strings |
| `[[injection]] commands` (`commands` 8, client chat tree) | **gone** | `register_commands` gives every player's chat box the same completion, within their level |
| `[[injection]] perform-command` (`perform_command` 8, HEAD) | `Hooks::run_mod_command` → `support::run_command` | typed `CommandInvocation` (`literals` + `arguments`), `Flow::Cancel` when handled; the usage strings keep `x:number y:number z:number gateway:word` so the arguments resolve to the same three doubles |
| `Cargo.toml` `[workspace] members = ["tools/audit"]` | **gone** | a second crate in the tree is a retired package path |

The glue's game calls, old → new:

| 0.1.0 `Game` call | 0.2.0 host call |
|---|---|
| `game.native_spawn_mob(dim, kind, pos, NativeSpawn{name})` | `host.summon(dim, kind, pos, nbt)` with `CustomName` in the compound (`support::name_nbt`) |
| `game.native_summon(dim, kind, pos, nbt)` | `host.summon(…)` (the same call; a reward stack's `Item` compound comes from `ItemStack::to_nbt(host)`) |
| `game.native_mobs(dim)` + `native_mob(id)` | `host.entity(id)` → `EntityRef::position(host)`; a gone entity is `None`, which is what the tick wants |
| `game.native_teleport_mob(dim, id, pos)` | `entity.teleport_to(host, pos, None)` |
| `game.native_remove_mob(dim, id)` | `entity.remove(host, RemovalReason::Discarded)` |
| `game.native_block_collision(pos)` (boxes relative to the block) | `level.collision_boxes(host, cell)` (world-space) for the click's top; `level.has_collisions(host, cell)` per cell for the spawn probe |
| `game.native_world_height().0` | `level.height(host)` → `(min_y, height)` |
| `game.native_server_players()` | `level.online_players(level)` → `PlayerRef::entity().position(host)` |
| `game.native_acting_player()` | not needed: `UseOnEvent.player` is the clicking player's UUID (`CommandSource::Player` on the command path) |
| `game.native_tell(uuid, text)` | `host.player(uuid)` → `PlayerRef::tell(host, Text::literal(..))` |
| `game.native_feedback(text)` | `host.commands().feedback(Text::literal(..))` |
| `game.held()` / `held_slot()` / `held_mut()` / `game.creative` / `hand_stack_replaced` / `flush_sync` (the LAN re-send) | `ItemPlace::PlayerHand { player, hand }.extract(host, tx, &ItemResource::of(stack), 1)` inside `host.transaction`; a creative player gives without losing any, so the separate creative branch is gone |
| `game.dim.id()` | `LevelRef::dimension().id(host)` (`Option<String>`) |
| `log::warn!(…)` in the content write-out | **none**: there is no logger in the SDK and no content write-out any more |
| `horizon_client::native_data!` (the embedded `.hmod`) | **none**: the content is in the package |

### Ladder (measured 2026-11-18)

`build-slot.sh` still speaks the retired tool, so it is used as the slot guard, not as the
driver; every command below ran **directly in the port** with `BUILD_LOCAL=1`,
`CARGO_INCREMENTAL=0`, no `--target-dir` and no staging step.

| Step | Command | Result | Evidence |
|---|---|---|---|
| runtime only, before any glue | `cargo test --offline` | **18 passed**, 0 failed | `evidence/gateways/sdk-test-runtime.log` |
| glue red | `cargo test --offline` (bodies `todo!()`) | **7 failed** on `not yet implemented`, 18 passed, exit 101 | `sdk-test-red.log` |
| tests | `cargo test --offline` | **25 passed**, 0 failed (18 `runtime::`, 7 `support::`), exit 0 | `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings | `sdk-clippy.log` |
| format | `cargo fmt --check` (**not** `--all`) | exit 0, empty diff | `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/gateways.hmod`, exit 0 | `sdk-build.log` |
| registry check | `cargo horizon-mod check gateways.hmod` | `gateways 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)` / `33298 bytes, SHA-256 9478db99…67199, unsigned`, **exit 0** | `sdk-check.log` |
| fits the pinned build | `cargo horizon-mod fits gateways.hmod --build nightly` | `Gateways to Eternity:` / `0b14f8a  made for`, **exit 0** | `sdk-fits.log` |
| same package twice | `rm -f Cargo.lock && cargo horizon-mod build` again | 33,298 B and `9478db99…67199` both times, **byte-identical** | `sdk-build-second.log`, `sdk-sha256.txt` |
| a cleaned tree rebuilds the same file | `target/` removed, then test + build from scratch | 25 passed; the package rebuilt to the same 33,298 B and `9478db99…67199` (`cmp` clean) | `sdk-test.log`, `sdk-build.log` |
| `runtime.rs` untouched | `unzip -p gateways.hmod src/runtime.rs \| sha256sum` | `95ba3e97…fca6c` = `src/runtime.rs` = the retired file | `sdk-sha256.txt` |
| content carry unchanged | `diff -r` against the retired `content/assets` + `content/data` | 0 differences, 7 files | `sdk-content-carry.log` |
| what it uses | `unzip -p gateways.hmod uses.json` | **217 surface entries**: 14 hook (+field), 20 method, 2 part, 1 capability, 2 macro, 18 trait, 98 type, 62 handle | `sdk-uses.json`, `sdk-uses-summary.txt` |

The archive is **16 entries**: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`,
`LICENSE-NOTICE.md`, `README.md`, `src/lib.rs`, `src/runtime.rs`, `src/support.rs`,
`uses.json`, and the seven carried content files under `assets/gateways/` and
`data/gateways/`. **No `Cargo.lock`** (trap 5).

### Traps this migration hit (the next port must know)

1. **`build-slot.sh run` forwards to the remote box unless `BUILD_LOCAL=1` is in the
   *runner's* environment.** Put it before the command
   (`BUILD_LOCAL=1 ports/tools/build-slot.sh run …`); inside the wrapped `bash -c` it is
   too late — the far box ran `cd ports/gateways: No such file or directory`.
2. **A `… | tee log` pipeline returns `tee`'s status.** The first test run printed
   `released slot (exit 0)` while the crate had failed to compile, and the 7-test red run
   printed `exit 0` too. Use `bash -o pipefail -c '…'` for every ladder step (and
   `echo "… exit ${PIPESTATUS[0]}"` where the log must carry the code).
3. **`Compound::get` is `BTreeMap::get` and wants a `&JavaString`, which the SDK does not
   export.** A `&str` lookup goes through `Tag::get` or a UTF-16 key walk; the glue has
   one `support::tag` helper for it, and both tests read fields through it.
4. **`DimensionKey::id(host)` takes the host and answers `Option<String>`** (the 0.1.0
   read `game.dim.id()`). `LevelRef::dimension()` gives the key; the *name* needs a host,
   so every place that compares dimensions resolves it once.
5. **A nameless dimension must not be reported as a space failure.** `open_at` takes the
   dimension string the caller resolved (`pearl_use_on` answers `None`, the command
   answers `NO_LEVEL_LINE`), because folding it into `OpenError` would print the wrong
   line.
6. **Entity and item verbs live on the handles, not on the host.** `entity.teleport_to`,
   `entity.remove`, `entity.position`; `host.entity_teleport_to` does not exist. Same for
   `player.tell`, `level.block`, `player.inventory_slots`.
7. **`PlayerRef` has no position.** The player's own entity handle does:
   `player.entity().position(host)`; `level.online_players(level)` is the list for the
   leash check.
8. **`ItemPart::use_on` is `ItemStack.useOn`, which runs *after* the block's own use.**
   That is exactly where upstream's `GatePearlItem.useOn` sits, so it is the faithful
   route — but it is a behaviour change from 0.1.0's `use_item_on` HEAD, which claimed
   the click first (a pearl clicked on a chest now opens the chest).
9. **`InteractionResult::Fail` is upstream's answer for the spacing refusal**, and it
   ends the interaction; 0.1.0 fell through without claiming, letting vanilla continue.
   The port now answers as upstream does.
10. **The Part must be attached to the pearl's own key**, so the gateway id is the only id
    the hook reads: `stack.components()?` → `minecraft:custom_data` → `gateway`
    (`stack.data()` and the old item-id test are both gone).
11. **`#[mod_api::horizon_part]` on the `impl ItemPart` is not decoration**: without it
    the subscription table is empty and the Part silently never fires.
12. **`Registrar::item_part(r.item_key(id), part)` is two mutable borrows of the
    registrar** — bind the key to a local first.
13. **The LAN inventory re-send workaround is gone.** 0.1.0 kept `SYNC`/`flush_sync` and
    wrote `host.hub.guests[…].inventory` by hand; `ItemPlace::PlayerHand{..}.extract`
    inside `host.transaction` writes the slot through the documented path, and creative
    is handled there, so the separate `game.creative` branch went with it.
14. **A poisoned `Mutex` is recovered, not re-raised**
    (`unwrap_or_else(PoisonError::into_inner)`): the game must not panic its tick a second
    time for a lock a previous panic poisoned. This is the one place the run-time
    guidance is deliberately inverted, and the reason is in the code.
15. **The e2e cannot run here.** `horizon-launcher` is not installed, so there is no
    launcher-built game to point `GAME=` at; `tools/e2e.sh` now *requires* `GAME`, has an
    optional `CONTROL_GAME=` leg (a game built without the package: the pearl and the
    recipe must be unknown) and the `.horizon-cache` embedded-content legs are gone with
    the format. No 0.2.0 in-game result is claimed.
16. **`uses.json` lists `hook:command` and `method:Hooks::command` even though this mod
    implements neither** — those are the `Hooks` trait's surface, not the mod's
    subscriptions (`enderstorage`'s 0.2.0 package lists them too). Read the `part:` and
    `method:ItemPart::use_on` entries for what this port really attaches.

### Behaviour that changed with the format (deliberate)

- **When the pearl's click is claimed** (trap 8): after the block's own use, as upstream's
  `ItemStack.useOn` does, instead of before it.
- **The spacing refusal answers `Fail`** (trap 9) instead of silently falling through.
- **`/gateways open` from the console has no summoner**, so its messages go nowhere
  (`say` finds no player); the player and command-block sources answer as before. The
  gateway still opens (the summoner is only a message target and the RNG seed).
- **The client chat-tree injection is gone**: completion comes from `register_commands`.
- **No content write-out, no log line**: the embedded `.hmod` and the `log::warn!` that
  reported a failed write are both gone with the format.

### Gaps after the migration

Everything in *Gaps (next slices)* above still stands (the entity, persistence, wave
modifiers, `entity_loot`, failure explosions, lives, the other seven gateways, the
tooltip and the size models). Added or restated by the migration:

1. **No in-game run for 0.2.0** (trap 15): the behaviour's evidence is the 0.1.0 e2e
   (`evidence/gateways/e2e-default.log`, `e2e-selftest.log`) on a **byte-identical**
   `runtime.rs`, plus the 25 green tests and the compile gate. The changed transport
   (hosts, a Part, typed commands) is exactly what that run would re-prove.
2. **The optional `CONTROL_GAME=` leg is unexercised** — it needs a second launcher build
   without the package.
3. **The pearl's Part is attached to a *content* item id** (`Registrar::item_key(
   "gateways:gate_pearl")`, a forward reference the registrar validates after all
   declarations). The SDK documents exactly this use and the API accepts it, but only an
   in-game run proves the attach lands on the carried item; `enderstorage` 0.2.0 has the
   same unverified step.
4. **`NO_LEVEL_LINE`** is a port line for a case 0.1.0 could not reach; it is recorded
   here rather than presented as upstream text.

### Changelog (0.2.0 — first publication)

> **0.2.0 — first publication.** Gateways to Eternity for Horizon, built as one flat SDK
> package for `api = "0.2"`. A gate pearl (`gateways:gate_pearl`, made by the emerald-grove
> recipe) opens `gateways:emerald_grove`, which runs the jar's four waves — zombies,
> spiders, a mixed wave and the named "Necrotic Farmer" — each after its setup time, with
> the wave timer, the 200-tick no-player grace period, out-of-leash re-placement, rewards
> split into stacks of four that drop from the gateway, and the completion summons; the
> pearl's placement lift and the spacing refusal follow `GatePearlItem.useOn`.
> `/gateways open <x> <y> <z> <type>` and `/gateways list` stand in for upstream's
> `/open_gateway`. There is no gateway entity yet (no portal, boss bar, particles or
> sound), an open gateway does not survive a reload, and wave attribute modifiers, gear
> sets, `entity_loot` rewards, failure explosions and lives are not carried. The package
> carries its own content (item, recipe, lang, texture, creative tab). Built against SDK
> `0b14f8acfd97fa2c750e42795249dcc981178ca4` and mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, for 26.3; `src/runtime.rs` is unchanged
> from the pre-migration port, byte for byte.

### Publication route (campaign ruling 2026-11-18)

The submission form is gone; publishing is `horizon-rust/horizon-mods#1147` — the artifact
is attached to a release of the author's own repository under the entry's exact `file`
name, a PR adds the `[[versions]]` record and links the file, and a **maintainer** runs
`scripts/stage.sh` (a red `check` before that is the expected `fetch.sh` failure). This is
a **first publication**, so its PR says so instead of naming a superseded PR.
`python3 ports/tools/publish-ready.py gateways` does the bookkeeping; this job owns no
submission.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
