#[test]
fn quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal() -> TestResult {
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    // Lane ownership comes from the coverage quest title, including its leading
    // article. Claim slugs can describe an unrelated item or donor storage key.
    let claims: BTreeSet<_> = coverage["quests"]
        .as_array()
        .ok_or("coverage quests")?
        .iter()
        .filter(|quest| {
            quest["quest"]
                .as_str()
                .and_then(|title| title.rsplit('/').next())
                .and_then(|title| title.bytes().next())
                .is_some_and(|initial| (b'a'..=b'm').contains(&initial))
        })
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .filter(|chest| chest["status"] == "supported_unqualified")
        .map(|chest| string(&chest["claim"]))
        .collect::<TestResult<_>>()?;
    assert!(!claims.is_empty(), "A..M qualification requires source claims");
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| claims.contains(&case.claim))
        .collect();
    assert_eq!(
        selected.iter().map(|case| &case.claim).collect::<BTreeSet<_>>(),
        claims.iter().collect(),
        "every A..M unqualified claim needs every source placement exercised"
    );
    execute(selected)
}
