//! Guild Leaders labels through the CHAT wire adapter and PostgreSQL World relay.
use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, join_two};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::chat::relay::{FRESHNESS_MICROS, RELAY_LEASE_MICROS, RelayKey, RelayKeyRing};
use crate::domain::CharacterId;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    chat_relay::ChatRelay,
    connection::{AdmittedSession, FreshAdmissionAuthority},
    guild_chat::wire,
    guild_intent::GuildReply,
    item_fence_of,
    spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture},
};
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::{
    chat::{self, ChatDisposition, ChatIntent, ChatLine, ChatRoom, ChatRoomSet},
    guild::{self, GuildIntent},
};
use std::{sync::Arc, time::Duration};

const ROOM: ChatRoom = ChatRoom::GuildLeaders;
const GUILD: &str = "Guild Leaders";

#[test]
fn guild_leaders_chat_postgres_labels_follow_current_guild_and_staff_facts() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "guild_leaders_facts", false)
}

#[test]
fn guild_leaders_chat_postgres_label_survives_cross_channel_delivery() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "guild_leaders_relay", true)
}

fn run(admin: String, database: &'static str, cross_channel: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h =
                        Harness::create_with_admission_seed(admin, database, true, false).await?;
                    let result = scenario(&h, cross_channel).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Guild Leaders test thread panicked")??;
    Ok(())
}

async fn scenario(h: &Harness, cross_channel: bool) -> TestResult {
    prepare_current_session(h).await?;
    let receiver = if cross_channel {
        Some(world::admit_second_node(h).await?)
    } else {
        None
    };
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut sender = Dependencies::new()?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(sender.admitted.game_session_id)
        .await?;
    let fence = item_fence_of(current).ok_or("sender fence")?;
    sender.admitted.item_fence = Some(fence);
    {
        let mut runtime = sender.runtime.lock().await;
        runtime
            .initialize_first_entry_position(sender.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                sender.actor,
                sender.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
    }
    let (consumer, clock, _, _) = purchase_fixture::current(id(40));
    let premium = SpellPremiumCoordinator::new(consumer, Some(clock), None, Some(h.root.clone()));
    let mut owner = sender.owner(h, &authority);
    owner.premium_coordinator = Some(&premium);
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [21; 32])]).map_err(debug)?;
    let options = h
        .root
        .chat_relay_connect_options()
        .ok_or("relay endpoint")?;
    let relay = Arc::new(ChatRelay::new(ring.clone(), id(42), options.clone()).map_err(debug)?);
    owner.states.chat.install_relay(relay.clone());
    let admitted = admit(&owner, sender.admitted).await?;
    let shutdown = CancellationToken::new();
    let listeners_timeout =
        Duration::from_micros(FRESHNESS_MICROS + RELAY_LEASE_MICROS) + Duration::from_secs(30);

    if let Some((node, receiver)) = receiver {
        let target = receiver.owner_from(&h.root, &node, &authority);
        assert_ne!(owner.channel_id, target.channel_id);
        let target_character = receiver
            .admitted
            .item_fence
            .ok_or("receiver fence")?
            .character_id;
        issue_access(h, target_character, 0, true, 90).await?;
        let target_relay = Arc::new(ChatRelay::new(ring, id(42), options).map_err(debug)?);
        target.states.chat.install_relay(target_relay.clone());
        let target_admitted = admit(&target, receiver.admitted).await?;
        tokio::select! {
            () = relay.run(&owner.states.chat, &shutdown) => Err("sender relay stopped".into()),
            () = target_relay.run(&target.states.chat, &shutdown) => Err("receiver relay stopped".into()),
            result = tokio::time::timeout(listeners_timeout, async {
                join_two(ready(&relay), ready(&target_relay)).await;
                found_guild(&owner, admitted).await?;
                open(&owner, admitted).await?;
                open(&target, target_admitted).await?;
                let message = "🦉".repeat(255);
                say(&owner, admitted, 102, &message).await?;
                let prefix = format!("{name} [{GUILD}]: ");
                let suffix = "🦉".repeat(255 - prefix.chars().count());
                assert_delivery(&target, target_admitted, &name, format!("{prefix}{suffix}")).await
            }) => result?,
        }
    } else {
        issue_access(h, fence.character_id, 0, true, 90).await?;
        tokio::select! {
            () = relay.run(&owner.states.chat, &shutdown) => Err("relay stopped".into()),
            result = tokio::time::timeout(listeners_timeout, async {
                ready(&relay).await;
                open(&owner, admitted).await?;
                say(&owner, admitted, 102, "First line").await?;
                assert_delivery(&owner, admitted, &name, format!("{name} [Staff]: First line")).await?;

                found_guild(&owner, admitted).await?;
                say(&owner, admitted, 103, "Second line").await?;
                assert_delivery(&owner, admitted, &name, format!("{name} [Staff][{GUILD}]: Second line")).await?;

                issue_access(h, fence.character_id, 1, false, 91).await?;
                say(&owner, admitted, 104, "[Staff] is player text").await?;
                assert_delivery(&owner, admitted, &name, format!("{name} [{GUILD}]: [Staff] is player text")).await
            }) => result?,
        }
    }
}

