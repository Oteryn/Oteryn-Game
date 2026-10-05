//! Actual current Character/Item reads for a cast, inside its owner's physical SQL transaction.
//! Premium/learning/Wheel producers are explicit ports. Unavailable authority never becomes a grant.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use crate::content::ActiveGeneration;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_equipment::{
    EquipmentError, assert_equipment_authority_in_transaction,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use crate::spell::cast::PlayerSpellState;
use crate::spell::owned_cast_facts::{
    AccessProjections, CastFactsBinding, CurrentProjection, OwnedCastFacts, OwnedFactsError,
};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) enum AccessFactsError {
    Equipment(EquipmentError),
    Database(sqlx::Error),
    Durability(DurabilityError),
    Facts(OwnedFactsError),
    Unavailable(&'static str),
}
impl From<EquipmentError> for AccessFactsError {
    fn from(e: EquipmentError) -> Self {
        Self::Equipment(e)
    }
}
impl From<sqlx::Error> for AccessFactsError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
impl From<OwnedFactsError> for AccessFactsError {
    fn from(e: OwnedFactsError) -> Self {
        Self::Facts(e)
    }
}
/// A producer implementation is an explicit, reviewed in-crate owner registration. The wire/client
/// cannot implement this sealed port. It must read authenticated Premium evidence with its durable
/// revision/conflict fence, or Character learning/Wheel receipts under the same exact binding.
pub(crate) mod owner_registration {
    pub(crate) trait Registered {}
}
pub(crate) trait CurrentSpellAccessOwner: owner_registration::Registered {
    fn account_id(&self) -> Option<[u8; 16]> {
        None
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now_micros: u64,
    ) -> Result<AccessProjections, AccessFactsError>;
}
/// Explicit missing external producer. This supplies no known-negative or zero-stage projection.
pub(crate) struct UnavailableAccessOwner;
impl owner_registration::Registered for UnavailableAccessOwner {}
impl CurrentSpellAccessOwner for UnavailableAccessOwner {
    fn read_current(
        &self,
        _: &CastFactsBinding,
        _: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        Ok(AccessProjections::default())
    }
}

/// This adapter is constructed only by the physical-transaction loader below. The ordinary
/// external access owner cannot replace the Wheel allocation with an admission-cache snapshot.
struct WheelCastAccess<'a, A> {
    other: &'a A,
    read: &'a crate::durability::character_wheel::CurrentWheelCastRead,
    raw: &'a crate::durability::character_equipment::RawCastDurableFacts,
    ruleset: &'a crate::durability::character_wheel::WheelRuleset,
}
impl<A> owner_registration::Registered for WheelCastAccess<'_, A> {}
impl<A: CurrentSpellAccessOwner> CurrentSpellAccessOwner for WheelCastAccess<'_, A> {
    fn account_id(&self) -> Option<[u8; 16]> {
        self.other.account_id()
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        let mut projections = self.other.read_current(binding, now)?;
        projections.wheel = None;
        let read = self.read.raw();
        if read.command != self.raw.command
            || read.fence != self.raw.fence
            || read.account != self.raw.account
            || read.equipment != self.raw.equipment
            || read.build != self.raw.build
            || read.level != self.raw.level
            || self.read.ruleset_revision() != self.ruleset.revision()
        {
            return Err(AccessFactsError::Unavailable(
                "Wheel cast transaction binding changed",
            ));
        }
        if wheel_raw_matches_binding(read, binding) {
            projections.wheel = project_wheel_stages(
                self.ruleset,
                self.read.allocation(),
                read.build.vocation(),
                read.level,
                binding,
                projections.premium.as_ref(),
                now,
            );
        }
        Ok(projections)
    }
}

fn wheel_raw_matches_binding(
    raw: &crate::durability::character_equipment::RawCastDurableFacts,
    binding: &CastFactsBinding,
) -> bool {
    binding.character == *raw.fence.character_id.as_bytes()
        && binding.character == raw.equipment.character
        && binding.session == raw.fence.game_session_id
        && binding.character_revision == raw.equipment.character_revision
        && binding.lease_generation == raw.fence.character_lease_generation
        && binding.connection_generation == raw.fence.connection_generation.get()
        && binding.content_digest == raw.equipment.content_digest
        && binding.equipment_revision == raw.equipment.revision
}

