use super::fixture::{Harness, debug, id, run};

use super::{Dependencies, TestResult};

use crate::domain::CharacterRevision;

use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterOutcome, ActivityEnterRequest, ActivityHandoffOwner,
    ActivityParticipant,
};

use crate::durability::character_last_position::{
    LastPositionCell, LastPositionRecord, LastVitals,
};

use crate::durability::character_progression::CurrentCharacterGameplayFence;

use crate::durability::fresh_admission::FreshAdmissionStore;

use crate::durability::soulpit::{
    SOULPIT_ACTIVITY_KEY, SoulpitEntryRequest, SoulpitParticipantFact,
};

use crate::foundation::{
    AuthenticatedTransportRefV1, CommandId, CommandRef, GameSessionId, GameSessionState,
};

use crate::gameplay_transport::item_fence_of;

#[path = "soulpit_shared_entry/native.rs"]
pub(super) mod native;

pub(super) struct AdmittedSoulpit {
    pub(super) handoff_id: [u8; 16],
    pub(super) destination: CurrentCharacterGameplayFence,
}

/// Keeps every admission, session, custody, audit and deferred group guard enabled.
/// Only initial inventory/content use the existing migration-owned fixture seed.
pub(super) async fn prepare_source(
    h: &Harness,
) -> TestResult<super::house_transfer_fixtures::RecoverySource> {
    // The shared successor owner transfers the real composition publisher's
    // Account/Character claims. A bare raw fresh receipt cannot authorize that.
    let evidence = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    super::house_transfer_fixtures::prepare_owned_session(h).await?;
    let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    evidence
        .evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    Ok(evidence)
}

pub(super) async fn admit(
    h: &Harness,
    deps: &Dependencies,
    evidence: &super::house_transfer_fixtures::RecoverySource,
) -> TestResult<AdmittedSoulpit> {
    admit_inner(h, deps, evidence, false).await
}

