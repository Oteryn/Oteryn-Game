//! Crystal 201189f data/libs/functions/boss_lever.lua:63-76,183-218:
//! publish only content-qualified lever policies; runtime admission stays fenced.
use super::*;
use oteryn_gameplay_content::content::bosses::activity_template::QualifiedActivityRoomTemplate;

impl ActivityHandoffOwner {
    pub async fn publish_qualified_boss_rooms(
        &self,
        node: &NodeIncarnationProof,
        templates: &[QualifiedActivityRoomTemplate],
    ) -> Result<()> {
        self.publish_qualified_boss_rooms_with_origin(node, templates, None)
            .await
    }

    pub(super) async fn publish_qualified_boss_rooms_from_world(
        &self,
        node: &NodeIncarnationProof,
        templates: &[QualifiedActivityRoomTemplate],
        active: &crate::content::ActiveGeneration,
        origin: &crate::content::QualifiedNativeEntryRoom,
        pin: &crate::foundation::ChannelContentPin,
    ) -> Result<()> {
        if active
            .qualify_published_world_context(origin, pin)
            .is_none()
            || templates.iter().any(|template| {
                template.issue_runtime_pin(active).is_err()
                    || template.room().movement_cells().scope().world_id != pin.world_id()
            })
        {
            return Err(DurabilityError::Unavailable);
        }
        self.publish_qualified_boss_rooms_with_origin(node, templates, Some(pin))
            .await
    }

    async fn publish_qualified_boss_rooms_with_origin(
        &self,
        node: &NodeIncarnationProof,
        templates: &[QualifiedActivityRoomTemplate],
        origin: Option<&crate::foundation::ChannelContentPin>,
    ) -> Result<()> {
        let cell = |p: oteryn_gameplay_content::content::LogicalCell| serde_json::json!({"x": p.x, "y": p.y, "floor": p.z});
        let mut templates: Vec<_> = templates.iter().collect();
        templates.sort_by_key(|t| (t.boss_key(), *t.room().movement_cells().scope().world_id.as_bytes()));
        let mut policies = Vec::with_capacity(templates.len());
        let mut worlds = Vec::with_capacity(templates.len());
        let mut frames = Vec::with_capacity(templates.len());
        let mut maps = Vec::with_capacity(templates.len());
        let mut artifacts = Vec::with_capacity(templates.len());
        let mut activation_frames = Vec::with_capacity(templates.len());
        for t in templates {
            let policy = serde_json::json!({
                "boss_key": t.boss_key(), "boss_creature_key": t.creature_key(),
                "content_revision": t.content_revision(), "room_template_key": t.template_key(),
                "template_revision": t.template_revision(), "minimum_players": t.minimum_players(),
                "maximum_players": t.maximum_players(), "minimum_level": t.minimum_level(),
                "cooldown_seconds": t.cooldown_seconds(), "time_limit_seconds": t.time_limit_seconds(),
                "after_kill_seconds": t.after_kill_seconds(),
                "lever_positions": t.lever_positions().iter().copied().map(cell).collect::<Vec<_>>(),
                "entry_positions": t.entry_positions().iter().copied().map(cell).collect::<Vec<_>>(),
                "exit_position": cell(t.exit_position()), "boss_position": cell(t.boss_position()),
                "reward_key": t.boss_key(), "enabled": true,
            });
            policies.push(policy);
            worlds.push(t.room().movement_cells().scope().world_id.as_bytes().to_vec());
            frames.push(origin.map_or_else(
                || t.room().frame_binding().digest(),
                |pin| pin.frame_binding_digest(),
            ).to_vec());
            maps.push(origin.map_or_else(
                || t.room().map_revision_digest(),
                |pin| pin.map_revision_digest(),
            ).to_vec());
            artifacts.push(t.room().compiled().server_digest().to_vec());
            activation_frames.push(t.activation_frame_binding_digest().to_vec());
        }
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                lock_admission_relations(&mut tx).await?;
                if !prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                if !policies.is_empty() {
                    // The global row is a historical FK identity. Give duplicate keys the
                    // same deterministic first policy as the original ordered publication.
                    sqlx::query(
                        "INSERT INTO game_boss_room_catalogue
                         SELECT r.* FROM unnest($1::jsonb[]) WITH ORDINALITY AS input(policy, ord)
                         CROSS JOIN LATERAL jsonb_populate_record(NULL::game_boss_room_catalogue,input.policy) r
                         ORDER BY input.ord ON CONFLICT (boss_key) DO NOTHING",
                    ).bind(&policies).execute(&mut *tx).await?;
                    sqlx::query(PUBLISH_ROOMS)
                        .bind(&policies).bind(&worlds).bind(&frames).bind(&maps)
                        .bind(&artifacts).bind(&activation_frames).execute(&mut *tx).await?;
                    // Every row must match, including preexisting rows and duplicate keys in
                    // this batch. A disagreement rolls back both inserts.
                    let equal: bool = sqlx::query_scalar(VALIDATE_ROOMS)
                        .bind(&policies).bind(&worlds).bind(&frames).bind(&maps)
                        .bind(&artifacts).bind(&activation_frames).fetch_one(&mut *tx).await?;
                    if !equal { return Err(DurabilityError::Unavailable); }
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(())
            })
        }).await
    }
}

const PUBLISH_ROOMS: &str = "
    INSERT INTO game_boss_room_publications
    SELECT r.*,encode(i.world,'hex')::uuid,i.frame,i.map,i.artifact,i.activation_frame
    FROM unnest($1::jsonb[],$2::bytea[],$3::bytea[],$4::bytea[],$5::bytea[],$6::bytea[])
        WITH ORDINALITY AS i(policy,world,frame,map,artifact,activation_frame,ord)
    CROSS JOIN LATERAL jsonb_populate_record(NULL::game_boss_room_catalogue,i.policy) r
    ORDER BY i.ord
    ON CONFLICT (world_id,boss_key,content_revision,origin_frame_digest,origin_map_digest,server_artifact_digest)
    DO NOTHING";

const VALIDATE_ROOMS: &str = "
    SELECT NOT EXISTS (
      SELECT 1
      FROM unnest($1::jsonb[],$2::bytea[],$3::bytea[],$4::bytea[],$5::bytea[],$6::bytea[])
          AS i(policy,world,frame,map,artifact,activation_frame)
      WHERE NOT EXISTS (
        SELECT 1 FROM game_boss_room_publications p
        WHERE p.world_id=encode(i.world,'hex')::uuid AND p.boss_key=i.policy->>'boss_key'
          AND p.content_revision=i.policy->>'content_revision' AND p.origin_frame_digest=i.frame
          AND p.origin_map_digest=i.map AND p.server_artifact_digest=i.artifact
          AND p.activation_frame_digest=i.activation_frame
          AND to_jsonb(p)-ARRAY['world_id','origin_frame_digest','origin_map_digest','server_artifact_digest','activation_frame_digest']=i.policy
      )
    )";
