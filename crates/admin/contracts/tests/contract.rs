use oteryn_admin_contracts::*;
use serde_json::json;

#[test]
fn rfc8785_numbers_strings_and_utf16_order() -> Result<(), WireError> {
    let input: serde_json::Value = serde_json::from_str(
        r#"{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001],"string":"€$\u000f\nA'B\"\\\"/","literals":[null,true,false]}"#,
    )?;
    assert_eq!(
        String::from_utf8(canonical(&input)?)?,
        r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\"/"}"#
    );
    assert_eq!(
        String::from_utf8(canonical(
            &json!({"דּ":7,"😀":6,"€":5,"ö":4,"\u{0080}":3,"1":2,"\r":1})
        )?)?,
        "{\"\\r\":1,\"1\":2,\"\u{0080}\":3,\"ö\":4,\"€\":5,\"😀\":6,\"דּ\":7}"
    );
    assert_eq!(canonical(&json!(-0.0))?, b"0");
    assert!(canonical(&json!(9_007_199_254_740_992u64)).is_err());
    Ok(())
}
#[test]
fn signature_excludes_only_signature_and_rejects_tampering() -> Result<(), WireError> {
    let key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let mut envelope = json!({"a":1,"signature":""});
    let sig = sign(&envelope, &key)?;
    envelope["signature"] = json!(sig);
    verify_signature(&envelope, &sig, &key.verifying_key())?;
    envelope["a"] = json!(2);
    assert!(verify_signature(&envelope, &sig, &key.verifying_key()).is_err());
    assert!(
        verify_signature(
            &envelope,
            &sig,
            &ed25519_dalek::SigningKey::from_bytes(&[8; 32]).verifying_key()
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn schemas_golden_check() -> Result<(), WireError> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/admin-control/v1");
    for (name, expected) in schemas()? {
        let path = root.join(name);
        if std::env::var_os("UPDATE_ADMIN_GOLDENS").is_some() {
            std::fs::write(&path, &expected)?;
        }
        assert_eq!(std::fs::read_to_string(path)?, expected);
    }
    for op in Operation::ALL {
        assert!(op.validate_params(&json!({"unknown":true})).is_err());
    }
    Ok(())
}

#[test]
fn rfc8785_appendix_b_and_invalid_inputs() -> Result<(), WireError> {
    for (bits, expected) in [
        (0x0000000000000001, "5e-324"),
        (0x8000000000000001, "-5e-324"),
        (0x7fefffffffffffff, "1.7976931348623157e+308"),
        (0x4430000000000000, "295147905179352830000"),
        (0x44b52d02c7e14af5, "9.999999999999997e+22"),
        (0x44b52d02c7e14af6, "1e+23"),
        (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
    ] {
        assert_eq!(
            String::from_utf8(canonical(&f64::from_bits(bits))?)?,
            expected
        );
    }
    assert!(canonical(&f64::NAN).is_err());
    assert!(canonical(&f64::INFINITY).is_err());
    assert!(parse_json(br#"{"params":{"a":1,"\u0061":2}}"#).is_err());
    assert!(parse_json(br#"{"params":[{"a":1,"a":2}]}"#).is_err());
    assert!(parse_json(br#""\ud800""#).is_err());
    assert!(parse_json(b"{} {}").is_err());
    Ok(())
}
