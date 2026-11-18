//! Lazaran's Skulled offer and confirmation through production NPC dispatch.
use super::TestResult;
use super::full_world_support::world_scenario;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};

#[test]
fn skulled_requires_zao_and_premium_then_starts_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-unnatural-selection", || {
        world_scenario!("qcx_unnatural_selection", |h, _request, _seal, authority, world_id,
            native, world, cells, room, pin, deps, actor, session, owner, fence, admitted| {
            use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, unnatural_selection};
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            let premium = SpellPremiumCoordinator::new(
                consumer, Some(clock), Some(activation), Some(h.root.clone()),
            );
            owner.quest_catalogue = Some(Arc::new(
                load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone(),
            ));
            owner.admit_quest_session(&admitted).await;
            let [lazaran] = super::classic_quests::install(
                &owner, &admitted, ["oteryn:npc.lazaran"],
            ).await?;

            // With Premium and no Zao prerequisite, the mission cannot start.
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), session);
            assert!(owner.session_premium_gate(session).await.premium_current());
            attempt(&mut owner, &mut admitted, lazaran, 100).await?;
            assert_eq!(super::m39_bone_journey::stage(&h, unnatural_selection::QUESTLINE_TRACK).await?, None);

            // Seed only the external Zao-access prerequisite.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,3,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice())
                .bind("oteryn:quest-progress/quest/u8_54/the_new_frontier/mission03")
                .bind("oteryn:quest.the_new_frontier_quest")
                .execute(&mut *tx).await?;
            tx.commit().await?;
            owner.refresh_quest_session(session, false).await;

            owner.premium_coordinator = None;
            assert!(!owner.session_premium_gate(session).await.premium_current());
            attempt(&mut owner, &mut admitted, lazaran, 200).await?;
            assert_eq!(super::m39_bone_journey::stage(&h, unnatural_selection::QUESTLINE_TRACK).await?, None);

            owner.premium_coordinator = Some(&premium);
            let reply = attempt(&mut owner, &mut admitted, lazaran, 300).await?;
            assert_eq!(reply, ["You hero of our tribe if bring back holy skull!"]);
            assert_eq!(super::m39_bone_journey::stage(&h, unnatural_selection::QUESTLINE_TRACK).await?, Some(1));
            assert_eq!(super::m39_bone_journey::stage(&h, "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission01").await?, Some(1));
            attempt(&mut owner, &mut admitted, lazaran, 400).await?;
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/unnatural-selection-skulled-start'")
                .fetch_one(&h.pool).await?;
            assert_eq!(receipts, 1);
            Ok(())
        })
    });
    eprintln!("POSTGRES_CASE_COMPLETED qcx_unnatural_selection duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}

async fn attempt(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    first: u64,
) -> TestResult<Vec<String>> {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
    let mut reply = Vec::new();
    for (offset, text) in ["hi", "help", "yes", "mission", "yes", "bye"].into_iter().enumerate() {
        owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
        let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
        let outcome = owner.npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand { game_session_id: admitted.game_session_id, command_id: first + u64::try_from(offset)?, item_fence: Some(fence.item_fence()) },
            NpcTalkIntent { npc_actor: npc, text: text.into() },
        ).await;
        assert!(!outcome.unknown, "NPC dispatch must return a known outcome for {text}");
        if offset == 4 {
            reply = outcome.conversation.map(|conversation| conversation.lines).unwrap_or_default();
        }
    }
    Ok(reply)
}
