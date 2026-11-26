//! MP09. Oracle: Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data/events/scripts/party.lua:73-91 (three vocations: ceil(pool * 1.70 / 3));
//! base XP per target is the authored creature `experience` in content/creatures/definitions (see HUNT_TARGET);
//! src/creatures/players/grouping/party.cpp:548-594 (level, distance, activity);
//! config.lua.dist:302 (share range multiplier 1.5, hence the 2/3 level rule).
use super::observations::{committed_swing, prepare_mp09_profiles};
use super::*;
use bots::{Bot, attack_target, party};
use oteryn_dev_client::{
    EntityDetail, EntityKind, EntityRef, SessionEvent, client_ui::cyclopedia_character as panel,
};
use oteryn_protocol_oteryn::{
    attack::{ChaseMode, FightMode, FightModes},
    cyclopedia_character as character,
    party::PartyIntent,
};

/// Hunt target (key, shown name, base XP), spawned next to the leader through provision.py --spawn. Experience is the
/// authored `experience` of the bonelord record (`oteryn:creature.bonelord`, creatures-*-00000-00499.json).
const HUNT_TARGET: (&str, &str, u64) = ("oteryn:creature.bonelord", "bonelord", 170);

// A low-armour hostile creature lets the level-one fixture register real party activity.
const ELIGIBILITY_TARGET: (&str, u64) = ("oteryn:creature.cave_rat", 10);
const LEVEL_PROBE: (&str, u64, u64) = ("oteryn:creature.crazed_beggar", 35, 100);

