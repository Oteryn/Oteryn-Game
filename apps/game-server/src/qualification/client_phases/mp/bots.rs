//! Real admissions and protocol actions. The sole setup writer is root provision.py.
use super::diagnostics::{logout_failure_cause, logout_failure_detail, role_context};
use super::*;
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
        let bot = bots.last_mut().ok_or("admitted bot absent")?;
        connect_bot(bot, 0xa0 + u8::try_from(ordinal)?, "connect-initial").await?;
        let session = bot.client()?.join_snapshot().game_session_id;
        if bots[..ordinal]
            .iter()
            .filter_map(|bot| bot.session.as_ref())
            .any(|old| old.join_snapshot().game_session_id == session)
        {
            return Err(format!(
                "{} error=MP roles reused a GameSession",
                bots[ordinal].context("connect-initial")
            )
            .into());
        }
        let bot = &mut bots[ordinal];
        if let Some(profile) = profile {
            provision_as(bot, profile).await?;
        }
        // Same bounded, read-only PgConnection pattern as qualification/death.rs.
        let result = async {
            let mut database = sqlx::PgConnection::connect(pair.url).await?;
            bot.name = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND account_id=$2::text::uuid AND world_id=encode($3,'hex')::uuid")
                .bind(bot.character.as_slice()).bind(&pair.accounts[index]).bind(pair.world.as_slice())
                .fetch_one(&mut database).await?;
            database.close().await?;
            TestResult::Ok(())
        }.await;
        bot.contextualize("read-character-name", result)?;
        pump_at(bots, "admissions-pump")
            .await
            .map_err(|error| format!("while_admitting_bot={ordinal} {error}"))?;
    }
    Ok(())
}

async fn connect_bot(bot: &mut Bot<'_, '_>, tag: u8, stage: &str) -> TestResult {
    bot.progress(stage, "begin")?;
    let result = connect_character(bot.pair, tag, bot.index).await;
    bot.session = Some(bot.contextualize(stage, result)?);
    bot.progress(stage, "pass")?;
    Ok(())
}

async fn logout_at(bot: &mut Bot<'_, '_>, stage: &str) -> TestResult {
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

async fn provision_current(bot: &mut Bot<'_, '_>, profile: &str) -> TestResult {
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
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
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let before = bots[index].client()?.world_spatial().actor_position;
        let expected = destination(world, tile(before)?, direction)
            .ok_or("MP step has no world destination")?;
        tokio::time::timeout_at(deadline, async {
            loop {
                let outcome = bots[index].client()?.step(direction).await?;
                let after = tile(bots[index].client()?.world_spatial().actor_position)?;
                if outcome.disposition == StepDisposition::Moved && after == expected { return TestResult::Ok(()); }
                if outcome.disposition != StepDisposition::Rejected || after != tile(before)? {
                    return Err(format!("MP movement refused:{outcome:?};source:apps/game-server/src/qualification/client_phases/world.rs:553").into());
                }
                pump(bots).await?;
            }
        }).await.map_err(|_| "MP movement deadline")??;
        pump(bots).await?;
    }
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
    let from = tile(bots[actor].client()?.world_spatial().actor_position)?;
    let occupied = bots
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != actor)
        .map(|(_, bot)| {
            tile(
                bot.session
                    .as_ref()
                    .ok_or("approach peer closed")?
                    .world_spatial()
                    .actor_position,
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    let path = route(world, from, |at| {
        at != other
            && !occupied.contains(&at)
            && at.floor == other.floor
            && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
            && (!outside_pz
                || world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| !t.protection_zone()))
    })
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
    let reply = party_client::send(bot.client()?.session_mut(), &command).await?;
    let result = party::decode_party_result(&reply.operation)
        .map_err(|e| format!("party result:{e:?};status={:?}", reply.status))?;
    if reply.status != CommandStatus::Accepted || result.disposition != party::PartyDisposition::Ok
    {
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
    for index in 0..bots.len() {
        let bot = &mut bots[index];
        connect_bot(bot, 0xa5, "connect-relog").await?;
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
