//! Shared modal input validation. Server authority remains with the caller.
/// Multiline editor with a wire-sized UTF-8 byte budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditor {
    pub text: String,
    maximum_bytes: usize,
}
impl TextEditor {
    pub fn new(text: &str, maximum_bytes: usize) -> Self {
        Self {
            text: text.into(),
            maximum_bytes,
        }
    }
    pub fn submit(&self) -> Option<&str> {
        (self.text.len() <= self.maximum_bytes && !self.text.contains('\0'))
            .then_some(self.text.as_str())
    }
}

/// Versioned notice keys allow a revised warning to be shown again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DismissedNotices(std::collections::BTreeSet<String>);
impl DismissedNotices {
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains(key)
    }
    pub fn dismiss(&mut self, key: &str) -> bool {
        if key.is_empty() || key.len() > 128 || key.contains('\0') || self.0.len() >= 256 {
            return false;
        }
        self.0.insert(key.into());
        true
    }
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self.0)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() > 65536 {
            return Err("Notice preferences exceed 64 KiB".into());
        }
        let keys: Vec<String> = serde_json::from_slice(bytes)?;
        let mut value = Self::default();
        for key in keys {
            if !value.dismiss(&key) {
                return Err("Invalid dismissed notice key".into());
            }
        }
        Ok(value)
    }
    /// Persist only after an explicit user dismissal. Failed writes preserve the old file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = self.encode().map_err(std::io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(&temporary, path)
    }
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        use std::io::Read;
        let mut bytes = Vec::new();
        match std::fs::File::open(path) {
            Ok(file) => {
                file.take(65537).read_to_end(&mut bytes)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        }
        Self::decode(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_preserves_multiline_text_and_checks_byte_limit() {
        let mut editor = TextEditor::new("first\nsecond", 16);
        assert_eq!(editor.submit(), Some("first\nsecond"));
        editor.text = "é".repeat(9);
        assert_eq!(editor.submit(), None);
        editor.text = "bad\0text".into();
        assert_eq!(editor.submit(), None);
    }
    #[test]
    fn dismissed_confirmation_is_written_to_disk() -> Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::temp_dir().join(format!("cg-modal-notices-{}.json", std::process::id()));
        let mut dismissed = DismissedNotices::default();
        assert!(dismissed.dismiss("notice:v1"));
        dismissed.save(&path)?;
        let restored = DismissedNotices::load(&path)?;
        std::fs::remove_file(path)?;
        assert!(restored.contains("notice:v1"));
        Ok(())
    }
    #[test]
    fn dismissed_confirmation_survives_reload() -> Result<(), Box<dyn std::error::Error>> {
        let mut dismissed = DismissedNotices::default();
        assert!(dismissed.dismiss("balance:2026-11"));
        let bytes = dismissed.encode()?;
        let loaded = DismissedNotices::decode(&bytes)?;
        assert!(loaded.contains("balance:2026-11"));
        assert!(!loaded.contains("balance:2026-12"));
        Ok(())
    }
}

#[cfg(test)]
mod server_dialog_tests {
    use super::*;
    #[test]
    fn server_dialog_keeps_ids_and_content_image_reference() {
        let dialog = ServerDialog {
            id: 41,
            title: "Choose a reward".into(),
            message: "One choice".into(),
            choices: vec![(7, "Sword".into()), (9, "Shield".into())],
            buttons: vec![(2, "Accept".into()), (4, "Cancel".into())],
            image: Some("oteryn:ui.reward".into()),
        };
        assert!(dialog.valid());
        assert!(dialog.text().contains("7: Sword"));
        assert!(dialog.text().contains("oteryn:ui.reward"));
        let mut invalid = dialog.clone();
        invalid.buttons.push((2, "Duplicate".into()));
        assert!(!invalid.valid());
        invalid = dialog;
        invalid.image = Some("https://example.org/image.png".into());
        assert!(!invalid.valid());
    }
}

/// Presentation of an identified server modal. Image keys resolve through client assets;
/// arbitrary network URLs are never fetched by the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerDialog {
    pub id: u32,
    pub title: String,
    pub message: String,
    pub choices: Vec<(u8, String)>,
    pub buttons: Vec<(u8, String)>,
    pub image: Option<String>,
}
impl ServerDialog {
    pub fn valid(&self) -> bool {
        fn items_valid(items: &[(u8, String)]) -> bool {
            items.len() <= 32
                && items.iter().enumerate().all(|(index, (id, text))| {
                    !text.is_empty()
                        && text.len() <= 256
                        && !text.contains('\0')
                        && !items[..index].iter().any(|(prior, _)| prior == id)
                })
        }
        self.title.len() <= 256
            && !self.title.contains('\0')
            && self.message.len() <= 8192
            && !self.message.contains('\0')
            && !self.buttons.is_empty()
            && items_valid(&self.buttons)
            && items_valid(&self.choices)
            && self.image.as_ref().is_none_or(|key| {
                key.starts_with("oteryn:")
                    && key.len() <= 128
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_:.-".contains(&b))
            })
    }
    pub fn text(&self) -> String {
        let mut text = format!("{}\r\n{}\r\n", self.title, self.message);
        for (id, choice) in &self.choices {
            text.push_str(&format!("{id}: {choice}\r\n"));
        }
        if let Some(image) = &self.image {
            text.push_str(&format!("Image: {image}\r\n"));
        }
        text
    }
}
