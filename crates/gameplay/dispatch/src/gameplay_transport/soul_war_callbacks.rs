//! W11 source callbacks consumed by the actual native creature-owner turn.
//! Crystal 201189f: monster/quests/soul_war/{goshnars_greed,weak_soul,
//! strong_soul,powerful_soul,soulsnatcher,soul_sphere}.lua and
//! data-global/lib/quests/soul_war.lua:954-976. No client event advances a clock.
use super::{actor_spell::ChannelSpellStates, attack::ChannelAttackStates};
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, ThinkOccurrence};
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_encounters::bosses::goshnars_greed::{
    self as greed_model, GreedEffect, GreedMinion,
};
use oteryn_gameplay_encounters::bosses::rotten_blood::RoomPosition;
use oteryn_gameplay_entities::carrier::soul_war::SoulWarCreatureChange as Change;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

mod attacks;
pub(super) mod cruelty;
mod entry;
mod entry_state;
pub(super) mod greed;
mod hatred;
pub(super) mod megalomania;
pub(super) mod player_source;
mod taint_five;
mod taint_one;
mod taints;
pub(super) mod torment;
pub(super) mod white;
pub(super) use attacks::{
    PreparedSoulWarArea, RetainedSoulWarAttack, capture_attack_proposal,
    drive_due_through as drive_attack_due_through, drive_retained_attack_proposal,
};
pub(super) use player_source::publish_command_source;
pub(super) use taint_one::drive_due_through as drive_taint_one_due_through;

#[derive(Clone, Copy, PartialEq, Eq)]
enum OmittedTimer {
    Malice,
    Megalomania,
    InlineCallbacks,
}
pub(super) use taint_five::QualifiedFifthTaint;
pub(super) use taints::refresh as refresh_taints;
#[cfg(test)]
pub(super) use taints::source_player_census;
pub(super) use taints::{QualifiedThirdTaintPolicy, third_policy as source_third_taint_policy};

#[derive(Debug)]
struct Observed {
    actor: ExactActorRef,
    id: u64,
    minion: Option<GreedMinion>,
    sphere: bool,
    death_observed: bool,
    death_at_ms: std::sync::Mutex<Option<u64>>,
    last_think: Option<ThinkOccurrence>,
}

#[derive(Debug, Default)]
pub(super) struct SoulWarCallbacks {
    pub(super) malice: Option<super::soul_war_malice::MaliceCallbacks>,
    hatred: Option<hatred::HatredCallbacks>,
    cruelty: Option<cruelty::CrueltyCallbacks>,
    pub(super) maw: cruelty::maw::MawState,
    pub(super) sanity: megalomania::sanity::SanityState,
    megalomania: Option<megalomania::MegalomaniaCallbacks>,
    pub(super) white: white::WhiteTileState,
    entry_state: entry_state::EntryState,
    attacks: attacks::SoulWarAttacks,
    taint_one: taint_one::TaintOne,
    fifth: taint_five::FifthCadence,
    taints: taints::CurrentTaints,
    torment: torment::TormentCallbacks,
    qualified_content: Option<[u8; 32]>,
    boss: Option<ExactActorRef>,
    spawn_source: Option<oteryn_gameplay_entities::carrier::soul_war::SoulWarSpawnSource>,
    greed: greed_model::GoshnarsGreedRoom,
    greed_work: greed::Retention,
    observed: Vec<Observed>,
    next_id: u64,
}

