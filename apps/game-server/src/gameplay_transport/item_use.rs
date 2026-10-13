//! ITEM-USE-1c (ITEM-USE-0 §5, §6): the Channel owner's use of food and potions from the main
//! backpack. PREPARE checks the requirements, the gate (one use in flight, `ITEMUSE0-RL-01`, the
//! cooldowns) and `FULL` in the owner lane and starts the cooldown; then the one-item DUR-03
//! transaction commits, and only a known commit applies the effect.

use std::time::Duration;

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use super::ComposedFreshAdmission;
use super::actor_spell::ChannelSpellStates;
use super::connection::{EarnedNotice, UseCommand, UseOutcome};
use super::item_view::ItemKey;
use super::world_object::UseDisposition;
use crate::content::native_gameplay::{ItemProfileRecord, NativeGameplayState};
use crate::content::{
    ActiveGeneration, ReferenceBaseVocation, ReferenceItemConsumption, ReferenceItemField,
    ReferenceItemTarget, ReferenceItemUseRequirements, ReferencePotionRestore,
    ReferenceRestoreResource,
};
use crate::durability::character_authority::SERVER_BUILD_ID;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    BackpackEntry, CharacterBackpack, GAMEITEM01_STACK_QUANTITY_MAX,
};
use crate::durability::item_use::{
    ItemUseError, ItemUseFlaskPlan, ItemUseKind, ItemUseOutcome, ItemUseRequest,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::interaction_chest_use::entry_chest;
use crate::spell::Vocation;
use crate::spell::actor_conditions::eat_food;
use crate::spell::cast::CharacterCastFacts;

/// `ITEMUSE0-RL-01`: item uses per actor per second.
pub(super) const ITEMUSE0_RL_01_USES_PER_SECOND: usize = 5;
/// `ITEMUSE0-RL-04`: the ambiguous-commit bound before the in-flight slot is freed.
pub(super) const ITEMUSE0_RL_04_AMBIGUOUS_MS: u64 = 2_000;
/// The `potion` cooldown, shared by every potion. `PARITY_PENDING`: no official value.
const POTION_COOLDOWN_MS: u64 = 1_000;
/// The `food` cooldown. `PARITY_PENDING`: no official value.
const FOOD_COOLDOWN_MS: u64 = 200;
/// The main backpack's entry ceiling that `commit_item_use` holds a fresh flask entry to.
const MAIN_BACKPACK_ENTRIES: usize = 20;

/// The item-use state of one present actor; runtime state only (ITEM-USE-0 §9 item 3).
#[derive(Debug, Default)]
struct ItemUseGate {
    /// Owner-clock milliseconds at which each cooldown ends. Both start at PREPARE and stay after
    /// a failed commit (§5), a deliberate deviation from Canary. `PARITY_PENDING`.
    food_ready_ms: u64,
    potion_ready_ms: u64,
    /// The PREPARE time of the one use in flight.
    in_flight_since_ms: Option<u64>,
    /// The owner-clock times of the attempts admitted in the last second (`ITEMUSE0-RL-01`).
    recent_ms: Vec<u64>,
    /// The items reserved by a use whose commit is ambiguous: unspendable until reconciliation,
    /// the next admission of the Character, which rereads the durable state (the gate is keyed by
    /// the GameSession and goes with it).
    unspendable: Vec<[u8; 16]>,
    /// The last decided CommandId and its result: a replay of it returns that result.
    last: Option<(u64, UseDisposition)>,
}

impl ItemUseGate {
    /// Admits one attempt before any read: a replay, the one use in flight and `ITEMUSE0-RL-01`.
    fn admit(&mut self, command_id: u64, now_ms: u64) -> Result<(), UseDisposition> {
        if let Some((last, disposition)) = self.last
            && last == command_id
        {
            return Err(disposition);
        }
        if self
            .in_flight_since_ms
            .is_some_and(|since| now_ms.saturating_sub(since) < ITEMUSE0_RL_04_AMBIGUOUS_MS)
        {
            return Err(UseDisposition::Exhausted);
        }
        self.in_flight_since_ms = None;
        self.recent_ms
            .retain(|admitted| now_ms.saturating_sub(*admitted) < 1_000);
        if self.recent_ms.len() >= ITEMUSE0_RL_01_USES_PER_SECOND {
            return Err(UseDisposition::Exhausted);
        }
        self.recent_ms.push(now_ms);
        Ok(())
    }

    /// PREPARE: the cooldown of `kind` is checked and started and the use is put in flight.
    fn start(&mut self, kind: ItemUseKind, now_ms: u64) -> Result<(), UseDisposition> {
        if self.in_flight_since_ms.is_some() {
            return Err(UseDisposition::Exhausted);
        }
        let (ready, cooldown) = match kind {
            ItemUseKind::Food => (&mut self.food_ready_ms, FOOD_COOLDOWN_MS),
            ItemUseKind::Potion => (&mut self.potion_ready_ms, POTION_COOLDOWN_MS),
        };
        if now_ms < *ready {
            return Err(UseDisposition::Exhausted);
        }
        *ready = now_ms.saturating_add(cooldown);
        self.in_flight_since_ms = Some(now_ms);
        Ok(())
    }

    /// A known outcome frees the slot. An ambiguous one keeps it until `ITEMUSE0-RL-04` from
    /// PREPARE and leaves the reserved items unspendable.
    fn settle(&mut self, command_id: u64, disposition: UseDisposition, ambiguous: &[[u8; 16]]) {
        if ambiguous.is_empty() {
            self.in_flight_since_ms = None;
        }
        self.unspendable.extend_from_slice(ambiguous);
        self.last = Some((command_id, disposition));
    }
}

/// The item-use gates of the Channel's present actors, kept beside their vitals (and so locked
/// after `runtime`).
#[derive(Debug, Default)]
pub(crate) struct ItemUseGates(Vec<(ExactActorRef, GameSessionId, ItemUseGate)>);

impl ItemUseGates {
    /// The gate of the committed player of `session`. Gates of actors no longer present go.
    fn gate(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<&mut ItemUseGate> {
        runtime.player_control_facts(actor, session).ok()?;
        self.0
            .retain(|(present, held, _)| runtime.player_control_facts(*present, *held).is_ok());
        let index = match self
            .0
            .iter()
            .position(|(a, s, _)| *a == actor && *s == session)
        {
            Some(index) => index,
            None => {
                self.0.push((actor, session, ItemUseGate::default()));
                self.0.len() - 1
            }
        };
        self.0.get_mut(index).map(|(_, _, gate)| gate)
    }
}

/// What a known commit applies.
enum Effect {
    Food(u16),
    Potion(Vec<ReferencePotionRestore>),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn owner_ms(&self) -> u64 {
        u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// One `USE` of the main backpack direct entry `item` under capability 15.
    pub(super) async fn use_backpack_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> UseOutcome {
        let disposition = match self.item_use(actor, command, item).await {
            Ok(disposition) | Err(disposition) => disposition,
        };
        UseOutcome {
            disposition,
            committed: None,
            earned: EarnedNotice::NoneEarned,
        }
    }

    async fn item_use(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Result<UseDisposition, UseDisposition> {
        let ItemKey::Instance(source) = item else {
            return Err(UseDisposition::NothingToUse);
        };
        let fence = command.item_fence.ok_or(UseDisposition::Rejected)?;
        let session = command.game_session_id;
        // Lock order (SPELL-LOCK-2 §1.2): the lane before any Channel guard, held through the
        // item commit and its effect, so no spell stages or commits a successor of this actor's
        // state meanwhile. `None` is retryable with no side effect while the lane is fenced.
        let _permit = self
            .spell_lane_permit()
            .await
            .ok_or(UseDisposition::Rejected)?;
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.spell_states.lock().await;
            // A pending spell commit or a durable spell reserving this actor refuses the use
            // before anything is burned.
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, session)
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return Err(UseDisposition::Rejected);
            }
            let gate = states.item_uses.gate(&runtime, actor, session);
            gate.ok_or(UseDisposition::Rejected)?
                .admit(command.command_id, self.owner_ms())?;
        }
        let native = self
            .active_generation
            .and_then(ActiveGeneration::native_gameplay)
            .ok_or(UseDisposition::Rejected)?;
        let backpack = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .map_err(|_| UseDisposition::Rejected)?
            .ok_or(UseDisposition::StaleState)?;
        let entry = backpack
            .entries
            .iter()
            .find(|entry| entry.item.item_instance_id == source)
            .ok_or(UseDisposition::StaleState)?;
        let record = record(native, &entry.item.definition).ok_or(UseDisposition::NothingToUse)?;
        let (kind, effect, flask) = match &record.semantics.consumption {
            ReferenceItemField::Known(ReferenceItemConsumption::Food(food))
                if food.regeneration_seconds > 0 =>
            {
                let effect = Effect::Food(food.regeneration_seconds);
                (ItemUseKind::Food, effect, ItemUseFlaskPlan::None)
            }
            ReferenceItemField::Known(ReferenceItemConsumption::Potion(potion)) => {
                let flask = flask_plan(native, &backpack, entry, &potion.empty_flask)?;
                let effect = Effect::Potion(potion.restores.clone());
                (ItemUseKind::Potion, effect, flask)
            }
            _ => return Err(UseDisposition::NothingToUse),
        };
        let reserved: Vec<[u8; 16]> = std::iter::once(source)
            .chain(match &flask {
                ItemUseFlaskPlan::Stack {
                    item_instance_id, ..
                }
                | ItemUseFlaskPlan::New {
                    item_instance_id, ..
                } => Some(*item_instance_id),
                ItemUseFlaskPlan::None | ItemUseFlaskPlan::Missing => None,
            })
            .collect();
        let ids = crate::node::operator_files::uuid_v7()
            .and_then(|transaction| Ok((transaction, crate::node::operator_files::uuid_v7()?)));
        let (Ok((transaction_id, event_id)), Ok(occurred_at_unix_ms)) = (ids, unix_ms()) else {
            return Err(UseDisposition::Rejected);
        };
        let request = ItemUseRequest {
            kind,
            command_id: command.command_id,
            content_revision: entry_chest::CONTENT_REVISION.to_owned(),
            ruleset_revision: entry_chest::RULESET_REVISION.to_owned(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
            transaction_id,
            event_id,
            occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID.to_owned(),
            source_item_instance_id: source,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask,
        };
        // PREPARE (§5): requirements before any reservation, then FULL, then the cooldown.
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.spell_states.lock().await;
            let state = (!states.is_dead(actor))
                .then(|| states.get(&runtime, actor, session))
                .flatten()
                .ok_or(UseDisposition::Rejected)?;
            if !requirements_met(&record.semantics.use_requirements, state.character_facts()) {
                return Err(UseDisposition::RequirementNotMet);
            }
            // FOOD-REGEN-1: eating that would reach the cap is refused; nothing is burned.
            if let Effect::Food(seconds) = effect
                && eat_food(&mut state.clone(), seconds, self.owner_ms()).is_err()
            {
                return Err(UseDisposition::Full);
            }
            let gate = states.item_uses.gate(&runtime, actor, session);
            let gate = gate.ok_or(UseDisposition::Rejected)?;
            if reserved.iter().any(|id| gate.unspendable.contains(id)) {
                return Err(UseDisposition::StaleState);
            }
            gate.start(kind, self.owner_ms())?;
        }
        let commit = self
            .root
            .commit_item_use(self.character, self.holder, fence, request);
        let bound = Duration::from_millis(ITEMUSE0_RL_04_AMBIGUOUS_MS);
        let (disposition, known_commit, ambiguous) = match tokio::time::timeout(bound, commit).await
        {
            Ok(Ok(ItemUseOutcome::Written { .. })) => (UseDisposition::Committed, true, false),
            // A replay committed earlier; its effect, if any, was applied then.
            Ok(Ok(ItemUseOutcome::AlreadyCommitted { .. })) => {
                (UseDisposition::Committed, false, false)
            }
            Ok(Err(ItemUseError::StaleState)) => (UseDisposition::StaleState, false, false),
            Ok(Err(ItemUseError::Unavailable(_))) | Err(_) => {
                (UseDisposition::Rejected, false, true)
            }
            Ok(Err(_)) => (UseDisposition::Rejected, false, false),
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if known_commit {
            apply_effect(
                &runtime,
                &mut states,
                actor,
                session,
                command.command_id,
                &effect,
                self.owner_ms(),
            );
        }
        if let Some(gate) = states.item_uses.gate(&runtime, actor, session) {
            gate.settle(
                command.command_id,
                disposition,
                if ambiguous { &reserved } else { &[] },
            );
        }
        Ok(disposition)
    }
}

fn unix_ms() -> Result<i64, ()> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ())?;
    i64::try_from(elapsed.as_millis()).map_err(|_| ())
}

