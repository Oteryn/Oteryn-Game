//! Fixed source Soul War recipes and all-roster Quest checks under current authority.
use super::*;
impl ActivityHandoffOwner {
    pub async fn activate_soul_war_boss_catalogue(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        active: &crate::content::ActiveGeneration,
        template: &crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<bool> {
        if !is_soul_war(template.boss_key())
            || template.source_path() != "data-global/lib/quests/soul_war.lua"
            || template.source_sha256()
                != "89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75"
            || template.issue_runtime_pin(active).is_err()
        {
            return Ok(false);
        }
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(false);
        };
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        // Publication is append-only and checks exact policy equality on replay.
        // The following pass independently re-proves the Character, Node,
        // runtime and current activation before accepting this interpretation.
        self.publish_qualified_boss_rooms(node, std::slice::from_ref(template))
            .await?;
        let root = self.root.clone();
        let node = node.clone();
        let fact = node.fact();
        let revision = template.content_revision().to_owned();
        let boss_key = template.boss_key().to_owned();
        let server = active.identity().server_artifact_digest();
        let client = active.identity().client_artifact_digest();
        let activation = active.activation_sequence();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline| {
            Box::pin(async move {
                let mut tx=begin_semantic_transaction(holder,deadline).await?;
                assert_recovery_fence(&mut tx,&recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if retained_participant(&mut tx,&root,fence).await?.is_none()
                    || assert_item_only_fence(&mut tx,&fence,&node).await?.is_err() {return Ok(false);}
                let accepted:bool=sqlx::query_scalar("SELECT game_soul_war_boss_catalogue_activate(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),encode($4,'hex')::uuid,$5::text::numeric(20,0),$6,$7::text::numeric(20,0),$8,$9,$10,$11)")
                    .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
                    .bind(activation.to_string()).bind(server.as_slice()).bind(client.as_slice()).bind(revision).bind(boss_key).fetch_one(&mut *tx).await?;
                commit_semantic_transaction(tx,deadline).await?;
                Ok(accepted)
            })
        }).await
    }
}

use oteryn_gameplay_quests::quest::soul_war::SoulWarState;

pub(super) fn is_soul_war(key: &str) -> bool {
    matches!(
        key,
        "oteryn:boss.goshnar_s_malice"
            | "oteryn:boss.goshnar_s_spite"
            | "oteryn:boss.goshnar_s_greed"
            | "oteryn:boss.goshnar_s_hatred"
            | "oteryn:boss.goshnar_s_cruelty"
            | "oteryn:boss.goshnar_s_megalomania"
    )
}

/// entry::enter_in_transaction already holds ALL source sessions, Character roots,
/// and current item fences before boss_room::prepare_in_transaction calls here.
/// Thus a Quest write cannot race these facts or evade a nonprincipal check.
pub(super) async fn validate_entry(
    tx: &mut Transaction<'_, Postgres>,
    r: &ActivityEnterRequest,
    now: i64,
) -> Result<bool> {
    if !is_soul_war(&r.activity_key) || now < 0 {
        return Ok(false);
    }
    for participant in &r.participants {
        let row = sqlx::query("SELECT teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed FROM game_character_soul_war WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
            .bind(participant.source.character_id.as_bytes().as_slice())
            .fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let state = SoulWarState {
            teleport_access: row.try_get("teleport_access")?,
            defeated_bosses: u8::try_from(row.try_get::<i16, _>("defeated_bosses")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            first_taint_at: row.try_get("first_taint_at")?,
            megalomania_completed: row.try_get("megalomania_completed")?,
            final_reward_claimed: row.try_get("final_reward_claimed")?,
        };
        state
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let state = state.expired(now / 1000);
        if !state.access()
            || (r.activity_key == "oteryn:boss.goshnar_s_megalomania" && !state.chain_ready())
        {
            return Ok(false);
        }
    }
    Ok(true)
}
