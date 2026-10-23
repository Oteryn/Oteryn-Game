use super::*;
impl DurabilityRoot {
    /// Found resolves the Account under the native session fence and replays a terminal receipt
    /// before asking Premium. NotActivated writes inside the gate's physical transaction.
    pub async fn commit_guild_found(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildFoundRequest,
        premium: &impl PremiumGateOwner,
    ) -> Result<GuildOutcome> {
        let common = request.operation();
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-found-proposal:v1");
        digest.update(binding(&fence, &common)?);
        digest.update(request.name.as_str().as_bytes());
        let binding = digest.finalize().to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let pre_recovery = recovery.clone();
        let pre_node = node.clone();
        let pre_request = common.clone();
        let pre_binding = binding.clone();
        let preflight = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &pre_recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result = async {
                        lock_operation(&mut tx, &pre_request).await?;
                        if let Some(outcome) = replay(&mut tx, &pre_request, &pre_binding).await? {
                            return Ok(FoundPreflight::Replayed(outcome));
                        }
                        let account = assert_session_fence(&mut tx, &fence, &pre_node)
                            .await?
                            .map_err(|_| GuildError::NotCurrent)?;
                        Ok(FoundPreflight::Fresh(uuid_text(&account)?))
                    }
                    .await;
                    match result {
                        Ok(result) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(result))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await??;
        let account = match preflight {
            FoundPreflight::Replayed(outcome) => return Ok(outcome),
            FoundPreflight::Fresh(account) => account,
        };
        let root = self.clone();
        let write = FoundWrite {
            node: node.clone(),
            fence,
            common,
            name: request.name,
            binding,
            account,
        };
        premium
            .with_premium_gate(account, move |status, tx| {
                Box::pin(async move {
                    match tx {
                        Some(tx) => {
                            if !matches!(&status, PremiumStatus::NotActivated(_)) {
                                return Err(invalid());
                            }
                            // The gate commits whatever the callback returns. The savepoint contains
                            // every Guild write, so a late rejection cannot leak partial creation.
                            let mut savepoint = tx.begin().await?;
                            let result = async {
                                assert_recovery_fence(&mut savepoint, &recovery).await?;
                                lock_admission_relations(&mut savepoint).await?;
                                write.execute(&mut savepoint, &status).await
                            }
                            .await;
                            match result {
                                Ok(outcome) => {
                                    savepoint.commit().await?;
                                    Ok(outcome)
                                }
                                Err(error) => {
                                    savepoint.rollback().await?;
                                    Err(error)
                                }
                            }
                        }
                        None => {
                            // Only the real gate chooses these delivered statuses. Never let a
                            // borrowed pre-delivery witness escape into a second transaction.
                            let status: PremiumStatus<'static> = match status {
                                PremiumStatus::Current => PremiumStatus::Current,
                                PremiumStatus::NotCurrent => PremiumStatus::NotCurrent,
                                PremiumStatus::NotActivated(_) => return Err(invalid()),
                            };
                            root.try_issue_semantic_pass()?
                                .run(move |holder, deadline| {
                                    Box::pin(async move {
                                        let mut tx =
                                            begin_semantic_transaction(holder, deadline).await?;
                                        assert_recovery_fence(&mut tx, &recovery).await?;
                                        lock_admission_relations(&mut tx).await?;
                                        let result = write.execute(&mut tx, &status).await;
                                        match result {
                                            Ok(outcome) => {
                                                commit_semantic_transaction(tx, deadline).await?;
                                                Ok(Ok(outcome))
                                            }
                                            Err(GuildError::Unavailable(error)) => Err(error),
                                            Err(error) => Ok(Err(error)),
                                        }
                                    })
                                })
                                .await?
                        }
                    }
                })
            })
            .await?
    }
}
pub(super) enum FoundPreflight {
    Replayed(GuildOutcome),
    Fresh([u8; 16]),
}

pub(super) struct FoundWrite {
    pub(super) node: NodeIncarnationProof,
    pub(super) fence: CurrentCharacterItemFence,
    pub(super) common: GuildRequest,
    pub(super) name: GuildName,
    pub(super) binding: Vec<u8>,
    pub(super) account: [u8; 16],
}

