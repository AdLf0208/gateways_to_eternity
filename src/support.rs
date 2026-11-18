//! Game-side glue for Gateways to Eternity: the open gateways, the world reads the
//! state machine needs, and the effects it asks for.
//!
//! [`crate::runtime`] is the pure truth (the wave state machine, the carried table,
//! the spawn placement); this module is the only place that reads the game and
//! writes it back. The retired `.hnmod` kept the same split (`support.rs` plus one
//! per-hook helper); the migration's mapping is in `notes.md` under
//! `## Migration to the SDK format`.
//!
//! Upstream keeps each gateway in a `NormalGatewayEntity` that ticks itself. An SDK
//! mod cannot give a content entity type a tick body, so an open gateway is a record
//! here ([`Live`]), driven once per level tick from `level_tick_post` and started by
//! the pearl's item Part (`use_on`) or by `/gateways open`. The record is memory
//! only: a world reload closes every gateway (a recorded gap in `notes.md`).

use std::sync::{Mutex, MutexGuard, PoisonError};

use mod_api::{
    Command, CommandArgument, CommandArgumentValue, Compound, Flow, Host, HostSide,
    HostTransaction, Identifier, InteractionResult, ItemPart, ItemPlace, ItemResource, ItemStack,
    LevelRef, LevelTickPostEvent, PermissionLevel, Registrar, RemovalReason, RunModCommandEvent,
    ServerHost, Tag, Text, UseOnEvent,
};

use crate::runtime::{self, Effect, GATEWAY_KEY, Gate, Probe, Rng, TickView, XorShift};

/// The pearl item (`GatewayObjects.GATE_PEARL`); the Part is attached to it alone.
pub const PEARL: &str = "gateways:gate_pearl";

/// The component the pearl's recipe writes, and the key inside it that names the
/// gateway (upstream's `gateways:gateway` data component).
pub const CUSTOM_DATA: &str = "minecraft:custom_data";

/// The key a wave mob's name is written under, as `/summon` takes it.
pub const CUSTOM_NAME: &str = "CustomName";

/// A dropped reward waits this long before it may be picked up (`Wave`'s rewards are
/// ordinary item entities; upstream leaves the default 10-tick pickup delay).
pub const PICKUP_DELAY: i16 = 10;

/// The entity a reward stack is dropped as.
pub const ITEM_ENTITY: &str = "minecraft:item";

/// `/gateways open <x> <y> <z> <type>` — upstream's `/open_gateway <pos> <type>`
/// (`command/GatewayCommand.java`) at its `GAMEMASTERS` level.
pub const OPEN_USAGE: &str = "gateways open <x:number> <y:number> <z:number> <gateway:word>";

/// `/gateways list`.
pub const LIST_USAGE: &str = "gateways list";

/// What a `/gateways` line asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    /// `/gateways list`
    List,
    /// `/gateways open <x> <y> <z> <gateway>`
    Open,
}

/// The line an unrecognised subcommand answers with (`GatewayCommand`'s usage).
pub const USAGE_LINE: &str = "Usage: /gateways open <x> <y> <z> <type> | /gateways list";

/// The line a command below the usages' level answers with.
pub const PERMISSION_LINE: &str = "You do not have permission to open a Gateway";

/// The line for a dimension this server has not loaded (the 0.1.0 read the game's
/// current dimension and could not meet this case).
pub const NO_LEVEL_LINE: &str = "This Gateway cannot open in that dimension";

/// Why a pearl or `/gateways open` did not open a gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// Another gateway already occupies the box (`GatePearlItem.useOn`'s spacing
    /// check).
    Spacing,
    /// The gateway's box does not fit (`error.gateways.no_space`).
    NoSpace,
}

// ---------------------------------------------------------------------------
// The open gateways
// ---------------------------------------------------------------------------

/// An open gateway and the randomness its spawn placement draws from.
struct Live {
    gate: Gate,
    rng: XorShift,
}

/// The gateways open in this process. `Hooks` is `Send + Sync` and the record is
/// process-wide, so the list is a `Mutex`: a `RefCell` would not compile. A poisoned
/// lock is recovered rather than re-raised — the world has already seen the panic
/// that poisoned it, and aborting the game's tick is worse than carrying on with
/// the records that are still there.
fn live() -> MutexGuard<'static, Vec<Live>> {
    static LIVE: Mutex<Vec<Live>> = Mutex::new(Vec::new());
    LIVE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The level as the spawn placement sees it: collision shapes and the world's
