//! Current Item revision resolution under the same spell lane as native preparation.
use super::super::actor_spell::{NativeCastDispatch, current_rune_revision};
use super::{ComposedFreshAdmission, LegacyRuneUse};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandRef, GameSessionId};
use oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn current_legacy_rune_revision(
        &self,
        permit: &SpellLanePermit,
        session: GameSessionId,
        command_id: u64,
        rune: &ItemSpellCastIntent,
        legacy: &LegacyRuneUse,
    ) -> Result<u64, NativeCastDispatch> {
        let Some(content) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            ));
        };
        let Some(room) = self.rooms().gate_native_combat_cast() else {
            return Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            ));
        };
        let Some(spell) = self.spells.indexed(rune.intent.spell) else {
            return Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            ));
        };
        use super::super::actor_spell::native_combat_cast::{
            corpse_companion_spell, retry_native_checkout,
        };
        let retry = corpse_companion_spell(spell)
            .then(|| super::super::AdmissionRetryBudget::for_command(self.root));
        let unavailable = || {
            if retry.is_some() {
                NativeCastDispatch::Outcome(super::super::actor_spell::SpellCastOutcome::rejected())
            } else {
                NativeCastDispatch::Pending
            }
        };
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let current = loop {
            let result = store.current_session_at(session).await;
            if !retry_native_checkout(retry.as_ref(), result.as_ref().err()).await {
                break result;
            }
        };
        let Ok((current, _)) = current else {
            return Err(unavailable());
        };
        let Some(fence) = super::super::item_fence_of(current) else {
            return Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            ));
        };
        if fence != legacy.fence {
            return Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            ));
        }
        let Ok(command) =
            crate::foundation::CommandId::new(command_id).map(|id| CommandRef::new(session, id))
        else {
            return Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            ));
        };
        let (runtime, pass) = loop {
            let runtime = self.runtime.lock().await;
            match self.root.try_issue_semantic_pass() {
                Ok(pass) => break (runtime, pass),
                Err(error) => {
                    drop(runtime);
                    if !retry_native_checkout(retry.as_ref(), Some(&error)).await {
                        return Err(unavailable());
                    }
                }
            }
        };
        let mut context = (self, permit, &runtime, content, room, spell);
        let revision = pass.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
                let (owner, permit, runtime, content, room, spell) = context;
                let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = crate::durability::spell_item_transaction::assert_spell_item_authority_in_transaction(
                    &mut tx, permit, owner.root, owner.character, owner.holder, &fence, command, content.source_digest(),
                ).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                current_rune_revision(&mut tx, &authority, runtime, room, content, spell, rune.rune_item_instance)
                    .await.map_err(|_| crate::durability::DurabilityError::Unavailable)
            })).await;
        match revision {
            Ok(revision) => Ok(revision),
            Err(_) => Err(NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            )),
        }
    }
}
