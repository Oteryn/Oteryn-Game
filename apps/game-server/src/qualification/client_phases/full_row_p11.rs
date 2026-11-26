//! P11: achievements query/notification and quest/event granters.
//!
//! Concrete player scenario derived from the P11 matrix/re-audit row and the routed owner:
//! 1. Admit the provisioned character and send account-achievement page 0; expect an accepted,
//!    internally consistent page and an achievement-notice watermark snapshot.
//! 2. Assert the live malformed-query refusal and unchanged pre-grant state.
//! 3. Check page boundaries and re-admit the same character to verify the query
//!    remains stable. A grant requires a reachable authored source and its notice.
use super::*;
use oteryn_dev_client::client_runtime::account_achievements::{
    AccountAchievementsResult, Response,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::account_achievements as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>, core: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let placed_at = std::cell::Cell::new(None);
    let result = verdict::indexed_timeout(
        Duration::from_secs(90),
        verdict::observe(journey(clients, core, &placed_at)),
    )
    .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step deadline: achievement lifecycle timed out".into()),
    };
    let (status, evidence_text) = match result {
        Ok(()) => (
            "PASS",
            "query-notice-grant-refusal-relogin-validated".to_owned(),
        ),
        Err(error) => ("FAIL", sanitize(&error.to_string())),
    };
    if let Some(placed_at) = placed_at.get() {
        evidence(&format!(
            "P11_TIMING placement_to_row_end_ms={} status={status}",
            placed_at.elapsed().as_millis()
        ));
    }
    evidence(&format!("P11_GAP status={status} observed={evidence_text}"));
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=P11.full status={status} seconds={:.3} evidence={evidence_text}",
        started.elapsed().as_secs_f64()
    ));
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:./=,".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

async fn page(client: &mut DevClientSession, number: u32) -> TestResult<AccountAchievementsResult> {
    match client.session_mut().account_achievements(number).await? {
        Response::Page(page)
            if page.page == number
                && page.fact_count >= u32::try_from(page.rows.len())?
                && page.total_points >= page.rows.iter().map(|row| row.points).sum() =>
        {
            Ok(page)
        }
        Response::Page(page) => Err(format!("step=query invalid server page: {page:?}").into()),
        Response::Refused => Err(format!("step=query page={number} observed=refused").into()),
    }
}

const STAGING: TilePos = TilePos {
    x: 33225,
    y: 31671,
    floor: -13,
};

async fn place_member(clients: &SeamClients<'_>, index: usize, at: TilePos) -> TestResult {
    wait_character_released(clients, index).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let changed = sqlx::query(
        "UPDATE game_character_last_positions AS p
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND (p.tile_x,p.tile_y,p.tile_floor) IS DISTINCT FROM ($3,$4,$5)
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions AS s
               WHERE s.character_id=p.character_id AND s.world_id=p.world_id AND s.session_state<>3
           )",
    )
    .bind(clients.characters[index].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(at.x))
    .bind(i32::from(at.y))
    .bind(i32::from(-at.floor))
    .execute(&mut db)
    .await?;
    db.close().await?;
    if changed.rows_affected() != 1 {
        return Err(
            format!("P11 fixture requires one different offline position for {at:?}").into(),
        );
    }
    evidence(&format!(
        "FIXTURE_PLACEMENT mechanic=P11 character={} x={} y={} floor={}",
        uuid_text(&clients.characters[index]),
        at.x,
        at.y,
        -at.floor
    ));
    Ok(())
}

fn assert_staging(client: &DevClientSession, expected: TilePos) -> TestResult {
    let actual = tile(client.world_spatial().actor_position)?;
    if actual != expected {
        return Err(
            format!("P11 expected fixture position {expected:?}, admitted at {actual:?}").into(),
        );
    }
    Ok(())
}