fn project_wheel_stages(
    ruleset: &crate::durability::character_wheel::WheelRuleset,
    allocation: &crate::durability::character_wheel::WheelAllocation,
    vocation: &str,
    level: u32,
    binding: &CastFactsBinding,
    premium: Option<&CurrentProjection<bool>>,
    now: u64,
) -> Option<CurrentProjection<BTreeMap<String, u8>>> {
    let premium = premium.filter(|p| p.current(binding, now) && p.value)?;
    if !allocation.current || binding.character_revision == 0 {
        return None;
    }
    let promoted = matches!(
        vocation,
        "elder_druid" | "elite_knight" | "exalted_monk" | "royal_paladin" | "master_sorcerer"
    );
    let rules =
        crate::durability::character_wheel::eligibility(ruleset, vocation, level, promoted).ok()?;
    let stages = crate::durability::character_wheel::WheelStages::derive(
        ruleset, allocation, vocation, level, promoted,
    );
    let mut value: BTreeMap<_, _> = rules
        .revelations
        .iter()
        .map(|key| (key.clone(), stages.revelation_stage(key)))
        .collect();
    for key in rules.augment_targets.keys() {
        value.insert(key.clone(), stages.augment_stage(key));
    }
    for target in rules.augment_targets.values().flatten() {
        value.insert(target.clone(), stages.spell_augment_stage(target));
    }
    // Closed aliases from the existing Wheel spell ABI and admitted native profile labels.
    // No case folding or inferred key authorizes an arbitrary perk.
    for (label, key) in [
        ("Executioner's Throw", "executioner_s_throw"),
        ("Combat Mastery", "combat_mastery"),
        ("Divine Grenade", "divine_grenade"),
        ("Divine Empowerment", "divine_empowerment"),
        ("Blessing of the Grove", "blessing_of_the_grove"),
        ("Twin Burst", "twin_bursts"),
        ("Beam Mastery", "beam_mastery"),
        ("Lord of Destruction", "lord_of_destruction"),
        ("Spiritual Outburst", "spiritual_outburst"),
        ("Ascetic", "ascetic"),
        ("Avatar of Steel", "avatar_of_steel"),
        ("Avatar of Light", "avatar_of_light"),
        ("Avatar of Nature", "avatar_of_nature"),
        ("Avatar of Storm", "avatar_of_storm"),
        ("Avatar of Balance", "avatar_of_balance"),
        ("avatar of steel", "avatar_of_steel"),
        ("avatar of light", "avatar_of_light"),
        ("avatar of nature", "avatar_of_nature"),
        ("avatar of storm", "avatar_of_storm"),
        ("avatar of balance", "avatar_of_balance"),
        ("energy wave", "Energy Wave"),
        ("front sweep", "Front Sweep"),
        ("mass healing", "Mass Healing"),
        ("strong ice wave", "Strong Ice Wave"),
    ] {
        if let Some(stage) = value.get(key).copied() {
            value.insert(label.to_owned(), stage);
        }
    }
    // CharacterRevision is independently read under the Character lock. Every Wheel write
    // advances it, including reset; no-row allocation does not need a fictional Wheel revision.
    Some(CurrentProjection {
        binding: binding.clone(),
        authority_revision: binding.character_revision,
        valid_until_micros: premium.valid_until_micros,
        value,
    })
}

