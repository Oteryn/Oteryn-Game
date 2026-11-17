//! MP09. Oracle: Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data/events/scripts/party.lua:73-91 (three vocations: ceil(pool * 1.70 / 3));
//! base XP per target is the authored creature `experience` in content/creatures/definitions (see HUNT_TARGETS);
//! src/creatures/players/grouping/party.cpp:548-594 (level, distance, activity);
//! config.lua.dist:302 (share range multiplier 1.5, hence the 2/3 level rule).
use super::*;
use bots::{Bot, attack_target, party};
use oteryn_dev_client::{EntityKind, SessionEvent, client_ui::cyclopedia_character as panel};
use oteryn_protocol_oteryn::{
    cyclopedia_character as character,
    party::PartyIntent,
    party_skull,
    presentation::{PresentationEventKind, PresentationValueKind},
};

/// Hunt targets in preference order (key, shown name, base XP): the first whose authored spawn the
/// bots can walk to (no rope, shovel or ladder use) is hunted. The authored dragon spawns are not
/// walk-reachable from the temple (run 44062547883). Experience is the authored `experience` of
/// each creature record in content/creatures/definitions/creatures-*-*.json (one-line JSON, key
/// `oteryn:creature.<name>`): giant_spider 900 (-00500-00999), ancient_scarab 720 and
/// demon_skeleton 240 and bonelord 170 and cyclops 150 (-00000-00499), stone_golem 160 (-01500-01864).
const HUNT_TARGETS: [(&str, &str, u64); 6] = [
    ("oteryn:creature.giant_spider", "giant spider", 900),
    ("oteryn:creature.ancient_scarab", "ancient scarab", 720),
    ("oteryn:creature.demon_skeleton", "demon skeleton", 240),
    ("oteryn:creature.bonelord", "bonelord", 170),
    ("oteryn:creature.stone_golem", "stone golem", 160),
    ("oteryn:creature.cyclops", "cyclops", 150),
];

const XP_SOURCE: &str = "crates/gameplay/social-rules/src/shared_experience_split.rs:128 (server three-vocation bonus=60%; Crystal=70%)";