async fn wait_for_lever_arrival(
    member: &mut DevClientSession,
    staged: TilePos,
    expected: TilePos,
    after_sequence: u64,
    deadline: tokio::time::Instant,
) -> TestResult {
    loop {
        let events = member.take_events();
        let mut arrived = false;
        for event in events {
            evidence(&format!("P11_LEVER_FRAME member_x={} {event:?}", staged.x));
            let (sequence, position) = match event {
                SessionEvent::WorldSpatial(delta) => {
                    (delta.server_sequence, delta.value.actor_position)
                }
                SessionEvent::WorldSpatialEntities(delta) => {
                    (delta.server_sequence, delta.value.actor_position)
                }
                _ => continue,
            };
            if sequence <= after_sequence {
                continue;
            }
            let actual = tile(position)?;
            if actual == staged && !arrived {
                continue;
            }
            if actual != expected {
                return Err(format!(
                    "step=lever wrong_arrival expected={expected:?} actual={actual:?} sequence={sequence}"
                ).into());
            }
            arrived = true;
        }
        if arrived {
            assert_staging(member, expected)?;
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "step=lever missing_arrival expected={expected:?} after_sequence={after_sequence} last_sequence={} actual={:?}",
                member.last_server_sequence(), tile(member.world_spatial().actor_position)?
            ).into());
        }
        member
            .service_liveness(remaining.min(Duration::from_millis(200)))
            .await?;
    }
}

async fn operate_lever(
    client: &mut DevClientSession,
    world: &BundleWorld,
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
) -> TestResult {
    assert_staging(client, STAGING)?;
    let mut companions = Vec::with_capacity(3);
    for (scope, index, x) in [(clients, 0, 33224), (core, 0, 33223), (core, 1, 33222)] {
        let finished = std::cell::Cell::new(false);
        let (member, serviced) = tokio::join!(
            async {
                let result = async {
                    let member = connect_knight(scope, index).await?;
                    assert_staging(&member, TilePos { x, ..STAGING })?;
                    Ok::<_, Box<dyn std::error::Error>>(member)
                }
                .await;
                finished.set(true);
                result
            },
            async {
                while !finished.get() {
                    for peer in std::iter::once(&mut *client).chain(companions.iter_mut()) {
                        peer.service_liveness(Duration::from_millis(100)).await?;
                    }
                }
                Ok::<(), oteryn_dev_client::DevClientError>(())
            }
        );
        let member = member?;
        serviced?;
        companions.push(member);
    }
    let lever = TilePos {
        x: 33226,
        ..STAGING
    };
    let key =
        crate::map::view::placement_key(lever, 1).ok_or("Annihilator lever placement absent")?;
    let mut target = world.base().digest().to_vec();
    target.extend_from_slice(&key.to_be_bytes());
    client.take_events();
    let [second, third, fourth] = companions.as_mut_slice() else {
        return Err("P11 requires exactly three lever companions".into());
    };
    let [second_sequence, third_sequence, fourth_sequence] =
        [&mut *second, &mut *third, &mut *fourth].map(|member| {
            member.take_events();
            member.last_server_sequence()
        });
    let reply = client.session_mut().use_object(&target, 0).await?;
    evidence(&format!(
        "P11_ROUTE step=annihilator-lever tile={lever:?} wire_reply={reply:?}"
    ));
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        for member in &mut companions {
            let _ = logout(member).await;
        }
        return Err(format!("step=lever authored_lever={lever:?} first_reply={reply:?}").into());
    }
    let destination = TilePos {
        x: 33222,
        y: 31659,
        floor: -13,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let (leader, second, third, fourth) = tokio::join!(
        wait_for_lever_arrival(
            client,
            STAGING,
            destination,
            reply.result_server_sequence,
            deadline
        ),
        wait_for_lever_arrival(
            second,
            TilePos {
                x: 33224,
                ..STAGING
            },
            TilePos {
                x: 33221,
                ..destination
            },
            second_sequence,
            deadline
        ),
        wait_for_lever_arrival(
            third,
            TilePos {
                x: 33223,
                ..STAGING
            },
            TilePos {
                x: 33220,
                ..destination
            },
            third_sequence,
            deadline
        ),
        wait_for_lever_arrival(
            fourth,
            TilePos {
                x: 33222,
                ..STAGING
            },
            TilePos {
                x: 33219,
                ..destination
            },
            fourth_sequence,
            deadline
        ),
    );
    leader?;
    second?;
    third?;
    fourth?;
    evidence("P11_LEVER_ARRIVAL all_four=exact");
    Err("step=annihilator-combat need=source-corridor-clearance".into())
}

