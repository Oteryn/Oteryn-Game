//! Effects remain in their canonical queue across each current Map await.
use super::*;
#[allow(
    clippy::too_many_arguments,
    reason = "current physical and Player owners remain separate source inputs"
)]
pub(super) async fn drain(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    _room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    now: u64,
) -> Result<(), ()> {
    if !states.rotten_blood_callbacks.advanced.pending.is_empty() {
        states.rotten_blood_callbacks.advanced.pending_due = Some(now.saturating_add(1_000_000));
    }
    while let Some(effect) = states
        .rotten_blood_callbacks
        .advanced
        .pending
        .front()
        .cloned()
    {
        match effect {
            AdvancedEffect::Spawn {
                source,
                parent,
                sequence,
                ordinal,
                key,
                around,
                offset,
                close,
            } => {
                if runtime.contains_live_creature(parent) {
                    super::super::spawn_child(
                        runtime, states, native, map, &source, parent, sequence, ordinal, key,
                        around, offset, close, now,
                    )
                    .await?;
                }
            }
            AdvancedEffect::Icon {
                source,
                actor,
                session,
                count,
            } => {
                if !states.is_dead(actor) {
                    runtime
                        .set_rotten_blood_player_icon(
                            &source,
                            actor,
                            session,
                            RottenBloodIconKey::AgonyStacks,
                            Some(u16::from(count)),
                        )
                        .map_err(|_| ())?;
                }
            }
            AdvancedEffect::Damage {
                source,
                boss,
                actor,
                session,
                amount,
            } => {
                if !states.is_dead(actor) && runtime.contains_live_creature(boss) {
                    #[cfg(test)]
                    let before = states
                        .get(runtime, actor, session)
                        .ok_or(())?
                        .vitals()
                        .health;
                    let hit = states
                        .apply_rotten_blood_agony(
                            runtime,
                            &source,
                            actor,
                            session,
                            amount,
                            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                        )
                        .ok_or(())?;
                    #[cfg(test)]
                    {
                        let commits = &mut states.rotten_blood_callbacks.advanced.commits;
                        if commits.len() >= 64 {
                            commits.remove(0);
                        }
                        commits.push(AgonyCommit {
                            actor,
                            before,
                            after: hit.damage.health_after,
                            amount,
                        });
                    }
                    #[cfg(not(test))]
                    let _ = hit;
                }
            }
            AdvancedEffect::Transform { at, from, to } => {
                // These literal preparation objects belong to the retained
                // public origin. Its sealed publication barrier already reset
                // them; the immutable private capture normally has inactive
                // copies, so Lua's getItemById(active) is genuinely nil here.
                // Never manufacture active copies merely to reset them.
                let present = map
                    .source_has_any_item(runtime, super::super::at(at), &[from])
                    .await
                    .ok_or(())?;
                if present {
                    // A changed private copy needs a separately qualified CAS;
                    // an unexplained active object is not a public write grant.
                    let _ = to;
                    return Err(());
                }
            }
        }
        states.rotten_blood_callbacks.advanced.pending.pop_front();
    }
    states.rotten_blood_callbacks.advanced.pending_due = None;
    Ok(())
}
