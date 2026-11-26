//! P11: achievements query/notification and quest/event granters.
//!
//! Concrete player scenario derived from the P11 matrix/re-audit row and the routed owner:
//! 1. Admit the provisioned character and send account-achievement page 0; expect an accepted,
//!    internally consistent page and an achievement-notice watermark snapshot.
//! 2. Refuse malformed query bytes at the local codec boundary without submission, then
//!    re-query unchanged state. The typed core API cannot submit a malformed wire query.
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
    let result = verdict::indexed_timeout(
        Duration::from_secs(90),
        verdict::observe(journey(clients, core)),
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

const APPROACH: TilePos = TilePos {
    x: 33213,
    y: 31671,
    floor: -13,
};

pub(super) fn qualification_entry(town: TilePos) -> TilePos {
    if std::env::var("NODE_BOOT_P11_APPROACH").as_deref() == Ok("1")
        && matches!(
            std::env::var("OTERYN_E2E_ROW_BATCH").as_deref(),
            Ok("54:1" | "166:1")
        )
    {
        APPROACH
    } else {
        town
    }
}

async fn open_approach_gate(
    client: &mut DevClientSession,
    world: &BundleWorld,
    from: TilePos,
    name: &str,
    landings: &[u16],
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let gate = TilePos {
        x: from.x + 1,
        ..from
    };
    let observed = tile(client.world_spatial().actor_position)?;
    if observed != from {
        return Err(format!("step={name} expected={from:?} actual={observed:?}").into());
    }
    let planned = destination(world, from, StepDirection::East);
    let base_tile = world
        .base()
        .tile(gate.x, gate.y, gate.floor)
        .ok_or_else(|| format!("step={name} blocking_tile={gate:?} reason=missing-from-bundle"))?;
    evidence(&format!(
        "P11_ROUTE step={name} from={from:?} blocking_tile={gate:?} planned={planned:?} walkable={} enterable={} appearance={:?}",
        base_tile.walkable(),
        world.enterable(gate),
        base_tile.appearance(1),
    ));
    let key = crate::map::view::placement_key(gate, 1).ok_or("approach gate placement absent")?;
    let mut target = world.base().digest().to_vec();
    target.extend_from_slice(&key.to_be_bytes());
    use oteryn_protocol_oteryn::world_map::MapOrigin;
    let position = ActorPosition {
        x: i32::from(gate.x),
        y: i32::from(gate.y),
        floor: -i16::from(gate.floor),
    };
    let closed = base_tile
        .appearance(1)
        .ok_or("P11 authored closed door has no appearance")?;
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("P11 live door map view absent")?;
    let revision = snapshot.tiles.iter()
        .find(|cell| cell.position == position)
        .and_then(|cell| cell.items.iter().find_map(|item| match item.origin {
            MapOrigin::BaseOrdinal { ordinal: 1, object_revision } if item.appearance_id == closed => Some(object_revision),
            _ => None,
        }))
        .ok_or_else(|| format!("step={name} live map did not show the authored closed door tile={gate:?} appearance={closed}"))?;
    evidence(&format!(
        "P11_ROUTE step={name} live_closed_appearance={closed} expected_revision={revision}"
    ));
    let reply = client.session_mut().use_object(&target, revision).await?;
    evidence(&format!(
        "P11_ROUTE step={name} tile={gate:?} wire_reply={reply:?}"
    ));
    if reply.disposition != UseDisposition::Committed {
        return Err(format!("step={name} authored_gate={gate:?} first_reply={reply:?}").into());
    }
    walk_east(client, from, name, landings).await
}

async fn walk_east(
    client: &mut DevClientSession,
    from: TilePos,
    name: &str,
    landings: &[u16],
) -> TestResult {
    for &x in landings {
        let expected = TilePos { x, ..from };
        let reply = client.step(StepDirection::East).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if reply.disposition != StepDisposition::Moved || actual != expected {
            return Err(format!(
                "step={name} blocking_tile={expected:?} reply={reply:?} actual={actual:?}"
            )
            .into());
        }
        if reply.step_wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(u64::from(reply.step_wait_ms))).await;
        }
    }
    Ok(())
}

async fn stage_member(client: &mut DevClientSession, world: &BundleWorld, x: u16) -> TestResult {
    open_approach_gate(client, world, APPROACH, "outer-level-door", &[33214, 33215]).await?;
    open_approach_gate(
        client,
        world,
        TilePos {
            x: 33215,
            ..APPROACH
        },
        "staging-door",
        &[33217],
    )
    .await?;
    let landings: Vec<_> = (33218..=x).collect();
    walk_east(client, APPROACH, "lever-staging", &landings).await
}

async fn operate_lever(
    client: &mut DevClientSession,
    world: &BundleWorld,
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
) -> TestResult {
    stage_member(client, world, 33225).await?;
    let mut companions = Vec::with_capacity(3);
    for (scope, index, x) in [(clients, 0, 33224), (core, 0, 33223), (core, 1, 33222)] {
        let finished = std::cell::Cell::new(false);
        let (member, serviced) = tokio::join!(
            async {
                let result = async {
                    let mut member = connect_knight(scope, index).await?;
                    stage_member(&mut member, world, x).await?;
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
        ..APPROACH
    };
    let key =
        crate::map::view::placement_key(lever, 1).ok_or("Annihilator lever placement absent")?;
    let mut target = world.base().digest().to_vec();
    target.extend_from_slice(&key.to_be_bytes());
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
    for (member, x) in std::iter::once(&mut *client)
        .chain(companions.iter_mut())
        .zip([33222, 33221, 33220, 33219])
    {
        let expected = TilePos {
            x,
            y: 31659,
            floor: -13,
        };
        let actual = tile(member.world_spatial().actor_position)?;
        if actual != expected {
            return Err(
                format!("step=lever expected_arrival={expected:?} actual={actual:?}").into(),
            );
        }
    }
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
    if qualification_entry(TOWN) == APPROACH {
        operate_lever(client, world, clients, core).await?;
    }
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
        "Annihilator outer gate eligibility",
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
) -> TestResult<GrantProof> {
    if qualification_entry(TOWN) != APPROACH {
        return Err("P11 needs its isolated authored approach profile".into());
    }
    // Complete setup with one live session at a time, before staging the party.
    for (scope, index) in [(clients, 1), (clients, 0), (core, 0), (core, 1)] {
        prepare_knight(scope, index).await?;
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

async fn journey(clients: &SeamClients<'_>, core: &SeamClients<'_>) -> TestResult {
    let proof = earn_and_logout(clients, core).await?;
    let mut fresh = connect_world(clients, 0xc2).await?;
    if fresh.join_snapshot().game_session_id.as_bytes() == &proof.session
        || page(&mut fresh, 0).await? != proof.page
    {
        return Err("step=relogin durable achievement page changed".into());
    }
    logout(&mut fresh).await?;
    Err("step=coverage observed=bounded-grant-relogin-validated;unproven=malformed-core-command-10-wire-refusal;source=crates/session/src/account_achievements.rs:18".into())
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
        assert!(fixture < first_admission);
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
