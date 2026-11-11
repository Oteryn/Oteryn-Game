//! Chosen Wrath opening: source Zalamon and exact native crate construction.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const STAGE: &str = "oteryn:quest-progress/chosen/wrath/stage";
const NPC: &str = "oteryn:npc.zalamon";

#[test]
fn questpc8_wrath_native_opening_exact_materials_crate_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-wrath-opening", || {
        super::quest_world_callbacks::scenario(
            "authored-wrath-opening",
            Position { x: 33354, y: 31410, floor: -8 },
            false,
        )
    });
    eprintln!("POSTGRES_CASE_FINISHED questpc8_wrath_native_opening duration={:?} result={result:?}", started.elapsed());
    result
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    super::questpc8_native_slice::world(
        active.native_gameplay().ok_or("native")?,
        &[(33349, 31405, 33359, 31415, -8)],
        &[],
        TilePos { x: 33353, y: 31410, floor: -8 },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    eprintln!("POSTGRES_CASE_STARTED questpc8_wrath_opening after PostgreSQL fixture initialization");
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(), &[3],
        ).ok_or("NPC capability")?;
    let npc = install(owner, admitted).await?;
    super::questpc8_mystery_geometry::arrive(owner, admitted,
        TilePos { x: 33353, y: 31410, floor: -8 }).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    super::grizzly_npc::wire::talk(owner, admitted, npc, &[(1, "hi"), (2, "wrath mission")]).await?;
    assert_eq!(stage(h).await?, 0, "real NPC starts the chosen mission");
    carry(owner, admitted, 953, 2, 810).await?;
    super::grizzly_npc::wire::talk_refused(owner, admitted, npc, &[(3, "hi"), (4, "crate")]).await?;
    assert_eq!(stage(h).await?, 0);
    assert_eq!(quantity(h, 953).await?, 2, "missing inputs never partially charge");
    assert_eq!(quantity(h, 11328).await?, 0);
    carry(owner, admitted, 5901, 1, 811).await?;
    super::grizzly_npc::wire::talk_refused(owner, admitted, npc, &[(5, "hi"), (6, "crate")]).await?;
    assert_eq!(quantity(h, 953).await?, 2, "two nails do not satisfy three");
    assert_eq!(quantity(h, 5901).await?, 1, "wood is retained on a failed hand-in");
    carry(owner, admitted, 953, 1, 812).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    super::grizzly_npc::wire::talk(owner, admitted, npc, &[(7, "hi"), (8, "crate")]).await?;
    assert_eq!(stage(h).await?, 1);
    assert_eq!(quantity(h, 953).await?, 0);
    assert_eq!(quantity(h, 5901).await?, 0);
    assert_eq!(quantity(h, 11328).await?, 1, "exact source marked crate");
    super::grizzly_npc::wire::talk(owner, admitted, npc, &[(8, "crate")]).await?;
    assert_eq!(quantity(h, 11328).await?, 1, "same command cannot duplicate the crate");
    owner.states.quest_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    super::grizzly_npc::wire::talk_refused(owner, admitted, npc, &[(9, "hi"), (10, "crate")]).await?;
    assert_eq!(stage(h).await?, 1);
    assert_eq!(quantity(h, 11328).await?, 1);
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(!completed, "the opening slice does not complete the whole quest");
    eprintln!("POSTGRES_WRATH_EXACT_CRATE_OPENING_AND_REPLAY_PASS");
    Ok(())
}

async fn install(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) -> TestResult<EntityRef> {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"), npc_catalogue_preproduction_limits(),
    ).map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(NpcTalkBook::from_model_with_native_quests(&model, &[NPC])));
    owner.states.chat.admit(admitted.game_session_id, "Rebel helper", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime.bind_attacker_lease(
        admitted.runtime_actor.ok_or("actor")?, admitted.game_session_id,
        crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        ).map_err(debug)?,
    ).map_err(debug)?;
    runtime.place_npcs(&NpcPlacementTable { npcs: vec![NpcPlacements {
        key: NPC.into(), placements: vec![NpcPlacement { x:33354, y:31410, floor:-8, direction:MovementFacing::North }],
    }] }, |_| true, true).map_err(debug)?;
    runtime.visible_entities().npcs.into_iter().find_map(|npc| {
        (runtime.npc_key(npc.actor)==Some(NPC)).then_some(EntityRef { identity:npc.actor.placement_identity(), generation:npc.generation })
    }).ok_or_else(|| "source Zalamon".into())
}

async fn carry(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession, item:u16, count:u16, occurrence:u64) -> TestResult {
    let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
        position:Position { x:33353,y:31410,floor:-8 },item:format!("oteryn:item.tibia.i{item}"),count,
    },occurrence,true).await
}
async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)")
        .bind(id(41).as_slice()).bind(STAGE).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness,item:u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
