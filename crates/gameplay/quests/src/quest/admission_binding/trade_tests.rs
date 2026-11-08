use super::*;

#[test]
fn registered_trade_continuations_reject_renaming_and_stripped_bindings() -> Result<(), String> {
    let empty =
        super::super::QuestStateCatalogue::new("trade-registry-proof", Vec::new(), Vec::new())
            .map_err(|error| format!("{error:?}"))?;
    let catalogue = super::super::trade_callbacks::extend(empty)?;
    assert!(!catalogue.transitions.is_empty());
    for operation in catalogue.transitions.values() {
        validate(operation, false)?;
        let mut renamed = operation.clone();
        renamed.key.push_str("-unregistered");
        assert!(validate(&renamed, false).is_err());
        let mut stripped = operation.clone();
        let mut removed = 0;
        for effect in &mut stripped.effects {
            if let QuestEffectKind::SetWithBinding { value, .. } = effect.effect {
                effect.effect = QuestEffectKind::Set(value);
                removed += 1;
            }
        }
        assert_eq!(removed, 1);
        assert!(validate(&stripped, false).is_err());
    }
    Ok(())
}
