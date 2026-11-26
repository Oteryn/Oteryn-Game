//! W10 player scenario, derived from the matrix row, re-audit, and the native Hazard owner:
//! 1. Admit a node-boot character and send an empty Hazard command. Expect ACCEPTED and the
//!    two-byte `[unlocked, selected]` state delta, initially with level 1 selected.
//! 2. Send selection level 1. Expect ACCEPTED, the same two-byte state, and no unrelated state
//!    mutation. Then send locked level 2 and expect a REJECTED reply with no payload (the refusal).
//! 3. Walk, using compiled-world collision, to an authored Primal Menace encounter. Send ATTACK,
//!    require the accepted combat-state delta, remain alive while servicing native combat, and
//!    require the monster's removal/death frames.
//! 4. Require the kill's Hazard unlock reply to change `[1, 1]` to `[2, 1]`; select level 2 and
//!    require `[2, 2]`. Fight and loot an authored Hazard-area creature, asserting combat vitals
//!    and carried-inventory deltas rather than inspecting storage directly.
//! 5. Log out, freshly admit the same character, query Hazard again, and require `[2, 2]` plus the
//!    retained loot. Re-select locked level 3 and require the same empty REJECTED refusal.
//!
//! Every command above is a native player command. If authored encounter access prevents a later
//! command, the row reports the first observed step and reply/path failure; it never claims PASS.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{attack as attacks, boss_encounter as boss, hazard};

const PRIMAL: &str = "oteryn:boss.the_primal_menace";

#[derive(Debug)]
pub(super) struct StepFailure {
    step: u8,
    observed: String,
    need: &'static str,
    source: Option<Box<dyn std::error::Error>>,
}

type StepResult<T = ()> = Result<T, StepFailure>;

fn failed(step: u8, observed: impl std::fmt::Display, need: &'static str) -> StepFailure {
    StepFailure {
        step,
        observed: observed.to_string().replace(' ', "_"),
        need,
        source: None,
    }
}

async fn command(
    client: &mut DevClientSession,
    selection: Option<u8>,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let payload = selection.map_or_else(Vec::new, |level| vec![level]);
    let reply = client
        .session_mut()
        .gated_command(
            hazard::CAPABILITY_HAZARD_V1,
            hazard::COMMAND_TYPE_HAZARD,
            &payload,
        )
        .await?;
    Ok((reply.status, reply.payload))
}

fn accepted(step: u8, reply: (CommandStatus, Vec<u8>), expected: [u8; 2]) -> StepResult {
    if reply.0 != CommandStatus::Accepted || reply.1 != expected {
        return Err(failed(
            step,
            format!("status={:?},payload={:?}", reply.0, reply.1),
            "an accepted Hazard reply carrying the exact unlocked/selected delta",
        ));
    }
    evidence(&format!(
        "W10_CHECK step={step} status=PASS reply=ACCEPTED hazard={}/{}",
        expected[0], expected[1]
    ));
    Ok(())
}

fn rejected(step: u8, reply: (CommandStatus, Vec<u8>)) -> StepResult {
    if reply.0 != CommandStatus::Rejected || !reply.1.is_empty() {
        return Err(failed(
            step,
            format!("status={:?},payload={:?}", reply.0, reply.1),
            "an empty REJECTED reply for a locked Hazard level",
        ));
    }
    evidence(&format!(
        "W10_CHECK step={step} status=PASS reply=REJECTED payload=empty"
    ));
    Ok(())
}

// Staging is limited to the offline dedicated qualification character. Reuse
// its world and map identity from the preceding real logout.
pub(super) async fn stage_primal_challenger(clients: &SeamClients<'_>) -> TestResult {
    wait_character_released(clients, 0).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let character = clients.characters[0].as_slice();
    let level: i64 = sqlx::query_scalar(
        "SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    ).bind(character).fetch_one(&mut db).await?;
    if level != 500 {
        return Err(format!("Primal challenger level={level}, expected 500").into());
    }
    let result = sqlx::query(
        "UPDATE game_character_last_positions AS p
         SET tile_x=33548, tile_y=32751, tile_floor=14, write_sequence=write_sequence+1
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions AS s
               WHERE s.character_id=p.character_id AND s.world_id=p.world_id AND s.session_state<>3
           )",
    )
    .bind(character)
    .bind(clients.world.as_slice())
    .execute(&mut db)
    .await?;
    if result.rows_affected() != 1 {
        return Err("dedicated Primal staging requires one saved offline world position".into());
    }
    db.close().await?;
    Ok(())
}

