//! Disposable node-boot build provisioning. The node retains its incarnation
//! proof; the root-only control socket supplies only an admitted session id.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::qualification_spell_character as qualification_build;
#[cfg(any(test, feature = "qualification-spawn"))]
use crate::qualification_wild_spawn as wild_spawn;

fn request(bytes: &[u8]) -> Option<([u8; 16], &str)> {
    let text = std::str::from_utf8(bytes).ok()?.trim_end_matches('\n');
    let (session, profile) = text.strip_prefix("qualify ")?.split_once(' ')?;
    let session = parse_operation_id(session.as_bytes())?;
    matches!(
        profile,
        "explorer" | "knight" | "paladin" | "sorcerer" | "druid" | "monk" | "knight-underlevel"
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
    let budget = SpawnBudget::new(tokio::time::Instant::now());
    let placed = spawn_with_watchdog(budget.reply_deadline, async |published| {
        place(
            root, authority, proof, native, owners, session, key, budget, published,
        )
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
        qualification_build::retry_holder(
            Some(root),
            budget.work_deadline,
            |error| matches!(error, DurabilityError::RootUnavailable),
            async || root.try_acquire_ready(),
        )
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
        Ok(eligible) => eligible,
        Err(error) => {
            if return_spawn_holder(&mut holder, budget).await.is_err() {
                root.request_ready();
            }
            return Err(error);
        }
    };

    loop {
        let result = spawn_collision(&mut holder, budget, async |tx| {
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
                *published = Some(wild_spawn::publish_spawn(
                    &mut runtime,
                    plan,
                    source,
                    key,
                    &tiles,
                )?);
            }
            Ok(())
        })
        .await;
        if let Err(error) = result {
            root.request_ready();
            return Err(error);
        }
        if published.is_some() {
            return Ok(());
        }
        // Gameplay may hold the runtime while waiting on Ground. Retry only
        // after the previous transaction and holder have been released.
        drop(
            tokio::time::timeout_at(budget.work_deadline, owners.runtime.lock())
                .await
                .map_err(|_| fence("qualification spawn runtime deadline".into()))?,
        );
        holder = acquire_holder().await?;
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
async fn spawn_transaction<T>(
    holder: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
    commit: bool,
    operation: impl AsyncFnOnce(
        &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<T, wild_spawn::SpawnRefusal>,
) -> Result<T, wild_spawn::SpawnRefusal> {
    let fence = wild_spawn::SpawnRefusal::Fence;
    tokio::time::timeout_at(budget.work_deadline, async {
        let mut tx = begin_spawn_transaction(holder, budget.backend_deadline).await?;
        let result = operation(&mut tx).await;
        let released = if commit && result.is_ok() {
            tx.commit().await
        } else {
            tx.rollback().await
        };
        let value = result?;
        released.map_err(|error| {
            fence(format!(
                "qualification spawn transaction release: {error:?}"
            ))
        })?;
        Ok(value)
    })
    .await
    .map_err(|_| fence("qualification spawn work deadline".into()))?
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn spawn_collision<T>(
    holder: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
    operation: impl AsyncFnOnce(
        &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<T, wild_spawn::SpawnRefusal>,
) -> Result<T, wild_spawn::SpawnRefusal> {
    let result = spawn_transaction(holder, budget, false, operation).await;
    let released = return_spawn_holder(holder, budget).await;
    let value = result?;
    released?;
    Ok(value)
}

#[cfg(any(test, feature = "qualification-spawn"))]
async fn return_spawn_holder(
    holder: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    budget: SpawnBudget,
) -> Result<(), wild_spawn::SpawnRefusal> {
    use sqlx::pool::PoolConnectionReturnDisposition;
    match holder
        .return_to_pool_observed_until(budget.release_deadline.into_std())
        .await
    {
        PoolConnectionReturnDisposition::ReturnedToIdle => Ok(()),
        _ => Err(wild_spawn::SpawnRefusal::Fence(
            "qualification spawn holder was retired".into(),
        )),
    }
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

async fn prepare_build(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: &NativeGameplayState,
    session: [u8; 16],
    profile: &str,
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
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )
    .map_err(|error| format!("qualification character: {error:?}"))?;
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
    let context = policy.context.clone();
    let (vocation, level) = match profile {
        "knight-underlevel" => ("knight", 66),
        "explorer" => ("knight", 100),
        _ => (profile, 100),
    };
    let occurrence_profile = if level == 100 { vocation } else { profile };
    qualification_build::prepare_character_with_policy_until(
        root,
        authority,
        proof,
        &current,
        qualification_build::QualificationBuildSpec {
            vocation,
            level,
            context,
            xp_occurrence: fixture_occurrence(character.as_bytes(), occurrence_profile, b"xp"),
            build_occurrence: fixture_occurrence(
                character.as_bytes(),
                occurrence_profile,
                b"vocation",
            ),
        },
        policy,
        deadline,
    )
    .await?;
    prepare_items(root, authority, proof, &current, native, deadline).await
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
        .map_err(|error| format!("qualification inventory: {error:?}"))
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
    // Axe, rope, shovel. Every profile needs the same map/melee prerequisites;
    // vocation and level still come from the served progression policy.
    for (ordinal, id) in [3276, 3003, 3457].into_iter().enumerate() {
        let key = format!("oteryn:item.tibia.i{id}");
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
            .any(|existing| existing.definition == definition && existing.quantity == 1)
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
                production_key: fixture_gear_claim(
                    current.current_game_session_id(),
                    &format!("i{id}"),
                ),
                revision_ref: "qualification-v1".into(),
            },
            source_placement: "oteryn:qualification.node-boot".into(),
            item,
            quantity: 1,
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
        fixture_item_committed(outcome)?;
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
            .find("spawn_collision(&mut holder, budget,")
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
            source.contains("return_spawn_holder(&mut holder, budget)"),
            "pool return must be observed"
        );
        assert!(
            !source.contains(".detach()"),
            "successful spawn must preserve the ready holder"
        );
        assert!(
            source.contains("spawn_collision(&mut holder, budget,"),
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
            spawn_transaction(&mut holder, budget, true, async |tx| {
                sqlx::query("CREATE TEMP TABLE qualification_spawn_committed_probe (id integer)")
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| wild_spawn::SpawnRefusal::Fence(e.to_string()))?;
                Ok(())
            })
            .await
            .map_err(|e| format!("{}: {:?}", e.token(), e.detail()))?;
            let key = spawn_collision(&mut holder, budget, async |tx| {
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
                Ok(key)
            })
            .await
            .map_err(|e| format!("{}: {:?}", e.token(), e.detail()))?;
            let mut holder = pool
                .try_acquire()
                .ok_or("spawn did not return the ready holder")?;
            let pid: i64 = sqlx::query_scalar("SELECT pg_backend_pid()::bigint")
                .fetch_one(&mut *holder)
                .await?;
            assert_eq!(key, pid);

            // Arm the actual backend deadline. Keep the client connected and idle
            // with the lock held to model a stalled rollback or missing FIN.
            let mut tx = begin_spawn_transaction(
                &mut holder,
                tokio::time::Instant::now() + Duration::from_secs(1),
            )
            .await
            .map_err(|e| format!("{}: {:?}", e.token(), e.detail()))?;
            sqlx::query("SELECT pg_advisory_xact_lock($1)")
                .bind(key)
                .execute(&mut *tx)
                .await?;
            tokio::time::timeout(
                Duration::from_secs(1),
                sqlx::query("SELECT pg_advisory_xact_lock($1)")
                    .bind(key)
                    .execute(&mut writer),
            )
            .await??;
            drop(tx);
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
        assert!(fixture_item_committed(RewardClaimMintOutcome::Committed(receipt.clone())).is_ok());
        assert!(fixture_item_committed(RewardClaimMintOutcome::AlreadyCommitted(receipt)).is_ok());
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
                            for profile in
                                ["knight", "druid", "paladin", "sorcerer", "explorer", "monk"]
                            {
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
        for id in [3276, 3003, 3457] {
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
