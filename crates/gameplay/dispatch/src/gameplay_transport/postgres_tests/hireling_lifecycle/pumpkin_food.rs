//! Source-qualified Halloween food enters the unchanged framed native FOOD owner.
use super::{Harness, TestResult, debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::world_object::{self as wire, UseDisposition};

pub(in crate::gameplay_transport) async fn scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    super::super::item_look::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i3594", 2),
            ("oteryn:item.tibia.i8177", 2),
        ],
    )
    .await?;
    for (identifier, total_seconds) in [(3594, 204_u64), (8177, 216)] {
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        let definition = {
            let runtime = owner.runtime.lock().await;
            let index = crate::gameplay_transport::item_ref_admission::pinned_index(
                &runtime,
                owner.active_generation,
            )
            .ok_or("native index")?;
            let key = format!("oteryn:item.tibia.i{identifier}");
            index
                .definition_ref(
                    "Item",
                    &key,
                    index.revision(&key).ok_or("source food revision")?,
                )
                .ok_or("source food")?
        };
        let body = wire::encode_item_use_intent(&wire::UseIntent {
            target: wire::UseTarget::ItemByDefinition(definition),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let (status, payload) =
            super::wire::exchange(owner, &mut admitted, wire::COMMAND_TYPE_USE_INTENT, &body)
                .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert_eq!(
            wire::decode_use_result(&payload).map_err(debug)?,
            UseDisposition::Committed
        );
        let quantity: i64 = sqlx::query_scalar(
            "SELECT quantity FROM game_item_instances WHERE definition_production_key=$1",
        )
        .bind(format!("oteryn:item.tibia.i{identifier}"))
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            quantity, 1,
            "ordinary FOOD burns exactly one actual source unit"
        );
        let now = u64::try_from(owner.clock_origin.elapsed().as_millis())
            .map_err(debug)?
            .saturating_mul(1000);
        let states = owner.states.spell_states.lock().await;
        let state = &states
            .actors
            .iter()
            .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
            .ok_or("native actor")?
            .2;
        let food = state
            .owned_conditions()
            .active_at(
                oteryn_gameplay_domain::domain::condition::ConflictKey::FoodRegeneration,
                now,
            )
            .ok_or("native food condition")?;
        let remaining = u64::try_from(food.remaining_duration_ms(now).ok_or("food duration")?)
            .map_err(debug)?;
        assert!(
            remaining <= total_seconds * 1000 && remaining + 3000 >= total_seconds * 1000,
            "current Wiki food duration: {remaining}"
        );
    }
    Ok(())
}
