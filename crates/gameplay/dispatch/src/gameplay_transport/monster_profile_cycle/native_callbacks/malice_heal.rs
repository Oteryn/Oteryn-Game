//! Soul Cage's selected native defense: CombatNull dispel, then named healing.
use super::*;
use crate::content::creature_attack_geometry::{self as geometry, Facing};
use crate::creature_area_heal::{AreaHealSource, CreatureHealCatalog};
use crate::foundation::{ConditionType, GameSessionId, MovementFacing};
use oteryn_simulation_determinism::SemanticTimeMicros;

#[derive(Debug)]
struct Work {
    proposal: ProfileAbilityProposal,
    sequence: u64,
    cures: Vec<(ExactActorRef, Option<GameSessionId>)>,
    next_cure: usize,
    heals: Vec<(ExactActorRef, String, u64, u64)>,
    tiles: Vec<MovementLocalPosition>,
    effect: u16,
}

#[derive(Debug, Default)]
pub(super) struct NativeMaliceHeal {
    memos: ActorMemoTable<Ref, Memo>,
    pending: Option<Work>,
}

impl NativeMaliceHeal {
    #[allow(
        clippy::too_many_arguments,
        reason = "the selected defense retains its current map, native source and player condition owners"
    )]
    pub(super) async fn execute(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        now: u64,
    ) -> Result<(), ()> {
        if self
            .pending
            .as_ref()
            .is_some_and(|work| !runtime.contains_live_creature(work.proposal.issuer))
        {
            self.pending = None;
        }
        use crate::map::boot::BundleCollisionIndexExt;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return if proposals
                .iter()
                .any(|p| p.ability.key == "oteryn:ability.spell.heal_malice")
            {
                Err(())
            } else {
                Ok(())
            };
        };
        for p in proposals {
            if p.list != crate::ai_think::profile_schedule::ScheduleList::Defence
                || p.ability.key != "oteryn:ability.spell.heal_malice"
                || !runtime.matches_live_creature_identity(p.issuer, b"oteryn:creature.soul_cage")
            {
                continue;
            }
            oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(
                runtime, p,
            )
            .map_err(|_| ())?;
            let prefix = format!(
                "ai-profile:{}:",
                p.issuer
                    .placement_identity()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let (sequence, tail) = p
                .occurrence
                .id()
                .as_str()
                .strip_prefix(&prefix)
                .and_then(|id| id.split_once(':'))
                .ok_or(())?;
            let sequence = sequence.parse::<u64>().map_err(|_| ())?;
            if tail != format!("defence:{}", p.entry_index)
                || p.range_tiles != 0
                || p.magnitude.is_some()
            {
                return Err(());
            }
            if self
                .memos
                .get(p.issuer, &p.ability)
                .is_some_and(|memo| memo.sequence >= sequence)
            {
                continue;
            }
            if self.pending.is_none() {
                self.memos.evict_stale_at(p.issuer);
                self.memos
                    .prune(|actor| runtime.contains_live_creature(actor));
                if self.memos.get(p.issuer, &p.ability).is_none()
                    && self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX
                {
                    return Err(());
                }
                self.pending =
                    Some(prepare(runtime, states, room, native, map, p, sequence).await?);
            }
            let work = self.pending.as_mut().ok_or(())?;
            if work.proposal != *p {
                return Err(());
            }
            // No await occurs between each canonical commit and its retained
            // cursor. A later refused target cannot repeat an earlier cure.
            while let Some(&(actor, session)) = work.cures.get(work.next_cure) {
                if let Some(session) = session {
                    // Player spell conditions belong to PlayerSpellState;
                    // the carrier condition slot is the creature projection.
                    let before = states.get(runtime, actor, session).ok_or(())?;
                    if !before.owned_conditions().accepts_time(now) {
                        return Err(());
                    }
                    let mut next = before.clone();
                    if crate::spell::actor_conditions::remove_condition(&mut next, "paralyze")
                        .map_err(|_| ())?
                    {
                        next.source_set_batch_revision(before.revision().checked_add(1).ok_or(())?);
                        if !states.commit(runtime, actor, session, next) {
                            return Err(());
                        }
                    }
                    work.next_cure += 1;
                    continue;
                }
                let digest = Sha256::new()
                    .chain_update(b"oteryn:heal-malice-dispel:v1")
                    .chain_update(p.occurrence.id().as_str().as_bytes())
                    .chain_update(actor.placement_identity())
                    .finalize();
                let mut id = [0; 16];
                id.copy_from_slice(&digest[..16]);
                let at = SemanticTimeMicros::from_micros(now);
                let plan = runtime
                    .prepare_actor_condition_removal(
                        actor,
                        session,
                        ConditionType::Paralysis,
                        DecisionOccurrenceId::from_bytes(id),
                        at,
                    )
                    .map_err(|_| ())?;
                runtime.commit_actor_condition(&plan, at).map_err(|_| ())?;
                work.next_cure += 1;
            }
            runtime
                .commit_source_creature_heal_batch(native.source_digest(), &work.heals)
                .map_err(|_| ())?;
            for &at in &work.tiles {
                index
                    .map()
                    .facts
                    .emit_movement_effect(visual_position(at).ok_or(())?, work.effect);
            }
            let work = self.pending.take().ok_or(())?;
            self.memos.insert(
                p.issuer,
                p.ability.clone(),
                Memo {
                    proposal: work.proposal,
                    sequence: work.sequence,
                },
            );
        }
        Ok(())
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "all source cells and physical recipients are read before the retained defense commits"
)]
async fn prepare(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    p: &ProfileAbilityProposal,
    sequence: u64,
) -> Result<Work, ()> {
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| {
            runtime.matches_live_creature_identity(p.issuer, record.profile.target.key.as_bytes())
        })
        .ok_or(())?;
    // This existing qualifier binds the actual Creature/defense/Ability/Heal
    // Effect/Formula and named-target filter, including the paralysis dispel.
    let source =
        AreaHealSource::from_current_native(runtime, native, &record.profile.target, p.entry_index)
            .map_err(|_| ())?;
    if source.ability != p.ability || !source.requires_paralysis_removal() {
        return Err(());
    }
    let filter = source.filter.as_ref().ok_or(())?;
    if filter.kind != crate::content::ProjectV2AffectsKind::NamedCreatures {
        return Err(());
    }
    let definitions = record.attack_definitions.as_ref().ok_or(())?;
    let details = definitions
        .profiles
        .iter()
        .find_map(|profile| {
            if profile.target != p.ability {
                return None;
            }
            let crate::content::ProjectV2AuthoringProfileData::Ability(ability) = &profile.data
            else {
                return None;
            };
            ability.details.as_deref()
        })
        .ok_or(())?;
    let area = details.area.as_ref().ok_or(())?;
    let from = runtime
        .read_actor_position(p.issuer)
        .map_err(|_| ())?
        .position();
    let facing = match runtime
        .read_actor_position(p.issuer)
        .map_err(|_| ())?
        .facing()
        .ok_or(())?
    {
        MovementFacing::North => Facing::North,
        MovementFacing::East => Facing::East,
        MovementFacing::South => Facing::South,
        MovementFacing::West => Facing::West,
    };
    let (dx, dy) = if details.needs_direction {
        geometry::step(facing)
    } else {
        (0, 0)
    };
    let center = MovementLocalPosition {
        x: from.x.checked_add(dx).ok_or(())?,
        y: from.y.checked_add(dy).ok_or(())?,
        floor: from.floor,
    };
    let sight = super::super::physical_spell::area_sight_origin(from, center).ok_or(())?;
    let mut tiles = Vec::new();
    for (dx, dy) in geometry::offsets(area, facing).map_err(|_| ())? {
        let at = MovementLocalPosition {
            x: center.x.checked_add(dx).ok_or(())?,
            y: center.y.checked_add(dy).ok_or(())?,
            floor: center.floor,
        };
        if room
            .movement_cells()
            .spell_tiles()
            .lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            )
            .is_ok_and(|tile| {
                tile.flags().block_projectile
                    || tile.flags().block_solid
                    || tile.flags().protection_zone
                    || tile.flags().floor_change
            })
        {
            continue;
        }
        if map.read(runtime, at).await.is_none() {
            if map.source_tile_exists(runtime, at) == Some(false) {
                continue;
            }
            return Err(());
        }
        if map.view().source_combat_tile_admits(at) != Some(true) {
            continue;
        }
        map.read_sight(runtime, sight, at).await;
        if !map.view().sight(sight, at) {
            continue;
        }
        visual_position(at).ok_or(())?;
        tiles.push(at);
    }
    if !map.validate_transaction(runtime).await {
        return Err(());
    }
    let catalog = CreatureHealCatalog::from_current_native(runtime, native).map_err(|_| ())?;
    let mut cures = Vec::new();
    let mut heals = Vec::new();
    for (actor, position, session) in runtime.positioned_actor_census().map_err(|_| ())? {
        if !tiles.contains(&position.position()) {
            continue;
        }
        if let Some(session) = session {
            if states.has_pending_spell_commit(actor, session) {
                return Err(());
            }
            if states
                .get(runtime, actor, session)
                .is_none_or(|state| state.vitals().health == 0)
            {
                continue;
            }
        } else if !runtime.contains_live_creature(actor) {
            continue;
        }
        runtime
            .assert_actor_spell_unreserved(actor)
            .map_err(|_| ())?;
        cures.push((actor, session));
        if session.is_some() {
            continue;
        }
        let (key, maximum) = catalog.current_target(runtime, actor).map_err(|_| ())?;
        if actor == p.issuer && !filter.includes_caster
            || filter.excludes_caster_name && key == record.profile.target.key
            || !filter.creatures.iter().any(|creature| creature.key == key)
            || filter.top_creature_only
                && runtime
                    .current_top_actor_on_cell(position.position())
                    .map_err(|_| ())?
                    != Some(actor)
        {
            continue;
        }
        let hash = Sha256::new()
            .chain_update(b"oteryn:heal-malice-target:v1")
            .chain_update(p.occurrence.id().as_str().as_bytes())
            .chain_update(actor.placement_identity())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        let draw = deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(native.source_digest()),
            DecisionOccurrenceId::from_bytes(id),
            "source_heal_magnitude",
            p.entry_index as u64,
        )
        .map_err(|_| ())?;
        let amount = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
            draw,
            i64::try_from(source.minimum).map_err(|_| ())?,
            i64::try_from(source.maximum).map_err(|_| ())?,
        ) as u64;
        heals.push((actor, key, maximum, amount));
    }
    if heals.len() > 64 {
        return Err(());
    }
    let effect = details
        .effects
        .iter()
        .find_map(|effect| {
            let crate::content::ProjectV2AbilityEffect::Inline(effect) = effect else {
                return None;
            };
            effect
                .presentation
                .as_ref()?
                .impact_asset_binding
                .as_deref()
        })
        .and_then(crate::spell::spell_presentations::resolve_source_cue)
        .ok_or(())?;
    let crate::spell::spell_presentations::Cue::Effect(effect) = effect else {
        return Err(());
    };
    Ok(Work {
        proposal: p.clone(),
        sequence,
        cures,
        next_cure: 0,
        heals,
        tiles,
        effect,
    })
}
