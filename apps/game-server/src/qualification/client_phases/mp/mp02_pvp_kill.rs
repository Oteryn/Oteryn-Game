//! MP02: knight versus paladin through their own client sessions.
use super::*;
use bots::{Bot, attack_target, party};
use oteryn_dev_client::{SessionEvent, client_ui::cyclopedia_character as panel};
use oteryn_protocol_oteryn::{cyclopedia_character as character, party_skull, pvp_expert};

fn check(step: &str, result: TestResult, failures: &mut Vec<String>) -> bool {
    match result {
        Ok(()) => {
            evidence(&format!("MP_CHECK MP02 {step} PASS verified"));
            true
        }
        Err(error) => {
            verdict::record_assertion(step);
            evidence(&format!(
                "MP_CHECK MP02 {step} FAIL {error};source:{}:{}",
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
) -> TestResult {
    let started = std::time::Instant::now();
    let mut failures = Vec::new();
    let (mut bots, setup) = tokio::time::timeout(
        Duration::from_secs(180),
        bots::connect_bots(&[
            (core, 0, Some("knight")),
            (packets, 1, Some("paladin")),
            
        ]),
    )
    .await
    .unwrap_or_else(|_| (Vec::new(), Err("MP02 admission/provision deadline".into())));
    // Keep the exact source error in the step rather than translating it into an incomplete row.
    let ready = check("admissions", setup, &mut failures);
    if ready {
        check(
            "pvp",
            bounded("pvp", 240, pvp_journey(&mut bots, world, towns)).await,
            &mut failures,
        );
        check(
            "stop-combat",
            bounded("stop-combat", 20, attack_target(&mut bots[0], None)).await,
            &mut failures,
        );
        let result = bounded("relog", 1100, relog(&mut bots, false)).await;
        check("relog", result, &mut failures);
    } else {
        check(
            "pvp",
            Err("blocked by admissions failure".into()),
            &mut failures,
        );
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
        "e2e_all scenario=MP02.full status={status} seconds={:.3}",
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
                failures.push("MP02 relog lost death/kill history".into());
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
            "MP_CHECK MP02 party-after-relog PASS terminal-release-removes-membership-in-all-four-views-and-PG",
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
        .map_err(|_| format!("MP02 {phase} deadline;source:{}:{}", file!(), line!()))?
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
        .ok_or("MP02 character tab request absent")?;
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
        "MP_CHECK MP02 skull-view PASS participant={} skull={} points={points:?} PG=matched",
        bot.name, view.skull
    ));
    Ok(())
}

async fn latest_death(bot: &Bot<'_, '_>) -> TestResult<Option<(String, String, String)>> {
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let receipt = sqlx::query_as("SELECT death_occurrence_id::text,original_character_revision::text,committed_character_revision::text FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid AND death_world_id=encode($2,'hex')::uuid AND death_channel_id=encode($3,'hex')::uuid ORDER BY committed_at DESC,death_occurrence_id DESC LIMIT 1")
        .bind(bot.character.as_slice()).bind(bot.pair.world.as_slice()).bind(bot.pair.channel.as_slice()).fetch_optional(&mut database).await?;
    database.close().await?;
    Ok(receipt)
}

async fn pvp_journey(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
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
            "MP02 needs neutral A for a single unjustified kill oracle;observed={baseline:?}"
        )
        .into());
    }
    let old_death = latest_death(&bots[1]).await?;
    let old_kills = history(&mut bots[0], character::CharacterTab::RecentPvpKills).await?;
    let old_deaths = history(&mut bots[1], character::CharacterTab::RecentDeaths).await?;
    // Provisioning sets the build, not its wielded weapon. Equip through A's own protocol.
    let definitions = provisioning::item_index()?;
    let axe = provisioning::item_reference(&definitions, 3276)?;
    let equipped = bots[0]
        .client()?
        .session_mut()
        .equip_item_definition(axe)
        .await?;
    if equipped.status != CommandStatus::Accepted
        || equipped.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    {
        return Err(format!("axe equip refused:{equipped:?};source:apps/game-server/src/qualification/client_phases/provision.rs:86").into());
    }
    let from = tile(bots[1].client()?.world_spatial().actor_position)?;
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
    bots::walk_bot(bots, 1, world, &path).await?;
    bots::approach(bots, 0, 1, world, true).await?;
    let attacker = bots[0].own()?;
    let victim = bots[1].own()?;
    let mode = pvp_expert::ExpertFightModes {
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
            &pvp_expert::encode_expert_fight_modes(&mode),
        )
        .await?;
    let disposition = oteryn_protocol_oteryn::attack::decode_attack_intent_result(&reply.payload)
        .map_err(|error| format!("expert mode decode:{error:?};status={:?};source:crates/protocol-oteryn/src/attack.rs:300", reply.status))?;
    if reply.status != CommandStatus::Accepted
        || disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    {
        return Err(format!("expert modes status={:?};disposition={disposition:?};source:crates/gameplay/dispatch/src/gameplay_transport/pvp_expert.rs:1", reply.status).into());
    }
    bots::wait_observation(bots, tokio::time::Instant::now() + Duration::from_secs(5), "expert fight mode update", |_, events| {
        for event in &events[0] {
            if let SessionEvent::ExpertFightModes(delta) = event {
                if delta.value != mode {
                    return Err(format!("expert modes requested={mode:?};served={:?};source:crates/gameplay/dispatch/src/gameplay_transport/pvp_expert.rs:1", delta.value).into());
                }
                return Ok(true);
            }
        }
        Ok(false)
    }).await?;
    // The target command's refusal is retained exactly, including the protected starter D.
    attack_target(&mut bots[0], Some(&victim)).await?;
    let mut victim_damaged = false;
    let mut attacker_saw_hit = false;
    bots::wait_observation(bots, tokio::time::Instant::now() + Duration::from_secs(180), "MP02 lethal hit and temple respawn", |bots, events| {
        for event in &events[0] {
            if let SessionEvent::Presentation(delta) = event {
                attacker_saw_hit |= delta.value.iter().any(|event| matches!(event.kind,
                    oteryn_protocol_oteryn::presentation::PresentationEventKind::ValueText { target, attacker: Some(source), .. }
                    if target == victim && source == attacker));
            }
        }
        for event in &events[1] { if let SessionEvent::ActorVitals(delta) = event { victim_damaged |= delta.value.health < delta.value.max_health; } }
        let client = bots[1].session.as_ref().ok_or("victim closed")?;
        Ok(victim_damaged && attacker_saw_hit && tile(client.world_spatial().actor_position)? == temple
            && client.actor_vitals().is_some_and(|v| v.health == v.max_health))
    }).await?;
    attack_target(&mut bots[0], None).await?;
    let death = latest_death(&bots[1])
        .await?
        .ok_or("MP02 respawn missing durable death receipt")?;
    if Some(&death) == old_death.as_ref() || death.1 == death.2 {
        return Err("MP02 respawn reused old death/revision".into());
    }
    let kills = history(&mut bots[0], character::CharacterTab::RecentPvpKills).await?;
    let description = character::pvp_kill_description(&bots[1].name);
    if !kills.iter().any(|k| {
        k.description == description
            && k.status == Some(character::KillStatus::Unjustified)
            && !old_kills.contains(k)
    }) || !history(&mut bots[1], character::CharacterTab::RecentDeaths)
        .await?
        .iter()
        .any(|d| !old_deaths.contains(d))
    {
        return Err("MP02 A kill view / D death view lack new attributed death".into());
    }
    let mut database = sqlx::PgConnection::connect(bots[0].pair.url).await?;
    let points: Option<i32> = sqlx::query_scalar("SELECT points_milli FROM game_character_unjustified_points WHERE character_id=encode($1,'hex')::uuid AND victim_character_id=encode($2,'hex')::uuid AND death_occurrence_id=$3::text::uuid")
        .bind(bots[0].character.as_slice()).bind(bots[1].character.as_slice()).bind(&death.0).fetch_optional(&mut database).await?;
    database.close().await?;
    if points
        != Some(i32::from(
            oteryn_gameplay_combat_rules::pvp::skull::DAMAGE_POINTS_MILLI,
        ))
    {
        return Err(format!("MP02 durable frag points={points:?};death={}", death.0).into());
    }
    // D has already proved its temple respawn; bring its new incarnation back into interest.
    bots::approach(bots, 1, 0, world, true).await?;
    for bot in bots.iter_mut() {
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
            return Err(format!("skull icons status={:?};source:crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:85", reply.status).into());
        }
        let icons = party_skull::decode_party_icons(&reply.payload)
            .map_err(|e| format!("skull icons:{e:?}"))?;
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
            "MP_CHECK MP02 attacker-skull-view PASS participant={} A_skull=white",
            bot.name
        ));
    }
    let awarded = pvp_status(&mut bots[0]).await?;
    let increment = oteryn_gameplay_combat_rules::pvp::skull::contributor_points(
        oteryn_gameplay_combat_rules::pvp::skull::PvpContributorKind::Damage,
        1,
    );
    if awarded.points_milli != [u64::from(increment); 3] || awarded.skull != 3 {
        return Err(format!("MP02 single unjustified kill expected white skull and {increment} points;observed={awarded:?}").into());
    }
    for bot in bots {
        verify_pvp_state(bot).await?;
    }
    evidence(&format!(
        "MP_CHECK MP02 death-frag PASS A=attributed-hit-and-new-kill D=damage-death-and-temple-respawn PG_death={} PG_points={points:?}",
        death.0
    ));
    Ok(())
}
