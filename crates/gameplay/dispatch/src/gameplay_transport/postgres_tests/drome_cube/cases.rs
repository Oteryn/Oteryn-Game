//! Each case uses production content and the shared real owner, not a test-only Cube writer.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_items::concoctions::Concoction;
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
use std::num::NonZeroU64;

pub(super) fn admitted(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<AdmittedSession> {
    let offered = [
        OfferedCapability {
            id: 4,
            requires: &[],
        },
        OfferedCapability {
            id: 14,
            requires: &[4],
        },
        OfferedCapability {
            id: 15,
            requires: &[4],
        },
        OfferedCapability {
            id: 31,
            requires: &[],
        },
    ];
    Ok(AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(&offered, &[4, 14, 15, 31])
                .ok_or("selected capabilities")?,
            ..SessionContinuity::FRESH
        },
        item_fence: Some(item_fence()?),
    })
}

pub(super) fn reward_notice(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    selected: Concoction,
) -> TestResult<TutorialNotice> {
    use crate::content::ReferenceItemField::Known;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native content")?;
    let revision = native
        .item_index()
        .and_then(|index| index.revision(selected.production_key()))
        .ok_or("output revision")?;
    let record = native
        .item_policy(selected.production_key(), revision)
        .ok_or("output policy")?
        .record();
    let Known(presentation) = &record.semantics.presentation else {
        return Err("missing output presentation".into());
    };
    let Known(name) = &presentation.name else {
        return Err("missing output name".into());
    };
    Ok(TutorialNotice::StatusText(format!(
        "You have received a {name}!"
    )))
}

fn intent(index: &ItemDefinitionIndex) -> TestResult<UseIntent> {
    let definition = index
        .definition_ref(
            "Item",
            DROME_CUBE_KEY,
            index.revision(DROME_CUBE_KEY).ok_or("cube revision")?,
        )
        .ok_or("cube definition")?;
    Ok(UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
}

pub(super) async fn handle_replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::encode_use_item_intent;
    // The main bag is handle 1, its only entry (the nested bag) is 2; opening it exposes 3.
    let open = encode_use_item_intent(NonZeroU64::new(2).ok_or("bag handle")?);
    let use_cube = encode_use_item_intent(NonZeroU64::new(3).ok_or("cube handle")?);
    crate::gameplay_transport::item_use::drome_cube::test_faults::lose_next_ack();
    let lost = wire::exchange(
        owner,
        admitted(owner, actor, session)?,
        &[open, use_cube.clone()],
    )
    .await?;
    assert_eq!(lost.results, vec![UseDisposition::Committed]);
    assert!(lost.notices.is_empty());
    assert_eq!(lost.retained.continuity.next_command_id, 2);
    assert_eq!(source_quantity(harness).await?, 0);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    // Reconnect allocates new handles; the consumed source is absent. The original receipt wins.
    let replayed = wire::exchange(owner, lost.retained, std::slice::from_ref(&use_cube)).await?;
    assert_eq!(replayed.results, vec![UseDisposition::Committed]);
    let selected = reward_index(id(110), 1).and_then(reward).ok_or("reward")?;
    assert_eq!(replayed.notices, vec![reward_notice(owner, selected)?]);
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    let replay = owner
        .use_drome_cube(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 2,
                item_fence: Some(item_fence()?),
            },
            UseIntent {
                target: UseTarget::Item(NonZeroU64::new(3).ok_or("old handle")?),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
            None,
        )
        .await
        .map_err(|_| "known receipt became unknown")?
        .ok_or("replay")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert!(
        owner.take_tutorial_notice(session).is_none(),
        "a settled receipt repeats no notice"
    );
    reload(harness, session, 2, &use_cube, selected.index() as u8).await?;
    eprintln!("DROME_CUBE_HANDLE POSTGRES_CASE_COMPLETED");
    Ok(())
}

pub(super) async fn stack(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let selected = reward_index(id(110), 1).and_then(reward).ok_or("reward")?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: selected.production_key().into(),
        revision_ref: index
            .revision(selected.production_key())
            .ok_or("output revision")?
            .into(),
    };
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native content")?;
    let facts = NativeItemWeights(native)
        .loop_facts(&definition)
        .ok_or("output facts")?;
    let (maximum, stackable) =
        oteryn_gameplay_items::stacking::stack_maximum(facts.stack).map_err(debug)?;
    assert!(
        stackable && maximum >= 5,
        "the selected donor concoction is stackable"
    );
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family, \
         definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,4,1,encode($5,'hex')::uuid)",
    )
    .bind(id(120).as_slice()).bind(id(42).as_slice()).bind(&definition.production_key)
    .bind(&definition.revision_ref).bind(id(121).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id, \
         parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid, \
         encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($5,'hex')::uuid)",
    )
    .bind(id(120).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice())
    .bind(id(70).as_slice()).bind(id(122).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let payload = encode_item_use_intent(&intent(index)?).map_err(debug)?;
    let exchange = wire::exchange(owner, admitted(owner, actor, session)?, &[payload]).await?;
    assert_eq!(exchange.results, vec![UseDisposition::Committed]);
    assert_eq!(exchange.notices, vec![reward_notice(owner, selected)?]);
    let (shape, output, before, after): (i16, Vec<u8>, i64, i64) = sqlx::query_as(
        "SELECT shape,uuid_send(flask_item_instance_id),flask_quantity_before,flask_quantity_after \
         FROM game_item_use_records WHERE cause_kind=578",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(
        (shape, output.as_slice(), before, after),
        (3, id(120).as_slice(), 4, 5)
    );
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(120).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(quantity, 5);
    assert_eq!(source_quantity(harness).await?, 0);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    eprintln!("DROME_CUBE_STACK POSTGRES_CASE_COMPLETED");
    Ok(())
}

pub(super) async fn concurrent(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let intent = intent(index)?;
    let fence = item_fence()?;
    let attempt = |command_id| {
        owner.use_drome_cube(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            },
            intent.clone(),
            Some(ItemKey::Instance(id(110))),
        )
    };
    let mut first = std::pin::pin!(attempt(1));
    let mut second = std::pin::pin!(attempt(2));
    let (mut a, mut b) = (None, None);
    let (first, second) = std::future::poll_fn(|cx| {
        use std::future::Future;
        if a.is_none()
            && let std::task::Poll::Ready(value) = first.as_mut().poll(cx)
        {
            a = Some(value);
        }
        if b.is_none()
            && let std::task::Poll::Ready(value) = second.as_mut().poll(cx)
        {
            b = Some(value);
        }
        match (a.take(), b.take()) {
            (Some(first), Some(second)) => std::task::Poll::Ready((first, second)),
            (first, second) => {
                a = first;
                b = second;
                std::task::Poll::Pending
            }
        }
    })
    .await;
    let committed = [first, second].into_iter().filter(|result| {
        matches!(result, Ok(Some(value)) if value.disposition == UseDisposition::Committed)
    }).count();
    assert_eq!(committed, 1);
    assert_eq!(source_quantity(harness).await?, 0);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    let next = attempt(3)
        .await
        .map_err(|_| "depleted result unknown")?
        .ok_or("depleted result")?;
    assert_eq!(next.disposition, UseDisposition::NothingToUse);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    eprintln!("DROME_CUBE_CONCURRENT POSTGRES_CASE_COMPLETED");
    Ok(())
}
