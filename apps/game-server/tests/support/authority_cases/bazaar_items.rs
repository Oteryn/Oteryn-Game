//! Real native mint/pickup followed by authority escrow, split and Inbox delivery.
use super::{allow, initialize_s2, now_secs};
use crate::durability::bazaar::Policy;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome, ItemWeightLookup, ItemWeightRefused,
};
use crate::item_transfer_postgres_cases as native;
use crate::native_admission_source::character_commands::{Decision, Input};
use oteryn_gameplay_content::content::*;
use oteryn_gameplay_content_promotion::content::protected_cw2_b1_full_item_family_import;
use serde_json::Value;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = "oteryn:character-authority:primary";

#[test]
fn bazaar_items_native_postgres_custody_split_replay_and_terminal_races() -> TestResult {
    let Some(admin) = native::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            (|| -> TestResult {
                native::runtime()?.block_on(async {
                    let start = std::time::Instant::now();
                    let h = native::Harness::create(admin, "bazaar_items").await?;
                    let result = items_flow(&h).await;
                    h.cleanup().await?;
                    eprintln!(
                        "GPROD_BAZAAR_PG_EXECUTED items_native_custody_split_replay elapsed_ms={}",
                        start.elapsed().as_millis()
                    );
                    result
                })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Bazaar item PostgreSQL worker failed")?
        .map_err(Into::into)
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn uuid(seed: u8) -> String {
    native::uuid_text(native::id(seed))
}
pub(super) fn lower_definition(key: &str, revision: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "item".into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    }
}

pub(super) struct Weights;
impl ItemWeightLookup for Weights {
    fn generation_digest(&self) -> [u8; 32] {
        crate::durability::item_weight_lookup::GENERATION_DIGEST
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        if matches!(definition.family.as_str(), "Item" | "item")
            && definition.production_key.starts_with("oteryn:item.")
        {
            Ok(1)
        } else {
            crate::durability::item_weight_lookup::TEST_WEIGHTS.weight_of(definition)
        }
    }
}

struct ResaleContent {
    definition: TypedDefinitionRef,
    digest: [u8; 32],
}
impl ItemWeightLookup for ResaleContent {
    fn generation_digest(&self) -> [u8; 32] {
        Weights.generation_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Weights.weight_of(definition)
    }
}
impl crate::durability::item_loop_move::ItemLoopContent for ResaleContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.digest
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if definition == &self.definition {
            Some(ItemDefinitionFacts {
                weight: Some(1),
                definition: definition.clone(),
                stack: ItemStackClass::NonStackable,
                container_capacity: None,
                container_slot_equip_pattern: false,
            })
        } else if definition == &native::backpack_facts().definition {
            Some(native::backpack_facts())
        } else {
            None
        }
    }
}

