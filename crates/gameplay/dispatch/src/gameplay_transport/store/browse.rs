//! Full-catalogue search/sort and server-resolved artwork; quotes stay with their owner.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use std::collections::{BTreeMap, BTreeSet};
use wire::browse::{Artwork, Category, Page, Product, Query};
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    q: &Query,
) -> Option<Page> {
    super::gameplay_fence(owner, admitted).await?;
    {
        let runtime = owner.runtime.lock().await;
        runtime
            .player_control_facts(admitted.runtime_actor?, admitted.game_session_id)
            .ok()?;
    }
    let native = owner.active_generation?.native_gameplay()?;
    let mut catalogues = vec![CATALOGUE.as_ref()?];
    if admitted.continuity.selected_capabilities.contains(250) {
        catalogues.push(HIRELING_CATALOGUE.as_ref()?);
    }
    let mut products = Vec::new();
    for catalogue in catalogues {
        for offer in catalogue.entries() {
            products.push(Product {
                category: offer.category.clone(),
                revision: catalogue.rules.revision().into(),
                offer: wire::StoreOffer {
                    offer_key: offer.key.clone(),
                    title: catalogue.title(&offer.key).into(),
                    price: offer.price_coins,
                    currency: currency_text(offer.currency).into(),
                },
                artwork: artwork(native, catalogue, offer)?,
            });
        }
    }
    let ranks = owner
        .root
        .store_popularity(products.iter().map(|p| p.offer.offer_key.clone()).collect())
        .await
        .ok()?;
    let mut page = project(products, q, &ranks)?;
    // Reuse the accepted Platform balance projection; never synthesize a commercial balance.
    let account = uuid_text(&admitted.controller?.account_id);
    let mut client = owner.store.as_ref()?.lock().await;
    let balances = client.balance(&account).await.ok()?;
    let [balance] = balances.balances.as_slice() else {
        return None;
    };
    if balance.currency != "COINS" {
        return None;
    }
    page.total = balance.available.parse().ok()?;
    page.transferable = 0;
    page.transferable_known = false;
    drop(client);
    super::gameplay_fence(owner, admitted).await?;
    {
        let runtime = owner.runtime.lock().await;
        runtime
            .player_control_facts(admitted.runtime_actor?, admitted.game_session_id)
            .ok()?;
    }
    Some(page)
}
fn artwork(
    native: &NativeGameplayState,
    catalogue: &RuntimeCatalogue,
    offer: &rules::AdmittedOffer,
) -> Option<Artwork> {
    if let Some((kind, key)) = catalogue.preview(&offer.key) {
        let looks = match kind {
            1 => {
                let p = native
                    .policies()
                    .policies()
                    .iter()
                    .find(|p| p.definition_key == *key)?;
                [p.object_look_type?; 2]
            }
            2 => {
                use crate::domain::appearance::Sex;
                let c = native.cosmetics()?;
                [c.look_type(key, Sex::Male)?, c.look_type(key, Sex::Female)?]
            }
            3 => [native.cosmetics()?.mount_metadata(key)?.0; 2],
            _ => return None,
        };
        return Some(Artwork { kind: *kind, looks });
    }
    if let rules::DeliveryKind::Item { item_key, .. } = &offer.delivery {
        // The admitted production binding supplies the public object identity; never
        // reinterpret the sorted wire definition reference as an appearance number.
        let profile = native.item_profiles().find(|p| {
            p.authoring.item.key == *item_key
                || p.production_definition.production_key == *item_key
        })?;
        let look = profile.production_binding.external_id.parse::<u32>().ok()?;
        if look == 0 { return None; }
        return Some(Artwork { kind: 1, looks: [look; 2] });
    }

    if let Some(h) = catalogue.hireling(&offer.key) {
        let looks = if h.effect == "hire" {
            Some([1108, 1107])
        } else {
            [
                (2001, 1110, 1109),
                (2002, 1114, 1113),
                (2003, 1116, 1115),
                (2004, 1112, 1111),
                (2005, 1118, 1117),
                (2006, 1130, 1129),
                (2007, 1132, 1131),
                (2008, 1124, 1123),
                (2009, 1126, 1125),
            ]
            .iter()
            .find(|r| r.0 == h.source_id)
            .map(|r| [r.1, r.2])
        };
        if let Some(looks) = looks {
            return Some(Artwork { kind: 2, looks });
        }
    }
    Some(Artwork::default())
}
fn title(key: &str) -> String {
    key.split('-')
        .map(|word| {
            let mut c = word.chars();
            c.next().map_or(String::new(), |first| {
                first.to_uppercase().collect::<String>() + c.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub(super) fn project(
    mut products: Vec<Product>,
    q: &Query,
    ranks: &BTreeMap<String, u64>,
) -> Option<Page> {
    q.encode().ok()?;
    let mut categories = vec![Category {
        key: "featured".into(),
        parent: String::new(),
        title: "Home".into(),
    }];
    let mut keys = BTreeSet::from([
        "premium-time".to_owned(),
        "consumables".into(),
        "cosmetics".into(),
        "boosts".into(),
        "extras".into(),
    ]);
    for p in &products {
        let parts: Vec<_> = p.category.split('.').collect();
        for end in 1..=parts.len() {
            keys.insert(parts[..end].join("."));
        }
    }
    keys.remove("featured");
    for key in keys {
        let parent = key.rsplit_once('.').map_or("", |r| r.0).to_owned();
        let label = title(key.rsplit('.').next()?);
        categories.push(Category {
            key,
            parent,
            title: label,
        });
    }
    if !categories.iter().any(|c| c.key == q.category) {
        return None;
    }
    let search = q.search.trim().to_lowercase();
    products.retain(|p| {
        (q.category == "featured"
            || p.category == q.category
            || p.category.starts_with(&(q.category.clone() + ".")))
            && (p.offer.title.to_lowercase().contains(&search)
                || p.offer.offer_key.to_lowercase().contains(&search))
    });
    products.sort_by(|a, c| {
        let order = match q.sort {
            0 => ranks
                .get(&c.offer.offer_key)
                .copied()
                .unwrap_or(0)
                .cmp(&ranks.get(&a.offer.offer_key).copied().unwrap_or(0)),
            1 => a
                .offer
                .title
                .to_lowercase()
                .cmp(&c.offer.title.to_lowercase()),
            2 => c
                .offer
                .title
                .to_lowercase()
                .cmp(&a.offer.title.to_lowercase()),
            3 => a.offer.price.cmp(&c.offer.price),
            4 => c.offer.price.cmp(&a.offer.price),
            _ => std::cmp::Ordering::Equal,
        };
        order.then_with(|| a.offer.offer_key.cmp(&c.offer.offer_key))
    });
    let mut digest = Sha256::new();
    digest.update(b"store-browse-v1\0");
    let mut bound = q.clone();
    bound.cursor.clear();
    digest.update(bound.encode().ok()?);
    for p in &products {
        digest.update((p.offer.offer_key.len() as u32).to_le_bytes());
        digest.update(p.offer.offer_key.as_bytes());
        digest.update((p.revision.len() as u32).to_le_bytes());
        digest.update(p.revision.as_bytes());
        digest.update(p.offer.price.to_le_bytes());
        digest.update((p.offer.title.len() as u32).to_le_bytes());
        digest.update(p.offer.title.as_bytes());
        digest.update([p.artwork.kind]);
        for look in p.artwork.looks {
            digest.update(look.to_le_bytes());
        }
        digest.update(
            ranks
                .get(&p.offer.offer_key)
                .copied()
                .unwrap_or(0)
                .to_le_bytes(),
        );
    }
    use std::fmt::Write;
    let mut signature = String::new();
    for byte in digest.finalize() {
        write!(&mut signature, "{byte:02x}").ok()?;
    }
    let offset = if q.cursor.is_empty() {
        0
    } else {
        let (hash, offset) = q.cursor.split_once(':')?;
        if hash != signature {
            return None;
        }
        offset.parse::<usize>().ok()?
    };
    if offset > products.len() || offset % 32 != 0 {
        return None;
    }
    let next = if products.len() - offset > wire::MAX_STORE_PAGE_ENTRIES {
        format!("{signature}:{}", offset + 32)
    } else {
        String::new()
    };
    let page = Page {
        categories,
        products: products.into_iter().skip(offset).take(32).collect(),
        next,
        total: 0,
        transferable: 0,
        transferable_known: false,
    };
    page.encode().ok()?;
    Some(page)
}
#[cfg(test)]
#[path = "browse/tests.rs"]
mod tests;
