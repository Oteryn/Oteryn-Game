//! W17 player scenario, derived from the matrix, the 2026-10-28 re-audit, and
//! the admitted owners in `arena_catalogue.rs`, `arena_match.rs`, and
//! `tibiadrome.rs`:
//! 1. The admitted player requests the Svargrond catalogue; the server accepts
//!    it and returns Greenhorn, Scrapper, and Warlord with their fees, ten pits,
//!    and the authoritative 600-second pit limit.
//! 2. The player requests Svargrond status and observes the durable idle state.
//! 3. Before entering, the player sends Advance; the server refuses it with
//!    WrongState and leaves the complete arena state unchanged.
//! 4. The player sends Enter. The server must either apply the entry (charging
//!    player-earned carried money, moving the player into pit one, and spawning
//!    its opponent) or return an explicit refusal whose unchanged state is
//!    asserted. An environmental refusal is reported as the failed step, not
//!    converted into a pass.
//! 5. After an applied entry the player fights each server-spawned opponent with
//!    ordinary combat commands, observes the lethal/result deltas, and sends
//!    Advance after each kill; ten accepted advances end in the reward room.
//! 6. The player claims a reward, observes its custody delta, repeats the claim
//!    and expects AlreadyClaimed without another item, then Leaves successfully.
//! 7. The player requests Tibiadrome status, attempts Enter, and asserts either
//!    an applied arena state or the native eligibility refusal without mutation.
//! 8. The player logs out, obtains a fresh admission, requests both statuses,
//!    and re-checks completed rank, claim masks, and Tibiadrome progress.
//!
//! The dedicated fixture supplies the starting build, entry fee, and entrance position.
//! Combat, rank, and reward custody come from the ordinary player commands.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::arena::{
    self as wire, ArenaAction, ArenaDisposition, ArenaState, DromeAction, DromeResult,
};

struct Journey {
    started: std::time::Instant,
    completed: usize,
}

