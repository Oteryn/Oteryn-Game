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

#[test]
fn ddepths_disjoint_achievement_use_routes_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::qengine_rewards::scenario("qengine-rewards-authored-achievement-routes")
}

const WARZONE_PORTAL_CASES: [&str; 3] = [
    "qengine-rewards-ddepths-portal-baron",
    "qengine-rewards-ddepths-portal-count",
    "qengine-rewards-ddepths-portal-duke",
];

pub(super) fn warzone_portal(name: &str) -> Option<(Position, &'static str)> {
    let (x, y, boss) = match name {
        "qengine-rewards-ddepths-portal-baron" => (33460, 32267, "oteryn:boss.the_baron_from_below"),
        "qengine-rewards-ddepths-portal-count" => (33324, 32109, "oteryn:boss.the_count_of_the_core"),
        "qengine-rewards-ddepths-portal-duke" => (33275, 32316, "oteryn:boss.the_duke_of_the_depths"),
        _ => return None,
    };
    Some((Position { x, y, floor: -15 }, boss))
}

#[test]
fn ddepths_warzone_portal_paced_step_checks_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    for name in WARZONE_PORTAL_CASES {
        super::qengine_rewards::scenario(name)?;
    }
    Ok(())
}

pub(super) async fn exercise_warzone_portal_cooldown(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    portal: Position,
    boss: &str,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;
    use oteryn_protocol_oteryn::chat::ChatLine;
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    let actor = admitted.runtime_actor.ok_or("portal actor")?;
    let session = admitted.game_session_id;
    let fence = admitted.item_fence.ok_or("portal Character fence")?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[CAPABILITY_BOSS_ENCOUNTER_V1],
    )
    .ok_or("boss capability")?;
    owner.activity_runtimes.remember(*admitted).map_err(debug)?;

    let clock: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    h.root.set_test_quest_clock(clock)?;
    owner
        .states
        .chat
        .admit(session, "Portal entrant", owner.owner_now());
    let until: i64 = sqlx::query_scalar(
        "INSERT INTO game_character_boss_cooldowns(character_id,boss_key,available_at,admission,reset_epoch)
         VALUES(encode($1,'hex')::uuid,$3,$4,
            encode($2,'hex')::uuid,0) RETURNING available_at"
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(super::fixture::id(92).as_slice())
    .bind(boss)
    .bind((clock + 3661) * 1000)
    .fetch_one(&h.pool).await?;

    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        (
            origin.position().x,
            origin.position().y,
            origin.position().floor
        ),
        (i32::from(portal.x) - 1, i32::from(portal.y), 15)
    );
    let (outcome, _) = owner
        .ordinary_paced_step(actor, session, StepDirection::East, Some(origin), None)
        .await;
    assert_eq!(outcome.disposition, StepDisposition::Moved);
    let returned = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(returned.position(), origin.position());
    assert!(returned.revision() > origin.revision());
    let boss_name = match boss {
        "oteryn:boss.the_baron_from_below" => "The Baron from Below",
        "oteryn:boss.the_count_of_the_core" => "The Count of the Core",
        "oteryn:boss.the_duke_of_the_depths" => "The Duke of the Depths",
        _ => return Err("unexpected Warzone boss".into()),
    };
    assert_eq!(
        owner.states.chat.drain(session),
        vec![ChatLine::Private {
            speaker_name: "Game".into(),
            text: format!("You have to wait 1 hour, 1 minute, 1 second to face {boss_name} again!"),
        }]
    );
    let effects: Vec<_> = owner
        .bundle_map()
        .ok_or("portal World")?
        .facts
        .movement_effects_since(0)
        .into_iter()
        .filter(|(_, position, _)| {
            position.x == portal.x - 1 && position.y == portal.y && position.floor == portal.floor
        })
        .map(|(_, _, effect)| effect)
        .collect();
    assert_eq!(
        effects,
        [11, 3],
        "cooldown refusal cues occur at the return tile"
    );
    let preserved: i64 = sqlx::query_scalar(
        "SELECT available_at FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid
           AND boss_key=$2",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(boss).fetch_one(&h.pool).await?;
    assert_eq!(preserved, until);

    // This fixture has no active room definition for the portal. Clearing the
    // cooldown still must not leave the player standing on a failed entrance.
    sqlx::query(
        "DELETE FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid
         AND boss_key=$2",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(boss)
    .execute(&h.pool)
    .await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(3))
        .ok_or("portal retry clock")?;
    let (outcome, _) = owner
        .ordinary_paced_step(actor, session, StepDirection::East, Some(returned), None)
        .await;
    assert_eq!(outcome.disposition, StepDisposition::Moved);
    let refused = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(refused.position(), origin.position());
    assert!(refused.revision() > returned.revision());
    Ok(())
}

