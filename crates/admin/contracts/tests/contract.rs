use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::VerifyingKey;
use oteryn_admin_contracts::*;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::Path};

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
fn contract_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/contracts/admin-control/v1")
}

#[test]
fn schemas_golden_check() -> Result<(), WireError> {
    let root = contract_dir();
    let schemas = schemas()?;
    for (name, expected) in &schemas {
        let path = root.join(name);
        if std::env::var_os("UPDATE_ADMIN_GOLDENS").is_some() {
            fs::write(&path, expected)?;
        }
        assert_eq!(&fs::read_to_string(path)?, expected, "{name}");
    }
    let files = fs::read_dir(root)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, WireError>>()?;
    let files: BTreeSet<_> = files
        .iter()
        .filter(|name| name.ends_with(".schema.json"))
        .collect();
    assert_eq!(files, schemas.keys().collect());
    for op in Operation::ALL {
        assert!(op.validate_params(&json!({"unknown":true})).is_err());
        assert_eq!(Operation::parse(op.name()), Some(op));
    }
    Ok(())
}

#[test]
fn phase_two_signed_vectors_cover_every_schema_pair() -> Result<(), WireError> {
    let root = contract_dir().join("vectors/phase-2");
    let mut expected_files = BTreeSet::new();
    for op in Operation::ALL.into_iter().filter(|op| op.phase() == 2) {
        if op == Operation::AnalyticsQuery {
            for query in ECONOMY_QUERY_IDS {
                check_vector_pair(&root, op, Some(query), &mut expected_files)?;
            }
        } else {
            check_vector_pair(&root, op, None, &mut expected_files)?;
        }
    }

    let actual_files = fs::read_dir(root)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<BTreeSet<_>, WireError>>()?;
    assert_eq!(actual_files, expected_files);
    Ok(())
}

fn check_vector_pair(
    root: &Path,
    op: Operation,
    query: Option<&str>,
    expected_files: &mut BTreeSet<String>,
) -> Result<(), WireError> {
    let stem = query.map_or_else(
        || op.name().to_owned(),
        |query| format!("{}.{query}", op.name()),
    );
    for (suffix, valid) in [("valid", true), ("invalid", false)] {
        let file = format!("{stem}-{suffix}.json");
        let vector = parse_json(&fs::read(root.join(&file))?)?;
        let request: Request = serde_json::from_value(vector["request"].clone())?;
        assert_eq!(request.operation, op.name(), "{file}");
        assert_eq!(request.operation_version, 1, "{file}");
        assert_eq!(request.kind, op.kind(), "{file}");
        assert_eq!(request.contract, CONTRACT, "{file}");
        assert_eq!(request.actor.permission, op.permission(), "{file}");
        op.validate_target(&request.target)?;

        assert_eq!(
            json!(String::from_utf8(signed_bytes(&request)?)?),
            vector["canonical_signed_utf8"],
            "{file}"
        );
        let key: [u8; 32] = URL_SAFE_NO_PAD
            .decode(
                vector["trusted_signers"][0]["public_key"]
                    .as_str()
                    .ok_or("missing vector signer")?,
            )?
            .try_into()
            .map_err(|_| "invalid vector key size")?;
        verify_signature(
            &request,
            &request.signature,
            &VerifyingKey::from_bytes(&key)?,
        )?;

        let expected_error = if valid {
            Value::Null
        } else {
            json!(ErrorCode::AdmInvalidParams)
        };
        assert_eq!(vector["expected_error"], expected_error, "{file}");
        assert_eq!(op.validate_params(&request.params).is_ok(), valid, "{file}");
        if let Some(query) = query {
            assert_eq!(request.params["query_id"], query, "{file}");
            assert_eq!(vector["result"]["query_id"], query, "{file}");
        }
        if valid {
            let risk = if query == Some("top_wealth") {
                RiskClass::R1
            } else {
                op.minimum()
            };
            assert_eq!(op.minimum_for_params(&request.params)?, risk, "{file}");
            assert_eq!(request.risk_class, risk, "{file}");
            assert_eq!(
                vector["policy"]["operations"][0]["min_risk_class"],
                json!(risk),
                "{file}"
            );
        }

        assert_eq!(vector["expected_result_valid"], valid, "{file}");
        let result = &vector["result"];
        assert_eq!(op.validate_result(result).is_ok(), valid, "{file}");
        assert_eq!(
            serde_json::from_value::<OperationResult>(result.clone()).is_ok(),
            valid,
            "{file}"
        );
        expected_files.insert(file);
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
