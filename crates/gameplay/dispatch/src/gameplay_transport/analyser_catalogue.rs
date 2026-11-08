//! Read-only pinned item identity enrichment for the local hunting analysers.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::item_ref::ItemDefinitionIndex;
use oteryn_protocol_oteryn::analyser_catalogue::{ItemInfo, encode_reply};

fn project(
    index: &ItemDefinitionIndex,
    ids: &[u32],
    name: impl Fn(&str, &str) -> String,
) -> Option<Vec<u8>> {
    let mut rows = Vec::with_capacity(ids.len());
    for id in ids {
        let reference = std::num::NonZeroU32::new(*id)?;
        let (key, revision) = index.definition_at(reference)?;
        let coin_worth =
            crate::domain::currency::Coin::from_production_key(key).map_or(0, |coin| coin.worth());
        rows.push(ItemInfo {
            item_type: *id,
            name: name(key, revision),
            coin_worth,
        });
    }
    encode_reply(&rows).ok()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn analyser_catalogue_page(
        &self,
        admitted: AdmittedSession,
        ids: &[u32],
    ) -> Option<Vec<u8>> {
        let runtime = self.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
            return None;
        }
        let native = self.active_generation?.native_gameplay()?;
        let index = super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        project(index, ids, |key, revision| {
            use oteryn_gameplay_content::content::ReferenceItemField::Known;
            native
                .item_policy(key, revision)
                .and_then(|policy| {
                    if let Known(presentation) = &policy.record().semantics.presentation
                        && let Known(name) = &presentation.name
                    {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        })
    }
}
macro_rules! authority_methods {
    () => {
        fn analyser_catalogue_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _ids: &[u32],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn analyser_catalogue_query(
            &self,
            admitted: connection::AdmittedSession,
            ids: &[u32],
        ) -> Option<Vec<u8>> {
            self.analyser_catalogue_page(admitted, ids).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analyser_catalogue_resolves_generation_refs_and_distinguishes_currency()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i3031","definition-r1"],["oteryn:item.tibia.i5000","definition-r1"]]}"#)?;
        let payload = project(&index, &[1, 2], |_, _| String::new()).ok_or("projection")?;
        let rows = oteryn_protocol_oteryn::analyser_catalogue::decode_reply(&payload)?;
        assert_eq!((rows[0].item_type, rows[0].coin_worth), (1, 1));
        assert_eq!((rows[1].item_type, rows[1].coin_worth), (2, 0));
        assert!(project(&index, &[3], |_, _| String::new()).is_none());
        assert!(project(&index, &[1, 1], |_, _| String::new()).is_none());
        Ok(())
    }
}
