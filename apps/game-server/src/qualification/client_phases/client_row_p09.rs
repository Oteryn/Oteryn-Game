//! P09 client replay: drive the daily-reward panel with the current journey's wire replies.
use super::*;
use oteryn_dev_client::client_ui::daily_reward::{
    DailyRewardCommand, DailyRewardPanel, DailyRewardReply, DailyRewardResponse, Intent,
    apply_result, decode_reply, panel_text, request,
};
use oteryn_protocol_oteryn::daily_reward as wire;

use std::path::PathBuf;

struct CaptureDirectory(PathBuf);

impl Drop for CaptureDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let mut client = connect_world(clients, 0x2b).await?;
    let session = client.join_snapshot().game_session_id;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let start = tile(client.world_spatial().actor_position)?;
    let shrine = full_row_p09::shrine_tile(world, start)?;
    let path = route(world, start, |at| {
        at.floor == shrine.floor && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
    })
    .ok_or("no route to the daily shrine")?;
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("daily shrine step has no destination")?;
        let step = client.step(direction).await?;
        if step.disposition != StepDisposition::Moved
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(
                format!("daily shrine step from {from:?} toward {expected:?}: {step:?}").into(),
            );
        }
    }
    let refusal_position = tile(client.world_spatial().actor_position)?;
    require_shrine_reach(refusal_position, shrine)?;
    let before = reward_state(clients).await?;
    if before.3 {
        return Err("daily refusal needs a character with no prior claims".into());
    }
    let query = daily_command(&mut client, wire::COMMAND_TYPE_DAILY_REWARD_QUERY).await?;
    let claim = daily_command(&mut client, wire::COMMAND_TYPE_DAILY_REWARD_CLAIM).await?;
    let after = daily_command(&mut client, wire::COMMAND_TYPE_DAILY_REWARD_QUERY).await?;
    if query.server_sequence >= claim.server_sequence
        || claim.server_sequence >= after.server_sequence
    {
        return Err("daily capture sequences did not advance".into());
    }
    if after.status != CommandStatus::Accepted || after.payload != query.payload {
        return Err("unopened claim changed the server wall".into());
    }
    unchanged_rewards(&before, &reward_state(clients).await?)?;
    if tile(client.world_spatial().actor_position)? != refusal_position {
        return Err("actor moved between refusal and shrine use".into());
    }
    let opened = client
        .session_mut()
        .use_object(b"oteryn:placement/daily-reward-shrine", 0)
        .await?;
    let ready = daily_command(&mut client, wire::COMMAND_TYPE_DAILY_REWARD_QUERY).await?;
    let expected_ready = wire::DailyRewardWall {
        lane_day: 1,
        streak: 0,
        jokers: 1,
        claimed: false,
        claim_enabled: true,
    };
    if opened.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        || ready.server_sequence <= after.server_sequence
        || ready.status != CommandStatus::Accepted
        || decode_reply(DailyRewardCommand::Query, &ready.payload)?
            != DailyRewardReply::Wall(expected_ready)
    {
        return Err(format!("same-position shrine use did not enable claim: {opened:?}").into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("daily logout refused: {logout:?}").into());
    }
    drop(client);
    let mut client = connect_world(clients, 0x2c).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("daily relog reused the original session".into());
    }
    require_shrine_reach(tile(client.world_spatial().actor_position)?, shrine)?;
    let persisted = daily_command(&mut client, wire::COMMAND_TYPE_DAILY_REWARD_QUERY).await?;
    unchanged_rewards(&before, &reward_state(clients).await?)?;
    if persisted.status != CommandStatus::Accepted || persisted.payload != query.payload {
        return Err("daily wall differed after relog".into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("daily relog cleanup refused: {logout:?}").into());
    }

    let output = std::process::Command::new("mktemp")
        .args(["-d", "-t", "oteryn-p09.XXXXXXXXXX"])
        .output()?;
    if !output.status.success() {
        return Err("could not create private daily capture directory".into());
    }
    let dir = CaptureDirectory(String::from_utf8(output.stdout)?.trim_end().into());
    let binding = format!(
        "{session:?}:{}:{}:{}\n",
        query.server_sequence, claim.server_sequence, ready.server_sequence
    );
    std::fs::write(dir.0.join("binding"), &binding)?;
    std::fs::write(dir.0.join("p09-wall.bin"), &query.payload)?;
    std::fs::write(dir.0.join("p09-claim.bin"), &claim.payload)?;
    std::fs::write(dir.0.join("p09-ready.bin"), &ready.payload)?;
    if std::fs::read_to_string(dir.0.join("binding"))? != binding {
        return Err("daily capture belongs to a different session or sequence".into());
    }
    let wall = std::fs::read(dir.0.join("p09-wall.bin"))?;
    let refusal = std::fs::read(dir.0.join("p09-claim.bin"))?;
    if wall != query.payload
        || refusal != claim.payload
        || std::fs::read(dir.0.join("p09-ready.bin"))? != ready.payload
    {
        return Err("daily capture does not match the bound live replies".into());
    }
    let expected = replay(query.status, &wall, claim.status, &refusal)?;
    let capture_path = dir.0.clone();
    let replay_output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("timeout")
            .args([
                "60",
                "cargo",
                "+1.100.0",
                "test",
                "--locked",
                "-p",
                "oteryn-client",
                "--features",
                "bevy-shell",
                "--lib",
                "bevy_shell::game_ui::reward_wall::tests::p09_live_reward_wall_replay",
                "--",
                "--ignored",
                "--exact",
            ])
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env("P09_LIVE_CAPTURE_DIR", capture_path)
            .output()
    })
    .await??;
    let stdout = String::from_utf8_lossy(&replay_output.stdout);
    if !replay_output.status.success()
        || !stdout.lines().any(|line| {
            line.starts_with("test ") && line.ends_with("::p09_live_reward_wall_replay ... ok")
        })
        || !stdout.contains("test result: ok. 1 passed; 0 failed;")
    {
        return Err(format!(
            "playable Reward Wall replay failed: {stdout} {}",
            String::from_utf8_lossy(&replay_output.stderr)
        )
        .into());
    }
    evidence("P09_CHECK playable Reward Wall PASS");
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        use std::io::Write;
        let root = Path::new(&root);
        for (name, bytes) in [
            ("p09-wall.bin", wall.as_slice()),
            ("p09-claim.bin", refusal.as_slice()),
            ("p09-expected.txt", expected.as_bytes()),
        ] {
            std::fs::File::create_new(root.join(name))?.write_all(bytes)?;
        }
    }
    Ok(expected.replace('\n', ";"))
}

