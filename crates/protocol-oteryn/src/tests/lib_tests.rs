// Included files share this module to preserve test paths and private helper access.
#[path = "capability_selection_tests.rs"]
mod capability_selection_tests;

use super::*;

include!("codec_seam_tests.rs");
include!("support.rs");
include!("handshake_tests.rs");
include!("client_message_tests.rs");
include!("envelope_tests.rs");
include!("error_registry_tests.rs");
include!("command_tests.rs");
include!("state_sync_tests.rs");

#[test]
fn module_registration_block_is_sorted_and_complete() -> Result<(), &'static str> {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let (before, rest) = source
        .split_once("// Keep sorted by module name; insert in order, never append.\n")
        .ok_or("module registration block header")?;
    let (block, after) = rest
        .split_once("// End sorted module declarations.\n")
        .ok_or("module registration block footer")?;

    fn module_name(line: &str) -> Option<&str> {
        let line = line.trim();
        let declaration = if let Some(rest) = line.strip_prefix("pub(") {
            rest.split_once(')')?.1.trim_start()
        } else {
            line.strip_prefix("pub ").unwrap_or(line)
        };
        declaration.strip_prefix("mod ").map(|name| {
            assert!(name.ends_with(';'), "one module declaration per line");
            name.trim_end_matches(';')
        })
    }

    for line in before.lines().chain(after.lines()) {
        assert!(module_name(line).is_none(), "module outside block: {line}");
    }
    let names: Vec<_> = block.lines().filter_map(module_name).collect();
    assert!(!names.is_empty(), "module registration block is empty");
    for pair in names.windows(2) {
        assert!(pair[0] < pair[1], "modules out of order: {pair:?}");
    }
    Ok(())
}
