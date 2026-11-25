//! Guild MOTD through admitted command, login snapshot and Guild Own frames.
use super::fixture::{Harness, debug, id, run};
use super::guild_wire_fixtures::{exchange, exchange_capture};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::{fresh_admission::FreshAdmissionStore, guild_chat_fixture as seed};
use crate::foundation::MessageType;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
    item_fence_of,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    chat::{
        self,
        guild_room::{self, GuildRoomState},
    },
    guild::*,
};

const OLD: &str = "Previous message";
const NEW: &str = "Guild hunt tonight";

#[derive(Clone, Copy, Debug)]
enum Case {
    Sets,
    Clears,
    Member,
    Vice,
    Stale,
    OtherChannel,
    Relog,
    Length,
    Replay,
}

#[test]
fn gmotd_leader_sets() -> TestResult {
    run_case(Case::Sets)
}

#[test]
fn gmotd_leader_clears() -> TestResult {
    run_case(Case::Clears)
}

#[test]
fn gmotd_member_refused() -> TestResult {
    run_case(Case::Member)
}

#[test]
fn gmotd_vice_refused() -> TestResult {
    run_case(Case::Vice)
}

#[test]
fn gmotd_stale_session_refused() -> TestResult {
    run_case(Case::Stale)
}

#[test]
fn gmotd_other_member_sees_on_next_open() -> TestResult {
    run_case(Case::OtherChannel)
}

#[test]
fn gmotd_shown_on_channel_open_after_relog() -> TestResult {
    run_case(Case::Relog)
}

#[test]
fn gmotd_length_limit() -> TestResult {
    run_case(Case::Length)
}

#[test]
fn gmotd_replay_same_command_id() -> TestResult {
    run_case(Case::Replay)
}

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| {
                let database = format!("gmotd_{case:?}").to_ascii_lowercase();
                let h = Harness::create_with_admission_seed(admin, &database, true, false).await?;
                let result = scenario(&h, case).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Guild MOTD test thread panicked")??;
    Ok(())
}

// Encode raw fields so invalid DATA reaches the server independently of client validation.
fn payload(guild: GuildId, revision: u8, text: &str) -> Vec<u8> {
    let mut bytes = vec![10, 16];
    bytes.extend(guild.as_bytes());
    bytes.extend([16, revision, 26]);
    let len = text.len();
    if len >= 128 {
        bytes.push((len & 127) as u8 | 128);
        bytes.push((len >> 7) as u8);
    } else {
        bytes.push(len as u8);
    }
    bytes.extend(text.as_bytes());
    bytes
}

async fn edit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    bytes: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    exchange(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_GUILD_SET_MESSAGE,
        bytes,
    )
    .await
}

async fn own(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<(Option<GuildSummary>, GuildRoomState)> {
    let mut frames = Vec::new();
    let (status, reply, _, _) = exchange_capture(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_GUILD_QUERY,
        |_| Ok(encode_guild_query(&GuildQuery::Own).map_err(debug)?),
        false,
        &mut frames,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let GuildQueryResult::Own(summary) = decode_guild_query_result(&reply).map_err(debug)? else {
        return Err("expected Guild Own".into());
    };
    for frame in frames {
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) =
                oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            if let Some(domain) = domains.iter().find(|domain| {
                domain.domain_id == chat::STATE_DOMAIN_CHAT
                    && domain.snapshot_type == guild_room::SNAPSHOT_TYPE_GUILD_CHAT_V1
            }) {
                return Ok((
                    summary,
                    guild_room::decode_snapshot(domain.payload)
                        .map_err(debug)?
                        .guild,
                ));
            }
        }
    }
    Err("missing login Guild room snapshot".into())
}

fn receipt(reply: &(CommandStatus, Vec<u8>), code: u8, revision: u64) -> TestResult {
    assert_eq!(reply.0, CommandStatus::Accepted);
    let receipt = decode_guild_intent_result(&reply.1).map_err(debug)?;
    assert_eq!((receipt.code, receipt.revision), (code, Some(revision)));
    Ok(())
}

fn select(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7, 36, 40])
            .ok_or("Guild chat selection")?;
    Ok(())
}

async fn stored(h: &Harness, message: &str, revision: i64, writes: i64) -> TestResult {
    let state: (String, i64) = sqlx::query_as("SELECT motd, revision::bigint FROM game_guilds")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(state, (message.to_owned(), revision));
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_event_outbox")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(events, writes);
    Ok(())
}

