//! Real admissions and protocol actions. The sole setup writer is root provision.py.
use super::diagnostics::{logout_failure_cause, logout_failure_detail, role_context};
use super::observations::while_servicing_liveness;
use super::*;
use std::collections::BTreeSet;

use oteryn_dev_client::client_runtime::party as party_client;
use oteryn_dev_client::{DevClientSession, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::{attack, logout::LogoutResult, party, target_move};

pub(super) struct Bot<'p, 'a> {
    pub pair: &'p SeamClients<'a>,
    pub index: usize,
    ordinal: usize,
    profile: Option<String>,
    pub session: Option<DevClientSession>,
    pub character: [u8; 16],
    pub name: String,
}

impl Bot<'_, '_> {
    fn context(&self, stage: &str) -> String {
        let role = role_context(
            self.ordinal,
            self.index,
            self.profile.as_deref(),
            self.pair.address,
            stage,
        );
        let (session, sequence) = self.session.as_ref().map_or_else(
            || ("none".to_owned(), 0),
            |client| {
                (
                    uuid_text(client.join_snapshot().game_session_id.as_bytes()),
                    client.last_server_sequence(),
                )
            },
        );
        format!(
            "{role} character={} session={session} observed_server_sequence={sequence}",
            uuid_text(&self.character)
        )
    }

    fn contextualize<T>(&self, stage: &str, result: TestResult<T>) -> TestResult<T> {
        result.map_err(|error| format!("{} error={error}", self.context(stage)).into())
    }

    fn progress(&self, stage: &str, outcome: &str) -> TestResult {
        let at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        evidence(&format!(
            "MP_BOT MP01 {} outcome={outcome} at_unix_ms={at_unix_ms}",
            self.context(stage)
        ));
        Ok(())
    }

    pub fn client(&mut self) -> TestResult<&mut DevClientSession> {
        self.session
            .as_mut()
            .ok_or_else(|| "bot session closed".into())
    }

    pub fn own(&self) -> TestResult<EntityRef> {
        let entities = self
            .session
            .as_ref()
            .and_then(DevClientSession::world_entities)
            .ok_or("bot entity snapshot absent")?;
        entities
            .iter()
            .find(|entity| &entity.entity.identity == entities.own_identity())
            .map(|entity| entity.entity)
            .ok_or_else(|| "bot own entity absent".into())
    }
}

/// Keep partial admissions in `bots` so the caller can release them even on setup failure.
pub(super) async fn connect_bots<'p, 'a>(
    roles: &[(&'p SeamClients<'a>, usize, Option<&str>)],
) -> (Vec<Bot<'p, 'a>>, TestResult) {
    let mut bots = Vec::new();
    let result = connect_into(roles, &mut bots).await;
    (bots, result)
}

async fn connect_into<'p, 'a>(
    roles: &[(&'p SeamClients<'a>, usize, Option<&str>)],
    bots: &mut Vec<Bot<'p, 'a>>,
) -> TestResult {
    for (ordinal, &(pair, index, profile)) in roles.iter().enumerate() {
        if bots.iter().any(|bot| {
            bot.character == pair.characters[index]
                || bot.pair.accounts[bot.index] == pair.accounts[index]
        }) {
            return Err("MP roles must own distinct Accounts and Characters".into());
        }
        bots.push(Bot {
            pair,
            index,
            ordinal,
            profile: profile.map(str::to_owned),
            session: None,
            character: pair.characters[index],
            name: String::new(),
        });
        let (admitted, current) = bots.split_at_mut(ordinal);
        let bot = current.first_mut().ok_or("admitted bot absent")?;
        with_liveness(
            admitted,
            "admissions-pump",
            connect_bot(bot, 0xa0 + u8::try_from(ordinal)?, "connect-initial"),
        )
        .await??;
        let session = bot.client()?.join_snapshot().game_session_id;
        if admitted
            .iter()
            .filter_map(|bot| bot.session.as_ref())
            .any(|old| old.join_snapshot().game_session_id == session)
        {
            return Err(format!(
                "{} error=MP roles reused a GameSession",
                bot.context("connect-initial")
            )
            .into());
        }
        // Provisioning (two relogs and a root subprocess) and the name read run long; the
        // admitted bots keep servicing liveness meanwhile.
        let result = with_liveness(admitted, "admissions-pump", async {
            if let Some(profile) = profile {
                provision_as(bot, profile).await?;
            }
            // Same bounded, read-only PgConnection pattern as qualification/death.rs.
            let result = async {
                let mut database = sqlx::PgConnection::connect(pair.url).await?;
                let name = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND account_id=$2::text::uuid AND world_id=encode($3,'hex')::uuid")
                    .bind(bot.character.as_slice()).bind(&pair.accounts[index]).bind(pair.world.as_slice())
                    .fetch_one(&mut database).await?;
                database.close().await?;
                TestResult::Ok(name)
            }.await;
            bot.name = bot.contextualize("read-character-name", result)?;
            TestResult::Ok(())
        })
        .await?;
        result?;
        pump_at(bots, "admissions-pump")
            .await
            .map_err(|error| format!("while_admitting_bot={ordinal} {error}"))?;
    }
    Ok(())
}

/// Service already-admitted peers throughout a long step, even when either side
/// is still reading a frame.
async fn with_liveness<T>(
    others: &mut [Bot<'_, '_>],
    stage: &str,
    step: impl std::future::Future<Output = T>,
) -> TestResult<T> {
    while_servicing_liveness(step, async || {
        verdict::Harness::global().progress("mp-admissions");
        pump_at(others, stage).await?;
        Ok(())
    })
    .await
}

async fn connect_bot(bot: &mut Bot<'_, '_>, tag: u8, stage: &str) -> TestResult {
    bot.progress(stage, "begin")?;
    let result = connect_character(bot.pair, tag, bot.index).await;
    bot.session = Some(bot.contextualize(stage, result)?);
    bot.progress(stage, "pass")?;
    Ok(())
}

pub(super) async fn logout_at(bot: &mut Bot<'_, '_>, stage: &str) -> TestResult {
    bot.progress(stage, "begin")?;
    let begin = bot
        .session
        .as_ref()
        .map_or(0, DevClientSession::last_server_sequence);
    // Drain what the server already sent: a close found here happened before the logout.
    let drained = async {
        let client = bot.client()?;
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
        TestResult::Ok(())
    }
    .await;
    let (before_send, result) = match drained {
        Err(error) => (true, Err(error)),
        Ok(()) => (false, logout(bot).await),
    };
    if let Err(error) = result {
        let error = error.to_string();
        let cause = logout_failure_cause(before_send, &error);
        // at_unix_ms and the observed sequence join this line to the server's serve_end.
        bot.progress(stage, &format!("fail cause={cause}"))?;
        let detail = logout_failure_detail(cause, begin, &error);
        let result = bot.contextualize(stage, Err(detail.into()));
        // A failed exchange poisons the protocol session. Drop its transport;
        // release_all still proves durable release for every Character below.
        bot.session = None;
        return result;
    }
    bot.progress(stage, "pass")?;
    Ok(())
}

/// Explicit profile; never mutates NODE_BOOT_PROFILE or provisions the no-vocation victim.
async fn provision_as(bot: &mut Bot<'_, '_>, profile: &str) -> TestResult {
    // Core's E2EF1 wrapper qualifies a reconnection, after a checked first-entry logout.
    // Match that lifecycle for this Character without running the unrelated movement rows.
    logout_at(bot, "logout-before-provision").await?;
    connect_bot(bot, 0xa6, "connect-for-provision").await?;
    bot.progress("provision", "begin")?;
    let result = provision_current(bot, profile).await;
    bot.contextualize("provision", result)?;
    bot.progress("provision", "pass")?;
    logout_at(bot, "logout-after-provision").await?;
    connect_bot(bot, 0xa4, "connect-profiled").await
}

/// Each node serves one World (`node/config.rs` `ScopeConfig`); the Open PvP World's node has
/// its own root-only control socket (`run.sh` `pvp_world_ready`).
fn control_socket(pair: &SeamClients<'_>) -> TestResult<String> {
    let pvp = std::env::var("NODE_BOOT_PVP_WORLD_ID")
        .ok()
        .and_then(|world| crate::node::uuid_bytes(&world));
    Ok(if pvp == Some(pair.world) {
        required("NODE_BOOT_PVP_CONTROL_SOCKET")?
    } else {
        required("NODE_BOOT_CONTROL_SOCKET")?
    })
}

async fn provision_current(bot: &mut Bot<'_, '_>, profile: &str) -> TestResult {
    let socket = control_socket(bot.pair)?;
    let session = uuid_text(bot.client()?.join_snapshot().game_session_id.as_bytes());
    let profile = profile.to_owned();
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("sudo")
            .args(["-n", "python3"])
            .arg(script)
            .args([
                "--socket",
                &socket,
                "--session",
                &session,
                "--profile",
                &profile,
            ])
            .output()
    })
    .await??;
    if !output.status.success() {
        let diagnostic =
            String::from_utf8_lossy(&output.stderr[output.stderr.len().saturating_sub(2048)..]);
        return Err(format!(
            "provision.py status={};diagnostic={diagnostic:?};source:tools/qualification/node_boot/provision.py:32",
            output.status,
        )
        .into());
    }
    Ok(())
}

