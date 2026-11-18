//! Fenced PostgreSQL USE and STEP through the world callback dispatcher.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data-global/scripts/quests:
//! the_pits_of_inferno_quest/actions_fountain.lua:3-15;
//! lions_rock/actions_lions_rock.lua:115-139;
//! the_first_dragon/actions_rewards.lua:43-72;
//! the_great_dragon_hunt_quest/actions_treasure.lua:19-40 and
//! the_great_dragon_hunt_quest/creaturescripts_the_great_dragon_hunt.lua:12-25.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) const PREFIX: &str = "qengine-rewards-qonce-";
const WELL: &str = "qengine-rewards-qonce-well";
const WELL_LOST_ACK: &str = "qengine-rewards-qonce-well-lost-ack";
const WELL_ROLLBACK: &str = "qengine-rewards-qonce-well-rollback";
const DAILY: &str = "qengine-rewards-qonce-daily";
const DAILY_HEART: &str = "qengine-rewards-qonce-daily-heart";
const FOUNTAIN_TRANSFORM: &str = "qengine-rewards-qonce-fountain-transform";
const ANNUAL: &str = "qengine-rewards-qonce-annual";
const FULL_BACKPACK: &str = "qengine-rewards-qonce-full-backpack";
const HOARD: &str = "qengine-rewards-qonce-hoard";
const HOARD_GEMS: &str = "qengine-rewards-qonce-hoard-gems";
const STEP: &str = "qengine-rewards-qonce-step";
const SOCKET_WINDOW: &str = "qengine-rewards-qonce-socket-window";
const SOCKET_REPEAT: &str = "qengine-rewards-qonce-socket-repeat";
const SOCKET_CATCHUP: &str = "qengine-rewards-qonce-socket-catchup";
const SOCKET_RECOVERY: &str = "qengine-rewards-qonce-socket-recovery";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const CLAIM: &str = "oteryn:quest-callback/qonce-dispatch-claim";
const LEDGER: &str = "oteryn:quest-progress/qonce-dispatch/ledger";
const CHOICE: &str = "oteryn:quest-progress/qonce-dispatch/choice";
const HUNT_GATE: &str = "oteryn:quest-progress/qonce-dispatch/hunt-gate";
const HUNT_KILL: &str = "oteryn:quest-transition/native-kills/qonce-hoard-dragon";
const DRAGON: &str = "oteryn:creature.dragon";
const LION: &str = "oteryn:creature.noble_lion";
const LION_MOUNT: &str = "oteryn:content.mount.noble_lion";
const LIONS_HEART: u16 = 21439;
const LIONS_DEN_ACHIEVEMENT: &str = "oteryn:achievement/lion_s_den_explorer";
const HOLY_AREA_EFFECT: u16 = 50;
const LIONS_FOUNTAIN: u16 = 6389;
const LIONS_SANCTUARY_ROCK: u16 = 1852;
const DAILY_PERIOD: i64 = 86_401;
const COUNT: &str = "oteryn:quest-progress/qonce-dispatch/count";
const HOARD_GOAL: i64 = 50;
const WELL_ACHIEVEMENT: &str = "oteryn:achievement/fountain_of_life";
const HOARD_ACHIEVEMENT: &str = "oteryn:achievement/hoard_of_the_dragon";
const DAILY_REFUSAL: &str = "At the moment there is neither a treasure nor anything else in the fountain. Perhaps you might return later.";
const FOUNTAIN_RED_GEM: &str = "Something sparkles in the fountain's water. You draw out a red gem.";
const ANNUAL_REFUSAL: &str = "The chest is empty.";
const ANNUAL_INELIGIBLE: &str = "You are not eligible to claim these rewards yet.";
const FEATHERS: &str = "oteryn:quest-progress/qonce-dispatch/feathers";
const HOARD_REFUSAL: &str = "You try to pick a treasure, but you hear further dragons approaching. You should kill some more before picking out something.";
const HOARD_MESSAGE: &str = "It is impossible to take along all of the treasures here. But you pick out ";
// Crystal actions_treasure.lua:1-14 orders the six draws and defines each bundle.
const HOARD_REWARDS: [(&str, &[(u16, u16)]); 6] = [
    ("a platinum coins", &[(3035, 5)]),
    ("some gems", &[(3029, 1), (3032, 1), (3030, 1)]),
    ("a life ring", &[(3089, 1)]),
    ("a red gem", &[(3039, 1)]),
    ("a mana potion", &[(237, 10)]),
    ("a health potion", &[(236, 8)]),
];
const AT: Position = Position { x: 33294, y: 31677, floor: -15 };
// A compact test room uses the donor's actual sockets, gems, fields and centre art.
const SOCKETS: [(Position, u16, u16, u16); 4] = [
    (AT, 21441, 3030, 21462),
    (Position { x: AT.x - 1, y: AT.y - 1, ..AT }, 21442, 3029, 21463),
    (Position { x: AT.x - 2, ..AT }, 21440, 3033, 21464),
    (Position { x: AT.x - 1, y: AT.y + 1, ..AT }, 21437, 9057, 21465),
];
const CENTRE: Position = Position { x: AT.x - 2, y: AT.y + 1, ..AT };
const SOCKET_PHASE: &str = "oteryn:quest-progress/qonce-dispatch/socket-phase";
const THUNDER: u16 = 73;
const SOCKET_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/gems.lua:35-89,130-189";
const ROOM_OPENS: &str = "Something happens at the center of the room ...";
const RUBY_ACCEPTED: &str = "You place the ruby on the small socket. A red flame begins to burn.";
const RUBY_EFFECT: u16 = 14;

#[test]
fn once_only_well_restores_saved_and_live_vitals_and_grants_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL)
}

#[test]
fn well_achievement_write_failure_rolls_back_the_heal_and_claim_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL_ROLLBACK)
}

#[test]
fn daily_claim_serializes_double_use_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(DAILY)
}

#[test]
fn fountain_heart_is_eligible_until_the_noble_lion_is_tamed_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(DAILY_HEART)
}

#[test]
fn annual_claim_delivers_backpack_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(ANNUAL)
}

#[test]
fn fountain_replay_projects_the_rock_and_refuses_it_after_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(FOUNTAIN_TRANSFORM)
}

pub(super) fn map_fixture(name: &str, callback: &mut Callback) {
    if name == FOUNTAIN_TRANSFORM {
        callback.selector.appearance = Some(LIONS_FOUNTAIN);
        callback.effects.transform = Some(LIONS_SANCTUARY_ROCK);
    }
    if is_socket_scenario(name) {
        callback.selector.appearance = Some(SOCKETS[0].1);
    }
}

fn is_socket_scenario(name: &str) -> bool {
    matches!(name, SOCKET_WINDOW | SOCKET_REPEAT | SOCKET_CATCHUP | SOCKET_RECOVERY)
}

pub(super) fn socket_world(name: &str, callback: &Callback) -> Option<TestResult<crate::map::boot::BundleWorld>> {
    is_socket_scenario(name).then(|| {
        let mut objects: Vec<_> = SOCKETS.iter().skip(1).map(|(pos, appearance, _, _)| (*pos, *appearance)).collect();
        objects.push((CENTRE, LIONS_SANCTUARY_ROCK));
        let mut appearances: Vec<_> = SOCKETS.iter().map(|(_, _, _, field)| *field).collect();
        appearances.push(LIONS_FOUNTAIN);
        super::quest_world_callbacks::world_with_objects(callback, AT, &objects, &appearances)
    })
}

#[test]
fn paid_lions_sockets_open_the_shared_fountain_and_expire_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(SOCKET_WINDOW)
}

#[test]
fn lions_socket_window_rearms_after_the_daily_claim_clock_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(SOCKET_REPEAT)
}

#[test]
fn lions_socket_window_opens_on_elapsed_overlap_when_scheduler_catches_up_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(SOCKET_CATCHUP)
}