async fn retrieve_for_resale(
    h: &native::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    definition: &TypedDefinitionRef,
    digest: [u8; 32],
    sequence: u64,
) -> TestResult {
    // This harness has no runtime actor; activate its seeded native session for
    // the genuine Inbox retrieval, then retire it before the offline auction.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(native::id(native::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_admission_character_guards SET holder_game_session_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid")
        .bind(native::id(native::SESSION).as_slice()).bind(native::id(native::CHARACTER).as_slice()).execute(&h.pool).await?;
    let content = ResaleContent {
        definition: definition.clone(),
        digest,
    };
    let client_id: u32 = definition
        .production_key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("native client id")?
        .parse()?;
    let fence = native::fence()?;
    let permit =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await
            .map_err(native::debug)?;
    let result = h
        .root
        .commit_depot_search_retrieval(
            &permit,
            authority,
            &h.node,
            fence,
            native::command(sequence)?,
            &content,
            client_id,
            false,
        )
        .await
        .map_err(native::debug)?;
    assert_eq!(
        result,
        Ok(crate::durability::item_loop_move::DepotSearchRetrieval::Retrieved(1))
    );
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(native::id(native::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_admission_character_guards SET holder_game_session_id=NULL WHERE character_id=encode($1,'hex')::uuid")
        .bind(native::id(native::CHARACTER).as_slice()).execute(&h.pool).await?;
    Ok(())
}

pub(super) fn artifact() -> TestResult<(Vec<u8>, [TypedDefinitionRef; 2])> {
    use ReferenceItemField::{Known, NotApplicable};
    let evidence = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json",
    ))?;
    let mut imported = protected_cw2_b1_full_item_family_import(&evidence)?;
    let source_client_ids: std::collections::BTreeMap<_, _> = imported
        .batch
        .candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .source_numeric_id
                .map(|id| (candidate.candidate_target.clone(), id))
        })
        .collect();
    let mut selected: [Option<TypedDefinitionRef>; 2] = [None, None];
    let mut authored_identity = None;
    for record in &mut imported.records {
        let ProjectReferenceRecord::Item {
            identity,
            stack_class,
            materializable,
            semantics,
            ..
        } = record
        else {
            continue;
        };
        if !*materializable {
            continue;
        }
        let index = match stack_class {
            ItemStackDocument::NonStackable => 0,
            ItemStackDocument::StackCapable => 1,
            ItemStackDocument::Unknown => continue,
        };
        if selected[index].is_some() {
            continue;
        }
        if index == 0 {
            // Native depot-search Command182 resolves the authored Tibia client
            // key. Derive that key from this exact imported source binding;
            // never invent a client id or change a persisted item definition.
            let source_id = source_client_ids
                .get(&format!("{}@{}", identity.key, identity.revision))
                .ok_or("selected item lacks original source client id")?;
            let original = identity.clone();
            identity.key = format!("oteryn:item.tibia.i{source_id}");
            authored_identity = Some((original, identity.clone()));
        }
        *semantics = ReferenceItemSemantics {
            physical: Known(ReferenceItemPhysical {
                weight: Known(1),
                movable: Known(true),
                pickupable: Known(true),
            }),
            stack: Known(ReferenceItemStack {
                stackable: Known(index == 1),
                stack_max: Known(if index == 1 { 100 } else { 1 }),
            }),
            trade_restrictions: Known(ReferenceItemTradeRestrictions {
                tradeable: Known(true),
                marketable: Known(false),
                vocations: NotApplicable,
                account_binding_policy: NotApplicable,
                character_binding_policy: NotApplicable,
            }),
            temporal: NotApplicable,
            charges: NotApplicable,
            container: NotApplicable,
            ..Default::default()
        };
        let mut definition = lower_definition(&identity.key, &identity.revision);
        // Exercise the native production enum tag and the canonical contract tag.
        if index == 0 {
            definition.family = "Item".into();
        }
        selected[index] = Some(definition);
        if selected.iter().all(Option::is_some) {
            break;
        }
    }
    if let Some((original, authored)) = authored_identity {
        let original_target = format!("{}@{}", original.key, original.revision);
        for candidate in &mut imported.batch.candidates {
            if candidate.candidate_target == original_target {
                candidate.candidate_target = format!("{}@{}", authored.key, authored.revision);
            }
            for field in &mut candidate.normalized_fields {
                if let CandidateValue::NativeItemBinding(binding) = &mut field.value
                    && binding.identity == original
                {
                    binding.identity = authored.clone();
                }
            }
        }
        for state in &mut imported.batch.reimport_states {
            if state.stable_identity == original.key {
                state.stable_identity = authored.key.clone();
            }
            for value in [&mut state.baseline, &mut state.upstream, &mut state.local] {
                if let Some(CandidateValue::NativeItemBinding(binding)) = value
                    && binding.identity == original
                {
                    binding.identity = authored.clone();
                }
            }
        }
    }
    let definitions = [
        selected[0]
            .take()
            .ok_or("nonstackable native Content item")?,
        selected[1].take().ok_or("stackable native Content item")?,
    ];
    let limits = ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 128_000_000,
        max_total_bytes: 220_000_000,
        max_json_depth: 24,
        max_decoded_fields: 3_000_000,
        max_string_bytes: 128_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_reimport_states: CW2_B1_FULL_ITEM_FAMILY_COUNT,
    };
    let canonical = CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: "project-r1".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "PENDING".into(),
            world_id: uuid(native::WORLD).replace('-', ""),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: imported.records,
            imports: vec![imported.batch],
            metadata: Vec::new(),
        },
        limits,
    )?;
    let linked = canonical.into_snapshot(limits)?.parse(limits)?.link()?;
    let compiled = compile_reference_playable(&linked)?;
    Ok((compiled.server_artifact, definitions))
}

