//! P07-specific audit bounds and carried/stash source proofs. Other fee causes keep their rows.
use super::*;
use serde_json::Value;

mod materials;

pub(crate) const COIN_INPUTS_MAX: usize = 508;
pub(crate) const MATERIAL_INPUTS_MAX: usize = 30;
pub(crate) const ASSET_LINES_MAX: u64 = 540;
pub(crate) const EFFECT_WORK_MAX: u64 = 1_620;
pub(crate) const VALUE_LINES_MAX: u64 = 2;
pub(crate) const ENVELOPE_BYTES_MAX: usize = 262_144;
// The existing global durable-audit payload ceiling remains smaller than its envelope.
pub(crate) const PAYLOAD_BYTES_MAX: usize = 196_608;

pub(super) fn cause(value: &OneItemFeeBurnV1) -> Option<&OneItemNpcPurchaseV1> {
    match value.cause.as_ref().and_then(|cause| cause.cause.as_ref()) {
        Some(FeeBurnCauseV1::GemReveal(cause) | FeeBurnCauseV1::GemGradeUp(cause) | FeeBurnCauseV1::GemSwitchDomain(cause)) => Some(cause),
        _ => None,
    }
}

fn proof(cause: &OneItemNpcPurchaseV1, value: &OneItemFeeBurnV1) -> Result<Value, AuditError> {
    check_uuid_v7(&cause.occurrence)?;
    let proof: Value = serde_json::from_slice(&cause.atelier_proof).map_err(|_| AuditError::InvalidInput)?;
    if proof["occurrence"] != serde_json::json!(cause.occurrence) || proof["character"] != serde_json::json!(value.character_id) {
        return Err(AuditError::InvalidInput);
    }
    Ok(proof)
}

pub(super) fn check(cause: &OneItemNpcPurchaseV1, value: &OneItemFeeBurnV1, action: &str) -> Result<(), AuditError> {
    let proof = proof(cause, value)?;
    let inputs = proof["inputs"].as_array().ok_or(AuditError::InvalidInput)?;
    if proof["mutation"].get(action).is_none() || proof["outputs"].as_array().is_none_or(|v| !v.is_empty()) || inputs.len() > MATERIAL_INPUTS_MAX {
        return Err(AuditError::InvalidInput);
    }
    // The new writer marks its carried-source form. Historical envelopes keep their original
    // accepted shape; new proofs must carry every source and before/after material fact.
    if proof.get("coin_inputs").is_some() {
        materials::check(&proof, value)?;
    }
    let (needed, key) = if action == "Reveal" {
        let fee = match proof["mutation"]["Reveal"]["quality"].as_str() { Some("Lesser") => 125_000, Some("Regular") => 1_000_000, Some("Greater") => 6_000_000, _ => return Err(AuditError::InvalidInput) };
        if fee != value.fee_gold_units || inputs.len() != 1 || !proof["stash"].is_null() { return Err(AuditError::InvalidInput); }
        (1, None)
    } else {
        let (needed, key) = match (proof["mutation"]["GradeUp"]["kind"].as_str(), value.fee_gold_units) {
            (Some("Basic"), 2_000_000) => (5, "oteryn:item.tibia.i46625"),
            (Some("Basic"), 5_000_000) => (15, "oteryn:item.tibia.i46625"),
            (Some("Basic"), 30_000_000) => (30, "oteryn:item.tibia.i46625"),
            (Some("Supreme"), 5_000_000) => (5, "oteryn:item.tibia.i46626"),
            (Some("Supreme"), 12_000_000) => (15, "oteryn:item.tibia.i46626"),
            (Some("Supreme"), 75_000_000) => (30, "oteryn:item.tibia.i46626"),
            _ => return Err(AuditError::InvalidInput),
        };
        (needed, Some(key))
    };
    let mut total = 0_u64;
    for input in inputs {
        let count = input["count"].as_u64().filter(|n| (1..=100).contains(n)).ok_or(AuditError::InvalidInput)?;
        if key.is_some_and(|key| input["before"]["key"] != key) { return Err(AuditError::InvalidInput); }
        total = total.checked_add(count).ok_or(AuditError::InvalidInput)?;
    }
    if !proof["stash"].is_null() {
        let stash = &proof["stash"];
        let count = stash["count"].as_u64().filter(|n| (1..=30).contains(n)).ok_or(AuditError::InvalidInput)?;
        if action != "GradeUp" || key.is_none_or(|key| stash["key"] != key)
            || stash["quantity_before"].as_u64().is_none_or(|before| before < count || before > 1_000_000_000)
            || stash["revision"].as_str().is_none_or(|revision| revision.is_empty() || revision.len() > mint::RL07_CONTENT_KEY_BYTES_MAX)
            || stash["entry"].as_str().is_none_or(|id| uuid(id).is_err())
            || stash["previous"].as_str().is_none_or(|id| uuid(id).is_err()) {
            return Err(AuditError::InvalidInput);
        }
        total = total.checked_add(count).ok_or(AuditError::InvalidInput)?;
    }
    if total != needed { return Err(AuditError::InvalidInput); }
    Ok(())
}

