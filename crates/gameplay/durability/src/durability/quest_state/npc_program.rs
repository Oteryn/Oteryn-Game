//! Ordered source NPC storage operations on the existing Character revision and quest receipt owners.
use super::*;
use oteryn_gameplay_quests::quest::npc_rules::{
    NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRulePlan, NpcRuleTrigger,
};
mod fathers_burden;
mod marlin_trophy;
mod items;
mod rewards;


/// Advisory preview facts. Locked execution replaces both from Character owners.
#[derive(Clone, Copy, Debug, Default)]
pub struct NpcProgramFacts<'a> {
    pub level: Option<u32>,
    /// The source catalogue's qualified vocation key, e.g. `oteryn:vocation.knight`.
    pub vocation: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcProgramRequest {
    pub command: CommandRef,
    pub npc: String,
    pub trigger: NpcRuleTrigger,
    pub message: String,
    pub topic: i32,
    pub transition_key: String,
}

#[derive(Clone, Debug)]
pub struct NpcProgramCommit {
    pub transition: QuestTransitionOutcome,
    pub reply: Vec<String>,
    pub next_topic: i32,
    /// The locked source branch only replied; it wrote no receipt or value.
    pub read_only: bool,
    pub inventory_changed: bool,
    pub end_conversation: bool,
    pub end_delay_ms: u64,
}

impl NpcProgramRequest {
    fn quest_request(&self) -> QuestTransitionRequest {
        QuestTransitionRequest {
            transition_key: self.transition_key.clone(),
            cause: QuestCause::Command(self.command),
        }
    }

    fn binding(&self, character: CharacterId) -> [u8; 33] {
        let mut digest =
            Sha256::new().chain_update(request_binding(character, &self.quest_request()));
        for value in [&self.npc, &self.message] {
            digest = digest
                .chain_update((value.len() as u64).to_be_bytes())
                .chain_update(value.as_bytes());
        }
        let trigger = match self.trigger {
            NpcRuleTrigger::Say => 0,
            NpcRuleTrigger::Greet => 1,
            NpcRuleTrigger::Farewell => 2,
            NpcRuleTrigger::Walkaway => 3,
        };
        let digest = digest
            .chain_update([trigger])
            .chain_update(self.topic.to_be_bytes())
            .finalize();
        let mut result = [1; 33];
        result[1..].copy_from_slice(&digest);
        result
    }
}

/// Pure preview and locked reevaluation use exactly the same source branch order.
pub fn preview(
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    now: i64,
    facts: NpcProgramFacts<'_>,
) -> std::result::Result<Option<NpcRulePlan>, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError>
{
    let Some(source) = NpcRuleCatalog::runtime()?.admitted_for_npc(&request.npc, catalogue)? else {
        return Ok(None);
    };
    match evaluate_source(&source, request, tracks, catalogue, now, None, facts) {
        Err(_) if request.npc == "oteryn:npc.tereban" => evaluate_source(
            &source,
            request,
            tracks,
            catalogue,
            now,
            Some(&fathers_burden::optimistic_inventory()),
            facts,
        ),
        Err(_) if marlin_trophy::owns(request) => evaluate_source(
            &source,
            request,
            tracks,
            catalogue,
            now,
            Some(&marlin_trophy::optimistic_inventory()),
            facts,
        ),
        Err(error) => match source.optimistic_item_inventory(&request.npc) {
            Some(inventory) => evaluate_source(&source, request, tracks, catalogue, now, Some(&inventory), facts),
            None => Err(error),
        },
        result => result,
    }
}

