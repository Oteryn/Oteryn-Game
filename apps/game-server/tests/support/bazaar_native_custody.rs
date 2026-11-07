//! Small fixture adapter to the existing Game Bazaar owner. It never fabricates
//! decisions, custody, signatures or history, and keeps every SQL guard enabled.
use crate::durability::{
    DurabilityRoot,
    bazaar::Policy,
    character_authority::ReconciledCharacterAuthority,
    content_activation::ContentActivationRequest,
    native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    },
    runtime_scope_assignment::NodeIncarnationProof,
};
use oteryn_gameplay_content::content::{
    ReferenceArtifactProjection, ReferencePlayableArtifactView,
};
use oteryn_gameplay_native_admission_source::{bazaar as wire, character_commands::Decision};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, String>;
fn error(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

pub(super) struct NativeBazaar {
    directory: PathBuf,
}
impl Drop for NativeBazaar {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
impl NativeBazaar {
    pub(super) async fn prepare(
        pool: &sqlx::PgPool,
        root: &DurabilityRoot,
        node: &NodeIncarnationProof,
        activation: ContentActivationRequest,
        artifact: &[u8],
    ) -> Result<Self> {
        let nonce: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(pool)
            .await
            .map_err(error)?;
        let directory = std::env::temp_dir().join(format!("srvenc-bazaar-{nonce}"));
        std::fs::create_dir(&directory).map_err(error)?;
        let fixture = Self { directory };
        std::fs::set_permissions(&fixture.directory, std::fs::Permissions::from_mode(0o700))
            .map_err(error)?;
        let artifact_path = fixture.directory.join("server-artifact.bin");
        std::fs::write(&artifact_path, artifact).map_err(error)?;
        let view = ReferencePlayableArtifactView::load(
            artifact,
            ReferenceArtifactProjection::ServerAuthoritative,
        )
        .map_err(error)?;
        if activation.server_artifact_digest != view.artifact_digest() {
            return Err("Bazaar fixture artifact does not match the loaded activation".into());
        }
        match root
            .read_current_content_activation(activation.world_id, activation.channel_id)
            .await
            .map_err(error)?
        {
            Some(current)
                if current.activation_sequence == activation.activation_sequence
                    && current.server_artifact_digest == activation.server_artifact_digest
                    && current.client_artifact_digest == activation.client_artifact_digest
                    && current.frame_binding_digest == activation.frame_binding_digest => {}
            Some(_) => {
                return Err("Bazaar fixture cannot replace a different active Content pin".into());
            }
            None => {
                sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
                    .bind(activation.world_id.as_bytes().as_slice()).bind(activation.channel_id.as_bytes().as_slice()).execute(pool).await.map_err(error)?;
                if !root
                    .record_content_activation(&activation)
                    .await
                    .map_err(error)?
                {
                    return Err("Bazaar fixture native Content activation refused".into());
                }
            }
        }
        // Same real fixture source setup used by the existing Bazaar authority cases.
        let provenance = FreshStoreProvenance {
            namespace: "store:one".into(),
            authorization: "owner:approved".into(),
            source_authority: "platform".into(),
            initialized_at: 10,
        };
        let descriptor = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: 10,
        };
        if !root
            .record_native_source_descriptor_issuance(
                "platform",
                descriptor.clone(),
                Some(provenance.clone()),
            )
            .await
            .map_err(error)?
        {
            return Err("Bazaar fixture native source descriptor refused".into());
        }
        root.initialize_native_admission_source(node, provenance, descriptor)
            .await
            .map_err(error)?;
        let key = fixture.directory.join("snapshot-key");
        std::fs::write(&key, [23; 32]).map_err(error)?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).map_err(error)?;
        let signing = ed25519_dalek::SigningKey::from_bytes(&[23; 32]);
        let public: String = signing
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let escrow: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(pool)
            .await
            .map_err(error)?;
        let mut policy = Policy::default();
        policy.cooldown_seconds = Some(0);
        policy.escrow_account_id = Some(escrow);
        policy.signing_key_file = Some(key.to_str().ok_or("non-UTF8 fixture key path")?.to_owned());
        policy.signing_kid = Some("w17-bazaar".into());
        policy
            .signing_public_keys
            .insert("w17-bazaar".into(), public);
        policy.content_artifact_file = Some(
            artifact_path
                .to_str()
                .ok_or("non-UTF8 fixture artifact path")?
                .to_owned(),
        );
        policy.content_artifact_sha256 = Some(
            Sha256::digest(artifact)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        );
        sqlx::query(
            "UPDATE game_bazaar_configuration SET enabled=true,policy=$1::jsonb WHERE singleton",
        )
        .bind(serde_json::to_value(policy).map_err(error)?)
        .execute(pool)
        .await
        .map_err(error)?;
        Ok(fixture)
    }

    pub(super) async fn command(
        &self,
        pool: &sqlx::PgPool,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        character: &[u8; 16],
        lock: bool,
    ) -> Result<Value> {
        let (account, world, subject, revision, operation, now): (String, String, String, String, String, i64) = sqlx::query_as("SELECT account_id::text,world_id::text,character_id::text,character_revision::text,game_character_uuid_v7()::text,floor(extract(epoch FROM statement_timestamp()))::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).fetch_one(pool).await.map_err(error)?;
        // Other legitimate producers (including bootstrap) may already have
        // observed this Account. Continue its actual retained source floor.
        let observed: String = sqlx::query_scalar(
            "SELECT (COALESCE(max(source_revision),0)+1)::text FROM game_durability_native_source_floors WHERE registration_id=1 AND source_authority='platform' AND floor_subject=$1",
        )
        .bind(format!("account:{account}"))
        .fetch_one(pool)
        .await
        .map_err(|error| format!("Bazaar fixture account source floor: {error:?}"))?;
        let observed = observed.parse::<u64>().map_err(error)?;
        let facts = json!({"version":1,"operation":"ReadAccountSecurityV1","result":"observed",
            "source_authority":"platform","source_revision":observed.to_string(),"decision_identity":observed.to_string(),
            "source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0","account_id":account,
            "purpose":"platform_security","scope":"fresh_admission","allowed":true,"minimum_valid_generation":"1"});
        root.accept_native_source_observation(
            node,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(account.clone()).map_err(error)?,
                source_revision: observed,
                decision_identity: observed.to_string(),
                observed_at: now,
                semantic_facts: serde_json::to_vec(&facts).map_err(error)?,
            },
        )
        .await
        .map_err(|error| format!("Bazaar fixture account security observation: {error:?}"))?;
        let command = if lock {
            "LockBazaarCharacter"
        } else {
            "UnlockBazaarCharacter"
        };
        let mut value = json!({"contract_version":2,"command":command,"variant":"PLATFORM_BAZAAR",
            "issuer_authority":wire::ISSUER,"issuer_decision_id":operation,"source_revision":observed.to_string(),
            "operation_id":operation,"listing_id":operation,"seller_account_id":account,
            "world_id":world,"character_id":subject,"issued_at_source":(now-1).to_string(),
            "expires_at_source":(now+120).to_string(),"audience":wire::AUDIENCE});
        let intent = if lock {
            value["expected_character_revision"] = json!(revision);
            value["eligibility_revision"] = json!("1");
            value["snapshot_schema_version"] = json!(1);
            serde_json::to_vec(
                &serde_json::from_value::<wire::LockBazaarCharacter>(value).map_err(error)?,
            )
            .map_err(error)?
        } else {
            let (id, listing, revision): (String, String, String) = sqlx::query_as("SELECT lock_id::text,listing_id::text,revision::text FROM game_bazaar_locks WHERE character_id=encode($1,'hex')::uuid AND state='locked' LIMIT 1")
                .bind(character.as_slice()).fetch_one(pool).await.map_err(error)?;
            value["listing_id"] = json!(listing);
            value["lock_id"] = json!(id);
            value["expected_lock_revision"] = json!(revision);
            serde_json::to_vec(
                &serde_json::from_value::<wire::UnlockBazaarCharacter>(value).map_err(error)?,
            )
            .map_err(error)?
        };
        wire::decode_intent(&intent, command, &operation).map_err(error)?;
        match root
            .decide_bazaar_command(
                authority,
                node,
                "oteryn:character-authority:primary",
                command,
                &operation,
                &intent,
            )
            .await
        {
            Decision::Publish {
                receipt,
                result_code,
            } if result_code == "CHAR_CMD_OK" => serde_json::from_slice(&receipt).map_err(error),
            Decision::Publish { result_code, .. } => {
                Err(format!("genuine Bazaar {command} refused: {result_code}"))
            }
            other => Err(format!("genuine Bazaar {command} unavailable: {other:?}")),
        }
    }
}
