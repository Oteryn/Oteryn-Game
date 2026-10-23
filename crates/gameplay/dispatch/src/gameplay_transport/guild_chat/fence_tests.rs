use super::*;

pub(super) async fn negatives(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    owner
        .chat_admit(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .await;
    assert!(matches!(
        owner.chat_guild_admit(admitted).await,
        GuildRoomState::Member { .. }
    ));
    for field in 0..7 {
        let mut forged = admitted;
        let fence = forged.item_fence.as_mut().ok_or("fence")?;
        match field {
            0 => {
                forged.continuity.connection_generation += 1;
                fence.connection_generation =
                    crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
            }
            1 => fence.character_lease_generation += 1,
            2 => {
                fence.scope_ownership_generation =
                    crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
            }
            3 => forged.world_id = crate::foundation::WorldId::decode(&id(99)).map_err(debug)?,
            4 => forged.runtime_actor = None,
            5 => fence.character_id = CharacterId::from_bytes(id(99)).map_err(debug)?,
            _ => {
                forged.game_session_id =
                    crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?
            }
        }
        assert_eq!(
            owner
                .chat_guild_intent(forged, "forged".into())
                .await
                .disposition,
            ChatDisposition::Rejected
        );
    }
    owner.chat.deliver(&payload(gid(91)?, "other guild")?);
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
    // The same queued identity is not delivered to an old connection after readmission.
    owner.chat.deliver(&payload(gid(90)?, "old queue")?);
    owner
        .chat_admit(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .await;
    owner.chat_guild_admit(admitted).await;
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
    // One refresh owns one shared read budget across successful dequeues, too.
    for _ in 0..3 {
        owner
            .chat
            .deliver(&payload(gid(90)?, "bounded owner batch")?);
    }
    let mut remaining = 2;
    for expected in [1, 0] {
        assert!(
            owner
                .chat_next_bound(admitted, &mut remaining)
                .await
                .is_some()
        );
        assert_eq!(remaining, expected);
        owner.chat_delivered(admitted.game_session_id).await;
    }
    assert!(
        owner
            .chat_next_bound(admitted, &mut remaining)
            .await
            .is_none()
    );
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_some());
    owner.chat_delivered(admitted.game_session_id).await;
    Ok(())
}
