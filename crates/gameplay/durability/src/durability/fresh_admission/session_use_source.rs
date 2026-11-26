async fn load_session_use_source(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: GameSessionUseRequestV1,
) -> Result<DurableSessionUseSource> {
    use sqlx::Row;
    let state = session_use_state(tx, request.character_id()).await?;
    if let Some(fence) = request.current_fence() {
        if request.expected_current() != Some(fence.current_session()) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let current: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid AND character_id = encode($2,'hex')::uuid AND current_generation = $3::text::numeric(20,0) AND character_lease_generation = $4::text::numeric(20,0) AND scope_ownership_generation = $5::text::numeric(20,0))")
            .bind(fence.current_session().as_bytes().as_slice()).bind(request.character_id().as_bytes().as_slice()).bind(fence.connection_generation().to_string()).bind(fence.character_lease_generation().to_string()).bind(fence.scope_ownership_generation().to_string()).fetch_one(&mut **tx).await?;
        if !current {
            return Err(DurabilityError::Unavailable);
        }
    } else if request.expected_current().is_some() {
        return Err(DurabilityError::Unavailable);
    }
    let member = sqlx::query("SELECT uuid_send(character_id) AS character_id, operation_binding, membership_revision::text FROM game_durability_session_use_memberships WHERE game_session_id = encode($1,'hex')::uuid")
        .bind(request.candidate().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let (membership, binding, committed_revision) = match member {
        None => (GameSessionCandidateMembershipV1::Unused, None, None),
        Some(row) => {
            let binding = row
                .try_get::<Option<Vec<u8>>, _>("operation_binding")?
                .map(|v| checked(<[u8; 16]>::try_from(v)))
                .transpose()?;
            let owner: Vec<u8> = row.try_get("character_id")?;
            let exact = owner.as_slice() == request.character_id().as_bytes()
                && binding == Some(request.operation_binding());
            let revision = checked(
                row.try_get::<String, _>("membership_revision")?
                    .parse::<u64>(),
            )?;
            (
                if exact {
                    GameSessionCandidateMembershipV1::UsedByExactOperation
                } else {
                    GameSessionCandidateMembershipV1::UsedByDifferentOperation
                },
                binding,
                Some(revision),
            )
        }
    };
    let observation = GameSessionUseObservationV1::from_owner_results(
        &request,
        state.revision,
        if state.complete {
            GameSessionUseCompletenessV1::Complete
        } else {
            GameSessionUseCompletenessV1::Incomplete
        },
        membership,
        binding,
        committed_revision,
        state.revision,
    );
    Ok(DurableSessionUseSource {
        request,
        observation,
    })
}

impl FreshAdmissionStore {
    pub async fn session_use_source(
        &self,
        request: GameSessionUseRequestV1,
    ) -> Result<DurableSessionUseSource> {
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let source = load_session_use_source(&mut tx, request).await?;
                    super::admission_journal::commit_pass_read(tx, deadline).await?;
                    Ok(source)
                })
            })
            .await
    }
}

pub(super) async fn unused_session_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    candidate: GameSessionId,
) -> Result<u64> {
    let state = session_use_state(tx, character).await?;
    if !state.complete {
        return Err(DurabilityError::Unavailable);
    }
    let used: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_session_use_memberships WHERE game_session_id = encode($1,'hex')::uuid)")
        .bind(candidate.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if used {
        return Err(DurabilityError::SessionUseAuthorization(
            GameSessionUseAuthorizationErrorV1::CandidateAlreadyUsed,
        ));
    }
    if state.revision == GAME_SESSION_USE_LEDGER_CAPACITY_V1 {
        return Err(DurabilityError::SessionUseAuthorization(
            GameSessionUseAuthorizationErrorV1::TerminalReplacementGameSessionLedgerExhausted,
        ));
    }
    Ok(state.revision)
}
