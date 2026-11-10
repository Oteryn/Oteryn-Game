//! Real native proposals survive cancellation after a committed ordinary hit.
use super::TestResult;
use super::fixture::debug;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::Duration;

#[test]
fn srx3_soul_war_native_attack_suffix_survives_physical_cancellation_postgres() -> TestResult {
    super::srx3_cruelty_callbacks::run_source_entry(true)
}

struct SelectedPass {
    observed_at_us: u64,
    completed_physical: usize,
    source_index: usize,
    source_occurrence: String,
    proposal_order: Vec<String>,
    effect_cursor: u64,
}

pub(super) async fn actual_source_suffix(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: ExactActorRef,
    session: GameSessionId,
    boss: ExactActorRef,
) -> TestResult {
    let start = owner.owner_now().get();
    let mut selected = None;
    // Exercise the actual source chance/interval owner. No fabricated proposal,
    // selected seed, replacement AI clock or manually accepted cast is admitted.
    'turns: for turn in 0..64_u64 {
        let now = start
            .checked_add(turn.checked_mul(7_000_000).ok_or("native turn interval")?)
            .ok_or("native turn time")?;
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let maximum = states
                .get(&runtime, player, session)
                .ok_or("actual admitted Player vitals")?
                .character_facts()
                .max_health;
            // Keep the search fixture alive through the existing health owner.
            // After selecting the witness, no fixture health credit is applied.
            states
                .apply_health_gain(&runtime, player, session, u64::from(maximum))
                .ok_or("actual current fixture health credit")?;
        }
        for _ in 0..16 {
            if !pause_after_next_physical(owner, now).await? {
                let states = owner.states.spell_states.lock().await;
                if !states.native_profile_pass.has_pending() {
                    break;
                }
                continue;
            }
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let pending = states
                .native_profile_pass
                .snapshot_for_test()
                .ok_or("cancelled real pass remains owned")?;
            let source_index = pending
                .proposals
                .iter()
                .position(|(proposal, source)| *source && proposal.issuer == boss);
            let committed = states.source_damage_receipt_for_test(player, session);
            let qualifies = pending.completed_physical > 0
                && pending
                    .proposals
                    .get(pending.completed_physical - 1)
                    .is_some_and(|(proposal, source)| {
                        !*source
                            && proposal.issuer == boss
                            && committed.as_ref().is_some_and(|(occurrence, _, receipt)| {
                                occurrence
                                    .strip_prefix(proposal.occurrence.id().as_str())
                                    .is_some_and(|suffix| suffix.starts_with(":target:"))
                                    && receipt.applied > 0
                                    && receipt.death.is_none()
                            })
                    })
                && source_index.is_some_and(|index| index >= pending.completed_physical)
                && pending
                    .proposals
                    .iter()
                    .filter(|(_, source)| *source)
                    .count()
                    == 1
                && states
                    .soul_war_callbacks
                    .source_attack_schedule_for_test()
                    .is_empty();
            if qualifies {
                let index = source_index.ok_or("actual captured source suffix")?;
                let source = &pending.proposals[index].0;
                assert_eq!(
                    source.ability.key,
                    "oteryn:ability.srx3.soul-war.goshnar_s_cruelty"
                );
                let occurrence = source.occurrence.id().as_str().to_owned();
                assert!(
                    !states
                        .soul_war_callbacks
                        .source_attack_seen_for_test(boss, &occurrence),
                    "captured DATA has not yet accepted or emitted its source cast"
                );
                let (_, _, receipt) = committed.ok_or("actual committed ordinary hit")?;
                let state = states
                    .get(&runtime, player, session)
                    .ok_or("current Player after ordinary hit")?;
                assert_eq!(state.vitals().health, receipt.health_after);
                assert_eq!(state.revision(), receipt.vitals_revision);
                assert!(receipt.health_after < state.character_facts().max_health);
                assert_eq!(pending.observed_at_us, now);
                selected = Some(SelectedPass {
                    observed_at_us: pending.observed_at_us,
                    completed_physical: pending.completed_physical,
                    source_index: index,
                    source_occurrence: occurrence,
                    proposal_order: pending
                        .proposals
                        .iter()
                        .map(|(proposal, _)| proposal.occurrence.id().as_str().to_owned())
                        .collect(),
                    effect_cursor: effects(owner)?.last().map_or(0, |effect| effect.0),
                });
                break 'turns;
            }
            let source_still_ahead =
                source_index.is_some_and(|index| index >= pending.completed_physical);
            drop(states);
            drop(runtime);
            if source_still_ahead {
                // A first chain/condition-only consumer can be a known no-op.
                // Keep its real completed cursor and require a later actual HP
                // receipt before selecting this cancellation witness.
                continue;
            }
            finish_profile(owner, now).await?;
            break;
        }
        finish_profile(owner, now).await?;
    }
    let selected = selected.ok_or("real native ordinary hit followed by source suffix")?;
    let retry_now = selected
        .observed_at_us
        .checked_add(1)
        .ok_or("actual later retry time")?;
    // Cancel each next consumer boundary too: its completed prefix must advance,
    // while the original opaque batch and all occurrence identities stay fixed.
    for completed in selected.completed_physical + 1..=selected.source_index + 1 {
        let mut reached = false;
        for _ in 0..16 {
            if pause_after_next_physical(owner, retry_now).await? {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "actual retained suffix reaches its next physical boundary"
        );
        let states = owner.states.spell_states.lock().await;
        let pending = states
            .native_profile_pass
            .snapshot_for_test()
            .ok_or("same real pass after suffix cancellation")?;
        assert_eq!(pending.completed_physical, completed);
        assert_eq!(pending.observed_at_us, selected.observed_at_us);
        assert_eq!(
            pending
                .proposals
                .iter()
                .map(|(proposal, _)| proposal.occurrence.id().as_str().to_owned())
                .collect::<Vec<_>>(),
            selected.proposal_order,
            "resume never reserves a replacement native Think or repeats its prefix"
        );
    }
    finish_profile(owner, retry_now).await?;
    let due = retry_now
        .checked_add(7_000_000)
        .ok_or("source seven-second delay")?;
    {
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states.soul_war_callbacks.source_attack_schedule_for_test(),
            vec![(
                boss,
                selected.source_occurrence.clone(),
                selected.observed_at_us,
                due
            )],
            "one retained occurrence: original ordering time, actual execution +7s"
        );
        assert!(
            states
                .soul_war_callbacks
                .source_attack_seen_for_test(boss, &selected.source_occurrence,)
        );
    }
    assert_eq!(
        effects(owner)?
            .iter()
            .filter(|(sequence, _, effect)| *sequence > selected.effect_cursor && *effect == 13)
            .count(),
        1,
        "the actual source telegraph is emitted once after the retained prefix"
    );
    let health_before = health(owner, player, session).await?;
    let cues_before = effects(owner)?;
    drain_attacks_at(owner, due - 1).await?;
    assert_eq!(health(owner, player, session).await?, health_before);
    assert_eq!(effects(owner)?, cues_before, "no early source area");
    owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .pause_attack_after_player_commit_for_test();
    assert!(
        drain_attacks_at(owner, due).await.is_err(),
        "the real source callback stops after its first canonical HP commit"
    );
    let health_after = health(owner, player, session).await?;
    let receipt = {
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states.soul_war_callbacks.source_attack_schedule_for_test(),
            vec![(
                boss,
                selected.source_occurrence.clone(),
                selected.observed_at_us,
                due
            )],
            "post-HP refusal retains the original source and its partial area"
        );
        states
            .source_heal_receipt_for_test(player, session)
            .ok_or("actual source positive Elemental health receipt")?
    };
    assert!(
        receipt
            .0
            .starts_with(&format!("{}:tile:", selected.source_occurrence))
    );
    assert!((2300..=3000).contains(&receipt.1));
    assert!(
        receipt.2.applied > 0,
        "literal positive source health reaches wounded Player"
    );
    assert_eq!(receipt.2.health_before, health_before.0);
    assert_eq!(receipt.2.health_after, health_after.0);
    assert_eq!(receipt.2.vitals_revision, health_after.1);
    assert_eq!(health_after.0 - health_before.0, receipt.2.applied);
    drain_attacks_at(owner, due).await?;
    assert_eq!(
        health(owner, player, session).await?,
        health_after,
        "resuming the partial source area does not heal its committed Player twice"
    );
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .soul_war_callbacks
            .source_attack_schedule_for_test()
            .is_empty()
    );
    let cues_after = effects(owner)?;
    let area_tiles: Vec<_> = cues_after
        .iter()
        .filter(|(sequence, _, effect)| {
            *sequence > cues_before.last().map_or(0, |cue| cue.0) && *effect == 7
        })
        .map(|(_, at, _)| (at.x, at.y, at.floor))
        .collect();
    assert_eq!(
        area_tiles
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        area_tiles.len(),
        "source area resumes without repeating completed tile cues"
    );
    assert!(cues_after.iter().any(|(sequence, _, effect)| {
        *sequence > cues_before.last().map_or(0, |cue| cue.0) && *effect == 7
    }));
    drain_attacks_at(owner, due).await?;
    assert_eq!(health(owner, player, session).await?, health_after);
    assert_eq!(
        effects(owner)?,
        cues_after,
        "replay emits no repeated source area cues"
    );
    assert_eq!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .source_heal_receipt_for_test(player, session),
        Some(receipt),
        "replay preserves the original heal receipt and vitals revision"
    );
    Ok(())
}