#[test]
fn full_backpack_rolls_back_bundle_and_cooldown_until_claim_retry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(FULL_BACKPACK)
}

#[test]
fn hoard_bundle_resets_eligibility_and_awards_tenth_claim_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(HOARD)
}

#[test]
fn hoard_gem_bundle_commits_all_three_gems_with_the_claim_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(HOARD_GEMS)
}

#[test]
fn interrupted_well_use_recovers_committed_heal_without_second_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL_LOST_ACK)
}

#[test]
fn interrupted_socket_use_keeps_the_paid_field_deadlines_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(SOCKET_RECOVERY)
}

#[test]
fn step_claim_credits_the_same_achievement_only_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(STEP)
}

fn storage(track: &str, minimum: i64, maximum: i64, set: i64) -> Value {
    json!({"track": track, "minimum": minimum, "maximum": maximum, "set": set})
}

fn track(key: &str, initial: i64, minimum: i64, maximum: i64) -> Value {
    json!({"key": key, "initial": initial, "minimum": minimum, "maximum": maximum})
}

async fn track_value(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn achievements(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
        .bind(CLAIM).fetch_one(&h.pool).await?)
}

fn hoard_reward(description: &str, items: &[(u16, u16)]) -> Value {
    let amount = |&(item, count): &(u16, u16)| json!({"item":format!("oteryn:item.tibia.i{item}"),"count":count});
    let mut give = amount(&items[0]);
    give["additional"] = json!(items[1..].iter().map(amount).collect::<Vec<_>>());
    json!({"give":give,"message":format!("{HOARD_MESSAGE}{description}")})
}

async fn hoard_inventory(h: &Harness) -> TestResult<Vec<(u16, i64)>> {
    let mut inventory = Vec::new();
    for (_, items) in HOARD_REWARDS {
        for &(item, _) in items {
            inventory.push((item, super::qengine_voucher_stories::quantity(h, item).await?));
        }
    }
    Ok(inventory)
}

async fn assert_hoard_delivery(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    before: &[(u16, i64)],
    choices: &[(&str, &[(u16, u16)])],
) -> TestResult {
    let draw = usize::try_from(track_value(h, CHOICE).await?)?;
    let index = draw.checked_sub(1).ok_or("one-based hoard draw")?;
    let (description, bundle) = choices.get(index).ok_or("valid hoard draw")?;
    for (item, previous) in before {
        let expected = bundle.iter().find(|(id, _)| id == item).map_or(0, |(_, count)| i64::from(*count));
        assert_eq!(super::qengine_voucher_stories::quantity(h, *item).await? - *previous, expected,
            "draw {draw} must deliver the recorded bundle for item {item}");
    }
    let message = format!("{HOARD_MESSAGE}{description}");
    let feedback = owner.states.chat.drain(admitted.game_session_id).into_iter().filter(|line| matches!(
        line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == &message
    )).count();
    assert_eq!(feedback, 1);
    Ok(())
}

async fn claim_hoard_gems(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    before: &[(u16, i64)],
    choices: &[(&str, &[(u16, u16)])],
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};

    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    assert_hoard_mint_rollback(owner, admitted, h, before, &payload).await?;
    let session = admitted.game_session_id;
    if owner.has_pending_quest_callback(session) {
        assert!(owner.reconcile_quest_callbacks(session).await);
        assert!(!owner.has_pending_quest_callback(session));
    } else {
        advance_interaction(owner)?;
        let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload).await?;
        assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    }
    assert_hoard_delivery(owner, admitted, h, before, choices).await?;
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(track_value(h, LEDGER).await?, 0);
    assert_eq!(achievements(h, HOARD_ACHIEVEMENT).await?, 1);

    // Crystal actions_treasure.lua:5-8,23-27 places all three items on one claim.
    let items: Vec<(String, i64, bool)> = sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity, \
                i.xmin=m.created_xact_id::xid AND e.xmin=m.created_xact_id::xid \
         FROM game_reward_claim_mint_receipts m \
         JOIN game_character_quest_receipts q ON q.character_id=m.character_id \
           AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id \
           AND q.created_xact_id=m.created_xact_id \
         JOIN LATERAL ( \
           SELECT m.item_instance_id \
           UNION ALL SELECT (entry->>'item_instance_id')::uuid \
           FROM jsonb_array_elements(m.additional_plan) AS extra(entry) \
         ) AS chosen ON true \
         JOIN game_item_instances i ON i.item_instance_id=chosen.item_instance_id \
         JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id \
           AND e.parent_item_instance_id=m.destination_parent_item_instance_id \
         WHERE q.transition_key=$1 ORDER BY i.definition_production_key",
    ).bind(CLAIM).fetch_all(&h.pool).await?;
    assert_eq!(items, [3029, 3030, 3032].map(|id| (format!("oteryn:item.tibia.i{id}"), 1, true)));
    Ok(())
}

async fn assert_hoard_mint_rollback(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    before: &[(u16, i64)],
    payload: &[u8],
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::decode_use_result;

    let track_sql = "SELECT track_key, value FROM game_character_quest_tracks \
                     WHERE track_key IN ($1, $2) ORDER BY track_key";
    let tracks: Vec<(String, i64)> = sqlx::query_as(track_sql)
        .bind(CHOICE).bind(COUNT).fetch_all(&h.pool).await?;
    sqlx::raw_sql(r#"
        CREATE SEQUENCE fixture_hoard_earlier_gems_staged;
        GRANT USAGE, SELECT ON SEQUENCE fixture_hoard_earlier_gems_staged TO oteryn_game_runtime;
        CREATE FUNCTION fixture_fail_last_hoard_gem() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.definition_production_key = 'oteryn:item.tibia.i3030' AND (
                SELECT count(DISTINCT i.definition_production_key)
                FROM game_item_instances i
                JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id
                WHERE i.definition_production_key IN ('oteryn:item.tibia.i3029', 'oteryn:item.tibia.i3032')
                  AND i.xmin = pg_current_xact_id()::xid AND e.xmin = pg_current_xact_id()::xid
            ) = 2 THEN
                PERFORM nextval('public.fixture_hoard_earlier_gems_staged');
                RAISE EXCEPTION 'fixture rejects the ruby after sapphire and emerald placement';
            END IF;
            RETURN NEW;
        END $$;
        CREATE TRIGGER fixture_fail_last_hoard_gem
        BEFORE INSERT ON game_item_instances
        FOR EACH ROW EXECUTE FUNCTION fixture_fail_last_hoard_gem();
    "#).execute(&h.pool).await?;

    let session = admitted.game_session_id;
    match super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.to_vec()).await {
        Ok(reply) => assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected),
        Err(error) => assert!(owner.has_pending_quest_callback(session), "unexpected wire error: {error}"),
    }
    let staged: bool = sqlx::query_scalar("SELECT is_called FROM fixture_hoard_earlier_gems_staged")
        .fetch_one(&h.pool).await?;
    assert!(staged, "the fault must follow both earlier inventory placements");
    assert_eq!(hoard_inventory(h).await?.as_slice(), before);
    assert_eq!(track_value(h, LEDGER).await?, HOARD_GOAL);
    let after: Vec<(String, i64)> = sqlx::query_as(track_sql)
        .bind(CHOICE).bind(COUNT).fetch_all(&h.pool).await?;
    assert_eq!(after, tracks);
    assert_eq!(achievements(h, HOARD_ACHIEVEMENT).await?, 0);
    assert_eq!(receipts(h).await?, 0);
    let mints: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts \
         WHERE game_session_id=encode($1,'hex')::uuid AND command_id=1",
    ).bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(mints, 0);

    sqlx::query("DROP TRIGGER fixture_fail_last_hoard_gem ON game_item_instances")
        .execute(&h.pool).await?;
    Ok(())
}

