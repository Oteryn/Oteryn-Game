// Generated from capabilities/*.json; keep registration and acceptance closure together.
include!(concat!(env!("OUT_DIR"), "/registered_capabilities.rs"));

#[cfg(test)]
#[path = "../build_capabilities.rs"]
mod build_capabilities;
#[cfg(test)]
#[path = "capability_fragments_tests.rs"]
mod tests;
