//! A complete, internally consistent receipt cannot grant an illegal Forge result.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::item_forge::support::{self, Case, request, seed_items};
use crate::durability::item_mint_audit::{
    CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1, OneItemOperationV1,
    OneItemTransactionV1, RETENTION_PROFILE_ID_V2, RL07_ENVELOPE_BYTES_MAX, RL07_PAYLOAD_BYTES_MAX,
    TransactionEventRefV1, encode_bounded,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::item_forge::{Operation, Request};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::num::NonZeroU64;

#[path = "item_forge_integrity/capacity.rs"]
mod capacity;

#[test]
fn forge_postgres_fabricated_complete_receipts_cannot_grant_illegal_tiers_or_custody() -> TestResult
{
    super::kill_reward_e2e::scenario_thread::run("forge-integrity", || {
        support::scenario(Case::Integrity)
    })
}

pub(super) async fn exercise(
    h: &Harness,
    _owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(
        h,
        &[
            (80, "oteryn:item.tibia.i3264", 1, 0),
            (81, "oteryn:item.tibia.i3264", 1, 0),
        ],
    )
    .await?;
    let before = snapshot(h).await?;
    let attack = ForgedOperation::prepare(h).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    attack.write(&mut tx, admitted).await?;
    // The first-layer guards have accepted every row. This is deliberately a
    // stronger attack than an unreceipted tier INSERT: both immutable snapshots,
    // correct reserved draws, a canonical Type2 event and matching history exist.
    let complete: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_forge_operations WHERE applied),(SELECT count(*) FROM game_forge_history),(SELECT count(*) FROM game_item_audit_outbox),(SELECT count(*) FROM game_item_tiers WHERE tier=10)")
        .fetch_one(&mut *tx).await?;
    assert_eq!(complete, (1, 1, 1, 1));
    let error = match tx.commit().await {
        Ok(()) => return Err("forged Forge result committed".into()),
        Err(error) => error,
    };
    let database = error.as_database_error().ok_or("native SQL guard error")?;
    assert_eq!(database.code().as_deref(), Some("23514"));
    assert!(
        database
            .message()
            .contains("Forge transition or nominal resources"),
        "the legal-transition proof, not a missing row, rejects the attack: {database}"
    );
    assert_eq!(snapshot(h).await?, before, "all forged effects roll back");
    capacity::exercise(h, admitted).await
}

async fn snapshot(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'tiers',(SELECT jsonb_agg(to_jsonb(t) ORDER BY item_instance_id) FROM game_item_tiers t),'custody',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),'dust',(SELECT jsonb_agg(to_jsonb(d) ORDER BY character_id) FROM game_character_forge_dust d),'operations',(SELECT count(*) FROM game_forge_operations),'reservations',(SELECT count(*) FROM game_forge_reservations),'history',(SELECT count(*) FROM game_forge_history),'tier_audit',(SELECT count(*) FROM game_item_tier_audit),'audit',(SELECT count(*) FROM game_item_audit_outbox),'dust_entries',(SELECT count(*) FROM game_character_forge_dust_entries),'fees',(SELECT count(*) FROM game_item_fee_burns))")
        .fetch_one(&h.pool).await?)
}

struct ForgedOperation {
    request: Request,
    content: Vec<u8>,
    rolls: [u16; 3],
    occurred_at: i64,
    inputs: Value,
    outputs: Value,
    dust_spent: i64,
    audit_item: Vec<u8>,
    reply: Value,
    proof: Vec<u8>,
}