/// The content record of exactly `definition`, as `generation_item_facts` resolves it.
fn record<'n>(
    native: &'n NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Option<&'n ItemProfileRecord> {
    native
        .item_policy(&definition.production_key, &definition.revision_ref)
        .map(|policy| policy.record())
        .filter(|record| production_ref(record) == *definition)
}

fn production_ref(record: &ItemProfileRecord) -> TypedDefinitionRef {
    let production = &record.production_definition;
    TypedDefinitionRef {
        family: production.family.clone(),
        production_key: production.production_key.clone(),
        revision_ref: production.revision_ref.clone(),
    }
}

/// The flask output (§4.1): a compatible stack with room, else a fresh entry, else `Missing`
/// (`PARITY_PENDING` until ITEM-MOVE-2b). `None` when content names no flask.
fn flask_plan(
    native: &NativeGameplayState,
    backpack: &CharacterBackpack,
    source: &BackpackEntry,
    flask: &ReferenceItemField<ReferenceItemTarget>,
) -> Result<ItemUseFlaskPlan, UseDisposition> {
    let target = match flask {
        ReferenceItemField::NotApplicable => return Ok(ItemUseFlaskPlan::None),
        ReferenceItemField::Known(target) => target,
        _ => return Err(UseDisposition::NothingToUse),
    };
    let definition = native
        .item_policy(&target.key, &target.revision)
        .map(|policy| production_ref(policy.record()))
        .ok_or(UseDisposition::NothingToUse)?;
    let entries = &backpack.entries;
    if let Some(stack) = entries.iter().find(|entry| {
        entry.item.definition == definition && entry.item.quantity < GAMEITEM01_STACK_QUANTITY_MAX
    }) {
        return Ok(ItemUseFlaskPlan::Stack {
            item_instance_id: stack.item.item_instance_id,
            definition,
            quantity_before: stack.item.quantity,
        });
    }
    let retiring = usize::from(source.item.quantity == 1);
    if entries.len() + 1 - retiring > MAIN_BACKPACK_ENTRIES {
        return Ok(ItemUseFlaskPlan::Missing);
    }
    let highest = entries.iter().map(|entry| entry.placement_ordinal).max();
    let placement_ordinal = highest.unwrap_or(0).checked_add(1);
    let item_instance_id = crate::node::operator_files::uuid_v7().ok();
    let (Some(placement_ordinal), Some(item_instance_id)) = (placement_ordinal, item_instance_id)
    else {
        return Err(UseDisposition::Rejected);
    };
    Ok(ItemUseFlaskPlan::New {
        item_instance_id,
        definition,
        placement_ordinal,
    })
}

