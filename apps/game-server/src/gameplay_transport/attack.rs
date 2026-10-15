//! ATTACK-1b: the Channel owner's attack-target state and auto-attack swing timer (ATTACK-0 §3,
//! §4, §9; `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! One entry per (player actor, session): the held creature target, the runtime-only fight modes
//! and the in-fight deadline. A due swing is one `AutoAttack` GAME-ABILITY-01 damage plan
//! committed by the physical owner under the swing identity `(character, lease generation,
//! session, lineage, swing ordinal)`: the lineage is the `CommandRef` of the `ATTACK_TARGET`
//! command that set the target and the ordinal counts the swings of that lineage from 0.
//! ATTACK-2: the swing uses the held melee weapon's attack (fists otherwise), and the creature
//! meets it with its block budget, its source defence and its armor (§4, §5).
//! Player targets (PvP) and distance weapons are out of scope. Server-driven
//! chase (CHASE-1) is not implemented: today the server only holds the out-of-range target and never
//! steps the player, so `CHASE` behaves like `STAND`; owner/CP decision (LOOP-COMBAT-MODES-1) keeps
//! chase client-driven (#1942) until the follow-up. `secure` has no creature-target effect before PvP.

use crate::foundation::encode_state_delta;
use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
    FightMode as WireFightMode, FightModes, MAX_ATTACK_TARGET_CHANGES_PER_SECOND,
    MAX_FIGHT_MODE_CHANGES_PER_SECOND, STATE_DOMAIN_ACTOR_COMBAT_STATE, encode_actor_combat_state,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use tokio::time::{Duration, Instant};

use super::ComposedFreshAdmission;
use crate::ability::exact_actor_resolution::{ExactActorProposal, resolve_exact_actor};
use crate::ability::{
    AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource, RevisionSet,
};
use crate::combat::charm_effects::{
    CharmAssignment, CharmAttackerFacts, CharmCatalogueRead, CharmEvaluationInput, CharmHitSource,
    CharmHookEvent, CharmStateRead, CharmStateUnavailable, evaluate_charm_hook,
};
use crate::combat_attack::{
    AttackConstants, AttackState, BlockBudget, FightMode, SwingPoll, TargetFacts, TargetRefusal,
    armor_reduction_bounds, player_fist_formula, player_melee_formula,
};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{
    LogicalCell, QualifiedNativeEntryRoom, ReferenceItemField, ReferenceWeaponType,
};
use crate::durability::character_equipment::EquippedItem;
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
use crate::spell::combat_execution::actor_atom;
use crate::spell::formula::FormulaInputs;

/// `ATTACK0-RL-01` and `ATTACK0-RL-02`: intents per [`ATTACK_INTENT_WINDOW`].
const MAX_INTENTS_PER_WINDOW: usize = 25;
const _: () = assert!(
    MAX_INTENTS_PER_WINDOW as u32 == MAX_ATTACK_TARGET_CHANGES_PER_SECOND
        && MAX_INTENTS_PER_WINDOW as u32 == MAX_FIGHT_MODE_CHANGES_PER_SECOND
);
pub(crate) const ATTACK_INTENT_WINDOW: Duration = Duration::from_secs(1);
/// How often a connection with domain 10 runs the owner's swing timer and looks for a changed
/// combat state.
pub(crate) const COMBAT_REFRESH: Duration = Duration::from_millis(250);

/// The sliding window of one intent limit: the instants of the last intents. `Copy` so it
/// travels with the session continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IntentWindow([Option<Instant>; MAX_INTENTS_PER_WINDOW]);

impl IntentWindow {
    pub(crate) const EMPTY: Self = Self([None; MAX_INTENTS_PER_WINDOW]);

    /// Whether one more intent fits the window ending at `now`; if so it is recorded.
    pub(crate) fn admit(&mut self, now: Instant) -> bool {
        let free = self.0.iter_mut().find(|slot| {
            !matches!(slot, Some(at) if now.saturating_duration_since(*at) < ATTACK_INTENT_WINDOW)
        });
        let Some(slot) = free else {
            return false;
        };
        *slot = Some(now);
        true
    }
}

/// What one GameSession keeps of domain 10 across connections: the revision (monotonic per
/// GameSession), the last state sent and the two intent windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatContinuity {
    pub(crate) revision: u64,
    pub(crate) sent: Option<ActorCombatState>,
    pub(crate) target_intents: IntentWindow,
    pub(crate) mode_intents: IntentWindow,
}

impl CombatContinuity {
    pub(crate) const FRESH: Self = Self {
        revision: 0,
        sent: None,
        target_intents: IntentWindow::EMPTY,
        mode_intents: IntentWindow::EMPTY,
    };

    /// The domain 10 snapshot above every revision this session has seen; recorded only by
    /// [`Self::snapshot_sent`] once transmitted.
    pub(crate) fn snapshot(&self, state: &ActorCombatState) -> Option<(u64, Vec<u8>)> {
        let revision = self.revision.checked_add(1)?;
        Some((revision, encode_actor_combat_state(state).ok()?))
    }

    pub(crate) fn snapshot_sent(&mut self, revision: u64, state: ActorCombatState) {
        self.revision = revision;
        self.sent = Some(state);
    }

