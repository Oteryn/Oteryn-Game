//! Both exclusive factions run from enlistment through the real lamp callbacks.
use super::questl2_journeys::{quantity, use_at};
use super::*;
use oteryn_protocol_oteryn::npc_service::NpcTradeSide;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const ROOT: &str = "oteryn:quest-progress/quest/u7_4/djinn_war/";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let efreet = name.ends_with("efreet");
    let faction = if efreet { "efreet" } else { "marid" };
    let other = if efreet { "marid" } else { "efreet" };
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
            .ok_or("Djinn capabilities")?;
    let keys = [
        "baa_leal",
        "partos",
        "alesar",
        "yaman",
        "bo_ques",
        "rata_mari",
        "fa_hradin",
        "gabel",
        "malor",
        "the_orc_king",
        "haroun",
        "nah_bob",
    ];
    let refs = keys.map(|k| format!("oteryn:npc.{k}"));
    let npcs = super::super::questl2_fixture::install_at(
        owner,
        admitted,
        refs.each_ref().map(String::as_str),
        // Every actor owns a distinct, reachable cell; leave callback cells clear.
        [
            (33291, 31675), (33292, 31675), (33293, 31675),
            (33294, 31675), (33295, 31675), (33291, 31676),
            (33292, 31676), (33293, 31676), (33294, 31676),
            (33295, 31676), (33291, 31677), (33295, 31677),
        ].map(|(x, y)| Position { x, y, floor: -15 }),
    )
    .await?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let callback = |faction: &str, suffix: &str| -> TestResult<&RewardCallback> {
        callbacks
            .iter()
            .find(|c| c.key.ends_with(&format!("/l2-{faction}-{suffix}")))
            .ok_or_else(|| format!("missing Djinn callback {faction}-{suffix}").into())
    };
    let actor = |callback: &RewardCallback| -> TestResult<EntityRef> {
        keys.iter()
            .position(|k| callback.npc == format!("oteryn:npc.{k}"))
            .map(|n| npcs[n])
            .ok_or_else(|| "Djinn NPC actor".into())
    };
    macro_rules! act {
        ($side:expr, $suffix:expr, $expected:expr) => {{
            let c = callback($side, $suffix)?;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            assert_eq!(
                wire_reward(owner, admitted, actor(c)?, c, false).await?.0,
                $expected,
                "{side}-{suffix}",
                side = $side,
                suffix = $suffix
            );
        }};
    }
    let merchant = if efreet { npcs[3] } else { npcs[10] };
    let merchant_key = if efreet {
        "oteryn:npc.yaman"
    } else {
        "oteryn:npc.haroun"
    };
    let merchandise = if efreet { 3085 } else { 3092 };
    let seller = if efreet { npcs[2] } else { npcs[11] };
    let seller_key = if efreet {
        "oteryn:npc.alesar"
    } else {
        "oteryn:npc.nah_bob"
    };
    let sold_item = if efreet { 3369 } else { 3385 };
    // Supply ordinary money and merchandise before testing the permission gate.
    supply(owner, admitted, 3043, 19020).await?;
    supply(owner, admitted, sold_item, 19021).await?;
    shop(
        owner,
        admitted,
        seller,
        seller_key,
        0,
        (sold_item, NpcTradeSide::Sell),
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, sold_item).await?, 1);
    shop(
        owner,
        admitted,
        merchant,
        merchant_key,
        0,
        (merchandise, NpcTradeSide::Buy),
        NpcIntentDisposition::Rejected,
    )
    .await?;
    act!(faction, "finish", NpcIntentDisposition::Rejected);
    act!(faction, "enlist", NpcIntentDisposition::Ok);
    act!(other, "enlist", NpcIntentDisposition::Rejected);
    assert_eq!(track(h, other, "start").await?, 0, "exclusive faction");
    assert_eq!(track(h, faction, "start").await?, 1);
    if efreet {
        act!(faction, "supplies", NpcIntentDisposition::Ok);
        act!(faction, "thief-report", NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 3035).await?, 6, "600 gold source payment");
        act!(faction, "tear-mission", NpcIntentDisposition::Ok);
        act!(faction, "tear-return", NpcIntentDisposition::Rejected);
        use_at(
            owner,
            admitted,
            Position {
                x: 33294,
                y: 31678,
                floor: -15,
            },
            UseDisposition::Committed,
        )
        .await?;
        assert_eq!(quantity(h, 3233).await?, 1);
        act!(faction, "tear-return", NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 3233).await?, 0);
    } else {
        act!(faction, "cookbook", NpcIntentDisposition::Rejected);
        supply(owner, admitted, 3234, 19000).await?;
        act!(faction, "cookbook", NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 3234).await?, 0);
        assert_eq!(quantity(h, 3029).await?, 3);
        act!(faction, "spy-mission", NpcIntentDisposition::Ok);
        act!(faction, "cheese-request", NpcIntentDisposition::Ok);
        act!(faction, "spy-report-issue", NpcIntentDisposition::Rejected);
        supply(owner, admitted, 3607, 19001).await?;
        act!(faction, "spy-report-issue", NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 3607).await?, 0);
        assert_eq!(quantity(h, 3232).await?, 1);
        act!(faction, "spy-report", NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 3232).await?, 0);
    }
    act!(faction, "lamp-mission", NpcIntentDisposition::Ok);
    let right = Position {
        x: 33292,
        y: if efreet { 31677 } else { 31678 },
        floor: -15,
    };
    let wrong = Position {
        y: if efreet { 31678 } else { 31677 },
        ..right
    };
    use_at(owner, admitted, right, UseDisposition::Rejected).await?;
    act!(faction, "lamp-issue", NpcIntentDisposition::Ok);
    assert_eq!(quantity(h, 3231).await?, 1);
    act!(faction, "lamp-issue", NpcIntentDisposition::Rejected);
    assert_eq!(quantity(h, 3231).await?, 1, "lamp issued exactly once");
    use_at(owner, admitted, wrong, UseDisposition::Rejected).await?;
    assert_eq!(quantity(h, 3231).await?, 1, "wrong faction preserves lamp");
    use_at(owner, admitted, right, UseDisposition::Committed).await?;
    assert_eq!(quantity(h, 3231).await?, 0);
    assert_eq!(quantity(h, 3243).await?, 1);
    assert_eq!(track(h, faction, "mission03").await?, 2);
    act!(faction, "finish", NpcIntentDisposition::Ok);
    let achievement = format!("oteryn:achievement/{faction}_ally");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1"
        )
        .bind(&achievement)
        .fetch_one(&h.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1"
        )
        .bind(format!("oteryn:achievement/{other}_ally"))
        .fetch_one(&h.pool)
        .await?,
        0
    );

    assert_eq!(track(h, faction, "mission03").await?, 3);
    act!(faction, "finish", NpcIntentDisposition::Rejected);
    assert_eq!(
        quantity(h, 3243).await?,
        1,
        "sleeping lamp preserved after completion"
    );
    let stage = track(h, faction, "mission03").await?;
    shop(
        owner,
        admitted,
        merchant,
        merchant_key,
        stage,
        (merchandise, NpcTradeSide::Buy),
        NpcIntentDisposition::Ok,
    )
    .await?;
    assert_eq!(
        quantity(h, merchandise).await?,
        1,
        "real entitled shop purchase"
    );
    if efreet {
        let charged: (Option<i32>, i64) = sqlx::query_as(
            "SELECT full_charges,item_units::bigint FROM game_npc_trades WHERE full_charges IS NOT NULL")
            .fetch_one(&h.pool).await?;
        assert_eq!(charged, (Some(200), 1), "one full-charge source necklace");
    }
    shop(
        owner,
        admitted,
        seller,
        seller_key,
        stage,
        (sold_item, NpcTradeSide::Sell),
        NpcIntentDisposition::Ok,
    )
    .await?;
    assert_eq!(
        quantity(h, sold_item).await?,
        0,
        "second faction merchant buys exactly once"
    );
    Ok(())
}

