//! WU26 capabilities 494 and 495 on two assigned nodes of one World: the joined channels and
//! their participants in the list, and the join, leave and exclusion notices between two reads,
//! through the real capability-80 wire dispatch and `read_private_channel_members`.
use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FreshAdmissionAuthority},
    guild_chat::wire,
};
use oteryn_protocol_oteryn::chat::line_extended::{ChannelNotice, ChannelNoticeKind};
use oteryn_protocol_oteryn::chat::private_channel::PrivateChannelIntent as Intent;
use oteryn_protocol_oteryn::chat::private_channel_extended::{
    ChannelListExtension, ChannelParticipants,
};
use oteryn_protocol_oteryn::chat::{self, ChatDisposition, ChatIntent};

#[test]
fn private_channel_participants_and_notices_follow_join_leave_and_exclusion() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("chat-world-participants".into())
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "chat_world_participants",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let (second_node, mut second) = world::admit_second_node(&h).await?;
                {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let mut first = Dependencies::new()?;
                    let mut fence = super::fixture::fence(1)?.item_fence();
                    fence.character_lease_generation = 2;
                    first.admitted.item_fence = Some(fence);
                    {
                        let mut runtime = first.runtime.lock().await;
                        runtime
                            .initialize_first_entry_position(first.actor)
                            .map_err(debug)?;
                        runtime
                            .bind_attacker_lease(
                                first.actor,
                                first.admitted.game_session_id,
                                crate::foundation::CharacterLease::new(
                                    crate::foundation::CharacterId::decode(&id(41))
                                        .map_err(debug)?,
                                    2,
                                )
                                .map_err(debug)?,
                            )
                            .map_err(debug)?;
                    }
                    let (consumer, clock, _, _) = purchase_fixture::current(id(40));
                    let coordinator =
                        SpellPremiumCoordinator::new(consumer, Some(clock), None, Some(h.root.clone()));
                    first.admitted.continuity.selected_capabilities = select(&[6, 7, 42, 80, 494, 495])?;
                    second.admitted.continuity.selected_capabilities = select(&[6, 7, 42, 80, 494, 495])?;
                    let mut owner = first.owner(&h, &authority);
                    owner.premium_coordinator = Some(&coordinator);
                    let mut target = second.owner(&h, &authority);
                    target.holder = &second_node;
                    assert_ne!(
                        owner.holder.fact().node_id(),
                        target.holder.fact().node_id()
                    );
                    first.initialize(&owner).await;
                    second.initialize(&target).await;
                    owner.chat_admit(first.actor, first.admitted.game_session_id).await;
                    target.chat_admit(second.actor, second.admitted.game_session_id).await;
                    owner
                        .public_chat_admit(first.admitted, chat::ChatRoomSet::default())
                        .await;
                    target
                        .public_chat_admit(second.admitted, chat::ChatRoomSet::default())
                        .await;
                    owner.private_channel_admit(first.admitted, true).await;
                    target.private_channel_admit(second.admitted, true).await;
                    let own_name: String = sqlx::query_scalar(
                        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
                    )
                    .bind(id(41).as_slice())
                    .fetch_one(&h.pool)
                    .await?;
                    let invitee = world::RECIPIENT_NAME;
                    let mut left = 100_u64;
                    let mut right = 200_u64;

                    // Without capability 494 or 495 nothing is read and nothing is shown.
                    let mut legacy = first.admitted;
                    legacy.continuity.selected_capabilities = select(&[6, 7, 42, 80])?;
                    assert_eq!(
                        owner.private_channel_list_extended(legacy).await,
                        (None, Vec::new())
                    );

                    assert_wire(&owner, first.admitted, &mut left, Intent::Create, ChatDisposition::Ok).await?;
                    // The first read is the baseline: the list and its extension, never a notice.
                    let (shown, notices) = owner.private_channel_list_extended(first.admitted).await;
                    let (list, extension) = shown.ok_or("owner baseline")?;
                    let channel = list.own_channel.ok_or("created channel")?;
                    assert!(notices.is_empty());
                    assert_eq!(extension, participants(channel, &[&own_name]));

                    assert_wire(&owner, first.admitted, &mut left, Intent::Invite { name: invitee.into() }, ChatDisposition::Ok).await?;
                    let (shown, notices) = target.private_channel_list_extended(second.admitted).await;
                    let (invited, extension) = shown.ok_or("invitee baseline")?;
                    assert_eq!(invited.invited.len(), 1);
                    assert_eq!(invited.invited[0].channel_id, channel);
                    assert_eq!(extension, ChannelListExtension::default(), "an invitation is no membership");
                    assert!(notices.is_empty());
                    assert_eq!(
                        owner.private_channel_list_extended(first.admitted).await,
                        (None, Vec::new()),
                        "an open invitation changes nothing the owner is shown"
                    );

                    assert_wire(&target, second.admitted, &mut right, Intent::Join { channel_id: channel }, ChatDisposition::Ok).await?;
                    let (shown, notices) = owner.private_channel_list_extended(first.admitted).await;
                    assert_eq!(notices, vec![notice(channel, ChannelNoticeKind::Joined, invitee)]);
                    assert_eq!(sorted(shown.ok_or("joined owner list")?.1), participants(channel, &[&own_name, invitee]));
                    let (shown, notices) = target.private_channel_list_extended(second.admitted).await;
                    assert!(notices.is_empty(), "the session's own join is no notice");
                    assert_eq!(sorted(shown.ok_or("joined invitee list")?.1), participants(channel, &[&own_name, invitee]));

                    assert_wire(&target, second.admitted, &mut right, Intent::Leave { channel_id: channel }, ChatDisposition::Ok).await?;
                    let (shown, notices) = owner.private_channel_list_extended(first.admitted).await;
                    assert_eq!(notices, vec![notice(channel, ChannelNoticeKind::Left, invitee)]);
                    assert_eq!(shown.ok_or("left owner list")?.1, participants(channel, &[&own_name]));
                    let (shown, notices) = target.private_channel_list_extended(second.admitted).await;
                    assert!(notices.is_empty());
                    assert_eq!(shown.ok_or("left invitee list")?.1, ChannelListExtension::default());

                    assert_wire(&target, second.admitted, &mut right, Intent::Join { channel_id: channel }, ChatDisposition::Ok).await?;
                    let (_, notices) = owner.private_channel_list_extended(first.admitted).await;
                    assert_eq!(notices, vec![notice(channel, ChannelNoticeKind::Joined, invitee)]);
                    let (_, notices) = target.private_channel_list_extended(second.admitted).await;
                    assert!(notices.is_empty());

                    assert_wire(&owner, first.admitted, &mut left, Intent::Exclude { name: invitee.into() }, ChatDisposition::Ok).await?;
                    let (shown, notices) = owner.private_channel_list_extended(first.admitted).await;
                    assert_eq!(
                        notices,
                        vec![notice(channel, ChannelNoticeKind::Excluded, invitee)],
                        "a removed member whose session still participates was excluded"
                    );
                    assert_eq!(shown.ok_or("excluded owner list")?.1, participants(channel, &[&own_name]));
                    let (shown, notices) = target.private_channel_list_extended(second.admitted).await;
                    assert!(notices.is_empty());
                    let (invited, extension) = shown.ok_or("excluded invitee list")?;
                    assert!(invited.invited.is_empty());
                    assert_eq!(extension, ChannelListExtension::default());
                    assert_eq!(
                        owner.private_channel_list_extended(first.admitted).await,
                        (None, Vec::new()),
                        "an unchanged read shows and notifies nothing"
                    );
                }
                h.cleanup().await?;
                eprintln!(
                    "PRIVATE_CHANNEL_PARTICIPANTS POSTGRES_CASE_COMPLETED: joined channels, participants and join, leave and exclusion notices"
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "private channel participants scenario panicked")?
        .map_err(Into::into)
}

