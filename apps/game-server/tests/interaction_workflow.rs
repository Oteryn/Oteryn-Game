#[path = "../src/interaction/mod.rs"]
mod interaction;

use interaction::{
    AuthorityFenceEvidence, ChildLifecycle, ChildLifecycleBook, ChildOccurrenceRef,
    DelegatedOperation, ForeignOwner, InteractionDispatcher, InteractionError, InteractionPlan,
    ProposalAdapter, ProposalResult, ReconciliationOutcome, RetainedChildLifecycles,
    RngDecisionRef, RootSourceOccurrenceRef, SelectedChildWork, SemanticRevisionContext,
    TriggerRegistration,
};
use std::marker::PhantomData;

fn root() -> Result<RootSourceOccurrenceRef, InteractionError> {
    RootSourceOccurrenceRef::new("command:root-1")
}

fn revisions() -> Result<SemanticRevisionContext, InteractionError> {
    SemanticRevisionContext::new("content:r1", "ruleset:r1", "sim:v1")
}

fn fence() -> Result<AuthorityFenceEvidence, InteractionError> {
    AuthorityFenceEvidence::new(7, 11, 13)
}

fn child(
    root: &RootSourceOccurrenceRef,
    target: &str,
    ordinal: Option<u16>,
) -> Result<ChildOccurrenceRef, InteractionError> {
    ChildOccurrenceRef::for_root(
        root,
        "interaction:door",
        target,
        "use",
        ordinal,
        &revisions()?,
    )
}

fn registrations(count: usize) -> Result<Vec<TriggerRegistration>, InteractionError> {
    (0..count)
        .map(|index| TriggerRegistration::new(&format!("trigger:{index}")))
        .collect()
}

fn selected_children(
    children: Vec<ChildOccurrenceRef>,
    registrations: &[TriggerRegistration],
) -> Vec<SelectedChildWork> {
    children
        .into_iter()
        .zip(registrations.iter().cloned())
        .map(|(child, registration)| SelectedChildWork::new(registration, child))
        .collect()
}

fn plan(
    root: RootSourceOccurrenceRef,
    children: Vec<ChildOccurrenceRef>,
    retained: Vec<ChildOccurrenceRef>,
) -> Result<InteractionPlan, InteractionError> {
    let registrations = registrations(children.len())?;
    InteractionPlan::build(
        root.clone(),
        registrations.clone(),
        selected_children(children, &registrations),
        RetainedChildLifecycles::new(root, retained),
    )
}

fn plan_with_one_child() -> Result<(InteractionPlan, ChildOccurrenceRef), InteractionError> {
    let source = root()?;
    let occurrence = child(&source, "world:main/tile:1,2,3", None)?;
    let plan = plan(
        source.clone(),
        vec![occurrence.clone()],
        vec![occurrence.clone()],
    )?;
    assert_eq!(plan.root(), &source);
    Ok((plan, occurrence))
}

#[test]
fn stable_child_identity_is_reconstructed_from_the_same_semantic_tuple()
-> Result<(), InteractionError> {
    let source = root()?;
    let first = child(&source, "world:main/tile:1,2,3", None)?;
    let retry = child(&source, "world:main/tile:1,2,3", None)?;

    assert_eq!(first, retry);
    Ok(())
}

#[test]
fn child_identity_preserves_parent_ancestry_and_bound_revisions() -> Result<(), InteractionError> {
    let source = root()?;
    let parent = child(&source, "world:main/tile:1,2,3", None)?;
    let descendant = ChildOccurrenceRef::for_child(
        &parent,
        "interaction:lever",
        "world:main/tile:1,2,4",
        "contact",
        None,
        &revisions()?,
    )?;
    let different_parent = child(&source, "world:main/tile:1,2,5", None)?;
    let same_target_from_other_parent = ChildOccurrenceRef::for_child(
        &different_parent,
        "interaction:lever",
        "world:main/tile:1,2,4",
        "contact",
        None,
        &revisions()?,
    )?;

    assert_ne!(descendant, same_target_from_other_parent);
    assert_eq!(descendant.ancestry_depth(), 2);
    Ok(())
}

#[test]
fn plan_uses_canonical_child_order_after_shuffled_typed_registration()
-> Result<(), InteractionError> {
    let source = root()?;
    let later = child(&source, "world:main/tile:z", None)?;
    let earlier = child(&source, "world:main/tile:a", None)?;
    let plan = plan(
        source,
        vec![later.clone(), earlier.clone()],
        vec![later, earlier.clone()],
    )?;

    assert_eq!(
        plan.children(),
        &[earlier, child(&root()?, "world:main/tile:z", None)?]
    );
    Ok(())
}

