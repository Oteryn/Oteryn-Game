//! Bazaar custody moves existing Game instances. Item projections never mint.
use super::*;
use crate::content::{
    DefinitionFamily, DefinitionRevisionRef, ProductionKey, ReferenceArtifactProjection,
    ReferenceItemField, ReferenceItemPhysicalClass, ReferenceItemStackClass,
    ReferencePlayableArtifactView, ReferenceServerItem, TypedDefinitionRef,
};
use serde_json::Value;
use std::io::Read;

const ITEM_REFUSAL: &str = "CHAR_CMD_BAZAAR_ITEM_NOT_TRANSFERABLE";
const STALE: &str = "CHAR_CMD_BAZAAR_STALE_REVISION";
const DESTINATION: &str = "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE";
const ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;

/// Immutable facts decoded from the bounded, integrity-checked artifact. Keeping
/// them with its identity avoids hashing/decoding megabytes during a DB pass.
#[derive(Debug)]
pub(super) struct PreparedArtifact {
    world_id: [u8; 16],
    digest: [u8; 32],
    items: Vec<(TypedDefinitionRef, ReferenceServerItem)>,
}

/// File acquisition and artifact validation run outside the semantic transaction.
/// The deciding transaction independently rechecks the exact configuration and
/// the current World activation before granting authority from these bytes.
pub(super) fn preload_artifact(policy: &Policy) -> Option<std::sync::Arc<PreparedArtifact>> {
    let (Some(path), Some(expected)) = (
        &policy.content_artifact_file,
        &policy.content_artifact_sha256,
    ) else {
        return None;
    };
    let before = std::fs::metadata(path).ok()?;
    if !before.is_file() || before.len() > ARTIFACT_BYTES {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > ARTIFACT_BYTES {
        return None;
    }
    let length = usize::try_from(metadata.len()).ok()?;
    let mut bytes = vec![0; length];
    let mut tail = [0u8; 1];
    if file.read_exact(&mut bytes).is_err()
        || file.read(&mut tail).ok() != Some(0)
        || hex(&Sha256::digest(&bytes)) != *expected
    {
        return None;
    }
    let artifact = ReferencePlayableArtifactView::load(
        &bytes,
        ReferenceArtifactProjection::ServerAuthoritative,
    )
    .ok()?;
    // The artifact profile and byte limit bound this complete typed index.
    let mut items = Vec::new();
    for identity in artifact.indexed_identities() {
        if identity.family() == DefinitionFamily::Item {
            items.push((
                identity.clone(),
                artifact.lookup_server_item(identity).ok()??,
            ));
        }
    }
    Some(std::sync::Arc::new(PreparedArtifact {
        world_id: *artifact.world_id().as_bytes(),
        digest: artifact.artifact_digest(),
        items,
    }))
}

/// Observe the already-loaded Content only after the owner has matched its
/// exact configured path and SHA. Native activation remains an independent,
/// current database fence, including the artifact's declared World identity.
pub(super) async fn admitted_artifact(
    tx: &mut Tx<'_>,
    policy: &Policy,
    world: &str,
) -> Result<Option<std::sync::Arc<PreparedArtifact>>, DurabilityError> {
    let Some(artifact) = policy.preloaded_artifact.as_ref().cloned() else {
        return Ok(None);
    };
    if artifact.world_id != super::super::character_authority::uuid_text(world)? {
        return Ok(None);
    }
    let digest = artifact.digest;
    // Issuance inserts cannot race this snapshot, including a previously unseen Channel.
    sqlx::query("LOCK TABLE game_content_activations IN SHARE MODE")
        .execute(&mut **tx)
        .await?;
    let rows = sqlx::query("SELECT a.server_artifact_digest FROM game_content_activations a WHERE a.world_id=$1::uuid AND a.activation_sequence=(SELECT max(b.activation_sequence) FROM game_content_activations b WHERE b.world_id=a.world_id AND b.channel_id=a.channel_id) ORDER BY a.channel_id LIMIT 257 FOR SHARE")
        .bind(world).fetch_all(&mut **tx).await?;
    if rows.is_empty()
        || rows.len() > 256
        || rows.iter().any(|row| {
            !row.try_get::<Vec<u8>, _>("server_artifact_digest")
                .is_ok_and(|value| value == digest)
        })
    {
        return Ok(None);
    }
    Ok(Some(artifact))
}

#[derive(Serialize, Deserialize)]
struct Definition {
    family: String,
    production_key: String,
    revision_ref: String,
}
#[derive(Serialize, Deserialize)]
struct Attribute {
    key: String,
    value: String,
}
#[derive(Serialize, Deserialize)]
struct Item {
    item_instance_id: String,
    world_id: String,
    definition: Definition,
    state_revision: String,
    quantity: u32,
    attributes_hash: String,
    attributes: Vec<Attribute>,
}
#[derive(Serialize)]
struct EscrowFacts<'a> {
    escrow_id: &'a str,
    escrow_revision: &'static str,
    seller_character_id: &'a str,
    item: &'a Item,
}
#[derive(Serialize)]
struct MoveFacts<'a> {
    escrow_id: &'a str,
    escrow_revision: String,
    destination_account_id: &'a str,
    destination_character_id: &'a str,
    delivery_id: &'a str,
    item: &'a Item,
}

