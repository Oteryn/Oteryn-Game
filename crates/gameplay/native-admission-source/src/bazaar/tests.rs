use super::*;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Fixtures {
    vectors: Vec<Vector>,
}

#[derive(Deserialize)]
struct Vector {
    name: String,
    wire: String,
    sha256: String,
}

fn vectors() -> Result<Vec<Vector>, serde_json::Error> {
    serde_json::from_str::<Fixtures>(include_str!(
        "../../../../../docs/contracts/fixtures/bazaar/decisions.json"
    ))
    .map(|fixtures| fixtures.vectors)
}

#[test]
fn bazaar_fixture_intents_are_exact_and_bound_to_the_requested_key() -> TestResult {
    for vector in vectors()? {
        let name = vector.name.as_str();
        let raw = vector.wire.as_bytes();
        assert_eq!(
            Sha256::digest(raw)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            vector.sha256
        );
        if matches!(name, "escrow_intent" | "delivery_intent" | "changed_retry") {
            let value: serde_json::Value = serde_json::from_slice(raw)?;
            let command = value["command"].as_str().ok_or("missing fixture command")?;
            let operation = value["operation_id"]
                .as_str()
                .ok_or("missing fixture operation")?;
            let intent = decode_intent(raw, command, operation)?;
            assert_eq!(intent.encode()?, raw);
            assert_eq!(intent.binding_digest()?, Sha256::digest(raw).as_slice());
            assert!(decode_intent(raw, "ReturnBazaarItem", operation).is_err());
            assert!(decode_intent(raw, command, "0192a000-0000-7000-8000-000000000099").is_err());
        }
    }
    Ok(())
}

#[test]
fn bazaar_wire_rejects_noncanonical_or_ambiguous_input() -> TestResult {
    let vectors = vectors()?;
    let raw = vectors
        .first()
        .ok_or("missing escrow fixture")?
        .wire
        .as_str();
    let op = "0192a000-0000-7000-8000-000000000002";
    for invalid in [
        format!("{raw}\n"),
        raw.replace("\"quantity\":1", "\"quantity\":0"),
        raw.replace("\"quantity\":1", "\"quantity\":4294967296"),
        raw.replace("\"quantity\":1", "\"quantity\":1.0"),
        raw.replace("\"quantity\":1", "\"quantity\":1,\"quantity\":1"),
        raw.replace("\"quantity\":1", "\"quantity\":1,\"unknown\":1"),
        raw.replace("\"source_revision\":\"18\"", "\"source_revision\":\"018\""),
        raw.replace("PLATFORM_BAZAAR", "PLATFORM_USER_CREATE"),
        raw.replace(
            "OTERYN_GAME_BAZAAR_AUTHORITY",
            "OTERYN_GAME_CHARACTER_AUTHORITY",
        ),
        raw.replace("1790000300", "1790000301"),
        raw.replace("1790000300", "1790000000"),
        raw.replace("\"quantity\":1", "\"quantity\": 1"),
        raw.replace("\"quantity\":1", "\"quanti\\u0074y\":1"),
        raw.replace("7000", "4000"),
    ] {
        assert!(
            decode_intent(invalid.as_bytes(), "EscrowBazaarItem", op).is_err(),
            "{invalid}"
        );
    }
    Ok(())
}

fn intent_wire(command: &str, members: &str) -> String {
    format!(
        "{{\"contract_version\":2,\"command\":\"{command}\",\"variant\":\"PLATFORM_BAZAAR\",\"issuer_authority\":\"OTERYN_PLATFORM_BAZAAR_AUTHORITY\",\"issuer_decision_id\":\"0192a000-0000-7000-8000-0000000000a1\",\"source_revision\":\"18\",\"operation_id\":\"0192a000-0000-7000-8000-000000000002\",\"listing_id\":\"0192a000-0000-7000-8000-000000000020\",\"seller_account_id\":\"0192a000-0000-7000-8000-000000000011\",\"world_id\":\"0192a000-0000-7000-8000-000000000012\",{members},\"issued_at_source\":\"1790000000\",\"expires_at_source\":\"1790000300\",\"audience\":\"OTERYN_GAME_BAZAAR_AUTHORITY\"}}"
    )
}

const OP: &str = "0192a000-0000-7000-8000-000000000002";
const CHARACTER: &str = "0192a000-0000-7000-8000-000000000021";
const BUYER: &str = "0192a000-0000-7000-8000-000000000013";
const HOLD: &str = "0192a000-0000-7000-8000-000000000025";
const DIGEST: &str = "5699ea4e045065a96e356f226df67a32fb5a19342af25678d80e35531ea080fc";

