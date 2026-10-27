//! Exercise the production feature scanner without editing any existing registration list.

#[path = "../build_support/play_features.rs"]
mod play_features;

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Result<Self, String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/client5b-fragments")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(path.join("features")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("crates/protocol-oteryn/capabilities"))
            .map_err(|e| e.to_string())?;
        Ok(Self(path))
    }

    fn feature(&self, name: &str, event: &str, handler: &str) -> Result<(), String> {
        let feature = self.0.join("features").join(name);
        fs::create_dir_all(&feature).map_err(|e| e.to_string())?;
        fs::write(feature.join("protocol.rs"), format!("fn {handler}() {{}}"))
            .map_err(|e| e.to_string())?;
        fs::write(
            feature.join("play.json"),
            json!({"concerns":["protocol"],"events":{event:handler}}).to_string(),
        )
        .map_err(|e| e.to_string())
    }

    fn generate(&self) -> Result<String, String> {
        play_features::generate(&self.0.join("features"), &self.0).map(|(rust, _)| rust)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_new_feature_extends_dispatch_without_editing_existing_sources() -> Result<(), String> {
    let f = Fixture::new()?;
    f.feature("local-zebra", "Zebra", "reduce_zebra")?;
    let original =
        fs::read(f.0.join("features/local-zebra/play.json")).map_err(|e| e.to_string())?;
    f.feature("local-alpha", "Alpha", "reduce_alpha")?;
    fs::create_dir_all(f.0.join("features/local-alpha/tests")).map_err(|e| e.to_string())?;
    fs::write(
        f.0.join("features/local-alpha/tests/play.rs"),
        "#[test] fn alpha() {}",
    )
    .map_err(|e| e.to_string())?;
    let (generated, tests) = play_features::generate(&f.0.join("features"), &f.0)?;
    assert!(tests.contains("mod feature_local_alpha"));
    assert!(!tests.contains("mod feature_local_zebra"));
    assert!(
        generated
            .contains("SessionEvent::Alpha(delta) => feature_local_alpha::protocol::reduce_alpha")
    );
    assert!(
        generated
            .contains("SessionEvent::Zebra(delta) => feature_local_zebra::protocol::reduce_zebra")
    );
    assert_eq!(
        fs::read(f.0.join("features/local-zebra/play.json")).map_err(|e| e.to_string())?,
        original
    );
    fs::remove_dir_all(f.0.join("features/local-zebra")).map_err(|e| e.to_string())?;
    f.feature("local-zebra", "Zebra", "reduce_zebra")?;
    assert_eq!(generated, f.generate()?);
    Ok(())
}

#[test]
fn duplicate_events_invalid_symbols_and_unknown_mechanics_are_rejected() -> Result<(), String> {
    let f = Fixture::new()?;
    f.feature("local-alpha", "Same", "reduce")?;
    f.feature("local-beta", "Same", "reduce")?;
    assert!(
        f.generate()
            .is_err_and(|e| e.contains("duplicate play event"))
    );
    f.feature("local-beta", "Other", "../escape")?;
    assert!(
        f.generate()
            .is_err_and(|e| e.contains("invalid play event/handler"))
    );
    fs::remove_dir_all(f.0.join("features/local-beta")).map_err(|e| e.to_string())?;
    f.feature("9999-missing", "Other", "reduce")?;
    assert!(
        f.generate()
            .is_err_and(|e| e.contains("unknown play mechanic"))
    );
    Ok(())
}
