//! Native haste mutations of the existing player and physical companion stores.
//! The Channel owner commits the complete staged set with its cast anchor.
//! The scalar compatibility entry point remains limited to actors without owned
//! familiars; `stage_haste` supports all four source-qualified haste profiles.
use super::native::CompiledNativeSpell;
use crate::ability::condition::{
    ApplicationFacts, Applied, ConditionDefinition, ConditionStore, ConditionType, ConditionValues,
    SpeedRange, SpellSpeedError,
};

#[derive(Debug)]
pub(crate) struct PaidHasteCast {
    pub(crate) next: super::cast::PlayerSpellState,
    pub(crate) anchor: super::combat_batch::SpellAnchor,
    pub(crate) companion_updates: Vec<(
        crate::foundation::CompanionSnapshot,
        crate::foundation::CompanionState,
    )>,
    /// Complete census, including ordinary summons that receive no condition.
    pub(crate) companions: Vec<crate::foundation::CompanionSnapshot>,
}

/// The actual caster, map, persisted equipment and complete summon census are
/// qualified before any source condition or anchor payment is prepared. Caller
/// joins all returned successors to the same physical/player cast transaction.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_haste_owner_cast_with_caster(
    runtime: &crate::foundation::ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    state: &super::cast::PlayerSpellState,
    spell: &super::SpellDefinition,
    operational: &super::OperationalCastFacts,
    qualified_caster: &super::CasterState,
    caster: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    equipment_delta: Option<i32>,
    facts: &ApplicationFacts<'_>,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<PaidHasteCast, oteryn_protocol_oteryn::actor_spell::SpellCastDisposition> {
    use super::native_companions::{
        CompanionFacts, ConditionSlotId, HasteFacts, SpeedCondition, SpeedKind, SpeedRecipient,
        SwiftGrade,
    };
    use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
    let rejected = SpellCastDisposition::Rejected;
    let super::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(rejected);
    };
    if profile.spell()["execution"]["native_behavior"]["key"] != "companion_haste"
        || !facts.target_is_player
        || u32::from(facts.base_speed) != state.base_speed
        || !runtime
            .player_control_facts(caster, session)
            .is_ok_and(|control| control.control_loss.is_none())
    {
        return Err(rejected);
    }
    let position = runtime.read_actor_position(caster).map_err(|_| rejected)?;
    let ground = crate::movement::speed::qualified_ground_cost(runtime, cells, caster, position)
        .map_err(|_| rejected)?;
    let equipment = equipment_delta.ok_or(rejected)?;
    // A source haste may remove an immobilizing paralysis. Validate the owned
    // pacing inputs here and its resulting speed after staging the condition.
    let companions = runtime
        .owned_companions(caster, session)
        .map_err(|_| rejected)?;
    let recipient = |id,
                     base,
                     familiar,
                     condition: Option<&crate::ability::condition::ConditionInstance<_>>|
     -> Result<SpeedRecipient, SpellCastDisposition> {
        let conditions = condition
            .map(|instance| -> Result<SpeedCondition, SpellCastDisposition> {
                let paralysis = matches!(
                    instance.definition().values(),
                    ConditionValues::Speed {
                        paralysis: true,
                        ..
                    }
                );
                Ok(SpeedCondition {
                    kind: if paralysis {
                        SpeedKind::Paralyze
                    } else {
                        SpeedKind::Haste
                    },
                    id: ConditionSlotId::Combat,
                    sub_id: 0,
                    ticks_ms: instance.remaining_duration_ms(facts.now).ok_or(rejected)?,
                    delta: i32::try_from(instance.speed_delta()).map_err(|_| rejected)?,
                })
            })
            .transpose()?
            .into_iter()
            .collect();
        Ok(SpeedRecipient {
            id,
            base_speed: base,
            is_familiar: familiar,
            // The accepted player/companion ConditionStore has no suppression
            // state. Its actual admission handles immunity/recovery/conflicts.
            haste_suppressed: false,
            paralyze_suppressed: false,
            conditions,
        })
    };
    let caster_recipient = recipient(
        1,
        i32::from(facts.base_speed),
        false,
        state
            .conditions
            .active_at(crate::ability::condition::ConflictKey::Speed, facts.now),
    )?;
    // String player provenance and ExactActorRef companion provenance are
    // distinct actual stores; construct companion snapshots without coercion.
    let mut owned_summons = Vec::with_capacity(companions.len());
    for (index, snapshot) in companions.iter().enumerate() {
        let instance = snapshot
            .state
            .conditions
            .active_at(crate::ability::condition::ConflictKey::Speed, facts.now);
        let conditions = instance
            .map(|instance| -> Result<SpeedCondition, SpellCastDisposition> {
                Ok(SpeedCondition {
                    kind: if instance.definition().condition_type() == ConditionType::Paralysis {
                        SpeedKind::Paralyze
                    } else {
                        SpeedKind::Haste
                    },
                    id: ConditionSlotId::Combat,
                    sub_id: 0,
                    ticks_ms: instance.remaining_duration_ms(facts.now).ok_or(rejected)?,
                    delta: i32::try_from(instance.speed_delta()).map_err(|_| rejected)?,
                })
            })
            .transpose()?
            .into_iter()
            .collect();
        owned_summons.push(SpeedRecipient {
            // These are temporary ordered-census plan coordinates. Actual
            // mutations retain each complete ExactActorRef-bound snapshot.
            id: u64::try_from(index)
                .ok()
                .and_then(|n| n.checked_add(2))
                .ok_or(rejected)?,
            base_speed: i32::try_from(snapshot.state.policy.base_speed).map_err(|_| rejected)?,
            is_familiar: snapshot.state.policy.is_familiar,
            haste_suppressed: false,
            paralyze_suppressed: false,
            conditions,
        });
    }
    let parameters = &profile.spell()["execution"]["native_behavior"]["parameters"];
    let percentages = &parameters["damage_dealt_percent"];
    // All canonical haste recipes are grade invariant. No Wheel stage or
    // absence is asserted: differing grades would require the actual producer.
    if percentages["none"] != percentages["regular"]
        || percentages["none"] != percentages["greater"]
    {
        return Err(rejected);
    }
    let native_facts = CompanionFacts::Haste(HasteFacts {
        caster: caster_recipient,
        owned_summons,
        swift_grade: SwiftGrade::None,
    });
    let mut paid = super::cast::prepare_native_owner_cast_with_caster(
        state,
        spell,
        operational,
        super::native::Facts::Companion(&native_facts),
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(facts.now),
        draw,
        qualified_caster,
    )?;
    let updates =
        stage_haste(&mut paid.next, profile, &companions, caster, facts).map_err(|_| rejected)?;
    let _ = crate::movement::speed::step_duration_ms(
        super::actor_conditions::movement_speed(&paid.next, facts.now, equipment),
        ground,
        false,
        false,
    )
    .map_err(|_| rejected)?;
    if !paid.next.paid_successor_of(state, &paid.anchor) {
        return Err(rejected);
    }
    Ok(PaidHasteCast {
        next: paid.next,
        anchor: paid.anchor,
        companion_updates: updates,
        companions,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HasteExecutionError {
    UnsupportedProfile,
    SwiftAttributeOwnerUnavailable,
    CompanionOwnerUnavailable,
    OwnedCompanionRecipientsUnsupported,
    Condition(SpellSpeedError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HasteApplied {
    pub(crate) transition: Applied,
    pub(crate) speed_delta_before: i64,
    pub(crate) speed_delta_after: i64,
    /// The presentation owner publishes this cue only with its successful cast.
    pub(crate) caster_magic_green: bool,
}

/// Apply only a completely qualified native profile. `owned_familiar_count` is
/// independently read by the real summon owner in this same work item; None is
/// not treated as zero. The caller owns admission and the spell's anchor payment.
/// The existing store performs condition immunity, conflict and lifetime changes.
pub(crate) fn apply_haste<S: Clone>(
    store: &mut ConditionStore<S>,
    spell: &CompiledNativeSpell,
    source: S,
    immunities: &[ConditionType],
    facts: &ApplicationFacts<'_>,
    owned_familiar_count: Option<usize>,
) -> Result<HasteApplied, HasteExecutionError> {
    let document = spell.spell();
    let native = &document["execution"]["native_behavior"];
    if native["key"] != "companion_haste" {
        return Err(HasteExecutionError::UnsupportedProfile);
    }
    let name = document["name"]
        .as_str()
        .ok_or(HasteExecutionError::UnsupportedProfile)?
        .to_ascii_lowercase();
    let parameters = &native["parameters"];
    let (a, b, duration_ms) = match name.as_str() {
        "haste" => (1300, 40, 30000),
        "strong haste" => (1700, 40, 22000),
        "charge" => (1900, 40, 5000),
        "swift foot" => return Err(HasteExecutionError::SwiftAttributeOwnerUnavailable),
        _ => return Err(HasteExecutionError::UnsupportedProfile),
    };
    let count = owned_familiar_count.ok_or(HasteExecutionError::CompanionOwnerUnavailable)?;
    if count > 0 {
        return Err(HasteExecutionError::OwnedCompanionRecipientsUnsupported);
    }
    // The sealed profile reader compared every field to its source-qualified
    // catalog. Check the bridge identity explicitly; no payload is ignored here.
    let formula = &parameters["caster"]["formula"];
    let expected_a = match a {
        1300 => "1.3",
        1700 => "1.7",
        1900 => "1.9",
        _ => return Err(HasteExecutionError::UnsupportedProfile),
    };
    if formula["mina"] != expected_a
        || formula["maxa"] != expected_a
        || formula["minb"] != "40"
        || formula["maxb"] != "40"
        || parameters["caster"]["duration_ms"] != duration_ms
        || parameters["caster"]["coefficient_arithmetic"] != "float32"
        || parameters["caster"]["bound_conversion"] != "truncate_toward_zero"
        || parameters["caster"]["condition"] != "haste"
    {
        return Err(HasteExecutionError::UnsupportedProfile);
    }
    let key = document["identity"]["key"]
        .as_str()
        .ok_or(HasteExecutionError::UnsupportedProfile)?;
    let definition = ConditionDefinition::new(
        key,
        1,
        ConditionValues::Speed {
            paralysis: false,
            range: SpeedRange {
                a_min: a,
                a_max: a,
                b_min: b,
                b_max: b,
            },
            duration_ms,
        },
    )
    .ok_or(HasteExecutionError::UnsupportedProfile)?;
    let before = store.speed_delta();
    let transition = store
        .apply_spell_haste(&definition, Some(source), immunities, facts)
        .map_err(HasteExecutionError::Condition)?;
    Ok(HasteApplied {
        transition,
        speed_delta_before: before,
        speed_delta_after: store.speed_delta(),
        caster_magic_green: true,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::ability::condition::TickFacts;
    use crate::spell::native::spell_from_bundle;
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    use serde_json::{Value, json};

    fn profile(name: &str) -> CompiledNativeSpell {
        let document: Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let profile = document["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| {
                p["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
            .unwrap();
        spell_from_bundle(&json!({"spell":profile["spell"]}), &profile["dependencies"]).unwrap()
    }
    #[test]
    fn canonical_profiles_mutate_the_existing_store_with_exact_source_lifetimes() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 1000000,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        for (name, delta, duration_ms) in [
            ("haste", 107, 30000),
            ("strong haste", 252, 22000),
            ("charge", 324, 5000),
        ] {
            let mut store = ConditionStore::<u64>::new();
            let result = apply_haste(&mut store, &profile(name), 7, &[], &facts, Some(0)).unwrap();
            assert_eq!(result.speed_delta_before, 0);
            assert_eq!(result.speed_delta_after, delta);
            assert_eq!(store.speed_delta(), delta);
            assert_eq!(store.instances().len(), 1);
            assert_eq!(store.instances()[0].provenance().source, Some(7));
            let end = facts.now + duration_ms * 1000;
            assert!(store.take_due(end - 1, TickFacts::default()).is_empty());
            assert_eq!(store.speed_delta(), delta);
            assert!(store.take_due(end, TickFacts::default()).is_empty());
            assert_eq!(store.speed_delta(), 0);
        }
    }
    #[test]
    fn source_weaker_refresh_replaces_the_real_store_and_retains_new_definition() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut facts = ApplicationFacts {
            now: 0,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let mut store = ConditionStore::<u64>::new();
        apply_haste(
            &mut store,
            &profile("strong haste"),
            7,
            &[],
            &facts,
            Some(0),
        )
        .unwrap();
        facts.now = 1000000;
        let result = apply_haste(&mut store, &profile("haste"), 7, &[], &facts, Some(0)).unwrap();
        assert!(result.transition.replaced);
        assert_eq!(result.speed_delta_before, 252);
        assert_eq!(store.speed_delta(), 107);
        assert_eq!(store.instances().len(), 1);
        assert_eq!(
            store.instances()[0].provenance().definition_key,
            "candidate:spell/haste"
        );
        assert!(store.take_due(31000000, TickFacts::default()).is_empty());
        assert!(store.instances().is_empty());
    }
    #[test]
    fn unavailable_services_and_immunity_never_partially_write_the_store() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let mut store = ConditionStore::<u64>::new();
        let before = store.clone();
        assert_eq!(
            apply_haste(&mut store, &profile("swift foot"), 7, &[], &facts, Some(0)),
            Err(HasteExecutionError::SwiftAttributeOwnerUnavailable)
        );
        assert_eq!(store, before);
        assert_eq!(
            apply_haste(&mut store, &profile("haste"), 7, &[], &facts, None),
            Err(HasteExecutionError::CompanionOwnerUnavailable)
        );
        assert_eq!(store, before);
        assert_eq!(
            apply_haste(&mut store, &profile("haste"), 7, &[], &facts, Some(1)),
            Err(HasteExecutionError::OwnedCompanionRecipientsUnsupported)
        );
        assert_eq!(store, before);
        assert_eq!(
            apply_haste(
                &mut store,
                &profile("haste"),
                7,
                &[ConditionType::Haste],
                &facts,
                Some(0)
            ),
            Err(HasteExecutionError::Condition(SpellSpeedError::Admission(
                crate::ability::condition::ConditionRefusal::Immune
            )))
        );
        assert_eq!(store, before);
        assert_eq!(
            apply_haste(
                &mut store,
                &profile("knight familiar"),
                7,
                &[],
                &facts,
                Some(0)
            ),
            Err(HasteExecutionError::UnsupportedProfile)
        );
        assert_eq!(store, before);
    }

    #[test]
    fn all_four_stage_real_player_and_familiar_stores_without_mutating_physical_slots() {
        use crate::foundation::{
            ChannelContentPin, ChannelId, ChannelRuntimeV1, CompiledCreaturePolicies,
            CompiledCreaturePolicy, CreatureFlags, GameSessionId, MovementLocalPosition, NodeId,
            WorldId,
        };
        fn id(n: u8) -> [u8; 16] {
            [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
        }
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        // Test Content policies are explicit data; the actual fixed-slot carrier
        // and actual ConditionStores execute the test, not a shadow summon map.
        let policy = |name: &str, familiar| CompiledCreaturePolicy {
            definition_key: format!("creature:{name}"),
            definition_revision: "test-1".into(),
            display_name: name.into(),
            maximum_health: 812,
            base_speed: 200,
            outfit_look_type: 991,
            object_look_type: None,
            summonable: true,
            convinceable: true,
            mana_cost: Some(250),
            is_familiar: familiar,
            condition_immunities: vec![ConditionType::Haste],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            armor: Some(10),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        };
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    [1; 32],
                    vec![policy("familiar", true), policy("ordinary", false)],
                )
                .unwrap(),
            )
            .unwrap();
        let familiar = runtime
            .create_companion(
                actor,
                session,
                "familiar",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                1,
            )
            .unwrap();
        let ordinary = runtime
            .create_companion(
                actor,
                session,
                "ordinary",
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        let census = runtime.owned_companions(actor, session).unwrap();
        let player = super::super::cast::PlayerSpellState::new(
            super::super::cast::CharacterCastFacts {
                vocation: super::super::Vocation::Knight,
                level: 291,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 1_000_000,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        for (name, caster_delta, familiar_delta, caster_ms, familiar_ms) in [
            ("haste", 107, 96, 30000, 33000),
            ("strong haste", 252, 224, 22000, 22000),
            ("charge", 324, 288, 5000, 5000),
            ("swift foot", 320, 248, 10000, 10000),
        ] {
            let mut next = player.clone();
            let updates = stage_haste(&mut next, &profile(name), &census, actor, &facts).unwrap();
            assert_eq!(updates.len(), 1);
            assert_eq!(updates[0].0.actor, familiar);
            assert_eq!(
                updates[0].1.conditions.speed_delta_at(facts.now),
                familiar_delta
            );
            assert_eq!(
                updates[0].1.current_speed(facts.now).unwrap(),
                200 + familiar_delta as i32
            );
            assert_eq!(next.conditions.speed_delta_at(facts.now), caster_delta);
            assert_eq!(
                next.conditions.speed_delta_at(facts.now + caster_ms * 1000),
                0
            );
            assert_eq!(
                updates[0]
                    .1
                    .conditions
                    .speed_delta_at(facts.now + familiar_ms * 1000),
                0
            );
            if name == "swift foot" {
                assert!(matches!(
                    next.conditions.attributes_at(facts.now),
                    Some(ConditionValues::Attributes {
                        damage_dealt_percent: 70,
                        ..
                    })
                ));
                assert_eq!(next.conditions.attributes_at(facts.now + 10_000_000), None);
            }
            assert_eq!(runtime.owned_companions(actor, session).unwrap(), census);
            assert!(
                runtime
                    .companion_snapshot(ordinary)
                    .unwrap()
                    .state
                    .conditions
                    .instances()
                    .is_empty()
            );
        }
    }
}

/// Staged changes of the existing physical companion carriers. The Channel
/// owner proves the complete snapshot set and atomically compare-commits these
/// together with its PlayerSpellState successor and cast anchor.
pub(crate) fn stage_haste(
    state: &mut super::cast::PlayerSpellState,
    spell: &CompiledNativeSpell,
    companions: &[crate::foundation::CompanionSnapshot],
    caster: crate::foundation::ExactActorRef,
    facts: &ApplicationFacts<'_>,
) -> Result<
    Vec<(
        crate::foundation::CompanionSnapshot,
        crate::foundation::CompanionState,
    )>,
    HasteExecutionError,
> {
    let doc = spell.spell();
    let native = &doc["execution"]["native_behavior"];
    if native["key"] != "companion_haste" || u32::from(facts.base_speed) != state.base_speed {
        return Err(HasteExecutionError::UnsupportedProfile);
    }
    let p = &native["parameters"];
    let caster_values = &p["caster"];
    let parse_thousandths = |value: &serde_json::Value| -> Result<i32, HasteExecutionError> {
        let text = value
            .as_str()
            .ok_or(HasteExecutionError::UnsupportedProfile)?;
        let value = text
            .parse::<f64>()
            .map_err(|_| HasteExecutionError::UnsupportedProfile)?
            * 1000.0;
        if !value.is_finite()
            || value < i32::MIN as f64
            || value > i32::MAX as f64
            || value.fract() != 0.0
        {
            return Err(HasteExecutionError::UnsupportedProfile);
        }
        Ok(value as i32)
    };
    let formula = &caster_values["formula"];
    let a = parse_thousandths(&formula["mina"])?;
    let b = i32::try_from(
        formula["minb"]
            .as_str()
            .ok_or(HasteExecutionError::UnsupportedProfile)?
            .parse::<i64>()
            .map_err(|_| HasteExecutionError::UnsupportedProfile)?,
    )
    .map_err(|_| HasteExecutionError::UnsupportedProfile)?;
    if formula["mina"] != formula["maxa"] || formula["minb"] != formula["maxb"] {
        return Err(HasteExecutionError::UnsupportedProfile);
    }
    let duration = u32::try_from(
        caster_values["duration_ms"]
            .as_u64()
            .ok_or(HasteExecutionError::UnsupportedProfile)?,
    )
    .map_err(|_| HasteExecutionError::UnsupportedProfile)?;
    let key = doc["identity"]["key"]
        .as_str()
        .ok_or(HasteExecutionError::UnsupportedProfile)?;
    let mut staged = state.clone();
    let mut outputs = Vec::new();
    let fp = &p["familiar"];
    let multiplier = fp["multiplier"]
        .as_str()
        .ok_or(HasteExecutionError::UnsupportedProfile)?
        .parse::<f64>()
        .map_err(|_| HasteExecutionError::UnsupportedProfile)?;
    let offset = fp["offset"]
        .as_str()
        .ok_or(HasteExecutionError::UnsupportedProfile)?
        .parse::<f64>()
        .map_err(|_| HasteExecutionError::UnsupportedProfile)?;
    let familiar_duration = u32::try_from(
        fp["duration_ms"]
            .as_u64()
            .ok_or(HasteExecutionError::UnsupportedProfile)?,
    )
    .map_err(|_| HasteExecutionError::UnsupportedProfile)?;
    for snapshot in companions.iter().filter(|s| s.state.policy.is_familiar) {
        if snapshot
            .state
            .master
            .as_ref()
            .is_none_or(|m| m.actor != caster)
        {
            return Err(HasteExecutionError::CompanionOwnerUnavailable);
        }
        let base = u16::try_from(snapshot.state.policy.base_speed)
            .map_err(|_| HasteExecutionError::UnsupportedProfile)?;
        let delta =
            (f64::from(state.base_speed.max(u32::from(base))) * multiplier + offset).trunc();
        if !delta.is_finite() || delta <= f64::from(i32::MIN) || delta > f64::from(i32::MAX) {
            return Err(HasteExecutionError::UnsupportedProfile);
        }
        let mut delta = delta as i32;
        let paralysis = delta <= 0;
        if delta == 0 {
            delta = 40 - i32::from(base);
        }
        let definition = ConditionDefinition::new(
            key,
            1,
            ConditionValues::Speed {
                paralysis,
                range: SpeedRange {
                    a_min: 0,
                    b_min: 0,
                    a_max: 0,
                    b_max: 0,
                },
                duration_ms: familiar_duration,
            },
        )
        .ok_or(HasteExecutionError::UnsupportedProfile)?;
        let mut next = snapshot.state.clone();
        let familiar_facts = ApplicationFacts {
            base_speed: base,
            target_is_player: false,
            ..*facts
        };
        next.conditions
            .apply_explicit_spell_speed(
                &definition,
                delta,
                Some(caster),
                // Pinned creature.cpp1378 direct addCondition checks suppression,
                // not combat immunity. Monster suppression inherits false.
                &[],
                &familiar_facts,
            )
            .map_err(HasteExecutionError::Condition)?;
        outputs.push((snapshot.clone(), next));
    }
    let definition = ConditionDefinition::new(
        key,
        1,
        ConditionValues::Speed {
            paralysis: false,
            range: SpeedRange {
                a_min: a,
                b_min: b,
                a_max: a,
                b_max: b,
            },
            duration_ms: duration,
        },
    )
    .ok_or(HasteExecutionError::UnsupportedProfile)?;
    staged
        .conditions
        .apply_spell_haste(&definition, None, &[], facts)
        .map_err(HasteExecutionError::Condition)?;
    if doc["name"] == "Swift Foot" {
        let attributes = ConditionDefinition::new(
            "spell.swift.attributes",
            1,
            ConditionValues::Attributes {
                duration_ms: 10000,
                critical_chance_percent: 0,
                critical_extra_percentage_points: 0,
                damage_dealt_percent: 70,
                incoming_reduction_percent: 0,
            },
        )
        .ok_or(HasteExecutionError::UnsupportedProfile)?;
        match staged.conditions.apply(
            &attributes,
            None,
            crate::ability::condition::ConditionSourceKind::SelfUse,
            &[],
            facts,
        ) {
            Ok(_) | Err(crate::ability::condition::ConditionRefusal::KeptCurrent) => {}
            Err(error) => {
                return Err(HasteExecutionError::Condition(SpellSpeedError::Admission(
                    error,
                )));
            }
        }
    }
    *state = staged;
    Ok(outputs)
}
