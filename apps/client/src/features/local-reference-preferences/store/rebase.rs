use super::*;

impl ReferencePreferences {
    /// Replay only edits made against the supplied baseline. This keeps untouched
    /// fields from an ordered restore or character load, including nested options.
    pub(crate) fn rebase_edits(&mut self, before: &Self, after: &Self) -> io::Result<()> {
        let miscellaneous = self.miscellaneous;
        let mut target = serde_json::to_value(&*self)?;
        overlay(
            false,
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

fn overlay(
    fixed_array: bool,
    target: &mut serde_json::Value,
    before: &serde_json::Value,
    after: serde_json::Value,
) {
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
                let fixed_array = matches!(
                    key.as_str(),
                    "effect_opacity" | "hud_names" | "hud_bounds" | "origin" | "size" | "visible" | "rows" | "bindings"
                );
                match (target.get_mut(&key), before.get(&key)) {
                    (Some(target), Some(before)) => overlay(fixed_array, target, before, value),
                    (None, Some(before)) if before == &value => {}
                    _ => {
                        target.insert(key, value);
                    }
                }
            }
        }
        (
            serde_json::Value::Array(target),
            serde_json::Value::Array(before),
            serde_json::Value::Array(after),
        ) if fixed_array && target.len() == before.len() && before.len() == after.len() => {
            for ((target, before), after) in target.iter_mut().zip(before).zip(after) {
                overlay(false, target, before, after);
            }
        }
        (target, _, after) => *target = after,
    }
}

#[cfg(test)]
#[test]
fn review_3376_effect_opacity_merges_per_index() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let mut first = PreferenceStore::load(path.clone())?;
    let mut second = PreferenceStore::load(path.clone())?;
    let mut own = first.preferences().clone();
    own.effect_opacity[0] = 21;
    first.apply(own)?;
    let mut others = second.preferences().clone();
    others.effect_opacity[1] = 38;
    second.apply(others)?;
    assert_eq!(
        PreferenceStore::read_transfer(&path)?.effect_opacity[..2],
        [21, 38]
    );
    Ok(())
}

#[cfg(test)]
#[test]
fn c2c_stale_bar_row_edits_merge() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let mut first = PreferenceStore::load(path.clone())?;
    let mut second = PreferenceStore::load(path.clone())?;
    let mut own = first.preferences().clone();
    own.bar_layout.rows[1].enabled = true;
    first.apply(own)?;
    let mut other = second.preferences().clone();
    other.bar_layout.rows[2].enabled = true;
    second.apply(other)?;
    let rows = PreferenceStore::read_transfer(&path)?.bar_layout.rows;
    assert!(rows[1].enabled && rows[2].enabled);
    Ok(())
}

#[cfg(test)]
#[test]
fn c2c_stale_item_bindings_merge() -> io::Result<()> {
    use std::num::NonZeroU32;
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let mut first = PreferenceStore::load(path.clone())?;
    let mut second = PreferenceStore::load(path.clone())?;
    let mut own = first.preferences().clone();
    own.item_hotkeys.bindings[0] = NonZeroU32::new(123);
    first.apply(own)?;
    let mut other = second.preferences().clone();
    other.item_hotkeys.bindings[1] = NonZeroU32::new(456);
    second.apply(other)?;
    let bindings = PreferenceStore::read_transfer(&path)?.item_hotkeys.bindings;
    assert_eq!(bindings[..2], [NonZeroU32::new(123), NonZeroU32::new(456)]);
    Ok(())
}
