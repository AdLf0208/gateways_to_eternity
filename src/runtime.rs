// Gateways to Eternity: the normal gateway's wave state machine.
//
// Upstream (`Shadows-of-Fire/GatewaysToEternity`, 26.1.2 source) keeps the
// gateway in a `NormalGatewayEntity` that ticks itself. A content mod at this
// pin cannot give an entity type a tick body, so the same machine runs here as
// pure state driven once per level tick, and the glue (`support.rs`) spawns,
// drops and messages.
//
// Every number below is carried from the jar's own data files
// (`data/gateways/gateways/emerald_grove.json`) or from the upstream classes
// named beside it (`entity/GatewayEntity.java` `tick`, `entity/NormalGatewayEntity.java`,
// `gate/Wave.java`, `gate/SpawnAlgorithms.java`, `gate/GateRules.java`).

use std::collections::VecDeque;

/// The one gateway this slice carries (`data/gateways/gateways/emerald_grove.json`).
pub const EMERALD_GROVE: &str = "gateways:emerald_grove";

/// `minecraft:custom_data` key on the pearl that names its gateway
/// (upstream's `gateways:gateway` data component, `GatewayObjects.GATEWAY_COMPONENT`).
pub const GATEWAY_KEY: &str = "gateway";

/// Upstream's `Gateway.Size`: the render scale and the entity's box.
// The injected module is not a library, so the sizes this slice does not use are
// dead there while a `pub` item in the standalone crate is reachable: `allow`,
// not `expect`, is the one that holds in both builds.
#[allow(
    dead_code,
    reason = "upstream's Size has three sizes; this slice carries the medium gateway"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    /// `Gateway.Size` in the upstream source: `EntityDimensions.fixed(width, height)`.
    #[must_use]
    pub fn dims(self) -> (f64, f64) {
        match self {
            Size::Small => (2.0, 2.0),
            Size::Medium => (4.0, 4.0),
            Size::Large => (5.5, 5.5),
        }
    }

    /// `Gateway.Size.getScale`, used by the spawn algorithm's `y` offset.
    #[must_use]
    pub fn scale(self) -> f64 {
        match self {
            Size::Small => 1.0,
            Size::Medium => 2.0,
            Size::Large => 2.5,
        }
    }
}

/// `gate/GateRules.java`: the rules a gateway runs under. Defaults are the
/// codec's `optionalFieldOf` values, which is what a definition without a
/// `rules` object gets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    pub spawn_range: f64,
    pub leash_range: f64,
    pub spacing: f64,
    pub remove_on_failure: bool,
    pub fail_on_out_of_bounds: bool,
    pub lives: i32,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            spawn_range: 8.0,
            leash_range: 32.0,
            spacing: 0.0,
            remove_on_failure: true,
            fail_on_out_of_bounds: false,
            lives: -1,
        }
    }
}

impl Rules {
    /// `Gateway.getLeashRangeSq`.
    #[must_use]
    pub fn leash_range_sq(self) -> f64 {
        self.leash_range * self.leash_range
    }
}

/// One `entities` entry of a wave (`gate/StandardWaveEntity.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct WaveEntity {
    pub kind: &'static str,
    pub count: u32,
    /// A name over the mob (upstream's `desc` / `nbt.CustomName`).
    pub name: Option<&'static str>,
}

/// One reward (`gate/Reward.java`), limited to the kinds this slice carries.
#[derive(Clone, Debug, PartialEq)]
pub enum Reward {
    /// `gateways:stack` / `gateways:stack_list`: item stacks, split into
    /// stacks of at most four by `Wave.spawnRewards`.
    Stack { item: &'static str, count: u32 },
    /// `gateways:summon`: mobs spawned when the gateway completes.
    Summon { kind: &'static str, count: u32 },
}

/// One wave (`gate/Wave.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct Wave {
    pub entities: Vec<WaveEntity>,
    pub rewards: Vec<Reward>,
    /// Ticks the player has to finish the wave before it fails.
    pub max_wave_time: u32,
    /// Ticks between the previous wave's end and this wave's start.
    pub setup_time: u32,
}

/// A gateway definition (`gate/normal/NormalGateway.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct Gateway {
    pub id: &'static str,
    pub size: Size,
    pub rules: Rules,
    pub waves: Vec<Wave>,
    /// Rewards paid once the last wave is done and its loot has been dropped.
    pub rewards: Vec<Reward>,
}

/// Why a gateway imploded (`GatewayEntity.FailureReason`), with upstream's lang
/// key. `NoSpace` is the pearl's placement refusal, not a failure of an open gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    SpawnFailed,
    TooFar,
    TimerElapsed,
    NoNearbyPlayer,
}