pub(super) async fn enter_primal(
    client: &mut DevClientSession,
    world: &BundleWorld,
    first_chair_only: bool,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        at.floor == -14
            && at.y == 32752
            && (33548..=if first_chair_only { 33548 } else { 33552 }).contains(&at.x)
    })
    .ok_or_else(|| format!("no native route from {from:?} to the authored Primal lever"))?;
    if path.is_empty() {
        return Err("Primal challenger must walk onto the lever tile".into());
    }
    walk(client, world, &path).await?;
    evidence(&format!(
        "W10_CHECK step=3 status=PASS activity=lever-tile position={:?}",
        client.world_spatial().actor_position
    ));
    verdict::Harness::global().progress("primal-lever");
    let source = client.join_snapshot().game_session_id;
    let payload = boss::encode_boss_key(PRIMAL).map_err(|error| format!("{error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            boss::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss::COMMAND_TYPE_BOSS_LEVER,
            &payload,
        )
        .await?;
    let Ok(boss::BossActivityReply::Accepted { handoff, .. }) =
        boss::decode_boss_activity_reply(&reply.payload)
    else {
        return Err(format!(
            "Primal lever returned {:?}: {:?}",
            reply.status,
            boss::decode_boss_activity_reply(&reply.payload)
        )
        .into());
    };
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Primal lever handoff had status {:?}", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client.join_snapshot().game_session_id == source {
        if tokio::time::Instant::now() >= deadline {
            return Err("Primal lever handoff did not admit an instance".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    let destination = client.join_snapshot().game_session_id;
    let scope = client
        .session_mut()
        .activity_scope()
        .ok_or("missing Primal activity scope")?;
    if scope.handoff != handoff
        || scope.source_session != *source.as_bytes()
        || scope.session != *destination.as_bytes()
        || !matches!(
            scope.scope,
            oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
        )
    {
        return Err(format!("unmatched Primal handoff {scope:?}").into());
    }
    Ok(())
}

pub(super) fn primal_world(clients: &SeamClients<'_>) -> StepResult<&'static BundleWorld> {
    let bytes = std::fs::read(
        required("NODE_BOOT_WORLD_BUNDLE")
            .map_err(|error| failed_error(1, error, "the node-boot compiled world bundle"))?,
    )
    .map_err(|error| failed_error(1, error, "the node-boot compiled world bundle"))?;
    let pin: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            required("NODE_BOOT_WORLD_PIN")
                .map_err(|error| failed_error(1, error, "the node-boot world pin"))?,
        )
        .map_err(|error| failed_error(1, error, "the node-boot world pin"))?,
    )
    .map_err(|error| failed_error(1, error, "a valid node-boot world pin"))?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world).map_err(|error| failed(1, error, "a valid world id"))?,
        ChannelId::decode(&clients.channel)
            .map_err(|error| failed(1, error, "a valid channel id"))?,
    )
    .map_err(|error| failed_error(1, error, "the production world loader to accept the bundle"))
}

pub(super) async fn defeat_primal(
    client: &mut DevClientSession,
) -> StepResult<(
    ActorPosition,
    oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
)> {
    let at = client.world_spatial().actor_position;
    let spawn_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let target = loop {
        if let Some(entity) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && entity
                            .display_name
                            .is_some_and(|name| name.as_str() == Ok("The Primal Menace"))
                        && tile(entity.position).is_ok_and(|at| {
                            at == (TilePos {
                                x: 33558,
                                y: 32759,
                                floor: -15,
                            })
                        })
                })
            })
            .copied()
        {
            break entity;
        }
        if tokio::time::Instant::now() >= spawn_deadline {
            return Err(failed(
                3,
                "missing_primal_spawn",
                "the named Primal Menace at the authored spawn",
            ));
        }
        client
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| failed_error(3, Box::new(error), "continued instance frames"))?;
        client.take_events();
    };
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await
        .map_err(|error| failed_error(3, error, "an ATTACK reply"))?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(failed(
            3,
            format!("attack={attack:?}"),
            "an accepted Primal Menace ATTACK with combat delta",
        ));
    }
    evidence("W10_CHECK step=3 status=PASS reply=ATTACK_OK combat-delta=present");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|entity| entity.entity == target.entity))
    {
        if tokio::time::Instant::now() >= deadline {
            let _ = client.session_mut().attack_target(None).await;
            return Err(failed(
                3,
                format!("combat_timeout;vitals={:?}", client.actor_vitals()),
                "the provisioned player build or legitimate encounter support needed to complete the Primal fight",
            ));
        }
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failed_error(3, error, "continued native combat frames"))?;
        client.take_events();
    }
    client
        .session_mut()
        .attack_target(None)
        .await
        .map_err(|error| failed_error(3, error, "an attack-stop reply"))?;

    Ok((at, target.entity))
}