/// lowest `y`.
struct LevelProbe<'a> {
    host: &'a dyn Host,
    level: LevelRef,
}

impl Probe for LevelProbe<'_> {
    fn empty(&self, pos: [i32; 3]) -> bool {
        let (x, y, z) = (f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]));
        // A read that fails (a stale level, a column this side does not hold) counts
        // as blocked, so a spawn is refused rather than placed inside a wall.
        !self
            .level
            .has_collisions(self.host, [x, y, z, x + 1.0, y + 1.0, z + 1.0])
            .unwrap_or(true)
    }

    fn min_y(&self) -> i32 {
        self.level.height(self.host).map_or(-64, |(min, _)| min)
    }
}

// ---------------------------------------------------------------------------
// The item Part: a gate pearl opens its gateway on the clicked block
// ---------------------------------------------------------------------------

/// The pearl's Part (`GatePearlItem`): attached to `gateways:gate_pearl` alone, so no
/// hook has to test the item id.
pub struct Pearl;

#[mod_api::horizon_part]
impl ItemPart for Pearl {
    /// `ItemStack.useOn` → `GatePearlItem.useOn`: once the clicked block let the click
    /// through, open the gateway the pearl names.
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        pearl_use_on(host, event)
    }
}

/// A bound pearl names its gateway; an unbound one is the errored pearl upstream
/// refuses (`text.gateways.errored_gate_pearl`).
fn gateway_of(stack: &ItemStack) -> Option<String> {
    let Some(Tag::Compound(custom)) = stack
        .components()
        .and_then(|data| tag(data.as_nbt(), CUSTOM_DATA))
    else {
        return None;
    };
    match tag(custom, GATEWAY_KEY) {
        Some(Tag::String(id)) => Some(id.to_string_lossy()),
        _ => None,
    }
}

fn pearl_use_on(host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
    let Some(id) = gateway_of(event.stack) else {
        return Some(InteractionResult::Fail);
    };
    let HostSide::Server(host) = host else {
        // Upstream answers SUCCESS on the client for a bound pearl; only the server
        // decides whether the gateway opens.
        return Some(InteractionResult::Success);
    };
    let player = host.player(event.player)?;
    let level = player.level(&*host)?;
    let dimension = level.dimension().id(&*host)?;
    let Some(def) = runtime::gateway(&id) else {
        say(host, event.player, format!("Unknown Gateway: {id}"));
        return Some(InteractionResult::Fail);
    };
    // `GatePearlItem.useOn` places the entity on the clicked block's collision shape,
    // then lifts it while the box collides.
    let top = block_top(&*host, level, event.pos);
    let placed = {
        let probe = LevelProbe {
            host: &*host,
            level,
        };
        runtime::placement(event.pos, top, def.size, &probe)
    };
    let Some(at) = placed else {
        say(host, event.player, open_error_line(OpenError::NoSpace));
        return Some(InteractionResult::Fail);
    };
    match open_at(&*host, level, &dimension, at, def, event.player) {
        Ok(()) => {
            consume(host, event);
            Some(InteractionResult::Success)
        }
        // Upstream's spacing answer is FAIL: nothing opens, nothing is spent.
        Err(error) => {
            if error == OpenError::NoSpace {
                say(host, event.player, open_error_line(error));
            }
            Some(InteractionResult::Fail)
        }
    }
}

/// Take one pearl from the clicking hand (`stack.shrink(1)`, unless the player is
/// creative — `ItemPlace::extract` gives without losing any for a creative player).
fn consume(host: &mut dyn ServerHost, event: &UseOnEvent<'_>) {
    let place = ItemPlace::PlayerHand {
        player: event.player,
        hand: event.hand,
    };
    let resource = ItemResource::of(event.stack);
    let _ = host.transaction(|host, tx| {
        place.extract(host, tx, &resource, 1);
        Ok::<(), ()>(())
    });
}

/// The height of the clicked block's collision shapes above its own `y`.
fn block_top(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> f64 {
    let (x, y, z) = (f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]));
    let floor = f64::from(pos[1]);
    level
        .collision_boxes(host, [x, y, z, x + 1.0, y + 1.0, z + 1.0])
        .unwrap_or_default()
        .iter()
        .map(|box_| box_[4])
        .fold(floor, f64::max)
        - floor
}

