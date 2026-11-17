//! Live Heoni and Glitterscale deaths, followed by ordinary item-handle USE.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn qfather_plain_use_grants_each_material_from_another_players_kill_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("qfather-corpses", || {
        super::quest_world_callbacks::scenario(
            "qfather-corpses",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    at: Position,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    let selected = &[3, 4, 6, 11, 15, 18];
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            selected,
        )
        .ok_or("corpse-use capabilities")?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let killer =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 60, 62, 70, now)
            .await?;
    let killer_session = killer.current_game_session_id();
    let killer_character = *killer.current_character_lease().character_id().as_bytes();
    let killer_actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(killer_session)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, killer_session, killer.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, super::quest_world_callbacks::local(at))
            .map_err(debug)?;
        actor
    };
    let mut killer = AdmittedSession {
        game_session_id: killer_session,
        runtime_actor: Some(killer_actor),
        controller: None,
        item_fence: crate::gameplay_transport::item_fence_of(killer),
        ..*admitted
    };
    Box::pin(owner.admit_progression_session(&mut killer)).await;
    super::questpc7_native_deaths::prepare_native_hunter(owner, &killer).await?;
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    for (creature, corpse, material, command) in [
        (
            "oteryn:creature.heoni",
            "oteryn:item.tibia.i11590",
            "oteryn:item.tibia.i11548",
            11301,
        ),
        (
            "oteryn:creature.glitterscale",
            "oteryn:item.tibia.i11589",
            "oteryn:item.tibia.i11550",
            11302,
        ),
    ] {
        super::questpc7_goblin::stalker_death(owner, &mut killer, creature, at, command).await?;
        let top_damage: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(r.corpse_top_damage_character_id)
             FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key=$1 AND r.materialized_at IS NOT NULL",
        )
        .bind(corpse)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(top_damage, killer_character, "{creature}");
        assert_ne!(top_damage, id(41), "the user did not kill {creature}");

        let index = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.item_index())
            .ok_or("native Item index")?;
        let source = index
            .definition_ref("Item", corpse, "definition-r1")
            .ok_or("corpse item definition")?;
        for expected in [UseDisposition::Committed, UseDisposition::Rejected] {
            let (end, result, _) = super::rust_lottery::exchange(
                owner,
                *admitted,
                admitted.continuity.next_command_id,
                &intent,
                Some(source),
                None,
            )
            .await?;
            *admitted = end;
            assert_eq!(result, expected, "{creature}");
            let awarded: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(i.quantity), 0)::bigint
                 FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
                 WHERE e.character_id=encode($1,'hex')::uuid
                   AND i.lifecycle=1 AND i.definition_production_key=$2",
            )
            .bind(id(41).as_slice())
            .bind(material)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(awarded, 1, "{creature}");
        }
    }
    Ok(())
}
