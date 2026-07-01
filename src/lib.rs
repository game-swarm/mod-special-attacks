use bevy::prelude::*;
use std::collections::BTreeSet;
use swarm_engine::components::{Drone, Owner, PlayerId, Position};

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ContinuousAction {
    pub disrupted: bool,
}

#[derive(Resource, Debug, Clone)]
pub struct SpecialAttacksConfig {
    pub enabled: BTreeSet<SpecialAttackKind>,
    pub damage_multiplier: u32,
}

impl Default for SpecialAttacksConfig {
    fn default() -> Self {
        Self {
            enabled: [
                SpecialAttackKind::Hack,
                SpecialAttackKind::Drain,
                SpecialAttackKind::Overload,
                SpecialAttackKind::Debilitate,
                SpecialAttackKind::Disrupt,
                SpecialAttackKind::Fortify,
                SpecialAttackKind::Leech,
                SpecialAttackKind::Fabricate,
            ]
            .into_iter()
            .collect(),
            damage_multiplier: 10_000,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ActionRegistry {
    pub handlers: BTreeSet<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecialAttackKind {
    Fortify = 1,
    Leech = 2,
    Fabricate = 3,
    Disrupt = 4,
    Debilitate = 5,
    Overload = 6,
    Drain = 7,
    Hack = 8,
}

#[derive(Debug, Clone)]
pub struct SpecialAttackIntent {
    pub source: Entity,
    pub target: Entity,
    pub amount: u32,
}

macro_rules! buffer {
    ($name:ident) => {
        #[derive(Resource, Debug, Clone, Default)]
        pub struct $name {
            pub entries: Vec<SpecialAttackIntent>,
        }
    };
}

buffer!(HackBuffer);
buffer!(DrainBuffer);
buffer!(OverloadBuffer);
buffer!(DebilitateBuffer);
buffer!(DisruptBuffer);
buffer!(FortifyBuffer);
buffer!(LeechBuffer);
buffer!(FabricateBuffer);

#[derive(Resource, Debug, Clone, Default)]
pub struct PendingDamage {
    pub entries: Vec<DamageIntent>,
}

#[derive(Debug, Clone)]
pub struct DamageIntent {
    pub source: Entity,
    pub target: Entity,
    pub amount: u32,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct PendingHeal {
    pub entries: Vec<HealIntent>,
}

#[derive(Debug, Clone)]
pub struct HealIntent {
    pub target: Entity,
    pub amount: u32,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct PendingIntents {
    pub entries: Vec<ResolvedIntent>,
}

#[derive(Debug, Clone)]
pub struct ResolvedIntent {
    pub kind: SpecialAttackKind,
    pub source: Entity,
    pub target: Entity,
    pub amount: u32,
}

#[derive(Component, Debug, Clone)]
pub struct HackState {
    pub controller: Entity,
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct DrainState {
    pub source: Entity,
    pub amount_per_tick: u32,
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct OverloadState {
    pub fatigue_per_tick: u32,
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct DebilitateState {
    pub penalty_bp: u32,
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct DisruptState {
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct FortifyState {
    pub resistance_bp: u32,
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct LeechState {
    pub source: Entity,
    pub amount_per_tick: u32,
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct FabricateState {
    pub remaining_ticks: u32,
}

#[derive(Component, Debug, Clone)]
pub struct Fabricated {
    pub owner: PlayerId,
    pub ttl: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SpecialAttacksModPlugin;

impl Plugin for SpecialAttacksModPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpecialAttacksConfig>()
            .init_resource::<ActionRegistry>()
            .init_resource::<HackBuffer>()
            .init_resource::<DrainBuffer>()
            .init_resource::<OverloadBuffer>()
            .init_resource::<DebilitateBuffer>()
            .init_resource::<DisruptBuffer>()
            .init_resource::<FortifyBuffer>()
            .init_resource::<LeechBuffer>()
            .init_resource::<FabricateBuffer>()
            .init_resource::<PendingDamage>()
            .init_resource::<PendingHeal>()
            .init_resource::<PendingIntents>()
            .add_systems(Startup, register_special_actions)
            .add_systems(
                Update,
                (
                    hack_buffer_system,
                    drain_buffer_system,
                    overload_buffer_system,
                    debilitate_buffer_system,
                    disrupt_buffer_system,
                    fortify_buffer_system,
                    leech_buffer_system,
                    fabricate_buffer_system,
                    status_advance_system,
                    status_effect_tick_system,
                    fabricated_ttl_system,
                )
                    .chain(),
            );
    }
}

pub fn register_special_actions(
    config: Res<SpecialAttacksConfig>,
    mut registry: ResMut<ActionRegistry>,
) {
    for (kind, name) in [
        (SpecialAttackKind::Hack, "Hack"),
        (SpecialAttackKind::Drain, "Drain"),
        (SpecialAttackKind::Overload, "Overload"),
        (SpecialAttackKind::Debilitate, "Debilitate"),
        (SpecialAttackKind::Disrupt, "Disrupt"),
        (SpecialAttackKind::Fortify, "Fortify"),
        (SpecialAttackKind::Leech, "Leech"),
        (SpecialAttackKind::Fabricate, "Fabricate"),
    ] {
        if config.enabled.contains(&kind) {
            registry.handlers.insert(name);
        }
    }
}

macro_rules! drain_buffer_system {
    ($fn_name:ident, $buffer:ident, $kind:expr) => {
        pub fn $fn_name(
            config: Res<SpecialAttacksConfig>,
            mut buffer: ResMut<$buffer>,
            mut intents: ResMut<PendingIntents>,
        ) {
            if !config.enabled.contains(&$kind) {
                buffer.entries.clear();
                return;
            }
            intents
                .entries
                .extend(buffer.entries.drain(..).map(|entry| ResolvedIntent {
                    kind: $kind,
                    source: entry.source,
                    target: entry.target,
                    amount: scale(entry.amount, config.damage_multiplier),
                }));
        }
    };
}

drain_buffer_system!(hack_buffer_system, HackBuffer, SpecialAttackKind::Hack);
drain_buffer_system!(drain_buffer_system, DrainBuffer, SpecialAttackKind::Drain);
drain_buffer_system!(
    overload_buffer_system,
    OverloadBuffer,
    SpecialAttackKind::Overload
);
drain_buffer_system!(
    debilitate_buffer_system,
    DebilitateBuffer,
    SpecialAttackKind::Debilitate
);
drain_buffer_system!(
    disrupt_buffer_system,
    DisruptBuffer,
    SpecialAttackKind::Disrupt
);
drain_buffer_system!(
    fortify_buffer_system,
    FortifyBuffer,
    SpecialAttackKind::Fortify
);
drain_buffer_system!(leech_buffer_system, LeechBuffer, SpecialAttackKind::Leech);
drain_buffer_system!(
    fabricate_buffer_system,
    FabricateBuffer,
    SpecialAttackKind::Fabricate
);

pub fn status_advance_system(
    mut commands: Commands,
    mut intents: ResMut<PendingIntents>,
    positions: Query<&Position>,
    owners: Query<Option<&Owner>>,
) {
    let mut raw = std::mem::take(&mut intents.entries);
    raw.sort_by(|a, b| {
        b.kind
            .cmp(&a.kind)
            .then_with(|| a.source.to_bits().cmp(&b.source.to_bits()))
            .then_with(|| a.target.to_bits().cmp(&b.target.to_bits()))
    });
    let mut seen = BTreeSet::new();
    for intent in raw.into_iter().filter(|intent| seen.insert(intent.target)) {
        match intent.kind {
            SpecialAttackKind::Hack => {
                commands.entity(intent.target).insert(HackState {
                    controller: intent.source,
                    remaining_ticks: 5,
                });
            }
            SpecialAttackKind::Drain => {
                commands.entity(intent.target).insert(DrainState {
                    source: intent.source,
                    amount_per_tick: intent.amount.max(1),
                    remaining_ticks: 3,
                });
            }
            SpecialAttackKind::Overload => {
                commands.entity(intent.target).insert(OverloadState {
                    fatigue_per_tick: intent.amount.max(10),
                    remaining_ticks: 3,
                });
            }
            SpecialAttackKind::Debilitate => {
                commands.entity(intent.target).insert(DebilitateState {
                    penalty_bp: intent.amount.clamp(1_000, 9_000),
                    remaining_ticks: 50,
                });
            }
            SpecialAttackKind::Disrupt => {
                commands.entity(intent.target).insert((
                    DisruptState { remaining_ticks: 1 },
                    ContinuousAction { disrupted: true },
                ));
            }
            SpecialAttackKind::Fortify => {
                commands.entity(intent.target).insert(FortifyState {
                    resistance_bp: intent.amount.clamp(1_000, 9_000),
                    remaining_ticks: 3,
                });
            }
            SpecialAttackKind::Leech => {
                commands.entity(intent.target).insert(LeechState {
                    source: intent.source,
                    amount_per_tick: intent.amount.max(1),
                    remaining_ticks: 3,
                });
            }
            SpecialAttackKind::Fabricate => {
                commands
                    .entity(intent.target)
                    .insert(FabricateState { remaining_ticks: 1 });
                if let Ok(position) = positions.get(intent.target) {
                    let owner = owners
                        .get(intent.source)
                        .ok()
                        .flatten()
                        .map(|owner| owner.0)
                        .unwrap_or(0);
                    commands.spawn((Fabricated { owner, ttl: 10 }, *position));
                }
            }
        }
    }
}

pub fn status_effect_tick_system(
    mut commands: Commands,
    mut drones: Query<&mut Drone>,
    mut continuous: Query<&mut ContinuousAction>,
    mut hack: Query<(Entity, &mut HackState)>,
    mut drain: Query<(Entity, &mut DrainState)>,
    mut overload: Query<(Entity, &mut OverloadState)>,
    mut debilitate: Query<(Entity, &mut DebilitateState)>,
    mut disrupt: Query<(Entity, &mut DisruptState)>,
    mut fortify: Query<(Entity, &mut FortifyState)>,
    mut leech: Query<(Entity, &mut LeechState)>,
    mut fabricate: Query<(Entity, &mut FabricateState)>,
) {
    for (entity, mut state) in &mut drain {
        let drained = if let Ok(mut target) = drones.get_mut(entity) {
            let drained = target
                .carry
                .get("Energy")
                .copied()
                .unwrap_or(0)
                .min(state.amount_per_tick);
            if drained > 0 {
                let remaining = target.carry.get("Energy").copied().unwrap_or(0) - drained;
                target.carry.insert("Energy".to_string(), remaining);
            }
            drained
        } else {
            0
        };
        if drained > 0 {
            if let Ok(mut source) = drones.get_mut(state.source) {
                let current = source.carry.get("Energy").copied().unwrap_or(0);
                source
                    .carry
                    .insert("Energy".to_string(), current.saturating_add(drained));
            }
        }
        tick_or_remove::<DrainState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut overload {
        if let Ok(mut drone) = drones.get_mut(entity) {
            drone.fatigue = drone.fatigue.saturating_add(state.fatigue_per_tick);
        }
        tick_or_remove::<OverloadState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut leech {
        let healed = if let Ok(mut target) = drones.get_mut(entity) {
            let damage = target.hits.min(state.amount_per_tick);
            target.hits -= damage;
            damage
        } else {
            0
        };
        if healed > 0 {
            if let Ok(mut source) = drones.get_mut(state.source) {
                source.hits = source.hits.saturating_add(healed).min(source.hits_max);
            }
        }
        tick_or_remove::<LeechState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut disrupt {
        if let Ok(mut action) = continuous.get_mut(entity) {
            action.disrupted = true;
        }
        tick_or_remove::<DisruptState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut hack {
        tick_or_remove::<HackState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut debilitate {
        tick_or_remove::<DebilitateState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut fortify {
        tick_or_remove::<FortifyState>(&mut commands, entity, &mut state.remaining_ticks);
    }
    for (entity, mut state) in &mut fabricate {
        tick_or_remove::<FabricateState>(&mut commands, entity, &mut state.remaining_ticks);
    }
}

pub fn fabricated_ttl_system(
    mut commands: Commands,
    mut fabricated: Query<(Entity, &mut Fabricated)>,
) {
    for (entity, mut fabricated) in &mut fabricated {
        fabricated.ttl = fabricated.ttl.saturating_sub(1);
        if fabricated.ttl == 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn tick_or_remove<T: Component>(commands: &mut Commands, entity: Entity, ticks: &mut u32) {
    *ticks = ticks.saturating_sub(1);
    if *ticks == 0 {
        commands.entity(entity).remove::<T>();
    }
}

fn scale(amount: u32, multiplier_bp: u32) -> u32 {
    ((amount as u64 * multiplier_bp as u64) / 10_000).min(u32::MAX as u64) as u32
}
