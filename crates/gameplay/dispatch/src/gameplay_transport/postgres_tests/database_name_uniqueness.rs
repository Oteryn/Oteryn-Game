use super::{
    TestResult,
    fixture::{debug, test_database_name},
};

#[test]
fn test_database_names_stay_unique_within_postgres_identifier_limit() -> TestResult {
    let tag = "quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal";
    let names = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    (0..128)
                        .map(|_| test_database_name(tag))
                        .collect::<Result<Vec<_>, _>>()
                })
            })
            .collect();
        let mut names = Vec::new();
        for thread in threads {
            names.extend(thread.join().map_err(debug)??);
        }
        Ok::<_, Box<dyn std::error::Error>>(names)
    })?;
    let postgres_names: std::collections::BTreeSet<_> = names
        .iter()
        .map(|name| &name[..name.len().min(63)])
        .collect();
    assert_eq!(postgres_names.len(), names.len());
    assert!(names.iter().all(|name| name.len() <= 63));
    Ok(())
}