#[test]
fn rng_decision_identity_is_stable_and_purpose_isolated() -> Result<(), InteractionError> {
    let source = root()?;
    let occurrence = child(&source, "world:main/tile:1,2,3", None)?;

    let retry = RngDecisionRef::new(&occurrence, "interaction:selection", 0)?;
    let same = RngDecisionRef::new(&occurrence, "interaction:selection", 0)?;
    let separate_purpose = RngDecisionRef::new(&occurrence, "interaction:effect", 0)?;

    assert_eq!(retry, same);
    assert_ne!(retry, separate_purpose);
    Ok(())
}

#[test]
fn duplicate_semantic_children_are_rejected_before_lifecycle_allocation()
-> Result<(), InteractionError> {
    let source = root()?;
    let occurrence = child(&source, "world:main/tile:1,2,3", None)?;

    assert!(matches!(
        plan(
            source,
            vec![occurrence.clone(), occurrence.clone()],
            vec![occurrence]
        ),
        Err(InteractionError::DuplicateChild)
    ));
    Ok(())
}

#[test]
fn cascade_depth_maximum_is_accepted_and_maximum_plus_one_is_rejected_before_plan()
-> Result<(), InteractionError> {
    let source = root()?;
    let first = child(&source, "world:main/tile:1", None)?;
    let second = ChildOccurrenceRef::for_child(
        &first,
        "interaction:lever",
        "world:main/tile:2",
        "contact",
        None,
        &revisions()?,
    )?;
    let third = ChildOccurrenceRef::for_child(
        &second,
        "interaction:lever",
        "world:main/tile:3",
        "contact",
        None,
        &revisions()?,
    )?;

    let allowed = plan(
        source.clone(),
        vec![first.clone(), second.clone()],
        vec![first, second],
    )?;
    assert_eq!(allowed.children().len(), 2);
    assert!(matches!(
        plan(source, vec![third.clone()], vec![third]),
        Err(InteractionError::CapacityExceeded {
            resource: "cascade depth",
            ..
        })
    ));
    Ok(())
}

#[test]
fn child_fanout_maximum_plus_one_is_rejected_from_typed_registrations()
-> Result<(), InteractionError> {
    let source = root()?;
    let mut allowed_children = Vec::new();
    for ordinal in 0..8 {
        allowed_children.push(child(
            &source,
            &format!("world:main/fanout:{ordinal}"),
            Some(ordinal),
        )?);
    }
    let allowed = plan(source.clone(), allowed_children.clone(), allowed_children)?;
    assert_eq!(allowed.children().len(), 8);

    let mut children = Vec::new();
    for ordinal in 0..=8 {
        children.push(child(
            &source,
            &format!("world:main/fanout:{ordinal}"),
            Some(ordinal),
        )?);
    }
    assert!(matches!(
        plan(source, children.clone(), children),
        Err(InteractionError::CapacityExceeded {
            resource: "child fan-out",
            ..
        })
    ));
    Ok(())
}

#[test]
fn root_work_maximum_plus_one_is_rejected_from_typed_registrations() -> Result<(), InteractionError>
{
    let source = root()?;
    let mut children = Vec::new();
    for ordinal in 0..4 {
        let parent = child(
            &source,
            &format!("world:main/root:{ordinal}"),
            Some(ordinal),
        )?;
        children.push(parent.clone());
        children.push(ChildOccurrenceRef::for_child(
            &parent,
            "interaction:lever",
            &format!("world:main/child:{ordinal}"),
            "contact",
            None,
            &revisions()?,
        )?);
    }
    let allowed = plan(source.clone(), children.clone(), children.clone())?;
    assert_eq!(allowed.children().len(), 8);
    children.push(child(&source, "world:main/root:extra", Some(9))?);

    assert!(matches!(
        plan(source, children.clone(), children),
        Err(InteractionError::CapacityExceeded {
            resource: "root work",
            ..
        })
    ));
    Ok(())
}