/// Level, magic level and vocation from content (§6.2). A value content does not know is not
/// enforced (`PARITY_PENDING`: food carries none); a conflicting one fails closed.
fn requirements_met(
    requirements: &ReferenceItemField<ReferenceItemUseRequirements>,
    facts: CharacterCastFacts,
) -> bool {
    fn holds<T>(field: &ReferenceItemField<T>, check: impl FnOnce(&T) -> bool) -> bool {
        match field {
            ReferenceItemField::Known(value) => check(value),
            ReferenceItemField::Conflict => false,
            ReferenceItemField::Unknown | ReferenceItemField::NotApplicable => true,
        }
    }
    holds(requirements, |requirements| {
        holds(&requirements.min_level, |min| {
            facts.level >= u32::from(*min)
        }) && holds(&requirements.min_magic_level, |min| {
            facts.magic_level >= u32::from(*min)
        }) && holds(&requirements.vocations, |vocations| {
            vocations.contains(&base_vocation(facts.vocation))
        })
    })
}

const fn base_vocation(vocation: Vocation) -> ReferenceBaseVocation {
    match vocation {
        Vocation::Druid | Vocation::ElderDruid => ReferenceBaseVocation::Druid,
        Vocation::Sorcerer | Vocation::MasterSorcerer => ReferenceBaseVocation::Sorcerer,
        Vocation::Knight | Vocation::EliteKnight => ReferenceBaseVocation::Knight,
        Vocation::Paladin | Vocation::RoyalPaladin => ReferenceBaseVocation::Paladin,
        Vocation::Monk | Vocation::ExaltedMonk => ReferenceBaseVocation::Monk,
    }
}

