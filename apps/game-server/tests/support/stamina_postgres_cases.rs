//! Mounted by dispatch's hunting_stamina library case: real terminal logout and new login.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, TestResult, fresh_fixture, prepare_current_session, track};

#[test]
fn hunting_stamina_postgres_logout_login_preserves_pending_minutes() -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| run(async |admin| {
        let h = Harness::create_with_admission_seed(admin,"stamina_logout_login",true,false).await?;
        eprintln!("POSTGRES_STARTED STAMINA-1 logout/login");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h,&authority);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        track(&owner,&deps)?;
        let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
        let current = store.current_session(deps.admitted.game_session_id).await?;
        let lease_generation = current.current_character_lease().generation();
        let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states.load_stamina(&runtime,deps.actor,deps.admitted.game_session_id,2520);
            states.hunting_activity(&runtime,deps.actor,deps.admitted.game_session_id,0);
            states.hunting_activity(&runtime,deps.actor,deps.admitted.game_session_id,60_000_000);
        }
        assert_eq!(h.root.read_hunting_stamina(&authority,character).await.map_err(debug)?,2520);
        assert_eq!(owner.logout_session(deps.admitted).await,
            crate::gameplay_transport::logout::LogoutReply::Result(oteryn_protocol_oteryn::logout::LogoutResult::Accepted));
        assert_eq!(h.root.read_hunting_stamina(&authority,character).await.map_err(debug)?,2519);
        assert_eq!(store.current_session(deps.admitted.game_session_id).await?.session_state(),crate::foundation::GameSessionState::Terminal);
        let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
        let mut source = fresh_fixture::fresh::Source::new(now)?;
        let keys = source.rows.iter().map(|row|row.key.clone()).collect::<Vec<_>>();
        source.rows = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(h.root.clone()).load(&keys).await?.into_iter().collect::<Option<Vec<_>>>().ok_or("current publications")?;
        source.current.character_lease_generation = lease_generation;
        let request = source.appearance_observer_request(&crate::gameplay_transport::canonical_uuid(character.as_bytes()),now)?;
        assert!(matches!(store.commit(&request).await?,crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)));
        let next = store.current_session(crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?).await?;
        assert!(next.current_character_lease().generation()>lease_generation);
        // A new runtime actor reads the same canonical value after the fresh admission.
        assert_eq!(h.root.read_hunting_stamina(&authority,character).await.map_err(debug)?,2519);
        drop(owner); drop(authority); drop(seal);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let reopened = h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(h.root.read_hunting_stamina(&reopened,character).await.map_err(debug)?,2519);
        drop(reopened); drop(seal); h.cleanup().await
    }).map_err(|e|e.to_string()))?.join().map_err(|_|"stamina persistence panic")??;
    Ok(())
}
