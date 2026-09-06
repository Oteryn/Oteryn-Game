#![allow(
    clippy::unwrap_used,
    reason = "Independent test fixture construction; malformed fixtures must fail the test"
)]
use oteryn_game_server::admission_evidence::{Request, decode_response, encode_request};

#[test]
fn request_golden_and_closed_failure() {
    let request = Request::Account {
        recovery: false,
        account_id: "01890f4e-7c00-7000-8000-000000000001",
        purpose: "fixture_security",
        scope: "fixture_fresh",
    };
    assert_eq!(
        encode_request(&request).unwrap(),
        r#"{"version":1,"operation":"ReadAccountSecurityV1","account_id":"01890f4e-7c00-7000-8000-000000000001","purpose":"fixture_security","scope":"fixture_fresh"}"#
    );
    assert!(
        decode_response(
            &request,
            "platform",
            br#"{"version":1,"operation":"ReadAccountSecurityV1","result":"not_found"}"#
        )
        .is_ok()
    );
    assert!(decode_response(&request,"platform", br#"{"version":1,"operation":"ReadAccountSecurityV1","result":"not_found","allowed":true}"#).is_err());
}

const ACCOUNT: &str = "01890f4e-7c00-7000-8000-000000000001";
fn requests() -> [Request<'static>; 4] {
    [
        Request::Account {
            recovery: false,
            account_id: ACCOUNT,
            purpose: "fixture_security",
            scope: "fixture_fresh",
        },
        Request::Trust {
            recovery: false,
            key_id: "key-1",
            key_purpose: "fixture_key",
        },
        Request::Account {
            recovery: true,
            account_id: ACCOUNT,
            purpose: "platform_security",
            scope: "existing_actor_recovery",
        },
        Request::Trust {
            recovery: true,
            key_id: "key-1",
            key_purpose: "existing_actor_recovery",
        },
    ]
}
fn golden(i: usize) -> serde_json::Value {
    let raw = match i {
        0 => {
            r#"{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"0","clock_uncertainty_seconds":"0","account_id":"01890f4e-7c00-7000-8000-000000000001","purpose":"fixture_security","scope":"fixture_fresh","allowed":false,"minimum_valid_generation":"1"}"#
        }
        1 => {
            r#"{"version":1,"operation":"ReadFreshSigningTrustV1","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"0","clock_uncertainty_seconds":"0","issuer":"urn:oteryn:platform:game-admission","profile":"oteryn-pre-admission-v1","key_purpose":"fixture_key","key_id":"key-1","trusted":false,"public_key":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#
        }
        2 => {
            r#"{"version":2,"operation":"ReadRecoveryAccountSecurityV2","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"0","clock_uncertainty_seconds":"0","account_id":"01890f4e-7c00-7000-8000-000000000001","purpose":"platform_security","scope":"existing_actor_recovery","allowed":false,"minimum_valid_generation":"1"}"#
        }
        _ => {
            r#"{"version":2,"operation":"ReadRecoverySigningTrustV2","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"0","clock_uncertainty_seconds":"0","issuer":"urn:oteryn:platform:game-recovery","profile":"oteryn-reauth-recovery-v1","key_purpose":"existing_actor_recovery","key_id":"key-1","trusted":false,"public_key":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#
        }
    };
    serde_json::from_str(raw).unwrap()
}
fn bytes(v: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
#[test]
fn all_families_and_each_field_boundary() {
    for (i, request) in requests().iter().enumerate() {
        let original = golden(i);
        assert!(decode_response(request, "platform", &bytes(&original)).is_ok());
        for key in original.as_object().unwrap().keys() {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                decode_response(request, "platform", &bytes(&missing)).is_err(),
                "missing {i}/{key}"
            );
            let mut wrong = original.clone();
            wrong[key] = serde_json::Value::Null;
            assert!(
                decode_response(request, "platform", &bytes(&wrong)).is_err(),
                "null {i}/{key}"
            );
            let raw = String::from_utf8(bytes(&original)).unwrap();
            let duplicate = format!("{},\"{}\":{}}}", &raw[..raw.len() - 1], key, original[key]);
            assert!(
                decode_response(request, "platform", duplicate.as_bytes()).is_err(),
                "duplicate {i}/{key}"
            );
        }
        for result in ["not_found", "unavailable", "unauthorized", "unsupported"] {
            let failure = serde_json::json!({"version":original["version"],"operation":original["operation"],"result":result});
            assert!(decode_response(request, "platform", &bytes(&failure)).is_ok());
        }
        for (j, other) in requests().iter().enumerate() {
            if i != j {
                assert!(decode_response(other, "platform", &bytes(&original)).is_err());
            }
        }
        let mut extra = original.clone();
        extra["unknown"] = serde_json::json!([]);
        assert!(decode_response(request, "platform", &bytes(&extra)).is_err());
        for field in ["source_authority", "operation", "decision_identity"] {
            let mut value = original.clone();
            value[field] = serde_json::json!("other");
            assert!(decode_response(request, "platform", &bytes(&value)).is_err());
        }
        let mut nested = original.clone();
        nested["source_revision"] = serde_json::json!({"x":"1"});
        assert!(decode_response(request, "platform", &bytes(&nested)).is_err());
        let mut trailing = bytes(&original);
        trailing.extend_from_slice(b" true");
        assert!(decode_response(request, "platform", &trailing).is_err());
    }
}
#[test]
fn numeric_canonical_limits() {
    for (i, request) in requests().iter().enumerate() {
        for field in [
            "source_revision",
            "clock_uncertainty_seconds",
            "source_observed_at",
        ] {
            for invalid in ["-1", "+1", "01", "1.0", "1e0", "18446744073709551616", ""] {
                let mut value = golden(i);
                value[field] = serde_json::json!(invalid);
                if field == "source_revision" {
                    value["decision_identity"] = serde_json::json!(invalid);
                }
                assert!(
                    decode_response(request, "platform", &bytes(&value)).is_err(),
                    "{i}/{field}/{invalid}"
                );
            }
        }
        let mut max = golden(i);
        max["source_revision"] = serde_json::json!("18446744073709551615");
        max["decision_identity"] = max["source_revision"].clone();
        max["clock_uncertainty_seconds"] = serde_json::json!("18446744073709551615");
        max["source_observed_at"] = serde_json::json!("9223372036854775807");
        assert!(decode_response(request, "platform", &bytes(&max)).is_ok());
        max["source_observed_at"] = serde_json::json!("9223372036854775808");
        assert!(decode_response(request, "platform", &bytes(&max)).is_err());
    }
}
#[test]
fn raw_and_decoded_bounds() {
    let request = requests()[0];
    let raw = bytes(&golden(0));
    let mut maximum = raw.clone();
    maximum.resize(8192, b' ');
    assert!(decode_response(&request, "platform", &maximum).is_ok());
    maximum.push(b' ');
    assert!(decode_response(&request, "platform", &maximum).is_err());
    let purpose = "x".repeat(256);
    let request = Request::Account {
        recovery: false,
        account_id: ACCOUNT,
        purpose: &purpose,
        scope: "fixture_fresh",
    };
    let mut value = golden(0);
    value["purpose"] = serde_json::json!(purpose);
    assert!(decode_response(&request, "platform", &bytes(&value)).is_ok());
    let oversized = "x".repeat(257);
    let bad = Request::Account {
        recovery: false,
        account_id: ACCOUNT,
        purpose: &oversized,
        scope: "fixture_fresh",
    };
    assert!(encode_request(&bad).is_err());
    value["purpose"] = serde_json::json!(oversized);
    assert!(decode_response(&request, "platform", &bytes(&value)).is_err());
    let text = String::from_utf8(raw).unwrap();
    let escaped = text.replace("fixture_security", "\\u0066ixture_security");
    assert!(decode_response(&requests()[0], "platform", escaped.as_bytes()).is_ok());
    for escape in ["\\uD800", "\\uDC00", "\\uD800\\u0041", "\\q", "\\u12"] {
        let bad = text.replace("fixture_security", escape);
        assert!(decode_response(&requests()[0], "platform", bad.as_bytes()).is_err());
    }
}
#[test]
fn key_encoding_and_expected_context() {
    for i in [1, 3] {
        let request = requests()[i];
        for key in [
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
        ] {
            let mut value = golden(i);
            value["public_key"] = serde_json::json!(key);
            assert!(decode_response(&request, "platform", &bytes(&value)).is_err());
        }
    }
    for invalid in ["", "bad key", "bad/key"] {
        assert!(
            encode_request(&Request::Trust {
                recovery: false,
                key_id: invalid,
                key_purpose: "fixture"
            })
            .is_err()
        );
    }
    assert!(
        encode_request(&Request::Account {
            recovery: true,
            account_id: ACCOUNT,
            purpose: "fixture_security",
            scope: "existing_actor_recovery"
        })
        .is_err()
    );
}

#[test]
fn independent_binding_substitutions_and_truncation() {
    for (i, request) in requests().iter().enumerate() {
        let original = golden(i);
        for field in [
            "account_id",
            "purpose",
            "scope",
            "issuer",
            "profile",
            "key_purpose",
            "key_id",
        ] {
            if original.get(field).is_some() {
                let mut changed = original.clone();
                changed[field] = serde_json::json!(if field == "account_id" {
                    "01890f4e-7c00-7000-8000-000000000002"
                } else {
                    "independent-other"
                });
                assert!(
                    decode_response(request, "platform", &bytes(&changed)).is_err(),
                    "{i}/{field}"
                );
            }
        }
        let raw = bytes(&original);
        for end in 0..raw.len() {
            assert!(
                decode_response(request, "platform", &raw[..end]).is_err(),
                "truncated {i}/{end}"
            );
        }
        assert!(decode_response(request, "different-source", &raw).is_err());
        let mut scalar = original.clone();
        let field = if i % 2 == 0 { "allowed" } else { "trusted" };
        scalar[field] = serde_json::json!("false");
        assert!(decode_response(request, "platform", &bytes(&scalar)).is_err());
        scalar[field] = serde_json::json!(true);
        assert!(decode_response(request, "platform", &bytes(&scalar)).is_ok());
    }
}
#[test]
fn expected_limits_unicode_and_request_expansion() {
    let authority = "a".repeat(128);
    let mut value = golden(0);
    value["source_authority"] = serde_json::json!(authority);
    assert!(decode_response(&requests()[0], &authority, &bytes(&value)).is_ok());
    let too_long = "a".repeat(129);
    value["source_authority"] = serde_json::json!(too_long);
    assert!(decode_response(&requests()[0], &too_long, &bytes(&value)).is_err());
    let key = "a".repeat(64);
    assert!(
        encode_request(&Request::Trust {
            recovery: false,
            key_id: &key,
            key_purpose: "fixture"
        })
        .is_ok()
    );
    let key = "a".repeat(65);
    assert!(
        encode_request(&Request::Trust {
            recovery: false,
            key_id: &key,
            key_purpose: "fixture"
        })
        .is_err()
    );
    let purpose = "😀".repeat(64);
    let request = Request::Account {
        recovery: false,
        account_id: ACCOUNT,
        purpose: &purpose,
        scope: "fixture_fresh",
    };
    let mut value = golden(0);
    value["purpose"] = serde_json::json!(purpose);
    let raw = String::from_utf8(bytes(&value))
        .unwrap()
        .replace('😀', "\\uD83D\\uDE00");
    assert!(decode_response(&request, "platform", raw.as_bytes()).is_ok());
    let excessive = "😀".repeat(65);
    assert!(
        encode_request(&Request::Account {
            recovery: false,
            account_id: ACCOUNT,
            purpose: &excessive,
            scope: "fixture_fresh"
        })
        .is_err()
    );
    let escaped = "\u{1}".repeat(256);
    assert!(
        encode_request(&Request::Account {
            recovery: false,
            account_id: ACCOUNT,
            purpose: &escaped,
            scope: "fixture_fresh"
        })
        .is_err()
    );
    let mut invalid = bytes(&golden(0));
    let pos = invalid.iter().position(|b| *b == b'p').unwrap();
    invalid[pos] = 255;
    assert!(decode_response(&requests()[0], "platform", &invalid).is_err());
}

#[test]
fn all_request_fields_match_independent_golden_bindings() {
    for (i, request) in requests().iter().enumerate() {
        let encoded = encode_request(request).unwrap();
        let actual: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        let observed = golden(i);
        let fields: &[&str] = if i % 2 == 0 {
            &["version", "operation", "account_id", "purpose", "scope"]
        } else {
            &[
                "version",
                "operation",
                "issuer",
                "profile",
                "key_purpose",
                "key_id",
            ]
        };
        assert_eq!(actual.as_object().unwrap().len(), fields.len());
        for field in fields {
            assert_eq!(actual[*field], observed[*field]);
        }
    }
}

#[test]
fn independent_numeric_zero_and_generation_boundaries() {
    for (i, request) in requests().iter().enumerate() {
        let mut zero_revision = golden(i);
        zero_revision["source_revision"] = serde_json::json!("0");
        zero_revision["decision_identity"] = serde_json::json!("0");
        assert!(decode_response(request, "platform", &bytes(&zero_revision)).is_err());
        let mut zero_times = golden(i);
        zero_times["source_observed_at"] = serde_json::json!("0");
        zero_times["clock_uncertainty_seconds"] = serde_json::json!("0");
        assert!(decode_response(request, "platform", &bytes(&zero_times)).is_ok());
        let mut mismatched_decision = golden(i);
        mismatched_decision["decision_identity"] = serde_json::json!("2");
        assert!(decode_response(request, "platform", &bytes(&mismatched_decision)).is_err());
        if i % 2 == 0 {
            for (generation, valid) in [
                ("0", false),
                ("18446744073709551615", true),
                ("18446744073709551616", false),
            ] {
                let mut value = golden(i);
                value["minimum_valid_generation"] = serde_json::json!(generation);
                assert_eq!(
                    decode_response(request, "platform", &bytes(&value)).is_ok(),
                    valid,
                    "generation {i}/{generation}"
                );
            }
        }
    }
}

#[test]
fn malformed_expected_uuid_cannot_hide_behind_binding_mismatch() {
    for i in [0, 2] {
        let (purpose, scope) = if i == 0 {
            ("fixture_security", "fixture_fresh")
        } else {
            ("platform_security", "existing_actor_recovery")
        };
        for invalid in [
            "01890F4e-7c00-7000-8000-000000000001",
            "01890f4e-7c00-4000-8000-000000000001",
            "01890f4e-7c00-7000-c000-000000000001",
            "00000000-0000-0000-0000-000000000000",
            "01890f4e-7c00-7000-8000-00000000001",
            "01890f4e-7c00-7000-8000-0000000000001",
            "01890f4e_7c00-7000-8000-000000000001",
            "01890f4e-7c00-7000-8000-00000000000g",
            "",
        ] {
            let request = Request::Account {
                recovery: i == 2,
                account_id: invalid,
                purpose,
                scope,
            };
            let mut response = golden(i);
            response["account_id"] = serde_json::json!(invalid);
            assert!(encode_request(&request).is_err(), "encode {i}/{invalid}");
            assert!(
                decode_response(&request, "platform", &bytes(&response)).is_err(),
                "decode matching malformed UUID {i}/{invalid}"
            );
        }
        assert!(encode_request(&requests()[i]).is_ok());
        assert!(decode_response(&requests()[i], "platform", &bytes(&golden(i))).is_ok());
    }
}

#[test]
fn decoded_observation_and_failure_payloads_are_exact() {
    use oteryn_game_server::admission_evidence::{Facts, Failure, Response};
    // Independent byte fixture: 00..1f encoded in canonical unpadded base64url.
    const KEY: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    for (i, request) in requests().iter().enumerate() {
        for flag in [false, true] {
            let mut value = golden(i);
            value["source_authority"] = serde_json::json!("independent-source");
            value["source_revision"] = serde_json::json!("37");
            value["decision_identity"] = serde_json::json!("37");
            value["source_observed_at"] = serde_json::json!("1700000123");
            value["clock_uncertainty_seconds"] = serde_json::json!("19");
            let expected = if i % 2 == 0 {
                value["allowed"] = serde_json::json!(flag);
                value["minimum_valid_generation"] = serde_json::json!("41");
                Facts::Account {
                    allowed: flag,
                    minimum_valid_generation: 41,
                }
            } else {
                value["trusted"] = serde_json::json!(flag);
                value["public_key"] =
                    serde_json::json!("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8");
                Facts::Trust {
                    trusted: flag,
                    public_key: KEY,
                }
            };
            let observation =
                match decode_response(request, "independent-source", &bytes(&value)).unwrap() {
                    Response::Observed(observation) => Some(observation),
                    Response::Failure(_) => None,
                }
                .unwrap();
            assert_eq!(observation.source_authority.as_str(), "independent-source");
            assert_eq!(observation.source_revision, 37);
            assert_eq!(observation.decision_identity.as_str(), "37");
            assert_eq!(observation.source_observed_at, 1_700_000_123);
            assert_eq!(observation.clock_uncertainty_seconds, 19);
            assert_eq!(observation.facts, expected, "facts {i}/{flag}");
        }
        for (result, expected) in [
            ("not_found", Failure::NotFound),
            ("unavailable", Failure::Unavailable),
            ("unauthorized", Failure::Unauthorized),
            ("unsupported", Failure::Unsupported),
        ] {
            let original = golden(i);
            let response = serde_json::json!({
                "version": original["version"], "operation": original["operation"],
                "result": result
            });
            assert_eq!(
                decode_response(request, "platform", &bytes(&response)),
                Ok(Response::Failure(expected)),
                "failure payload {i}/{result}"
            );
        }
    }
}
