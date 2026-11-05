//! Existing world-context codecs exposed at the client session boundary.
pub use oteryn_protocol_oteryn::{bed, item_look, world_object};

#[cfg(test)]
mod tests {
    use crate::capability_support::{CLIENT_SUPPORTED_CAPABILITIES, gated_routes};
    #[test]
    fn look_and_bed_are_advertised_and_routed() {
        for (capability, command) in [(28, 30), (11, 31)] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
            assert!(
                gated_routes()
                    .any(|(id, commands, _)| id == capability && commands.contains(&command))
            );
        }
    }
}
