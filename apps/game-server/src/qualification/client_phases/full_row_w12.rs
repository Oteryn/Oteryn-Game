//! W12 full-row player scenario, derived from the matrix row, the Crystal reaudit,
//! and the production Cyclopedia owner (`gameplay_transport/cyclopedia.rs`):
//! 1. Admit the node-boot Character and send a Cyclopedia query; expect an accepted,
//!    decodable state frame identifying revision, discovery, POIs, donation pool,
//!    reset epoch and improved-respawn state.
//! 2. Send FIND in the automatically active exploration. Verify either the exact
//!    no-op or the current visible POI transition, with a bound census on completion.
//! 3. Send legacy START: active exploration returns code 0 unchanged; completed
//!    exploration returns code 3 with the same confirmed state, without Premium.
//! 4. When START succeeds, discover the POIs only by walking the Character through
//!    the compiled world and sending FIND at each server-revealed coordinate; expect
//!    found-count deltas culminating in discovered=true, found=7 and 100 percent.
//! 5. Send a one-gold DONATE through the ordinary physical trade path; expect either
//!    code 0 with an exact +1 pool delta or code 6 with no delta when player-earned
//!    funds are insufficient.  Then really attempt the 10,000,000-gold goal; expect
//!    code 0, pool/epoch/improved deltas and the improved respawn selection, or record
//!    code 6 as the observed refusal (the test never mints money).
//! 6. Send a malformed/unknown-area intent; expect the command-status refusal with no
//!    result payload, then query again to prove that the connection and state survived.
//! 7. Logout normally, make a fresh admission, query again, and require the discovery,
//!    donation pool, epoch and improved-respawn state from the last accepted action to
//!    be durable.  Movement-speed parity is not inferred from travel time: W12 needs a
//!    server result/delta exposing the donated-area respawn speed before it can PASS.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::cyclopedia as wire;

const SOURCE_GAP: &str = "source:crates/gameplay/domain/src/domain/cyclopedia/basic.rs:6";

#[derive(Debug)]
struct Failure {
    step: u8,
    observed: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl Failure {
    fn new(step: u8, observed: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            step,
            observed: observed.into(),
            source: None,
        })
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "step-{}:{}", self.step, self.observed)
    }
}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

fn intent(operation: u8, area: u32, amount: u32) -> [u8; wire::MAX_REQUEST_BYTES] {
    let mut bytes = [0; wire::MAX_REQUEST_BYTES];
    bytes[0] = operation;
    bytes[1..5].copy_from_slice(&area.to_le_bytes());
    bytes[5..9].copy_from_slice(&amount.to_le_bytes());
    bytes
}

async fn exchange(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, payload)
        .await?)
}

async fn query(client: &mut DevClientSession, step: u8) -> Result<wire::MapResult, Box<Failure>> {
    let reply = exchange(client, wire::COMMAND_TYPE_CYCLOPEDIA_QUERY, &[])
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            let mut failure = Failure::new(step, format!("query-error={error}"));
            failure.source = Some(error);
            failure
        })?;
    if reply.status != CommandStatus::Accepted {
        return Err(Failure::new(step, format!("query-reply={reply:?}")));
    }
    wire::decode_result(&reply.payload)
        .map_err(|error| Failure::new(step, format!("result-decode={error:?};reply={reply:?}")))
}

async fn action(
    client: &mut DevClientSession,
    step: u8,
    operation: u8,
    amount: u32,
) -> Result<wire::MapResult, Box<Failure>> {
    let reply = exchange(
        client,
        wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
        &intent(operation, 1, amount),
    )
    .await
    .map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(step, format!("command-error={error}"));
        failure.source = Some(error);
        failure
    })?;
    if reply.status != CommandStatus::Accepted {
        return Err(Failure::new(step, format!("command-reply={reply:?}")));
    }
    wire::decode_result(&reply.payload)
        .map_err(|error| Failure::new(step, format!("result-decode={error:?};reply={reply:?}")))
}

fn stable(before: &wire::MapResult, after: &wire::MapResult) -> bool {
    before.discovered == after.discovered
        && before.found == after.found
        && before.area_percent == after.area_percent
        && before.pool == after.pool
        && before.epoch == after.epoch
        && before.improved == after.improved
}

