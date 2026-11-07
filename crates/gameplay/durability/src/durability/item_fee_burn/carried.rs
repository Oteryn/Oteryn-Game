//! Dispatch only between admitted source-specific cash proofs; physical coins share one owner.
use super::*;
pub(super) enum Inputs {
    Atelier(atelier::CarriedInputs),
    Npc(npc_cash::Inputs),
}
pub(super) fn is_cause(cause: &FeeBurnCause) -> bool {
    atelier::is_cause(cause) || npc_cash::is_cause(cause)
}
pub(super) async fn lock_inputs<C: FeeChangeInput>(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &FeeBurnRequest<C>,
    world: WorldId,
) -> Result<Inputs> {
    if npc_cash::is_cause(&request.cause) {
        npc_cash::lock_inputs(connection, fence, request, world)
            .await
            .map(Inputs::Npc)
    } else {
        atelier::lock_inputs(connection, fence, request, world)
            .await
            .map(Inputs::Atelier)
    }
}
pub(super) fn plan(
    fee: u64,
    stacks: &[CoinStack],
    inputs: &Inputs,
    entries: usize,
    capacity: usize,
) -> Result<FeePlan> {
    super::carried_coins::plan(fee, stacks, inputs.physical(), entries, capacity)
}
impl Inputs {
    fn physical(&self) -> &carried_coins::PhysicalInputs {
        match self {
            Self::Atelier(v) => v,
            Self::Npc(v) => &v.physical,
        }
    }
    pub(super) fn inputs(&self) -> &[(CoinStack, [u8; 16], String)] {
        &self.physical().inputs
    }
    pub(super) fn audit_cause<C>(
        &self,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
    ) -> Result<FeeBurnCauseV1> {
        match self {
            Self::Atelier(v) => v.audit_cause(fence, request, plan),
            Self::Npc(v) => v.audit_cause(fence, request, plan),
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is one already-fenced physical payment witness"
    )]
    pub(super) async fn record<C>(
        &self,
        connection: &mut PgConnection,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
        backpack: Option<[u8; 16]>,
        capacity: usize,
        entries: &[u64],
    ) -> Result<()> {
        match self {
            Self::Atelier(v) => {
                v.record(
                    connection, fence, request, plan, backpack, capacity, entries,
                )
                .await
            }
            Self::Npc(v) => {
                v.record(
                    connection, fence, request, plan, backpack, capacity, entries,
                )
                .await
            }
        }
    }
    pub(super) async fn remove(
        &self,
        connection: &mut PgConnection,
        line: &BurnedCoinStack,
    ) -> Result<()> {
        self.physical().remove(connection, line).await
    }
    pub(super) async fn advance_equipment<C>(
        &self,
        connection: &mut PgConnection,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
    ) -> Result<()> {
        self.physical()
            .advance_equipment(connection, fence, request, plan)
            .await
    }
}