async fn pause_after_next_physical(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    now: u64,
) -> TestResult<bool> {
    let signal = owner
        .states
        .spell_states
        .lock()
        .await
        .native_profile_pass
        .pause_after_physical_for_test(1)
        .map_err(|_| "arm actual physical boundary")?;
    let mut reached = std::pin::pin!(signal.notified());
    let mut actual =
        std::pin::pin!(owner.drain_monster_melee_at(SemanticTimeMicros::from_micros(now)));
    // The actual owner future, including its SQL transaction and locks, is
    // dropped on return. Notify marks the synchronous post-HP cursor boundary.
    Ok(tokio::time::timeout(
        Duration::from_secs(10),
        poll_fn(|cx| {
            if reached.as_mut().poll(cx).is_ready() {
                Poll::Ready(true)
            } else if actual.as_mut().poll(cx).is_ready() {
                Poll::Ready(false)
            } else {
                Poll::Pending
            }
        }),
    )
    .await?)
}

async fn finish_profile(owner: &ComposedFreshAdmission<'_, '_, '_>, now: u64) -> TestResult {
    owner
        .states
        .spell_states
        .lock()
        .await
        .native_profile_pass
        .clear_physical_pause_for_test();
    for _ in 0..32 {
        if !owner
            .states
            .spell_states
            .lock()
            .await
            .native_profile_pass
            .has_pending()
        {
            return Ok(());
        }
        owner
            .drain_monster_melee_at(SemanticTimeMicros::from_micros(now))
            .await;
    }
    Err("real retained profile pass did not finish".into())
}

