//! GM-1 real admitted session -> command envelope -> atomic signed audit/effect -> wire result.
#[path = "staff/evidence_cases.rs"]
mod evidence_cases;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::staff::{StaffConfig, StaffTrust, rules};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{self, AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
};
use oteryn_protocol_oteryn::{self as protocol, staff as wire};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn canonical(value: &impl Serialize) -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(&serde_json::to_value(value)?)?)
}
struct Signing {
    directory: PathBuf,
    key: PathBuf,
    config: StaffConfig,
    activation: wire::SignedEnvelope,
    fingerprint: String,
}
impl Drop for Signing {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
impl Signing {
    fn new() -> TestResult<Self> {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".staff-test-{}-{suffix}", std::process::id()));
        std::fs::create_dir(&directory)?;
        let key = directory.join("fixture-key");
        let generated = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        assert!(generated.success());
        let public = std::fs::read_to_string(key.with_extension("pub"))?;
        let parts: Vec<_> = public.split_whitespace().collect();
        let roster = format!(
            "# oteryn-staff-role operator GM\noperator {} {}\n",
            parts[0], parts[1]
        );
        use base64::Engine;
        let raw = base64::engine::general_purpose::STANDARD.decode(parts[1])?;
        let fingerprint = format!(
            "SHA256:{}",
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(Sha256::digest(raw))
        );
        let config = StaffConfig {
            environment: "gm-test".into(),
            state_directory: directory.clone(),
            activation_history_file: directory.join("history.json"),
            allowed_signers_sha256: sha(roster.as_bytes()),
            newest_activation_sha256: String::new(),
        };
        let mut fixture = Self {
            directory,
            key,
            config,
            activation: wire::SignedEnvelope {
                principal: "operator".into(),
                canonical: Vec::new(),
                signature: Vec::new(),
            },
            fingerprint,
        };
        let activation = rules::Activation {
            control_seq: "1".into(),
            environment: "gm-test".into(),
            previous_link_sha256: "0".repeat(64),
            roster_sha256: fixture.config.allowed_signers_sha256.clone(),
        };
        fixture.activation = fixture.sign(&canonical(&activation)?, false)?;
        fixture.config.newest_activation_sha256 = sha(&fixture.activation.canonical);
        let history = serde_json::json!({"revisions":[{"allowed_signers":roster,"activation":String::from_utf8(fixture.activation.canonical.clone())?,"signature":String::from_utf8(fixture.activation.signature.clone())?,"principal":"operator"}]});
        std::fs::write(
            &fixture.config.activation_history_file,
            serde_json::to_vec(&history)?,
        )?;
        Ok(fixture)
    }
    fn sign(&self, bytes: &[u8], receipt: bool) -> TestResult<wire::SignedEnvelope> {
        let mut child = Command::new("ssh-keygen")
            .args(["-Y", "sign", "-f"])
            .arg(&self.key)
            .args([
                "-n",
                if receipt {
                    "oteryn-ops-submission-receipt-v1"
                } else {
                    "oteryn-ops-request-v1"
                },
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        child.stdin.take().ok_or("stdin")?.write_all(bytes)?;
        let output = child.wait_with_output()?;
        assert!(output.status.success());
        Ok(wire::SignedEnvelope {
            principal: "operator".into(),
            canonical: bytes.to_vec(),
            signature: output.stdout,
        })
    }
    fn request(
        &self,
        seq: u64,
        previous: &str,
        operation: u8,
        now: u64,
        action: rules::Action,
    ) -> rules::Request {
        rules::Request {
            action,
            activation_sha256: self.config.newest_activation_sha256.clone(),
            channel_id: hex(&id(43)),
            control_seq: seq.to_string(),
            environment: self.config.environment.clone(),
            issued_at: now.to_string(),
            operation_id: hex(&id(operation)),
            previous_link_sha256: previous.into(),
            reason: rules::Reason::Support,
            roster_sha256: self.config.allowed_signers_sha256.clone(),
            world_id: hex(&id(42)),
        }
    }
    async fn bootstrap(&self, h: &Harness) -> TestResult {
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_staff_rosters(environment,activation_sha256,roster_sha256,principal,key_fingerprint,role) VALUES($1,$2,$3,'operator',$4,'GM')")
            .bind(&self.config.environment).bind(&self.config.newest_activation_sha256).bind(&self.config.allowed_signers_sha256).bind(&self.fingerprint).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,kind,link_sha256,previous_link_sha256,activation_sha256,roster_sha256,principal,key_fingerprint,canonical,signature) VALUES($1,1,'ACTIVATION',$2,$3,$2,$4,'operator',$5,$6,$7)")
            .bind(&self.config.environment).bind(&self.config.newest_activation_sha256).bind("0".repeat(64)).bind(&self.config.allowed_signers_sha256).bind(&self.fingerprint).bind(&self.activation.canonical).bind(&self.activation.signature).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_staff_control_heads(environment,control_seq,link_sha256,activation_sha256) VALUES($1,1,$2,$2)").bind(&self.config.environment).bind(&self.config.newest_activation_sha256).execute(&mut *tx).await?;
        tx.commit().await?;
        h.root
            .configure_staff(Some(self.config.clone()))
            .await
            .map_err(debug)?;
        Ok(())
    }
}
struct WireOwner<'a, 'b, 'f, 's>(&'a ComposedFreshAdmission<'b, 'f, 's>);
impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    async fn admit(
        &self,
        _: connection::FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, connection::AdmissionRefusal> {
        Err(connection::AdmissionRefusal::Unavailable)
    }
    async fn observe(
        &self,
        actor: crate::foundation::ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.0.observe(actor).await
    }
    async fn staff_command(
        &self,
        admitted: AdmittedSession,
        receipt: bool,
        payload: &[u8],
    ) -> crate::gameplay_transport::StaffReply {
        self.0.staff_command(admitted, receipt, payload).await
    }
    async fn chat_intent(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        command: u64,
        intent: protocol::chat::ChatIntent,
    ) -> protocol::chat::ChatIntentResult {
        self.0.chat_intent(actor, session, command, intent).await
    }
    async fn cast_spell_completion(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        command: u64,
        intent: protocol::actor_spell::SpellCastIntent,
    ) -> Option<crate::gameplay_transport::actor_spell::SpellCastOutcome> {
        self.0
            .cast_spell_completion(actor, session, command, intent)
            .await
    }
}
async fn exchange_raw(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_type: u32,
    payload: &[u8],
) -> TestResult<Option<(protocol::CommandStatus, Vec<u8>)>> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let frame = protocol::encode_client_command(
        admitted.continuity.connection_generation,
        &protocol::ClientCommandValue {
            command_id: 1,
            command_type,
            payload,
        },
    )?;
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = connection::serve_admitted(
        &mut server,
        admitted,
        &WireOwner(owner),
        connection::IDLE_LIVENESS,
    )
    .await;
    assert!(
        matches!(end, connection::ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut at = 0;
    let mut result = None;
    while at < output.len() {
        let len = u32::from_be_bytes(output[at..at + 4].try_into()?) as usize;
        let e = protocol::decode_wire_envelope(&output[at + 4..at + 4 + len])?;
        if e.message_type() == protocol::MessageType::CommandResult {
            let r = protocol::decode_command_result(e.payload())?;
            assert_eq!(r.command_id, 1);
            result = Some((r.status, r.payload.to_vec()));
        }
        at += len + 4;
    }
    Ok(result)
}
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u32,
    payload: &[u8],
) -> TestResult<Option<Vec<u8>>> {
    let (status, bytes) = exchange_raw(owner, admitted, command, payload)
        .await?
        .ok_or("missing command result")?;
    if status == protocol::CommandStatus::Accepted {
        Ok(Some(bytes))
    } else {
        assert!(bytes.is_empty());
        Ok(None)
    }
}
async fn submit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    envelope: &wire::SignedEnvelope,
    receipt: bool,
) -> TestResult<Option<Vec<u8>>> {
    exchange(
        owner,
        admitted,
        if receipt {
            wire::COMMAND_TYPE_STAFF_RECEIPT
        } else {
            wire::COMMAND_TYPE_STAFF_REQUEST
        },
        &wire::encode_envelope(envelope)?,
    )
    .await
    .map_err(|error| {
        let body =
            serde_json::from_slice::<serde_json::Value>(&envelope.canonical).unwrap_or_default();
        format!(
            "staff sequence {} kind {}: {error}",
            body["control_seq"], body["action"]["kind"]
        )
        .into()
    })
}
async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_staff_operations),(SELECT count(*) FROM game_staff_action_outbox),(SELECT count(*) FROM game_character_sanctions)").fetch_one(&h.pool).await?)
}
#[test]
fn staff_pg_signed_wire_mute_receipt_replay_restart_and_fail_closed() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "GM PostgreSQL thread panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let mut signing = Signing::new()?;
        evidence_cases::history_boundaries(&signing)?;
        let h = Harness::create_with_admission_seed(admin, "gm_signed_wire", true, false)
            .await
            .map_err(debug)?;
        eprintln!("POSTGRES_STARTED GM-1 staff_pg_signed_wire");
        prepare_current_session(&h).await?;
        signing.bootstrap(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[wire::CAPABILITY_STAFF_V1],
        )
        .ok_or("staff negotiation")?;
        admitted.item_fence = Some(
            owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("current fence")?
                .item_fence(),
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;
        crate::gameplay_transport::staff::test_clock::set(Some((now, now)));
        let mute = signing.request(
            2,
            &signing.config.newest_activation_sha256,
            90,
            now,
            rules::Action::Mute {
                character_id: hex(&id(41)),
                duration_ms: "60000".into(),
            },
        );
        let signed = signing.sign(&canonical(&mute)?, false)?;
        let encoded = wire::encode_envelope(&signed)?;
        // No capability, no signature, unknown signer or wrong namespace cannot write.
        let mut unselected = admitted;
        unselected.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert!(submit(&owner, unselected, &signed, false).await?.is_none());
        assert!(
            exchange(&owner, admitted, wire::COMMAND_TYPE_STAFF_REQUEST, &[])
                .await?
                .is_none()
        );
        let mut unsigned = encoded.clone();
        unsigned.pop();
        assert!(
            exchange(
                &owner,
                admitted,
                wire::COMMAND_TYPE_STAFF_REQUEST,
                &unsigned
            )
            .await?
            .is_none()
        );
        let mut unknown = signed.clone();
        unknown.principal = "unknown".into();
        assert!(submit(&owner, admitted, &unknown, false).await?.is_none());
        assert!(
            submit(
                &owner,
                admitted,
                &signing.sign(&canonical(&mute)?, true)?,
                false
            )
            .await?
            .is_none()
        );
        // Every negative changes one invariant while retaining a valid detached signature.
        for field in [
            "environment",
            "world",
            "channel",
            "roster",
            "activation",
            "sequence",
            "previous",
            "expired",
            "future",
            "duration",
        ] {
            let mut bad = mute.clone();
            match field {
                "environment" => bad.environment = "other".into(),
                "world" => bad.world_id = hex(&id(99)),
                "channel" => bad.channel_id = hex(&id(99)),
                "roster" => bad.roster_sha256 = "a".repeat(64),
                "activation" => bad.activation_sha256 = "a".repeat(64),
                "sequence" => bad.control_seq = "3".into(),
                "previous" => bad.previous_link_sha256 = "a".repeat(64),
                "expired" => bad.issued_at = (now - 600_001).to_string(),
                "future" => bad.issued_at = (now + 300_001).to_string(),
                _ => {
                    bad.action = rules::Action::Mute {
                        character_id: hex(&id(41)),
                        duration_ms: "86400001".into(),
                    }
                }
            }
            assert!(
                submit(
                    &owner,
                    admitted,
                    &signing.sign(&canonical(&bad)?, false)?,
                    false
                )
                .await?
                .is_none(),
                "{field}"
            );
        }
        for bad in [
            format!(" {}", String::from_utf8(canonical(&mute)?)?),
            String::from_utf8(canonical(&mute)?)?.replacen("{", "{\"unknown\":\"x\",", 1),
        ] {
            assert!(
                submit(
                    &owner,
                    admitted,
                    &signing.sign(bad.as_bytes(), false)?,
                    false
                )
                .await?
                .is_none()
            );
        }
        let mut stale = admitted;
        stale.continuity.connection_generation = 2;
        assert!(submit(&owner, stale, &signed, false).await?.is_none());
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        assert!(submit(&owner, stale, &signed, false).await?.is_none());
        stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .character_lease_generation = 1;
        assert!(submit(&owner, stale, &signed, false).await?.is_none());
        stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .scope_ownership_generation =
            crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
        assert!(submit(&owner, stale, &signed, false).await?.is_none());
        stale = admitted;
        stale.item_fence.as_mut().ok_or("fence")?.game_session_id =
            crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
        assert!(submit(&owner, stale, &signed, false).await?.is_none());
        stale = admitted;
        stale.channel_id = crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?;
        assert!(submit(&owner, stale, &signed, false).await?.is_none());
        for bounds in [None, Some((now + 1, now)), Some((0, 0))] {
            crate::gameplay_transport::staff::test_clock::set(bounds);
            assert!(submit(&owner, admitted, &signed, false).await?.is_none());
        }
        crate::gameplay_transport::staff::test_clock::set(Some((now, now)));
        assert_eq!(counts(&h).await?, (0, 0, 0));
        let retained_history = std::fs::read(&signing.config.activation_history_file)?;
        let mut invalid_history: serde_json::Value = serde_json::from_slice(&retained_history)?;
        invalid_history["revisions"][0]["allowed_signers"] =
            serde_json::Value::String("operator ssh-ed25519 invalid\n".into());
        std::fs::write(
            &signing.config.activation_history_file,
            serde_json::to_vec(&invalid_history)?,
        )?;
        assert!(StaffTrust::load(signing.config.clone()).is_err());
        // The running node keeps boot's reviewed bytes; disk edits never activate trust.
        let outcome = submit(&owner, admitted, &signed, false)
            .await?
            .ok_or("mute not committed")?;
        std::fs::write(&signing.config.activation_history_file, &retained_history)?;
        let result = wire::decode_result(&outcome)?;
        assert_eq!(result.disposition, 1);
        assert_eq!(result.control_seq, 2);
        assert_eq!(counts(&h).await?, (1, 1, 1));
        assert!(owner.staff_muted(admitted.game_session_id).await);
        let mut chatting = admitted;
        chatting.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[7, wire::CAPABILITY_STAFF_V1],
        )
        .ok_or("chat and staff negotiation")?;
        let say = protocol::chat::encode_chat_intent(&protocol::chat::ChatIntent::Say {
            mode: protocol::chat::ChatSpeechMode::Say,
            text: "blocked by durable mute".into(),
        })
        .map_err(debug)?;
        let (_, answer) = exchange_raw(&owner, chatting, 13, &say)
            .await?
            .ok_or("chat result")?;
        assert_eq!(
            protocol::chat::decode_chat_intent_result(&answer)
                .map_err(debug)?
                .disposition,
            protocol::chat::ChatDisposition::Muted
        );
        let spell = protocol::actor_spell::encode_spell_cast_intent(
            &protocol::actor_spell::SpellCastIntent {
                spell: std::num::NonZeroU32::new(1).ok_or("spell")?,
                target: protocol::actor_spell::SpellTarget::None,
                aim_at_target: false,
            },
        );
        let before = owner
            .observe_vitals(deps.actor, admitted.game_session_id)
            .await;
        let (_, answer) = exchange_raw(&owner, admitted, 3, &spell)
            .await?
            .ok_or("spell result")?;
        assert_eq!(
            protocol::actor_spell::decode_spell_cast_result(&answer).map_err(debug)?,
            protocol::actor_spell::SpellCastDisposition::Rejected
        );
        assert_eq!(
            owner
                .observe_vitals(deps.actor, admitted.game_session_id)
                .await,
            before
        );

        // No second execution even when time is unavailable or the command is now expired.
        crate::gameplay_transport::staff::test_clock::set(None);
        assert_eq!(
            submit(&owner, admitted, &signed, false).await?,
            Some(outcome.clone())
        );
        crate::gameplay_transport::staff::test_clock::set(Some((now + 700_000, now + 700_000)));
        assert_eq!(
            submit(&owner, admitted, &signed, false).await?,
            Some(outcome.clone())
        );
        assert_eq!(counts(&h).await?, (1, 1, 1));
        let mut conflict = mute.clone();
        conflict.reason = rules::Reason::Cheating;
        assert!(
            submit(
                &owner,
                admitted,
                &signing.sign(&canonical(&conflict)?, false)?,
                false
            )
            .await?
            .is_none()
        );
        crate::gameplay_transport::staff::test_clock::set(Some((now, now)));
        // A mutation's signed receipt must arrive before another action extends the head.
        let inspect = signing.request(
            3,
            &sha(&signed.canonical),
            91,
            now,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        assert!(
            submit(
                &owner,
                admitted,
                &signing.sign(&canonical(&inspect)?, false)?,
                false
            )
            .await?
            .is_none()
        );
        let receipt = rules::Receipt {
            activation_sha256: signing.config.newest_activation_sha256.clone(),
            control_seq: "3".into(),
            disposition: rules::ReceiptDisposition::Committed,
            environment: signing.config.environment.clone(),
            operation_id: mute.operation_id.clone(),
            outcome_sha256: sha(&outcome),
            previous_link_sha256: sha(&signed.canonical),
            request_sha256: sha(&signed.canonical),
            roster_sha256: signing.config.allowed_signers_sha256.clone(),
        };
        let mut wrong = receipt.clone();
        wrong.outcome_sha256 = "a".repeat(64);
        assert!(
            submit(
                &owner,
                admitted,
                &signing.sign(&canonical(&wrong)?, true)?,
                true
            )
            .await?
            .is_none()
        );
        let signed_receipt = signing.sign(&canonical(&receipt)?, true)?;
        let receipt_outcome = submit(&owner, admitted, &signed_receipt, true)
            .await?
            .ok_or("receipt")?;
        assert_eq!(wire::decode_result(&receipt_outcome)?.disposition, 3);
        assert_eq!(
            submit(&owner, admitted, &signed_receipt, true).await?,
            Some(receipt_outcome)
        );
        let inspect = signing.request(
            4,
            &sha(&signed_receipt.canonical),
            91,
            now,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        let signed_inspect = signing.sign(&canonical(&inspect)?, false)?;
        let inspect_outcome = submit(&owner, admitted, &signed_inspect, false)
            .await?
            .ok_or("inspect")?;
        assert_eq!(
            wire::decode_result(&inspect_outcome)?.mute_until_ms,
            now + 60_000
        );
        // Reconstruct trust and the composed owner after a process restart; sanctions are DB truth.
        StaffTrust::load(signing.config.clone())?;
        use sqlx::ConnectOptions;
        let options = h
            .pool
            .connect_options()
            .as_ref()
            .clone()
            .options([("role", "oteryn_game_runtime")]);
        let runtime_url = options.to_url_lossy();
        {
            // A second node without operator trust still enforces the durable mute.
            let player_root =
                crate::durability::DurabilityRoot::connect_test_runtime(runtime_url.as_str())?;
            assert!(player_root.maintain_ready_once().await?);
            player_root.configure_staff(None).await?;
            let player_authority = player_root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut player_owner = deps.owner(&h, &player_authority);
            player_owner.root = &player_root;
            assert!(player_owner.staff_muted(admitted.game_session_id).await);
            let (_, answer) = exchange_raw(&player_owner, chatting, 13, &say)
                .await?
                .ok_or("cross-node chat result")?;
            assert_eq!(
                protocol::chat::decode_chat_intent_result(&answer)
                    .map_err(debug)?
                    .disposition,
                protocol::chat::ChatDisposition::Muted
            );
            assert!(
                submit(&player_owner, admitted, &signed, false)
                    .await?
                    .is_none(),
                "a node without operator trust cannot execute even a valid signed request"
            );
        }
        let restarted_root =
            crate::durability::DurabilityRoot::connect_test_runtime(runtime_url.as_str())?;
        assert!(restarted_root.maintain_ready_once().await?);
        restarted_root
            .configure_staff(Some(signing.config.clone()))
            .await?;
        let restarted_authority = restarted_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut restarted = deps.owner(&h, &restarted_authority);
        restarted.root = &restarted_root;
        assert!(restarted.staff_muted(admitted.game_session_id).await);
        assert_eq!(
            submit(&restarted, admitted, &signed, false).await?,
            Some(outcome)
        );
        let unmute = signing.request(
            5,
            &sha(&signed_inspect.canonical),
            92,
            now,
            rules::Action::Unmute {
                character_id: hex(&id(41)),
                sanction_id: mute.operation_id.clone(),
            },
        );
        let signed_unmute = signing.sign(&canonical(&unmute)?, false)?;
        let unmute_outcome = submit(&restarted, admitted, &signed_unmute, false)
            .await?
            .ok_or("unmute")?;
        assert_eq!(wire::decode_result(&unmute_outcome)?.disposition, 1);
        assert!(!restarted.staff_muted(admitted.game_session_id).await);
        assert_eq!(counts(&h).await?, (3, 3, 1));
        let unmute_receipt = rules::Receipt {
            control_seq: "6".into(),
            operation_id: unmute.operation_id.clone(),
            previous_link_sha256: sha(&signed_unmute.canonical),
            request_sha256: sha(&signed_unmute.canonical),
            outcome_sha256: sha(&unmute_outcome),
            ..receipt
        };
        let signed_unmute_receipt = signing.sign(&canonical(&unmute_receipt)?, true)?;
        assert!(
            submit(&restarted, admitted, &signed_unmute_receipt, true)
                .await?
                .is_some()
        );
        assert_eq!(restarted_root.expire_staff_audit().await?, 0);
        // A failed audit insert rolls back both effect and chain; a read exposes no data.
        sqlx::raw_sql("CREATE SEQUENCE gm_test_audit_attempts; GRANT USAGE ON SEQUENCE gm_test_audit_attempts TO oteryn_game_runtime; CREATE FUNCTION gm_test_audit_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('gm_test_audit_attempts'); RAISE EXCEPTION 'test audit failure'; END; $$; CREATE TRIGGER gm_test_audit_failure BEFORE INSERT ON game_staff_action_outbox FOR EACH ROW EXECUTE FUNCTION gm_test_audit_failure();").execute(&h.pool).await?;
        for action in [
            rules::Action::Mute {
                character_id: hex(&id(41)),
                duration_ms: "60000".into(),
            },
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        ] {
            assert!(restarted_root.maintain_ready_once().await?);
            let failed =
                signing.request(7, &sha(&signed_unmute_receipt.canonical), 93, now, action);
            let bytes = wire::encode_envelope(&signing.sign(&canonical(&failed)?, false)?)?;
            assert!(
                exchange_raw(
                    &restarted,
                    admitted,
                    wire::COMMAND_TYPE_STAFF_REQUEST,
                    &bytes
                )
                .await?
                .is_none(),
                "audit failure closes without a terminal result"
            );
            assert_eq!(counts(&h).await?, (3, 3, 1));
        }
        let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM gm_test_audit_attempts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            attempts, 2,
            "both audited read and mutation reached the failing audit writer"
        );
        sqlx::raw_sql("DROP TRIGGER gm_test_audit_failure ON game_staff_action_outbox; DROP FUNCTION gm_test_audit_failure(); DROP SEQUENCE gm_test_audit_attempts;").execute(&h.pool).await?;
        assert!(restarted_root.maintain_ready_once().await?);
        let refused = signing.request(
            7,
            &sha(&signed_unmute_receipt.canonical),
            94,
            now,
            rules::Action::Mute {
                character_id: hex(&id(99)),
                duration_ms: "60000".into(),
            },
        );
        let signed_refused = signing.sign(&canonical(&refused)?, false)?;
        let refused_outcome = submit(&restarted, admitted, &signed_refused, false)
            .await?
            .ok_or("audited refusal")?;
        assert_eq!(
            wire::decode_result(&refused_outcome)?.code,
            wire::StaffCode::TargetNotFound
        );
        let refusal_receipt = rules::Receipt {
            activation_sha256: signing.config.newest_activation_sha256.clone(),
            control_seq: "8".into(),
            disposition: rules::ReceiptDisposition::Refused,
            environment: signing.config.environment.clone(),
            operation_id: refused.operation_id.clone(),
            outcome_sha256: sha(&refused_outcome),
            previous_link_sha256: sha(&signed_refused.canonical),
            request_sha256: sha(&signed_refused.canonical),
            roster_sha256: signing.config.allowed_signers_sha256.clone(),
        };
        let signed_refusal_receipt = signing.sign(&canonical(&refusal_receipt)?, true)?;
        assert!(
            submit(&restarted, admitted, &signed_refusal_receipt, true)
                .await?
                .is_some()
        );
        // Expiring the audit never destroys immutable operation/sanction evidence or re-executes a retry.
        let old = now - 31_536_000_000 - 900_001;
        crate::gameplay_transport::staff::test_clock::set(Some((old, old)));
        let expired = signing.request(
            9,
            &sha(&signed_refusal_receipt.canonical),
            95,
            old,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        let signed_expired = signing.sign(&canonical(&expired)?, false)?;
        let expired_outcome = submit(&restarted, admitted, &signed_expired, false)
            .await?
            .ok_or("expiring inspect")?;
        assert_eq!(restarted_root.expire_staff_audit().await?, 1);
        crate::gameplay_transport::staff::test_clock::set(None);
        assert_eq!(
            submit(&restarted, admitted, &signed_expired, false).await?,
            Some(expired_outcome)
        );
        crate::gameplay_transport::staff::test_clock::set(Some((now, now)));
        let next = signing.request(
            10,
            &sha(&signed_expired.canonical),
            96,
            now,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        assert!(
            submit(
                &restarted,
                admitted,
                &signing.sign(&canonical(&next)?, false)?,
                false
            )
            .await?
            .is_some(),
            "legitimately expired audit evidence does not break the retained chain"
        );
        assert_eq!(counts(&h).await?, (6, 5, 1));
        // Privilege checks use the real runtime role rather than the test administrator.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert!(
            sqlx::query("DELETE FROM game_staff_operations")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        tx.rollback().await?;
        evidence_cases::schema_and_retained_evidence(&h, &signing, runtime_url.as_str()).await?;
        evidence_cases::retained_key_across_revisions(
            &h,
            &mut signing,
            &deps,
            admitted,
            &signed,
            now,
            runtime_url.as_str(),
        )
        .await?;
        drop(restarted);
        drop(restarted_authority);
        drop(restarted_root);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