async fn logout(client: &mut DevClientSession, step: u8) -> Result<(), Box<Failure>> {
    let reply = client.session_mut().logout().await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(step, format!("logout-error={error}"));
        failure.source = Some(error.into());
        failure
    })?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(Failure::new(step, format!("logout-reply={reply:?}")));
    }
    Ok(())
}

// Query activates discovery. FIND can be a no-op, find the current visible POI,
// or complete the current run. Only completion needs the canonical area census.
pub(super) async fn validate_initial_find<F, Fut>(
    before: &wire::MapResult,
    after: &wire::MapResult,
    position: ActorPosition,
    witness: F,
) -> TestResult
where
    F: FnOnce(ActorPosition, wire::MapResult) -> Fut,
    Fut: Future<Output = TestResult<u8>>,
{
    if before.code != 0 {
        return Err(format!("automatic-query-not-success={before:?}").into());
    }
    let point = (
        u16::try_from(position.x)?,
        u16::try_from(position.y)?,
        i8::try_from(position.floor)?,
    );
    let hits = before.visible.iter().filter(|p| **p == point).count();
    if hits > 1 || (before.discovered && (before.found != 7 || !before.visible.is_empty())) {
        return Err(format!("automatic-query-inconsistent={before:?}").into());
    }
    let mut expected = before.clone();
    if !before.discovered {
        let found = before.found.checked_add(u8::try_from(hits)?).ok_or("found overflow")?;
        let completed = found >= 7;
        if hits != 0 || completed {
            expected.revision = before.revision.checked_add(1).ok_or("revision overflow")?;
            expected.found = found.min(7);
            expected.visible.retain(|p| *p != point);
        }
        if completed {
            expected.discovered = true;
            expected.visible.clear();
            // The witness is selected from the predicted transition, never from an
            // unverified claim in the reply being checked.
            expected.area_percent = witness(position, after.clone()).await?;
        }
    }
    if after != &expected {
        return Err(format!("automatic-find-mismatch expected={expected:?};observed={after:?}").into());
    }
    Ok(())
}

pub(super) fn validate_legacy_start(before: &wire::MapResult, after: &wire::MapResult) -> TestResult {
    let mut expected = before.clone();
    expected.code = if before.discovered { 3 } else { 0 };
    if after != &expected {
        return Err(format!("idempotent-start-mismatch expected={expected:?};observed={after:?}").into());
    }
    Ok(())
}