pub(super) fn warzone_pump(name: &str) -> Option<(Position, &'static str, bool, u8)> {
    let (x, y, award, combined, ordinal) = match name {
        "qengine-rewards-ddepths-pump" => (33673, 32304, "buried_the_baron", false, 2),
        "qengine-rewards-ddepths-pump-count" => (33679, 32310, "his_days_are_counted", false, 1),
        "qengine-rewards-ddepths-pump-duke" => (33685, 32304, "duked_it_out", true, 4),
        _ => return None,
    };
    Some((Position { x, y, floor: -15 }, award, combined, ordinal))
}

#[test]
fn ddepths_warzone_other_pumps_and_combined_award_native_use_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::qengine_rewards::scenario("qengine-rewards-ddepths-pump-count")?;
    super::qengine_rewards::scenario("qengine-rewards-ddepths-pump-duke")
}

#[test]
fn ddepths_warzone_pump_native_use_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::qengine_rewards::scenario("qengine-rewards-ddepths-pump")
}

pub(super) async fn exercise_warzone_pump(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    (pos, award, combined, ordinal): (Position, &str, bool, u8),
) -> TestResult {
    if combined {
        use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
        use crate::foundation::{CommandId, CommandRef};
        let fence = owner.current_quest_fence(admitted.game_session_id)
            .await.map_err(debug)?.ok_or("pump Character fence")?;
        let catalogue = owner.quest_catalogue.clone().ok_or("pump catalogue")?;
        // Commit a real quest transition to establish the first Character revision
        // before staging the previously earned pump flags for this isolated case.
        let result = owner.revision_sequencer.acquire(fence.character_id).await
            .commit_quest_transition(
                owner.root, owner.character, owner.holder, fence,
                QuestTransitionRequest {
                    transition_key: "oteryn:quest-transition/quest/u11_50/dangerous_depths/gnomes/measurements/npc_2".into(),
                    cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(8001).map_err(debug)?)),
                },
                catalogue,
            ).await.map_err(debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)), "{result:?}");
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx).await?;
        for key in ["the_baron_from_below_achiev", "the_count_of_the_core_achiev"] {
            sqlx::query(
                "INSERT INTO game_character_quest_tracks
                (character_id, track_key, quest_key, value, committed_character_revision)
                SELECT character_id, $2, 'oteryn:quest.dangerous_depths_quest', 1, character_revision
                FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(format!("oteryn:quest-progress/quest/u11_50/dangerous_depths/bosses/{key}"))
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)
        .ok_or("pump placement")?;
    let map = owner.bundle_map().ok_or("pump map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    }).map_err(debug)?;
    let fire_tiles: Vec<_> = (pos.x - 1..=pos.x + 1)
        .flat_map(|x| (pos.y - 1..=pos.y + 1).map(move |y| (x, y)))
        .filter(|(x, y)| map.overlay.base().tile(*x, *y, pos.floor).is_some())
        .collect();
    assert!(fire_tiles.contains(&(pos.x, pos.y)));
    let mut after = 0;
    for command in [1, 2] {
        let result =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        let observed: Vec<_> = map
            .facts
            .movement_effects_since(after)
            .into_iter()
            .filter_map(|(sequence, position, effect)| {
                after = sequence;
                (effect == 7 && position.floor == pos.floor).then_some((position.x, position.y))
            })
            .collect();
        assert_eq!(observed, fire_tiles, "pump fire effect on command {command}");
    }
    let award = format!("oteryn:achievement/{award}");
    let combined_key = "oteryn:achievement/death_in_the_depths";
    let awards: Vec<String> = sqlx::query_scalar(
        "SELECT achievement_key FROM game_account_achievements WHERE achievement_key IN ($1,$2)
         ORDER BY achievement_key",
    ).bind(&award).bind(combined_key).fetch_all(&h.pool).await?;
    let expected = if combined { vec![combined_key.to_owned(), award] } else { vec![award] };
    assert_eq!(awards, expected);
    Ok(())
}

pub(super) async fn exercise_achievement_routes(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    const INDIVIDUAL: &str = "oteryn:achievement/buried_the_baron";
    const COMBINED: &str = "oteryn:achievement/death_in_the_depths";
    let mut first = placeholder(AT)?;
    first.completes = false;
    first.requires_world_admission = true;
    first.achievements = vec![INDIVIDUAL.into()];
    let mut second = first.clone();
    second.key = format!("{STEP}-combined");
    second.storage[0].minimum = 2;
    second.storage[0].maximum = 2;
    second.storage[0].set = 3;
    second.achievements.push(COMBINED.into());
    let document = serde_json::json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": "c6bb54b0c423a165891b0ca32fb305a0963d1a06417249b0b317ec34cf65e708",
            "sources": ["Engine fixture for disjoint achievement USE routes"],
            "tracks": [{"key": PHASE, "initial": 0, "minimum": 0, "maximum": 3}],
            "stages": [
                {"key": "individual", "completes": false, "trigger": {"kind": "map_use", "callback": first}},
                {"key": "combined", "completes": false, "trigger": {"kind": "map_use", "callback": second}}
            ]
        }]
    });
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let revision = owner.quest_catalogue.as_deref().ok_or("native catalogue")?.content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));

    for (command, expected) in [(1, vec![INDIVIDUAL]), (2, vec![INDIVIDUAL, COMBINED])] {
        let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
            .ok_or("placement")?;
        let revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
        let target = target(owner, revision)?;
        let disposition = if command == 1 {
            let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&target)
                .map_err(debug)?;
            let result = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?
        } else {
            // Reproduce progress committed outside this connection's quest
            // projection. Call the real USE owner with the same admission.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            let changed = sqlx::query(
                "UPDATE game_character_quest_tracks SET value=2 WHERE quest_key=$1 AND track_key=$2 AND value=1",
            )
            .bind(QUEST)
            .bind(PHASE)
            .execute(&mut *tx)
            .await?;
            assert_eq!(changed.rows_affected(), 1);
            tx.commit().await?;
            tokio::time::sleep(std::time::Duration::from_millis(210)).await;
            owner.use_world_object(
                admitted.runtime_actor.ok_or("actor")?,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command,
                    item_fence: admitted.item_fence,
                },
                target,
            ).await.disposition
        };
        assert_eq!(disposition, UseDisposition::Committed);
        let earned: Vec<String> = sqlx::query_scalar(
            "SELECT achievement_key FROM game_account_achievements WHERE achievement_key IN ($1,$2) ORDER BY achievement_key DESC"
        )
        .bind(INDIVIDUAL)
        .bind(COMBINED)
        .fetch_all(&h.pool)
        .await?;
        let mut expected = expected;
        expected.sort_unstable_by(|left, right| right.cmp(left));
        assert_eq!(earned, expected);
    }
    Ok(())
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