async fn track(h: &Harness, faction: &str, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
    )
    .bind(format!("{ROOT}{faction}_faction/{suffix}"))
    .fetch_optional(&h.pool)
    .await?
    .unwrap_or(0))
}

async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    id: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "Djinn input fence")?
        .ok_or("Djinn input fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        id,
        true,
    )
    .await
}

async fn shop(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    npc_key: &str,
    stage: i64,
    selection: (u32, NpcTradeSide),
    expected: NpcIntentDisposition,
) -> TestResult {
    use crate::content::npc_catalogue::quest_shops;
    use oteryn_protocol_oteryn::npc_service::{NpcTradeIntent, encode_npc_trade_intent};
    let (item, side) = selection;
    // The preceding mission conversation used several talk intents. Respect the
    // native four-intents-per-second limit before this greeting and trade.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let book = owner.states.chat.npc_talk().ok_or("shop book")?;
    let entry = book.get(npc_key).ok_or("Djinn shop")?;
    let shops = quest_shops::shops().map_err(debug)?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("shop native")?;
    let visible: Vec<_> = entry
        .offers()
        .iter()
        .filter(|offer| {
            quest_shops::required(
                shops,
                npc_key,
                &offer.item,
                offer.direction,
                offer.unit_price,
                offer.count.unwrap_or(1),
            )
            .is_ok_and(|required| {
                required.is_none_or(|r| quest_shops::eligible(r, |_| Some(stage)))
            }) && crate::gameplay_transport::item_ref_admission::generation_item_facts(
                native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: offer.item.key.clone(),
                    revision_ref: offer.item.revision.clone(),
                },
            )
            .is_ok()
        })
        .cloned()
        .collect();
    let runtime = owner.runtime.lock().await;
    let index = crate::gameplay_transport::item_ref_admission::pinned_index(
        &runtime,
        owner.active_generation,
    )
    .ok_or("shop index")?;
    let target = format!("oteryn:item.tibia.i{item}");
    let shown = oteryn_gameplay_npc::npc_command::projected(&visible, index)
        .enumerate()
        .find(|(_, (_, source))| source.item.key == target);
    // Before completion, send the same valid offer and price an ungated shop
    // would expose. A wrong price must not be the reason access is refused.
    let ungated: Vec<_> = entry
        .offers()
        .iter()
        .filter(|offer| {
            crate::gameplay_transport::item_ref_admission::generation_item_facts(
                native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: offer.item.key.clone(),
                    revision_ref: offer.item.revision.clone(),
                },
            )
            .is_ok()
        })
        .cloned()
        .collect();
    let source_offer = oteryn_gameplay_npc::npc_command::projected(&ungated, index)
        .enumerate()
        .find(|(_, (_, source))| source.item.key == target)
        .map(|(n, (offer, _))| (n, offer.unit_price))
        .ok_or("Djinn source offer must be materializable")?;
    let (offer_index, price) = shown
        .map(|(n, (offer, _))| (n, offer.unit_price))
        .unwrap_or(source_offer);
    drop(runtime);
    let first = admitted.continuity.next_command_id;
    let mut commands = Vec::new();
    for (offset, text) in ["hi", "trade"].into_iter().enumerate() {
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        })
        .map_err(debug)?;
        commands.push((first + u64::try_from(offset)?, 7, payload));
    }
    let request = NpcTradeIntent {
        npc_actor: npc,
        catalogue_revision: book.catalogue_revision(),
        offer_index: u32::try_from(offer_index)?,
        side,
        quantity: 1,
        expected_unit_price: price,
    };
    let payload = encode_npc_trade_intent(&request).map_err(debug)?;
    let id = first + 2;
    commands.push((id, 8, payload.clone()));
    commands.push((id, 8, payload));
    let results =
        super::super::questl2_fixture::wire_sequence(owner, admitted, commands).await?;
    assert_eq!(results.len(), 4);
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&results[2])
            .map_err(debug)?,
        expected
    );
    assert_eq!(results[3], results[2], "literal shop replay");
    Ok(())
}
