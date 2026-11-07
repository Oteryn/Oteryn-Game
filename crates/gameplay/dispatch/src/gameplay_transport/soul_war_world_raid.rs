//! Physical consumers of fixed source cues; no parallel map or creature store.
use super::{ComposedFreshAdmission, raid_live::RaidEffectProgress};
use crate::durability::{DurabilityError, raid_runner::ClaustrophobicRaidCue};
use crate::foundation::MovementLocalPosition;
use crate::map::boot::BundleCollisionIndexExt;
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_encounter_rules::raid::source_claustrophobic::{Effect, Raid, spec};
use oteryn_gameplay_entities::carrier::{NativeRaidSpawnSpec, RaidSpawnIdentity};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drive_claustrophobic_raid(
        &self,
        firing: [u8; 16],
        raid: Raid,
        now_ms: i64,
        progress: &mut RaidEffectProgress,
    ) -> Result<(), ()> {
        let generation = self.runtime.lock().await.binding().scope_generation();
        // One bounded source phase is <=94 cues. A delayed turn drains all
        // already-due warnings before its source-relative Advance callback.
        for _ in 0..128 {
            let Some(cue) = self
                .root
                .next_claustrophobic_raid_cue(
                    self.holder,
                    firing,
                    self.channel_id,
                    generation,
                    now_ms,
                )
                .await
                .map_err(|_| ())?
            else {
                break;
            };
            if cue.event.raid() != raid {
                return Err(());
            }
            if !progress.source_applied(firing, cue.identity.ordinal) {
                self.apply_claustrophobic_cue(&cue).await?;
                progress.mark_source_applied(firing, cue.identity.ordinal);
            }
            self.root
                .acknowledge_claustrophobic_raid_cue(self.holder, &cue, now_ms)
                .await
                .map_err(|_| ())?;
        }
        if self
            .root
            .claustrophobic_raid_complete(self.holder, firing, self.channel_id, generation)
            .await
            .map_err(|_| ())?
        {
            let mut runtime = self.runtime.lock().await;
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            runtime
                .finish_raid_run(stamp, firing, false)
                .map_err(|_| ())?;
            progress.completed_source(firing);
        }
        Ok(())
    }
    async fn apply_claustrophobic_cue(&self, cue: &ClaustrophobicRaidCue) -> Result<(), ()> {
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        let room = self.effective_rooms().gate_monster_ai().ok_or(())?;
        let active = self.active_generation.ok_or(())?;
        let native = active.native_gameplay().ok_or(())?;
        if !native.qualifies_soul_war_callbacks(runtime.content_pin().server_artifact_digest()) {
            return Err(());
        }
        let context = active
            .qualify_published_world_context(room, runtime.content_pin())
            .ok_or(())?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return Err(());
        };
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut work = (
            self,
            &mut *runtime,
            &permit,
            room,
            native,
            cue,
            &context,
            index,
        );
        pass.run_with_context(&mut work,move|holder,deadline,work|Box::pin(async move{
            use crate::durability::spell_item_transaction::{assert_spell_item_scope_in_transaction,begin_spell_owner_transaction};
            let(owner,runtime,permit,room,native,cue,context,index)=work;
            let mut tx=begin_spell_owner_transaction(holder,deadline).await?;
            let binding=runtime.binding();
            let authority=assert_spell_item_scope_in_transaction(&mut tx,permit,owner.root,owner.character,owner.holder,binding.runtime_scope(),binding.scope_generation().get()).await.map_err(|_|DurabilityError::Unavailable)?;
            if !crate::durability::raid_runner::claustrophobic_cue_pending_in_transaction(&mut tx,owner.holder,cue,owner.world_id,owner.reset_epoch,index.map().overlay.base().digest()).await?{return Err(DurabilityError::Unavailable)}
            let objects=owner.door.lock().await;
            let mut map=CurrentMonsterMap::new(&mut tx,&authority,runtime,room,&objects,native);
            let frame=context.source_frame();
            let physical=|p:oteryn_gameplay_encounter_rules::raid::source_claustrophobic::Position|frame.physical_position([p.x,p.y,i32::from(p.floor)]).map(|(x,y,floor)|MovementLocalPosition{x,y,floor});
            match cue.event.effect{
                Effect::Warning{at,..}=>{
                    let at=physical(at).ok_or(DurabilityError::Unavailable)?;
                    map.read(runtime,at).await;
                    if !map.validate_transaction(runtime).await{return Err(DurabilityError::Unavailable)}
                    let at=super::soul_war_scene::wire_position(at).ok_or(DurabilityError::Unavailable)?;
                    // Crystal encounters.lua204: CONST_ME_TELEPORT.
                    index.map().facts.emit_movement_effect(at,11);
                }
                Effect::Spawn{phase,entry,index:creature_index,species,at}=>{
                    let at=physical(at).ok_or(DurabilityError::Unavailable)?;
                    let key=species.native_key();
                    let policy=runtime.companion_policy(key).map_err(|_|DurabilityError::Unavailable)?;
                    let spawn=NativeRaidSpawnSpec::qualified(key,&policy.definition_revision,runtime.content_pin().server_artifact_digest(),false).map_err(|_|DurabilityError::Unavailable)?;
                    let identity=RaidSpawnIdentity{firing_id:cue.identity.firing_id,wave:u16::from(phase),entry,index:creature_index};
                    let retained=runtime.raid_creatures(cue.identity.firing_id).map_err(|_|DurabilityError::Unavailable)?.iter().any(|r|r.identity==identity);
                    let selected=if retained{None}else{
                        let mut ordinal=0u16;
                        let mut hash=Sha256::new();
                        hash.update(index.map().overlay.base().digest());
                        hash.update(cue.event.source_id().to_be_bytes());
                        let random=oteryn_gameplay_encounter_rules::raid::RaidRandom::new(hash.finalize().into(),cue.identity.firing_id);
                        // Game.createMonster(name,pos) uses default N8/forcefalse.
                        // Existing RaidRandom rejects modulo bias; each shuffle
                        // draw keeps the original source event and draw ordinal.
                        let plan=crate::movement::source_placement::SourcePlacementPlan::prepare(at,false,false,|bound|{
                            let current=ordinal;ordinal=ordinal.saturating_add(1);
                            bound.checked_sub(1).and_then(|max|u16::try_from(max).ok()).and_then(|max|random.amount(0,current,0,max).ok()).map(u32::from).unwrap_or(bound)
                        }).map_err(|_|DurabilityError::Unavailable)?;
                        for at in plan.candidates(){map.read(runtime,*at).await;}
                        plan.select(|at|map.source_placement_cell_in_world(runtime,key,at,context),|_,_|Some(true)).map_err(|_|DurabilityError::Unavailable)?
                    };
                    if !map.validate_transaction(runtime).await{return Err(DurabilityError::Unavailable)}
                    let stamp=runtime.issue_owner_work().map_err(|_|DurabilityError::Unavailable)?;
                    runtime.realize_raid_spawn(stamp,identity,&spawn,selected,|current,at|Some(map.view().matches(current)&&map.source_placement_cell_in_world(current,key,at,context).is_some_and(|cell|cell.query_add))).map_err(|_|DurabilityError::Unavailable)?;
                }
                Effect::Advance=>{}
                Effect::Reset|Effect::KickCurrentPlayers=>{
                    let record=spec(cue.event.raid());
                    let census=runtime.positioned_actor_census().map_err(|_|DurabilityError::Unavailable)?;
                    let mut states=owner.states.spell_states.lock().await;
                    let exit=physical(record.exit).ok_or(DurabilityError::Unavailable)?;
                    if cue.event.effect==Effect::KickCurrentPlayers{map.read(runtime,exit).await;}
                    if !map.validate_transaction(runtime).await{return Err(DurabilityError::Unavailable)}
                    for (actor,position,session) in census{
                        let at=position.position();
                        let Some([x,y,z])=frame.source_position((at.x,at.y,at.floor)) else{continue};
                        if !record.contains(oteryn_gameplay_encounter_rules::raid::source_claustrophobic::Position{x,y,floor:z as i16}){continue}
                        if cue.event.effect==Effect::Reset{
                            if runtime.contains_live_creature(actor){runtime.remove_encounter_actor(actor).map_err(|_|DurabilityError::Unavailable)?;}
                        }else if let Some(session)=session{
                            if states.has_pending_spell_commit(actor,session){return Err(DurabilityError::Unavailable)}
                            let next=states.get(runtime,actor,session).and_then(|p|p.stage_soul_war_torment_icon_removed());
                            map.view().relocate_source_player(runtime,actor,session,exit).map_err(|_|DurabilityError::Unavailable)?;
                            if let Some(next)=next && !states.commit(runtime,actor,session,next){return Err(DurabilityError::Unavailable)}
                        }
                    }
                }
            }
            drop(map);tx.rollback().await.map_err(|_|DurabilityError::Unavailable)?;Ok(())
        })).await.map_err(|_|())
    }
}
