//! MP01: on the no-PvP World, three party members and a separately admitted, unprovisioned
//! victim; on the Open PvP World (its own node), a provisioned attacker kills a provisioned victim.
use super::observations::is_temple_arrival;
use super::starter;
use super::*;
use bots::{Bot, attack_target, party};
use oteryn_dev_client::{SessionEvent, client_ui::cyclopedia_character as panel};
use oteryn_gameplay_world::movement::interest::{VisibilityPosition, VisibilitySettings};
use oteryn_protocol_oteryn::{
    attack::FightMode, cyclopedia_character as character, party_skull, pvp_expert,
};

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
    towns: &oteryn_world_bundle::subarea::Table,
    profiles_confirmed: &mut bool,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut failures = Vec::new();
    let profile = std::env::var("NODE_BOOT_PROFILE").unwrap_or_else(|_| "sorcerer".into());
    let birth = if profile == "explorer" {
        "knight"
    } else {
        &profile
    };
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
    let profiled = ready
        && check(
            "profiles",
            bounded("profiles", 30, async {
                let (victim, party_members) = bots.split_last_mut().ok_or("MP01 victim missing")?;
                profiles(party_members, &["knight", "druid", "paladin"]).await?;
                starter::check_profile(victim, Some(birth)).await
            })
            .await,
            &mut failures,
        );
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
        check(
            "stop-combat",
            bounded("stop-combat", 20, attack_target(&mut bots[0], None)).await,
            &mut failures,
        );
        let result = bounded("relog", 1100, relog(&mut bots, joined, birth)).await;
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
        check(
            "relog",
            Err("blocked by admissions failure".into()),
            &mut failures,
        );
    }
    let released = check(
        "release",
        bounded("release", 1800, bots::release_all(&mut bots, core, packets)).await,
        &mut failures,
    );
    *profiles_confirmed = profiled && released;
    // The no-PvP bots are released; the Open PvP World runs next on its own node.
    pvp_world_rows(core, world, towns, &mut failures).await;
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