/// Open the gateway `def` at `pos` (its feet) for `summoner`.
fn open_at(
    host: &dyn Host,
    level: LevelRef,
    dimension: &str,
    pos: [f64; 3],
    def: runtime::Gateway,
    summoner: u128,
) -> Result<(), OpenError> {
    {
        let guard = live();
        let clashes = guard.iter().any(|open| {
            open.gate.dimension() == dimension
                && runtime::overlaps(
                    pos,
                    def.size,
                    open.gate.pos(),
                    open.gate.def().size,
                    def.rules.spacing,
                )
        });
        if clashes {
            return Err(OpenError::Spacing);
        }
    }
    let clear = {
        let probe = LevelProbe { host, level };
        runtime::box_is_clear(&probe, pos, def.size)
    };
    if !clear {
        return Err(OpenError::NoSpace);
    }
    let seed = pos[0].to_bits() ^ (pos[2].to_bits() << 32) ^ summoner as u64;
    live().push(Live {
        gate: Gate::new(def, dimension, pos, summoner),
        rng: XorShift::seeded(seed),
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// The level tick: every open gateway in that level
// ---------------------------------------------------------------------------

/// After a level's tick, with the level in place: advance every gateway open in it.
pub fn level_tick(host: HostSide<'_>, event: &mut LevelTickPostEvent) {
    let HostSide::Server(host) = host else {
        return;
    };
    tick_level(host, event.level);
}

fn tick_level(host: &mut dyn ServerHost, level: LevelRef) {
    let Some(dimension) = level.dimension().id(&*host) else {
        return;
    };
    let dimension = dimension.as_str();
    let mut guard = live();
    if guard.is_empty() {
        return;
    }
    let mut closed = Vec::new();
    for index in 0..guard.len() {
        if guard[index].gate.dimension() != dimension {
            continue;
        }
        let gate_pos = guard[index].gate.pos();
        let leash_sq = guard[index].gate.def().rules.leash_range_sq();
        let nearby = level.online_players(host).is_some_and(|players| {
            players.iter().any(|player| {
                player
                    .entity()
                    .position(host)
                    .is_some_and(|pos| distance_sq(pos, gate_pos) <= leash_sq)
            })
        });
        let effects = {
            let distance = |id: u32| {
                host.entity(id)
                    .and_then(|entity| entity.position(host))
                    .map(|pos| distance_sq(pos, gate_pos))
            };
            let view = TickView {
                nearby_player: nearby,
                distance_sq: &distance,
            };
            let probe = LevelProbe {
                host: &*host,
                level,
            };
            let open = &mut guard[index];
            open.gate.tick(&view, &mut open.rng, &probe)
        };
        let mut spawned = Vec::new();
        for effect in effects {
            apply(host, dimension, &mut guard[index], effect, &mut spawned);
        }
        if !spawned.is_empty() {
            guard[index].gate.add_spawned(spawned);
        }
        if guard[index].gate.is_finished() {
            closed.push(index);
        }
    }
    for index in closed.into_iter().rev() {
        guard.remove(index);
    }
}

/// Do one effect. `spawned` collects the ids the wave now tracks.
fn apply(
    host: &mut dyn ServerHost,
    dimension: &str,
    open: &mut Live,
    effect: Effect,
    spawned: &mut Vec<u32>,
) {
    let pos = open.gate.pos();
    let summoner = open.gate.summoner();
    match effect {
        Effect::Spawn {
            kind,
            name,
            pos: at,
        } => match host.summon(dimension, kind, at, name_nbt(name)) {
            Some(id) => spawned.push(id),
            None => {
                if open.gate.spawn_failed() {
                    say(host, summoner, runtime::FailureReason::SpawnFailed.text());
                }
            }
        },
        Effect::Drop { item, count } => {
            let Some(item_nbt) = item_stack(host, item, count).and_then(|stack| stack.to_nbt(host))
            else {
                return;
            };
            let x = pos[0] + (open.rng.next_f64() - 0.5);
            let z = pos[2] + (open.rng.next_f64() - 0.5);
            host.summon(
                dimension,
                ITEM_ENTITY,
                [x, pos[1] + 1.5, z],
                drop_tag(item_nbt),
            );
        }
        Effect::Summon { kind, pos } => {
            host.summon(dimension, kind, pos, Compound::new());
        }
        Effect::Move { id, pos } => {
            if let Some(entity) = host.entity(id) {
                entity.teleport_to(host, pos, None);
            }
        }
        Effect::Failed(reason) => {
            say(host, summoner, reason.text());
            if open.gate.def().rules.remove_on_failure {
                for id in open.gate.tracked().to_vec() {
                    if let Some(entity) = host.entity(id) {
                        entity.remove(host, RemovalReason::Discarded);
                    }
                }
            }
        }
        Effect::Completed => {}
    }
}

fn distance_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// One line to a player; `tell`'s `bool` is not interesting here.
fn say(host: &mut dyn ServerHost, player: u128, line: impl Into<String>) {
    let Some(target) = host.player(player) else {
        return;
    };
    let _ = target.tell(host, Text::literal(line.into()));
}

/// A command's answer, in the source's feedback (what `/gateways list` prints to the
/// player or the console that ran it).
fn note(host: &mut dyn ServerHost, line: impl Into<String>) {
    let _ = host.commands().feedback(Text::literal(line.into()));
}

// ---------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------

/// `/gateways …` in the server's command tree, at upstream's `GAMEMASTERS` level.
pub fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::op(OPEN_USAGE, 2));
    commands.push(Command::op(LIST_USAGE, 2));
}

/// Run a parsed `/gateways` line, or leave it alone.
pub fn run_command(host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
    let Some(sub) = subcommand(&event.command.literals) else {
        return Flow::Continue;
    };
    if event.source.permission() < PermissionLevel::Gamemasters {
        note(host, PERMISSION_LINE);
        return Flow::Cancel;
    }
    match sub {
        Sub::List => note(host, list_line()),
        Sub::Open => open_command(host, event),
    }
    Flow::Cancel
}

/// `/gateways open <x> <y> <z> <type>` — upstream's `/open_gateway <pos> <type>`.
fn open_command(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) {
    let Some((at, id)) = open_arguments(&event.command.arguments) else {
        note(host, USAGE_LINE);
        return;
    };
    let Some(def) = runtime::gateway(id) else {
        note(host, format!("Unknown Gateway: {id}"));
        return;
    };
    let summoner = match &event.source {
        mod_api::CommandSource::Player { player, .. } => player.uuid(),
        _ => 0,
    };
    let Some(dimension) = event.command.dimension.id(&*host) else {
        note(host, NO_LEVEL_LINE);
        return;
    };
    let Some(level) = host.level(&dimension) else {
        note(host, NO_LEVEL_LINE);
        return;
    };
    if let Err(error) = open_at(&*host, level, &dimension, at, def, summoner) {
        note(host, open_error_line(error));
    }
}

/// The subcommand a parsed literal path names, or `None` for another command.
pub fn subcommand(literals: &[String]) -> Option<Sub> {
    if literals.first().map(String::as_str) != Some("gateways") {
        return None;
    }
    match literals.get(1).map(String::as_str) {
        Some("open") => Some(Sub::Open),
        Some("list") => Some(Sub::List),
        _ => None,
    }
}

/// The position and the gateway id of `/gateways open`'s arguments, or `None` when
/// one is missing or mistyped.
pub fn open_arguments(arguments: &[CommandArgument]) -> Option<([f64; 3], &str)> {
    let number = |name: &str| match argument(arguments, name) {
        Some(CommandArgumentValue::Number(value)) => Some(*value),
        _ => None,
    };
    let Some(CommandArgumentValue::String(gateway)) = argument(arguments, "gateway") else {
        return None;
    };
    Some(([number("x")?, number("y")?, number("z")?], gateway.as_str()))
}

fn argument<'a>(arguments: &'a [CommandArgument], name: &str) -> Option<&'a CommandArgumentValue> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
}

