//! The compiler-facing bundle API; format writing is shared with native fixtures.

pub use oteryn_world_bundle::bundle::*;
pub use oteryn_world_bundle::writer::{write, write_with_npcs};

/// The `compiler_version` this build writes: the crate version and the zstd library version,
/// because both decide the output bytes (format document §6).
pub fn compiler_version() -> String {
    let zstd = zstd::zstd_safe::version_number();
    format!(
        "oteryn-world-bundle-compiler/{} zstd/{}.{}.{}",
        env!("CARGO_PKG_VERSION"),
        zstd / 10000,
        zstd / 100 % 100,
        zstd % 100
    )
}
