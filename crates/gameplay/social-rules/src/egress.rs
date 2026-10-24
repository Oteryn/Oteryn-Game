//! Session chat queue bounds and line drop priorities.

/// The delivery class of a line, independent of its wire payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineClass {
    Room,
    Local,
    Private,
    Dropped,
}

/// `CHAT0-RL-11`: undelivered lines per session.
pub const EGRESS_MAX_LINES: usize = 64;

/// Drop order: a higher rank is dropped first.
pub fn drop_rank(line: LineClass) -> u8 {
    match line {
        LineClass::Room => 3,
        LineClass::Local => 2,
        LineClass::Private | LineClass::Dropped => 1,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn egress_bound_is_the_registered_chat0_rl_11_row() {
        let registry: serde_json::Value =
            serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON)
                .expect("resource registry");
        let rows: Vec<&serde_json::Value> = registry["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .filter(|entry| entry["id"] == "CHAT0-RL-11")
            .collect();
        assert_eq!(rows.len(), 1, "CHAT0-RL-11 registered once");
        assert_eq!(
            rows[0]["hard_maximum"].as_u64(),
            Some(EGRESS_MAX_LINES as u64)
        );
    }
}
