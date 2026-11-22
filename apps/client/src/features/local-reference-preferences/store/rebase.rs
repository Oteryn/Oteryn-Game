use super::*;

impl ReferencePreferences {
    /// Replay only edits made against the supplied baseline. This keeps untouched
    /// fields from an ordered restore or character load, including nested options.
    pub(crate) fn rebase_edits(&mut self, before: &Self, after: &Self) -> io::Result<()> {
        let miscellaneous = self.miscellaneous;
        let mut target = serde_json::to_value(&*self)?;
        overlay(
            &mut target,
            &serde_json::to_value(before)?,
            serde_json::to_value(after)?,
        );
        let mut merged: Self = serde_json::from_value(target)?;
        merged.miscellaneous = miscellaneous;
        merged.validate().map_err(io::Error::other)?;
        merged.reconcile_controls(Some(self));
        *self = merged;
        Ok(())
    }
}

fn overlay(target: &mut serde_json::Value, before: &serde_json::Value, after: serde_json::Value) {
    if *before == after {
        return;
    }
    match (target, before, after) {
        (
            serde_json::Value::Object(target),
            serde_json::Value::Object(before),
            serde_json::Value::Object(after),
        ) => {
            for key in before.keys().filter(|key| !after.contains_key(*key)) {
                target.remove(key);
            }
            for (key, value) in after {
                match (target.get_mut(&key), before.get(&key)) {
                    (Some(target), Some(before)) => overlay(target, before, value),
                    (None, Some(before)) if before == &value => {}
                    _ => {
                        target.insert(key, value);
                    }
                }
            }
        }
        (target, _, after) => *target = after,
    }
}