fn evaluate_source(
    source: &NpcRuleCatalog,
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    now: i64,
    inventory: Option<&NpcInventorySnapshot>,
    facts: NpcProgramFacts<'_>,
) -> std::result::Result<Option<NpcRulePlan>, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError>
{
    let mut complete = tracks.clone();
    for program in source.programs() {
        for rule in program.document.rules.iter().filter(|rule| rule.npc == request.npc) {
            for guard in &rule.guards {
                if let oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Storage {
                    track, ..
                }
                | oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::StorageTime {
                    track,
                    ..
                } = guard
                    // Local absent values belong to the source evaluator. Only
                    // foreign guards need a catalogue-backed snapshot entry.
                    && !program.document.tracks.iter().any(|definition| definition.key == *track)
                    && let Some(definition) = catalogue.track(track)
                {
                    let initial = source
                        .programs()
                        .iter()
                        .flat_map(|program| &program.document.tracks)
                        .find(|source_track| source_track.key == *track)
                        .map_or(definition.initial, |source_track| source_track.initial);
                    complete.entry(track.clone()).or_insert(initial);
                }
            }
        }
    }
    let empty = NpcInventorySnapshot::default();
    let input = NpcRuleInput {
        npc: &request.npc,
        trigger: request.trigger,
        message: &request.message,
        topic: request.topic,
        tracks: &complete,
        inventory: inventory.unwrap_or(&empty),
        now_epoch_seconds: now,
        calendar_month: utc_calendar_month(now)?,
        level: facts.level.unwrap_or(0),
        vocation: facts.vocation.unwrap_or(""),
        sex: 0,
        outfits: &BTreeMap::new(),
    };
    if inventory.is_some() && marlin_trophy::owns(request) {
        source.plan_marlin_items(&input)
    } else if inventory.is_some() && fathers_burden::owns(request) {
        source.plan_father_items(&input)
    } else if inventory.is_some() {
        source.plan_item_exchange(&input)
    } else {
        source.plan_storage(&input)
    }
}