async fn roll_back_well_grant(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    payload: &[u8],
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::decode_use_result;

    let saved_sql = "SELECT to_jsonb(p) FROM game_character_last_positions p";
    let before: Option<Value> = sqlx::query_scalar(saved_sql).fetch_optional(&h.pool).await?;
    sqlx::raw_sql(r#"
        CREATE SEQUENCE fixture_well_heal_staged;
        GRANT USAGE, SELECT ON SEQUENCE fixture_well_heal_staged TO oteryn_game_runtime;
        CREATE FUNCTION fixture_fail_well_achievement() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.achievement_key = 'oteryn:achievement/fountain_of_life' AND EXISTS (
                SELECT 1 FROM game_character_last_positions
                WHERE character_id = NEW.character_id AND health > 1 AND mana > 0
                    AND xmin = pg_current_xact_id()::xid
            ) THEN
                PERFORM nextval('public.fixture_well_heal_staged');
                RAISE EXCEPTION 'fixture rejects the achievement after staging the healed vitals';
            END IF;
            RETURN NEW;
        END $$;
        CREATE TRIGGER fixture_fail_well_achievement
        BEFORE INSERT ON game_account_achievement_grant_requests
        FOR EACH ROW EXECUTE FUNCTION fixture_fail_well_achievement();
    "#).execute(&h.pool).await?;

    let session = admitted.game_session_id;
    match super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.to_vec()).await {
        Ok(reply) => assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected),
        Err(error) => assert!(owner.has_pending_quest_callback(session), "unexpected wire error: {error}"),
    }
    let staged: bool = sqlx::query_scalar("SELECT is_called FROM fixture_well_heal_staged")
        .fetch_one(&h.pool).await?;
    assert!(staged, "the failure must follow the saved heal in its transaction");
    let saved: Option<Value> = sqlx::query_scalar(saved_sql).fetch_optional(&h.pool).await?;
    assert_eq!(saved, before);
    assert_eq!(receipts(h).await?, 0);
    assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 0);
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(LEDGER).fetch_one(&h.pool).await?;
    assert_eq!(claims, 0);
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let actor = admitted.runtime_actor.ok_or("well actor")?;
        let vitals = states.get(&runtime, actor, session).ok_or("well vitals")?.vitals();
        assert_eq!((vitals.health, vitals.mana), (1, 0));
    }

    sqlx::query("DROP TRIGGER fixture_fail_well_achievement ON game_account_achievement_grant_requests")
        .execute(&h.pool).await?;
    if owner.has_pending_quest_callback(session) {
        assert!(owner.reconcile_quest_callbacks(session).await);
        assert!(!owner.has_pending_quest_callback(session));
    } else {
        advance_interaction(owner)?;
        let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload.to_vec()).await?;
        assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    }
    Ok(())
}

fn advance_interaction(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1))
        .ok_or("world interaction clock")?;
    owner.states.world_gate.lock().map_err(|_| "world interaction gate")?.begin_turn();
    Ok(())
}

fn claim_target(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("claim map")?;
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("claim placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget { placement, expected_revision: map.facts.object_revision(key) })
}

async fn use_claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> TestResult<UseDisposition> {
    let actor = admitted.runtime_actor.ok_or("admitted actor")?;
    let map = owner.bundle_map().ok_or("claim map")?;
    let target = claim_target(owner)?;
    Ok(owner.use_quest_callback(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        },
        map,
        &target,
    ).await.ok_or("claim callback did not dispatch")?.disposition)
}