impl ForgedOperation {
    async fn prepare(h: &Harness) -> TestResult<Self> {
        let mut action = request(Operation::Fusion, 201);
        action.first_item = NonZeroU64::new(1);
        action.second_item = NonZeroU64::new(2);
        let secret: Vec<u8> =
            sqlx::query_scalar("SELECT secret FROM game_forge_decision_root WHERE singleton")
                .fetch_one(&h.pool)
                .await?;
        let root = GameplayDecisionRoot::from_bytes(
            Sha256::digest([secret.as_slice(), id(41).as_slice()].concat()).into(),
        );
        let draw = |index, bound| {
            deterministic_decision_u64(
                &root,
                DecisionOccurrenceId::from_bytes(action.occurrence),
                "forge_fusion",
                index,
            )
            .map(|value| (value % bound) as u16)
            .map_err(debug)
        };
        let rolls = [draw(0, 100)?, draw(1, 100)?, draw(2, 10_001)?];
        let sql_rolls: Vec<i16> = sqlx::query_scalar(
            "SELECT game_forge_expected_rolls(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
        )
        .bind(id(41).as_slice())
        .bind(action.occurrence.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let expected_rolls = rolls
            .into_iter()
            .map(i16::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(sql_rolls, expected_rolls);
        let content: Vec<u8> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).fetch_one(&h.pool).await?;
        let occurred_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        let inputs: Value = sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('item',i.item_instance_id,'before',to_jsonb(i),'location',jsonb_build_object('kind',1,'parent',e.parent_item_instance_id,'ordinal',e.placement_ordinal::text,'depth',1),'quantity_after',1,'tier_before',0,'tier_after',CASE WHEN i.item_instance_id=encode($1,'hex')::uuid THEN 10 ELSE 0 END) ORDER BY i.item_instance_id) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(id(80).as_slice()).bind(id(81).as_slice()).fetch_one(&h.pool).await?;
        let reply = json!({"operation":"fusion","succeeded":true,"applied":true,"reason":null,"bonus":0,
            "result":{"first_item":1,"first_tier":10,"second_item":2,"second_tier":0,
                "dust":1000,"dust_limit":1000,"gold_spent":0,"cores_spent":0,
                "slivers_spent":0,"slivers_created":0,"cores_created":0}});
        let hash = |value: &Value| -> TestResult<Vec<u8>> {
            Ok(Sha256::digest(serde_json::to_vec(value)?).to_vec())
        };
        let proof = serde_json::to_vec(&json!({
            "occurrence":action.occurrence,"character":id(41),"world":id(42),"operation":1,
            "content":content,"ruleset":"crystal-201189f-forge-v1","simulation_revision":1,
            "rolls":rolls,"reply":reply,"inputs_hash":hash(&inputs)?,"outputs_hash":hash(&json!([]))?,
            "stash_hash":hash(&Value::Null)?,"inputs":2,"outputs":0,"retired":0,"tier_changes":1,
            "dust_spent":0,"gold_spent":0,"stash_spent":false
        }))?;
        // Verify cross-language snapshot hashing against actual database JSON,
        // including UUIDs, ordinals, integers and native nullable columns.
        let canonical: String = sqlx::query_scalar("SELECT game_forge_canonical_json($1)")
            .bind(&inputs)
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(canonical, serde_json::to_string(&inputs)?);
        Ok(Self {
            request: action,
            content,
            rolls,
            occurred_at,
            inputs,
            outputs: json!([]),
            dust_spent: 0,
            audit_item: id(80).to_vec(),
            reply,
            proof,
        })
    }

