//! Disposable node-boot build provisioning. The node retains its incarnation
//! proof; the root-only control socket supplies only an admitted session id.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::qualification_spell_character as qualification_build;
#[cfg(any(test, feature = "qualification-spawn"))]
use crate::qualification_wild_spawn as wild_spawn;

fn profile_build(profile: &str) -> (&str, u32) {
    match profile {
        "atelier" | "sweep" => ("knight", SWEEP_LEVEL),
        "explorer" => ("knight", 100),
        "knight-underlevel" => ("knight", 66),
        "victim" => (crate::durability::character_build::NO_VOCATION, 100),
        other => (other, 100),
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
pub(super) fn recovery_replay(enabled: bool, bytes: &[u8]) -> String {
    let parsed = || {
        let (session, epoch) = std::str::from_utf8(bytes)
            .ok()?
            .trim_ascii_end()
            .strip_prefix("recovery-replay ")?
            .split_once(' ')?;
        Some((parse_operation_id(session.as_bytes())?, epoch.parse().ok()?))
    };
    if enabled
        && let Some((session, epoch)) = parsed()
        && crate::durability::fresh_admission::qualification_replay::take(session, epoch)
    {
        "recovery-grant-consumed".into()
    } else {
        "rejected".into()
    }
}

pub(super) fn item_instance(enabled: bool, bytes: &[u8]) -> String {
    let resolve = || {
        if !enabled {
            return None;
        }
        let (session, handle) = std::str::from_utf8(bytes)
            .ok()?
            .trim_end_matches('\n')
            .strip_prefix("item-instance ")?
            .split_once(' ')?;
        let session = parse_operation_id(session.as_bytes())?;
        let handle = handle.parse().ok()?;
        crate::gameplay_transport::qualification_observation::instance(session, handle)
    };
    resolve().map_or_else(
        || "rejected".into(),
        |item| format!("instance {}", uuid_text(&item)),
    )
}

fn request(bytes: &[u8]) -> Option<([u8; 16], &str)> {
    let text = std::str::from_utf8(bytes).ok()?.trim_end_matches('\n');
    let (session, profile) = text.strip_prefix("qualify ")?.split_once(' ')?;
    let session = parse_operation_id(session.as_bytes())?;
    matches!(
        profile.strip_suffix("-reconcile").unwrap_or(profile),
        "explorer"
            | "knight"
            | "paladin"
            | "sorcerer"
            | "druid"
            | "monk"
            | "combat-progression"
            | "private-channel"
            | "coin-converters"
            | "rune-training"
            | "sweep"
            | "atelier"
            | "victim"
            | "knight-underlevel"
            | "c17a"
            | "c17c"
            | "c04"
            | "w22b-wealth"
            | "w22b-stamina"
    )
    .then_some((session, profile))
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

#[derive(Debug)]
struct FixtureStepFailure {
    stage: &'static str,
    reason: &'static str,
}
impl std::fmt::Display for FixtureStepFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stage={} reason={}", self.stage, self.reason)
    }
}
impl std::error::Error for FixtureStepFailure {}

fn fixture_failure_reply(profile: &str, error: &(dyn std::error::Error + 'static)) -> &'static str {
    if matches!(
        profile,
        "c17a-reconcile"
            | "c17c-reconcile"
            | "c04-reconcile"
            | "w22b-wealth-reconcile"
            | "w22b-stamina-reconcile"
    ) && error
        .downcast_ref::<FixtureStepFailure>()
        .is_some_and(|failure| {
            matches!(
                failure.reason,
                "commit_unknown"
                    | "post_commit_read_unavailable"
                    | "fixture_read_unavailable"
                    | "holder_busy"
                    | "pass_deadline"
            )
        })
    {
        "unavailable"
    } else {
        "rejected"
    }
}

pub(super) async fn prepare(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: Option<&NativeGameplayState>,
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
    match prepare_build(root, authority, proof, config, native, session, profile).await {
        Ok(()) => "qualified",
        Err(error) => {
            let reason = if error.to_string().contains("untouched disposable Character") {
                "character_not_initial"
            } else if error.to_string().contains("reconciliation") {
                "reconciliation_mismatch"
            } else {
                "refused"
            };
            event(&format!(
                "event=qualification_fixture_failure stage=guard reason={reason}"
            ));
            emit(PROCESS, &refusal_warning(&session, profile, error.as_ref()));
            fixture_failure_reply(profile, error.as_ref())
        }
    }
}

/// Observe the effective carried-weight limit from the same owner as inventory transfers.
pub(super) async fn carry_capacity(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    premium: Option<&crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator>,
    bytes: &[u8],
) -> String {
    let Some(session) = bytes
        .strip_prefix(b"capacity ")
        .and_then(parse_operation_id)
    else {
        return "rejected".into();
    };
    if !config.qualification_fixture {
        return "rejected".into();
    }
    match observe_carry_capacity(root, proof, config, premium, session).await {
        Ok((level, capacity)) => format!("capacity {level} {capacity}"),
        Err(_) => "rejected".into(),
    }
}

async fn observe_carry_capacity(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    premium: Option<&crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator>,
    session: [u8; 16],
) -> Result<(u32, u64), Box<dyn std::error::Error>> {
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::durability::item_transfer::{
        CurrentCharacterItemFence, read_qualification_carry_capacity,
    };
    use crate::durability::spell_item_transaction::begin_spell_owner_transaction;
    use crate::foundation::GameSessionId;

    let session = GameSessionId::decode(&session)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut holder = qualification_build::retry_holder(
        Some(root),
        deadline,
        |error| matches!(error, DurabilityError::RootUnavailable),
        async || root.try_acquire_ready(),
    )
    .await?;
    let mut tx = begin_spell_owner_transaction(&mut holder, deadline.into_std()).await?;
    let current = FreshAdmissionStore::from_root(root.clone())
        .current_session_in_transaction(&mut tx, session)
        .await?;
    let scope = current.current_runtime_scope();
    if Some(*scope.world_id().as_bytes()) != super::super::uuid_bytes(&config.scope.world_id)
        || scope.channel_id().map(|id| *id.as_bytes())
            != super::super::uuid_bytes(&config.scope.channel_id)
    {
        return Err("qualification capacity scope mismatch".into());
    }
    let lease = current.current_character_lease();
    let fence = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes())
            .map_err(|error| format!("qualification capacity character: {error:?}"))?,
        game_session_id: session,
        connection_generation: current.current_connection_generation(),
        character_lease_generation: lease.generation(),
        runtime_scope: scope,
        scope_ownership_generation: current.current_scope_generation(),
    };
    let observed = read_qualification_carry_capacity(&mut tx, proof, &fence, |account| {
        premium.is_some_and(|owner| owner.view(account).premium_gate().premium_current())
    })
    .await?;
    tx.commit().await?;
    Ok(observed)
}

/// The live Channel owner and the room the `spawn` command places creatures in.
/// Only the qualification-spawn build (and tests) reads the fields; other builds refuse unplaced.
#[cfg_attr(not(any(test, feature = "qualification-spawn")), allow(dead_code))]
#[derive(Clone, Copy)]
pub(super) struct SpawnOwners<'a> {
    pub(super) runtime: &'a Mutex<ChannelRuntimeV1>,
    pub(super) room: &'a crate::content::QualifiedNativeEntryRoom,
    #[cfg(any(test, feature = "qualification-spawn"))]
    pub(super) specimens: &'a Mutex<Vec<QualificationSpecimen>>,
}