fn check(case: &str, result: TestResult<String>, failures: &mut Vec<String>) -> bool {
    match result {
        Ok(detail) => {
            evidence(&format!("MP_CHECK MP09 {case} PASS {detail}"));
            true
        }
        Err(error) => {
            let detail = if error.to_string().starts_with("dependent") {
                format!(
                    "{error};upstream_reply={}",
                    failures.first().map_or("no upstream reply", String::as_str)
                )
            } else {
                error.to_string()
            };
            verdict::record_assertion(case);
            evidence(&format!("MP_CHECK MP09 {case} FAIL {detail}"));
            failures.push(format!("{case}: {detail}"));
            false
        }
    }
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let started = std::time::Instant::now();
    let (mut bots, setup) = match profile_roles(core, packets).await {
        Ok(profiles) => {
            let (bots, setup) =
                bots::connect_bots(&[(core, 0, None), (packets, 0, None), (packets, 1, None)])
                    .await;
            (
                bots,
                setup.map(|()| format!("three distinct real sessions;{profiles}")),
            )
        }
        Err(error) => (Vec::new(), Err(error)),
    };
    let mut failures = Vec::new();
    let admitted = check("admissions", setup, &mut failures);
    let mut before = Vec::new();
    let prepared = if admitted {
        check("join-and-level-range", async {
            for (bot, vocation) in bots.iter_mut().zip(["knight", "druid", "paladin"]) {
                let (xp, level, observed) = stats(bot).await?;
                if observed != character::vocation_id(vocation) { return Err(format!("GeneralStats vocation={observed:?} expected={vocation} level={level} xp={xp}").into()); }
                before.push((xp, level));
            }
            let highest = before.iter().map(|(_, level)| *level).max().ok_or("no levels")?;
            if before.iter().any(|(_, level)| u64::from(*level) * 3 < u64::from(highest) * 2) { return Err(format!("GeneralStats={before:?};Crystal party.cpp:594 requires ceil(highest*2/3)").into()); }
            bots::approach(&mut bots, 1, 0, world, false).await?;
            bots::approach(&mut bots, 2, 0, world, false).await?;
            bots::invite_accept(&mut bots, 0, 1).await?;
            bots::invite_accept(&mut bots, 0, 2).await?;
            views(&mut bots, 0, 3).await?;
            Ok(format!("own party views A/B/C agree; GeneralStats={before:?};Crystal party.cpp:548-594"))
        }.await, &mut failures)
    } else {
        check(
            "join-and-level-range",
            Err("dependent on admissions; see concrete admissions reply above".into()),
            &mut failures,
        )
    };
    let enabled = if prepared {
        check("enable-shared-xp", async {
            let reply = bots[0].client()?.chat(&oteryn_dev_client::ChatIntent::Say { mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say, text: "/sharedxp on".into() }).await?;
            if reply.disposition != oteryn_dev_client::ChatDisposition::Ok { return Err(format!("reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_xp_command.rs:18").into()); }
            for bot in &mut bots { let view = party(bot, None).await?; if !view.shared_xp_enabled { return Err(format!("reply={view:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_command.rs:56").into()); } }
            Ok(format!("leader chat reply={reply:?};A/B/C shared_xp_enabled=true"))
        }.await, &mut failures)
    } else {
        check(
            "enable-shared-xp",
            Err("dependent on join-and-level-range; see prior reply".into()),
            &mut failures,
        )
    };
    let mut earned = None;
    if enabled {
        let encounter =
            tokio::time::timeout(Duration::from_secs(240), encounter(&mut bots, world)).await;
        let result = match encounter { Ok(result) => result, Err(_) => Err("encounter deadline;source:crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs:1905".into()) };
        match result {
            Ok((start, end, hits, pool)) => {
                earned = Some(end);
                check(
                    "all-contributed-kill",
                    Ok(format!(
                        "target disappeared;base_xp={pool};positive attributed damage={hits:?};XP before={start:?} after={end:?}"
                    )),
                    &mut failures,
                );
                for index in 0..3 {
                    let actual = end[index].checked_sub(start[index]);
                    let expected = crystal_share(pool);
                    let detail = format!(
                        "bot={index} actual={actual:?} expected={expected} base={pool} vocations=3 bonus=70% ceil({pool}*170/300);Crystal data/events/scripts/party.lua:73-91;source:{XP_SOURCE}"
                    );
                    check(
                        &format!("shared-xp-{index}"),
                        if actual == Some(expected) {
                            Ok(detail)
                        } else {
                            Err(detail.into())
                        },
                        &mut failures,
                    );
                }
            }
            Err(error) => {
                let cause = error.to_string();
                check("all-contributed-kill", Err(error), &mut failures);
                for index in 0..3 {
                    check(&format!("shared-xp-{index}"), Err(format!("expected=ceil(base*170/300) actual=unproven;encounter_reply={cause};source:{XP_SOURCE}").into()), &mut failures);
                }
            }
        }
    } else {
        for case in [
            "all-contributed-kill",
            "shared-xp-0",
            "shared-xp-1",
            "shared-xp-2",
        ] {
            check(
                case,
                Err("dependent on enable-shared-xp; see concrete prior reply".into()),
                &mut failures,
            );
        }
    }
    if admitted {
        for (index, bot) in bots.iter_mut().enumerate() {
            let result = attack_target(bot, None)
                .await
                .map(|()| "attack stopped".into());
            check(&format!("stop-combat-{index}"), result, &mut failures);
        }
    }
    if earned.is_some() {
        let result = bounded("distance-inactive", 180, out_of_range(&mut bots, world)).await;
        check("distance-inactive", result, &mut failures);
    } else {
        check(
            "distance-inactive",
            Err("dependent on shared-XP party; see prior reply".into()),
            &mut failures,
        );
    }
    if prepared {
        check(
            "leader-pass",
            async {
                let view = party(&mut bots[0], None).await?;
                let handle = view
                    .members
                    .iter()
                    .find(|m| m.name == bots[1].name)
                    .ok_or_else(|| format!("B handle absent;reply={view:?}"))?
                    .handle;
                let reply = party(&mut bots[0], Some(PartyIntent::PassLeadership(handle))).await?;
                views(&mut bots, 1, 3).await?;
                Ok(format!(
                    "reply={reply:?};all own views leader=B;Crystal party.cpp:233-264"
                ))
            }
            .await,
            &mut failures,
        );
        check(
            "leave",
            async {
                clear_fight(&mut bots).await?;
                let reply = party(&mut bots[2], Some(PartyIntent::Leave)).await?;
                if !reply.members.is_empty() {
                    return Err(format!("C leave reply={reply:?}").into());
                }
                views(&mut bots[..2], 1, 2).await?;
                Ok(format!("C reply={reply:?};A/B remain members"))
            }
            .await,
            &mut failures,
        );
    } else {
        for case in ["leader-pass", "leave"] {
            check(
                case,
                Err("dependent on party join; see prior reply".into()),
                &mut failures,
            );
        }
    }
    if admitted {
        check("relog-xp", async {
            let mut saved = Vec::new();
            for bot in &mut bots { saved.push((stats(bot).await?.0, bot.client()?.join_snapshot().game_session_id)); }
            bots::relog_all(&mut bots).await?;
            for (index, bot) in bots.iter_mut().enumerate() {
                let reply = stats(bot).await?;
                if reply.0 != saved[index].0 || bot.client()?.join_snapshot().game_session_id == saved[index].1 || earned.is_some_and(|xp| reply.0 != xp[index]) { return Err(format!("bot={index} GeneralStats={reply:?} before_relog={} earned={earned:?}", saved[index].0).into()); }
                let view = party(bot, None).await?;
                if !view.members.is_empty() { return Err(format!("terminal logout must leave party;reply={view:?};source:crates/gameplay/durability/src/durability/world_party.rs:792").into()); }
            }
            if earned.is_none() { return Err(format!("fresh sessions retained XP={saved:?};earned-XP persistence unproven because encounter failed;source:{XP_SOURCE}").into()); }
            Ok(format!("fresh sessions;XP retained={:?};earned={earned:?}", saved.iter().map(|s| s.0).collect::<Vec<_>>()))
        }.await, &mut failures);
    } else {
        check(
            "relog-xp",
            Err("dependent on admissions; see prior reply".into()),
            &mut failures,
        );
    }
    check(
        "release",
        bots::release_all(&mut bots, core, packets)
            .await
            .map(|()| "all four pair indexes released".into()),
        &mut failures,
    );
    evidence(&format!(
        "e2e_all scenario=MP09.full status={} seconds={:.3}",
        if failures.is_empty() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

/// MP01 normally profiles these roles, but an aborted MP01 admission leaves later roles
/// fresh. The fixture provisions only fresh Characters
/// (apps/game-server/src/qualification_spell_character.rs:234-269), so each role is
/// read from durable build state and a fresh one is provisioned with no other bot online: another
/// session's admission churn closed MP01's waiting leader
/// (crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs:412).
async fn profile_roles(core: &SeamClients<'_>, packets: &SeamClients<'_>) -> TestResult<String> {
    let mut reused = Vec::new();
    let mut provisioned = Vec::new();
    for (pair, index, profile) in [
        (core, 0, "knight"),
        (packets, 0, "druid"),
        (packets, 1, "paladin"),
    ] {
        let mut database = sqlx::PgConnection::connect(pair.url).await?;
        let stored: Option<String> = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(pair.characters[index].as_slice()).fetch_optional(&mut database).await?;
        database.close().await?;
        match stored.as_deref() {
            Some(stored) if stored == profile => {
                reused.push(profile);
                continue;
            }
            None | Some("none") => {}
            Some(stored) => return Err(format!("role={profile} durable vocation={stored};fixture provisions only fresh Characters;source:apps/game-server/src/qualification_spell_character.rs:234").into()),
        }
        let (mut alone, setup) = bots::connect_bots(&[(pair, index, Some(profile))]).await;
        let released = bots::release_all(&mut alone, core, packets).await;
        if let Err(error) = setup {
            return Err(
                format!("provision role={profile} error={error};release={released:?}").into(),
            );
        }
        released?;
        provisioned.push(profile);
    }
    Ok(format!(
        "profiles reused={reused:?} provisioned={provisioned:?}"
    ))
}

fn crystal_share(pool: u64) -> u64 {
    (pool * 170).div_ceil(300)
}

async fn bounded<T>(
    stage: &str,
    seconds: u64,
    future: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    verdict::Harness::global().bounded_wait(deadline, "MP09-phase");
    tokio::time::timeout_at(deadline, future)
        .await
        .map_err(|_| format!("MP09 {stage} deadline;source:{}:{}", file!(), line!()))?
}

async fn stats(bot: &mut Bot<'_, '_>) -> TestResult<(u64, u32, Option<u8>)> {
    tokio::time::timeout(Duration::from_secs(15), read_stats(bot)).await
        .map_err(|_| "GeneralStats/durable observation deadline;source:crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs:485")?
}

async fn read_stats(bot: &mut Bot<'_, '_>) -> TestResult<(u64, u32, Option<u8>)> {
    let client = bot.client()?;
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(client),
            death_details: false,
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(character::CharacterTab::GeneralStats),
    )
    .ok_or("GeneralStats request absent")?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut view, &reply) {
        return Err(format!("GeneralStats reply={reply:?}").into());
    }
    let stats = view
        .general
        .ok_or_else(|| format!("GeneralStats absent;reply={reply:?}"))?;
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let durable: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(bot.character.as_slice()).fetch_one(&mut database).await?;
    database.close().await?;
    if u64::try_from(durable)? != stats.experience {
        return Err(format!("GeneralStats reply={reply:?};durable_xp={durable};source:crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs:485").into());
    }
    Ok((stats.experience, stats.level, view.vocation_id))
}

async fn views(bots: &mut [Bot<'_, '_>], leader: usize, count: usize) -> TestResult {
    let name = bots[leader].name.clone();
    let names: Vec<_> = bots.iter().map(|b| b.name.clone()).collect();
    for bot in bots {
        let reply = party(bot, None).await?;
        let durable = tokio::time::timeout(Duration::from_secs(15), async {
            let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
            let rows: Vec<(String, String)> = sqlx::query_as("SELECT r.name,l.name FROM game_party_members own JOIN game_parties p ON p.party_id=own.party_id JOIN game_party_members m ON m.party_id=p.party_id JOIN game_character_roots r ON r.character_id=m.character_id JOIN game_character_roots l ON l.character_id=p.leader_character_id WHERE own.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid ORDER BY m.seq LIMIT 50")
                .bind(bot.character.as_slice()).bind(bot.pair.world.as_slice()).fetch_all(&mut database).await?;
            database.close().await?;
            TestResult::Ok(rows)
        }).await.map_err(|_| "durable party observation deadline")??;
        if durable.len() != count
            || durable
                .iter()
                .any(|(member, leader)| !names.contains(member) || leader != &name)
        {
            return Err(format!("own party reply={reply:?};durable_members_and_leaders={durable:?};expected_leader={name}").into());
        }
        if reply.members.len() != count
            || !reply.members.iter().any(|m| m.leader && m.name == name)
            || names
                .iter()
                .any(|name| !reply.members.iter().any(|m| &m.name == name))
        {
            return Err(format!("own party query participant={} reply={reply:?}", bot.name).into());
        }
    }
    Ok(())
}

async fn clear_fight(bots: &mut [Bot<'_, '_>]) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    verdict::Harness::global().bounded_wait(deadline, "MP09-natural-fight-expiry");
    loop {
        bots::pump(bots).await?;
        let mut states = Vec::new();
        for bot in bots.iter_mut() {
            let state = bot.client()?.session_mut().combat_state().copied()
                .ok_or("ActorCombatState reply absent;source:crates/gameplay/durability/src/durability/world_party/ingress.rs:110")?;
            states.push(state);
        }
        if states.iter().all(|state| !state.in_fight) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("natural fight expiry deadline;ActorCombatState replies={states:?};source:crates/gameplay/durability/src/durability/world_party/ingress.rs:110").into());
        }
    }
}

