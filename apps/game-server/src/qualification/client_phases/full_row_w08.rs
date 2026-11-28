//! W08 full-row journey: Animus Mastery completion and species XP multiplier.
//!
//! Concrete player scenario derived from the W08 audit/matrix row and the named
//! `kill_reward`/Animus owners:
//! The dedicated character starts with a Minotaur Soul Core, a seeded Knight build,
//! and an empty mastery page. Command 232 must reject a zero limit without
//! changing the page. The intended gameplay leg earns Minotaur mastery in Soulpit,
//! then checks the exact species after relog and measures its XP bonus.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::animus_mastery as wire;
use std::collections::BTreeSet;

const MINOTAUR: &str = "oteryn:creature.minotaur";

const ACTIONS: &[&str] = &[
    "seeded-prerequisites",
    "initial-query",
    "invalid-query-refused",
    "soulpit-entry",
    "completion-observed",
    "fresh-admission-persistence",
    "species-xp-multiplier",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    failure: Option<String>,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
            failure: None,
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("W08_CHECK {action} PASS observed={observed:?}"));
        } else {
            verdict::record_assertion(action);
            self.failure = Some(format!("duplicate-or-unknown-action:{action}"));
        }
    }

    fn fail(&mut self, step: &'static str, observed: impl std::fmt::Display) {
        verdict::record_assertion(step);
        let message = format!("step={step};observed={observed}");
        evidence(&format!("W08_CHECK {step} FAIL {message}"));
        self.failure.get_or_insert(message);
    }

    fn finish(self) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("W08_GAP {action} assertion-not-completed"));
            }
        }
        let passed = self.failure.is_none() && ACTIONS.iter().all(|a| self.passed.contains(a));
        let suffix = self
            .failure
            .as_deref()
            .map(|failure| format!(" evidence={}", evidence_token(failure)))
            .unwrap_or_default();
        evidence(&format!(
            "e2e_all scenario=W08.full status={} seconds={:.3}{suffix}",
            if passed { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:=,".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(
        Duration::from_secs(90),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => row.fail("journey", error),
        Err(_) => row.fail("journey", "timeout-after-90s"),
    }
    row.finish();
}

async fn command(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_ANIMUS_MASTERY_V1,
            wire::COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
            payload,
        )
        .await?)
}

async fn page(client: &mut DevClientSession) -> TestResult<wire::AnimusMasteries> {
    let payload = wire::AnimusPage {
        offset: 0,
        limit: wire::MAX_ANIMUS_PAGE_KEYS,
    }
    .encode()
    .ok_or("W08 valid page did not encode")?;
    let reply = command(client, &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("valid command-232 reply={reply:?}").into());
    }
    wire::decode_page(&reply.payload)
        .ok_or_else(|| format!("accepted command-232 malformed-page={:?}", reply.payload).into())
}

const ENTRY: TilePos = TilePos {
    x: 32375,
    y: 31158,
    floor: -8,
};

async fn stage(clients: &SeamClients<'_>) -> TestResult {
    wait_character_released(clients, 0).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let result = sqlx::query(
        "UPDATE game_character_last_positions AS p
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions AS s
               WHERE s.character_id=p.character_id AND s.world_id=p.world_id AND s.session_state<>3
           )",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(ENTRY.x))
    .bind(i32::from(ENTRY.y))
    .bind(-i32::from(ENTRY.floor))
    .execute(&mut db)
    .await?;
    if result.rows_affected() != 1 {
        return Err("W08 staging requires one saved offline world position".into());
    }
    db.close().await?;
    evidence("W08_FIXTURE placement=(32375,31158,-8) source=authored-soulpit-slot-zero");
    Ok(())
}

fn core_count(client: &DevClientSession, core: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("W08 admitted without inventory")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == core)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