impl SoulWarCallbacks {
    /// Initialized source bosses always rematch their actual committed handoff,
    /// even after the once-only timer activation has already happened.
    pub(super) fn requires_committed_entry(&self) -> bool {
        self.entry_state.has_ambient()
            || (self.qualified_content.is_some()
                && (self.boss.is_some()
                    || self.malice.is_some()
                    || self.hatred.is_some()
                    || self.cruelty.is_some()
                    || self.megalomania.is_some()))
    }
    /// The actual current handoff calls this after physical/state installation.
    /// One never-reused Instance owns this bit across all participants/retries.
    pub(super) fn committed_entry(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        elapsed_ms: u64,
    ) -> Result<bool, ()> {
        if !self.requires_committed_entry()
            || !self
                .entry_state
                .claim_with_ambient(runtime, now_us, elapsed_ms)?
        {
            return Ok(false);
        }
        if self.boss.is_some() {
            self.greed
                .lever_pulled_with_elapsed(now_us / 1_000, elapsed_ms);
        }
        if let Some(malice) = &mut self.malice {
            malice.committed_entry(now_us, elapsed_ms);
        }
        if let Some(hatred) = &mut self.hatred {
            hatred.committed_entry();
        }
        Ok(true)
    }
    /// Real source scheduler work; copied deaths belong to the canonical FIFO.
    pub(super) fn next_native_due(&self) -> Option<u64> {
        self.next_native_due_except(None)
    }
    /// Runtime-born source callbacks share the same original-due arbiter.
    pub(super) fn next_splinter_due(
        &self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
    ) -> Result<Option<u64>, ()> {
        if self.megalomania.is_none() {
            return Ok(None);
        }
        megalomania::splinter::next_due_us(runtime, native)
    }
    pub(super) fn next_native_due_without_malice(&self) -> Option<u64> {
        self.next_native_due_except(Some(OmittedTimer::Malice))
    }
    pub(super) fn next_native_due_without_megalomania(&self) -> Option<u64> {
        self.next_native_due_except(Some(OmittedTimer::Megalomania))
    }
    pub(super) fn next_native_due_without_inline_callbacks(&self) -> Option<u64> {
        self.next_native_due_except(Some(OmittedTimer::InlineCallbacks))
    }
    pub(super) fn next_taint_one_due(&self) -> Option<u64> {
        self.taint_one.next_due()
    }
    pub(super) fn next_attack_due(&self) -> Option<u64> {
        self.attacks.next_due()
    }
    fn next_native_due_except(&self, omitted: Option<OmittedTimer>) -> Option<u64> {
        self.greed_work
            .next_due_us(&self.greed)
            .filter(|_| omitted != Some(OmittedTimer::InlineCallbacks))
            .into_iter()
            .chain(
                self.malice
                    .as_ref()
                    .filter(|_| omitted != Some(OmittedTimer::Malice))
                    .and_then(|owner| owner.next_due_ms())
                    .and_then(|ms| ms.checked_mul(1_000)),
            )
            .chain(
                self.megalomania
                    .as_ref()
                    .filter(|_| omitted != Some(OmittedTimer::Megalomania))
                    .and_then(|owner| owner.next_due_ms())
                    .and_then(|ms| ms.checked_mul(1_000)),
            )
            .chain(
                self.attacks
                    .next_due()
                    .filter(|_| omitted != Some(OmittedTimer::InlineCallbacks)),
            )
            .chain(
                self.taint_one
                    .next_due()
                    .filter(|_| omitted != Some(OmittedTimer::InlineCallbacks)),
            )
            .min()
    }
    pub(super) fn has_native_work(&self) -> bool {
        self.next_native_due().is_some()
    }
    pub(super) fn has_native_due(&self, now: u64) -> bool {
        self.next_native_due().is_some_and(|due| due <= now)
    }