pub(super) async fn pump(bots: &mut [Bot<'_, '_>]) -> TestResult<Vec<Vec<SessionEvent>>> {
    pump_at(bots, "liveness").await
}

async fn pump_at(bots: &mut [Bot<'_, '_>], stage: &str) -> TestResult<Vec<Vec<SessionEvent>>> {
    let mut events = Vec::with_capacity(bots.len());
    for bot in bots {
        let result = async {
            let client = bot.client()?;
            client.service_liveness(Duration::from_millis(20)).await?;
            TestResult::Ok(client.take_events())
        }
        .await;
        let result = bot.contextualize(stage, result);
        if result.is_err() {
            // Liveness errors poison the session just like failed command exchanges.
            // Preserve the diagnostic before dropping it; cleanup must prove durable release.
            bot.session = None;
        }
        events.push(result?);
    }
    Ok(events)
}

/// Every observation is bounded and services all participants, including idle witnesses.
pub(super) async fn wait_observation(
    bots: &mut [Bot<'_, '_>],
    deadline: tokio::time::Instant,
    detail: &str,
    mut observed: impl FnMut(&[Bot<'_, '_>], &[Vec<SessionEvent>]) -> TestResult<bool>,
) -> TestResult {
    verdict::Harness::global().bounded_wait(deadline, "MP-observation");
    tokio::time::timeout_at(deadline, async {
        loop {
            let events = pump(bots).await?;
            if observed(bots, &events)? {
                return Ok(());
            }
        }
    })
    .await
    .map_err(|_| format!("observation deadline:{detail}"))?
}