    async fn write(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        admitted: &AdmittedSession,
    ) -> TestResult {
        let intent = self.request.encode().ok_or("canonical Forge intent")?;
        sqlx::query("INSERT INTO game_forge_reservations(character_id,occurrence_id,transaction_id,event_id,entry_id,world_id,channel_id,intent,content_digest,ruleset_revision,simulation_revision,success_roll,protection_roll,bonus_roll,occurred_at,exhausted,first_item_instance_id,second_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,'crystal-201189f-forge-v1',1,$10,$11,$12,$13,false,encode($14,'hex')::uuid,encode($15,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(self.request.occurrence.as_slice()).bind(id(202).as_slice())
            .bind(id(203).as_slice()).bind(id(204).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(intent.as_slice()).bind(&self.content).bind(i16::try_from(self.rolls[0])?)
            .bind(i16::try_from(self.rolls[1])?).bind(i16::try_from(self.rolls[2])?).bind(self.occurred_at)
            .bind(self.request.operation.requires_pair().then(|| id(80).to_vec()))
            .bind(self.request.operation.requires_pair().then(|| id(81).to_vec()))
            .execute(&mut **tx).await?;
        sqlx::query("UPDATE game_forge_reservations SET work_units_used=1 WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
            .bind(id(41).as_slice()).bind(self.request.occurrence.as_slice()).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_forge_operations(character_id,occurrence_id,transaction_id,event_id,game_session_id,command_id,operation,reply,applied,inputs,outputs,stash,dust_spent,gold_spent,dust_before,dust_limit,occurred_at,audit_proof) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,900,$10,$6,true,$7,$11,NULL,$12,0,1000,1000,$8,$9)")
            .bind(id(41).as_slice()).bind(self.request.occurrence.as_slice()).bind(id(202).as_slice())
            .bind(id(203).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice())
            .bind(&self.reply).bind(&self.inputs).bind(self.occurred_at).bind(&self.proof)
            .bind(self.request.operation as i16).bind(&self.outputs).bind(self.dust_spent)
            .execute(&mut **tx).await?;
        if self.request.operation.requires_pair() {
            sqlx::query("UPDATE game_item_instances SET last_transaction_id=encode($1,'hex')::uuid WHERE item_instance_id IN(encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                .bind(id(202).as_slice()).bind(id(80).as_slice()).bind(id(81).as_slice()).execute(&mut **tx).await?;
            sqlx::query(
                "INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,10)",
            )
            .bind(id(80).as_slice())
            .execute(&mut **tx)
            .await?;
        } else {
            capacity::write_effects(self, tx).await?;
        }
        let envelope = self.envelope(admitted)?;
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,2,$3,encode($4,'hex')::uuid,$5,$5+7776000000,$6,sha256($6),1)")
            .bind(id(203).as_slice()).bind(id(202).as_slice()).bind(RETENTION_PROFILE_ID_V2)
            .bind(&self.audit_item).bind(self.occurred_at).bind(&envelope).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_forge_operations SET envelope_sha256=sha256($2) WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(id(202).as_slice()).bind(&envelope).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_forge_history(character_id,occurrence_id,history_ordinal,reply) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3)")
            .bind(id(41).as_slice()).bind(self.request.occurrence.as_slice()).bind(&self.reply).execute(&mut **tx).await?;
        Ok(())
    }

    fn envelope(&self, admitted: &AdmittedSession) -> TestResult<Vec<u8>> {
        let mut operation = OneItemOperationV1::Forge(Default::default());
        if let OneItemOperationV1::Forge(ref mut forge) = operation {
            forge.proof = self.proof.clone();
        }
        let payload = encode_bounded(
            &OneItemTransactionV1 {
                interpretation_revision: 1,
                operation: Some(operation),
            },
            RL07_PAYLOAD_BYTES_MAX,
        )
        .map_err(debug)?;
        let session = admitted.game_session_id.as_bytes().to_vec();
        let envelope = EventEnvelopeV1 {
            envelope_revision: 1,
            event_id: id(203).to_vec(),
            event_type_id: 2,
            event_schema_revision: 2,
            durability_class: 2,
            privacy_class: 3,
            retention_profile_id: RETENTION_PROFILE_ID_V2.into(),
            occurred_at_unix_ms: self.occurred_at,
            world_id: Some(id(42).to_vec()),
            channel_id: Some(id(43).to_vec()),
            game_session_id: Some(session.clone()),
            connection_generation: Some(
                admitted
                    .item_fence
                    .ok_or("current item fence")?
                    .connection_generation
                    .get(),
            ),
            command_id: Some(900),
            transaction_event: Some(TransactionEventRefV1 {
                transaction_id: id(202).to_vec(),
                ordinal: 1,
                count: 1,
            }),
            causation: Some(CausationRefV1 {
                cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                    game_session_id: session,
                    command_id: 900,
                })),
            }),
            server_build_id: "oteryn-forge-integrity-test".into(),
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload,
            ..EventEnvelopeV1::default()
        };
        encode_bounded(&envelope, RL07_ENVELOPE_BYTES_MAX).map_err(debug)
    }
}