async fn health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<(u32, u64)> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let current = states
        .get(&runtime, actor, session)
        .ok_or("current Player vitals")?;
    Ok((current.vitals().health, current.revision()))
}

fn effects(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<(u64, crate::map::overlay::TilePos, u16)>> {
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("actual source room")?;
    let map = crate::gameplay_transport::soul_war_scene::source_map(room)
        .ok_or("actual source effect owner")?;
    Ok(map.facts.movement_effects_since(0))
}

async fn drain_attacks_at(owner: &ComposedFreshAdmission<'_, '_, '_>, now: u64) -> TestResult {
    use crate::durability::DurabilityError;
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    use crate::movement::monster_map::CurrentMonsterMap;
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("actual source timer lane")?;
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("actual source timer room")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual source definitions")?;
    let pass = owner.root.try_issue_semantic_pass()?;
    let mut context = (owner, &mut *runtime, &mut *states, &permit, room, native);
    pass.run_with_context(&mut context, move |holder, deadline, context| {
        Box::pin(async move {
            let (owner, runtime, states, permit, room, native) = context;
            let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
            let binding = runtime.binding();
            let authority = assert_spell_item_scope_in_transaction(
                &mut tx,
                permit,
                owner.root,
                owner.character,
                owner.holder,
                binding.runtime_scope(),
                binding.scope_generation().get(),
            )
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
            let objects = owner.door.lock().await;
            {
                let mut map =
                    CurrentMonsterMap::new(&mut tx, &authority, runtime, room, &objects, native);
                for _ in 0..64 {
                    if !states
                        .soul_war_callbacks
                        .next_attack_due()
                        .is_some_and(|due| due <= now)
                    {
                        break;
                    }
                    crate::gameplay_transport::soul_war_callbacks::drive_attack_due_through(
                        runtime, states, room, native, &mut map, now, now,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                }
                if states
                    .soul_war_callbacks
                    .next_attack_due()
                    .is_some_and(|due| due <= now)
                {
                    return Err(DurabilityError::Unavailable);
                }
            }
            tx.rollback().await?;
            Ok(())
        })
    })
    .await
    .map_err(debug)?;
    Ok(())
}
