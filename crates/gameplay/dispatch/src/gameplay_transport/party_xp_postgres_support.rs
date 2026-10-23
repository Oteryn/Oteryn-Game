//! Shared private fixture mechanics for the real PARTY-XP PostgreSQL dispatch cases.
use super::*;

pub(super) async fn join_two<A: std::future::Future, B: std::future::Future>(
    a: A,
    b: B,
) -> (A::Output, B::Output) {
    let mut a = std::pin::pin!(a);
    let mut b = std::pin::pin!(b);
    let mut x = None;
    let mut y = None;
    std::future::poll_fn(move |cx| {
        if x.is_none()
            && let std::task::Poll::Ready(v) = a.as_mut().poll(cx)
        {
            x = Some(v);
        }
        if y.is_none()
            && let std::task::Poll::Ready(v) = b.as_mut().poll(cx)
        {
            y = Some(v);
        }
        if x.is_some() && y.is_some() {
            std::task::Poll::Ready((x.take().expect("first"), y.take().expect("second")))
        } else {
            std::task::Poll::Pending
        }
    })
    .await
}

pub(super) async fn initialize_build(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    vocation: &str,
    seed: u8,
) -> TestResult<(
    crate::spell::cast::CharacterCastFacts,
    crate::durability::character_build::DurableBuildState,
)> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current fence")?
        .ok_or("fence")?;
    let binding = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            crate::durability::character_progression::ProgressionInitializationRequest {
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-1",
    )
    .map_err(debug)?;
    let after = crate::durability::character_build::convert_vocation(&formula, &before, vocation)
        .map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_build(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        crate::durability::character_build::BuildChangeRequest {
            occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(seed))
                .map_err(debug)?,
            cause: crate::durability::character_build::BuildCause::VocationChoice,
            before,
            after: after.clone(),
            pruned_stance: None,
        },
        &formula,
        None,
    )
    .await
    .map_err(debug)?;
    let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::load_character_cast_facts(
            owner.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("official classed Character facts".into());
    };
    Ok((facts, after))
}