fn all_commands() -> Vec<(&'static str, String)> {
    let funding = format!(
        r#"{{"currency":"GOLD","reservation_id":"{HOLD}","hold_revision":"1","gross_amount":"10000"}}"#
    );
    vec![
        (
            "LockBazaarCharacter",
            format!(
                r#""character_id":"{CHARACTER}","expected_character_revision":"7","eligibility_revision":"4","snapshot_schema_version":1"#
            ),
        ),
        (
            "BindBazaarCharacterTransfer",
            format!(
                r#""character_id":"{CHARACTER}","lock_id":"{HOLD}","expected_lock_revision":"1","transfer_operation_id":"{OP}","transfer_binding_digest":"{DIGEST}","purpose":"BAZAAR_SETTLEMENT","sale_id":"{OP}","funding":{funding}"#
            ),
        ),
        (
            "UnlockBazaarCharacter",
            format!(
                r#""character_id":"{CHARACTER}","lock_id":"{HOLD}","expected_lock_revision":"1""#
            ),
        ),
        (
            "EscrowBazaarItem",
            format!(
                r#""seller_character_id":"{CHARACTER}","item_instance_id":"{HOLD}","expected_state_revision":"7","quantity":1,"transferability_revision":"4""#
            ),
        ),
        (
            "DeliverBazaarItem",
            format!(
                r#""escrow_id":"{HOLD}","expected_escrow_revision":"1","buyer_account_id":"{BUYER}","buyer_character_id":"{CHARACTER}","escrow_receipt_digest":"{DIGEST}","sale_id":"{OP}","funding":{funding}"#
            ),
        ),
        (
            "ReturnBazaarItem",
            format!(
                r#""escrow_id":"{HOLD}","expected_escrow_revision":"1","seller_character_id":"{CHARACTER}","escrow_receipt_digest":"{DIGEST}""#
            ),
        ),
        (
            "HoldBazaarGold",
            format!(
                r#""buyer_account_id":"{BUYER}","buyer_character_id":"{CHARACTER}","offer_id":"{OP}","hold_purpose":"OFFER","amount":"10000","expected_hold_revision":"0""#
            ),
        ),
        (
            "ReleaseBazaarGold",
            format!(
                r#""buyer_account_id":"{BUYER}","hold_id":"{HOLD}","expected_hold_revision":"1","release_reason":"EXCESS","amount":"10000""#
            ),
        ),
        (
            "PayoutBazaarGold",
            format!(
                r#""buyer_account_id":"{BUYER}","hold_id":"{HOLD}","expected_hold_revision":"1","sale_id":"{OP}","gross_amount":"10000","fee_bps":250,"fee_amount":"250","seller_amount":"9750","delivery_operation_id":"{OP}","delivery_receipt_digest":"{DIGEST}""#
            ),
        ),
    ]
}

#[test]
fn bazaar_all_nine_intent_schemas_preserve_bytes_and_time_boundaries() -> TestResult {
    for (command, members) in all_commands() {
        let wire = intent_wire(command, &members);
        let decoded = decode_intent(wire.as_bytes(), command, OP);
        assert!(decoded.is_ok(), "{command}: {decoded:?}");
        let intent = decoded?;
        assert_eq!(intent.encode()?, wire.as_bytes(), "{command}");
        assert!(intent.time_valid(1_789_999_995));
        assert!(!intent.time_valid(1_789_999_994));
        assert!(intent.time_valid(1_790_000_299));
        assert!(!intent.time_valid(1_790_000_300));
        assert!(
            decode_intent(
                wire.replace(
                    "\"source_revision\":\"18\"",
                    "\"source_revision\":\"18446744073709551616\""
                )
                .as_bytes(),
                command,
                OP
            )
            .is_err()
        );
        assert!(
            decode_intent(
                wire.replace("\"contract_version\":2", "\"contract_version\":1")
                    .as_bytes(),
                command,
                OP
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn bazaar_funding_and_binding_nulls_are_required_and_currency_scoped() -> TestResult {
    for (command, members) in all_commands() {
        if !matches!(command, "BindBazaarCharacterTransfer" | "DeliverBazaarItem") {
            continue;
        }
        let wire = intent_wire(command, &members);
        for invalid in [
            wire.replace("\"hold_revision\":\"1\"", "\"hold_revision\":null"),
            wire.replace("\"hold_revision\":\"1\",", ""),
            wire.replace("\"currency\":\"GOLD\"", "\"currency\":\"COINS\""),
            wire.replace(
                "\"gross_amount\":\"10000\"",
                "\"gross_amount\":\"1000000000000\"",
            ),
        ] {
            assert!(decode_intent(invalid.as_bytes(), command, OP).is_err());
        }
        let coins = wire
            .replace("\"currency\":\"GOLD\"", "\"currency\":\"COINS\"")
            .replace("\"hold_revision\":\"1\"", "\"hold_revision\":null");
        assert!(decode_intent(coins.as_bytes(), command, OP).is_ok());
        assert!(
            decode_intent(
                coins
                    .replace(
                        "\"gross_amount\":\"10000\"",
                        "\"gross_amount\":\"9223372036854775808\""
                    )
                    .as_bytes(),
                command,
                OP
            )
            .is_err()
        );
        if command == "BindBazaarCharacterTransfer" {
            let unbound = wire.replace(
                "\"purpose\":\"BAZAAR_SETTLEMENT\"",
                "\"purpose\":\"BAZAAR_LISTING_ESCROW\"",
            );
            assert!(decode_intent(unbound.as_bytes(), command, OP).is_err());
            let start = unbound.find(",\"sale_id\":").ok_or("missing sale member")?;
            let end = unbound
                .find(",\"issued_at_source\":")
                .ok_or("missing time member")?;
            let correct = format!(
                "{},\"sale_id\":null,\"funding\":null{}",
                &unbound[..start],
                &unbound[end..]
            );
            assert!(decode_intent(correct.as_bytes(), command, OP).is_ok());
            assert!(
                decode_intent(
                    correct.replace(",\"funding\":null", "").as_bytes(),
                    command,
                    OP
                )
                .is_err()
            );
        }
    }
    Ok(())
}