/// What `/gateways list` prints (`Carried gateways: …`).
pub fn list_line() -> String {
    format!("Carried gateways: {}", runtime::EMERALD_GROVE)
}

/// The summon compound for one wave mob: its name when it has one, else nothing.
pub fn name_nbt(name: Option<&str>) -> Compound {
    let mut nbt = Compound::new();
    if let Some(name) = name {
        nbt.insert(CUSTOM_NAME.into(), Tag::String(name.into()));
    }
    nbt
}

/// The summon compound of one dropped reward stack.
pub fn drop_tag(item: Compound) -> Compound {
    let mut tag = Compound::new();
    tag.insert("Item".into(), Tag::Compound(item));
    tag.insert("PickupDelay".into(), Tag::Short(PICKUP_DELAY));
    tag
}

/// The line an [`OpenError`] answers with (`error.gateways.*`).
pub fn open_error_line(error: OpenError) -> &'static str {
    match error {
        OpenError::Spacing => "Another Gateway is too close",
        OpenError::NoSpace => "There's not enough space to open this Gateway",
    }
}

/// One field of a compound. `Compound` is a `BTreeMap<JavaString, Tag>` and
/// `JavaString` is not part of the mod surface, so a lookup by `&str` goes through the
/// UTF-16 keys (what `Tag::get` does).
fn tag<'a>(compound: &'a Compound, key: &str) -> Option<&'a Tag> {
    compound
        .iter()
        .find(|(name, _)| name.to_string_lossy() == key)
        .map(|(_, value)| value)
}

