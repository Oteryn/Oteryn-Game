use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

use super::item_forge_fusion_fixtures::fixture::{bank, seed_bank};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const TRACK: &str = "oteryn:quest-progress/questpc7/golden/commission";

async fn money(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn legacy(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use super::classic_quests_fixtures::{install, progress, talk};
    const LEGACY_TRACK: &str = "oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit";
    let [king] = install(owner, admitted, ["oteryn:npc.king_tibianus"]).await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING")
        .execute(&h.pool).await?;
    seed_bank(h).await?;
    for (command, word) in [(800, "hi"), (801, "outfit"), (802, "yes")] {
        talk(owner, admitted, king, command, word).await?;
    }
    for (index, (keyword, price, stage)) in [
        ("armor", 500_000_000, 1),
        ("boots", 250_000_000, 2),
        ("helmet", 250_000_000, 4),
    ]
    .into_iter()
    .enumerate()
    {
        let command = 803 + 2 * u64::try_from(index)?;
        let before = bank(h).await?;
        let price_quote = talk(owner, admitted, king, command, keyword).await?;
        assert!(price_quote.iter().any(|line| line.contains(&format!("{price} gold"))));
        talk(owner, admitted, king, command + 1, "yes").await?;
        assert_eq!(bank(h).await?, before - price);
        assert_eq!(progress(owner, admitted, authority, LEGACY_TRACK).await?, stage);
    }
    let amount: i64 = sqlx::query_scalar(
        "SELECT sum(bank_debit_gold_units)::bigint FROM game_item_fee_burns WHERE cause_kind=18",
    ).fetch_one(&h.pool).await?;
    assert_eq!(amount, 1_000_000_000);
    assert_eq!(bank(h).await?, 0);
    assert_eq!(money(h).await?, 0);
    Ok(())
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
    let [king] = super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.king_tibianus"]).await?;
    super::classic_quests_fixtures::talk(owner, admitted, king, 800, "hi").await?;
    super::classic_quests_fixtures::talk(owner, admitted, king, 801, "golden commission").await?;
    assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?, 0);
    let base = callbacks.iter().find(|c| c.key == "oteryn:quest-reward/questpc7-golden-base").ok_or("base donation")?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING")
        .execute(&h.pool).await?;
    let (unpaid, _, _) = super::qengine_rewards_fixtures::wire_reward(owner, admitted, king, base, false).await?;
    assert_eq!(unpaid, NpcIntentDisposition::Rejected, "the commission requires payment");
    assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?, 0);
    seed_bank(h).await?;
    for (index, (suffix, before, after, price)) in [
        ("base", 0, 1, 100_000_i64),
        ("gloves", 1, 2, 50_000),
        ("helmet", 2, 3, 50_000),
    ].into_iter().enumerate() {
        let callback = callbacks.iter().find(|c| c.key == format!("oteryn:quest-reward/questpc7-golden-{suffix}")).ok_or("callback")?;
        assert_eq!(money(h).await?, 0);
        assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?, before);
        let before_payment = bank(h).await?;
        let (result, _, last) = super::qengine_rewards_fixtures::wire_grant(owner, admitted, king, callback).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(bank(h).await?, before_payment - price, "each milestone debits exactly its own price");
        assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?, after);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE character_id=encode($1,'hex')::uuid AND cause_kind=18")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(receipts, i64::try_from(index + 1)?);
        let replay = super::quest_world_callbacks_fixtures::wire(owner, admitted, last, 7,
            oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&oteryn_protocol_oteryn::npc_service::NpcTalkIntent { npc_actor: king, text: "yes".into() }).map_err(debug)?).await?;
        assert_eq!(oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?, NpcIntentDisposition::Ok);
        assert_eq!(money(h).await?, 0);
        assert_eq!(bank(h).await?, before_payment - price, "replay cannot debit the bank twice");
    }
    super::classic_quests_fixtures::talk(owner, admitted, king, 899, "hi").await?;
    super::classic_quests_fixtures::talk(owner, admitted, king, 900, "complete commission").await?;
    assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?, 4);
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
    let result = super::quest_world_callbacks_fixtures::wire(owner, admitted, next, 25, change.encode().map_err(debug)?).await?;
    assert!(matches!(oteryn_protocol_oteryn::appearance::ChangeResult::decode(&result).map_err(debug)?, oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)));

    Ok(())
}
