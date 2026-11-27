//! W13 client replay capture: the real DAILY_BOOST_QUERY reply (the day's boosted creature, boss
//! and reset epoch) and a malformed query refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0151-daily-boost/protocol.rs`.
use super::*;
use oteryn_dev_client::client_ui::daily_boost::{self, DailyBoostPanel};
use oteryn_protocol_oteryn::daily_boost as wire;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

type Reply = oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;

const FRAME_FILES: [&str; 3] = ["w13-query.json", "w13-refusal.json", "w13-readmitted.json"];

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

impl Envelope {
    fn from_live(game_session_id: String, reply: Reply) -> Self {
        Self {
            command_id: reply.command_id,
            game_session_id,
            payload: reply.payload,
            server_sequence: reply.server_sequence,
            status: reply.status as u32,
        }
    }

    fn into_reply(self) -> TestResult<Reply> {
        let status = match self.status {
            1 => CommandStatus::Accepted,
            2 => CommandStatus::Rejected,
            actual => return Err(format!("unexpected captured daily boost status {actual}").into()),
        };
        Ok(Reply {
            command_id: self.command_id,
            status,
            server_sequence: self.server_sequence,
            payload: self.payload,
        })
    }
}

fn provenance(frames: &[Envelope; 3]) -> String {
    let [query, refusal, readmitted] = frames;
    format!(
        "session={} fresh_session={} replies={}:{},{}:{},{}:{}",
        query.game_session_id,
        readmitted.game_session_id,
        query.command_id,
        query.server_sequence,
        refusal.command_id,
        refusal.server_sequence,
        readmitted.command_id,
        readmitted.server_sequence,
    )
}

fn require_query_command(capability: u32, kind: u32, payload: &[u8]) -> TestResult {
    if (capability, kind, payload)
        != (
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[][..],
        )
    {
        return Err(format!("wrong daily boost panel command: capability={capability} kind={kind} payload={payload:?}").into());
    }
    Ok(())
}

fn panel_query(
    panel: &mut DailyBoostPanel,
) -> TestResult<oteryn_dev_client::client_runtime::raw_feature::FeatureCommand> {
    let command = daily_boost::request(
        panel,
        daily_boost::Intent {
            command: wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            payload: Vec::new(),
        },
    )
    .ok_or("daily boost panel did not emit a query")?;
    require_query_command(command.capability, command.kind, &command.payload)?;
    Ok(command)
}

async fn query(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<Reply> {
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    let command = panel_query(&mut panel)?;
    Ok(client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?)
}

fn require_reply(reply: &Reply, expected: CommandStatus) -> TestResult {
    if reply.status != expected
        || (expected == CommandStatus::Rejected
            && wire::DailyBoostRefusal::decode(&reply.payload)
                != Ok(wire::DailyBoostRefusal::MalformedQuery))
    {
        return Err(format!(
            "daily boost expected {expected:?}, observed {reply:?}, refusal={:?}",
            wire::DailyBoostRefusal::decode(&reply.payload)
        )
        .into());
    }
    Ok(())
}

type Selection = (String, String, String);

fn require_unchanged(expected: &Selection, observed: &Selection, phase: &str) -> TestResult {
    if observed != expected {
        return Err(format!(
            "{phase} changed the persisted daily boost: expected {expected:?}, observed {observed:?}"
        ).into());
    }
    Ok(())
}

async fn persisted(clients: &SeamClients<'_>) -> TestResult<Selection> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let selected = sqlx::query_as(
        "SELECT a.reset_epoch::text, b.creature_key, b.boss_key
         FROM game_world_reset_active a JOIN game_world_daily_boosts b USING (world_id, reset_epoch)
         WHERE a.world_id = encode($1, 'hex')::uuid",
    )
    .bind(clients.world.as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(selected)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let mut client = connect_world(clients, 0xfa).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Err("admission did not offer daily-boost capability 151".into());
    }
    let session = client.join_snapshot().game_session_id;
    let expected = persisted(clients).await?;
    let reply = query(&mut client).await?;
    require_reply(&reply, CommandStatus::Accepted)?;
    require_unchanged(&expected, &persisted(clients).await?, "initial query")?;
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[0],
        )
        .await?;
    require_reply(&refused, CommandStatus::Rejected)?;
    require_unchanged(&expected, &persisted(clients).await?, "malformed query")?;
    if client.session_mut().logout().await?.result != Some(LogoutResult::Accepted) {
        return Err("daily boost logout refused".into());
    }
    drop(client);

    let mut fresh = connect_world(clients, 0xfb).await?;
    let fresh_session = fresh.join_snapshot().game_session_id;
    if session == fresh_session {
        return Err("daily boost readmission failed to retain the persisted selection".into());
    }
    require_unchanged(&expected, &persisted(clients).await?, "readmission")?;
    let readmitted = query(&mut fresh).await?;
    require_reply(&readmitted, CommandStatus::Accepted)?;
    require_unchanged(&expected, &persisted(clients).await?, "post-relog query")?;
    if fresh.session_mut().logout().await?.result != Some(LogoutResult::Accepted) {
        return Err("daily boost readmission logout refused".into());
    }
    let observed = [
        Envelope::from_live(uuid_text(session.as_bytes()), reply),
        Envelope::from_live(uuid_text(session.as_bytes()), refused),
        Envelope::from_live(uuid_text(fresh_session.as_bytes()), readmitted),
    ];
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory missing")?;
    let root = Path::new(&root);
    for (name, frame) in FRAME_FILES.into_iter().zip(&observed) {
        capture(root, name, &serde_json::to_vec(frame)?)?;
    }
    capture(root, "w13-expected.json", &serde_json::to_vec(&expected)?)?;
    Ok(provenance(&observed))
}