pub(super) fn check_switch(cause: &OneItemNpcPurchaseV1, value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    if cause.atelier_proof.is_empty() { return check_uuid_v7(&cause.occurrence); }
    let proof = proof(cause, value)?;
    if !proof["stash"].is_null() || !proof["coin_inputs"].is_array() { return Err(AuditError::InvalidInput); }
    Ok(())
}

#[derive(serde::Deserialize)]
pub(super) struct CoinSource {
    item: String,
    surface: u8,
    parent: Option<String>,
    ordinal: u64,
    slot: u8,
    pub(super) selection_order: u64,
}

impl CoinSource {
    pub(super) fn main_ordinal(&self) -> Option<u64> {
        (self.surface == 1).then_some(self.ordinal)
    }
}

fn uuid(value: &str) -> Result<[u8; 16], AuditError> {
    if value.len() != 36 || [8, 13, 18, 23].into_iter().any(|i| value.as_bytes()[i] != b'-') { return Err(AuditError::InvalidInput); }
    let bytes = u128::from_str_radix(&value.replace('-', ""), 16).map_err(|_| AuditError::InvalidInput)?.to_be_bytes();
    check_uuid_v7(&bytes)?;
    Ok(bytes)
}

pub(super) fn coin_sources(value: &OneItemFeeBurnV1) -> Result<Option<Vec<CoinSource>>, AuditError> {
    let Some(cause) = cause(value) else { return Ok(None); };
    if cause.atelier_proof.is_empty() { return Ok(None); }
    let proof: Value = serde_json::from_slice(&cause.atelier_proof).map_err(|_| AuditError::InvalidInput)?;
    let Some(sources) = proof.get("coin_inputs") else { return Ok(None); };
    let sources: Vec<CoinSource> = serde_json::from_value(sources.clone()).map_err(|_| AuditError::InvalidInput)?;
    if sources.len() != value.lines.len() || sources.len() > COIN_INPUTS_MAX { return Err(AuditError::InvalidInput); }
    for (index, (source, line)) in sources.iter().zip(&value.lines).enumerate() {
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        if uuid(&source.item)?.as_slice() != before.item_instance_id.as_slice() || source.ordinal != line.placement_ordinal
            || !(1..=509).contains(&source.selection_order)
            || sources[..index].iter().any(|previous| previous.selection_order == source.selection_order) { return Err(AuditError::InvalidInput); }
        match source.surface {
            1 | 2 if source.slot == 0 => {
                let parent = uuid(source.parent.as_deref().ok_or(AuditError::InvalidInput)?)?;
                if source.surface == 1 && parent.as_slice() != value.backpack_item_instance_id.as_slice() { return Err(AuditError::InvalidInput); }
            }
            3 if source.parent.is_none() && (1..=10).contains(&source.slot) && source.slot != 9 && source.ordinal == u64::from(source.slot) => (),
            _ => return Err(AuditError::InvalidInput),
        }
    }
    Ok(Some(sources))
}

pub(super) fn usage(cause: &OneItemNpcPurchaseV1) -> TransactionResourceUsage {
    let proof = serde_json::from_slice::<Value>(&cause.atelier_proof).unwrap_or(Value::Null);
    let count = proof["inputs"].as_array().map_or(0, |inputs| inputs.len() as u64);
    let stash = u64::from(!proof["stash"].is_null());
    TransactionResourceUsage { touched_item_instances: count, location_custody_lines: count, value_lines: stash, transform_lines: 0, container_expansion: 0, participants: count + stash, effect_work_units: 3 * count + 2 * stash, events: 0 }
}

