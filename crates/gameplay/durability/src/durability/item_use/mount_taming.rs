//! MOUNTTAME: the existing one-unit use, earned appearance fact and achievement owners,
//! composed in one fenced transaction. Non-consuming failures still retain their draw.
use super::*;
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::content::native_gameplay::mount_taming::{PROVENANCE, rules};
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetic, EarnedCosmetics, UnlockFact,
};
use crate::durability::account_achievement::{
    AchievementGrantRequest, FencedGrantingCharacter, record_achievement_grant,
};
use crate::foundation::CommandRef;

pub const TAMING_CAUSE: u32 = 328;

pub struct MountTamingRequest {
    pub item: ItemUseRequest,
    pub binding: [u8; 32],
    pub target_identity: [u8; 16],
    pub target_generation: u64,
    pub creature_key: String,
    pub cosmetics: NativeCosmeticsView,
    pub draw: u64,
    pub achievements: Vec<AchievementGrantRequest>,
}

#[derive(Debug, Clone, Copy)]
pub struct MountTamingReceipt {
    pub outcome: u8,
}

impl DurabilityRoot {
    /// Historical outcome only: never returns a current Character or runtime authority.
    pub async fn read_mount_taming_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        binding: [u8; 32],
    ) -> Result<Option<MountTamingReceipt>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let result = receipt(&mut tx, &fence, command, binding).await;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await?
    }

    /// `None` means already owned: no attempt, consumption, grant or receipt was written.
    pub async fn commit_mount_taming(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: MountTamingRequest,
    ) -> Result<Option<MountTamingReceipt>> {
        request.item.validate()?;
        if request.item.kind != ItemUseKind::MountTaming || request.target_generation == 0 {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result = attempt(&mut tx, &node, &fence, &request).await;
                    match result {
                        Ok(value) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(ItemUseError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn receipt(
    tx: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    binding: [u8; 32],
) -> Result<Option<MountTamingReceipt>> {
    if command.game_session_id() != fence.game_session_id {
        return Err(ItemUseError::InvalidInput);
    }
    let row = sqlx::query(
        "SELECT character_id::text, world_id::text, channel_id::text, wire_binding, outcome \
        FROM game_mount_taming_attempts WHERE game_session_id=encode($1,'hex')::uuid \
        AND command_id=$2::text::numeric",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let scope = RuntimeScopeRefV1::channel(
        crate::foundation::WorldId::decode(&uuid_text(&row.try_get::<String, _>("world_id")?)?)
            .map_err(|_| ItemUseError::InvalidInput)?,
        crate::foundation::ChannelId::decode(&uuid_text(&row.try_get::<String, _>("channel_id")?)?)
            .map_err(|_| ItemUseError::InvalidInput)?,
    );
    if uuid_text(&row.try_get::<String, _>("character_id")?)? != *fence.character_id.as_bytes()
        || scope != fence.runtime_scope
        || row.try_get::<Vec<u8>, _>("wire_binding")? != binding
    {
        return Err(ItemUseError::ConflictingCommand);
    }
    Ok(Some(MountTamingReceipt {
        outcome: u8::try_from(row.try_get::<i16, _>("outcome")?)
            .map_err(|_| ItemUseError::InvalidInput)?,
    }))
}

async fn attempt(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &MountTamingRequest,
) -> Result<Option<MountTamingReceipt>> {
    let item = &request.item;
    let command = CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(item.command_id)
            .map_err(|_| ItemUseError::InvalidInput)?,
    );
    if let Some(found) = receipt(tx, fence, command, request.binding).await? {
        return Ok(Some(found));
    }
    let root = assert_item_only_fence(tx, fence, node)
        .await?
        .map_err(|_| ItemUseError::AuthorityRejected)?;
    if root.content_revision != item.content_revision
        || root.ruleset_revision != item.ruleset_revision
    {
        return Err(ItemUseError::StaleState);
    }
    let rule = rules()
        .iter()
        .find(|r| {
            r.item_key == item.source_definition.production_key
                && r.creature_key == request.creature_key
        })
        .ok_or(ItemUseError::InvalidInput)?;
    if request.cosmetics.earned_unlock(
        &rule.grant_key,
        PROVENANCE,
        EarnedCosmetic::Mount(&rule.mount_key),
    ) != UnlockFact::Compatible
    {
        return Err(ItemUseError::InvalidInput);
    }
    let inventory = super::carried_inventory::read_use_inventory(
        tx,
        fence,
        item.content_revision.clone(),
        item.ruleset_revision.clone(),
    )
    .await?;
    let bag = inventory
        .backpack
        .as_ref()
        .ok_or(ItemUseError::StaleState)?;
    let source = bag
        .entries
        .iter()
        .chain(inventory.nested.iter().map(|(_, e)| e))
        .find(|e| e.item.item_instance_id == item.source_item_instance_id)
        .ok_or(ItemUseError::StaleState)?;
    if source.item.definition != item.source_definition
        || source.item.quantity != item.source_quantity_before
    {
        return Err(ItemUseError::StaleState);
    }
    if rule.required_equipment.as_ref().is_some_and(|key| {
        !inventory
            .equipment
            .iter()
            .any(|item| item.definition.production_key == *key)
    }) {
        return Err(ItemUseError::StaleState);
    }
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(ItemUseError::InvalidInput);
    };
    lock_use_path(
        tx,
        fence.character_id.as_bytes(),
        &bag.backpack.item_instance_id,
        world_id.as_bytes(),
        item,
    )
    .await?;
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut ***tx)
    .await?;
    // Different Characters of one account must not burn two items for the same grant.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("mount-taming:{account}:{}", rule.grant_key))
        .execute(&mut ***tx)
        .await?;
    let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=$2)")
        .bind(account).bind(&rule.grant_key).fetch_one(&mut ***tx).await?;
    if owned {
        return Ok(None);
    }
    let account_id = crate::domain::AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| ItemUseError::InvalidInput)?;
    let facts = super::super::character_appearance::read_account_appearance_unlocks(
        tx,
        account_id,
        &request.cosmetics.earned_grant_keys(),
    )
    .await
    .map_err(|_| ItemUseError::InvalidInput)?;
    let facts: Vec<_> = facts
        .into_iter()
        .filter(|fact| matches!(fact.source_kind.as_str(), "quest" | "npc" | "mount_taming"))
        .map(|fact| (fact.unlock_key, fact.content_revision))
        .collect();
    if (EarnedCosmetics {
        content: &request.cosmetics,
        facts: &facts,
    })
    .mount(&rule.mount_key)
        == UnlockFact::Compatible
    {
        return Ok(None);
    }
    let outcome = rule.decide(request.draw);
    let (consume, remove, _) = rule.effects(outcome).ok_or(ItemUseError::InvalidInput)?;
    if outcome == 0 {
        if request.achievements.len() != 2
            || request.achievements[0].achievement_key != rule.achievement_key
            || request.achievements[1].achievement_key != "oteryn:achievement/natural_born_cowboy"
            || request.achievements.iter().any(|a| {
                a.source.kind != "mount_taming" || a.source.event_id != item.transaction_id
            })
        {
            return Err(ItemUseError::InvalidInput);
        }
        for achievement in &request.achievements {
            let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
            record_achievement_grant(tx, granter, achievement)
                .await?
                .map_err(|_| ItemUseError::InvalidInput)?;
        }
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts \
            (grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) \
            VALUES(encode($1,'hex')::uuid,$2,$3,$4,encode($5,'hex')::uuid,'mount_taming',$1)")
            .bind(item.transaction_id.as_slice()).bind(account).bind(&rule.grant_key).bind(PROVENANCE)
            .bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
        sqlx::query(
            "INSERT INTO game_account_appearance_unlocks VALUES($1,$2,encode($3,'hex')::uuid)",
        )
        .bind(account)
        .bind(&rule.grant_key)
        .bind(item.transaction_id.as_slice())
        .execute(&mut ***tx)
        .await?;
    }
    if consume {
        let tuple = tx.tuple();
        write(tx, fence, item, tuple).await?;
    }
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(ItemUseError::InvalidInput);
    };
    sqlx::query("INSERT INTO game_mount_taming_attempts \
        (attempt_id,character_id,world_id,channel_id,scope_generation,game_session_id,command_id,wire_binding,\
         target_identity,target_generation,item_key,outcome,draw,removed,consumed_transaction_id,grant_key) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
        $5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,$8,$9,$10::text::numeric,$11,$12,$13::text::numeric,$14,\
        CASE WHEN $15 THEN encode($1,'hex')::uuid ELSE NULL END,$16)")
        .bind(item.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(item.command_id.to_string()).bind(request.binding.as_slice()).bind(request.target_identity.as_slice())
        .bind(request.target_generation.to_string()).bind(&rule.item_key).bind(i16::from(outcome))
        .bind(request.draw.to_string()).bind(remove).bind(consume).bind(&rule.grant_key).execute(&mut ***tx).await?;
    Ok(Some(MountTamingReceipt { outcome }))
}
