//! Native conversations from the pinned Ape City spoiler.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const QUESTLINE: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
const SHAMAN: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/shaman_outfit";

#[derive(Clone, Copy)]
enum Case {
    Syrup,
    Shaman,
    Catacombs,
}

#[test]
fn padreia_sells_one_syrup_for_fifty_gold_before_quest_start() -> TestResult {
    run_case("ape_city_syrup", Case::Syrup)
}

#[test]
fn hairycles_grants_shaman_on_confirmed_request_after_completion() -> TestResult {
    run_case("ape_city_shaman", Case::Shaman)
}

#[test]
fn hairycles_issues_snake_destroyer_when_catacombs_mission_is_accepted() -> TestResult {
    run_case("ape_city_catacombs", Case::Catacombs)
}

fn run_case(name: &'static str, case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || scenario(name, case));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario(name: &'static str, case: Case) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
        let fresh = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let pin = ChannelContentPin::from_activation(
            world,
            activated.activation_sequence(),
            activated.identity().server_artifact_digest(),
            activated.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            activated.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            pin,
            activated.qualified_room().clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(activated.identity().server_artifact_digest().as_slice())
            .bind(activated.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active content")?);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("session fence")?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                activated.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        admitted.item_fence = Some(fence.item_fence());
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        deps.initialize(&owner).await;
        match case {
            Case::Syrup => syrup_purchase(&h, &mut owner, &mut admitted).await?,
            Case::Shaman => shaman_reward(&h, &mut owner, &mut admitted).await?,
            Case::Catacombs => catacombs_mission(&h, &mut owner, &mut admitted).await?,
        }
        h.cleanup().await
    })
}

async fn syrup_purchase(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 100)]).await?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.padreia"]).await?;

    // Two purchases exercise the same dialogue with no quest state seeded.
    for (i, word) in ["hi", "cough syrup", "yes", "cough syrup", "yes"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            super::qengine_rewards::talk(owner, admitted, npc, 100 + u64::try_from(i)?, word)
                .await?,
            NpcIntentDisposition::Ok,
            "Padreia should accept {word}"
        );
    }
    let purchases: (i64, i64) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(fee_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18",
        )
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(purchases, (2, 100));
    let syrup: (i64, i64) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i4828'",
        )
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(syrup, (2, 2), "one bottle per purchase");
    Ok(())
}

async fn unlocks(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key LIKE 'oteryn:outfit.shaman/quest-grant/%'",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn shaman_reward(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.hairycles"]).await?;
    conversation(owner, admitted, npc, 20, "shamanic powers").await?;
    assert_eq!(unlocks(h).await?, 0, "before quest completion");

    // Set only the historical questline prerequisite. NPC_TALK must write the
    // separate outfit flag and the durable outfit receipt in the same request.
    historical_stage(h, owner, admitted, 18).await?;
    conversation(owner, admitted, npc, 30, "shamanic powers").await?;
    assert_eq!(unlocks(h).await?, 1, "confirmed Shaman base outfit");
    let flag: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(SHAMAN)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(flag, 1);

    conversation(owner, admitted, npc, 40, "shamanic powers").await?;
    assert_eq!(unlocks(h).await?, 1, "repeated request cannot grant twice");
    Ok(())
}

async fn catacombs_mission(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    super::item_look::seed_main_backpack(h, &[]).await?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.hairycles"]).await?;
    historical_stage(h, owner, admitted, 16).await?;
    conversation(owner, admitted, npc, 20, "mission").await?;

    let stage: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(QUESTLINE)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stage, 17, "acceptance enables the stage-17 entry condition");
    conversation(owner, admitted, npc, 30, "mission").await?;
    let tools: Vec<i64> = sqlx::query_scalar(
        "SELECT i.quantity FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i4835'",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(tools, [1], "acceptance gives a single Snake Destroyer once");
    Ok(())
}

async fn historical_stage(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    stage: i64,
) -> TestResult {
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(QUESTLINE.into(), stage)],
        106,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    first: u64,
    keyword: &str,
) -> TestResult {
    for (offset, word) in ["hi", keyword, "yes", "bye"].into_iter().enumerate() {
        super::qengine_rewards::talk(owner, admitted, npc, first + u64::try_from(offset)?, word)
            .await?;
    }
    Ok(())
}
