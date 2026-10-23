use super::*;

#[test]
fn daily_reward_registry_uses_only_cp_allocated_ids() -> Result<(), Box<dyn std::error::Error>> {
    let registry: serde_json::Value =
        serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)?;
    let only = |section: &str, id: u32| -> Result<&serde_json::Value, Box<dyn std::error::Error>> {
        let entries = registry[section].as_array().ok_or("registry array")?;
        let mut matches = entries.iter().filter(|entry| entry["id"] == id);
        let entry = matches.next().ok_or("missing DAILY entry")?;
        assert!(matches.next().is_none());
        Ok(entry)
    };
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_DAILY_REWARD_V1));
    assert_eq!(
        crate::validate_acceptance_value(1, 1, 1, &[CAPABILITY_DAILY_REWARD_V1]),
        Ok(())
    );
    let capability = only("capabilities", CAPABILITY_DAILY_REWARD_V1)?;
    assert_eq!(capability["name"], "DAILY_REWARD_V1");
    assert_eq!(capability["command_types"], serde_json::json!([32, 33]));
    assert_eq!(capability["state_domains"], serde_json::json!([24]));
    assert_eq!(capability["offered"], false);
    for (id, limit) in [
        (
            COMMAND_TYPE_DAILY_REWARD_CLAIM,
            MAX_DAILY_REWARD_CLAIM_BYTES,
        ),
        (COMMAND_TYPE_DAILY_REWARD_QUERY, 0),
    ] {
        let entry = only("command_types", id)?;
        assert_eq!(entry["capability"], CAPABILITY_DAILY_REWARD_V1);
        assert_eq!(entry["max_payload_bytes"], limit);
    }
    assert_eq!(
        only("command_types", COMMAND_TYPE_DAILY_REWARD_CLAIM)?["max_result_payload_bytes"],
        MAX_DAILY_REWARD_RESULT_BYTES
    );
    assert_eq!(
        only("command_types", COMMAND_TYPE_DAILY_REWARD_QUERY)?["max_result_payload_bytes"],
        MAX_DAILY_REWARD_VIEW_BYTES
    );
    let domain = only("state_domains", STATE_DOMAIN_DAILY_REWARD)?;
    assert_eq!(domain["capability"], CAPABILITY_DAILY_REWARD_V1);
    for kind in ["delta_types", "snapshot_types"] {
        assert_eq!(
            domain[kind][0]["max_payload_bytes"],
            MAX_DAILY_REWARD_VIEW_BYTES
        );
        assert_eq!(
            domain[kind][0]["payload_schema"],
            "docs/contracts/protocol-oteryn/v1/daily_reward_v1.proto#DailyRewardWallV1"
        );
    }
    assert!(
        crate::capability_tables::GATED
            .iter()
            .any(|(id, commands, domains)| *id == CAPABILITY_DAILY_REWARD_V1
                && *commands == [32, 33]
                && *domains == [24])
    );
    Ok(())
}