impl FailureReason {
    /// The line upstream sends to the summoner (`error.gateways.*` in
    /// `assets/gateways/lang/en_us.json`): the port has no chat-component
    /// translation, so the English text is sent.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            FailureReason::SpawnFailed => {
                "The Gateway imploded because there was not enough space to spawn the next wave"
            }
            FailureReason::TooFar => "The Gateway imploded because a wave entity ran too far away",
            FailureReason::TimerElapsed => "Wave Timer Expired",
            FailureReason::NoNearbyPlayer => {
                "The Gateway imploded because there was no player nearby"
            }
        }
    }
}

/// A source of randomness, so the spawn placement is testable without a world.
pub trait Rng {
    /// `[0, 1)`.
    fn next_f64(&mut self) -> f64;
    /// `[0, bound)`; zero when `bound` is not positive.
    fn next_below(&mut self, bound: i32) -> i32;
}

/// What the level says about a block position, for the spawn placement.
pub trait Probe {
    /// No block collision at this cell (air, a fluid, grass...).
    fn empty(&self, pos: [i32; 3]) -> bool;
    /// The lowest `y` of the level.
    fn min_y(&self) -> i32;
}

/// A small deterministic `Rng` for the glue: xorshift64*, seeded per gateway.
#[derive(Clone, Debug)]
pub struct XorShift(pub u64);

