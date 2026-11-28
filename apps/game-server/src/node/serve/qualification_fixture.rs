//! Disposable node-boot build provisioning. The node retains its incarnation
//! proof; the root-only control socket supplies only an admitted session id.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::qualification_spell_character as qualification_build;
#[cfg(any(test, feature = "qualification-spawn"))]
use crate::qualification_wild_spawn as wild_spawn;

#[cfg(feature = "qualification-spawn")]
fn named_item(profile: &str) -> Option<(u32, u32)> {
    let (id, count) = profile.strip_prefix("item-")?.split_once('-')?;
    if id.starts_with('0')
        || id.len() > 10
        || !id.bytes().all(|byte| byte.is_ascii_digit())
        || count.starts_with('0')
        || !count.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let id: u32 = id.parse().ok()?;
    let count: u32 = count.parse().ok()?;
    (id != 0 && (1..=20).contains(&count)).then_some((id, count))
}

#[cfg(feature = "qualification-spawn")]
pub(super) const UNASSIGNED_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000014";
#[cfg(feature = "qualification-spawn")]
const KNIGHT_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000015";
#[cfg(feature = "qualification-spawn")]
const MONK_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000016";

#[cfg(feature = "qualification-spawn")]
const PVP_KNIGHT_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000005";

#[cfg(feature = "qualification-spawn")]
const PVP_SORCERER_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000006";

#[cfg(feature = "qualification-spawn")]
const LIGHT_ACCOUNT: &str = "01934f10-7c00-7000-8000-00000000000d";

#[cfg(feature = "qualification-spawn")]
const HASTE_ACCOUNT: &str = "01934f10-7c00-7000-8000-00000000000f";

#[cfg(feature = "qualification-spawn")]
const CHALLENGE_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000013";

#[cfg(feature = "qualification-spawn")]
const PARTY_CASTER_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000017";

#[cfg(feature = "qualification-spawn")]
const C06_CASTER_ACCOUNT: &str = "01934f10-7c00-7000-8000-00000000001c";

#[cfg(feature = "qualification-spawn")]
const C10_FULL_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000024";
#[cfg(feature = "qualification-spawn")]
const C10_CLIENT_ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000025";

#[cfg(feature = "qualification-spawn")]
fn lane_spell_build(profile: &str) -> Option<(&'static str, u32)> {
    match profile {
        "s08-knight" => Some(("knight", 8)),
        "s08-sorcerer" => Some(("sorcerer", 8)),
        "s13-party" => Some(("sorcerer", 32)),
        "w10-primal" => Some(("knight", 500)),
        "w20b-light" => Some(("sorcerer", 26)),
        "c05-exiva" => Some(("sorcerer", 8)),
        "c10-sorcerer" => Some(("sorcerer", 20)),
        "c11-unassigned" => Some((crate::durability::character_build::NO_VOCATION, 8)),
        "c11-knight" => Some(("knight", 625)),
        "c12-monk" => Some(("monk", 20)),
        "c06-sorcerer" => Some(("sorcerer", 15)),
        "f5-c17a" => Some(("sorcerer", 20)),
        "f5-c17c" => Some(("sorcerer", 23)),
        "f5-c13" => Some(("sorcerer", 200)),
        "f5-c18a" => Some(("sorcerer", 1)),
        "f5-c17d" => Some(("knight", 20)),
        _ => None,
    }
}

fn request(bytes: &[u8]) -> Option<([u8; 16], &str)> {
    let text = std::str::from_utf8(bytes).ok()?.trim_end_matches('\n');
    let (session, profile) = text.strip_prefix("qualify ")?.split_once(' ')?;
    let session = parse_operation_id(session.as_bytes())?;
    let allowed = spell_build(profile).is_some();
    #[cfg(feature = "qualification-spawn")]
    let allowed = allowed || named_item(profile).is_some() || profile == "premium";
    allowed.then_some((session, profile))
}

fn spell_build(profile: &str) -> Option<(&str, u32, bool)> {
    #[cfg(feature = "qualification-spawn")]
    if let Some((vocation, level)) = lane_spell_build(profile) {
        return Some((vocation, level, false));
    }
    match profile {
        "explorer" => Some(("knight", 100, false)),
        "knight-underlevel" => Some(("knight", 66, false)),
        "levitate" => Some(("sorcerer", 12, true)),
        "magic-rope" => Some(("sorcerer", 9, true)),
        vocation if qualification_build::PLAYABLE_VOCATIONS.contains(&vocation) => {
            Some((vocation, 100, false))
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum EarlyRefusal {
    FixtureDisabled,
    NativeUnavailable,
    MalformedRequest,
}

fn prepare_request<T>(
    enabled: bool,
    native: Option<T>,
    bytes: &[u8],
) -> Result<(T, [u8; 16], &str), EarlyRefusal> {
    if !enabled {
        return Err(EarlyRefusal::FixtureDisabled);
    }
    let native = native.ok_or(EarlyRefusal::NativeUnavailable)?;
    let (session, profile) = request(bytes).ok_or(EarlyRefusal::MalformedRequest)?;
    Ok((native, session, profile))
}

fn early_refusal_warning(reason: EarlyRefusal) -> oteryn_error_codes::Line<'static> {
    use oteryn_error_codes::{FieldValue, Level, Line};
    let reason = match reason {
        EarlyRefusal::FixtureDisabled => "fixture_disabled",
        EarlyRefusal::NativeUnavailable => "native_unavailable",
        EarlyRefusal::MalformedRequest => "malformed_request",
    };
    Line::new(Level::Warn, "node", "qualification_fixture_refused")
        .field("reason", FieldValue::Token(reason))
}

pub(super) async fn prepare(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: Option<&NativeGameplayState>,
    #[cfg(feature = "qualification-spawn")] premium: Option<
        &crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator,
    >,
    bytes: &[u8],
) -> &'static str {
    let (native, session, profile) =
        match prepare_request(config.qualification_fixture, native, bytes) {
            Ok(prepared) => prepared,
            Err(reason) => {
                emit(PROCESS, &early_refusal_warning(reason));
                return "rejected";
            }
        };
    // The supervisor polls other owners in the same task as the control loop.
    match Box::pin(DurabilityRoot::with_holder_owner(prepare_build(
        root,
        authority,
        proof,
        config,
        native,
        session,
        profile,
        #[cfg(feature = "qualification-spawn")]
        premium,
    )))
    .await
    {
        Ok(()) => "qualified",
        Err(error) => {
            emit(PROCESS, &refusal_warning(&session, profile, error.as_ref()));
            "rejected"
        }
    }
}

/// The live Channel owner and the room the `spawn` command places creatures in.
/// Only the qualification-spawn build (and tests) reads the fields; other builds refuse unplaced.
#[cfg_attr(not(any(test, feature = "qualification-spawn")), allow(dead_code))]
#[derive(Clone, Copy)]
pub(super) struct SpawnOwners<'a> {
    pub(super) runtime: &'a Mutex<ChannelRuntimeV1>,
    pub(super) room: &'a crate::content::QualifiedNativeEntryRoom,
}

/// `spawn <session> <creature key>`: only a node in the qualification fixture mode serves it.
fn spawn_request(enabled: bool, bytes: &[u8]) -> Result<([u8; 16], &str), &'static str> {
    if !enabled {
        return Err("not_qualification");
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "malformed_request")?
        .trim_end_matches('\n');
    let (session, key) = text
        .strip_prefix("spawn ")
        .and_then(|rest| rest.split_once(' '))
        .ok_or("malformed_request")?;
    let session = parse_operation_id(session.as_bytes()).ok_or("malformed_request")?;
    let bounded = !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":._/-".contains(&byte));
    bounded.then_some((session, key)).ok_or("malformed_request")
}

fn spawn_rejected(reason: &str) -> String {
    format!("rejected reason={reason}")
}

fn spawn_refusal_warning(
    reason: &'static str,
    detail: Option<&str>,
) -> oteryn_error_codes::Line<'static> {
    use oteryn_error_codes::{FieldValue, Level, Line};
    let line = Line::new(Level::Warn, "node", "qualification_spawn_refused")
        .field("reason", FieldValue::Token(reason));
    match detail {
        // Line's token renderer bounds and sanitizes every extension field.
        Some(detail) => line.field(
            "detail",
            FieldValue::Owned(detail.chars().take(200).collect()),
        ),
        None => line,
    }
}