async fn authored_granter(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
) -> TestResult<(Vec<u8>, String)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let items = provisioning::item_index()?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|item| {
            item.key()
                .as_str()
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|id| id.parse().ok())
                .is_some_and(|id| provisioning::item_reference(&items, id).is_ok())
        },
    )?;
    operate_lever(client, world, clients, core).await?;
    const REWARD_CHEST: TilePos = TilePos {
        x: 33227,
        y: 31656,
        floor: -13,
    };
    let mut matches = report
        .served
        .into_iter()
        .flat_map(|claim| claim.placements)
        .filter(|placement| {
            placement.cell.x == REWARD_CHEST.x
                && placement.cell.y == REWARD_CHEST.y
                && placement.cell.floor == REWARD_CHEST.floor
                && placement.placement.achievement.as_deref() == Some(ACHIEVEMENT)
        });
    let placement = matches
        .next()
        .ok_or("step=granter missing fixed authored Annihilator chest at 33227,31656,13")?;
    if matches.next().is_some() {
        return Err("step=granter ambiguous authored Annihilator chest at 33227,31656,13".into());
    }
    let from = tile(client.world_spatial().actor_position)?;
    let interaction = TilePos {
        y: 31657,
        ..REWARD_CHEST
    };
    let path = route(world, from, |at| at == interaction)
        .ok_or("step=granter fixed Annihilator chest position 33227,31657,13 unreachable")?;
    walk(client, world, &path).await?;
    let mut key = world.base().digest().to_vec();
    key.extend_from_slice(&placement.bundle_placement_key.to_be_bytes());
    Ok((key, ACHIEVEMENT.to_owned()))
}

async fn prepare_knight(clients: &SeamClients<'_>, index: usize) -> TestResult {
    character_prep::declare_seeded(
        "P11.full",
        "knight-level-100",
        "Annihilator party lever eligibility",
    );
    let mut initial = connect_character(clients, 0xc1, index).await?;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(initial.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "knight".into(),
    )
    .await?;
    logout(&mut initial).await
}

async fn connect_knight(clients: &SeamClients<'_>, index: usize) -> TestResult<DevClientSession> {
    let mut fresh = connect_character(clients, 0xc1, index).await?;
    use oteryn_dev_client::client_ui::cyclopedia_character as panel;
    use oteryn_protocol_oteryn::cyclopedia_character::CharacterTab;
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(&fresh),
            death_details: false,
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(CharacterTab::GeneralStats),
    )
    .ok_or("P11 GeneralStats request absent")?;
    let reply = fresh
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if !panel::apply_result(&mut view, &reply)
        || view.general.as_ref().map(|stats| stats.level) != Some(100)
    {
        return Err(format!("P11 seeded Knight expected level 100; reply={reply:?}").into());
    }
    let vitals = fresh
        .actor_vitals()
        .ok_or("P11 seeded Knight missing vitals")?;
    evidence(&format!(
        "P11_ROUTE seeded_level=100 seeded_vitals={vitals:?}"
    ));
    if (vitals.max_health, vitals.max_mana) != (1565, 550) {
        return Err(format!("P11 Knight derived vitals mismatch: {vitals:?}").into());
    }
    Ok(fresh)
}

pub(super) struct GrantProof {
    pub(super) page: AccountAchievementsResult,
    pub(super) session: [u8; 16],
}

const ACHIEVEMENT: &str = "oteryn:achievement/annihilator";
const DESCRIPTION: &str = "You've daringly jumped into the infamous Annihilator and survived - taking home fame, glory and your reward.";

fn expected_grant(page: &AccountAchievementsResult, earliest: i64, latest: i64) -> bool {
    page.page == 0
        && page.total_points == 5
        && page.fact_count == 1
        && !page.has_more
        && matches!(page.rows.as_slice(), [entry]
            if entry.key == ACHIEVEMENT && entry.name == "Annihilator"
                && entry.description == DESCRIPTION && entry.grade == 2 && entry.points == 5
                && !entry.secret && (earliest..=latest).contains(&entry.earned_at))
}

