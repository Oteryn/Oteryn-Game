use super::*;
use crate::cast::Scene;
use tokio::io::AsyncWriteExt;
#[test]
fn s17b_observer_accepts_only_bound_public_frames_and_no_player_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (client, mut server) = tokio::io::duplex(32768);
            let peer = tokio::spawn(async move {
                let request = crate::read_frame(&mut server).await?;
                assert!(matches!(
                    Envelope::decode(&request, true)
                        .map_err(|e| format!("{e:?}"))?
                        .message,
                    Message::Join(_)
                ));
                for message in [
                    Message::Ready {
                        epoch: 4,
                        chat: false,
                        delay: 300,
                    },
                    Message::Scene {
                        epoch: 4,
                        sequence: 1,
                        scene: Scene {
                            tiles: vec![],
                            actors: vec![(1, 16, 272, 1, 75)],
                        },
                    },
                    Message::Scene {
                        epoch: 3,
                        sequence: 2,
                        scene: Scene {
                            tiles: vec![],
                            actors: vec![],
                        },
                    },
                ] {
                    let bytes = Envelope {
                        generation: 7,
                        message,
                    }
                    .encode()
                    .map_err(|e| format!("{e:?}"))?;
                    crate::write_frame(&mut server, &bytes).await?;
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            });
            let mut session =
                CastViewerSession::join(client, b"ticket", Duration::from_secs(2)).await?;
            assert_eq!(session.epoch(), 4);
            assert!(!session.chat_allowed());
            assert_eq!(session.delay(), 300);
            assert!(session.chat("disabled").await.is_err());
            assert!(matches!(
                session.next().await?,
                Message::Scene { epoch: 4, .. }
            ));
            assert!(session.next().await.is_err());
            assert!(session.keepalive().await.is_err());
            peer.await?.map_err(|e| e.to_string())?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
#[test]
fn s17b_observer_rejects_oversized_prefix_before_allocating_body()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (client, mut server) = tokio::io::duplex(8192);
            let peer = tokio::spawn(async move {
                crate::read_frame(&mut server).await?;
                server.write_all(&16433u32.to_be_bytes()).await?;
                Ok::<(), SessionError>(())
            });
            assert!(
                CastViewerSession::join(client, b"ticket", Duration::from_secs(2))
                    .await
                    .is_err()
            );
            peer.await??;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