/// Realises creature `key` next to the actor of `session` through the same fenced owner path
/// as the harness Rat, and answers `spawned <entity identity hex> <generation>`.
pub(super) async fn spawn(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: Option<&NativeGameplayState>,
    owners: Option<SpawnOwners<'_>>,
    bytes: &[u8],
) -> String {
    let refuse = |reason: &'static str, detail: Option<&str>| {
        emit(PROCESS, &spawn_refusal_warning(reason, detail));
        spawn_rejected(reason)
    };
    let (session, key) = match spawn_request(config.qualification_fixture, bytes) {
        Ok(request) => request,
        Err(reason) => return refuse(reason, None),
    };
    let (Some(native), Some(owners)) = (native, owners) else {
        return refuse("native_unavailable", None);
    };
    Box::pin(spawn_placed(
        root, authority, proof, native, owners, session, key, refuse,
    ))
    .await
}

#[cfg(any(test, feature = "qualification-spawn"))]
pub(super) async fn inspect_damage(
    config: &NodeConfig,
    owners: Option<SpawnOwners<'_>>,
    request: &[u8],
) -> Option<String> {
    if !config.qualification_fixture {
        return None;
    }
    let text = std::str::from_utf8(request).ok()?.strip_suffix('\n')?;
    let mut fields = text.split(' ');
    if fields.next()? != "inspect-damage" {
        return None;
    }
    let session =
        crate::foundation::GameSessionId::decode(&parse_operation_id(fields.next()?.as_bytes())?)
            .ok()?;
    let identity = super::super::operator_files::hex_bytes::<16>(fields.next()?).ok()?;
    let generation = fields.next()?.parse::<std::num::NonZeroU64>().ok()?.get();
    if fields.next().is_some() {
        return None;
    }
    let mut runtime = owners?.runtime.lock().await;
    runtime.positioned_player_for_session(session).ok()??;
    let actor = runtime
        .positioned_actor_census()
        .ok()?
        .into_iter()
        .find(|(actor, _, session)| {
            session.is_none()
                && actor.actor_local_generation() == generation
                && actor.placement_identity() == identity
        })?
        .0;
    let total = runtime
        .borrow_combat_death()
        .current_session_damage_shares(actor)
        .ok()?
        .into_iter()
        .find(|share| share.session == Some(session))
        .map_or(0, |share| share.total);
    let corpse = actor
        .corpse_identity()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Some(format!("damage {total} {corpse}"))
}

