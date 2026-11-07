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
        let cell = |p: oteryn_gameplay_content::content::LogicalCell| {
            serde_json::json!({"x": p.x, "y": p.y, "floor": p.z})
        };
        let mut rows: Vec<_> = templates.iter().map(|t| (
            t.boss_key().to_owned(), t.creature_key().to_owned(),
            t.content_revision().to_owned(), t.template_key().to_owned(),
            t.template_revision().to_owned(), t.minimum_players() as i16,
            t.maximum_players() as i16, i64::from(t.minimum_level()),
            i64::from(t.cooldown_seconds()), i64::from(t.time_limit_seconds()),
            i64::from(t.after_kill_seconds()),
            serde_json::Value::Array(t.lever_positions().iter().copied().map(cell).collect()),
            serde_json::Value::Array(t.entry_positions().iter().copied().map(cell).collect()),
            cell(t.exit_position()), cell(t.boss_position()),
        )).collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                lock_admission_relations(&mut tx).await?;
                if !prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                for r in rows {
                    sqlx::query("INSERT INTO game_boss_room_catalogue (boss_key,boss_creature_key,content_revision,room_template_key,template_revision,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,after_kill_seconds,lever_positions,entry_positions,exit_position,boss_position,reward_key,enabled) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$1,true) ON CONFLICT(boss_key) DO UPDATE SET boss_creature_key=EXCLUDED.boss_creature_key,content_revision=EXCLUDED.content_revision,room_template_key=EXCLUDED.room_template_key,template_revision=EXCLUDED.template_revision,minimum_players=EXCLUDED.minimum_players,maximum_players=EXCLUDED.maximum_players,minimum_level=EXCLUDED.minimum_level,cooldown_seconds=EXCLUDED.cooldown_seconds,time_limit_seconds=EXCLUDED.time_limit_seconds,after_kill_seconds=EXCLUDED.after_kill_seconds,lever_positions=EXCLUDED.lever_positions,entry_positions=EXCLUDED.entry_positions,exit_position=EXCLUDED.exit_position,boss_position=EXCLUDED.boss_position,enabled=true")
                        .bind(r.0).bind(r.1).bind(r.2).bind(r.3).bind(r.4).bind(r.5).bind(r.6).bind(r.7).bind(r.8).bind(r.9).bind(r.10).bind(r.11).bind(r.12).bind(r.13).bind(r.14).execute(&mut *tx).await?;
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(())
            })
        }).await
    }
}
