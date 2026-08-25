use super::{
    ChildOccurrenceRef, InteractionPlan, RetainedChildLifecycles, RootSourceOccurrenceRef,
    SelectedChildWork, SemanticRevisionContext, TriggerRegistration,
};

#[test]
fn owned_module_tree_compiles_without_composition_root_publication()
-> Result<(), super::InteractionError> {
    let root = RootSourceOccurrenceRef::new("command:standalone")?;
    let revisions = SemanticRevisionContext::new("content:r1", "ruleset:r1", "sim:v1")?;
    let child = ChildOccurrenceRef::for_root(
        &root,
        "interaction:standalone",
        "world:main/fixture:1",
        "use",
        None,
        &revisions,
    )?;
    let retained = RetainedChildLifecycles::new(root.clone(), vec![child.clone()]);
    let registration = TriggerRegistration::new("trigger:standalone")?;
    let plan = InteractionPlan::build(
        root,
        vec![registration.clone()],
        vec![SelectedChildWork::new(registration, child)],
        retained,
    )?;

    assert_eq!(plan.children().len(), 1);
    Ok(())
}