pub(super) async fn walk_bot(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        step_confirmed(bots, index, world, direction).await?;
        pump(bots).await?;
    }
    Ok(())
}

pub(super) fn positions(bots: &[Bot<'_, '_>]) -> TestResult<Vec<TilePos>> {
    bots.iter()
        .map(|bot| {
            let session = bot.session.as_ref().ok_or("MP route peer closed")?;
            tile(session.world_spatial().actor_position)
        })
        .collect()
}

pub(super) fn route_bot(
    bots: &[Bot<'_, '_>],
    index: usize,
    world: &BundleWorld,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult<Option<Vec<StepDirection>>> {
    let positions = positions(bots)?;
    Ok(route_avoiding(world, positions[index], goal, |next| {
        tile_available(&positions, index, next)
    }))
}

pub(super) fn tile_available(positions: &[TilePos], actor: usize, next: TilePos) -> bool {
    positions
        .iter()
        .enumerate()
        .all(|(index, &at)| index == actor || at != next)
}

struct GroupRoute {
    goal: TilePos,
    remaining: VecDeque<StepDirection>,
}

fn plan_round(
    routes: &mut [GroupRoute],
    positions: &[TilePos],
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> Vec<(usize, StepDirection)> {
    let mut priority = None;
    'round: loop {
        let mut reserved = BTreeSet::new();
        let mut steps = Vec::new();
        let order = priority
            .into_iter()
            .chain((0..routes.len()).filter(|&index| Some(index) != priority));
        for index in order {
            let route = &mut routes[index];
            let at = positions[index];
            if at == route.goal {
                continue;
            }
            let unoccupied = |at, direction| {
                landing(at, direction).filter(|&next| tile_available(positions, index, next))
            };
            let available =
                |at, direction| unoccupied(at, direction).filter(|next| !reserved.contains(next));
            if route
                .remaining
                .front()
                .and_then(|&direction| available(at, direction))
                .is_none()
            {
                let Ok(path) = find_route(at, |at| at == route.goal, available) else {
                    // Give a peer first pick if a reservation would cut off its only
                    // route. If the route is already occupied, the peer must wait.
                    if priority.is_none()
                        && !reserved.is_empty()
                        && find_route(at, |at| at == route.goal, unoccupied).is_ok()
                    {
                        priority = Some(index);
                        continue 'round;
                    }
                    continue;
                };
                route.remaining = path.into();
            }
            if let Some((direction, next)) = route
                .remaining
                .front()
                .and_then(|&direction| available(at, direction).map(|next| (direction, next)))
            {
                reserved.insert(next);
                steps.push((index, direction));
            }
        }
        for &(index, _) in &steps {
            routes[index].remaining.pop_front();
        }
        return steps;
    }
}

/// One confirmed step per bot per round, with a distinct unoccupied landing for each step.
pub(super) async fn walk_group(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    paths: Vec<(TilePos, Vec<StepDirection>)>,
) -> TestResult {
    // The original routes can be walked in separate rounds when peers must yield.
    // Replanning can spend this budget, but cannot replenish it.
    let mut rounds_left: usize = paths.iter().map(|(_, path)| path.len()).sum();
    let mut routes: Vec<_> = paths
        .into_iter()
        .map(|(goal, path)| GroupRoute {
            goal,
            remaining: path.into(),
        })
        .collect();
    let mut seen = BTreeSet::new();
    loop {
        let positions = positions(bots)?;
        if routes
            .iter()
            .zip(&positions)
            .all(|(route, &at)| route.goal == at)
        {
            return Ok(());
        }
        if rounds_left == 0 {
            return Err(format!(
                "MP group route exhausted its round budget;positions={positions:?}"
            )
            .into());
        }
        let remaining: Vec<Vec<u8>> = routes
            .iter()
            .map(|route| {
                route
                    .remaining
                    .iter()
                    .map(|&direction| direction as u8)
                    .collect()
            })
            .collect();
        if !seen.insert((positions.clone(), remaining)) {
            return Err(format!(
                "MP group route revisited positions and routes;positions={positions:?}"
            )
            .into());
        }
        let steps = plan_round(&mut routes, &positions, |at, direction| {
            destination(world, at, direction)
        });
        if steps.is_empty() {
            return Err(
                format!("MP group route has no unoccupied step;positions={positions:?}").into(),
            );
        }
        rounds_left -= 1;
        for (index, direction) in steps {
            step_confirmed(bots, index, world, direction).await?;
        }
        pump(bots).await?;
    }
}

async fn step_confirmed(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    world: &BundleWorld,
    direction: StepDirection,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let origin = tile(bots[index].client()?.world_spatial().actor_position)?;
    let landing = destination(world, origin, direction);
    let context = format!(
        "bot={index}:{};origin={origin:?};direction={direction:?};landing={landing:?}",
        bots[index].name,
    );
    let expected = landing.ok_or_else(|| format!("MP step has no world destination;{context}"))?;
    tokio::time::timeout_at(deadline, async {
        loop {
            let outcome = bots[index].client()?.step(direction).await?;
            let after = tile(bots[index].client()?.world_spatial().actor_position)?;
            if outcome.disposition == StepDisposition::Moved && after == expected {
                return TestResult::Ok(());
            }
            if outcome.disposition != StepDisposition::Rejected || after != origin {
                return Err(format!("MP movement refused;reply={outcome:?}").into());
            }
            pump(bots).await?;
        }
    })
    .await
    .map_err(|_| format!("MP movement deadline;{context}"))?
    .map_err(|error| {
        verdict::HarnessError::boxed(
            verdict::classify(error.as_ref()),
            format!("{context};error={error}"),
        )
    })?;
    // A server-confirmed tile change is real progress of the running row.
    verdict::Harness::global().progress("walk");
    Ok(())
}

pub(super) async fn approach(
    bots: &mut [Bot<'_, '_>],
    actor: usize,
    peer: usize,
    world: &BundleWorld,
    outside_pz: bool,
) -> TestResult {
    let other = tile(bots[peer].client()?.world_spatial().actor_position)?;
    let path = route_bot(bots, actor, world, |at| {
        at != other
            && at.floor == other.floor
            && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
            && (!outside_pz
                || world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| !t.protection_zone()))
    })?
    .ok_or("MP peer approach has no reachable tile")?;
    walk_bot(bots, actor, world, &path).await
}