fn require_shrine_reach(actor: TilePos, shrine: TilePos) -> TestResult {
    if actor.floor != shrine.floor || actor.x.abs_diff(shrine.x).max(actor.y.abs_diff(shrine.y)) > 1
    {
        return Err(
            format!("refusal occurred outside shrine reach: {actor:?} -> {shrine:?}").into(),
        );
    }
    Ok(())
}

type RewardState = (i64, i64, i16, bool, Option<String>, Option<String>);

async fn reward_state(clients: &SeamClients<'_>) -> TestResult<RewardState> {
    use sqlx::Connection;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT
          coalesce((SELECT balance FROM game_character_cosmetic_tokens WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid),0),
          coalesce((SELECT balance FROM game_character_bounty_reroll_tokens WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid),0),
          coalesce((SELECT balance FROM game_account_prey_wildcards WHERE account_id=$3::text::uuid AND world_id=encode($2,'hex')::uuid),0::smallint),
          EXISTS(SELECT 1 FROM game_character_daily_reward_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid),
          (SELECT to_jsonb(j)::text FROM game_account_daily_reward_jokers j
             WHERE account_id=$3::text::uuid AND world_id=encode($2,'hex')::uuid),
          (SELECT to_jsonb(e)::text FROM game_account_daily_reward_joker_entries e
             WHERE account_id=$3::text::uuid AND world_id=encode($2,'hex')::uuid
             ORDER BY entry_id DESC LIMIT 1)",
    )
    .bind(clients.characters[1].as_slice())
    .bind(clients.world.as_slice())
    .bind(&clients.accounts[1])
    .fetch_one(&mut db)
    .await?)
}

fn unchanged_rewards(before: &RewardState, after: &RewardState) -> TestResult {
    if before != after {
        return Err(format!("unopened claim changed rewards: {before:?} -> {after:?}").into());
    }
    Ok(())
}

fn rendered_refusal(text: &str) -> TestResult {
    if !text
        .lines()
        .any(|line| line == "Open the daily reward shrine within reach, then refresh")
    {
        return Err("rendered wall omitted the shrine refusal".into());
    }
    Ok(())
}

async fn daily_command(
    client: &mut oteryn_dev_client::DevClientSession,
    command: u32,
) -> TestResult<oteryn_dev_client::native_session::GatedCommandResult> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_DAILY_REWARD_V2, command, &[])
        .await?)
}

