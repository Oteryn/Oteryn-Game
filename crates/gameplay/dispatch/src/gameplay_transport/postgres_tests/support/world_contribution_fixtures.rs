//! Shared contribution scenarios and authored travel data.
use super::TestResult;
use crate::durability::npc_travel::TravelRefusal;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

pub(super) fn junkar_route(
    key: &str,
) -> TestResult<crate::content::npc_catalogue::travel::TravelRoute> {
    use super::fixture::debug;
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    let content = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = content.service_model_for_project().map_err(debug)?;
    let book = crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model);
    Ok(book
        .get("oteryn:npc.junkar_thais")
        .and_then(|entry| {
            entry
                .travel_routes
                .iter()
                .find(|route| route.route.key == key)
        })
        .cloned()
        .ok_or("native Junkar route")?)
}

pub(super) async fn save(
    h: &super::fixture::Harness,
    world: crate::foundation::WorldId,
    epoch: i64,
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
        VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z'::timestamptz + $2 * interval '1 day','2031-06')")
        .bind(world.as_bytes().as_slice()).bind(epoch).bind([1u8; 32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
    assert!(
        h.root
            .world_changes(world, u64::try_from(epoch)?)
            .await?
            .is_some()
    );
    Ok(())
}

pub(super) async fn quote_junkar_route(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    h: &super::fixture::Harness,
    session: crate::foundation::GameSessionId,
    route: &crate::content::npc_catalogue::travel::TravelRoute,
) -> TestResult<Result<u64, TravelRefusal>> {
    let fence = super::death_ml_loss_composed_fixtures::current_fence(owner, session).await?;
    h.root
        .quote_npc_travel(
            owner.character,
            &h.node,
            fence,
            route.clone(),
            &super::npcfix_m38_travel_guards_fixtures::NoPremium,
        )
        .await
        .map_err(super::fixture::debug)
}

pub(super) async fn steamship_donation(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
) -> TestResult {
    use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, reward_callbacks};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

    const NPC: &str = "oteryn:npc.junkar_thais";
    let loaded = load_embedded_quest_state("content-1").map_err(super::fixture::debug)?;
    let callback = reward_callbacks::load_for(loaded.catalogue())?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/authored/steamship/donation")
        .ok_or("production coal donation")?;
    owner.quest_catalogue = Some(std::sync::Arc::new(loaded.catalogue().clone()));
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.push(oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC service capability")?;
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    let world = admitted.world_id;
    super::worldchg_trader_fixtures::day(h, world, 0).await?;
    let mut ordinal = 8702;
    let (refused, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner,
        admitted,
        npc,
        &callback,
        false,
        vec!["hi", "coal", "yes"],
    )
    .await?;
    assert_eq!(refused, NpcIntentDisposition::Rejected);
    let route = junkar_route("kazordoon")?;
    assert_eq!(
        quote_junkar_route(owner, h, admitted.game_session_id, &route).await?,
        Err(TravelRefusal::UnavailableRoute)
    );

    for count in [20; 9].into_iter().chain([19]) {
        super::qengine_voucher_stories_fixtures::mint(owner, admitted, 12600, count, &mut ordinal)
            .await?;
        for _ in 0..count {
            let (accepted, _, _) = super::qengine_rewards_fixtures::wire_conversation(
                owner,
                admitted,
                npc,
                &callback,
                false,
                vec!["hi", "coal", "yes"],
            )
            .await?;
            assert_eq!(accepted, NpcIntentDisposition::Ok);
        }
    }
    super::qengine_voucher_stories_fixtures::mint(owner, admitted, 12600, 2, &mut ordinal).await?;
    let (accepted, _, command) = super::qengine_rewards_fixtures::wire_conversation(
        owner,
        admitted,
        npc,
        &callback,
        false,
        vec!["hi", "coal", "yes"],
    )
    .await?;
    assert_eq!(accepted, NpcIntentDisposition::Ok);
    let confirm = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: "yes".into(),
    })
    .map_err(super::fixture::debug)?;
    super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 7, confirm).await?;
    assert_eq!(
        super::qengine_voucher_stories_fixtures::quantity(h, 12600).await?,
        1
    );
    let tally: String = sqlx::query_scalar(
        "SELECT tally::text FROM game_world_change_tallies WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0 AND resource='coal'",
    ).bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(tally, "200");
    assert_eq!(
        quote_junkar_route(owner, h, admitted.game_session_id, &route).await?,
        Err(TravelRefusal::UnavailableRoute)
    );
    save(h, world, 1).await?;
    assert_eq!(
        quote_junkar_route(owner, h, admitted.game_session_id, &route).await?,
        Ok(route.route.price)
    );
    Ok(())
}
