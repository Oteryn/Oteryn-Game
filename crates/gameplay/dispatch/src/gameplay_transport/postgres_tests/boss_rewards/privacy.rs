//! A live personal reward stays private between two independently admitted Characters.
use super::*;
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1,
};
use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, CharacterLease, GameSessionId, GameSessionState,
    RuntimeScopeRefV1,
};
use crate::gameplay_transport::connection::{ControllerBinding, SessionContinuity};
use oteryn_protocol_oteryn::boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN;

pub(super) async fn live_other_character(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    recipient: &mut AdmittedSession,
    recipient_fence: CurrentCharacterItemFence,
    death: [u8; 16],
) -> TestResult {
    let browse = encode_browse_request(&BrowseRequest {
        death,
        after_ordinal: 0,
    })
    .map_err(debug)?;
    let command = recipient.continuity.next_command_id;
    let (_, bytes) = exchange(
        owner,
        recipient,
        command,
        COMMAND_TYPE_BOSS_REWARD_BROWSE,
        &browse,
    )
    .await?;
    let own_before = checked_browse_reply("live recipient positive control", &bytes)?;
    assert_eq!(own_before.status, BossRewardStatus::Ok);
    assert_eq!(own_before.rows.len(), 4);
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    assert!(own_before.expires_at_unix_ms > u64::try_from(now)?);

    // Reuse the pager's real source-account refresh/publication/commit path.
    // No session, claim, lease, transport or admission-history row is fabricated.
    let evidence = super::super::house_transfer::RecoverySource::start(h).await?;
    let session = admit_peer(h, &evidence.evidence, 1).await?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let current = store.current_session_at(session).await?.0;
    assert_eq!(current.session_state(), GameSessionState::Active);
    let transport = current
        .current_transport()
        .ok_or("peer's committed transport")?;
    let peer_fence = item_fence_of(current).ok_or("peer's independently current fence")?;
    assert_ne!(peer_fence.character_id, recipient_fence.character_id);
    assert_ne!(peer_fence.game_session_id, recipient_fence.game_session_id);
    assert_eq!(peer_fence.runtime_scope, recipient_fence.runtime_scope);
    assert_eq!(
        peer_fence.scope_ownership_generation,
        recipient_fence.scope_ownership_generation
    );
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = peer_fence.runtime_scope
    else {
        return Err("independently admitted peer is in the actual Channel".into());
    };
    let account: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(account_id) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let account_id: [u8; 16] = account.try_into().map_err(|_| "peer account UUID")?;
    assert_ne!(
        account_id,
        recipient
            .controller
            .ok_or("recipient controller")?
            .account_id
    );
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(peer_fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let actor = {
        let _permit = owner
            .spell_lane_permit()
            .await
            .ok_or("current Channel lane")?;
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.binding().runtime_scope(), peer_fence.runtime_scope);
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    CharacterId::decode(peer_fence.character_id.as_bytes()).map_err(debug)?,
                    peer_fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let position = MovementLocalPosition {
            x: 7,
            y: 1,
            floor: 7,
        };
        assert_eq!(
            owner.movement_cells.index().lookup(
                owner.movement_cells.scope(),
                crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor)
                },
            ),
            Ok(crate::content::CollisionClass::Walkable)
        );
        assert!(
            !runtime
                .position_occupied_by_other(actor, position)
                .map_err(debug)?
        );
        // One-time placement of the newly committed actor on the actual test map,
        // adjacent to the same physical chest; this is not a reach/fence bypass.
        runtime
            .initialize_pinned_test_position(actor, position)
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, actor, session).await;
    let supported = [4, 6, 117, CAPABILITY_BOSS_REWARDS_V1];
    let selected = SelectedCapabilities::select(owner.offered_capabilities(), &supported)
        .ok_or("peer reward capability closure")?;
    assert_eq!(selected.as_slice(), supported.as_slice());
    assert_eq!(
        peer_fence.connection_generation.get(),
        SessionContinuity::FRESH.connection_generation
    );
    let mut continuity = SessionContinuity::FRESH;
    continuity.selected_capabilities = selected;
    let mut peer = AdmittedSession {
        game_session_id: session,
        world_id,
        channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport,
            account_id,
        }),
        continuity,
        item_fence: Some(peer_fence),
    };
    let before = reward_and_custody_snapshot(h, recipient_fence, death).await?;
    let (status, list) = exchange(owner, &mut peer, 1, COMMAND_TYPE_BOSS_REWARD_OPEN, &[]).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(
        list.len() < 18,
        "an empty personal list contains only the existing now field"
    );
    assert!(!list.windows(16).any(|bytes| bytes == death));
    let (_, bytes) = exchange(
        owner,
        &mut peer,
        2,
        COMMAND_TYPE_BOSS_REWARD_BROWSE,
        &browse,
    )
    .await?;
    let hidden = checked_browse_reply("valid peer cannot browse recipient's live group", &bytes)?;
    assert_eq!(hidden.status, BossRewardStatus::Empty);
    assert_eq!(hidden.expires_at_unix_ms, 0);
    assert!(hidden.rows.is_empty());
    assert!(!hidden.more);
    let ordinal = own_before
        .rows
        .first()
        .ok_or("known live reward ordinal")?
        .ordinal;
    let take = encode_take_request(&TakeRequest { death, ordinal }).map_err(debug)?;
    let (status, bytes) =
        exchange(owner, &mut peer, 3, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        checked_collect_reply("valid peer cannot take recipient's live Item", &bytes)?,
        CollectReply {
            status: BossRewardStatus::Empty,
            moved: 0,
            remaining: 0
        }
    );
    let (status, replay) =
        exchange(owner, &mut peer, 3, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(status, CommandStatus::DuplicateReplay);
    assert_eq!(replay, bytes);
    assert_eq!(
        reward_and_custody_snapshot(h, recipient_fence, death).await?,
        before
    );
    let command = recipient.continuity.next_command_id;
    let (_, bytes) = exchange(
        owner,
        recipient,
        command,
        COMMAND_TYPE_BOSS_REWARD_BROWSE,
        &browse,
    )
    .await?;
    assert_eq!(
        checked_browse_reply("recipient live group remains unchanged", &bytes)?,
        own_before
    );
    let current = store.current_session_at(session).await?.0;
    assert_eq!(current.session_state(), GameSessionState::Active);
    assert_eq!(item_fence_of(current), Some(peer_fence));
    eprintln!("BOSS_REWARDS_LIVE_CHARACTER_PRIVACY POSTGRES_CASE_COMPLETED");
    Ok(())
}

