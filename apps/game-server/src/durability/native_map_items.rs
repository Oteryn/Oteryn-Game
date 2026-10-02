//! Source-map Item initialization on the existing Item/Ground owner. The caller
//! holds the actual physical map owner throughout this transaction and commits
//! through the durability semantic deadline; this helper never commits.
use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::native_map_items_abi::*;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_item_transaction::SpellItemError;
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, SpellItemError>;
fn hex(v: &[u8]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
fn intent(binding: &NativeMapOwnerBinding, p: &NativeMapItemPlacement) -> Result<Vec<u8>> {
    serde_json::to_vec(&json!({"schema":"OTERYN_NATIVE_MAP_ITEM_INIT/v1",
        "world":hex(binding.world.as_bytes()),"channel":hex(binding.channel.as_bytes()),
        "scope_generation":binding.scope_generation.to_string(),
        "content_digest":hex(&binding.content_digest),"map_digest":hex(&binding.map_digest),"frame_digest":hex(&binding.frame_digest),
        "placement_key":p.placement_key,"definition_family":p.definition.family,"definition_key":p.definition.production_key,
        "definition_revision":p.definition.revision_ref,"quantity":p.quantity,
        "spatial_position":hex(&p.ground.spatial_position),"source_binding":hex(&p.source_binding),"source_attributes":p.source_attributes,
        "blocks_movement":p.blocks_movement,"blocks_projectile":p.blocks_projectile,"immovable_block_solid":p.immovable_block_solid,"owner_kind":p.owner_kind
    })).map_err(|_| SpellItemError::Rejected("map intent serialization"))
}

pub(crate) async fn initialize_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    proof: &(impl NativeMapInitializationProof + ?Sized),
) -> Result<Vec<MaterializedNativeMapItem>> {
    let binding = proof.current_binding().map_err(SpellItemError::Rejected)?;
    if binding.scope_generation == 0 || proof.placements().len() > 128 {
        return Err(SpellItemError::Rejected("map initialization budget/scope"));
    }
    let record = recovery
        .record_for(root)
        .map_err(|_| SpellItemError::Rejected("map recovery authority"))?;
    assert_recovery_fence(tx, &record).await?;
    super::db::lock_admission_relations(tx).await?;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(binding.world, binding.channel);
    let guard=super::admission_authority_guards::AdmissionGuardStore::from_root(root.clone())
        .load_locked(tx,&crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(scope)).await?
        .ok_or(SpellItemError::Rejected("map runtime publication absent"))?;
    if !matches!(guard.state,crate::foundation::admission_authority_publication::AdmissionAuthorityGuardStateV1::Runtime {ready:true,ownership_generation,..} if ownership_generation==binding.scope_generation)
    {
        return Err(SpellItemError::Rejected(
            "map held runtime publication mismatch",
        ));
    }
    let fact = node.fact();
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE s.world_id=encode($1,'hex')::uuid AND s.channel_id=encode($2,'hex')::uuid AND s.state=1 AND s.ownership_generation=$3::text::numeric(20,0) AND s.holder_node_id=encode($4,'hex')::uuid AND s.holder_registration_revision=$5::text::numeric(20,0) AND g.ready AND g.ownership_generation=s.ownership_generation)")
        .bind(binding.world.as_bytes().as_slice()).bind(binding.channel.as_bytes().as_slice()).bind(binding.scope_generation.to_string())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
    if !current || !super::runtime_scope_assignment::prove_current_incarnation(tx, node).await? {
        return Err(SpellItemError::Rejected("current map scope/node authority"));
    }
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'), 33))",
    )
    .bind(binding.world.as_bytes().as_slice())
    .bind(binding.channel.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    let mut keys = BTreeSet::new();
    let mut out = Vec::new();
    for p in proof.placements() {
        if !keys.insert(&p.placement_key)
            || p.definition.family != "Item"
            || p.quantity == 0
            || p.ground.spatial_position.len() != 10
            || p.ground.map_revision != format!("sha256:{}", hex(&binding.map_digest))
            || p.ground.content_revision != format!("sha256:{}", hex(&binding.content_digest))
            || p.ground.native_room_placement_context != binding.frame_digest
        {
            return Err(SpellItemError::Rejected("map placement binding"));
        }
        let bytes = intent(&binding, p)?;
        // This scope's initializer holds the advisory transaction lock above.
        // Receipts are immutable; SELECT needs no receipt UPDATE privilege.
        let previous=sqlx::query("SELECT uuid_send(transaction_id),uuid_send(item_instance_id),source_intent,scope_generation::text FROM game_native_map_item_receipts WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND content_digest=$3 AND placement_key=$4")
            .bind(binding.world.as_bytes().as_slice()).bind(binding.channel.as_bytes().as_slice()).bind(binding.content_digest.as_slice()).bind(&p.placement_key).fetch_optional(&mut **tx).await?;
        if let Some(row) = previous {
            let original: serde_json::Value =
                serde_json::from_slice(&row.try_get::<Vec<u8>, _>(2)?)
                    .map_err(|_| SpellItemError::Rejected("map historical intent"))?;
            let mut previous = original.clone();
            let mut current: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|_| SpellItemError::Rejected("map current intent"))?;
            previous
                .as_object_mut()
                .ok_or(SpellItemError::Rejected("map historical intent shape"))?
                .remove("scope_generation");
            current
                .as_object_mut()
                .ok_or(SpellItemError::Rejected("map current intent shape"))?
                .remove("scope_generation");
            if previous != current {
                return Err(SpellItemError::Rejected(
                    "map initialization replay mismatch",
                ));
            }
            let prior_generation = row
                .try_get::<String, _>(3)?
                .parse::<u64>()
                .map_err(|_| SpellItemError::Rejected("map source generation"))?;
            if prior_generation > binding.scope_generation {
                return Err(SpellItemError::Rejected("map generation went backwards"));
            }
            if prior_generation < binding.scope_generation {
                adopt_current_ground(
                    tx,
                    &binding,
                    node,
                    &row.try_get::<Vec<u8>, _>(0)?,
                    &row.try_get::<Vec<u8>, _>(1)?,
                )
                .await?;
            }
            out.push(MaterializedNativeMapItem {
                placement_key: p.placement_key.clone(),
                transaction_id: row
                    .try_get::<Vec<u8>, _>(0)?
                    .try_into()
                    .map_err(|_| SpellItemError::Rejected("map transaction UUID"))?,
                item_instance_id: row
                    .try_get::<Vec<u8>, _>(1)?
                    .try_into()
                    .map_err(|_| SpellItemError::Rejected("map Item UUID"))?,
                replayed: true,
            });
            // History proves the original mint only. Never recreate moved,
            // picked-up, transformed or retired instances from this receipt.
            continue;
        }
        let ids=sqlx::query("SELECT uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7())").fetch_one(&mut **tx).await?;
        let transaction: Vec<u8> = ids.try_get(0)?;
        let event: Vec<u8> = ids.try_get(1)?;
        let item: Vec<u8> = ids.try_get(2)?;
        sqlx::query("INSERT INTO game_native_map_item_receipts(transaction_id,event_id,item_instance_id,world_id,channel_id,scope_generation,holder_node_id,holder_registration_revision,content_digest,map_digest,frame_digest,placement_key,definition_family,definition_key,definition_revision,quantity,spatial_position,map_revision,content_revision,placement_context,source_binding,source_attributes,blocks_movement,blocks_projectile,immovable_block_solid,owner_kind,binding,source_intent) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric(20,0),encode($7,'hex')::uuid,$8::text::numeric(20,0),$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,sha256($27),$27)")
            .bind(&transaction).bind(&event).bind(&item).bind(binding.world.as_bytes().as_slice()).bind(binding.channel.as_bytes().as_slice()).bind(binding.scope_generation.to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
            .bind(binding.content_digest.as_slice()).bind(binding.map_digest.as_slice()).bind(binding.frame_digest.as_slice()).bind(&p.placement_key).bind(&p.definition.family).bind(&p.definition.production_key).bind(&p.definition.revision_ref).bind(i64::from(p.quantity))
            .bind(&p.ground.spatial_position).bind(&p.ground.map_revision).bind(&p.ground.content_revision).bind(&p.ground.native_room_placement_context).bind(&p.source_binding).bind(&p.source_attributes).bind(p.blocks_movement).bind(p.blocks_projectile).bind(p.immovable_block_solid).bind(&p.owner_kind).bind(&bytes).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id,last_transaction_id,state_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,1,encode($7,'hex')::uuid,NULL,1)")
            .bind(&item).bind(binding.world.as_bytes().as_slice()).bind(&p.definition.family).bind(&p.definition.production_key).bind(&p.definition.revision_ref).bind(i64::from(p.quantity)).bind(&transaction).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5,$6,$7,$8,$9)")
            .bind(&item).bind(binding.world.as_bytes().as_slice()).bind(binding.channel.as_bytes().as_slice()).bind(binding.scope_generation.to_string()).bind(&p.ground.spatial_position).bind(&p.ground.corpse_ref).bind(&p.ground.map_revision).bind(&p.ground.content_revision).bind(&p.ground.native_room_placement_context).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_native_map_item_audit(event_id,transaction_id,item_instance_id,envelope,envelope_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,sha256($4))")
            .bind(&event).bind(&transaction).bind(&item).bind(&bytes).execute(&mut **tx).await?;
        out.push(MaterializedNativeMapItem {
            placement_key: p.placement_key.clone(),
            transaction_id: transaction
                .try_into()
                .map_err(|_| SpellItemError::Rejected("map transaction UUID"))?,
            item_instance_id: item
                .try_into()
                .map_err(|_| SpellItemError::Rejected("map Item UUID"))?,
            replayed: false,
        });
    }
    if proof.current_binding().map_err(SpellItemError::Rejected)? != binding {
        return Err(SpellItemError::Rejected("map owner changed before commit"));
    }
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(out)
}