#[cfg(any(test, feature = "qualification-spawn"))]
pub(super) struct QualificationSpecimen {
    session: [u8; 16],
    character: [u8; 16],
    actor: crate::foundation::ExactActorRef,
    key: String,
    content: [u8; 32],
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
/// as the harness Rat, and answers `spawned <entity identity hex> <generation> <actor local id>`.
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
    spawn_placed(root, authority, proof, native, owners, session, key, refuse).await
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
    if key == "cleanup" {
        let mut specimens = owners.specimens.lock().await;
        let mut runtime = owners.runtime.lock().await;
        return match cleanup_specimens(&mut runtime, &mut specimens, |specimen| {
            specimen.session == session
        }) {
            Ok(()) => "cleaned".into(),
            Err(error) => refuse(error.token(), error.detail()),
        };
    }
    let budget = SpawnBudget::new(tokio::time::Instant::now());
    let placed = spawn_with_watchdog(budget.reply_deadline, async |published| {
        place(
            root,
            authority,
            proof,
            native,
            owners,
            SpawnRequest {
                session,
                key,
                budget,
            },
            published,
        )
        .await
        .map(drop)
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
                "spawned {identity} {} {}",
                creature.actor.actor_local_generation(),
                creature.actor.actor_local_id()
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
struct SpawnRequest<'a> {
    session: [u8; 16],
    key: &'a str,
    budget: SpawnBudget,
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn place(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    native: &NativeGameplayState,
    owners: SpawnOwners<'_>,
    request: SpawnRequest<'_>,
    published: &mut Option<crate::foundation::CompanionSnapshot>,
) -> Result<crate::foundation::CompanionSnapshot, wild_spawn::SpawnRefusal> {
    let SpawnRequest {
        session,
        key,
        budget,
    } = request;
    let fence = wild_spawn::SpawnRefusal::Fence;
    let session = crate::foundation::GameSessionId::decode(&session)
        .map_err(|_| wild_spawn::SpawnRefusal::SessionUnknown)?;
    let deadline = budget.work_deadline;
    let mut specimens = owners.specimens.lock().await;
    retry_publication(
        owners.runtime,
        deadline,
        async || {
            let ((plan, character), (permit, mut holder)) = prepare_and_checkout(
                owners.runtime,
                |mut runtime| {
                    let (actor, _) = runtime
                        .positioned_player_for_session(session)
                        .map_err(|error| fence(format!("spawn player: {error:?}")))?
                        .ok_or(wild_spawn::SpawnRefusal::SessionUnknown)?;
                    let lease = runtime
                        .bound_attacker_lease(actor, session)
                        .map_err(|error| fence(format!("spawn lease: {error:?}")))?;
                    let character = *lease.character_id().as_bytes();
                    cleanup_specimens(&mut runtime, &mut specimens, |specimen| {
                        specimen.character == character
                    })?;
                    Ok((
                        wild_spawn::prepare_placement_adjacent(
                            owners.room,
                            native,
                            &mut runtime,
                            key,
                            session,
                        )?,
                        character,
                    ))
                },
                async |(plan, _)| {
                    let permit =
                        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
                            plan.scope,
                        )
                        .await
                        .map_err(|error| fence(format!("spawn lane: {error:?}")))?;
                    let holder = qualification_build::retry_holder(
                        Some(root),
                        deadline,
                        |error| matches!(error, DurabilityError::RootUnavailable),
                        async || root.try_acquire_ready(),
                    )
                    .await
                    .map_err(|error| fence(format!("spawn holder: {error:?}")))?;
                    Ok((permit, holder))
                },
            )
            .await?;
            let eligible = match spawn_transaction(&mut holder, budget, true, async |tx| {
                wild_spawn::validate_placement(tx, root, authority, proof, &permit, &plan).await
            })
            .await
            {
                SpawnTransaction::Clean(Ok(eligible)) => eligible,
                outcome => {
                    let result = finish_spawn_holder(holder, budget, outcome).await;
                    root.request_ready();
                    return result.map(|_| None);
                }
            };
            let result = retry_spawn_collisions(
                holder,
                budget,
                published,
                async || Ok(None),
                async |tx| {
                    let fresh =
                        wild_spawn::validate_placement(tx, root, authority, proof, &permit, &plan)
                            .await?;
                    let Ok(mut runtime) = owners.runtime.try_lock() else {
                        return Ok(None);
                    };
                    if tokio::time::Instant::now() >= budget.work_deadline {
                        return Err(wild_spawn::SpawnRefusal::Deadline);
                    }
                    let tiles = publication_candidates(fresh, eligible.clone());
                    let creature = wild_spawn::publish_placement(&mut runtime, key, &plan, &tiles)?;
                    specimens.push(QualificationSpecimen {
                        session: *session.as_bytes(),
                        character,
                        actor: creature.actor,
                        key: key.to_owned(),
                        content: runtime.content_pin().server_artifact_digest(),
                    });
                    Ok(Some(creature))
                },
            )
            .await;
            if result.is_err() {
                root.request_ready();
            }
            if let Some(creature) = published.clone() {
                return Ok(Some(creature));
            }
            result.map(|()| None)
        },
        || wild_spawn::SpawnRefusal::Deadline,
    )
    .await
}

#[cfg(any(test, feature = "qualification-spawn"))]
fn cleanup_specimens(
    runtime: &mut ChannelRuntimeV1,
    specimens: &mut Vec<QualificationSpecimen>,
    selected: impl Fn(&QualificationSpecimen) -> bool,
) -> Result<(), wild_spawn::SpawnRefusal> {
    let mut index = 0;
    while index < specimens.len() {
        let specimen = &specimens[index];
        if !selected(specimen) {
            index += 1;
            continue;
        }
        runtime
            .remove_native_qualification_once(specimen.actor, &specimen.key, specimen.content)
            .map_err(|error| {
                wild_spawn::SpawnRefusal::Fence(format!("specimen cleanup: {error:?}"))
            })?;
        specimens.remove(index);
    }
    Ok(())
}

#[cfg(any(test, feature = "qualification-spawn"))]
#[cfg(test)]
fn classify_spawn_publication<T>(
    published: Option<T>,
    outcome: Result<T, wild_spawn::SpawnRefusal>,
) -> Result<T, wild_spawn::SpawnRefusal> {
    match published {
        Some(creature) => Ok(creature),
        None => outcome.and_then(|_| {
            Err(wild_spawn::SpawnRefusal::Fence(
                "no publication witness".into(),
            ))
        }),
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn retry_publication<T, R, E>(
    runtime: &Mutex<T>,
    deadline: tokio::time::Instant,
    mut attempt: impl AsyncFnMut() -> Result<Option<R>, E>,
    expired: impl Fn() -> E,
) -> Result<R, E> {
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(expired());
        }
        if let Some(published) = attempt().await? {
            return Ok(published);
        }
        // The attempt owns and drops every transaction/holder before it returns Busy.
        // Wait without either, then rebuild and revalidate the complete placement plan.
        let available = tokio::time::timeout_at(deadline, runtime.lock())
            .await
            .map_err(|_| expired())?;
        drop(available);
    }
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn prepare_and_checkout<'a, T, P, H, E>(
    runtime: &'a Mutex<T>,
    prepare: impl FnOnce(tokio::sync::MutexGuard<'a, T>) -> Result<P, E>,
    checkout: impl AsyncFnOnce(&P) -> Result<H, E>,
) -> Result<(P, H), E> {
    let prepared = prepare(runtime.lock().await)?;
    let holder = checkout(&prepared).await?;
    Ok((prepared, holder))
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
        (backend_deadline - setup).into_std(),
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
        Option<sqlx::pool::PoolConnection<sqlx::Postgres>>,
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
        let Some(next) = reacquire().await? else {
            return Ok(());
        };
        holder = next;
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

/// Source level of the `sweep` profile; the served progression table has 2000 levels.
const SWEEP_LEVEL: u32 = 1000;
/// Ruthless axe (6553, attack 50, level 75, knight) joins the common items for `sweep`.
const SWEEP_ITEM_IDS: [u32; 4] = [3276, 3003, 3457, 6553];

// The Character, named profile and stage identify one retryable control workflow.
fn fixture_occurrence(character: &[u8; 16], profile: &str, stage: u8) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:qualification-control-profile:v1")
        .chain_update(character)
        .chain_update([stage])
        .chain_update(profile.as_bytes())
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    occurrence[6] = (occurrence[6] & 0x0f) | 0x70;
    occurrence[8] = (occurrence[8] & 0x3f) | 0x80;
    occurrence
}

async fn prepare_build(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: &NativeGameplayState,
    session: [u8; 16],
    profile: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let reconcile = profile.ends_with("-reconcile");
    let profile = profile.strip_suffix("-reconcile").unwrap_or(profile);
    if reconcile
        && !matches!(
            profile,
            "c17a" | "c17c" | "c04" | "w22b-wealth" | "w22b-stamina"
        )
    {
        return Err("reconciliation is spell-fixture only".into());
    }
    let session = crate::foundation::GameSessionId::decode(&session)?;
    // Independently resolve current admission, never synthesize a fence from
    // fixture SQL or expose the node proof to the client process.
    let budget = tokio::time::Instant::now() + Duration::from_secs(15);
    let admissions =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone());
    let current = fixture_step(root, budget, "admission", async || {
        admissions.current_session(session).await
    })
    .await?;
    let scope = current.current_runtime_scope();
    if Some(*scope.world_id().as_bytes()) != super::super::uuid_bytes(&config.scope.world_id)
        || scope.channel_id().map(|id| *id.as_bytes())
            != super::super::uuid_bytes(&config.scope.channel_id)
    {
        return Err("qualification scope mismatch".into());
    }
    if profile == "coin-converters" {
        return prepare_items(
            root,
            authority,
            proof,
            &current,
            native,
            budget,
            &[25719, 28525],
        )
        .await;
    }
    if profile == "private-channel" {
        #[cfg(feature = "qualification-spawn")]
        if current.session_state() == crate::foundation::GameSessionState::Active
            && crate::gameplay_transport::prepare_channel_owner(session)
        {
            return Ok(());
        }
        return Err("private-channel fixture requires an active qualification session".into());
    }
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )
    .map_err(|error| format!("qualification character: {error:?}"))?;
    let interpretation = fixture_step(root, budget, "interpretation", async || {
        root.read_character_root_interpretation(authority, character)
            .await
    })
    .await?
    .ok_or("qualification interpretation unavailable")?;
    let [profile_revision, ruleset_revision, content_revision, _] = interpretation.revisions();
    if profile == "rune-training" {
        return qualification_build::prepare_rune_magic(
            root,
            authority,
            proof,
            &current,
            content_revision,
            uuid_v7()?,
            budget,
        )
        .await;
    }
    let progression = native
        .progression()
        .ok_or("qualification progression unavailable")?;
    let policy = progression.policy_for(profile_revision, ruleset_revision, content_revision)?;
    let context = policy.context.clone();
    if profile == "w22b-wealth" {
        return prepare_items(root, authority, proof, &current, native, budget, &[36727]).await;
    }
    if profile == "w22b-stamina" {
        return prepare_exhausted_hunting_fixture(
            root, authority, proof, &current, &context, budget,
        )
        .await;
    }
    if matches!(profile, "c17a" | "c17c" | "c04") {
        let (vocation, level) = {
            let words = match profile {
                "c17a" => "utani hur",
                "c04" => "exevo vis lux",
                _ => "utevo res ina",
            };
            let spell = native
                .spell_book()
                .spoken(words)
                .ok_or("qualification spell absent")?
                .spell;
            let (vocation, required) = ("sorcerer", crate::spell::Vocation::Sorcerer);
            if !spell.vocations.contains(&required) {
                return Err("qualification spell vocation does not match source".into());
            }
            let level = if profile == "c17a" {
                let strong = native
                    .spell_book()
                    .spoken("utani gran hur")
                    .ok_or("Strong Haste source absent")?
                    .spell;
                if !strong.vocations.contains(&required) {
                    return Err("Strong Haste source vocation differs".into());
                }
                spell.level.max(strong.level)
            } else {
                spell.level
            };
            (vocation, level)
        };
        let spec = qualification_build::QualificationBuildSpec {
            vocation,
            level,
            context: context.clone(),
            xp_occurrence: if reconcile {
                spell_occurrence(character.as_bytes(), profile, level, &context, "experience")
            } else {
                uuid_v7()?
            },
            build_occurrence: if reconcile {
                spell_occurrence(character.as_bytes(), profile, level, &context, "build")
            } else {
                uuid_v7()?
            },
        };
        return prepare_admitted_spell_character(
            root,
            authority,
            proof,
            &current,
            spec,
            policy,
            SpellOptions { budget, reconcile },
        )
        .await;
    }
    let combat_progression = profile == "combat-progression";
    let (vocation, level) = if combat_progression {
        let book = native.spell_book();
        let unlock = book
            .spells
            .iter()
            .enumerate()
            .filter(|(index, spell)| {
                !book.inactive_aliases.contains(index)
                    && !spell.learning_required
                    && spell.level > 1
                    && spell.vocations.contains(&crate::spell::Vocation::Sorcerer)
            })
            .map(|(_, spell)| spell.level)
            .min()
            .ok_or("qualification progression fixture has no authored spell unlock")?;
        ("sorcerer", unlock - 1)
    } else {
        profile_build(profile)
    };
    let (xp_occurrence, build_occurrence) =
        if matches!(profile, "sweep" | "victim" | "knight-underlevel") {
            (
                fixture_occurrence(character.as_bytes(), profile, 1),
                fixture_occurrence(character.as_bytes(), profile, 2),
            )
        } else {
            qualification_build::fixture_occurrences(character, vocation, level, combat_progression)
        };
    qualification_build::prepare_character_with_policy_until(
        root,
        authority,
        proof,
        &current,
        qualification_build::QualificationBuildSpec {
            vocation,
            level,
            context,
            xp_occurrence,
            build_occurrence,
        },
        policy,
        combat_progression,
        budget,
    )
    .await?;
    if !combat_progression {
        let tools = [3276, 3003, 3457, 3147];
        let items = if profile == "sweep" {
            SWEEP_ITEM_IDS.as_slice()
        } else if matches!(vocation, "paladin" | "monk") {
            tools.as_slice()
        } else {
            &tools[..3]
        };
        prepare_items(root, authority, proof, &current, native, budget, items).await?;
    }
    Ok(())
}

struct SpellOptions {
    budget: tokio::time::Instant,
    reconcile: bool,
}

fn spell_occurrence(
    character: &[u8; 16],
    profile: &str,
    level: u32,
    context: &crate::domain::progression::ProgressionRevisionContext<String>,
    step: &str,
) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn:qualification:spell-build:v1");
    hash.update(character);
    for value in [profile, &level.to_string(), &format!("{context:?}"), step] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    let digest = hash.finalize();
    let mut id = *character;
    id[6..].copy_from_slice(&digest[..10]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

#[cfg(test)]
std::thread_local! { static POST_COMMIT_READ_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

// The spell rows use a fully admitted Character: admission has already bound
// level-1 progression. Retain the one-shot contract for legacy calls and only
// reconcile the explicit spell request through its own source-bound receipts.
// Neither route resets a Character or creates a commercial entitlement.
async fn prepare_admitted_spell_character<const N: usize>(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    spec: qualification_build::QualificationBuildSpec<'_>,
    policy: crate::domain::progression::FiniteProgressionPolicy<String, N>,
    options: SpellOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, NO_VOCATION, convert_vocation,
    };
    use crate::durability::character_progression::{
        CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
        ProgressionInitializationRequest,
    };
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    let SpellOptions { budget, reconcile } = options;
    if !matches!(spec.vocation, "sorcerer" | "knight")
        || !(1..=1000).contains(&spec.level)
        || policy.context != spec.context
    {
        return Err("C17 requires its source free Sorcerer build and policy".into());
    }
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )
    .map_err(fixture_error)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(character).await;
    let before = fixture_step(root, budget, "build_read", async || {
        root.read_character_build_state(authority, character).await
    })
    .await?;
    let progression = fixture_step(root, budget, "progression_read", async || {
        root.read_character_progression(authority, character).await
    })
    .await?;
    let experience = policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == spec.level)
        .ok_or("C17 spell level absent from served policy")?
        .minimum_experience;
    let xp_occurrence =
        ExperienceRewardOccurrence::from_bytes(spec.xp_occurrence).map_err(fixture_error)?;
    let build_occurrence =
        BuildOccurrence::from_bytes(spec.build_occurrence).map_err(fixture_error)?;
    let (xp, build) = if reconcile {
        let xp = fixture_step(root, budget, "experience_reconcile", async || {
            root.reconcile_character_experience(authority, xp_occurrence)
                .await
        })
        .await?;
        let build = fixture_step(root, budget, "build_reconcile", async || {
            root.reconcile_character_build(authority, build_occurrence)
                .await
        })
        .await?;
        (xp, build)
    } else {
        (None, None)
    };
    // Receipts prove only our own committed steps. A missing step still needs
    // independently current admission, lease, scope and revision fences below.
    if let Some(receipt) = &xp {
        if receipt.character_id != character
            || receipt.level_before != 1
            || receipt.experience_before.get() != 0
            || receipt.level_after != spec.level
            || receipt.experience_awarded != experience
            || receipt.experience_after != experience
            || !progression.as_ref().is_some_and(|state| {
                state.level == spec.level
                    && state.total_experience == experience
                    && state.context == policy.context
                    && state.policy_revision == policy.policy_revision
                    && state.reward_revision == policy.reward_revision
            })
        {
            return Err(
                "C17 reconciliation experience does not match source and current state".into(),
            );
        }
    } else if progression
        .as_ref()
        .is_some_and(|state| state.level != 1 || state.total_experience.get() != 0)
    {
        return Err("C17 spell fixture requires an untouched disposable Character".into());
    }
    if let Some(receipt) = build {
        let Some(xp) = &xp else {
            return Err("C17 build receipt has no fixture experience".into());
        };
        if receipt.character_id != character
            || receipt.cause != BuildCause::VocationChoice
            || receipt.before != crate::durability::character_build::DurableBuildState::default()
            || receipt.after != before
            || receipt.after.vocation() != spec.vocation
            || receipt.original_character_revision < xp.committed_character_revision
        {
            return Err("C17 reconciliation build does not match source and current state".into());
        }
        return Ok(());
    }
    if before.vocation() != NO_VOCATION
        || before != crate::durability::character_build::DurableBuildState::default()
    {
        return Err("C17 spell fixture requires an untouched disposable Character".into());
    }
    let mut fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: fixture_step(root, budget, "revision_read", async || {
            root.read_current_character(authority, character).await
        })
        .await?
        .revision,
    };
    // The initializer reconciles an existing admission row only when every
    // context/policy revision matches. A mismatched policy cannot reach XP.
    let initialization = ProgressionInitializationRequest {
        context: spec.context.clone(),
        policy_revision: policy.policy_revision.clone(),
        reward_revision: policy.reward_revision.clone(),
        policy: policy.clone(),
    };
    fixture_step(root, budget, "initialize", async || {
        root.initialize_character_progression(authority, proof, fence, initialization.clone())
            .await
    })
    .await?;
    if spec.level > 1 && xp.is_none() {
        let request = ExperienceAwardRequest {
            occurrence: ExperienceRewardOccurrence::from_bytes(spec.xp_occurrence)
                .map_err(fixture_error)?,
            amount: experience,
            context: spec.context.clone(),
            policy_revision: policy.policy_revision.clone(),
            reward_revision: policy.reward_revision.clone(),
            policy,
        };
        fixture_step(root, budget, "experience", async || {
            slot.commit_experience(root, authority, proof, fence, request.clone(), None)
                .await
        })
        .await?;
    }
    fence.expected_character_revision = fixture_step(root, budget, "revision_read", async || {
        root.read_current_character(authority, character).await
    })
    .await?
    .revision;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/content-schema/native-gameplay/build-training.json"
        )),
        &spec.context.content,
    )
    .map_err(fixture_error)?;
    let after = convert_vocation(&formula, &before, spec.vocation).map_err(fixture_error)?;
    let request = BuildChangeRequest {
        occurrence: BuildOccurrence::from_bytes(spec.build_occurrence).map_err(fixture_error)?,
        cause: BuildCause::VocationChoice,
        before,
        after,
        pruned_stance: None,
    };
    fixture_step(root, budget, "build", async || {
        slot.commit_build(
            root,
            authority,
            proof,
            fence,
            request.clone(),
            &formula,
            None,
        )
        .await
    })
    .await?;
    let facts =
        crate::gameplay_transport::load_character_cast_facts(root, authority, character).await;
    #[cfg(test)]
    let facts = if POST_COMMIT_READ_FAILURE.with(|fail| fail.replace(false)) {
        crate::gameplay_transport::CastFactsLoad::Unavailable
    } else {
        facts
    };
    if !matches!(facts,
        crate::gameplay_transport::CastFactsLoad::Ready { facts, .. }
        if facts.level == spec.level && facts.vocation == if spec.vocation == "knight" { crate::spell::Vocation::Knight } else { crate::spell::Vocation::Sorcerer })
    {
        let reason = if matches!(facts, crate::gameplay_transport::CastFactsLoad::Unavailable) {
            "post_commit_read_unavailable"
        } else {
            "source_build_mismatch"
        };
        event(&format!(
            "event=qualification_fixture_failure stage=cast_facts reason={reason}"
        ));
        return Err(Box::new(FixtureStepFailure {
            stage: "cast_facts",
            reason,
        }));
    }
    Ok(())
}