async fn socket_window(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use crate::gameplay_transport::{capabilities::SelectedCapabilities, connection::FreshAdmissionAuthority};
    use crate::map::overlay::TilePos;
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;
    use std::sync::atomic::Ordering;

    let offered = owner.offered_capabilities();
    let mut pending = vec![CAPABILITY_WORLD_INTERACTION_V1];
    let mut supported = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if supported.insert(id) {
            let capability = offered.iter().find(|capability| capability.id == id)
                .ok_or("socket use capability offer")?;
            pending.extend_from_slice(capability.requires);
        }
    }
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        offered, &supported.into_iter().collect::<Vec<_>>(),
    ).ok_or("socket use capability negotiation")?;
    let repeat = name == SOCKET_REPEAT;
    let catchup = name == SOCKET_CATCHUP;
    let mut stages = Vec::new();
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "socket fence unavailable")?.ok_or("socket fence")?;
    for (index, (position, appearance, gem, field)) in SOCKETS.into_iter().enumerate() {
        let phase = i64::try_from(index)? + 6;
        // Crystal gems.lua:179-189 consumes the gem used on the socket.
        let tool = json!({"item":format!("oteryn:item.tibia.i{gem}"),"count":1,"selected_instance":true});
        stages.push(json!({"key":format!("socket-{index}"),"completes":false,
            "trigger":{"kind":"map_use","callback":{
                "key":format!("oteryn:quest-callback/qonce-dispatch-socket-{index}"),
                "quest":QUEST,"trigger":"use",
                "selector":{"position":position,"appearance":appearance,"unique":(index == 0).then_some(63001)},
                "requires_world_admission":true,"reversible":false,"passages":[],
                "storage":[storage(SOCKET_PHASE,phase,phase,phase+1),{
                    "track":LEDGER,"minimum":-1,"maximum":i64::MAX,"set":0,"preserve":true
                }],"offerings":[],
                "return_on_failure":false,"sources":[SOCKET_SOURCE],"item_on_target":tool,
                "success_message":(index == 0).then_some(RUBY_ACCEPTED),
                "effects":{"magic":(index == 0).then_some(json!({"position":position,"effect":RUBY_EFFECT})),
                    "timed_presence":{"position":position,"appearance":field,
                    "spawn_after_ms":2000,"expires_after_ms":60000,"window":{
                        "position":CENTRE,"from":LIONS_SANCTUARY_ROCK,"to":LIONS_FOUNTAIN,
                        "count":4,"effect":THUNDER,"repeat":{"claim_track":LEDGER,"count":3},
                        "on_open":{"track":SOCKET_PHASE,"minimum":7,"value":10,"message":ROOM_OPENS}
                    }}}
            }}
        }));
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
            position, item: format!("oteryn:item.tibia.i{gem}"), count: 1,
        }, 70 + u64::try_from(index)?, true).await?;
    }
    // The same source has a disjoint post-claim stage. Reuse the claim clock as
    // a read-only transaction precondition before consuming the next ruby.
    let mut rearm = stages[0].clone();
    rearm["key"] = json!("socket-rearm");
    let callback = &mut rearm["trigger"]["callback"];
    callback["key"] = json!("oteryn:quest-callback/qonce-dispatch-socket-rearm");
    callback["storage"] = json!([storage(SOCKET_PHASE,11,11,7)]);
    callback["state_operations"] = json!([{
        "track":LEDGER,"from":{"kind":"elapsed_at_least","seconds":DAILY_PERIOD},
        "effect":{"kind":"add","value":0}
    }]);
    stages.push(rearm);
    const PREVIOUS_CLAIM: i64 = 100_000_000;
    let recipe_sha256: String = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter().map(|byte| format!("{byte:02x}")).collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":recipe_sha256,"sources":[SOCKET_SOURCE],
        "tracks":[track(SOCKET_PHASE,if repeat {11} else {6},6,11),
            track(LEDGER,if repeat {PREVIOUS_CLAIM} else {-1},-1,i64::MAX)],"stages":stages
    }]}))?)?;
    let revision = owner.quest_catalogue.as_deref().ok_or("catalogue")?.content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner.states.chat.admit(admitted.game_session_id, "Claimant", owner.owner_now());
    let _clock = owner.freeze_owner_clock_for_test();
    if name == SOCKET_RECOVERY {
        return socket_recovery(owner, admitted, h).await;
    }
    let centre = socket_key(CENTRE)?;
    if name == SOCKET_WINDOW {
        let wrong_gem = SOCKETS[1].2;
        let before = super::qengine_voucher_stories::quantity(h, wrong_gem).await?;
        assert_eq!(use_socket(owner, admitted, AT, wrong_gem).await?, UseDisposition::NothingToUse);
        assert_eq!(super::qengine_voucher_stories::quantity(h, wrong_gem).await?, before);
        assert_eq!(owner.bundle_map().ok_or("map")?.facts.next_movement_decay_time(), None);
    }
    if repeat {
        h.root.set_test_quest_clock(PREVIOUS_CLAIM + DAILY_PERIOD - 1).map_err(debug)?;
        let before = super::qengine_voucher_stories::quantity(h, SOCKETS[0].2).await?;
        assert_eq!(use_socket(owner, admitted, AT, SOCKETS[0].2).await?, UseDisposition::Rejected);
        assert_eq!(super::qengine_voucher_stories::quantity(h, SOCKETS[0].2).await?, before);
        assert_eq!(owner.bundle_map().ok_or("map")?.facts.next_movement_decay_time(), None);
        h.root.advance_test_quest_clock(1).map_err(debug)?;
    }
    let mut expiry = Vec::new();
    let count = if repeat { 3 } else { 4 };
    for (index, (position, _, gem, field)) in SOCKETS.into_iter().take(count).enumerate() {
        // Crystal gems.lua:179-188 stages a field two seconds after the paid USE.
        let spacing = if catchup { 400_000 } else { 3_000_000 };
        let at = 5_000_000 + spacing * u64::try_from(index)?;
        // The fixture stores owner time plus one because zero enables the real clock.
        owner.states.test_owner_time.store(at + 1, Ordering::Relaxed);
        let before = super::qengine_voucher_stories::quantity(h, gem).await?;
        assert_eq!(use_socket(owner, admitted, position, gem).await?, UseDisposition::Committed);
        assert_eq!(super::qengine_voucher_stories::quantity(h, gem).await?, before - 1);
        assert_eq!(track_value(h, SOCKET_PHASE).await?, i64::try_from(index)? + 7);
        let map = owner.bundle_map().ok_or("socket map")?;
        let tile = TilePos { x: position.x, y: position.y, floor: position.floor };
        let present = || map.facts.movement_items(tile).into_iter()
            .any(|item| item.facts.appearance_id == field);
        expiry.push((at + 60_000_000, tile));
        if catchup {
            assert!(!present());
            continue;
        }
        let starts = at + 2_000_000;
        assert_eq!(map.facts.next_movement_decay_time(), Some(starts));
        assert_eq!(map.facts.expire_movement_items(starts - 1), 0);
        assert!(!present());
        assert_eq!(map.facts.expire_movement_items(starts), 1);
        assert!(present());
        assert_eq!(map.facts.object_revision(centre), 0, "wait for durable room progress");
        if repeat && index + 1 == count {
            commit_opening_without_publication(owner, admitted, h, centre).await?;
        }
        owner.drain_paid_presence_opening().await;
        assert_eq!(map.facts.object_revision(centre), if index + 1 == count { 1 } else { 0 });
    }
    let map = owner.bundle_map().ok_or("socket map")?;
    if catchup {
        // The scheduler first runs after even the last field's visible lifetime.
        let last_expiry = expiry.last().ok_or("socket deadlines")?.0;
        owner.states.test_owner_time.store(last_expiry + 1, Ordering::Relaxed);
        assert_eq!(map.facts.expire_movement_items(last_expiry), count * 2);
        assert!(map.facts.next_movement_decay_time().is_none());
        for (_, tile) in expiry.drain(..) {
            assert!(map.facts.movement_items(tile).is_empty());
        }
        assert_eq!(map.facts.object_revision(centre), 0);
        assert!(map.facts.next_presence_opening().is_some());
        owner.drain_paid_presence_opening().await;
    }
    assert_eq!(track_value(h, SOCKET_PHASE).await?, 10);
    owner.drain_paid_presence_opening().await;
    let opened: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT values_before[1],values_after[1] FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-timed-presence/%'",
    ).fetch_all(&h.pool).await?;
    assert_eq!(opened, [(if repeat { 9 } else { 10 }, 10)]);
    let messages = owner.states.chat.drain(admitted.game_session_id);
    assert_eq!(messages.iter().filter(|line| matches!(
        line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == ROOM_OPENS
    )).count(), 1);
    let centre_tile = TilePos { x: CENTRE.x, y: CENTRE.y, floor: CENTRE.floor };
    assert_eq!(map.facts.movement_effects_since(0).into_iter()
        .filter(|(_, at, effect)| *at == centre_tile && *effect == THUNDER).count(), 1);
    for (deadline, tile) in expiry {
        assert_eq!(map.facts.expire_movement_items(deadline - 1), 0);
        assert_eq!(map.facts.expire_movement_items(deadline), 1);
        assert!(map.facts.movement_items(tile).is_empty());
    }
    assert_eq!(map.facts.object_revision(centre), 1, "fields expire while the fountain stays open for its claim");
    Ok(())
}

async fn socket_recovery(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use std::sync::atomic::Ordering;

    let (position, _, gem, field) = SOCKETS[0];
    let session = admitted.game_session_id;
    let paid = 5_000_000;
    owner.states.test_owner_time.store(paid + 1, Ordering::Relaxed);
    let before = super::qengine_voucher_stories::quantity(h, gem).await?;
    owner.states.quest_callbacks.interrupt_after_commit.store(true, Ordering::Release);
    assert!(use_socket(owner, admitted, position, gem).await.is_err());
    assert!(owner.has_pending_quest_callback(session));
    assert_eq!(track_value(h, SOCKET_PHASE).await?, 7);
    assert_eq!(super::qengine_voucher_stories::quantity(h, gem).await?, before - 1);

    let map = owner.bundle_map().ok_or("socket map")?;
    let ruby_feedback = || owner.states.chat.drain(session).into_iter().filter(|line| matches!(
        line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == RUBY_ACCEPTED
    )).count();
    let tile = crate::map::overlay::TilePos { x: position.x, y: position.y, floor: position.floor };
    let ruby_effects = || map.facts.movement_effects_since(0).into_iter()
        .filter(|(_, at, effect)| *at == tile && *effect == RUBY_EFFECT).count();
    assert!(map.facts.next_movement_decay_time().is_none());
    assert_eq!(ruby_feedback(), 0);
    assert_eq!(ruby_effects(), 0);
    // The connection recovers after the original spawn deadline. Crystal gems.lua:187-188
    // schedules spawn and expiry from the paid USE, so recovery cannot prolong the field.
    let resumed = paid + 3_000_000;
    owner.states.test_owner_time.store(resumed + 1, Ordering::Relaxed);
    assert!(owner.reconcile_quest_callbacks(session).await);
    assert!(!owner.has_pending_quest_callback(session));
    // Crystal gems.lua:43-44,183-184 presents the socket confirmation once.
    assert_eq!(ruby_feedback(), 1);
    assert_eq!(ruby_effects(), 1);
    assert_eq!(map.facts.next_movement_decay_time(), Some(paid + 2_000_000));
    assert_eq!(map.facts.expire_movement_items(resumed), 1);
    assert!(map.facts.movement_items(tile).iter().any(|item| item.facts.appearance_id == field));
    assert_eq!(map.facts.expire_movement_items(paid + 60_000_000), 1);
    assert!(map.facts.movement_items(tile).is_empty());
    assert!(owner.reconcile_quest_callbacks(session).await);
    assert!(map.facts.next_movement_decay_time().is_none());
    assert_eq!(ruby_feedback(), 0);
    assert_eq!(ruby_effects(), 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, gem).await?, before - 1);
    Ok(())
}

