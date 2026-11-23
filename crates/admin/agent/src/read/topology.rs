use super::*;
use sqlx::{PgConnection, Row};

pub(super) async fn world_page(
    conn: Option<&mut PgConnection>,
    worlds: &[String],
    params: Page,
) -> Result<WorldPage, E> {
    let rows: Vec<_> = worlds
        .iter()
        .filter(|id| params.cursor.as_ref().is_none_or(|c| *id > c))
        .take(usize::from(params.limit) + 1)
        .map(|id| World {
            world_id: id.clone(),
            name: String::new(),
            state: "unavailable".into(),
            channels: vec![],
            ruleset_revision: String::new(),
            content_revision: String::new(),
            map_revision: String::new(),
            policy_revision: String::new(),
        })
        .collect();
    let (mut worlds, next_cursor) = page(rows, params.limit, |row| row.world_id.clone());
    if let Some(conn) = conn {
        let ids: Vec<_> = worlds
            .iter()
            .filter_map(|row| Uuid::parse_str(&row.world_id).ok())
            .collect();
        let channels: std::collections::BTreeMap<_, _> = sqlx::query_as::<_, (Uuid, Vec<String>)>(
            "SELECT id, ARRAY(SELECT channel_id::text FROM admin_read.runtime_scope_assignments
                              WHERE world_id=id AND scope_kind=1 AND state=1
                              ORDER BY scope_key LIMIT 101)
             FROM unnest($1::uuid[]) AS id",
        )
        .bind(ids)
        .fetch_all(conn)
        .await
        .map_err(db_error)?
        .into_iter()
        .collect();
        for world in &mut worlds {
            if let Some(ids) = Uuid::parse_str(&world.world_id)
                .ok()
                .and_then(|id| channels.get(&id))
            {
                if ids.len() > 100 {
                    return Err(E::AdmLimitExceeded);
                }
                world.channels.clone_from(ids);
            }
        }
    }
    Ok(WorldPage {
        worlds,
        next_cursor,
    })
}

pub(super) async fn node_page(
    conn: &mut PgConnection,
    nodes: &[Uuid],
    worlds: &[Uuid],
    params: Page,
) -> Result<NodePage, E> {
    let rows = sqlx::query(
        "SELECT n.node_id::text, n.registered_at,
                ARRAY(SELECT DISTINCT s.world_id::text FROM admin_read.runtime_scope_assignments s
                      WHERE s.state=1 AND s.holder_node_id=n.node_id
                        AND s.holder_registration_revision=n.registration_revision
                        AND s.world_id=ANY($2) ORDER BY 1 LIMIT 101) AS worlds,
                EXISTS(SELECT 1 FROM admin_read.runtime_scope_assignments s
                       JOIN admin_read.runtime_guards g USING(scope_key)
                       WHERE s.holder_node_id=n.node_id AND s.state=1
                         AND s.holder_registration_revision=n.registration_revision
                         AND s.world_id=ANY($2)
                         AND g.ownership_generation=s.ownership_generation AND g.ready) AS ready,
                EXTRACT(EPOCH FROM statement_timestamp())::bigint AS now
         FROM admin_read.node_registrations n WHERE n.state=1 AND n.node_id=ANY($1)
           AND ($3::uuid IS NULL OR n.node_id>$3)
           AND EXISTS(SELECT 1 FROM admin_read.runtime_scope_assignments s
                WHERE s.state=1 AND s.holder_node_id=n.node_id
                  AND s.holder_registration_revision=n.registration_revision AND s.world_id=ANY($2))
         ORDER BY n.node_id LIMIT $4",
    )
    .bind(nodes)
    .bind(worlds)
    .bind(optional_uuid(params.cursor.as_deref())?)
    .bind(i64::from(params.limit) + 1)
    .fetch_all(conn)
    .await
    .map_err(db_error)?;
    let mut nodes = Vec::new();
    for row in rows {
        let worlds: Vec<String> = row.try_get("worlds").map_err(db_error)?;
        if worlds.len() > 100 {
            return Err(E::AdmLimitExceeded);
        }
        let registered_at: i64 = row.try_get("registered_at").map_err(db_error)?;
        let now: i64 = row.try_get("now").map_err(db_error)?;
        nodes.push(Node {
            node_id: row.try_get("node_id").map_err(db_error)?,
            host: String::new(),
            build: String::new(),
            uptime_seconds: now
                .saturating_sub(registered_at)
                .clamp(0, i64::from(u32::MAX)) as u32,
            worlds,
            ready: row.try_get("ready").map_err(db_error)?,
            cpu_percent: 0.0,
            memory_bytes: String::new(),
        });
    }
    let (nodes, next_cursor) = page(nodes, params.limit, |row| row.node_id.clone());
    Ok(NodePage { nodes, next_cursor })
}

pub(super) async fn overview(
    pool: Option<&PgPool>,
    worlds: &[String],
    world_ids: &[Uuid],
    nodes: &[Uuid],
    deadline: Instant,
) -> Result<Overview, E> {
    let params = || Page {
        limit: 100,
        cursor: None,
    };
    let mut overview = Overview {
        agents: vec![],
        worlds: world_page(None, worlds, params()).await?.worlds,
        nodes: vec![],
        players_online: 0,
        db_available: false,
        backup_age_seconds: None,
        alerts: vec![],
    };
    let Some(pool) = pool else {
        overview
            .alerts
            .push("Game read connection unavailable".into());
        return Ok(overview);
    };
    let mut tx = match begin(pool, deadline).await {
        Ok(tx) => tx,
        Err(E::AdmTargetUnavailable) => {
            overview
                .alerts
                .push("Game read connection unavailable".into());
            return Ok(overview);
        }
        Err(e) => return Err(e),
    };
    let worlds = world_page(Some(&mut tx), worlds, params()).await?;
    let nodes = node_page(&mut tx, nodes, world_ids, params()).await?;
    if worlds.next_cursor.is_some() || nodes.next_cursor.is_some() {
        overview
            .alerts
            .push("Use world.list and node.list for more results".into());
    }
    overview.worlds = worlds.worlds;
    overview.nodes = nodes.nodes;
    let count = players::online_count(&mut tx, world_ids).await?;
    overview.players_online = u32::try_from(count).map_err(|_| E::AdmLimitExceeded)?;
    overview.db_available = true;
    tx.commit().await.map_err(db_error)?;
    Ok(overview)
}