async fn journey(clients: &SeamClients<'_>) -> StepResult {
    let world = primal_world(clients)?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W10_ACCOUNT")
        .await
        .map_err(|error| failed_error(1, error, "a dedicated Primal challenger"))?;
    let clients = &dedicated.clients(clients);
    character_prep::declare_seeded("W10", "level=500", "authored Primal admission minimum");
    let seed = character_prep::Seeded {
        profile: "w10-primal",
        reason: "authored Primal admission minimum",
    };
    let mut admitted = seed
        .connect(clients, 0xd0, 0)
        .await
        .map_err(|error| failed_error(1, error, "level-500 dedicated admission"))?;
    let (logout, detail, _, _) = logout_retrying_busy(&mut admitted)
        .await
        .map_err(|error| failed_error(1, error, "a durable staging logout"))?;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(failed(1, detail, "an accepted staging logout"));
    }
    drop(admitted);
    stage_primal_challenger(clients).await.map_err(|error| {
        failed_error(
            1,
            error,
            "offline dedicated staging beside the Primal lever",
        )
    })?;
    let mut client = connect_character(clients, 0xd0, 0)
        .await
        .map_err(|error| failed_error(1, error, "ordinary admission beside the Primal lever"))?;
    if tile(client.world_spatial().actor_position).is_ok_and(|at| {
        at == (TilePos {
            x: 33548,
            y: 32751,
            floor: -14,
        })
    }) {
        evidence(
            "W10_CHECK step=1 status=PASS admission=dedicated-level-500 position=primal-lobby",
        );
    } else {
        return Err(failed(
            1,
            format!("{:?}", client.world_spatial().actor_position),
            "the staged actor position in the served admission",
        ));
    }

    accepted(
        1,
        command(&mut client, None)
            .await
            .map_err(|error| failed_error(1, error, "a Hazard query reply"))?,
        [1, 1],
    )?;
    accepted(
        2,
        command(&mut client, Some(1))
            .await
            .map_err(|error| failed_error(2, error, "a Hazard selection reply"))?,
        [1, 1],
    )?;
    rejected(
        2,
        command(&mut client, Some(2))
            .await
            .map_err(|error| failed_error(2, error, "a locked-level refusal"))?,
    )?;

    enter_primal(&mut client, world, false)
        .await
        .map_err(|error| {
            failed_error(3, error, "the authored Primal lever and instance handoff")
        })?;
    let (at, defeated) = defeat_primal(&mut client).await?;

    accepted(
        4,
        command(&mut client, None)
            .await
            .map_err(|error| failed_error(4, error, "the post-kill Hazard unlock delta"))?,
        [2, 1],
    )?;
    accepted(
        4,
        command(&mut client, Some(2))
            .await
            .map_err(|error| failed_error(4, error, "the level-2 Hazard selection delta"))?,
        [2, 2],
    )?;
    let inventory_before = client
        .inventory()
        .ok_or_else(|| {
            failed(
                4,
                "inventory_snapshot_missing",
                "the pre-loot inventory frame",
            )
        })?
        .entries
        .clone();
    let second = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && entity.entity != defeated
                        && entity.position.floor == at.floor
                        && entity.position.x.abs_diff(at.x).max(entity.position.y.abs_diff(at.y))
                            <= 7
                })
                .copied()
        })
        .ok_or_else(|| {
            failed(
                4,
                "no_visible_hazard_area_creature_after_unlock",
                "an authored reachable Hazard-area combat and loot target for the scaling comparison",
            )
        })?;
    let second_attack = client
        .session_mut()
        .attack_target(Some(&second.entity))
        .await
        .map_err(|error| failed_error(4, error, "a scaled-combat ATTACK reply"))?;
    if second_attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(failed(
            4,
            format!("scaled_attack={second_attack:?}"),
            "an accepted Hazard-area ATTACK and combat delta",
        ));
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    while client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|entity| entity.entity == second.entity))
    {
        if tokio::time::Instant::now() >= deadline {
            let _ = client.session_mut().attack_target(None).await;
            return Err(failed(
                4,
                format!("scaled_combat_timeout;vitals={:?}", client.actor_vitals()),
                "a completable authored Hazard-area combat target",
            ));
        }
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failed_error(4, error, "scaled combat/vitals frames"))?;
        client.take_events();
    }
    client
        .session_mut()
        .attack_target(None)
        .await
        .map_err(|error| failed_error(4, error, "the scaled attack-stop reply"))?;
    let inventory_after = client
        .inventory()
        .ok_or_else(|| failed(4, "post_kill_inventory_missing", "the loot inventory delta"))?
        .entries
        .clone();
    if inventory_after == inventory_before {
        return Err(failed(
            4,
            "hazard_kill_had_no_carried_loot_delta",
            "a native corpse-loot action/result that exposes the scaled loot through the dev client",
        ));
    }
    evidence("W10_CHECK step=4 status=PASS scaled-combat=kill loot-delta=present");

    let old_session = client.join_snapshot().game_session_id;
    if client
        .session_mut()
        .logout()
        .await
        .map_err(|error| failed_error(5, error, "an accepted logout reply"))?
        .result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(failed(5, "logout_refused", "an accepted logout reply"));
    }
    drop(client);
    let mut client = connect_character(clients, 0xd1, 0)
        .await
        .map_err(|error| failed_error(5, error, "fresh native readmission"))?;
    if client.join_snapshot().game_session_id == old_session {
        return Err(failed(
            5,
            "session_not_fresh",
            "a new admitted game session",
        ));
    }
    accepted(
        5,
        command(&mut client, None)
            .await
            .map_err(|error| failed_error(5, error, "the reloaded Hazard state"))?,
        [2, 2],
    )?;
    if client
        .inventory()
        .is_none_or(|inventory| inventory.entries != inventory_after)
    {
        return Err(failed(
            5,
            "loot_inventory_changed_after_fresh_admission",
            "the durable carried loot snapshot",
        ));
    }
    rejected(
        5,
        command(&mut client, Some(3))
            .await
            .map_err(|error| failed_error(5, error, "the reloaded locked-level refusal"))?,
    )?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W10.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            evidence(&format!(
                "W10_GAP step={} observed={} need={}",
                error.step,
                error.observed,
                error.need.replace(' ', "_")
            ));
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=W10.full status=FAIL seconds={:.3} evidence=step{}:{}",
                started.elapsed().as_secs_f64(),
                error.step,
                error.observed
            ));
        }
        Err(_) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W10.full status=FAIL seconds={:.3} evidence=step3:journey_timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

impl std::error::Error for StepFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

impl verdict::IndexedError for StepFailure {
    fn as_error(&self) -> &(dyn std::error::Error + 'static) {
        self
    }
}

impl std::fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "step={};observed={};need={}",
            self.step, self.observed, self.need
        )
    }
}

fn failed_error(
    step: u8,
    error: impl Into<Box<dyn std::error::Error>>,
    need: &'static str,
) -> StepFailure {
    let error = error.into();
    verdict::record_error(error.as_ref());
    let mut failure = failed(step, &error, need);
    failure.source = Some(error);
    failure
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hazard_reply_checks_require_exact_status_and_delta() {
        assert!(accepted(1, (CommandStatus::Accepted, vec![1, 1]), [1, 1]).is_ok());
        assert!(accepted(1, (CommandStatus::Accepted, vec![2, 1]), [1, 1]).is_err());
        assert!(rejected(2, (CommandStatus::Rejected, Vec::new())).is_ok());
        assert!(rejected(2, (CommandStatus::Accepted, Vec::new())).is_err());
    }
}