impl DurabilityRoot {
    /// Replay a bound NPC response without granting current gameplay authority.
    pub async fn reconcile_npc_program(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        request: NpcProgramRequest,
    ) -> Result<Option<NpcProgramCommit>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_receipt_key(&mut tx, character, &request.quest_request()).await?;
                    let result = load(&mut tx, character, &request).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await?
    }

    /// One source storage operation, including Father's eleven-track acceptance, is one transaction.
    #[allow(clippy::too_many_arguments, reason = "physical lane, current authority and item content are independent proofs")]
    pub async fn commit_character_npc_program<C: super::super::item_loop_move::ItemLoopContent>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
    ) -> Result<NpcProgramCommit> {
        self.commit_character_npc_program_with_premium(
            authority, node, permit, fence, request, catalogue, items,
            &super::grizzly::NoGrizzlyPremium,
        ).await
    }

    /// Join native Premium authority to the existing fenced NPC transaction.
    #[allow(clippy::too_many_arguments, reason = "the Premium participant joins the existing independent authority, lane and item proofs")]
    pub async fn commit_character_npc_program_with_premium<
        C: super::super::item_loop_move::ItemLoopContent,
        G: super::grizzly::GrizzlyPremiumGate,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        premium: &G,
    ) -> Result<NpcProgramCommit> {
        let source = NpcRuleCatalog::runtime()
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        self.commit_npc_program_from_source(
            authority, node, permit, fence, request, catalogue, items, source, premium,
        ).await
    }

    /// Qualify a minimal bound source fixture through the production transaction.
    #[cfg(feature = "test-support")]
    #[allow(clippy::too_many_arguments, reason = "the fixture supplies one trusted source catalogue alongside the production proofs")]
    pub async fn fixture_commit_npc_program_from_source<
        C: super::super::item_loop_move::ItemLoopContent,
        G: super::grizzly::GrizzlyPremiumGate,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        source: &NpcRuleCatalog,
        premium: &G,
    ) -> Result<NpcProgramCommit> {
        self.commit_npc_program_from_source(
            authority, node, permit, fence, request, catalogue, items, source, premium,
        ).await
    }

    #[allow(clippy::too_many_arguments, reason = "source admission and the Premium participant preserve the existing independent transaction proofs")]
    async fn commit_npc_program_from_source<
        C: super::super::item_loop_move::ItemLoopContent,
        G: super::grizzly::GrizzlyPremiumGate,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        source: &NpcRuleCatalog,
        premium: &G,
    ) -> Result<NpcProgramCommit> {
        if request.command.game_session_id() != fence.game_session_id
            || request.message.len() > 255
            || !request.transition_key.starts_with("oteryn:quest-npc/")
            || !valid_quest_key(&request.transition_key)
            || request.topic < 0
            || fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit.check_scope(fence.runtime_scope)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let source = source.admitted_for_npc(&request.npc, &catalogue)
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let mut context = (items, premium);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder,deadline,context| Box::pin(async move {
            let (content, premium) = *context;
            let mut tx=super::super::db::begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let character=fence.character_id;
            let q=request.quest_request();
            lock_receipt_key(&mut tx,character,&q).await?;
            match load(&mut tx,character,&request).await? {
                Ok(Some(replay)) => {tx.commit(deadline).await?;return Ok(Ok(replay));}
                Err(error) => return Ok(Err(error)),
                Ok(None) => {}
            }
            let root=match assert_gameplay_fence(&mut tx,&fence,&node).await? {
                Ok(root)=>root,Err(e)=>return Ok(Err(e)),
            };
            let id=character.as_bytes().as_slice();
            let state=sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(id).fetch_optional(&mut **tx).await?;
            let Some(state)=state else {return Ok(Err(CharacterProgressionError::MissingProgressionState));};
            if numeric_u64(&state,"character_revision")?!=root.revision {return Err(DurabilityError::InvalidStoredState);}
            if !state_matches_root(&state,&root) || state.try_get::<String,_>("content_revision")?!=catalogue.content_revision() {
                return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
            }
            let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)").bind(id).fetch_one(&mut **tx).await?;
            if pending {return Ok(Err(CharacterProgressionError::RespawnPending));}
            // Root lock serializes every Character writer; sorted bounded tracks include foreign guard reads.
            let rows=sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid ORDER BY track_key LIMIT $2 FOR UPDATE")
                .bind(id).bind((QUESTSTATE0_RL_08_TRACKS+1) as i64).fetch_all(&mut **tx).await?;
            if rows.len()>QUESTSTATE0_RL_08_TRACKS {return Err(DurabilityError::InvalidStoredState);}
            let mut tracks=BTreeMap::new();
            for row in rows {tracks.insert(row.try_get::<String,_>("track_key")?,row.try_get::<i64,_>("value")?);}
            let states=sqlx::query("SELECT quest_key,definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid ORDER BY quest_key LIMIT $2 FOR UPDATE")
                .bind(id).bind((QUESTSTATE0_RL_08_STATES+1) as i64).fetch_all(&mut **tx).await?;
            if states.len()>QUESTSTATE0_RL_08_STATES {return Err(DurabilityError::InvalidStoredState);}
            let mut effective_tracks=tracks.clone();
            for state in states {
                let quest:String=state.try_get("quest_key")?;
                let stored:[u8;32]=state.try_get::<Vec<u8>,_>("definition_hash")?.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
                if let Some(defaults)=oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(&catalogue,&quest,&stored) {
                    for (key,value) in defaults {effective_tracks.entry(key.clone()).or_insert(*value);}
                }
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint").fetch_one(&mut **tx).await?;
            let father_items = fathers_burden::owns(&request);
            let marlin_items = marlin_trophy::owns(&request);
            let plain_keys = source.item_exchange_keys(&request.npc);
            let generic_items = !father_items && !marlin_items && plain_keys.is_some();
            let inventory_owned = father_items || marlin_items || generic_items;
            let inventory = if marlin_items {
                Some(fathers_burden::inventory_for(&mut tx, fence, &[marlin_trophy::FISH.into()]).await?)
            } else if father_items {
                Some(fathers_burden::inventory(&mut tx, fence).await?)
            } else if let Some(keys) = plain_keys.filter(|_| generic_items) {
                Some(fathers_burden::inventory_for(&mut tx, fence, &keys).await?)
            } else { None };
            // Progression is already locked under this Character root; preview facts
            // never authorize a source level guard.
            let level = u32::try_from(state.try_get::<i64,_>("level")?).map_err(|_| DurabilityError::InvalidStoredState)?;
            // Build writes share the locked Character root. Its absent row has the
            // same `none` meaning as read_character_build_state; preview is never authority.
            let vocation = if source.requires_vocation(&request.npc) {
                let stored: Option<String> = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(id).fetch_optional(&mut **tx).await?;
                Some(format!("oteryn:vocation.{}", stored.as_deref().unwrap_or(crate::domain::character_build::NO_VOCATION)))
            } else { None };
            let facts = NpcProgramFacts { level: Some(level), vocation: vocation.as_deref() };
            let evaluated = evaluate_source(&source,&request,&effective_tracks,&catalogue,now,inventory.as_ref(),facts);
            if inventory_owned && let Ok(Some(plan)) = &evaluated && plan.track_changes.is_empty() && !rewards::has_experience(&plan.effects) {
                let result = NpcProgramCommit { transition: QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), reply: plan.reply.clone(), next_topic: plan.next_topic, read_only: true, inventory_changed: false, end_conversation: items::ends(plan), end_delay_ms: items::end_delay(plan) };
                tx.commit(deadline).await?;
                return Ok(Ok(result));
            }
            let plan=match evaluated {
                Ok(Some(plan)) if plan.selected_rule_keys.first()==Some(&request.transition_key) && !plan.track_changes.is_empty()=>plan,
                _=>return Ok(Ok(refused(QuestRefusal::StageMismatch))),
            };
            // Independent programmes can share one aggregate quest. Requirements
            // belong to the programmes whose rules actually contributed to this plan.
            if source.programs().iter().any(|program| program.document.premium_required
                && program.document.rules.iter().any(|rule| plan.selected_rule_keys.contains(&rule.key))) {
                let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id).fetch_one(&mut **tx).await?;
                let account: [u8;16] = account.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                if !premium.task_permitted(&mut tx, account).await? {
                    return Ok(Ok(refused(QuestRefusal::StageMismatch)));
                }
            }
            let experience = rewards::experience(&plan.effects)?;
            if father_items && fathers_burden::experience(&plan)? != experience { return Err(DurabilityError::InvalidStoredState); }
            if experience > 0 {
                let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id).fetch_one(&mut **tx).await?;
                if pending >= QUESTGATE0_RL_10 as i64 { return Ok(Ok(refused(QuestRefusal::CapacityExceeded))); }
            }
            let item_action = father_items || marlin_items || (generic_items && items::changes_items(&plan));
            let quest=&plan.quest;
            let hash=catalogue.definition_hash(quest).ok_or(DurabilityError::InvalidStoredState)?;
            let stored_state=sqlx::query("SELECT pinned_content_revision,definition_hash,completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE").bind(id).bind(quest).fetch_optional(&mut **tx).await?;
            let (pin,hash)=match &stored_state {
                Some(row)=>{
                    let stored:[u8;32]=row.try_get::<Vec<u8>,_>("definition_hash")?.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
                    if stored!=hash && !row.try_get::<bool,_>("completed")?
                        && oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(&catalogue,quest,&stored).is_none()
                    {return Ok(Ok(refused(QuestRefusal::RevisionMismatch)));}
                    (row.try_get::<String,_>("pinned_content_revision")?,stored)
                }
                None=>{
                    if !capacity::new_state_fits(&mut tx,character).await? {return Ok(Ok(refused(QuestRefusal::CapacityExceeded)));}
                    (catalogue.content_revision().to_owned(),hash)
                }
            };
            let keys=plan.track_changes.iter().map(|c|c.key.as_str()).collect::<Vec<_>>();
            let existing=keys.iter().filter(|k|tracks.contains_key(**k)).count();
            if !capacity::tracks_fit(&mut tx,character,&keys,existing).await? {return Ok(Ok(refused(QuestRefusal::CapacityExceeded)));}
            if item_action {
                match super::super::character_equipment::assert_current_equipment_in_transaction(
                    &mut tx, &recovery, &node, &fence.item_fence(), Some(request.command), content.loop_content_digest(),
                ).await {
                    Ok(_) => {},
                    Err(super::super::character_equipment::EquipmentError::Rejected(_)) => return Ok(Err(CharacterProgressionError::AuthorityRejected)),
                    Err(super::super::character_equipment::EquipmentError::Durability(error)) => return Err(error),
                    Err(super::super::character_equipment::EquipmentError::Database(error)) => return Err(error.into()),
                }
                let applied = if marlin_items {
                    marlin_trophy::apply(&mut tx, fence, &request, &plan, content, &node).await?
                } else if generic_items {
                    items::apply(&mut tx, fence, &request, &plan, content, &node, permit).await?
                } else {
                    fathers_burden::apply(&mut tx, fence, &request, &plan, content, &node).await?
                };
                match applied {
                    true => {},
                    false => return Ok(Ok(refused(QuestRefusal::CapacityExceeded))),
                }
            }
            let revision=root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            let original=root.revision.to_string();let committed=revision.to_string();
            for query in ["UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric", "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric"] {
                if sqlx::query(query).bind(id).bind(&committed).bind(&original).execute(&mut **tx).await?.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
            }
            let before=plan.track_changes.iter().map(|c|c.before).collect::<Vec<_>>();
            let after=plan.track_changes.iter().map(|c|c.after).collect::<Vec<_>>();
            sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT s.character_id,'command',encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8,false,$9::text[],$10::bigint[],$11::bigint[],$12::text::numeric,$13::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE character_id=encode($1,'hex')::uuid")
                .bind(id).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.transition_key).bind(quest).bind(request.binding(character).as_slice()).bind(&pin).bind(hash.as_slice()).bind(&keys).bind(&before).bind(&after).bind(&original).bind(&committed).execute(&mut **tx).await?;
            for change in &plan.track_changes {
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
                    .bind(id).bind(&change.key).bind(quest).bind(change.after).bind(&committed).execute(&mut **tx).await?;
            }
            if stored_state.is_some() {
                sqlx::query("UPDATE game_character_quest_states SET committed_character_revision=$3::text::numeric WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id).bind(quest).bind(&committed).execute(&mut **tx).await?;
            } else {
                sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric)").bind(id).bind(quest).bind(&pin).bind(hash.as_slice()).bind(&committed).execute(&mut **tx).await?;
            }
            sqlx::query("INSERT INTO game_quest_npc_program_receipts(character_id,game_session_id,command_id,transition_key,reply,next_topic,inventory_changed,end_conversation,end_delay_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8,$9::text::integer)")
                .bind(id).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.transition_key).bind(&plan.reply).bind(plan.next_topic).bind(item_action).bind(items::ends(&plan)).bind(items::end_delay(&plan).to_string()).execute(&mut **tx).await?;
            if experience > 0 {
                insert_xp_obligation(&mut tx, character, &q, experience, &pin).await?;
            }
            let receipt=load_receipt(&mut tx,character,&q).await?.ok_or(DurabilityError::InvalidStoredState)?;
            let receipt=decode_receipt(&receipt)?;
            tx.commit(deadline).await?;
            Ok(Ok(NpcProgramCommit{transition:QuestTransitionOutcome::Committed(receipt),end_conversation:items::ends(&plan),end_delay_ms:items::end_delay(&plan),reply:plan.reply,next_topic:plan.next_topic,read_only:false,inventory_changed:item_action}))
        })).await?
    }
}

