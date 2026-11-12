//! Canonical Bosstiary identities plus the admitted generation's actual presentation policy.
use super::*;
use wire::catalogue::{Entry, PER_PAGE, Page, Query, SCAN_LIMIT};
pub(super) fn project(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    query: &Query,
    runtime: &crate::foundation::ChannelRuntimeV1,
) -> Option<Page> {
    let bosses = super::super::charm_progression::canonical_bosses()?;
    let native = owner.active_generation?.native_gameplay()?;
    if native.policies().source_digest() != runtime.content_pin().server_artifact_digest() {
        return None;
    }
    page(bosses, query, |boss| {
        let category = match boss.category.as_str() {
            "bane" => 1,
            "archfoe" => 2,
            "nemesis" => 3,
            _ => return None,
        };
        let thresholds = crate::domain::bosstiary::kill_thresholds(&boss.category)?;
        let mut points = [0; 3];
        for (i, goal) in thresholds.into_iter().enumerate() {
            points[i] = crate::domain::bosstiary::progress(&boss.category, goal)?.points;
        }
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|p| p.definition_key == boss.key);
        let name = policy.map(|p| p.display_name.clone()).or_else(|| {
            native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == boss.key)
                .and_then(|r| match &r.profile.data {
                    crate::content::ProjectV2AuthoringProfileData::Creature(c) => {
                        c.details.as_ref().map(|d| d.display_name.clone())
                    }
                    _ => None,
                })
        })?;
        Some(Entry {
            key: boss.key.clone(),
            name,
            category,
            kills: 0,
            thresholds,
            points,
            look: policy.map_or(0, |p| p.object_look_type.unwrap_or(p.outfit_look_type)),
            object: policy.is_some_and(|p| p.object_look_type.is_some()),
        })
    })
}
fn page(
    bosses: &[crate::domain::bosstiary::BosstiaryBoss],
    q: &Query,
    mut lookup: impl FnMut(&crate::domain::bosstiary::BosstiaryBoss) -> Option<Entry>,
) -> Option<Page> {
    let start = usize::try_from(q.after).ok()?;
    if start > bosses.len() {
        return None;
    }
    let mut cursor = start;
    let mut entries = Vec::new();
    let search = q.search.trim().to_lowercase();
    for boss in bosses.iter().skip(start).take(SCAN_LIMIT) {
        cursor += 1;
        if let Some(entry) = lookup(boss)
            && (q.category == 0 || entry.category == q.category)
            && entry.name.to_lowercase().contains(&search)
        {
            entries.push(entry);
            if entries.len() == PER_PAGE {
                break;
            }
        }
    }
    Some(Page {
        next: (cursor < bosses.len()).then_some(u32::try_from(cursor).ok()?),
        points: 0,
        entries,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boss_catalogue_filters_canonical_categories_and_advances_over_unqualified_records() {
        let bosses: Vec<_> = (0..70)
            .map(|i| crate::domain::bosstiary::BosstiaryBoss {
                key: format!("oteryn:creature.boss{i}"),
                category: "archfoe".into(),
            })
            .collect();
        let lookup = |b: &crate::domain::bosstiary::BosstiaryBoss| {
            (b.key.ends_with("64")).then(|| Entry {
                key: b.key.clone(),
                name: "Oberon".into(),
                category: 2,
                kills: 0,
                thresholds: [5, 20, 60],
                points: [10, 40, 100],
                look: 1250,
                object: false,
            })
        };
        let q = Query {
            after: 0,
            category: 2,
            search: "ober".into(),
        };
        let first = page(&bosses, &q, lookup);
        assert_eq!(first.as_ref().and_then(|p| p.next), Some(64));
        assert!(first.is_some_and(|p| p.entries.is_empty()));
        let q = Query { after: 64, ..q };
        let last = page(&bosses, &q, lookup);
        assert!(last.is_some_and(|p| p.next.is_none()
            && p.entries.len() == 1
            && p.entries[0].thresholds == [5, 20, 60]));
        let q = Query { category: 1, ..q };
        assert!(page(&bosses, &q, lookup).is_some_and(|p| p.entries.is_empty()));
    }
}
