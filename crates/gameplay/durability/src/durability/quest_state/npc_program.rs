//! Ordered source NPC storage operations on the existing Character revision and quest receipt owners.
use super::*;
use oteryn_gameplay_quests::quest::npc_rules::{
    NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRulePlan, NpcRuleTrigger,
};
mod achievements;
mod equipped;
mod fathers_burden;
pub(in crate::durability) mod hot_cuisine_m42;
mod items;
mod marlin_trophy;
mod rewards;
pub mod teleport;
#[cfg(test)]
mod hot_cuisine_m42_tests;

/// Advisory for preview; the receipt owner reloads every fact under the Character fence.
#[derive(Clone, Debug, Default)]
pub struct NpcEligibility {
    pub level: u32,
    pub vocation: String,
    pub premium_current: bool,
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
    /// The source branch wrote no quest revision or inventory value. A free teleport
    /// still owns a separate response receipt and pending arrival.
    pub read_only: bool,
    pub inventory_changed: bool,
    pub end_conversation: bool,
    pub end_delay_ms: u64,
    /// Fresh-commit presentation only; receipt replays never repeat effects.
    pub magic_effects: Vec<u16>,
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
    eligibility: &NpcEligibility,
) -> std::result::Result<Option<NpcRulePlan>, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError>
{
    preview_with_player_name(request, tracks, catalogue, now, eligibility, None)
}

/// The name is advisory in preview; the writer rereads the locked Character name.
pub fn preview_with_player_name(
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    now: i64,
    eligibility: &NpcEligibility,
    player_name: Option<&str>,
) -> std::result::Result<Option<NpcRulePlan>, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError> {
    preview_with_appearance(request, tracks, catalogue, now, eligibility, player_name, (&BTreeMap::new(), 0))
}

/// Advisory appearance preview; the writer reloads the locked account snapshot.
pub fn preview_with_appearance(
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    now: i64,
    eligibility: &NpcEligibility,
    player_name: Option<&str>,
    appearance: (&BTreeMap<u16, u8>, u8),
) -> std::result::Result<Option<NpcRulePlan>, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError>
{
    let facts = NpcEvaluationFacts {
        eligibility,
        player_name,
        appearance,
    };
    let Some(source) = NpcRuleCatalog::runtime()?.admitted_for_npc(&request.npc, catalogue)? else {
        return Ok(None);
    };
    let cooking_inventory = if hot_cuisine_m42::owns(request) { Some(hot_cuisine_m42::optimistic()?) } else { None };
    match evaluate_source(&source, request, tracks, catalogue, now, facts, cooking_inventory.as_ref()) {
        Err(_) if request.npc == "oteryn:npc.tereban" => evaluate_source(
            &source,
            request,
            tracks,
            catalogue,
            now,
            facts,
            Some(&fathers_burden::optimistic_inventory()),
        ),
        Err(_) if marlin_trophy::owns(request) => evaluate_source(
            &source,
            request,
            tracks,
            catalogue,
            now,
            facts,
            Some(&marlin_trophy::optimistic_inventory()),
        ),
        Err(error) => match source
            .outfit_exchange_keys(&request.npc)
            .map(|keys| NpcInventorySnapshot {
                items: keys
                    .into_iter()
                    .map(|item| {
                        (
                            oteryn_gameplay_quests::quest::npc_rules::NpcItemKey {
                                item,
                                subtype: None,
                            },
                            100,
                        )
                    })
                    .collect(),
                ..NpcInventorySnapshot::default()
            })
            .or_else(|| source.optimistic_item_inventory(&request.npc))
            .or_else(|| source.optimistic_equipped_inventory(&request.npc))
        {
            Some(inventory) => evaluate_source(
                &source,
                request,
                tracks,
                catalogue,
                now,
                facts,
                Some(&inventory),
            ),
            None => Err(error),
        },
        result => result,
    }
}

#[derive(Clone, Copy)]
struct NpcEvaluationFacts<'a> {
    eligibility: &'a NpcEligibility,
    player_name: Option<&'a str>,
    appearance: (&'a BTreeMap<u16, u8>, u8),
}

