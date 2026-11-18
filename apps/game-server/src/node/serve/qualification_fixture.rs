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
        profile.strip_suffix("-reconcile").unwrap_or(profile),
        "explorer"
            | "knight"
            | "paladin"
            | "sorcerer"
            | "druid"
            | "monk"
            | "c17a"
            | "c17c"
            | "c17d"
            | "c04"
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
        "c17a-reconcile" | "c17c-reconcile" | "c04-reconcile"
    ) && error
        .downcast_ref::<FixtureStepFailure>()
        .is_some_and(|failure| failure.reason == "commit_unknown")
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
    let placed = tokio::time::timeout(
        Duration::from_secs(10),
        place(root, authority, proof, native, owners, session, key),
    )
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
async fn place(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    native: &NativeGameplayState,
    owners: SpawnOwners<'_>,
    session: [u8; 16],
    key: &str,
) -> Result<crate::foundation::CompanionSnapshot, wild_spawn::SpawnRefusal> {
    let fence = wild_spawn::SpawnRefusal::Fence;
    let session = crate::foundation::GameSessionId::decode(&session)
        .map_err(|_| wild_spawn::SpawnRefusal::SessionUnknown)?;
    let source = wild_spawn::next_command_source()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut holder = qualification_build::retry_holder(
        Some(root),
        deadline,
        |error| matches!(error, DurabilityError::RootUnavailable),
        async || root.try_acquire_ready(),
    )
    .await
    .map_err(|error| fence(format!("qualification spawn holder: {error:?}")))?;
    let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(
        &mut holder,
        deadline.into_std(),
    )
    .await
    .map_err(|error| fence(format!("qualification spawn transaction: {error:?}")))?;
    // The Channel owner stays held through the read-only fence and the realisation, as in the
    // harness; the control deadline bounds it, and this path exists only in the fixture mode.
    let mut runtime = owners.runtime.lock().await;
    let creature = wild_spawn::realize_creature_in_transaction(
        &mut tx,
        root,
        authority,
        proof,
        owners.room,
        native,
        &mut runtime,
        key,
        session,
        source,
    )
    .await?;
    drop(runtime);
    // The spawn is runtime state, not a SQL mutation; the transaction only held the fences.
    tx.commit()
        .await
        .map_err(|error| fence(format!("qualification spawn commit: {error:?}")))?;
    Ok(creature)
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
    let reconcile = profile.ends_with("-reconcile");
    let profile = profile.strip_suffix("-reconcile").unwrap_or(profile);
    if reconcile && !matches!(profile, "c17a" | "c17c" | "c04") {
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
    let progression = native
        .progression()
        .ok_or("qualification progression unavailable")?;
    let policy = progression.policy_for(profile_revision, ruleset_revision, content_revision)?;
    let context = policy.context.clone();
    let (vocation, level) = match profile {
        "c17a" | "c17c" | "c04" => {
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
        }
        "explorer" => ("knight", 100),
        _ => (profile, 100),
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
    if matches!(profile, "c17a" | "c17c" | "c04") {
        prepare_admitted_spell_character(
            root,
            authority,
            proof,
            &current,
            spec,
            policy,
            SpellOptions { budget, reconcile },
        )
        .await
    } else {
        qualification_build::prepare_character_with_policy_until(
            root, authority, proof, &current, spec, policy, budget,
        )
        .await?;
        prepare_items(root, authority, proof, &current, native, budget).await
    }
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
    if !matches!(crate::gameplay_transport::load_character_cast_facts(root, authority, character).await,
        crate::gameplay_transport::CastFactsLoad::Ready { facts, .. }
        if facts.level == spec.level && facts.vocation == if spec.vocation == "knight" { crate::spell::Vocation::Knight } else { crate::spell::Vocation::Sorcerer })
    {
        event("event=qualification_fixture_failure stage=cast_facts reason=refused");
        return Err("C17 fenced fixture did not produce the source build".into());
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
    use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
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
    let inventory = qualification_build::retry_holder(
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
    .map_err(|error| format!("qualification inventory: {error:?}"))?;
    let bag = inventory
        .backpack
        .ok_or("qualification requires the real starter backpack")?;
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
                production_key: format!("oteryn:qualification.item.i{id}"),
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
        if !matches!(outcome, RewardClaimMintOutcome::Committed(_)) {
            return Err(format!("qualification Item not committed: {outcome:?}").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn r2_c04_has_a_real_admitted_sorcerer_fixture() -> crate::bestiary_postgres_harness::TestResult
    {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                admitted_profiles(false, false, &[("c04", 23)]).map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "C04 provisioning test panicked")?
            .map_err(Into::into)
    }

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

    fn c17_admitted_profiles(
        busy: bool,
        reconcile: bool,
    ) -> crate::bestiary_postgres_harness::TestResult {
        admitted_profiles(busy, reconcile, &[("c17a", 20), ("c17c", 23)])
    }

    fn admitted_profiles(
        busy: bool,
        reconcile: bool,
        profiles: &[(&str, u32)],
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
                let h = Harness::create_with_admission_seed(admin.clone(), profile, false, false).await?;
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
                assert_eq!(fixture.await, "qualified", "profile={profile}");
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