fn refused(refusal: QuestRefusal) -> NpcProgramCommit {
    NpcProgramCommit {
        transition: QuestTransitionOutcome::Refused(refusal),
        reply: Vec::new(),
        next_topic: 0,
        read_only: false,
        inventory_changed: false,
        end_conversation: false,
        end_delay_ms: 0,
    }
}

async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &NpcProgramRequest,
) -> StoredResult<Result<Option<NpcProgramCommit>>> {
    let Some(row) = load_receipt(tx, character, &request.quest_request()).await? else {
        return Ok(Ok(None));
    };
    if row.try_get::<Vec<u8>, _>("request_binding")? != request.binding(character) {
        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
    }
    let response=sqlx::query("SELECT reply,next_topic,inventory_changed,end_conversation,end_delay_ms FROM game_quest_npc_program_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric AND transition_key=$4")
        .bind(character.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.transition_key).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Ok(Some(NpcProgramCommit {
        transition: QuestTransitionOutcome::AlreadyCommitted(decode_receipt(&row)?),
        reply: response.try_get("reply")?,
        next_topic: response.try_get("next_topic")?,
        read_only: false,
        inventory_changed: response.try_get("inventory_changed")?,
        end_conversation: response.try_get("end_conversation")?,
        end_delay_ms: u64::try_from(response.try_get::<i32, _>("end_delay_ms")?).map_err(|_| DurabilityError::InvalidStoredState)?,
    })))
}

// Civil-from-days, inverse of the UTC scheduler conversion in gameplay-foundation.
// Locked evaluation supplies PostgreSQL transaction time; previews supply server time.
fn utc_calendar_month(seconds: i64) -> std::result::Result<u8, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError> {
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    u8::try_from(month).map_err(|_| oteryn_gameplay_quests::quest::npc_rules::NpcRuleError::Arithmetic("calendar month".into()))
}

#[cfg(test)]
mod calendar_tests {
    use super::utc_calendar_month;

    #[test]
    fn authoritative_epoch_month_boundaries() -> Result<(), String> {
        for (second, before, after) in [(0, 12, 1), (1_706_745_600, 1, 2), (1_709_251_200, 2, 3), (1_735_689_600, 12, 1)] {
            assert_eq!(utc_calendar_month(second - 1).map_err(|e| e.to_string())?, before);
            assert_eq!(utc_calendar_month(second).map_err(|e| e.to_string())?, after);
        }
        Ok(())
    }
}
