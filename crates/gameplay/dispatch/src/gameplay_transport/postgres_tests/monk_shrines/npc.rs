use super::*;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<(EntityRef, EntityRef, MovementLocalPosition)> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(NpcTalkBook::from_model(&model)));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Monk pilgrim", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    runtime
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: ["oteryn:npc.ambassador_manop", "oteryn:npc.enpa_deia_pema"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, key)| {
                        Ok(NpcPlacements {
                            key: key.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + i as i32)?,
                                y: u16::try_from(at.y + 1)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        })
                    })
                    .collect::<TestResult<Vec<_>>>()?,
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let find = |key| {
        runtime.visible_entities().npcs.into_iter().find_map(|n| {
            (runtime.npc_key(n.actor) == Some(key)).then_some(EntityRef {
                identity: n.actor.placement_identity(),
                generation: n.generation,
            })
        })
    };
    Ok((
        find("oteryn:npc.ambassador_manop").ok_or("Manop")?,
        find("oteryn:npc.enpa_deia_pema").ok_or("Pema")?,
        at,
    ))
}

pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    id: u64,
    text: &str,
) -> TestResult<String> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(
        outcome.disposition,
        NpcIntentDisposition::Ok,
        "{id}: {text}"
    );
    assert!(!outcome.unknown);
    Ok(outcome
        .conversation
        .map(|c| c.lines.join(" "))
        .unwrap_or_default())
}

pub(super) async fn finish(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    world: &crate::map::boot::BundleWorld,
    pema: EntityRef,
    at: MovementLocalPosition,
) -> TestResult {
    {
        let mut runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let expected = runtime.read_actor_position(actor).map_err(debug)?;
        let doors = crate::map::floor::BundleDoors(world.facts().doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells,
            actor,
            admitted.game_session_id,
            expected,
            at,
            false,
            &doors,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    talk(owner, admitted, pema, 1100, "hi").await?;
    let line = talk(owner, admitted, pema, 1101, "pilgrimage").await?;
    assert!(line.contains("complete"));
    let again = talk(owner, admitted, pema, 1102, "pilgrimage").await?;
    assert!(again.contains("reported"));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let copy = h
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(copy.tracks().get(monk_shrines::FINAL_TRACK), Some(&1));
    journal(&copy, 10, true)?;
    assert!(
        copy.states()
            .get(monk_shrines::QUEST)
            .is_some_and(|state| state.completed)
    );
    let report_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(monk_shrines::FINISH)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(report_count, 1);
    let title_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE title=5")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(title_count, 1);
    let achievement_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/hope_of_the_merudri'")
        .fetch_one(&h.pool).await?;
    assert_eq!(achievement_count, 1);
    Ok(())
}

pub(super) fn journal(
    copy: &crate::durability::quest_state::QuestStateCopy,
    expected_count: usize,
    completed: bool,
) -> TestResult {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let loaded = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let index = loaded
        .content
        .active()
        .quests()
        .iter()
        .position(|q| q.key == monk_shrines::QUEST)
        .ok_or("journal")?;
    let line = loaded
        .content
        .line(copy, index as u32)
        .ok_or("listed Monk journal")?;
    assert_eq!(line.completed, completed);
    assert_eq!(line.missions.len(), expected_count + 1);
    assert!(line.missions.iter().skip(1).all(|mission| mission.done));
    if completed {
        assert!(line.missions[0].text.contains("reported"));
    } else {
        assert!(
            line.missions[0]
                .text
                .contains(&format!("{expected_count}/10"))
        );
    }
    Ok(())
}
