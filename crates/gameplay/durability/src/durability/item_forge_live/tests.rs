use super::*;
use crate::durability::item_mint_audit::{AuditError, TransactionResourceUsage};

fn identity() -> Vec<u8> {
    let mut id = vec![255; 16];
    id[6] = 0x7f;
    id[8] = 0xbf;
    id
}

fn maximum_reply() -> Value {
    json!({"operation":"convergence_transfer","succeeded":false,"applied":true,"reason":null,"bonus":7,
        "result":{"first_item":u64::MAX,"first_tier":10,
        "second_item":u64::MAX-1,"second_tier":10,"dust":4600,"dust_limit":4600,
        "gold_spent":300_000_000_000_u64,"cores_spent":85,"slivers_spent":50,"slivers_created":3,"cores_created":1}})
}

fn maximum_proof() -> audit::Proof {
    audit::Proof {
        occurrence: identity(),
        character: identity(),
        world: identity(),
        operation: 4,
        content: vec![255; 32],
        ruleset: storage::RULESET.into(),
        simulation_revision: 1,
        rolls: [99, 99, 10_000],
        reply: maximum_reply(),
        inputs_hash: vec![255; 32],
        outputs_hash: vec![255; 32],
        stash_hash: vec![255; 32],
        inputs: 87,
        outputs: 0,
        retired: 85,
        tier_changes: 2,
        dust_spent: 160,
        gold_spent: 300_000_000_000,
        stash_spent: true,
    }
}

#[test]
fn forge_audit_bounds_cover_donor_materials_and_reject_max_plus_one()
-> std::result::Result<(), serde_json::Error> {
    assert!(
        oteryn_gameplay_items::forge::FORGE_REFERENCE_PRICES
            .iter()
            .all(|row| u64::from(row.transfer_cores) + 2 <= audit::FORGE_INPUTS_MAX)
    );
    let proof = maximum_proof();
    let bytes = serde_json::to_vec(&proof)?;
    assert!(bytes.len() <= audit::PROOF_BYTES_MAX);
    assert!(audit::decode(&bytes).is_ok());
    println!("FORGE_MAX_PROOF_BYTES {}", bytes.len());
    for malformed in [
        {
            let mut p = maximum_proof();
            p.inputs = 88;
            p
        },
        {
            let mut p = maximum_proof();
            p.outputs = 2;
            p
        },
        {
            let mut p = maximum_proof();
            p.tier_changes = 3;
            p
        },
        {
            let mut p = maximum_proof();
            p.rolls[0] = 100;
            p
        },
        {
            let mut p = maximum_proof();
            p.rolls[1] = 100;
            p
        },
        {
            let mut p = maximum_proof();
            p.rolls[2] = 10_001;
            p
        },
        {
            let mut p = maximum_proof();
            p.occurrence[6] = 0;
            p
        },
        {
            let mut p = maximum_proof();
            p.reply["result"]["unregistered_field"] = json!(1);
            p
        },
        {
            let mut p = maximum_proof();
            p.reply["result"]["first_item"] = json!("ffffffff-ffff-7fff-bfff-ffffffffffff");
            p
        },
        {
            let mut p = maximum_proof();
            p.reply["result"]["dust_limit"] = json!(4601);
            p
        },
    ] {
        assert!(matches!(
            audit::decode(&serde_json::to_vec(&malformed)?),
            Err(AuditError::InvalidInput)
        ));
    }
    assert!(matches!(
        audit::decode(&vec![0; audit::PROOF_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    ));
    Ok(())
}

#[test]
fn forge_audit_accepts_protected_failure_bonus_eight_and_rejects_unknown_bonus()
-> std::result::Result<(), serde_json::Error> {
    // Crystal player.cpp:10736: a failed protected fusion can retain both tier-zero
    // operands and reports bonus8 while still consuming its dust, core and gold.
    let mut proof = audit::Proof {
        operation: 1,
        rolls: [99, 99, 0],
        reply: json!({"operation":"fusion","succeeded":false,"applied":true,"reason":null,"bonus":8,
            "result":{"first_item":17,"first_tier":0,"second_item":23,"second_tier":0,
            "dust":0,"dust_limit":100,"gold_spent":25_000,"cores_spent":1,
            "slivers_spent":0,"slivers_created":0,"cores_created":0}}),
        inputs: 3,
        outputs: 0,
        retired: 1,
        tier_changes: 0,
        dust_spent: 100,
        gold_spent: 25_000,
        stash_spent: false,
        ..maximum_proof()
    };
    assert!(audit::decode(&serde_json::to_vec(&proof)?).is_ok());
    for unknown_bonus in [9, 10, u64::MAX] {
        proof.reply["bonus"] = json!(unknown_bonus);
        assert!(matches!(
            audit::decode(&serde_json::to_vec(&proof)?),
            Err(AuditError::InvalidInput)
        ));
    }
    Ok(())
}

#[test]
fn forge_combined_fee_usage_keeps_its_closed_child_bounds() {
    let mut usage = TransactionResourceUsage {
        touched_item_instances: 109,
        location_custody_lines: 109,
        value_lines: 3,
        transform_lines: 2,
        container_expansion: 0,
        participants: 112,
        effect_work_units: 331,
        events: 1,
    };
    assert!(audit::check_combined_usage(&usage).is_ok());
    usage.touched_item_instances += 1;
    assert_eq!(
        audit::check_combined_usage(&usage),
        Err(AuditError::CapacityExceeded)
    );
    usage.touched_item_instances -= 1;
    usage.value_lines += 1;
    assert_eq!(
        audit::check_combined_usage(&usage),
        Err(AuditError::CapacityExceeded)
    );
}

#[test]
fn forge_history_fifty_maximum_receipts_fit_command181()
-> std::result::Result<(), serde_json::Error> {
    let reply = maximum_reply();
    assert_eq!(serde_json::to_vec(&reply)?.len(), 336);
    let history = json!(
        (0..50)
            .map(|_| json!({"ordinal":i64::MAX,"receipt":reply}))
            .collect::<Vec<_>>()
    );
    let wire = serde_json::to_vec(&history)?;
    assert_eq!(wire.len(), 18_951);
    assert!(wire.len() <= 32_768);
    println!("FORGE_HISTORY50_BYTES {}", wire.len());
    for operation in [
        Operation::Fusion,
        Operation::ConvergenceFusion,
        Operation::Transfer,
        Operation::ConvergenceTransfer,
        Operation::DustToSlivers,
        Operation::SliversToCores,
        Operation::DustLimit,
    ] {
        let request = Request {
            operation,
            occurrence: [0; 16],
            first_item: None,
            second_item: None,
            boost: false,
            protect: false,
            history_limit: 0,
        };
        assert!(serde_json::to_vec(&actions::refusal(request, "REVISION_CHANGED"))?.len() < 512);
    }
    Ok(())
}
