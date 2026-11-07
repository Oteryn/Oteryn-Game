//! Bounded, stateless source object effects; movable value stays with durable owners.
use super::*;

pub const MAX_MUTATIONS: usize = 16;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectMutation {
    pub position: Position,
    pub remove: Option<u16>,
    pub create: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectEffects {
    pub consume_trigger: bool,
    pub mutations: Vec<ObjectMutation>,
}

pub(super) fn valid(callback: &Callback) -> bool {
    let Some(objects) = &callback.objects else {
        return true;
    };
    let count = objects.mutations.len() + usize::from(objects.consume_trigger);
    let positions: BTreeSet<_> = objects.mutations.iter().map(|m| m.position).collect();
    count > 0
        && count <= MAX_MUTATIONS
        && positions.len() == objects.mutations.len()
        && (!objects.consume_trigger || callback.trigger == Trigger::Use)
        && callback.lever.is_none()
        && callback.passages.is_empty()
        && callback.teleport.is_none()
        && callback.storage.is_empty()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.minimum_level.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.native_rule.is_none()
        && objects.mutations.iter().all(|m| {
            (-15..=0).contains(&m.position.floor)
                && (m.remove.is_some() || m.create.is_some())
                && m.remove != Some(0)
                && m.create != Some(0)
                && (m.remove.is_none() || m.remove != m.create)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_effects_are_closed_bounded_and_stateless() -> Result<(), String> {
        let source: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        let mut callback = source["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/behemoth")
            .ok_or("behemoth")?
            .clone();
        callback["lever"] = serde_json::Value::Null;
        callback["passages"] = serde_json::json!([]);
        callback["objects"] = serde_json::json!({"consume_trigger":true,"mutations":[]});
        let document = |c| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(super::super::parse(&document(callback.clone())).is_ok());
        callback["objects"]["mutations"] = serde_json::json!([
            {"position":{"x":100,"y":200,"floor":-7},"remove":1791},
            {"position":{"x":101,"y":200,"floor":-7},"create":1270}
        ]);
        assert!(super::super::parse(&document(callback.clone())).is_ok());
        for case in 0..4 {
            let mut invalid = callback.clone();
            match case {
                0 => invalid["objects"] = serde_json::json!({"consume_trigger":false,"mutations":[]}),
                1 => invalid["objects"]["mutations"] = serde_json::json!((0..16)
                    .map(|x| serde_json::json!({"position":{"x":x,"y":200,"floor":-7},"create":1270}))
                    .collect::<Vec<_>>()),
                2 => invalid["storage"] = serde_json::json!([{"track":"oteryn:quest-track/test","minimum":-1,"maximum":1,"set":1}]),
                _ => invalid["objects"]["mutations"] = serde_json::json!([
                    {"position":{"x":100,"y":200,"floor":-7},"create":0}]),
            }
            assert!(
                super::super::parse(&document(invalid)).is_err(),
                "object restriction {case}"
            );
        }
        Ok(())
    }
}
