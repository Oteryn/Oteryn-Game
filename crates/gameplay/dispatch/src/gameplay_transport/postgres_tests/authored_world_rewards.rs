//! Synthetic engine fixture: no acquisition story or packet quest is qualified here.
//! The retained Behemoth callback supplies valid template provenance only.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Callback, Position, Selector, Storage, Trigger,
};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const PHASE: &str = "oteryn:quest-progress/authored-world-reward/phase";
const STEP: &str = "oteryn:quest-callback/authored-engine-world-reward";
const AT: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};

#[test]
fn authored_world_use_item_delivery_is_atomic_and_replay_bound_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-world-rewards")
}

#[test]
fn authored_world_full_backpack_rolls_back_reward_and_quest_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-world-rewards-full")
}

#[test]
fn authored_native_boot_recipe_admission_dispatch_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-native-admission")
}

/// This runs before the PG harness: it deliberately uses only already-supported fields.
pub(super) fn placeholder(at: Position) -> TestResult<Callback> {
    let mut callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("behemoth"))
        .ok_or("world template")?;
    callback.key = STEP.into();
    callback.quest = QUEST.into();
    callback.trigger = Trigger::Use;
    callback.selector = Selector {
        ordinal: None,
        native: None,
        position: Some(at),
        unique: Some(63001),
        action: None,
        appearance: Some(100),
        position_aliases: Vec::new(),
    };
    callback.lever = None;
    callback.reversible = false;
    callback.passages.clear();
    callback.teleport = None;
    callback.storage = vec![Storage {
        read_only: false,
        at_least: false,
        increment: false,
        track: PHASE.into(),
        minimum: 0,
        maximum: 0,
        set: 1,
        preserve: false,
        set_if: None,
        add: None,
    }];
    callback.minimum_level = None;
    callback.world_change = None;
    callback.experience = None;
    callback.monk_reward = None;
    callback.offerings.clear();
    callback.item_on_target = None;
    callback.return_on_failure = false;
    callback.failure_message = None;
    callback.success_message = None;
    callback.native_rule = None;
    callback.effects = Default::default();
    callback
        .sources
        .push("synthetic engine fixture; template metadata only".into());
    Ok(callback)
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    revision: u64,
) -> TestResult<WorldObjectTarget> {
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
        expected_revision: revision,
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    full: bool,
) -> TestResult {
    exercise_mode(owner, admitted, h, full, false).await
}

pub(super) async fn exercise_native_admission(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    exercise_mode(owner, admitted, h, false, true).await
}

async fn exercise_mode(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    full: bool,
    native_admission: bool,
) -> TestResult {
    // Parse the NEW reward field after Harness creation, so RED proves actual PG execution.
    let mut callback = serde_json::to_value(placeholder(AT)?)?;
    callback["effects"]["item_reward"] =
        serde_json::json!({"item":"oteryn:item.tibia.i3031","count":2,"contents":[]});
    let document = serde_json::json!({"schema":1,"quests":[{"quest":QUEST,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["synthetic engine fixture; no production quest authoring"],
        "tracks":[{"key":PHASE,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"claim","completes":true,"trigger":{"kind":"map_use","callback":callback}}]}]});
    let mut bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let revision = owner
        .quest_catalogue
        .as_deref()
        .ok_or("native catalogue")?
        .content_revision()
        .to_owned();
    let catalogue = if native_admission {
        let native = owner.quest_catalogue.as_deref().ok_or("native catalogue")?;
        let Err(error) = bindings.attach(native) else {
            return Err("native boot accepted a stale authored recipe digest".into());
        };
        assert!(
            error.contains("recipe digest differs"),
            "unexpected refusal: {error}"
        );
        // Exact canonical chosen recipe, not a production quest qualification.
        bindings.quests[0].recipe_sha256 =
            "c6bb54b0c423a165891b0ca32fb305a0963d1a06417249b0b317ec34cf65e708".into();
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state_with_bindings(
            &revision, &bindings,
        )
        .map_err(debug)?
        .catalogue()
        .clone()
    } else {
        let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
        bindings.attach(&base)?
    };
    owner.quest_catalogue = Some(Arc::new(catalogue));
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
        assert!(
            occupied <= capacity,
            "fixture begins within native backpack capacity"
        );
        let mut ordinal = 4000;
        for _ in occupied..capacity {
            super::qengine_voucher_stories::mint(owner, admitted, 3031, 100, &mut ordinal).await?;
        }
        let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            occupied, capacity,
            "all native backpack slots must be occupied"
        );
        let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&target(owner, 0)?)
            .map_err(debug)?;
        let result = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Rejected
        );
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, 3031).await?,
            before,
            "full backpack never grants partial rewards"
        );
        let phase:i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2),0)").bind(QUEST).bind(PHASE).fetch_one(&h.pool).await?;
        assert_eq!(phase, 0, "MINT refusal rolls back the quest write");
        let mints: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
        )
        .bind(STEP)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(mints, 0, "refused gift has no committed MINT receipt");
        let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
            .ok_or("placement")?;
        assert_eq!(
            owner.bundle_map().ok_or("map")?.facts.object_revision(key),
            0,
            "refusal preserves source revision"
        );
        return Ok(());
    }
    let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&target(owner, 0)?)
        .map_err(debug)?;
    for _ in 0..2 {
        let result =
            super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, 3031).await?,
            before + 2,
            "replay never duplicates world MINT"
        );
    }
    let phase: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(QUEST)
    .bind(PHASE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(phase, 1, "world state and reward commit together");
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("placement")?;
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0,
        "per-character gift preserves the shared source object revision"
    );
    let mints: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(STEP)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(mints, 1, "exactly one MINT receipt owns the gift");
    let atomic: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.created_xact_id=m.created_xact_id WHERE m.claim_production_key=$1 AND q.transition_key=$1 AND q.cause_kind='use' AND q.completes")
        .bind(STEP).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 1,
        "reward MINT and completed USE share one transaction"
    );
    let result = super::quest_world_callbacks::wire(
        owner,
        admitted,
        2,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target(owner, 0)?)
            .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Rejected,
        "completed stage refuses a fresh claim"
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 2
    );
    Ok(())
}
