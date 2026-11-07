//! Literal source Maxxen/Zamulosh teleports; no arena or player spell grant.
use super::*;
#[derive(Debug)]
struct Pending {
    captured_at_us: u64,
    proposal: ProfileAbilityProposal,
    sequence: u64,
    parent: Ref,
    destination: MovementLocalPosition,
    context: crate::content::QualifiedCallbackSourceContext,
}
#[derive(Debug, Default)]
pub(super) struct NativeTeleports {
    maxxen: Option<crate::content::QualifiedCallbackSourceContext>,
    zamulosh: Option<crate::content::QualifiedCallbackSourceContext>,
    pending: Vec<Pending>,
    memos: ActorMemoTable<Ref, Memo>,
}
fn callback(p: &ProfileAbilityProposal) -> bool {
    matches!(
        p.ability.key.as_str(),
        "oteryn:ability.spell.maxxenteleport" | "oteryn:ability.spell.zamulosh_tp"
    )
}
fn effect(room: &QualifiedNativeEntryRoom, at: MovementLocalPosition, id: u16) {
    use crate::map::boot::BundleCollisionIndexExt;
    if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        room.movement_cells().index()
        && let Some(pos) = super::visual_position(at)
    {
        index.map().facts.emit_movement_effect(pos, id);
    }
}
impl NativeTeleports {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.pending
            .iter()
            .map(|pending| pending.captured_at_us)
            .min()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "same current native source and map transaction"
    )]
    pub(super) async fn drive(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(active, runtime, room, native, map, proposals, now, now)
            .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "qualified source, current runtime, map, proposals and original cutoff require independent authority checks"
    )]
    pub(super) async fn drive_due_through(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            active,
            runtime,
            room,
            native,
            map,
            &[],
            eligible_through_us,
            now,
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "qualified source, current runtime, map, proposals and original cutoff require independent authority checks"
    )]
    async fn drive_inner(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        for p in proposals.iter().filter(|p| callback(p)) {
            if self
                .pending
                .iter()
                .any(|entry| entry.proposal.occurrence.id() == p.occurrence.id())
            {
                continue;
            }
            let snapshot = runtime.companion_snapshot(p.issuer).map_err(|_| ())?;
            let Some(record) = native.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            let Some(source) =
                SevenSource::qualify_native(native, &record.profile.target, p.entry_index, p.list)
                    .map_err(|_| ())?
            else {
                continue;
            };
            let maxxen = source.kind() == SevenKind::Maxxen;
            if !maxxen && source.kind() != SevenKind::Zamulosh {
                continue;
            }
            if oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(runtime,p).is_err(){continue;}
            let mut normalized = p.clone();
            normalized.target = p.issuer;
            let atom = format!(
                "actor:{}",
                oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::hex(
                    p.issuer.placement_identity()
                )
            );
            normalized.intent =
                crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).map_err(|_| ())?;
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            let work = source
                .prepare_map_work(
                    runtime,
                    runtime.owner_fence().map_err(|_| ())?,
                    stamp,
                    &normalized,
                )
                .map_err(|_| ())?;
            if self
                .memos
                .get(p.issuer, &p.ability)
                .is_some_and(|memo| memo.sequence >= work.sequence)
            {
                continue;
            }
            let cached = if maxxen {
                &mut self.maxxen
            } else {
                &mut self.zamulosh
            };
            if !cached
                .as_ref()
                .is_some_and(|proof| proof.matches(room, runtime.content_pin()))
            {
                *cached=active.qualify_callback_source_context(room,runtime.content_pin(),if maxxen{crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackProfessorMaxxen}else{crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackZamulosh});
            }
            let Some(context) = cached.clone() else {
                continue;
            };
            let source_position = if maxxen {
                let hash = Sha256::digest(p.occurrence.id().as_str().as_bytes());
                let mut id = [0; 16];
                id.copy_from_slice(&hash[..16]);
                let root = GameplayDecisionRoot::from_bytes(native.source_digest());
                let x = deterministic_decision_u64(
                    &root,
                    DecisionOccurrenceId::from_bytes(id),
                    "maxxen_teleport_x",
                    0,
                )
                .map_err(|_| ())?;
                let y = deterministic_decision_u64(
                    &root,
                    DecisionOccurrenceId::from_bytes(id),
                    "maxxen_teleport_y",
                    0,
                )
                .map_err(|_| ())?;
                [
                    oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(x, 33704, 33718)
                        as i32,
                    oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(y, 32040, 32053)
                        as i32,
                    15,
                ]
            } else {
                [33644, 32757, 11]
            };
            let (x, y, floor) = context
                .source_frame()
                .physical_position(source_position)
                .ok_or(())?;
            self.memos.evict_stale_at(p.issuer);
            self.memos
                .prune(|actor| runtime.contains_live_creature(actor));
            if self.pending.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX
                || self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX
                    && self.memos.get(p.issuer, &p.ability).is_none()
            {
                return Err(());
            }
            self.pending.try_reserve(1).map_err(|_| ())?;
            self.pending.push(Pending {
                captured_at_us: now,
                proposal: p.clone(),
                sequence: work.sequence,
                parent: record.profile.target.clone(),
                destination: MovementLocalPosition { x, y, floor },
                context,
            });
        }
        while let Some(pending) = self.pending.first() {
            if pending.captured_at_us > eligible_through_us {
                break;
            }
            let p = &pending.proposal;
            if !runtime.matches_live_creature_identity(p.issuer, pending.parent.key.as_bytes()) {
                self.pending.remove(0);
                continue;
            }
            let snapshot = runtime.companion_snapshot(p.issuer).map_err(|_| ())?;
            if snapshot.state.policy.definition_revision != pending.parent.revision
                || !pending.context.matches(room, runtime.content_pin())
            {
                self.pending.remove(0);
                continue;
            }
            map.read(runtime, pending.destination).await;
            let exists = map
                .source_tile_exists_in_callback(runtime, pending.destination, &pending.context)
                .ok_or(())?;
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            let before = runtime.read_actor_position(p.issuer).map_err(|_| ())?;
            effect(room, before.position(), 3);
            if exists {
                map.view()
                    .relocate_source_creature(runtime, p.issuer, pending.destination)
                    .map_err(|_| ())?;
            }
            let after = runtime.read_actor_position(p.issuer).map_err(|_| ())?;
            effect(room, after.position(), 11);
            self.memos.insert(
                p.issuer,
                p.ability.clone(),
                Memo {
                    proposal: p.clone(),
                    sequence: pending.sequence,
                },
            );
            self.pending.remove(0);
        }
        Ok(())
    }
}