pub(super) async fn party(
    bot: &mut Bot<'_, '_>,
    intent: Option<party::PartyIntent>,
) -> TestResult<party::PartyResult> {
    let command = match intent {
        Some(intent) => {
            party_client::PartyCommand::intent(intent).map_err(|e| format!("party input:{e:?}"))?
        }
        None => party_client::PartyCommand::query(),
    };
    let (command_id, reply) =
        party_client::send_with_receipt(bot.client()?.session_mut(), &command).await?;
    if reply.status != CommandStatus::Accepted {
        let operation = match intent {
            Some(party::PartyIntent::Invite(_)) => "Invite",
            Some(party::PartyIntent::Accept(_)) => "Accept",
            Some(party::PartyIntent::Leave) => "Leave",
            Some(party::PartyIntent::Kick(_)) => "Kick",
            Some(party::PartyIntent::PassLeadership(_)) => "PassLeadership",
            None => "Query",
        };
        return Err(format!(
            "party operation={operation};command_type={};command_id={command_id};status={:?}",
            command.kind, reply.status,
        )
        .into());
    }
    let result = party::decode_party_result(&reply.operation)
        .map_err(|e| format!("party result:{e:?};status={:?}", reply.status))?;
    if result.disposition != party::PartyDisposition::Ok {
        return Err(format!("party status={:?};disposition={:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_command.rs:105", reply.status, result.disposition).into());
    }
    party::decode_party_result(reply.view.as_deref().ok_or("party view absent")?)
        .map_err(|e| format!("party view:{e:?}").into())
}

