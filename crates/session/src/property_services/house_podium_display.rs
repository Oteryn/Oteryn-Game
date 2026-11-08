//! Wire adapter and explicit native session-route coverage.
pub use oteryn_protocol_oteryn::house_podium_display::*;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn podium_display_is_advertised_and_uses_the_real_gated_route() {
        assert!(
            crate::capability_support::CLIENT_SUPPORTED_CAPABILITIES
                .contains(&CAPABILITY_HOUSE_PODIUM_DISPLAY_V1)
        );
        assert!(
            crate::capability_support::gated_routes().any(|(id, commands, domains)| id
                == CAPABILITY_HOUSE_PODIUM_DISPLAY_V1
                && commands == [COMMAND_TYPE_HOUSE_PODIUM_DISPLAY]
                && domains.is_empty())
        );
    }
}
