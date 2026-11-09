//! The actual delayed Eruption Lua event heals its current master, attempts a
//! forced Demon birth, then executes Combat. Retained data share its original
//! cast-stage identity and timer; no second clock or HP owner is introduced.
use super::*;
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::{
    CallbackStageId, CallbackStagePlan,
};

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct EruptionMasterReceipt {
    #[cfg(test)]
    pub master: crate::foundation::ExactActorRef,
    #[cfg(test)]
    pub requested: u32,
    #[cfg(test)]
    pub health_before: i64,
    #[cfg(test)]
    pub health_after: i64,
    pub demon: Option<crate::foundation::ExactActorRef>,
}
#[derive(Debug)]
struct Pending {
    id: CallbackStageId,
    #[cfg(test)]
    caster: crate::foundation::ExactActorRef,
    at: MovementLocalPosition,
    receipt: EruptionMasterReceipt,
    attempted: bool,
    registered: bool,
    complete: bool,
}
#[derive(Debug, Default)]
pub(super) struct EruptionPreludes(Vec<Pending>);
impl EruptionPreludes {
    pub(super) fn completed(&self, id: CallbackStageId) -> bool {
        self.0.iter().any(|entry| entry.id == id && entry.complete)
    }
    #[cfg(test)]
    pub(super) fn receipt(
        &self,
        caster: crate::foundation::ExactActorRef,
    ) -> Option<EruptionMasterReceipt> {
        self.0
            .iter()
            .rev()
            .find(|entry| entry.caster == caster && entry.complete)
            .map(|entry| entry.receipt.clone())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one source event shares existing current physical owners"
    )]
    pub(super) async fn prepare(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut crate::ai_think::CreatureAiTable,
        plan: &CallbackStagePlan,
        stamp: crate::foundation::RuntimeWorkStamp,
        now: u64,
    ) -> Result<bool, ()> {
        if plan.source().creature_key != "oteryn:creature.eruption_of_destruction" {
            return Ok(true);
        }
        let caster = plan.proposal().issuer;
        if !self.0.iter().any(|entry| entry.id == plan.id()) {
            let master = current_source_creature_role(runtime, caster)
                .map_err(|_| ())?
                .intrinsic_master();
            let Some(master) = master.filter(|master| runtime.contains_live_creature(*master))
            else {
                // The ordinary stage consumer records the literal nil-master abort.
                return Ok(true);
            };
            if self.0.len() >= 128 {
                let index = self.0.iter().position(|entry| entry.complete).ok_or(())?;
                self.0.remove(index);
            }
            self.0.try_reserve(1).map_err(|_| ())?;
            let at = plan.anchor_position(runtime).map_err(|_| ())?.ok_or(())?;
            let digest = Sha256::new()
                .chain_update(b"oteryn:eruption-master-heal:v1")
                .chain_update(plan.proposal().occurrence.id().as_str().as_bytes())
                .finalize();
            let mut id = [0; 16];
            id.copy_from_slice(&digest[..16]);
            let draw = deterministic_decision_u64(
                &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                    native.source_digest(),
                ),
                DecisionOccurrenceId::from_bytes(id),
                "lua_math_random",
                0,
            )
            .map_err(|_| ())?;
            let requested = u32::try_from(
                oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw, 20_000, 30_000),
            )
            .map_err(|_| ())?;
            let snapshot = runtime.companion_snapshot(master).map_err(|_| ())?;
            let _result = runtime
                .commit_source_creature_heal_batch(
                    native.source_digest(),
                    &[(
                        master,
                        snapshot.state.policy.definition_key.clone(),
                        u64::try_from(snapshot.maximum_health).map_err(|_| ())?,
                        u64::from(requested),
                    )],
                )
                .map_err(|_| ())?
                .into_iter()
                .next()
                .ok_or(())?;
            // No await separates the canonical heal from retaining its receipt.
            self.0.push(Pending {
                id: plan.id(),
                #[cfg(test)]
                caster,
                at,
                receipt: EruptionMasterReceipt {
                    #[cfg(test)]
                    master,
                    #[cfg(test)]
                    requested,
                    #[cfg(test)]
                    health_before: _result.health_before,
                    #[cfg(test)]
                    health_after: _result.health_after,
                    demon: None,
                },
                attempted: false,
                registered: false,
                complete: false,
            });
        }
        let entry = self
            .0
            .iter_mut()
            .find(|entry| entry.id == plan.id())
            .ok_or(())?;
        if entry.complete {
            return Ok(true);
        }
        let parent = plan.native_stage().ok_or(())?.parent();
        let child = &native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == "oteryn:creature.demon")
            .ok_or(())?
            .profile
            .target;
        if !entry.attempted {
            map.read(runtime, entry.at).await;
            if !map.view().known(entry.at) {
                return Ok(false);
            }
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            let token = native
                .callback_spawn_source(
                    &parent.entry().creature,
                    parent.entry().index,
                    crate::content::native_monster_callbacks::NativeCallbackList::Attack,
                    child,
                )
                .map_err(|_| ())?
                .ok_or(())?;
            if !token.extended() || !token.forced() {
                return Err(());
            }
            let spec = oteryn_gameplay_entities::NativeEncounterSpawnSpec::qualified(
                &token.parent().key,
                &token.child().key,
                &token.child().revision,
                token.source_digest(),
            )
            .map_err(|_| ())?;
            match runtime.admit_current_native_callback_masterless(
                native,
                stamp,
                caster,
                &spec,
                &token,
                entry.at,
                entry.at,
                |_, _| Some(true),
            ) {
                Ok(actor) => entry.receipt.demon = Some(actor),
                Err(oteryn_gameplay_entities::EncounterSpawnError::MissingMap) => return Ok(false),
                Err(oteryn_gameplay_entities::EncounterSpawnError::StaleOwner) => return Err(()),
                // A known failed Game.createMonster does not abort the following Combat.
                Err(_) => {}
            }
            entry.attempted = true;
        }
        if !entry.registered {
            if let Some(actor) = entry.receipt.demon {
                ai.admit_native_creature(runtime, native, actor, child, now)
                    .map_err(|_| ())?;
            }
            entry.registered = true;
        }
        entry.complete = true;
        Ok(true)
    }
}