impl DurabilityRoot {
    /// Callable admission/activation owner port. The physical Channel owner and
    /// qualified active room stay borrowed throughout the bounded pass. Unknown
    /// COMMIT is reconciled by the same immutable placement identities on retry;
    /// it cannot create a second map stack or revive an already moved Item.
    pub(crate) async fn initialize_native_map_current_owner<P>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        proof: &P,
    ) -> Result<Vec<MaterializedNativeMapItem>>
    where
        P: NativeMapInitializationProof + Sync,
    {
        let mut context = (self.clone(), recovery, node.clone(), proof);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let outcome = initialize_in_transaction(
                        &mut tx, &context.0, context.1, &context.2, context.3,
                    )
                    .await;
                    match outcome {
                        Ok(items) => {
                            super::db::commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(items))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn adopt_current_ground(
    tx: &mut Transaction<'_, Postgres>,
    binding: &NativeMapOwnerBinding,
    node: &NodeIncarnationProof,
    source_transaction: &[u8],
    item: &[u8],
) -> Result<()> {
    let previous:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_native_map_scope_adoptions WHERE source_transaction_id=encode($1,'hex')::uuid AND ownership_generation=$2::text::numeric(20,0))")
        .bind(source_transaction).bind(binding.scope_generation.to_string()).fetch_one(&mut **tx).await?;
    if previous {
        return Ok(());
    }
    let exists=sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE").bind(item).fetch_optional(&mut **tx).await?;
    if exists.is_none() {
        return Err(SpellItemError::Rejected("map source Item absent"));
    }
    // The actual item row is already locked above. Custody writers and the
    // location-exclusivity trigger serialize on that item; Ground is immutable.
    let ground=sqlx::query("SELECT to_jsonb(g),world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND map_revision=$4 AND content_revision=$5 AND native_room_placement_context=$6 AS compatible,runtime_scope_ownership_generation::text FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item).bind(binding.world.as_bytes().as_slice()).bind(binding.channel.as_bytes().as_slice()).bind(format!("sha256:{}",hex(&binding.map_digest))).bind(format!("sha256:{}",hex(&binding.content_digest))).bind(binding.frame_digest.as_slice()).fetch_optional(&mut **tx).await?;
    let before = match ground {
        Some(row) if row.try_get::<bool, _>(1)? => {
            let generation = row
                .try_get::<String, _>(2)?
                .parse::<u64>()
                .map_err(|_| SpellItemError::Rejected("map Ground generation"))?;
            if generation > binding.scope_generation {
                return Err(SpellItemError::Rejected("map Ground future generation"));
            }
            if generation == binding.scope_generation {
                None
            } else {
                Some(row.try_get::<serde_json::Value, _>(0)?)
            }
        }
        _ => None,
    };
    let fact = node.fact();
    sqlx::query("INSERT INTO game_native_map_scope_adoptions(source_transaction_id,ownership_generation,holder_node_id,holder_registration_revision,ground_before) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid,$4::text::numeric(20,0),$5)")
        .bind(source_transaction).bind(binding.scope_generation.to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(&before).execute(&mut **tx).await?;
    if let Some(before) = before {
        let changed=sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=$2::text::numeric(20,0) WHERE item_instance_id=encode($1,'hex')::uuid AND to_jsonb(game_item_ground_locations)=$3")
            .bind(item).bind(binding.scope_generation.to_string()).bind(before).execute(&mut **tx).await?.rows_affected();
        if changed != 1 {
            return Err(SpellItemError::Rejected(
                "map Ground changed during custody handoff",
            ));
        }
    }
    Ok(())
}
