use super::*;
use crate::map::boot::tests as fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn m51_pillar_portal_view_and_collision_reset_together() -> TestResult {
    let (bytes, pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1841", "oteryn:item.tibia.i5022");
    let bundle = oteryn_world_bundle::bundle::read(&bytes)?;
    let mut facts = BundleFacts::palette(&bundle.manifest, |key| {
        let mut item = fixture::items(if key.ends_with("i1841") {
            "item:box"
        } else {
            "item:coin"
        })?;
        item.pickupable = false;
        Some(item)
    })
    .map_err(|e| format!("{e:?}"))?;
    for sector in &bundle.sectors {
        facts.push(&bundle.manifest, sector, &mut BTreeSet::new());
    }
    facts.finish();
    let base = crate::map::load(&bytes, &pins)?;
    let pos = TilePos {
        x: 6,
        y: 0,
        floor: -7,
    };
    let key = placement_key(pos, 1).ok_or("pillar key")?;
    assert!(facts.quest_callback_blocked(&base, pos, true));
    assert!(facts.set_svargrond_pillar(pos, 1, 0, [1; 16], ArenaPillarProjection::Victory));
    let portal = facts.base_entry(pos, 1, 0).ok_or("visible portal")?;
    assert_eq!(portal.appearance_id, 5022);
    assert!(facts.svargrond_portal_open(pos, [1; 16]));
    assert!(!facts.svargrond_portal_open(pos, [2; 16]));
    assert!(portal.bound);
    assert!(!portal.pickupable);
    assert!(!facts.quest_callback_blocked(&base, pos, true));
    let revision = facts.object_revision(key);
    assert!(facts.set_svargrond_pillar(pos, 1, 0, [1; 16], ArenaPillarProjection::Victory));
    assert_eq!(facts.object_revision(key), revision);
    assert!(facts.set_svargrond_pillar(pos, 1, 0, [1; 16], ArenaPillarProjection::Release));
    assert_eq!(
        facts
            .base_entry(pos, 1, 0)
            .ok_or("reset pillar")?
            .appearance_id,
        1841
    );
    assert!(facts.quest_callback_blocked(&base, pos, true));
    assert!(!facts.svargrond_portal_open(pos, [1; 16]));
    assert!(!facts.set_svargrond_pillar(pos, 0, 0, [1; 16], ArenaPillarProjection::Victory));
    assert!(!facts.set_svargrond_pillar(
        TilePos { x: 3, ..pos },
        1,
        0,
        [1; 16],
        ArenaPillarProjection::Victory
    ));
    assert!(facts.set_svargrond_pillar(pos, 1, 0, [1; 16], ArenaPillarProjection::Victory));
    assert!(facts.set_svargrond_pillar(pos, 1, 0, [2; 16], ArenaPillarProjection::Victory));
    assert!(!facts.set_svargrond_pillar(pos, 1, 0, [1; 16], ArenaPillarProjection::Release));
    assert_eq!(
        facts
            .base_entry(pos, 1, 0)
            .ok_or("new run portal")?
            .appearance_id,
        5022
    );
    assert!(facts.set_svargrond_pillar(pos, 1, 0, [2; 16], ArenaPillarProjection::Release));
    {
        let mut state = facts.callbacks.lock().map_err(|_| "callback lock")?;
        Arc::make_mut(&mut state).hidden.insert(key);
    }
    assert!(!facts.set_svargrond_pillar(pos, 1, 0, [1; 16], ArenaPillarProjection::Victory));
    Ok(())
}