async fn admit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult<AdmittedSession> {
    let supported = [
        6,
        7,
        guild::CAPABILITY_GUILD_V1,
        80,
        chat::public_channels::CAPABILITY_CHAT_PUBLIC_CHANNELS_V1,
    ];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &supported)
            .ok_or("public channels selection")?;
    owner
        .chat_admit(
            admitted.runtime_actor.ok_or("chat actor")?,
            admitted.game_session_id,
        )
        .await;
    owner
        .public_chat_admit(admitted, ChatRoomSet::default())
        .await;
    Ok(admitted)
}

async fn issue_access(
    h: &Harness,
    character: CharacterId,
    expected: i64,
    access: bool,
    receipt: u8,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control")
        .execute(&mut *tx)
        .await?;
    let revision: i64 = sqlx::query_scalar(
        "SELECT game_chat_role_issue(encode($1,'hex')::uuid,$2,$3,$3,$4,encode($5,'hex')::uuid,'guild-leaders-fixture')",
    )
    .bind(character.as_bytes().as_slice())
    .bind(expected)
    .bind(if access { 4_i16 } else { 1 })
    .bind(access)
    .bind(id(receipt).as_slice())
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(revision, expected + 1);
    tx.commit().await?;
    Ok(())
}

async fn found_guild(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let payload =
        guild::encode_guild_intent(&GuildIntent::Found { name: GUILD.into() }).map_err(debug)?;
    let reply = owner
        .guild_wire(admitted, 100, guild::COMMAND_TYPE_GUILD_INTENT, &payload)
        .await;
    let GuildReply::Payload(result) = reply else {
        return Err(format!("Guild founding reply: {reply:?}").into());
    };
    assert_eq!(
        guild::decode_guild_intent_result(&result)
            .map_err(debug)?
            .code,
        1
    );
    Ok(())
}

async fn ready(relay: &ChatRelay) {
    while !relay.is_ready() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn open(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession) -> TestResult {
    dispatch(owner, admitted, 101, ChatIntent::OpenRoom(ROOM)).await
}

async fn say(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    text: &str,
) -> TestResult {
    dispatch(
        owner,
        admitted,
        command,
        ChatIntent::Room {
            room: ROOM,
            text: text.into(),
        },
    )
    .await
}

async fn dispatch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    intent: ChatIntent,
) -> TestResult {
    let payload = chat::encode_chat_intent(&intent).map_err(debug)?;
    let (result, spell) = wire::dispatch(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        command,
        &payload,
    )
    .await
    .map_err(|_| "CHAT wire rejected")?
    .ok_or("CHAT result")?;
    assert!(spell.is_none());
    assert_eq!(result.disposition, ChatDisposition::Ok);
    Ok(())
}

async fn assert_delivery(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    name: &str,
    text: String,
) -> TestResult {
    let line = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(line) = owner.chat_next_bound(admitted, &mut 64).await {
                break line;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert_eq!(
        line,
        ChatLine::Room {
            room: ROOM,
            speaker_name: name.into(),
            text
        }
    );
    owner.chat_delivered(admitted.game_session_id).await;
    Ok(())
}
