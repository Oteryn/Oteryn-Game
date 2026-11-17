use super::*;

#[test]
fn unselected_target_walk_is_refused_without_consuming_a_command() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, peer) = script_peer(vec![]);
                let mut session = Session::admit(client, admission()?).await?;
                let command = session.next_command_id();
                let sequence = session.last_server_sequence();
                assert!(matches!(
                    session
                        .target_walk(ActorPosition {
                            x: -10,
                            y: 200,
                            floor: -3
                        })
                        .await,
                    Err(SessionError::CapabilityNotSelected { capability: 64 })
                ));
                assert_eq!(session.next_command_id(), command);
                assert_eq!(session.last_server_sequence(), sequence);
                drop(session);
                peer.await??;
                Ok::<_, BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "target walk session thread failed")?
}
