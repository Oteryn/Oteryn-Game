//! Native death goals reuse their existing canonical lowering at the admission boundary.
use super::super::QuestStateCatalogue;

pub(super) fn catalogue() -> Result<QuestStateCatalogue, String> {
    let empty = QuestStateCatalogue::new("native-goal-owners", Vec::new(), Vec::new())
        .map_err(|error| format!("{error:?}"))?;
    super::super::native_kill_goals::extend(empty)
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_goal_owners_admit_exact_operations_and_reject_forged_counters() -> Result<(), String>
    {
        let catalogue = super::catalogue()?;
        for transition in catalogue.transitions.values() {
            super::super::validate(transition, false)?;
            let mut forged = transition.clone();
            forged.key = "oteryn:quest-transition/forged-native-goal".into();
            assert!(super::super::validate(&forged, false).is_err());
        }
        Ok(())
    }
}