async fn icons(bot: &mut Bot<'_, '_>) -> TestResult<Vec<party_skull::PartyIcon>> {
    let reply = bot
        .client()?
        .session_mut()
        .gated_command(
            party_skull::CAPABILITY_PARTY_SKULL_V1,
            party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("shield reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:75").into());
    }
    party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("shield decode={e:?};reply={reply:?}").into())
}

async fn out_of_range(bots: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult<String> {
    let own = bots[0].own()?;
    let active = icons(&mut bots[0]).await?;
    if !active
        .iter()
        .any(|icon| icon.entity == own && icon.shield == 6)
    {
        return Err(format!("need active shared leader before distance transition;reply={active:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:27").into());
    }
    let leader = tile(bots[0].client()?.world_spatial().actor_position)?;
    let from = tile(bots[2].client()?.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        at.floor == leader.floor && at.x.abs_diff(leader.x).max(at.y.abs_diff(leader.y)) > 30
    })
    .ok_or("no walkable tile outside Crystal 30-tile range")?;
    bots::walk_bot(bots, 2, world, &path).await?;
    let reply = icons(&mut bots[0]).await?;
    if !reply
        .iter()
        .any(|icon| icon.entity == own && icon.shield == 10)
    {
        return Err(format!("inactive leader shield absent;reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:31;Crystal party.cpp:562").into());
    }
    let peer = bots[2].own()?;
    let peer_reply = icons(&mut bots[2]).await?;
    if !peer_reply
        .iter()
        .any(|icon| icon.entity == peer && icon.shield == 7)
    {
        return Err(format!("distant C own inactive shield absent;reply={peer_reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:28").into());
    }
    // Returning is ordinary movement, allowing subsequent leader/leave commands nearby.
    bots::approach(bots, 2, 0, world, false).await?;
    Ok(format!(
        "C walked beyond 30;A own inactive shield;reply={reply:?};Crystal party.cpp:562"
    ))
}