pub(super) async fn admit_inner(
    h: &Harness,
    deps: &Dependencies,
    evidence: &super::house_transfer_fixtures::RecoverySource,
    gm_case: bool,
) -> TestResult<AdmittedSoulpit> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let source = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await
            .map_err(|error| format!("Soulpit shared source session: {error:?}"))?
            .0,
    )
    .ok_or("source item fence")?;
    let world = source.runtime_scope.world_id();
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native Soulpit content")?;
    let core = native
        .creature_profiles()
        .records
        .iter()
        .find_map(|record| {
            let crate::content::ProjectV2AuthoringProfileData::Creature(profile) =
                &record.profile.data
            else {
                return None;
            };
            let core = profile.details.as_ref()?.soul_core_item.as_ref()?;
            let policy = native.item_policy(&core.key, &core.revision)?.record();
            let source_id = policy.production_binding.external_id.parse::<u32>().ok()?;
            (core.revision == "definition-r1"
                && crate::domain::soulpit::is_soul_core(source_id)
                && policy.admission.materializable)
                .then(|| core.key.clone())
        })
        .ok_or("materializable Soul Core")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(source.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    super::item_look_fixtures::seed_main_backpack(h, &[(&core, 2)]).await?;
    let scope = super::activity_runtime_fixtures::allocate_ready_instance(h, 88)
        .await
        .map_err(|error| format!("Soulpit shared allocate ready Instance: {error:?}"))?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    {
        let mut tx = h.pool.begin().await?;
        let node_history: bool =
            sqlx::query_scalar("SELECT game_node_registration_history_valid()")
                .fetch_one(&mut *tx)
                .await?;
        let node_proven = crate::durability::runtime_scope_assignment::prove_current_incarnation(
            &mut tx, &h.node,
        )
        .await
        .map_err(|error| format!("Soulpit shared node proof diagnostic: {error:?}"))?;
        tx.rollback().await?;
        assert!(
            node_history,
            "Soulpit shared node registration history must be valid"
        );
        assert!(
            node_proven,
            "Soulpit shared node incarnation proof must be current"
        );
    }
    let allocation = owner
        .ready_allocations(&authority, &h.node, world, None)
        .await
        .map_err(|error| format!("Soulpit shared ready allocations: {error:?}"))?
        .into_iter()
        .find(|allocation| allocation.scope == scope)
        .ok_or("ready Soulpit allocation")?;
    let position = LastPositionRecord {
        world_id: world,
        cell: LastPositionCell {
            x: 32375,
            y: 31158,
            floor: 8,
        },
        coordinate_frame_digest: pin.frame_binding().digest(),
        map_revision_digest: pin.qualified_room().map_revision_digest(),
        server_artifact_digest: pin.identity().server_artifact_digest(),
        vitals: LastVitals::new(150, 50),
        rest: None,
    };
    let group = ActivityEnterRequest {
        handoff_id: id(92),
        command_id: 1,
        initiating_character_id: crate::foundation::CharacterId::decode(
            source.character_id.as_bytes(),
        )
        .map_err(debug)?,
        activity_key: SOULPIT_ACTIVITY_KEY.into(),
        template_revision: "soulpit-template-1".into(),
        content_revision: "content-1".into(),
        destination_scope: scope,
        destination_generation: allocation.generation,
        destination_assignment_source_revision: allocation.source_revision,
        destination_content: ActivityContentPin {
            coordinate_frame_digest: position.coordinate_frame_digest,
            map_revision_digest: position.map_revision_digest,
            server_artifact_digest: position.server_artifact_digest,
        },
        time_limit_ms: 600_000,
        participants: vec![ActivityParticipant {
            source,
            destination_game_session_id: GameSessionId::decode(&id(93)).map_err(debug)?,
            destination_transport: Some(
                AuthenticatedTransportRefV1::decode(&[93; 16]).map_err(debug)?,
            ),
            selected_capabilities: vec![117],
            source_position: position,
            source_position_write_sequence: 1,
            destination_cell: LastPositionCell {
                x: 32373,
                y: 31151,
                floor: 8,
            },
            exit_position: LastPositionRecord {
                cell: LastPositionCell {
                    x: 32373,
                    y: 31158,
                    floor: 8,
                },
                ..position
            },
        }],
    };
    let entry = SoulpitEntryRequest {
        funding: crate::durability::soulpit::SoulpitFunding::Core { source: id(71), definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: core.clone(),
            revision_ref: "definition-r1".into(),
        } },
        handoff_id: group.handoff_id,
        command: CommandRef::new(source.game_session_id, CommandId::new(1).map_err(debug)?),
        intent: vec![1, 2, 3],
        started_at_micros: 0,
        participants: vec![SoulpitParticipantFact {
            character: source.character_id,
            game_master: false,
        }],
    };
    let composed = deps.owner(h, &authority);
    let permit = composed
        .spell_lane_permit()
        .await
        .ok_or("current source spell lane")?;
    let _staff = if gm_case {
        Some(
            staff::GmProof {
                h,
                authority: &authority,
                source,
                group: &group,
                entry: &entry,
                owner: &owner,
                permit: &permit,
                native,
            }
            .verify()
            .await?,
        )
    } else {
        None
    };
    assert_eq!(
        owner
            .enter(&authority, &h.node, group.clone())
            .await
            .map_err(|error| format!("Soulpit shared generic bypass refusal: {error:?}"))?,
        ActivityEnterOutcome::Refused(
            crate::durability::activity_handoff::ActivityRefusal::InvalidInput
        ),
        "generic entry must not bypass the Soul Core debit"
    );
    let mut stale_definition = entry.clone();
    if let crate::durability::soulpit::SoulpitFunding::Core { definition, .. } = &mut stale_definition.funding { definition.revision_ref="definition-r2".into(); } else { return Err("core fixture".into()); }
    assert_eq!(
        owner
            .enter_soulpit(
                &authority,
                &h.node,
                &permit,
                group.clone(),
                &stale_definition,
                native
            )
            .await
            .map_err(|error| format!("Soulpit shared stale definition refusal: {error:?}"))?,
        ActivityEnterOutcome::Refused(
            crate::durability::activity_handoff::ActivityRefusal::AuthorityRejected
        ),
        "a custody definition changed since preflight must refuse the whole handoff"
    );
    assert_eq!(entry_counts(h).await?, (2, 0, 0, 0));
    // A failure after the debit and before successor issuance must preserve the source
    // and every custody/audit row. No replication-role change is used for this proof.
    sqlx::query("CREATE SEQUENCE soulpit_test_after_debit")
        .execute(&h.pool)
        .await?;
    sqlx::query("GRANT USAGE,SELECT ON SEQUENCE soulpit_test_after_debit TO oteryn_game_runtime")
        .execute(&h.pool)
        .await?;
    sqlx::query("CREATE FUNCTION soulpit_test_reject_destination() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.runtime_scope_kind=2 THEN IF (SELECT count(*) FROM game_soulpit_encounters)<>1 OR (SELECT count(*) FROM game_item_craft_records)<>1 OR (SELECT count(*) FROM game_item_audit_outbox)<>1 THEN RAISE EXCEPTION 'Soulpit probe did not observe complete debit'; END IF; PERFORM nextval('soulpit_test_after_debit'); RAISE EXCEPTION 'Soulpit destination failure' USING ERRCODE='23514'; END IF; RETURN NEW; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER soulpit_test_reject_destination BEFORE INSERT ON game_durability_reconnect_sessions FOR EACH ROW EXECUTE FUNCTION soulpit_test_reject_destination()")
        .execute(&h.pool).await?;
    let rejected = owner
        .enter_soulpit(&authority, &h.node, &permit, group.clone(), &entry, native)
        .await;
    assert!(
        rejected.is_err(),
        "injected successor failure must roll back entry: {rejected:?}"
    );
    let reached_debit: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM soulpit_test_after_debit")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        reached_debit,
        (1, true),
        "failure must occur after the complete debit, not at an earlier admission guard"
    );
    assert_eq!(entry_counts(h).await?, (2, 0, 0, 0));
    assert_eq!(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(source.game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Active
    );
    sqlx::query(
        "DROP TRIGGER soulpit_test_reject_destination ON game_durability_reconnect_sessions",
    )
    .execute(&h.pool)
    .await?;
    sqlx::query("DROP FUNCTION soulpit_test_reject_destination()")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP SEQUENCE soulpit_test_after_debit")
        .execute(&h.pool)
        .await?;
    // Native entry refreshes and holds current Account evidence immediately
    // before transfer. Content qualification above can outlive its five-second
    // admission freshness window, so the durable fixture must do the same.
    let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(source.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let held = evidence
        .evidence
        .refresh_account_held(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let entered = owner
        .enter_soulpit(&authority, &h.node, &permit, group.clone(), &entry, native)
        .await;
    held.release().await.map_err(debug)?;
    let receipt =
        match entered.map_err(|error| format!("Soulpit shared successful entry: {error:?}"))? {
            ActivityEnterOutcome::Entered(receipt) => receipt,
            other => return Err(format!("shared Soulpit entry refused: {other:?}").into()),
        };
    assert_eq!(receipt.handoff_id, entry.handoff_id);
    assert_eq!(entry_counts(h).await?, (1, 1, 1, 1));
    assert!(matches!(
        owner
            .enter_soulpit(&authority, &h.node, &permit, group.clone(), &entry, native)
            .await
            .map_err(|error| format!("Soulpit shared exact replay: {error:?}"))?,
        ActivityEnterOutcome::Replayed(_)
    ));
    let mut rebound = entry.clone();
    if let crate::durability::soulpit::SoulpitFunding::Core {source,..}=&mut rebound.funding { *source=id(72); } else {return Err("core fixture".into());}
    assert!(matches!(
        owner
            .enter_soulpit(
                &authority,
                &h.node,
                &permit,
                group.clone(),
                &rebound,
                native
            )
            .await?,
        ActivityEnterOutcome::Refused(
            crate::durability::activity_handoff::ActivityRefusal::ReplayConflict
        )
    ));
    rebound = entry.clone();
    rebound.intent.push(4);
    assert!(matches!(
        owner
            .enter_soulpit(
                &authority,
                &h.node,
                &permit,
                group.clone(),
                &rebound,
                native
            )
            .await?,
        ActivityEnterOutcome::Refused(
            crate::durability::activity_handoff::ActivityRefusal::ReplayConflict
        )
    ));
    rebound = entry.clone();
    if let crate::durability::soulpit::SoulpitFunding::Core { definition, .. } = &mut rebound.funding { definition.revision_ref="definition-r2".into(); } else { return Err("core fixture".into()); }
    assert!(matches!(
        owner
            .enter_soulpit(
                &authority,
                &h.node,
                &permit,
                group.clone(),
                &rebound,
                native
            )
            .await?,
        ActivityEnterOutcome::Refused(
            crate::durability::activity_handoff::ActivityRefusal::ReplayConflict
        )
    ));
    assert_eq!(
        entry_counts(h).await?,
        (1, 1, 1, 1),
        "replay burned a second core or duplicated its audit"
    );
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(group.participants[0].destination_game_session_id)
        .await?
        .0;
    assert_eq!(current.session_state(), GameSessionState::Active);
    assert_eq!(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(source.game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    let destination = item_fence_of(current).ok_or("committed Instance fence")?;
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(destination.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    Ok(AdmittedSoulpit {
        handoff_id: entry.handoff_id,
        destination: CurrentCharacterGameplayFence {
            character_id: destination.character_id,
            game_session_id: destination.game_session_id,
            connection_generation: destination.connection_generation,
            character_lease_generation: destination.character_lease_generation,
            runtime_scope: destination.runtime_scope,
            scope_ownership_generation: destination.scope_ownership_generation,
            expected_character_revision: CharacterRevision::new(revision.parse()?)
                .map_err(debug)?,
        },
    })
}

pub(super) async fn entry_counts(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_soulpit_encounters), (SELECT count(*) FROM game_item_craft_records), (SELECT count(*) FROM game_item_audit_outbox)")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}

#[path = "soulpit_shared_entry/staff.rs"]
pub(super) mod staff;