async fn commit_opening_without_publication(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    centre: u64,
) -> TestResult {
    use crate::durability::quest_state::QuestTransitionOutcome;
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::QuestRefusal;

    let map = owner.bundle_map().ok_or("socket map")?;
    let opening = map.facts.next_presence_opening().ok_or("room threshold")?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let session = admitted.game_session_id;
    let fence = owner.current_quest_fence(session).await
        .map_err(|()| "opening fence unavailable")?.ok_or("opening fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let unpaid = CommandRef::new(session, CommandId::new(90).map_err(debug)?);
    let refused = slot.commit_paid_presence_progress(
        owner.root, owner.character, owner.holder, fence, Arc::clone(catalogue),
        &opening.source.callback, unpaid,
    ).await?;
    assert_eq!(refused, QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
    assert_eq!(track_value(h, SOCKET_PHASE).await?, 9);

    // Stop after the real writer has committed but before the map publication.
    // The normal scheduler below must resume the same retained opening.
    let committed = slot.commit_paid_presence_progress(
        owner.root, owner.character, owner.holder, fence, Arc::clone(catalogue),
        &opening.source.callback, opening.source.command,
    ).await?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    assert_eq!(track_value(h, SOCKET_PHASE).await?, 10);
    assert_eq!(map.facts.object_revision(centre), 0);
    assert!(map.facts.next_presence_opening().is_some());
    let messages = owner.states.chat.drain(session);
    assert!(!messages.iter().any(|line| matches!(
        line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == ROOM_OPENS
    )));
    Ok(())
}

fn socket_key(at: Position) -> TestResult<u64> {
    oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1).ok_or_else(|| "socket key".into())
}

async fn use_socket(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
    gem: u16,
) -> TestResult<UseDisposition> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use oteryn_protocol_oteryn::item_view::{CAPABILITY_ITEM_EQUIP_DROP_V1,
        STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory_with_equip_drop};
    use oteryn_protocol_oteryn::world_object::{COMMAND_TYPE_USE_INTENT, MapUseTarget, UseIntent,
        UseTarget, decode_use_result, encode_item_use_intent};

    advance_interaction(owner)?;
    let map = owner.bundle_map().ok_or("socket map")?;
    let key = socket_key(at)?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget { placement, expected_revision: map.facts.object_revision(key) };
    let definition = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", &format!("oteryn:item.tibia.i{gem}"), "definition-r1"))
        .ok_or("socket gem definition")?;
    let equip_drop = admitted.continuity.selected_capabilities.contains(CAPABILITY_ITEM_EQUIP_DROP_V1);
    let command_id = admitted.continuity.next_command_id;
    let (status, reply, scope, _) = super::guild_wire::exchange_capture(owner, admitted, command_id,
        COMMAND_TYPE_USE_INTENT, |frames| {
            let mut handle = None;
            for frame in frames {
                let envelope = decode_wire_envelope(frame).map_err(debug)?;
                if envelope.message_type() != MessageType::SnapshotChunk { continue; }
                let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                for domain in domains.into_iter().filter(|domain| domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY) {
                    let inventory = decode_character_inventory_with_equip_drop(domain.payload, equip_drop).map_err(debug)?;
                    for entry in inventory.entries.into_iter().filter(|entry| entry.item_definition_ref == definition) {
                        assert!(handle.replace(entry.handle).is_none(), "one carried copy of the selected gem");
                    }
                }
            }
            encode_item_use_intent(&UseIntent {target: UseTarget::Item(handle.ok_or("current carried gem handle")?),
                use_with_map_item: Some(MapUseTarget::Object(target)), use_with: None,
                use_with_position: None}).map_err(debug)
        }, false, &mut Vec::new()).await?;
    let disposition = decode_use_result(&reply).map_err(debug)?;
    assert_eq!(status, if disposition == UseDisposition::Rejected {
        oteryn_protocol_oteryn::CommandStatus::Rejected
    } else {
        oteryn_protocol_oteryn::CommandStatus::Accepted
    });
    assert!(scope.is_none());
    Ok(disposition)
}

async fn contend_for_daily_claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
) -> TestResult<[UseDisposition; 2]> {
    let first_command = 21_u64;
    let mut cause = admitted.game_session_id.as_bytes().to_vec();
    cause.extend_from_slice(&first_command.to_be_bytes());
    let mut blocker = h.pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:reward-claim-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(cause)
    .execute(&mut *blocker)
    .await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;

    let first = use_claim(owner, admitted, first_command);
    tokio::pin!(first);
    // Observe the first USE inside the durable writer before dispatching the second.
    let writer_waiting = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity \
                 WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(blocker_pid)
            .fetch_one(&h.pool)
            .await?;
            if blocked {
                return Ok::<(), sqlx::Error>(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });
    tokio::select! {
        result = &mut first => return Err(format!("first claim finished before acquiring the reward cause: {result:?}").into()),
        waiting = writer_waiting => waiting??,
    }

    // Poll the second USE while the first is still waiting on the reward cause.
    let (first, second, released) = tokio::join!(biased;
        first, use_claim(owner, admitted, 22), blocker.rollback(),
    );
    released?;
    let first = first?;
    assert_eq!(first, UseDisposition::Committed);
    Ok([first, second?])
}

async fn tame_noble_lion(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

    let source = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i21439", "definition-r1"))
        .ok_or("Lion's Heart definition")?;
    let actor = admitted.runtime_actor.ok_or("admitted lion tamer")?;
    let entity = {
        let mut runtime = owner.runtime.lock().await;
        let from = runtime.read_actor_position(actor).map_err(debug)?.position();
        let target = runtime.admit_source_pinned_lab_creature(
            crate::foundation::MovementLocalPosition { x: from.x + 1, ..from },
            LION, 100,
        ).map_err(debug)?;
        let visible = runtime.visible_entities();
        let lion = visible.creatures.iter().find(|creature| creature.actor == target)
            .ok_or("visible Noble Lion")?;
        EntityRef { identity: target.placement_identity(), generation: lion.generation }
    };
    crate::gameplay_transport::mount_taming::test_draw::set(0);
    let result = owner.tame_mount_impl(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 88_101,
            item_fence: admitted.item_fence,
        },
        UseIntent {
            target: UseTarget::ItemByDefinition(source),
            use_with: Some(entity),
            use_with_map_item: None,
            use_with_position: None,
        },
        None,
        None,
        true,
    ).await.map_err(|()| "Noble Lion taming dispatch")?.ok_or("Noble Lion taming result")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    let entitlement: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:taming.noble_lion')",
    ).fetch_one(&h.pool).await?;
    assert!(entitlement, "the normal taming owner grants the Noble Lion");
    Ok(())
}

async fn claim_then_tame_lion(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};

    let before = super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?;
    assert_eq!(before, 0);
    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    assert_eq!(super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?, 1);
    let first_claim = track_value(h, LEDGER).await?;
    assert_eq!(track_value(h, CHOICE).await?, 1);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(achievements(h, LIONS_DEN_ACHIEVEMENT).await?, 1);

    tame_noble_lion(owner, admitted, h).await?;
    assert_eq!(super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?, 0);
    h.root.advance_test_quest_clock(DAILY_PERIOD).map_err(debug)?;
    advance_interaction(owner)?;
    // Isolate this candidate to test exclusion independently of the random draw.
    // Crystal actions_lions_rock.lua:121-127 excludes the Heart after taming.
    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(track_value(h, LEDGER).await?, first_claim);
    assert_eq!(track_value(h, CHOICE).await?, 1);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(achievements(h, LIONS_DEN_ACHIEVEMENT).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?, 0);
    Ok(())
}