#[cfg(any(test, feature = "qualification-spawn"))]
#[allow(clippy::too_many_arguments)]
async fn spawn_placed(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    native: &NativeGameplayState,
    owners: SpawnOwners<'_>,
    session: [u8; 16],
    key: &str,
    refuse: impl Fn(&'static str, Option<&str>) -> String,
) -> String {
    let budget = SpawnBudget::new(tokio::time::Instant::now());
    let placed = spawn_with_watchdog(budget.reply_deadline, async |published| {
        Box::pin(place(
            root, authority, proof, native, owners, session, key, budget, published,
        ))
        .await
    })
    .await;
    match placed {
        Ok(Ok(creature)) => {
            let identity = creature
                .actor
                .placement_identity()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            format!(
                "spawned {identity} {}",
                creature.actor.actor_local_generation()
            )
        }
        Ok(Err(refusal)) => refuse(refusal.token(), refusal.detail()),
        Err(_) => refuse("deadline", None),
    }
}

#[cfg(not(any(test, feature = "qualification-spawn")))]
#[allow(clippy::too_many_arguments)]
async fn spawn_placed(
    _root: &DurabilityRoot,
    _authority: &ReconciledCharacterAuthority<'_, '_>,
    _proof: &NodeIncarnationProof,
    _native: &NativeGameplayState,
    _owners: SpawnOwners<'_>,
    _session: [u8; 16],
    _key: &str,
    refuse: impl Fn(&'static str, Option<&str>) -> String,
) -> String {
    // The test-support owners are not in this build; only the qualification-spawn build places.
    refuse("not_qualification", None)
}

#[cfg(any(test, feature = "qualification-spawn"))]
#[derive(Clone, Copy)]
struct SpawnBudget {
    work_deadline: tokio::time::Instant,
    backend_deadline: tokio::time::Instant,
    release_deadline: tokio::time::Instant,
    reply_deadline: tokio::time::Instant,
}

#[cfg(any(test, feature = "qualification-spawn"))]
impl SpawnBudget {
    fn new(start: tokio::time::Instant) -> Self {
        Self {
            work_deadline: start + Duration::from_secs(8),
            backend_deadline: start + Duration::from_secs(9),
            release_deadline: start + Duration::from_millis(9_500),
            reply_deadline: start + Duration::from_secs(10),
        }
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn spawn_with_watchdog<T>(
    deadline: tokio::time::Instant,
    operation: impl AsyncFnOnce(&mut Option<T>) -> Result<(), wild_spawn::SpawnRefusal>,
) -> Result<Result<T, wild_spawn::SpawnRefusal>, tokio::time::error::Elapsed> {
    let mut published = None;
    let result = tokio::time::timeout_at(deadline, operation(&mut published)).await;
    if let Some(published) = published {
        if result.is_err() {
            spawn_cleanup_warning("watchdog", "deadline_after_publication");
        }
        return Ok(Ok(published));
    }
    result.map(|result| {
        result.and(Err(wild_spawn::SpawnRefusal::Fence(
            "qualification spawn produced no receipt".into(),
        )))
    })
}

#[cfg(any(test, feature = "qualification-spawn"))]
#[allow(clippy::too_many_arguments)]
async fn place(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    native: &NativeGameplayState,
    owners: SpawnOwners<'_>,
    session: [u8; 16],
    key: &str,
    budget: SpawnBudget,
    published: &mut Option<crate::foundation::CompanionSnapshot>,
) -> Result<(), wild_spawn::SpawnRefusal> {
    let fence = wild_spawn::SpawnRefusal::Fence;
    let session = crate::foundation::GameSessionId::decode(&session)
        .map_err(|_| wild_spawn::SpawnRefusal::SessionUnknown)?;
    let source = wild_spawn::next_command_source()?;
    let mut runtime = owners.runtime.lock().await;
    let mut plan = Some(wild_spawn::snapshot_adjacent_spawn(
        owners.room,
        native,
        &mut runtime,
        key,
        session,
    )?);
    drop(runtime);
    let acquire_holder = async || {
        acquire_spawn_holder(root, budget.work_deadline)
            .await
            .map_err(|error| fence(format!("qualification spawn holder: {error:?}")))
    };
    let mut holder = acquire_holder().await?;
    let eligible = spawn_transaction(&mut holder, budget, true, async |tx| {
        let plan = plan
            .as_ref()
            .ok_or_else(|| fence("missing qualification spawn plan".into()))?;
        wild_spawn::read_spawn_tiles(tx, root, authority, proof, plan).await
    })
    .await;
    let mut eligible = match eligible {
        SpawnTransaction::Clean(Ok(eligible)) => eligible,
        failure => {
            let result = finish_spawn_holder(holder, budget, failure).await.map(drop);
            if result.is_err() {
                root.request_ready();
            }
            return result;
        }
    };

    let result = retry_spawn_collisions(
        holder,
        budget,
        published,
        async || {
            // Gameplay can hold the runtime while waiting on Ground.
            drop(
                tokio::time::timeout_at(budget.work_deadline, owners.runtime.lock())
                    .await
                    .map_err(|_| fence("qualification spawn runtime deadline".into()))?,
            );
            acquire_holder().await
        },
        async |tx| {
            let fresh = wild_spawn::read_spawn_tiles(
                tx,
                root,
                authority,
                proof,
                plan.as_ref()
                    .ok_or_else(|| fence("missing qualification spawn plan".into()))?,
            )
            .await?;
            if let Ok(mut runtime) = owners.runtime.try_lock() {
                if tokio::time::Instant::now() >= budget.work_deadline {
                    return Err(fence("qualification spawn publication deadline".into()));
                }
                let plan = plan
                    .take()
                    .ok_or_else(|| fence("missing qualification spawn plan".into()))?;
                let tiles = publication_candidates(fresh, std::mem::take(&mut eligible));
                return Ok(Some(wild_spawn::publish_spawn(
                    &mut runtime,
                    plan,
                    source,
                    key,
                    &tiles,
                )?));
            }
            Ok(None)
        },
    )
    .await;
    if result.is_err() {
        root.request_ready();
    }
    result
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn acquire_spawn_holder(
    root: &DurabilityRoot,
    deadline: tokio::time::Instant,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, DurabilityError> {
    let mut waiting = None;
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(DurabilityError::RootUnavailable);
        }
        match root.try_acquire_ready() {
            Err(DurabilityError::RootUnavailable) => {}
            outcome => return outcome,
        }
        // Hold priority and a FIFO turn only while retrying the checkout.
        if waiting.is_none() {
            waiting = Some((root.foreground_demand()?, root.holder_waiter()));
        }
        let now = tokio::time::Instant::now();
        let latest = (now + Duration::from_millis(250)).min(deadline);
        if !root.wait_holder_turn(now, latest).await {
            // A retired connection has no turn to wait for. The node's maintenance
            // can re-establish it and signal readiness within the same retry window.
            root.wait_holder_retry(now, latest).await;
        }
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn begin_spawn_transaction(
    holder: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    backend_deadline: tokio::time::Instant,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, wild_spawn::SpawnRefusal> {
    let fence = wild_spawn::SpawnRefusal::Fence;
    let setup = Duration::from_millis(500);
    let started = tokio::time::Instant::now();
    // Reserve one setup round trip between the server timer and the backend
    // cutoff. A slower setup may have armed the timer late, so take no locks.
    let tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(
        holder,
        backend_deadline - setup,
    )
    .await
    .map_err(|error| fence(format!("qualification spawn transaction: {error:?}")))?;
    if started.elapsed() >= setup {
        return Err(fence(
            "qualification spawn transaction setup deadline".into(),
        ));
    }
    Ok(tx)
}

#[cfg(any(test, feature = "qualification-spawn"))]
enum SpawnTransaction<T> {
    Clean(Result<T, wild_spawn::SpawnRefusal>),
    Uncertain(wild_spawn::SpawnRefusal),
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn retry_spawn_collisions<T>(
    mut holder: sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
    published: &mut Option<T>,
    mut reacquire: impl AsyncFnMut() -> Result<
        sqlx::pool::PoolConnection<sqlx::Postgres>,
        wild_spawn::SpawnRefusal,
    >,
    mut attempt: impl AsyncFnMut(
        &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Option<T>, wild_spawn::SpawnRefusal>,
) -> Result<(), wild_spawn::SpawnRefusal> {
    loop {
        spawn_collision(holder, budget, published, &mut attempt).await?;
        if published.is_some() {
            return Ok(());
        }
        holder = reacquire().await?;
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn spawn_transaction<T>(
    holder: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
    commit: bool,
    operation: impl AsyncFnOnce(
        &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<T, wild_spawn::SpawnRefusal>,
) -> SpawnTransaction<T> {
    let fence = wild_spawn::SpawnRefusal::Fence;
    let mut phase = "transaction_begin";
    let outcome = tokio::time::timeout_at(budget.work_deadline, async {
        let mut tx = match begin_spawn_transaction(holder, budget.backend_deadline).await {
            Ok(tx) => tx,
            Err(error) => return SpawnTransaction::Uncertain(error),
        };
        phase = "transaction_work";
        let result = operation(&mut tx).await;
        phase = "transaction_release";
        let released = if commit && result.is_ok() {
            tx.commit().await
        } else {
            tx.rollback().await
        };
        if let Err(error) = released {
            spawn_cleanup_warning(phase, &format!("{error:?}"));
            return SpawnTransaction::Uncertain(result.err().unwrap_or_else(|| {
                fence("qualification spawn transaction release failed".into())
            }));
        }
        SpawnTransaction::Clean(result)
    })
    .await;
    match outcome {
        Ok(result) => result,
        Err(_) => {
            spawn_cleanup_warning(phase, "work_deadline");
            SpawnTransaction::Uncertain(fence(format!("qualification spawn {phase} deadline")))
        }
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn spawn_collision<T>(
    mut holder: sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
    published: &mut Option<T>,
    operation: impl AsyncFnOnce(
        &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Option<T>, wild_spawn::SpawnRefusal>,
) -> Result<(), wild_spawn::SpawnRefusal> {
    let result = spawn_transaction(&mut holder, budget, false, async |tx| {
        *published = operation(tx).await?;
        Ok(())
    })
    .await;
    finish_spawn_holder(holder, budget, result).await
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn finish_spawn_holder<T>(
    mut holder: sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
    transaction: SpawnTransaction<T>,
) -> Result<T, wild_spawn::SpawnRefusal> {
    use sqlx::pool::PoolConnectionReturnDisposition;
    let result = match transaction {
        SpawnTransaction::Clean(result) => result,
        SpawnTransaction::Uncertain(error) => {
            // An unacknowledged BEGIN can leave a live transaction that SQLx
            // has not recorded. Retire the socket before releasing the slot.
            drop(holder.detach());
            spawn_cleanup_warning("holder_retired", error.detail().unwrap_or(error.token()));
            return Err(error);
        }
    };
    let (disposition, cause) = holder
        .return_to_pool_observed_until_with_cause(budget.release_deadline.into_std())
        .await;
    match disposition {
        PoolConnectionReturnDisposition::ReturnedToIdle => result,
        _ => {
            let cause = cause.unwrap_or("unknown");
            spawn_cleanup_warning("pool_return", cause);
            result.and(Err(wild_spawn::SpawnRefusal::Fence(format!(
                "qualification spawn pool return: {cause}"
            ))))
        }
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
fn spawn_cleanup_warning(phase: &'static str, cause: &str) {
    use oteryn_error_codes::{FieldValue, Level, Line};
    emit(
        PROCESS,
        &Line::new(Level::Warn, "node", "qualification_spawn_cleanup_failed")
            .field("phase", FieldValue::Token(phase))
            .field(
                "cause",
                FieldValue::Owned(cause.chars().take(200).collect()),
            ),
    );
}

#[cfg(any(test, feature = "qualification-spawn"))]
fn publication_candidates<T: PartialEq>(fresh: Vec<T>, mut committed: Vec<T>) -> Vec<T> {
    committed.retain(|candidate| fresh.contains(candidate));
    committed
}

/// This fixture emits only classified errors/static context, never request bytes or node proof.
fn refusal_warning(
    session: &[u8; 16],
    profile: &str,
    error: &(dyn std::error::Error + 'static),
) -> oteryn_error_codes::Line<'static> {
    use oteryn_error_codes::{FieldValue, Level, Line};
    let text = error.to_string();
    // Suppress common credential-bearing forms if a future adapter adds raw diagnostic text.
    let lower = text.to_ascii_lowercase();
    let sensitive = [
        "://",
        "password",
        "token:",
        "token=",
        "token(",
        "secret",
        "authorization:",
        "authorization=",
        "bearer ",
        "private key",
        "credential",
        "seed",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let reason = if sensitive {
        "redacted_sensitive_error".to_owned()
    } else {
        text.chars().take(200).collect()
    };
    // Line's token renderer additionally bounds and sanitizes every extension field.
    Line::new(Level::Warn, "node", "qualification_fixture_refused")
        .field("reason", FieldValue::Owned(reason))
        .field("session", FieldValue::Owned(uuid_text(session)))
        .field("profile", FieldValue::Owned(profile.to_owned()))
}

#[allow(clippy::too_many_arguments)] // The operator command borrows the node-owned services.
async fn prepare_build(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: &NativeGameplayState,
    session: [u8; 16],
    profile: &str,
    #[cfg(feature = "qualification-spawn")] premium: Option<
        &crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let session = crate::foundation::GameSessionId::decode(&session)?;
    // Independently resolve current admission, never synthesize a fence from
    // fixture SQL or expose the node proof to the client process.
    let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone());
    let current = qualification_build::retry_holder(
        Some(root),
        deadline,
        |error| matches!(error, DurabilityError::RootUnavailable),
        async || store.current_session(session).await,
    )
    .await
    .map_err(|error| format!("qualification admission: {error:?}"))?;
    let scope = current.current_runtime_scope();
    if Some(*scope.world_id().as_bytes()) != super::super::uuid_bytes(&config.scope.world_id)
        || scope.channel_id().map(|id| *id.as_bytes())
            != super::super::uuid_bytes(&config.scope.channel_id)
    {
        return Err("qualification scope mismatch".into());
    }
    #[cfg(feature = "qualification-spawn")]
    if let Some(item) = named_item(profile) {
        return prepare_items(
            root,
            authority,
            proof,
            &current,
            native,
            deadline,
            Some(item),
        )
        .await;
    }
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )
    .map_err(|error| format!("qualification character: {error:?}"))?;
    #[cfg(feature = "qualification-spawn")]
    if matches!(
        profile,
        "premium"
            | "c10-sorcerer"
            | "c11-unassigned"
            | "c11-knight"
            | "c12-monk"
            | "c06-sorcerer"
            | "s13-party"
            | "w10-primal"
            | "w20b-light"
            | "s08-knight"
            | "s08-sorcerer"
    ) {
        if current.session_state() != crate::foundation::GameSessionState::Active
            || current.current_transport().is_none()
        {
            return Err("dedicated preparation requires an active admission".into());
        }
        let record = qualification_build::retry_holder(
            Some(root),
            deadline,
            qualification_build::authority_busy,
            async || root.read_current_character(authority, character).await,
        )
        .await
        .map_err(qualification_build::debug)?;
        let account = uuid_text(record.account_id.as_bytes());
        match (profile, account.as_str()) {
            (
                "premium",
                KNIGHT_ACCOUNT | MONK_ACCOUNT | HASTE_ACCOUNT | CHALLENGE_ACCOUNT | LIGHT_ACCOUNT
                | PARTY_CASTER_ACCOUNT,
            ) => {
                if !config
                    .premium_activation
                    .as_ref()
                    .is_some_and(|record| record.activation_id == "node-boot-fixture-premium")
                {
                    return Err("qualification Premium activation absent".into());
                }
                return grant_premium(root, premium, *record.account_id.as_bytes(), deadline).await;
            }
            ("c11-unassigned", UNASSIGNED_ACCOUNT)
            | ("c10-sorcerer", C10_FULL_ACCOUNT | C10_CLIENT_ACCOUNT)
            | ("c11-knight", KNIGHT_ACCOUNT)
            | ("c12-monk", MONK_ACCOUNT)
            | ("c06-sorcerer", C06_CASTER_ACCOUNT)
            | ("s13-party", PARTY_CASTER_ACCOUNT)
            | ("w10-primal", "01934f10-7c00-7000-8000-000000000020")
            | ("w20b-light", LIGHT_ACCOUNT)
            | ("s08-knight", PVP_KNIGHT_ACCOUNT)
            | ("s08-sorcerer", PVP_SORCERER_ACCOUNT) => {}
            _ => return Err("preparation profile does not match dedicated account".into()),
        }
    }
    let interpretation = qualification_build::retry_holder(
        Some(root),
        deadline,
        qualification_build::authority_busy,
        async || {
            root.read_character_root_interpretation(authority, character)
                .await
        },
    )
    .await
    .map_err(|error| format!("qualification interpretation: {error:?}"))?
    .ok_or("qualification interpretation unavailable")?;
    let [profile_revision, ruleset_revision, content_revision, _] = interpretation.revisions();
    let progression = native
        .progression()
        .ok_or("qualification progression unavailable")?;
    let policy = progression.policy_for(profile_revision, ruleset_revision, content_revision)?;
    let spec = fixture_build_spec(profile, character.as_bytes(), policy.context.clone())
        .ok_or("qualification build profile unavailable")?;
    qualification_build::prepare_character_with_policy_until(
        root, authority, proof, &current, spec, policy, deadline,
    )
    .await?;
    #[cfg(feature = "qualification-spawn")]
    if lane_spell_build(profile).is_some() {
        return Ok(());
    }
    prepare_items(
        root,
        authority,
        proof,
        &current,
        native,
        deadline,
        #[cfg(feature = "qualification-spawn")]
        None,
    )
    .await
}

#[cfg(feature = "qualification-spawn")]
async fn grant_premium(
    root: &DurabilityRoot,
    coordinator: Option<
        &crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator,
    >,
    account: [u8; 16],
    deadline: tokio::time::Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::premium::{
        IngestOutcome, PRODUCER_PROFILE, PRODUCT_ID, PRODUCT_VERSION, SNAPSHOT_SCHEMA,
    };
    let account_text = uuid_text(&account);
    let coordinator = coordinator.ok_or("qualification Premium coordinator absent")?;
    let now = unix_now();
    let timestamp = |seconds| -> Result<String, time::error::ComponentRange> {
        let utc = time::OffsetDateTime::from_unix_timestamp(seconds)?;
        Ok(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            utc.year(),
            u8::from(utc.month()),
            utc.day(),
            utc.hour(),
            utc.minute(),
            utc.second()
        ))
    };
    let nonce =
        crate::premium::client::fresh_nonce().ok_or("qualification Premium nonce unavailable")?;
    let body = serde_json::to_vec(&serde_json::json!({
        "schema": SNAPSHOT_SCHEMA,
        "producer_revision": "0000000000000000000000000000000000000001",
        "producer_profile": PRODUCER_PROFILE,
        "nonce": nonce,
        "account_id": account_text,
        "product_id": PRODUCT_ID,
        "product_version": PRODUCT_VERSION,
        "entitlement_id": account_text,
        "entitlement_state": "ACTIVE",
        "lifecycle_revision": 1,
        "authority_revision": now,
        "effective_from": "2026-01-01T00:00:00Z",
        "effective_until": "2030-01-01T00:00:00Z",
        "authority_issued_at": timestamp(now)?,
        "authority_valid_until": timestamp(now + 3600)?,
        "refresh_after": timestamp(now + 2400)?,
    }))?;
    let consumer = coordinator.consumer();
    loop {
        let outcome = consumer.ingest(root, account, &nonce, &body).await;
        if matches!(outcome, IngestOutcome::Accepted | IngestOutcome::Replayed) {
            break;
        }
        if outcome != IngestOutcome::FenceUnavailable || tokio::time::Instant::now() >= deadline {
            return Err(format!("qualification Premium delivery refused: {outcome:?}").into());
        }
        root.request_ready();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    coordinator
        .premium_gate(account)
        .await
        .require()
        .map_err(|_| "qualification Premium gate remained closed".into())
}

pub(crate) fn fixture_build_spec<'a>(
    profile: &'a str,
    character: &[u8; 16],
    context: crate::domain::progression::ProgressionRevisionContext<String>,
) -> Option<qualification_build::QualificationBuildSpec<'a>> {
    let (vocation, level, exact_level) = spell_build(profile)?;
    let occurrence_profile = if level == 100 { vocation } else { profile };
    Some(qualification_build::QualificationBuildSpec {
        vocation,
        level,
        exact_level,
        context,
        xp_occurrence: fixture_occurrence(character, occurrence_profile, b"xp"),
        build_occurrence: fixture_occurrence(character, occurrence_profile, b"vocation"),
    })
}

// Stable, domain-separated occurrence IDs give retries an exact retained receipt.
fn fixture_occurrence(character: &[u8; 16], vocation: &str, phase: &[u8]) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:node-boot-fixture:v1:")
        .chain_update(character)
        .chain_update(vocation.as_bytes())
        .chain_update(b":")
        .chain_update(phase)
        .finalize();
    let mut result = [0; 16];
    result.copy_from_slice(&digest[..16]);
    result[6] = (result[6] & 0x0f) | 0x70;
    result[8] = (result[8] & 0x3f) | 0x80;
    result
}

// Fixture claims use the ordinary fenced/audited reward owner, never SQL item
// inserts. They are setup evidence only, not proof of player chest interaction.
async fn prepare_items(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    native: &NativeGameplayState,
    deadline: tokio::time::Instant,
    #[cfg(feature = "qualification-spawn")] named: Option<(u32, u32)>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "qualification-spawn")]
    use crate::domain::item_definition::ItemStackClass;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::CurrentCharacterItemFence;
    #[cfg(feature = "qualification-spawn")]
    use crate::durability::reward_claim_mint::RewardClaimMintContent;
    use crate::durability::reward_claim_mint::RewardClaimMintRequest;
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

    let fence = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(
            *current.current_character_lease().character_id().as_bytes(),
        )
        .map_err(|error| format!("qualification character: {error:?}"))?,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
    };
    let read_inventory = async || {
        qualification_build::retry_holder(
            Some(root),
            deadline,
            |error| {
                matches!(
                    error,
                    crate::durability::item_use::ItemUseError::Unavailable(
                        DurabilityError::RootUnavailable
                    )
                )
            },
            async || root.read_item_use_inventory(authority, proof, fence).await,
        )
        .await
        .map_err(|error| format!("qualification inventory: {error:?}"))
    };
    #[cfg(not(feature = "qualification-spawn"))]
    let named: Option<(u32, u32)> = None;
    let mut inventory = read_inventory().await?;
    if inventory.backpack.is_none() && named.is_none() {
        use crate::durability::starter_grant::{StarterGrantOutcome, StarterTemplate};
        let template = StarterTemplate {
            key: fixture_gear_claim(current.current_game_session_id(), "backpack"),
            ..StarterTemplate::main()
        };
        let outcome = qualification_build::retry_holder(
            Some(root),
            deadline,
            |error| matches!(error, DurabilityError::RootUnavailable),
            async || {
                root.grant_starter_items(
                    authority,
                    proof,
                    &fence,
                    &FixtureItemContent(native),
                    &template,
                )
                .await
            },
        )
        .await
        .map_err(|error| format!("qualification backpack recovery: {error:?}"))?;
        if outcome == StarterGrantOutcome::NotAdmitted {
            return Err("qualification backpack recovery was not admitted".into());
        }
        inventory = read_inventory().await?;
    }
    let bag = inventory
        .backpack
        .as_ref()
        .ok_or("qualification backpack missing after recovery")?;
    let backpack = generation_item_facts(native, &bag.backpack.definition)
        .map_err(|error| format!("qualification backpack: {error:?}"))?;
    let index = native
        .item_index()
        .ok_or("qualification Item index missing")?;
    // Legacy build profiles retain their axe, rope and shovel.
    let legacy = [(3276, 1), (3003, 1), (3457, 1)];
    for (ordinal, &(id, count)) in named
        .as_slice()
        .iter()
        .chain(legacy.iter().filter(|_| named.is_none()))
        .enumerate()
    {
        let key = format!("oteryn:item.tibia.i{id}");
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.clone(),
            revision_ref: index
                .revision(&key)
                .ok_or("qualification Item absent")?
                .into(),
        };
        if named.is_none()
            && inventory
                .equipment
                .iter()
                .chain(bag.entries.iter().map(|entry| &entry.item))
                .any(|existing| existing.definition == definition && existing.quantity == 1)
        {
            continue;
        }
        let item = generation_item_facts(native, &definition)
            .map_err(|error| format!("qualification Item {key}: {error:?}"))?;
        #[cfg(feature = "qualification-spawn")]
        let (quantity, additional_items) = {
            let maximum = match item.stack {
                ItemStackClass::NonStackable => 1,
                ItemStackClass::Stackable { proven_maximum } => proven_maximum.unwrap_or(100),
                ItemStackClass::Unknown => {
                    return Err("qualification Item stack class unknown".into());
                }
            };
            if maximum == 0 {
                return Err("qualification Item stack maximum is zero".into());
            }
            let quantity = count.min(maximum);
            let mut additional_items = Vec::new();
            let mut remaining = count - quantity;
            while remaining != 0 {
                let quantity = remaining.min(maximum);
                additional_items.push(RewardClaimMintContent {
                    item: item.clone(),
                    quantity,
                    key_number: None,
                    text: None,
                    fluid_subtype: None,
                    charges_remaining: None,
                });
                remaining -= quantity;
            }
            (quantity, additional_items)
        };
        #[cfg(not(feature = "qualification-spawn"))]
        let (quantity, additional_items) = (count, Vec::new());
        let offset = named.map_or(ordinal as u64, |(id, count)| {
            (u64::from(id) << 5) | u64::from(count)
        });
        let claim_key = if named.is_some() {
            format!("named-i{id}-{count}")
        } else {
            format!("i{id}")
        };
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            additional_items,
            // Reserved fixture ids cannot collide with the client's low command ids.
            command: crate::foundation::CommandRef::new(
                current.current_game_session_id(),
                crate::foundation::CommandId::new(u64::MAX - offset)?,
            ),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: fixture_gear_claim(current.current_game_session_id(), &claim_key),
                revision_ref: "qualification-v1".into(),
            },
            source_placement: "oteryn:qualification.node-boot".into(),
            item,
            quantity,
            contents: Vec::new(),
            root_text: None,
            root_fluid_subtype: None,
            root_charges_remaining: None,
            backpack: backpack.clone(),
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: "qualification-v1".into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        // Reservation replay retains the same CommandRef and frozen semantic intent.
        let mut candidate = qualification_build::retry_holder(
            Some(root),
            deadline,
            |error| {
                matches!(
                    error,
                    crate::durability::reward_claim_mint::RewardClaimMintError::Unavailable(
                        DurabilityError::RootUnavailable
                    )
                )
            },
            async || {
                root.freeze_reward_claim_mint(authority, proof, fence, request.clone())
                    .await
            },
        )
        .await
        .map_err(|error| format!("qualification Item reserve: {error:?}"))?;
        let outcome = root
            .commit_reward_claim_mint_waiting(authority, proof, fence, &mut candidate, deadline)
            .await
            .map_err(|error| format!("qualification Item commit: {error:?}"))?;
        fixture_item_committed(outcome, quantity)?;
    }
    Ok(())
}

// A successful setup reloads through a new admission. Inventory determines
// what needs replacement; within one admission, retries use the same receipt.
fn fixture_gear_claim(session: crate::foundation::GameSessionId, item: &str) -> String {
    let session = u128::from_be_bytes(*session.as_bytes());
    format!("oteryn:qualification.gear.{item}:{session:032x}")
}

struct FixtureItemContent<'a>(&'a NativeGameplayState);

impl crate::durability::item_transfer::ItemWeightLookup for FixtureItemContent<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }

    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        use crate::durability::item_transfer::ItemWeightRefused;
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(self.0, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?
            .weight
            .ok_or(ItemWeightRefused::Unknown)
    }
}

impl crate::durability::item_loop_move::ItemLoopContent for FixtureItemContent<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }

    fn loop_facts(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(self.0, definition).ok()
    }
}

fn fixture_item_committed(
    outcome: crate::durability::reward_claim_mint::RewardClaimMintOutcome,
    quantity: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::reward_claim_mint::RewardClaimMintOutcome;
    let receipt = match outcome {
        RewardClaimMintOutcome::Committed(receipt)
        | RewardClaimMintOutcome::AlreadyCommitted(receipt) => receipt,
    };
    if receipt.quantity != quantity {
        return Err(format!(
            "qualification Item receipt has quantity {}",
            receipt.quantity
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control_spawn_source() -> Result<&'static str, &'static str> {
        include_str!("qualification_fixture.rs")
            .split_once("async fn place(")
            .and_then(|(_, body)| body.split_once("async fn begin_spawn_transaction("))
            .map(|(body, _)| body)
            .ok_or("missing control spawn boundary")
    }

    #[test]
    fn qualification_spawn_releases_runtime_before_durable_reads() -> Result<(), &'static str> {
        let source = control_spawn_source()?;
        let release = source
            .find("drop(runtime)")
            .ok_or("missing snapshot lock release")?;
        let read = source
            .find("read_spawn_tiles(")
            .or_else(|| source.find("realize_creature_in_transaction("))
            .ok_or("missing durable read")?;
        assert!(release < read);
        Ok(())
    }

    #[test]
    fn qualification_spawn_keeps_collision_fence_until_publication() -> Result<(), &'static str> {
        let source = control_spawn_source()?;
        let probe = source
            .find("spawn_transaction(&mut holder, budget, true")
            .ok_or("missing committed probe")?;
        let guard = source
            .find("retry_spawn_collisions(")
            .ok_or("missing collision transaction")?;
        let fresh = source
            .rfind("read_spawn_tiles(")
            .ok_or("missing fresh collision read")?;
        let publish = source.find("publish_spawn(").ok_or("missing publish")?;
        assert!(probe < guard && guard < fresh && fresh < publish);
        assert!(source[fresh..publish].contains(">= budget.work_deadline"));
        Ok(())
    }

    #[test]
    fn qualification_spawn_watchdog_caller_contract() -> Result<(), &'static str> {
        let source = control_spawn_source()?;
        assert!(
            source.contains("finish_spawn_holder(holder, budget, failure)"),
            "pool return must be observed"
        );
        assert!(
            !source.contains(".detach()"),
            "successful spawn must preserve the ready holder"
        );
        assert!(
            source.contains("retry_spawn_collisions("),
            "backend expiry must share the request clock"
        );
        assert!(
            source.contains("published"),
            "the caller must retain a published receipt"
        );
        Ok(())
    }

    #[tokio::test]
    async fn qualification_spawn_reports_publication_if_cleanup_stalls() -> Result<(), &'static str>
    {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
        let reply = spawn_with_watchdog(deadline, async |published| {
            *published = Some(17);
            std::future::pending().await
        })
        .await;
        assert!(matches!(reply, Ok(Ok(17))));
        let rejected = spawn_with_watchdog(deadline, async |_: &mut Option<()>| {
            std::future::pending().await
        })
        .await;
        assert!(rejected.is_err());
        Ok(())
    }

    #[test]
    fn qualification_spawn_gets_its_turn_after_a_busy_holder()
    -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{Harness, debug, runtime};

        let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
            return Ok(());
        };
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                runtime()
                    .map_err(debug)?
                    .block_on(async {
                        let harness = Harness::create(admin, "spawn_holder", false).await?;
                        let result = exercise_busy_spawn(&harness).await;
                        harness.cleanup().await?;
                        result
                    })
                    .map_err(debug)
            })?
            .join()
            .map_err(|_| "qualification spawn test thread failed")?
            .map_err(Into::into)
    }

    async fn exercise_busy_spawn(
        harness: &crate::bestiary_postgres_harness::Harness,
    ) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{SESSION, debug, id, scope};
        use crate::foundation::GameSessionId;
        use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
        use sqlx::pool::PoolConnectionReturnDisposition;
        use std::future::{Future, poll_fn};
        use std::task::Poll;

        let scope = scope()?;
        let world = scope.world_id();
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/spells.manifest.json");
        let room =
            crate::content::qualify_native_entry_room_from_gameplay_manifest(world, &manifest)?;
        let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
        // Put the admitted player at the Thalom hunt cell used by the seam fixture.
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (0, 11, -7),
        );
        let fact = harness.node.fact();
        let mut channel = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            fact.node_id(),
            fact.registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )?;
        let session = GameSessionId::decode(&id(SESSION))?;
        let reservation = channel.reserve_fresh_session(session)?;
        let player = channel.commit_fresh_session(reservation)?;
        channel.initialize_first_entry_position(player)?;
        let channel = Mutex::new(channel);
        let owners = SpawnOwners {
            runtime: &channel,
            room: &room,
        };
        let config = format!(
            "qualification_fixture = true\n{}",
            crate::node::config::tests::NODE
        )
        .replace("database = \"oteryn\"", "database = \"oteryn_node_boot\"")
        .replace(
            "oteryn:runtime:world-1:channel-1",
            "oteryn:runtime:node-boot",
        );
        let config = NodeConfig::parse(config.as_bytes())?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let root = &harness.root;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(root.maintain_ready_once().await?);
        let mut busy = root.try_acquire_ready()?;
        assert!(!root.has_ready_demand());
        let request = format!(
            "spawn {} canary:creature/rat\n",
            uuid_text(session.as_bytes())
        );
        let spawn = spawn(
            root,
            &authority,
            &harness.node,
            &config,
            Some(&native),
            Some(owners),
            request.as_bytes(),
        );
        tokio::pin!(spawn);
        poll_fn(|cx| match spawn.as_mut().poll(cx) {
            Poll::Ready(reply) => Poll::Ready(Err(format!(
                "spawn completed while the holder was busy: {reply}"
            ))),
            Poll::Pending if root.has_ready_demand() => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        })
        .await?;

        assert_eq!(
            busy.return_to_pool_observed().await,
            PoolConnectionReturnDisposition::ReturnedToIdle
        );
        // Try the later owner before polling spawn again so the race has a fixed order.
        let competing = DurabilityRoot::with_holder_owner(async { root.try_acquire_ready() }).await;
        if !matches!(competing, Err(DurabilityError::RootUnavailable)) {
            return Err("the later owner took the returned holder before the waiting spawn".into());
        }

        let reply = spawn.await;
        let channel = channel.lock().await;
        let census = channel.positioned_actor_census()?;
        let [
            (_, player_position, Some(actual_session)),
            (creature, creature_position, None),
        ] = census.as_slice()
        else {
            return Err(
                format!("expected one player and one spawned creature, reply={reply}").into(),
            );
        };
        assert_eq!(*actual_session, session);
        assert!(
            wild_spawn::adjacent_cells(player_position.position())
                .contains(&creature_position.position())
        );
        let identity = creature
            .placement_identity()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            reply,
            format!("spawned {identity} {}", creature.actor_local_generation())
        );
        Ok(())
    }

    #[cfg(feature = "qualification-spawn")]
    #[tokio::test]
    async fn qualification_spawn_transaction_returns_pool_and_bounds_backend()
    -> Result<(), Box<dyn std::error::Error>> {
        use sqlx::Connection as _;
        let url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL").map_err(
            |_| "OTERYN_TEST_POSTGRES_ADMIN_URL is required for the spawn transaction regression",
        )?;
        tokio::time::timeout(Duration::from_secs(12), async {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect(&url)
                .await?;
            let mut holder = pool.acquire().await?;
            let mut writer = sqlx::PgConnection::connect(&url).await?;
            let start = tokio::time::Instant::now();
            let budget = SpawnBudget::new(start);
            let probe = spawn_transaction(&mut holder, budget, true, async |tx| {
                sqlx::query("CREATE TEMP TABLE qualification_spawn_committed_probe (id integer)")
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                Ok(())
            })
            .await;
            assert!(matches!(probe, SpawnTransaction::Clean(Ok(()))));
            let mut published = None;
            spawn_collision(holder, budget, &mut published, async |tx| {
                let committed: bool = sqlx::query_scalar(
                    "SELECT to_regclass('pg_temp.qualification_spawn_committed_probe') IS NOT NULL",
                )
                .fetch_one(&mut **tx)
                .await
                .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                assert!(
                    committed,
                    "the first probe must commit before the collision guard"
                );
                let key = sqlx::query_scalar::<_, i64>("SELECT pg_backend_pid()::bigint")
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                sqlx::query("SELECT pg_advisory_xact_lock($1)")
                    .bind(key)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                let could_write: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
                    .bind(key)
                    .fetch_one(&mut writer)
                    .await
                    .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                assert!(!could_write);
                Ok(Some(key))
            })
            .await
            .map_err(|e| format!("{}: {:?}", e.token(), e.detail()))?;
            let key = published.ok_or("collision did not record the receipt")?;
            let mut holder = pool
                .try_acquire()
                .ok_or("spawn did not return the ready holder")?;
            let pid: i64 = sqlx::query_scalar("SELECT pg_backend_pid()::bigint")
                .fetch_one(&mut *holder)
                .await?;
            assert_eq!(key, pid);

            // Model a BEGIN accepted by PostgreSQL while SQLx has not yet
            // recorded transaction depth. The failure path must retire this PID.
            sqlx::query("BEGIN").execute(&mut *holder).await?;
            let failed = finish_spawn_holder::<()>(
                holder, budget, SpawnTransaction::Uncertain(wild_spawn::SpawnRefusal::Fence("late BEGIN".into())),
            ).await;
            assert!(failed.is_err());
            let mut holder = pool.acquire().await?;
            let replacement: i64 = sqlx::query_scalar("SELECT pg_backend_pid()::bigint")
                .fetch_one(&mut *holder).await?;
            assert_ne!(replacement, pid, "a cancelled BEGIN backend was reused");

            // A refused collision rolls back cleanly and retains the exact holder.
            let mut published = None::<()>;
            let refusal = spawn_collision(holder, budget, &mut published, async |_| {
                Err(wild_spawn::SpawnRefusal::NoFreeTile("blocked".into()))
            }).await;
            assert!(matches!(refusal, Err(wild_spawn::SpawnRefusal::NoFreeTile(_))));
            let mut holder = pool.try_acquire().ok_or("clean refusal did not return the holder")?;
            let after_refusal: i64 = sqlx::query_scalar("SELECT pg_backend_pid()::bigint")
                .fetch_one(&mut *holder).await?;
            assert_eq!(replacement, after_refusal);

            let start = tokio::time::Instant::now();
            let late = SpawnBudget {
                work_deadline: start + Duration::from_millis(900),
                backend_deadline: start + Duration::from_millis(1500),
                release_deadline: start + Duration::from_millis(1700),
                reply_deadline: start + Duration::from_millis(1900),
            };
            let mut attempt = 0;
            let mut published = None;
            retry_spawn_collisions(holder, late, &mut published, async || {
                // Simulate contention for the runtime on the first attempt.
                tokio::time::sleep(Duration::from_millis(250)).await;
                pool.acquire().await.map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))
            }, async |tx| {
                attempt += 1;
                if attempt == 1 {
                    return Ok(None);
                }
                let timeout: f64 = sqlx::query_scalar(
                    "SELECT extract(epoch FROM current_setting('transaction_timeout')::interval)::float8",
                ).fetch_one(&mut **tx).await
                    .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                assert!((0.4..=0.75).contains(&timeout), "incorrect retry backend timer: {timeout}");
                sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(key)
                    .execute(&mut **tx).await.map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                let held: bool = sqlx::query_scalar("SELECT NOT pg_try_advisory_xact_lock($1)")
                    .bind(key).fetch_one(&mut writer).await.map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                assert!(held);
                Ok(Some(()))
            }).await.map_err(|e| format!("{}: {:?}", e.token(), e.detail()))?;
            assert_eq!(attempt, 2);
            assert_eq!(published, Some(()));

            // Keep the socket alive and idle with a held lock. PostgreSQL
            // must release it by itself even if no rollback or FIN arrives.
            let mut holder = pool.acquire().await?;
            let mut tx = begin_spawn_transaction(
                &mut holder, tokio::time::Instant::now() + Duration::from_secs(1),
            ).await.map_err(|e| format!("{}: {:?}", e.token(), e.detail()))?;
            sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(key)
                .execute(&mut *tx).await?;
            tokio::time::timeout(
                Duration::from_secs(1),
                sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(key).execute(&mut writer),
            ).await??;
            drop(tx);
            drop(holder.detach());

            // Stall the real collision caller during pool cleanup after its
            // guarded publication. The watchdog must still return the receipt.
            let stall = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let release_stall = stall.clone();
            let entered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let hook_entered = entered.clone();
            let stalled_pool = sqlx::postgres::PgPoolOptions::new().max_connections(1)
                .after_release(move |_, _| {
                    let stall = release_stall.clone();
                    let entered = hook_entered.clone();
                    Box::pin(async move {
                        if stall.load(std::sync::atomic::Ordering::Relaxed) {
                            entered.store(true, std::sync::atomic::Ordering::Relaxed);
                            std::future::pending().await
                        } else {
                            Ok(true)
                        }
                    })
                }).connect(&url).await?;
            let holder = stalled_pool.acquire().await?;
            let start = tokio::time::Instant::now();
            let budget = SpawnBudget {
                work_deadline: start + Duration::from_millis(350),
                backend_deadline: start + Duration::from_millis(750),
                release_deadline: start + Duration::from_millis(850),
                reply_deadline: start + Duration::from_millis(1250),
            };
            let reply = spawn_with_watchdog(budget.reply_deadline, async |published| {
                let result = spawn_collision(holder, budget, published, async |_| {
                    stall.store(true, std::sync::atomic::Ordering::Relaxed);
                    Ok(Some(17))
                }).await;
                assert!(matches!(&result, Err(error) if error.detail()
                    == Some("qualification spawn pool return: after_release_timeout")));
                result
            }).await;
            assert!(entered.load(std::sync::atomic::Ordering::Relaxed));
            assert!(matches!(reply, Ok(Ok(17))));
            let mut holder = pool.acquire().await?;
            let victim: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *holder).await?;
            let terminated: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1, 1000)")
                .bind(victim).fetch_one(&mut writer).await?;
            assert!(terminated);
            let outcome = finish_spawn_holder(
                holder, SpawnBudget::new(tokio::time::Instant::now()), SpawnTransaction::Clean(Ok(())),
            ).await;
            assert!(matches!(outcome, Err(error) if error.detail()
                == Some("qualification spawn pool return: ping_failed")));

            let mut empty = pool.acquire().await?;
            let returned = empty.return_to_pool_observed().await;
            assert_eq!(returned, sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle);
            let outcome = finish_spawn_holder(
                empty, SpawnBudget::new(tokio::time::Instant::now()), SpawnTransaction::Clean(Ok(())),
            ).await;
            assert!(matches!(outcome, Err(error) if error.detail()
                == Some("qualification spawn pool return: no_evidence")));
            Ok(())
        })
        .await?
    }

    #[test]
    fn qualification_fixture_restores_missing_carried_gear() -> Result<(), &'static str> {
        let source = include_str!("qualification_fixture.rs");
        let prepared = source
            .split_once("async fn prepare_items(")
            .and_then(|(_, body)| body.split_once("fn fixture_item_committed("))
            .map(|(body, _)| body)
            .ok_or("missing fixture equipment preparation")?;
        assert!(prepared.contains("grant_starter_items("));
        assert!(prepared.contains("fixture_gear_claim("));
        Ok(())
    }

    #[test]
    fn delta_spawn_drops_tiles_blocked_since_the_fence_commit() {
        assert!(publication_candidates(Vec::<u8>::new(), vec![1]).is_empty());
        assert_eq!(publication_candidates(vec![2], vec![1, 2]), vec![2]);
    }

    #[test]
    fn qualification_spawn_commits_before_placement() -> Result<(), &'static str> {
        let place = control_spawn_source()?;
        let probe = place
            .find("spawn_transaction(&mut holder, budget, true")
            .ok_or("missing committed fence probe")?;
        let publish = place
            .find("publish_spawn(")
            .ok_or("missing runtime placement")?;
        assert!(probe < publish);
        let source = include_str!("qualification_fixture.rs");
        let transaction = source
            .split_once("async fn spawn_transaction<T>(")
            .and_then(|(_, body)| body.split_once("async fn spawn_collision<T>("))
            .map(|(body, _)| body)
            .ok_or("missing spawn transaction boundary")?;
        assert!(transaction.contains("if commit && result.is_ok()"));
        assert!(transaction.contains("tx.commit().await"));
        Ok(())
    }

    #[test]
    fn review_accepts_fixture_item_receipt_replay() {
        use crate::durability::reward_claim_mint::{
            CommittedRewardClaimMint, RewardClaimMintOutcome,
        };
        let receipt = CommittedRewardClaimMint {
            transaction_id: [1; 16],
            event_id: [2; 16],
            item_instance_id: [3; 16],
            occurred_at_unix_ms: 1,
            envelope_sha256: [4; 32],
            quantity: 1,
            destination: crate::durability::item_transfer::ContainerEntryPosition {
                parent_item_instance_id: [5; 16],
                placement_ordinal: 0,
            },
        };
        assert!(
            fixture_item_committed(RewardClaimMintOutcome::Committed(receipt.clone()), 1).is_ok()
        );
        assert!(
            fixture_item_committed(RewardClaimMintOutcome::AlreadyCommitted(receipt.clone()), 1)
                .is_ok()
        );
        let stack = CommittedRewardClaimMint {
            quantity: 20,
            ..receipt
        };
        assert!(
            fixture_item_committed(RewardClaimMintOutcome::Committed(stack.clone()), 1).is_err()
        );
        assert!(
            fixture_item_committed(RewardClaimMintOutcome::AlreadyCommitted(stack), 20).is_ok()
        );
    }

    fn assert_early_warning(reason: EarlyRefusal, expected: &str) {
        let line = early_refusal_warning(reason);
        assert_eq!(line.level, oteryn_error_codes::Level::Warn);
        let rendered = line.render("test", 0);
        assert!(rendered.contains("event=qualification_fixture_refused"));
        assert!(rendered.contains(&format!("reason={expected}")));
        assert!(!rendered.contains("session=") && !rendered.contains("profile="));
        assert_eq!(rendered.lines().count(), 1);
    }

    #[test]
    fn fixture_disabled_has_a_classified_warning() {
        for request in [
            b"qualify 01890f4c-3b2a-7c01-8d11-9a321b7c0004 c05-exiva".as_slice(),
            b"qualify 01890f4c-3b2a-7c01-8d11-9a321b7c0004 item-6506-2",
        ] {
            let refused = prepare_request(false, Some(()), request).err();
            assert_eq!(refused, Some(EarlyRefusal::FixtureDisabled));
        }
        assert_early_warning(EarlyRefusal::FixtureDisabled, "fixture_disabled");
    }

    #[test]
    fn native_unavailable_has_a_classified_warning() {
        let refused = prepare_request(true, None::<()>, b"untrusted request").err();
        assert_eq!(refused, Some(EarlyRefusal::NativeUnavailable));
        assert_early_warning(EarlyRefusal::NativeUnavailable, "native_unavailable");
    }

    #[test]
    fn malformed_request_has_a_classified_warning() {
        for bytes in [
            b"untrusted\nproof=do-not-log".as_slice(),
            &[0xff],
            b"qualify invalid knight",
        ] {
            let refused = prepare_request(true, Some(()), bytes).err();
            assert_eq!(refused, Some(EarlyRefusal::MalformedRequest));
        }
        assert_early_warning(EarlyRefusal::MalformedRequest, "malformed_request");
    }

    #[test]
    fn python_provision_requests_reach_the_real_control_parser()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async {
                        let session = uuid_v7()?;
                        let path = std::env::temp_dir()
                            .join(format!("qualification-{}.sock", uuid_text(&session)));
                        let result = tokio::time::timeout(Duration::from_secs(10), async {
                            let listener = UnixListener::bind(&path)?;
                            for profile in [
                                "knight",
                                "druid",
                                "paladin",
                                "sorcerer",
                                "explorer",
                                "monk",
                                "levitate",
                                "magic-rope",
                                "c05-exiva",
                                "item-6506-2",
                            ] {
                                let script = Path::new(env!("CARGO_MANIFEST_DIR"))
                                    .join("../../tools/qualification/node_boot/provision.py");
                                let socket = path.clone();
                                let id = uuid_text(&session);
                                let child = tokio::task::spawn_blocking(move || {
                                    std::process::Command::new("python3")
                                        .arg(script)
                                        .arg("--socket")
                                        .arg(socket)
                                        .args(["--session", &id, "--profile", profile])
                                        .output()
                                });
                                let (mut stream, _) = listener.accept().await?;
                                let bytes = handle_control(&mut stream)
                                    .await
                                    .ok_or("Python request exceeded control bound")?;
                                assert_eq!(
                                    bytes,
                                    format!("qualify {} {profile}\n", uuid_text(&session))
                                        .as_bytes()
                                );
                                let allowed = cfg!(feature = "qualification-spawn")
                                    || !matches!(profile, "c05-exiva" | "item-6506-2");
                                let request = prepare_request(true, Some(()), &bytes);
                                if allowed {
                                    let (_, actual_session, actual_profile) = request
                                        .map_err(|_| "Python request refused by node parser")?;
                                    assert_eq!(actual_session, session);
                                    assert_eq!(actual_profile, profile);
                                    stream.write_all(b"qualified\n").await?;
                                } else {
                                    assert_eq!(request, Err(EarlyRefusal::MalformedRequest));
                                    stream.write_all(b"rejected\n").await?;
                                }
                                stream.shutdown().await?;
                                let output = child.await??;
                                if output.status.success() != allowed {
                                    return Err(format!(
                                        "Python client disagreed with node parser: {:?}",
                                        output.status
                                    )
                                    .into());
                                }
                            }
                            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                        })
                        .await?;
                        std::fs::remove_file(path)?;
                        result
                    })
            })?
            .join()
            .map_err(|_| "control parser test thread failed")?
    }

    #[test]
    fn refusal_warning_retains_error_kind_and_validated_context() {
        let session = [0x12; 16];
        let error: Box<dyn std::error::Error> = "qualification admission: Unavailable".into();
        let line = refusal_warning(&session, "knight", error.as_ref());
        assert_eq!(line.level, oteryn_error_codes::Level::Warn);
        let rendered = line.render("test", 0);
        assert!(rendered.contains("event=qualification_fixture_refused"));
        assert!(rendered.contains("reason=qualification_admission:_Unavailable"));
        assert!(rendered.contains("session=12121212-1212-1212-1212-121212121212"));
        assert!(rendered.contains("profile=knight"));
        let error: Box<dyn std::error::Error> =
            "qualification admission: SessionUseAuthorization(AuthorityRejected)".into();
        let rendered = refusal_warning(&session, "knight", error.as_ref()).render("test", 0);
        assert!(rendered.contains("SessionUseAuthorization_AuthorityRejected_"));
        assert!(!rendered.contains("redacted_sensitive_error"));
    }

    #[test]
    fn refusal_warning_bounds_reason_and_cannot_inject_a_log_line() {
        let error: Box<dyn std::error::Error> = "invalid\n\r\x1b[31m 世界 ".repeat(100).into();
        let rendered = refusal_warning(&[0x12; 16], "druid", error.as_ref()).render("test", 0);
        let reason = rendered
            .split("reason=")
            .nth(1)
            .and_then(|s| s.split_whitespace().next());
        assert!(reason.is_some_and(|reason| reason.chars().count() <= 200 && reason.is_ascii()));
        assert_eq!(rendered.lines().count(), 1);
        assert!(!rendered.contains('\r') && !rendered.contains('\x1b'));
    }

    #[test]
    fn refusal_warning_redacts_sensitive_error_text() {
        for detail in [
            "postgresql://user:sensitive@host/database",
            "password=sensitive",
            "Token: sensitive",
            "secret=sensitive",
            "Authorization: Bearer sensitive",
            "private key sensitive",
            "credential=sensitive",
            "seed=sensitive",
            "-----BEGIN sensitive",
        ] {
            let error: Box<dyn std::error::Error> = detail.into();
            let rendered =
                refusal_warning(&[0x12; 16], "paladin", error.as_ref()).render("test", 0);
            assert!(rendered.contains("reason=redacted_sensitive_error"));
            assert!(!rendered.contains("sensitive@") && !rendered.contains("=sensitive"));
        }
    }

    #[test]
    fn qualification_items_resolve_from_the_real_served_catalogue()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = crate::foundation::WorldId::decode(
            &super::super::super::uuid_bytes("01890f4c-3b2a-7c01-8d11-9a321b7c0004")
                .ok_or("fixture world")?,
        )?;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/spells.manifest.json");
        let room =
            crate::content::qualify_native_entry_room_from_gameplay_manifest(world, &manifest)?;
        let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
        let index = native.item_index().ok_or("Item index")?;
        for id in [3276, 3003, 3457, 39546, 43895, 6506, 11444, 3264] {
            let key = format!("oteryn:item.tibia.i{id}");
            let definition = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.clone(),
                revision_ref: index.revision(&key).ok_or("Item missing")?.into(),
            };
            let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
                &native,
                &definition,
            )
            .map_err(|error| format!("Item {id}: {error:?}"))?;
            assert_ne!(
                facts.stack,
                crate::durability::item_transfer::ItemStackClass::Unknown
            );
        }
        Ok(())
    }

    #[test]
    fn spawn_is_rejected_outside_qualification_mode() {
        let id = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
        let request = format!("spawn {id} oteryn:creature.rat\n");
        assert_eq!(
            spawn_request(false, request.as_bytes()).err(),
            Some("not_qualification")
        );
        assert_eq!(
            spawn_rejected("not_qualification"),
            "rejected reason=not_qualification"
        );
        // The same bytes are a valid request once the node runs in the fixture mode.
        assert!(matches!(
            spawn_request(true, request.as_bytes()),
            Ok((_, "oteryn:creature.rat"))
        ));
    }

    #[test]
    fn longest_valid_spawn_line_fits_the_control_request_bound() {
        let id = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
        let line = format!("spawn {id} {}\n", "k".repeat(128));
        assert_eq!(line.len(), CONTROL_REQUEST_BYTES);
        assert!(spawn_request(true, line.as_bytes()).is_ok());
    }

    #[test]
    fn spawn_request_accepts_only_v7_sessions_and_bounded_keys() {
        let id = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
        let long = "x".repeat(129);
        for text in [
            format!("spawn {id}"),
            format!("spawn {id} "),
            format!("spawn {id} bad key"),
            format!("spawn {id} key\nspawn {id} other"),
            format!("spawn {id} {long}"),
            format!("spawn {id} key;rm"),
            "spawn 01890f4c-3b2a-4c01-8d11-9a321b7c0004 oteryn:creature.rat".into(),
            format!("qualify {id} knight"),
        ] {
            assert_eq!(
                spawn_request(true, text.as_bytes()).err(),
                Some("malformed_request"),
                "{text:?}"
            );
        }
    }

    #[test]
    fn qualification_request_accepts_only_named_profiles_and_v7_sessions() {
        let id = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
        for profile in ["explorer", "knight", "paladin", "sorcerer", "druid", "monk"] {
            assert!(request(format!("qualify {id} {profile}\n").as_bytes()).is_some());
        }
        for profile in [
            "c05-exiva",
            "f5-c17a",
            "f5-c17c",
            "f5-c13",
            "f5-c18a",
            "f5-c17d",
            "c11-unassigned",
            "c11-knight",
            "c10-sorcerer",
            "c12-monk",
            "c06-sorcerer",
            "premium",
            "item-43895-1",
            "item-6506-2",
        ] {
            assert_eq!(
                request(format!("qualify {id} {profile}\n").as_bytes()).is_some(),
                cfg!(feature = "qualification-spawn")
            );
        }
        for text in [
            format!("qualify {id} master_sorcerer"),
            format!("qualify {id} levitate 13"),
            format!("qualify {id} magic-rope 10"),
            format!("qualify {id} c11-oressa-entry"),
            format!("qualify {id} c11-town-return"),
            format!("qualify {id} item-0-1"),
            format!("qualify {id} item-43895-0"),
            format!("qualify {id} item-43895-21"),
            format!("qualify {id} item-4294967296-1"),
            format!("qualify {id} item-43895-1-extra"),
            format!("qualify {id} knight extra"),
            format!("qualify {id} knight\nqualify {id} druid"),
            "qualify 01890f4c-3b2a-4c01-8d11-9a321b7c0004 knight".into(),
        ] {
            assert!(request(text.as_bytes()).is_none());
        }
    }
}
