# Gateways to Eternity — findings

Jar: `ports/_shared/licence-cache/GatewaysToEternity-1.21.1-5.1.0.jar`
(649,614 B, 168 entries). Upstream source:
<https://github.com/Shadows-of-Fire/GatewaysToEternity>, default branch
`73aa0726c998214a180df1e2981ef1752ddb56e9` (2026-07-24, version **6.0.2, MC
26.1.2**), shallow clone in `ports/_shared/upstream/gateways/` (deleted after
these findings). The pack ships 1.21.1-5.1.0 and the repo has no 5.1.0 tag, so
the **26.1.2 source was read** (it is closer to the 26.3 target than 1.21.1; the
same precedent as `ironjetpacks` and `silentgems`). The jar's own data files are
byte-evidence for the 1.21.1 numbers and are what the port carries.

## Licence

`META-INF/neoforge.mods.toml`: `license="MIT License"`, `modId="gateways"`,
`version="5.1.0"`, author `Shadows_of_Fire`. Rank 0 (permissive). The repo has a
`LICENSE` file (MIT, `Copyright (c) 2021 Shadows_of_Fire`).

## Jar inventory (the artefact, not the name)

| Family | Count | Notes |
|---|---:|---|
| item ids | **1** | `gateways:gate_pearl` (`GatewayObjects.GATE_PEARL`, rarity uncommon). The census's "4 items" counts the item **model files**: `gate_pearl`, `gate_pearl_med`, `gate_pearl_large` are the small/medium/large **size textures** selected by an item-property override on one item, not four items. The artefact wins. |
| blockstates | 0 | the gateway is an **entity**, not a block |
| entity types | 2 | `gateways:gateway` (`normal_gateway` and `endless_gateway` are both registered under the one key) |
| recipes | 7 | all of custom type `gateways:gate_recipe` (4 basic, 3 named) |
| gateway definitions | 8 | `data/gateways/gateways/**` — `basic/{blaze,enderman,slime}`, `emerald_grove`, `overworldian_nights`, `hellish_fortress`, `endless/blaze`, `endless/{…}` |
| gear sets | 5 | `data/gateways/gear_sets/*` (equipment a wave modifier puts on a mob) |
| sounds | 4 + `sounds.json` | `gate_ambient`, `gate_warp`, `gate_start`, `gate_end` |
| textures | 5 | entity/gateway, particle/glow, item/{small,med,large} |
| lang | 4 | en_us, pt_br, ru_ru, zh_cn |
| classes | 96 | `entity/{GatewayEntity,NormalGatewayEntity,EndlessGatewayEntity}`, `gate/*` (Gateway, Wave, Reward, Failure, GateRules, SpawnAlgorithms), `item/GatePearlItem`, `command/GatewayCommand`, `GatewayEvents`, client renderer/particles |

## Behaviour extracted (26.1.2 source; files read)

- **`item/GatePearlItem.java` `useOn`** — the pearl carries the gateway id in a
  data component (`GatewayObjects.GATEWAY_COMPONENT`). Unbound → `FAIL`. Client →
  `SUCCESS`. `gate.canOpen(player)` → red message + `FAIL`. Position: clicked
  block centre, `y + collision-shape max Y`. **Spacing**: any `GatewayEntity`
  inside the new entity's box inflated by `rules.spacing` → `FAIL`. Then lift up
  to 4 blocks while colliding; still colliding → `error.gateways.no_space` +
  `FAIL`. `addFreshEntity`, `onGateCreated()`, and the pearl is consumed unless
  the player is creative.
- **`entity/GatewayEntity.java` `tick`** — the state machine the port carries:
  wave active and `ticksActive > maxWaveTime` → fail `TIMER_ELAPSED`;
  `ticksActive++`; no nearby player (`getNearestPlayer(this, leashRange²)`)
  decrements a 200-tick timer → fail `NO_NEARBY_PLAYER`, a player resets it to
  200; enemies = tracked entities alive and not removed for an invalid reason;
  every 20 ticks an out-of-range enemy fails (`ENTITY_TOO_FAR`) or is re-placed;
  `ENEMIES` is the count; queued reward stacks drip out **3 + queued/100 every 4
  ticks**; a wave with no enemies left completes (`completeWave`, clear, wave+1,
  `ticksActive` 0); inactive and `ticksActive > setupTime` and not the last wave →
  `startNextWave`; inactive, last wave and nothing queued → `completeGateway`
  (final rewards, remove).