fn retain_wheel_read(
    read: Result<crate::durability::character_wheel::CurrentWheelCastRead, DurabilityError>,
) -> Result<Option<crate::durability::character_wheel::CurrentWheelCastRead>, AccessFactsError> {
    match read {
        Ok(read) => Ok(Some(read)),
        // Corrupt Wheel data withholds Wheel authority; it does not invalidate the separately
        // checked Character/Item authority of a base spell. A failed SQL transaction or owner
        // fence still rejects the cast and can never be silently converted into absence.
        Err(DurabilityError::InvalidStoredState) => Ok(None),
        Err(error) => Err(AccessFactsError::Durability(error)),
    }
}
/// The caller holds the actual Channel owner turn and SQL transaction until the whole cast stages
/// and commits. This helper never issues session, lease, scope or Content authority; each is checked
/// independently from the live owner or actual durable fence. No fixture map substitutes for Item.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_owned_cast_facts_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    state: &PlayerSpellState,
    active: &ActiveGeneration,
    access_owner: &impl CurrentSpellAccessOwner,
    now_micros: u64,
) -> Result<OwnedCastFacts, AccessFactsError> {
    runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| AccessFactsError::Unavailable("current player owner"))?;
    let native = active
        .native_gameplay()
        .ok_or(AccessFactsError::Unavailable("active gameplay artifact"))?;
    if active.identity().server_artifact_digest() != native.source_digest()
        || runtime.content_pin().server_artifact_digest() != native.source_digest()
    {
        return Err(AccessFactsError::Unavailable("current native Content pin"));
    }
    let authority = assert_equipment_authority_in_transaction(
        tx,
        root,
        recovery,
        node,
        fence,
        command,
        native.source_digest(),
    )
    .await?;
    // An uninitialized equipment owner is unavailable. Initialization belongs to its independent
    // admission/Item command, never to a rejected cast's hidden side effects.
    let raw =
        crate::durability::character_equipment::read_raw_cast_facts_in_transaction(tx, &authority)
            .await?;
    let ruleset = crate::wheel_gem_data::WheelGemData::embedded()
        .ok()
        .and_then(|data| data.wheel_ruleset().ok());
    let wheel = match ruleset.as_ref() {
        Some(ruleset) => retain_wheel_read(
            crate::durability::character_wheel::read_current_allocation_in_transaction(
                tx, &authority, ruleset,
            )
            .await,
        )?,
        None => None,
    };
    if let (Some(read), Some(ruleset)) = (wheel.as_ref(), ruleset.as_ref()) {
        return qualify_current_cast_facts(
            &raw,
            command,
            fence,
            runtime,
            actor,
            state,
            active,
            &WheelCastAccess {
                other: access_owner,
                read,
                raw: &raw,
                ruleset,
            },
            now_micros,
            true,
        );
    }
    qualify_raw_owned_cast_facts(
        &raw,
        command,
        fence,
        runtime,
        actor,
        state,
        active,
        access_owner,
        now_micros,
    )
}
/// Qualifies a real SQL snapshot only after the caller rechecks the current actual owner turn.
/// This data producer grants no authority; final transaction staging still compares every revision.
#[allow(clippy::too_many_arguments)]
pub(crate) fn qualify_raw_owned_cast_facts(
    raw: &crate::durability::character_equipment::RawCastDurableFacts,
    command: CommandRef,
    fence: &CurrentCharacterItemFence,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    state: &PlayerSpellState,
    active: &ActiveGeneration,
    access_owner: &impl CurrentSpellAccessOwner,
    now_micros: u64,
) -> Result<OwnedCastFacts, AccessFactsError> {
    qualify_current_cast_facts(
        raw,
        command,
        fence,
        runtime,
        actor,
        state,
        active,
        access_owner,
        now_micros,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn qualify_current_cast_facts(
    raw: &crate::durability::character_equipment::RawCastDurableFacts,
    command: CommandRef,
    fence: &CurrentCharacterItemFence,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    state: &PlayerSpellState,
    active: &ActiveGeneration,
    access_owner: &impl CurrentSpellAccessOwner,
    now_micros: u64,
    current_wheel_transaction: bool,
) -> Result<OwnedCastFacts, AccessFactsError> {
    if raw.command != command
        || &raw.fence != fence
        || access_owner
            .account_id()
            .is_some_and(|account| account != raw.account)
    {
        return Err(AccessFactsError::Unavailable(
            "raw cast owner binding changed",
        ));
    }
    runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| AccessFactsError::Unavailable("current raw cast actor"))?;
    let native = active
        .native_gameplay()
        .ok_or(AccessFactsError::Unavailable("active gameplay artifact"))?;
    if active.identity().server_artifact_digest() != raw.equipment.content_digest
        || runtime.content_pin().server_artifact_digest() != raw.equipment.content_digest
        || native.source_digest() != raw.equipment.content_digest
    {
        return Err(AccessFactsError::Unavailable(
            "raw cast Content binding changed",
        ));
    }
    let equipment = raw.equipment.clone();
    let build = raw.build.clone();
    let level = raw.level;
    let binding = CastFactsBinding {
        actor,
        session: fence.game_session_id,
        character: *fence.character_id.as_bytes(),
        character_revision: equipment.character_revision,
        lease_generation: fence.character_lease_generation,
        connection_generation: fence.connection_generation.get(),
        player_revision: state.revision(),
        content_digest: native.source_digest(),
        equipment_revision: equipment.revision,
    };
    let mut projections = access_owner.read_current(&binding, now_micros)?;
    // Only an exact current projection survives. The transaction loader supplies the Wheel
    // owner read; missing/stale allocation or expired Premium never becomes a zero-stage grant.
    projections.wheel = projections.wheel.filter(|wheel| {
        current_wheel_transaction
            && wheel.current(&binding, now_micros)
            && projections
                .premium
                .as_ref()
                .is_some_and(|premium| premium.current(&binding, now_micros) && premium.value)
    });
    projections.magnitude = super::spell_magnitude_facts::project(
        raw,
        &binding,
        native,
        projections.wheel.as_ref(),
        projections.premium.as_ref(),
        now_micros,
    )?;
    let facts = OwnedCastFacts::from_owner_reads(binding, build, level, equipment, projections)?;
    // No await follows this final independent owner recheck; the caller keeps the same lock/tx.
    runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| AccessFactsError::Unavailable("player owner changed"))?;
    Ok(facts)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod wheel_projection_tests {
    use super::*;
    use crate::durability::character_wheel::{WheelAllocation, WheelDomain, WheelSlots};

    fn binding() -> CastFactsBinding {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 3,
            content_digest: [9; 32],
            equipment_revision: 2,
        }
    }
    fn ruleset() -> crate::durability::character_wheel::WheelRuleset {
        crate::wheel_gem_data::WheelGemData::embedded()
            .expect("accepted embedded Wheel")
            .wheel_ruleset()
            .expect("Wheel ruleset")
    }
    fn premium(binding: &CastFactsBinding) -> CurrentProjection<bool> {
        CurrentProjection {
            binding: binding.clone(),
            authority_revision: 8,
            valid_until_micros: 100,
            value: true,
        }
    }
    fn zero() -> WheelAllocation {
        WheelAllocation {
            wheel_ruleset_revision: None,
            wheel_revision: 0,
            slots: WheelSlots::ZERO,
            current: true,
        }
    }
    #[test]
    fn corrupt_wheel_withholds_projection_but_transaction_or_owner_failure_rejects() {
        assert!(
            retain_wheel_read(Err(DurabilityError::InvalidStoredState))
                .expect("Wheel withheld")
                .is_none()
        );
        assert!(retain_wheel_read(Err(DurabilityError::Unavailable)).is_err());
        assert!(retain_wheel_read(Err(DurabilityError::from(sqlx::Error::RowNotFound))).is_err());
    }
    #[test]
    fn real_no_row_allocation_has_known_zero_without_inventing_wheel_revision() {
        let binding = binding();
        let ruleset = ruleset();
        let allocation = zero();
        let result = project_wheel_stages(
            &ruleset,
            &allocation,
            "elder_druid",
            1050,
            &binding,
            Some(&premium(&binding)),
            99,
        )
        .expect("current zero read");
        assert_eq!(result.authority_revision, binding.character_revision);
        assert_eq!(allocation.wheel_revision, 0);
        assert_eq!(result.valid_until_micros, 100);
        assert!(result.value.values().all(|stage| *stage == 0));
        assert!(!result.current(&binding, 100));
    }
    #[test]
    fn positive_stages_come_from_current_slots_and_current_vocation() {
        let binding = binding();
        let ruleset = ruleset();
        let mut slots = [0; crate::durability::character_wheel::WHEEL_SLOTS];
        for (index, slot) in ruleset.slots().iter().enumerate() {
            if slot.domain == WheelDomain::Green {
                slots[index] = slot.capacity;
            }
        }
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(ruleset.revision().to_owned()),
            wheel_revision: 7,
            slots: WheelSlots::new(slots).expect("source slots"),
            current: true,
        };
        let projected = project_wheel_stages(
            &ruleset,
            &allocation,
            "elder_druid",
            1050,
            &binding,
            Some(&premium(&binding)),
            1,
        )
        .expect("current stages");
        let rules = ruleset.vocation("elder_druid").expect("actual vocation");
        assert_eq!(projected.value[&rules.revelations[0]], 3);
        assert_eq!(projected.value[&rules.revelations[1]], 0);
    }
    #[test]
    fn existing_native_consumer_labels_resolve_actual_revelation_and_spell_augment_stages() {
        let binding = binding();
        let ruleset = ruleset();
        let mut points = [0; crate::durability::character_wheel::WHEEL_SLOTS];
        for (index, slot) in ruleset.slots().iter().enumerate() {
            points[index] = slot.capacity;
        }
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(ruleset.revision().to_owned()),
            wheel_revision: 2,
            slots: WheelSlots::new(points).expect("full source allocation"),
            current: true,
        };
        for (vocation, revelation, augment) in [
            ("elder_druid", "Twin Burst", "mass healing"),
            ("elite_knight", "Executioner's Throw", "front sweep"),
            ("master_sorcerer", "Beam Mastery", "energy wave"),
            ("royal_paladin", "Divine Empowerment", "Divine Dazzle"),
            ("exalted_monk", "Spiritual Outburst", "Mass Spirit Mend"),
        ] {
            let projected = project_wheel_stages(
                &ruleset,
                &allocation,
                vocation,
                10000,
                &binding,
                Some(&premium(&binding)),
                1,
            )
            .expect("source-bound consumer labels");
            assert_eq!(projected.value[revelation], 3);
            assert_eq!(projected.value[augment], 2);
            assert!(!projected.value.contains_key("Arbitrary invented perk"));
        }
    }
    #[test]
    fn wheel_rejects_missing_expired_negative_or_revisionless_premium() {
        let binding = binding();
        let ruleset = ruleset();
        let allocation = zero();
        assert!(
            project_wheel_stages(
                &ruleset,
                &allocation,
                "elder_druid",
                1050,
                &binding,
                None,
                1
            )
            .is_none()
        );
        let mut premium = premium(&binding);
        assert!(
            project_wheel_stages(
                &ruleset,
                &allocation,
                "elder_druid",
                1050,
                &binding,
                Some(&premium),
                100
            )
            .is_none()
        );
        premium.value = false;
        assert!(
            project_wheel_stages(
                &ruleset,
                &allocation,
                "elder_druid",
                1050,
                &binding,
                Some(&premium),
                1
            )
            .is_none()
        );
        premium.value = true;
        premium.authority_revision = 0;
        assert!(
            project_wheel_stages(
                &ruleset,
                &allocation,
                "elder_druid",
                1050,
                &binding,
                Some(&premium),
                1
            )
            .is_none()
        );
    }
    #[test]
    fn independently_retained_premium_cannot_authorize_a_replaced_owner_binding() {
        let expected = binding();
        let premium = premium(&expected);
        let ruleset = ruleset();
        let (mut independent_runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        let replacement_session =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(88).2;
        let reservation = independent_runtime
            .reserve_fresh_session(replacement_session)
            .expect("independent actor reservation");
        let replacement_actor = independent_runtime
            .commit_fresh_session(reservation)
            .expect("independent actor admission");
        assert_ne!(replacement_actor, expected.actor);
        for field in 0..9 {
            let mut current = expected.clone();
            match field {
                0 => current.character[0] ^= 1,
                1 => current.character_revision += 1,
                2 => current.equipment_revision += 1,
                3 => current.lease_generation += 1,
                4 => current.connection_generation += 1,
                5 => current.player_revision += 1,
                6 => current.content_digest[0] ^= 1,
                7 => current.actor = replacement_actor,
                _ => {
                    current.session =
                        crate::gameplay_transport::actor_spell::tests::runtime_with_player(88).2
                }
            }
            assert!(
                project_wheel_stages(
                    &ruleset,
                    &zero(),
                    "elder_druid",
                    1050,
                    &current,
                    Some(&premium),
                    1
                )
                .is_none(),
                "owner field {field}"
            );
        }
    }
    #[test]
    fn outdated_ruleset_and_ineligible_build_never_become_a_zero_stage_grant() {
        let binding = binding();
        let ruleset = ruleset();
        let premium = premium(&binding);
        let mut allocation = zero();
        allocation.current = false;
        assert!(
            project_wheel_stages(
                &ruleset,
                &allocation,
                "elder_druid",
                1050,
                &binding,
                Some(&premium),
                1
            )
            .is_none()
        );
        allocation.current = true;
        for (vocation, level) in [
            ("druid", 1050),
            ("none", 1050),
            ("elder_druid", 50),
            ("unrecognized", 1050),
        ] {
            assert!(
                project_wheel_stages(
                    &ruleset,
                    &allocation,
                    vocation,
                    level,
                    &binding,
                    Some(&premium),
                    1
                )
                .is_none()
            );
        }
        let mut seed = binding;
        seed.character_revision = 0;
        assert!(
            project_wheel_stages(
                &ruleset,
                &allocation,
                "elder_druid",
                1050,
                &seed,
                Some(&super::wheel_projection_tests::premium(&seed)),
                1
            )
            .is_none()
        );
    }

    #[test]
    fn durable_wheel_snapshot_must_match_current_character_item_and_session_fences() {
        let (runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        let scope = runtime.binding();
        let mut expected = binding();
        expected.character = [
            0x01, 0x90, 0, 0, 0, 0x36, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x36,
        ];
        let raw = crate::durability::character_equipment::RawCastDurableFacts {
            account: [6; 16],
            equipment: crate::durability::character_equipment::EquipmentSnapshot {
                character: expected.character,
                content_digest: expected.content_digest,
                character_revision: expected.character_revision,
                revision: expected.equipment_revision,
                combat_mode: None,
                items: vec![],
            },
            build: crate::durability::character_build::DurableBuildState::new(
                "elder_druid",
                (0, 0),
                [(10, 0); 7],
            )
            .expect("current build"),
            level: 1050,
            command: CommandRef::new(
                expected.session,
                crate::foundation::CommandId::new(1).expect("command"),
            ),
            fence: CurrentCharacterItemFence {
                character_id: crate::domain::CharacterId::from_bytes(expected.character)
                    .expect("character"),
                game_session_id: expected.session,
                connection_generation: crate::foundation::ConnectionGeneration::new(
                    expected.connection_generation,
                )
                .expect("connection"),
                character_lease_generation: expected.lease_generation,
                runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                    scope.world_id(),
                    scope.channel_id(),
                ),
                scope_ownership_generation: scope.scope_generation(),
            },
        };
        assert!(wheel_raw_matches_binding(&raw, &expected));
        for field in 0..7 {
            let mut actual = expected.clone();
            match field {
                0 => actual.character[15] ^= 1,
                1 => actual.character_revision += 1,
                2 => actual.equipment_revision += 1,
                3 => actual.lease_generation += 1,
                4 => actual.connection_generation += 1,
                5 => actual.content_digest[0] ^= 1,
                _ => {
                    actual.session =
                        crate::gameplay_transport::actor_spell::tests::runtime_with_player(88).2
                }
            }
            assert!(
                !wheel_raw_matches_binding(&raw, &actual),
                "durable field {field}"
            );
        }
    }
}

#[cfg(test)]
#[path = "../../tests/support/character_wheel_cast_postgres_cases.rs"]
mod character_wheel_cast_postgres_cases;