    /// The domain 10 delta frame to `state` at the sequence after `sequence` when it differs
    /// from the last state sent; `Some(None)` when unchanged, `None` on a fault. The revision
    /// advances before the write, so a revision that may have reached the client is never
    /// reused.
    pub(crate) fn delta(
        &mut self,
        generation: u64,
        sequence: u64,
        state: ActorCombatState,
    ) -> Option<Option<(u64, Vec<u8>)>> {
        if self.sent == Some(state) {
            return Some(None);
        }
        let from = self.revision;
        let to = from.checked_add(1)?;
        let delta_sequence = sequence.checked_add(1)?;
        let payload = encode_actor_combat_state(&state).ok()?;
        let frame = encode_state_delta(
            generation,
            delta_sequence,
            STATE_DOMAIN_ACTOR_COMBAT_STATE,
            from,
            to,
            DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
            &payload,
        )
        .ok()?;
        self.revision = to;
        self.sent = Some(state);
        Some(Some((delta_sequence, frame)))
    }
}

/// How often the attacker's own connection re-reads its swing weapon while it holds a target.
pub(crate) const WEAPON_REFRESH_MICROS: u64 = 1_000_000;

/// The weapon a swing is drawn with (ATTACK-0 §4): fists, or a melee weapon's attack value. A
/// distance weapon, a throwing weapon, a wand or a rod is no weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwingWeapon {
    Fists,
    Melee { attack: u32 },
}

/// The swing weapon read from the equipment owner under `content_digest` at `read_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WeaponFacts {
    weapon: SwingWeapon,
    content_digest: [u8; 32],
    read_at: SemanticTimeMicros,
}

/// The swing weapon of `items` (`player.cpp:405-464` `getWeapon`): the first hand, left (6) then
/// right (5), that holds a weapon other than a shield, ammunition or a spellbook. A club, sword,
/// axe or fist weapon swings with its attack; any other weapon, or none, swings with fists.
/// `None` (fail closed) when a hand item's policy is unknown or its weapon facts are not Known.
pub(crate) fn swing_weapon(
    content: &NativeGameplayState,
    items: &[EquippedItem],
) -> Option<SwingWeapon> {
    for slot in [6_u8, 5] {
        let Some(item) = items.iter().find(|item| item.slot == slot) else {
            continue;
        };
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        let weapon = match &policy.record().semantics.weapon {
            ReferenceItemField::NotApplicable => continue,
            ReferenceItemField::Known(weapon) => weapon,
            ReferenceItemField::Unknown | ReferenceItemField::Conflict => return None,
        };
        let ReferenceItemField::Known(kind) = weapon.weapon_type else {
            return None;
        };
        match kind {
            ReferenceWeaponType::Shield
            | ReferenceWeaponType::Ammunition
            | ReferenceWeaponType::Spellbook => continue,
            ReferenceWeaponType::Distance | ReferenceWeaponType::Wand => {
                return Some(SwingWeapon::Fists);
            }
            ReferenceWeaponType::Club
            | ReferenceWeaponType::Sword
            | ReferenceWeaponType::Axe
            | ReferenceWeaponType::Fist => {
                let ReferenceItemField::Known(attack) = weapon.attack else {
                    return None;
                };
                return Some(SwingWeapon::Melee {
                    attack: u32::try_from(attack.0).ok()?,
                });
            }
        }
    }
    Some(SwingWeapon::Fists)
}

/// The attack state of one player actor in one session.
#[derive(Debug, Clone)]
struct ActorAttack {
    actor: ExactActorRef,
    session: GameSessionId,
    state: AttackState<ExactActorRef, CommandRef>,
    modes: FightModes,
    /// The wire reference of the held target, as the client named it.
    target_ref: Option<EntityRef>,
    /// The swing ordinal of the next swing of the held target's lineage.
    next_ordinal: u16,
    /// The last swing weapon read; a swing waits while it is missing or stale.
    weapon: Option<WeaponFacts>,
    /// Bumped by every equipment move, so a read that raced one is not stored.
    weapon_epoch: u64,
}

impl ActorAttack {
    fn clear_target(&mut self) {
        self.state.clear_target();
        self.target_ref = None;
    }
}

/// The Channel owner's attack entries. Locked after `runtime` and `spell_states`, never before.
#[derive(Debug, Default)]
pub(crate) struct ChannelAttackStates {
    entries: Vec<ActorAttack>,
    /// The block budget of each live creature a player has hit (ATTACK-0 §5), created at its
    /// first hit.
    blocks: Vec<(ExactActorRef, BlockBudget)>,
    /// The Channel's unsettled creature kills (ARCH-KILL-REWARD-LOGOUT-1 §1.3).
    kills: super::kill_reward::KillSettlementQueue,
}

impl ChannelAttackStates {
    pub(crate) const fn kills(&self) -> &super::kill_reward::KillSettlementQueue {
        &self.kills
    }
}

/// CONDITIONS-0 §3.5: the actor's runtime condition slot holds a current `pacified`. An
/// actor without a slot holds no condition there.
fn pacified_slot(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    now: SemanticTimeMicros,
) -> bool {
    runtime
        .actor_conditions(actor, Some(session))
        .is_ok_and(|store| store.attack_target_gate(now.get()).is_err())
}

