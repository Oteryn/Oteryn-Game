//! Exact mutable Ground prerequisites; sequence order is top to bottom.
use super::*;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundStackItem {
    pub item: String,
    pub quantity: u32,
}

pub(super) fn valid(callback: &Callback) -> bool {
    let ground: Vec<_> = callback
        .world_requirements
        .iter()
        .filter(|requirement| requirement.ground_stack.is_some())
        .collect();
    if ground.is_empty() {
        return true;
    }
    ground.len() <= MAX_EFFECTS
        && callback.actor_kind == ActorKind::Player
        && callback.native_rule.is_none()
        && callback.requires_world_admission
        && !callback.reversible
        && !callback.opening_only
        && callback.trigger != Trigger::StepOut
        && callback.storage.iter().any(|state| !state.preserve)
        && ground
            .iter()
            .map(|r| r.position)
            .collect::<BTreeSet<_>>()
            .len()
            == ground.len()
        && ground.iter().all(|requirement| {
            (-15..=0).contains(&requirement.position.floor)
                && requirement.appearance == 0
                && requirement.ground_stack.as_ref().is_some_and(|items| {
                    items.len() <= 64
                        && items.iter().all(|entry| {
                            entry.quantity > 0
                                && entry.item.starts_with("oteryn:item.")
                                && super::super::valid_quest_key(&entry.item)
                        })
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_ground_requirements_are_bounded_and_bind_the_transition() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| {
                c.storage.iter().any(|s| !s.preserve)
                    && c.native_rule.is_none()
                    && c.actor_kind == ActorKind::Player
                    && !c.reversible
                    && !c.opening_only
                    && c.trigger != Trigger::StepOut
            })
            .ok_or("fixture")?;
        callback.requires_world_admission = true;
        callback.world_requirements = vec![WorldRequirement {
            position: Position {
                x: 100,
                y: 100,
                floor: -7,
            },
            appearance: 0,
            ground_stack: Some(vec![GroundStackItem {
                item: "oteryn:item.tibia.i1735".into(),
                quantity: 1,
            }]),
        }];
        assert!(valid(&callback));
        let binding = gated_binding(&callback)?.ok_or("ground binding")?;
        callback.world_requirements[0]
            .ground_stack
            .as_mut()
            .ok_or("stack")?[0]
            .quantity = 2;
        assert_ne!(gated_binding(&callback)?, Some(binding));
        callback.world_requirements[0]
            .ground_stack
            .as_mut()
            .ok_or("stack")?[0]
            .quantity = 0;
        assert!(!valid(&callback));
        callback.world_requirements[0].ground_stack = Some(vec![
            GroundStackItem {
                item: "oteryn:item.tibia.i1735".into(),
                quantity: 1
            };
            65
        ]);
        assert!(!valid(&callback));
        callback.world_requirements[0].ground_stack = Some(vec![]);
        assert!(valid(&callback));
        callback.storage.clear();
        assert!(!valid(&callback));
        Ok(())
    }
}