#[test]
fn trigger_candidates_and_retained_lifecycles_reject_typed_maximum_plus_one()
-> Result<(), InteractionError> {
    let source = root()?;
    let occurrence = child(&source, "world:main/tile:1", None)?;
    let candidates = registrations(16)?;
    let allowed = InteractionPlan::build(
        source.clone(),
        candidates.clone(),
        selected_children(vec![occurrence.clone()], &candidates),
        RetainedChildLifecycles::new(source.clone(), vec![occurrence.clone()]),
    )?;
    assert_eq!(allowed.children().len(), 1);

    let trigger_overflow = registrations(17)?;
    assert!(matches!(
        InteractionPlan::build(
            source.clone(),
            trigger_overflow.clone(),
            selected_children(vec![occurrence.clone()], &trigger_overflow),
            RetainedChildLifecycles::new(source.clone(), vec![occurrence.clone()]),
        ),
        Err(InteractionError::CapacityExceeded {
            resource: "trigger candidates",
            ..
        })
    ));

    let mut retained_overflow = Vec::new();
    for ordinal in 0..9 {
        retained_overflow.push(child(
            &source,
            &format!("world:main/retained:{ordinal}"),
            Some(ordinal),
        )?);
    }
    assert!(matches!(
        plan(source, vec![occurrence], retained_overflow),
        Err(InteractionError::CapacityExceeded {
            resource: "retained child lifecycles",
            ..
        })
    ));
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AbilityOperation(&'static str);

impl DelegatedOperation for AbilityOperation {
    fn owner(&self) -> ForeignOwner {
        ForeignOwner::Ability
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DurabilityOperation(&'static str);

impl DelegatedOperation for DurabilityOperation {
    fn owner(&self) -> ForeignOwner {
        ForeignOwner::Durability
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MisboundOperation(&'static str);

impl DelegatedOperation for MisboundOperation {
    fn owner(&self) -> ForeignOwner {
        ForeignOwner::Ability
    }
}

#[derive(Debug)]
struct RecordingAdapter<O> {
    owner: ForeignOwner,
    result: ProposalResult,
    proposal_calls: usize,
    direct_foreign_mutations: usize,
    operation: PhantomData<O>,
}

impl<O> RecordingAdapter<O> {
    const fn pending(owner: ForeignOwner) -> Self {
        Self {
            owner,
            result: ProposalResult::Pending,
            proposal_calls: 0,
            direct_foreign_mutations: 0,
            operation: PhantomData,
        }
    }
}

impl<O: DelegatedOperation> ProposalAdapter for RecordingAdapter<O> {
    type Operation = O;

    fn owner(&self) -> ForeignOwner {
        self.owner
    }

    fn propose(
        &mut self,
        _child: &ChildOccurrenceRef,
        _operation: &Self::Operation,
        _fence: &AuthorityFenceEvidence,
    ) -> ProposalResult {
        self.proposal_calls += 1;
        self.result
    }
}

#[test]
fn duplicate_dispatch_does_not_repeat_foreign_proposal_and_pending_reconciles_same_occurrence()
-> Result<(), InteractionError> {
    let (plan, occurrence) = plan_with_one_child()?;
    let mut lifecycle = ChildLifecycleBook::<AbilityOperation>::from_plan(&plan);
    let mut dispatcher = InteractionDispatcher::new();
    let operation = AbilityOperation("ability-op:1");
    let mut adapter = RecordingAdapter::<AbilityOperation>::pending(ForeignOwner::Ability);
    let authority = fence()?;

    assert_eq!(
        dispatcher.dispatch(
            &mut lifecycle,
            &occurrence,
            &operation,
            &authority,
            &mut adapter,
        )?,
        ChildLifecycle::Pending
    );
    assert_eq!(
        dispatcher.dispatch(
            &mut lifecycle,
            &occurrence,
            &operation,
            &authority,
            &mut adapter,
        )?,
        ChildLifecycle::Pending
    );
    assert_eq!(adapter.proposal_calls, 1);
    assert_eq!(adapter.direct_foreign_mutations, 0);

    assert_eq!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &authority,
            ReconciliationOutcome::Ambiguous,
        )?,
        ChildLifecycle::Pending
    );
    assert_eq!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &authority,
            ReconciliationOutcome::Committed,
        )?,
        ChildLifecycle::Committed
    );
    assert_eq!(
        dispatcher.dispatch(
            &mut lifecycle,
            &occurrence,
            &operation,
            &authority,
            &mut adapter,
        )?,
        ChildLifecycle::Committed
    );
    assert_eq!(adapter.proposal_calls, 1);
    Ok(())
}

#[test]
fn stale_operation_and_unproven_cancellation_leave_the_same_child_pending()
-> Result<(), InteractionError> {
    let (plan, occurrence) = plan_with_one_child()?;
    let mut lifecycle = ChildLifecycleBook::<DurabilityOperation>::from_plan(&plan);
    let mut dispatcher = InteractionDispatcher::new();
    let operation = DurabilityOperation("dur-op:1");
    let stale = DurabilityOperation("dur-op:2");
    let mut adapter = RecordingAdapter::<DurabilityOperation>::pending(ForeignOwner::Durability);
    let authority = fence()?;

    dispatcher.dispatch(
        &mut lifecycle,
        &occurrence,
        &operation,
        &authority,
        &mut adapter,
    )?;
    assert!(matches!(
        lifecycle.reconcile(
            &occurrence,
            &stale,
            &authority,
            ReconciliationOutcome::Committed,
        ),
        Err(InteractionError::StaleOwnerOperation)
    ));
    assert_eq!(lifecycle.state(&occurrence)?, ChildLifecycle::Pending);
    assert_eq!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &authority,
            ReconciliationOutcome::CancellationRequested,
        )?,
        ChildLifecycle::Pending
    );
    assert_eq!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &authority,
            ReconciliationOutcome::CancelledWithRetirementProof,
        )?,
        ChildLifecycle::Rejected
    );
    Ok(())
}

