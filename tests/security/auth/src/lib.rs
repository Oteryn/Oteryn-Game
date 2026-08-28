//! Authentication security regression package.

#[cfg(test)]
mod tests {
    use oteryn_client_runtime::ClientRuntime;
    use oteryn_diagnostics::{DiagnosticValue, SensitiveKind};
    use oteryn_foundation::CancellationToken;
    use oteryn_identity::{CallbackResult, IdentityError, IdentityFlow, StateNonce};
    use oteryn_platform_client::{PlatformClientError, parse_directory};
    use std::error::Error;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[test]
    fn duplicate_callback_parameters_fail_closed() {
        assert_eq!(
            CallbackResult::parse_query("code=one&code=two&state=s"),
            Err(IdentityError::InvalidCallback)
        );
    }

    #[test]
    fn callback_cancellation_has_no_secret_output() -> Result<(), Box<dyn Error>> {
        let runtime = ClientRuntime::new()?;
        let state = StateNonce::new("state")?;
        let flow = IdentityFlow::new(state, Duration::from_secs(1));
        let (_sender, mut receiver) = mpsc::channel(1);
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let result = runtime.block_on(flow.await_callback(&mut receiver, cancellation))?;
        assert_eq!(result, Err(IdentityError::Cancelled));
        runtime.shutdown(Duration::from_millis(50));

        let marker = "synthetic-session-secret";
        let value = DiagnosticValue::redacted(SensitiveKind::SessionSecret, marker);
        assert!(!format!("{value:?}").contains(marker));
        assert!(!value.to_string().contains(marker));
        Ok(())
    }

    #[test]
    fn directory_rejects_nested_gameplay_routes() {
        let payload = br#"{"epoch":1,"worlds":[{"id":"w","name":"W","channels":[],"characters":[],"metadata":{"port":7171}}]}"#;
        assert_eq!(
            parse_directory(payload),
            Err(PlatformClientError::ForbiddenGameplayField)
        );
    }
}
