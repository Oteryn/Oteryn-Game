use super::fixture::{Harness, debug, id};

use super::TestResult;

use crate::durability::staff::{StaffConfig, rules};

use oteryn_protocol_oteryn::staff as wire;

use serde::Serialize;

use sha2::{Digest, Sha256};

use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(super) fn canonical(value: &impl Serialize) -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(&serde_json::to_value(value)?)?)
}

pub(super) struct Signing {
    pub(super) directory: PathBuf,
    key: PathBuf,
    pub(super) config: StaffConfig,
    activation: wire::SignedEnvelope,
    pub(super) fingerprint: String,
}

impl Drop for Signing {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

impl Signing {
    pub(super) fn new() -> TestResult<Self> {
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
            "# oteryn-staff-role operator ADMIN\noperator {} {}\n",
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
    pub(super) fn sign(&self, bytes: &[u8], receipt: bool) -> TestResult<wire::SignedEnvelope> {
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
    pub(super) fn request(
        &self,
        seq: u64,
        previous: &str,
        operation: u8,
        now: u64,
        action: rules::Action,
    ) -> rules::Request {
        rules::Request {
            issuer_character_id: Some(hex(&id(41))),
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
    pub(super) async fn bootstrap(&self, h: &Harness) -> TestResult {
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_staff_rosters(environment,activation_sha256,roster_sha256,principal,key_fingerprint,role) VALUES($1,$2,$3,'operator',$4,'ADMIN')")
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
