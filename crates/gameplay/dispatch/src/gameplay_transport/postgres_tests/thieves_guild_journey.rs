//! A single fresh-character journey through the chosen Thieves Guild recipe.

use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::{self as callbacks, wire};
use super::questpc8_trader::{prepare_item, say, teleport, trade_intent, visit};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTradeSide};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, encode_use_intent,
};

pub(super) const NAME: &str = "authored-thieves-guild";
const QUEST: &str = "oteryn:quest.the_thieves_guild_quest";
const STAGE: &str = "oteryn:quest-progress/chosen/the-thieves-guild/stage";
const CLAIM: &str = "oteryn:reward-claim.quest.u8_2.the_thieves_guild_quest.reward_end";
const DORIAN: &str = "oteryn:npc.dorian";
const OSWALD: &str = "oteryn:npc.oswald";
const HERBERT: &str = "oteryn:npc.herbert";
const CARINA: &str = "oteryn:npc.carina";
const CHANTALLE: &str = "oteryn:npc.chantalle";
const BERT: &str = "oteryn:npc.black_bert";

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

const NPCS: [(&str, Position); 6] = [
    (DORIAN, at(32318, 32209, -8)),
    (OSWALD, at(32381, 32220, -7)),
    (HERBERT, at(32332, 32839, -7)),
    (CARINA, at(33015, 32048, -6)),
    (CHANTALLE, at(32363, 32792, -7)),
    (BERT, at(32340, 32206, -6)),
];
const TUSK_LOOT: Position = at(32316, 32209, -8);
const VINE: Position = at(32337, 31814, -7);
const VASE: Position = at(32337, 31812, -6);
const GOBLET: Position = at(32551, 32652, -10);
const ROOM: Position = at(32359, 32787, -6);
const VINE_EXIT: Position = at(32337, 31814, -6);
const FISH: Position = at(32359, 32791, -6);
const MESSAGE: Position = at(32598, 32381, -10);
const DAGGER: Position = at(32309, 32211, -8);
const OTHER_REWARDS: [Position; 2] = [at(32309, 32209, -8), at(32309, 32213, -8)];

#[test]
fn thieves_guild_full_journey_earns_dagger_and_bert_trade() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        callbacks::scenario(NAME, at(32317, 32209, -8), false)
    })
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let chests = super::quest_catalogue_chests_fixtures::cases()?
        .into_iter()
        .filter(|case| (6271..=6277).contains(&case.unique))
        .collect::<Vec<_>>();
    assert_eq!(chests.len(), 7);
    let props = [
        (VINE, 7890, Default::default()),
        (VASE, 7892, Default::default()),
        (VINE_EXIT, 7891, Default::default()),
        (ROOM, 7941, Default::default()),
        (FISH, 7936, Default::default()),
        (MESSAGE, 1451, Default::default()),
    ];
    let mut cells: Vec<_> = NPCS
        .iter()
        .flat_map(|(_, p)| [*p, at(p.x - 1, p.y, p.floor)])
        .collect();
    cells.extend(props.iter().map(|(p, _, _)| at(p.x - 1, p.y, p.floor)));
    cells.extend([
        TUSK_LOOT,
        at(32336, 31813, -6),
        at(32337, 31815, -7),
        at(32359, 32786, -6),
        at(32359, 32788, -6),
    ]);
    cells.extend(
        [GOBLET, DAGGER, OTHER_REWARDS[0], OTHER_REWARDS[1]].map(|p| at(p.x - 1, p.y, p.floor)),
    );
    super::quest_catalogue_chests_fixtures::catalogue_source_worlds(&chests, &props, &cells, false)
}

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let model = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?
    .service_model_for_project()
    .map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Thieves Guild", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let npcs = NPCS
        .into_iter()
        .map(|(key, position)| NpcPlacements {
            key: key.into(),
            placements: vec![NpcPlacement {
                x: position.x,
                y: position.y,
                floor: position.floor,
                direction: MovementFacing::North,
            }],
        })
        .collect();
    owner
        .runtime
        .lock()
        .await
        .place_npcs(&NpcPlacementTable { npcs }, |_| true, true)
        .map_err(debug)?;
    Ok(())
}

async fn expect(h: &Harness, expected_stage: i64, items: &[(u32, i64)]) -> TestResult {
    let stage: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks
          WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3),0)",
    )
    .bind(id(41).as_slice())
    .bind(QUEST)
    .bind(STAGE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stage, expected_stage);
    for &(item, count) in items {
        assert_eq!(
            quantity(h, item).await?,
            count,
            "stage {stage}, item {item}"
        );
    }
    Ok(())
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(i.quantity),0)::bigint
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
         JOIN game_item_instances i ON i.item_instance_id=c.item
         WHERE i.definition_production_key=$3 AND i.lifecycle=1",
    )
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(format!("oteryn:item.tibia.i{item}"))
    .fetch_one(&h.pool)
    .await?)
}

