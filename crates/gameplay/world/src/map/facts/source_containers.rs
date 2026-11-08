//! Immutable nested map sources. Virtual ordinals follow all top-level ordinals,
//! preserving every existing top-level placement identity.
use super::*;
use oteryn_world_bundle::sector::Tile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedMapItem {
    pub placement_key: u64,
    pub root: u64,
    pub parent: u64,
    pub facts: EntryFacts,
    pub attrs: Attrs,
    pub children: Vec<u64>,
}

impl BundleFacts {
    pub(super) fn capture_nested_sources(&mut self, pos: TilePos, tile: &Tile) {
        let top_count = tile.items.iter().filter(|item| item.depth == 0).count();
        if tile.items.len() > usize::from(MAX_BASE_ORDINAL) + 1 {
            for ordinal in 0..top_count {
                if let Some(key) = u8::try_from(ordinal)
                    .ok()
                    .and_then(|n| placement_key(pos, n))
                {
                    self.unsupported_source_containers.insert(key);
                }
            }
            return;
        }
        let mut ordinal = 0_usize;
        let mut nested = top_count;
        let mut parents: Vec<u64> = Vec::new();
        let mut root = 0;
        for item in &tile.items {
            let number = if item.depth == 0 {
                let n = ordinal;
                ordinal += 1;
                n
            } else {
                let n = nested;
                nested += 1;
                n
            };
            let Some(key) = u8::try_from(number)
                .ok()
                .and_then(|n| placement_key(pos, n))
            else {
                return;
            };
            if item.depth == 0 {
                root = key;
                parents.clear();
                parents.push(key);
                continue;
            }
            let depth = usize::from(item.depth);
            if depth > parents.len() {
                self.unsupported_source_containers.insert(root);
                continue;
            }
            parents.truncate(depth);
            let parent = parents[depth - 1];
            let Some(palette) = self.palette.get(item.palette as usize) else {
                self.unsupported_source_containers.insert(root);
                continue;
            };
            if !matches!(palette.facts.definition, MapDefinition::Item(_)) {
                self.unsupported_source_containers.insert(root);
                continue;
            }
            let mut facts = palette.facts;
            facts.count = u32::from(item.attrs.count.unwrap_or(1));
            facts.sub_type = u32::from(item.attrs.charges.unwrap_or(0));
            facts.bound = bound(&item.attrs);
            self.map_source_children
                .entry(parent)
                .or_default()
                .push(key);
            self.map_source_items.insert(
                key,
                CapturedMapItem {
                    placement_key: key,
                    root,
                    parent,
                    facts,
                    attrs: item.attrs.clone(),
                    children: Vec::new(),
                },
            );
            parents.push(key);
        }
    }

    /// Original nested facts, bound to this immutable bundle. Never current custody.
    pub fn captured_map_item(&self, key: u64) -> Option<CapturedMapItem> {
        let mut node = self.map_source_items.get(&key)?.clone();
        if self.unsupported_source_containers.contains(&node.root) {
            return None;
        }
        node.children = self
            .map_source_children
            .get(&key)
            .cloned()
            .unwrap_or_default();
        Some(node)
    }

    /// `None` refuses a source graph outside the existing placement-address budget.
    pub fn captured_map_children(&self, key: u64) -> Option<Vec<CapturedMapItem>> {
        if self.unsupported_source_containers.contains(&key) {
            return None;
        }
        self.map_source_children
            .get(&key)
            .into_iter()
            .flatten()
            .map(|child| self.captured_map_item(*child))
            .collect()
    }
}