/// Why an `ATTACK_TARGET` intent was refused before any owner state changed.
fn refusal_disposition(refusal: TargetRefusal) -> AttackIntentDisposition {
    match refusal {
        TargetRefusal::NotVisible | TargetRefusal::Dead => {
            AttackIntentDisposition::TargetNotVisible
        }
        TargetRefusal::NotAttackableKind => AttackIntentDisposition::TargetNotACreature,
        TargetRefusal::TargetInProtectionZone | TargetRefusal::AttackerInProtectionZone => {
            AttackIntentDisposition::ProtectionZone
        }
        TargetRefusal::ReentryProtected | TargetRefusal::TargetReentryProtected => {
            AttackIntentDisposition::ReentryProtected
        }
    }
}

const fn fight_mode(mode: WireFightMode) -> FightMode {
    match mode {
        WireFightMode::Offensive => FightMode::Offensive,
        WireFightMode::Balanced => FightMode::Balanced,
        WireFightMode::Defensive => FightMode::Defensive,
    }
}

fn in_protection_zone(
    room: &QualifiedNativeEntryRoom,
    position: MovementLocalPosition,
) -> Option<bool> {
    let cells = room.movement_cells();
    cells
        .spell_tiles()
        .lookup(
            cells.scope(),
            LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            },
        )
        .ok()
        .map(|tile| tile.flags().protection_zone)
}

/// Whether an attacker at `from` sees `to` under the reference view. A source world's native
/// floor sees as the Tibia floor of its negation (`VisibilityPosition`).
pub(super) fn sees(from: MovementLocalPosition, to: MovementLocalPosition) -> bool {
    match (
        VisibilityPosition::new(from.x, from.y, from.floor),
        VisibilityPosition::new(to.x, to.y, to.floor),
    ) {
        (Ok(observer), Ok(seen)) => VisibilitySettings::REFERENCE.can_see(observer, seen),
        _ => false,
    }
}

/// The §3/§4 facts of `target` for `attacker` at `now`. `None` when the attacker's own facts
/// cannot be read; the caller fails closed.
fn target_facts(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    attacker: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
    now: SemanticTimeMicros,
) -> Option<TargetFacts> {
    let attacker_snapshot = runtime.read_actor_position(attacker).ok()?;
    let from = attacker_snapshot.position();
    let attacker_in_protection_zone = in_protection_zone(room, from)?;
    let attacker_reentry_protected = runtime
        .current_player_reentry_protection(attacker, session, now.get())
        .ok()?;
    let absent = TargetFacts {
        present_and_visible: false,
        attackable_kind: false,
        alive: false,
        same_floor: false,
        distance: u32::MAX,
        attacker_in_protection_zone,
        target_in_protection_zone: false,
        attacker_reentry_protected,
        target_reentry_protected: false,
    };
    if !runtime.contains_live_creature(target) {
        return Some(absent);
    }
    let Ok(target_snapshot) = runtime.read_actor_position(target) else {
        return Some(absent);
    };
    let to = target_snapshot.position();
    let visible = target_snapshot.context() == attacker_snapshot.context() && sees(from, to);
    let (attackable_kind, alive) = match runtime.companion_snapshot(target) {
        Ok(snapshot) => (
            snapshot.state.master.is_none() && !snapshot.state.policy.is_familiar,
            snapshot.health > 0,
        ),
        Err(_) => (true, false),
    };
    let dx = (i64::from(to.x) - i64::from(from.x)).unsigned_abs();
    let dy = (i64::from(to.y) - i64::from(from.y)).unsigned_abs();
    Some(TargetFacts {
        present_and_visible: visible,
        attackable_kind,
        alive,
        same_floor: to.floor == from.floor,
        distance: u32::try_from(dx.max(dy)).unwrap_or(u32::MAX),
        attacker_in_protection_zone,
        target_in_protection_zone: in_protection_zone(room, to)?,
        attacker_reentry_protected,
        target_reentry_protected: false,
    })
}

