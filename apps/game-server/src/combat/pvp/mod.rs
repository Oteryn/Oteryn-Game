//! PVP-RT-1 (pure half): the PARTY-PVP-0 §7.2 legality rules and the §8.1 combat lock.
//!
//! Contracts: `docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md`
//! (accepted) and, pending acceptance, `..._PVP0_PVP_GAP_CLOSURE_CONTRACT_CANDIDATE_2026-10-10.md`
//! (rules 3b and 6b, the 15-minute white skull).
//!
//! Runtime-only and not wired: PvP goes live on a World only when PVP-1, PVP-RT-1, PVP-DEATH-1,
//! PVP-WIRE-1, PVP-BLOCK-1 and PVP-CLIENT-1 have merged (PVP-0 §8), so `attackable_kind` stays
//! creature-only and nothing here changes a live decision. The durable write-ahead of the
//! deadlines needs the PVP-1 tables and is not part of this slice.

mod death;
mod legality;
mod lock;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use legality::{
    ExpertMode, PvpActor, PvpRefusal, PvpRelation, PvpTarget, WorldPvpType, check_pvp_legality,
    pvp_damage,
};
#[cfg(test)]
pub(crate) use lock::{AggressionOutcome, AggressionRelations, CombatLock};