impl XorShift {
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        XorShift(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

impl Rng for XorShift {
    fn next_f64(&mut self) -> f64 {
        // 53 bits of mantissa, as `java.util.Random.nextDouble` takes.
        ((self.next_u64() >> 11) as f64) * (1.0 / (1u64 << 53) as f64)
    }

    fn next_below(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as i32
    }
}

/// What one tick sees of the world.
pub struct TickView<'a> {
    /// `Level.getNearestPlayer(this, leashRangeSq) != null`.
    pub nearby_player: bool,
    /// Distance squared to tracked mob `id`, or `None` once it is gone.
    pub distance_sq: &'a dyn Fn(u32) -> Option<f64>,
}

/// What the glue does after a tick.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Spawn one mob of a wave at `pos`.
    Spawn {
        kind: &'static str,
        name: Option<&'static str>,
        pos: [f64; 3],
    },
    /// Drop one reward stack from the gateway.
    Drop { item: &'static str, count: u32 },
    /// Summon a completion-reward mob at `pos`.
    Summon { kind: &'static str, pos: [f64; 3] },
    /// Move an out-of-leash mob back near the gateway.
    Move { id: u32, pos: [f64; 3] },
    /// The gateway imploded; the glue removes the tracked mobs and says why.
    Failed(FailureReason),
    /// The last wave is done and paid: the gateway closes.
    Completed,
}

/// The live state of one open gateway.
#[derive(Clone, Debug)]
pub struct Gate {
    def: Gateway,
    dimension: String,
    pos: [f64; 3],
    summoner: u128,
    /// Ticks since the gateway opened (`GatewayEntity.tickCount`), for the
    /// periodic checks.
    age: u64,
    wave: usize,
    active: bool,
    ticks_active: u32,
    nearby_timer: u32,
    tracked: Vec<u32>,
    queued: VecDeque<(&'static str, u32)>,
    finished: bool,
}

impl Gate {
    /// A freshly opened gateway at `pos` (its feet), summoning player `summoner`.
    #[must_use]
    pub fn new(def: Gateway, dimension: &str, pos: [f64; 3], summoner: u128) -> Self {
        Gate {
            def,
            dimension: dimension.to_owned(),
            pos,
            summoner,
            age: 0,
            wave: 0,
            active: false,
            ticks_active: 0,
            nearby_timer: 200,
            tracked: Vec::new(),
            queued: VecDeque::new(),
            finished: false,
        }
    }

    #[must_use]
    pub fn def(&self) -> &Gateway {
        &self.def
    }

    #[must_use]
    pub fn dimension(&self) -> &str {
        &self.dimension
    }

    #[must_use]
    pub fn pos(&self) -> [f64; 3] {
        self.pos
    }

    #[must_use]
    pub fn summoner(&self) -> u128 {
        self.summoner
    }

    /// The wave being fought, or the number of waves once all are done
    /// (`GatewayEntity.getWave`).
    #[cfg(test)]
    #[must_use]
    pub fn wave(&self) -> usize {
        self.wave
    }

    #[cfg(test)]
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The gateway has closed: completed or failed.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The mobs of the wave being fought.
    #[must_use]
    pub fn tracked(&self) -> &[u32] {
        &self.tracked
    }

    /// How many reward stacks are still waiting to drop.
    #[cfg(test)]
    #[must_use]
    pub fn queued(&self) -> usize {
        self.queued.len()
    }

    /// The ids the glue spawned for this wave.
    pub fn add_spawned(&mut self, ids: impl IntoIterator<Item = u32>) {
        self.tracked.extend(ids);
    }

    /// The glue could not spawn a wave mob: upstream fails the gate at once.
    /// True when this is the failure, so the glue reports it once.
    pub fn spawn_failed(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.finished = true;
        true
    }

    /// One server tick. `rng` places spawned mobs, `probe` reads the level.
    pub fn tick(&mut self, view: &TickView, rng: &mut dyn Rng, probe: &dyn Probe) -> Vec<Effect> {
        let mut out = Vec::new();
        if self.finished {
            return out;
        }
        self.age = self.age.saturating_add(1);

        // `GatewayEntity.tick`: an active wave past its timer fails.
        if self.active && self.ticks_active > self.max_wave_time() {
            return fail(&mut self.finished, FailureReason::TimerElapsed);
        }
        self.ticks_active = self.ticks_active.saturating_add(1);

        // No nearby player counts down a 200-tick grace period.
        if view.nearby_player {
            self.nearby_timer = 200;
        } else {
            self.nearby_timer = self.nearby_timer.saturating_sub(1);
            if self.nearby_timer == 0 {
                return fail(&mut self.finished, FailureReason::NoNearbyPlayer);
            }
        }

        // Mobs that are gone no longer hold the wave open.
        self.tracked.retain(|id| (view.distance_sq)(*id).is_some());

        // Every 20 ticks an out-of-leash mob fails the gate or is re-placed.
        if self.age.is_multiple_of(20) {
            let leash_sq = self.def.rules.leash_range_sq();
            let far: Vec<u32> = self
                .tracked
                .iter()
                .copied()
                .filter(|id| (view.distance_sq)(*id).is_some_and(|d| d > leash_sq))
                .collect();
            for id in far {
                if self.def.rules.fail_on_out_of_bounds {
                    return fail(&mut self.finished, FailureReason::TooFar);
                }
                let Some(pos) = self.spawn_position(rng, probe) else {
                    return fail(&mut self.finished, FailureReason::TooFar);
                };
                out.push(Effect::Move { id, pos });
            }
        }

        // Queued reward stacks drip out, three per fourth tick plus one per
        // hundred still queued (`GatewayEntity.getDropCount`).
        if self.age.is_multiple_of(4) && !self.queued.is_empty() {
            let per = 3 + self.queued.len() / 100;
            for _ in 0..per {
                let Some((item, count)) = self.queued.pop_front() else {
                    break;
                };
                out.push(Effect::Drop { item, count });
            }
        }

        if self.active {
            if self.tracked.is_empty() {
                // `NormalGatewayEntity.completeWave`.
                self.queue_wave_rewards();
                self.active = false;
                self.ticks_active = 0;
                self.wave += 1;
            }
        } else if self.can_start_next_wave() {
            self.active = true;
            self.ticks_active = 0;
            for wave_entity in self.current_wave_entities() {
                for _ in 0..wave_entity.count {
                    let Some(pos) = self.spawn_position(rng, probe) else {
                        return fail(&mut self.finished, FailureReason::SpawnFailed);
                    };
                    out.push(Effect::Spawn {
                        kind: wave_entity.kind,
                        name: wave_entity.name,
                        pos,
                    });
                }
            }
        } else if self.is_completed() {
            // `NormalGatewayEntity.completeGateway`.
            self.finished = true;
            for reward in &self.def.rewards {
                if let Reward::Summon { kind, count } = reward {
                    for _ in 0..*count {
                        let pos = self.spawn_position(rng, probe).unwrap_or(self.pos);
                        out.push(Effect::Summon { kind, pos });
                    }
                }
            }
            out.push(Effect::Completed);
        }
        out
    }

    /// `GatewayEntity.canStartNextWave`: the setup time has elapsed and another
    /// wave is left.
    fn can_start_next_wave(&self) -> bool {
        self.ticks_active > self.setup_time() && self.wave < self.def.waves.len()
    }

    /// `NormalGatewayEntity.isCompleted`.
    fn is_completed(&self) -> bool {
        self.queued.is_empty() && self.wave >= self.def.waves.len()
    }

    /// `GatewayEntity.getSetupTime` for the wave about to start.
    fn setup_time(&self) -> u32 {
        self.def
            .waves
            .get(self.wave)
            .map_or(0, |wave| wave.setup_time)
    }

    /// `GatewayEntity.getMaxWaveTime` for the wave being fought.
    fn max_wave_time(&self) -> u32 {
        self.def
            .waves
            .get(self.wave.min(self.def.waves.len().saturating_sub(1)))
            .map_or(0, |wave| wave.max_wave_time)
    }

    /// The entities of the wave about to start (`NormalGatewayEntity.getCurrentWave`).
    fn current_wave_entities(&self) -> &[WaveEntity] {
        self.def
            .waves
            .get(self.wave)
            .map_or(&[], |wave| wave.entities.as_slice())
    }

    /// `Wave.spawnRewards`: split each stack into copies of at most four.
    fn queue_wave_rewards(&mut self) {
        let Some(wave) = self.def.waves.get(self.wave) else {
            return;
        };
        for reward in &wave.rewards {
            if let Reward::Stack { item, count } = reward {
                for (item, count) in split_stack(item, *count) {
                    self.queued.push_back((item, count));
                }
            }
        }
    }

    /// `SpawnAlgorithms.open_field` against `probe`.
    #[must_use]
    pub fn spawn_position(&self, rng: &mut dyn Rng, probe: &dyn Probe) -> Option<[f64; 3]> {
        let (width, _) = self.def.size.dims();
        let range = width / 2.0 + self.def.rules.spawn_range;
        let scale = self.def.size.scale();
        let leash_sq = self.def.rules.leash_range_sq();
        let min_y = probe.min_y();
        // `SpawnAlgorithms.MAX_SPAWN_TRIES`.
        for _ in 0..15 {
            let x = self.pos[0] + triangular(rng) * range + 0.5;
            let mut y = self.pos[1] + f64::from(rng.next_below(3 * scale as i32) + 1);
            let z = self.pos[2] + triangular(rng) * range + 0.5;

            // Descend to the floor while the block below is empty.
            while y > f64::from(min_y) && probe.empty([floor(x), floor(y) - 1, floor(z)]) {
                y -= 1.0;
            }
            // Ascend until the mob fits (a two-block body).
            let mut lifted = 0;
            while !fits(probe, [floor(x), floor(y), floor(z)]) && lifted < 24 {
                y += 1.0;
                lifted += 1;
            }
            if !fits(probe, [floor(x), floor(y), floor(z)]) {
                continue;
            }
            let dx = x - self.pos[0];
            let dy = y - self.pos[1];
            let dz = z - self.pos[2];
            if dx * dx + dy * dy + dz * dz > leash_sq {
                continue;
            }
            return Some([x, y, z]);
        }
        None
    }
}

/// `(rng - rng)`: the triangular offset `open_field` draws.
fn triangular(rng: &mut dyn Rng) -> f64 {
    rng.next_f64() - rng.next_f64()
}

fn floor(v: f64) -> i32 {
    v.floor() as i32
}

/// Whether a two-block-tall body fits at this cell: the cell and the one above
/// are both collision-free.
fn fits(probe: &dyn Probe, pos: [i32; 3]) -> bool {
    probe.empty(pos) && probe.empty([pos[0], pos[1] + 1, pos[2]])
}

fn fail(finished: &mut bool, reason: FailureReason) -> Vec<Effect> {
    *finished = true;
    vec![Effect::Failed(reason)]
}

/// `Wave.spawnRewards`' split: every stack over four becomes copies of four.
#[must_use]
pub fn split_stack(item: &'static str, count: u32) -> Vec<(&'static str, u32)> {
    let mut out = Vec::new();
    let mut left = count;
    while left > 4 {
        out.push((item, 4));
        left -= 4;
    }
    if left > 0 {
        out.push((item, left));
    }
    out
}

/// The carried gateway for `id`, or `None` for an id this slice does not carry.
#[must_use]
pub fn gateway(id: &str) -> Option<Gateway> {
    match id {
        EMERALD_GROVE => Some(emerald_grove()),
        _ => None,
    }
}

/// `data/gateways/gateways/emerald_grove.json`, carried verbatim except that the
/// wave attribute modifiers (no attribute API at the pin), the gear set and the
/// failure explosion are recorded gaps.
#[must_use]
pub fn emerald_grove() -> Gateway {
    Gateway {
        id: EMERALD_GROVE,
        size: Size::Medium,
        rules: Rules::default(),
        waves: vec![
            Wave {
                entities: vec![WaveEntity {
                    kind: "minecraft:zombie",
                    count: 5,
                    name: None,
                }],
                rewards: vec![Reward::Stack {
                    item: "minecraft:hay_block",
                    count: 16,
                }],
                max_wave_time: 1000,
                setup_time: 200,
            },
            Wave {
                entities: vec![WaveEntity {
                    kind: "minecraft:spider",
                    count: 5,
                    name: None,
                }],
                rewards: vec![
                    Reward::Stack {
                        item: "minecraft:cactus",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:sugar_cane",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:bamboo",
                        count: 16,
                    },
                ],
                max_wave_time: 1000,
                setup_time: 200,
            },
            Wave {
                entities: vec![
                    WaveEntity {
                        kind: "minecraft:zombie",
                        count: 5,
                        name: None,
                    },
                    WaveEntity {
                        kind: "minecraft:spider",
                        count: 2,
                        name: None,
                    },
                ],
                rewards: vec![
                    Reward::Stack {
                        item: "minecraft:carrot",
                        count: 32,
                    },
                    Reward::Stack {
                        item: "minecraft:potato",
                        count: 32,
                    },
                    Reward::Stack {
                        item: "minecraft:apple",
                        count: 32,
                    },
                    Reward::Stack {
                        item: "minecraft:beetroot",
                        count: 32,
                    },
                ],
                max_wave_time: 1000,
                setup_time: 200,
            },
            Wave {
                entities: vec![
                    WaveEntity {
                        kind: "minecraft:zombie_villager",
                        count: 1,
                        name: Some("Necrotic Farmer"),
                    },
                    WaveEntity {
                        kind: "minecraft:zombie",
                        count: 3,
                        name: None,
                    },
                    WaveEntity {
                        kind: "minecraft:spider",
                        count: 2,
                        name: None,
                    },
                ],
                rewards: vec![
                    Reward::Stack {
                        item: "minecraft:oak_sapling",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:birch_sapling",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:spruce_sapling",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:dark_oak_sapling",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:acacia_sapling",
                        count: 16,
                    },
                    Reward::Stack {
                        item: "minecraft:cherry_sapling",
                        count: 16,
                    },
                ],
                max_wave_time: 1200,
                setup_time: 200,
            },
        ],
        rewards: vec![
            Reward::Summon {
                kind: "minecraft:cow",
                count: 6,
            },
            Reward::Summon {
                kind: "minecraft:chicken",
                count: 6,
            },
            Reward::Summon {
                kind: "minecraft:sheep",
                count: 6,
            },
            Reward::Summon {
                kind: "minecraft:pig",
                count: 6,
            },
        ],
    }
}

/// `GatewayEntity.getBoundingBox` at `pos`: a box `width` wide and `height` tall
/// with its feet at `pos`.
#[must_use]
pub fn bounds(pos: [f64; 3], size: Size) -> ([f64; 3], [f64; 3]) {
    let (width, height) = size.dims();
    let half = width / 2.0;
    (
        [pos[0] - half, pos[1], pos[2] - half],
        [pos[0] + half, pos[1] + height, pos[2] + half],
    )
}

/// `AABB.intersects` for two gateway boxes, the new one inflated by `spacing`
/// (`GatePearlItem.useOn`).
#[must_use]
pub fn overlaps(
    a_pos: [f64; 3],
    a_size: Size,
    b_pos: [f64; 3],
    b_size: Size,
    spacing: f64,
) -> bool {
    let (a_min, a_max) = bounds(a_pos, a_size);
    let (b_min, b_max) = bounds(b_pos, b_size);
    a_min[0] - spacing < b_max[0]
        && a_max[0] + spacing > b_min[0]
        && a_min[1] - spacing < b_max[1]
        && a_max[1] + spacing > b_min[1]
        && a_min[2] - spacing < b_max[2]
        && a_max[2] + spacing > b_min[2]
}

/// Whether every cell the gateway's box covers is collision-free
/// (`Level.noCollision(entity)`).
#[must_use]
pub fn box_is_clear(probe: &dyn Probe, pos: [f64; 3], size: Size) -> bool {
    let (min, max) = bounds(pos, size);
    for x in floor(min[0])..ceil(max[0]) {
        for y in floor(min[1])..ceil(max[1]) {
            for z in floor(min[2])..ceil(max[2]) {
                if !probe.empty([x, y, z]) {
                    return false;
                }
            }
        }
    }
    true
}

fn ceil(v: f64) -> i32 {
    v.ceil() as i32
}

/// `GatePearlItem.useOn`'s placement: the clicked block's top, then up to four
/// lifts of one block while the gateway's box collides, else `None` (`no_space`).
#[must_use]
pub fn placement(clicked: [i32; 3], top: f64, size: Size, probe: &dyn Probe) -> Option<[f64; 3]> {
    let mut pos = [
        f64::from(clicked[0]) + 0.5,
        f64::from(clicked[1]) + top,
        f64::from(clicked[2]) + 0.5,
    ];
    for _ in 0..4 {
        if box_is_clear(probe, pos, size) {
            return Some(pos);
        }
        pos[1] += 1.0;
    }
    box_is_clear(probe, pos, size).then_some(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A probe for a flat floor at `y = 99` with air above, and walls outside
    /// `±40` so the placement tests are bounded.
    struct Flat {
        floor: i32,
    }

    impl Probe for Flat {
        fn empty(&self, pos: [i32; 3]) -> bool {
            pos[1] > self.floor
        }
        fn min_y(&self) -> i32 {
            -64
        }
    }

    /// A probe that is air everywhere (no floor).
    struct Void;

    impl Probe for Void {
        fn empty(&self, _pos: [i32; 3]) -> bool {
            true
        }
        fn min_y(&self) -> i32 {
            -64
        }
    }

    /// Records the mobs the glue reports as spawned.
    struct World {
        alive: Vec<u32>,
        distances: Vec<(u32, f64)>,
    }

    impl World {
        /// Distance squared to a tracked mob, or `None` once it is dead.
        fn distance(&self, id: u32) -> Option<f64> {
            self.distances
                .iter()
                .find(|(tracked, _)| *tracked == id)
                .map(|(_, d)| *d)
                .filter(|_| self.alive.contains(&id))
        }
    }

    /// One tick against a flat floor at `y = 99`.
    fn tick_once(gate: &mut Gate, world: &World, nearby: bool) -> Vec<Effect> {
        let distance = |id: u32| world.distance(id);
        let view = TickView {
            nearby_player: nearby,
            distance_sq: &distance,
        };
        gate.tick(&view, &mut rng(), &Flat { floor: 99 })
    }

    fn rng() -> XorShift {
        XorShift::seeded(20261112)
    }

    fn gate() -> Gate {
        Gate::new(emerald_grove(), "minecraft:overworld", [0.5, 100.0, 0.5], 7)
    }

    /// Ticks the gate `n` times, feeding back the spawns the glue would make and
    /// returning every effect.
    fn run(gate: &mut Gate, world: &mut World, n: usize, nearby: bool) -> Vec<Effect> {
        let mut all = Vec::new();
        for _ in 0..n {
            let effects = tick_once(gate, world, nearby);
            for effect in &effects {
                if let Effect::Spawn { .. } = effect {
                    let id = 100 + world.alive.len() as u32;
                    world.alive.push(id);
                    world.distances.push((id, 1.0));
                    gate.add_spawned([id]);
                }
            }
            all.extend(effects);
        }
        all
    }

    fn spawns(effects: &[Effect]) -> Vec<(&'static str, Option<&'static str>)> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Spawn { kind, name, .. } => Some((*kind, *name)),
                _ => None,
            })
            .collect()
    }

    fn drops(effects: &[Effect]) -> Vec<(&'static str, u32)> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Drop { item, count } => Some((*item, *count)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_carried_gateway_is_the_jar_definition() {
        let def = gateway(EMERALD_GROVE).expect("the carried gateway resolves");
        assert_eq!(def.size, Size::Medium);
        assert_eq!(def.rules, Rules::default());
        assert_eq!(def.waves.len(), 4, "emerald_grove.json has four waves");
        assert_eq!(def.waves[0].setup_time, 200);
        assert_eq!(def.waves[0].max_wave_time, 1000);
        assert_eq!(def.waves[0].entities[0].count, 5);
        assert_eq!(def.waves[3].max_wave_time, 1200);
        assert_eq!(def.waves[3].entities[0].name, Some("Necrotic Farmer"));
        assert_eq!(def.rewards.len(), 4, "four animal summons");
        assert!(
            gateway("gateways:basic/blaze").is_none(),
            "other gateways are a gap"
        );
    }

    #[test]
    fn setup_time_gates_the_first_wave() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        let early = run(&mut gate, &mut world, 200, true);
        assert!(
            spawns(&early).is_empty(),
            "wave 1 spawned before its 200-tick setup time"
        );
        let late = run(&mut gate, &mut world, 1, true);
        assert_eq!(
            spawns(&late).len(),
            5,
            "wave 1 must spawn five zombies on tick 201"
        );
        assert!(gate.is_active());
        assert_eq!(
            gate.wave(),
            0,
            "the wave index moves on completion, not start"
        );
    }