fn expected_names(expected: &Selection) -> TestResult<[String; 2]> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/creatures/definitions/spell-native-profiles.json");
    let profiles: serde_json::Value = serde_json::from_reader(std::fs::File::open(source)?)?;
    let records = profiles["records"]
        .as_array()
        .ok_or("creature profile records missing")?;
    let canonical = |key: &str| -> TestResult<String> {
        // TibiaData's boostable boss list supplies this exact display spelling.
        if key == "oteryn:creature.timira_the_many_headed" {
            return Ok("Timira The Many-Headed".into());
        }
        records
            .iter()
            .find(|record| record["profile"]["target"]["key"].as_str() == Some(key))
            .and_then(|record| {
                record["profile"]["data"]["profile"]["details"]["display_name"].as_str()
            })
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("missing authored daily boost display name for {key}").into())
    };
    Ok([canonical(&expected.1)?, canonical(&expected.2)?])
}

fn require_card_projection(
    cards: &[daily_boost::SpotlightCard],
    expected: &Selection,
    names: &[String; 2],
) -> TestResult<String> {
    let (epoch, creature, boss) = expected;
    let epoch_value = epoch.parse::<u64>()?;
    let [creature_name, boss_name] = names;
    let [creature_card, boss_card] = cards else {
        return Err(format!("expected two spotlight cards, observed {cards:?}").into());
    };
    for (card, key, name, kind) in [
        (creature_card, creature, creature_name, "Boosted creature"),
        (boss_card, boss, boss_name, "Boosted boss"),
    ] {
        if (&card.key, &card.name, card.kind, card.reset_epoch) != (key, name, kind, epoch_value) {
            return Err(format!(
                "expected spotlight card ({key}, {name}, {kind}, {epoch_value}), observed {card:?}"
            )
            .into());
        }
    }
    Ok(format!(
        "Daily spotlight\nReset epoch: {epoch}\nBoosted creature: {creature_name}\n{creature}\nBoosted boss: {boss_name}\n{boss}\n"
    ))
}

#[test]
fn w13_audit_checks_reject_mutated_selection_and_cards() -> TestResult {
    let expected: Selection = (
        "42".into(),
        "oteryn:creature.braindeath".into(),
        "oteryn:creature.sir_baeloc".into(),
    );
    let replacement = (
        "42".into(),
        "oteryn:creature.rat".into(),
        expected.2.clone(),
    );
    for phase in [
        "initial query",
        "malformed query",
        "readmission",
        "post-relog query",
    ] {
        require_unchanged(&expected, &expected, phase)?;
        assert!(require_unchanged(&expected, &replacement, phase).is_err());
    }
    let cards = [
        daily_boost::SpotlightCard {
            kind: "Boosted creature",
            key: expected.1.clone(),
            name: "Braindeath".into(),
            reset_epoch: 42,
        },
        daily_boost::SpotlightCard {
            kind: "Boosted boss",
            key: expected.2.clone(),
            name: "Sir Baeloc".into(),
            reset_epoch: 42,
        },
    ];
    let names = expected_names(&expected)?;
    require_card_projection(&cards, &expected, &names)?;
    for index in 0..cards.len() {
        let mut wrong_name = cards.clone();
        wrong_name[index].name = "Wrong name".into();
        assert!(require_card_projection(&wrong_name, &expected, &names).is_err());
        let mut wrong_epoch = cards.clone();
        wrong_epoch[index].reset_epoch = 41;
        assert!(require_card_projection(&wrong_epoch, &expected, &names).is_err());
    }
    Ok(())
}