pub(super) async fn earn_and_logout(
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
    placed_at: &std::cell::Cell<Option<std::time::Instant>>,
) -> TestResult<GrantProof> {
    // Complete setup with one live session at a time, before staging the party.
    for (scope, index) in [(clients, 1), (clients, 0), (core, 0), (core, 1)] {
        prepare_knight(scope, index).await?;
    }
    for (scope, index, x) in [
        (clients, 1, 33225),
        (clients, 0, 33224),
        (core, 0, 33223),
        (core, 1, 33222),
    ] {
        place_member(scope, index, TilePos { x, ..STAGING }).await?;
        if placed_at.get().is_none() {
            placed_at.set(Some(std::time::Instant::now()));
        }
    }
    let mut client = connect_knight(clients, 1).await?;
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let before = page(&mut client, 0).await?;
    if before != AccountAchievementsResult::default() {
        return Err(format!(
            "P11 fresh account expected no previous achievement facts; actual={before:?}"
        )
        .into());
    }
    let notice_snapshot = client
        .session_mut()
        .gated_snapshot(
            oteryn_protocol_oteryn::achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        )
        .ok_or("step=query missing achievement-notice snapshot")?;
    let watermark =
        oteryn_protocol_oteryn::achievement_notices::decode_achievement_notices_snapshot(
            &notice_snapshot.payload,
        )
        .map_err(|error| format!("step=query malformed notice snapshot: {error:?}"))?;
    if watermark.fact_count != 0 || watermark.total_points != 0 {
        return Err(
            format!("P11 initial achievement watermark was not empty: {watermark:?}").into(),
        );
    }

    client.take_events();
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    if wire::decode_account_achievements_query(&[0x08]).is_ok()
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
    {
        return Err("step=local-codec-refusal observed=submitted-or-not-refused".into());
    }
    if client
        .take_events()
        .iter()
        .any(|event| matches!(event, SessionEvent::AchievementEarned(_)))
        || page(&mut client, 0).await? != before
    {
        return Err("step=local-codec-refusal observed=unexpected-achievement-delta".into());
    }
    evidence(
        "P11_CHECK step=malformed-query boundary=local-codec-refusal wire_submission=false command-and-sequence=unchanged reread=unchanged",
    );

    let beyond = before
        .fact_count
        .div_ceil(wire::ACCOUNT_ACHIEVEMENTS_PAGE_ROWS as u32)
        .max(1);
    let empty = page(&mut client, beyond).await?;
    if empty.has_more
        || !empty.rows.is_empty()
        || empty.fact_count != before.fact_count
        || empty.total_points != before.total_points
    {
        return Err(format!("step=pagination page={beyond} observed={empty:?}").into());
    }
    let (placement, achievement) = authored_granter(&mut client, clients, core).await?;
    if achievement != ACHIEVEMENT {
        return Err(
            format!("P11 expected the authored Annihilator claim, got {achievement}").into(),
        );
    }
    let earliest = i64::try_from(now_seconds()?)?;
    client.take_events();
    let use_reply = client.session_mut().use_object(&placement, 0).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    let earned = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::AchievementEarned(delta) => Some(delta.value),
            _ => None,
        });
    if use_reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!(
            "step=granter achievement={achievement} authored chest declined: {use_reply:?}"
        )
        .into());
    }
    let Some(earned) = earned else {
        // Still complete the durable lifecycle so this is runtime evidence, not a source probe.
        let unchanged = page(&mut client, 0).await?;
        logout(&mut client).await?;
        drop(client);
        let mut fresh = connect_world(clients, 0xc2).await?;
        let persisted = page(&mut fresh, 0).await?;
        logout(&mut fresh).await?;
        return Err(format!(
            "step=granter observed_reply={use_reply:?},missing_delta=AchievementEarned,query_unchanged={},relogin_unchanged={};need=reachable_quest_or_event_source_granter",
            unchanged == before,
            persisted == before
        )
        .into());
    };
    let granted = page(&mut client, 0).await?;
    let latest = i64::try_from(now_seconds()?)?;
    if earned.key != ACHIEVEMENT
        || earned.name != "Annihilator"
        || earned.watermark.fact_count != 1
        || earned.watermark.total_points != 5
        || !expected_grant(&granted, earliest, latest)
    {
        return Err(format!("step=granter delta/query mismatch: {earned:?} {granted:?}").into());
    }

    client.take_events();
    let duplicate_reply = client.session_mut().use_object(&placement, 0).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    if duplicate_reply.disposition
        != oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse
        || client
            .take_events()
            .iter()
            .any(|event| matches!(event, SessionEvent::AchievementEarned(_)))
        || page(&mut client, 0).await? != granted
    {
        return Err(
            format!("step=duplicate observed_reply={duplicate_reply:?},duplicate-grant").into(),
        );
    }
    logout(&mut client).await?;
    drop(client);
    Ok(GrantProof {
        page: granted,
        session,
    })
}

