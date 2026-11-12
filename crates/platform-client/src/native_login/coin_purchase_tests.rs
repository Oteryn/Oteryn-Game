use super::*;
use crate::native_login::coin_purchase::*;

#[test]
fn coin_purchase_wire_refuses_missing_fields_real_provider_and_foreign_checkout() -> TestResult {
    let runtime = tokio::runtime::Runtime::new()?;
    let valid = serde_json::json!({"version":1,"order_id":CHARACTER,"request_id":ATTEMPT,"revision":1,"package_id":WORLD,"quote_id":CHARACTER,"character_id":null,"coins":100,"price":{"minor_units":499,"currency":"USD","exponent":2},"sandbox":true,"state":"awaiting_payment","checkout_url":"https://untrusted.invalid/checkout","receipt_id":null,"review_required":false,"code":"SANDBOX"});
    let mut bodies = vec![valid.clone()];
    for (key, value) in [
        ("version", serde_json::json!(2)),
        ("sandbox", serde_json::json!(false)),
        ("request_id", serde_json::json!(WORLD)),
        ("state", serde_json::json!("paid")),
        ("coins", serde_json::json!(0)),
    ] {
        let mut body = valid.clone();
        body[key] = value;
        bodies.push(body);
    }
    let mut missing = valid.clone();
    missing
        .as_object_mut()
        .ok_or("object")?
        .remove("receipt_id");
    bodies.push(missing);
    for body in bodies {
        let server = spawn_server(
            &runtime,
            vec![
                (
                    200,
                    "",
                    r#"{"token_type":"Bearer","access_token":"services"}"#.into(),
                ),
                (200, "", body.to_string()),
            ],
        )?;
        let client = client_for(&server)?;
        runtime.block_on(async {
            let token = client
                .exchange_code("code", "pkce", "redirect", CancellationToken::new())
                .await?;
            assert!(
                client
                    .coin_order_status(&token, ATTEMPT, CancellationToken::new())
                    .await
                    .is_err()
            );
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
    }
    Ok(())
}

#[test]
fn coin_checkout_policy_binds_origin_path_and_order_without_url_credentials() -> TestResult {
    let runtime = tokio::runtime::Runtime::new()?;
    let server = spawn_server(&runtime, vec![])?;
    let client = client_for(&server)?;
    // client_for uses this fixture's exact configured origin.
    let url = client
        .platform
        .join(&format!("/native-account/coin-checkout/{ATTEMPT}"))?;
    let link: CheckoutLink = serde_json::from_value(serde_json::json!(url.as_str()))?;
    assert!(client.valid_coin_checkout(&link, ATTEMPT));
    for suffix in ["?success=1", "#paid", "/extra"] {
        let link: CheckoutLink =
            serde_json::from_value(serde_json::json!(format!("{url}{suffix}")))?;
        assert!(!client.valid_coin_checkout(&link, ATTEMPT));
    }
    let mut credentialed = url.clone();
    let _ = credentialed.set_username("payer");
    let link: CheckoutLink = serde_json::from_value(serde_json::json!(credentialed.as_str()))?;
    assert!(!client.valid_coin_checkout(&link, ATTEMPT));
    Ok(())
}
