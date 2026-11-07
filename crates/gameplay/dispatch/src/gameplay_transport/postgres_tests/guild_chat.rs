//! Real Guild readers, admission, relay and connection; reuse the existing fresh owner.
use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::chat::relay::{
    RelayDestination, RelayKey, RelayKeyRing, RelayLine, ReplayWindow, channel_name,
};
use crate::domain::{
    CharacterId,
    guild::{GuildId, GuildRank},
};
use crate::durability::guild_chat_fixture as guild_fixture;
use crate::durability::{guild::*, item_transfer::CurrentCharacterItemFence};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::chat::{
    self, ChatDisposition, ChatIntent, ChatLine,
    guild_room::{self, GuildRoomState},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_snapshot_chunk,
    decode_state_delta, encode_client_command,
};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "guild_chat/custody_tests.rs"]
mod custody_tests;
#[path = "guild_chat/exclusion_race_tests.rs"]
mod exclusion_race_tests;
#[path = "guild_chat/fence_tests.rs"]
mod fence_tests;
#[path = "guild_chat/transitions_tests.rs"]
mod transitions_tests;
#[path = "guild_chat/wire_tests.rs"]
mod wire_tests;

fn gid(n: u32) -> TestResult<GuildId> {
    GuildId::from_bytes(guild_fixture::sid(n, 3)).map_err(debug)
}
use guild_fixture::request;
fn payload(guild: GuildId, text: &str) -> TestResult<RelayLine> {
    Ok(crate::gameplay_transport::chat_intent::guild_test_support::line(guild, text))
}

async fn serve(
    mut server: tokio::io::DuplexStream,
    admitted: AdmittedSession,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<AdmittedSession> {
    let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    // Close the peer even on an early refusal so the driver observes EOF immediately.
    drop(server);
    match end {
        ConnectionEnd::AdmittedThenDisconnected(ended) => Ok(ended),
        other => Err(format!("unexpected guild connection end: {other:?}").into()),
    }
}

#[test]
fn guild_chat_postgres_real_dispatch_login_relay_custody_exclusion_and_fences() -> TestResult {
    // Like every auto-mounted suite here (README.md), return early without routed PostgreSQL:
    // the workspace library legs run without it; the PostgreSQL harness legs run this scenario.
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guild chat e2e panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guild_chat_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        guild_fixture::prepare(&h.pool).await?;
        for n in [90, 91] {
            let character = CharacterId::from_bytes(guild_fixture::sid(n, 1)).map_err(debug)?;
            guild_fixture::historical_character(
                &h.pool,
                n,
                character,
                guild_fixture::sid(n, 2),
                id(42),
                if n == 90 {
                    "Guild Leader"
                } else {
                    "Other Leader"
                },
            )
            .await?;
            guild_fixture::initial_guild(
                &h.pool,
                gid(n)?,
                id(42),
                if n == 90 {
                    "First Guild"
                } else {
                    "Second Guild"
                },
                character,
                guild_fixture::sid(n, 2),
            )
            .await?;
        }
        guild_fixture::initial_member(
            &h.pool,
            gid(90)?,
            CharacterId::from_bytes(id(41)).map_err(debug)?,
            id(40),
            3,
        )
        .await?;
        sqlx::query(
            "UPDATE game_guilds SET motd='Welcome members' WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(gid(90)?.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
        let base = super::fixture::fence(1)?;
        let leader_base = CurrentCharacterItemFence {
            character_id: base.character_id,
            game_session_id: base.game_session_id,
            connection_generation: base.connection_generation,
            character_lease_generation: 1,
            runtime_scope: base.runtime_scope,
            scope_ownership_generation: base.scope_ownership_generation,
        };
        let leader = guild_fixture::historical_session(&h.pool, 90, leader_base).await?;
        let other_leader = guild_fixture::historical_session(&h.pool, 91, leader_base).await?;
        guild_fixture::guard(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7, 36])
                .ok_or("guild capability selected")?;
        let options = h
            .root
            .chat_relay_connect_options()
            .ok_or("relay endpoint")?;
        let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [1; 32])]).map_err(debug)?;
        let relay = crate::gameplay_transport::chat_relay::ChatRelay::new(
            ring.clone(),
            id(42),
            options.clone(),
        )
        .map_err(debug)?;
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint",
        )
        .fetch_one(&h.pool)
        .await?;
        relay.force_ready(u64::try_from(now)?);
        owner.states.chat.install_relay(Arc::new(relay));
        let mut listener = sqlx::postgres::PgListener::connect_with(&h.pool).await?;
        let channel = channel_name(&guild_fixture::uuid(id(42))).map_err(debug)?;
        listener.listen(&channel).await?;
        eprintln!("GUILD-CHAT-PG real connection, native Guild writes and relay active");
        let (accepted, admitted) =
            wire_tests::roundtrip(&h, &owner, admitted, &mut listener, &ring, &channel).await?;
        fence_tests::negatives(&owner, admitted).await?;
        let admitted = custody_tests::accept_before_exclude(&h, &owner, admitted, leader).await?;
        transitions_tests::move_and_disband(&h, &owner, admitted, other_leader, accepted).await?;
        drop(listener);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
