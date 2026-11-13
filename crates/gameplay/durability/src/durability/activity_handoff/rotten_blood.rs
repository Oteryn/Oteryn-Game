//! Publish only five pinned Rotten Blood recipes under current native authority.
use super::*;
impl ActivityHandoffOwner {
    pub async fn activate_rotten_blood_boss_catalogue(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        active: &crate::content::ActiveGeneration,
        template: &crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<bool> {
        if ![
            (
                "oteryn:boss.ichgahal",
                "oteryn:creature.ichgahal",
                "data-global/scripts/quests/rotten_blood_quest/rotten_ichgahal.lua",
                "0822f2c430b389aa74bc3109db975e2bb96cd69385e6fc8fe26991b9b69f9b7c",
            ),
            (
                "oteryn:boss.murcion",
                "oteryn:creature.murcion",
                "data-global/scripts/quests/rotten_blood_quest/rotten_murcion.lua",
                "48f572ec9c418340bb07490334a216e81f4361627495e5c221d2a2d45b71d727",
            ),
            (
                "oteryn:boss.vemiath",
                "oteryn:creature.vemiath",
                "data-global/scripts/quests/rotten_blood_quest/rotten_vemiath.lua",
                "9b163af79214effbb5e9d2bd3d54290c266b988b33a89d809a70563e24c0c6c5",
            ),
            (
                "oteryn:boss.chagorz",
                "oteryn:creature.chagorz",
                "data-global/scripts/quests/rotten_blood_quest/rotten_chagorz.lua",
                "d3f632880a7e871e0303994905ef3e814b22d6f797e51b732d4ce7f7c49050c2",
            ),
            (
                "oteryn:boss.bakragore",
                "oteryn:creature.bakragore",
                "data-global/scripts/quests/rotten_blood_quest/rotten_bakragore.lua",
                "821b49caf5e1c55f8a0e9a83b5f1158e5f34d3b3876681c5d9adfb6eedd96189",
            ),
        ]
        .iter()
        .any(|(boss, creature, path, sha)| {
            template.boss_key() == *boss
                && template.creature_key() == *creature
                && template.source_path() == *path
                && template.source_sha256() == *sha
        }) || template.issue_runtime_pin(active).is_err()
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
        let boss = template.boss_key().to_owned();
        let revision = template.content_revision().to_owned();
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
                let accepted:bool=sqlx::query_scalar("SELECT game_rotten_blood_boss_catalogue_activate($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),encode($5,'hex')::uuid,$6::text::numeric(20,0),$7,$8::text::numeric(20,0),$9,$10,$11)")
                    .bind(boss).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
                    .bind(activation.to_string()).bind(server.as_slice()).bind(client.as_slice()).bind(revision).fetch_one(&mut *tx).await?;
                commit_semantic_transaction(tx,deadline).await?;
                Ok(accepted)
            })
        }).await
    }
}

/// The shared admission owner already holds every current Character root/fence.
/// Reload actual Quest state in that transaction for the full resolved roster.
pub(super) async fn validate_entry(
    tx: &mut Transaction<'_, Postgres>,
    request: &ActivityEnterRequest,
    _now: i64,
) -> Result<bool> {
    let creature = match request.activity_key.as_str() {
        "oteryn:boss.ichgahal" => "oteryn:creature.ichgahal",
        "oteryn:boss.murcion" => "oteryn:creature.murcion",
        "oteryn:boss.vemiath" => "oteryn:creature.vemiath",
        "oteryn:boss.chagorz" => "oteryn:creature.chagorz",
        "oteryn:boss.bakragore" => "oteryn:creature.bakragore",
        _ => return Ok(false),
    };
    let boss =
        oteryn_gameplay_quests::quest::rotten_blood::RottenBloodBoss::from_creature_key(creature)
            .ok_or(DurabilityError::InvalidStoredState)?;
    if boss == oteryn_gameplay_quests::quest::rotten_blood::RottenBloodBoss::Bakragore {
        let mut selected = None;
        for participant in &request.participants {
            let p = participant.source_position.cell;
            let partition = if p.floor == 15 && p.y == 32392 && (33010..=33014).contains(&p.x) {
                0
            } else if p.floor == 15 && p.y == 32398 && (33074..=33078).contains(&p.x) {
                1
            } else {
                return Ok(false);
            };
            if selected.is_some_and(|before| before != partition) {
                return Ok(false);
            }
            selected = Some(partition);
        }
    }
    for participant in &request.participants {
        let character = participant.source.character_id;
        let level:Option<i64>=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid AND content_revision=$2 FOR SHARE")
            .bind(character.as_bytes().as_slice()).bind(&request.content_revision).fetch_optional(&mut **tx).await?;
        let Some(level) = level.and_then(|value| u32::try_from(value).ok()) else {
            return Ok(false);
        };
        let state = super::super::quest_state::rotten_blood::read_state(tx, character).await?;
        if state.validate_entry(level, Some(boss)).is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn is_rotten(key: &str) -> bool {
    matches!(
        key,
        "oteryn:boss.murcion"
            | "oteryn:boss.ichgahal"
            | "oteryn:boss.vemiath"
            | "oteryn:boss.chagorz"
            | "oteryn:boss.bakragore"
    )
}