async fn buy(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: &str,
    item: u32,
    expected: NpcIntentDisposition,
) -> TestResult {
    let entity = visit(owner, admitted, npc).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let intent = trade_intent(
        owner,
        entity,
        npc,
        &format!("oteryn:item.tibia.i{item}"),
        NpcTradeSide::Buy,
    )
    .await?;
    let first_id = admitted.continuity.next_command_id;
    let outcome = super::grizzly_npc_fixtures::wire::trade(
        owner, admitted, h, first_id, &intent, None, false,
    )
    .await?;
    assert_eq!(outcome, [expected], "{npc}, item {item}");
    Ok(())
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: &str,
    texts: &[&str],
) -> TestResult {
    let entity = visit(owner, admitted, npc).await?;
    let mut next = admitted.continuity.next_command_id;
    say(owner, admitted, entity, &mut next, texts).await
}

async fn use_payload(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
    tool: Option<u32>,
) -> TestResult<Vec<u8>> {
    teleport(
        owner,
        admitted,
        MovementLocalPosition {
            x: i32::from(at.x - 1),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        },
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1)
        .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let object = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    if let Some(item) = tool {
        let provider = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let definition = provider
            .item_index()
            .ok_or("items")?
            .definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{item}"),
                "definition-r1",
            )
            .ok_or("tool definition")?;
        Ok(encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(
                std::num::NonZeroU32::new(definition.get()).ok_or("tool index")?,
            ),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(object)),
            use_with_position: None,
        })
        .map_err(debug)?)
    } else {
        Ok(encode_use_intent(&object).map_err(debug)?)
    }
}

async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
    tool: Option<u32>,
) -> TestResult {
    let payload = use_payload(owner, admitted, at, tool).await?;
    owner.clock_origin -= std::time::Duration::from_secs(1);
    let result = wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed,
        "{at:?}"
    );
    Ok(())
}