async fn relog(bots: &mut [Bot<'_, '_>], joined: bool, birth: &str) -> TestResult {
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
            if index == 3 {
                starter::check_profile(bot, Some(birth)).await?;
            } else {
                tab(bot, character::CharacterTab::GeneralStats).await?;
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

async fn profiles(bots: &mut [Bot<'_, '_>], vocations: &[&str]) -> TestResult {
    for (bot, &vocation) in bots.iter_mut().zip(vocations) {
        let view = tab(bot, character::CharacterTab::GeneralStats).await?;
        let expected =
            character::vocation_id(vocation).ok_or("MP01 vocation has no protocol id")?;
        if view.vocation_id != Some(expected)
            || view.general.as_ref().is_none_or(|stats| stats.level != 100)
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

pub(super) async fn tab(
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
    let payload = bots::party_skull_query(bot, &[8, 1]).await?;
    party_skull::PvpStatus::decode(&payload).map_err(|e| format!("PvP status decode:{e:?}").into())
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

fn expert_update(events: &[SessionEvent], after: u64) -> Option<pvp_expert::ExpertFightModes> {
    events.iter().find_map(|event| match event {
        SessionEvent::ExpertFightModes(delta) if delta.server_sequence > after => Some(delta.value),
        _ => None,
    })
}

async fn set_expert_modes(
    bot: &mut Bot<'_, '_>,
    requested: &pvp_expert::ExpertFightModes,
) -> TestResult {
    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, decode_attack_intent_result};

    let client = bot.client()?;
    let payload = pvp_expert::encode_expert_fight_modes(requested);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    tokio::time::timeout_at(deadline, async {
        loop {
            let reply = client
                .session_mut()
                .gated_command(
                    pvp_expert::CAPABILITY_PVP_EXPERT_V1,
                    pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
                    &payload,
                )
                .await?;
            if reply.status == CommandStatus::Rejected
                && decode_attack_intent_result(&reply.payload)
                    == Ok(AttackIntentDisposition::Rejected)
            {
                client.service_liveness(Duration::from_millis(50)).await?;
                continue;
            }
            if reply.status != CommandStatus::Accepted {
                return Err(format!("expert modes refused: {reply:?}").into());
            }
            let disposition = decode_attack_intent_result(&reply.payload)
                .map_err(|error| format!("expert mode decode:{error:?};reply={reply:?}"))?;
            if disposition != AttackIntentDisposition::Ok {
                return Err(format!("expert modes disposition={disposition:?}").into());
            }
            return Ok(());
        }
    })
    .await
    .map_err(|_| "expert modes exchange deadline")?
}

/// The node-boot World is no-PvP (`tools/qualification/node_boot/run.sh:1021`, `world_type=1`), so
/// the server pins every expert mode to Dove (`pvp_expert.rs:83-92` set, `:121-131` observe).
async fn nopvp_expert(bots: &mut [Bot<'_, '_>]) -> TestResult {
    let requested = pvp_expert::ExpertFightModes {
        modes: oteryn_protocol_oteryn::attack::FightModes {
            secure: false,
            ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
        },
        expert: pvp_expert::ExpertMode::RedFist,
    };
    let after = bots[0].client()?.last_server_sequence();
    set_expert_modes(&mut bots[0], &requested).await?;
    bots::wait_observation(bots, tokio::time::Instant::now() + Duration::from_secs(5), "expert fight mode update", |_, events| {
        if let Some(served) = expert_update(&events[0], after) {
            if served.expert != pvp_expert::ExpertMode::Dove {
                return Err(format!("no-PvP World must serve Dove;requested={requested:?};served={:?};source:crates/gameplay/dispatch/src/gameplay_transport/pvp_expert.rs:83-92;world:tools/qualification/node_boot/run.sh:1021", served).into());
            }
            return Ok(true);
        }
        Ok(false)
    }).await
}

fn refused_without_damage(
    status: CommandStatus,
    disposition: oteryn_protocol_oteryn::attack::AttackIntentDisposition,
    before: Option<u32>,
    after: Option<u32>,
    damaged: bool,
    hit_seen: bool,
) -> bool {
    status == CommandStatus::Rejected
        && disposition == oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected
        && before.is_some()
        && !damaged
        && !hit_seen
        && before == after
}

/// A's attack on player D is refused on a no-PvP World (`pvp_live.rs:184` via
/// `check_pvp_legality`, `legality.rs:90`) and D takes no damage in a bounded window.
async fn nopvp_attack_refused(bots: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult {
    let path = bots::route_bot(bots, 3, world, |at| {
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
    })?
    .ok_or("no reachable non-PZ combat pair in pinned world")?;
    bots::walk_bot(bots, 3, world, &path).await?;
    bots::approach(bots, 0, 3, world, true).await?;
    let attacker = bots[0].own()?;
    let victim = bots[3].own()?;
    let before = bots[3]
        .client()?
        .actor_vitals()
        .copied()
        .ok_or("D pre-attack vitals absent")?;
    let outcome = bots[0]
        .client()?
        .session_mut()
        .attack_target(Some(&victim))
        .await?;
    if outcome.status != CommandStatus::Rejected
        || outcome.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected
    {
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
    let after = bots[3]
        .client()?
        .actor_vitals()
        .copied()
        .ok_or("D post-attack vitals absent")?;
    if !refused_without_damage(
        outcome.status,
        outcome.disposition,
        Some(before.health),
        Some(after.health),
        damaged,
        hit_seen,
    ) {
        return Err(format!("D took damage on a no-PvP World;damaged={damaged};hit_seen={hit_seen};before={before:?};after={after:?}").into());
    }
    Ok(())
}

/// The PvP rows; each is PASS or FAIL, also when an earlier PvP step blocks it.
const PVP_ROWS: [&str; 6] = [
    "pvp-admissions",
    "pvp-profiles",
    "pvp-combat",
    "pvp-stop-combat",
    "pvp-relog",
    "pvp-release",
];
/// Both PvP Characters are fixture profiles at level 100, above `PVP_MIN_LEVEL` (`legality.rs:7`).
const PVP_VOCATIONS: [&str; 2] = ["knight", "sorcerer"];

/// The Open PvP World's serving node, Characters and scope (`run.sh` `pvp_world_ready`). One
/// node serves one World/Channel (`node/config.rs:81-83`), so this World has its own node.
struct PvpWorld {
    address: std::net::SocketAddr,
    url: String,
    accounts: [String; 2],
    characters: Vec<[u8; 16]>,
    world: [u8; 16],
    channel: [u8; 16],
    scope_generation: u64,
}

async fn pvp_world() -> TestResult<PvpWorld> {
    let url = required("NODE_BOOT_PVP_DATABASE_URL")?;
    let accounts = [
        required("NODE_BOOT_PVP_ACCOUNT_ID")?,
        required("NODE_BOOT_PVP_SECOND_ACCOUNT_ID")?,
    ];
    let world = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_WORLD_ID")?)
        .ok_or("invalid NODE_BOOT_PVP_WORLD_ID")?;
    let channel = crate::node::uuid_bytes(&required("NODE_BOOT_PVP_CHANNEL_ID")?)
        .ok_or("invalid NODE_BOOT_PVP_CHANNEL_ID")?;
    // Same bounded, read-only PgConnection pattern as the node-boot fixture read.
    let mut database = sqlx::PgConnection::connect(&url).await?;
    let world_type: i16 = sqlx::query_scalar("SELECT world_type FROM game_spell_field_world_policies WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision='policy-s3b-1'")
        .bind(world.as_slice()).fetch_one(&mut database).await?;
    if world_type != 2 {
        return Err(format!("PvP World policy world_type={world_type};expected=2 (Open PvP);source:tools/qualification/node_boot/run.sh:1291").into());
    }
    let mut characters = Vec::new();
    for account in &accounts {
        let found: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_character_roots WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2")
            .bind(account).bind(world.as_slice()).fetch_all(&mut database).await?;
        let [character] = found.as_slice() else {
            return Err(format!(
                "PvP World Account must own exactly one Character: account={account} count={}",
                found.len()
            )
            .into());
        };
        characters.push(<[u8; 16]>::try_from(character.as_slice())?);
    }
    let scope_generation: String = sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
        .bind(world.as_slice()).bind(channel.as_slice()).fetch_one(&mut database).await?;
    database.close().await?;
    Ok(PvpWorld {
        address: required("NODE_BOOT_PVP_ADDRESS")?.parse()?,
        url,
        accounts,
        characters,
        world,
        channel,
        scope_generation: scope_generation.parse()?,
    })
}

/// A (knight, index 0) kills B (sorcerer, index 1) on the Open PvP World. Platform trust, the
/// grant signer and the gameplay certificate are shared with the no-PvP World's pair.
async fn pvp_world_rows(
    core: &SeamClients<'_>,
    world: &BundleWorld,
    towns: &oteryn_world_bundle::subarea::Table,
    failures: &mut Vec<String>,
) {
    let fixture = match bounded("pvp-world", 60, pvp_world()).await {
        Ok(fixture) => fixture,
        Err(error) => {
            check("pvp-world", Err(error), failures);
            for step in PVP_ROWS {
                check(step, Err("blocked by pvp-world failure".into()), failures);
            }
            return;
        }
    };
    check("pvp-world", Ok(()), failures);
    let pair = SeamClients {
        address: fixture.address,
        accounts: &fixture.accounts,
        characters: &fixture.characters,
        world: fixture.world,
        channel: fixture.channel,
        scope_generation: fixture.scope_generation,
        url: &fixture.url,
        ..*core
    };
    let cells = (|| {
        let room = crate::content::qualify_selected_native_gameplay_room(
            crate::foundation::WorldId::decode(&pair.world)?,
            native_input()?,
        )?;
        TestResult::Ok(world.movement_cells(room.movement_cells())?)
    })();
    let (mut bots, setup) = tokio::time::timeout(
        Duration::from_secs(180),
        bots::connect_bots(&[
            (&pair, 0, Some(PVP_VOCATIONS[0])),
            (&pair, 1, Some(PVP_VOCATIONS[1])),
        ]),
    )
    .await
    .unwrap_or_else(|_| {
        (
            Vec::new(),
            Err("MP01 PvP admission/provision deadline".into()),
        )
    });
    if check("pvp-admissions", setup, failures) {
        check(
            "pvp-profiles",
            bounded("pvp-profiles", 30, profiles(&mut bots, &PVP_VOCATIONS)).await,
            failures,
        );
        check(
            "pvp-combat",
            bounded("pvp-combat", 480, async {
                pvp_journey(&mut bots, world, towns, &cells?).await
            })
            .await,
            failures,
        );
        check(
            "pvp-stop-combat",
            bounded("pvp-stop-combat", 20, attack_target(&mut bots[0], None)).await,
            failures,
        );
        check(
            "pvp-relog",
            bounded("pvp-relog", 1100, relog(&mut bots, false, PVP_VOCATIONS[1])).await,
            failures,
        );
    } else {
        for step in &PVP_ROWS[1..5] {
            check(
                step,
                Err("blocked by pvp-admissions failure".into()),
                failures,
            );
        }
    }
    // release_all waits for both indexes of two pairs; this World has one pair.
    check(
        "pvp-release",
        bounded(
            "pvp-release",
            1800,
            bots::release_all(&mut bots, &pair, &pair),
        )
        .await,
        failures,
    );
}

fn victim_death_seen(
    rows: &[panel::HistoryRow],
    old: &[panel::HistoryRow],
    at: i64,
    level: u32,
    killer: &str,
) -> bool {
    let description = character::death_cause(u64::from(level), &[killer.to_owned()]);
    let killer = character::bounded_death_name(killer);
    rows.iter().any(|row| {
        !old.contains(row)
            && row.at == at
            && row.description == description
            && row.death.as_ref().is_some_and(|death| {
                death.level == level
                    && death.killer == killer
                    && death.most_damage == killer
                    && death.unjustified
            })
    })
}

#[derive(PartialEq, Eq)]
struct DeathReceipt {
    id: String,
    original_revision: String,
    committed_revision: String,
    at: i64,
    level: u32,
}

async fn latest_death(bot: &Bot<'_, '_>) -> TestResult<Option<DeathReceipt>> {
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let receipt = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT death_occurrence_id::text, original_character_revision::text, \
         committed_character_revision::text, committed_at, level_before \
         FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid \
         AND death_world_id=encode($2,'hex')::uuid AND death_channel_id=encode($3,'hex')::uuid \
         ORDER BY committed_at DESC, death_occurrence_id DESC LIMIT 1",
    )
    .bind(bot.character.as_slice())
    .bind(bot.pair.world.as_slice())
    .bind(bot.pair.channel.as_slice())
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    receipt
        .map(|(id, original_revision, committed_revision, at, level)| {
            Ok(DeathReceipt {
                id,
                original_revision,
                committed_revision,
                at,
                level: u32::try_from(level)?,
            })
        })
        .transpose()
}

async fn remove_victim_equipment(bots: &mut [Bot<'_, '_>]) -> TestResult {
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    use oteryn_dev_client::{ItemMoveDestination, ItemMoveOutcome};

    let equipment = bots[1]
        .client()?
        .inventory()
        .ok_or("MP01 victim has no inventory view")?
        .equipment
        .clone();
    for entry in equipment {
        let moved = bots[1]
            .client()?
            .session_mut()
            .move_item_counted(MoveRequest {
                source: entry.item.handle,
                destination: ItemMoveDestination::MainBackpack,
                count: None,
            })
            .await?;
        if moved.outcome != ItemMoveOutcome::Moved {
            return Err(format!("MP01 victim could not stow {:?}: {moved:?}", entry.slot).into());
        }
    }
    bots::wait_observation(
        bots,
        tokio::time::Instant::now() + Duration::from_secs(5),
        "MP01 victim equipment is empty",
        |bots, _| {
            Ok(bots[1]
                .session
                .as_ref()
                .and_then(|client| client.inventory())
                .is_some_and(|inventory| inventory.equipment.is_empty()))
        },
    )
    .await
}

/// A's attack, B's damage, death and temple respawn, the durable death receipt, A's frag
/// points and white skull, checked in both clients and PostgreSQL.
async fn pvp_journey(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    towns: &oteryn_world_bundle::subarea::Table,
    cells: &crate::content::NativeEntryMovementCells,
) -> TestResult {
    // The durable PvP admission rule gives the attacker ten seconds after login.
    // Starting the clock after the joined profile is stronger than the server's
    // authorization-decision timestamp and requires no synthetic clock advance.
    let attack_ready_at = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut database = sqlx::PgConnection::connect(bots[1].pair.url).await?;
    let home: Option<i32> = sqlx::query_scalar(
        "SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(bots[1].character.as_slice())
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    let town = match home {
        Some(id) => u32::try_from(id)?,
        None => towns.default_town.ok_or("world default town absent")?,
    };
    let temple = towns
        .town(town)
        .and_then(|t| t.temple)
        .ok_or("victim hometown temple absent from pinned world")?;
    let temple = TilePos {
        x: temple.x,
        y: temple.y,
        floor: temple.floor,
    };
    let baseline = pvp_status(&mut bots[0]).await?;
    if baseline.skull != 0 || baseline.points_milli != [0; 3] {
        return Err(format!(
            "MP01 needs neutral A for a single unjustified kill oracle;observed={baseline:?}"
        )
        .into());
    }
    let old_death = latest_death(&bots[1]).await?;
    let old_kills = history(&mut bots[0], character::CharacterTab::RecentPvpKills).await?;
    let old_deaths = history(&mut bots[1], character::CharacterTab::RecentDeaths).await?;
    let axe = provisioning::item_reference(&provisioning::item_index()?, 3276)?;
    bots::equip_axe(bots, 0, axe).await?;
    // Direct live PvP currently qualifies an unarmoured target. Changing vocation
    // for the fixture keeps the birth equipment, so stow it through ordinary MOVE.
    remove_victim_equipment(bots).await?;
    let combat_tile = |at: TilePos| {
        cells
            .spell_tiles()
            .lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: i32::from(at.x),
                    y: i32::from(at.y),
                    z: -i32::from(at.floor),
                },
            )
            .is_ok_and(|facts| !facts.flags().protection_zone && facts.no_pvp_zone() == Some(false))
    };
    let attacker_start = tile(bots[0].client()?.world_spatial().actor_position)?;
    let directions = [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ];
    let path = bots::route_bot(bots, 1, world, |at| {
        at != temple
            && at != attacker_start
            && combat_tile(at)
            && directions.iter().any(|direction| {
                destination(world, at, *direction).is_some_and(|neighbor| {
                    neighbor.floor == at.floor
                        && neighbor.x.abs_diff(at.x) + neighbor.y.abs_diff(at.y) == 1
                        && combat_tile(neighbor)
                })
            })
    })?
    .ok_or("no reachable PvP-enabled combat pair in pinned world")?;
    bots::walk_bot(bots, 1, world, &path).await?;
    let victim_tile = tile(bots[1].client()?.world_spatial().actor_position)?;
    let path = bots::route_bot(bots, 0, world, |at| {
        at.floor == victim_tile.floor
            && at.x.abs_diff(victim_tile.x) + at.y.abs_diff(victim_tile.y) == 1
            && combat_tile(at)
    })?
    .ok_or("no reachable adjacent PvP-enabled attacker tile")?;
    bots::walk_bot(bots, 0, world, &path).await?;
    let attacker = bots[0].own()?;
    let victim = bots[1].own()?;
    let mode = pvp_expert::ExpertFightModes {
        modes: oteryn_protocol_oteryn::attack::FightModes {
            fight_mode: FightMode::Offensive,
            secure: false,
            ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
        },
        expert: pvp_expert::ExpertMode::RedFist,
    };
    let after = bots[0].client()?.last_server_sequence();
    set_expert_modes(&mut bots[0], &mode).await?;
    // An Open PvP World serves the requested Red Fist, not the no-PvP Dove.
    bots::wait_observation(bots, tokio::time::Instant::now() + Duration::from_secs(5), "expert fight mode update", |_, events| {
        if let Some(served) = expert_update(&events[0], after) {
            if served != mode {
                return Err(format!("expert modes requested={mode:?};served={:?};source:crates/gameplay/dispatch/src/gameplay_transport/pvp_expert.rs:1", served).into());
            }
            return Ok(true);
        }
        Ok(false)
    }).await?;
    if tokio::time::Instant::now() < attack_ready_at {
        super::observations::while_servicing_liveness(
            tokio::time::sleep_until(attack_ready_at),
            async || {
                bots::pump(bots).await?;
                Ok(())
            },
        )
        .await?;
    }
    let victim_client = bots[1].client()?;
    let temple_position = spatial_basis::current(victim_client)?.position(temple);
    let after = victim_client.last_server_sequence();
    victim_client.take_events();
    attack_target(&mut bots[0], Some(&victim)).await?;
    let mut temple_event = false;
    let respawn = bots::wait_observation(
        bots,
        tokio::time::Instant::now() + Duration::from_secs(360),
        "MP01 lethal hit and temple respawn",
        |bots, events| {
            for event in &events[1] {
                temple_event |= is_temple_arrival(event, after, temple_position);
            }
            let client = bots[1].session.as_ref().ok_or("victim closed")?;
            Ok(temple_event
                && client
                    .actor_vitals()
                    .is_some_and(|v| v.health > 0 && v.health == v.max_health))
        },
    )
    .await;
    if let Err(error) = respawn {
        let observed_target = bots[0]
            .session
            .as_mut()
            .and_then(|client| client.session_mut().combat_state()?.target);
        let observed_victim = bots[1].session.as_ref().map(|client| {
            (
                client.world_spatial().actor_position,
                client.actor_vitals().map(|v| (v.health, v.max_health)),
                client.last_server_sequence(),
            )
        });
        return Err(format!(
            "{error};expected_target={victim:?};observed_target={observed_target:?};temple={temple_position:?};temple_arrival={temple_event};victim_position_hp_sequence={observed_victim:?}"
        )
        .into());
    }
    attack_target(&mut bots[0], None).await?;
    let death = latest_death(&bots[1])
        .await?
        .ok_or("MP01 respawn missing durable death receipt")?;
    if Some(&death) == old_death.as_ref() || death.original_revision == death.committed_revision {
        return Err("MP01 respawn reused old death/revision".into());
    }
    let kills = history(&mut bots[0], character::CharacterTab::RecentPvpKills).await?;
    let description = character::pvp_kill_description(&bots[1].name);
    let deaths = history(&mut bots[1], character::CharacterTab::RecentDeaths).await?;
    if !kills.iter().any(|k| {
        k.description == description
            && k.status == Some(character::KillStatus::Unjustified)
            && !old_kills.contains(k)
    }) || !victim_death_seen(&deaths, &old_deaths, death.at, death.level, &bots[0].name)
    {
        return Err("MP01 A kill view / B death view lack new attributed death".into());
    }
    let mut database = sqlx::PgConnection::connect(bots[0].pair.url).await?;
    // The death owner ranks the final player first and classifies the actual lethal source.
    let points: Option<i32> = sqlx::query_scalar(
        "SELECT p.points_milli FROM game_character_unjustified_points p \
         JOIN game_public_death_classifications c USING (death_occurrence_id) \
         JOIN game_public_death_killers k USING (death_occurrence_id) \
         WHERE p.character_id=encode($1,'hex')::uuid AND p.victim_character_id=encode($2,'hex')::uuid \
         AND p.death_occurrence_id=$3::text::uuid AND c.pvp AND c.killer_key='player' \
         AND k.ordinal=0 AND k.kind='player' AND k.character_id=p.character_id",
    )
        .bind(bots[0].character.as_slice()).bind(bots[1].character.as_slice()).bind(&death.id).fetch_optional(&mut database).await?;
    database.close().await?;
    if points
        != Some(i32::from(
            oteryn_gameplay_combat_rules::pvp::skull::DAMAGE_POINTS_MILLI,
        ))
    {
        return Err(format!("MP01 durable frag points={points:?};death={}", death.id).into());
    }
    // The frag blocks the attacker from entering protection zones. Move only
    // the attacker along legal tiles into the victim's temple view.
    let temple_view =
        VisibilityPosition::new(temple.x.into(), temple.y.into(), temple.floor.into())?;
    let positions = bots::positions(bots)?;
    let path = route_avoiding(
        world,
        positions[0],
        |at| {
            VisibilityPosition::new(at.x.into(), at.y.into(), at.floor.into())
                .is_ok_and(|at| VisibilitySettings::REFERENCE.can_see(temple_view, at))
        },
        |at| {
            bots::tile_available(&positions, 0, at)
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| !t.protection_zone())
        },
    )
    .ok_or("no legal attacker route into the temple view")?;
    bots::walk_bot(bots, 0, world, &path).await?;
    bots::wait_observation(
        bots,
        tokio::time::Instant::now() + Duration::from_secs(15),
        "MP01 attacker visible to the victim at the temple",
        |bots, _| {
            Ok(bots[1]
                .session
                .as_ref()
                .and_then(|client| client.world_entities())
                .is_some_and(|entities| entities.get(&attacker).is_some()))
        },
    )
    .await?;
    for bot in bots.iter_mut() {
        let icons = bots::party_icons(bot).await?;
        if !icons
            .iter()
            .any(|icon| icon.entity == attacker && icon.skull == 3)
        {
            return Err(format!(
                "{} did not see A's computed white skull;icons={icons:?}",
                bot.name
            )
            .into());
        }
        evidence(&format!(
            "MP_CHECK MP01 attacker-skull-view PASS participant={} A_skull=white",
            bot.name
        ));
    }
    let victim_position = bots[1].client()?.world_spatial().actor_position;
    if victim_position != temple_position {
        return Err(format!("MP01 victim left temple during skull observation;expected={temple_position:?};observed={victim_position:?}").into());
    }
    let awarded = pvp_status(&mut bots[0]).await?;
    let increment = oteryn_gameplay_combat_rules::pvp::skull::contributor_points(
        oteryn_gameplay_combat_rules::pvp::skull::PvpContributorKind::Damage,
        1,
    );
    if awarded.points_milli != [u64::from(increment); 3] || awarded.skull != 3 {
        return Err(format!("MP01 single unjustified kill expected white skull and {increment} points;observed={awarded:?}").into());
    }
    for bot in bots {
        verify_pvp_state(bot).await?;
    }
    evidence(&format!(
        "MP_CHECK MP01 death-frag PASS A=attributed-hit-and-new-kill B=damage-death-and-temple-respawn PG_death={} PG_points={points:?}",
        death.id
    ));
    Ok(())
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;

    #[test]
    fn review_expert_observation_ignores_queued_mode() {
        let mode = |server_sequence, expert| {
            SessionEvent::ExpertFightModes(oteryn_dev_client::AppliedDelta {
                server_sequence,
                base_revision: 1,
                new_revision: 2,
                value: pvp_expert::ExpertFightModes {
                    modes: oteryn_protocol_oteryn::attack::FightModes::DEFAULT,
                    expert,
                },
            })
        };
        let stale = mode(8, pvp_expert::ExpertMode::Dove);
        assert!(expert_update(std::slice::from_ref(&stale), 8).is_none());
        let fresh = mode(9, pvp_expert::ExpertMode::RedFist);
        assert_eq!(
            expert_update(&[stale, fresh], 8).map(|update| update.expert),
            Some(pvp_expert::ExpertMode::RedFist)
        );
    }

    #[test]
    fn review_new_victim_death_must_match_its_receipt() {
        let expected = panel::HistoryRow {
            at: 9_000,
            status: None,
            description: "Died at Level 100 by Attacker".into(),
            death: Some(character::DeathDetails {
                level: 100,
                killer: "Attacker".into(),
                most_damage: "Attacker".into(),
                unjustified: true,
            }),
        };
        let mut wrong = expected.clone();
        if let Some(death) = wrong.death.as_mut() {
            death.killer = "Bystander".into();
        }
        assert!(!victim_death_seen(&[wrong], &[], 9_000, 100, "Attacker"));
        assert!(victim_death_seen(&[expected], &[], 9_000, 100, "Attacker"));
    }

    #[test]
    fn review_nopvp_requires_victim_vitals() {
        assert!(!refused_without_damage(
            CommandStatus::Rejected,
            AttackIntentDisposition::Rejected,
            None,
            None,
            false,
            false,
        ));
    }

    #[test]
    fn review_nopvp_requires_rejected_command_status() {
        assert!(!refused_without_damage(
            CommandStatus::Accepted,
            AttackIntentDisposition::Rejected,
            Some(100),
            Some(100),
            false,
            false,
        ));
    }
}
