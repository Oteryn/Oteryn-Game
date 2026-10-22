use super::postgres_scenario::equipment_scenario;
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_domain::domain::condition::{ConditionRefusal, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};

use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::equipment_move::{move_equipment, read_current_equipment};
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::item_view::EquipmentSlot as WireSlot;

#[test]
fn equipment_effects_owner_committed_moves_refresh_suppression_and_movement_on_postgres()
-> TestResult {
    std::thread::Builder::new()
        .name("equipment-effects-e2e".into())
        .stack_size(64 << 20)
        .spawn(|| equipment_scenario(None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "equipment effects e2e thread panicked")?
        .map_err(Into::into)
}

pub(super) async fn assert_committed_moves(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    boot_item: [u8; 16],
    f: crate::durability::item_transfer::CurrentCharacterItemFence,
    drunk: &ConditionDefinition,
    facts: &ApplicationFacts<'_>,
) -> TestResult {
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.spell_states.lock().await;
        states
            .initialize(
                &runtime,
                actor,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("spell state")?;
        states
            .get_mut(&runtime, actor, session)
            .ok_or("state")?
            .conditions
            .apply(
                drunk,
                Some("creature".into()),
                ConditionSourceKind::Creature,
                &[],
                facts,
            )
            .map_err(debug)?;
    }
    for (command_id, request, speed, suppressed) in [
        (
            3,
            EquipmentMove::Equip {
                item: ItemKey::Instance(boot_item),
                to: WireSlot::Feet,
            },
            30,
            true,
        ),
        (
            4,
            EquipmentMove::Unequip {
                item: ItemKey::Instance(boot_item),
                from: WireSlot::Feet,
            },
            0,
            false,
        ),
    ] {
        assert_eq!(
            move_equipment(
                owner,
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(f)
                },
                request
            )
            .await,
            EquipmentMoveResult::Committed
        );
        let equipment = read_current_equipment(owner, session)
            .await
            .ok_or("equipment")?;
        assert_eq!(speed_term(&equipment, native), Some(speed));
        let movement = owner
            .read_movement_equipment(actor, session)
            .await
            .ok_or("movement equipment")?;
        let runtime = owner.runtime.lock().await;
        let mut states = owner.spell_states.lock().await;
        assert_eq!(
            movement.current_delta(&runtime, &states, actor, session),
            Some(speed)
        );
        let conditions = &mut states
            .get_mut(&runtime, actor, session)
            .ok_or("state")?
            .conditions;
        assert!(!conditions.has_status(StatusKind::Drunk, 0));
        let result = conditions.apply(
            drunk,
            Some("creature".into()),
            ConditionSourceKind::Creature,
            &[],
            facts,
        );
        if suppressed {
            assert_eq!(result, Err(ConditionRefusal::Immune));
        } else {
            assert!(result.is_ok());
        }
    }
    Ok(())
}
