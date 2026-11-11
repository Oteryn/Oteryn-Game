//! Match migration 1286's exact command-scoped counter claim without changing paid claims.
pub(super) fn key(callback: &str, session: &[u8; 16], ordinal: u64) -> String {
    let session = session
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("{callback}/occurrence/{session}/{ordinal}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn counter_claim_matches_sql_uuid_and_unsigned_ordinal_encoding() {
        assert_eq!(
            super::key("service", &[0; 16], 1),
            "service/occurrence/00000000000000000000000000000000/1"
        );
        assert_eq!(
            super::key("service", &[255; 16], u64::MAX),
            "service/occurrence/ffffffffffffffffffffffffffffffff/18446744073709551615"
        );
    }
}
