//! Real carried USE in a named House; the same native source owner serves Channel custody.
use super::{Harness, TestResult, debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
pub(in crate::gameplay_transport) async fn scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    sqlx::query("CREATE OR REPLACE FUNCTION public.game_daily_effect_now() RETURNS TIMESTAMPTZ LANGUAGE SQL STABLE AS $$ SELECT '2026-11-06 12:00:00+00'::timestamptz $$").execute(&h.pool).await?;
    super::super::item_look::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i28484", 2),
            ("oteryn:item.tibia.i28485", 1),
            ("oteryn:item.tibia.i28486", 1),
            ("oteryn:item.tibia.i29409", 1),
        ],
    )
    .await?;
    let refs = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .ok_or("index")?;
        [28484, 28485, 28486].map(|id| {
            let key = format!("oteryn:item.tibia.i{id}");
            index
                .revision(&key)
                .and_then(|revision| index.definition_ref("Item", &key, revision))
                .ok_or("cupcake")
        })
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: admitted.item_fence,
    };
    let carrot = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .ok_or("index")?;
        let key = "oteryn:item.tibia.i29409";
        index
            .revision(key)
            .and_then(|revision| index.definition_ref("Item", key, revision))
            .ok_or("carrot")?
    };
    assert_eq!(
        owner
            .use_item_definition(
                actor,
                UseCommand {
                    command_id: 8999,
                    ..command
                },
                carrot
            )
            .await
            .disposition,
        UseDisposition::Committed,
        "existing source Gourmet condition is a real predecessor"
    );
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;

    let revision = {
        let mut states = owner.states.spell_states.lock().await;
        let (_, _, state) = states
            .actors
            .iter_mut()
            .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
            .ok_or("state")?;
        state.restore_saved_vitals(7, 9);
        state.revision()
    };
    let body = oteryn_protocol_oteryn::world_object::encode_item_use_intent(
        &oteryn_protocol_oteryn::world_object::UseIntent {
            target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(
                refs[0].map_err(debug)?,
            ),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        },
    )
    .map_err(debug)?;
    let (status, payload) = super::wire::exchange(
        owner,
        &mut admitted,
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
        &body,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&payload).map_err(debug)?,
        UseDisposition::Committed
    );

    assert_eq!(
        owner
            .use_item_definition(actor, command, refs[0].map_err(debug)?)
            .await
            .disposition,
        UseDisposition::Committed,
        "native replay cannot consume twice"
    );
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    assert_eq!(
        owner
            .use_item_definition(
                actor,
                UseCommand {
                    command_id: 9001,
                    ..command
                },
                refs[0].map_err(debug)?
            )
            .await
            .disposition,
        UseDisposition::Exhausted
    );
    for (offset, reference) in refs.iter().enumerate().skip(1) {
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        assert_eq!(
            owner
                .use_item_definition(
                    actor,
                    UseCommand {
                        command_id: 9001 + offset as u64,
                        ..command
                    },
                    (*reference).map_err(debug)?
                )
                .await
                .disposition,
            UseDisposition::Committed,
            "other cupcake cooldown is independent"
        );
    }
    let states = owner.states.spell_states.lock().await;
    let (_, _, state) = states
        .actors
        .iter()
        .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
        .ok_or("after state")?;
    assert_eq!(state.revision(), revision + 3);
    assert_eq!(state.vitals().mana, state.character_facts().max_mana);
    assert_eq!(state.health, state.character_facts().max_health);
    assert_eq!(
        state
            .owned_skill_adjustments(owner.owner_now().get())
            .map_err(debug)?
            .adjust_skill(10, 4),
        Some(20)
    );
    drop(states);
    let old_attributes:bool=sqlx::query_scalar("SELECT attributes_item IS NULL AND attributes_ms=0 AND cooldown_until='2026-11-06 12:10:00+00'::timestamptz FROM game_hireling_gourmet_state").fetch_one(&h.pool).await?;
    assert!(
        old_attributes,
        "Lemon FORCEUPDATE retires the real old attribute projection while preserving its independent exhaustion"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_seasonal_cupcake_uses")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 3);
    let burns: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_use_records WHERE cause_kind=579")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(burns, 3);
    sqlx::query("CREATE OR REPLACE FUNCTION public.game_daily_effect_now() RETURNS TIMESTAMPTZ LANGUAGE SQL STABLE AS $$ SELECT '2026-11-06 12:10:00+00'::timestamptz $$").execute(&h.pool).await?;
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    assert_eq!(
        owner
            .use_item_definition(
                actor,
                UseCommand {
                    command_id: 9010,
                    ..command
                },
                refs[0].map_err(debug)?
            )
            .await
            .disposition,
        UseDisposition::Committed,
        "the independent600s deadline is exclusive; use remains available in November"
    );
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM game_seasonal_cupcake_uses")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(total, 4);
    let mut stale = command.item_fence.ok_or("fence")?;
    stale.character_lease_generation += 1;
    assert_ne!(
        owner
            .use_item_definition(
                actor,
                UseCommand {
                    command_id: 9011,
                    item_fence: Some(stale),
                    ..command
                },
                refs[2].map_err(debug)?
            )
            .await
            .disposition,
        UseDisposition::Committed
    );
    Ok(())
}