impl ChannelAttackStates {
    fn index(&self, actor: ExactActorRef, session: GameSessionId) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.actor == actor && entry.session == session)
    }

    /// Drop the entries whose actor is no longer controlled by its session's slot, and the
    /// block budgets of creatures no longer live.
    fn prune(&mut self, runtime: &ChannelRuntimeV1) {
        self.entries.retain(|entry| {
            runtime
                .player_control_facts(entry.actor, entry.session)
                .is_ok()
        });
        self.blocks
            .retain(|(creature, _)| runtime.contains_live_creature(*creature));
    }

    /// The equipment epoch of `actor` when it holds a target whose swing weapon is missing,
    /// read under another digest or older than [`WEAPON_REFRESH_MICROS`]; `None` when no read
    /// is due.
    fn weapon_read_due(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        digest: &[u8; 32],
        now: SemanticTimeMicros,
    ) -> Option<u64> {
        let entry = &self.entries[self.index(actor, session)?];
        entry.state.target()?;
        let fresh = entry.weapon.is_some_and(|facts| {
            &facts.content_digest == digest
                && now
                    .elapsed_since(facts.read_at)
                    .is_ok_and(|age| age < WEAPON_REFRESH_MICROS)
        });
        (!fresh).then_some(entry.weapon_epoch)
    }

    /// Store a swing weapon read that began at `epoch`; dropped when a move came since.
    fn store_weapon(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        epoch: u64,
        facts: WeaponFacts,
    ) {
        if let Some(index) = self.index(actor, session) {
            let entry = &mut self.entries[index];
            if entry.weapon_epoch == epoch {
                entry.weapon = Some(facts);
            }
        }
    }

    /// An equipment move of `actor`: its swing weapon must be read again.
    pub(crate) fn forget_weapon(&mut self, actor: ExactActorRef, session: GameSessionId) {
        if let Some(index) = self.index(actor, session) {
            let entry = &mut self.entries[index];
            entry.weapon = None;
            entry.weapon_epoch = entry.weapon_epoch.wrapping_add(1);
        }
    }

    /// The entry of a currently placed `actor` in `session`, created on first use.
    fn entry(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<&mut ActorAttack> {
        runtime.player_control_facts(actor, session).ok()?;
        let index = match self.index(actor, session) {
            Some(index) => index,
            None => {
                self.prune(runtime);
                // SPAWN-CEILINGS-1 (Codex 4242381774): one entry per controlled player slot,
                // which `prune` keeps, so the carrier's slots bound it; the old
                // `AI01-ACTIVE-ACTORS` 256 refused the 257th player of a Channel built for 500.
                if self.entries.len() >= runtime.actor_slot_capacity() {
                    return None;
                }
                self.entries.push(ActorAttack {
                    actor,
                    session,
                    state: AttackState::default(),
                    modes: FightModes::DEFAULT,
                    target_ref: None,
                    next_ordinal: 0,
                    weapon: None,
                    weapon_epoch: 0,
                });
                self.entries.len() - 1
            }
        };
        self.entries.get_mut(index)
    }

    /// One `ATTACK_TARGET` intent (§3). `None` stops attacking. A new target starts the lineage
    /// of command `command_id` at ordinal 0; a refused target leaves the held one unchanged.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: Option<&QualifiedNativeEntryRoom>,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        target: Option<EntityRef>,
        command_id: u64,
    ) -> AttackIntentDisposition {
        let Some(entry) = self.entry(runtime, actor, session) else {
            return AttackIntentDisposition::Rejected;
        };
        let Some(target_ref) = target else {
            entry.clear_target();
            return AttackIntentDisposition::Ok;
        };
        // CONDITIONS-0 §3.5: `pacified` refuses setting a target before every other check.
        if pacified_slot(runtime, actor, session, now) {
            return AttackIntentDisposition::Rejected;
        }
        let Some(room) = room else {
            return AttackIntentDisposition::Rejected;
        };
        let Ok(command) = CommandId::new(command_id) else {
            return AttackIntentDisposition::Rejected;
        };
        let visible = runtime.visible_entities();
        let named = |actor: ExactActorRef, generation: u64| {
            actor.placement_identity() == target_ref.identity && generation == target_ref.generation
        };
        if visible
            .players
            .iter()
            .any(|player| named(player.actor, player.generation))
        {
            return AttackIntentDisposition::TargetNotACreature;
        }
        let Some(creature) = visible
            .creatures
            .iter()
            .find(|creature| named(creature.actor, creature.generation))
            .map(|creature| creature.actor)
        else {
            return AttackIntentDisposition::TargetNotVisible;
        };
        let Some(facts) = target_facts(runtime, room, actor, session, creature, now) else {
            return AttackIntentDisposition::Rejected;
        };
        match entry
            .state
            .set_target(creature, CommandRef::new(session, command), &facts)
        {
            Ok(()) => {
                entry.target_ref = Some(target_ref);
                entry.next_ordinal = 0;
                AttackIntentDisposition::Ok
            }
            Err(refusal) => refusal_disposition(refusal),
        }
    }

    /// One `FIGHT_MODES` intent: runtime-only, never persisted (§9).
    pub(crate) fn set_modes(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        modes: FightModes,
    ) -> bool {
        match self.entry(runtime, actor, session) {
            Some(entry) => {
                entry.modes = modes;
                true
            }
            None => false,
        }
    }

    /// The `ACTOR_COMBAT_STATE` of `actor` at `now`; the defaults when it holds no entry.
    pub(crate) fn combat_state(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> ActorCombatState {
        match self.index(actor, session).map(|index| &self.entries[index]) {
            Some(entry) => ActorCombatState {
                target: entry.target_ref,
                modes: entry.modes,
                in_fight: entry.state.in_fight(now),
            },
            None => ActorCombatState {
                target: None,
                modes: FightModes::DEFAULT,
                in_fight: false,
            },
        }
    }

    /// The running in-fight deadline of `actor` (`ATTACK0-RL-03`): a logout is refused and a
    /// closed client stays in the world until it ends.
    pub(crate) fn in_fight_until(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<SemanticTimeMicros> {
        let entry = &self.entries[self.index(actor, session)?];
        entry
            .state
            .in_fight(now)
            .then(|| entry.state.in_fight_until())
            .flatten()
    }

    /// Stop attacking (reconnect, control loss); the in-fight deadline is kept (§3 Reconnect).
    pub(crate) fn clear_target(&mut self, actor: ExactActorRef, session: GameSessionId) {
        if let Some(index) = self.index(actor, session) {
            self.entries[index].clear_target();
        }
    }

    /// Whether a creature may resolve `actor` as its melee target, and if so whether re-entry
    /// protection covers it. A closed client held in the world by its in-fight deadline
    /// (ATTACK0-RL-03) stays a target with no re-entry window, so a hit extends the hold; any
    /// other actor under control loss is no target.
    pub(crate) fn creature_target_protection(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<bool> {
        match runtime.current_player_reentry_protection(actor, session, now.get()) {
            Ok(protected) => Some(protected),
            Err(_) => {
                runtime
                    .player_control_facts(actor, session)
                    .ok()?
                    .control_loss?;
                self.in_fight_until(actor, session, now)?;
                Some(false)
            }
        }
    }

    /// A creature hit `actor`: the in-fight deadline runs from `now`.
    pub(crate) fn record_hit_taken(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) {
        let Some(constants) = AttackConstants::checked_in() else {
            return;
        };
        if let Some(entry) = self.entry(runtime, actor, session) {
            entry.state.record_hit(now, constants.in_fight_micros());
        }
    }
}

/// ATTACK-1b has no Charm state reader: every hook is evaluated against no assignments and its
/// outcomes are discarded. The Charm state wiring is a later stage.
struct NoCharmAssignments([u8; 16]);

impl CharmStateRead for NoCharmAssignments {
    fn character(&self) -> [u8; 16] {
        self.0
    }

    fn assignments_for_race(
        &self,
        _race_key: &str,
    ) -> Result<Vec<CharmAssignment>, CharmStateUnavailable> {
        Ok(Vec::new())
    }
}

/// Evaluate one Charm hook and discard its outcomes (see [`NoCharmAssignments`]).
pub(super) fn run_charm_hook(
    catalogue: &dyn CharmCatalogueRead,
    character: [u8; 16],
    race_key: &str,
    decision_root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    event: CharmHookEvent,
) {
    let _ = evaluate_charm_hook(
        &CharmEvaluationInput {
            character,
            race_key,
            decision_root,
            occurrence,
            event,
        },
        &NoCharmAssignments(character),
        catalogue,
    );
}

/// The decision occurrence of one swing event: unique per attacker, lineage, ordinal and tag.
fn swing_occurrence(
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    tag: &[u8],
) -> DecisionOccurrenceId {
    let hash = Sha256::new()
        .chain_update(b"oteryn:auto-attack-swing:v1")
        .chain_update(attacker.placement_identity())
        .chain_update(lineage.game_session_id().as_bytes())
        .chain_update(lineage.command_id().get().to_be_bytes())
        .chain_update(ordinal.to_be_bytes())
        .chain_update(tag)
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    DecisionOccurrenceId::from_bytes(bytes)
}

/// One draw in `[minimum, maximum]` of the swing's occurrence `tag` under `key`; a degenerate
/// range is its bound without a draw. `None` on an RNG fault.
#[allow(clippy::too_many_arguments)]
fn swing_draw(
    root: &GameplayDecisionRoot,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    tag: &[u8],
    key: &str,
    minimum: i64,
    maximum: i64,
) -> Option<i64> {
    if maximum <= minimum {
        return Some(minimum);
    }
    let draw = deterministic_decision_u64(
        root,
        swing_occurrence(attacker, lineage, ordinal, tag),
        key,
        0,
    )
    .ok()?;
    let span = u64::try_from(maximum - minimum).ok()?.saturating_add(1);
    Some(minimum.saturating_add(i64::try_from(draw % span).ok()?))
}

/// What the creature's block (when its budget allows), defence and armor leave of `damage`
/// (`creature.cpp:958-990`): defence draws in `[defense / 2, defense]` and, when it stops the
/// whole hit, armor is not reached; armor draws in [`armor_reduction_bounds`].
#[allow(clippy::too_many_arguments)]
fn creature_mitigated(
    root: &GameplayDecisionRoot,
    constants: &AttackConstants,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    damage: i64,
    blocked: bool,
    defense: u32,
    armor: u32,
) -> Option<i64> {
    let mut remaining = damage;
    if blocked {
        let defence = swing_draw(
            root,
            attacker,
            lineage,
            ordinal,
            b"defence-draw",
            "attack.defence",
            i64::from(defense / 2),
            i64::from(defense),
        )?;
        remaining = remaining.saturating_sub(defence);
        if remaining <= 0 {
            return Some(0);
        }
    }
    let (minimum, maximum) = armor_reduction_bounds(constants, armor);
    let reduction = swing_draw(
        root,
        attacker,
        lineage,
        ordinal,
        b"armor-draw",
        "attack.armor",
        i64::from(minimum),
        i64::from(maximum),
    )?;
    Some(remaining.saturating_sub(reduction).max(0))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The swing as one Ability plan: a client-proposed damage effect on the held target,
/// committed atomically by the Channel owner under the swing's identity. The target atom is the
/// creature slot's `target_identity` (its definition key), which the owner commit compares.
fn swing_plan(
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    magnitude: i64,
    content_digest: &[u8; 32],
    target_identity: &str,
    formula: &str,
) -> Option<EffectPlan> {
    let issuer = actor_atom(attacker);
    let target_atom = target_identity.to_owned();
    let occurrence_id = format!(
        "auto-attack:{}:{}:{}:{ordinal}",
        hex(&attacker.placement_identity()),
        hex(lineage.game_session_id().as_bytes()),
        lineage.command_id().get(),
    );
    let revisions = RevisionSet::new(
        "ruleset:attack-v1",
        &format!("content:{}", hex(content_digest)),
        "world:attack-v1",
        formula,
        "simulation:v1",
    )
    .ok()?;
    let occurrence = AbilityOccurrence::new(&occurrence_id, revisions).ok()?;
    let proposal = AbilityIntent::resolve(
        ProposalSource::Client,
        &issuer,
        &[&target_atom],
        &[&target_atom],
    )
    .ok()?;
    let effect = Effect::damage(&target_atom, magnitude).ok()?;
    let group = CommitGroup::atomic("channel-owner", &occurrence_id).ok()?;
    EffectPlan::immediate(occurrence, proposal, vec![effect], Vec::new(), group).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Read the swing weapon of this connection's `actor` from the equipment owner when its
    /// held target's swings need one ([`ChannelAttackStates::weapon_read_due`]). A failed read
    /// stores nothing, so the swings wait (fail closed).
    pub(in crate::gameplay_transport) async fn refresh_attack_weapon(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let now = self.owner_now();
        let digest = self
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        let Some(epoch) = self
            .attack
            .lock()
            .await
            .weapon_read_due(actor, session, &digest, now)
        else {
            return;
        };
        let Some(equipment) = super::equipment_move::read_current_equipment(self, session).await
        else {
            return;
        };
        if equipment.content_digest != digest || content.source_digest() != digest {
            return;
        }
        let Some(weapon) = swing_weapon(content, &equipment.items) else {
            return;
        };
        self.attack.lock().await.store_weapon(
            actor,
            session,
            epoch,
            WeaponFacts {
                weapon,
                content_digest: digest,
                read_at: now,
            },
        );
    }

    /// One owner pass over every held target: at most one swing per attacker (`DEADLINE_STATE`
    /// catch-up), never a backlog.
    pub(in crate::gameplay_transport) async fn drain_auto_attacks(&self) {
        let Some(room) = self.combat_room else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let Some(constants) = AttackConstants::checked_in() else {
            return;
        };
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let digest = runtime.content_pin().server_artifact_digest();
        if runtime.owner_fence().is_err()
            || content.source_digest() != digest
            || room.compiled().server_digest() != digest
        {
            return;
        }
        let states = self.spell_states.lock().await;
        let mut attack = self.attack.lock().await;
        attack.prune(&runtime);
        let ChannelAttackStates {
            entries,
            blocks,
            kills,
        } = &mut *attack;
        let sink = super::kill_reward::KillSink {
            queue: kills,
            rewards: self.reward_table(),
        };
        let root = GameplayDecisionRoot::from_bytes(digest);
        for entry in entries.iter_mut() {
            let Some(target) = entry.state.target() else {
                continue;
            };
            let Some(player) = states.get(&runtime, entry.actor, entry.session) else {
                continue;
            };
            if states.is_dead(entry.actor) {
                entry.clear_target();
                continue;
            }
            // CONDITIONS-0 §3.5: applying `pacified` clears a running target.
            if player
                .owned_conditions()
                .attack_target_gate(now.get())
                .is_err()
                || pacified_slot(&runtime, entry.actor, entry.session, now)
            {
                entry.clear_target();
                continue;
            }
            // SPELL-LOCK-2 §1.3: while the attacker's own slot is reserved by a pending spell
            // batch, its swing defers to a later turn; the held target and swing clock stay.
            if runtime.assert_actor_spell_unreserved(entry.actor).is_err() {
                continue;
            }
            // ATTACK-2: a swing waits for a swing weapon read under this pin within one attack
            // interval; the attacker's own connection refreshes it (`refresh_attack_weapon`).
            let Some(weapon) = entry.weapon.filter(|facts| {
                facts.content_digest == digest
                    && now
                        .elapsed_since(facts.read_at)
                        .is_ok_and(|age| age <= constants.attack_interval_micros())
            }) else {
                continue;
            };
            let character_facts = player.character_facts();
            let Some(facts) = target_facts(&runtime, room, entry.actor, entry.session, target, now)
            else {
                continue;
            };
            let swing =
                match entry
                    .state
                    .poll_swing(now, constants.attack_interval_micros(), &facts)
                {
                    SwingPoll::Swing(swing) => swing,
                    SwingPoll::Cleared(_) => {
                        entry.target_ref = None;
                        continue;
                    }
                    SwingPoll::NoTarget | SwingPoll::Waiting(_) | SwingPoll::NotDue { .. } => {
                        continue;
                    }
                };
            // The ordinal advances on every swing, a zero draw included, so every swing has its
            // own RNG occurrence; the lineage ends at the last ordinal.
            let ordinal = entry.next_ordinal;
            match ordinal.checked_add(1) {
                Some(next) => entry.next_ordinal = next,
                None => entry.clear_target(),
            }
            let Ok(lease) = runtime
                .borrow_exact_actor_commit()
                .bound_attacker_lease(entry.actor, swing.lineage)
            else {
                entry.clear_target();
                continue;
            };
            let character = *lease.character_id().as_bytes();
            let Ok(snapshot) = runtime.companion_snapshot(target) else {
                entry.clear_target();
                continue;
            };
            let race_key = snapshot.state.policy.definition_key.clone();
            // The creature's content defence and armor (ATTACK-0 §5, `monster.cpp:242-251`);
            // a creature without them takes no hit (fail closed).
            let (Some(defense), Some(armor)) =
                (snapshot.state.policy.defense, snapshot.state.policy.armor)
            else {
                continue;
            };
            let creature_max_health = u64::try_from(snapshot.maximum_health).unwrap_or(0);
            run_charm_hook(
                self.imported_charms,
                character,
                &race_key,
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"damage-calculation"),
                CharmHookEvent::AttackDamageCalculation {
                    source: CharmHitSource::CharacterAttack,
                },
            );
            let mode = constants.fight_mode(fight_mode(entry.modes.fight_mode));
            let (formula, attack_value, revision) = match weapon.weapon {
                SwingWeapon::Fists => (
                    player_fist_formula(constants),
                    constants.fist.attack,
                    "formula:player-fist",
                ),
                SwingWeapon::Melee { attack } => (
                    player_melee_formula(constants),
                    attack,
                    "formula:player-melee",
                ),
            };
            let inputs = FormulaInputs {
                level: character_facts.level,
                magic_level: character_facts.magic_level,
                base_power: None,
                attack_skill: constants.starting_skill,
                attack_value,
                attack_factor: mode.attack_factor,
                shielding_skill: constants.starting_skill,
                shield_defense: None,
            };
            let Ok((minimum, maximum)) = formula.bounds(&inputs) else {
                continue;
            };
            let Some(damage) = swing_draw(
                &root,
                entry.actor,
                swing.lineage,
                ordinal,
                b"damage-draw",
                "attack.damage",
                minimum,
                maximum,
            ) else {
                continue;
            };
            entry
                .state
                .record_hit(swing.at, constants.in_fight_micros());
            if damage <= 0 {
                continue;
            }
            let index = match blocks.iter().position(|(creature, _)| *creature == target) {
                Some(index) => index,
                None => {
                    if blocks.len() >= runtime.actor_slot_capacity() {
                        continue;
                    }
                    // A creature's budget refills from spawn, not from the first hit on it.
                    blocks.push((target, BlockBudget::settled(&constants.block, swing.at)));
                    blocks.len() - 1
                }
            };
            let blocked = blocks[index].1.try_block(&constants.block, swing.at);
            let Some(magnitude) = creature_mitigated(
                &root,
                constants,
                entry.actor,
                swing.lineage,
                ordinal,
                damage,
                blocked,
                defense,
                armor,
            ) else {
                continue;
            };
            if magnitude <= 0 {
                continue;
            }
            let Some(plan) = swing_plan(
                entry.actor,
                swing.lineage,
                ordinal,
                magnitude,
                &digest,
                &race_key,
                revision,
            ) else {
                continue;
            };
            let Ok(resolved) = resolve_exact_actor(
                &runtime.borrow_exact_actor_lookup(),
                plan.occurrence(),
                ExactActorProposal::client(target),
            ) else {
                entry.clear_target();
                continue;
            };
            // The owner clock of this turn, in ms, stamps the hit's contributor and, for a lethal
            // hit, the death time the kill reward reads (§1.2).
            let committed = crate::ability::commit::commit_exact_owner_swing_damage(
                &mut runtime.borrow_exact_actor_commit_at(now.get() / 1_000),
                &resolved,
                &plan,
                entry.actor,
                swing.lineage,
                ordinal,
            );
            let Ok(result) = committed else {
                entry.clear_target();
                continue;
            };
            run_charm_hook(
                self.imported_charms,
                character,
                &race_key,
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"committed-hit"),
                CharmHookEvent::committed_hit(
                    &result,
                    CharmHitSource::CharacterAttack,
                    CharmAttackerFacts {
                        level: character_facts.level,
                        max_health: u64::from(character_facts.max_health),
                        max_mana: u64::from(character_facts.max_mana),
                    },
                    creature_max_health,
                ),
            );
            if result.health_after <= 0 {
                let lethal = crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session: entry.session,
                    actor: entry.actor,
                };
                super::kill_reward::record_creature_kill(
                    &mut runtime,
                    sink,
                    target,
                    lethal,
                    now.get() / 1_000,
                );
                entry.clear_target();
            }
        }
    }
}