pub(super) async fn apply(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    now: i64,
) -> Result<Effect, DurabilityError> {
    match r.command.as_str() {
        "EscrowBazaarItem" => escrow(tx, r, p, v, now).await,
        "DeliverBazaarItem" | "ReturnBazaarItem" => move_item(tx, r, p, v, now).await,
        _ => Ok(Effect::Rejected("CHAR_CMD_UNSUPPORTED")),
    }
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, DurabilityError> {
    serde_json::to_vec(value).map_err(|_| DurabilityError::InvalidStoredState)
}
fn json_string<T: Serialize>(value: &T) -> Result<String, DurabilityError> {
    serde_json::to_string(value).map_err(|_| DurabilityError::InvalidStoredState)
}

async fn escrow(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    _now: i64,
) -> Result<Effect, DurabilityError> {
    let world = field(v, "world_id")?;
    let seller = field(v, "seller_account_id")?;
    let character = field(v, "seller_character_id")?;
    let item_id = field(v, "item_instance_id")?;
    let listing = field(v, "listing_id")?;
    if p.item_classes.is_empty() || p.obligations_revision.is_none() {
        return Ok(Effect::Pending);
    }
    if p.obligations_revision != Some(number(v, "transferability_revision")?) {
        return Ok(Effect::Rejected(STALE));
    }
    let Some(root)=sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(character).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_FOUND"));};
    if root.try_get::<String, _>("account_id")? != seller
        || root.try_get::<String, _>("world_id")? != world
    {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    if root.try_get::<i16, _>("lifecycle")? != 1 {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));
    }
    if !super::transfer::offline(tx, character).await? {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_IN_SESSION"));
    }
    if sqlx::query("SELECT 1 FROM game_bazaar_escrows WHERE listing_id=$1::uuid LIMIT 1 FOR UPDATE")
        .bind(listing)
        .fetch_optional(&mut **tx)
        .await?
        .is_some()
    {
        return Ok(Effect::Rejected(STALE));
    }
    // Only direct, actually owned backpack entries are an admitted v1 source.
    // Other equipment and nested/depot state are refused rather than normalized.
    let row=sqlx::query("SELECT to_jsonb(i) AS item,to_jsonb(e) AS entry,coalesce(t.tier,0)::int AS tier FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) JOIN game_item_container_slots s ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_tiers t ON t.item_instance_id=i.item_instance_id WHERE i.item_instance_id=$1::uuid AND e.character_id=$2::uuid AND i.world_id=$3::uuid AND i.lifecycle=1 FOR UPDATE OF i,e")
        .bind(item_id).bind(character).bind(world).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    };
    let before: Value = row.try_get("item")?;
    let entry: Value = row.try_get("entry")?;
    let revision = before["state_revision"]
        .as_u64()
        .ok_or(DurabilityError::InvalidStoredState)?;
    if revision != number(v, "expected_state_revision")? {
        return Ok(Effect::Rejected(STALE));
    }
    let quantity = u32::try_from(
        v["quantity"]
            .as_u64()
            .ok_or(DurabilityError::InvalidStoredState)?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let whole = before["quantity"]
        .as_u64()
        .ok_or(DurabilityError::InvalidStoredState)?;
    if u64::from(quantity) > whole || quantity == 0 {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    let unknown:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_bindings WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_text_revisions WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=$1::uuid)")
        .bind(item_id).fetch_one(&mut **tx).await?;
    if unknown {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    let Some(artifact) = admitted_artifact(tx, p, world).await? else {
        return Ok(Effect::Pending);
    };
    let production_key = before["definition_production_key"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let revision_ref = before["definition_revision_ref"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    if stored_item_family(before["definition_family"].as_str()).is_none()
        || !wire_key(production_key)
        || !wire_key(revision_ref)
    {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    let Ok(key) = ProductionKey::new(production_key) else {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    };
    let Ok(def_revision) = DefinitionRevisionRef::new(revision_ref) else {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    };
    let identity = TypedDefinitionRef::new(DefinitionFamily::Item, key, def_revision);
    let Some((_, facts)) = artifact
        .items
        .iter()
        .find(|(candidate, _)| candidate == &identity)
    else {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    };
    if !facts.materializable || facts.physical_class != ReferenceItemPhysicalClass::Physical {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    if !matches!(&facts.semantics.trade_restrictions,ReferenceItemField::Known(restrictions) if restrictions.tradeable==ReferenceItemField::Known(true))
        || !matches!(&facts.semantics.physical,ReferenceItemField::Known(physical) if physical.pickupable==ReferenceItemField::Known(true))
    {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    // Any explicit temporal/charge capability requires a future exact-state profile.
    if matches!(facts.semantics.temporal, ReferenceItemField::Known(_))
        || matches!(facts.semantics.charges, ReferenceItemField::Known(_))
    {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    let class = if matches!(facts.semantics.container, ReferenceItemField::Known(_)) {
        "empty_container"
    } else {
        match facts.stack_class {
            ReferenceItemStackClass::NonStackable => "nonstackable",
            ReferenceItemStackClass::StackCapable => "stackable",
            ReferenceItemStackClass::Unknown => return Ok(Effect::Rejected(ITEM_REFUSAL)),
        }
    };
    if !p.item_classes.iter().any(|c| c == class) {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    if class != "stackable" && (quantity != 1 || whole != 1) {
        return Ok(Effect::Rejected(ITEM_REFUSAL));
    }
    if class == "stackable" {
        let ReferenceItemField::Known(stack) = &facts.semantics.stack else {
            return Ok(Effect::Rejected(ITEM_REFUSAL));
        };
        let ReferenceItemField::Known(maximum) = stack.stack_max else {
            return Ok(Effect::Rejected(ITEM_REFUSAL));
        };
        if stack.stackable != ReferenceItemField::Known(true) || whole > u64::from(maximum) {
            return Ok(Effect::Rejected(ITEM_REFUSAL));
        }
    }
    let tier: u32 = u32::try_from(row.try_get::<i32, _>("tier")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let attributes = if tier == 0 {
        "{}".to_owned()
    } else {
        format!("{{\"tier\":\"{tier}\"}}")
    };
    let hash = hex(&Sha256::digest(attributes.as_bytes()));
    let escrow_id = id(tx).await?;
    let transaction = id(tx).await?;
    let split = u64::from(quantity) < whole;
    let escrow_item = if split {
        id(tx).await?
    } else {
        item_id.to_owned()
    };
    let item = Item {
        item_instance_id: escrow_item.clone(),
        world_id: world.to_owned(),
        definition: Definition {
            family: "item".to_owned(),
            production_key: production_key.to_owned(),
            revision_ref: revision_ref.to_owned(),
        },
        state_revision: if split {
            "1".to_owned()
        } else {
            revision
                .checked_add(1)
                .ok_or(DurabilityError::Unavailable)?
                .to_string()
        },
        quantity,
        attributes_hash: hash,
        attributes: if tier == 0 {
            Vec::new()
        } else {
            vec![Attribute {
                key: "tier".to_owned(),
                value: tier.to_string(),
            }]
        },
    };
    sqlx::query("INSERT INTO game_bazaar_escrows(escrow_id,listing_id,seller_account_id,seller_character_id,world_id,item_instance_id,revision,state,item,attributes,attributes_hash,last_operation_id) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,$6::uuid,1,'ESCROWED',$7,$8,sha256($8),$9::uuid)")
        .bind(&escrow_id).bind(listing).bind(seller).bind(character).bind(world).bind(&escrow_item).bind(encode(&item)?).bind(attributes.as_bytes()).bind(&r.operation).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_bazaar_item_transitions(operation_id,transaction_id,escrow_id,item_instance_id,source_item_instance_id,kind,before_item,before_entry,quantity,split,source_character_id,issuer) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,'ESCROW',$6,$7,$8,$9,$10::uuid,$11)")
        .bind(&r.operation).bind(&transaction).bind(&escrow_id).bind(&escrow_item).bind(item_id).bind(&before).bind(&entry).bind(i64::from(quantity)).bind(split).bind(character).bind(&r.issuer).execute(&mut **tx).await?;
    if split {
        // The split is a conserved identity successor, never an entitlement mint.
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,state_revision=state_revision+1,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid").bind(item_id).bind(i64::from(quantity)).bind(&transaction).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id,last_transaction_id,state_revision) VALUES($1::uuid,$2::uuid,$7,$3,$4,$5,1,$6::uuid,NULL,1)")
            .bind(&escrow_item).bind(world).bind(production_key).bind(revision_ref).bind(i64::from(quantity)).bind(&transaction).bind(before["definition_family"].as_str().ok_or(DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?;
        if tier > 0 {
            sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier) VALUES($1::uuid,$2)")
                .bind(&escrow_item)
                .bind(i16::try_from(tier).map_err(|_| DurabilityError::InvalidStoredState)?)
                .execute(&mut **tx)
                .await?;
        }
    } else {
        sqlx::query("UPDATE game_item_instances SET state_revision=state_revision+1,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid").bind(item_id).bind(&transaction).execute(&mut **tx).await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid")
            .bind(item_id)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_bazaar_item_locations(item_instance_id,escrow_id) VALUES($1::uuid,$2::uuid)").bind(&escrow_item).bind(&escrow_id).execute(&mut **tx).await?;
    Ok(Effect::Committed(json_string(&EscrowFacts {
        escrow_id: &escrow_id,
        escrow_revision: "1",
        seller_character_id: character,
        item: &item,
    })?))
}

async fn move_item(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    _now: i64,
) -> Result<Effect, DurabilityError> {
    let escrow_id = field(v, "escrow_id")?;
    let world = field(v, "world_id")?;
    let seller = field(v, "seller_account_id")?;
    let Some(row)=sqlx::query("SELECT *,revision::text AS revision_text,seller_account_id::text AS seller_text,seller_character_id::text AS seller_character_text,listing_id::text AS listing_text,world_id::text AS world_text,item_instance_id::text AS item_text FROM game_bazaar_escrows WHERE escrow_id=$1::uuid FOR UPDATE").bind(escrow_id).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected(STALE));};
    let revision = row
        .try_get::<String, _>("revision_text")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if row.try_get::<String, _>("state")? != "ESCROWED"
        || revision != number(v, "expected_escrow_revision")?
        || row.try_get::<String, _>("seller_text")? != seller
        || row.try_get::<String, _>("world_text")? != world
        || row.try_get::<String, _>("listing_text")? != field(v, "listing_id")?
    {
        return Ok(Effect::Rejected(STALE));
    }
    let digest: Option<Vec<u8>> = row.try_get("escrow_receipt_digest")?;
    if digest.as_deref().map(hex).as_deref() != Some(field(v, "escrow_receipt_digest")?) {
        return Ok(Effect::Rejected(STALE));
    }
    let returning = r.command == "ReturnBazaarItem";
    let destination_account = if returning {
        seller
    } else {
        field(v, "buyer_account_id")?
    };
    let destination = field(
        v,
        if returning {
            "seller_character_id"
        } else {
            "buyer_character_id"
        },
    )?;
    let Some(root)=sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(destination).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected(DESTINATION));};
    if root.try_get::<String, _>("account_id")? != destination_account
        || root.try_get::<String, _>("world_id")? != world
        || root.try_get::<i16, _>("lifecycle")? != 1
    {
        return Ok(Effect::Rejected(DESTINATION));
    }
    let Some(capacity) = p.inbox_capacity.filter(|cap| *cap > 0) else {
        return Ok(Effect::Pending);
    };
    sqlx::query("INSERT INTO game_character_inbox_counters(character_id) VALUES($1::uuid) ON CONFLICT DO NOTHING").bind(destination).execute(&mut **tx).await?;
    let count:String=sqlx::query_scalar("SELECT committed::text FROM game_character_inbox_counters WHERE character_id=$1::uuid FOR UPDATE").bind(destination).fetch_one(&mut **tx).await?;
    if count
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?
        >= capacity
    {
        return Ok(Effect::Rejected(DESTINATION));
    }
    let item_id: String = row.try_get("item_text")?;
    if sqlx::query("SELECT 1 FROM game_bazaar_item_locations WHERE item_instance_id=$1::uuid AND escrow_id=$2::uuid FOR UPDATE")
        .bind(&item_id).bind(escrow_id).fetch_optional(&mut **tx).await?.is_none() {return Ok(Effect::Pending);}

    let before: Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=$1::uuid FOR UPDATE",
    )
    .bind(&item_id)
    .fetch_one(&mut **tx)
    .await?;
    let mut item: Item = serde_json::from_slice(&row.try_get::<Vec<u8>, _>("item")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if !returning {
        let Some(policy_json) = transferability(tx, p, world).await? else {
            return Ok(Effect::Pending);
        };
        let decisions: Value =
            serde_json::from_str(&policy_json).map_err(|_| DurabilityError::InvalidStoredState)?;
        let allowed = decisions.as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry["definition"]["family"].as_str() == Some(item.definition.family.as_str())
                    && entry["definition"]["production_key"].as_str()
                        == Some(item.definition.production_key.as_str())
                    && entry["definition"]["revision_ref"].as_str()
                        == Some(item.definition.revision_ref.as_str())
                    && entry["tradeable"].as_bool() == Some(true)
                    && entry["max_quantity"]
                        .as_u64()
                        .is_some_and(|maximum| u64::from(item.quantity) <= maximum)
            })
        });
        if !allowed {
            return Ok(Effect::Rejected(ITEM_REFUSAL));
        }
    }
    let attributes: Vec<u8> = row.try_get("attributes")?;
    if hex(&Sha256::digest(&attributes)) != item.attributes_hash {
        return Ok(Effect::Pending);
    }
    let tier: Option<i16> = sqlx::query_scalar(
        "SELECT tier FROM game_item_tiers WHERE item_instance_id=$1::uuid FOR SHARE",
    )
    .bind(&item_id)
    .fetch_optional(&mut **tx)
    .await?;
    let actual_attributes = tier.map_or_else(
        || "{}".to_owned(),
        |tier| format!("{{\"tier\":\"{tier}\"}}"),
    );
    if actual_attributes.as_bytes() != attributes {
        return Ok(Effect::Pending);
    }
    let state_revision = before["state_revision"]
        .as_u64()
        .ok_or(DurabilityError::InvalidStoredState)?;
    if item.state_revision != state_revision.to_string()
        || before["quantity"].as_u64() != Some(u64::from(item.quantity))
    {
        return Ok(Effect::Pending);
    }
    if !returning {
        match super::gold::fence_funding(
            tx,
            r,
            &v["funding"],
            destination_account,
            field(v, "sale_id")?,
            &r.operation,
        )
        .await?
        {
            Effect::Committed(_) => {}
            other => return Ok(other),
        }
    }
    let transaction = id(tx).await?;
    let delivery = id(tx).await?;
    let next = revision
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    item.state_revision = state_revision
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?
        .to_string();
    sqlx::query("INSERT INTO game_bazaar_item_transitions(operation_id,transaction_id,escrow_id,item_instance_id,source_item_instance_id,kind,before_item,quantity,split,source_character_id,destination_character_id,issuer) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$4::uuid,$5,$6,$7,false,$8::uuid,$9::uuid,$10)")
        .bind(&r.operation).bind(&transaction).bind(escrow_id).bind(&item_id).bind(if returning{"RETURN"}else{"DELIVER"}).bind(&before).bind(i64::from(item.quantity)).bind(row.try_get::<String,_>("seller_character_text")?).bind(destination).bind(&r.issuer).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_item_instances SET state_revision=state_revision+1,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid").bind(&item_id).bind(&transaction).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM game_bazaar_item_locations WHERE item_instance_id=$1::uuid AND escrow_id=$2::uuid").bind(&item_id).bind(escrow_id).execute(&mut **tx).await?;
    sqlx::query("SELECT * FROM game_character_inbox_deliver($1::uuid,$2::uuid,$3::uuid,$4,$5)")
        .bind(&item_id)
        .bind(destination)
        .bind(world)
        .bind(if returning {
            "BAZAAR_RETURN"
        } else {
            "BAZAAR_DELIVERY"
        })
        .bind(&r.operation)
        .fetch_one(&mut **tx)
        .await?;
    sqlx::query("UPDATE game_bazaar_escrows SET revision=revision+1,state=$2,item=$3,destination_account_id=$4::uuid,destination_character_id=$5::uuid,sale_id=$6::uuid,funding=$7,last_operation_id=$8::uuid WHERE escrow_id=$1::uuid")
        .bind(escrow_id).bind(if returning{"RETURNED"}else{"DELIVERED"}).bind(encode(&item)?).bind(destination_account).bind(destination).bind(if returning{None}else{Some(field(v,"sale_id")?)}).bind(if returning{None}else{Some(encode(&v["funding"])?)}).bind(&r.operation).execute(&mut **tx).await?;
    Ok(Effect::Committed(json_string(&MoveFacts {
        escrow_id,
        escrow_revision: next.to_string(),
        destination_account_id: destination_account,
        destination_character_id: destination,
        delivery_id: &delivery,
        item: &item,
    })?))
}