    /// Called at actual source boss admission, before publishing a playable room.
    /// The caller retains this owner alongside the physical Instance runtime.
    pub(super) fn initialize_greed(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<Self, ()> {
        if room.pinned_activity_source_room()
            != Some(
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsGreed,
            )
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
            || !qualifies_source(runtime, native)
            || source_actor(runtime, native, boss).as_deref() != Some(greed_model::GOSHNARS_GREED)
            || !runtime
                .read_actor_position(boss)
                .is_ok_and(|p| in_greed(p.position()))
        {
            return Err(());
        }
        let mut owner = Self {
            taints: taints::CurrentTaints::for_room(runtime, room),
            taint_one: taint_one::TaintOne::at_birth(now_us),
            greed: entry_state::greed_at_birth(),
            greed_work: greed::Retention::capture(runtime)?,
            qualified_content: Some(native.source_digest()),
            boss: Some(boss),
            spawn_source: Some(
                runtime
                    .qualify_soul_war_spawn_source(boss)
                    .map_err(|_| ())?,
            ),
            ..Default::default()
        };
        for effect in owner.greed.boss_spawned() {
            match effect {
                GreedEffect::BossImmune { immune } => {
                    change(runtime, boss, Change::Immune(immune))?
                }
                _ => return Err(()),
            }
        }
        observe(&mut owner, boss, None, false)?;
        Ok(owner)
    }

    pub(super) fn initialize_malice(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<Self, ()> {
        Ok(Self {
            qualified_content: Some(native.source_digest()),
            taints: taints::CurrentTaints::for_room(runtime, room),
            taint_one: taint_one::TaintOne::at_birth(now_us),
            white: white::WhiteTileState::for_room(room)?,
            malice: Some(super::soul_war_malice::MaliceCallbacks::initialize(
                runtime, native, room, boss, now_us,
            )?),
            ..Default::default()
        })
    }

    pub(super) fn initialize_megalomania(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<Self, ()> {
        Ok(Self {
            qualified_content: Some(native.source_digest()),
            taints: taints::CurrentTaints::for_room(runtime, room),
            taint_one: taint_one::TaintOne::at_birth(now_us),
            white: white::WhiteTileState::for_room(room)?,
            megalomania: Some(megalomania::MegalomaniaCallbacks::initialize(
                runtime, native, room, boss,
            )?),
            ..Default::default()
        })
    }

    pub(super) fn initialize_cruelty(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<Self, ()> {
        Ok(Self {
            qualified_content: Some(native.source_digest()),
            taints: taints::CurrentTaints::for_room(runtime, room),
            taint_one: taint_one::TaintOne::at_birth(now_us),
            cruelty: Some(cruelty::CrueltyCallbacks::initialize(
                runtime, native, room, boss,
            )?),
            ..Default::default()
        })
    }

    pub(super) fn initialize_hatred(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<Self, ()> {
        Ok(Self {
            qualified_content: Some(native.source_digest()),
            taints: taints::CurrentTaints::for_room(runtime, room),
            taint_one: taint_one::TaintOne::at_birth(now_us),
            hatred: Some(hatred::HatredCallbacks::initialize(
                runtime, native, room, boss,
            )?),
            ..Default::default()
        })
    }

    /// The lethal owner calls before corpse projection can retire this actor.
    /// This is observation DATA only. The canonical copied SourceDeath event
    /// owns the counter and spawn schedule; this slot never orders or drains work.
    pub(super) fn record_committed_death(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        now_ms: u64,
    ) {
        if self.qualified_content != Some(runtime.content_pin().server_artifact_digest())
            || runtime.owner_fence().is_err()
            || !runtime
                .companion_snapshot_including_dead(actor)
                .is_ok_and(|view| view.health == 0)
        {
            return;
        }
        if let Some(entry) = self
            .observed
            .iter()
            .find(|entry| entry.actor == actor && !entry.death_observed)
            && (entry.minion.is_some() || entry.sphere)
        {
            let mut notice = entry
                .death_at_ms
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            notice.get_or_insert(now_ms);
        }
    }
}

fn at(position: RoomPosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}

fn sphere_cell_has_boss(
    runtime: &ChannelRuntimeV1,
    boss: ExactActorRef,
    west: MovementLocalPosition,
) -> bool {
    // Source scans getCreatures(), so a later player on this tile cannot hide
    // the independently source-qualified exact live boss incarnation.
    runtime.contains_live_creature(boss)
        && runtime
            .read_actor_position(boss)
            .is_ok_and(|position| position.position() == west)
}

fn in_greed(position: MovementLocalPosition) -> bool {
    position.floor == 14
        && (33737..=33755).contains(&position.x)
        && (31658..=31673).contains(&position.y)
}

fn soul_damage(root: [u8; 32], actor: ExactActorRef, sequence: u64, ordinal: u64) -> u32 {
    // The source root retains all 256 bits of the bound content/actor digest.
    // The 128-bit occurrence stores the full two u64 counters without truncation.
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:soulsnatcher:v1")
            .chain_update(root)
            .chain_update(actor.placement_identity())
            .finalize()
            .into(),
    );
    let mut occurrence = [0; 16];
    occurrence[..8].copy_from_slice(&sequence.to_be_bytes());
    occurrence[8..].copy_from_slice(&ordinal.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    // Rejection sampling preserves uniform 500..=1000; modulo alone biases the
    // source's inclusive integer draw. A failed derivation refuses the callback.
    let limit = u64::MAX - u64::MAX % 501;
    for draw in 0..64 {
        let Ok(value) =
            deterministic_decision_u64(&root, occurrence, "soul_war_soulsnatcher", draw)
        else {
            return 0;
        };
        if value < limit {
            return 500 + (value % 501) as u32;
        }
    }
    0
}

pub(super) fn source_actor(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    actor: ExactActorRef,
) -> Option<String> {
    let current = runtime.companion_snapshot_including_dead(actor).ok()?;
    if current.state.master.is_some()
        || runtime.native_summon_master(actor).is_some()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
    {
        return None;
    }
    native
        .creature_profiles()
        .records
        .iter()
        .find(|record| {
            record.profile.target.key == current.state.policy.definition_key
                && record.profile.target.revision == current.state.policy.definition_revision
        })
        .map(|record| record.profile.target.key.clone())
}

pub(super) fn qualifies_source(runtime: &ChannelRuntimeV1, native: &NativeGameplayState) -> bool {
    native.qualifies_soul_war_callbacks(runtime.content_pin().server_artifact_digest())
}
#[cfg(test)]
fn qualifies_document(
    document: &crate::content::native_gameplay::CreatureProfilesDocument,
) -> bool {
    crate::content::native_soul_war_callbacks::qualifies_document(document)
}

/// Called with the same runtime/spell/attack locks and current Map transaction as
/// native scheduled attacks. `occurrences` is the actual reserved native think batch.
#[allow(clippy::too_many_arguments)]
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    _attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    now: u64,
    execution_now: u64,
    messages: &super::raid_notices::ServerMessageHub,
) -> Result<(), ()> {
    if now > execution_now {
        return Err(());
    }
    taint_one::drive_due_through(runtime, states, room, native, map, now, execution_now).await?;
    torment::drive(
        runtime,
        states,
        room,
        native,
        map,
        occurrences,
        messages,
        now,
        execution_now,
    )
    .await?;
    if room.pinned_activity_source_room()
        == Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMalice)
    {
        return super::soul_war_malice::drive(
            runtime,
            states,
            room,
            native,
            map,
            occurrences,
            now,
            execution_now,
        )
        .await;
    }
    if room.pinned_activity_source_room()
        == Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsHatred)
    {
        return hatred::drive(runtime, states, room, native, occurrences);
    }
    if room.pinned_activity_source_room()
        == Some(
            crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsCruelty,
        )
    {
        return cruelty::drive(runtime, states, native, map, occurrences).await;
    }
    if room.pinned_activity_source_room()==Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMegalomania) {
        return megalomania::drive(runtime,states,room,native,map,occurrences,now,execution_now).await;
    }
    greed::drive(
        runtime,
        states,
        room,
        native,
        map,
        occurrences,
        execution_now,
    )
    .await
}

/// Crystal map.cpp:234-319: centre first, then separately shuffled
/// extended groups; nearby cells require PZ preservation, no teleport and LOS.
#[allow(clippy::too_many_arguments)]
pub(super) async fn spawn_source_child(
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &oteryn_gameplay_entities::carrier::soul_war::SoulWarSpawnSource,
    boss: ExactActorRef,
    child: &str,
    center: MovementLocalPosition,
    forced: bool,
    now: u64,
    event: u64,
) -> Option<ExactActorRef> {
    if forced
        != matches!(
            child,
            greed_model::SOUL_SPHERE | "oteryn:creature.soul_cage"
        )
    {
        return None;
    }
    let center_admits = map.read(runtime, center).await;
    let source_center_exists = room
        .movement_cells()
        .spell_tiles()
        .lookup(
            room.movement_cells().scope(),
            crate::content::LogicalCell {
                x: center.x,
                y: center.y,
                z: i32::from(center.floor),
            },
        )
        .is_ok();
    if (center_admits == Some(true) || (forced && center_admits.is_some() && source_center_exists))
        && (forced || runtime.native_summon_cell_free(center))
    {
        if !map.validate_transaction(runtime).await {
            return None;
        }
        return runtime
            .spawn_soul_war_child_from_source(source, child, center, |r, p| {
                map.view()
                    .matches(r)
                    .then(|| forced || map.view().admits(p))
            })
            .ok();
    }
    let cells = room.movement_cells();
    let pz = |at: MovementLocalPosition| {
        cells
            .spell_tiles()
            .lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            )
            .ok()
            .map(|tile| tile.flags().protection_zone)
    };
    let center_pz = pz(center)?;
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:greed:extended-placement:v1")
            .chain_update(native.source_digest())
            .chain_update(boss.placement_identity())
            .chain_update(child.as_bytes())
            .finalize()
            .into(),
    );
    let mut occurrence = [0; 16];
    occurrence[..8].copy_from_slice(&now.to_be_bytes());
    occurrence[8..].copy_from_slice(&event.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let mut ordinal = 0;
    let offsets = greed_model::extended_spawn_offsets(|bound| {
        let limit = u64::MAX - u64::MAX % u64::from(bound);
        for _ in 0..64 {
            let draw =
                deterministic_decision_u64(&root, occurrence, "greed_spawn_placement", ordinal);
            ordinal += 1;
            if let Ok(value) = draw
                && value < limit
            {
                return (value % u64::from(bound)) as u32;
            }
        }
        bound // The pure owner rejects a failed deterministic draw.
    })
    .ok()?;
    for (dx, dy) in offsets {
        let destination = MovementLocalPosition {
            x: center.x.checked_add(dx)?,
            y: center.y.checked_add(dy)?,
            floor: center.floor,
        };
        if (center_pz && pz(destination) != Some(true))
            || crate::map::floor::is_teleport(
                cells,
                crate::content::LogicalCell {
                    x: destination.x,
                    y: destination.y,
                    z: i32::from(destination.floor),
                },
            )
            || map.read(runtime, destination).await != Some(true)
            || !runtime.native_summon_cell_free(destination)
        {
            continue;
        }
        map.read_sight(runtime, center, destination).await;
        if !map.view().matches(runtime) || !map.view().sight(center, destination) {
            continue;
        }
        if !map.validate_transaction(runtime).await {
            return None;
        }
        if let Ok(actor) =
            runtime.spawn_soul_war_child_from_source(source, child, destination, |r, p| {
                map.view().matches(r).then(|| map.view().admits(p))
            })
        {
            return Some(actor);
        }
    }
    None
}

