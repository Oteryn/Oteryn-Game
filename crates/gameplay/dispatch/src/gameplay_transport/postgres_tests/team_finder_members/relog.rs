//! Q3B: a genuine successor Fresh admission retains the template but needs a new upsert.
use super::support::Peer;
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};
use crate::durability::{
    admission_authority_guards::AdmissionGuardStore, fresh_admission::FreshAdmissionStore,
};
use crate::foundation::{
    GameSessionId,
    admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityGuardStateV1 as State,
        AdmissionAuthorityPublicationV1, AdmissionPublicationPreconditionV1,
    },
    fresh_admission_durability::FreshAdmissionDurableOutcomeV1,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{
        AdmittedSession, ConnectionEnd, ControllerBinding, FirstEntryOutcome, IDLE_LIVENESS,
        SessionContinuity, serve_admitted,
    },
    last_position::AdmissionPosition,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    team_finder::{self as leader, LeaderListing, TeamTarget},
    team_finder_members::{MemberStatus, TeamFinderQuery, TeamFinderResult},
};

pub(super) async fn check(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    previous: AdmittedSession,
    character: [u8; 16],
    account: [u8; 16],
) -> TestResult {
    // This is the existing full_world_e2e::relogin publication/commit sequence,
    // parameterized for the three-player Party fixture's actual identity.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let make_source = || {
        super::super::fresh_fixture::fresh::Source::appearance_observer(
            now,
            crate::gameplay_transport::canonical_uuid(&account),
            crate::foundation::CharacterId::decode(&character).map_err(debug)?,
        )
    };
    let mut source = make_source()?;
    let keys: Vec<_> = source.rows.iter().map(|row| row.key.clone()).collect();
    source.rows = guards
        .load(&keys)
        .await
        .map_err(|error| format!("relog load current publications: {error:?}"))?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current source publications")?;
    let mut refresh = make_source()?;
    refresh.rows = source
        .rows
        .iter()
        .filter(|row| matches!(row.key, Key::Account { .. } | Key::SigningTrust { .. }))
        .cloned()
        .collect();
    for row in &mut refresh.rows {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: row.publication_revision,
        };
        row.publication_revision = row
            .publication_revision
            .checked_add(1)
            .ok_or("publication revision")?;
        row.source.source_revision = row
            .source
            .source_revision
            .checked_add(1)
            .ok_or("source revision")?;
        row.source.decision_identity = format!("team-finder-relog-{}", row.source.source_revision);
        row.source.source_observed_at = now;
        if let State::Account { security, .. } = &mut row.state {
            security.provenance.publication_revision = row.publication_revision;
            security.provenance.source_revision = row.source.source_revision;
            security.provenance.accepted_source_revision = row.source.source_revision;
            security.provenance.decision_identity = row.source.decision_identity.clone();
            security.provenance.accepted_decision_identity = row.source.decision_identity.clone();
            security.provenance.source_observed_at = now;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&refresh, now)
        .map_err(|error| format!("relog prepare publication: {error:?}"))?;
    guards
        .publish(&publication)
        .await
        .map_err(|error| format!("relog publish refreshed authority: {error:?}"))?;
    source.rows = guards
        .load(&keys)
        .await
        .map_err(|error| format!("relog load refreshed publications: {error:?}"))?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("refreshed source publications")?;
    // The real multi-player fixture publishes world-policy-1, while the shared
    // observer fixture defaults to policy-1. Preserve the current Runtime owner's
    // exact decision instead of trying to republish it for this successor.
    source.current.world_policy_revision = source
        .rows
        .iter()
        .find_map(|row| {
            if let State::Runtime {
                world_policy_revision,
                ..
            } = &row.state
            {
                Some(world_policy_revision.clone())
            } else {
                None
            }
        })
        .ok_or("relog published Runtime world policy")?;
    let previous_lease = previous
        .item_fence
        .ok_or("old item fence")?
        .character_lease_generation;
    source.current.character_lease_generation = previous_lease;
    let request = super::fresh_request::prepare(&source, now)
        .map_err(|error| format!("relog prepare Fresh request: {error:?}"))?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let outcome = store
        .commit(&request)
        .await
        .map_err(|error| format!("relog commit Fresh request: {error:?}"))?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "relog Fresh outcome: {outcome:?}"
    );
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let current = store
        .current_session(session)
        .await
        .map_err(|error| format!("relog read committed successor: {error:?}"))?;
    assert_eq!(
        current.current_character_lease().character_id().as_bytes(),
        &character
    );
    assert_eq!(
        current.current_character_lease().generation(),
        previous_lease.checked_add(1).ok_or("successor lease")?
    );
    assert_ne!(session, previous.game_session_id);
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        actor
    };
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = current.current_connection_generation().get();
    continuity.selected_capabilities = previous.continuity.selected_capabilities;
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: previous.world_id,
        channel_id: previous.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("new controller")?,
            account_id: account,
        }),
        continuity,
        item_fence: crate::gameplay_transport::item_fence_of(current),
    };
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_loop_training(&mut admitted).await;
    assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
    assert!(
        owner.progression_binding(session).is_some(),
        "successor native progression binding"
    );
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("successor initialized gameplay fence")?;
    assert!(owner.track_restored(session, actor, &AdmissionPosition::FirstLogin, fence));
    assert_eq!(h.count("game_team_finder_leader_listings").await?, 1);
    assert_eq!(h.count("game_team_finder_active_listings").await?, 0);
    eprintln!("S14BC tf_logout stage=successor_wire");
    let (mut server, client) = tokio::io::duplex(65_536);
    let serving = async move { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async move {
        let mut peer = Peer::new(client, admitted);
        peer.snapshot().await?;
        assert_eq!(
            peer.query(TeamFinderQuery::Browse { after: None }).await?,
            TeamFinderResult::Browse {
                listings: vec![],
                next: None
            }
        );
        assert_eq!(
            peer.query(TeamFinderQuery::Applicants { after: None })
                .await?,
            TeamFinderResult::Applicants {
                listing: None,
                occupied_slots: 0,
                members: vec![],
                next: None
            }
        );
        assert_eq!(
            h.count("game_team_finder_active_listings").await?,
            0,
            "relogin queries cannot reactivate the retained template"
        );
        let listing = LeaderListing {
            min_level: 8,
            max_level: 120,
            vocation_mask: 15,
            slots: 4,
            free_slots: 4,
            party_members_join: false,
            timestamp: 1_700_000_000,
            target: TeamTarget::Boss(1234),
        };
        peer.action(
            leader::COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT,
            &leader::encode_upsert(listing).map_err(debug)?,
            CommandStatus::Accepted,
        )
        .await?;
        assert_eq!(h.count("game_team_finder_active_listings").await?, 1);
        assert_eq!(
            h.count("game_team_finder_members").await?,
            0,
            "previous session's requests never return"
        );
        let TeamFinderResult::Applicants {
            listing: restored,
            occupied_slots,
            members,
            next,
        } = peer
            .query(TeamFinderQuery::Applicants { after: None })
            .await?
        else {
            return Err("expected relogged leader view".into());
        };
        assert_eq!(restored, Some(listing));
        assert_eq!(occupied_slots, 0);
        assert_eq!(next, None);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].status, MemberStatus::Accepted);
        peer.shutdown().await
    };
    let (ended, result): (_, TestResult) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::super::fixture::join_two(serving, driving),
    )
    .await
    .map_err(|error| format!("successor connection deadline: {error}"))?;
    result.map_err(|error| format!("successor driver: {error}; connection end: {ended:?}"))?;
    assert!(
        matches!(ended, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{ended:?}"
    );
    Ok(())
}