pub(super) async fn invite_accept(
    bots: &mut [Bot<'_, '_>],
    leader: usize,
    peer: usize,
) -> TestResult {
    let target = bots[peer].own()?;
    party(&mut bots[leader], Some(party::PartyIntent::Invite(target))).await?;
    let invitations = party(&mut bots[peer], None).await?;
    let leader_name = bots[leader].name.as_str();
    let handle = invitations
        .invitations
        .iter()
        .find(|invite| invite.name == leader_name)
        .ok_or("invited bot did not observe leader invitation")?
        .handle;
    party(&mut bots[peer], Some(party::PartyIntent::Accept(handle))).await?;
    Ok(())
}

pub(super) async fn attack_target(bot: &mut Bot<'_, '_>, target: Option<&EntityRef>) -> TestResult {
    let outcome = bot.client()?.session_mut().attack_target(target).await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != attack::AttackIntentDisposition::Ok
    {
        return Err(format!("attack status={:?};disposition={:?};source:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/combat.rs:4;PvP-refusal:crates/gameplay/dispatch/src/gameplay_transport/pvp_live.rs:184;level-rule:crates/gameplay/combat-rules/src/pvp/legality.rs:97;reason-detail=not-carried-by-ATTACK_V1;reply={outcome:?}", outcome.status, outcome.disposition).into());
    }
    Ok(())
}

