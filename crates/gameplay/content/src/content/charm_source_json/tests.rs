use super::*;

#[test]
fn percentages_preserve_decimal_literals_and_integer_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    for (token, expected) in [
        ("0", 0),
        ("0.01", 1),
        ("0.8", 80),
        ("1.60", 160),
        ("2.5", 250),
        ("1000", 100_000),
        ("42949672.95", u32::MAX),
    ] {
        assert_eq!(
            serde_json::from_str::<ExactPercent>(token)?,
            ExactPercent(expected)
        );
    }
    for token in [
        "0.80000000000000001",
        "0.801",
        "8e-1",
        "1e0",
        "-0.8",
        "00.8",
        "0.",
        "42949672.96",
        "42949673",
        "4294967296",
        "\"0.8\"",
        "{\"hundredths\":80}",
        "[]",
        "null",
        "true",
        "\"unterminated\\",
    ] {
        assert!(
            serde_json::from_str::<ExactPercent>(token).is_err(),
            "{token}"
        );
    }
    Ok(())
}

#[test]
fn duplicate_decoded_member_names_are_rejected_before_overwrite() {
    for source in [
        r#"{"type":"life_leech","type":"mana_leech"}"#,
        r#"{"duration_ms":10000,"duration_ms":10000}"#,
        r#"{"type":"life_leech","t\u0079pe":"life_leech"}"#,
        "null",
        "[]",
        "true",
        "\"object\"",
    ] {
        assert!(
            serde_json::from_str::<CharmSourceMembers>(source).is_err(),
            "{source}"
        );
    }
}

#[test]
fn typed_consumption_rejects_missing_wrong_and_leftover_members()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"{"value":0.8,"name":"\\\"value\\\":0.80000000000000001; slash\\; λ"}"#;
    let mut members = serde_json::from_str::<CharmSourceMembers>(source)?;
    assert_eq!(
        take::<ExactPercent>(&mut members, "value"),
        Ok(ExactPercent(80))
    );
    assert!(ensure_empty(&members).is_err());
    assert_eq!(
        take::<String>(&mut members, "name")?,
        "\\\"value\\\":0.80000000000000001; slash\\; λ"
    );
    assert_eq!(ensure_empty(&members), Ok(()));
    assert!(take::<ExactPercent>(&mut members, "value").is_err());
    for source in [
        r#"{"value":"0.8"}"#,
        r#"{"value":false}"#,
        r#"{"value":{}}"#,
    ] {
        let mut members = serde_json::from_str::<CharmSourceMembers>(source)?;
        assert!(take::<ExactPercent>(&mut members, "value").is_err());
    }
    let mut escaped = serde_json::from_str::<CharmSourceMembers>(r#"{"v\u0061lue":0.8}"#)?;
    assert_eq!(
        take::<ExactPercent>(&mut escaped, "value"),
        Ok(ExactPercent(80))
    );
    assert_eq!(ensure_empty(&escaped), Ok(()));
    Ok(())
}
