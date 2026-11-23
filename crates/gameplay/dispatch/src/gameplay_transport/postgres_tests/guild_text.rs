//! S03 text edits over a real admitted connection and the native Guild transactions.
use super::fixture::{Harness, debug, fence, id, run};
use super::guild_wire_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::{fresh_admission::FreshAdmissionStore, guild_chat_fixture as seed};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
    guild_intent::offer_guild_text,
    item_fence_of,
};
use oteryn_protocol_oteryn::{CommandStatus, guild as wire};

const EDIT: u32 = 6411;
const TEXT: u32 = 638;

#[test]
fn guild_text_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let start = std::time::Instant::now();
                let harness =
                    Harness::create_with_admission_seed(admin, "guild_text_dispatch", true, false)
                        .await?;
                let result = scenario(&harness).await;
                harness.cleanup().await?;
                eprintln!(
                    "S03_GUILD_TEXT_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
                    start.elapsed(),
                    result.is_ok()
                );
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "guild text dispatch test panicked")??;
    Ok(())
}

fn request(kind: u8, guild: GuildId, revision: u64, body: &[u8]) -> Vec<u8> {
    let mut bytes = vec![kind];
    bytes.extend_from_slice(guild.as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(body);
    bytes
}

async fn edit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    request: &[u8],
    expected: (u8, Option<u64>),
) -> TestResult<Vec<u8>> {
    let command = session.continuity.next_command_id;
    let (status, bytes) = exchange(owner, session, command, EDIT, request).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("S03 command {command}: expected Accepted, got {status:?}").into());
    }
    let result = wire::decode_guild_intent_result(&bytes).map_err(debug)?;
    assert_eq!((result.code, result.revision), expected);
    assert_eq!(result.guild.as_slice(), &request[1..17]);
    Ok(bytes)
}