- **`entity/NormalGatewayEntity.java`** — `isLastWave` = `wave >= numWaves`;
  `completeWave` queues `getCurrentWave().spawnRewards(...)`; `isCompleted` =
  queue empty and last wave; `completeGateway` runs the gateway's `rewards`.
- **`gate/Wave.java`** — `spawnWave` spawns `count` of each wave entity (a failed
  spawn fails the gate with `SPAWN_FAILED`); `spawnRewards` **splits any stack
  over 4** into copies of 4. `spawnWaveEntity` sets the owner tag, snaps, applies
  the wave modifiers, `setHealth(max)`, gives 5 ticks of Resistance 100, targets
  the summoner, `setPersistenceRequired`, and (rules) `followRangeBoost` /
  `defaultDropChance`.
- **`gate/SpawnAlgorithms.java`** — `open_field` (the default): 15 tries, a
  triangular `(rand - rand)` offset within `gateway width/2 + rules.spawnRange`,
  `y = pos.y + rand(3*size.scale) + 1`, descend while the block below is air,
  ascend while the entity box collides, skip if outside the leash, accept when it
  fits.
- **`gate/GateRules.java`** — defaults `spawn_range` 8, `leash_range` 32,
  `allow_discarding` false, `allow_dim_change` false, `player_damage_only` false,
  `remove_mobs_on_failure` true, `fail_on_out_of_bounds` false, `spacing` 0,
  `follow_range_boost` 32, `default_drop_chance` 0, `requires_nearby_player`
  false, `lives` -1.
- **`gate/Gateway.java` `Size`** — SMALL 2×2 (scale 1), MEDIUM 4×4 (scale 2),
  LARGE 5.5×5.5 (scale 2.5).
- **`gate/Reward.java`** — `stack`, `stack_list`, `entity_loot` (kills a copy of
  the mob `rolls` times and collects its drops), `loot_table`, `chanced`,
  `command`, `experience`, `summon`, `counted`.
- **`command/GatewayCommand.java`** — `/open_gateway <pos> <type>` and
  `/open_gateway <entity> <type>`, permission level 2 (`GAMEMASTERS`).
- **`GatewayEvents.java`** — teleport clamp, conversion re-tracking, damage gate
  (`player_damage_only`), despawn deny, player death → `playerDied` (a life within
  `leash + 25`), spawn-cancel protection.

## The gateway the slice carries

`data/gateways/gateways/emerald_grove.json` — `size` medium, `color` `#056608`,
**4 waves**, one completion reward per animal family and an explosion failure:

| wave | entities | max_wave_time | setup_time | wave reward |
|---:|---|---:|---:|---|
| 1 | 5 × `minecraft:zombie` (+25 % max health) | 1000 | 200 | `stack` 16 × `minecraft:hay_block` |
| 2 | 5 × `minecraft:spider` (modifiers) | 1000 | 200 | `stack_list` 16 cactus + 16 sugar cane + 16 bamboo |
| 3 | 5 × zombie + 2 × spider | 1000 | 200 | `stack_list` 32 carrot/potato/apple/beetroot |
| 4 | 1 × zombie_villager "Necrotic Farmer" (gear set iron, `finalize_spawn` false) + 3 zombie + 2 spider | 1200 | 200 | `stack_list` 16 of each sapling |

completion: `summon` 6 cows, 6 chickens, 6 sheep, 6 pigs. failure: `explosion`
strength 2, fire, block damage.

Wave 1's reward is a **fixed stack**, so "the reward is granted once" is a
deterministic assertion (16 hay blocks), not a loot-table expectation.

## What the pin can and cannot express (probed before planning)

- **Can:** `use_item_on` (API 13) with the held stack and the clicked block;
  `level_tick_post` (API 23) with the level in place; `native_spawn_mob`,
  `native_mobs`, `native_mob`, `native_kill_mob`, `native_remove_mob` (API 37);
  `native_summon` (an item entity from NBT, or an animal); `native_tell`;
  `native_block_collision` / `native_block` (spawn placement); `commands` +
  `perform_command` (API 8/21); `content_root` (the AE2 embedding pattern).
- **Cannot:** register an entity type with a **custom tick body** (content entity
  types copy a template's class); read or write entity **attributes**
  (`max_health` etc.) — the pin has no attribute API, so the wave attribute
  modifiers are a recorded gap; give a custom item **property** for the size
  texture override.
