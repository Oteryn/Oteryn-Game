//! A newly admitted controller's first achievements page can overlap a durability pass.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::achievement_catalogue::AccountAchievementsRequest;
use crate::durability::DurabilityRoot;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
use crate::gameplay_transport::connection::{
    AccountAchievementsReply, ConnectionIdentifiers, FirstEntryOutcome, FreshAdmissionAuthority,
    LogoutReply, admit_frame,
};
use oteryn_protocol_oteryn::account_achievements::{
    AchievementCompletion, decode_account_achievements_result,
};
use oteryn_protocol_oteryn::{ClientBootstrapValue, encode_client_bootstrap, logout::LogoutResult};
use std::future::{Future, poll_fn};
use std::task::Poll;

#[test]
fn p11_achievement_page_after_relog_waits_for_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run(async |_| scenario(admin).await).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "P11 achievement page scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "p11_achievement_page", true, false).await?;
    let recovery = RecoverySource::start(&h).await?;
    prepare_owned_session(&h).await?;
    seed_sibling_achievement(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &recovery.evidence;
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session(admitted.game_session_id)
            .await?,
    );
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted),
    );
    drop(owner);

    let next_deps = Dependencies::new()?;
    let mut owner = next_deps.owner(&h, &authority);
    owner.evidence = &recovery.evidence;
    let token = RecoverySource::fresh_token(91)?;
    let frame = encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: 1,
        character_id: CharacterId::decode(
            admitted
                .item_fence
                .ok_or("old item fence")?
                .character_id
                .as_bytes(),
        )
        .map_err(debug)?,
        admission_material: token.as_bytes(),
        client_build_id: "p11-achievements",
        supported_capabilities: &[],
    })
    .map_err(debug)?;
    let (mut server, _client) = tokio::io::duplex(1 << 20);
    let fresh = admit_frame(&mut server, &frame, &owner, &RelogIdentifiers)
        .await
        .map_err(debug)?;
    assert_ne!(fresh.game_session_id, admitted.game_session_id);
    assert!(matches!(
        fresh.first_entry,
        FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
    ));
    assert_eq!(fresh.continuity.next_command_id, 1);
    let controller = fresh.controller.ok_or("fresh controller")?;
    assert_eq!(controller.account_id, id(40));

    let payload = {
        let held = h.root.try_issue_semantic_pass()?;
        let query = owner.account_achievements(AccountAchievementsRequest {
            account_id: controller.account_id,
            page: 0,
            completion: AchievementCompletion::Completed,
            game_session_id: *fresh.game_session_id.as_bytes(),
            command_id: fresh.continuity.next_command_id,
        });
        let query = DurabilityRoot::with_holder_owner(query);
        let mut query = std::pin::pin!(query);
        let first = poll_fn(|cx| Poll::Ready(query.as_mut().poll(cx))).await;
        assert!(
            first.is_pending(),
            "busy holder refused the first page after relog: {first:?}"
        );
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        let reply = query.await;
        let AccountAchievementsReply::Page(payload) = reply else {
            return Err(format!("first page after relog returned {reply:?}").into());
        };
        payload
    };
    let page = decode_account_achievements_result(&payload).map_err(debug)?;
    assert_eq!(page.page, 0);
    assert_eq!(page.fact_count, 0, "the sibling's grant must stay off the relogged Character's page");
    assert!(page.rows.is_empty());
    eprintln!("P11 answered the first achievements page after relog and holder handoff");

    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn seed_sibling_achievement(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
          1,1,'profile-1','ruleset-1','content-1','starter-1','Sibling Hero')",
    )
    .bind(id(92).as_slice())
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_account_achievement_grant_requests VALUES
         ('oteryn:test-history', $1, 'oteryn:achievement/allow_cookies', 'r1',
          encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1)",
    )
    .bind(id(93).as_slice())
    .bind(id(40).as_slice())
    .bind(id(92).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_account_achievements VALUES
         (encode($1,'hex')::uuid, 'oteryn:achievement/allow_cookies', 'oteryn:test-history', $2)",
    )
    .bind(id(40).as_slice())
    .bind(id(93).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

struct RelogIdentifiers;
impl ConnectionIdentifiers for RelogIdentifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(91)).ok()
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
    }
}