fn observe(
    owner: &mut SoulWarCallbacks,
    actor: ExactActorRef,
    minion: Option<GreedMinion>,
    sphere: bool,
) -> Result<(), ()> {
    if owner.observed.iter().any(|entry| entry.actor == actor) {
        return Ok(());
    }
    owner.next_id = owner.next_id.checked_add(1).ok_or(())?;
    owner.observed.push(Observed {
        actor,
        id: owner.next_id,
        minion,
        sphere,
        death_observed: false,
        death_at_ms: std::sync::Mutex::new(None),
        last_think: None,
    });
    if let Some(minion) = minion {
        owner.greed.minion_spawned(minion);
    }
    if sphere {
        owner.greed.sphere_spawned(owner.next_id);
    }
    Ok(())
}

pub(super) fn change(
    runtime: &mut ChannelRuntimeV1,
    boss: ExactActorRef,
    change: Change,
) -> Result<(), ()> {
    if !runtime.contains_live_creature(boss) {
        return Ok(());
    }
    let current = runtime.companion_snapshot(boss).map_err(|_| ())?;
    runtime
        .change_soul_war_creature(&current, change)
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srx3_soul_sphere_finds_boss_beneath_current_player_stack()
    -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, player, _) =
            super::super::actor_spell::tests::runtime_with_capacity(180, 4);
        let west = at(greed_model::BOSS_SPHERE_SPOT);
        let boss = runtime
            .admit_pinned_monster_lab_creature(west, greed_model::GOSHNARS_GREED, 20)
            .map_err(|e| format!("{e:?}"))?;
        runtime
            .initialize_pinned_test_position(player, west)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            runtime
                .current_top_actor_on_cell(west)
                .map_err(|e| format!("{e:?}"))?,
            Some(player)
        );
        assert!(
            sphere_cell_has_boss(&runtime, boss, west),
            "source getCreatures scans all current creatures, not only the top actor"
        );
        Ok(())
    }

    #[test]
    fn srx3_soul_war_witness_accepts_actual_native_closure_and_rejects_changed_body()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .name("srx3-soul-war-source".into())
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = oteryn_protocol_oteryn::WorldId::decode(&[
                    1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
                ])
                .map_err(|error| format!("{error:?}"))?;
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                assert!(
                    qualifies_document(native.creature_profiles()),
                    "real artifact closure must qualify"
                );
                let mut changed = native.creature_profiles().clone();
                changed.source_definitions_sha256 = Some("0".repeat(64));
                assert!(!qualifies_document(&changed));
                let mut changed = native.creature_profiles().clone();
                let record = changed
                    .records
                    .iter_mut()
                    .find(|record| record.profile.target.key == greed_model::GOSHNARS_GREED)
                    .ok_or("actual Greed source profile missing")?;
                let definitions = record
                    .attack_definitions
                    .as_mut()
                    .ok_or("actual source attack closure missing")?;
                let profile = definitions
                    .profiles
                    .first_mut()
                    .ok_or("source attack profile missing")?;
                profile.target.revision.push_str("-substituted");
                assert!(!qualifies_document(&changed));
                Ok(())
            })?
            .join()
            .map_err(|_| "source witness test thread failed")??;
        Ok(())
    }
}
