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
        "explorer" | "knight" | "paladin" | "sorcerer" | "druid" | "monk" | "sweep" | "victim"
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
    let mut published = None;
    let placed = tokio::time::timeout(
        Duration::from_secs(10),
        place(
            root,
            authority,
            proof,
            native,
            owners,
            SpawnRequest { session, key },
            &mut published,
        ),
    )
    .await;
    let outcome = match placed {
        Ok(outcome) => outcome,
        Err(_) => Err(wild_spawn::SpawnRefusal::Deadline),
    };
    match classify_spawn_publication(published, outcome) {
        Ok(creature) => {
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
        Err(refusal) => refuse(refusal.token(), refusal.detail()),
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
    let SpawnRequest { session, key } = request;
    let fence = wild_spawn::SpawnRefusal::Fence;
    let session = crate::foundation::GameSessionId::decode(&session)
        .map_err(|_| wild_spawn::SpawnRefusal::SessionUnknown)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
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
            let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(
                &mut holder,
                deadline.into_std(),
            )
            .await
            .map_err(|error| fence(format!("qualification spawn transaction: {error:?}")))?;
            let eligible =
                wild_spawn::validate_placement(&mut tx, root, authority, proof, &permit, &plan)
                    .await?;
            if tokio::time::Instant::now() >= deadline {
                return Err(wild_spawn::SpawnRefusal::Deadline);
            }
            // Busy returns from this attempt, dropping SQL and the holder before the retry waits.
            let Ok(mut runtime) = owners.runtime.try_lock() else {
                return Ok(None);
            };
            let creature = wild_spawn::publish_placement(&mut runtime, key, &plan, &eligible)?;
            specimens.push(QualificationSpecimen {
                session: *session.as_bytes(),
                character,
                actor: creature.actor,
                key: key.to_owned(),
                content: runtime.content_pin().server_artifact_digest(),
            });
            *published = Some(creature.clone());
            drop(runtime);
            // The spawn is runtime state, not a SQL mutation; the transaction only held the fences.
            drop(tx);
            Ok(Some(creature))
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
const FIXTURE_ITEM_IDS: [u32; 3] = [3276, 3003, 3457];
/// Ruthless axe (6553, attack 50, level 75, knight) joins the common items for `sweep`.
const SWEEP_ITEM_IDS: [u32; 4] = [3276, 3003, 3457, 6553];

fn profile_build(profile: &str) -> (&str, u32) {
    match profile {
        "explorer" => ("knight", 100),
        "sweep" => ("knight", SWEEP_LEVEL),
        "victim" => (crate::durability::character_build::NO_VOCATION, 100),
        other => (other, 100),
    }
}

// The session, named profile and stage identify one retryable control workflow.
fn fixture_occurrence(session: &[u8; 16], profile: &str, stage: u8) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:qualification-control-profile:v1")
        .chain_update(session)
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
    let (vocation, level) = profile_build(profile);
    qualification_build::prepare_character_with_policy_until(
        root,
        authority,
        proof,
        &current,
        qualification_build::QualificationBuildSpec {
            vocation,
            level,
            context,
            xp_occurrence: fixture_occurrence(session.as_bytes(), profile, 1),
            build_occurrence: fixture_occurrence(session.as_bytes(), profile, 2),
        },
        policy,
        deadline,
    )
    .await?;
    prepare_items(root, authority, proof, &current, native, profile, deadline).await
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
    profile: &str,
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
    // vocation and level still come from the served progression policy. The sweep build
    // also carries the ruthless axe, whose level and knight requirements the content knows.
    let ids: &[u32] = if profile == "sweep" {
        &SWEEP_ITEM_IDS
    } else {
        &FIXTURE_ITEM_IDS
    };
    for (ordinal, id) in ids.iter().copied().enumerate() {
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
        if !matches!(
            outcome,
            RewardClaimMintOutcome::Committed(_) | RewardClaimMintOutcome::AlreadyCommitted(_)
        ) {
            return Err(format!("qualification Item not committed: {outcome:?}").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .split_once("wild_spawn::validate_placement(")
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
        assert!(publication.contains("*published = Some(creature.clone())"));
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
                                "knight", "druid", "paladin", "sorcerer", "explorer", "monk",
                                "sweep",
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
        for id in SWEEP_ITEM_IDS {
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
            "explorer", "knight", "paladin", "sorcerer", "druid", "monk", "sweep",
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