#[test]
fn delegated_rejection_is_terminal_for_the_same_child() -> Result<(), InteractionError> {
    let (plan, occurrence) = plan_with_one_child()?;
    let mut lifecycle = ChildLifecycleBook::<AbilityOperation>::from_plan(&plan);
    let mut dispatcher = InteractionDispatcher::new();
    let operation = AbilityOperation("ability-op:rejected");
    let authority = fence()?;
    let mut adapter = RecordingAdapter::<AbilityOperation>::pending(ForeignOwner::Ability);

    dispatcher.dispatch(
        &mut lifecycle,
        &occurrence,
        &operation,
        &authority,
        &mut adapter,
    )?;
    assert_eq!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &authority,
            ReconciliationOutcome::Rejected,
        )?,
        ChildLifecycle::Rejected
    );
    assert_eq!(adapter.proposal_calls, 1);
    Ok(())
}

#[test]
fn stale_authority_generation_or_revision_cannot_complete_pending_child()
-> Result<(), InteractionError> {
    let (plan, occurrence) = plan_with_one_child()?;
    let mut lifecycle = ChildLifecycleBook::<AbilityOperation>::from_plan(&plan);
    let mut dispatcher = InteractionDispatcher::new();
    let operation = AbilityOperation("ability-op:1");
    let mut adapter = RecordingAdapter::<AbilityOperation>::pending(ForeignOwner::Ability);
    let authority = fence()?;

    dispatcher.dispatch(
        &mut lifecycle,
        &occurrence,
        &operation,
        &authority,
        &mut adapter,
    )?;
    let stale_generation = AuthorityFenceEvidence::new(6, 11, 13)?;
    let stale_revision = AuthorityFenceEvidence::new(7, 12, 13)?;
    assert!(matches!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &stale_generation,
            ReconciliationOutcome::Committed,
        ),
        Err(InteractionError::StaleAuthorityFence)
    ));
    assert!(matches!(
        lifecycle.reconcile(
            &occurrence,
            &operation,
            &stale_revision,
            ReconciliationOutcome::Committed,
        ),
        Err(InteractionError::StaleAuthorityFence)
    ));
    assert_eq!(lifecycle.state(&occurrence)?, ChildLifecycle::Pending);
    assert_eq!(adapter.proposal_calls, 1);
    Ok(())
}

#[test]
fn dispatcher_rejects_mismatched_owner_without_direct_write() -> Result<(), InteractionError> {
    let (plan, occurrence) = plan_with_one_child()?;
    let mut lifecycle = ChildLifecycleBook::<MisboundOperation>::from_plan(&plan);
    let mut dispatcher = InteractionDispatcher::new();
    let operation = MisboundOperation("ability-op:1");
    let mut adapter = RecordingAdapter::<MisboundOperation>::pending(ForeignOwner::Foundation);

    assert!(matches!(
        dispatcher.dispatch(
            &mut lifecycle,
            &occurrence,
            &operation,
            &fence()?,
            &mut adapter,
        ),
        Err(InteractionError::ForeignOwnerMismatch)
    ));
    assert_eq!(adapter.proposal_calls, 0);
    assert_eq!(adapter.direct_foreign_mutations, 0);
    assert_eq!(lifecycle.state(&occurrence)?, ChildLifecycle::Unstarted);
    Ok(())
}