fn evaluate_source(
    source: &NpcRuleCatalog,
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    now: i64,
    facts: NpcEvaluationFacts<'_>,
    inventory: Option<&NpcInventorySnapshot>,
) -> std::result::Result<Option<NpcRulePlan>, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError>
{
    let mut complete = tracks.clone();
    for program in source.programs() {
        for rule in program
            .document
            .rules
            .iter()
            .filter(|rule| rule.npc == request.npc)
        {
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
        player_name: facts.player_name,
        topic: request.topic,
        tracks: &complete,
        inventory: inventory.unwrap_or(&empty),
        now_epoch_seconds: now,
        calendar_month: utc_calendar_month(now)?,
        premium_current: facts.eligibility.premium_current,
        level: facts.eligibility.level,
        vocation: &facts.eligibility.vocation,
        sex: facts.appearance.1,
        outfits: facts.appearance.0,
    };
    if hot_cuisine_m42::owns(request) {
        hot_cuisine_m42::plan(source, &input)
    } else if inventory.is_some() && marlin_trophy::owns(request) {
        source.plan_marlin_items(&input)
    } else if inventory.is_some() && fathers_burden::owns(request) {
        source.plan_father_items(&input)
    } else if inventory.is_some() && source.outfit_exchange_keys(&request.npc).is_some() {
        source.plan_outfit_exchange(&input)
    } else if inventory.is_some() && source.equipped_storage_requirements(&request.npc).is_some() {
        source.plan_equipped_storage(&input)
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
    #[allow(
        clippy::too_many_arguments,
        reason = "physical lane, current authority and item content are independent proofs"
    )]
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
        self.commit_npc_program_source(
            authority,
            node,
            permit,
            fence,
            request,
            catalogue,
            items,
            NpcRuleCatalog::runtime().map_err(|_| CharacterProgressionError::InvalidInput)?,
            None,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "immutable source shares all production authority inputs"
    )]
    async fn commit_npc_program_source<C: super::super::item_loop_move::ItemLoopContent>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        source: &NpcRuleCatalog,
        teleport: Option<teleport::NpcTeleportAdmission>,
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
        permit
            .check_scope(fence.runtime_scope)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let source = source
            .admitted_for_npc(&request.npc, &catalogue)
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let mut content = items;
        self.try_issue_semantic_pass()?.run_with_context(&mut content, move |holder,deadline,content| Box::pin(async move {
            let mut tx=super::super::db::begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let character=fence.character_id;
            let q=request.quest_request();
            lock_receipt_key(&mut tx,character,&q).await?;
            if let Some(admission) = teleport
                && !teleport::replay_matches(&mut tx, character, &request, admission).await? { return Ok(Err(CharacterProgressionError::ConflictingOccurrence)); }

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
            if request.npc == super::super::stag_beaks::HARTFORD {
                super::super::stag_beaks::overlay_locked(&mut tx, character, &mut effective_tracks).await?;
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint").fetch_one(&mut **tx).await?;
            let cooking_items = hot_cuisine_m42::owns(&request);
            let father_items = fathers_burden::owns(&request);
            let marlin_items = marlin_trophy::owns(&request);
            let plain_keys = source.item_exchange_keys(&request.npc);
            let generic_items = !cooking_items && !father_items && !marlin_items && plain_keys.is_some();
            let plain_keys = source.outfit_exchange_keys(&request.npc).or_else(|| source.item_exchange_keys(&request.npc));
            let generic_items = !father_items && !marlin_items && plain_keys.is_some();
            let equipped_storage = source.equipped_storage_requirements(&request.npc).is_some();
            let inventory_owned = cooking_items || father_items || marlin_items || generic_items || equipped_storage;
            let inventory = if cooking_items {
                let keys = hot_cuisine_m42::keys().map_err(|_| DurabilityError::InvalidStoredState)?;
                Some(fathers_burden::inventory_for(&mut tx, fence, &keys).await?)
            } else if equipped_storage {
                Some(equipped::inventory(&mut tx, &recovery, &node, fence, request.command, content.loop_content_digest()).await?)
            } else if marlin_items {
                Some(fathers_burden::inventory_for(&mut tx, fence, &[marlin_trophy::FISH.into()]).await?)
            } else if father_items {
                Some(fathers_burden::inventory(&mut tx, fence).await?)
            } else if let Some(keys) = plain_keys.filter(|_| generic_items) {
                Some(fathers_burden::inventory_for(&mut tx, fence, &keys).await?)
            } else { None };
            let vocation: Option<String> = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(id).fetch_optional(&mut **tx).await?;
            // 0030: no build row is the canonical no-vocation chain seed.
            let vocation = match vocation.as_deref().unwrap_or("none") {
                "elite_knight" => "knight", "royal_paladin" => "paladin",
                "master_sorcerer" => "sorcerer", "elder_druid" => "druid",
                "exalted_monk" => "monk", value => value,
            };
            let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
                .bind(id).fetch_one(&mut **tx).await?;
            let account: [u8;16] = account.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
            let eligibility = NpcEligibility {
                level: u32::try_from(state.try_get::<i64,_>("level")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                vocation: format!("oteryn:vocation.{vocation}"),
                premium_current: content.wheel_premium_current(account),
            };
            let native = content.npc_outfit_content();
            if native.is_some_and(|native| native.source_digest() != content.loop_content_digest()) {
                return Err(DurabilityError::InvalidStoredState);
            }
            let needs_appearance = source.programs().iter().flat_map(|p| &p.document.rules)
                .filter(|rule| rule.npc == request.npc).any(|rule|
                    rule.guards.iter().any(|guard| matches!(guard,
                        oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Outfit { .. } |
                        oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Sex { .. })) ||
                    rule.effects.iter().any(|effect| matches!(effect,
                        oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::Outfit { .. } |
                        oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::OutfitAddon { .. })));
            if needs_appearance && native.is_none() { return Ok(Ok(refused(QuestRefusal::StageMismatch))); }
            let outfits = if let Some(native) = native.filter(|_| needs_appearance) {
                super::super::character_appearance::npc_outfits::locked_snapshot(&mut tx, character, native).await?
            } else { BTreeMap::new() };
            let sex = if needs_appearance {
                match super::super::character_appearance::item_action_sex(&mut tx, character).await? {
                    crate::domain::appearance::Sex::Female => 0,
                    crate::domain::appearance::Sex::Male => 1,
                }
            } else { 0 };
            let player_name = if source.programs().iter().flat_map(|p| &p.document.rules)
                .any(|rule| rule.npc == request.npc && rule.match_player_name) {
                Some(super::super::character_chat_name::read_character_name_for_share_in_transaction(
                    &mut tx, character.as_bytes()).await?)
            } else { None };
            let facts = NpcEvaluationFacts { eligibility: &eligibility, player_name: player_name.as_deref(), appearance: (&outfits,sex) };
            let teleport_plan = if teleport.is_some() {
                match teleport::evaluate(&source, &request, &effective_tracks, now, facts) {
                    Ok(Some(plan)) => Some(plan),
                    _ => return Ok(Ok(refused(QuestRefusal::StageMismatch))),
                }
            } else { None };
            if let (Some(admission), Some(plan)) = (teleport, &teleport_plan) {
                if !admission.matches(fence, plan) { return Ok(Err(CharacterProgressionError::InvalidInput)); }
                let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)").bind(id).fetch_one(&mut **tx).await?;
                if pending { return Ok(Ok(refused(QuestRefusal::StageMismatch))); }
                if plan.source.selected_rule_keys.first() != Some(&request.transition_key) { return Ok(Ok(refused(QuestRefusal::StageMismatch))); }
                if let Some(row) = sqlx::query("SELECT definition_hash,completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id).bind(&plan.source.quest).fetch_optional(&mut **tx).await? {
                    let stored: [u8;32] = row.try_get::<Vec<u8>,_>("definition_hash")?.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
                    if Some(stored) != catalogue.definition_hash(&plan.source.quest) && !row.try_get::<bool,_>("completed")?
                        && oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(&catalogue,&plan.source.quest,&stored).is_none() {
                        return Ok(Ok(refused(QuestRefusal::RevisionMismatch)));
                    }
                }
                if plan.source.track_changes.is_empty() && !plan.source.completes {
                    teleport::insert(&mut tx, character, &request, &plan.source, admission).await?;
                    let result = teleport::response(&plan.source);
                    tx.commit(deadline).await?;
                    return Ok(Ok(result));
                }
            }
            let evaluated = match teleport_plan { Some(plan) => Ok(Some(plan.source)), None => evaluate_source(&source,&request,&effective_tracks,&catalogue,now,facts,inventory.as_ref()) };
            if inventory_owned && let Ok(Some(plan)) = &evaluated && plan.track_changes.is_empty() && !plan.completes && !rewards::has_experience(&plan.effects) && !plan.effects.iter().any(|effect| matches!(effect, oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::Outfit { .. } | oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::OutfitAddon { .. })) {
                let result = NpcProgramCommit { transition: QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), reply: plan.reply.clone(), next_topic: plan.next_topic, read_only: true, inventory_changed: false, end_conversation: items::ends(plan), end_delay_ms: items::end_delay(plan), magic_effects: Vec::new() };
                tx.commit(deadline).await?;
                return Ok(Ok(result));
            }
            let plan=match evaluated {
                Ok(Some(plan)) if plan.selected_rule_keys.first()==Some(&request.transition_key) && (!plan.track_changes.is_empty() || plan.completes)=>plan,
                _=>return Ok(Ok(refused(QuestRefusal::StageMismatch))),
            };
            let outfit_rewards = if plan.effects.iter().any(|effect| matches!(effect,
                oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::Outfit { .. } |
                oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::OutfitAddon { .. })) {
                let Some(native) = native else { return Ok(Ok(refused(QuestRefusal::StageMismatch))); };
                super::super::character_appearance::npc_outfits::rewards(native, &plan.effects)?
            } else { Vec::new() };
            let experience = rewards::experience(&plan.effects)?;
            let achievement_keys = achievements::keys(&plan.effects)?;
            if father_items && fathers_burden::experience(&plan)? != experience { return Err(DurabilityError::InvalidStoredState); }
            if experience > 0 {
                let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id).fetch_one(&mut **tx).await?;
                if pending >= QUESTGATE0_RL_10 as i64 { return Ok(Ok(refused(QuestRefusal::CapacityExceeded))); }
            }
            let item_action = father_items || marlin_items || ((cooking_items || generic_items) && items::changes_items(&plan));
            let quest=&plan.quest;
            let hash=catalogue.definition_hash(quest).ok_or(DurabilityError::InvalidStoredState)?;
            let stored_state=sqlx::query("SELECT pinned_content_revision,definition_hash,completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE").bind(id).bind(quest).fetch_optional(&mut **tx).await?;
            if plan.completes && stored_state.as_ref().map(|row| row.try_get::<bool,_>("completed")).transpose()?.unwrap_or(false) {
                return Ok(Ok(refused(QuestRefusal::StageMismatch)));
            }
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
                    marlin_trophy::apply(&mut tx, fence, &request, &plan, *content, &node).await?
                } else if cooking_items || generic_items {
                    items::apply(&mut tx, fence, &request, &plan, *content, &node, permit).await?
                } else {
                    fathers_burden::apply(&mut tx, fence, &request, &plan, *content, &node).await?
                };
                match applied {
                    true => {},
                    false => return Ok(Ok(refused(QuestRefusal::CapacityExceeded))),
                }
            }
            for reward in &outfit_rewards {
                super::super::character_appearance::grant_content_outfit(
                    &mut tx, character, &request.binding(character), reward, native.ok_or(DurabilityError::InvalidStoredState)?,
                ).await?;
            }
            let revision=root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            let original=root.revision.to_string();let committed=revision.to_string();
            for query in ["UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric", "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric"] {
                if sqlx::query(query).bind(id).bind(&committed).bind(&original).execute(&mut **tx).await?.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
            }
            let before=plan.track_changes.iter().map(|c|c.before).collect::<Vec<_>>();
            let after=plan.track_changes.iter().map(|c|c.after).collect::<Vec<_>>();
            sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT s.character_id,'command',encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8,$14,$9::text[],$10::bigint[],$11::bigint[],$12::text::numeric,$13::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE character_id=encode($1,'hex')::uuid")
                .bind(id).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.transition_key).bind(quest).bind(request.binding(character).as_slice()).bind(&pin).bind(hash.as_slice()).bind(&keys).bind(&before).bind(&after).bind(&original).bind(&committed).bind(plan.completes).execute(&mut **tx).await?;
            for change in &plan.track_changes {
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
                    .bind(id).bind(&change.key).bind(quest).bind(change.after).bind(&committed).execute(&mut **tx).await?;
            }
            let completion = plan.completes.then_some(committed.as_str());
            if stored_state.is_some() {
                sqlx::query("UPDATE game_character_quest_states SET committed_character_revision=$3::text::numeric,completed_character_revision=coalesce(completed_character_revision,$4::text::numeric) WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id).bind(quest).bind(&committed).bind(completion).execute(&mut **tx).await?;
            } else {
                sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision,completed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric,$6::text::numeric)").bind(id).bind(quest).bind(&pin).bind(hash.as_slice()).bind(&committed).bind(completion).execute(&mut **tx).await?;
            }
            sqlx::query("INSERT INTO game_quest_npc_program_receipts(character_id,game_session_id,command_id,transition_key,reply,next_topic,inventory_changed,end_conversation,end_delay_ms,completes) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8,$9::text::integer,$10)")
                .bind(id).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.transition_key).bind(&plan.reply).bind(plan.next_topic).bind(item_action).bind(items::ends(&plan)).bind(items::end_delay(&plan).to_string()).bind(plan.completes).execute(&mut **tx).await?;
            achievements::grant(&mut tx, character, &request, &achievement_keys).await?;
            if let Some(admission) = teleport { teleport::insert(&mut tx, character, &request, &plan, admission).await?; }
            if experience > 0 {
                insert_xp_obligation(&mut tx, character, &q, experience, &pin).await?;
            }
            let receipt=load_receipt(&mut tx,character,&q).await?.ok_or(DurabilityError::InvalidStoredState)?;
            let receipt=decode_receipt(&receipt)?;
            tx.commit(deadline).await?;
            Ok(Ok(NpcProgramCommit{transition:QuestTransitionOutcome::Committed(receipt),end_conversation:items::ends(&plan),end_delay_ms:items::end_delay(&plan),magic_effects:plan.effects.iter().filter_map(|effect| match effect { oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::MagicEffect { effect } if effect == "CONST_ME_MAGIC_BLUE" => Some(13), _ => None }).collect(),reply:plan.reply,next_topic:plan.next_topic,read_only:false,inventory_changed:item_action}))
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
        magic_effects: Vec::new(),
    }
}

