use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, run};
use super::guild_wire_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::guild::{GuildCommand, GuildOutcome, GuildResult};
use crate::durability::guild_chat_fixture as seed;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome},
    item_fence_of,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    guild::{COMMAND_TYPE_GUILD_QUERY, activity::*},
};

#[test]
fn guild_activity_postgres_member_only_keyset_and_retention() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move |_| scenario(admin).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "guild activity scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "guild_activity", true, false).await?;
    prepare_current_session(&h).await?;
    seed::prepare(&h.pool).await?;
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/3405_guild_activity_keyset.sql"
    )))
    .execute(&h.pool)
    .await?;
    let (second_node, second) = world::admit_second_node(&h).await?;
    let guild = GuildId::from_bytes(seed::sid(92, 3)).map_err(debug)?;
    seed::initial_guild(
        &h.pool,
        guild,
        id(42),
        "Activity Guild",
        CharacterId::from_bytes(id(41)).map_err(debug)?,
        id(40),
    )
    .await?;
    seed::guard(&h.pool).await?;
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
    let fence = admitted.item_fence.ok_or("current guild activity actor")?;
    let legacy = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 40])
        .ok_or("legacy guild selection")?;
    let selected = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[6, 40, CAPABILITY_GUILD_ACTIVITY_V1],
    )
    .ok_or("guild activity selection")?;
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let target = CharacterId::from_bytes(id(world::RECIPIENT)).map_err(debug)?;

    // 32 recent entries at a single timestamp exercise the event-id tie breaker.
    // The first, older invitation is written by the same fenced owner.
    let recent = ACTIVITY_PAGE_ENTRIES + 2;
    for index in 0..=recent {
        let op = if index % 2 == 0 {
            GuildCommand::Invite(target)
        } else {
            GuildCommand::Revoke(target)
        };
        let mut request = seed::request(1000 + u32::try_from(index)?, guild, fence, op)?;
        request.identity.occurred_at_unix_ms = if index == 0 { 1 } else { now };
        let outcome = h
            .root
            .commit_guild_membership(&authority, &h.node, fence, request)
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, GuildOutcome::Committed(receipt) if receipt.result == GuildResult::Ok)
        );
    }

    eprintln!("GUILD-ACTIVITY-PG committed history and executing member queries");
    let query = ActivityQuery {
        guild: *guild.as_bytes(),
        before: None,
    };
    admitted.continuity.selected_capabilities = legacy;
    assert_eq!(
        command(&owner, &mut admitted, query).await?.0,
        CommandStatus::Rejected
    );
    admitted.continuity.selected_capabilities = selected;
    let (status, bytes) = command(&owner, &mut admitted, query).await?;
    assert_eq!(
        status,
        CommandStatus::Accepted,
        "member activity section reaches guild dispatch"
    );
    let page = decode_activity_page(&bytes).map_err(debug)?;
    assert_eq!(page.entries.len(), ACTIVITY_PAGE_ENTRIES);
    assert_eq!(page.entries[0].kind, ActivityKind::Invited);
    assert_eq!(page.entries[0].target, world::RECIPIENT_NAME);
    assert_eq!(
        page.entries[0].event,
        seed::sid(1000 + u32::try_from(recent)?, 12)
    );
    let before = page.next.ok_or("activity continuation")?;
    let (status, bytes) = command(
        &owner,
        &mut admitted,
        ActivityQuery {
            before: Some(before),
            ..query
        },
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let page = decode_activity_page(&bytes).map_err(debug)?;
    assert_eq!(page.entries.len(), recent - ACTIVITY_PAGE_ENTRIES);
    assert_eq!(
        page.entries.last().ok_or("activity tail")?.event,
        seed::sid(1001, 12)
    );
    assert!(page.next.is_none());

    let mut other = second.owner(&h, &authority);
    other.holder = &second_node;
    second.initialize(&other).await;
    let mut nonmember = second.admitted;
    nonmember.continuity.selected_capabilities = selected;
    assert_eq!(
        command(&other, &mut nonmember, query).await?.0,
        CommandStatus::Rejected
    );
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    query: ActivityQuery,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let id = admitted.continuity.next_command_id;
    exchange(
        owner,
        admitted,
        id,
        COMMAND_TYPE_GUILD_QUERY,
        &encode_activity_query(&query).map_err(debug)?,
    )
    .await
}
