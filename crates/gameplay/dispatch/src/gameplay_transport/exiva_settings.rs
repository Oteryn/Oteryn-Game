//! Fenced, target-owned Exiva mode and player allow-list editing.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::exiva::{Operation, Request};

macro_rules! authority_methods {
    () => {
        fn exiva_settings(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command: u64,
            _request: oteryn_protocol_oteryn::exiva::Request,
        ) -> impl std::future::Future<Output = Option<bool>> {
            async { Some(false) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn exiva_settings(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::exiva::Request,
        ) -> Option<bool> {
            exiva_settings::execute(self, admitted, command, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: Request,
) -> Option<bool> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::exiva::CAPABILITY_EXIVA_SETTINGS_V1)
    {
        return Some(false);
    }
    let fence = admitted.item_fence?;
    let active = owner.active_generation?;
    let content = active.native_gameplay()?;
    let permit = owner.spell_lane_permit().await?;
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    let pass = owner.root.try_issue_semantic_pass().ok()?;
    let mut context = owner;
    pass.run_with_context(&mut context, move |holder, deadline, owner| Box::pin(async move {
        let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
        let authority = item_tx::assert_spell_item_authority_in_transaction(
            &mut tx, &permit, owner.root, owner.character, owner.holder, &fence,
            command, content.source_digest(),
        ).await.map_err(|_| DurabilityError::Unavailable)?;
        // The owner seal locks the character; keep the writer's lock order even
        // when a preference row has not yet been initialized.
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(authority.character_id_bytes().as_slice()).fetch_one(&mut *tx).await?;
        let replay: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_spell_privacy_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0))")
            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command_id.to_string()).fetch_one(&mut *tx).await?;
        if replay { return Ok(true); }
        let row: Option<(String, serde_json::Value)> = sqlx::query_as("SELECT revision::text,preferences FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(authority.character_id_bytes().as_slice()).fetch_optional(&mut *tx).await?;
        let (revision, mut preferences) = match row {
            Some((revision, value)) => (Some(revision.parse().map_err(|_| DurabilityError::InvalidStoredState)?), serde_json::from_value(value).map_err(|_| DurabilityError::InvalidStoredState)?),
            None => (None, crate::durability::spell_privacy::ExivaPreferences::source_initial_preferences()),
        };
        match request.operation {
            Operation::SetModes => {
                preferences.allow_all = request.modes & 1 != 0;
                preferences.allow_player_whitelist = request.modes & 2 != 0;
                preferences.allow_own_guild = request.modes & 4 != 0;
                preferences.allow_own_party = request.modes & 8 != 0;
                preferences.allow_vip_list = request.modes & 16 != 0;
            }
            Operation::AddPlayer => {
                let player = request.player.ok_or(DurabilityError::InvalidStoredState)?;
                let known: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots source JOIN game_character_roots target USING(world_id) WHERE source.character_id=encode($1,'hex')::uuid AND target.character_id=encode($2,'hex')::uuid AND target.lifecycle=1)")
                    .bind(authority.character_id_bytes().as_slice()).bind(player.as_slice()).fetch_one(&mut *tx).await?;
                if !known { return Ok(false); }
                preferences.player_whitelist.push(player);
                preferences.player_whitelist.sort_unstable();
                preferences.player_whitelist.dedup();
                // Canary @47dfd51f config.lua.dist:99 (maxExivaWhitelist).
                if preferences.player_whitelist.len() > 100 {
                    return Ok(false);
                }
            }
            Operation::RemovePlayer => {
                preferences.player_whitelist.retain(|id| Some(*id) != request.player);
            }
            Operation::AddGuild => {
                let guild = request.guild.ok_or(DurabilityError::InvalidStoredState)?;
                let known: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots source JOIN game_guilds guild ON guild.world_id=source.world_id WHERE source.character_id=encode($1,'hex')::uuid AND guild.guild_id=encode($2,'hex')::uuid AND guild.state<>4)")
                    .bind(authority.character_id_bytes().as_slice()).bind(guild.as_slice()).fetch_one(&mut *tx).await?;
                if !known { return Ok(false); }
                preferences.guild_whitelist.push(guild);
                preferences.guild_whitelist.sort_unstable();
                preferences.guild_whitelist.dedup();
                if preferences.guild_whitelist.len() > 100 {
                    return Ok(false);
                }
            }
            Operation::RemoveGuild => {
                preferences.guild_whitelist.retain(|id| Some(*id) != request.guild);
            }
        }
        crate::durability::spell_privacy::write_exiva_preferences_in_transaction(
            &mut tx, &authority, revision, &preferences,
        ).await.map_err(|_| DurabilityError::Unavailable)?;
        tx.commit().await?;
        Ok(true)
    })).await.map_or(Some(false), Some)
}
