//! MP01: three party members and a separately admitted, unprovisioned victim.
use super::*;
use bots::{Bot, attack_target, party};
use oteryn_dev_client::{SessionEvent, client_ui::cyclopedia_character as panel};
use oteryn_protocol_oteryn::{cyclopedia_character as character, party_skull, pvp_expert};

fn check(step: &str, result: TestResult, failures: &mut Vec<String>) -> bool {
    match result {
        Ok(()) => {
            evidence(&format!("MP_CHECK MP01 {step} PASS verified"));
            true
        }
        Err(error) => {
            verdict::record_assertion(step);
            evidence(&format!(
                "MP_CHECK MP01 {step} FAIL {error};source:{}:{}",
                file!(),
                line!()
            ));
            failures.push(format!("{step}:{error}"));
            false
        }
    }
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
    _towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut failures = Vec::new();
    let (mut bots, setup) = tokio::time::timeout(
        Duration::from_secs(180),
        bots::connect_bots(&[
            (core, 0, Some("knight")),
            (packets, 0, Some("druid")),
            (packets, 1, Some("paladin")),
            (core, 1, None),
        ]),
    )
    .await
    .unwrap_or_else(|_| (Vec::new(), Err("MP01 admission/provision deadline".into())));
    // Keep the exact source error in the step rather than translating it into an incomplete row.
    let ready = check("admissions", setup, &mut failures);
    if ready {
        check(
            "profiles",
            bounded("profiles", 30, profiles(&mut bots)).await,
            &mut failures,
        );
    }
    let joined = if ready {
        check(
            "party",
            bounded("party", 120, party_journey(&mut bots, world)).await,
            &mut failures,
        )
    } else {
        check(
            "party",
            Err("blocked by admissions failure".into()),
            &mut failures,
        )
    };
    if ready {
        check(
            "nopvp-expert",
            bounded("nopvp-expert", 30, nopvp_expert(&mut bots)).await,
            &mut failures,
        );
        check(
            "nopvp-attack-refused",
            bounded(
                "nopvp-attack-refused",
                120,
                nopvp_attack_refused(&mut bots, world),
            )
            .await,
            &mut failures,
        );
        not_attempted("pvp-combat", NOPVP_REASON);
        check(
            "stop-combat",
            bounded("stop-combat", 20, attack_target(&mut bots[0], None)).await,
            &mut failures,
        );
        let result = bounded("relog", 1100, relog(&mut bots, joined)).await;
        check("relog", result, &mut failures);
    } else {
        check(
            "nopvp-expert",
            Err("blocked by admissions failure".into()),
            &mut failures,
        );
        check(
            "nopvp-attack-refused",
            Err("blocked by admissions failure".into()),
            &mut failures,
        );
        not_attempted("pvp-combat", NOPVP_REASON);
        check(
            "relog",
            Err("blocked by admissions failure".into()),
            &mut failures,
        );
    }
    check(
        "release",
        bounded("release", 1800, bots::release_all(&mut bots, core, packets)).await,
        &mut failures,
    );
    let status = if failures.is_empty() { "PASS" } else { "FAIL" };
    evidence(&format!(
        "e2e_all scenario=MP01.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

async fn relog(bots: &mut [Bot<'_, '_>], joined: bool) -> TestResult {
    let mut failures = Vec::new();
    let before = snapshots(bots).await;
    let sessions = bots
        .iter_mut()
        .map(|bot| {
            let client = bot.client()?;
            Ok((
                client.join_snapshot().game_session_id,
                client.world_spatial().actor_position,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    // Even an unsupported history tab must not prevent the four real relog attempts.
    bots::relog_all(bots).await?;
    for (bot, (previous, position)) in bots.iter_mut().zip(sessions) {
        let client = bot.client()?;
        if client.join_snapshot().game_session_id == previous
            || client.world_spatial().actor_position != position
        {
            failures.push(format!("{} relog session/position not persisted", bot.name));
        }
    }
    match (before, snapshots(bots).await) {
        (Ok(before), Ok(after)) => {
            if before != after {
                failures.push("MP01 relog lost death/kill history".into());
            }
        }
        (Err(error), _) | (_, Err(error)) => failures.push(error.to_string()),
    }
    // Terminal release removes membership: world_party.rs:792 and terminal_release.rs:88.
    for (index, bot) in bots.iter_mut().enumerate() {
        let observation: TestResult = async {
            if !party(bot, None).await?.members.is_empty() || !durable_party(bot).await?.is_empty() {
                return Err("terminal logout retained membership;source:crates/gameplay/durability/src/durability/world_party.rs:792".into());
            }
            verify_pvp_state(bot).await?;
            let view = tab(bot, character::CharacterTab::GeneralStats).await?;
            if index == 3 && view.vocation_id != character::vocation_id("none") {
                return Err("D lost the no-vocation path after relog".into());
            }
            Ok(())
        }.await;
        check("relog-view", observation, &mut failures);
    }
    if joined && failures.is_empty() {
        evidence(
            "MP_CHECK MP01 party-after-relog PASS terminal-release-removes-membership-in-all-four-views-and-PG",
        );
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

async fn bounded<T>(
    phase: &'static str,
    seconds: u64,
    action: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    verdict::Harness::global().bounded_wait(deadline, phase);
    tokio::time::timeout_at(deadline, action)
        .await
        .map_err(|_| format!("MP01 {phase} deadline;source:{}:{}", file!(), line!()))?
}

async fn profiles(bots: &mut [Bot<'_, '_>]) -> TestResult {
    for (bot, vocation) in bots.iter_mut().zip(["knight", "druid", "paladin", "none"]) {
        let view = tab(bot, character::CharacterTab::GeneralStats).await?;
        let expected =
            character::vocation_id(vocation).ok_or("MP01 vocation has no protocol id")?;
        if view.vocation_id != Some(expected)
            || view
                .general
                .as_ref()
                .is_none_or(|stats| vocation != "none" && stats.level != 100)
        {
            return Err(format!(
                "MP01 role={} expected={vocation} vocation_id={:?} stats={:?}",
                bot.name, view.vocation_id, view.general
            )
            .into());
        }
        let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
        let stored: String = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(bot.character.as_slice()).fetch_one(&mut database).await?;
        database.close().await?;
        if stored != vocation {
            return Err(format!(
                "MP01 role={} PG vocation={stored};expected={vocation}",
                bot.name
            )
            .into());
        }
        evidence(&format!(
            "MP_CHECK MP01 profile-view PASS participant={} vocation={vocation} level={:?} PG_vocation={stored}",
            bot.name,
            view.general.as_ref().map(|stats| stats.level)
        ));
    }
    Ok(())
}

async fn party_journey(bots: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult {
    for peer in [1, 2] {
        bots::approach(bots, 0, peer, world, false).await?;
        bots::invite_accept(bots, 0, peer).await?;
    }
    let mut expected = bots[..3]
        .iter()
        .map(|b| b.name.as_str())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    let expected = expected.into_iter().map(str::to_owned).collect::<Vec<_>>();
    let leader = bots[0].name.to_owned();
    let mut party_id = None;
    for bot in &mut bots[..3] {
        let view = party(bot, None).await?;
        let mut members = view
            .members
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>();
        members.sort_unstable();
        if members != expected
            || view.members.iter().filter(|m| m.leader).count() != 1
            || !view.members.iter().any(|m| m.leader && m.name == leader)
        {
            return Err(format!(
                "MP01 participant={} party view={view:?};expected={expected:?}",
                bot.name
            )
            .into());
        }
        let rows = durable_party(bot).await?;
        let mut names = rows.iter().map(|r| r.1.as_str()).collect::<Vec<_>>();
        names.sort_unstable();
        if names != expected
            || rows.iter().any(|r| r.2 != leader)
            || rows
                .iter()
                .any(|r| party_id.as_ref().is_some_and(|id| id != &r.0))
        {
            return Err(format!(
                "MP01 participant={} durable party differs:{rows:?}",
                bot.name
            )
            .into());
        }
        party_id = rows.first().map(|r| r.0.to_owned());
        evidence(&format!(
            "MP_CHECK MP01 party-view PASS participant={} members={members:?} PG={rows:?}",
            bot.name
        ));
    }
    if !party(&mut bots[3], None).await?.members.is_empty()
        || !durable_party(&bots[3]).await?.is_empty()
    {
        return Err("MP01 D unexpectedly belongs to the party".into());
    }
    Ok(())
}

/// Read-only qualification observation, following death.rs's bounded durable receipt reader.
/// No runtime authority is fabricated to call a production writer/read-under-write-lock port.
async fn durable_party(bot: &Bot<'_, '_>) -> TestResult<Vec<(String, String, String)>> {
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let rows = sqlx::query_as("SELECT p.party_id::text,r.name,l.name FROM game_party_members own JOIN game_parties p ON p.party_id=own.party_id JOIN game_party_members m ON m.party_id=p.party_id JOIN game_character_roots r ON r.character_id=m.character_id JOIN game_character_roots l ON l.character_id=p.leader_character_id WHERE own.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid ORDER BY m.seq LIMIT 50")
        .bind(bot.character.as_slice()).bind(bot.pair.world.as_slice()).fetch_all(&mut database).await?;
    database.close().await?;
    Ok(rows)
}

async fn tab(
    bot: &mut Bot<'_, '_>,
    tab: character::CharacterTab,
) -> TestResult<panel::CyclopediaCharacterPanel> {
    let client = bot.client()?;
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(client),
            death_details: client.selected_capabilities().contains(&488),
        },
        ..Default::default()
    };
    let command = panel::request(&mut view, panel::Intent::SelectTab(tab))
        .ok_or("MP01 character tab request absent")?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut view, &reply) {
        return Err(format!("character tab={tab:?};status={:?};disposition={:?};source:crates/client-ui/src/features/0199-cyclopedia-character/protocol.rs:1", reply.status, view.feedback).into());
    }
    Ok(view)
}

async fn history(
    bot: &mut Bot<'_, '_>,
    kind: character::CharacterTab,
) -> TestResult<Vec<panel::HistoryRow>> {
    Ok(tab(bot, kind).await?.history)
}

async fn snapshots(
    bots: &mut [Bot<'_, '_>],
) -> TestResult<Vec<(Vec<panel::HistoryRow>, Vec<panel::HistoryRow>)>> {
    let mut result = Vec::new();
    for bot in bots {
        result.push((
            history(bot, character::CharacterTab::RecentDeaths).await?,
            history(bot, character::CharacterTab::RecentPvpKills).await?,
        ));
    }
    Ok(result)
}

async fn pvp_status(bot: &mut Bot<'_, '_>) -> TestResult<party_skull::PvpStatus> {
    let reply = bot
        .client()?
        .session_mut()
        .gated_command(
            party_skull::CAPABILITY_PARTY_SKULL_V1,
            party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[8, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "PvP status={:?};source:crates/protocol-oteryn/src/party_skull/status.rs:1",
            reply.status
        )
        .into());
    }
    party_skull::PvpStatus::decode(&reply.payload)
        .map_err(|e| format!("PvP status decode:{e:?}").into())
}

async fn verify_pvp_state(bot: &mut Bot<'_, '_>) -> TestResult {
    use oteryn_gameplay_combat_rules::pvp::skull::{BLACK_THRESHOLDS_MILLI, RED_THRESHOLDS_MILLI};
    let view = pvp_status(bot).await?;
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let stored: Option<(i16, bool)> = sqlx::query_as("SELECT CASE WHEN skull_until>clock_timestamp() THEN skull ELSE 0::smallint END,COALESCE(white_skull_until>clock_timestamp(),false) FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(bot.character.as_slice()).bind(bot.pair.world.as_slice()).fetch_optional(&mut database).await?;
    let points: Vec<i64> = sqlx::query_scalar("SELECT COALESCE(sum(points_milli),0)::bigint FROM unnest(ARRAY[1,7,30]) days LEFT JOIN game_character_unjustified_points ON character_id=encode($1,'hex')::uuid AND committed_at>clock_timestamp()-days*interval '1 day' GROUP BY days ORDER BY days LIMIT 3")
        .bind(bot.character.as_slice()).fetch_all(&mut database).await?;
    database.close().await?;
    let (skull, white) = stored.unwrap_or((0, false));
    let expected = match skull {
        0 if white => 3,
        0 => 0,
        1 => 4,
        2 => 5,
        _ => return Err("invalid stored skull".into()),
    };
    if view.skull != expected
        || view.red_limits_milli != RED_THRESHOLDS_MILLI
        || view.black_limits_milli != BLACK_THRESHOLDS_MILLI
        || view
            .points_milli
            .iter()
            .copied()
            .map(i64::try_from)
            .collect::<Result<Vec<_>, _>>()?
            != points
    {
        return Err(format!(
            "participant={} PvP view={view:?};PG skull={stored:?};points={points:?}",
            bot.name
        )
        .into());
    }
    evidence(&format!(
        "MP_CHECK MP01 skull-view PASS participant={} skull={} points={points:?} PG=matched",
        bot.name, view.skull
    ));
    Ok(())
}

const NOPVP_REASON: &str = "node-boot world is NoPvp (run.sh:1010 world_type=1)";

/// A row that is neither PASS nor FAIL: the check is not attempted and cannot fail the scenario.
fn not_attempted(step: &str, reason: &str) {
    evidence(&format!(
        "MP_CHECK MP01 {step} NOT_ATTEMPTED not attempted: {reason}"
    ));
}

/// The node-boot World is no-PvP (`tools/qualification/node_boot/run.sh:1010`, `world_type=1`), so
/// the server pins every expert mode to Dove (`pvp_expert.rs:83-92` set, `:121-131` observe).
async fn nopvp_expert(bots: &mut [Bot<'_, '_>]) -> TestResult {
    let requested = pvp_expert::ExpertFightModes {
        modes: oteryn_protocol_oteryn::attack::FightModes {
            secure: false,
            ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
        },
        expert: pvp_expert::ExpertMode::RedFist,
    };
    let reply = bots[0]
        .client()?
        .session_mut()
        .gated_command(
            pvp_expert::CAPABILITY_PVP_EXPERT_V1,
            pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &pvp_expert::encode_expert_fight_modes(&requested),
        )
        .await?;
    let disposition = oteryn_protocol_oteryn::attack::decode_attack_intent_result(&reply.payload)
        .map_err(|error| format!("expert mode decode:{error:?};status={:?};source:crates/protocol-oteryn/src/attack.rs:300", reply.status))?;
    if reply.status != CommandStatus::Accepted
        || disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    {
        return Err(format!("expert modes status={:?};disposition={disposition:?};source:crates/gameplay/dispatch/src/gameplay_transport/pvp_expert.rs:83", reply.status).into());
    }
    bots::wait_observation(bots, tokio::time::Instant::now() + Duration::from_secs(5), "expert fight mode update", |_, events| {
        for event in &events[0] {
            if let SessionEvent::ExpertFightModes(delta) = event {
                if delta.value.expert != pvp_expert::ExpertMode::Dove {
                    return Err(format!("no-PvP World must serve Dove;requested={requested:?};served={:?};source:crates/gameplay/dispatch/src/gameplay_transport/pvp_expert.rs:83-92;world:tools/qualification/node_boot/run.sh:1010", delta.value).into());
                }
                return Ok(true);
            }
        }
        Ok(false)
    }).await
}

/// A's attack on player D is refused on a no-PvP World (`pvp_live.rs:184` via
/// `check_pvp_legality`, `legality.rs:90`) and D takes no damage in a bounded window.
async fn nopvp_attack_refused(bots: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult {
    let from = tile(bots[3].client()?.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .iter()
            .any(|d| {
                destination(world, at, *d).is_some_and(|p| {
                    world
                        .base()
                        .tile(p.x, p.y, p.floor)
                        .is_some_and(|t| !t.protection_zone())
                })
            })
    })
    .ok_or("no reachable non-PZ combat pair in pinned world")?;
    bots::walk_bot(bots, 3, world, &path).await?;
    bots::approach(bots, 0, 3, world, true).await?;
    let attacker = bots[0].own()?;
    let victim = bots[3].own()?;
    let before = bots[3].client()?.actor_vitals();
    let outcome = bots[0]
        .client()?
        .session_mut()
        .attack_target(Some(&victim))
        .await?;
    if outcome.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected {
        return Err(format!("attack on a player in a no-PvP World must be Rejected;status={:?};disposition={:?};source:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/combat.rs:62;pvp_live.rs:184;legality.rs:90", outcome.status, outcome.disposition).into());
    }
    let mut damaged = false;
    let mut hit_seen = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    verdict::Harness::global().bounded_wait(deadline, "MP01-nopvp-window");
    // Bounded no-damage window: pump until the deadline, which is the expected end.
    while tokio::time::Instant::now() < deadline {
        let events = bots::pump(bots).await?;
        for event in &events[0] {
            if let SessionEvent::Presentation(delta) = event {
                hit_seen |= delta.value.iter().any(|event| matches!(event.kind,
                    oteryn_protocol_oteryn::presentation::PresentationEventKind::ValueText { target, attacker: Some(source), .. }
                    if target == victim && source == attacker));
            }
        }
        for event in &events[3] {
            if let SessionEvent::ActorVitals(delta) = event {
                damaged |= delta.value.health < delta.value.max_health;
            }
        }
    }
    let after = bots[3].client()?.actor_vitals();
    if damaged || hit_seen || before.map(|v| v.health) != after.map(|v| v.health) {
        return Err(format!("D took damage on a no-PvP World;damaged={damaged};hit_seen={hit_seen};before={before:?};after={after:?}").into());
    }
    Ok(())
}