async fn full_backpack_retry(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};

    let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .ok_or("native item policies")?;
    let bag = generation_item_facts(native, &TypedDefinitionRef {
        family:"Item".into(), production_key:"oteryn:item.tibia.i2854".into(),
        revision_ref:"definition-r1".into(),
    }).map_err(debug)?;
    let capacity = i64::from(bag.container_capacity.ok_or("backpack capacity")?);
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool).await?;
    assert!(occupied < capacity);
    let mut ordinal = 89_000;
    for _ in occupied..capacity {
        super::qengine_voucher_stories::mint(owner, admitted, 3031, 100, &mut ordinal).await?;
    }
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool).await?;
    assert_eq!(occupied, capacity);
    let before = super::qengine_voucher_stories::quantity(h, 2854).await?;
    let before_red = super::qengine_voucher_stories::quantity(h, 3039).await?;
    let before_yellow = super::qengine_voucher_stories::quantity(h, 3037).await?;
    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(receipts(h).await?, 0);
    let (tracks, mints): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key IN ($1,$2)),                 (SELECT count(*) FROM game_reward_claim_mint_receipts                  WHERE game_session_id=encode($3,'hex')::uuid AND command_id=1)",
    ).bind(LEDGER).bind(CHOICE).bind(admitted.game_session_id.as_bytes().as_slice())
        .fetch_one(&h.pool).await?;
    assert_eq!((tracks, mints), (0, 0), "failed delivery leaves no claim or cooldown");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 2854).await?, before);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before_red);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3037).await?, before_yellow);

    let filler: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT i.item_instance_id FROM game_item_container_entries e          JOIN game_item_instances i USING (item_instance_id)          WHERE i.definition_production_key='oteryn:item.tibia.i3031' AND i.quantity=100          ORDER BY i.item_instance_id LIMIT 1",
    ).fetch_one(&h.pool).await?;
    let fence = admitted.item_fence.ok_or("item fence")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (runtime.content_pin().server_artifact_digest(), runtime.content_pin().frame_binding_digest())
    };
    let revision: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let map = owner.bundle_map().ok_or("claim map")?;
    let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
    let moved = owner.root.commit_item_loop_move(
        &permit, owner.character, owner.holder, &fence,
        CommandRef::new(admitted.game_session_id, CommandId::new(89_990).map_err(debug)?),
        &NativeItemWeights(native),
        LoopMoveRequest {
            item:*filler.as_bytes(), count:None, pickup_cell:None,
            destination:LoopDestination::Ground(LoopGround {
                spatial_position:crate::durability::map_item_mint_audit::map_item_spatial_position(
                    AT.x - 1, AT.y, AT.floor,
                ),
                map_revision:crate::map::overlay::map_revision(map.overlay.base()),
                content_revision:format!("sha256:{revision}"),
                placement_context:frame.to_vec(),
            }),
        },
    ).await.map_err(debug)?;
    assert!(moved.is_ok(), "the production move frees a backpack slot: {moved:?}");
    drop(permit);

    advance_interaction(owner)?;
    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(track_value(h, LEDGER).await?, 100_000_000);
    assert_eq!(track_value(h, CHOICE).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 2854).await?, before + 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before_red + 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3037).await?, before_yellow + 1);
    let children: Vec<(String, i64, bool)> = sqlx::query_as(
        "SELECT child.definition_production_key, child.quantity,
                 claim.created_xact_id=quest.created_xact_id
                 AND child.xmin::text=claim.xmin::text AND nested.xmin::text=claim.xmin::text
         FROM game_reward_claim_mint_receipts claim
         JOIN game_character_quest_receipts quest ON quest.character_id=claim.character_id
           AND quest.cause_id=claim.game_session_id AND quest.cause_ordinal=claim.command_id
         JOIN game_item_nested_entries nested ON nested.parent_item_instance_id=claim.item_instance_id
         JOIN game_item_instances child ON child.item_instance_id=nested.item_instance_id
         WHERE quest.transition_key=$1 AND claim.command_id=2
         ORDER BY nested.placement_ordinal",
    ).bind(CLAIM).fetch_all(&h.pool).await?;
    assert_eq!(children, [
        ("oteryn:item.tibia.i3039".to_owned(), 1_i64, true),
        ("oteryn:item.tibia.i3037".to_owned(), 1_i64, true),
    ], "the retry places both children in the claimed backpack in the claim transaction");
    Ok(())
}