/// Busy uses the shared helper; real combat blocks expire on server time, never a clock poke.
async fn logout(bot: &mut Bot<'_, '_>) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(930);
    verdict::Harness::global().bounded_wait(deadline, "MP-combat-logout");
    let mut activity_due = tokio::time::Instant::now();
    let context = bot.context("logout-attempt");
    let mut observe = |attempt, detail: &str| {
        evidence(&format!("MP_BOT MP01 {context} attempt={attempt} {detail}"))
    };
    tokio::time::timeout_at(deadline, async {
        loop {
            let (result, reply, attempts, busy_timed_out) = logout_retrying_busy_observed(bot.client()?, Some(&mut observe)).await?;
            match result {
                Some(LogoutResult::Accepted) => { bot.session = None; return Ok(()); }
                Some(LogoutResult::InFight { .. }) => {
                    bot.client()?.service_liveness(Duration::from_millis(50)).await?;
                    bot.client()?.take_events();
                    if tokio::time::Instant::now() >= activity_due {
                        let payload = target_move::encode_target_move_intent(target_move::TargetMoveIntent::Turn(StepDirection::North)).map_err(|error| format!("MP Turn encode:{error:?}"))?;
                        let reply = bot.client()?.session_mut().gated_command(target_move::CAPABILITY_TARGET_MOVE_V1,
                            target_move::COMMAND_TYPE_TARGET_MOVE_INTENT, &payload).await?;
                        // A registered Turn refreshes idle activity even when gameplay refuses it.
                        if !matches!((reply.status, reply.payload.as_slice()),
                            (CommandStatus::Accepted, [8, 1]) | (CommandStatus::Rejected, [8, 3])) {
                            return Err(format!("MP lock-wait Turn refused:{reply:?};source:crates/protocol-oteryn/src/target_move.rs:1").into());
                        }
                        activity_due = tokio::time::Instant::now() + Duration::from_secs(30);
                    }
                }
                _ => return Err(format!("MP logout refused:{reply};attempts={attempts};busy_timed_out={busy_timed_out};source:apps/game-server/src/qualification/client_phases/world.rs:608").into()),
            }
        }
    }).await.map_err(|_| "MP logout combat release deadline")?
}