/// `route` limits its planning box to 160 tiles and two floors around the start; the authored dragon
/// spawns can lie beyond it, so this plans over the same production collision facts (`destination`)
/// across the whole map. Err carries every tile the search reached, for the failure message.
fn far_route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Result<Vec<StepDirection>, Vec<TilePos>> {
    let mut queue = std::collections::VecDeque::from([from]);
    let mut parents = std::collections::BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Ok(path);
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if parents.contains_key(&next) {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    Err(parents.into_keys().collect())
}

async fn encounter(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
) -> TestResult<([u64; 3], [u64; 3], [bool; 3], u64)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let from = tile(bots[0].client()?.world_spatial().actor_position)?;
    let mut tried = Vec::new();
    let mut chosen = None;
    for (key, name, xp) in HUNT_TARGETS {
        let spawns: Vec<TilePos> = metadata
            .spawns
            .sources
            .iter()
            .flat_map(|source| source.points.iter().map(move |p| (source.floor, p)))
            .filter(|(_, p)| {
                metadata
                    .spawns
                    .creatures
                    .get(p.creature as usize)
                    .is_some_and(|c| c.key == key)
            })
            .map(|(floor, p)| TilePos {
                x: p.x,
                y: p.y,
                floor,
            })
            .collect();
        let goal = |at: TilePos| {
            spawns
                .iter()
                .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 1)
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| !t.protection_zone())
        };
        match far_route(world, from, goal) {
            Ok(path) => {
                chosen = Some((name, xp, spawns, path));
                break;
            }
            Err(reached) => tried.push(format!(
                "{key}:spawns={},walkable_reached={}",
                spawns.len(),
                reached.len()
            )),
        }
    }
    let (name, pool, spawns, path) = chosen.ok_or_else(|| format!("no reachable authored hunt spawn;tried={tried:?};start={from:?};source:apps/game-server/src/qualification/client_phases/world.rs:441"))?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-route PASS target={name} base_xp={pool} spawns={} steps={} from={from:?}",
        spawns.len(),
        path.len()
    ));
    let leader = path
        .iter()
        .try_fold(from, |at, &direction| destination(world, at, direction))
        .ok_or("hunt route leaves the walkable world")?;
    let mut walks = vec![(0, path)];
    let mut parked = vec![leader];
    for actor in [1, 2] {
        let here = tile(bots[actor].client()?.world_spatial().actor_position)?;
        let near = |at: TilePos| {
            !parked.contains(&at)
                && at.floor == leader.floor
                && at.x.abs_diff(leader.x).max(at.y.abs_diff(leader.y)) <= 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| !t.protection_zone())
        };
        let path = far_route(world, here, near)
            .map_err(|reached| format!("MP peer approach has no reachable tile;walkable_reached={};start={here:?};leader={leader:?}", reached.len()))?;
        parked.push(
            path.iter()
                .try_fold(here, |at, &direction| destination(world, at, direction))
                .ok_or("peer route leaves the walkable world")?,
        );
        walks.push((actor, path));
    }
    // One lockstep walk: sequential walks of three ~200-step routes exceeded the 240 s encounter budget.
    bots::walk_group(bots, world, &walks).await?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-arrived PASS leader={leader:?} peers within 1 tile outside protection zone"
    ));
    let mut start = [0; 3];
    for (index, bot) in bots.iter_mut().enumerate() {
        start[index] = stats(bot).await?.0;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let mut target = None;
    bots::wait_observation(
        bots,
        deadline,
        "visible hunt target at authored spawn",
        |bots, _| {
            let client = bots[0].session.as_ref().ok_or("A closed")?;
            target = client.world_entities().and_then(|entities| {
                entities
                    .iter()
                    .find(|e| {
                        e.kind == EntityKind::Creature
                            && matches!(
                                e.detail,
                                oteryn_dev_client::EntityDetail::Actor {
                                    health_percent: 100,
                                    ..
                                }
                            )
                            && e.display_name.is_some_and(|label| {
                                label
                                    .as_str()
                                    .is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
                            })
                            && spawns.iter().any(|p| {
                                i16::from(p.floor) == e.position.floor
                                    && i32::from(p.x)
                                        .abs_diff(e.position.x)
                                        .max(i32::from(p.y).abs_diff(e.position.y))
                                        <= 1
                            })
                    })
                    .map(|e| e.entity)
            });
            Ok(target.is_some())
        },
    )
    .await?;
    let target = target.ok_or("hunt target entity absent")?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-target-found PASS name={name} entity={target:?}"
    ));
    let attackers = [bots[0].own()?, bots[1].own()?, bots[2].own()?];
    for bot in bots.iter_mut() {
        let modes = oteryn_protocol_oteryn::attack::FightModes {
            chase: oteryn_protocol_oteryn::attack::ChaseMode::Chase,
            ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
        };
        let reply = bot.client()?.session_mut().set_fight_modes(&modes).await?;
        if reply.status != CommandStatus::Accepted
            || reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        {
            return Err(format!("chase modes reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/attack.rs:1").into());
        }
        attack_target(bot, Some(&target)).await?;
    }
    evidence("MP_CHECK MP09 hunt-attack-started PASS all three bots attacking in chase mode");
    let mut hits = [false; 3];
    bots::wait_observation(bots, deadline, "all three damage the hunt target and kill", |bots, events| {
        for stream in events {
            for event in stream {
                let SessionEvent::Presentation(delta) = event else { continue };
                for event in &delta.value {
                    let PresentationEventKind::ValueText { kind, target: victim, attacker: Some(source), primary, .. } = event.kind else { continue };
                    if victim != target || primary.value == 0 || !matches!(kind, PresentationValueKind::DamageDealt | PresentationValueKind::DamageOthers) { continue; }
                    if !attackers.contains(&source) { return Err(format!("foreign damage on the hunt target reply={event:?};cannot assume the full {pool}-XP party pool").into()); }
                    for (index, attacker) in attackers.iter().enumerate() { hits[index] |= source == *attacker; }
                }
            }
        }
        let gone = bots[0].session.as_ref().and_then(|c| c.world_entities()).is_some_and(|e| e.get(&target).is_none());
        if gone && !hits.iter().all(|hit| *hit) { return Err(format!("hunt target removed before all contributions;attributed_damage={hits:?};source:crates/gameplay/dispatch/src/gameplay_transport/attack/shared_experience.rs:52").into()); }
        Ok(gone && hits.iter().all(|hit| *hit))
    }).await?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-kill PASS target gone;attributed_damage={hits:?}"
    ));
    for bot in bots.iter_mut() {
        attack_target(bot, None).await?;
    }
    let settled = tokio::time::Instant::now() + Duration::from_secs(20);
    let mut end = [0; 3];
    loop {
        for (index, bot) in bots.iter_mut().enumerate() {
            end[index] = stats(bot).await?.0;
        }
        if end.iter().zip(start).all(|(after, before)| *after > before) {
            break;
        }
        if tokio::time::Instant::now() >= settled {
            return Err(format!("XP settlement deadline;GeneralStats before={start:?} after={end:?};source:{XP_SOURCE}").into());
        }
        bots::pump(bots).await?;
    }
    Ok((start, end, hits, pool))
}
