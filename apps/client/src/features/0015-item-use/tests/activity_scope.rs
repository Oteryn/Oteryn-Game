// Source Soulpit tiles through real Session -> native USE -> scope barrier -> rendered scene.
use super::*;

type ScopeError = Box<dyn std::error::Error + Send + Sync>;
const CAPS: &[u32] = &[4, 6, 11, 15, 18, 117];

fn id(value: u8) -> [u8; 16] {
    let mut value = [value; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}

#[path = "activity_scope_wire.rs"]
mod scope_wire;
use scope_wire::source_peer;
#[path = "actual_capture.rs"]
mod actual_capture;

async fn render_until(
    view: &mut PlayView,
    link: &PlayLink,
    position: TileCoord,
    label: &str,
) -> Result<(), ScopeError> {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            view.tick(link)
                .map_err(|error| format!("native session ended: {error:?}"))?;
            if view.own() == position && view.combat_status().contains(label) {
                return Ok::<(), ScopeError>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| format!("native scope render timed out awaiting {label} at {position:?}"))??;
    Ok(())
}

#[test]
fn srx3_soulpit_native_use_scope_and_return_render_source_scene() -> Result<(), ScopeError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (client, peer) = tokio::io::duplex(1024 * 1024);
                let peer = tokio::spawn(source_peer(peer));
                let session = oteryn_session::Session::admit(
                    client,
                    oteryn_session::Admission {
                        appearance_items: None,
                        schema_revision: 1,
                        character_id: oteryn_session::CharacterId::decode(&id(4))?,
                        admission_material: b"soulpit-native",
                        client_build_id: "srx3-native-play",
                        supported_capabilities: CAPS,
                        deadline: std::time::Duration::from_secs(5),
                    },
                )
                .await;
                let session = match session {
                    Ok(session) => session,
                    Err(error) => {
                        let peer_error = peer.await?;
                        return Err(format!(
                            "initial native admission failed: {error}; source peer: {peer_error:?}"
                        )
                        .into());
                    }
                };
                let mut view = PlayView::from_join(session.join_snapshot())?;
                view.set_live_map(&session.world_map_snapshot().ok_or("initial source map")?)?;
                let (link, commands, outbox) = play_channel();
                let session = tokio::spawn(run_session(session, commands, outbox));
                tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    while link.inventory_panel().rows().len() != 2 {
                        tokio::task::yield_now().await;
                    }
                })
                .await.map_err(|_| "initial inventory panel did not decode")?;
                eprintln!("native Soulpit initial source map and inventory decoded; submitting actual core USE");
                assert!(link.inventory_select(1.try_into()?));
                assert!(link.inventory_use_with());
                assert!(view.use_with_click(TileCoord::new(32375, 31157), &link));
                view.in_flight = true;
                view.inputs
                    .push_back(Input::Arrow(crate::input::StepDir::North));
                render_until(
                    &mut view,
                    &link,
                    TileCoord::new(32373, 31151),
                    "Entered boss instance",
                )
                .await?;
                assert!(view.inputs.is_empty());
                assert!(!view.in_flight);
                assert!(view.scene().sprites().len() > 1);
                assert!(link.inventory_panel().selected_item().is_none());
                let menu = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    loop {
                        view.tick(&link).map_err(|error| format!("destination scope initialization: {error:?}"))?;
                        if let Some(menu) = view.world_context(TileCoord::new(32375, 31153), &link) {
                            return Ok::<_, ScopeError>(menu);
                        }
                        tokio::task::yield_now().await;
                    }
                }).await.map_err(|_| "native destination exit interaction not initialized")??;
                assert!(view.world_context_choose(
                    &link,
                    &menu,
                    oteryn_client_runtime::world_interaction::Action::Use
                ));
                render_until(
                    &mut view,
                    &link,
                    TileCoord::new(32373, 31158),
                    "Returned to channel",
                )
                .await?;
                assert!(view.inputs.is_empty());
                assert!(view.scene().sprites().len() > 1);
                drop(link);
                session.await?;
                peer.await??;
                Ok::<(), ScopeError>(())
            })?
        })?
        .join()
        .map_err(|_| "native Soulpit scope play thread panicked")?
}
