use super::*;
use crate::play::PlayView;
type HarePlayError = Box<dyn std::error::Error + Send + Sync>;
include!("../features/local-world-view/tests/source_outfit_pixels.rs");

#[test]
fn srv7_actual_messenger_initial_native_entry_frame_has_palette()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> Result<(), Box<dyn std::error::Error>> {
                let capture: serde_json::Value = serde_json::from_str(include_str!(
                    "../features/local-world-view/tests/messenger-wire.json"
                ))?;
                let source = &capture["source"];
                let text = capture["initial_frames_hex"]
                    .as_str()
                    .ok_or("actual Messenger frames")?;
                let raw = text
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
                    .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                assert_eq!(
                    oteryn_client_assets::text_sha256(&raw),
                    capture["initial_frames_sha256"]
                        .as_str()
                        .ok_or("actual frameSHA")?
                );
                let mut frames = Vec::new();
                let mut cursor = 0;
                while cursor < raw.len() {
                    let size = u32::from_be_bytes(
                        raw.get(cursor..cursor + 4)
                            .ok_or("actual prefix")?
                            .try_into()?,
                    ) as usize;
                    cursor += 4;
                    frames.push(
                        raw.get(cursor..cursor + size)
                            .ok_or("actual body")?
                            .to_vec(),
                    );
                    cursor += size;
                }
                let stack = test_support::Stack::start_with_frames(Vec::new(), frames)?;
                let root = load_root_certificate(&stack.root_path)?;
                let actor_text = source["actor_identity_hex"].as_str().ok_or("actual NPC")?;
                let actor: Vec<u8> = actor_text
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
                    .collect::<Result<_, Box<dyn std::error::Error>>>()?;
                let actor: [u8; 16] = actor.try_into().map_err(|_| "actual NPC identity width")?;
                let character_text = source["character_id_hex"]
                    .as_str()
                    .ok_or("actual Character")?;
                let character: Vec<u8> = character_text
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
                    .collect::<Result<_, Box<dyn std::error::Error>>>()?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                let view = runtime.block_on(async {
                    let address = std::net::SocketAddr::from(([127, 0, 0, 1], stack.game_port));
                    let stream =
                        oteryn_session_tcp::connection::connect_any(&[address], "localhost", &root)
                            .await
                            .ok_or("actual TLS playback connection")?;
                    let session = oteryn_session::Session::admit(
                        stream,
                        oteryn_session::Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: oteryn_session::CharacterId::decode(&character)?,
                            admission_material: b"srv7-real-capture-initial-view",
                            client_build_id: "srv7-native-play",
                            supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
                            deadline: std::time::Duration::from_secs(3),
                        },
                    )
                    .await?;
                    let admitted = AdmittedSession {
                        character_name: source["character_id_hex"]
                            .as_str()
                            .ok_or("actual viewer label")?
                            .into(),
                        world_id: source["world_id_hex"]
                            .as_str()
                            .ok_or("actual world")?
                            .into(),
                        channel_id: "actual captured session".into(),
                        session,
                    };
                    // EXACT production builder, before any session-loop publish can repair it.
                    Ok::<_, Box<dyn std::error::Error>>(admitted.play_view()?)
                })?;
                let palette = capture["palette_rgb"]
                    .as_array()
                    .ok_or("actual source palette")?
                    .iter()
                    .map(|v| {
                        Ok([
                            u8::try_from(v[0].as_u64().ok_or("r")?)?,
                            u8::try_from(v[1].as_u64().ok_or("g")?)?,
                            u8::try_from(v[2].as_u64().ok_or("b")?)?,
                        ])
                    })
                    .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                let failures = SourceOutfitPixels::load()
                    .map_err(|e| e.to_string())?
                    .check(&view, actor, false, &palette)
                    .map_err(|e| e.to_string())?;
                stack.finish();
                assert!(
                    failures.is_empty(),
                    "actual initial native-entry palette frame: {}",
                    failures.join("; ")
                );
                Ok(())
            };
            run().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "initial native palette thread")?
        .map_err(Into::into)
}