/// PRIMARY COMMIT of the effect after a known commit (§5). An actor that died or left meanwhile
/// gets nothing and the unit stays spent. A potion's restore is drawn from the owner stream bound
/// to the CommandRef and committed by the same health/mana credit primitives the spell pipeline
/// uses (`HealingOrigin::ItemUse` marks it outside Sustain's qualified scope: no stance scales it).
fn apply_effect(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
    effect: &Effect,
    now_ms: u64,
) {
    match effect {
        Effect::Food(seconds) => {
            // Checked FULL at PREPARE; with one use in flight it cannot have filled since.
            let _ = states.apply_food(runtime, actor, session, *seconds, now_ms);
        }
        Effect::Potion(restores) => {
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let decision = DecisionOccurrenceId::from_bytes(occurrence(actor, session, command_id));
            for (index, restore) in (0_u64..).zip(restores) {
                let value = deterministic_decision_u64(&root, decision, "item.use.draw", index)
                    .unwrap_or(0);
                let drawn = crate::spell::uniform_draw(
                    value,
                    i64::from(restore.min),
                    i64::from(restore.max),
                );
                let amount = u64::try_from(drawn).unwrap_or(0);
                let _ = match restore.resource {
                    ReferenceRestoreResource::Health => {
                        states.apply_health_gain(runtime, actor, session, amount)
                    }
                    ReferenceRestoreResource::Mana => {
                        states.apply_mana_gain(runtime, actor, session, amount)
                    }
                };
            }
        }
    }
}