fn wire_key(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .as_bytes()
            .first()
            .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._:/-".contains(&byte)
        })
}

#[derive(Serialize)]
struct TransferabilityDefinition {
    definition: Definition,
    tradeable: bool,
    max_quantity: u32,
    allowed_attribute_keys: [&'static str; 1],
}

/// A complete policy snapshot, never a truncated eligibility list. Unknown
/// Content facts are absent and therefore deny; explicit false remains false.
pub(super) async fn transferability(
    tx: &mut Tx<'_>,
    policy: &Policy,
    world: &str,
) -> Result<Option<String>, DurabilityError> {
    let Some(artifact) = admitted_artifact(tx, policy, world).await? else {
        return Ok(None);
    };
    let mut definitions = Vec::new();
    for (identity, facts) in &artifact.items {
        if identity.family() != DefinitionFamily::Item
            || !wire_key(identity.key().as_str())
            || !wire_key(identity.revision().as_str())
        {
            continue;
        }
        if !facts.materializable || facts.physical_class != ReferenceItemPhysicalClass::Physical {
            continue;
        }
        let ReferenceItemField::Known(restrictions) = &facts.semantics.trade_restrictions else {
            continue;
        };
        let ReferenceItemField::Known(tradeable) = restrictions.tradeable else {
            continue;
        };
        let ReferenceItemField::Known(physical) = &facts.semantics.physical else {
            continue;
        };
        let ReferenceItemField::Known(pickupable) = physical.pickupable else {
            continue;
        };
        if matches!(facts.semantics.temporal, ReferenceItemField::Known(_))
            || matches!(facts.semantics.charges, ReferenceItemField::Known(_))
        {
            continue;
        }
        let (class, maximum) = if matches!(facts.semantics.container, ReferenceItemField::Known(_))
        {
            ("empty_container", 1)
        } else {
            match facts.stack_class {
                ReferenceItemStackClass::NonStackable => ("nonstackable", 1),
                ReferenceItemStackClass::StackCapable => {
                    let ReferenceItemField::Known(stack) = &facts.semantics.stack else {
                        continue;
                    };
                    let ReferenceItemField::Known(maximum) = stack.stack_max else {
                        continue;
                    };
                    if stack.stackable != ReferenceItemField::Known(true) || maximum == 0 {
                        continue;
                    }
                    ("stackable", u32::from(maximum))
                }
                ReferenceItemStackClass::Unknown => continue,
            }
        };
        definitions.push(TransferabilityDefinition {
            definition: Definition {
                family: "item".to_owned(),
                production_key: identity.key().as_str().to_owned(),
                revision_ref: identity.revision().as_str().to_owned(),
            },
            tradeable: tradeable
                && pickupable
                && policy.item_classes.iter().any(|allowed| allowed == class),
            max_quantity: maximum,
            allowed_attribute_keys: ["tier"],
        });
        if definitions.len() > 1024 {
            return Ok(None);
        }
    }
    definitions.sort_by(|a, b| {
        (
            &a.definition.family,
            &a.definition.production_key,
            &a.definition.revision_ref,
        )
            .cmp(&(
                &b.definition.family,
                &b.definition.production_key,
                &b.definition.revision_ref,
            ))
    });
    Ok(Some(json_string(&definitions)?))
}

// Native typed enum serialization and the already-canonical contract spelling.
// This closed mapping never changes a stored definition or accepts other families.
fn stored_item_family(family: Option<&str>) -> Option<&'static str> {
    match family {
        Some("Item" | "item") => Some("item"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_retained_tier_state_matches_bazaar_fixture_digest() {
        assert_eq!(
            hex(&Sha256::digest(br#"{"tier":"2"}"#)),
            "fd8184df56101f486d8766c87c216edddf9c25c4f29ba4c48eb48219dc137c19"
        );
        assert_eq!(stored_item_family(Some("Item")), Some("item"));
        assert_eq!(stored_item_family(Some("item")), Some("item"));
        for family in ["ITEM", "iTem", "Container", ""] {
            assert_eq!(stored_item_family(Some(family)), None);
        }
        assert_eq!(stored_item_family(None), None);
        assert!(!wire_key("Item"));
        assert!(!wire_key("definition key"));
        assert!(wire_key("fixture-sword"));
    }
}
