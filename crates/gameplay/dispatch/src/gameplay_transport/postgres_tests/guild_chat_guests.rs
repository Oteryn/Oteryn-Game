use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, run};
use super::guild_wire_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::guild_chat_fixture as guild;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
    item_fence_of,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    chat::{
        self, ChatDisposition, ChatLine,
        guild_guest::{self, GuildGuestIntent},
    },
};

#[test]
fn guild_chat_guests_postgres_cross_channel_exclusion_and_logout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move |_| scenario(admin).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "guild guests scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "guild_guests", true, false).await?;
    prepare_current_session(&h).await?;
    // Rebuild the historical guild fixture and reinstall the guest FK and cleanup trigger.
    sqlx::query("DROP TABLE game_guild_chat_guests")
        .execute(&h.pool)
        .await?;
    guild::prepare(&h.pool).await?;
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/3404_guild_chat_guests.sql"
    )))
    .execute(&h.pool)
    .await?;
    let (second_node, second) = world::admit_second_node(&h).await?;
    let guild_id = crate::domain::guild::GuildId::from_bytes(guild::sid(90, 3)).map_err(debug)?;
    let leader_character =
        crate::domain::CharacterId::from_bytes(guild::sid(90, 1)).map_err(debug)?;
    guild::historical_character(
        &h.pool,
        90,
        leader_character,
        guild::sid(90, 2),
        id(42),
        "Guild Leader",
    )
    .await?;
    guild::initial_guild(
        &h.pool,
        guild_id,
        id(42),
        "Guest Guild",
        leader_character,
        guild::sid(90, 2),
    )
    .await?;
    guild::initial_member(
        &h.pool,
        guild_id,
        crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        id(40),
        2,
    )
    .await?;
    guild::guard(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let first = Dependencies::new()?;
    let owner = first.owner(&h, &authority);
    first.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(first.actor)
        .map_err(debug)?;
    let mut target = second.owner(&h, &authority);
    target.holder = &second_node;
    second.initialize(&target).await;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(first.admitted.game_session_id)
            .await?;
    let mut vice = first.admitted;
    vice.first_entry = FirstEntryOutcome::Positioned;
    vice.item_fence = item_fence_of(current);
    let mut visitor = second.admitted;
    let supported = [6, 7, 36, 40, guild_guest::CAPABILITY_CHAT_GUILD_GUEST_V1];
    vice.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &supported)
            .ok_or("guild guest selection")?;
    visitor.continuity.selected_capabilities = vice.continuity.selected_capabilities;
    let character = id(world::RECIPIENT);
    let invite = GuildGuestIntent::Invite { character };
    assert_eq!(
        disposition(
            &target,
            &mut visitor,
            &GuildGuestIntent::Invite { character: id(41) }
        )
        .await?,
        ChatDisposition::NotAllowed,
        "outsider cannot grant access"
    );
    eprintln!("GUILD-GUEST-PG executing negotiated invite across Channels");
    assert_eq!(
        disposition(&owner, &mut vice, &invite).await?,
        ChatDisposition::Ok
    );
    let id = visitor.continuity.next_command_id;
    let (status, bytes) = exchange(
        &target,
        &mut visitor,
        id,
        oteryn_protocol_oteryn::guild::COMMAND_TYPE_GUILD_QUERY,
        &guild_guest::encode_rooms_query(None).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        guild_guest::decode_rooms(&bytes).map_err(debug)?,
        [guild_guest::GuestRoom {
            guild: *guild_id.as_bytes(),
            name: "Guest Guild".into()
        }]
    );

    target
        .chat_admit(second.actor, visitor.game_session_id)
        .await;
    target.guild_chat_admit(visitor).await;
    let mut line = crate::gameplay_transport::chat_intent::guild_test_support::line(
        guild_id,
        "Guest-visible line",
    );
    line.sent_at_micros = 1;
    target.states.chat.deliver(&line);
    assert!(target.chat_next_bound(visitor, &mut 8).await.is_none());
    let sent_at: i64 = sqlx::query_scalar("SELECT granted_at_us FROM game_guild_chat_guests")
        .fetch_one(&h.pool)
        .await?;
    line.sent_at_micros = u64::try_from(sent_at)? + 1;
    target.states.chat.deliver(&line);
    assert!(matches!(target.chat_next_bound(visitor, &mut 8).await,
        Some(ChatLine::Guild(line)) if line.text == "Guest-visible line"));
    target.chat_delivered(visitor.game_session_id).await;

    guest_speech(
        &h,
        &owner,
        &target,
        vice,
        &mut visitor,
        guild_id,
        line.sent_at_micros,
    )
    .await?;
    target
        .chat_admit(second.actor, visitor.game_session_id)
        .await;
    target.guild_chat_admit(visitor).await;
    target.states.chat.deliver(&line);
    assert_eq!(
        disposition(&owner, &mut vice, &GuildGuestIntent::Exclude { character }).await?,
        ChatDisposition::Ok
    );
    assert!(
        target.chat_next_bound(visitor, &mut 8).await.is_none(),
        "queued line after exclusion"
    );
    assert_eq!(
        disposition(&owner, &mut vice, &invite).await?,
        ChatDisposition::Ok
    );
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL
        WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(visitor.game_session_id.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_chat_guests")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(remaining, 0, "terminal session removes guest grants");
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    intent: &GuildGuestIntent,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let id = admitted.continuity.next_command_id;
    exchange(
        owner,
        admitted,
        id,
        chat::COMMAND_TYPE_CHAT_INTENT,
        &guild_guest::encode_intent(intent).map_err(debug)?,
    )
    .await
}

