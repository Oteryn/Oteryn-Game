//! Frozen wire frames through real Session -> run_session -> generated dispatch -> panel.
use oteryn_client::play::{PlayLink, play_channel, run_session};
use oteryn_client_runtime::quest_log::{QuestLogQuery, QuestRefusal};
use oteryn_client_ui::quest_log::{panel_text, tracking_query};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{Admission, CharacterId, Session, read_frame, write_frame};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};

type Error = Box<dyn std::error::Error + Send + Sync>;
fn bytes(value: &serde_json::Value) -> Result<Vec<u8>, Error> {
    Ok(serde_json::from_value(value.clone())?)
}
async fn peer(mut server: DuplexStream, recording: serde_json::Value) -> Result<(), Error> {
    read_frame(&mut server).await?; // bootstrap is validated by Session's admission tests
    for value in recording["startup"].as_array().ok_or("startup")? {
        write_frame(&mut server, &bytes(value)?).await?;
    }
    for exchange in recording["exchanges"].as_array().ok_or("exchanges")? {
        // Full command envelopes, including generation, IDs, type 22 and payload, match
        // frozen protocol-oteryn encoder output; no parallel test encoder is used.
        assert_eq!(read_frame(&mut server).await?, bytes(&exchange["command"])?);
        for value in exchange["frames"].as_array().ok_or("frames")? {
            write_frame(&mut server, &bytes(value)?).await?;
        }
    }
    let mut byte = [0];
    assert_eq!(server.read(&mut byte).await?, 0); // link shutdown closes the real session
    Ok(())
}
async fn revision(link: &PlayLink, wanted: u64) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(2), async {
        while link.quest_panel().state.revision != Some(wanted) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn quest_log_recorded_domain_16_frames_reach_the_play_panel() -> Result<(), Error> {
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(async {
        let recording = serde_json::from_str(include_str!("wire-recorded.json"))?;
        let (client, server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(peer(server, recording));
        let session = Session::admit(
            client,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: CharacterId::decode(&[
                    1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, 4, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33,
                    0x44, 4,
                ])?,
                admission_material: b"quest-ui-fixture",
                client_build_id: "quest-ui-test",
                supported_capabilities: &[16],
                deadline: Duration::from_secs(2),
            },
        )
        .await?;
        let (link, commands, outbox) = play_channel();
        let task = tokio::spawn(run_session(session, commands, outbox));
        revision(&link, 0).await?;
        let open = NormalizedInputEvent::Key {
            code: KeyCode::new(13)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        };
        assert!(link.quest_input(&[open]));
        revision(&link, 1).await?;
        assert!(panel_text(&link.quest_panel()).contains("Quest 1"));
        link.quest_request(&QuestLogQuery::Quest { quest_index: 1 });
        revision(&link, 2).await?;
        let text = panel_text(&link.quest_panel());
        assert!(text.contains("The tower"));
        assert!(text.contains("Find the key."));
        link.quest_request(&tracking_query(&link.quest_panel()).ok_or("tracking control")?);
        revision(&link, 3).await?;
        assert!(panel_text(&link.quest_panel()).contains("Tracked quests (1/10)"));
        link.quest_request(&tracking_query(&link.quest_panel()).ok_or("untrack control")?);
        revision(&link, 4).await?;
        assert!(link.quest_panel().state.log.tracked.is_empty());
        link.quest_request(&QuestLogQuery::Quest { quest_index: 99 });
        assert_eq!(
            link.quest_panel().state.refusal,
            Some(QuestRefusal::UnknownQuest)
        );
        drop(link);
        tokio::time::timeout(Duration::from_secs(2), task).await??;
        tokio::time::timeout(Duration::from_secs(2), peer).await???;
        Ok::<(), Error>(())
    })??;
    Ok(())
}