// Retry only a busy holder, which refuses before SQL. Never replay the whole
// one-shot fixture: XP/build occurrence identities and completed steps stay fixed.
async fn fixture_step<T, E: FixtureError>(
    root: &DurabilityRoot,
    budget: tokio::time::Instant,
    stage: &'static str,
    step: impl AsyncFnMut() -> Result<T, E>,
) -> Result<T, Box<dyn std::error::Error>> {
    retry_busy(
        Some(root),
        budget,
        |error: &E| error.reason() == "holder_busy",
        step,
    )
    .await
    .map_err(|error| {
        event(&format!(
            "event=qualification_fixture_failure stage={stage} reason={}",
            error.reason()
        ));
        Box::new(FixtureStepFailure {
            stage,
            reason: error.reason(),
        }) as Box<dyn std::error::Error>
    })
}

// Only closed tokens reach stderr; never print an error's database/TLS detail.
trait FixtureError: std::fmt::Debug {
    fn reason(&self) -> &'static str;
}

impl FixtureError for DurabilityError {
    fn reason(&self) -> &'static str {
        match self {
            Self::RootUnavailable => "holder_busy",
            Self::RootPassDeadlineExceeded => "pass_deadline",
            Self::CommitOutcomeUnknown => "commit_unknown",
            Self::Database(_) => "database",
            Self::InvalidStoredState => "invalid_stored_state",
            _ => "durability",
        }
    }
}

