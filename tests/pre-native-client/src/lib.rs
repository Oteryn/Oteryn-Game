//! Pre-native product-state acceptance package.

#[cfg(test)]
mod tests {
    use oteryn_client::{ClientBootstrap, GameplayEntryError, pre_native_status};
    use oteryn_platform_contracts::GameplayAvailability;
    use std::error::Error;

    #[test]
    fn product_state_is_explicit_and_fail_closed() -> Result<(), Box<dyn Error>> {
        let client = ClientBootstrap::new()?;
        assert_eq!(
            client.request_gameplay_entry(),
            Err(GameplayEntryError::NativeProtocolUnavailable)
        );
        assert_eq!(
            client.availability(),
            GameplayAvailability::PreNativeProtocol
        );
        assert!(pre_native_status().contains("not available"));
        client.shutdown();
        Ok(())
    }
}
