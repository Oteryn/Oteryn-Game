//! Explicit startup selectors materialized into compiled sectors, preserving B3 source bytes.
use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::{Error, b3::Region};

/// The checked source selector family. An absent file is an empty synthetic family.
#[derive(Clone, Debug, Default)]
pub struct Source {
    bindings: Vec<Binding>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Attribute {
    Unique,
    Action,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    position: (u16, u16, u8),
    item: u16,
    attribute: Attribute,
    value: u16,
    source: String,
    line: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    path: String,
    sha256: String,
    git_blob_sha1: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    repository: String,
    revision: String,
    evidence_class: String,
    sources: Vec<Witness>,
    bindings: Vec<Binding>,
}

impl Source {
    /// Load the source-only projection; no source region is rewritten.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let path = root.join("content/world/quest-startup-bindings.json");
        if !path.is_file() {
            return Ok(Self::default());
        }
        Self::parse(&crate::source::read(&path)?)
    }

    /// Decode the closed, bounded startup authoring shape.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(Error::Format(
                "startup selector document exceeds its byte bound".into(),
            ));
        }
        let doc: Document = serde_json::from_slice(bytes)
            .map_err(|e| Error::Format(format!("startup selectors: {e}")))?;
        let digest = |s: &str, n| {
            s.len() == n
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        };
        let mut paths = BTreeSet::new();
        let mut keys = BTreeSet::new();
        if doc.schema != 1
            || !matches!(
                doc.repository.as_str(),
                "zimbadev/crystalserver" | "opentibiabr/canary"
            )
            || !digest(&doc.revision, 40)
            || doc.evidence_class != "OTS_HYPOTHESIS_ONLY"
            || !(1..=32).contains(&doc.sources.len())
            || !(1..=4096).contains(&doc.bindings.len())
            || doc.sources.iter().any(|s| {
                s.path.len() > 256
                    || s.path.contains(['\\', '\0'])
                    || s.path.contains("..")
                    || !(s.path.starts_with("data-global/startup/tables/")
                        || s.path.starts_with("data-otservbr-global/startup/tables/"))
                    || !s.path.ends_with(".lua")
                    || !paths.insert(s.path.as_str())
                    || !digest(&s.sha256, 64)
                    || !digest(&s.git_blob_sha1, 40)
            })
            || doc.bindings.iter().any(|b| {
                b.position.2 > 15
                    || b.item == 0
                    || b.value == 0
                    || b.line == 0
                    || !paths.contains(b.source.as_str())
                    || !keys.insert((b.position, b.item, b.attribute))
            })
        {
            return Err(Error::Format(
                "invalid startup selector source binding".into(),
            ));
        }
        Ok(Self {
            bindings: doc.bindings,
        })
    }

    pub(crate) fn apply(&self, region: &mut Region, palette: &[String]) -> Result<(), Error> {
        for binding in self.bindings.iter().filter(|b| {
            b.position.2 == region.z
                && b.position.0 / 256 == region.rx
                && b.position.1 / 256 == region.ry
        }) {
            let fail = |reason| {
                Error::Format(format!(
                    "startup selector {reason}: {}:{} item {} at {:?}",
                    binding.source, binding.line, binding.item, binding.position
                ))
            };
            let tile = region
                .sectors
                .iter_mut()
                .flat_map(|(_, _, tiles)| tiles)
                .find(|t| (t.x, t.y) == (binding.position.0, binding.position.1))
                .ok_or_else(|| fail("cell missing"))?;
            if tile.house != 0 {
                return Err(fail("belongs to a house"));
            }
            let expected = format!("oteryn:item.tibia.i{}", binding.item);
            let mut candidates = tile
                .items
                .iter_mut()
                .filter(|i| i.depth == 0 && palette.get(i.palette as usize) == Some(&expected));
            let item = candidates.next().ok_or_else(|| fail("object missing"))?;
            if candidates.next().is_some() {
                return Err(fail("object ambiguous"));
            }
            let value = match binding.attribute {
                Attribute::Unique => &mut item.attrs.unique,
                Attribute::Action => &mut item.attrs.action,
            };
            if value.is_some_and(|old| old != binding.value) {
                return Err(fail("conflicts with source attribute"));
            }
            *value = Some(binding.value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sector::{Attrs, Item, Tile};

    fn document() -> serde_json::Value {
        serde_json::json!({"schema":1,"repository":"zimbadev/crystalserver",
            "revision":"1".repeat(40),"evidence_class":"OTS_HYPOTHESIS_ONLY",
            "sources":[{"path":"data-global/startup/tables/teleport.lua",
                "sha256":"2".repeat(64),"git_blob_sha1":"3".repeat(40)}],
            "bindings":[{"position":[1,1,7],"item":1949,"attribute":"unique",
                "value":3059,"source":"data-global/startup/tables/teleport.lua","line":387}]})
    }

    fn region() -> Region {
        Region {
            z: 7,
            rx: 0,
            ry: 0,
            sectors: vec![(
                0,
                0,
                vec![Tile {
                    x: 1,
                    y: 1,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs {
                            count: Some(3),
                            action: Some(4000),
                            text: Some("keep existing source text".into()),
                            ..Attrs::default()
                        },
                    }],
                }],
            )],
        }
    }

    #[test]
    fn startup_selectors_are_idempotent_and_preserve_unrelated_attributes() -> Result<(), Error> {
        let source = Source::parse(document().to_string().as_bytes())?;
        let mut region = region();
        let palette = vec!["oteryn:item.tibia.i1949".into()];
        source.apply(&mut region, &palette)?;
        let first = crate::sector::encode(&region.sectors[0].2)?;
        source.apply(&mut region, &palette)?;
        assert_eq!(first, crate::sector::encode(&region.sectors[0].2)?);
        let attrs = &region.sectors[0].2[0].items[0].attrs;
        assert_eq!(attrs.unique, Some(3059));
        assert_eq!(attrs.action, Some(4000));
        assert_eq!(attrs.count, Some(3));
        assert_eq!(attrs.text.as_deref(), Some("keep existing source text"));
        Ok(())
    }

    #[test]
    fn startup_selectors_refuse_missing_ambiguous_conflicting_or_house_objects() -> Result<(), Error>
    {
        let source = Source::parse(document().to_string().as_bytes())?;
        let palette = vec!["oteryn:item.tibia.i1949".into()];
        for case in 0..5 {
            let mut region = region();
            let tile = &mut region.sectors[0].2[0];
            match case {
                0 => tile.items.clear(),
                1 => tile.items.push(tile.items[0].clone()),
                2 => tile.items[0].attrs.unique = Some(999),
                3 => tile.house = 1,
                _ => tile.items[0].depth = 1,
            }
            assert!(source.apply(&mut region, &palette).is_err(), "case {case}");
        }
        Ok(())
    }

    #[test]
    fn startup_selectors_reject_unbound_provenance_unknown_fields_and_invalid_bounds() {
        for case in 0..7 {
            let mut value = document();
            match case {
                0 => value["unknown"] = true.into(),
                1 => value["bindings"][0]["position"][2] = 16.into(),
                2 => value["bindings"][0]["value"] = 0.into(),
                3 => value["bindings"][0]["source"] = "missing.lua".into(),
                4 => value["sources"][0]["sha256"] = "wrong".into(),
                5 => value["sources"][0]["path"] = "../teleport.lua".into(),
                _ => {
                    value["bindings"] =
                        serde_json::json!([document()["bindings"][0], document()["bindings"][0]])
                }
            }
            assert!(
                Source::parse(value.to_string().as_bytes()).is_err(),
                "case {case}"
            );
        }
    }
}
