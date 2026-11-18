//! Gateways to Eternity for Horizon — the one `impl Hooks` at the crate root.
//!
//! Ported from Gateways to Eternity 5.1.0 (`Shadows-of-Fire/GatewaysToEternity`, MIT).
//! [`runtime`] is the pure truth — the wave state machine, the carried
//! `gateways:emerald_grove` table, the spawn placement and the reward split, with its
//! tests — and is byte-identical to the retired `.hnmod` port's file. [`support`] is
//! the glue: the open gateways, the level tick, the pearl's item Part and
//! `/gateways`. This file is the one `impl Hooks`.
//!
//! # Where the retired container's hooks went
//!
//! The 2026-11-17 mod-API merge (#16941, launcher ≥ 0.2.70) retired `.hnmod`: the plan,
//! the injection points and the per-hook helper files are gone. A hook is a typed
//! method here, and a moment that belongs to one item is a **Part** attached to it.
//!
//! | `.hnmod` hook (API) | here |
//! |---|---|
//! | the `content_root` injection (AE2 pattern) | gone: one flat `.hmod` carries `assets/` and `data/` itself |
//! | `use_item_on` (13) | `support::Pearl` (`ItemPart::use_on`), attached to `gateways:gate_pearl` |
//! | `level_tick_post` (23) | [`Mod::level_tick_post`] → `support::level_tick` |
//! | `register_commands` (8) | [`Mod::register_commands`] → `support::declare_commands` |
//! | `perform_command` (8) | [`Mod::run_mod_command`] (typed arguments, not a raw line) |
//! | `commands` (8, the client chat tree) | gone: `register_commands` gives every player's chat box the same completion |
//! | `register` (none) | [`Mod::register`]: the pearl's Part |

pub mod runtime;
pub mod support;

use mod_api::{
    self, Flow, Hooks, HostSide, LevelTickPostEvent, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost,
};

/// The mod keeps no state of its own: the open gateways live in `support`'s list, so
/// one record serves every dimension.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Attach the pearl's Part: its click is the item's moment, not a global hook.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        support::declare(r);
    }

    /// `/gateways open <x> <y> <z> <type>` and `/gateways list`, at level 2
    /// (`GAMEMASTERS`, upstream's `/open_gateway`).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::declare_commands(event.commands);
    }

    /// Run one of the commands above and take it.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::run_command(host, event)
    }

    /// After a level's tick: advance every gateway open in that level (the level is in
    /// place, which the spawn, teleport and removal helpers need).
    fn level_tick_post(&self, host: HostSide<'_>, event: &mut LevelTickPostEvent) {
        support::level_tick(host, event);
    }
}