fn select(requested: &[u32]) -> TestResult<SelectedCapabilities> {
    SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, requested)
        .ok_or_else(|| "extended private capability selection".into())
}

fn participants(channel_id: u16, names: &[&str]) -> ChannelListExtension {
    let mut names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    names.sort();
    ChannelListExtension {
        joined: vec![channel_id],
        participants: vec![ChannelParticipants { channel_id, names }],
    }
}

/// The read orders a channel's members by name key; compare them as a set.
fn sorted(mut extension: ChannelListExtension) -> ChannelListExtension {
    for entry in &mut extension.participants {
        entry.names.sort();
    }
    extension
}

fn notice(channel_id: u16, kind: ChannelNoticeKind, name: &str) -> ChannelNotice {
    ChannelNotice {
        channel_id,
        kind,
        name: name.into(),
    }
}

async fn assert_wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: &mut u64,
    intent: Intent,
    expected: ChatDisposition,
) -> TestResult {
    let payload =
        chat::encode_chat_intent(&ChatIntent::PrivateChannel(intent)).map_err(debug)?;
    let (result, spell) = wire::dispatch(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        *command,
        &payload,
    )
    .await
    .map_err(|_| "private wire rejected")?
    .ok_or("private unknown result")?;
    assert!(spell.is_none());
    assert_eq!(result.disposition, expected);
    *command += 1;
    Ok(())
}