fn wire(command: &str, operation: u8, listing: u8, members: &str, now: i64) -> String {
    format!(
        r#"{{"contract_version":2,"command":"{command}","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"{}","source_revision":"{operation}","operation_id":"{}","listing_id":"{}","seller_account_id":"{}","world_id":"{}",{members},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}}"#,
        uuid(210),
        uuid(operation),
        uuid(listing),
        uuid(40),
        uuid(native::WORLD),
        now - 1,
        now + 120
    )
}

fn facts(decision: &Decision, context: &str) -> TestResult<(Value, Vec<u8>)> {
    let Decision::Publish {
        receipt,
        result_code,
    } = decision
    else {
        return Err(format!("{context}: nonterminal Bazaar decision {decision:?}").into());
    };
    if result_code != "CHAR_CMD_OK" {
        return Err(format!(
            "{context}: Bazaar refusal: {result_code}, {}",
            String::from_utf8_lossy(receipt)
        )
        .into());
    }
    let value: Value = serde_json::from_slice(receipt)?;
    Ok((value["result_data"].clone(), receipt.clone()))
}

async fn items_flow(h: &native::Harness) -> TestResult {
    let (bytes, definitions) = artifact()?;
    let artifact_path = std::env::temp_dir().join(format!(
        "gprod-bazaar-artifact-{}-{}.bin",
        std::process::id(),
        now_secs()?
    ));
    std::fs::write(&artifact_path, &bytes)?;
    let result = items_flow_with_artifact(h, &bytes, &definitions, &artifact_path).await;
    std::fs::remove_file(artifact_path)?;
    result
}