impl FixtureError for crate::durability::reward_claim_mint::RewardClaimMintError {
    fn reason(&self) -> &'static str {
        match self {
            Self::Unavailable(error) => error.reason(),
            _ => "item_refused",
        }
    }
}

impl FixtureError for CharacterAuthorityError {
    fn reason(&self) -> &'static str {
        match self {
            Self::Unavailable(error) => error.reason(),
            _ => "authority_refused",
        }
    }
}

impl FixtureError for crate::durability::character_progression::CharacterProgressionError {
    fn reason(&self) -> &'static str {
        match self {
            Self::Unavailable(error) => error.reason(),
            Self::CharacterRevisionMismatch => "revision_mismatch",
            Self::AuthorityRejected => "authority_refused",
            Self::BuildStateMismatch => "build_mismatch",
            Self::ProgressionContextMismatch => "context_mismatch",
            Self::ConflictingOccurrence => "occurrence_conflict",
            _ => "progression_refused",
        }
    }
}

fn fixture_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

// Setup only: the row still needs a real kill and the native loot suppression.
async fn prepare_exhausted_hunting_fixture(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    context: &crate::domain::progression::ProgressionRevisionContext<String>,
    budget: tokio::time::Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::character_build::BuildOccurrence;
    use crate::durability::character_progression::CurrentCharacterGameplayFence;
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    use oteryn_gameplay_characters::stamina::{MAX_STAMINA_MINUTES, hunting_loot_allowed};
    let boundary = (0..=u16::try_from(MAX_STAMINA_MINUTES)?)
        .take_while(|minutes| !hunting_loot_allowed(*minutes))
        .last()
        .ok_or("source exhaustion boundary absent")?;
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )?;
    let sequencer = CharacterRevisionSequencer::new();
    let _slot = sequencer.acquire(character).await;
    let before = fixture_step(root, budget, "stamina_read", async || {
        root.read_hunting_stamina(authority, character).await
    })
    .await?;
    if !hunting_loot_allowed(before) {
        return Ok(());
    }
    let revision = fixture_step(root, budget, "revision_read", async || {
        root.read_current_character(authority, character).await
    })
    .await?
    .revision;
    let fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: revision,
    };
    let occurrence = BuildOccurrence::from_bytes(spell_occurrence(
        character.as_bytes(),
        "w22b-stamina",
        u32::from(boundary),
        context,
        "fixture-exhaustion",
    ))?;
    fixture_step(root, budget, "stamina", async || {
        root.commit_hunting_stamina(authority, proof, fence, occurrence, (before, boundary))
            .await
    })
    .await?;
    Ok(())
}

// Fixture claims use the ordinary fenced/audited reward owner, never SQL item
// inserts. They are setup evidence only, not proof of player chest interaction.
fn fixture_item_claim_key(id: u32, session: &[u8; 16]) -> String {
    if matches!(id, 25719 | 28525) {
        format!("oteryn:qualification.item.i{id}.{}", uuid_text(session))
    } else {
        format!("oteryn:qualification.item.i{id}")
    }
}

