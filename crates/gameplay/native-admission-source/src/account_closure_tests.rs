use super::*;
#[test]
fn lifecycle_fixture_is_exact_and_rejects_authority_substitution()
-> Result<(), Box<dyn std::error::Error>> {
    let intents: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/fixtures/account-deletion/closure.json"
    ))?;
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/fixtures/account-deletion/lifecycle.json"
    ))?;
    let intent = intents["vectors"][0]["wire"]
        .as_str()
        .ok_or("intent missing")?;
    let request =
        LifecycleRequest::from_intent("oteryn:character-authority:primary", intent.as_bytes(), 3)?;
    assert_eq!(
        serde_json::to_string(&request)?,
        fixtures["vectors"][0]["wire"]
            .as_str()
            .ok_or("request missing")?
    );
    let wire = fixtures["vectors"][1]["wire"]
        .as_str()
        .ok_or("response missing")?;
    let observed = decode_lifecycle(&request, wire.as_bytes())?;
    assert_eq!(observed.lifecycle_generation, "8");
    assert!(serde_json::to_vec(&request)?.len() <= 512);
    assert!(wire.len() <= 2048);
    for bad in [
        wire.replace("OTERYN_PLATFORM_CHARACTER_AUTHORITY", "OTHER_AUTHORITY"),
        wire.replace("\"source_revision\":\"30\"", "\"source_revision\":\"030\""),
        wire.replace(
            "\"recovery_generation\":\"3\"",
            "\"recovery_generation\":\"2\"",
        ),
        wire.replace("1790000007", "1790000008"),
        wire.replace("CLOSE", "UNKNOWN"),
        wire.replace("000000000011", "000000000012"),
        wire.replace("\"contract_version\":2", "\"contract_version\":1"),
        wire.replace("}", ",\"unknown\":true}"),
        wire.replace("\"phase\":\"CLOSE\"", "\"phase\":null"),
        wire.replace("}", ",\"phase\":\"CLOSE\"}"),
        format!(" {wire}"),
    ] {
        assert!(decode_lifecycle(&request, bad.as_bytes()).is_err(), "{bad}");
    }
    assert!(decode_receipt_ack(br#"{"contract_version":2,"result":"superseded"}"#).is_err());
    assert!(decode_receipt_ack(br#"{"contract_version":2,"result":"accepted"}"#).is_ok());
    assert!(decode_ack(br#"{"contract_version":2,"result":"superseded"}"#).is_ok());
    assert!(decode_ack(br#"{"contract_version":1,"result":"accepted"}"#).is_err());
    Ok(())
}