async fn journal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    quest_index: u32,
    completed: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    let payload = encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let response = wire(owner, admitted, command, 22, payload).await?;
    let Some(QuestLogView::List(quests)) = decode_quest_log(&response).map_err(debug)?.view else {
        return Err("quest journal list".into());
    };
    let quest = quests
        .iter()
        .find(|q| q.quest_index == quest_index)
        .ok_or("Thieves Guild journal entry")?;
    assert_eq!(quest.completed, completed);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3, 4, 6, 11, 13, 15, 16, 18],
        )
        .ok_or("journey capabilities")?;
    install(owner, admitted).await?;
    assert!(owner.premium_gate(id(40)).await.premium_current());
    expect(h, 0, &[(3044, 0), (7404, 0)]).await?;
    talk(owner, admitted, DORIAN, &["join", "yes"]).await?;
    expect(h, 1, &[(3044, 0)]).await?;
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let log =
        oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?.content;
    let quest_index = u32::try_from(
        log.active()
            .quests()
            .iter()
            .position(|q| q.key == QUEST)
            .ok_or("native Thieves Guild journal")?,
    )?;
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(log)
            .ok_or("quest journal bounds")?,
    );
    journal(owner, admitted, quest_index, false).await?;
    talk(owner, admitted, DORIAN, &["mission"]).await?;
    expect(h, 2, &[(3044, 0)]).await?;

    // Supply outside-world loot and spending money through the ordinary item owner.
    super::questpc8_magic_sword_fixtures::raise_level(owner, admitted, 50, 200).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "loot fence")?
        .ok_or("loot fence")?;
    callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: TUSK_LOOT,
            item: "oteryn:item.tibia.i3044".into(),
            count: 10,
        },
        401,
        false,
    )
    .await?;
    expect(h, 2, &[(3044, 0)]).await?;
    super::annihilator_route_fixtures::step(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::West,
    )
    .await?;
    expect(h, 2, &[(3044, 0)]).await?;
    super::l6_world_pickups_fixtures::pickup_at(
        owner,
        admitted,
        TUSK_LOOT,
        "oteryn:item.tibia.i3044",
        10,
        false,
    )
    .await?;
    prepare_item(owner, admitted, 3035, 84, 402).await?;
    expect(h, 2, &[(3044, 10), (3035, 84)]).await?;
    buy(
        owner,
        admitted,
        h,
        BERT,
        130,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    expect(h, 2, &[(130, 0), (3035, 84)]).await?;
    talk(owner, admitted, DORIAN, &["mission", "yes"]).await?;
    expect(h, 3, &[(3044, 0)]).await?;
    talk(owner, admitted, DORIAN, &["mission"]).await?;
    expect(h, 4, &[(7889, 0), (227, 0)]).await?;
    buy(owner, admitted, h, DORIAN, 7889, NpcIntentDisposition::Ok).await?;
    expect(h, 4, &[(7889, 1), (227, 0)]).await?;
    use_at(owner, admitted, VINE, None).await?;
    expect(h, 4, &[(7889, 1), (227, 0)]).await?;
    for attempt in 0..24 {
        if attempt > 0 {
            buy(owner, admitted, h, DORIAN, 7889, NpcIntentDisposition::Ok).await?;
            expect(h, 4, &[(7889, 1), (227, 0)]).await?;
        }
        use_at(owner, admitted, VASE, Some(7889)).await?;
        if quantity(h, 227).await? == 1 {
            break;
        }
        expect(h, 4, &[(7889, 0), (227, 0)]).await?;
    }
    expect(h, 5, &[(7889, 1), (227, 1)]).await?;
    use_at(owner, admitted, VINE_EXIT, None).await?;
    expect(h, 5, &[(227, 1)]).await?;
    talk(owner, admitted, DORIAN, &["mission", "yes"]).await?;
    expect(h, 6, &[(227, 0)]).await?;
    talk(owner, admitted, DORIAN, &["mission"]).await?;
    expect(h, 7, &[(7933, 0)]).await?;
    talk(owner, admitted, OSWALD, &["invitation", "gold", "yes"]).await?;
    expect(h, 8, &[(7933, 1)]).await?;
    talk(owner, admitted, DORIAN, &["mission", "yes"]).await?;
    expect(h, 9, &[(7933, 0)]).await?;
    talk(owner, admitted, DORIAN, &["mission"]).await?;
    expect(h, 10, &[(7870, 0)]).await?;
    use_at(owner, admitted, GOBLET, None).await?;
    expect(h, 10, &[(7870, 1)]).await?;
    talk(owner, admitted, DORIAN, &["mission", "yes"]).await?;
    expect(h, 11, &[(7870, 0)]).await?;
    talk(owner, admitted, DORIAN, &["mission"]).await?;
    expect(h, 12, &[(7940, 0), (7934, 0)]).await?;
    talk(owner, admitted, HERBERT, &["letter", "gold", "yes"]).await?;
    expect(h, 13, &[(7940, 1)]).await?;
    talk(owner, admitted, CARINA, &["precious necklace", "yes"]).await?;
    expect(h, 14, &[(7940, 0), (7939, 1)]).await?;
    talk(owner, admitted, CHANTALLE, &["key"]).await?;
    expect(h, 15, &[(7939, 0), (7934, 1)]).await?;
    use_at(owner, admitted, ROOM, Some(7934)).await?;
    expect(h, 16, &[(7934, 0), (7936, 0)]).await?;
    use_at(owner, admitted, FISH, None).await?;
    expect(h, 17, &[(7936, 1)]).await?;
    use_at(owner, admitted, ROOM, None).await?;
    expect(h, 18, &[(7936, 1)]).await?;
    talk(owner, admitted, DORIAN, &["mission", "yes"]).await?;
    expect(h, 19, &[(7936, 0)]).await?;
    talk(owner, admitted, DORIAN, &["mission"]).await?;
    expect(h, 20, &[(7873, 1)]).await?;
    use_at(owner, admitted, MESSAGE, Some(7873)).await?;
    expect(h, 21, &[(7873, 0), (7404, 0)]).await?;
    talk(owner, admitted, DORIAN, &["mission", "yes"]).await?;
    expect(h, 22, &[(7404, 0)]).await?;
    journal(owner, admitted, quest_index, true).await?;
    buy(owner, admitted, h, BERT, 130, NpcIntentDisposition::Ok).await?;
    expect(h, 22, &[(7404, 0), (130, 1)]).await?;

    let payload = use_payload(owner, admitted, DAGGER, None).await?;
    let command = admitted.continuity.next_command_id;
    let result = wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    expect(h, 22, &[(7404, 1), (8021, 0), (8073, 0)]).await?;

    // Each wire call ends at EOF and resumes from the retained admitted continuation.
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    assert_eq!(wire(owner, admitted, command, 2, payload).await?, result);
    for at in [DAGGER, OTHER_REWARDS[0], OTHER_REWARDS[1]] {
        let payload = use_payload(owner, admitted, at, None).await?;
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let refused = wire(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            2,
            payload,
        )
        .await?;
        assert_eq!(
            decode_use_result(&refused).map_err(debug)?,
            UseDisposition::NothingToUse,
            "{at:?}"
        );
        expect(h, 22, &[(7404, 1), (8021, 0), (8073, 0)]).await?;
    }
    let fees: Vec<i64> = sqlx::query_scalar(
        "SELECT fee_gold_units FROM game_quest_reward_fees ORDER BY fee_gold_units",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(fees, [1000, 1000, 5000]);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    let claims: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(CLAIM)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(claims, 1);
    Ok(())
}
