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
        "explorer" | "knight" | "paladin" | "sorcerer" | "druid" | "monk" | "sweep"
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

/// Source level of the `sweep` profile; the served progression table has 2000 levels.
const SWEEP_LEVEL: u32 = 1000;
const FIXTURE_ITEM_IDS: [u32; 3] = [3276, 3003, 3457];
/// Ruthless axe (6553, attack 50, level 75, knight) joins the common items for `sweep`.
const SWEEP_ITEM_IDS: [u32; 4] = [3276, 3003, 3457, 6553];

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
    qualification_build::prepare_character_with_policy_until(
        root,
        authority,
        proof,
        &current,
        qualification_build::QualificationBuildSpec {
            vocation: if matches!(profile, "explorer" | "sweep") {
                "knight"
            } else {
                profile
            },
            level: if profile == "sweep" { SWEEP_LEVEL } else { 100 },
            context,
            xp_occurrence: uuid_v7()?,
            build_occurrence: uuid_v7()?,
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
