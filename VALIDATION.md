# Validation — Gateways to Eternity

## 0.2.0 — the SDK package (2026-11-18)

Validated on Linux with rustc 1.97.0 / cargo 1.97.0 against SDK nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (`package_format` 6, `content_api`
"0.2") and mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, for Minecraft 26.3.
Every row is a run recorded under
[`ports/_shared/evidence/gateways/`](../_shared/evidence/gateways/) (`sdk-*`).

| Check | Command | Result |
|---|---|---|
| runtime alone, before any glue | `cargo test --offline` | **18 passed**, 0 failed — `sdk-test-runtime.log` |
| glue red first | `cargo test --offline` (bodies `todo!()`) | **7 failed** on `not yet implemented`, 18 passed, exit 101 — `sdk-test-red.log` |
| tests | `cargo test --offline` | **25 passed**, 0 failed (18 `runtime::`, 7 `support::`) — `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings — `sdk-clippy.log` |
| format | `cargo fmt --check` | exit 0, empty diff — `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/gateways.hmod`, exit 0 — `sdk-build.log` |
| registry check | `cargo horizon-mod check gateways.hmod` | `gateways 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`, **33,298 B**, sha256 `9478db99…67199`, exit 0 — `sdk-check.log` |
| fits the pinned build | `cargo horizon-mod fits gateways.hmod --build nightly` | `0b14f8a  made for`, exit 0 — `sdk-fits.log` |
| reproducible | a second `cargo horizon-mod build` | 33,298 B and `9478db99…67199` both times, byte-identical — `sdk-build-second.log`, `sdk-sha256.txt` |
| from a cleaned tree | `target/` removed, then `cargo test` + `cargo horizon-mod build` | 25 passed; the package rebuilt to the same 33,298 B and `9478db99…67199` — `sdk-test.log`, `sdk-build.log` |
| `runtime.rs` untouched | `unzip -p gateways.hmod src/runtime.rs \| sha256sum` | `95ba3e97…fca6c` = `src/runtime.rs` = the retired file — `sdk-sha256.txt` |
| content carry | `diff -r` against the retired `content/assets` + `content/data` | 0 differences, 7 files — `sdk-content-carry.log` |
| package entries | `unzip -l gateways.hmod` | **16 entries**: manifest, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `src/{lib,runtime,support}.rs`, `uses.json`, 7 content files; **no `Cargo.lock`** — `sdk-entries.log` |
| what it uses | `unzip -p gateways.hmod uses.json` | 217 surface entries (14 hook, 20 method, 2 part, 1 capability, 2 macro, 18 trait, 98 type, 62 handle) — `sdk-uses.json`, `sdk-uses-summary.txt` |
| licence | the jar's `META-INF/neoforge.mods.toml` | `license="MIT License"` → rank 0 — `sdk-licence-read.log` |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/gateways --strict` | `Skills line present`, **PASS** |

### Not re-run in this migration

- **The in-game e2e.** `horizon-launcher` is not installed, so there is no game built with
  this package to point `GAME=` at. `tools/e2e.sh` is kept with the same scenario and
  assertions, now requiring `GAME=` (and optionally `CONTROL_GAME=`); no 0.2.0 in-game
  result is claimed. The 0.1.0 runs below stand as the behaviour's evidence for a
  `src/runtime.rs` that is byte-identical.
- **The staged ladder** (`tools/horizon.py stage/check/test/build`): those tools are
  retired with the `.hnmod` format and live in
  `evidence/gateways/retired-hnmod-0.1.0/tools/`.

## 0.1.0 — the retired `.hnmod` (historical, 2026-11-13)

Validated against Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (`game.rs`
`64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f`) and the 26.3
prepared resources at `/home/adlf/Desktop/horizon/.local/horizon-client/26.3`.

| Check | Result |
|---|---|
| `cargo fmt -- --check` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| `cargo test --locked --offline` | `18 passed; 0 failed` |
| `python3 tools/horizon.py stage` | pinned export + plan + apply, exit 0 |
| `python3 tools/horizon.py check` | `cargo_exit: 0`, 0 warnings |
| `python3 tools/horizon.py test` | `test result: ok. 18 passed; 0 failed` (`game::gateways::tests`) |
| `python3 tools/horizon.py build` | `cargo_exit: 0`, fast profile, 0 warnings |
| `tools/package.sh` (content + bundle) | `content/gateways.hmod` 5,513 B; `gateways.hnmod` 25,682 B, 13 entries |
| `cargo horizon-mod check gateways.hnmod --kind native-mod` | exit 0 |
| content check | `no code: content only, nothing to run` |
| `tools/e2e.sh` (embedded, no `--dev-mod`, unusable-`.horizon-cache` control) | **PASS**, `EXIT:0` — `evidence/gateways/e2e-default.log` |
| `SELFTEST=1 tools/e2e.sh` | **FAIL** (`the wave paid 16 hay blocks`), `EXIT:1` — `evidence/gateways/e2e-selftest.log` |
| `python3 ports/tools/check-e2e-logs.py gateways --strict` | `logs=2 ok=2`, exit 0 |

### What the 0.1.0 default leg proves (and how it could fail)

The scenario runs in The End on a built 17×17 stone floor, headless, creative:

1. the emerald-grove recipe is registered (`/recipe give` → "Unlocked 1 recipe(s)"), and
   in the **control** launch — the same binary with `.horizon-cache` made unusable as a
   plain file — the same command answers "Unknown recipe", so the content provably came
   from the bundle;
2. `ctl interact` with a pearl carrying `custom_data={gateway:"gateways:emerald_grove"}`
   opens the gateway; a creative player keeps the pearl (upstream's `isCreative` branch);
3. a second use while it is open opens nothing: after the first wave's setup there are
   **five** zombies, not ten;
4. the first 192 ticks spawn nothing (wave 1's `setup_time` is 200);
5. 100 ticks with the five zombies alive pay nothing (0 hay);
6. killing them pays exactly **16 hay blocks**, dropped from the gateway;
7. 60 ticks later the total is still 16 — the wave paid once;
8. wave 2's **five spiders** spawn after its own 200-tick setup;
9. `/gateways list` names the carried gateway and `/gateways open` opens a second one
   (five more zombies after its setup), with the first gateway's spiders undisturbed.

`SELFTEST=1` inverts step 6 (it expects no reward) and the run ends
`FAIL: SELFTEST: the wave paid 16 hay blocks…` with `EXIT:1`, so the reward assertion is
not vacuous. The unit suite was proved load-bearing the same way: removing
`self.active = false` on wave completion fails four tests including
`the_reward_is_granted_once`, and paying a wave with living mobs fails
`an_unkilled_wave_never_pays`; both mutations were reverted and the suite re-ran green.
