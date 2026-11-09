//! Local point-only presets. Character-bound gem identities are never shared.
use super::{SLOT_CAPACITIES, WheelPanel};
use std::{collections::BTreeMap, io::Read, path::Path};

const MAX_PRESETS: usize = 32;
const MAX_FILE: u64 = 32768;
const MAX_CODE: usize = 256;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresetBook(pub BTreeMap<String, String>);
fn decode(code: &str) -> Result<[u16; 36], &'static str> {
    if code.len() > MAX_CODE {
        return Err(tr!("Build code is too long"));
    }
    let body = code
        .trim()
        .strip_prefix("OW1:")
        .or_else(|| code.trim().strip_prefix("oteryn://wheel/v1/"))
        .ok_or(tr!("Use an Oteryn Wheel code or URL"));
    let mut points = [0; 36];
    let mut values = body?.split(',');
    for (slot, point) in points.iter_mut().enumerate() {
        *point = values
            .next()
            .ok_or(tr!("Build needs 36 slots"))?
            .parse()
            .map_err(|_| tr!("Invalid slot points"))?;
        if *point > SLOT_CAPACITIES[slot] {
            return Err(tr!("Slot points exceed capacity"));
        }
    }
    if values.next().is_some() {
        return Err(tr!("Build needs exactly 36 slots"));
    }
    Ok(points)
}
impl WheelPanel {
    pub fn build_code(&self) -> String {
        format!(
            "OW1:{}",
            self.points
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    }
    pub fn import_build(&mut self, code: &str) -> Result<(), &'static str> {
        if !self.ready() {
            return Err(tr!("Refresh first and settle any pending request"));
        }
        let points = decode(code)?;
        self.points = points;
        self.feedback = Some(tr!(
            "Build loaded as a proposal; Save Wheel applies it through the server"
        ));
        Ok(())
    }
}
impl PresetBook {
    pub fn insert(&mut self, name: &str, code: &str) -> Result<(), &'static str> {
        let name = name.trim();
        if name.is_empty() || name.len() > 64 || name.chars().any(char::is_control) {
            return Err(tr!("Use a preset name of 1 to 64 bytes"));
        }
        if self.0.len() >= MAX_PRESETS && !self.0.contains_key(name) {
            return Err(tr!("At most 32 presets"));
        }
        decode(code)?;
        self.0.insert(name.into(), code.into());
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, &'static str> {
        if !path.exists() {
            return Ok(Self::default());
        }
        if std::fs::metadata(path)
            .map_err(|_| tr!("Cannot inspect presets"))?
            .len()
            > MAX_FILE
        {
            return Err(tr!("Preset file is too large"));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| tr!("Cannot read presets"))?
            .take(MAX_FILE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| tr!("Cannot read presets"))?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(tr!("Preset file is too large"));
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| tr!("Invalid preset file"))?;
        if value["version"] != 1 {
            return Err(tr!("Unsupported preset version"));
        }
        let entries = value["presets"]
            .as_object()
            .ok_or(tr!("Invalid preset list"))?;
        let mut book = Self::default();
        for (name, code) in entries {
            book.insert(name, code.as_str().ok_or(tr!("Invalid preset code"))?)?;
        }
        Ok(book)
    }
    pub fn save(&self, path: &Path) -> Result<(), &'static str> {
        let mut validated = Self::default();
        for (name, code) in &self.0 {
            validated.insert(name, code)?;
        }
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({"version":1,"presets":self.0}))
            .map_err(|_| tr!("Cannot encode presets"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| tr!("Cannot create preset directory"))?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|_| tr!("Cannot write presets"))?;
        std::fs::rename(temporary, path).map_err(|_| tr!("Cannot save presets"))
    }
}