/// The occurrence of one item use, bound to its CommandRef and actor like a spell cast's.
fn occurrence(actor: ExactActorRef, session: GameSessionId, command_id: u64) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(b"oteryn:item-use-occurrence:v1")
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::ReferenceUseEnforcementMode;

    fn limit(id: &str) -> u64 {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))
        .expect("registry");
        let entries = registry["entries"].as_array().expect("entries");
        let entry = entries.iter().find(|entry| entry["id"] == id).expect(id);
        entry["hard_maximum"].as_u64().expect("hard maximum")
    }

    #[test]
    fn the_limits_match_their_registered_rows() {
        assert_eq!(
            limit("ITEMUSE0-RL-01"),
            ITEMUSE0_RL_01_USES_PER_SECOND as u64
        );
        assert_eq!(limit("ITEMUSE0-RL-04"), ITEMUSE0_RL_04_AMBIGUOUS_MS);
    }

    #[test]
    fn rl_01_admits_five_uses_per_second_and_the_sixth_is_exhausted() {
        let mut gate = ItemUseGate::default();
        for command in 1..=5 {
            assert_eq!(gate.admit(command, 100 + command), Ok(()));
        }
        assert_eq!(gate.admit(6, 999), Err(UseDisposition::Exhausted));
        assert_eq!(gate.admit(7, 1_100), Err(UseDisposition::Exhausted));
        assert_eq!(gate.admit(8, 1_101), Ok(()));
    }

    #[test]
    fn cooldowns_start_at_prepare_and_survive_a_failed_commit() {
        let mut gate = ItemUseGate::default();
        assert_eq!(gate.start(ItemUseKind::Potion, 0), Ok(()));
        gate.settle(1, UseDisposition::StaleState, &[]);
        assert_eq!(
            gate.start(ItemUseKind::Potion, 999),
            Err(UseDisposition::Exhausted)
        );
        // Food has its own cooldown.
        assert_eq!(gate.start(ItemUseKind::Food, 999), Ok(()));
        gate.settle(2, UseDisposition::Committed, &[]);
        assert_eq!(
            gate.start(ItemUseKind::Food, 1_198),
            Err(UseDisposition::Exhausted)
        );
        assert_eq!(gate.start(ItemUseKind::Potion, 1_000), Ok(()));
        gate.settle(3, UseDisposition::Committed, &[]);
        assert_eq!(gate.start(ItemUseKind::Food, 1_199), Ok(()));
    }

    #[test]
    fn one_use_is_in_flight_and_an_ambiguous_commit_holds_it_for_rl_04() {
        let mut gate = ItemUseGate::default();
        assert_eq!(gate.admit(1, 0), Ok(()));
        assert_eq!(gate.start(ItemUseKind::Food, 0), Ok(()));
        assert_eq!(
            gate.start(ItemUseKind::Potion, 0),
            Err(UseDisposition::Exhausted)
        );
        assert_eq!(gate.admit(2, 10), Err(UseDisposition::Exhausted));
        gate.settle(1, UseDisposition::Rejected, &[[7; 16]]);
        assert_eq!(gate.unspendable, vec![[7; 16]]);
        assert_eq!(
            gate.admit(3, ITEMUSE0_RL_04_AMBIGUOUS_MS - 1),
            Err(UseDisposition::Exhausted)
        );
        assert_eq!(gate.admit(4, ITEMUSE0_RL_04_AMBIGUOUS_MS), Ok(()));
        assert_eq!(gate.in_flight_since_ms, None);
        // A replay of the last decided command returns its result.
        assert_eq!(gate.admit(1, 5_000), Err(UseDisposition::Rejected));
    }

    fn facts(vocation: Vocation, level: u32) -> CharacterCastFacts {
        CharacterCastFacts {
            vocation,
            level,
            magic_level: 0,
            max_health: 150,
            max_mana: 50,
            max_soul: 100,
        }
    }

    #[test]
    fn requirements_enforce_what_content_knows() {
        let potion = ReferenceItemField::Known(ReferenceItemUseRequirements {
            min_level: ReferenceItemField::Known(50),
            min_magic_level: ReferenceItemField::Unknown,
            vocations: ReferenceItemField::Known(vec![
                ReferenceBaseVocation::Druid,
                ReferenceBaseVocation::Sorcerer,
            ]),
            enforcement_mode: ReferenceUseEnforcementMode::OnUse,
        });
        assert!(requirements_met(&potion, facts(Vocation::ElderDruid, 50)));
        assert!(!requirements_met(
            &potion,
            facts(Vocation::MasterSorcerer, 49)
        ));
        assert!(!requirements_met(&potion, facts(Vocation::EliteKnight, 80)));
        assert!(requirements_met(
            &ReferenceItemField::Unknown,
            facts(Vocation::Monk, 1)
        ));
        assert!(!requirements_met(
            &ReferenceItemField::Conflict,
            facts(Vocation::Monk, 1)
        ));
    }
}
