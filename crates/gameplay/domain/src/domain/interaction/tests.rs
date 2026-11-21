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

#[derive(Clone, PartialEq, Eq)]
struct Operation(u8);

impl super::DelegatedOperation for Operation {
    fn owner(&self) -> super::ForeignOwner {
        super::ForeignOwner::Foundation
    }
}

#[test]
fn terminal_replay_rejects_different_operation() -> Result<(), super::InteractionError> {
    let root = RootSourceOccurrenceRef::new("replay")?;
    let revisions = SemanticRevisionContext::new("c", "r", "s")?;
    let child = ChildOccurrenceRef::for_root(&root, "i", "t", "v", None, &revisions)?;
    let registration = TriggerRegistration::new("trigger")?;
    let plan = InteractionPlan::build(
        root.clone(),
        vec![registration.clone()],
        vec![SelectedChildWork::new(registration, child.clone())],
        RetainedChildLifecycles::new(root, vec![child.clone()]),
    )?;
    let fence = super::AuthorityFenceEvidence::new(1, 1, 1)?;
    for outcome in [
        super::ReconciliationOutcome::Committed,
        super::ReconciliationOutcome::Rejected,
    ] {
        let mut book = super::ChildLifecycleBook::from_plan(&plan);
        book.begin(&child, &Operation(1), &fence)?;
        let state = book.reconcile(&child, &Operation(1), &fence, outcome)?;
        assert_eq!(
            book.begin(&child, &Operation(2), &fence),
            Err(super::InteractionError::StaleOwnerOperation)
        );
        assert_eq!(book.begin(&child, &Operation(1), &fence)?, (state, false));
    }
    Ok(())
}

#[test]
fn plan_rejects_oversized_inputs_before_child_validation() -> Result<(), super::InteractionError> {
    let root = RootSourceOccurrenceRef::new("bounded")?;
    let revisions = SemanticRevisionContext::new("c", "r", "s")?;
    let child = ChildOccurrenceRef::for_root(&root, "i", "t", "v", None, &revisions)?;
    let registration = TriggerRegistration::new("unregistered")?;
    for (selected_count, retained_count, resource) in
        [(9, 0, "root work"), (1, 9, "retained child lifecycles")]
    {
        assert_eq!(
            InteractionPlan::build(
                root.clone(),
                vec![],
                vec![SelectedChildWork::new(registration.clone(), child.clone()); selected_count],
                RetainedChildLifecycles::new(root.clone(), vec![child.clone(); retained_count]),
            ),
            Err(super::InteractionError::CapacityExceeded {
                resource,
                limit: 8,
                observed: 9,
            })
        );
    }
    Ok(())
}