#[cfg(test)]
#[path = "../foundation/channel_owner_auto_attack_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "attack_gate_tests.rs"]
mod gate_tests;

/// SPELL-TARGET-1: a held target inside the spell range but outside the reference window is
/// out of sight (`TargetIllegal` in ordinary combat).
#[cfg(test)]
mod sees_tests {
    use super::*;

    fn at(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    #[test]
    fn reference_window_bounds_the_held_target() {
        let caster = at(100, 100);
        assert!(sees(caster, at(108, 107)));
        assert!(sees(caster, at(92, 94)));
        assert!(!sees(caster, at(100, 108)), "dy +8 is outside the window");
        assert!(!sees(caster, at(100, 93)), "dy -7 is outside the window");
        assert!(!sees(caster, at(110, 100)), "dx +10 is outside the window");
        assert!(!sees(caster, at(91, 100)), "dx -9 is outside the window");
    }
}

/// ATTACK-2: the swing weapon of the production Item policies, and the creature's block,
/// defence and armor (`player.cpp:405-464`, `creature.cpp:958-990`).
#[cfg(test)]
#[allow(clippy::expect_used)]
mod weapon_tests {
    use super::*;
    use crate::durability::item_mint::TypedDefinitionRef;

    fn item(slot: u8, tibia_id: u32) -> EquippedItem {
        EquippedItem {
            slot,
            item_instance_id: [slot; 16],
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{tibia_id}"),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            state_revision: 1,
        }
    }