fn obelisk_result(
    payload: &[u8],
) -> TestResult<(
    oteryn_protocol_oteryn::world_object::UseDisposition,
    Option<oteryn_protocol_oteryn::refusal_detail::RefusalDetail>,
)> {
    use oteryn_protocol_oteryn::{refusal_detail, world_object};
    let (body, detail) = refusal_detail::split_refusal_detail(payload)
        .map_err(|error| format!("W08 obelisk refusal detail: {error:?}"))?;
    let disposition = world_object::decode_use_result(body)
        .map_err(|error| format!("W08 obelisk USE result: {error:?}"))?;
    Ok((disposition, detail))
}

async fn enter_soulpit(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
    core: std::num::NonZeroU32,
) -> TestResult {
    use oteryn_gameplay_domain::domain::soulpit;
    use oteryn_protocol_oteryn::world_object::{
        self, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    };

    let obelisk = soulpit::OBELISK_POSITION;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let mut ordinals = Vec::new();
    oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        if sector.floor == obelisk.floor {
            for tile in sector.tiles {
                if (tile.x, tile.y) == (obelisk.x, obelisk.y) {
                    for (ordinal, item) in
                        tile.items.iter().filter(|item| item.depth == 0).enumerate()
                    {
                        if manifest.palette[item.palette as usize].key == "oteryn:item.tibia.i47367"
                        {
                            ordinals.push(ordinal);
                        }
                    }
                }
            }
        }
        Ok(())
    })?;
    let [ordinal] = ordinals.as_slice() else {
        return Err(format!(
            "W08 expected one authored Soulpit obelisk, found {}",
            ordinals.len()
        )
        .into());
    };
    let world = full_e2ef7::qualification_world(clients)?;
    let mut placement = world.base().digest().to_vec();
    let key = (u64::from(obelisk.x) << 32)
        | (u64::from(obelisk.y) << 16)
        | (u64::from(u8::try_from(-i16::from(obelisk.floor))?) << 8)
        | u64::from(u8::try_from(*ordinal)?);
    placement.extend_from_slice(&key.to_be_bytes());

    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(core),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("W08 obelisk USE encoding: {error:?}"))?;
    let source = client.join_snapshot().game_session_id;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let result = obelisk_result(&reply.payload);
    if reply.status != CommandStatus::Accepted
        || !matches!(result, Ok((UseDisposition::Committed, None)))
    {
        return Err(format!(
            "W08 obelisk USE status={:?} result={result:?} payload={:02x?}",
            reply.status, reply.payload
        )
        .into());
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client.join_snapshot().game_session_id == source {
        if tokio::time::Instant::now() >= deadline {
            return Err("W08 obelisk USE did not admit an Instance".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let destination = client.join_snapshot().game_session_id;
    let position = tile(client.world_spatial().actor_position)?;
    let scope = client
        .session_mut()
        .activity_scope()
        .ok_or("W08 Instance scope absent")?;
    if scope.source_session != *source.as_bytes()
        || scope.session != *destination.as_bytes()
        || !matches!(
            scope.scope,
            oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
        )
        || position != soulpit::entry_destination(0)?
    {
        return Err(
            format!("W08 incorrect Soulpit handoff scope={scope:?} position={position:?}").into(),
        );
    }
    if core_count(client, core)? != 0 {
        return Err("W08 Soulpit entry did not debit the Minotaur Soul Core".into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W08_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    character_prep::declare_seeded(
        "W08",
        "knight-level=100,minotaur-soul-core=1",
        "ordinary leveling and core acquisition are outside the mastery scenario",
    );
    let mut setup = connect_character(clients, 0xc8, 0).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(setup_session.as_bytes()),
        "--profile",
        "w08-soulpit".into(),
    )
    .await?;
    if setup.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W08 fixture setup logout refused".into());
    }
    drop(setup);
    stage(clients).await?;
    let mut client = connect_character(clients, 0xc9, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("W08 fixture setup reused its admission".into());
    }
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "knight",
        100,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("W08 Knight vitals unavailable")?;
    let vitals = client.actor_vitals().ok_or("W08 admitted without vitals")?;
    if (vitals.max_health, vitals.max_mana) != (max_health, max_mana) {
        return Err(format!("W08 seeded Knight vitals differ: {vitals:?}").into());
    }
    let core = provisioning::item_reference(&provisioning::item_index()?, 47402)?;
    let count = core_count(&client, core)?;
    if count != 1 || tile(client.world_spatial().actor_position)? != ENTRY {
        return Err(format!(
            "W08 expected one Minotaur Soul Core and exact entry slot, found count={count}"
        )
        .into());
    }
    row.pass("seeded-prerequisites", (max_health, max_mana, count, ENTRY));
    let before = page(&mut client).await?;
    if before.total != 0 || !before.keys.is_empty() {
        row.fail(
            "initial-query",
            format!("expected empty mastery page, got {before:?}"),
        );
        return Ok(());
    }
    row.pass("initial-query", (&before.total, &before.keys));

    let refused = command(&mut client, &[0, 0, 0, 0]).await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        row.fail("invalid-query-refused", format!("reply={refused:?}"));
        return Ok(());
    }
    let after_refusal = page(&mut client).await?;
    if after_refusal != before {
        row.fail(
            "invalid-query-refused",
            format!("mastery-set-mutated-by-refusal before={before:?},after={after_refusal:?}"),
        );
        return Ok(());
    }
    row.pass("invalid-query-refused", (refused.status, &after_refusal));

    enter_soulpit(clients, &mut client, core).await?;
    row.pass(
        "soulpit-entry",
        "slot-zero Instance admission and one core consumed",
    );

    // The acquisition driver must earn mastery before the completion query.
    let completed = page(&mut client).await?;
    if completed.total != 1 || completed.keys.as_slice() != [MINOTAUR] {
        row.fail(
            "completion-observed",
            format!("expected player-earned Minotaur mastery, got {completed:?}"),
        );
        return Ok(());
    }
    row.pass("completion-observed", (&completed.total, &completed.keys));

    let mastery_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        row.fail("fresh-admission-persistence", "logout-reply-not-accepted");
        return Ok(());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0xca, 0).await?;
    if fresh.join_snapshot().game_session_id == mastery_session {
        row.fail(
            "fresh-admission-persistence",
            "mastery admission was reused",
        );
        return Ok(());
    }
    let durable = page(&mut fresh).await?;
    if durable != completed {
        row.fail(
            "fresh-admission-persistence",
            format!("completed={completed:?},fresh={durable:?}"),
        );
        return Ok(());
    }
    row.pass(
        "fresh-admission-persistence",
        (&durable.total, &durable.keys),
    );

    // Reaching this branch proves durable completion. The native client currently
    // exposes no species identity on visible actors, so selecting and proving the
    // matching kill without guessing would be false evidence.
    row.fail(
        "species-xp-multiplier",
        format!(
            "durable-species={:?};missing-native-visible-actor-species-key-needed-to-select-matching-combat-target",
            durable.keys.first()
        ),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obelisk_result_preserves_negotiated_refusal() -> TestResult {
        use oteryn_protocol_oteryn::refusal_detail::{
            CAPABILITY_REFUSAL_DETAIL_V1, RefusalDetail, append_refusal_detail,
        };
        use oteryn_protocol_oteryn::world_object::{UseDisposition, encode_use_result};
        let mut rejected = encode_use_result(UseDisposition::Rejected)
            .map_err(|error| format!("encode rejection: {error:?}"))?;
        append_refusal_detail(
            &mut rejected,
            &[CAPABILITY_REFUSAL_DETAIL_V1],
            Some(RefusalDetail::NotPossible),
        );
        assert_eq!(
            obelisk_result(&rejected)?,
            (UseDisposition::Rejected, Some(RefusalDetail::NotPossible)),
        );
        let committed = encode_use_result(UseDisposition::Committed)
            .map_err(|error| format!("encode commit: {error:?}"))?;
        assert_eq!(
            obelisk_result(&committed)?,
            (UseDisposition::Committed, None)
        );
        Ok(())
    }

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(evidence_token("reply=[]; need x"), "reply=----need-x");
    }
}
