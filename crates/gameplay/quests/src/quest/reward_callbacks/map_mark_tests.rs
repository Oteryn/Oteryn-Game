use super::*;

#[test]
fn personal_map_mark_admits_only_bounded_native_cross() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key.ends_with("sam-backpack"))
        .ok_or("callback")?;
    let mut document = serde_json::json!({"schema":1,"callbacks":[callback]});
    document["callbacks"][0]["map_mark"] = serde_json::json!({
        "position":{"x":31948,"y":31924,"floor":-7},"kind":"cross","text":"Treasure"
    });
    assert!(
        parse(&document.to_string()).is_ok(),
        "native personal marker must be admitted"
    );
    for (field, value) in [
        ("text", serde_json::json!("")),
        ("text", serde_json::json!("bad\0label")),
        ("text", serde_json::json!("x".repeat(257))),
        ("kind", serde_json::json!("unknown")),
    ] {
        let mut invalid = document.clone();
        invalid["callbacks"][0]["map_mark"][field] = value;
        assert!(parse(&invalid.to_string()).is_err());
    }
    document["callbacks"][0]["map_mark"]["position"]["floor"] = 1.into();
    assert!(parse(&document.to_string()).is_err());
    Ok(())
}
