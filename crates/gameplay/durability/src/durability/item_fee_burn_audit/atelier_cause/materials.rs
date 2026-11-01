//! Semantic material sources in the carried-coin Atelier audit form. SQL independently
//! retains and validates each complete item row in the same Character transaction.
use super::*;

pub(super) fn check(proof: &Value, fee: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    if !proof["coin_inputs"].is_array() { return Err(AuditError::InvalidInput); }
    let parent = proof.get("parent").ok_or(AuditError::InvalidInput)?;
    let parent = if parent.is_null() { None } else { Some(uuid(parent.as_str().ok_or(AuditError::InvalidInput)?)?) };
    if parent.as_ref().map_or(&[][..], |id| id.as_slice()) != fee.backpack_item_instance_id.as_slice() {
        return Err(AuditError::InvalidInput);
    }
    let inputs = proof["inputs"].as_array().ok_or(AuditError::InvalidInput)?;
    let mut seen = Vec::with_capacity(inputs.len());
    for input in inputs {
        let item = uuid(input["item"].as_str().ok_or(AuditError::InvalidInput)?)?;
        if seen.contains(&item) || fee.lines.iter().any(|line| line.before.as_ref().is_some_and(|before| before.item_instance_id.as_slice() == item.as_slice())) {
            return Err(AuditError::InvalidInput);
        }
        seen.push(item);
        let before = &input["before"];
        if before["family"] != "Item" { return Err(AuditError::InvalidInput); }
        let definition = mint::OneItemTypedDefinitionRevisionV1 {
            family: "Item".into(),
            production_key: before["key"].as_str().ok_or(AuditError::InvalidInput)?.into(),
            revision_ref: before["revision"].as_str().ok_or(AuditError::InvalidInput)?.into(),
        };
        check_definition(Some(&definition))?;
        let quantity = before["quantity"].as_u64().filter(|n| (1..=100).contains(n)).ok_or(AuditError::InvalidInput)?;
        let count = input["count"].as_u64().filter(|n| (1..=quantity).contains(n)).ok_or(AuditError::InvalidInput)?;
        if before["state_revision"].as_u64().is_none_or(|n| n == 0 || n == u64::MAX)
            || input["quantity_after"].as_u64() != quantity.checked_sub(count) {
            return Err(AuditError::InvalidInput);
        }
        let ordinal_text = input["ordinal"].as_str().ok_or(AuditError::InvalidInput)?;
        let ordinal: u64 = ordinal_text.parse().map_err(|_| AuditError::InvalidInput)?;
        if ordinal.to_string() != ordinal_text { return Err(AuditError::InvalidInput); }
        let slot = input["slot"].as_u64().filter(|n| *n <= 10 && *n != 9).ok_or(AuditError::InvalidInput)?;
        let input_parent = input.get("parent").ok_or(AuditError::InvalidInput)?;
        let input_parent = if input_parent.is_null() { None } else { Some(uuid(input_parent.as_str().ok_or(AuditError::InvalidInput)?)?) };
        if input_parent == Some(item) { return Err(AuditError::InvalidInput); }
        match input["location"].as_str() {
            Some("main") if ordinal > 0 && slot == 0 && input_parent.is_some() && input_parent == parent => (),
            // Nested inputs retain the equipped root's slot code, or zero for the main bag.
            Some("nested") if ordinal > 0 && input_parent.is_some() => (),
            Some("equipment") if ordinal == 0 && slot > 0 && input_parent.is_none() => (),
            _ => return Err(AuditError::InvalidInput),
        }
    }
    if let Some(stash) = proof.get("stash").filter(|stash| !stash.is_null()) {
        let entry = uuid(stash["entry"].as_str().ok_or(AuditError::InvalidInput)?)?;
        let previous = uuid(stash["previous"].as_str().ok_or(AuditError::InvalidInput)?)?;
        if entry == previous { return Err(AuditError::InvalidInput); }
        let definition = mint::OneItemTypedDefinitionRevisionV1 { family: "Item".into(), production_key: stash["key"].as_str().ok_or(AuditError::InvalidInput)?.into(), revision_ref: stash["revision"].as_str().ok_or(AuditError::InvalidInput)?.into() };
        check_definition(Some(&definition))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assert complete persisted audit fixtures")]
    use super::*;
    use serde_json::json;

    const ITEM: &str = "01010101-0101-7001-8001-010101010101";
    const BAG: &str = "01010101-0101-7001-8001-010101010102";
    const NESTED: &str = "01010101-0101-7001-8001-010101010103";

    fn fixture() -> (Value, OneItemFeeBurnV1) {
        let id = uuid(ITEM).unwrap().to_vec();
        (json!({"character":id,"occurrence":id,"mutation":{"GradeUp":{"kind":"Basic","mod_id":0}},"inputs":[{"item":ITEM,"ordinal":"7","parent":BAG,"location":"main","slot":0,"count":5,"before":{"family":"Item","key":"oteryn:item.tibia.i46625","revision":"revision","quantity":8,"state_revision":1},"quantity_after":3}],"outputs":[],"parent":BAG,"stash":null,"coin_inputs":[]}),
         OneItemFeeBurnV1 { character_id: id, fee_gold_units: 2_000_000, backpack_item_instance_id: uuid(BAG).unwrap().to_vec(), ..Default::default() })
    }

    fn grade(proof: &Value, value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
        let cause = OneItemNpcPurchaseV1 { service_key:"oteryn:atelier.grade_up".into(), occurrence:uuid(ITEM).unwrap().to_vec(), atelier_proof:serde_json::to_vec(proof).unwrap() };
        super::super::check(&cause, value, "GradeUp")
    }

    #[test]
    fn new_atelier_audit_proves_main_nested_and_equipped_material_sources() {
        let (mut proof, mut value) = fixture();
        assert_eq!(grade(&proof,&value),Ok(()));
        proof["inputs"][0]["location"]=json!("nested"); proof["inputs"][0]["parent"]=json!(NESTED);
        assert_eq!(grade(&proof,&value),Ok(()));
        proof["inputs"][0]["slot"]=json!(10); proof["parent"]=Value::Null; value.backpack_item_instance_id.clear();
        assert_eq!(grade(&proof,&value),Ok(()));
        proof["inputs"][0]["location"]=json!("equipment"); proof["inputs"][0]["parent"]=Value::Null; proof["inputs"][0]["ordinal"]=json!("0");
        assert_eq!(grade(&proof,&value),Ok(()));
    }

    #[test]
    fn new_atelier_audit_rejects_incomplete_or_inconsistent_material_facts() {
        let (proof, value) = fixture();
        for (pointer, invalid) in [
            ("/inputs/0/item",json!("invalid")), ("/inputs/0/item",json!(BAG)),
            ("/inputs/0/ordinal",json!("0")), ("/inputs/0/ordinal",json!("01")), ("/inputs/0/ordinal",json!(7)),
            ("/inputs/0/parent",Value::Null), ("/inputs/0/parent",json!(NESTED)), ("/inputs/0/location",json!("stash")), ("/inputs/0/slot",json!(9)),
            ("/inputs/0/before/family",json!("Creature")), ("/inputs/0/before/key",json!("")),
            ("/inputs/0/before/revision",json!("")), ("/inputs/0/before/revision",json!("r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX+1))),
            ("/inputs/0/before/quantity",json!(0)), ("/inputs/0/before/quantity",json!(101)),
            ("/inputs/0/before/state_revision",json!(0)), ("/inputs/0/before/state_revision",json!(u64::MAX)),
            ("/inputs/0/count",json!(9)), ("/inputs/0/quantity_after",json!(2)),
            ("/parent",Value::Null), ("/coin_inputs",Value::Null),
        ] {
            let mut invalid_proof=proof.clone(); *invalid_proof.pointer_mut(pointer).unwrap()=invalid;
            assert_eq!(grade(&invalid_proof,&value),Err(AuditError::InvalidInput),"invalid field {pointer}");
        }
        for field in ["item","parent","ordinal","location","slot","before","quantity_after"] {
            let mut incomplete=proof.clone(); incomplete["inputs"][0].as_object_mut().unwrap().remove(field);
            assert_eq!(grade(&incomplete,&value),Err(AuditError::InvalidInput),"missing field {field}");
        }
        let mut duplicated=proof.clone(); let mut second=duplicated["inputs"][0].clone();
        duplicated["inputs"][0]["count"]=json!(2); duplicated["inputs"][0]["quantity_after"]=json!(6);
        second["count"]=json!(3); second["quantity_after"]=json!(5); duplicated["inputs"].as_array_mut().unwrap().push(second);
        assert_eq!(grade(&duplicated,&value),Err(AuditError::InvalidInput));
        let mut overlapping=value.clone(); overlapping.lines.push(OneItemFeeBurnLineV1 {before:Some(OneItemStateV1 {item_instance_id:uuid(ITEM).unwrap().to_vec(),..Default::default()}),..Default::default()});
        assert_eq!(grade(&proof,&overlapping),Err(AuditError::InvalidInput));
    }

    #[test]
    fn historical_atelier_material_proof_keeps_its_original_decoding_shape() {
        let (mut proof, value)=fixture(); proof.as_object_mut().unwrap().remove("coin_inputs");
        for field in ["parent","location","slot"] { proof["inputs"][0].as_object_mut().unwrap().remove(field); }
        assert_eq!(grade(&proof,&value),Ok(()));
        proof["coin_inputs"]=json!([]);
        assert_eq!(grade(&proof,&value),Err(AuditError::InvalidInput));
    }
}
