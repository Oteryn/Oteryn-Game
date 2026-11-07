use super::*;
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Snapshot {
    pub name: String,
    pub achievement_progress: BTreeMap<String, i64>,
    pub tracks: BTreeMap<String, i64>,
    pub source_values: BTreeMap<String, i64>,
    pub claim_projections: BTreeMap<String, [u8; 16]>,
    pub inventory: BTreeMap<u32, u32>,
    pub now: i64,
    pub level: u32,
    pub sex: source::Sex,
    pub premium: bool,
    pub looks: BTreeSet<(u32, u8)>,
    pub mounts: BTreeSet<u16>,
    pub selected: Option<i64>,
    pub hazard: Option<u8>,
    pub primal: Option<super::primal_hazard::PrimalNpcFacts>,
    pub actor_drunk: Option<bool>,
}
impl source::Facts for Snapshot {
    fn actor_condition(&self, condition: source::ActorCondition) -> Result<bool, source::Blocked> {
        match condition {
            source::ActorCondition::Drunk => {
                self.actor_drunk.ok_or(source::Blocked::MissingSourceFact)
            }
        }
    }
    fn now(&self) -> i64 {
        self.now
    }
    fn track(&self, symbol: &str) -> Result<i64, source::Blocked> {
        let binding = source::embedded()?
            .binding(symbol)
            .ok_or(source::Blocked::MissingSourceFact)?;
        Ok(self
            .source_values
            .get(&binding.key)
            .or_else(|| self.tracks.get(&binding.key))
            .copied()
            .unwrap_or(-1))
    }
    fn inventory(&self, item: u32) -> Result<u32, source::Blocked> {
        if !source::embedded()?.items.iter().any(|i| i.id == item) {
            return Err(source::Blocked::MissingSourceFact);
        }
        Ok(self.inventory.get(&item).copied().unwrap_or(0))
    }
    fn premium(&self) -> Result<bool, source::Blocked> {
        Ok(self.premium)
    }
    fn sex(&self) -> source::Sex {
        self.sex
    }
    fn owned_look_addon(&self, look: u32, addon: u8) -> Result<bool, source::Blocked> {
        Ok(self.looks.contains(&(look, addon)))
    }
    fn kv_bool(&self, scope: &str, key: &str) -> Result<bool, source::Blocked> {
        if scope == "primal-ordeal" && key == "received-prize" {
            return self
                .primal
                .map(|facts| facts.received_prize)
                .ok_or(source::Blocked::MissingSourceFact);
        }
        Err(source::Blocked::MissingSourceFact)
    }
    fn level(&self) -> u32 {
        self.level
    }
    fn owner_points(&self, owner: &str) -> Result<i64, source::Blocked> {
        if owner != "SpikeFame" {
            return Err(source::Blocked::MissingSourceFact);
        }
        Ok(self
            .tracks
            .get("oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_fame_points")
            .copied()
            .unwrap_or(-1)
            .max(0))
    }
    fn owned_mount(&self, id: u16) -> Result<bool, source::Blocked> {
        Ok(self.mounts.contains(&id))
    }
    fn selected_integer(&self, name: &str) -> Option<i64> {
        (name == "hazard_level").then_some(self.selected).flatten()
    }
    fn hazard_can_select(&self, owner: &str, selected: i64) -> Result<bool, source::Blocked> {
        if owner != "hazard.gnomprona-gardens" {
            return Err(source::Blocked::MissingSourceFact);
        }
        Ok(self
            .hazard
            .is_some_and(|max| selected >= 1 && selected <= i64::from(max)))
    }
    fn world_storage(&self, _id: u32, _missing: i64) -> Result<i64, source::Blocked> {
        Err(source::Blocked::MissingSourceFact)
    }
}
impl Snapshot {
    pub fn proof(&self) -> Value {
        let claims: BTreeMap<_, _> = self
            .claim_projections
            .iter()
            .map(|(key, id)| (key, sqlx::types::Uuid::from_bytes(*id).to_string()))
            .collect();
        json!({"tracks":self.tracks,"inventory":self.inventory,"now":self.now,"level":self.level,
        "achievement_progress":self.achievement_progress,"name":self.name,
        "quest_source_values":self.source_values,"quest_claims":claims,
        "actor_condition_drunk":self.actor_drunk,"selected_integer":self.selected,
        "hazard_state":self.primal.map(|p|json!({"unlocked":p.unlocked,"selected":p.selected,"received_prize":p.received_prize})),
        "sex":match self.sex{source::Sex::Female=>"female",source::Sex::Male=>"male"},"premium":self.premium})
    }
}
pub(super) async fn load(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    account: sqlx::types::Uuid,
    node: &NodeIncarnationProof,
    native: &NativeGameplayState,
    premium: Option<&dyn super::super::super::consume_transform_cosmetics::ItemCosmeticPremium>,
    source_request: (&str, &str),
) -> Result<Snapshot, DurabilityError> {
    let (message, npc_key) = source_request;
    let program = source::embedded().map_err(|_| DurabilityError::InvalidStoredState)?;
    let keys: Vec<_> = program.bindings.iter().map(|b| b.key.as_str()).collect();
    if keys.len() > 512 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let tracks:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key LIMIT 513")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut ***tx).await?;
    if tracks.len() > 512 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let (now,level,name):(i64,i64,String)=sqlx::query_as("SELECT game_npc_source_observe_clock(encode($2,'hex')::uuid,$3::text::numeric,$4),npc_progression.level,npc_character.name FROM game_character_progression_state npc_progression JOIN game_character_roots npc_character ON npc_character.character_id=npc_progression.character_id WHERE npc_progression.character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(node.fact().node_id().as_bytes().as_slice()).bind(node.fact().registration_revision().to_string()).bind(node.sql_possession_secret()).fetch_one(&mut ***tx).await?;
    let sex =
        super::super::super::character_appearance::item_action_sex(tx, fence.character_id).await?;
    let definitions: Vec<_> = program
        .items
        .iter()
        .map(|item| {
            (
                item.id,
                super::super::super::item_mint::TypedDefinitionRef {
                    family: item.definition.family.clone(),
                    production_key: item.definition.production_key.clone(),
                    revision_ref: item.definition.revision_ref.clone(),
                },
            )
        })
        .collect();
    let inventory =
        super::super::super::carried_cost::available_many(tx, fence.item_fence(), &definitions)
            .await?;
    let content = native
        .cosmetics()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let facts = super::super::super::character_appearance::read_account_appearance_unlocks(
        tx,
        AccountId::from_bytes(*account.as_bytes())
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        &content.earned_grant_keys(),
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts: Vec<_> = facts
        .into_iter()
        .filter(|f| {
            matches!(
                f.source_kind.as_str(),
                "quest" | "npc" | "npc_wedding" | "mount_taming" | "task_board" | "item_action"
            )
        })
        .map(|f| (f.unlock_key, f.content_revision))
        .collect();
    let earned = EarnedCosmetics {
        content,
        facts: &facts,
    };
    let mut looks = BTreeSet::new();
    for look in &program.looks {
        let wanted = match look.sex.as_str() {
            "female" => crate::domain::appearance::Sex::Female,
            "male" => crate::domain::appearance::Sex::Male,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        for addon in 0..=look.addon_count {
            let starter = if addon == 0 {
                content.outfit_is_starter(&look.outfit_key, wanted)
            } else {
                content.addon_is_starter(&look.outfit_key, wanted, addon)
            };
            if starter || earned.exact_source_look_addon(look.look, addon) == UnlockFact::Compatible
            {
                looks.insert((look.look, addon));
            }
        }
    }
    let mounts = program
        .mounts
        .iter()
        .filter(|m| earned.mount(&m.key) == UnlockFact::Compatible)
        .map(|m| m.id)
        .collect();
    let keys: Vec<_> = program
        .npcs
        .iter()
        .flat_map(|npc| &npc.transitions)
        .flat_map(|transition| &transition.effects)
        .filter_map(|effect| match effect {
            source::Effect::AchievementProgress { name, .. } => Some(plan::achievement(name)),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let rows:Vec<(String,i64)>=sqlx::query_as("SELECT achievement_key,progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key=ANY($2::text[])").bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut ***tx).await?;
    let mut achievement_progress: BTreeMap<_, _> = keys.into_iter().map(|key| (key, 0)).collect();
    achievement_progress.extend(rows);
    let mut source_values = BTreeMap::new();
    let mut claim_projections = BTreeMap::new();
    if npc_key == "oteryn:npc.avar_tar" {
        for key in [
            "oteryn:quest-progress/quest/u7_24/the_annihilator/reward",
            "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet",
        ] {
            let (value,claim):(i64,Option<sqlx::types::Uuid>)=sqlx::query_as("SELECT source_value,claim_transaction_id FROM game_npc_final_claim_projection(encode($1,'hex')::uuid,$2)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(key).fetch_one(&mut ***tx).await?;
            source_values.insert(key.to_owned(), value);
            if let Some(claim) = claim {
                claim_projections.insert(key.to_owned(), *claim.as_bytes());
            }
        }
    }
    let primal = if npc_key == "oteryn:npc.gnomadness" {
        Some(super::primal_hazard::load(tx, fence.character_id).await?)
    } else {
        None
    };
    let hazard = primal
        .map(|facts| u8::try_from(facts.unlocked))
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Snapshot {
        name,
        achievement_progress,
        tracks: tracks.into_iter().collect(),
        source_values,
        claim_projections,
        inventory,
        now,
        level: u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
        sex: match sex {
            crate::domain::appearance::Sex::Female => source::Sex::Female,
            crate::domain::appearance::Sex::Male => source::Sex::Male,
        },
        premium: premium.is_some_and(|owner| owner.current(*account.as_bytes())),
        looks,
        mounts,
        selected: super::money_selector::selected_money(message),
        hazard,
        primal,
        actor_drunk: None,
    })
}
