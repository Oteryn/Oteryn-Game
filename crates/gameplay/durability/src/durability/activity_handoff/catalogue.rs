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
        let cell = |p: oteryn_gameplay_content::content::LogicalCell| serde_json::json!({"x": p.x, "y": p.y, "floor": p.z});
        let mut rows = Vec::with_capacity(templates.len());
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
            rows.push((
                t.boss_key().to_owned(),
                t.room().movement_cells().scope().world_id,
                ActivityContentPin {
                    coordinate_frame_digest: t.room().frame_binding().digest(),
                    map_revision_digest: t.room().map_revision_digest(),
                    server_artifact_digest: t.room().compiled().server_digest(),
                },
                t.activation_frame_binding_digest(),
                policy,
            ));
        }
        rows.sort_by(|a, b| (&a.0, a.1.as_bytes()).cmp(&(&b.0, b.1.as_bytes())));
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                lock_admission_relations(&mut tx).await?;
                if !prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                for (key, world, pin, activation_frame, policy) in rows {
                    // The global row is only the historical FK identity. It is
                    // never overwritten or used instead of a scoped publication.
                    sqlx::query("INSERT INTO game_boss_room_catalogue SELECT r.* FROM jsonb_populate_record(NULL::game_boss_room_catalogue,$1) r ON CONFLICT(boss_key) DO NOTHING")
                        .bind(&policy).execute(&mut *tx).await?;
                    let inserted = sqlx::query("INSERT INTO game_boss_room_publications SELECT r.*,encode($2,'hex')::uuid,$3,$4,$5,$6 FROM jsonb_populate_record(NULL::game_boss_room_catalogue,$1) r ON CONFLICT(world_id,boss_key,content_revision,origin_frame_digest,origin_map_digest,server_artifact_digest) DO NOTHING")
                        .bind(&policy).bind(world.as_bytes().as_slice())
                        .bind(pin.coordinate_frame_digest.as_slice()).bind(pin.map_revision_digest.as_slice())
                        .bind(pin.server_artifact_digest.as_slice()).bind(activation_frame.as_slice())
                        .execute(&mut *tx).await?.rows_affected();
                    if inserted == 0 {
                        let equal: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_boss_room_publications p WHERE p.world_id=encode($1,'hex')::uuid AND p.boss_key=$2 AND p.content_revision=$3 AND p.origin_frame_digest=$4 AND p.origin_map_digest=$5 AND p.server_artifact_digest=$6 AND p.activation_frame_digest=$7 AND to_jsonb(p)-ARRAY['world_id','origin_frame_digest','origin_map_digest','server_artifact_digest','activation_frame_digest']=$8)")
                            .bind(world.as_bytes().as_slice()).bind(&key).bind(policy["content_revision"].as_str())
                            .bind(pin.coordinate_frame_digest.as_slice()).bind(pin.map_revision_digest.as_slice())
                            .bind(pin.server_artifact_digest.as_slice()).bind(activation_frame.as_slice()).bind(&policy)
                            .fetch_one(&mut *tx).await?;
                        if !equal { return Err(DurabilityError::Unavailable); }
                    }
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(())
            })
        }).await
    }
}
