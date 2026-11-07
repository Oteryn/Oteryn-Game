//! Complete physical admission, native policy and AI registration in one replay memo.
use super::*;
use crate::ai_think::CreatureAiTable;
use crate::content::native_gameplay::NativeGameplayState;

impl NativeSummonOwner {
    #[allow(clippy::too_many_arguments)]
    pub fn execute_registered(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        catalog: &NativeSummonCatalog,
        proposal: &ProfileSummonProposal,
        policy: &mut dyn SummonLocationPolicy,
        native: &NativeGameplayState,
        table: &mut CreatureAiTable,
        now: u64,
    ) -> Result<SummonReceipt, SummonError> {
        // execute validates authority and replay before touching the physical owner. Finish the
        // same memo synchronously: no await, new writer or unregistered child can intervene.
        let receipt = self.execute_bound(runtime, None, stamp, catalog, proposal, policy)?;
        if !receipt.newly_created {
            return Ok(receipt);
        }
        let registration = (|| {
            let mut records = native
                .creature_profiles()
                .records
                .iter()
                .filter(|r| r.profile.target == proposal.creature);
            let record = records.next().ok_or(SummonError::UndefinedTarget)?;
            if records.next().is_some() || native.source_digest() != catalog.content {
                return Err(SummonError::InvalidSource);
            }
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            native
                .install_companion_policies(runtime)
                .map_err(|_| SummonError::InvalidSource)?;
            runtime
                .install_creature_policy(receipt.child, &proposal.creature.key)
                .map_err(SummonError::Carrier)?;
            table
                .admit_native_creature(runtime, native, receipt.child, &record.profile.target, now)
                .map_err(|_| SummonError::InvalidSource)?;
            Ok(())
        })();
        if let Err(error) = registration {
            runtime
                .retire_current_native_summon_registration_failure(stamp, receipt.child)
                .map_err(SummonError::Carrier)?;
            if let Some(memo) = self
                .memos
                .get(proposal.occurrence.actor, &proposal.entry_index)
            {
                let failed = SummonMemo {
                    sequence: memo.sequence,
                    binding: memo.binding,
                    result: Err(error),
                };
                self.memos
                    .insert(proposal.occurrence.actor, proposal.entry_index, failed);
            }
            return Err(error);
        }
        Ok(receipt)
    }
}

impl NativeSummonOwner {
    #[allow(clippy::too_many_arguments)]
    pub fn consume_report(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        table: &mut CreatureAiTable,
        native: &NativeGameplayState,
        actor: ExactActorRef,
        report: &crate::ai_think::ThinkReport,
        policy: &mut dyn SummonLocationPolicy,
        now: u64,
    ) {
        let Ok(catalog) = NativeSummonCatalog::for_actor(runtime, native, actor) else {
            return;
        };
        let Ok(stamp) = runtime.issue_owner_work() else {
            return;
        };
        let _ = runtime.retire_current_native_summons(stamp);
        for proposal in &report.summon_proposals {
            let _ = self.execute_registered(
                runtime, stamp, &catalog, proposal, policy, native, table, now,
            );
        }
        table.forget_dead(runtime);
    }
}
