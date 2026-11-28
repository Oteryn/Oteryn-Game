//! Peer profiles respect account badge visibility across the owner's fresh admission.
use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, id};
use super::{Dependencies, TestResult, chat_world_fixture, debug, prepare_current_session, reset};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::{canonical_uuid, item_fence_of};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    Badge, CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_BADGES,
    COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, badge_visibility, profile,
};

#[test]
fn cyclopedia_peer_profile_hides_badge_after_owner_relogin() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(scenario(admin)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Cyclopedia peer profile scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "badge_peer_profile", true, false).await?;
    eprintln!("POSTGRES_STARTED cyclopedia_peer_profile_hides_badge_after_owner_relogin");
    prepare_current_session(&h).await?;
    let (peer_node, peer_deps) = chat_world_fixture::admit_second_node(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let owner = deps.owner(&h, &authority);
    deps.initialize(&owner).await;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = item_fence_of(store.current_session(admitted.game_session_id).await?);
    let capability = CAPABILITY_CYCLOPEDIA_CHARACTER_V1;
    let selected = SelectedCapabilities::select(
        &[OfferedCapability {
            id: capability,
            requires: &[],
        }],
        &[capability],
    )
    .ok_or("Cyclopedia capability")?;
    admitted.continuity.selected_capabilities = selected;

    let peer = peer_deps.owner_from(&h.root, &peer_node, &authority);
    peer_deps.initialize(&peer).await;
    let mut peer_session = peer_deps.admitted;
    peer_session.continuity.selected_capabilities = selected;
    let mut consent = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *consent)
        .await?;
    sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(chat_world_fixture::RECIPIENT).as_slice()).bind(id(41).as_slice()).execute(&mut *consent).await?;
    consent.commit().await?;

    let badge = Badge::FledgelingHero;
    sqlx::query("INSERT INTO game_account_badges(account_id,badge,earned_at_unix_ms) VALUES(encode($1,'hex')::uuid,$2,1)")
            .bind(id(40).as_slice()).bind(i16::from(badge.id())).execute(&h.pool).await?;
    let target = profile::request(Some(id(41)));
    let tab = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
    let (status, before) = exchange(&peer, &mut peer_session, 1, tab, &target).await?;
    assert_eq!(status, CommandStatus::Accepted, "the peer has consent");
    assert!(
        profile::Profile::decode(&before)
            .map_err(debug)?
            .badges
            .contains(&badge)
    );

    let hide = badge_visibility::Request::Set {
        badge,
        public: false,
    }
    .encode();
    let (status, hidden) = exchange(
        &owner,
        &mut admitted,
        1,
        COMMAND_TYPE_CYCLOPEDIA_BADGES,
        &hide,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(
        badge_visibility::Reply::decode(&hidden)
            .map_err(debug)?
            .hidden
            .contains(&badge)
    );
    let (status, own) = exchange(&owner, &mut admitted, 2, tab, &profile::request(None)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(
        profile::Profile::decode(&own)
            .map_err(debug)?
            .badges
            .contains(&badge)
    );
    let (status, after_hide) = exchange(&peer, &mut peer_session, 2, tab, &target).await?;
    assert_eq!(status, CommandStatus::Accepted);

    let current = store.current_session(admitted.game_session_id).await?;
    let released = store
        .release_logout_session(
            admitted.game_session_id,
            &canonical_uuid(&id(40)),
            current
                .current_transport()
                .ok_or("owner controller transport")?,
        )
        .await?;
    assert!(matches!(released, ExpiredLossReleaseV1::Released { .. }));
    let successor = super::full_world_support::relogin_as(&h, id(41), id(40), id(111)).await?;
    assert_ne!(successor, admitted.game_session_id);
    let (status, after_relogin) = exchange(&peer, &mut peer_session, 3, tab, &target).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(
        !profile::Profile::decode(&after_relogin)
            .map_err(debug)?
            .badges
            .contains(&badge),
        "the peer profile must omit the hidden badge after owner relogin"
    );
    assert!(
        !profile::Profile::decode(&after_hide)
            .map_err(debug)?
            .badges
            .contains(&badge),
        "the next peer profile must omit the badge immediately after the hide"
    );
    drop(peer);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