async fn scenario(h: &Harness, case: Case) -> TestResult {
    prepare_current_session(h).await?;
    let second = if matches!(case, Case::OtherChannel) {
        Some(super::chat_world_fixture::admit_second_node(h).await?)
    } else {
        None
    };
    seed::prepare(&h.pool).await?;
    let guild = GuildId::from_bytes(seed::sid(90, 3)).map_err(debug)?;
    let player = CharacterId::from_bytes(id(41)).map_err(debug)?;
    let rank = match case {
        Case::Member => 3,
        Case::Vice => 2,
        _ => 1,
    };
    if rank == 1 {
        seed::initial_guild(&h.pool, guild, id(42), "Guild Notice", player, id(40)).await?;
    } else {
        let leader = CharacterId::from_bytes(seed::sid(90, 1)).map_err(debug)?;
        let account = seed::sid(90, 2);
        seed::historical_character(&h.pool, 90, leader, account, id(42), "Guild Leader").await?;
        seed::initial_guild(&h.pool, guild, id(42), "Guild Notice", leader, account).await?;
        seed::initial_member(&h.pool, guild, player, id(40), rank).await?;
    }
    let old = if matches!(case, Case::Length) {
        "🦉".repeat(255)
    } else {
        OLD.to_owned()
    };
    sqlx::query("UPDATE game_guilds SET motd=$1")
        .bind(&old)
        .execute(&h.pool)
        .await?;
    seed::guard(&h.pool).await?;
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/3317_guild_motd.sql"
    )))
    .execute(&h.pool)
    .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.item_fence = item_fence_of(current);
    select(&owner, &mut admitted)?;
    let initial = admitted;
    eprintln!(
        "GMOTD-PG {:?}: live admitted Guild command and Own assertions",
        case
    );

    match case {
        Case::Sets => {
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 1, NEW)).await?,
                1,
                2,
            )?;
            let (summary, _) = own(&owner, &mut admitted).await?;
            assert_eq!(summary.ok_or("editor's Guild Own")?.message, NEW);
            stored(h, NEW, 2, 1).await?;
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 1, OLD)).await?,
                13,
                2,
            )?;
            stored(h, NEW, 2, 1).await
        }
        Case::Clears => {
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 1, "")).await?,
                1,
                2,
            )?;
            let (summary, login) = own(&owner, &mut admitted).await?;
            assert!(summary.ok_or("cleared Guild Own")?.message.is_empty());
            assert!(matches!(login, GuildRoomState::Member { motd, .. } if motd.is_empty()));
            stored(h, "", 2, 1).await
        }
        Case::Member | Case::Vice => {
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 1, NEW)).await?,
                2,
                1,
            )?;
            stored(h, OLD, 1, 0).await
        }
        Case::Stale => {
            let mut stale_generation = admitted;
            let generation = stale_generation.continuity.connection_generation + 1;
            stale_generation.continuity.connection_generation = generation;
            stale_generation
                .item_fence
                .as_mut()
                .ok_or("stale fence")?
                .connection_generation =
                crate::foundation::ConnectionGeneration::new(generation).map_err(debug)?;
            let request = payload(guild, 1, NEW);
            // Exercise the durable connection-generation check independently of the
            // earlier admission recheck that would close this socket before dispatch.
            assert!(matches!(
                owner
                    .guild_wire(
                        stale_generation,
                        admitted.continuity.next_command_id,
                        COMMAND_TYPE_GUILD_SET_MESSAGE,
                        &request,
                    )
                    .await,
                crate::gameplay_transport::guild_intent::GuildReply::Rejected,
            ));

            // A matching connection with a stale Character lease reaches the same
            // durable session fence through the admitted wire command path.
            admitted
                .item_fence
                .as_mut()
                .ok_or("stale lease")?
                .character_lease_generation += 1;
            assert_eq!(
                edit(&owner, &mut admitted, &request).await?,
                (CommandStatus::Rejected, vec![])
            );
            stored(h, OLD, 1, 0).await
        }
        Case::OtherChannel => {
            let (node, receiver) = second.ok_or("second Channel owner")?;
            let other = receiver.owner_from(&h.root, &node, &authority);
            let mut recipient = receiver.admitted;
            select(&other, &mut recipient)?;
            assert_ne!(admitted.channel_id, recipient.channel_id);
            assert_eq!(
                own(&other, &mut recipient).await?,
                (None, GuildRoomState::Absent)
            );

            let target = *recipient
                .item_fence
                .ok_or("recipient fence")?
                .character_id
                .as_bytes();
            let name: String = sqlx::query_scalar(
                "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(target.as_slice())
            .fetch_one(&h.pool)
            .await?;
            let guild_bytes = *guild.as_bytes();
            let invite = encode_guild_intent(&GuildIntent::Invite {
                guild: guild_bytes,
                name,
            })
            .map_err(debug)?;
            let id = admitted.continuity.next_command_id;
            receipt(
                &exchange(
                    &owner,
                    &mut admitted,
                    id,
                    COMMAND_TYPE_GUILD_INTENT,
                    &invite,
                )
                .await?,
                1,
                2,
            )?;
            let join =
                encode_guild_intent(&GuildIntent::Join { guild: guild_bytes }).map_err(debug)?;
            let id = recipient.continuity.next_command_id;
            receipt(
                &exchange(&other, &mut recipient, id, COMMAND_TYPE_GUILD_INTENT, &join).await?,
                1,
                3,
            )?;
            let (before, _) = own(&other, &mut recipient).await?;
            assert_eq!(
                before.ok_or("other member's initial Guild Own")?.message,
                OLD
            );

            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 3, NEW)).await?,
                1,
                4,
            )?;
            let (summary, _) = own(&other, &mut recipient).await?;
            assert_eq!(
                summary.ok_or("other member's refreshed Guild Own")?.message,
                NEW
            );

            let kick = encode_guild_intent(&GuildIntent::Kick {
                guild: guild_bytes,
                character: target,
            })
            .map_err(debug)?;
            let id = admitted.continuity.next_command_id;
            receipt(
                &exchange(&owner, &mut admitted, id, COMMAND_TYPE_GUILD_INTENT, &kick).await?,
                1,
                5,
            )?;
            assert_eq!(
                own(&other, &mut recipient).await?,
                (None, GuildRoomState::Absent)
            );
            stored(h, NEW, 5, 4).await
        }
        Case::Relog => {
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 1, NEW)).await?,
                1,
                2,
            )?;
            super::track(&owner, &deps)?;
            assert_eq!(
                owner.logout_session(admitted).await,
                crate::gameplay_transport::connection::LogoutReply::Result(
                    oteryn_protocol_oteryn::logout::LogoutResult::Accepted,
                )
            );
            let session = super::full_world_support::relogin(h).await?;
            assert_ne!(session, admitted.game_session_id);
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session(session)
                .await?;
            let actor = {
                let mut runtime = owner.runtime.lock().await;
                let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                actor
            };
            deps.initialize_player(&owner, actor, session).await;
            let mut reloaded = AdmittedSession {
                game_session_id: session,
                runtime_actor: Some(actor),
                item_fence: item_fence_of(current),
                controller: None,
                ..initial
            };
            let (summary, login) = own(&owner, &mut reloaded).await?;
            assert_eq!(
                login,
                GuildRoomState::Member {
                    guild_id: *guild.as_bytes(),
                    rank: 1,
                    motd: NEW.into()
                }
            );
            assert_eq!(summary.ok_or("Guild Own after relog")?.message, NEW);
            Ok(())
        }
        Case::Length => {
            let (summary, login) = own(&owner, &mut admitted).await?;
            assert_eq!(summary.ok_or("historical Guild Own")?.message, old);
            assert!(matches!(login, GuildRoomState::Member { motd, .. } if motd == old));
            for invalid in [
                "x".repeat(256),
                "é".repeat(128),
                " padded ".repeat(32),
                "a\nb".into(),
                "a\u{85}b".into(),
                "a\0b".into(),
                " \u{a0} ".into(),
            ] {
                assert_eq!(
                    edit(&owner, &mut admitted, &payload(guild, 1, &invalid)).await?,
                    (CommandStatus::Rejected, vec![])
                );
            }
            stored(h, &old, 1, 0).await?;
            let ascii = "x".repeat(255);
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 1, &ascii)).await?,
                1,
                2,
            )?;
            let multibyte = format!("{}x", "é".repeat(127));
            receipt(
                &edit(&owner, &mut admitted, &payload(guild, 2, &multibyte)).await?,
                1,
                3,
            )?;
            stored(h, &multibyte, 3, 2).await
        }
        Case::Replay => {
            let request = payload(guild, 1, NEW);
            let command = admitted.continuity.next_command_id;
            let committed = edit(&owner, &mut admitted, &request).await?;
            receipt(&committed, 1, 2)?;
            assert_eq!(
                exchange(
                    &owner,
                    &mut admitted,
                    command,
                    COMMAND_TYPE_GUILD_SET_MESSAGE,
                    &request
                )
                .await?,
                (CommandStatus::DuplicateReplay, committed.1.clone())
            );
            let changed = payload(guild, 1, &format!(" {NEW}"));
            let conflict = exchange(
                &owner,
                &mut admitted,
                command,
                COMMAND_TYPE_GUILD_SET_MESSAGE,
                &changed,
            )
            .await;
            assert!(
                matches!(conflict, Err(error) if error.to_string().contains("CommandOutcomeExpired"))
            );
            drop(owner);
            let restarted = deps.owner(h, &authority);
            deps.initialize(&restarted).await;
            let mut replay = initial;
            assert_eq!(edit(&restarted, &mut replay, &request).await?, committed);
            drop(restarted);
            let restarted = deps.owner(h, &authority);
            deps.initialize(&restarted).await;
            assert_eq!(
                edit(&restarted, &mut { initial }, &changed).await?,
                (CommandStatus::Rejected, vec![])
            );
            stored(h, NEW, 2, 1).await
        }
    }
}