// An independent reader must acquire the canonical writer gates before any
// outbox row. The writer below uses the same Content/admission/domain order as
// a real handoff, then requests the exact listing outbox. No state is fabricated.
async fn publication_writer_order(
    h: &native::Harness,
    seal: &crate::character_recovery_fence::SealedCharacterRecoveryFence<'_>,
    listing: &str,
) -> TestResult {
    let reader = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(reader.maintain_ready_once().await?);
    let authority = reader
        .open_character_authority(seal)
        .await
        .map_err(native::debug)?;
    let mut writer = h.pool.begin().await?;
    sqlx::query("LOCK TABLE game_content_activations IN SHARE MODE")
        .execute(&mut *writer)
        .await?;
    sqlx::query("LOCK TABLE game_durability_admission_account_guards IN EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await?;
    sqlx::query("SELECT 1 FROM game_bazaar_escrows WHERE listing_id=$1::uuid FOR UPDATE")
        .bind(listing)
        .fetch_one(&mut *writer)
        .await?;
    let snapshot = reader.bazaar_transport_snapshot(&authority, &h.node, "oteryn:game:primary", 24);
    let writer_finish = async {
        let until = tokio::time::Instant::now() + std::time::Duration::from_secs(12);
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND state='active' AND wait_event_type='Lock')")
                .fetch_one(&h.pool).await?;
            if blocked {
                break;
            }
            if tokio::time::Instant::now() >= until {
                return Err("publication did not reach the controlled writer barrier".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        tokio::time::timeout(std::time::Duration::from_secs(1),
            sqlx::query("SELECT subject FROM game_bazaar_publication_outbox WHERE subject='listing:'||$1::text FOR UPDATE")
                .bind(listing).fetch_one(&mut *writer)).await??;
        writer.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (snapshot, finished) = native::join_two(snapshot, writer_finish).await;
    finished?;
    let snapshot = snapshot.map_err(native::debug)?;
    assert!(
        snapshot
            .receipts
            .iter()
            .any(|r| r.command == "EscrowBazaarItem")
    );
    eprintln!(
        "GPROD_BAZAAR_PUBLICATION_LOCK_ORDER_EXECUTED independent_roots=2 writer_completed=true snapshot_completed=true"
    );
    Ok(())
}

async fn items_flow_with_artifact(
    h: &native::Harness,
    bytes: &[u8],
    definitions: &[TypedDefinitionRef; 2],
    path: &std::path::Path,
) -> TestResult {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let backpack = h.mint(&authority, native::BACKPACK, 1).await?;
    h.committed(
        &authority,
        native::to_slot(native::command(1)?, backpack, native::backpack_facts()),
    )
    .await?;
    let whole = h
        .mint_definition(&authority, definitions[0].clone(), 1)
        .await?;
    let stack = h
        .mint_definition(&authority, definitions[1].clone(), 10)
        .await?;
    for (index, item, quantity) in [(0, whole, 1), (1, stack, 10)] {
        let item_facts = ItemDefinitionFacts {
            weight: Some(1),
            definition: definitions[index].clone(),
            stack: if index == 0 {
                ItemStackClass::NonStackable
            } else {
                ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                }
            },
            container_capacity: None,
            container_slot_equip_pattern: false,
        };
        let fence = native::fence()?;
        let request = native::to_backpack(native::command(2 + index as u64)?, item, item_facts);
        let mut candidate = h
            .root
            .freeze_item_transfer(&authority, &h.node, fence, request, &Weights)
            .await
            .map_err(native::debug)?;
        let outcome = h
            .root
            .commit_item_transfer(
                &crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
                    fence.runtime_scope,
                )
                .await
                .map_err(native::debug)?,
                &authority,
                &h.node,
                fence,
                &mut candidate,
                &Weights,
            )
            .await
            .map_err(native::debug)?;
        assert!(matches!(outcome, ItemTransferOutcome::Committed(_)));
        assert_eq!(h.item_state(item).await?.0, quantity);
    }
    for item in [whole, stack] {
        sqlx::query(
            "INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,2)",
        )
        .bind(item.as_slice())
        .execute(&h.pool)
        .await?;
    }
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
    eprintln!("GPROD_BAZAAR_CONTENT_ARTIFACT bytes={}", bytes.len());
    let artifact = ReferencePlayableArtifactView::load(
        bytes,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: crate::foundation::WorldId::decode(&native::id(native::WORLD))
                        .map_err(native::debug)?,
                    channel_id: crate::foundation::ChannelId::decode(&native::id(native::CHANNEL))
                        .map_err(native::debug)?,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: artifact.artifact_digest(),
                    client_artifact_digest: [3; 32],
                    frame_binding_digest: [4; 32]
                }
            )
            .await
            .map_err(native::debug)?
    );
    initialize_s2(&h.root, &h.node).await?;
    allow(&h.root, &h.node, 32).await?;
    let now = now_secs()?;
    let create = super::create_intent(
        220,
        32,
        native::WORLD,
        22,
        "Bazaar Buyer",
        (now - 1, now + 120),
    );
    let created = h
        .root
        .decide_character_command(
            &authority,
            &h.node,
            SOURCE,
            "CreateCharacter",
            &uuid(220),
            Input::Intent(create.as_bytes()),
        )
        .await;
    let Decision::Publish { receipt, .. } = created else {
        return Err("buyer create pending".into());
    };
    let buyer: Value = serde_json::from_slice(&receipt)?;
    let buyer_character = buyer["character_id"]
        .as_str()
        .ok_or("buyer Character missing")?;
    let policy: Policy = serde_json::from_value(serde_json::json!({
        "obligations_revision":1,"inbox_capacity":10000,"item_classes":["nonstackable","stackable"],
        "common_restore_scope":true,"proof_retention_seconds":7_776_000,"v2_receipt_bytes":270336,
        "content_artifact_file":path.to_str().ok_or("artifact path")?,"content_artifact_sha256":hash(bytes)
    }))?;

    let whole_id = native::uuid_text(whole);
    let stack_id = native::uuid_text(stack);
    sqlx::query(
        "UPDATE game_bazaar_configuration SET enabled=true,policy=$1::jsonb WHERE singleton",
    )
    .bind(serde_json::to_value(&policy)?)
    .execute(&h.pool)
    .await?;
    allow(&h.root, &h.node, 40).await?;
    let active_wire = wire(
        "EscrowBazaarItem",
        199,
        198,
        &format!(
            r#""seller_character_id":"{}","item_instance_id":"{whole_id}","expected_state_revision":"2","quantity":1,"transferability_revision":"1""#,
            uuid(native::CHARACTER)
        ),
        now_secs()?,
    );
    let active = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "EscrowBazaarItem",
            &uuid(199),
            active_wire.as_bytes(),
        )
        .await;
    assert!(
        matches!(&active,Decision::Publish{result_code,..} if result_code=="CHAR_CMD_CHARACTER_IN_SESSION")
    );
    assert_eq!(h.count("game_bazaar_item_locations").await?, 0);
    // The fixture has no runtime actor. Mark the seeded native session terminal
    // without disabling any admission or item guard, then exercise the offline authority.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid").bind(native::id(native::SESSION).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_admission_character_guards SET holder_game_session_id=NULL WHERE character_id=encode($1,'hex')::uuid").bind(native::id(native::CHARACTER).as_slice()).execute(&h.pool).await?;

    let escrow = wire(
        "EscrowBazaarItem",
        201,
        200,
        &format!(
            r#""seller_character_id":"{}","item_instance_id":"{whole_id}","expected_state_revision":"2","quantity":1,"transferability_revision":"1""#,
            uuid(native::CHARACTER)
        ),
        now_secs()?,
    );
    sqlx::query(
        "UPDATE game_bazaar_configuration SET enabled=false,policy=$1::jsonb WHERE singleton",
    )
    .bind(serde_json::to_value(&policy)?)
    .execute(&h.pool)
    .await?;
    assert_eq!(
        h.root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                "EscrowBazaarItem",
                &uuid(201),
                escrow.as_bytes()
            )
            .await,
        Decision::Transient
    );
    assert_eq!(h.count("game_bazaar_item_locations").await?, 0);
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=true WHERE singleton")
        .execute(&h.pool)
        .await?;
    let oversized_path = path.with_extension("oversized");
    std::fs::File::create(&oversized_path)?.set_len(33_554_433)?;
    let mut oversized_policy = policy.clone();
    oversized_policy.content_artifact_file = Some(
        oversized_path
            .to_str()
            .ok_or("oversized artifact path")?
            .to_owned(),
    );
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(serde_json::to_value(&oversized_policy)?)
        .execute(&h.pool)
        .await?;
    allow(&h.root, &h.node, 40).await?;
    let oversized_wire = wire(
        "EscrowBazaarItem",
        197,
        196,
        &format!(
            r#""seller_character_id":"{}","item_instance_id":"{whole_id}","expected_state_revision":"2","quantity":1,"transferability_revision":"1""#,
            uuid(native::CHARACTER)
        ),
        now_secs()?,
    );
    let oversized_decision = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "EscrowBazaarItem",
            &uuid(197),
            oversized_wire.as_bytes(),
        )
        .await;
    std::fs::remove_file(oversized_path)?;
    assert_eq!(oversized_decision, Decision::Transient);
    assert_eq!(h.count("game_bazaar_item_locations").await?, 0);
    sqlx::query("UPDATE game_bazaar_configuration SET policy=$1::jsonb WHERE singleton")
        .bind(serde_json::to_value(&policy)?)
        .execute(&h.pool)
        .await?;
    allow(&h.root, &h.node, 40).await?;
    let first = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "EscrowBazaarItem",
            &uuid(201),
            escrow.as_bytes(),
        )
        .await;
    let (whole_facts, whole_receipt) = facts(&first, "EscrowBazaarItem whole operation201")?;
    publication_writer_order(h, &seal, &uuid(200)).await?;
    h.root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    assert_eq!(whole_facts["item"]["item_instance_id"], whole_id);
    assert_eq!(
        whole_facts["item"]["attributes_hash"],
        "fd8184df56101f486d8766c87c216edddf9c25c4f29ba4c48eb48219dc137c19"
    );
    assert_eq!(
        h.root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                "EscrowBazaarItem",
                &uuid(201),
                escrow.as_bytes()
            )
            .await,
        first
    );
    assert_eq!(
        h.root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                "EscrowBazaarItem",
                &uuid(201),
                escrow
                    .replace("\"quantity\":1", "\"quantity\":2")
                    .as_bytes()
            )
            .await,
        Decision::Conflict
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    assert!(
        sqlx::query("UPDATE game_item_tiers SET tier=3 WHERE item_instance_id=$1::uuid")
            .bind(&whole_id)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await?;
    let whole_escrow = whole_facts["escrow_id"].as_str().ok_or("whole escrow")?;
    let wrong_destination = wire(
        "ReturnBazaarItem",
        209,
        200,
        &format!(
            r#""escrow_id":"{whole_escrow}","expected_escrow_revision":"1","seller_character_id":"{buyer_character}","escrow_receipt_digest":"{}""#,
            hash(&whole_receipt)
        ),
        now_secs()?,
    );
    let refused = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "ReturnBazaarItem",
            &uuid(209),
            wrong_destination.as_bytes(),
        )
        .await;
    assert!(
        matches!(&refused,Decision::Publish{result_code,..} if result_code=="CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE")
    );
    assert_eq!(h.count("game_bazaar_item_locations").await?, 1);
    // Suspension narrows new acquisitions but never strands a safe seller return.
    super::observe_account(&h.root, &h.node, 40, false, now_secs()?).await?;
    let returning = wire(
        "ReturnBazaarItem",
        202,
        200,
        &format!(
            r#""escrow_id":"{whole_escrow}","expected_escrow_revision":"1","seller_character_id":"{}","escrow_receipt_digest":"{}""#,
            uuid(native::CHARACTER),
            hash(&whole_receipt)
        ),
        now_secs()?,
    );
    let returned = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "ReturnBazaarItem",
            &uuid(202),
            returning.as_bytes(),
        )
        .await;
    let (returned_facts, _) = facts(&returned, "ReturnBazaarItem whole operation202")?;
    assert_eq!(returned_facts["item"]["item_instance_id"], whole_id);
    let split = wire(
        "EscrowBazaarItem",
        204,
        203,
        &format!(
            r#""seller_character_id":"{}","item_instance_id":"{stack_id}","expected_state_revision":"2","quantity":4,"transferability_revision":"1""#,
            uuid(native::CHARACTER)
        ),
        now_secs()?,
    );
    allow(&h.root, &h.node, 40).await?;
    let split_result = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "EscrowBazaarItem",
            &uuid(204),
            split.as_bytes(),
        )
        .await;
    let (split_facts, split_receipt) = facts(&split_result, "EscrowBazaarItem split operation204")?;
    h.root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    assert_ne!(split_facts["item"]["item_instance_id"], stack_id);
    assert_eq!(split_facts["item"]["quantity"], 4);
    assert_eq!(h.item_state(stack).await?.0, 6);
    let split_escrow = split_facts["escrow_id"].as_str().ok_or("split escrow")?;
    let deliver = wire(
        "DeliverBazaarItem",
        205,
        203,
        &format!(
            r#""escrow_id":"{split_escrow}","expected_escrow_revision":"1","buyer_account_id":"{}","buyer_character_id":"{buyer_character}","escrow_receipt_digest":"{}","sale_id":"{}","funding":{{"currency":"COINS","reservation_id":"{}","hold_revision":null,"gross_amount":"10000"}}"#,
            uuid(32),
            hash(&split_receipt),
            uuid(206),
            uuid(207)
        ),
        now_secs()?,
    );
    allow(&h.root, &h.node, 32).await?;
    let delivered = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "DeliverBazaarItem",
            &uuid(205),
            deliver.as_bytes(),
        )
        .await;
    let (delivered_facts, _) = facts(&delivered, "DeliverBazaarItem split operation205")?;
    assert_eq!(delivered_facts["item"]["quantity"], 4);
    assert_eq!(delivered_facts["destination_character_id"], buyer_character);
    let losing_return = wire(
        "ReturnBazaarItem",
        208,
        203,
        &format!(
            r#""escrow_id":"{split_escrow}","expected_escrow_revision":"1","seller_character_id":"{}","escrow_receipt_digest":"{}""#,
            uuid(native::CHARACTER),
            hash(&split_receipt)
        ),
        now_secs()?,
    );
    let loser = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "ReturnBazaarItem",
            &uuid(208),
            losing_return.as_bytes(),
        )
        .await;
    assert!(
        matches!(&loser,Decision::Publish{result_code,..} if result_code=="CHAR_CMD_BAZAAR_STALE_REVISION")
    );
    let inbox: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=$1::uuid",
    )
    .bind(buyer_character)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(inbox, 1);
    assert_eq!(h.count("game_bazaar_item_locations").await?, 0);
    assert_eq!(h.item_state(stack).await?.0, 6);
    // A retained terminal escrow must not reserve the physical identity forever.
    // Retrieve and re-list the RETURNED whole item twice, then deliver it
    // to the existing distinct buyer while retaining all terminal histories.
    let content_digest = ReferencePlayableArtifactView::load(
        bytes,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?
    .artifact_digest();
    for (ordinal, outcome) in [(0_u8, "RETURNED"), (1_u8, "DELIVERED")] {
        retrieve_for_resale(
            h,
            &authority,
            &definitions[0],
            content_digest,
            20 + u64::from(ordinal),
        )
        .await?;
        let revision: String = sqlx::query_scalar(
            "SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=$1::uuid",
        )
        .bind(&whole_id)
        .fetch_one(&h.pool)
        .await?;
        let operation = 230 + ordinal * 3;
        let listing = 231 + ordinal * 3;
        let escrow_wire = wire(
            "EscrowBazaarItem",
            operation,
            listing,
            &format!(
                r#""seller_character_id":"{}","item_instance_id":"{whole_id}","expected_state_revision":"{revision}","quantity":1,"transferability_revision":"1""#,
                uuid(native::CHARACTER)
            ),
            now_secs()?,
        );
        allow(&h.root, &h.node, 40).await?;
        let escrow_decision = h
            .root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                "EscrowBazaarItem",
                &uuid(operation),
                escrow_wire.as_bytes(),
            )
            .await;
        let (resale, receipt) = facts(&escrow_decision, "EscrowBazaarItem same-identity resale")?;
        let escrow_id = resale["escrow_id"].as_str().ok_or("resale escrow")?;
        let terminal_operation = operation + 2;
        let (command, fields) = if outcome == "DELIVERED" {
            (
                "DeliverBazaarItem",
                format!(
                    r#""escrow_id":"{escrow_id}","expected_escrow_revision":"1","buyer_account_id":"{}","buyer_character_id":"{}","escrow_receipt_digest":"{}","sale_id":"{}","funding":{{"currency":"COINS","reservation_id":"{}","hold_revision":null,"gross_amount":"10000"}}"#,
                    uuid(32),
                    buyer_character,
                    hash(&receipt),
                    uuid(240),
                    uuid(241)
                ),
            )
        } else {
            (
                "ReturnBazaarItem",
                format!(
                    r#""escrow_id":"{escrow_id}","expected_escrow_revision":"1","seller_character_id":"{}","escrow_receipt_digest":"{}""#,
                    uuid(native::CHARACTER),
                    hash(&receipt)
                ),
            )
        };
        if outcome == "DELIVERED" {
            allow(&h.root, &h.node, 32).await?;
        }
        let terminal_wire = wire(command, terminal_operation, listing, &fields, now_secs()?);
        let terminal = h
            .root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                command,
                &uuid(terminal_operation),
                terminal_wire.as_bytes(),
            )
            .await;
        let (terminal_facts, _) = facts(&terminal, "same-identity resale terminal")?;
        assert_eq!(terminal_facts["item"]["item_instance_id"], whole_id);
    }
    let histories: i64 = sqlx::query_scalar("SELECT count(*) FROM game_bazaar_escrows WHERE item_instance_id=$1::uuid AND state IN ('RETURNED','DELIVERED')")
        .bind(&whole_id).fetch_one(&h.pool).await?;
    assert_eq!(histories, 3);
    h.root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    assert_eq!(h.count("game_bazaar_item_locations").await?, 0);
    eprintln!(
        "GPROD_BAZAAR_ITEM_RESALE_EXECUTED terminal_histories={histories} native_inbox_retrievals=2"
    );
    // Two independent durability roots race over the same escrow fence.
    sqlx::query(
        "UPDATE game_bazaar_configuration SET enabled=true,policy=$1::jsonb WHERE singleton",
    )
    .bind(serde_json::to_value(&policy)?)
    .execute(&h.pool)
    .await?;
    allow(&h.root, &h.node, 40).await?;
    allow(&h.root, &h.node, 32).await?;
    let race_source_revision: String = sqlx::query_scalar(
        "SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=$1::uuid",
    )
    .bind(&stack_id)
    .fetch_one(&h.pool)
    .await?;
    let race_escrow_wire = wire(
        "EscrowBazaarItem",
        240,
        244,
        &format!(
            r#""seller_character_id":"{}","item_instance_id":"{stack_id}","expected_state_revision":"{race_source_revision}","quantity":1,"transferability_revision":"1""#,
            uuid(native::CHARACTER)
        ),
        now_secs()?,
    );
    let race_escrow_decision = h
        .root
        .decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "EscrowBazaarItem",
            &uuid(240),
            race_escrow_wire.as_bytes(),
        )
        .await;
    let (race_escrow_facts, race_receipt) = facts(
        &race_escrow_decision,
        "EscrowBazaarItem terminal race operation240",
    )?;
    let race_escrow_id = race_escrow_facts["escrow_id"]
        .as_str()
        .ok_or("race escrow id")?;
    let race_item_id = race_escrow_facts["item"]["item_instance_id"]
        .as_str()
        .ok_or("race item id")?;
    let race_digest = hash(&race_receipt);
    let race_deliver = wire(
        "DeliverBazaarItem",
        241,
        244,
        &format!(
            r#""escrow_id":"{race_escrow_id}","expected_escrow_revision":"1","buyer_account_id":"{}","buyer_character_id":"{buyer_character}","escrow_receipt_digest":"{race_digest}","sale_id":"{}","funding":{{"currency":"COINS","reservation_id":"{}","hold_revision":null,"gross_amount":"10000"}}"#,
            uuid(32),
            uuid(245),
            uuid(246)
        ),
        now_secs()?,
    );
    let race_return = wire(
        "ReturnBazaarItem",
        242,
        244,
        &format!(
            r#""escrow_id":"{race_escrow_id}","expected_escrow_revision":"1","seller_character_id":"{}","escrow_receipt_digest":"{race_digest}""#,
            uuid(native::CHARACTER)
        ),
        now_secs()?,
    );
    let other = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(other.maintain_ready_once().await?);
    let other_authority = other
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let deliver_operation = uuid(241);
    let return_operation = uuid(242);
    let (race_delivery, race_returned) = native::join_two(
        h.root.decide_bazaar_command(
            &authority,
            &h.node,
            SOURCE,
            "DeliverBazaarItem",
            &deliver_operation,
            race_deliver.as_bytes(),
        ),
        other.decide_bazaar_command(
            &other_authority,
            &h.node,
            SOURCE,
            "ReturnBazaarItem",
            &return_operation,
            race_return.as_bytes(),
        ),
    )
    .await;
    let mut committed = 0;
    let mut stale = 0;
    for outcome in [&race_delivery, &race_returned] {
        match outcome {
            Decision::Publish { result_code, .. } if result_code == "CHAR_CMD_OK" => committed += 1,
            Decision::Publish { result_code, .. }
                if result_code == "CHAR_CMD_BAZAAR_STALE_REVISION" =>
            {
                stale += 1
            }
            other => {
                return Err(format!("delivery/return race unexpected outcome: {other:?}").into());
            }
        }
    }
    assert_eq!((committed, stale), (1, 1));
    let (race_state, race_revision): (String, String) = sqlx::query_as(
        "SELECT state,revision::text FROM game_bazaar_escrows WHERE escrow_id=$1::uuid",
    )
    .bind(race_escrow_id)
    .fetch_one(&h.pool)
    .await?;
    assert!(matches!(race_state.as_str(), "DELIVERED" | "RETURNED"));
    assert_eq!(race_revision, "2");
    let race_inbox_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=$1::uuid",
    )
    .bind(race_item_id)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(race_inbox_count, 1);
    assert_eq!(h.count("game_bazaar_item_locations").await?, 0);
    let terminal_transitions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_bazaar_item_transitions WHERE escrow_id=$1::uuid AND kind IN ('DELIVER','RETURN')")
        .bind(race_escrow_id).fetch_one(&h.pool).await?;
    assert_eq!(terminal_transitions, 1);
    other
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    h.root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    eprintln!("GPROD_BAZAAR_DELIVERY_RETURN_RACE_EXECUTED independent_roots=2 committed=1 stale=1");
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=false WHERE singleton")
        .execute(&h.pool)
        .await?;
    assert_eq!(
        h.root
            .decide_bazaar_command(
                &authority,
                &h.node,
                SOURCE,
                "DeliverBazaarItem",
                &uuid(205),
                deliver.as_bytes()
            )
            .await,
        delivered
    );
    Ok(())
}
