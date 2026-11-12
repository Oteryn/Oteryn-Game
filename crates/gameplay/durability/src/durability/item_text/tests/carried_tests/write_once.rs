//! GAP-SV2/I04: the public fenced writer, real PostgreSQL, replay and atomic rollback.
use super::*;

async fn once(h: &Harness, item: u8, command: u64, text: &str) -> TestResult<ItemTextWriteOutcome> {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let fence = native::item_fence()?;
    let mut request = carried_request(item, command, text)?;
    request.definition.production_key = "oteryn:item.tibia.i2818".into();
    let mut policy =
        MailTextPolicy::from_qualified_content(request.definition.clone(), [1; 32], 1023);
    policy.write_once_target = Some(TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2834".into(),
        revision_ref: "definition-r1".into(),
    });
    let permit =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await?;
    Ok(h.root
        .commit_item_text_rewrite(
            &permit, &authority, &h.node, fence, policy, request, None, None,
        )
        .await
        .map_err(|error| format!("I04 fixture: {error:?}"))?)
}

#[test]
fn gap_sv2_i04_postgres_write_once_atomic_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "I04 worker panicked")?
        .map_err(Into::into)
}

fn run(admin: String) -> TestResult {
    runtime()?.block_on(async move {
            let started = std::time::Instant::now();
            let h = Harness::create(admin, "gap_sv2_i04", false).await?;
            let result = async {
                fixture(&h).await?;
                let mut fixture_tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *fixture_tx).await?;
                sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i2818' WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(id(108).as_slice()).bind(id(101).as_slice()).execute(&mut *fixture_tx).await?;
                fixture_tx.commit().await?;
                let written = once(&h, 108, 1, "private text").await?;
                let state: (String, String, i64) = sqlx::query_as("SELECT definition_production_key,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(108).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(state, ("oteryn:item.tibia.i2834".into(), "2".into(), 1));
                assert_eq!(h.count("game_item_audit_outbox").await?, 1);
                let audit_bytes: Vec<u8> = sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(108).as_slice()).fetch_one(&h.pool).await?;
                let (envelope, operation) = crate::durability::item_text::write_once::decode_event(&audit_bytes)
                    .map_err(|_| "invalid write-once audit")?;
                assert_eq!(envelope.event_type_id, 2);
                assert_eq!(operation.scope_ownership_generation, 1);
                assert_eq!(operation.character_lease_generation, 1);
                assert_eq!(operation.before.as_ref().ok_or("before")?.item_instance_id, id(108));
                assert_eq!(operation.after.as_ref().ok_or("after")?.item_instance_id, id(108));
                assert!(!audit_bytes.windows(b"private text".len()).any(|bytes| bytes == b"private text"));
                // The same bounded decoder refuses an extra revision step and a
                // zero ownership fence even when the envelope hash is recomputed.
                use prost::Message;
                for mutate in [
                    (|operation: &mut crate::durability::item_text::write_once::OneItemWriteOnceV1| operation.state_revision_after += 1) as fn(&mut _),
                    |operation| operation.scope_ownership_generation = 0,
                ] {
                    let mut changed = operation.clone();
                    mutate(&mut changed);
                    let payload = crate::durability::item_mint_audit::OneItemTransactionV1 {
                        interpretation_revision: 1,
                        operation: Some(crate::durability::item_mint_audit::OneItemOperationV1::WriteOnce(changed)),
                    }.encode_to_vec();
                    let changed = crate::durability::item_mint_audit::EventEnvelopeV1 {
                        payload_sha256: Sha256::digest(&payload).to_vec(), payload, ..envelope.clone()
                    }.encode_to_vec();
                    assert!(crate::durability::item_text::write_once::decode_event(&changed).is_err());
                }
                assert_eq!(once(&h, 108, 1, "private text").await?, ItemTextWriteOutcome { replayed: true, ..written });
                assert!(once(&h, 108, 1, "conflicting text").await.is_err());
                assert!(once(&h, 108, 2, "second write").await.is_err());
                assert_eq!(h.count("game_item_audit_outbox").await?, 1);
                let stored: String = sqlx::query_scalar("SELECT text FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(108).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(stored, "private text");
                // An audit failure must roll back text, identity-preserving transform and receipt.
                sqlx::raw_sql("CREATE FUNCTION gap_sv2_fail_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit refusal'; END $$; CREATE TRIGGER gap_sv2_fail_audit BEFORE INSERT ON game_item_audit_outbox FOR EACH ROW EXECUTE FUNCTION gap_sv2_fail_audit();").execute(&h.pool).await?;
                assert!(once(&h, 101, 3, "rollback").await.is_err());
                let unchanged: (String, String, bool) = sqlx::query_as("SELECT definition_production_key,state_revision::text,NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(101).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(unchanged, ("oteryn:item.tibia.i2818".into(), "1".into(), true));
                assert_eq!(h.count("game_item_text_write_receipts").await?, 1);
                sqlx::query("DROP TRIGGER gap_sv2_fail_audit ON game_item_audit_outbox").execute(&h.pool).await?;
                // Empty accepted text still seals a write-once item.
                once(&h, 101, 4, "").await?;
                assert_eq!(h.count("game_item_audit_outbox").await?, 2);
                Ok::<(), Box<dyn std::error::Error>>(())
            }.await;
            h.cleanup().await?;
            if result.is_ok() { eprintln!("GAP-SV2 I04 PostgreSQL atomic transform/replay/rollback/empty PASS in {:?}", started.elapsed()); }
            result
        })
}
