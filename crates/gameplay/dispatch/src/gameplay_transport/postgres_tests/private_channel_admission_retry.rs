//! The initial channel-list push must recover when another pass owns the root at join.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::guild_chat::wire;
use oteryn_protocol_oteryn::chat::{ChatRoomSet, private_channel::ChatChannelList};

#[test]
fn private_channel_initial_list_recovers_after_busy_admission() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("private-channel-admission-retry".into())
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let started = std::time::Instant::now();
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "private_channel_admission_retry",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                eprintln!("PG_EXECUTED private_channel_initial_list_recovers_after_busy_admission: session prepared");
                {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let mut deps = Dependencies::new()?;
                    let mut item_fence = fence(1)?.item_fence();
                    item_fence.character_lease_generation = 2;
                    deps.admitted.item_fence = Some(item_fence);
                    deps.admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        PRODUCTION_OFFERED_CAPABILITIES,
                        &[6, 7, 42, 80],
                    )
                    .ok_or("fixture chat selection")?;
                    {
                        let mut runtime = deps.runtime.lock().await;
                        runtime
                            .initialize_first_entry_position(deps.actor)
                            .map_err(debug)?;
                        runtime
                            .bind_attacker_lease(
                                deps.actor,
                                deps.admitted.game_session_id,
                                crate::foundation::CharacterLease::new(
                                    crate::foundation::CharacterId::decode(&id(41))
                                        .map_err(debug)?,
                                    2,
                                )
                                .map_err(debug)?,
                            )
                            .map_err(debug)?;
                    }
                    let owner = deps.owner(&h, &authority);
                    deps.initialize(&owner).await;
                    // Deterministically model a scheduler taking the sole holder during join.
                    let busy = h.root.try_issue_semantic_pass().map_err(debug)?;
                    assert!(
                        wire::snapshot(&owner, deps.admitted, ChatRoomSet::default())
                            .await
                            .is_some()
                    );
                    assert!(owner.chat_channel_list_bound(deps.admitted).await.is_none());
                    busy.run(|_, _| Box::pin(async { Ok(()) }))
                        .await
                        .map_err(debug)?;
                    assert_eq!(
                        owner.chat_channel_list_bound(deps.admitted).await,
                        Some(ChatChannelList::default()),
                        "periodic refresh must finish the deferred negotiated admission"
                    );
                    assert!(
                        owner.chat_channel_list_bound(deps.admitted).await.is_none(),
                        "the initial list is pushed exactly once"
                    );
                    let mut unselected = deps.admitted;
                    unselected.continuity.selected_capabilities = SelectedCapabilities::select(
                        PRODUCTION_OFFERED_CAPABILITIES,
                        &[6, 7, 42],
                    )
                    .ok_or("legacy chat selection")?;
                    assert!(owner.chat_channel_list_bound(unselected).await.is_none());
                    assert!(
                        owner.states.chat.private_world.matches(&deps.admitted),
                        "a reader without capability 80 cannot disable the admitted participant"
                    );
                    let mut stale = deps.admitted;
                    stale.continuity.connection_generation += 1;
                    assert!(owner.chat_channel_list_bound(stale).await.is_none());
                    assert!(
                        owner.states.chat.private_world.matches(&deps.admitted),
                        "a mismatched reader cannot replace an already negotiated connection"
                    );
                }
                h.cleanup().await?;
                eprintln!(
                    "PG_PASS private_channel_initial_list_recovers_after_busy_admission: {:?}",
                    started.elapsed()
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "private channel admission retry thread panicked")?
        .map_err(Into::into)
}