async fn journey(
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
    placed_at: &std::cell::Cell<Option<std::time::Instant>>,
) -> TestResult {
    client_row_p11::malformed_live(clients, 1).await?;
    let proof = earn_and_logout(clients, core, placed_at).await?;
    let mut fresh = connect_world(clients, 0xc2).await?;
    if fresh.join_snapshot().game_session_id.as_bytes() == &proof.session
        || page(&mut fresh, 0).await? != proof.page
    {
        return Err("step=relogin durable achievement page changed".into());
    }
    logout(&mut fresh).await?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn p11_uses_only_the_authored_annihilator_reward_position() {
        let source = include_str!("full_row_p11.rs");
        let Some((_, granter)) = source.split_once("async fn authored_granter(") else {
            panic!("missing P11 granter");
        };
        let Some((granter, _)) = granter.split_once("\n}") else {
            panic!("missing P11 granter body");
        };
        assert!(granter.contains("placement.cell.x == REWARD_CHEST.x"));
        assert!(granter.contains("placement.cell.y == REWARD_CHEST.y"));
        assert!(granter.contains("placement.cell.floor == REWARD_CHEST.floor"));
        assert!(
            granter.contains("placement.placement.achievement.as_deref() == Some(ACHIEVEMENT)")
        );
        assert!(granter.contains("matches.next().is_some()"));
        assert!(granter.contains("at == interaction"));
    }
    #[test]
    fn p11_serializes_fixture_logouts_before_party_staging() -> super::TestResult {
        let source = include_str!("full_row_p11.rs");
        let (_, staging) = source
            .split_once("async fn operate_lever(")
            .ok_or("lever staging helper")?;
        let (staging, _) = staging
            .split_once("async fn authored_granter(")
            .ok_or("granter helper")?;
        let (_, earn) = source
            .split_once("pub(super) async fn earn_and_logout(")
            .ok_or("earn helper")?;
        let (earn, _) = earn
            .split_once("async fn journey(")
            .ok_or("journey helper")?;
        assert!(!staging.contains("prepare_knight("));
        assert!(staging.contains("service_liveness("));
        let fixture = earn.find("prepare_knight(").ok_or("fixture preparation")?;
        let first_admission = earn
            .find("let mut client = connect_knight(")
            .ok_or("first live admission")?;
        let placement = earn.find("place_member(").ok_or("fixture placement")?;
        assert!(fixture < placement && placement < first_admission);
        assert!(!staging.contains("stage_member("));
        assert!(staging.contains("assert_staging("));
        Ok(())
    }

    #[test]
    fn p11_grant_proof_rejects_each_audit_false_positive() {
        use super::*;
        let entry = wire::AccountAchievementRow {
            key: ACHIEVEMENT.into(),
            name: "Annihilator".into(),
            description: DESCRIPTION.into(),
            grade: 2,
            points: 5,
            earned_at: 150,
            secret: false,
        };
        let correct = AccountAchievementsResult {
            rows: vec![entry],
            total_points: 5,
            fact_count: 1,
            ..Default::default()
        };
        assert!(expected_grant(&correct, 140, 160));
        assert!(!expected_grant(
            &AccountAchievementsResult::default(),
            140,
            160
        ));
        let mutate: [fn(&mut AccountAchievementsResult); 10] = [
            |page| page.total_points += 1,
            |page| page.fact_count += 1,
            |page| page.page += 1,
            |page| page.has_more = true,
            |page| page.rows[0].key.push('x'),
            |page| page.rows[0].name.push('x'),
            |page| page.rows[0].description.push('x'),
            |page| page.rows[0].grade += 1,
            |page| page.rows[0].points += 1,
            |page| page.rows[0].secret = true,
        ];
        for wrong in mutate {
            let mut page = correct.clone();
            wrong(&mut page);
            assert!(!expected_grant(&page, 140, 160));
        }
        assert!(!expected_grant(&correct, 151, 160));
        assert!(!expected_grant(&correct, 140, 149));
    }

    #[test]
    fn evidence_sanitizer_produces_one_report_token() {
        assert_eq!(
            super::sanitize("step failed\n(reply)"),
            "step_failed__reply_"
        );
    }
}

#[cfg(test)]
#[test]
fn row_11_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p11.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P11_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p11_guards_the_journey() {
    assert!(
        include_str!("full_row_p11.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("duplicate_reply"))
    );
}
