//! Shared stations remain unambiguous only with disjoint gates in the same quest.
use super::Callback;

pub(super) fn disjoint(left: &Callback, right: &Callback) -> bool {
    left.quest == right.quest
        && left.storage.iter().any(|a| {
            right
                .storage
                .iter()
                .any(|b| a.track == b.track && (a.maximum < b.minimum || b.maximum < a.minimum))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_station_requires_proven_disjoint_same_quest_gates() -> Result<(), String> {
        let rows = super::super::parse_with_provenance(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/quests/authored-world-callbacks.json"
            )),
            true,
        )?;
        let left = rows
            .iter()
            .find(|c| c.key.ends_with("winterlight-gather"))
            .ok_or("gather")?
            .clone();
        let mut right = rows
            .iter()
            .find(|c| c.key.ends_with("winterlight-disguise"))
            .ok_or("disguise")?
            .clone();
        assert_eq!(left.selector, right.selector);
        assert!(disjoint(&left, &right));
        super::super::validate_registry(&[left.clone(), right.clone()])?;
        right.storage[0].minimum = left.storage[0].minimum;
        assert!(!disjoint(&left, &right));
        assert!(super::super::validate_registry(&[left.clone(), right.clone()]).is_err());
        right.storage[0].minimum = 1;
        right.quest = "oteryn:quest.other".into();
        assert!(!disjoint(&left, &right));
        assert!(super::super::validate_registry(&[left, right]).is_err());
        Ok(())
    }
}
