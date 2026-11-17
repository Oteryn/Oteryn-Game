//! Physical progress and repeatable exact floor inputs through the admitted PostgreSQL owner.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::qengine_rewards::scenario(name)
}
#[test]
fn l6_world_admission_opening_only_charges_exact_inputs_and_closes_free() -> TestResult {
    scenario("qengine-rewards-questpc4-admission-opening")
}
#[test]
fn l6_world_admission_opening_fifth_food_pair_closes_free() -> TestResult {
    scenario("qengine-rewards-questpc4-admission-opening-five")
}
#[test]
fn l6_world_admission_physical_checkpoint_rejects_generic_writer() -> TestResult {
    scenario("qengine-rewards-questpc4-admission-physical")
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let suffix = if name.ends_with("-physical") {
        "/paradox-floor-four-arrived"
    } else if name.ends_with("-five") {
        "/paradox-food-five"
    } else {
        "/paradox-food-four"
    };
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with(suffix))
        .ok_or_else(|| "admitted world consumer".into())
}
pub(super) fn trigger(name: &str) -> Position {
    if name.ends_with("-physical") {
        Position {
            x: 32478,
            y: 31905,
            floor: -4,
        }
    } else if name.ends_with("-five") {
        Position {
            x: 32478,
            y: 31904,
            floor: -3,
        }
    } else {
        Position {
            x: 32479,
            y: 31905,
            floor: -4,
        }
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY {name} version={version}");
    let callback = callback(name)?;
    let before = state(h).await?;
    // The old generic entry point must never write this source-owned progress, even without a gift.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            authority,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9000).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
        ),
        "physical callback used weaker API: {result:?}"
    );
    drop(slot);
    assert_eq!(state(h).await?, before);
    if name.ends_with("-physical") {
        native_checkpoint_facts()?;
        use oteryn_protocol_oteryn::world_spatial::*;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(progress(h, &callback.storage[0].track).await?, 1);
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(&callback.key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipt_count, 1);
        return Ok(());
    }
    let pos = trigger(name);
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, 1)
        .ok_or("placement")?;
    assert_eq!(
        use_source(owner, admitted, pos, 0).await?,
        UseDisposition::Rejected
    );
    assert_eq!(state(h).await?, before, "missing inputs write nothing");
    // Populate all but one exact input and place the wrong definition at the last position.
    for (index, input) in callback
        .offerings
        .iter()
        .take(callback.offerings.len() - 1)
        .enumerate()
    {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            input,
            10 + u64::try_from(index)?,
            false,
        )
        .await?;
    }
    let mut wrong = callback.offerings.last().ok_or("last input")?.clone();
    wrong.item = callback.offerings[0].item.clone();
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &wrong, 20, false).await?;
    let mismatch = state(h).await?;
    assert_eq!(
        use_source(owner, admitted, pos, 0).await?,
        UseDisposition::Rejected
    );
    assert_eq!(
        state(h).await?,
        mismatch,
        "one wrong definition cannot partially consume the other inputs"
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0
    );
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        callback.offerings.last().ok_or("last")?,
        21,
        false,
    )
    .await?;
    assert_eq!(
        use_source(owner, admitted, pos, 0).await?,
        UseDisposition::Committed
    );
    let amount = i64::try_from(callback.offerings.len())?;
    assert_eq!(consumed(h, &callback.key).await?, amount);
    assert_eq!(progress(h, &callback.storage[0].track).await?, 0);
    let ladder = callback
        .effects
        .item_presence
        .as_ref()
        .ok_or("owned ladder")?
        .position;
    let tile = crate::map::overlay::TilePos {
        x: ladder.x,
        y: ladder.y,
        floor: ladder.floor,
    };
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_items(tile)
            .len(),
        1
    );
    let transactions:i64=sqlx::query_scalar("SELECT count(DISTINCT transaction) FROM (SELECT xmin::text AS transaction FROM game_quest_callback_consumptions WHERE transition_key=$1 UNION ALL SELECT xmin::text FROM game_character_quest_receipts WHERE transition_key=$1) AS rows").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(
        transactions, 1,
        "all exact debits and progress share one transaction"
    );
    let open = state(h).await?;
    assert_eq!(
        use_source(owner, admitted, pos, 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        state(h).await?,
        open,
        "closing writes/debits/grants nothing durable"
    );
    assert!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_items(tile)
            .is_empty()
    );
    assert_eq!(
        use_source(owner, admitted, pos, 2).await?,
        UseDisposition::Rejected
    );
    assert_eq!(
        state(h).await?,
        open,
        "reopening needs all exact new inputs"
    );
    for (index, input) in callback.offerings.iter().enumerate() {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            input,
            30 + u64::try_from(index)?,
            false,
        )
        .await?;
    }
    assert_eq!(
        use_source(owner, admitted, pos, 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(consumed(h, &callback.key).await?, amount * 2);
    assert_eq!(progress(h, &callback.storage[0].track).await?, 1);
    Ok(())
}

async fn state(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(r)) FROM game_character_roots r),'states',(SELECT jsonb_agg(to_jsonb(s) ORDER BY quest_key) FROM game_character_quest_states s),'tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT count(*) FROM game_character_quest_receipts),'consumes',(SELECT count(*) FROM game_quest_callback_consumptions),'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i))").fetch_one(&h.pool).await?)
}
async fn progress(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}
async fn consumed(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1").bind(key).fetch_one(&h.pool).await?)
}
pub(super) async fn use_source(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pos: Position,
    revision: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(
        &oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, 1)
            .ok_or("key")?
            .to_be_bytes(),
    );
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    decode_use_result(&reply).map_err(debug)
}

fn native_checkpoint_facts() -> TestResult {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z04-x126-y124.b3"
    ));
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        bytes,
        oteryn_world_bundle::bundle::TILE_LIMITS,
        &mut oteryn_world_bundle::sector::Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/world/placements/index.json"),
    )?)?;
    let tile = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .find(|tile| tile.x == 32478 && tile.y == 31905)
        .ok_or("native fourth-floor arrival tile")?;
    let first = tile.items.first().ok_or("native arrival ground")?;
    assert_eq!(
        index["palette"][first.palette as usize]["source_item_id"].as_u64(),
        Some(409)
    );
    assert_eq!(tile.house, 0);
    Ok(())
}