    #[test]
    fn wave_one_spawns_the_jars_five_zombies() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 200, true);
        let effects = run(&mut gate, &mut world, 1, true);
        let spawned = spawns(&effects);
        assert_eq!(spawned.len(), 5);
        assert!(spawned.iter().all(|(kind, _)| *kind == "minecraft:zombie"));
        assert_eq!(gate.tracked().len(), 5);
    }

    #[test]
    fn an_unkilled_wave_never_pays() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        let effects = run(&mut gate, &mut world, 201, true);
        assert_eq!(spawns(&effects).len(), 5);
        let waited = run(&mut gate, &mut world, 100, true);
        assert!(
            drops(&waited).is_empty(),
            "a wave with living mobs paid out: {:?}",
            drops(&waited)
        );
        assert_eq!(gate.queued(), 0);
    }

    #[test]
    fn the_wave_pays_when_every_mob_is_gone() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 201, true);
        assert_eq!(gate.tracked().len(), 5);
        world.alive.clear();
        // The first tick after the last mob is gone completes the wave and
        // queues its reward; the drop happens on the next fourth tick.
        run(&mut gate, &mut world, 1, true);
        assert_eq!(gate.wave(), 1, "the wave index advanced once");
        assert!(!gate.is_active());
        let effects = run(&mut gate, &mut world, 4, true);
        assert_eq!(
            drops(&effects),
            vec![
                ("minecraft:hay_block", 4),
                ("minecraft:hay_block", 4),
                ("minecraft:hay_block", 4),
            ],
            "16 hay blocks drip out as three stacks of four on the first drop tick"
        );
        let rest = run(&mut gate, &mut world, 4, true);
        assert_eq!(drops(&rest), vec![("minecraft:hay_block", 4)]);
    }

    #[test]
    fn the_reward_is_granted_once() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 201, true);
        world.alive.clear();
        let paid = run(&mut gate, &mut world, 20, true);
        let total: u32 = drops(&paid).iter().map(|(_, count)| *count).sum();
        assert_eq!(total, 16, "the wave must pay exactly 16 hay blocks");
        let again = run(&mut gate, &mut world, 60, true);
        let more: u32 = drops(&again).iter().map(|(_, count)| *count).sum();
        assert_eq!(more, 0, "the same wave paid twice");
    }

    #[test]
    fn stacks_over_four_are_split() {
        assert_eq!(
            split_stack("minecraft:hay_block", 16),
            vec![("minecraft:hay_block", 4); 4]
        );
        assert_eq!(split_stack("minecraft:carrot", 32).len(), 8);
        assert_eq!(
            split_stack("minecraft:apple", 4),
            vec![("minecraft:apple", 4)]
        );
        assert_eq!(
            split_stack("minecraft:apple", 3),
            vec![("minecraft:apple", 3)]
        );
        assert!(split_stack("minecraft:apple", 0).is_empty());
    }

    #[test]
    fn the_wave_timer_fails_an_unfinished_wave() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 201, true);
        // Wave 1 allows 1000 ticks after it starts.
        let effects = run(&mut gate, &mut world, 1000, true);
        assert!(
            !effects.contains(&Effect::Failed(FailureReason::TimerElapsed)),
            "the wave failed before its 1000-tick timer"
        );
        let late = run(&mut gate, &mut world, 2, true);
        assert!(late.contains(&Effect::Failed(FailureReason::TimerElapsed)));
        assert!(gate.is_finished());
        assert!(drops(&late).is_empty(), "a timed-out wave must not pay");
    }

    #[test]
    fn no_player_nearby_fails_after_two_hundred_ticks() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        let effects = run(&mut gate, &mut world, 199, false);
        assert!(!effects.contains(&Effect::Failed(FailureReason::NoNearbyPlayer)));
        let late = run(&mut gate, &mut world, 2, false);
        assert!(late.contains(&Effect::Failed(FailureReason::NoNearbyPlayer)));
        assert!(gate.is_finished());
    }

    #[test]
    fn a_player_returning_resets_the_grace_period() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 150, false);
        run(&mut gate, &mut world, 1, true);
        let effects = run(&mut gate, &mut world, 150, false);
        assert!(
            !effects.contains(&Effect::Failed(FailureReason::NoNearbyPlayer)),
            "the grace period did not reset when a player came back"
        );
        assert!(!gate.is_finished());
    }

    #[test]
    fn the_last_wave_queues_the_completion_summons() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        // Walk all four waves: spawn, kill, wait out the setup and the drip.
        let mut all = Vec::new();
        for wave in 0..4 {
            let effects = run(&mut gate, &mut world, 201, true);
            assert_eq!(
                spawns(&effects).len(),
                gate.def().waves[wave]
                    .entities
                    .iter()
                    .map(|entity| entity.count as usize)
                    .sum::<usize>(),
                "wave {wave} spawned the wrong count"
            );
            all.extend(effects);
            world.alive.clear();
            all.extend(run(&mut gate, &mut world, 40, true));
        }
        all.extend(run(&mut gate, &mut world, 8, true));
        let summoned: Vec<&'static str> = all
            .iter()
            .filter_map(|effect| match effect {
                Effect::Summon { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        assert_eq!(summoned.len(), 24, "six of each animal");
        assert_eq!(
            summoned
                .iter()
                .filter(|kind| **kind == "minecraft:cow")
                .count(),
            6
        );
        assert!(all.contains(&Effect::Completed));
        assert!(gate.is_finished());
    }

    #[test]
    fn the_wave_list_advances_only_after_its_setup_time() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 201, true);
        world.alive.clear();
        // One tick to complete wave 1 and start its 200-tick setup.
        run(&mut gate, &mut world, 1, true);
        let early = run(&mut gate, &mut world, 200, true);
        assert!(
            spawns(&early).is_empty(),
            "wave 2 spawned before its setup time"
        );
        let late = run(&mut gate, &mut world, 1, true);
        assert_eq!(spawns(&late).len(), 5);
        assert!(
            spawns(&late)
                .iter()
                .all(|(kind, _)| *kind == "minecraft:spider")
        );
    }

    #[test]
    fn an_out_of_leash_mob_is_moved_back_by_default() {
        let mut gate = gate();
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 201, true);
        // Put the tracked mobs 40 blocks out (leash is 32).
        for (_, distance) in &mut world.distances {
            *distance = 1600.0;
        }
        let effects = run(&mut gate, &mut world, 20, true);
        let moved = effects
            .iter()
            .filter(|effect| matches!(effect, Effect::Move { .. }))
            .count();
        assert_eq!(
            moved, 5,
            "every far mob is re-placed when the rule allows it"
        );
        assert!(!effects.contains(&Effect::Failed(FailureReason::TooFar)));
        assert!(!gate.is_finished());
    }

    #[test]
    fn out_of_leash_fails_when_the_rule_says_so() {
        let mut def = emerald_grove();
        def.rules.fail_on_out_of_bounds = true;
        let mut gate = Gate::new(def, "minecraft:overworld", [0.5, 100.0, 0.5], 7);
        let mut world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        run(&mut gate, &mut world, 201, true);
        for (_, distance) in &mut world.distances {
            *distance = 1600.0;
        }
        let effects = run(&mut gate, &mut world, 20, true);
        assert!(effects.contains(&Effect::Failed(FailureReason::TooFar)));
        assert!(gate.is_finished());
    }

    #[test]
    fn a_failed_spawn_implodes_the_gateway() {
        let mut gate = gate();
        // A probe with no air at all: the spawn placement can never find room.
        struct Solid;
        impl Probe for Solid {
            fn empty(&self, _pos: [i32; 3]) -> bool {
                false
            }
            fn min_y(&self) -> i32 {
                -64
            }
        }
        let world = World {
            alive: Vec::new(),
            distances: Vec::new(),
        };
        let mut effects = Vec::new();
        for _ in 0..201 {
            let distance = |id: u32| world.distance(id);
            let view = TickView {
                nearby_player: true,
                distance_sq: &distance,
            };
            effects.extend(gate.tick(&view, &mut rng(), &Solid));
        }
        assert!(effects.contains(&Effect::Failed(FailureReason::SpawnFailed)));
        assert!(gate.is_finished());
    }

    #[test]
    fn the_spawn_position_lands_on_the_floor_inside_the_leash() {
        let gate = gate();
        let probe = Flat { floor: 99 };
        for seed in 0..64 {
            let mut rng = XorShift::seeded(seed);
            let pos = gate
                .spawn_position(&mut rng, &probe)
                .unwrap_or_else(|| panic!("no spawn position for seed {seed}"));
            assert_eq!(pos[1], 100.0, "the mob stands on the floor at y=100");
            let (dx, dy, dz) = (pos[0] - 0.5, pos[1] - 100.0, pos[2] - 0.5);
            assert!(
                dx * dx + dy * dy + dz * dz <= 32.0 * 32.0,
                "outside the leash: {pos:?}"
            );
            // Medium gateway: width 4, so 2 + spawn_range 8 = 10 blocks.
            assert!(
                dx.abs() <= 10.0 && dz.abs() <= 10.0,
                "outside the spawn range: {pos:?}"
            );
        }
    }

    #[test]
    fn the_placement_lifts_a_block_at_a_time_and_refuses_when_full() {
        let size = Size::Medium;
        // A floor with a ceiling 1 block above it: no 4-high box fits.
        struct Cramped;
        impl Probe for Cramped {
            fn empty(&self, pos: [i32; 3]) -> bool {
                pos[1] == 100
            }
            fn min_y(&self) -> i32 {
                -64
            }
        }
        assert!(placement([0, 99, 0], 1.0, size, &Cramped).is_none());
        // A floor with room: the box sits on top of the clicked block.
        let open = Flat { floor: 99 };
        let pos = placement([0, 99, 0], 1.0, size, &open).expect("room above the floor");
        assert_eq!(pos, [0.5, 100.0, 0.5]);
        // A ceiling one block above the floor at y=100: the box lifts until it fits.
        struct Step;
        impl Probe for Step {
            fn empty(&self, pos: [i32; 3]) -> bool {
                pos[1] > 99 && pos[1] != 100
            }
            fn min_y(&self) -> i32 {
                -64
            }
        }
        let lifted = placement([0, 99, 0], 1.0, size, &Step).expect("lifts clear");
        assert_eq!(
            lifted[1], 101.0,
            "lifted past the blocked layer: {lifted:?}"
        );
        // The `Void` probe is air everywhere, so the box always fits.
        assert!(placement([0, 99, 0], 1.0, size, &Void).is_some());
    }

    #[test]
    fn a_second_gateway_in_the_same_box_is_refused() {
        let size = Size::Medium;
        assert!(overlaps(
            [0.5, 100.0, 0.5],
            size,
            [0.5, 100.0, 0.5],
            size,
            0.0
        ));
        assert!(overlaps(
            [0.5, 100.0, 0.5],
            size,
            [3.0, 100.0, 0.5],
            size,
            0.0
        ));
        assert!(!overlaps(
            [0.5, 100.0, 0.5],
            size,
            [9.0, 100.0, 0.5],
            size,
            0.0
        ));
        assert!(overlaps(
            [0.5, 100.0, 0.5],
            size,
            [9.0, 100.0, 0.5],
            size,
            5.0
        ));
        // Vertically: the box is four tall.
        assert!(!overlaps(
            [0.5, 100.0, 0.5],
            size,
            [0.5, 104.0, 0.5],
            size,
            0.0
        ));
    }
}
