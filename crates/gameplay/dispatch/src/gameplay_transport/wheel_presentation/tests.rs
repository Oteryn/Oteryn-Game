use super::*;
#[test]
fn wheel_presentation_all_canonical_vocations_preserve_exact_graph_and_executed_perks()
-> Result<(), &'static str> {
    let data = WheelGemData::embedded().map_err(|_| "data")?;
    let rules = data.wheel_ruleset().map_err(|_| "rules")?;
    for vocation in ["knight", "paladin", "sorcerer", "druid", "monk"] {
        let (nodes, revelations) = project(&data, &rules, vocation).ok_or("project")?;
        assert_eq!(nodes.len(), 36);
        assert_eq!(revelations.len(), 4);
        for (n, r) in nodes.iter().zip(rules.slots()) {
            assert_eq!(n.capacity, r.capacity);
            assert_eq!(n.minimum_points, r.minimum_points);
            let mut expected = r.unlock_from.iter().map(|v| v - 1).collect::<Vec<_>>();
            expected.sort_unstable();
            assert_eq!(n.unlock_from, expected);
        }
        let p = rules.capacities();
        let a = WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().into()),
            wheel_revision: 1,
            slots: WheelSlots::new(p).map_err(|_| "points")?,
            current: true,
        };
        let current = effects::configured(
            &data,
            &rules,
            &a,
            effects::Facts {
                vocation,
                level: u32::MAX,
                promoted: true,
                premium: true,
            },
            None,
        )
        .ok_or("effects")?;
        let reply = wire::Reply {
            request: wire::Request {
                nonce: 1,
                wheel_revision: 1,
                atelier_revision: 0,
                points: p,
                vessels: [Some([1; 16]), Some([2; 16]), Some([3; 16]), Some([4; 16])],
            },
            catalogue: format!("{}@{}", data.revision(), data.source_candidate_sha256()),
            vocation: vocation.into(),
            available: 10000,
            unlock_points: 10000,
            removal_permitted: true,
            eligible: true,
            draft_valid: true,
            reason: String::new(),
            nodes,
            revelations,
            current: current.clone(),
            draft: current,
        };
        let bytes = reply.encode().ok_or("all real nodes/perks must fit")?;
        assert!(bytes.len() <= wire::MAX_REPLY_BYTES);
        assert_eq!(wire::Reply::decode(&bytes), Some(reply));
    }
    Ok(())
}
#[test]
fn wheel_presentation_single_point_and_full_slot_use_existing_executable_projection()
-> Result<(), &'static str> {
    let data = WheelGemData::embedded().map_err(|_| "data")?;
    let rules = data.wheel_ruleset().map_err(|_| "rules")?;
    let nodes = project(&data, &rules, "knight").ok_or("nodes")?.0;
    assert!(nodes[13].per_point.iter().any(|s| s == "Maximum mana: +1"));
    assert_eq!(nodes[13].capacity, 75);
    assert!(nodes[13].full_slot.iter().any(|s| s == "Maximum mana: +75"));
    assert!(nodes[13].full_slot.iter().any(|s| s == "weapon skill: +1"));
    Ok(())
}
