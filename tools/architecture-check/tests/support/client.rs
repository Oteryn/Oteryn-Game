use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Fixture(pub(super) PathBuf);

impl Fixture {
    pub(super) fn new() -> Result<Self, String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            ".client-guard-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        Ok(Self(path))
    }

    pub(super) fn write(&self, path: &str, text: &str) -> Result<(), String> {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().ok_or("parent")?).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())
    }

    pub(super) fn json(&self, path: &str, value: &Value) -> Result<(), String> {
        self.write(
            path,
            &serde_json::to_string_pretty(value).map_err(|e| e.to_string())?,
        )
    }

    pub(super) fn structure() -> Result<Self, String> {
        let fixture = Self::new()?;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        fixture.write(
            "tools/architecture-check/client-fragment.schema.json",
            &fs::read_to_string(root.join("client-fragment.schema.json"))
                .map_err(|e| e.to_string())?,
        )?;
        fixture.json(
            "tools/architecture-check/client-size-baseline.json",
            &json!({}),
        )?;
        fixture.json(
            "tools/architecture-check/client-unsupported-baseline.json",
            &json!(["0057-cyclopedia-map"]),
        )?;
        fixture.json(
            "crates/protocol-oteryn/capabilities/0057-cyclopedia-map-v1.json",
            &json!({"id":57,"name":"CYCLOPEDIA_MAP_V1","commands":[72,73]}),
        )?;
        fixture.entries(&[json!({"mechanic":"0057-cyclopedia-map","kind":"handlers","id":"map-unsupported","status":"unsupported","reason":"pending migration","commands":[72,73],"not_applicable":{"state":"pending","protocol":"pending","view":"pending","input":"pending","settings":"pending"}})])?;
        Ok(fixture)
    }

    pub(super) fn entries(&self, entries: &[Value]) -> Result<(), String> {
        let dir = "crates/client-ui/src/features/0057-cyclopedia-map";
        let registrations = self.0.join(format!("{dir}/registrations"));
        if registrations.exists() {
            fs::remove_dir_all(&registrations).map_err(|e| e.to_string())?;
        }
        for (index, entry) in entries.iter().enumerate() {
            self.json(&format!("{dir}/registrations/{index:04}.json"), entry)?;
        }
        let mut entries = entries.to_vec();
        entries.sort_by_key(|e| e["id"].as_str().unwrap_or_default().to_owned());
        self.json(
            &format!("{dir}/fixtures/registrations.json"),
            &json!(entries),
        )
    }

    pub(super) fn current(&self) -> Result<Vec<Value>, String> {
        serde_json::from_str(
            &fs::read_to_string(self.0.join(
                "crates/client-ui/src/features/0057-cyclopedia-map/fixtures/registrations.json",
            ))
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