async fn prepare_items(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    native: &NativeGameplayState,
    deadline: tokio::time::Instant,
    items: &[u32],
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::CurrentCharacterItemFence;
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
        .map_err(|error| {
            Box::new(FixtureStepFailure {
                stage: "inventory_read",
                reason: if matches!(
                    error,
                    crate::durability::item_use::ItemUseError::Unavailable(_)
                ) {
                    "fixture_read_unavailable"
                } else {
                    "inventory_refused"
                },
            }) as Box<dyn std::error::Error>
        })
    };
    let mut inventory = read_inventory().await?;
    if inventory.backpack.is_none() {
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
    for (ordinal, &id) in items.iter().enumerate() {
        if matches!(id, 3276 | 3003 | 3457)
            && qualification_build::retry_holder(
                Some(root),
                deadline,
                fixture_item_holder_busy,
                async || {
                    root.qualification_fixture_item_is_claimed(authority, proof, fence, id)
                        .await
                },
            )
            .await
            .map_err(|error| format!("qualification Item claim lookup: {error:?}"))?
        {
            continue;
        }
        let key = format!("oteryn:item.tibia.i{id}");
        if matches!(id, 25719 | 28525)
            && bag.entries.iter().any(|entry| {
                entry.item.definition.production_key == key
                    || (id == 28525
                        && entry.item.definition.production_key == "oteryn:item.tibia.i28526")
            })
        {
            continue;
        }
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.clone(),
            revision_ref: index
                .revision(&key)
                .ok_or("qualification Item absent")?
                .into(),
        };
        if inventory
            .equipment
            .iter()
            .chain(bag.entries.iter().map(|entry| &entry.item))
            .any(|existing| {
                existing.definition == definition
                    && (existing.quantity == 1 || (id == 36727 && existing.quantity >= 1))
            })
        {
            continue;
        }
        let item = generation_item_facts(native, &definition)
            .map_err(|error| format!("qualification Item {key}: {error:?}"))?;
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            additional_items: Vec::new(),
            // Reserved fixture ids cannot collide with the client's low command ids.
            command: crate::foundation::CommandRef::new(
                current.current_game_session_id(),
                crate::foundation::CommandId::new(u64::MAX - ordinal as u64)?,
            ),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: fixture_item_claim_key(
                    id,
                    current.current_game_session_id().as_bytes(),
                ),
                revision_ref: "qualification-v1".into(),
            },
            source_placement: "oteryn:qualification.node-boot".into(),
            item,
            quantity: if id == 3147 {
                u32::from(crate::QUALIFICATION_MAP_SAMPLES_PER_KIND) + 1
            } else {
                1
            },
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
        let candidate = qualification_build::retry_holder(
            Some(root),
            deadline,
            fixture_item_holder_busy,
            async || {
                root.freeze_reward_claim_mint(authority, proof, fence, request.clone())
                    .await
            },
        )
        .await;
        let mut candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) if previous_fixture_item_committed(&error) => continue,
            Err(error) => {
                return Err(Box::new(FixtureStepFailure {
                    stage: "item_reserve",
                    reason: error.reason(),
                }));
            }
        };
        // A frozen, already committed reservation needs no further work unit.
        if read_fixture_item_receipt(Some(root), deadline, async || {
            root.read_committed_reward_claim_mint(authority, &candidate)
                .await
        })
        .await
        .map_err(|error| FixtureStepFailure {
            stage: "item_receipt",
            reason: error.reason(),
        })?
        .is_some()
        {
            continue;
        }
        finish_fixture_item(
            root.commit_reward_claim_mint_waiting(
                authority,
                proof,
                fence,
                &mut candidate,
                deadline,
            )
            .await,
        )?;
    }
    Ok(())
}

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

#[cfg(test)]
fn fixture_item_committed(
    outcome: crate::durability::reward_claim_mint::RewardClaimMintOutcome,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::reward_claim_mint::RewardClaimMintOutcome;
    let receipt = match outcome {
        RewardClaimMintOutcome::Committed(receipt)
        | RewardClaimMintOutcome::AlreadyCommitted(receipt) => receipt,
    };
    if receipt.quantity != 1 {
        return Err(format!(
            "qualification Item receipt has quantity {}",
            receipt.quantity
        )
        .into());
    }
    Ok(())
}

fn finish_fixture_item(
    result: Result<
        crate::durability::reward_claim_mint::RewardClaimMintOutcome,
        crate::durability::reward_claim_mint::RewardClaimMintError,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::reward_claim_mint::{
        RewardClaimMintError, RewardClaimMintOutcome, RewardClaimRefusal,
    };
    match result {
        Ok(RewardClaimMintOutcome::Committed(_) | RewardClaimMintOutcome::AlreadyCommitted(_))
        | Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)) => Ok(()),
        Err(error) => Err(Box::new(FixtureStepFailure {
            stage: "item_commit",
            reason: error.reason(),
        })),
    }
}

fn fixture_item_holder_busy(
    error: &crate::durability::reward_claim_mint::RewardClaimMintError,
) -> bool {
    matches!(
        error,
        crate::durability::reward_claim_mint::RewardClaimMintError::Unavailable(
            DurabilityError::RootUnavailable
        )
    )
}

