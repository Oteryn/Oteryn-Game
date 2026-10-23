use super::*;

impl AppearanceState {
    pub(super) fn validate_looks(&self, item_view: bool) -> Result<(), SessionError> {
        let mut refs = std::collections::BTreeSet::new();
        for binding in self.bindings.values() {
            if !refs.insert(binding.appearance_ref)
                || binding.appearance_ref > self.high_water
                || binding.context != self.catalogue.context
            {
                return Err(invalid());
            }
            match binding.look {
                wire::Look::Outfit { colours, .. }
                    if colours
                        .iter()
                        .any(|c| usize::from(*c) >= self.catalogue.palette.len()) =>
                {
                    return Err(invalid());
                }
                wire::Look::Item { definition_ref } => {
                    self.validate_item(definition_ref, item_view)?
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub(super) fn apply(
        &mut self,
        base: u64,
        new: u64,
        kind: u32,
        payload: &[u8],
        items: bool,
    ) -> Result<AppearanceUpdate, SessionError> {
        if base != self.revision || base.checked_add(1) != Some(new) {
            return Err(SessionError::Protocol(
                FoundationProtocolError::StateRevisionMismatch,
            ));
        }
        let mut next = self.clone();
        let update = match kind {
            wire::DELTA_TYPE_APPEARANCE_MAPPINGS => {
                let mappings = wire::Mappings::decode(payload).map_err(codec)?;
                if mappings.context != self.catalogue.context {
                    return Err(invalid());
                }
                for binding in &mappings.upserts {
                    if let Some(prior) = next.bindings.get(&binding.entity) {
                        if binding.appearance_ref != prior.appearance_ref
                            || binding.root_revision < prior.root_revision
                            || binding.condition_revision < prior.condition_revision
                            || binding.owner_time_us < prior.owner_time_us
                            || binding == prior
                        {
                            return Err(invalid());
                        }
                    } else {
                        next.high_water = next.high_water.checked_add(1).ok_or_else(invalid)?;
                        if binding.appearance_ref != next.high_water {
                            return Err(invalid());
                        }
                    }
                    next.bindings.insert(binding.entity, binding.clone());
                }
                for entity in &mappings.forgets {
                    if next.bindings.remove(entity).is_none() {
                        return Err(invalid());
                    }
                }
                AppearanceUpdate::Mappings(mappings)
            }
            wire::DELTA_TYPE_APPEARANCE_CATALOGUE => {
                let catalogue = wire::Catalogue::decode(payload).map_err(codec)?;
                if catalogue.context != self.catalogue.context
                    || catalogue.root_revision < self.catalogue.root_revision
                    || catalogue.palette != self.catalogue.palette
                    || catalogue == self.catalogue
                {
                    return Err(invalid());
                }
                next.catalogue = catalogue.clone();
                AppearanceUpdate::Catalogue(catalogue)
            }
            _ => return Err(invalid()),
        };
        if next.bindings.len() > 512 {
            return Err(SessionError::Protocol(
                FoundationProtocolError::PayloadLimitExceeded,
            ));
        }
        next.validate_looks(items)?;
        next.revision = new;
        *self = next;
        Ok(update)
    }
}
