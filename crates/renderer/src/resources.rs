use crate::RendererError;
use oteryn_foundation::ProcessGeneration;
use std::collections::BTreeMap;

/// Generic generation-fenced renderer resource ownership extracted from the source
/// `renderer-resource` package. Synthetic fixture adapters remain outside the production crate.
#[derive(Debug, Clone)]
pub struct ResourceCache<K, V> {
    generation: ProcessGeneration,
    entries: BTreeMap<K, V>,
}

impl<K, V> ResourceCache<K, V>
where
    K: Ord,
{
    #[must_use]
    pub const fn new(generation: ProcessGeneration) -> Self {
        Self {
            generation,
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        generation: ProcessGeneration,
        key: K,
        value: V,
    ) -> Result<Option<V>, RendererError> {
        if generation != self.generation {
            return Err(RendererError::StaleGeneration {
                expected: self.generation,
                received: generation,
            });
        }
        Ok(self.entries.insert(key, value))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generation_cannot_replace_resource() -> Result<(), RendererError> {
        let current = ProcessGeneration::new(2);
        let mut cache = ResourceCache::new(current);
        cache.insert(current, "texture", 1_u8)?;
        assert!(matches!(
            cache.insert(ProcessGeneration::new(1), "texture", 2_u8),
            Err(RendererError::StaleGeneration { .. })
        ));
        assert_eq!(cache.len(), 1);
        Ok(())
    }
}