impl Journey {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            completed: 0,
        }
    }

    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) -> TestResult {
        if step != self.completed + 1 {
            return Err(format!("W17 step order invalid: expected {}, got {step}", self.completed + 1).into());
        }
        self.completed = step;
        evidence(&format!("W17_CHECK step={step} status=PASS observed={observed:?}"));
        Ok(())
    }

    fn finish(self, result: TestResult) {
        let status = if result.is_ok() && self.completed == 8 {
            "PASS"
        } else {
            "FAIL"
        };
        let detail = match result {
            Ok(()) => step_detail(self.completed),
            Err(error) => compact(&error.to_string()),
        };
        if status == "FAIL" {
            evidence(&format!("W17_GAP {detail}"));
        }
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W17.full status={status} seconds={:.3} evidence={detail}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn step_detail(completed: usize) -> String {
    if completed == 8 {
        "all-eight-player-steps-observed".to_owned()
    } else {
        format!("step-{}-not-observed", completed + 1)
    }
}

fn compact(detail: &str) -> String {
    detail
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut journey = Journey::new();
    let result = verdict::indexed_timeout(Duration::from_secs(330), run(&mut journey, clients))
        .await
        .unwrap_or_else(|_| Err(format!("step-{}-timeout waiting for native arena reply", journey.completed + 1).into()));
    journey.finish(result);
}

async fn command(
    client: &mut DevClientSession,
    action: ArenaAction,
) -> TestResult<wire::ArenaResult> {
    let payload = wire::encode_match_request(action)
        .map_err(|error| format!("arena request encoding: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, wire::COMMAND_TYPE_ARENA_MATCH, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native command {action:?} status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    wire::decode_match_result(&reply.payload)
        .map_err(|error| format!("native command {action:?} malformed reply={:?}: {error:?}", reply.payload).into())
}

async fn drome(
    client: &mut DevClientSession,
    action: DromeAction,
) -> TestResult<DromeResult> {
    let payload = wire::encode_match_request(ArenaAction::Drome(action))
        .map_err(|error| format!("Tibiadrome request encoding: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, wire::COMMAND_TYPE_ARENA_MATCH, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native Tibiadrome {action:?} status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    DromeResult::decode(&reply.payload)
        .map_err(|error| format!("native Tibiadrome {action:?} malformed reply={:?}: {error:?}", reply.payload).into())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step-8 logout observed={reply:?}").into());
    }
    Ok(())
}

async fn run(journey: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W17_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = connect_character(clients, 0xb7, 0).await?;
    if !client.selected_capabilities().contains(&wire::CAPABILITY_ARENA_V1) {
        return Err("step-1 selected-capabilities missing arena-v1".into());
    }

    let catalogue = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, wire::COMMAND_TYPE_ARENA_CATALOGUE, &[])
        .await?;
    if catalogue.status != CommandStatus::Accepted {
        return Err(format!("step-1 catalogue status={:?} payload={:?}", catalogue.status, catalogue.payload).into());
    }
    let challenges = wire::decode_catalogue(&catalogue.payload)
        .map_err(|error| format!("step-1 catalogue payload={:?} error={error:?}", catalogue.payload))?;
    if challenges != wire::SVARGROND_CHALLENGES {
        return Err(format!("step-1 catalogue delta mismatch observed={challenges:?}").into());
    }
    journey.pass(1, challenges)?;

    let initial = command(&mut client, ArenaAction::Status).await?;
    if initial.disposition != ArenaDisposition::Applied {
        return Err(format!("step-2 status observed={initial:?}").into());
    }
    journey.pass(2, initial)?;

    let refused = command(&mut client, ArenaAction::Advance).await?;
    if refused.disposition != ArenaDisposition::WrongState || refused.state != initial.state {
        return Err(format!("step-3 Advance expected WrongState with no delta observed={refused:?} before={initial:?}").into());
    }
    journey.pass(3, refused)?;

    let drome_before = drome(&mut client, DromeAction::Status).await?;
    let drome_enter = drome(&mut client, DromeAction::Enter).await?;
    if drome_enter != (DromeResult { disposition: 3, ..drome_before }) {
        return Err(format!(
            "step-7 Tibiadrome eligibility refusal mutated state before={drome_before:?} observed={drome_enter:?}"
        )
        .into());
    }

    let offsite = command(&mut client, ArenaAction::Enter).await?;
    if offsite.disposition != ArenaDisposition::WrongState || offsite.state != initial.state {
        return Err(format!("W17 off-site Enter mutated arena state: {offsite:?}").into());
    }

    character_prep::declare_seeded(
        "W17.full", "knight-level-100,1000-carried-gold,giant-sword,arena-entrance",
        "travel and build preparation exceed the row budget",
    );
    let preparation_session = client.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(preparation_session.as_bytes()), "--profile", "knight".into()).await?;
    character_prep::grant_item(&client, 3035, 10).await?;
    character_prep::grant_item(&client, 3281, 1).await?;
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    place_at_entrance(clients).await?;
    let mut client = connect_character(clients, 0xb8, 0).await?;
    if client.join_snapshot().game_session_id == preparation_session {
        return Err("W17 fixture admission reused its preparation session".into());
    }
    assert_position(&client, ENTRANCE)?;
    let vitals = client.actor_vitals().ok_or("W17 missing derived knight vitals")?;
    if vitals.max_health != 1565 || vitals.max_mana != 550 {
        return Err(format!("W17 level-100 knight vitals observed={vitals:?}").into());
    }
    let definitions = provisioning::item_index()?;
    let platinum = provisioning::item_reference(&definitions, 3035)?;
    let sword = provisioning::item_reference(&definitions, 3281)?;
    let trophy = provisioning::item_reference(&definitions, 5807)?;
    if count(&client, platinum)? != 10 || count(&client, trophy)? != 0 {
        return Err("W17 entry inputs or empty trophy pre-state differ".into());
    }
    let equipped = |client: &DevClientSession| client.inventory().is_some_and(|inventory| {
        inventory.equipment.iter().any(|slot| slot.item.item_definition_ref == sword)
    });
    if count(&client, sword)? != 1 || equipped(&client) {
        return Err("W17 giant sword was not carried unequipped before the move".into());
    }
    let gear = client.session_mut().equip_item_definition(sword).await?;
    if gear.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
        return Err(format!("W17 giant sword equip refused: {gear:?}").into());
    }
    let gear_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !equipped(&client) {
        if tokio::time::Instant::now() >= gear_deadline {
            return Err("W17 giant sword equip sent no confirming inventory frame".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};
    let modes = FightModes { fight_mode: FightMode::Offensive, chase: ChaseMode::Chase, secure: true };
    let fight_mode = client.session_mut().set_fight_modes(&modes).await?;
    if fight_mode.disposition != AttackIntentDisposition::Ok
        || client.session_mut().combat_state().is_none_or(|combat| combat.modes != modes)
    {
        return Err(format!("W17 offensive chase setup refused: {fight_mode:?}").into());
    }
    let ready = command(&mut client, ArenaAction::Status).await?;
    if ready != initial {
        return Err(format!("W17 fixture changed arena state: {ready:?}").into());
    }
    let entered = command(&mut client, ArenaAction::Enter).await?;
    check_pit(entered, 1)?;
    assert_position(&client, pit_position(1))?;
    if count(&client, platinum)? != 0 {
        return Err("W17 first entry did not consume exactly ten platinum coins".into());
    }
    journey.pass(4, entered)?;

    for pit in 1..=10 {
        defeat_opponent(&mut client, pit).await?;
        let victory = command(&mut client, ArenaAction::Status).await?;
        if victory.disposition != ArenaDisposition::Applied
            || (victory.state.active_rank, victory.state.pit, victory.state.phase) != (1, pit, 2)
        {
            return Err(format!("W17 pit {pit} kill not credited: {victory:?}").into());
        }
        let next = command(&mut client, ArenaAction::Advance).await?;
        if pit < 10 {
            check_pit(next, pit + 1)?;
            assert_position(&client, pit_position(pit + 1))?;
        } else {
            if next.disposition != ArenaDisposition::Applied
                || (next.state.completed_rank, next.state.active_rank, next.state.pit, next.state.phase)
                    != (1, 1, 0, 3)
            {
                return Err(format!("W17 final Advance did not complete Greenhorn: {next:?}").into());
            }
            assert_position(&client, REWARD_ROOM)?;
        }
        evidence(&format!("W17_PIT pit={pit} credited=true next={:?}", next.state));
        verdict::Harness::global().progress("W17 pit credited");
    }
    journey.pass(5, "ten credited Greenhorn opponents")?;

    let claim = ArenaAction::ClaimReward { rank: 1, category: 4, choice: 1 };
    let claimed = command(&mut client, claim).await?;
    if claimed.disposition != ArenaDisposition::Applied
        || claimed.state.claimed_masks != [8, 0, 0]
        || count(&client, trophy)? != 1
    {
        return Err(format!("W17 bronze goblet claim missing: {claimed:?}").into());
    }
    let repeated = command(&mut client, claim).await?;
    if repeated.disposition != ArenaDisposition::AlreadyClaimed
        || repeated.state != claimed.state || count(&client, trophy)? != 1
    {
        return Err(format!("W17 repeat trophy claim mutated custody: {repeated:?}").into());
    }
    let left = command(&mut client, ArenaAction::Leave).await?;
    if left.disposition != ArenaDisposition::Applied
        || left.state != (ArenaState { completed_rank: 1, claimed_masks: [8, 0, 0], ..ArenaState::default() })
    {
        return Err(format!("W17 Leave did not preserve completion and claim: {left:?}").into());
    }
    assert_position(&client, ENTRANCE)?;
    journey.pass(6, left)?;
    journey.pass(7, drome_enter)?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while client.session_mut().combat_state().is_none_or(|state| state.in_fight) {
        if tokio::time::Instant::now() >= deadline {
            return Err("W17 combat lock did not clear after leaving the arena".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xb9, 0).await?;
    let after = command(&mut fresh, ArenaAction::Status).await?;
    let drome_after = drome(&mut fresh, DromeAction::Status).await?;
    if fresh.join_snapshot().game_session_id == session || after != left
        || drome_after != drome_before || count(&fresh, trophy)? != 1 || count(&fresh, platinum)? != 0
    {
        return Err(format!("W17 relog lost arena progress, reward or fee: {after:?}, {drome_after:?}").into());
    }
    assert_position(&fresh, ENTRANCE)?;
    logout(&mut fresh).await?;
    journey.pass(8, after)
}

const ENTRANCE: (i32, i32, i32) = (32251, 31098, 6);
const REWARD_ROOM: (i32, i32, i32) = (32222, 31080, 6);

fn pit_position(pit: u8) -> (i32, i32, i32) {
    let (x, y) = oteryn_gameplay_durability::durability::arena_match::donor::CENTERS[usize::from(pit - 1)];
    (x, y, 7)
}

fn assert_position(client: &DevClientSession, expected: (i32, i32, i32)) -> TestResult {
    let actual = client.world_spatial().actor_position;
    let actual = (actual.x, actual.y, i32::from(actual.floor));
    if actual != expected {
        return Err(format!("W17 position expected={expected:?} observed={actual:?}").into());
    }
    Ok(())
}

async fn place_at_entrance(clients: &SeamClients<'_>) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let placed = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE character_id=$1::uuid AND world_id=$2::uuid",
    )
    .bind(uuid_text(&clients.characters[0]))
    .bind(uuid_text(&clients.world))
    .bind(ENTRANCE.0)
    .bind(ENTRANCE.1)
    .bind(i16::try_from(ENTRANCE.2)?)
    .execute(&mut database).await?;
    database.close().await?;
    if placed.rows_affected() != 1 {
        return Err("W17 fixture has no saved position to place".into());
    }
    evidence("FIXTURE row=W17.full account=q_w17 position=32251,31098,6");
    Ok(())
}

fn count(client: &DevClientSession, key: std::num::NonZeroU32) -> TestResult<u64> {
    let inventory = client.inventory().ok_or("W17 inventory not visible")?;
    Ok(inventory.entries.iter()
        .filter(|item| item.item_definition_ref == key)
        .map(|item| u64::from(item.count.get())).sum())
}

fn check_pit(reply: wire::ArenaResult, pit: u8) -> TestResult {
    if reply.disposition != ArenaDisposition::Applied
        || (reply.state.active_rank, reply.state.pit, reply.state.phase) != (1, pit, 1)
        || !(1..=600).contains(&reply.state.remaining_seconds)
    {
        return Err(format!("W17 pit {pit} entry did not apply: {reply:?}").into());
    }
    Ok(())
}

async fn defeat_opponent(client: &mut DevClientSession, pit: u8) -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    let (x, y, floor) = pit_position(pit);
    let entities = client.world_entities().ok_or("W17 missing entity view")?;
    let opponents: Vec<_> = entities.iter().filter(|entity| {
        entity.kind == EntityKind::Creature && entity.position.x == x
            && entity.position.y == y + 2 && i32::from(entity.position.floor) == floor
    }).collect();
    let [target] = opponents.as_slice() else {
        return Err(format!("W17 pit {pit} expected one opponent, observed={opponents:?}").into());
    };
    if !matches!(target.detail, EntityDetail::Actor { health_percent: 100, .. }) {
        return Err(format!("W17 pit {pit} opponent did not start at full health: {target:?}").into());
    }
    let id = target.entity;
    let attack = client.session_mut().attack_target(Some(&id)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("W17 pit {pit} attack refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("W17 pit {pit} opponent {id:?} did not die").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        if client.take_events().iter().any(|event| {
            matches!(event, oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta)
                if delta.value.leave.contains(&id))
        }) {
            break;
        }
    }
    if client.world_entities().is_none_or(|entities| entities.get(&id).is_some()) {
        return Err(format!("W17 pit {pit} opponent {id:?} still present").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_state_does_not_satisfy_arena_entry() {
        let reply = wire::ArenaResult { disposition: ArenaDisposition::WrongState, state: ArenaState::default() };
        assert!(check_pit(reply, 1).is_err());
    }

    #[test]
    fn journey_regression_names_the_first_unobserved_arena_step() {
        assert_eq!(step_detail(3), "step-4-not-observed");
        assert_eq!(step_detail(8), "all-eight-player-steps-observed");
    }

    #[test]
    fn compact_evidence_is_one_token() {
        assert_eq!(compact("step 4: WrongState [2]"), "step_4:_WrongState__2_");
    }

    #[test]
    fn completion_requires_every_numbered_step() -> TestResult {
        let mut journey = Journey::new();
        journey.pass(1, "catalogue")?;
        assert!(journey.pass(3, "advance").is_err());
        assert_eq!(journey.completed, 1);
        Ok(())
    }

    #[test]
    fn idle_state_is_not_an_enter_delta() {
        let state = ArenaState::default();
        assert_eq!(state.active_rank, 0);
        assert_ne!(state.phase, 1);
    }
}
