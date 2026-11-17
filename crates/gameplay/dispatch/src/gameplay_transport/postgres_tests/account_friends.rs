//! T026: existing command envelope, independent account consent, real PostgreSQL owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome},
};
use oteryn_protocol_oteryn::{CommandStatus, vip};
async fn call(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    id: u64,
    operation: vip::Operation,
    number: u64,
    text: &str,
) -> TestResult<vip::ResultView> {
    let (kind, bytes) = vip::encode_request(&vip::Request {
        operation,
        number,
        text: text.into(),
        icon: 0,
        notify: false,
        groups: vec![],
    })
    .map_err(debug)?;
    let (status, result) =
        super::cyclopedia::support::exchange(owner, session, id, kind, &bytes).await?;
    assert_eq!(status, CommandStatus::Accepted);
    vip::decode_result(&result).map_err(debug)
}
#[test]
fn account_friends_postgres_main_consent_revocation_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "account Friends test thread failed")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "account_friends", true, false).await?;
        prepare_current_session(&h).await?;
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
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[43])
                .ok_or("VIP capability")?;
        use vip::Operation::*;
        let query = call(&owner, &mut admitted, 1, FriendsQuery, 0, "").await?;
        assert_eq!(query.code, 1);
        let view: serde_json::Value = serde_json::from_str(&query.view)?;
        assert!(view["main_character"].is_null());
        assert_eq!(
            call(&owner, &mut admitted, 2, MainCharacter, 0, "")
                .await?
                .code,
            1
        );
        let query = call(&owner, &mut admitted, 3, FriendsQuery, 0, "").await?;
        let view: serde_json::Value = serde_json::from_str(&query.view)?;
        let own: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(view["main_character"].as_str(), Some(own.as_str()));
        // A second account supplies the accepted counterpart; no character VIP grant is added.
        use crate::durability::guild_chat_fixture as characters;
        let peer = characters::sid(90, 1);
        let peer_account = characters::sid(90, 2);
        characters::historical_character(
            &h.pool,
            90,
            crate::domain::CharacterId::from_bytes(peer).map_err(debug)?,
            peer_account,
            id(42),
            "Alice",
        )
        .await?;
        let base = super::fixture::fence(1)?.item_fence();
        let peer_fence = characters::historical_session(
            &h.pool,
            90,
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: 1,
                ..base
            },
        )
        .await?;
        let peer_main = h
            .root
            .prepare_vip(
                &authority,
                &h.node,
                peer_fence,
                1,
                vip::Request {
                    operation: MainCharacter,
                    number: 0,
                    text: String::new(),
                    icon: 0,
                    notify: false,
                    groups: vec![],
                },
            )
            .await?;
        assert_eq!(commit(peer_main).await?["code"], 1);
        assert_eq!(
            call(&owner, &mut admitted, 4, FriendInvite, 0, "Alice")
                .await?
                .code,
            1
        );
        let query = call(&owner, &mut admitted, 5, FriendsQuery, 0, "").await?;
        let view: serde_json::Value = serde_json::from_str(&query.view)?;
        assert_eq!(view["account_friends"][0]["state"], "outgoing");
        assert!(view["account_friends"][0]["main_character"].is_null());
        let handle = view["account_friends"][0]["handle"]
            .as_u64()
            .ok_or("friend handle")?;
        assert_eq!(
            call(&owner, &mut admitted, 6, FriendAccept, handle, "")
                .await?
                .code,
            10,
            "sender cannot accept their own invitation"
        );
        let peer_query = h
            .root
            .prepare_vip(
                &authority,
                &h.node,
                peer_fence,
                2,
                vip::Request {
                    operation: FriendsQuery,
                    number: 0,
                    text: String::new(),
                    icon: 0,
                    notify: false,
                    groups: vec![],
                },
            )
            .await?;
        let peer_view = commit(peer_query).await?;
        let incoming = peer_view["view"]["account_friends"][0]["handle"]
            .as_u64()
            .ok_or("incoming handle")?;
        let accept = h
            .root
            .prepare_vip(
                &authority,
                &h.node,
                peer_fence,
                3,
                vip::Request {
                    operation: FriendAccept,
                    number: incoming,
                    text: String::new(),
                    icon: 0,
                    notify: false,
                    groups: vec![],
                },
            )
            .await?;
        assert_eq!(commit(accept).await?["code"], 1);
        let query = call(&owner, &mut admitted, 7, FriendsQuery, 0, "").await?;
        let view: serde_json::Value = serde_json::from_str(&query.view)?;
        assert_eq!(view["account_friends"][0]["main_character"], "Alice");
        let legacy: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_consents")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(legacy, 0);
        assert_eq!(
            call(&owner, &mut admitted, 8, FriendRevoke, handle, "")
                .await?
                .code,
            1
        );
        let query = call(&owner, &mut admitted, 9, FriendsQuery, 0, "").await?;
        let view: serde_json::Value = serde_json::from_str(&query.view)?;
        assert!(
            view["account_friends"]
                .as_array()
                .ok_or("friends")?
                .is_empty()
        );
        assert_eq!(
            call(&owner, &mut admitted, 10, FriendInvite, 0, "Alice")
                .await?
                .code,
            1
        );
        let query = call(&owner, &mut admitted, 11, FriendsQuery, 0, "").await?;
        let view: serde_json::Value = serde_json::from_str(&query.view)?;
        assert_ne!(
            view["account_friends"][0]["handle"].as_u64(),
            Some(handle),
            "deleted handles never select replacement invitations"
        );
        assert_eq!(
            call(&owner, &mut admitted, 12, FriendRevoke, handle, "")
                .await?
                .code,
            7
        );
        let fence = admitted.item_fence.ok_or("fence")?;
        let request = vip::Request {
            operation: MainCharacter,
            number: 0,
            text: String::new(),
            icon: 0,
            notify: false,
            groups: vec![],
        };
        let replay = h
            .root
            .prepare_vip(&authority, &h.node, fence, 2, request.clone())
            .await?;
        assert!(matches!(
            replay,
            crate::durability::vip::VipPrepared::Replayed(_)
        ));
        let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_lease_generation: fence.character_lease_generation + 1,
            ..fence
        };
        let refused = h
            .root
            .prepare_vip(&authority, &h.node, stale, 50, request)
            .await?;
        match refused {
            crate::durability::vip::VipPrepared::Replayed(value) => assert_eq!(value["code"], 10),
            _ => return Err("stale lease prepared a mutation".into()),
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn commit(prepared: crate::durability::vip::VipPrepared) -> TestResult<serde_json::Value> {
    match prepared {
        crate::durability::vip::VipPrepared::Replayed(value) => Ok(value),
        crate::durability::vip::VipPrepared::Fresh(write) => write
            .commit(crate::domain::premium::PremiumStatus::NotCurrent, None)
            .await
            .map_err(Into::into),
    }
}