pub(crate) fn check_usage(usage: &TransactionResourceUsage) -> Result<(), AuditError> {
    if usage.value_lines > VALUE_LINES_MAX || usage.transform_lines != 0 || usage.container_expansion != 0 { return Err(AuditError::InvalidInput); }
    if usage.touched_item_instances > ASSET_LINES_MAX || usage.location_custody_lines > ASSET_LINES_MAX || usage.participants > ASSET_LINES_MAX || usage.effect_work_units > EFFECT_WORK_MAX || usage.events > 1 { return Err(AuditError::CapacityExceeded); }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test fixture assertions")]
    use super::*;
    use serde_json::json;

    fn grade(stash: u32, carried: u32) -> (OneItemNpcPurchaseV1, OneItemFeeBurnV1) {
        let mut id = vec![1; 16]; id[6] = 0x70; id[8] = 0x80;
        let value = OneItemFeeBurnV1 { character_id: id.clone(), fee_gold_units: 2_000_000, ..Default::default() };
        let mut inputs = Vec::new();
        if carried > 0 { inputs.push(json!({"count":carried,"before":{"key":"oteryn:item.tibia.i46625"}})); }
        let stash = (stash > 0).then(|| json!({"key":"oteryn:item.tibia.i46625","revision":"revision","count":stash,"quantity_before":100,"entry":"01010101-0101-7001-8001-010101010101","previous":"01010101-0101-7001-8001-010101010102"}));
        let proof = json!({"character":id,"occurrence":id,"mutation":{"GradeUp":{"kind":"Basic","mod_id":0}},"inputs":inputs,"outputs":[],"stash":stash});
        (OneItemNpcPurchaseV1 { service_key:"oteryn:atelier.grade_up".into(),occurrence:id,atelier_proof:serde_json::to_vec(&proof).unwrap() }, value)
    }

    #[test]
    fn atelier_audit_accepts_stash_and_mixed_materials_and_refuses_wrong_totals() {
        for (stash, carried) in [(5, 0), (3, 2), (0, 5)] { let (cause, value) = grade(stash, carried); assert_eq!(check(&cause, &value, "GradeUp"), Ok(())); assert_eq!(usage(&cause).value_lines, u64::from(stash > 0)); }
        for (stash, carried) in [(0, 0), (2, 2), (5, 1)] { let (cause, value) = grade(stash, carried); assert_eq!(check(&cause, &value, "GradeUp"), Err(AuditError::InvalidInput)); }
        let (cause, value) = grade(5, 0); assert_eq!(check(&cause, &value, "Reveal"), Err(AuditError::InvalidInput));
    }

    #[test]
    fn atelier_audit_accepts_thirty_material_leaves_but_not_thirty_one() {
        let (mut cause, mut value) = grade(0, 5);
        value.fee_gold_units = 30_000_000;
        let mut proof: Value = serde_json::from_slice(&cause.atelier_proof).unwrap();
        proof["inputs"] = json!(vec![json!({"count":1,"before":{"key":"oteryn:item.tibia.i46625"}});30]);
        cause.atelier_proof=serde_json::to_vec(&proof).unwrap();
        assert_eq!(check(&cause,&value,"GradeUp"),Ok(()));
        proof["inputs"].as_array_mut().unwrap().push(json!({"count":1,"before":{"key":"oteryn:item.tibia.i46625"}}));
        cause.atelier_proof=serde_json::to_vec(&proof).unwrap();
        assert_eq!(check(&cause,&value,"GradeUp"),Err(AuditError::InvalidInput));
    }

    #[test]
    fn atelier_coin_audit_binds_current_source_shape_and_item_identity() {
        let item="01010101-0101-7001-8001-010101010101";
        let parent="01010101-0101-7001-8001-010101010102";
        let mut value=OneItemFeeBurnV1 { backpack_item_instance_id:uuid(parent).unwrap().to_vec(), lines:vec![OneItemFeeBurnLineV1 { before:Some(OneItemStateV1 { item_instance_id:uuid(item).unwrap().to_vec(), ..Default::default() }),placement_ordinal:7,..Default::default() }],..Default::default() };
        let source=json!({"item":item,"parent":parent,"surface":1,"ordinal":7,"slot":0,"selection_order":1});
        let set=|value:&mut OneItemFeeBurnV1,source:Value| { value.cause=Some(OneItemFeeBurnCauseV1 {cause:Some(FeeBurnCauseV1::GemSwitchDomain(OneItemNpcPurchaseV1 {atelier_proof:serde_json::to_vec(&json!({"coin_inputs":[source]})).unwrap(),..Default::default()}))}); };
        set(&mut value,source.clone()); assert!(coin_sources(&value).unwrap().is_some());
        for rank in [0, 510, u64::MAX] { let mut invalid = source.clone(); invalid["selection_order"] = json!(rank); set(&mut value, invalid); assert!(coin_sources(&value).is_err()); }
        let mut nested=source.clone(); nested["surface"]=json!(2); nested["ordinal"]=json!(7); value.backpack_item_instance_id.clear();
        set(&mut value,nested.clone()); assert!(coin_sources(&value).unwrap().is_some());
        nested["parent"]=Value::Null; set(&mut value,nested); assert!(coin_sources(&value).is_err());
        let mut equipped=source.clone(); equipped["surface"]=json!(3); equipped["parent"]=Value::Null; equipped["slot"]=json!(7);
        set(&mut value,equipped.clone()); assert!(coin_sources(&value).unwrap().is_some());
        equipped["item"]=json!(parent); set(&mut value,equipped); assert!(coin_sources(&value).is_err());
        set(&mut value,source); assert!(coin_sources(&value).is_err(),"main source requires its actual backpack identity");
    }

    #[test]
    fn atelier_audit_budgets_match_registry_and_refuse_each_overflow() {
        let registry: Value = serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON).unwrap();
        for (id, bound) in [("ATELIER-RL-ASSET-LINES",ASSET_LINES_MAX),("ATELIER-RL-EFFECT-WORK",EFFECT_WORK_MAX),("ATELIER-RL-VALUE-LINES",VALUE_LINES_MAX),("ATELIER-RL-AUDIT-BYTES",ENVELOPE_BYTES_MAX as u64)] {
            let rows: Vec<_> = registry["entries"].as_array().unwrap().iter().filter(|row| row["id"] == id).collect(); assert_eq!(rows.len(),1); assert_eq!(rows[0]["hard_maximum"],bound);
        }
        let usage = TransactionResourceUsage { touched_item_instances: ASSET_LINES_MAX, location_custody_lines: ASSET_LINES_MAX, value_lines: VALUE_LINES_MAX, transform_lines: 0, container_expansion: 0, participants: ASSET_LINES_MAX, effect_work_units: EFFECT_WORK_MAX, events: 1 };
        assert_eq!(check_usage(&usage), Ok(()));
        for field in 0..5 { let mut over=usage; match field { 0 => over.touched_item_instances+=1, 1 => over.location_custody_lines+=1, 2 => over.participants+=1, 3 => over.effect_work_units+=1, _ => over.events+=1 }; assert_eq!(check_usage(&over),Err(AuditError::CapacityExceeded)); }
        let mut over=usage; over.value_lines+=1; assert_eq!(check_usage(&over),Err(AuditError::InvalidInput));
        assert_eq!(super::super::check_fee_usage(&usage),Err(AuditError::InvalidInput));
        // A canonical Atelier payload reaches semantic validation at the ceiling.
        // One byte more must fail the size gate before inspecting those missing facts.
        let payload = |padding| mint::OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(mint::OneItemOperationV1::FeeBurn(OneItemFeeBurnV1 {
                cause: Some(OneItemFeeBurnCauseV1 {
                    cause: Some(FeeBurnCauseV1::GemSwitchDomain(OneItemNpcPurchaseV1 {
                        atelier_proof: vec![b' '; padding],
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })),
        }.encode_to_vec();
        let overhead = payload(PAYLOAD_BYTES_MAX).len() - PAYLOAD_BYTES_MAX;
        let at_limit = payload(PAYLOAD_BYTES_MAX - overhead);
        let over_limit = payload(PAYLOAD_BYTES_MAX - overhead + 1);
        assert_eq!(at_limit.len(), 196_608);
        assert_eq!(over_limit.len(), 196_609);
        assert_eq!(super::super::decode_fee_burn_payload(&at_limit).map(|_| ()), Err(AuditError::InvalidInput));
        assert_eq!(super::super::decode_fee_burn_payload(&over_limit).map(|_| ()), Err(AuditError::CapacityExceeded));
        let at_limit = vec![0xff; ENVELOPE_BYTES_MAX];
        let over_limit = vec![0xff; ENVELOPE_BYTES_MAX + 1];
        assert_eq!(over_limit.len(), 262_145);
        assert_eq!(mint::decode_common_envelope_within(&at_limit, ENVELOPE_BYTES_MAX).map(|_| ()), Err(AuditError::InvalidInput));
        assert_eq!(super::super::decode_fee_burn_envelope(&over_limit).map(|_| ()), Err(AuditError::CapacityExceeded));
        assert_eq!(mint::decode_common_envelope_within(&over_limit, ENVELOPE_BYTES_MAX).map(|_| ()), Err(AuditError::CapacityExceeded));
    }
}