async fn logout_all(bots: &mut [Bot<'_, '_>]) -> TestResult {
    let mut failures = Vec::new();
    let mut pending = Vec::new();
    for bot in bots.iter_mut().filter(|bot| bot.session.is_some()) {
        pending.push(Box::pin(logout_at(bot, "logout-all")));
    }
    while !pending.is_empty() {
        let result = std::future::poll_fn(|cx| {
            for index in 0..pending.len() {
                if let std::task::Poll::Ready(result) = pending[index].as_mut().poll(cx) {
                    drop(pending.remove(index));
                    return std::task::Poll::Ready(result);
                }
            }
            std::task::Poll::Pending
        })
        .await;
        if let Err(error) = result {
            failures.push(error.to_string());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

pub(super) async fn relog_all(bots: &mut [Bot<'_, '_>]) -> TestResult {
    logout_all(bots).await?;
    // A logout reply only closes the client exchange. Complete every prior holder's
    // durable release before starting any new session's liveness clock.
    for bot in bots.iter() {
        let released = wait_character_released(bot.pair, bot.index).await;
        bot.contextualize("relog-terminal-release", released)?;
    }
    for index in 0..bots.len() {
        let (readmitted, current) = bots.split_at_mut(index);
        let bot = current.first_mut().ok_or("relog bot absent")?;
        with_liveness(
            readmitted,
            "relog-pump",
            connect_bot(bot, 0xa5, "connect-relog"),
        )
        .await??;
        // Service only the already re-admitted sessions; the rest are intentionally closed.
        pump_at(&mut bots[..=index], "relog-pump").await?;
    }
    Ok(())
}

pub(super) async fn release_all(
    bots: &mut [Bot<'_, '_>],
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
) -> TestResult {
    let mut failures = Vec::new();
    if let Err(error) = logout_all(bots).await {
        failures.push(error.to_string());
    }
    for bot in bots {
        bot.session = None;
    }
    for pair in [core, packets] {
        for index in 0..pair.accounts.len() {
            if let Err(error) = wait_character_released(pair, index).await {
                failures.push(error.to_string());
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;

    #[test]
    fn group_planner_leaves_the_junction_for_the_bot_that_must_pass_through() {
        let cell = |x, y| TilePos { x, y, floor: 7 };
        let junction = cell(1, 1);
        let north = cell(1, 0);
        let west = cell(0, 1);
        let east = cell(2, 1);
        let landing = |at: TilePos, direction: StepDirection| {
            let (dx, dy) = direction.delta();
            let next = cell(
                u16::try_from(i32::from(at.x) + dx).ok()?,
                u16::try_from(i32::from(at.y) + dy).ok()?,
            );
            ([junction, north, west, east].contains(&next) && (at.x == next.x || at.y == next.y))
                .then_some(next)
        };
        let mut routes = [
            GroupRoute {
                goal: junction,
                remaining: VecDeque::from([StepDirection::East]),
            },
            GroupRoute {
                goal: north,
                remaining: VecDeque::from([StepDirection::West, StepDirection::North]),
            },
        ];
        assert_eq!(
            plan_round(&mut routes, &[west, east], landing),
            [(1, StepDirection::West)]
        );
        assert_eq!(
            plan_round(&mut routes, &[west, junction], landing),
            [(1, StepDirection::North)]
        );
        assert_eq!(
            plan_round(&mut routes, &[west, north], landing),
            [(0, StepDirection::East)]
        );
    }

    #[test]
    fn group_planner_routes_both_bots_around_occupied_and_reserved_landings() -> TestResult {
        let cell = |x, y| TilePos { x, y, floor: 7 };
        let landing = |at: TilePos, direction| {
            let (x, y) = match direction {
                StepDirection::North => (at.x, at.y.checked_sub(1)?),
                StepDirection::East => (at.x.checked_add(1)?, at.y),
                StepDirection::South => (at.x, at.y.checked_add(1)?),
                StepDirection::West => (at.x.checked_sub(1)?, at.y),
                _ => return None,
            };
            (x <= 3 && y <= 2).then(|| cell(x, y))
        };
        let mut positions = [cell(0, 1), cell(1, 1)];
        let goals = [cell(3, 1), cell(0, 2)];
        let mut routes = [
            GroupRoute {
                goal: goals[0],
                remaining: VecDeque::from([StepDirection::East; 3]),
            },
            GroupRoute {
                goal: goals[1],
                remaining: VecDeque::from([StepDirection::South, StepDirection::West]),
            },
        ];
        // The old first step went east onto the second bot. The replanned steps must
        // leave both bots on distinct tiles all the way to their goals.
        for round in 0..8 {
            if positions == goals {
                break;
            }
            let mut next = positions;
            let mut reserved = BTreeSet::new();
            let steps = plan_round(&mut routes, &positions, landing);
            assert!(!steps.is_empty(), "stuck in round {round}: {positions:?}");
            for (actor, direction) in steps {
                let at = landing(positions[actor], direction).ok_or("missing landing")?;
                assert!(tile_available(&positions, actor, at), "occupied {at:?}");
                assert!(reserved.insert(at), "already reserved {at:?}");
                next[actor] = at;
            }
            positions = next;
        }
        assert_eq!(positions, goals);

        let positions = [cell(0, 0), cell(2, 0)];
        let mut routes = [
            GroupRoute {
                goal: cell(1, 0),
                remaining: VecDeque::from([StepDirection::East]),
            },
            GroupRoute {
                goal: cell(0, 1),
                remaining: VecDeque::from([
                    StepDirection::West,
                    StepDirection::West,
                    StepDirection::South,
                ]),
            },
        ];
        assert_eq!(
            plan_round(&mut routes, &positions, landing),
            [(0, StepDirection::East), (1, StepDirection::South)],
            "the second bot must route around the first bot's reserved landing"
        );
        Ok(())
    }
}