pub(super) async fn completed_area_percentage(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    position: ActorPosition,
    observed: &wire::MapResult,
) -> TestResult<u8> {
    use crate::domain::cyclopedia::{BasicArea, MapProgress};
    let at = tile(position)?;
    let table = &world.base().subareas;
    let (area, subarea) = table.resolve(at.x, at.y, at.floor).ok_or("completion membership absent")?;
    let ids: Vec<_> = table.areas.iter().filter(|a| a.parent == Some(area)).map(|a| a.id).collect();
    if ids.is_empty() || ids.len() > 512 {
        return Err("completion census outside catalogue bound".into());
    }
    let character = clients.characters.get(1).ok_or("W12 character absent")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    // One SELECT observes a single MVCC snapshot. It cannot manufacture progress;
    // the returned Character revision and both content digests must match the reply.
    let (revision, bundle, catalogue, progress): (String, Vec<u8>, Vec<u8>, String) =
        sqlx::query_as("SELECT r.character_revision::text,c.bundle_digest,c.catalogue_digest,(c.progress->'subareas')::text FROM game_character_roots r JOIN game_character_cyclopedia c USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).fetch_one(&mut database).await?;
    if revision.parse::<u64>()? != observed.revision
        || bundle != world.base().digest()
        || catalogue != BasicArea::content_digest()
        || progress.len() > 262_144
    {
        return Err("completion census revision/content mismatch".into());
    }
    let progress: BTreeMap<u32, MapProgress> = serde_json::from_str(&progress)?;
    percentage_from_completed_census(&ids, &progress, subarea)
}

fn percentage_from_completed_census(
    ids: &[u32],
    progress: &BTreeMap<u32, crate::domain::cyclopedia::MapProgress>,
    subarea: u32,
) -> TestResult<u8> {
    if ids.is_empty() || ids.len() > 512 || progress.len() > 512
        || !ids.contains(&subarea) || !ids.iter().all(|id| progress.contains_key(id))
    {
        return Err("completion census is incomplete".into());
    }
    if !progress.get(&subarea).is_some_and(|p| p.discovered && p.exploration.is_none()) {
        return Err("current subarea completion not persisted".into());
    }
    let completed = ids.iter().filter(|id| progress.get(id).is_some_and(|p| p.discovered)).count();
    Ok(u8::try_from(completed * 100 / ids.len())?)
}

async fn observe_initial_discovery<F, Fut>(
    client: &mut DevClientSession,
    witness: F,
) -> Result<wire::MapResult, Box<Failure>>
where
    F: FnOnce(ActorPosition, wire::MapResult) -> Fut,
    Fut: Future<Output = TestResult<u8>>,
{
    let initial = query(client, 1).await?;
    let position = client.world_spatial().actor_position;
    let found = action(client, 2, 3, 0).await?;
    if client.world_spatial().actor_position != position {
        return Err(Failure::new(2, "position changed during stationary discovery check"));
    }
    validate_initial_find(&initial, &found, position, witness)
        .await
        .map_err(|error| Failure::new(2, error.to_string()))?;
    let started = action(client, 3, 1, 0).await?;
    if client.world_spatial().actor_position != position {
        return Err(Failure::new(3, "position changed during stationary discovery check"));
    }
    validate_legacy_start(&found, &started).map_err(|error| Failure::new(3, error.to_string()))?;
    Ok(started)
}

fn poi_target(x: u16, y: u16, floor: i8) -> ActorPosition {
    ActorPosition { x: i32::from(x), y: i32::from(y), floor: i16::from(floor) }
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> Result<(), Box<Failure>> {
    let mut client = connect_world(clients, 0xb8).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(1, format!("admission={error}"));
        failure.source = Some(error);
        failure
    })?;
    let first_session = client.join_snapshot().game_session_id;
    let started = observe_initial_discovery(&mut client, |position, observed| async move {
        completed_area_percentage(clients, world, position, &observed).await
    }).await?;
    // START's private points are deliberately revealed only in the player's viewport.
    // Exercise every currently revealed point with real automatic-walk and FIND commands.
    let mut progress = started.clone();
    if started.code == 0 {
        for _ in 0..7 {
            let Some(&(x, y, floor)) = progress.visible.first() else {
                break;
            };
            let walked = client
                .session_mut()
                .target_walk(poi_target(x, y, floor))
                .await
                .map_err(|error| {
                    verdict::record_error(verdict::IndexedError::as_error(&error));
                    let mut failure =
                        Failure::new(4, format!("walk-to-poi={x},{y},{floor}:{error}"));
                    failure.source = Some(error.into());
                    failure
                })?;
            if walked.status != CommandStatus::Accepted || walked.payload != [8, 1] {
                return Err(Failure::new(4, format!("walk-reply={walked:?}")));
            }
            let target = poi_target(x, y, floor);
            let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
            while client.world_spatial().actor_position != target {
                if tokio::time::Instant::now() >= deadline {
                    return Err(Failure::new(
                        4,
                        format!(
                            "walk-timeout target={target:?} observed={:?}",
                            client.world_spatial().actor_position
                        ),
                    ));
                }
                client
                    .service_liveness(Duration::from_millis(100))
                    .await
                    .map_err(|error| {
                        verdict::record_error(verdict::IndexedError::as_error(&error));
                        let mut failure = Failure::new(4, format!("walk-liveness={error}"));
                        failure.source = Some(error.into());
                        failure
                    })?;
            }
            let before = progress.found;
            progress = action(&mut client, 4, 3, 0).await?;
            if progress.code != 0 || (!progress.discovered && progress.found <= before) {
                return Err(Failure::new(4, format!("find-no-delta={progress:?}")));
            }
        }
    }

    let before_donation = query(&mut client, 5).await?;
    let donated = action(&mut client, 5, 4, 1).await?;
    if !((donated.code == 0 && donated.pool == before_donation.pool + 1)
        || (donated.code == 6 && stable(&before_donation, &donated)))
    {
        return Err(Failure::new(5, format!("one-gold-donation={donated:?}")));
    }
    let goal = action(&mut client, 5, 4, 10_000_000).await?;

    let malformed = exchange(
        &mut client,
        wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
        &intent(1, 2, 0),
    )
    .await
    .map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(6, format!("malformed-command-error={error}"));
        failure.source = Some(error);
        failure
    })?;
    if malformed.status != CommandStatus::Rejected || !malformed.payload.is_empty() {
        return Err(Failure::new(6, format!("malformed-reply={malformed:?}")));
    }
    let retained = query(&mut client, 6).await?;
    logout(&mut client, 7).await?;
    drop(client);

    let mut fresh = connect_world(clients, 0xb9).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(7, format!("fresh-admission={error}"));
        failure.source = Some(error);
        failure
    })?;
    if fresh.join_snapshot().game_session_id == first_session {
        return Err(Failure::new(7, "fresh-admission-reused-session"));
    }
    let after = query(&mut fresh, 7).await?;
    let cleanup = logout(&mut fresh, 7).await;
    if !stable(&retained, &after) {
        return Err(Failure::new(
            7,
            format!("before={retained:?};after={after:?}"),
        ));
    }
    cleanup?;

    if started.code == 2 {
        return Err(Failure::new(
            3,
            format!("premium-required;reply={started:?}"),
        ));
    }
    if !progress.discovered || progress.found != 7 || progress.area_percent != 100 {
        return Err(Failure::new(
            4,
            format!("discovery-incomplete={progress:?}"),
        ));
    }
    if goal.code != 0 {
        return Err(Failure::new(5, format!("goal-donation-refused={goal:?}")));
    }
    // Reaching this point proves the playable discovery/donation lifecycle.  The owner
    // still emits no speed result/delta, so the full row cannot honestly claim PASS.
    Err(Failure::new(
        7,
        format!("movement-speed-delta-unavailable;{SOURCE_GAP}"),
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients, world)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=W12.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            let missing = error.observed.contains(SOURCE_GAP);
            evidence(&format!(
                "W12_CHECK step={} FAIL observed={}",
                error.step, error.observed
            ));
            evidence(&format!(
                "e2e_all scenario=W12.full status={} seconds={:.3}{}",
                if missing { "MISSING-SERVER" } else { "FAIL" },
                started.elapsed().as_secs_f64(),
                if missing {
                    format!(" evidence={SOURCE_GAP}")
                } else {
                    String::new()
                }
            ));
        }
        Err(_) => evidence(&format!(
            "e2e_all scenario=W12.full status=FAIL seconds={:.3} evidence=step-timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_encoder_and_durable_comparison_cover_the_wire_facts() {
        assert_eq!(
            wire::decode_request(wire::COMMAND_TYPE_CYCLOPEDIA_INTENT, &intent(4, 1, 7)),
            Ok(wire::MapIntent {
                operation: 4,
                amount: 7
            })
        );
        assert!(
            wire::decode_request(wire::COMMAND_TYPE_CYCLOPEDIA_INTENT, &intent(1, 2, 0)).is_err()
        );
        let state = wire::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: Vec::new(),
        };
        let mut refusal = state.clone();
        refusal.code = 4;
        refusal.revision = 2;
        assert!(stable(&state, &refusal));
        refusal.pool = 1;
        assert!(!stable(&state, &refusal));
    }

    #[test]
    fn canonical_completion_requires_every_child_in_the_bound_census() -> TestResult {
        use crate::domain::cyclopedia::MapProgress;
        let mut progress = BTreeMap::from([
            (1, MapProgress { discovered: true, exploration: None }),
            (2, MapProgress::default()),
            (3, MapProgress::default()),
        ]);
        assert_eq!(percentage_from_completed_census(&[1, 2, 3], &progress, 1)?, 33);
        progress.remove(&2);
        assert!(percentage_from_completed_census(&[1, 2, 3], &progress, 1).is_err());
        Ok(())
    }

    #[test]
    fn w12_poi_target_retains_global_floor() -> TestResult {
        for floor in [0, 7, 15] {
            let mut state = opening_state();
            state.visible.push((32_100, 32_200, floor));
            let bytes = wire::encode_wire_result(&state).map_err(|error| format!("{error:?}"))?;
            let decoded = wire::decode_result(&bytes).map_err(|error| format!("{error:?}"))?;
            let &(x, y, observed_floor) = decoded.visible.first().ok_or("decoded POI absent")?;
            assert_eq!(poi_target(x, y, observed_floor), ActorPosition {
                x: 32_100, y: 32_200, floor: i16::from(floor),
            });
        }
        Ok(())
    }

    fn opening_state() -> wire::MapResult {
        wire::MapResult { code: 0, revision: 28, discovered: false, found: 0,
            area_percent: 0, pool: 12, epoch: 3, improved: false, visible: Vec::new() }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn automatic_find_checks_noop_visible_poi_and_witnessed_completion() -> TestResult {
        let at = ActorPosition { x: 32_100, y: 32_200, floor: 7 };
        let no_hit = opening_state();
        validate_initial_find(&no_hit, &no_hit, at, |_, _| async { Err("unexpected census".into()) }).await?;
        let mut initial = no_hit.clone();
        initial.visible = vec![(32_100, 32_200, 7), (32_104, 32_200, 7)];
        let mut found = initial.clone();
        found.revision = 29;
        found.found = 1;
        found.visible.remove(0);
        validate_initial_find(&initial, &found, at, |_, _| async { Err("unexpected census".into()) }).await?;
        initial.found = 6;
        initial.visible.truncate(1);
        found.found = 7;
        found.discovered = true;
        found.area_percent = 33;
        found.visible.clear();
        validate_initial_find(&initial, &found, at, |position, observed| async move {
            assert_eq!(position, at);
            assert_eq!(observed.revision, 29);
            Ok(33)
        }).await?;
        // A retained eight-point run can already have seven found before this FIND.
        initial.found = 7;
        initial.visible = vec![(32_104, 32_200, 7)];
        validate_initial_find(&initial, &found, at, |_, _| async { Ok(33) }).await?;
        validate_initial_find(&found, &found, at, |_, _| async { Err("unexpected census".into()) }).await?;
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn automatic_find_rejects_mutated_noop_and_unwitnessed_percentage() -> TestResult {
        let at = ActorPosition { x: 32_100, y: 32_200, floor: 7 };
        let initial = opening_state();
        let mut invalid = Vec::new();
        let mut altered = initial.clone(); altered.code = 4; invalid.push(altered);
        let mut altered = initial.clone(); altered.revision += 1; invalid.push(altered);
        let mut altered = initial.clone(); altered.found = 1; invalid.push(altered);
        let mut altered = initial.clone(); altered.area_percent = 1; invalid.push(altered);
        let mut altered = initial.clone(); altered.pool += 1; invalid.push(altered);
        let mut altered = initial.clone(); altered.epoch += 1; invalid.push(altered);
        let mut altered = initial.clone(); altered.improved = true; invalid.push(altered);
        let mut altered = initial.clone(); altered.discovered = true; invalid.push(altered);
        let mut altered = initial.clone(); altered.visible.push((32_104, 32_200, 7)); invalid.push(altered);
        for altered in invalid {
            let failure = validate_initial_find(&initial, &altered, at, |_, _| async { Err("unexpected census".into()) }).await;
            assert!(failure.is_err(), "accepted altered no-op: {altered:?}");
        }
        let mut initial = initial;
        initial.found = 6;
        initial.visible.push((32_100, 32_200, 7));
        let mut found = initial.clone();
        found.revision += 1; found.found = 7; found.discovered = true;
        found.area_percent = 34; found.visible.clear();
        assert!(validate_initial_find(&initial, &found, at, |_, _| async { Ok(33) }).await.is_err());
        found.area_percent = 33;
        assert!(validate_initial_find(&initial, &found, at, |_, _| async { Err("missing census".into()) }).await.is_err());
        Ok(())
    }

    #[test]
    fn automatic_start_requires_exact_active_or_completed_state() -> TestResult {
        let active = opening_state();
        validate_legacy_start(&active, &active)?;
        for code in [2, 3, 4] {
            let mut refused = active.clone(); refused.code = code;
            assert!(validate_legacy_start(&active, &refused).is_err());
        }
        let mut changed = active.clone(); changed.revision += 1;
        assert!(validate_legacy_start(&active, &changed).is_err());
        let mut completed = active; completed.discovered = true; completed.found = 7; completed.area_percent = 33;
        let mut refused = completed.clone(); refused.code = 3;
        validate_legacy_start(&completed, &refused)?;
        assert!(validate_legacy_start(&completed, &completed).is_err());
        Ok(())
    }

}

#[cfg(test)]
mod transport_tests {
    //! Regression for both actual W12 opening qualifiers over native TLS/Session.
    use super::*;
    use oteryn_protocol_oteryn as native;

    #[derive(Clone, Copy, Debug)]
    enum Qualifier {
        Full,
        Client,
    }

    #[test]
    fn w12_full_accepts_automatic_discovery_and_idempotent_start() -> TestResult {
        run_transport(Qualifier::Full)
    }

    #[test]
    fn w12_client_captures_automatic_discovery_and_idempotent_start() -> TestResult {
        run_transport(Qualifier::Client)
    }

    fn run_transport(qualifier: Qualifier) -> TestResult {
        let thread = std::thread::Builder::new()
            .name("w12-discovery-tls".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let result = (|| -> TestResult {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    runtime.block_on(tokio::task::LocalSet::new().run_until(async {
                        tokio::time::timeout(Duration::from_secs(20), exercise(qualifier))
                            .await
                            .map_err(|_| "W12 discovery TLS fixture deadline")?
                    }))
                })();
                result.map_err(|error| error.to_string())
            })?;
        match thread.join() {
            Ok(result) => result.map_err(Into::into),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

    async fn read(stream: &mut ServerStream) -> TestResult<Vec<u8>> {
        let size = stream.read_u32().await?;
        if size == 0 || size > native::MAX_WIRE_FRAME_BYTES {
            return Err("W12 frame outside native size bound".into());
        }
        let mut bytes = vec![0; usize::try_from(size)?];
        stream.read_exact(&mut bytes).await?;
        Ok(bytes)
    }

    async fn send(stream: &mut ServerStream, bytes: &[u8]) -> TestResult {
        stream.write_all(&framed(bytes)).await?;
        stream.flush().await?;
        Ok(())
    }

    #[derive(Debug)]
    struct PeerObservation {
        requests: Vec<(u64, u32, Vec<u8>)>,
        payload: Vec<u8>,
    }

    enum Opening {
        Full(wire::MapResult),
        Client((wire::MapResult, wire::MapResult, wire::MapResult)),
    }

    async fn exercise(qualifier: Qualifier) -> TestResult {
        let expected = wire::MapResult {
            code: 0,
            revision: 28,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 12,
            epoch: 3,
            improved: false,
            visible: Vec::new(),
        };
        let payload = wire::encode_wire_result(&expected)
            .map_err(|error| format!("W12 native map encoder: {error:?}"))?;
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let certificate = generated.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
        let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        config.alpn_protocols = vec![native::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let (finish, mut finished) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::task::spawn_local(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            let bytes = read(&mut stream).await?;
            let envelope = native::decode_wire_envelope(&bytes)?;
            assert!(envelope.client_bootstrap()?.supported_capabilities.contains(&57));
            send(&mut stream, &native::encode_server_accepted(&native::ServerAcceptedValue {
                game_session_id: native::GameSessionId::decode(&v7(1, 1))?,
                world_id: native::WorldId::decode(&v7(2, 2))?,
                channel_id: native::ChannelId::decode(&v7(3, 3))?,
                connection_generation: 1,
                current_server_sequence: 40,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &[57],
            })?).await?;
            let spatial = encode_world_spatial(&WorldSpatialObservation {
                content_generation: [0x11; 32],
                actor_position: ActorPosition { x: 32_100, y: 32_200, floor: 7 },
            });
            let overlay = native::world_object::encode_world_object_overlay_snapshot(&[])
                .map_err(|error| format!("W12 overlay: {error:?}"))?;
            for frame in native::encode_single_chunk_snapshot(1, 1, 40, &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial,
                },
                DomainSnapshot {
                    domain_id: native::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: native::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
            ])? {
                send(&mut stream, &frame).await?;
            }
            let mut requests = Vec::new();
            for (offset, operation) in [0u8, 3, 1].into_iter().enumerate() {
                let bytes = tokio::select! {
                    biased;
                    _ = &mut finished => break,
                    bytes = read(&mut stream) => bytes?,
                };
                let command = native::decode_wire_envelope(&bytes)?.client_command(1)?;
                let next_id = 7 + u64::try_from(offset)?;
                let kind = if operation == 0 { 72 } else { 73 };
                let expected_request = if operation == 0 {
                    Vec::new()
                } else {
                    vec![operation, 1, 0, 0, 0, 0, 0, 0, 0]
                };
                assert_eq!(command.command_id, next_id);
                assert_eq!(command.command_type, kind);
                assert_eq!(command.payload, expected_request);
                assert_eq!(
                    wire::decode_request(command.command_type, command.payload),
                    Ok(wire::MapIntent { operation, amount: 0 }),
                );
                requests.push((next_id, kind, command.payload.to_vec()));
                send(&mut stream, &native::encode_command_result(
                    1, 41 + u64::try_from(offset)?, next_id, CommandStatus::Accepted, &payload,
                )?).await?;
            }
            Ok::<_, Box<dyn std::error::Error>>(PeerObservation { requests, payload })
        });
        let admitted = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id: CharacterId::decode(&v7(4, 4))?,
            admission_material: b"w12-discovery-fixture",
            client_build_id: "w12-discovery-tls-test",
            deadline: Duration::from_secs(5),
        }).await;
        let mut client = match admitted {
            Ok(client) => client,
            Err(error) => {
                let _ = finish.send(());
                let peer = server.await;
                return Err(format!("W12 fixture admission failed: {error}; peer={peer:?}").into());
            }
        };
        let mut captures = Vec::new();
        let result = match qualifier {
            Qualifier::Full => observe_initial_discovery(&mut client, |_, _| async {
                Err("unexpected completion witness in no-hit fixture".into())
            })
                .await
                .map(Opening::Full)
                .map_err(|error| error.to_string()),
            Qualifier::Client => {
                let mut record = |name: &str, bytes: &[u8]| {
                    captures.push((name.to_owned(), bytes.to_vec()));
                };
                super::super::client_row_w12::observe_initial_discovery_for_test(&mut client, &mut record, |_, _| async { Err("unexpected completion witness in no-hit fixture".into()) })
                    .await
                    .map(Opening::Client)
                    .map_err(|error| error.to_string())
            }
        };
        // Wake a peer waiting for the third command when the old qualifier rejects FIND.
        // The actual semantic error and peer state are collected before returning.
        let _ = finish.send(());
        let peer = match server.await {
            Ok(Ok(peer)) => peer,
            other => return Err(format!("W12 fixture peer failed: {other:?}; qualifier_error={:?}", result.as_ref().err()).into()),
        };
        let opening = result.map_err(|error| format!(
            "W12 actual {qualifier:?} qualifier rejected native Accepted/code0 automatic discovery: {error}; decoded_requests={:?}",
            peer.requests,
        ))?;
        assert_eq!(peer.requests.len(), 3, "Query, Find and idempotent Start must all execute");
        match opening {
            Opening::Full(started) => assert_eq!(started, expected),
            Opening::Client((query, find, started)) => {
                assert_eq!(query, expected);
                assert_eq!(find, expected);
                assert_eq!(started, expected);
                assert_eq!(captures, vec![
                    ("w12-query.bin".into(), peer.payload.clone()),
                    ("w12-find.bin".into(), peer.payload.clone()),
                    ("w12-start.bin".into(), peer.payload),
                ]);
            }
        }
        Ok(())
    }
}
