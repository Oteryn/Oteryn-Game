//! Exact owner-authored Asura callbacks; no donor implementation is claimed.
use super::Callback;

pub(super) fn admitted(callback: &Callback) -> bool {
    if callback.quest != "oteryn:quest.authored.asura_palace_quest"
        || !callback
            .key
            .starts_with("oteryn:quest-callback/engenc/asura/")
        || !callback
            .sources
            .iter()
            .any(|s| s == "authored per owner rule")
    {
        return false;
    }
    let Ok(document) = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
    )) else {
        return false;
    };
    document["quests"][0]["stages"]
        .as_array()
        .is_some_and(|stages| {
            stages.iter().any(|stage| {
                serde_json::from_value::<Callback>(stage["trigger"]["callback"].clone())
                    .is_ok_and(|bound| bound == *callback)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engenc_authored_admission_is_exact_and_cannot_rebind_native_doors() -> Result<(), String> {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))
        .map_err(|e| e.to_string())?;
        let mut count = 0;
        for stage in document["quests"][0]["stages"].as_array().ok_or("stages")? {
            let Ok(mut callback) =
                serde_json::from_value::<Callback>(stage["trigger"]["callback"].clone())
            else {
                continue;
            };
            if !callback
                .key
                .starts_with("oteryn:quest-callback/engenc/asura/")
            {
                continue;
            }
            assert!(admitted(&callback));
            count += 1;
            callback.selector.position.as_mut().ok_or("position")?.x += 1;
            assert!(!admitted(&callback));
        }
        assert_eq!(count, 6);
        Ok(())
    }
}