async fn fountain_transform(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::encode_use_intent;
    let source_appearance = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<u16> {
        let map = owner.bundle_map().ok_or("fountain map")?;
        let tile = map.overlay.base().tile(AT.x, AT.y, AT.floor).ok_or("fountain tile")?;
        let (id, _) = tile.ids().iter().zip(tile.depths())
            .filter(|(_, depth)| **depth == 0).nth(1).ok_or("fountain stack entry")?;
        Ok(map.facts.base_entry(
            crate::map::overlay::TilePos { x: AT.x, y: AT.y, floor: AT.floor },
            1,
            *id,
        ).ok_or("fountain source facts")?.appearance_id)
    };
    let holy_area_count = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<usize> {
        let map = owner.bundle_map().ok_or("fountain map")?;
        Ok(map.facts.movement_effects_since(0).into_iter()
            .filter(|(_, _, effect)| *effect == HOLY_AREA_EFFECT).count())
    };
    assert_eq!(source_appearance(owner)?, LIONS_FOUNTAIN);
    let before = super::qengine_voucher_stories::quantity(h, 3039).await?;
    let original = claim_target(owner)?;
    let payload = encode_use_intent(&original).map_err(debug)?;
    owner.states.quest_callbacks.interrupt_after_commit
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await.is_err());
    assert!(owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(source_appearance(owner)?, LIONS_FOUNTAIN);
    assert_eq!(holy_area_count(owner)?, 0);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(track_value(h, CHOICE).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before + 1);

    assert!(owner.reconcile_quest_callbacks(admitted.game_session_id).await);
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(source_appearance(owner)?, LIONS_SANCTUARY_ROCK);
    assert_eq!(claim_target(owner)?.expected_revision, original.expected_revision + 1);
    assert_eq!(holy_area_count(owner)?, 1);
    assert!(owner.reconcile_quest_callbacks(admitted.game_session_id).await);
    h.root.advance_test_quest_clock(DAILY_PERIOD).map_err(debug)?;
    advance_interaction(owner)?;
    assert_eq!(use_claim(owner, *admitted, 2).await?, UseDisposition::NothingToUse);
    assert_eq!(receipts(h).await?, 1, "an expired personal clock cannot reopen the shared rock");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before + 1);
    assert_eq!(holy_area_count(owner)?, 1);
    let reward_lines = owner.states.chat.drain(admitted.game_session_id).into_iter()
        .filter(|line| matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
            if text == FOUNTAIN_RED_GEM)).count();
    assert_eq!(reward_lines, 1, "recover the selected reward message once");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    if is_socket_scenario(name) {
        return Box::pin(socket_window(owner, admitted, h, name)).await;
    }
    let hoard_rewards = match name {
        HOARD => HOARD_REWARDS.as_slice(),
        HOARD_GEMS => &HOARD_REWARDS[1..2],
        _ => &[],
    };
    let hoard = !hoard_rewards.is_empty();

    // The source tile is installed by the common dispatch fixture. Build the binding
    // independently so its guards and effects come entirely from this scenario.
    let mut callback = json!({
        "key":CLAIM,"quest":QUEST,"trigger":"use",
        "selector":{"position":AT,"unique":63001,"appearance":100},
        "requires_world_admission":true,"reversible":false,"passages":[],
        "storage":[],"offerings":[],"return_on_failure":false,
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_pits_of_inferno_quest/actions_fountain.lua:3-15",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/lions_rock/actions_lions_rock.lua:115-139",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/actions_rewards.lua:43-72",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_great_dragon_hunt_quest/actions_treasure.lua:19-40"
        ]
    });
    let well = matches!(name, WELL | WELL_LOST_ACK | WELL_ROLLBACK);
    let mut tracks = vec![];
    let mut effects;
    let period = match name {
        DAILY | DAILY_HEART | FOUNTAIN_TRANSFORM => Some((DAILY_PERIOD, DAILY_REFUSAL)),
        ANNUAL | FULL_BACKPACK => Some((31_536_001_i64, ANNUAL_REFUSAL)),
        _ => None,
    };
    if let Some((seconds, refusal)) = period {
        tracks.push(track(LEDGER, -1, -1, i64::MAX));
        let choices = if name == DAILY { 2 } else { 1 };
        tracks.push(track(CHOICE, 0, 0, choices));
        callback["storage"] = json!([storage(CHOICE, 0, choices, 0)]);
        callback["state_operations"] = json!([{
            "track": LEDGER, "from": {"kind":"elapsed_at_least","seconds":seconds},
            "effect":{"kind":"set_now"}
        }]);
        callback["failure_message"] = json!(refusal);
        let give = match name {
            ANNUAL => json!({"item":"oteryn:item.tibia.i2854","count":1,
                "contents":[{"item":"oteryn:item.tibia.i3031","count":2}]}),
            FOUNTAIN_TRANSFORM => json!({"item":"oteryn:item.tibia.i3039","count":1}),
            FULL_BACKPACK => json!({"item":"oteryn:item.tibia.i2854","count":1,"contents":[
                {"item":"oteryn:item.tibia.i3039","count":1},
                {"item":"oteryn:item.tibia.i3037","count":1}
            ]}),
            _ => json!({"item":"oteryn:item.tibia.i3031","count":2}),
        };
        let heart = || json!({
                "give":{"item":"oteryn:item.tibia.i21439","count":1},
                "exclude_owned_mount":LION_MOUNT
            });
        let mut rewards = match name {
            DAILY => vec![heart(), json!({"give":give})],
            DAILY_HEART => vec![heart()],
            _ => vec![json!({"give":give})],
        };
        if name == FOUNTAIN_TRANSFORM {
            rewards[0]["message"] = json!(FOUNTAIN_RED_GEM);
        }
        effects = json!({"reset":{"kind":"elapsed","track":LEDGER,"seconds":seconds},
            "selection_track":CHOICE,"rewards":rewards});
        if matches!(name, DAILY | DAILY_HEART | FOUNTAIN_TRANSFORM) {
            effects["actor_magic_effect"] = json!(HOLY_AREA_EFFECT);
            callback["achievements"] = json!([LIONS_DEN_ACHIEVEMENT]);
        }
        if name == ANNUAL {
            tracks.push(track(FEATHERS, 1, 1, 2));
            let mut feathers = storage(FEATHERS, 2, 2, 2);
            feathers["preserve"] = json!(true);
            callback["storage"] = json!([storage(CHOICE, 0, choices, 0), feathers]);
            effects["eligibility"] = json!({"track":FEATHERS,"message":ANNUAL_INELIGIBLE});
        }
    } else if hoard {
        let choices = i64::try_from(hoard_rewards.len())?;
        tracks.extend([
            track(LEDGER, HOARD_GOAL - 1, 0, HOARD_GOAL), track(COUNT, 9, 0, i64::MAX),
            track(CHOICE, 0, 0, choices), track(HUNT_GATE, 0, 0, 0),
        ]);
        // Crystal actions_treasure.lua:29-37 keeps counting after the tenth claim.
        let mut count = storage(COUNT, 0, i64::MAX - 1, 1);
        count["increment"] = json!(true);
        callback["storage"] = json!([storage(LEDGER, HOARD_GOAL, HOARD_GOAL, 0), count, storage(CHOICE, 0, choices, 0)]);
        callback["failure_message"] = json!(HOARD_REFUSAL);
        effects = json!({"reset":{"kind":"progress","track":LEDGER,"minimum":HOARD_GOAL},
            "selection_track":CHOICE,
            "milestone":{"track":COUNT,"count":10,"achievement":HOARD_ACHIEVEMENT},
            "rewards":hoard_rewards.iter().map(|&(description, items)| hoard_reward(description, items)).collect::<Vec<_>>()});
    } else {
        assert!(well || name == STEP);
        tracks.push(track(LEDGER, 0, 0, 1));
        callback["storage"] = json!([storage(LEDGER, 0, 0, 1)]);
        callback["achievements"] = json!([WELL_ACHIEVEMENT]);
        callback["failure_message"] = json!("Sorry, not possible.");
        effects = json!({"reset":{"kind":"once","track":LEDGER}});
        if well {
            effects["restore_vitals"] = json!(true);
        } else {
            callback["trigger"] = json!("step_in");
            callback["selector"]["unique"] = Value::Null;
            callback["selector"]["appearance"] = Value::Null;
        }
    }
    callback["effects"]["claim"] = effects;
    if name == FOUNTAIN_TRANSFORM {
        callback["selector"]["appearance"] = json!(LIONS_FOUNTAIN);
        callback["effects"]["transform"] = json!(LIONS_SANCTUARY_ROCK);
    }
    let recipe_sha256: String = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let trigger = if name == STEP { "step_in" } else { "map_use" };
    let mut stages = vec![json!({
        "key":"claim","completes":false,"trigger":{"kind":trigger,"callback":callback}
    })];
    if hoard {
        stages.push(json!({
            "key":"kill-dragon","completes":false,"trigger":{"kind":"kill","counter":{
                "key":HUNT_KILL,"creature":DRAGON,"track":LEDGER,
                "gate":HUNT_GATE,"gate_value":0,"maximum":HOARD_GOAL,
                "death_area":{"minimum":[AT.x-2, AT.y-2, -AT.floor],
                              "maximum":[AT.x+2, AT.y+2, -AT.floor]}
            }}
        }));
    }
    if name == ANNUAL {
        stages.push(json!({
            "key":"feathers","completes":false,"trigger":{"kind":"step_in","callback":{
                "key":"oteryn:quest-callback/qonce-dispatch-feathers","quest":QUEST,
                "trigger":"step_in","selector":{"position":AT},
                "requires_world_admission":true,"reversible":false,"passages":[],
                "storage":[storage(FEATHERS,1,1,2)],"offerings":[],
                "return_on_failure":false,"sources":[
                    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/actions_rewards.lua:43-45"
                ]
            }}
        }));
    }
    let binding = json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":recipe_sha256,
        "sources":["synthetic dispatch tests exercising the Crystal claim shapes"],
        "tracks":tracks,
        "stages":stages
    }]});
    if hoard {
        let mut invalid = binding.clone();
        invalid["quests"][0]["stages"][0]["trigger"]["callback"]["storage"][1]["maximum"] = json!(8);
        assert!(Bindings::parse(&serde_json::to_vec(&invalid)?).is_err());
        invalid["quests"][0]["stages"][0]["trigger"]["callback"]["storage"][1]["maximum"] = json!(9);
        invalid["quests"][0]["tracks"][1]["maximum"] = json!(9);
        assert_eq!(
            Bindings::parse(&serde_json::to_vec(&invalid)?).err().as_deref(),
            Some("authored claim milestone exceeds track bounds"),
        );
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&binding)?)?;
    let revision = owner.quest_catalogue.as_deref().ok_or("catalogue")?.content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner.states.chat.admit(admitted.game_session_id, "Claimant", owner.owner_now());

    if name == STEP {
        for (command, direction) in [(1, StepDirection::East), (2, StepDirection::West), (3, StepDirection::East)] {
            let reply = super::quest_world_callbacks::wire(owner, admitted, command, 1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction)).await?;
            assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
        }
        assert_eq!(receipts(h).await?, 1);
        assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
        return Ok(());
    }

    if well {
        use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};

        let actor = admitted.runtime_actor.ok_or("well actor")?;
        let session = admitted.game_session_id;
        let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
        let full = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("well vitals")?;
            let full = state.vitals();
            assert!(full.health > 1 && full.mana > 0);
            state.restore_saved_vitals(1, 0);
            (full.health, full.mana)
        };
        if name == WELL_ROLLBACK {
            // Crystal actions_fountain.lua:7-10 keeps the restored pools and the one-time award together.
            roll_back_well_grant(owner, admitted, h, &payload).await?;
        } else if name == WELL_LOST_ACK {
            owner.states.quest_callbacks.interrupt_after_commit
                .store(true, std::sync::atomic::Ordering::Release);
            assert!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await.is_err());
            assert!(owner.states.quest_callbacks.has_pending_claim_vitals(session));
            assert_eq!(receipts(h).await?, 1);
            assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
            let saved: (i32, i32) = sqlx::query_as("SELECT health,mana FROM game_character_last_positions")
                .fetch_one(&h.pool).await?;
            assert_eq!(saved, (i32::try_from(full.0)?, i32::try_from(full.1)?));
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let vitals = states.get(&runtime, actor, session).ok_or("interrupted actor")?.vitals();
                assert_eq!((vitals.health, vitals.mana), (1, 0));
            }
            assert!(owner.reconcile_quest_callbacks(session).await);
            assert!(!owner.states.quest_callbacks.has_pending_claim_vitals(session));
        } else {
            let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
            assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
        }
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("healed actor")?;
            let vitals = state.vitals();
            assert_eq!((vitals.health, vitals.mana), full);
            let (damaged, _) = state.after_creature_damage(full.0 - 1).ok_or("well damage")?;
            *state = damaged;
        }
        if name == WELL {
            let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
            assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states.get(&runtime, actor, session).ok_or("replayed actor")?.vitals();
            assert_eq!((vitals.health, vitals.mana), (1, full.1));
        }
        advance_interaction(owner)?;
        let next = if name == WELL_ROLLBACK { 3 } else { 2 };
        assert_eq!(use_claim(owner, *admitted, next).await?, UseDisposition::Rejected);
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states.get(&runtime, actor, session).ok_or("refused actor")?.vitals();
            assert_eq!((vitals.health, vitals.mana), (1, full.1));
        }
        let saved: (i32, i32) = sqlx::query_as("SELECT health,mana FROM game_character_last_positions")
            .fetch_one(&h.pool).await?;
        assert_eq!(saved, (i32::try_from(full.0)?, i32::try_from(full.1)?));
        assert_eq!(receipts(h).await?, 1);
        assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
        return Ok(());
    }

    h.root.set_test_quest_clock(100_000_000).map_err(debug)?;
    if name == FOUNTAIN_TRANSFORM {
        return Box::pin(fountain_transform(owner, admitted, h)).await;
    }
    if name == FULL_BACKPACK {
        return Box::pin(full_backpack_retry(owner, admitted, h)).await;
    }
    if name == DAILY_HEART {
        return Box::pin(claim_then_tame_lion(owner, admitted, h)).await;
    }
    if name == DAILY {
        let mut ordinal = 88_100;
        super::qengine_voucher_stories::mint(owner, admitted, LIONS_HEART, 1, &mut ordinal).await?;
        tame_noble_lion(owner, admitted, h).await?;
    }
    if hoard {
        owner.active_generation.and_then(|generation| generation.native_gameplay())
            .ok_or("native creature policies")?
            .install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
        assert_eq!(use_claim(owner, *admitted, 20).await?, UseDisposition::Rejected);
        assert_eq!(receipts(h).await?, 0);
        advance_interaction(owner)?;
        super::qengine_kill_stories::death(owner, admitted, DRAGON, 88_001).await?;
        assert_eq!(track_value(h, LEDGER).await?, HOARD_GOAL, "the death opens the hoard");
    }
    let hoard_before = if hoard { hoard_inventory(h).await? } else { Vec::new() };
    if name == HOARD_GEMS {
        return Box::pin(claim_hoard_gems(owner, admitted, h, &hoard_before, hoard_rewards)).await;
    }
    let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
    if name == ANNUAL {
        assert_eq!(use_claim(owner, *admitted, 20).await?, UseDisposition::Rejected);
        assert_eq!(receipts(h).await?, 0);
        assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before);
        let chat = owner.states.chat.drain(admitted.game_session_id);
        assert!(chat.iter().any(|line| matches!(line,
            oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
                if text == ANNUAL_INELIGIBLE
        )));
        let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East)).await?;
        assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
        assert_eq!(track_value(h, FEATHERS).await?, 2);
        advance_interaction(owner)?;
    }
    let before_hearts = super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?;
    let outcomes = if name == DAILY {
        Box::pin(contend_for_daily_claim(owner, *admitted, h)).await?
    } else {
        let (first, second) = tokio::join!(use_claim(owner, *admitted, 21), use_claim(owner, *admitted, 22));
        [first?, second?]
    };
    let committed = outcomes.into_iter().filter(|outcome| *outcome == UseDisposition::Committed).count();
    assert_eq!(committed, 1);
    assert_eq!(receipts(h).await?, 1);
    if hoard {
        assert_hoard_delivery(owner, admitted, h, &hoard_before, hoard_rewards).await?;
    } else {
        assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before + 2);
    }
    advance_interaction(owner)?;
    assert_eq!(use_claim(owner, *admitted, 23).await?, UseDisposition::Rejected);
    let refusal = period.map_or(HOARD_REFUSAL, |(_, refusal)| refusal);
    let chat = owner.states.chat.drain(admitted.game_session_id);
    assert!(chat.iter().any(|line| matches!(
        line,
        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == refusal
    )));
    if let Some((seconds, _)) = period {
        h.root.advance_test_quest_clock(seconds - 1).map_err(debug)?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 24).await?, UseDisposition::Rejected);
        h.root.advance_test_quest_clock(1).map_err(debug)?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 25).await?, UseDisposition::Committed);
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 26).await?, UseDisposition::Rejected);
        assert_eq!(receipts(h).await?, 2);
        assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before + 4);
    } else {
        assert_eq!(achievements(h, HOARD_ACHIEVEMENT).await?, 1);
        let (ledger, count): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1), \
                    (SELECT value FROM game_character_quest_tracks WHERE track_key=$2)",
        ).bind(LEDGER).bind(COUNT).fetch_one(&h.pool).await?;
        assert_eq!((ledger, count), (0, 10));
        // Crystal actions_treasure.lua:20,27,29-37 opens the next hunt at zero
        // and permits subsequent bundles without re-awarding the achievement.
        for expected in 1..=HOARD_GOAL {
            let command = 88_001 + u64::try_from(expected)?;
            super::qengine_kill_stories::death(owner, admitted, DRAGON, command).await?;
            assert_eq!(track_value(h, LEDGER).await?, expected);
        }
        let deaths: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        ).bind(HUNT_KILL).fetch_one(&h.pool).await?;
        assert_eq!(deaths, HOARD_GOAL + 1, "duplicate death delivery earns one credit per death");
        let next_inventory = hoard_inventory(h).await?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 24).await?, UseDisposition::Committed);
        assert_eq!(track_value(h, LEDGER).await?, 0);
        assert_eq!(track_value(h, COUNT).await?, 11);
        assert_eq!(achievements(h, HOARD_ACHIEVEMENT).await?, 1);
        assert_eq!(receipts(h).await?, 2);
        assert_hoard_delivery(owner, admitted, h, &next_inventory, hoard_rewards).await?;
    }
    if name == DAILY {
        assert_eq!(track_value(h, CHOICE).await?, 2);
        assert_eq!(super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?, before_hearts);
        assert_eq!(achievements(h, LIONS_DEN_ACHIEVEMENT).await?, 1);
        let actor = admitted.runtime_actor.ok_or("fountain claimant")?;
        let cell = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
        let claimant_cell = crate::map::overlay::TilePos {
            x: cell.x.try_into()?, y: cell.y.try_into()?, floor: (-cell.floor).try_into()?,
        };
        let holy_area: Vec<_> = owner.bundle_map().ok_or("fountain map")?.facts
            .movement_effects_since(0).into_iter()
            .filter_map(|(_, position, effect)| (effect == HOLY_AREA_EFFECT).then_some(position))
            .collect();
        assert_eq!(holy_area, [claimant_cell, claimant_cell]);

    }
    let atomic: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts m \
         JOIN game_character_quest_receipts q ON q.character_id=m.character_id \
          AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id \
          AND q.created_xact_id=m.created_xact_id WHERE q.transition_key=$1",
    ).bind(CLAIM).fetch_one(&h.pool).await?;
    assert_eq!(atomic, receipts(h).await?);
    Ok(())
}