/// The item `id` names as a stack of `count`, or `None` for an id this build does not
/// know or a count outside a stack.
fn item_stack(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    let key = host.item_key(&Identifier::parse(id).ok()?)?;
    ItemStack::new(key, count)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Attach the pearl's Part. The key is bound first: `r.item_part(r.item_key(..), ..)`
/// borrows the registrar twice.
pub fn declare(r: &mut dyn Registrar) {
    let pearl = r.item_key(PEARL);
    r.item_part(pearl, Pearl);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argument(name: &str, value: CommandArgumentValue) -> CommandArgument {
        CommandArgument::new(name.to_owned(), value)
    }

    fn path(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn the_literal_path_names_its_subcommand() {
        assert_eq!(subcommand(&path(&["gateways", "list"])), Some(Sub::List));
        assert_eq!(subcommand(&path(&["gateways", "open"])), Some(Sub::Open));
        assert_eq!(subcommand(&path(&["gateways"])), None);
        assert_eq!(subcommand(&path(&["other", "open"])), None);
        assert_eq!(subcommand(&path(&["gateways", "frobnicate"])), None);
    }

    #[test]
    fn the_open_arguments_are_three_numbers_and_a_gateway() {
        let arguments = vec![
            argument("x", CommandArgumentValue::Number(0.5)),
            argument("y", CommandArgumentValue::Number(100.0)),
            argument("z", CommandArgumentValue::Number(6.5)),
            argument(
                "gateway",
                CommandArgumentValue::String(runtime::EMERALD_GROVE.to_owned()),
            ),
        ];
        assert_eq!(
            open_arguments(&arguments),
            Some(([0.5, 100.0, 6.5], runtime::EMERALD_GROVE))
        );
    }

    #[test]
    fn a_missing_or_mistyped_open_argument_is_refused() {
        let two = vec![
            argument("x", CommandArgumentValue::Number(1.0)),
            argument("y", CommandArgumentValue::Number(2.0)),
        ];
        assert_eq!(open_arguments(&two), None);
        let mistyped = vec![
            argument("x", CommandArgumentValue::Integer(1)),
            argument("y", CommandArgumentValue::Number(2.0)),
            argument("z", CommandArgumentValue::Number(3.0)),
            argument("gateway", CommandArgumentValue::Number(4.0)),
        ];
        assert_eq!(open_arguments(&mistyped), None);
    }

    #[test]
    fn the_list_line_names_the_carried_gateway() {
        assert_eq!(
            list_line(),
            format!("Carried gateways: {}", runtime::EMERALD_GROVE)
        );
    }

    #[test]
    fn a_named_wave_mob_carries_its_name_and_an_unnamed_one_nothing() {
        let named = name_nbt(Some("Necrotic Farmer"));
        assert_eq!(
            tag(&named, CUSTOM_NAME),
            Some(&Tag::String("Necrotic Farmer".into()))
        );
        assert_eq!(tag(&name_nbt(None), CUSTOM_NAME), None);
    }

    #[test]
    fn a_reward_drop_is_an_item_entity_with_a_pickup_delay() {
        let dropped = drop_tag(Compound::new());
        assert_eq!(
            tag(&dropped, "PickupDelay"),
            Some(&Tag::Short(PICKUP_DELAY))
        );
        assert_eq!(tag(&dropped, "Item"), Some(&Tag::Compound(Compound::new())));
    }

    #[test]
    fn an_open_refusal_has_the_sources_line() {
        assert_eq!(
            open_error_line(OpenError::NoSpace),
            "There's not enough space to open this Gateway"
        );
        assert_eq!(
            open_error_line(OpenError::Spacing),
            "Another Gateway is too close"
        );
    }
}
