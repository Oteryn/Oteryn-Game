//! Observe lethal PvE damage and the server's own respawn deltas.
use super::*;
pub(super) async fn death(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    death_by(client, clients, None).await
}

pub(super) async fn death_by(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    expected_killer: Option<&str>,
) -> TestResult<()> {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        evidence("mechanic=death skipped=entry_room_has_no_native_lethal_encounter_content");
        return Ok(());
    }
    let previous_receipt = latest_receipt(clients).await?;
    client.session_mut().attack_target(None).await?;
    let before = client
        .actor_vitals()
        .ok_or("death journey vitals missing")?
        .health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let encounter = client.world_spatial().actor_position;
    let mut damaged = false;
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::ActorVitals(delta) = event {
                damaged |= delta.value.health < before;
            }
        }
        let position = client.world_spatial().actor_position;
        if damaged
            && (position.floor != encounter.floor
                || position
                    .x
                    .abs_diff(encounter.x)
                    .max(position.y.abs_diff(encounter.y))
                    > 8)
            && client
                .actor_vitals()
                .is_some_and(|vitals| vitals.health == vitals.max_health)
        {
            let receipt = latest_receipt(clients)
                .await?
                .ok_or("death respawn has no durable server receipt")?;
            if Some(&receipt) == previous_receipt.as_ref() || receipt.1 == receipt.2 {
                return Err(
                    "death journey reused an old receipt or did not advance Character revision"
                        .into(),
                );
            }
            if let Some(expected) = expected_killer {
                let observed = killing_creature(clients.url, &receipt.0).await?;
                if observed.as_deref() != Some(expected) {
                    return Err(format!(
                        "death receipt names {observed:?}; required killer {expected}"
                    )
                    .into());
                }
            }
            evidence(&format!(
                "mechanic=death state=damaged_then_respawned receipt={} character_revision={} health_before={before} health_after={} position={:?}",
                receipt.0,
                receipt.2,
                client.actor_vitals().ok_or("respawn vitals")?.health,
                client.world_spatial().actor_position
            ));
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "death journey did not observe lethal damage and respawn: damaged={damaged}"
            )
            .into());
        }
    }
}

async fn latest_receipt(clients: &SeamClients<'_>) -> TestResult<Option<(String, String, String)>> {
    within_database_deadline(tokio::time::Instant::now() + Duration::from_secs(20), async {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let receipt = sqlx::query_as("SELECT death_occurrence_id::text, original_character_revision::text, committed_character_revision::text FROM game_character_death_receipts WHERE character_id = encode($1, 'hex')::uuid AND death_world_id = encode($2, 'hex')::uuid AND death_channel_id = encode($3, 'hex')::uuid ORDER BY committed_at DESC, death_occurrence_id DESC LIMIT 1")
        .bind(clients.characters[1].as_slice()).bind(clients.world.as_slice()).bind(clients.channel.as_slice()).fetch_optional(&mut database).await?;
    database.close().await?;
    Ok(receipt)
    }).await
}

async fn killing_creature(url: &str, death: &str) -> TestResult<Option<String>> {
    within_database_deadline(tokio::time::Instant::now() + Duration::from_secs(20), async {
        let mut database = sqlx::PgConnection::connect(url).await?;
        let killer = sqlx::query_scalar(
            "SELECT creature_key FROM game_public_death_killers WHERE death_occurrence_id=$1::uuid AND ordinal=0 AND kind='creature'",
        ).bind(death).fetch_optional(&mut database).await?;
        database.close().await?;
        Ok(killer)
    }).await
}
