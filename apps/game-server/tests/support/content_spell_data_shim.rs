// Minimal `crate::content` for test binaries that path-load Foundation/combat but no other
// Content module; see `spell_data_shim.rs`.

#[path = "spell_data_shim.rs"]
pub mod spell_data;
