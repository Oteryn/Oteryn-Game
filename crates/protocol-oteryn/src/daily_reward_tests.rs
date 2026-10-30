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
    // V1 is retained registry history; v2 must not rewrite its contract.
    for (id, name, commands, domains, claim_limit) in [
        (
            CAPABILITY_DAILY_REWARD_V1,
            "DAILY_REWARD_V1",
            &[32, 33][..],
            &[24][..],
            10_500,
        ),
        (
            CAPABILITY_DAILY_REWARD_V2,
            "DAILY_REWARD_V2",
            &[
                COMMAND_TYPE_DAILY_REWARD_CLAIM,
                COMMAND_TYPE_DAILY_REWARD_QUERY,
            ][..],
            &[][..],
            MAX_DAILY_REWARD_CLAIM_BYTES,
        ),
    ] {
        assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&id));
        assert_eq!(crate::validate_acceptance_value(1, 1, 1, &[id]), Ok(()));
        let capability = only("capabilities", id)?;
        assert_eq!(capability["name"], name);
        assert_eq!(capability["command_types"], serde_json::json!(commands));
        assert_eq!(capability["state_domains"], serde_json::json!(domains));
        assert_eq!(capability["offered"], false);
        for (command, limit, result_limit) in [
            (commands[0], claim_limit, MAX_DAILY_REWARD_RESULT_BYTES),
            (commands[1], 0, MAX_DAILY_REWARD_VIEW_BYTES),
        ] {
            let entry = only("command_types", command)?;
            assert_eq!(entry["capability"], id);
            assert_eq!(entry["max_payload_bytes"], limit);
            assert_eq!(entry["max_result_payload_bytes"], result_limit);
        }
        assert!(crate::capability_tables::GATED.iter().any(
            |(candidate, registered_commands, registered_domains)| *candidate == id
                && *registered_commands == commands
                && *registered_domains == domains
        ));
    }
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
    Ok(())
}
