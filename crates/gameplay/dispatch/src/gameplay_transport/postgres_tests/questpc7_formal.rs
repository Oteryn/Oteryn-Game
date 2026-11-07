//! Personal exhibition, native display, consumptive accessories and final achievement.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

const TRACK: &str = "oteryn:quest-progress/questpc7/formal/programme";
#[test]
fn questpc7_formal_exhibition_display_and_consumptive_accessories() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-formal", || {
        super::quest_world_callbacks::scenario(
            "questpc7-outfits-formal",
            Position {
                x: 32315,
                y: 32170,
                floor: -6,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FORMAL POSTGRES_CASE_COMPLETED duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}
async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<i64> {
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let state = owner.root.read_character_quest_state(owner.character, fence.character_id)
        .await.map_err(debug)?;
    Ok(state.tracks().get(TRACK).copied().unwrap_or(-1))
}
async fn display(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: &Callback,
) -> TestResult<UseDisposition> {
    // Refused world actions retain the production 200 ms admission cooldown.
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let p = callback.selector.position.ok_or("position")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("key")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let next = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, admitted, next, 2, payload).await?;
    decode_use_result(&reply).map_err(debug)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_FORMAL POSTGRES_STARTED version={version}");
    let [king] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.king_tibianus"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert_eq!(
        progress(owner, admitted).await?,
        -1,
        "no subject stage fixture seeds"
    );
    assert_eq!(
        display(owner, admitted, callback).await?,
        UseDisposition::Rejected,
        "no display reward before admission and exploration"
    );
    let mut sequence = 3000;
    assert_eq!(
        super::questpc7_outfit_items::talk(
            owner,
            admitted,
            king,
            &mut sequence,
            "anniversary exhibition"
        )
        .await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(progress(owner, admitted).await?, 0);
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    for direction in [StepDirection::West, StepDirection::East] {
        let next = admitted.continuity.next_command_id;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            next,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
    }
    assert_eq!(
        progress(owner, admitted).await?,
        1,
        "real native exploration enters the exhibition"
    );
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    let refused = display(owner, admitted, callback).await?;
    owner.premium_coordinator = premium;
    assert_eq!(
        refused,
        UseDisposition::Rejected,
        "a prior admission or session Premium bit cannot replace current world reward authority"
    );
    assert_eq!(progress(owner, admitted).await?, 1);
    let grants:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key LIKE 'oteryn:outfit.formal_dress/quest-grant/%'").fetch_one(&h.pool).await?;
    assert_eq!(grants, 0);
    assert_eq!(
        display(owner, admitted, callback).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(owner, admitted).await?, 2);
    assert_eq!(
        super::questpc7_outfit_items::talk(
            owner,
            admitted,
            king,
            &mut sequence,
            "formal brass button"
        )
        .await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(super::questpc7_outfit_items::quantity(h, 37604).await?, 1);
    assert_eq!(
        super::questpc7_outfit_items::talk(
            owner,
            admitted,
            king,
            &mut sequence,
            "formal decorative plume"
        )
        .await?,
        NpcIntentDisposition::Rejected
    );
    super::questpc7_outfit_items::use_token(owner, admitted, 37604).await?;
    assert_eq!(super::questpc7_outfit_items::quantity(h, 37604).await?, 0);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let seen = owner
        .appearance_observation(actor, admitted.game_session_id, vec![])
        .await
        .ok_or("appearance")?;
    assert_eq!(
        seen.catalogue
            .outfits
            .iter()
            .find(|o| o.key == "oteryn:outfit.formal_dress")
            .ok_or("formal")?
            .permitted_addons,
        2,
        "Brass Button retains the real source addon2"
    );
    assert_eq!(
        super::questpc7_outfit_items::talk(
            owner,
            admitted,
            king,
            &mut sequence,
            "formal decorative plume"
        )
        .await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(super::questpc7_outfit_items::quantity(h, 37605).await?, 1);
    assert_eq!(
        super::questpc7_outfit_items::talk(
            owner,
            admitted,
            king,
            &mut sequence,
            "complete formal dress"
        )
        .await?,
        NpcIntentDisposition::Rejected
    );
    super::questpc7_outfit_items::use_token(owner, admitted, 37605).await?;
    assert_eq!(super::questpc7_outfit_items::quantity(h, 37605).await?, 0);
    assert_eq!(
        super::questpc7_outfit_items::talk(
            owner,
            admitted,
            king,
            &mut sequence,
            "complete formal dress"
        )
        .await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(progress(owner, admitted).await?, 5);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.formal_dress_outfits_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    let achievements:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/sharp_dressed'").fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);
    let seen = owner
        .appearance_observation(actor, admitted.game_session_id, vec![])
        .await
        .ok_or("appearance")?;
    assert_eq!(
        seen.catalogue
            .outfits
            .iter()
            .find(|o| o.key == "oteryn:outfit.formal_dress")
            .ok_or("formal")?
            .permitted_addons,
        3
    );
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: seen.catalogue.context,
        expected_root_revision: seen.catalogue.root_revision,
        prior: seen.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: "oteryn:outfit.formal_dress".into(),
            colours: [0; 4],
            addons: 3,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let next = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        next,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    Ok(())
}