#[test]
fn w13_audit_rejects_title_cased_boss_names_and_wrong_commands() -> TestResult {
    for (key, canonical, wrong) in [
        (
            "oteryn:creature.goshnar_s_cruelty",
            "Goshnar's Cruelty",
            "Goshnar S Cruelty",
        ),
        (
            "oteryn:creature.timira_the_many_headed",
            "Timira The Many-Headed",
            "Timira The Many Headed",
        ),
    ] {
        let selection = ("42".into(), "oteryn:creature.braindeath".into(), key.into());
        let names = expected_names(&selection)?;
        assert_eq!(names[1], canonical);
        let mut cards = [
            daily_boost::SpotlightCard {
                kind: "Boosted creature",
                key: selection.1.clone(),
                name: names[0].clone(),
                reset_epoch: 42,
            },
            daily_boost::SpotlightCard {
                kind: "Boosted boss",
                key: selection.2.clone(),
                name: canonical.into(),
                reset_epoch: 42,
            },
        ];
        require_card_projection(&cards, &selection, &names)?;
        cards[1].name = wrong.into();
        assert!(require_card_projection(&cards, &selection, &names).is_err());
    }
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    panel_query(&mut panel)?;
    for (capability, kind, payload) in [
        (150, 177, &[][..]),
        (151, 176, &[][..]),
        (151, 177, &[0][..]),
    ] {
        assert!(require_query_command(capability, kind, payload).is_err());
    }
    Ok(())
}

#[test]
#[ignore = "requires a fresh W13 live capture"]
fn e2e_w13_replays_daily_boost_selection_and_refusal() -> TestResult {
    use daily_boost::{apply_result, panel_text, spotlight_cards};

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory missing")?;
    let root = Path::new(&root);
    let read = |file: &str| -> TestResult<Envelope> {
        Ok(serde_json::from_slice(&std::fs::read(root.join(file))?)?)
    };
    let observed = [
        read(FRAME_FILES[0])?,
        read(FRAME_FILES[1])?,
        read(FRAME_FILES[2])?,
    ];
    assert_eq!(provenance(&observed), required("W13_CAPTURE_PROVENANCE")?);
    let [accepted, refused, readmitted] = observed;
    assert_eq!(accepted.game_session_id, refused.game_session_id);
    assert_ne!(accepted.game_session_id, readmitted.game_session_id);
    for session in [&accepted.game_session_id, &readmitted.game_session_id] {
        assert!(
            crate::node::uuid_bytes(session)
                .is_some_and(|id| { id[6] >> 4 == 7 && id[8] >> 6 == 2 })
        );
    }
    assert_eq!(accepted.command_id.checked_add(1), Some(refused.command_id));
    assert!(accepted.server_sequence < refused.server_sequence);
    let expected: Selection =
        serde_json::from_slice(&std::fs::read(root.join("w13-expected.json"))?)?;
    let names = expected_names(&expected)?;
    let (epoch, creature, boss) = &expected;
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    assert!(panel.boost.is_none());
    for (record, status) in [
        (accepted, CommandStatus::Accepted),
        (refused, CommandStatus::Rejected),
        (readmitted, CommandStatus::Accepted),
    ] {
        let reply = record.into_reply()?;
        assert_ne!(reply.command_id, 0);
        assert_ne!(reply.server_sequence, 0);
        require_reply(&reply, status)?;
        let before = panel.boost.clone();
        panel_query(&mut panel)?;
        assert_eq!(panel_text(&panel), "Loading daily boost...");
        apply_result(&mut panel, &reply);
        assert!(!panel.pending);
        if status == CommandStatus::Rejected {
            assert_eq!(
                wire::DailyBoostRefusal::decode(&reply.payload),
                Ok(wire::DailyBoostRefusal::MalformedQuery)
            );
            assert_eq!(panel.boost, before);
            assert_eq!(panel.feedback, Some("The server refused the action"));
            panel = DailyBoostPanel {
                selected: true,
                ..Default::default()
            };
            continue;
        }
        let view = panel.boost.as_ref().ok_or("accepted reply has no boost")?;
        assert_eq!(
            (
                &view.reset_epoch.to_string(),
                &view.creature_key,
                &view.boss_key
            ),
            (epoch, creature, boss)
        );
        assert_eq!(panel.feedback, None);
        assert_eq!(
            panel_text(&panel),
            require_card_projection(&spotlight_cards(&panel), &expected, &names)?
        );
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_capture scenario=W13.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W13.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W13.client status=FAIL evidence=timeout"),
    }
}