async fn read_fixture_item_receipt<T>(
    root: Option<&DurabilityRoot>,
    deadline: tokio::time::Instant,
    read: impl AsyncFnMut() -> Result<T, crate::durability::reward_claim_mint::RewardClaimMintError>,
) -> Result<T, crate::durability::reward_claim_mint::RewardClaimMintError> {
    qualification_build::retry_holder(root, deadline, fixture_item_holder_busy, read).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r3_postgres_cases_are_only_selected_in_the_explicit_lane()
    -> crate::bestiary_postgres_harness::TestResult {
        let output = std::process::Command::new(std::env::current_exe()?)
            .arg("--list")
            .output()?;
        assert!(output.status.success());
        let listed = String::from_utf8(output.stdout)?;
        for name in [
            "r2_c04_has_a_real_admitted_sorcerer_fixture",
            "r3_post_commit_unavailable_replays_through_the_actual_prepare_handler",
            "r3_w22_wealth_fixture_mints_real_custody_once",
            "r3_w22_stamina_fixture_commits_the_native_exhaustion_boundary_once",
            "qualification_c17_profiles_accept_admission_initialized_progression",
            "qualification_c17_profiles_wait_for_a_busy_holder_without_reapplying",
            "qualification_c17_reconciles_partial_and_lost_ack_builds",
        ] {
            assert_eq!(
                listed.contains(&format!("::{name}: test")),
                cfg!(feature = "test-support"),
                "PostgreSQL case selected in the wrong lane: {name}"
            );
        }
        Ok(())
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn r3_w22_wealth_fixture_mints_real_custody_once()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                admitted_profiles_with_setup(false, false, &[("c17c", 23)], Some("w22b-wealth"))
                    .map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "wealth test panicked")?
            .map_err(Into::into)
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn r3_w22_stamina_fixture_commits_the_native_exhaustion_boundary_once()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                admitted_profiles_with_setup(false, false, &[("c17c", 23)], Some("w22b-stamina"))
                    .map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "stamina test panicked")?
            .map_err(Into::into)
    }

    // Real PostgreSQL cases belong to the explicit test-support lane, not lib-rest.
    #[cfg(feature = "test-support")]
    #[test]
    fn r3_post_commit_unavailable_replays_through_the_actual_prepare_handler()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| admitted_profiles(false, true, &[("c04", 23)]).map_err(|e| e.to_string()))?
            .join()
            .map_err(|_| "fixture test panicked")?
            .map_err(Into::into)
    }

    #[test]
    fn r2_unknown_fixture_commit_requires_reconciliation_not_refusal() {
        for stage in ["experience", "build"] {
            let failure = FixtureStepFailure {
                stage,
                reason: DurabilityError::CommitOutcomeUnknown.reason(),
            };
            assert_eq!(
                fixture_failure_reply("c17a-reconcile", &failure),
                "unavailable"
            );
            assert_eq!(fixture_failure_reply("c17a", &failure), "rejected");
            let semantic = FixtureStepFailure {
                stage,
                reason: "authority",
            };
            assert_eq!(
                fixture_failure_reply("c17a-reconcile", &semantic),
                "rejected"
            );
        }
    }

    // Real PostgreSQL cases belong to the explicit test-support lane, not lib-rest.
    #[cfg(feature = "test-support")]
    #[test]
    fn r2_c04_has_a_real_admitted_sorcerer_fixture() -> crate::bestiary_postgres_harness::TestResult
    {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                admitted_profiles(false, true, &[("c04", 23)]).map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "C04 provisioning test panicked")?
            .map_err(Into::into)
    }

    // Real PostgreSQL cases belong to the explicit test-support lane, not lib-rest.
    #[cfg(feature = "test-support")]
    #[test]
    fn qualification_c17_profiles_accept_admission_initialized_progression()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| c17_admitted_profiles(false, false).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "C17 provisioning test panicked")?
            .map_err(Into::into)
    }

    // Real PostgreSQL cases belong to the explicit test-support lane, not lib-rest.
    #[cfg(feature = "test-support")]
    #[test]
    fn qualification_c17_profiles_wait_for_a_busy_holder_without_reapplying()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| c17_admitted_profiles(true, false).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "C17 busy-holder test panicked")?
            .map_err(Into::into)
    }

    // Real PostgreSQL cases belong to the explicit test-support lane, not lib-rest.
    #[cfg(feature = "test-support")]
    #[test]
    fn qualification_c17_reconciles_partial_and_lost_ack_builds()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| c17_admitted_profiles(false, true).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "C17 recovery test panicked")?
            .map_err(Into::into)
    }

    #[cfg(feature = "test-support")]
    fn c17_admitted_profiles(
        busy: bool,
        reconcile: bool,
    ) -> crate::bestiary_postgres_harness::TestResult {
        admitted_profiles(busy, reconcile, &[("c17a", 20), ("c17c", 23)])
    }

    #[cfg(feature = "test-support")]
    fn admitted_profiles(
        busy: bool,
        reconcile: bool,
        profiles: &[(&str, u32)],
    ) -> crate::bestiary_postgres_harness::TestResult {
        admitted_profiles_with_setup(busy, reconcile, profiles, None)
    }

    #[cfg(feature = "test-support")]
    struct SourceItems<'a>(&'a NativeGameplayState);
    #[cfg(feature = "test-support")]
    impl crate::durability::item_transfer::ItemWeightLookup for SourceItems<'_> {
        fn generation_digest(&self) -> [u8; 32] {
            self.0.source_digest()
        }
        fn weight_of(
            &self,
            definition: &crate::durability::item_mint::TypedDefinitionRef,
        ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
            use crate::durability::item_transfer::{ItemStackClass, ItemWeightRefused};
            let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
                self.0, definition,
            )
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?;
            if facts.stack == ItemStackClass::Unknown {
                return Err(ItemWeightRefused::DefinitionMismatch);
            }
            facts.weight.ok_or(ItemWeightRefused::Unknown)
        }
    }
    #[cfg(feature = "test-support")]
    impl crate::durability::item_loop_move::ItemLoopContent for SourceItems<'_> {
        fn loop_content_digest(&self) -> [u8; 32] {
            self.0.source_digest()
        }
        fn loop_facts(
            &self,
            definition: &crate::durability::item_mint::TypedDefinitionRef,
        ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
            oteryn_gameplay_items::item_ref_admission::generation_item_facts(self.0, definition)
                .ok()
        }
    }

    #[cfg(feature = "test-support")]
    fn admitted_profiles_with_setup(
        busy: bool,
        reconcile: bool,
        profiles: &[(&str, u32)],
        setup: Option<&str>,
    ) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{self as fixture, Harness, debug};
        let admin = fixture::configured_admin().ok_or(
            "qualification fixture PostgreSQL tests require OTERYN_TEST_POSTGRES_ADMIN_URL",
        )?;
        fixture::runtime()?.block_on(async {
            let world = crate::foundation::WorldId::decode(&fixture::id(fixture::WORLD))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/spells.manifest.json");
            let room = crate::content::qualify_native_entry_room_from_gameplay_manifest(world, &manifest)?;
            let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
            for &(profile, level) in profiles {
                static NEXT_CASE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let tag = format!("qualification_{}", NEXT_CASE.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
                let h = Harness::create_with_admission_seed(admin.clone(), &tag, false, false).await?;
                admit_fixture(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut config = NodeConfig::parse(super::super::super::config::tests::NODE.as_bytes())?;
                config.qualification_fixture = true;
                config.scope.world_id = uuid_text(&fixture::id(fixture::WORLD));
                config.scope.channel_id = uuid_text(&fixture::id(fixture::CHANNEL));
                let policy = native.progression().ok_or("C17 progression absent")?
                    .policy_for("profile-1", "ruleset-1", "content-1")?;
                // Normal admission binds this exact level-1, zero-XP row before
                // the control-socket fixture can resolve the admitted session.
                h.root.initialize_character_progression(&authority, &h.node, {
                    let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(crate::foundation::GameSessionId::decode(&fixture::id(fixture::SESSION))?).await?;
                    let mut fence = fixture::fence(1)?;
                    fence.character_lease_generation = current.current_character_lease().generation();
                    fence
                },
                    crate::durability::character_progression::ProgressionInitializationRequest {
                        context: policy.context.clone(), policy_revision: policy.policy_revision.clone(),
                        reward_revision: policy.reward_revision.clone(), policy,
                    }).await.map_err(debug)?;
                if let Some(setup) = setup {
                    let character = crate::domain::CharacterId::from_bytes(fixture::id(fixture::CHARACTER))?;
                    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(level, 1, "W22 setup test must start from the admitted starter");
                    if setup == "w22b-wealth" {
                        let key = "oteryn:item.tibia.i2854";
                        let revision = native.item_index().and_then(|index| index.revision(key)).ok_or("source starter bag absent")?;
                        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING").bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
                        assert!(h.root.record_content_activation(&crate::durability::content_activation::ContentActivationRequest {
                            world_id: crate::foundation::WorldId::decode(&fixture::id(fixture::WORLD))?,
                            channel_id: crate::foundation::ChannelId::decode(&fixture::id(fixture::CHANNEL))?,
                            activation_sequence: 1, previous_sequence: None,
                            server_artifact_digest: native.source_digest(), client_artifact_digest: native.source_digest(), frame_binding_digest: native.source_digest(),
                        }).await.map_err(debug)?);
                        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session(crate::foundation::GameSessionId::decode(&fixture::id(fixture::SESSION))?).await?;
                        let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
                            character_id: character, game_session_id: current.current_game_session_id(), connection_generation: current.current_connection_generation(),
                            character_lease_generation: current.current_character_lease().generation(), runtime_scope: current.current_runtime_scope(), scope_ownership_generation: current.current_scope_generation(),
                        };
                        let admission = oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::fresh_fixture::fresh::Source::new(0)?.request()?;
                        h.root.initialize_admission_equipment(&authority, &h.node, &fence, native.source_digest(), admission.operation()).await.map_err(|error| format!("native equipment admission: {error:?}"))?;
                        let template = crate::durability::starter_grant::StarterTemplate {
                            definition: crate::durability::item_mint::TypedDefinitionRef { family: "Item".into(), production_key: key.into(), revision_ref: revision.into() },
                            ..crate::durability::starter_grant::StarterTemplate::main()
                        };
                        assert!(matches!(h.root.grant_starter_items(&authority, &h.node, &fence, &SourceItems(&native), &template).await.map_err(|error| format!("native starter grant: {error:?}"))?, crate::durability::starter_grant::StarterGrantOutcome::Minted(_)));
                    }
                    let request = format!("qualify {} {setup}-reconcile\n", uuid_text(&fixture::id(fixture::SESSION)));
                    let before = h.root_revision().await?;
                    let good_world = std::mem::replace(&mut config.scope.world_id, uuid_text(&fixture::id(121)));
                    assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, "rejected");
                    assert_eq!(before, h.root_revision().await?, "mismatched scope changed the Character");
                    config.scope.world_id = good_world;
                    assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, "qualified", "native W22 fixture refused");
                    let revision = h.root_revision().await?;
                    let work_before: i16 = if setup == "w22b-wealth" {
                        sqlx::query_scalar("SELECT work_units_used FROM game_reward_claim_mint_reservations WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?
                    } else { 0 };
                    for _ in 0..8 {
                        assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, "qualified");
                        if setup == "w22b-wealth" {
                            let after: i16 = sqlx::query_scalar("SELECT work_units_used FROM game_reward_claim_mint_reservations WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
                            assert_eq!(after, work_before, "committed setup replay charged another durable work unit");
                        }
                    }
                    assert_eq!(revision, h.root_revision().await?, "fixture replay performed another write");
                    if setup == "w22b-stamina" {
                        assert_eq!(h.root.read_hunting_stamina(&authority, character).await.map_err(debug)?, 840);
                    } else {
                        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries c USING(item_instance_id,world_id) WHERE i.definition_production_key='oteryn:item.tibia.i36727' AND c.character_id=encode($1,'hex')::uuid AND i.quantity=1").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
                        assert_eq!(count, 1, "real Wealth Duplex custody missing or duplicated");
                    }
                    drop(authority);
                    drop(seal);
                    h.cleanup().await?;
                    continue;
                }
                let suffix = if reconcile { "-reconcile" } else { "" };
                let request = format!("qualify {} {profile}{suffix}\n", uuid_text(&fixture::id(fixture::SESSION)));
                if reconcile {
                    sqlx::raw_sql("CREATE FUNCTION c17_refuse_build() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'qualification injected build failure'; END $$; CREATE TRIGGER c17_refuse_build BEFORE INSERT OR UPDATE ON game_character_build_state FOR EACH ROW WHEN (NEW.vocation <> 'none') EXECUTE FUNCTION c17_refuse_build();").execute(&h.pool).await?;
                    assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, "rejected");
                    let progressed: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(u32::try_from(progressed)?, level, "XP did not commit before injected build refusal");
                    sqlx::raw_sql("DROP TRIGGER c17_refuse_build ON game_character_build_state; DROP FUNCTION c17_refuse_build();").execute(&h.pool).await?;
                    let unknown = fixture_step(&h.root, tokio::time::Instant::now() + Duration::from_secs(1), "experience", async || {
                        Err::<(), _>(crate::durability::character_progression::CharacterProgressionError::Unavailable(DurabilityError::CommitOutcomeUnknown))
                    }).await.err().ok_or("lost XP acknowledgement became success")?;
                    assert_eq!(fixture_failure_reply(&format!("{profile}-reconcile"), unknown.as_ref()), "unavailable");
                }
                {
                if reconcile { POST_COMMIT_READ_FAILURE.with(|fail| fail.set(true)); }
                let fixture = prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes());
                tokio::pin!(fixture);
                if busy {
                    let held = h.root.try_issue_semantic_pass().map_err(debug)?;
                    tokio::time::pause();
                    let early = tokio::time::timeout(Duration::from_millis(100), &mut fixture).await;
                    tokio::time::resume();
                    held.run(|_, _| Box::pin(async { Ok(()) })).await.map_err(debug)?;
                    assert!(early.is_err(), "busy holder prematurely ended fixture: {early:?}");
                }
                let reply = fixture.await;
                if reconcile {
                    assert_eq!(reply, "unavailable", "post-commit read failure must preserve replay");
                    assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, "qualified");
                } else { assert_eq!(reply, "qualified", "profile={profile}"); }
                }
                let character = crate::domain::CharacterId::from_bytes(fixture::id(fixture::CHARACTER))?;
                assert!(matches!(crate::gameplay_transport::load_character_cast_facts(&h.root, &authority, character).await,
                    crate::gameplay_transport::CastFactsLoad::Ready { facts, .. }
                    if facts.level == level && facts.vocation == crate::spell::Vocation::Sorcerer && facts.max_mana >= 100));
                let revision = h.root_revision().await?;
                if reconcile {
                    let unknown = fixture_step(&h.root, tokio::time::Instant::now() + Duration::from_secs(1), "build", async || {
                        Err::<(), _>(crate::durability::character_progression::CharacterProgressionError::Unavailable(DurabilityError::CommitOutcomeUnknown))
                    }).await.err().ok_or("lost build acknowledgement became success")?;
                    assert_eq!(fixture_failure_reply(&format!("{profile}-reconcile"), unknown.as_ref()), "unavailable");
                }
                assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, if reconcile { "qualified" } else { "rejected" });
                assert_eq!(h.root_revision().await?, revision, "one-shot rejection changed the Character");
                drop(authority);
                drop(seal);
                h.cleanup().await?;
            }
            Ok(())
        })
    }

    #[cfg(feature = "test-support")]
    async fn admit_fixture(
        h: &crate::bestiary_postgres_harness::Harness,
    ) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::debug;
        use crate::foundation::admission_authority_publication::*;
        use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::fresh_fixture;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut runtime = fresh_fixture::fresh::Source::new(now)?;
        runtime
            .rows
            .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
        let runtime = AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?;
        h.root
            .publish_runtime_readiness(&h.node, &runtime)
            .await
            .map_err(debug)?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut source = fresh_fixture::fresh::Source::new(now)?;
        let runtime_row = source
            .rows
            .iter_mut()
            .find(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)))
            .ok_or("runtime observation absent")?;
        *runtime_row = runtime
            .changes()
            .first()
            .ok_or("runtime publication absent")?
            .clone();
        let mut admission = fresh_fixture::fresh::Source::new(now)?;
        admission
            .rows
            .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
        let publication =
            AdmissionAuthorityPublicationV1::prepare(&admission, now).map_err(debug)?;
        crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
            h.root.clone(),
        )
        .publish(&publication)
        .await
        .map_err(debug)?;
        let result =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .commit(&source.request()?)
                .await
                .map_err(debug)?;
        assert!(matches!(result, crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)), "{result:?}");
        Ok(())
    }

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
        let prepare = source
            .find("prepare_and_checkout(")
            .ok_or("missing scope plan")?;
        let read = source
            .find("wild_spawn::validate_placement(")
            .ok_or("missing durable read")?;
        assert!(prepare < read);
        let helper = include_str!("qualification_fixture.rs")
            .split_once("async fn prepare_and_checkout")
            .ok_or("missing plan helper")?
            .1
            .split_once("struct SpawnBudget")
            .ok_or("missing plan helper boundary")?
            .0;
        let release = helper
            .find("prepare(runtime.lock().await)?;")
            .ok_or("missing synchronous plan")?;
        let checkout = helper
            .find("checkout(&prepared).await?;")
            .ok_or("missing holder checkout")?;
        assert!(release < checkout);
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
            .rfind("wild_spawn::validate_placement(")
            .ok_or("missing fresh collision read")?;
        let publish = source
            .find("wild_spawn::publish_placement(")
            .ok_or("missing publish")?;
        assert!(probe < guard && guard < fresh && fresh < publish);
        assert!(source[fresh..publish].contains(">= budget.work_deadline"));
        Ok(())
    }

    #[test]
    fn qualification_spawn_watchdog_caller_contract() -> Result<(), &'static str> {
        let source = control_spawn_source()?;
        assert!(
            source.contains("finish_spawn_holder(holder, budget, outcome)"),
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
                pool.acquire().await.map(Some).map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))
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
            .find("wild_spawn::publish_placement(")
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
        assert!(fixture_item_committed(RewardClaimMintOutcome::Committed(receipt.clone())).is_ok());
        assert!(fixture_item_committed(RewardClaimMintOutcome::AlreadyCommitted(receipt)).is_ok());
    }

    #[test]
    fn retained_fixture_item_accepts_both_session_replay_outcomes() {
        use crate::durability::reward_claim_mint::{
            CommittedRewardClaimMint, RewardClaimMintError, RewardClaimMintOutcome,
            RewardClaimRefusal,
        };
        let retained = CommittedRewardClaimMint {
            transaction_id: [1; 16],
            event_id: [2; 16],
            item_instance_id: [3; 16],
            occurred_at_unix_ms: 1,
            envelope_sha256: [4; 32],
            quantity: 1,
            destination: oteryn_gameplay_items::item_transfer::ContainerEntryPosition {
                parent_item_instance_id: [5; 16],
                placement_ordinal: 0,
            },
        };
        assert!(
            finish_fixture_item(Ok(RewardClaimMintOutcome::AlreadyCommitted(retained))).is_ok()
        );
        assert!(
            finish_fixture_item(Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::AlreadyClaimed
            )))
            .is_ok()
        );
        assert!(finish_fixture_item(Err(RewardClaimMintError::AuthorityRejected)).is_err());
    }

    #[tokio::test]
    async fn committed_receipt_waits_for_a_busy_holder() {
        use crate::durability::reward_claim_mint::RewardClaimMintError;
        let mut attempts = 0;
        let receipt = read_fixture_item_receipt(
            None,
            tokio::time::Instant::now() + Duration::from_secs(1),
            async || {
                attempts += 1;
                if attempts == 1 {
                    Err(RewardClaimMintError::Unavailable(
                        DurabilityError::RootUnavailable,
                    ))
                } else {
                    Ok(Some(9))
                }
            },
        )
        .await;
        assert!(matches!(receipt, Ok(Some(9))));
        assert_eq!(attempts, 2);
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
    fn qualification_next_round_named_profiles_have_distinct_workflows() {
        let session = [7; 16];
        let victim = fixture_occurrence(&session, "victim", 1);
        let knight = fixture_occurrence(&session, "knight", 1);
        assert_ne!(victim, knight);
        assert_ne!(knight, fixture_occurrence(&session, "explorer", 1));
        assert_ne!(knight, fixture_occurrence(&session, "knight", 2));
        assert_eq!(knight, fixture_occurrence(&session, "knight", 1));
    }

    #[tokio::test]
    async fn qualification_spawn_never_checks_out_the_holder_while_waiting_for_runtime()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::atomic::{AtomicBool, Ordering};
        let runtime = Mutex::new(());
        let blocked = runtime.lock().await;
        let checked_out = AtomicBool::new(false);
        let mut preparing = Box::pin(prepare_and_checkout(
            &runtime,
            |guard| {
                drop(guard);
                Ok(())
            },
            async |_| {
                checked_out.store(true, Ordering::SeqCst);
                Ok::<_, String>(())
            },
        ));
        let first = std::future::poll_fn(|context| {
            std::task::Poll::Ready(std::future::Future::poll(preparing.as_mut(), context))
        })
        .await;
        assert!(first.is_pending());
        assert!(
            !checked_out.load(Ordering::SeqCst),
            "a blocked runtime must not reserve the sole durability holder"
        );
        drop(blocked);
        preparing.await?;
        assert!(checked_out.load(Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn qualification_next_round_runtime_busy_retries_without_holder()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::cell::Cell;
        let runtime = Mutex::new(0u32);
        let blocked = runtime.lock().await;
        let holding = Cell::new(false);
        let attempts = Cell::new(0u32);
        let mut operation = Box::pin(retry_publication(
            &runtime,
            tokio::time::Instant::now() + Duration::from_secs(1),
            async || {
                attempts.set(attempts.get() + 1);
                holding.set(true);
                let result = match runtime.try_lock() {
                    Ok(mut value) => {
                        *value += 1;
                        Some(*value)
                    }
                    Err(_) => None,
                };
                holding.set(false);
                Ok::<_, String>(result)
            },
            || "publication deadline".to_owned(),
        ));
        let first = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(operation.as_mut(), cx))
        })
        .await;
        assert!(
            first.is_pending(),
            "transient contention must retry, not refuse"
        );
        assert!(!holding.get(), "the holder must be dropped before waiting");
        drop(blocked);
        assert_eq!(operation.await?, 1);
        assert_eq!(attempts.get(), 2);
        assert_eq!(*runtime.lock().await, 1);
        Ok(())
    }

    #[test]
    fn qualification_next_round_production_publication_has_no_await_boundary()
    -> Result<(), Box<dyn std::error::Error>> {
        // Check the real production attempt, rather than just an ordering helper.
        // A synchronous runtime acquisition and tail are required while SQL is held.
        let source = include_str!("qualification_fixture.rs");
        let place = source
            .split_once("async fn place(")
            .ok_or("place")?
            .1
            .split_once("fn cleanup_specimens(")
            .ok_or("cleanup")?
            .0;
        assert!(
            !place.contains("next_command_source"),
            "one-shot placement has no source-id budget"
        );
        let after_validation = place
            .rsplit_once("wild_spawn::validate_placement(")
            .ok_or("validation")?
            .1
            .split_once(".await?;")
            .ok_or("validation await")?
            .1;
        let acquisition = after_validation
            .split_once("let creature =")
            .ok_or("publication")?
            .0;
        assert!(acquisition.contains("owners.runtime.try_lock()"));
        assert!(
            !acquisition.contains(".await"),
            "runtime waiting must release SQL first"
        );
        let publication = after_validation
            .split_once("wild_spawn::publish_placement(")
            .ok_or("publication")?
            .1
            .split_once("Ok(Some(creature))")
            .ok_or("witness")?
            .0;
        assert!(
            !publication.contains(".await"),
            "publication must have a synchronous tail"
        );
        let collision = source
            .split_once("async fn spawn_collision<T>(")
            .ok_or("collision receipt")?
            .1;
        let publish = collision
            .find("*published = operation(tx).await?;")
            .ok_or("missing published receipt")?;
        let finish = collision
            .find("finish_spawn_holder(holder, budget, result).await")
            .ok_or("missing holder finish")?;
        assert!(publish < finish);
        Ok(())
    }

    #[test]
    fn a_published_spawn_cannot_be_reported_as_a_retryable_refusal()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = classify_spawn_publication(Some(7u8), Err(wild_spawn::SpawnRefusal::Deadline));
        assert_eq!(result, Ok(7));
        assert!(
            classify_spawn_publication::<u8>(None, Ok(7)).is_err(),
            "success needs an actual publication witness"
        );
        assert!(
            classify_spawn_publication::<u8>(None, Err(wild_spawn::SpawnRefusal::Deadline))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn fixture_disabled_has_a_classified_warning() {
        let refused = prepare_request(false, Some(()), b"untrusted request").err();
        assert_eq!(refused, Some(EarlyRefusal::FixtureDisabled));
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
    fn qualification_prefix_is_limited_to_fixture_commands()
    -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                for enabled in [false, true] {
                    let (mut client, mut server) = UnixStream::pair()?;
                    let (sent, request) = tokio::join!(
                        async {
                            client.write_all(b"!health\n").await?;
                            client.shutdown().await
                        },
                        handle_control(&mut server, enabled),
                    );
                    sent?;
                    assert!(request.is_none());
                }
                Ok(())
            })
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
                                "coin-converters",
                                "sweep",
                                "atelier",
                                "victim",
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
                                let (bytes, cancellation) = handle_control(&mut stream, true)
                                    .await
                                    .ok_or("Python request exceeded control bound")?;
                                assert!(cancellation);
                                assert_eq!(
                                    bytes,
                                    format!("qualify {} {profile}\n", uuid_text(&session))
                                        .as_bytes()
                                );
                                let (_, actual_session, actual_profile) =
                                    prepare_request(true, Some(()), &bytes)
                                        .map_err(|_| "Python request refused by node parser")?;
                                assert_eq!(actual_session, session);
                                assert_eq!(actual_profile, profile);
                                stream.write_all(b"qualified\n").await?;
                                stream.shutdown().await?;
                                let output = child.await??;
                                if !output.status.success() {
                                    return Err(format!(
                                        "Python client rejected the parsed exchange: {:?}",
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
        for id in [3276, 3003, 3457, 3147, 6553] {
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
        // The sweep weapon's equip requirements are known to the content, so the equip owner
        // accepts it for a level >= 75 knight (the hatchet 3276 has unknown ones).
        use oteryn_gameplay_domain::domain::equipment_policy_abi::{
            EquipmentBaseVocation, EquipmentPolicyLookup,
        };
        let key = "oteryn:item.tibia.i6553";
        let policy = native
            .equipment_policy(key, index.revision(key).ok_or("Item missing")?)
            .ok_or("equipment policy")?;
        let claims = policy
            .claims()
            .iter()
            .find(|(slot, _)| *slot == 5)
            .ok_or("weapon slot claims")?
            .1
            .clone()?;
        assert_eq!(claims.level, 75);
        assert_eq!(claims.vocations, vec![EquipmentBaseVocation::Knight]);
        assert!(SWEEP_LEVEL >= u32::from(claims.level));
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
    fn qualification_victim_request_preserves_a_pvp_eligible_no_vocation_role() {
        assert!(request(b"qualify 01890f4c-3b2a-7c01-8d11-9a321b7c0004 victim").is_some());
        let (vocation, level) = profile_build("victim");
        assert_eq!(vocation, crate::durability::character_build::NO_VOCATION);
        assert_eq!(level, 100);
        assert!(level >= oteryn_gameplay_combat_rules::pvp::PVP_MIN_LEVEL);
    }

    #[test]
    fn qualification_request_accepts_only_named_profiles_and_v7_sessions() {
        let id = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
        for profile in [
            "explorer", "knight", "paladin", "sorcerer", "druid", "monk", "sweep", "atelier",
            "victim",
        ] {
            assert!(request(format!("qualify {id} {profile}\n").as_bytes()).is_some());
        }
        for text in [
            format!("qualify {id} master_sorcerer"),
            format!("qualify {id} knight extra"),
            format!("qualify {id} knight\nqualify {id} druid"),
            "qualify 01890f4c-3b2a-4c01-8d11-9a321b7c0004 knight".into(),
        ] {
            assert!(request(text.as_bytes()).is_none());
        }
    }
}

#[cfg(test)]
#[test]
fn final_review_atelier_fixture_is_within_the_native_writer_limit() {
    assert!((100..=1000).contains(&profile_build("atelier").1));
}

#[cfg(test)]
#[test]
fn temporary_converter_occurrences_are_replayable_and_replenishable() {
    assert_eq!(
        fixture_item_claim_key(25719, &[1; 16]),
        fixture_item_claim_key(25719, &[1; 16])
    );
    assert_ne!(
        fixture_item_claim_key(25719, &[1; 16]),
        fixture_item_claim_key(25719, &[2; 16])
    );
    assert_ne!(
        fixture_item_claim_key(28525, &[1; 16]),
        fixture_item_claim_key(28525, &[2; 16])
    );
}

// The reward owner checks this Character's immutable claim key under the current
// item fence. The fixture exclusively owns the per-item qualification keys.
fn previous_fixture_item_committed(
    error: &crate::durability::reward_claim_mint::RewardClaimMintError,
) -> bool {
    use crate::durability::reward_claim_mint::{RewardClaimMintError, RewardClaimRefusal};
    matches!(
        error,
        RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)
    )
}

#[test]
fn fixture_repeat_accepts_only_the_same_durable_claim() {
    use crate::durability::reward_claim_mint::{
        RewardClaimMintError as Error, RewardClaimRefusal as Refusal,
    };
    assert!(previous_fixture_item_committed(&Error::Refused(
        Refusal::AlreadyClaimed
    )));
    assert!(!previous_fixture_item_committed(&Error::Refused(
        Refusal::ClaimPending
    )));
    assert!(!previous_fixture_item_committed(&Error::ConflictingCause));
}