impl FoundWrite {
    pub(super) async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        premium: &PremiumStatus<'_>,
    ) -> Result<GuildOutcome> {
        lock_operation(tx, &self.common).await?;
        if let Some(outcome) = replay(tx, &self.common, &self.binding).await? {
            return Ok(outcome);
        }
        let account = assert_session_fence(tx, &self.fence, &self.node)
            .await?
            .map_err(|_| GuildError::NotCurrent)?;
        if uuid_text(&account)? != self.account {
            return Err(GuildError::NotCurrent);
        }
        let actor = self.fence.character_id;
        let root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(actor.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some(root) = root else {
            return Err(GuildError::NotCurrent);
        };
        if root.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(&root.try_get::<String, _>("account_id")?)? != self.account
            || uuid_text(&root.try_get::<String, _>("world_id")?)?
                != *self.fence.runtime_scope.world_id().as_bytes()
        {
            return Err(GuildError::NotCurrent);
        }
        // Guild namespace reservation is at the Guild class, after the actor root. The native
        // unique index is still the final invariant; no Character-name reservation is taken.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-name:' || encode($1,'hex') || ':' || $2,0))")
            .bind(self.fence.runtime_scope.world_id().as_bytes().as_slice()).bind(self.name.comparison_key()).execute(&mut **tx).await?;
        let existing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid)",
        )
        .bind(self.common.guild_id.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        if existing {
            return Err(GuildError::ConflictingOccurrence);
        }
        let name_taken:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guilds WHERE world_id=encode($1,'hex')::uuid AND name_key=$2 AND state<>4)")
            .bind(self.fence.runtime_scope.world_id().as_bytes().as_slice()).bind(self.name.comparison_key()).fetch_one(&mut **tx).await?;
        let member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid)")
            .bind(actor.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        // Lock an existing native Account guard to serialize the absent global leadership row.
        // This is not a new Account authority or a Guild-local substitute for that source.
        let guard:Option<i32>=sqlx::query_scalar("SELECT 1 FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(self.account.as_slice()).fetch_optional(&mut **tx).await?;
        if guard.is_none() {
            return Err(GuildError::NotCurrent);
        }
        let position:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid)")
            .bind(self.account.as_slice()).fetch_one(&mut **tx).await?;
        let result = if member {
            GuildResult::AlreadyInGuild
        } else if crate::durability::bank::fee_payer_is_junior(actor) {
            GuildResult::Junior
        } else if position {
            GuildResult::AccountHasPosition
        } else if name_taken {
            GuildResult::NameTaken
        } else if matches!(premium, PremiumStatus::NotCurrent) {
            GuildResult::NotPremium
        } else {
            GuildResult::Ok
        };
        if result != GuildResult::Ok {
            return store_operation(
                tx,
                &self.fence,
                &self.common,
                &self.binding,
                result,
                OperationPayload {
                    revisions: (None, None),
                    audit: None,
                    found_name: Some(self.name.as_str()),
                    text: None,
                    rank: None,
                    resign: None,
                    edit_ranks: None,
                    disband: None,
                },
            )
            .await;
        }
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
        )
        .fetch_one(&mut **tx)
        .await?;
        let audit = GuildAuditProposal {
            version: 1,
            occurrence: self.common.identity.occurrence,
            transaction_id: self.common.identity.transaction_id,
            event_id: self.common.identity.event_id,
            guild_id: *self.common.guild_id.as_bytes(),
            world_id: *self.fence.runtime_scope.world_id().as_bytes(),
            actor: *actor.as_bytes(),
            target: *actor.as_bytes(),
            action: 6,
            previous_revision: 0,
            revision: 1,
            rank_before: None,
            rank_after: Some(1),
            removed_invitations: 0,
            occurred_at_unix_ms: self.common.identity.occurred_at_unix_ms,
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
        let outcome = store_operation(
            tx,
            &self.fence,
            &self.common,
            &self.binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(0), Some(1)),
                audit: Some(&audit),
                found_name: Some(self.name.as_str()),
                text: None,
                rank: None,
                resign: None,
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,$4,1,$5)")
            .bind(self.common.guild_id.as_bytes().as_slice()).bind(self.fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(self.name.as_str()).bind(now.checked_add(FORMATION_LIFETIME_MS).ok_or_else(invalid)?).bind(now).execute(&mut **tx).await?;
        for (level, name) in DEFAULT_RANKS {
            sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
                .bind(self.common.guild_id.as_bytes().as_slice()).bind(i16::from(level)).bind(name).execute(&mut **tx).await?;
        }
        sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',$3)")
            .bind(actor.as_bytes().as_slice()).bind(self.common.guild_id.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(self.account.as_slice()).bind(self.common.guild_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).execute(&mut **tx).await?;
        Ok(outcome)
    }
}