    /// The production generation's Item policies, loaded once for every case.
    fn production() -> crate::content::ContentActivationController {
        let world = crate::foundation::WorldId::decode(&uuid(1)).expect("world id");
        crate::content::native_gameplay::tests::activated_with_loot_tables(world, None).0
    }

    fn weapon_of(
        controller: &crate::content::ContentActivationController,
        items: &[(u8, u32)],
    ) -> Option<SwingWeapon> {
        let content = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .expect("the production generation");
        let items: Vec<_> = items
            .iter()
            .map(|(slot, tibia_id)| item(*slot, *tibia_id))
            .collect();
        swing_weapon(content, &items)
    }

    #[test]
    fn a_held_melee_weapon_swings_with_its_attack_and_anything_else_with_fists() {
        let production = production();
        let weapon_of = |items: &[(u8, u32)]| weapon_of(&production, items);
        // Sword (3264, attack 14), staff (3289, club, attack 10), wooden shield (3412),
        // spear (3277, distance), arrow (3447, ammunition), wand of darkness (25760).
        assert_eq!(weapon_of(&[]), Some(SwingWeapon::Fists));
        assert_eq!(
            weapon_of(&[(5, 3264)]),
            Some(SwingWeapon::Melee { attack: 14 })
        );
        assert_eq!(
            weapon_of(&[(6, 3289)]),
            Some(SwingWeapon::Melee { attack: 10 })
        );
        assert_eq!(
            weapon_of(&[(6, 3412), (5, 3264)]),
            Some(SwingWeapon::Melee { attack: 14 }),
            "a shield is skipped for the other hand"
        );
        assert_eq!(
            weapon_of(&[(6, 3289), (5, 3264)]),
            Some(SwingWeapon::Melee { attack: 10 }),
            "the left hand is read first"
        );
        assert_eq!(weapon_of(&[(6, 3412)]), Some(SwingWeapon::Fists));
        assert_eq!(weapon_of(&[(5, 3277)]), Some(SwingWeapon::Fists));
        assert_eq!(weapon_of(&[(5, 25760)]), Some(SwingWeapon::Fists));
        assert_eq!(
            weapon_of(&[(6, 3447), (5, 3264)]),
            Some(SwingWeapon::Melee { attack: 14 }),
            "ammunition is skipped"
        );
        assert_eq!(
            weapon_of(&[(10, 3264)]),
            Some(SwingWeapon::Fists),
            "only the hands are read"
        );
        // A hand item without Known weapon facts or policy fails closed: poison field (105)
        // has `weapon: UNKNOWN`; 1 names no policy.
        assert_eq!(weapon_of(&[(5, 105)]), None);
        assert_eq!(weapon_of(&[(6, 1)]), None);
    }

