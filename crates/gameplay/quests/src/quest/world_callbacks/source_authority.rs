//! Exact pinned donor revisions accepted by the callback source registry.
pub(in crate::quest) fn source_backend(source: &str) -> Option<&'static str> {
    if source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
        || source.starts_with("crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
    {
        Some("crystalserver")
    } else if source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:") {
        Some("canary")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::source_backend;
    #[test]
    fn merged_native_callbacks_retain_their_pinned_authority() {
        let source =
            "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/orockle.lua";
        assert_eq!(source_backend(source), Some("crystalserver"), "{source}");
    }
    #[test]
    fn unknown_revisions_and_spoofed_authority_fail_closed() {
        for source in [
            "",
            "crystalserver:unknown:data-global/npc/orockle.lua",
            "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/orockle.lua",
            "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6e:data-global/npc/orockle.lua",
            "untrusted:crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/orockle.lua",
        ] {
            assert_eq!(source_backend(source), None);
        }
    }
    #[test]
    fn every_registered_callback_retains_known_source_authority() -> Result<(), String> {
        crate::quest::loader::load_embedded_quest_state("authority-regression")
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    }
}
