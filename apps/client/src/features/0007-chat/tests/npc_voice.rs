use super::*;
use oteryn_session::{Admission, CharacterId, Session, read_frame, write_frame};
use tokio::io::AsyncReadExt;

#[test]
fn nx4_source_npc_say_and_yell_decode_into_native_chat_panel() -> Result<(), Error> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let frames: Vec<Vec<u8>> = serde_json::from_str(include_str!("npc-voice-wire.json"))?;
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                read_frame(&mut server).await?;
                for frame in frames { write_frame(&mut server, &frame).await?; }
                let mut byte = [0];
                assert_eq!(server.read(&mut byte).await?, 0);
                Ok::<(), Error>(())
            });
            let session = Session::admit(client, Admission {
                appearance_items: None, schema_revision: 1,
                character_id: CharacterId::decode(&[1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, 4, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44, 4])?,
                admission_material: b"nx4-npc-voice", client_build_id: "nx4-native-test",
                supported_capabilities: &[7], deadline: Duration::from_secs(2),
            }).await?;
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(session, incoming, outbox));
            wait_for(&link, |panel| panel.lines.len() == 2).await?;
            // The HWND chat panel displays this same formatter's output.
            let shown = panel_text(&link.chat_panel());
            assert!(shown.contains("Captain Bluebear: Passages to Carlin, Ab'Dendriel"), "{shown}");
            assert!(shown.contains("[Yell] Captain Bluebear: Come closer, traveller."), "{shown}");
            drop(link);
            task.await?;
            peer.await??;
            Ok::<(), Error>(())
        })?
    })?.join().map_err(|_| "NPC chat play test thread panicked")?
}
