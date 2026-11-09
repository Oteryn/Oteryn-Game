use super::*;
fn domain(d: WheelDomain) -> u8 {
    match d {
        WheelDomain::Green => 0,
        WheelDomain::Red => 1,
        WheelDomain::Purple => 2,
        WheelDomain::Blue => 3,
    }
}
pub(super) fn project(
    data: &WheelGemData,
    rules: &WheelRuleset,
    vocation: &str,
) -> Option<(Vec<wire::Node>, Vec<wire::Revelation>)> {
    let family =
        &data.wheel()["vocations"][oteryn_gameplay_progression::wheel::base_vocation(vocation)?];
    let slots = family["slots"].as_array()?;
    if slots.len() != 36 || rules.slots().len() != 36 {
        return None;
    }
    let mut nodes = Vec::with_capacity(36);
    for (index, (node, source)) in rules.slots().iter().zip(slots).enumerate() {
        if source["state_slot"].as_u64()? != index as u64 + 1 {
            return None;
        }
        let mut unlock_from = node
            .unlock_from
            .iter()
            .map(|i| i.checked_sub(1))
            .collect::<Option<Vec<_>>>()?;
        unlock_from.sort_unstable();
        nodes.push(wire::Node {
            domain: domain(node.domain),
            capacity: node.capacity,
            minimum_points: node.minimum_points,
            unlock_from,
            name: source["conviction"]["name"].as_str()?.into(),
            per_point: effects::node_lines(data, rules, vocation, index, 1)?,
            full_slot: effects::node_lines(data, rules, vocation, index, node.capacity)?,
        });
    }
    let mut revelations = Vec::with_capacity(4);
    for d in [
        WheelDomain::Green,
        WheelDomain::Red,
        WheelDomain::Purple,
        WheelDomain::Blue,
    ] {
        let key = &rules.vocation(vocation)?.revelations[usize::from(domain(d))];
        let source = family["revelations"]
            .as_array()?
            .iter()
            .find(|r| r["key"].as_str() == Some(key))?;
        let mut stages = Vec::new();
        for stage in source["stages"].as_array()? {
            let minimum_points = u32::try_from(stage["minimum_domain_points"].as_u64()?).ok()?;
            stages.push(wire::Stage {
                minimum_points,
                effects: effects::revelation_lines(data, rules, vocation, d, minimum_points, key)?,
            });
        }
        revelations.push(wire::Revelation {
            domain: domain(d),
            name: source["name"].as_str()?.into(),
            stages,
        });
    }
    Some((nodes, revelations))
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