async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &NpcProgramRequest,
) -> StoredResult<Result<Option<NpcProgramCommit>>> {
    let Some(row) = load_receipt(tx, character, &request.quest_request()).await? else {
        return teleport::load_read_only(tx, character, request).await;
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
        end_delay_ms: u64::try_from(response.try_get::<i32, _>("end_delay_ms")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        magic_effects: Vec::new(),
    })))
}

// Civil-from-days, inverse of the UTC scheduler conversion in gameplay-foundation.
// Locked evaluation supplies PostgreSQL transaction time; previews supply server time.
fn utc_calendar_month(
    seconds: i64,
) -> std::result::Result<u8, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError> {
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    u8::try_from(month).map_err(|_| {
        oteryn_gameplay_quests::quest::npc_rules::NpcRuleError::Arithmetic("calendar month".into())
    })
}

#[cfg(test)]
mod calendar_tests {
    use super::utc_calendar_month;

    #[test]
    fn authoritative_epoch_month_boundaries() -> Result<(), String> {
        for (second, before, after) in [
            (0, 12, 1),
            (1_706_745_600, 1, 2),
            (1_709_251_200, 2, 3),
            (1_735_689_600, 12, 1),
        ] {
            assert_eq!(
                utc_calendar_month(second - 1).map_err(|e| e.to_string())?,
                before
            );
            assert_eq!(
                utc_calendar_month(second).map_err(|e| e.to_string())?,
                after
            );
        }
        Ok(())
    }
}
