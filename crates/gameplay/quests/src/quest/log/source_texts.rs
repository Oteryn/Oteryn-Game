//! Resolve captured journal prose by its exact content reference; never execute source code.

use std::collections::BTreeMap;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::super::loader::QuestTextRef;

#[derive(Deserialize)]
struct Registry {
    schema: String,
    texts: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    classification: String,
    reference: QuestTextRef,
    text: String,
}

pub(super) struct SourceTexts(BTreeMap<String, Entry>);

impl SourceTexts {
    pub(super) fn parse(text: &str) -> Result<Self, String> {
        let registry: Registry = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if registry.schema != "OTERYN_SOURCE_TEXT_REGISTRY/v1" {
            return Err("journal text registry schema".into());
        }
        let mut entries = BTreeMap::new();
        for entry in registry.texts {
            // The source authoring reference counts Unicode scalars; SHA-256 binds UTF-8 bytes.
            if entry.classification != "KNOWN"
                || entry.reference.length != entry.text.chars().count() as u64
                || !digest_matches(&entry.reference.sha256, &entry.text)
            {
                return Err("journal text registry integrity".into());
            }
            if entries
                .insert(entry.reference.sha256.clone(), entry)
                .is_some()
            {
                return Err("duplicate journal text".into());
            }
        }
        Ok(Self(entries))
    }

    pub(super) fn resolve(&self, reference: &QuestTextRef) -> Result<&str, String> {
        let entry = self
            .0
            .get(&reference.sha256)
            .ok_or("missing journal text")?;
        if &entry.reference != reference {
            return Err("journal text reference mismatch".into());
        }
        Ok(&entry.text)
    }
}

fn digest_matches(reference: &str, text: &str) -> bool {
    reference.len() == 64
        && reference.is_ascii()
        && Sha256::digest(text.as_bytes())
            .iter()
            .enumerate()
            .all(|(index, byte)| {
                u8::from_str_radix(&reference[index * 2..index * 2 + 2], 16).ok() == Some(*byte)
            })
}
