//! Gnomadness calls the existing bounded Hazard state; NPC owns the sole Root step.
use super::context::NpcProgramContext;
use crate::durability::{DurabilityError, item_mint_audit::Type2Transaction};
use sqlx::Row;
#[derive(Clone, Copy)]
pub(crate) struct PrimalNpcFacts {
    pub unlocked: u16,
    pub selected: u16,
    pub received_prize: bool,
}
pub(crate) async fn load(
    tx: &mut Type2Transaction<'_>,
    character: crate::domain::CharacterId,
) -> Result<PrimalNpcFacts, DurabilityError> {
    let state:Option<(i16,i16)>=sqlx::query_as("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let (unlocked, selected) = state.unwrap_or((1, 1));
    let received:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_npc_primal_prize_claims WHERE character_id=encode($1,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    if !(1..=12).contains(&unlocked) || !(1..=unlocked).contains(&selected) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(PrimalNpcFacts {
        unlocked: u16::try_from(unlocked).map_err(|_| DurabilityError::InvalidStoredState)?,
        selected: u16::try_from(selected).map_err(|_| DurabilityError::InvalidStoredState)?,
        received_prize: received,
    })
}
pub(crate) async fn persist_child(
    tx: &mut Type2Transaction<'_>,
    ctx: &NpcProgramContext<'_>,
    requested: u16,
) -> Result<(), DurabilityError> {
    let prize = match (ctx.npc_key, ctx.source_key) {
        ("oteryn:npc.gnomadness", "oteryn:npc-cosmetic/gnomadness/ripptor-prize") => true,
        ("oteryn:npc.gnomadness", "oteryn:npc-cosmetic/gnomadness/ripptor-select") => false,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let before = load(tx, ctx.fence.character_id).await?;
    if !(1..=before.unlocked).contains(&requested)
        || (prize && (requested < 6 || before.received_prize))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_character_hazard(character_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(ctx.fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    let result = vec![
        u8::try_from(before.unlocked).map_err(|_| DurabilityError::InvalidStoredState)?,
        u8::try_from(requested).map_err(|_| DurabilityError::InvalidStoredState)?,
    ];
    let inserted=sqlx::query("INSERT INTO game_npc_source_hazard_effects(npc_transaction_id,character_id,game_session_id,command_id,source_key,program_sha256,committed_character_revision,unlocked,selected_before,selected_after,result,prize) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7::text::numeric,$8,$9,$10,$11,$12)")
        .bind(ctx.npc_transaction.as_slice()).bind(ctx.fence.character_id.as_bytes().as_slice()).bind(ctx.fence.game_session_id.as_bytes().as_slice())
        .bind(ctx.command.command_id().get().to_string()).bind(ctx.source_key).bind(ctx.program_sha256.as_slice()).bind(ctx.next_revision.to_string())
        .bind(i16::try_from(before.unlocked).map_err(|_|DurabilityError::InvalidStoredState)?).bind(i16::try_from(before.selected).map_err(|_|DurabilityError::InvalidStoredState)?)
        .bind(i16::try_from(requested).map_err(|_|DurabilityError::InvalidStoredState)?).bind(result).bind(prize).execute(&mut ***tx).await?.rows_affected();
    if inserted != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if before.selected != requested {
        let changed=sqlx::query("UPDATE game_character_hazard SET selected=$2 WHERE character_id=encode($1,'hex')::uuid AND unlocked=$3 AND selected=$4")
            .bind(ctx.fence.character_id.as_bytes().as_slice()).bind(i16::try_from(requested).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(i16::try_from(before.unlocked).map_err(|_|DurabilityError::InvalidStoredState)?).bind(i16::try_from(before.selected).map_err(|_|DurabilityError::InvalidStoredState)?)
            .execute(&mut ***tx).await?.rows_affected();
        if changed != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    if prize {
        sqlx::query("INSERT INTO game_npc_primal_prize_claims(character_id,npc_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(ctx.fence.character_id.as_bytes().as_slice()).bind(ctx.npc_transaction.as_slice()).execute(&mut ***tx).await?;
    }
    Ok(())
}

impl crate::durability::DurabilityRoot {
    /// Resolve only the original immutable source command while its advisory
    /// lock excludes an in-flight commit. No new hazard request is performed.
    pub async fn read_npc_primal_hazard_child(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        request: super::super::NpcCosmeticRequest,
    ) -> Result<Option<u16>, crate::durability::character_progression::CharacterProgressionError>
    {
        if request.npc != "oteryn:npc.gnomadness"
            || request.command.game_session_id() != fence.game_session_id
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let binding = super::super::binding(fence.character_id, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:npc-cosmetic:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut *tx).await?;
            let row=sqlx::query("SELECT r.request_binding,r.original_character_revision::text,r.source_key,a.selected_after,a.unlocked,a.source_key=r.source_key AND a.character_id=r.character_id AND a.game_session_id=r.game_session_id AND a.command_id=r.command_id AND a.program_sha256=r.program_sha256 AND a.committed_character_revision=r.committed_character_revision AS same_child FROM game_npc_cosmetic_receipts r LEFT JOIN game_npc_source_hazard_effects a ON a.npc_transaction_id=r.transaction_id WHERE r.character_id=encode($1,'hex')::uuid AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric AND r.npc_key='oteryn:npc.gnomadness'")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let selected=if let Some(row)=row {
                if row.try_get::<Vec<u8>,_>("request_binding")?!=binding
                    || super::super::numeric_u64(&row,"original_character_revision")?!=fence.expected_character_revision.get() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                match row.try_get::<Option<i16>,_>("selected_after")? {
                    Some(selected)=>{
                        let unlocked:i16=row.try_get("unlocked")?;
                        if !row.try_get::<Option<bool>,_>("same_child")?.unwrap_or(false)
                            || !(1..=12).contains(&unlocked) || !(1..=unlocked).contains(&selected) {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        Some(u16::try_from(selected).map_err(|_|DurabilityError::InvalidStoredState)?)
                    },
                    None=>{
                        if matches!(row.try_get::<String,_>("source_key")?.as_str(),
                            "oteryn:npc-cosmetic/gnomadness/ripptor-prize"|"oteryn:npc-cosmetic/gnomadness/ripptor-select") {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        None
                    }
                }
            } else {None};
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(selected)
        })).await.map_err(Into::into)
    }
}
