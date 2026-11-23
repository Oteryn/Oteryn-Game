//! Local point-only presets. Character-bound gem identities are never shared.
use super::{SLOT_CAPACITIES, WheelPanel};
use std::{collections::BTreeMap, io::Read, path::Path};

const MAX_PRESETS: usize = 32;
const MAX_FILE: u64 = 32768;
const MAX_CODE: usize = 256;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresetBook(
    pub BTreeMap<String, String>,
    std::collections::BTreeSet<String>,
);
const EMPTY_VESSELS: &str = "#-,-,-,-";
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
type GemSlots = Vec<Option<[u8; 16]>>;
fn named_parts(code: &str) -> Result<(&str, Option<GemSlots>), &'static str> {
    let Some((points, gems)) = code.split_once('#') else {
        return Ok((code, None));
    };
    let (count, fields) = gems.strip_prefix("v2:").map_or((4, gems), |v| (12, v));
    if fields.len() > count * 33 || fields.split(',').count() != count {
        return Err("Invalid preset vessel count");
    }
    let mut slots = Vec::with_capacity(count);
    for field in fields.split(',') {
        if field == "-" {
            slots.push(None);
            continue;
        }
        if field.len() != 32 || !field.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid preset gem");
        }
        let id = u128::from_str_radix(field, 16)
            .map_err(|_| "Invalid preset gem")?
            .to_be_bytes();
        if id == [0; 16] || slots.contains(&Some(id)) {
            return Err("Invalid preset gem");
        }
        slots.push(Some(id));
    }
    Ok((points, Some(slots)))
}
impl WheelPanel {
    pub fn named_build_code(&self) -> String {
        let mut code = self.build_code();
        let slots = if self.atelier_v2 {
            let Some(board) = self.vessel_board.as_ref().filter(|b| b.slots.len() == 12) else {
                return code;
            };
            code.push_str("#v2:");
            board.slots.iter().map(|s| s.gem).collect::<Vec<_>>()
        } else if self
            .confirmed_wheel
            .as_ref()
            .is_some_and(|w| w.vessels.is_some())
            || self.vessels.iter().any(Option::is_some)
        {
            code.push('#');
            self.vessels.to_vec()
        } else {
            return code;
        };
        code.push_str(
            &slots
                .iter()
                .map(|id| {
                    id.map_or_else(
                        || "-".into(),
                        |id| format!("{:032x}", u128::from_be_bytes(id)),
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        code
    }
    pub fn load_named_build(&mut self, code: &str) -> Result<(), &'static str> {
        let (points, slots) = named_parts(code)?;
        if let Some(slots) = &slots {
            if self.atelier_v2 != (slots.len() == 12) {
                return Err("Preset uses a different vessel layout");
            }
            for (slot, id) in slots.iter().enumerate() {
                let Some(id) = id else { continue };
                let gem = self
                    .confirmed_atelier
                    .as_ref()
                    .and_then(|a| a.gems.iter().find(|g| g.id == *id))
                    .ok_or("A saved gem is unavailable for its vessel")?;
                let valid = if self.atelier_v2 {
                    self.vessel_board
                        .as_ref()
                        .and_then(|b| b.slots.get(slot))
                        .is_some_and(|v| v.open && v.quality == gem.quality)
                } else {
                    usize::from(gem.domain) == slot
                };
                if !valid {
                    return Err("A saved gem is unavailable for its vessel");
                }
            }
        }
        self.import_build(points)?;
        if self.atelier_v2 {
            self.preset_vessels = slots;
        } else if let Some(slots) = slots {
            self.vessels.copy_from_slice(&slots);
        }
        Ok(())
    }
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
        self.preset_vessels = None;
        // Every OW1 code predates the Gem Vessel rework of the Wheel.
        self.feedback = Some(
            "You have imported an older Wheel of Destiny configuration. Some values may have changed. Save Wheel applies it through the server",
        );
        Ok(())
    }
}
impl PresetBook {
    /// Older point-only records leave vessels alone. New empty builds explicitly clear them.
    pub fn named_code(&self, name: &str) -> Option<String> {
        let mut code = self.0.get(name)?.clone();
        if self.1.contains(name) {
            code.push_str(EMPTY_VESSELS)
        }
        Some(code)
    }
    pub fn remove(&mut self, name: &str) -> Option<String> {
        let code = self.named_code(name)?;
        self.0.remove(name);
        self.1.remove(name);
        Some(code)
    }
    pub fn encoded(&self) -> Result<Vec<u8>, &'static str> {
        let mut entries = BTreeMap::new();
        for name in self.0.keys() {
            let code = self.named_code(name).ok_or(tr!("Invalid preset code"))?;
            decode(named_parts(&code)?.0)?;
            entries.insert(name, code);
        }
        serde_json::to_vec_pretty(&serde_json::json!({"version":1,"presets":entries}))
            .map_err(|_| tr!("Cannot encode presets"))
    }

    pub fn insert(&mut self, name: &str, code: &str) -> Result<(), &'static str> {
        let name = name.trim();
        if name.is_empty() || name.len() > 64 || name.chars().any(char::is_control) {
            return Err(tr!("Use a preset name of 1 to 64 bytes"));
        }
        if self.0.len() >= MAX_PRESETS && !self.0.contains_key(name) {
            return Err(tr!("At most 32 presets"));
        }
        decode(named_parts(code)?.0)?;
        if let Some(points) = code.strip_suffix(EMPTY_VESSELS) {
            self.1.insert(name.into());
            self.0.insert(name.into(), points.into());
        } else {
            self.1.remove(name);
            self.0.insert(name.into(), code.into());
        }
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
        let bytes = self.encoded()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| tr!("Cannot create preset directory"))?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|_| tr!("Cannot write presets"))?;
        std::fs::rename(temporary, path).map_err(|_| tr!("Cannot save presets"))
    }
}
