use super::{TestResult, party_membership_fixtures::HuntCase};
use crate::gameplay_transport::{
    ComposedFreshAdmission, NpcTalkBook,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, test_analyser_route},
    party_command::SessionPartyView,
};
use oteryn_protocol_oteryn::{
    ClientCommandView,
    analyser::{AnalyserFact, AnalyserItemCount},
    analyser_catalogue as catalogue, analyser_party as wire,
};
use std::{num::NonZeroU16, sync::Arc};

async fn request(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    command_type: u32,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let command_id = player.continuity.next_command_id;
    player.continuity.next_command_id += 1;
    test_analyser_route(
        owner,
        player,
        ClientCommandView {
            command_id,
            command_type,
            payload,
        },
        &mut SessionPartyView::default(),
    )
    .await
}

pub(super) async fn check(
    h: &crate::bestiary_postgres_harness::Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &mut [AdmittedSession],
    case: HuntCase,
) -> TestResult {
    let index = owner
        .active_generation
        .ok_or("generation")?
        .native_gameplay()
        .ok_or("native gameplay")?
        .item_index()
        .ok_or("index")?;
    for player in players.iter_mut() {
        player.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[10, 42, 597, 598, 599],
        )
        .ok_or("analyser selection")?;
        owner
            .states
            .analyser
            .snapshot(
                player.runtime_actor.ok_or("player actor")?,
                player.game_session_id,
                player.continuity.connection_generation,
                Some(Arc::clone(index)),
            )
            .ok_or("analyser snapshot")?;
    }
    let key = "oteryn:item.tibia.i3415";
    let item = index
        .definition_ref("Item", key, "definition-r1")
        .ok_or("item")?;
    let piece = index
        .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
        .ok_or("currency")?;
    if matches!(case, HuntCase::NpcPrices) {
        let book = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(super::debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(
                &book.service_model_for_project().map_err(super::debug)?,
            )));
        let payload = catalogue::encode_request(&[item.get(), piece.get()])?;
        let result = request(
            owner,
            &mut players[0],
            catalogue::COMMAND_TYPE_ANALYSER_NPC_PRICES,
            &payload,
        )
        .await
        .ok_or("NPC mean reply")?;
        // The admitted buy quotes are 150 and 180.
        assert_eq!(
            catalogue::decode_npc_prices(&result)?,
            [(item.get(), Some(165)), (piece.get(), None)]
        );
        return Ok(());
    }

    for player in players.iter() {
        let actor = player.runtime_actor.ok_or("actor")?;
        owner.states.analyser.push(
            actor,
            player.game_session_id,
            AnalyserFact::Kill { race: None },
        );
        owner.states.analyser.push(
            actor,
            player.game_session_id,
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![AnalyserItemCount {
                    item_type: item,
                    count: NonZeroU16::new(2).ok_or("count")?,
                }],
                gold: 11,
            },
        );
        owner.states.analyser.push(
            actor,
            player.game_session_id,
            AnalyserFact::SupplyUsed(AnalyserItemCount {
                item_type: item,
                count: NonZeroU16::MIN,
            }),
        );
    }
    let initial = request(
        owner,
        &mut players[1],
        wire::COMMAND_TYPE_ANALYSER_PARTY_INTERVAL,
        &[],
    )
    .await
    .ok_or("initial interval")?;
    let initial = wire::HuntInterval::decode(&initial)?;
    assert_eq!(initial.number, 0);
    assert!(
        initial
            .members
            .iter()
            .all(|row| row.available && row.kills == 1)
    );
    if matches!(case, HuntCase::Reset) {
        assert!(
            request(
                owner,
                &mut players[1],
                wire::COMMAND_TYPE_ANALYSER_PARTY_RESET,
                &[]
            )
            .await
            .is_none()
        );
        let reply = request(
            owner,
            &mut players[0],
            wire::COMMAND_TYPE_ANALYSER_PARTY_RESET,
            &[],
        )
        .await
        .ok_or("leader reset")?;
        let reset = wire::HuntInterval::decode(&reply)?;
        assert_eq!(reset.number, 1);
        assert!(
            reset.members.iter().all(|row| row.available
                && !row.incomplete
                && row.kills == 0
                && row.loot_value == 0)
        );
        let observer = players[1];
        owner.states.analyser.push(
            observer.runtime_actor.ok_or("observer")?,
            observer.game_session_id,
            AnalyserFact::Kill { race: None },
        );
        let reply = request(
            owner,
            &mut players[1],
            wire::COMMAND_TYPE_ANALYSER_PARTY_INTERVAL,
            &[],
        )
        .await
        .ok_or("member interval")?;
        let current = wire::HuntInterval::decode(&reply)?;
        assert_eq!(current.number, reset.number);
        assert_eq!(current.members.iter().map(|row| row.kills).sum::<u64>(), 1);
        return Ok(());
    }

    let mut leader = vec![1];
    leader.extend(wire::encode_request(&[wire::Price {
        item: item.get(),
        loot: Some(20),
        supply: Some(30),
    }])?);
    assert!(
        request(
            owner,
            &mut players[1],
            wire::COMMAND_TYPE_ANALYSER_PARTY_VALUATION,
            &leader
        )
        .await
        .is_none()
    );
    let reply = request(
        owner,
        &mut players[0],
        wire::COMMAND_TYPE_ANALYSER_PARTY_VALUATION,
        &leader,
    )
    .await
    .ok_or("leader valuation")?;
    let valuation = wire::HuntInterval::decode(&reply)?;
    assert_eq!(valuation.source, wire::ValuationSource::Leader);
    assert_eq!(valuation.number, initial.number);
    assert!(
        valuation
            .members
            .iter()
            .all(|row| row.loot_value == 51 && row.supply_cost == 30)
    );

    // The existing market owner stores execution observations separately from current asking prices.
    // Seed two observations for one actual settlement to exercise definition and World projection.
    let world = players[0].world_id;
    let ids = [
        crate::bestiary_postgres_harness::id(60),
        crate::bestiary_postgres_harness::id(63),
        crate::bestiary_postgres_harness::id(90),
        crate::bestiary_postgres_harness::id(91),
    ];
    sqlx::query("INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,created_at_us,expires_at_us,state,escrow_gold,held_credit_gold)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'definition-r1',1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,90,2,2,1,2592000000001,1,180,0)")
        .bind(ids[2].as_slice()).bind(world.as_bytes().as_slice()).bind(key).bind(ids[1].as_slice()).bind(ids[0].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_market_settlements(transaction_id,offer_id,world_id,actor,account_id,kind,fee,escrow_delta,moved_units,occurred_at_us)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,2,0,0,1,floor(extract(epoch FROM clock_timestamp())*1000000)::bigint)")
        .bind(ids[3].as_slice()).bind(ids[2].as_slice()).bind(world.as_bytes().as_slice()).bind(ids[0].as_slice()).bind(ids[1].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_market_history(transaction_id,character_id,offer_id,amount,price,occurred_at_us)
        SELECT transaction_id,actor,offer_id,1,40,occurred_at_us FROM game_market_settlements WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(ids[3].as_slice()).execute(&h.pool).await?;
    let reply = request(
        owner,
        &mut players[0],
        wire::COMMAND_TYPE_ANALYSER_PARTY_VALUATION,
        &[0],
    )
    .await
    .ok_or("market valuation")?;
    let market = wire::HuntInterval::decode(&reply)?;
    assert_eq!(market.source, wire::ValuationSource::Market);
    assert_eq!(market.number, initial.number);
    assert!(
        market
            .members
            .iter()
            .all(|row| row.loot_value == 91 && row.supply_cost == 40)
    );
    let read = request(
        owner,
        &mut players[1],
        wire::COMMAND_TYPE_ANALYSER_PARTY_INTERVAL,
        &[],
    )
    .await
    .ok_or("member valuation")?;
    assert_eq!(
        wire::HuntInterval::decode(&read)?.source,
        wire::ValuationSource::Market
    );
    Ok(())
}