const XP_SOURCE: &str = "crates/gameplay/social-rules/src/shared_experience_split.rs:122";

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
    mp01_profiles_confirmed: bool,
) -> TestResult {
    let started = std::time::Instant::now();
    let profiles =
        prepare_mp09_profiles(mp01_profiles_confirmed, profile_roles(core, packets)).await;
    let (mut bots, setup) = match profiles {
        Ok(profiles) => {
            let (bots, setup) = bots::connect_bots(&[
                (core, 0, None),
                (packets, 0, None),
                (packets, 1, None),
                (core, 1, Some("knight-underlevel")),
            ])
            .await;
            (
                bots,
                setup.map(|()| {
                    format!("three eligible sessions and one underlevel session;{profiles}")
                }),
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
            // The MP profiles change vocation but keep the birth equipment. Equip the
            // granted melee weapon before asking these non-sorcerers to hunt in melee range.
            let axe = provisioning::item_reference(&provisioning::item_index()?, 3276)?;
            for index in 0..bots.len() {
                bots::equip_axe(&mut bots, index, axe).await?;
            }
            underlevel_party(&mut bots, world).await?;
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
    let mut activity_started = None;
    if enabled {
        let encounter =
            tokio::time::timeout(Duration::from_secs(240), encounter(&mut bots, world)).await;
        let result = match encounter { Ok(result) => result, Err(_) => Err("encounter deadline;source:crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs:1905".into()) };
        match result {
            Ok(HuntResult {
                start,
                end,
                hits,
                pool,
                activity_started_at_ms,
            }) => {
                earned = Some(end);
                activity_started = Some(activity_started_at_ms);
                check(
                    "all-contributed-kill",
                    Ok(format!(
                        "target disappeared;base_xp={pool};positive attributed damage={hits:?};XP before={start:?} after={end:?}"
                    )),
                    &mut failures,
                );
                for index in 0..3 {
                    let actual = end[index].checked_sub(start[index]);
                    let expected = native_three_vocation_share(pool);
                    let detail = format!(
                        "bot={index} actual={actual:?} expected={expected} base={pool} vocations=3 native_bonus=60% crystal_bonus=70%;source:{XP_SOURCE}"
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
                    check(&format!("shared-xp-{index}"), Err(format!("expected=ceil(base*160/300) actual=unproven;encounter_reply={cause};source:{XP_SOURCE}").into()), &mut failures);
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
    if let Some(started_at_ms) = activity_started {
        let result = out_of_range(&mut bots, world, started_at_ms).await;

        check(
            "distance-inactive",
            result.map(|(detail, after)| {
                earned = Some(after);
                detail
            }),
            &mut failures,
        );
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
            // An interrupted hunt can leave live creatures refreshing the combat
            // lock. Use ordinary movement to reach cells that admit logout.
            for index in 0..bots.len() {
                let path = bots::route_bot(&bots, index, world, |at| is_logout_rest_tile(world, at))?
                    .ok_or_else(|| format!("hunter {index} has no reachable logout protection tile"))?;
                bots::walk_bot_to_logout(&mut bots, index, world, &path).await?;
            }
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

/// If MP01 did not confirm its profiles, ensure the fixtures with one bot online at a time.
/// The fixture owner reuses its exact retained progression/build receipts when present.
async fn profile_roles(core: &SeamClients<'_>, packets: &SeamClients<'_>) -> TestResult<String> {
    let mut confirmed = Vec::new();
    for (pair, index, profile) in [
        (core, 0, "knight"),
        (packets, 0, "druid"),
        (packets, 1, "paladin"),
    ] {
        let (mut alone, setup) = bots::connect_bots(&[(pair, index, Some(profile))]).await;
        let released = bots::release_all(&mut alone, core, packets).await;
        if let Err(error) = setup {
            return Err(
                format!("provision role={profile} error={error};release={released:?}").into(),
            );
        }
        released?;
        confirmed.push(profile);
    }
    Ok(format!("profiles and items confirmed={confirmed:?}"))
}

fn native_three_vocation_share(pool: u64) -> u64 {
    (pool * 160).div_ceil(300)
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

async fn out_of_range(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    activity_started_at_ms: i64,
) -> TestResult<(String, [u64; 3])> {
    let own = bots[0].own()?;
    let active = bots::party_icons(&mut bots[0]).await?;
    if !active
        .iter()
        .any(|icon| icon.entity == own && icon.shield == 6)
    {
        return Err(format!("need active shared leader before distance transition;reply={active:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:27").into());
    }
    let leader = tile(bots[0].client()?.world_spatial().actor_position)?;
    let path = bots::route_bot(bots, 2, world, |at| {
        at.floor == leader.floor && at.x.abs_diff(leader.x).max(at.y.abs_diff(leader.y)) > 35
    })?
    .ok_or("no walkable tile outside Crystal 30-tile range")?;
    bots::walk_bot(bots, 2, world, &path).await?;
    let reply = bots::party_icons(&mut bots[0]).await?;
    if !reply
        .iter()
        .any(|icon| icon.entity == own && icon.shield == 10)
    {
        return Err(format!("inactive leader shield absent;reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:31;Crystal party.cpp:562").into());
    }
    let peer = bots[2].own()?;
    let peer_reply = bots::party_icons(&mut bots[2]).await?;
    if !peer_reply
        .iter()
        .any(|icon| icon.entity == peer && icon.shield == 7)
    {
        return Err(format!("distant C own inactive shield absent;reply={peer_reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:28").into());
    }
    let (award, after) = solo_hunt_award(bots, activity_started_at_ms).await?;
    Ok((
        format!(
            "C walked beyond 30;A own inactive shield;reply={reply:?};{award};Crystal party.cpp:562"
        ),
        after,
    ))
}

/// `route` limits its planning box to 160 tiles and two floors around the start; the authored dragon
/// spawns can lie beyond it, so this plans over the same production collision facts (`destination`)
/// across the whole map. Err carries every tile the search reached, for the failure message.
fn far_route(
    world: &BundleWorld,
    positions: &[TilePos],
    actor: usize,
    goal: impl Fn(TilePos) -> bool,
) -> Result<Vec<StepDirection>, usize> {
    find_route(positions[actor], goal, |at, direction| {
        destination(world, at, direction)
            .filter(|&next| bots::tile_available(positions, actor, next))
    })
}

async fn prepare_hunt_ground(bots: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult<TilePos> {
    let positions = bots::positions(bots)?;
    let from = positions[0];
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
    };
    // The spawn needs free open cells next to the leader (as creature_sweep's leave_protection_zone).
    let participants = bots.len();
    let roomy = |at: TilePos| {
        open(at)
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .filter(|direction| destination(world, at, *direction).is_some_and(open))
            .count()
                >= participants
    };
    let path = far_route(world, &positions, 0, |at| {
        roomy(at) && bots::tile_available(&positions, 0, at)
    })
    .map_err(|reached| format!("no open ground from {from:?};walkable_reached={reached}"))?;
    let leader = path
        .iter()
        .try_fold(from, |at, &direction| destination(world, at, direction))
        .ok_or("route to open ground leaves the walkable world")?;
    let mut walks = vec![(leader, path)];
    let mut parked = vec![leader];
    for (actor, &here) in positions.iter().enumerate().skip(1) {
        let near = |at: TilePos| {
            !parked.contains(&at)
                && bots::tile_available(&positions, actor, at)
                && at.floor == leader.floor
                && at.x.abs_diff(leader.x).max(at.y.abs_diff(leader.y)) <= 1
                && open(at)
        };
        let path = far_route(world, &positions, actor, near).map_err(|reached| {
            format!("MP peer approach has no reachable tile;walkable_reached={reached};start={here:?};leader={leader:?}")
        })?;
        let goal = path
            .iter()
            .try_fold(here, |at, &direction| destination(world, at, direction))
            .ok_or("peer route leaves the walkable world")?;
        parked.push(goal);
        walks.push((goal, path));
    }
    bots::walk_group(bots, world, walks).await?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-arrived PASS leader={leader:?} peers within 1 tile on open ground outside the protection zone"
    ));
    Ok(leader)
}

struct HuntResult {
    start: [u64; 3],
    end: [u64; 3],
    hits: [bool; 3],
    pool: u64,
    activity_started_at_ms: i64,
}

async fn encounter(bots: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult<HuntResult> {
    let leader = prepare_hunt_ground(bots, world).await?;
    let (key, name, pool) = HUNT_TARGET;
    let selection = client_daily_boost::reward_selection(bots[0].client()?).await?;
    let pool = client_daily_boost::boosted_xp(pool, key, &selection)?;
    let mut start = [0; 3];
    for (index, bot) in bots.iter_mut().enumerate() {
        start[index] = stats(bot).await?.0;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let session = uuid_text(bots[0].client()?.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, key.to_owned())
        .await
        .map_err(|reason| format!("hunt target spawn rejected:{reason};source:tools/qualification/node_boot/provision.py"))?;
    let target = parse_spawned_entity(&answer)?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-route PASS target={name} base_xp={pool} spawn_answer={answer} leader={leader:?}"
    ));
    bots::wait_observation(
        bots,
        deadline,
        "spawned hunt target visible to all members",
        |bots, _| {
            Ok(bots.iter().all(|bot| {
                bot.session
                    .as_ref()
                    .and_then(|client| client.world_entities())
                    .and_then(|entities| entities.get(&target))
                    .is_some_and(|entity| entity.kind == EntityKind::Creature)
            }))
        },
    )
    .await?;

    // Compare timestamps from the same database clock. Start before any hit so
    // the later award must commit while even the earliest contribution is fresh.
    let mut database = sqlx::PgConnection::connect(bots[0].pair.url).await?;
    let activity_started_at_ms =
        sqlx::query_scalar("SELECT floor(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(&mut database)
            .await?;
    database.close().await?;
    let mut hits = [false; 3];
    let mut corpse_identity = [0; 16];
    for index in 0..bots.len() {
        contribute(bots, index, target, deadline).await?;
        attack_target(&mut bots[index], None).await?;
        let (damage, corpse) = inspect_hunt_damage(bots, index, target).await?;
        if damage == 0 {
            return Err(format!("no committed damage from hunter {index} on {target:?}").into());
        }
        corpse_identity = corpse;
        hits[index] = true;
        evidence(&format!(
            "MP_CHECK MP09 hunt-contribution PASS participant={} target={target:?} damage={damage}",
            bots[index].name
        ));
    }

    bots::pump(bots).await?;
    attack_target(&mut bots[0], Some(&target)).await?;
    bots::wait_observation(bots, deadline, "exact hunt corpse", |bots, _| {
        let leader = bots[0].session.as_ref().ok_or("leader closed")?;
        Ok(leader.world_entities().is_some_and(|view| {
            view.get(&target).is_none()
                && view.iter().any(|entity| {
                    entity.kind == EntityKind::Corpse && entity.entity.identity == corpse_identity
                })
        }))
    })
    .await?;
    evidence(&format!(
        "MP_CHECK MP09 hunt-kill PASS committed kill;attributed_damage={hits:?}"
    ));
    for bot in bots.iter_mut() {
        attack_target(bot, None).await?;
    }
    let expected = bots
        .iter()
        .map(|bot| (bot.character, native_three_vocation_share(pool)))
        .collect();
    let death = observed_hunt_death(bots, None, key, target).await?;
    if !exact_award_matches(death.shared, true, &death.awards, &expected) {
        return Err(format!(
            "exact hunt receipt has shared={} awards={:?}; expected={expected:?}",
            death.shared, death.awards
        )
        .into());
    }
    let mut end = [0; 3];
    for (index, bot) in bots.iter_mut().enumerate() {
        end[index] = stats(bot).await?.0;
    }
    Ok(HuntResult {
        start,
        end,
        hits,
        pool,
        activity_started_at_ms,
    })
}

const HUNT_MODES: FightModes = FightModes {
    fight_mode: FightMode::Offensive,
    chase: ChaseMode::Chase,
    ..FightModes::DEFAULT
};

async fn set_hunt_modes(bot: &mut Bot<'_, '_>) -> TestResult {
    let client = bot.client()?;
    let reply = client.session_mut().set_fight_modes(&HUNT_MODES).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    {
        return Err(format!("hunt fight modes refused: {reply:?}").into());
    }
    Ok(())
}

async fn contribute(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    target: EntityRef,
    deadline: tokio::time::Instant,
) -> TestResult {
    set_hunt_modes(&mut bots[index]).await?;
    bots::pump(bots).await?;
    let client = bots[index].client()?;
    let after = client.last_server_sequence();
    let health_before = client
        .world_entities()
        .and_then(|view| view.get(&target))
        .and_then(|entity| hunt_health(entity.detail))
        .ok_or("spawned target health absent before swing")?;
    attack_target(&mut bots[index], Some(&target)).await?;
    let (mut selected, mut damaged) = (false, false);
    bots::wait_observation(bots, deadline, "member lands committed damage on the hunt target", |bots, events| {
            let client = bots[index].session.as_ref().ok_or("hunter closed")?;
            let target_view = client.world_entities().and_then(|view| view.get(&target))
                .ok_or_else(|| format!("spawned target left hunter {index}'s view during contribution"))?;
            let actor = client.world_spatial().actor_position;
            let in_range = actor.floor == target_view.position.floor
                && actor.x.abs_diff(target_view.position.x).max(actor.y.abs_diff(target_view.position.y)) <= 1;
            selected |= events[index].iter().any(|event| {
                matches!(event, SessionEvent::ActorCombatState(delta)
                    if delta.server_sequence > after && delta.value.target == Some(target) && delta.value.modes == HUNT_MODES)
            });
            damaged |= events[index].iter().any(|event| committed_swing(event, after));
            Ok(in_range && selected && damaged
                && hunt_health(target_view.detail).is_some_and(|health| health < health_before))
        }).await?;
    Ok(())
}

async fn inspect_hunt_damage(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    target: EntityRef,
) -> TestResult<(u64, [u8; 16])> {
    let session = uuid_text(
        bots[index]
            .client()?
            .join_snapshot()
            .game_session_id
            .as_bytes(),
    );
    let identity = target
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let request = request_fixture_control(
        session,
        "--inspect-damage",
        format!("{identity} {}", target.generation),
    );
    let answer = super::observations::while_servicing_liveness(request, async || {
        bots::pump(bots).await?;
        Ok(())
    })
    .await?
    .map_err(|error| format!("hunt damage inspection: {error}"))?;
    let mut fields = answer.split_ascii_whitespace();
    let (Some("damage"), Some(total), Some(corpse), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(format!("invalid hunt damage response: {answer}").into());
    };
    Ok((
        total.parse()?,
        crate::node::operator_files::hex_bytes::<16>(corpse)
            .map_err(|_| "invalid corpse identity")?,
    ))
}

async fn observed_hunt_death(
    bots: &mut [Bot<'_, '_>],
    principal: Option<[u8; 16]>,
    key: &str,
    target: EntityRef,
) -> TestResult<HuntDeath> {
    let pair = bots[0].pair;
    let (leader, peers) = bots.split_at_mut(1);
    let death = committed_hunt_death(
        leader[0].client()?,
        pair,
        principal,
        key,
        target,
        tokio::time::Instant::now() + Duration::from_secs(20),
    );
    super::observations::while_servicing_liveness(death, async || {
        bots::pump(peers).await?;
        Ok(())
    })
    .await?
}

async fn solo_hunt_award(
    bots: &mut [Bot<'_, '_>],
    activity_started_at_ms: i64,
) -> TestResult<(String, [u64; 3])> {
    let (key, pool) = ELIGIBILITY_TARGET;
    let selection = client_daily_boost::reward_selection(bots[0].client()?).await?;
    let pool = client_daily_boost::boosted_xp(pool, key, &selection)?;
    let mut start = Vec::new();
    for bot in bots.iter_mut() {
        start.push(stats(bot).await?.0);
    }
    let session = uuid_text(bots[0].client()?.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, key.to_owned()).await?;
    let target = parse_spawned_entity(&answer)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    bots::wait_observation(bots, deadline, "solo XP target visible", |bots, _| {
        Ok(bots[0]
            .session
            .as_ref()
            .and_then(|client| client.world_entities())
            .is_some_and(|view| view.get(&target).is_some()))
    })
    .await?;
    let (_, corpse) = inspect_hunt_damage(bots, 0, target).await?;
    set_hunt_modes(&mut bots[0]).await?;
    attack_target(&mut bots[0], Some(&target)).await?;
    bots::wait_observation(bots, deadline, "solo XP exact corpse", |bots, _| {
        Ok(bots[0]
            .session
            .as_ref()
            .and_then(|client| client.world_entities())
            .is_some_and(|view| {
                view.get(&target).is_none()
                    && view.iter().any(|entity| {
                        entity.kind == EntityKind::Corpse && entity.entity.identity == corpse
                    })
            }))
    })
    .await?;
    attack_target(&mut bots[0], None).await?;
    let leader = tile(bots[0].client()?.world_spatial().actor_position)?;
    let distant = tile(bots[2].client()?.world_spatial().actor_position)?;
    if distant
        .x
        .abs_diff(leader.x)
        .max(distant.y.abs_diff(leader.y))
        <= 30
    {
        return Err("solo XP creature died with C inside shared XP range".into());
    }
    let character = bots[0].character;
    let death = observed_hunt_death(bots, Some(character), key, target).await?;
    let activity_age_ms = death.committed_at_ms.saturating_sub(activity_started_at_ms);
    if !distance_activity_is_current(activity_age_ms)
        || !exact_award_matches(
            death.shared,
            false,
            &death.awards,
            &BTreeMap::from([(character, pool)]),
        )
    {
        return Err(format!(
            "out-of-range receipt did not prove a live-activity solo award;activity_age_ms={activity_age_ms};awards={:?}",
            death.awards
        )
        .into());
    }
    let mut end = Vec::new();
    // Observe all totals after the exact durable award committed.
    for bot in bots.iter_mut() {
        end.push(stats(bot).await?.0);
    }
    let expected = start[0].checked_add(pool).ok_or("solo XP overflow")?;
    if end[0] != expected || end[1..] != start[1..] {
        return Err(format!(
            "out-of-range hunt must give A all {pool} XP; before={start:?};after={end:?}"
        )
        .into());
    }
    let detail =
        format!("solo XP before={start:?};after={end:?};activity_age_ms={activity_age_ms}");
    Ok((
        detail,
        end.try_into()
            .map_err(|_| "expected three XP observations")?,
    ))
}

async fn underlevel_party(bots: &mut Vec<Bot<'_, '_>>, world: &BundleWorld) -> TestResult {
    let (_, leader_level, _) = stats(&mut bots[0]).await?;
    let (_, low_level, _) = stats(&mut bots[3]).await?;
    if !is_threshold_probe(leader_level, low_level) {
        return Err(format!(
            "underlevel probe is eligible: leader={leader_level};member={low_level}"
        )
        .into());
    }
    prepare_hunt_ground(bots, world).await?;
    bots::invite_accept(bots, 0, 3).await?;
    for index in [0, 3] {
        let view = party(&mut bots[index], None).await?;
        if view.members.len() != 2
            || !view
                .members
                .iter()
                .any(|member| member.name == bots[3].name)
        {
            return Err(format!("two-member underlevel party absent: {view:?}").into());
        }
    }
    let reply = bots[0]
        .client()?
        .chat(&oteryn_dev_client::ChatIntent::Say {
            mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
            text: "/sharedxp on".into(),
        })
        .await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_dev_client::ChatDisposition::Ok
    {
        return Err(format!("underlevel shared-XP toggle refused: {reply:?}").into());
    }
    let session = uuid_text(bots[3].client()?.join_snapshot().game_session_id.as_bytes());
    let selection = client_daily_boost::reward_selection(bots[0].client()?).await?;
    let pool = client_daily_boost::boosted_xp(LEVEL_PROBE.1, LEVEL_PROBE.0, &selection)?;
    let answer = request_spawn(session, LEVEL_PROBE.0.into()).await?;
    let target = parse_spawned_entity(&answer)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(100);
    bots::wait_observation(
        bots,
        deadline,
        "underlevel activity target visible",
        |bots, _| {
            Ok(bots[3]
                .session
                .as_ref()
                .and_then(|client| client.world_entities())
                .is_some_and(|view| view.get(&target).is_some()))
        },
    )
    .await?;
    contribute(bots, 3, target, deadline).await?;
    attack_target(&mut bots[3], None).await?;
    let (damage, corpse) = inspect_hunt_damage(bots, 3, target).await?;
    if damage == 0 {
        return Err("underlevel member did not exercise attack activity".into());
    }
    contribute(bots, 0, target, deadline).await?;
    attack_target(&mut bots[0], None).await?;
    let (leader_damage_before_finish, leader_corpse) = inspect_hunt_damage(bots, 0, target).await?;
    if leader_damage_before_finish == 0 || leader_corpse != corpse {
        return Err("underlevel probe lacked an active leader on the same target".into());
    }
    let own = bots[3].own()?;
    let view = bots::party_icons(&mut bots[3]).await?;
    let leader = tile(bots[0].client()?.world_spatial().actor_position)?;
    let low = tile(bots[3].client()?.world_spatial().actor_position)?;
    if low.floor != leader.floor
        || low.x.abs_diff(leader.x).max(low.y.abs_diff(leader.y)) > 30
        || !view
            .iter()
            .any(|entry| entry.entity == own && entry.shield == 7)
    {
        return Err(format!(
            "active, nearby underlevel member should be individually XP-ineligible: {view:?}"
        )
        .into());
    }
    // Finish the probe creature before returning the other party members to their normal journey.
    set_hunt_modes(&mut bots[0]).await?;
    attack_target(&mut bots[0], Some(&target)).await?;
    bots::wait_observation(bots, deadline, "underlevel activity corpse", |bots, _| {
        Ok(bots[0]
            .session
            .as_ref()
            .and_then(|client| client.world_entities())
            .is_some_and(|view| {
                view.iter().any(|entity| {
                    entity.kind == EntityKind::Corpse && entity.entity.identity == corpse
                })
            }))
    })
    .await?;
    attack_target(&mut bots[0], None).await?;
    let death = observed_hunt_death(bots, None, LEVEL_PROBE.0, target).await?;
    // The spawned Crazed Beggar has 100 HP and only its melee ability. The ledger credits
    // actual HP removed, including the capped lethal strike.
    let leader_damage = LEVEL_PROBE
        .2
        .checked_sub(damage)
        .filter(|remaining| *remaining > 0)
        .ok_or("underlevel participant exhausted the target's starting HP")?;
    let expected = [
        (bots[0].character, leader_damage),
        (bots[3].character, damage),
    ]
    .into_iter()
    .map(|(id, amount)| (id, pool * amount / LEVEL_PROBE.2))
    .filter(|(_, xp)| *xp > 0)
    .collect();
    if !exact_award_matches(death.shared, false, &death.awards, &expected) {
        return Err(format!(
            "underlevel receipt awards={:?}; expected damage shares={expected:?}",
            death.awards
        )
        .into());
    }
    clear_fight(bots).await?;
    party(&mut bots[3], Some(PartyIntent::Leave)).await?;
    let (members, low) = bots.split_at_mut(3);
    let low = low.first_mut().ok_or("missing underlevel bot")?;
    super::observations::while_servicing_liveness(
        bots::logout_at(low, "underlevel-release"),
        async || {
            bots::pump(members).await?;
            Ok(())
        },
    )
    .await??;
    bots.pop();
    Ok(())
}

fn hunt_health(detail: EntityDetail) -> Option<u8> {
    match detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => Some(health_percent),
        _ => None,
    }
}

fn exact_award_matches<K: Ord>(
    shared: bool,
    expected_shared: bool,
    awards: &BTreeMap<K, u64>,
    expected: &BTreeMap<K, u64>,
) -> bool {
    shared == expected_shared && awards == expected
}

fn distance_activity_is_current(age_ms: i64) -> bool {
    (0..120_000).contains(&age_ms)
}

fn is_threshold_probe(high: u32, low: u32) -> bool {
    let threshold = (u64::from(high) * 2).div_ceil(3);
    u64::from(low) + 1 == threshold && u64::from(low) * 2 >= u64::from(high)
}

#[cfg(test)]
mod qualification_review_tests {
    use super::*;

    #[test]
    fn delta_mp09_requires_exact_shared_and_solo_receipt_members() {
        let expected = BTreeMap::from([(1, 91), (2, 91), (3, 91)]);
        assert!(!exact_award_matches(
            true,
            true,
            &BTreeMap::new(),
            &expected
        ));
        assert!(!exact_award_matches(
            true,
            true,
            &BTreeMap::from([(1, 91), (2, 91), (4, 91)]),
            &expected
        ));
        assert!(!exact_award_matches(
            false,
            false,
            &BTreeMap::from([(1, 0)]),
            &BTreeMap::from([(1, 10)])
        ));
        assert!(exact_award_matches(true, true, &expected, &expected));
    }

    #[test]
    fn delta_mp09_distance_probe_requires_unexpired_contribution() {
        assert!(!distance_activity_is_current(-1));
        assert!(distance_activity_is_current(119_999));
        assert!(!distance_activity_is_current(120_000));
    }

    #[test]
    fn delta_mp09_threshold_probe_distinguishes_half_from_two_thirds() {
        assert!(!is_threshold_probe(100, 1));
        assert!(is_threshold_probe(100, 66));
        assert!(!is_threshold_probe(100, 67));
    }

    #[test]
    fn qualification_mp09_rejects_foreign_damage_and_foreign_corpses() -> TestResult {
        let source = include_str!("mp09_party_xp.rs");
        let (_, encounter) = source
            .split_once("async fn encounter(")
            .ok_or("missing hunt")?;
        let (encounter, _) = encounter
            .split_once("#[cfg(test)]")
            .ok_or("missing hunt boundary")?;
        assert!(encounter.contains("inspect_hunt_damage("));
        assert!(encounter.contains("corpse_identity"));
        Ok(())
    }

    #[test]
    fn qualification_mp09_distance_measures_a_real_award() -> TestResult {
        let source = include_str!("mp09_party_xp.rs");
        let (_, distance) = source
            .split_once("async fn out_of_range(")
            .ok_or("missing distance row")?;
        let (distance, _) = distance
            .split_once("fn whole_map_route(")
            .or_else(|| distance.split_once("async fn encounter("))
            .ok_or("missing distance boundary")?;
        assert!(distance.contains("solo_hunt_award("));
        Ok(())
    }

    #[test]
    fn qualification_mp09_checks_the_underlevel_party_on_dispatch() -> TestResult {
        let source = include_str!("mp09_party_xp.rs");
        let (_, run) = source
            .split_once("async fn run(")
            .ok_or("missing MP09 run")?;
        let (run, _) = run
            .split_once("async fn profile_roles(")
            .ok_or("missing MP09 boundary")?;
        assert!(run.contains("underlevel_party("));
        Ok(())
    }
}
