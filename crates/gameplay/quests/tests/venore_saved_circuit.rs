#[test]
fn donor_quest_venore_saved_circuit_preserves_progress_and_cooldowns() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::{QuestRefusal, loader::load_embedded_quest_state};
    use std::collections::BTreeMap;

    let loaded = load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let progress = |name| format!("oteryn:quest-progress/chosen/venore-daily/{name}");
    let transition = |name: &str| {
        catalogue
            .transition(&format!("oteryn:quest-reward/chosen/venore-daily/{name}"))
            .ok_or_else(|| format!("missing Venore transition {name}"))
    };
    let completed = BTreeMap::from([(progress("stage"), 13), (progress("last-report"), 100_000)]);
    for job in ["maintenance", "pathfinder", "sabotage"] {
        let accept = transition(&format!("{job}/accept"))?;
        let restart = transition(&format!("{job}/restart"))?;
        assert_eq!(
            catalogue.evaluate(accept, &completed, 100_001),
            Err(QuestRefusal::StageMismatch),
            "{job} bypasses the predecessor cooldown"
        );
        assert_eq!(
            catalogue.evaluate(restart, &completed, 171_999),
            Err(QuestRefusal::StageMismatch)
        );
        assert!(
            catalogue.evaluate(restart, &completed, 172_000).is_ok(),
            "{job} reopens at the predecessor cutoff"
        );
    }
    assert_eq!(
        catalogue.evaluate(transition("bonus")?, &completed, 100_001),
        Err(QuestRefusal::StageMismatch)
    );

    let circuit = BTreeMap::from([(progress("stage"), 11), (progress("kills"), 0)]);
    assert!(
        catalogue
            .evaluate(transition("maintenance/report")?, &circuit, 100_001)
            .is_ok()
    );
    assert_eq!(
        catalogue.evaluate(transition("scott")?, &circuit, 100_001),
        Err(QuestRefusal::StageMismatch)
    );
    assert!(
        catalogue
            .evaluate(transition("frank")?, &circuit, 100_001)
            .is_ok()
    );
    Ok(())
}
