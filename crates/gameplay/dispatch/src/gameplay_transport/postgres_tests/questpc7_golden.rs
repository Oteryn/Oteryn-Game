//! The selected Golden commission starts fresh and pays each exact milestone.
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const TRACK: &str = "oteryn:quest-progress/questpc7/golden/commission";

#[test]
fn questpc7_golden_full_commission_charges_exactly_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-golden", || {
        super::quest_world_callbacks::scenario(
            "questpc7-golden", Position { x: 33294, y: 31677, floor: -15 }, false,
        )
    });
    eprintln!("QUESTPC7_GOLDEN POSTGRES_CASE_COMPLETED duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}

async fn money(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOLDEN POSTGRES_STARTED version={version}");
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert!(admitted.continuity.selected_capabilities.contains(3));
    assert!(admitted.continuity.selected_capabilities.contains(6));
    assert!(admitted.continuity.selected_capabilities.contains(25));
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let [king] = super::classic_quests::install(owner, admitted, ["oteryn:npc.king_tibianus"]).await?;
    super::classic_quests::talk(owner, admitted, king, 800, "hi").await?;
    super::classic_quests::talk(owner, admitted, king, 801, "golden commission").await?;
    assert_eq!(super::classic_quests::progress(owner, admitted, authority, TRACK).await?, 0);
    for (index, (suffix, _offer, price)) in [
        ("base", "base outfit", 100000_i64),
        ("gloves", "boots and gloves", 50000),
        ("helmet", "golden helmet", 50000),
    ].into_iter().enumerate() {
        let callback = callbacks.iter().find(|c| c.key == format!("oteryn:quest-reward/questpc7-golden-{suffix}")).ok_or("callback")?;
        assert_eq!(money(h).await?, 0);
        // Keep the actual offer and confirmation on one connection, as required
        // by the normal conversation owner. Closing it clears engagement.
        let (unpaid, _, _) = super::qengine_rewards::wire_reward(owner, admitted, king, callback, false).await?;
        assert_eq!(unpaid, NpcIntentDisposition::Rejected, "a commission never grants without its payment");
        assert_eq!(super::classic_quests::progress(owner, admitted, authority, TRACK).await?, i64::try_from(index)?);
        let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
            position: Position { x: 33293, y: 31677, floor: -15 },
            item: "oteryn:item.tibia.i3043".into(), count: u16::try_from(price / 10000)?,
        }, 950 + u64::try_from(index)?, true).await?;
        assert_eq!(money(h).await?, price);
        let (result, _, last) = super::qengine_rewards::wire_grant(owner, admitted, king, callback).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(money(h).await?, 0, "each milestone consumes exactly its own price");
        assert_eq!(super::classic_quests::progress(owner, admitted, authority, TRACK).await?, i64::try_from(index + 1)?);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE character_id=encode($1,'hex')::uuid AND cause_kind=18")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(receipts, i64::try_from(index + 1)?);
        let replay = super::quest_world_callbacks::wire(owner, admitted, last, 7,
            oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&oteryn_protocol_oteryn::npc_service::NpcTalkIntent { npc_actor: king, text: "yes".into() }).map_err(debug)?).await?;
        assert_eq!(oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?, NpcIntentDisposition::Ok);
        assert_eq!(money(h).await?, 0);
    }
    super::classic_quests::talk(owner, admitted, king, 899, "hi").await?;
    super::classic_quests::talk(owner, admitted, king, 900, "complete commission").await?;
    assert_eq!(super::classic_quests::progress(owner, admitted, authority, TRACK).await?, 4);
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.golden_outfits_quest'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key LIKE 'oteryn:outfit.golden/quest-grant/%'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(grants, 3);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let observed = owner.appearance_observation(actor, admitted.game_session_id, vec![]).await.ok_or("appearance catalogue")?;
    let option = observed.catalogue.outfits.iter().find(|o| o.key == "oteryn:outfit.golden").ok_or("Golden entitlement")?;
    assert_eq!(option.permitted_addons, 3);
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection { outfit_key: "oteryn:outfit.golden".into(), colours: [0;4], addons: 3, mount_key: None },
        content_revision: owner.appearance_context().ok_or("cosmetics")?.1.content_revision().into(),
    };
    let next = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(owner, admitted, next, 25, change.encode().map_err(debug)?).await?;
    assert!(matches!(oteryn_protocol_oteryn::appearance::ChangeResult::decode(&result).map_err(debug)?, oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)));

    Ok(())
}
