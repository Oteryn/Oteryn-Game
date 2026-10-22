use super::postgres_scenario::equipment_scenario;
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_domain::domain::condition::{ConditionRefusal, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};

#[test]
fn equipment_effects_owner_first_entry_uses_equipped_suppression_on_postgres() -> TestResult {
    std::thread::Builder::new()
        .name("equipment-effects-admission-e2e".into())
        .stack_size(64 << 20)
        .spawn(|| {
            for equipped in [true, false] {
                equipment_scenario(Some(equipped)).map_err(|error| error.to_string())?;
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "equipment effects admission e2e thread panicked")?
        .map_err(Into::into)
}

pub(super) async fn assert_first_entry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    boot_item: [u8; 16],
    request: crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
    equipped: bool,
    drunk: &ConditionDefinition,
    facts: &ApplicationFacts<'_>,
) -> TestResult {
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
        convert_vocation,
    };
    // Durable setup represents a returning Character with a class and, in the
    // positive case, an item already in its own slot before any runtime state.
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    let formula = native.training_formula().ok_or("training formula")?;
    let chosen = owner
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "build fence")?
                .ok_or("current session")?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: convert_vocation(formula, &DurableBuildState::default(), "druid")
                    .map_err(debug)?,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
    if equipped {
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(owner.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("equipment fence")?;
        let moved = owner
            .root
            .commit_equipment_move(
                owner.character,
                owner.holder,
                &fence,
                crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(3).map_err(debug)?,
                ),
                &weights,
                crate::durability::character_equipment::EquipmentMove {
                    item: boot_item,
                    from_slot: None,
                    to_slot: Some(4),
                },
            )
            .await
            .map_err(debug)?;
        assert!(moved.is_ok(), "{moved:?}");
    }
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .is_none()
        );
        assert!(runtime.read_actor_position(actor).is_err());
    }
    let (outcome, fence, position) = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        owner.initialize_first_entry(
            &request,
            session,
            crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
            actor,
        ),
    )
    .await?;
    assert_eq!(
        outcome,
        crate::gameplay_transport::connection::FirstEntryOutcome::Positioned
    );
    assert!(fence.is_some());
    assert!(position.is_some());
    let runtime = owner.runtime.lock().await;
    let mut states = owner.spell_states.lock().await;
    let conditions = &mut states
        .get_mut(&runtime, actor, session)
        .ok_or("admitted state")?
        .conditions;
    let result = conditions.apply(
        drunk,
        Some("creature".into()),
        ConditionSourceKind::Creature,
        &[],
        facts,
    );
    if equipped {
        assert_eq!(result, Err(ConditionRefusal::Immune));
        assert!(!conditions.has_status(StatusKind::Drunk, 0));
    } else {
        assert!(result.is_ok());
        assert!(conditions.has_status(StatusKind::Drunk, 0));
    }
    Ok(())
}
