//! Pre-native product-state acceptance package.

#[cfg(test)]
mod tests {
    use oteryn_client::{ClientBootstrap, GameplayEntryError, pre_native_status};
    use oteryn_platform_contracts::GameplayAvailability;
    use std::error::Error;

    #[test]
    fn product_state_is_explicit_and_fail_closed() -> Result<(), Box<dyn Error>> {
        // N4-1 contract: a client without a native login configuration fails before any
        // directory or credential request; no Platform or Gateway endpoint is known to it.
        let client = ClientBootstrap::new()?;
        assert!(matches!(
            client.request_gameplay_entry(),
            Err(GameplayEntryError::NativeProtocolUnavailable)
        ));
        assert_eq!(
            client.availability(),
            GameplayAvailability::PreNativeProtocol
        );
        assert!(pre_native_status().contains("not available"));
        client.shutdown();
        Ok(())
    }
}