type RewardAndCustodySnapshot = (serde_json::Value, serde_json::Value, i64, i64, i64, i64);

async fn reward_and_custody_snapshot(
    h: &Harness,
    recipient: CurrentCharacterItemFence,
    death: [u8; 16],
) -> TestResult<RewardAndCustodySnapshot> {
    // Require the group to remain live at both samples. Empty refusal receipts
    // may be retained for the peer; they must create no Item custody or audit.
    Ok(sqlx::query_as(
        "SELECT to_jsonb(c), \
         (SELECT jsonb_agg(to_jsonb(i) ORDER BY i.item_ordinal) FROM game_boss_reward_items i \
          WHERE i.character_id=c.character_id AND i.world_id=c.world_id AND i.death_id=c.death_id), \
         (SELECT count(*) FROM game_boss_reward_item_collections), \
         (SELECT count(*) FROM game_item_instances), \
         (SELECT count(*) FROM game_item_container_entries), \
         (SELECT count(*) FROM game_item_audit_outbox) \
         FROM game_character_boss_reward_chests c WHERE c.character_id=encode($1,'hex')::uuid \
         AND c.world_id=encode($2,'hex')::uuid AND c.death_id=encode($3,'hex')::uuid \
         AND c.expires_at > floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
    )
    .bind(recipient.character_id.as_bytes().as_slice())
    .bind(recipient.runtime_scope.world_id().as_bytes().as_slice())
    .bind(death.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

// The existing deadline-pager admission pattern, kept local to this one test.
async fn admit_peer(
    h: &Harness,
    evidence: &crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
    member: u8,
) -> TestResult<GameSessionId> {
    let character = id(100 + member);
    let account = id(120 + member);
    // Only initial Character/progression input is fixture data, as in Harness.
    // Session, claim, ledger and transport facts are committed by their owner.
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(account.as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1',$4,NULL)")
        .bind(character.as_slice()).bind(account.as_slice()).bind(id(42).as_slice())
        .bind(format!("Fixture Member {}",char::from(b'A' + member))).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(character.as_slice()).execute(&h.pool).await?;
    let (account, character_text): (String, String) = sqlx::query_as("SELECT account_id::text,character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).fetch_one(&h.pool).await?;
    // The real producer-owned security floor is independent of published guards.
    evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::super::fresh_fixture::fresh::Source::appearance_observer(
        now,
        account,
        CharacterId::decode(&character).map_err(debug)?,
    )?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let keys = source
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.key,
                AdmissionAuthorityGuardKeyV1::Runtime(_)
                    | AdmissionAuthorityGuardKeyV1::SigningTrust { .. }
            )
        })
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    for prior in guards.load(&keys).await.map_err(debug)? {
        let prior = prior.ok_or("current global admission publication")?;
        let row = source
            .rows
            .iter_mut()
            .find(|row| row.key == prior.key)
            .ok_or("global fixture key")?;
        if matches!(row.key, AdmissionAuthorityGuardKeyV1::SigningTrust { .. }) {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: prior.publication_revision,
            };
            row.publication_revision = prior
                .publication_revision
                .checked_add(1)
                .ok_or("publication revision")?;
            row.source.source_revision = prior
                .source
                .source_revision
                .checked_add(1)
                .ok_or("source revision")?;
            row.source.decision_identity =
                format!("platform-observation-{}", row.source.source_revision);
        } else {
            *row = prior;
        }
    }
    for row in &mut source.rows {
        if matches!(
            row.key,
            AdmissionAuthorityGuardKeyV1::Account { .. }
                | AdmissionAuthorityGuardKeyV1::Character(_)
        ) {
            row.source.authority =
                crate::durability::fresh_admission_composition::COMPOSITION_SOURCE_AUTHORITY.into();
            if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                security.provenance.source_authority = "platform-test".into();
            }
        }
    }
    let rows = source.rows.clone();
    source
        .rows
        .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    guards
        .publish(&AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    source.rows = rows;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let session = GameSessionId::decode(&id(140 + member)).map_err(debug)?;
    let transport = AuthenticatedTransportRefV1::decode(&[150 + member; 16]).map_err(debug)?;
    let request = source.activity_room_request(
        session,
        transport,
        &character_text,
        [170 + member; 32],
        now,
    )?;
    let outcome = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&request)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "real privacy peer admission: {outcome:?}"
    );
    Ok(session)
}