async fn disposition(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    intent: &GuildGuestIntent,
) -> TestResult<ChatDisposition> {
    let (status, bytes) = command(owner, admitted, intent).await?;
    assert_eq!(
        status,
        CommandStatus::Accepted,
        "guild guest command reaches dispatch"
    );
    Ok(chat::decode_chat_intent_result(&bytes)
        .map_err(debug)?
        .disposition)
}

async fn guest_speech(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    target: &ComposedFreshAdmission<'_, '_, '_>,
    vice: AdmittedSession,
    visitor: &mut AdmittedSession,
    guild_id: crate::domain::guild::GuildId,
    now: u64,
) -> TestResult {
    use crate::chat::relay::{
        RelayDestination, RelayKey, RelayKeyRing, ReplayWindow, channel_name,
    };
    let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [1; 32])]).map_err(debug)?;
    let options = h
        .root
        .chat_relay_connect_options()
        .ok_or("relay endpoint")?;
    let relay =
        crate::gameplay_transport::chat_relay::ChatRelay::new(ring.clone(), id(42), options)
            .map_err(debug)?;
    relay.force_ready(now);
    target.states.chat.install_relay(std::sync::Arc::new(relay));
    let mut listener = sqlx::postgres::PgListener::connect_with(&h.pool).await?;
    let channel = channel_name(&guild::uuid(id(42))).map_err(debug)?;
    listener.listen(&channel).await?;
    let send = GuildGuestIntent::Send {
        guild: *guild_id.as_bytes(),
        text: "Hello from a guest".into(),
    };
    assert_eq!(
        disposition(target, visitor, &send).await?,
        ChatDisposition::Ok
    );
    let notification =
        tokio::time::timeout(std::time::Duration::from_secs(5), listener.recv()).await??;
    let current: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let line = ReplayWindow::starting_at(0)
        .receive(
            &ring,
            &channel,
            notification.payload(),
            u64::try_from(current)?,
        )
        .map_err(debug)?;
    assert_eq!(line.destination, RelayDestination::GuildGuest { guild_id });
    owner
        .chat_admit(
            vice.runtime_actor.ok_or("vice actor")?,
            vice.game_session_id,
        )
        .await;
    owner.guild_chat_admit(vice).await;
    owner.states.chat.deliver(&line);
    let routed = owner
        .chat_next_bound(vice, &mut 8)
        .await
        .ok_or("guest speech delivery")?;
    let bytes = chat::encode_chat_line(&routed).map_err(debug)?;
    assert!(matches!(chat::decode_chat_line(&bytes).map_err(debug)?,
        ChatLine::Guild(line) if line.rank == guild_guest::GUILD_GUEST_RANK && line.text == "Hello from a guest"));
    owner.chat_delivered(vice.game_session_id).await;
    Ok(())
}
