//! Shared clock engine only; no requesting lane's quest story is qualified here.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Position, Trigger};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const PHASE: &str = "oteryn:quest-progress/world-clock/phase";
const CLOCK: &str = "oteryn:quest-progress/world-clock/harvested-at";
const CLAIM: &str = "oteryn:quest-callback/world-clock-harvest";
const SHORT: &str = "oteryn:quest-callback/world-clock-age-short";
const REST: &str = "oteryn:quest-callback/world-clock-age-rest";
// Preserve the requesting lane's literal 60*30*1000 SECONDS, not 30 minutes.
const COOLDOWN: i64 = 60 * 30 * 1000;
const AT: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};

#[test]
fn world_callback_clock_mint_cooldown_and_replay_dispatch_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-world-clock")
}

#[test]
fn world_callback_clock_full_bag_rolls_back_timestamp_dispatch_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-world-clock-full")
}

fn callback(
    key: &str,
    at: Position,
    step: bool,
    operation: serde_json::Value,
) -> TestResult<serde_json::Value> {
    let mut callback = super::authored_world_rewards::placeholder(AT)?;
    callback.key = key.into();
    callback.requires_world_admission = true;
    callback.storage[0].track = PHASE.into();
    callback.storage[0].set = 0;
    callback.storage[0].preserve = true;
    if step {
        callback.trigger = Trigger::StepIn;
        callback.selector.position = Some(at);
        callback.selector.unique = None;
        callback.selector.appearance = None;
    }
    let mut value = serde_json::to_value(callback)?;
    value["state_operations"] = serde_json::json!([operation]);
    Ok(value)
}

fn target(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<WorldObjectTarget> {
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("placement")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}

async fn clock(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2),-1)"
    ).bind(QUEST).bind(CLOCK).fetch_one(&h.pool).await?)
}

async fn use_(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
) -> TestResult<UseDisposition> {
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, id, 2, payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    direction: StepDirection,
) -> TestResult {
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        id,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    full: bool,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("WORLD_CALLBACK_CLOCK_POSTGRES {version}");
    let mut harvest = callback(
        CLAIM,
        AT,
        false,
        serde_json::json!({
            "track":CLOCK,"from":{"kind":"elapsed_at_least","seconds":COOLDOWN},"effect":{"kind":"set_now"}
        }),
    )?;
    harvest["effects"]["item_reward"] =
        serde_json::json!({"item":"oteryn:item.tibia.i3031","count":2,"contents":[]});
    let short = callback(
        SHORT,
        AT,
        true,
        serde_json::json!({"track":CLOCK,"from":{"kind":"any"},"effect":{"kind":"add","value":-1800}}),
    )?;
    let rest = callback(
        REST,
        Position { x: AT.x - 1, ..AT },
        true,
        serde_json::json!({"track":CLOCK,"from":{"kind":"any"},"effect":{"kind":"add","value":-(COOLDOWN-1800)}}),
    )?;
    // New fields are parsed only AFTER the real PostgreSQL harness was created.
    let document = serde_json::json!({"schema":1,"quests":[{"quest":QUEST,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["synthetic transaction-clock engine fixture; no production quest authoring"],
        "tracks":[{"key":PHASE,"initial":0,"minimum":0,"maximum":0},
                  {"key":CLOCK,"initial":-1,"minimum":-1,"maximum":i64::MAX}],
        "stages":[{"key":"harvest","completes":false,"trigger":{"kind":"map_use","callback":harvest}},
                  {"key":"age-short","completes":false,"trigger":{"kind":"step_in","callback":short}},
                  {"key":"age-rest","completes":false,"trigger":{"kind":"step_in","callback":rest}}]}]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let revision = owner
        .quest_catalogue
        .as_deref()
        .ok_or("catalogue")?
        .content_revision()
        .to_owned();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    if full {
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
            native,
            &crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)?;
        let capacity = i64::from(facts.container_capacity.ok_or("capacity")?);
        let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
            .fetch_one(&h.pool)
            .await?;
        assert!(occupied <= capacity);
        let mut ordinal = 9000;
        for _ in occupied..capacity {
            super::qengine_voucher_stories::mint(owner, admitted, 3031, 100, &mut ordinal).await?;
        }
        let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
        assert_eq!(use_(owner, admitted, 1).await?, UseDisposition::Rejected);
        assert_eq!(clock(h).await?, -1, "failed MINT cannot start the cooldown");
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, 3031).await?,
            before
        );
        return Ok(());
    }
    let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
    let before_time: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    for id in [1, 1] {
        assert_eq!(use_(owner, admitted, id).await?, UseDisposition::Committed);
    }
    let timestamp = clock(h).await?;
    let after_time: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    assert!(
        (before_time..=after_time).contains(&timestamp),
        "SetNow uses the database transaction's Unix seconds"
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 2
    );
    assert_eq!(use_(owner, admitted, 2).await?, UseDisposition::Rejected);
    // Age the clock through real, fenced STEP callbacks, never SQL edits or disabled guards.
    step(owner, admitted, 3, StepDirection::East).await?;
    assert_eq!(clock(h).await?, timestamp - 1800);
    assert_eq!(
        use_(owner, admitted, 4).await?,
        UseDisposition::Rejected,
        "the literal interval is not silently shortened to30minutes"
    );
    step(owner, admitted, 5, StepDirection::West).await?;
    assert_eq!(clock(h).await?, timestamp - COOLDOWN);
    for id in [6, 6] {
        assert_eq!(use_(owner, admitted, id).await?, UseDisposition::Committed);
    }
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 4
    );
    assert_eq!(use_(owner, admitted, 7).await?, UseDisposition::Rejected);
    let atomic: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.created_xact_id=m.created_xact_id WHERE q.transition_key=$1").bind(CLAIM).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 2,
        "each accepted period owns one atomic MINT/clock transaction"
    );
    Ok(())
}
