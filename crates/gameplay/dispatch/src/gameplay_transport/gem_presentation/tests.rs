use super::*;
#[test]
fn gem_presentation_all_canonical_vocations_fit_and_use_executed_corrections()
-> Result<(), &'static str> {
    let data = WheelGemData::embedded().map_err(|_| "catalogue")?;
    for vocation in ["druid", "knight", "monk", "paladin", "sorcerer"] {
        let rows = project(&data, vocation).ok_or("project")?;
        assert_eq!(rows.len(), 69);
        let reply = wire::Reply {
            grades_current: true,
            qualified_gems: (1_u128..=225).map(u128::to_be_bytes).collect(),
            request: wire::Request {
                nonce: 1,
                revision: u64::MAX,
            },
            catalogue: format!("{}@{}", data.revision(), data.source_candidate_sha256()),
            vocation: vocation.into(),
            modifiers: rows.clone(),
        };
        let bytes = reply.encode().ok_or("all real entries must fit")?;
        assert!(bytes.len() <= wire::MAX_REPLY_BYTES);
        assert_eq!(wire::Reply::decode(&bytes), Some(reply));
        if vocation == "monk" {
            let capacity = rows
                .iter()
                .find(|r| r.kind == 0 && r.id == 48)
                .ok_or("capacity")?;
            assert_eq!(capacity.effects[0].values[3], 750000);
            let mana = rows
                .iter()
                .find(|r| r.kind == 0 && r.id == 33)
                .ok_or("mixed mana")?;
            assert!(
                mana.effects
                    .iter()
                    .find(|e| e.name == "Mana")
                    .ok_or("mana")?
                    .values
                    .iter()
                    .all(|v| *v == 0)
            );
        }
        let dodge = rows
            .iter()
            .find(|r| r.kind == 1 && r.id == 0)
            .ok_or("dodge")?;
        assert_eq!(dodge.effects[0].values, [280, 300, 330, 420]);
        if let Some(mastery) = rows.iter().find(|r| {
            r.kind == 1
                && r.effects
                    .iter()
                    .any(|e| e.name == "revelation_mastery_points")
        }) {
            assert!(
                mastery
                    .effects
                    .iter()
                    .filter(|e| e.name == "revelation_mastery_points")
                    .all(|e| e.values[0] >= 300000)
            );
        }
    }
    assert!(project(&data, "none").is_none());
    Ok(())
}