fn replay(
    query_status: CommandStatus,
    query_payload: &[u8],
    claim_status: CommandStatus,
    claim_payload: &[u8],
) -> TestResult<String> {
    let mut panel = DailyRewardPanel {
        selected: true,
        ..DailyRewardPanel::default()
    };
    if request(&mut panel, Intent::Refresh) != Some(DailyRewardCommand::Query) {
        return Err("daily panel did not request a wall refresh".into());
    }
    if query_status != CommandStatus::Accepted {
        return Err(format!("wall query status={:?}", query_status).into());
    }
    let DailyRewardReply::Wall(wall) = decode_reply(DailyRewardCommand::Query, query_payload)?
    else {
        return Err("wall query did not decode as a wall".into());
    };
    if wall.claimed || wall.claim_enabled || (wall.lane_day, wall.streak, wall.jokers) != (1, 0, 1)
    {
        return Err(format!("expected fresh unopened wall observed={wall:?}").into());
    }
    if !apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: query_status,
            reply: Some(DailyRewardReply::Wall(wall)),
        },
    ) || panel.wall != Some(wall)
    {
        return Err("daily panel did not apply the live wall".into());
    }
    let text = panel_text(&panel);
    for expected in [
        format!("Day {} of 7", wall.lane_day),
        format!("Streak: {}", wall.streak),
        format!("Jokers: {}", wall.jokers),
        "Status: not available".to_owned(),
    ] {
        if !text.lines().any(|line| line == expected) {
            return Err(format!("daily panel omitted {expected:?}: {text:?}").into());
        }
    }
    if request(&mut panel, Intent::Claim).is_some()
        || panel.feedback != Some("The daily reward cannot be claimed now")
    {
        return Err("daily panel did not disable the unopened claim".into());
    }
    if claim_payload.is_empty() {
        return Err(format!("claim reply without payload status={:?}", claim_status).into());
    }
    let DailyRewardReply::Result(result) = decode_reply(DailyRewardCommand::Claim, claim_payload)?
    else {
        return Err("claim did not decode as a result".into());
    };
    if claim_status != CommandStatus::Rejected
        || result.disposition != 4
        || result.transaction_id.is_some()
    {
        return Err(format!(
            "expected unopened shrine refusal status={:?} result={result:?}",
            claim_status
        )
        .into());
    }
    if apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: claim_status,
            reply: Some(DailyRewardReply::Result(result)),
        },
    ) || panel.wall != Some(wall)
        || panel.feedback != Some("Open the daily reward shrine within reach, then refresh")
    {
        return Err("daily panel did not retain the wall and display the shrine refusal".into());
    }
    rendered_refusal(&panel_text(&panel))?;
    let expected = format!(
        "lane_day={}\nstreak={}\njokers={}\nclaimed={}\nclaim_enabled={}\nclaim_status=refused\nclaim_disposition={}\n",
        wall.lane_day,
        wall.streak,
        wall.jokers,
        wall.claimed,
        wall.claim_enabled,
        result.disposition
    );
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P09.client status=PASS evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P09.client status=FAIL evidence=timeout"),
    }
}

#[test]
fn p09_replay_rejects_incorrect_fresh_wall_values() -> TestResult {
    let refuse = wire::DailyRewardResult {
        disposition: 4,
        transaction_id: None,
    }
    .encode()
    .map_err(|error| format!("{error:?}"))?;
    for (day, streak, jokers, valid) in [
        (1, 0, 1, true),
        (2, 0, 1, false),
        (1, 1, 1, false),
        (1, 0, 2, false),
    ] {
        let wall = wire::DailyRewardWall {
            lane_day: day,
            streak,
            jokers,
            claimed: false,
            claim_enabled: false,
        }
        .encode()
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            replay(
                CommandStatus::Accepted,
                &wall,
                CommandStatus::Rejected,
                &refuse
            )
            .is_ok(),
            valid
        );
    }
    Ok(())
}

#[test]
fn p09_rejects_missing_rendered_refusal_and_unintended_grants() {
    assert!(rendered_refusal("Status: not available").is_err());
    assert!(rendered_refusal("Open the daily reward shrine within reach, then refresh").is_ok());
    let before = (0, 0, 0, false, None, None);
    for after in [
        (2, 0, 0, false, None, None),
        (0, 1, 0, false, None, None),
        (0, 0, 1, false, None, None),
        (0, 0, 0, true, None, None),
        (0, 0, 0, false, Some("materialized joker".into()), None),
        (0, 0, 0, false, None, Some("grant ledger entry".into())),
    ] {
        assert!(unchanged_rewards(&before, &after).is_err());
    }
    assert!(unchanged_rewards(&before, &before).is_ok());
}

#[test]
fn p09_rejects_out_of_reach_refusal() {
    let shrine = TilePos {
        x: 10,
        y: 10,
        floor: -7,
    };
    assert!(require_shrine_reach(TilePos { x: 11, ..shrine }, shrine).is_ok());
    assert!(require_shrine_reach(TilePos { x: 12, ..shrine }, shrine).is_err());
    assert!(
        require_shrine_reach(
            TilePos {
                floor: -6,
                ..shrine
            },
            shrine
        )
        .is_err()
    );
}