    fn uuid(tag: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    fn lineage() -> CommandRef {
        CommandRef::new(
            GameSessionId::decode(&uuid(3)).expect("session id"),
            CommandId::new(1).expect("command id"),
        )
    }

    fn mitigated(ordinal: u16, damage: i64, blocked: bool, defense: u32, armor: u32) -> i64 {
        let constants = AttackConstants::checked_in().expect("constants");
        creature_mitigated(
            &GameplayDecisionRoot::from_bytes([7; 32]),
            constants,
            ExactActorRef::transport_fixture(
                crate::foundation::WorldId::decode(&uuid(1)).expect("world id"),
                crate::foundation::ChannelId::decode(&uuid(2)).expect("channel id"),
            ),
            lineage(),
            ordinal,
            damage,
            blocked,
            defense,
            armor,
        )
        .expect("draws")
    }

    #[test]
    fn defence_applies_only_to_a_blocked_hit_and_armor_draws_in_its_bounds() {
        for ordinal in 0..64 {
            assert_eq!(
                mitigated(ordinal, 10, false, 1_000, 0),
                10,
                "no block, no armor"
            );
            assert_eq!(
                mitigated(ordinal, 10, true, 1_000, 0),
                0,
                "defence stops the hit"
            );
            assert_eq!(
                mitigated(ordinal, 10, false, 0, 2),
                9,
                "low armor reduces by 1"
            );
            let defended = mitigated(ordinal, 100, true, 10, 0);
            assert!((90..=95).contains(&defended), "defence draws in [5, 10]");
            let armored = mitigated(ordinal, 100, false, 0, 10);
            assert!((91..=95).contains(&armored), "armor 10 draws in [5, 9]");
            assert_eq!(mitigated(ordinal, 3, false, 0, 10), 0, "armor never heals");
        }
        assert_eq!(
            mitigated(3, 100, true, 10, 10),
            mitigated(3, 100, true, 10, 10),
            "every draw is a pure function of the swing"
        );
    }
}