async fn scenario(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    seed::prepare(&h.pool).await?;
    let guild = GuildId::from_bytes(seed::sid(80, 3)).map_err(debug)?;
    seed::initial_guild(
        &h.pool,
        guild,
        id(42),
        "Text Guild",
        CharacterId::from_bytes(id(41)).map_err(debug)?,
        id(40),
    )
    .await?;
    let mut guest_base = fence(1)?.item_fence();
    guest_base.character_lease_generation = 1;
    let mut guests = Vec::new();
    for (n, name, rank) in [(90, "Text Vice", 2), (91, "Text Member", 3)] {
        let character = CharacterId::from_bytes(seed::sid(n, 1)).map_err(debug)?;
        let account = seed::sid(n, 2);
        seed::historical_character(&h.pool, n, character, account, id(42), name).await?;
        seed::initial_member(&h.pool, guild, character, account, rank).await?;
        guests.push(seed::historical_session(&h.pool, n, guest_base).await?);
    }
    seed::guard(&h.pool).await?;

    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = guest_base.runtime_scope.world_id();
    let deps = Dependencies::with_content_and_capacity(
        crate::foundation::ChannelContentPin::test(world),
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        3,
    )?;
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
    let mut leader = deps.admitted;
    leader.first_entry = FirstEntryOutcome::Positioned;
    leader.item_fence = item_fence_of(current);
    let offered = offer_guild_text(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        owner.guild_text_available(),
    );
    let selected = SelectedCapabilities::select(&offered, &[wire::CAPABILITY_GUILD_V1, TEXT])
        .ok_or("guild selection")?;
    assert!(selected.contains(TEXT));
    assert!(SelectedCapabilities::select(&offered, &[TEXT]).is_some_and(|s| !s.contains(TEXT)));
    leader.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &[wire::CAPABILITY_GUILD_V1])
            .ok_or("legacy guild selection")?;
    let motd = request(1, guild, 1, b"Meet at the guildhall.");
    let legacy_id = leader.continuity.next_command_id;
    let (status, bytes) = exchange(&owner, &mut leader, legacy_id, EDIT, &motd).await?;
    assert_eq!(
        (status, bytes.as_slice()),
        (CommandStatus::Rejected, &[][..])
    );
    let initial_leader = AdmittedSession {
        continuity: crate::gameplay_transport::connection::SessionContinuity {
            selected_capabilities: selected,
            ..deps.admitted.continuity
        },
        ..leader
    };
    leader.continuity.selected_capabilities = selected;

    let mut sessions = Vec::new();
    for fence in guests {
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(fence.game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            actor
        };
        deps.initialize_player(&owner, actor, fence.game_session_id)
            .await;
        // Each GameSession starts at command 1 and sequence 0.
        let mut continuity = deps.admitted.continuity;
        continuity.selected_capabilities = selected;
        sessions.push(AdmittedSession {
            game_session_id: fence.game_session_id,
            runtime_actor: Some(actor),
            item_fence: Some(fence),
            controller: None,
            continuity,
            ..leader
        });
    }
    let mut vice = sessions[0];
    let mut member = sessions[1];
    edit(&owner, &mut member, &motd, (2, Some(1))).await?;
    edit(&owner, &mut vice, &motd, (1, Some(2))).await?;

    edit(&owner, &mut vice, &motd, (13, Some(2))).await?;

    let mut title_body = seed::sid(91, 1).to_vec();
    title_body.extend_from_slice(b"Trail Scout");
    let title = request(2, guild, 2, &title_body);
    edit(&owner, &mut vice, &title, (2, Some(2))).await?;
    let title_id = leader.continuity.next_command_id;
    let receipt = edit(&owner, &mut leader, &title, (1, Some(3))).await?;

    // The latest unacknowledged reply survives transport replacement.
    assert_eq!(
        exchange(&owner, &mut leader, title_id, EDIT, &title).await?,
        (CommandStatus::DuplicateReplay, receipt)
    );
    edit(&owner, &mut leader, &title, (13, Some(3))).await?;

    let ranks_body = b"\x03\x06Leader\x0bVice Leader\x06Ranger\x07Visitor";
    let ranks = request(3, guild, 3, ranks_body);
    edit(&owner, &mut vice, &ranks, (2, Some(3))).await?;
    for attempted in [
        request(3, guild, 3, b"\x03\x06Leader\x0bVice Leader\x06Ranger"),
        request(
            3,
            guild,
            3,
            b"\x03\x06Chief!\x0bVice Leader\x06Ranger\x07Visitor",
        ),
        request(3, guild, 4, ranks_body),
    ] {
        let command = leader.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut leader, command, EDIT, &attempted).await?,
            (CommandStatus::Rejected, Vec::new())
        );
    }
    edit(
        &owner,
        &mut leader,
        &request(3, guild, 2, ranks_body),
        (13, Some(3)),
    )
    .await?;
    let rank_id = leader.continuity.next_command_id;
    let rank_receipt = edit(&owner, &mut leader, &ranks, (1, Some(4))).await?;

    // A new ingress owner has no retained command results. Advance its stream with reads
    // to the original rank command ID, so the rank reply must come from PostgreSQL.
    let restarted = deps.owner(h, &authority);
    deps.initialize(&restarted).await;
    let mut rank_retry = initial_leader;
    let own_query = wire::encode_guild_query(&wire::GuildQuery::Own).map_err(debug)?;
    for command in rank_retry.continuity.next_command_id..rank_id {
        let (status, _) = exchange(
            &restarted,
            &mut rank_retry,
            command,
            wire::COMMAND_TYPE_GUILD_QUERY,
            &own_query,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
    }
    assert_eq!(
        edit(&restarted, &mut rank_retry, &ranks, (1, Some(4))).await?,
        rank_receipt
    );
    let audit: Vec<(i16, String)> = sqlx::query_as(
        "SELECT o.action, o.revision::text FROM game_guild_operations o
         JOIN game_guild_event_outbox e USING (occurrence) ORDER BY o.revision",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(audit, [(8, "2".into()), (7, "3".into()), (11, "4".into())]);
    let receipts: Vec<(i16, i64)> = sqlx::query_as(
        "SELECT result, count(*) FROM game_guild_operations GROUP BY result ORDER BY result",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(receipts, [(0, 3), (1, 3), (12, 3)]);

    // A new transport reads the persisted guild summary through the existing query.
    let query_id = member.continuity.next_command_id;
    let (status, bytes) = exchange(
        &owner,
        &mut member,
        query_id,
        wire::COMMAND_TYPE_GUILD_QUERY,
        &own_query,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let wire::GuildQueryResult::Own(Some(summary)) =
        wire::decode_guild_query_result(&bytes).map_err(debug)?
    else {
        return Err("missing own guild summary".into());
    };
    assert_eq!(summary.guild, *guild.as_bytes());
    assert_eq!(summary.revision, 4);
    assert_eq!(summary.title, "Trail Scout");
    assert_eq!(summary.message, "Meet at the guildhall.");
    assert_eq!(
        summary.ranks,
        ["Leader", "Vice Leader", "Ranger", "Visitor"]
    );
    Ok(())
}
