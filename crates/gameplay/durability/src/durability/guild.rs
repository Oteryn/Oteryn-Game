//! GUILD0 core. Commands are session fenced; World jobs independently prove recovery and assignment.
use crate::domain::CharacterId;
use crate::domain::guild::{
    DEFAULT_RANKS, FORMATION_LIFETIME_MS, GUILD_INVITATIONS_MAX, GuildId, GuildName, GuildRank,
    GuildState, INVITATION_LIFETIME_MS, MEMBERS_MAX, TARGET_INVITATIONS_MAX,
};
use crate::domain::premium::PremiumStatus;
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::{assert_session_fence, numeric_u64, uuid_text};
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::guild_audit::{
    GuildAuditProposal, GuildDeclineAuditProposal, GuildEditRanksAuditProposal,
    GuildRankAuditProposal, GuildResignAuditProposal, GuildTextAuditProposal,
};
use crate::durability::item_mint_audit::check_uuid_v7;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::premium_activation_latch::PremiumGateOwner;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_social_rules::guild::{GuildRankRules, GuildStateRules};
use sha2::{Digest, Sha256};
use sqlx::{Acquire, Postgres, Row, Transaction};

mod invitee;
mod query;
mod request;
pub use request::*;
mod receipt;
pub use receipt::*;
mod found;
mod membership;
mod rank;
mod text;
pub use rank::*;
mod resign;
pub use resign::*;
mod edit_ranks;
pub use edit_ranks::*;
mod chat_membership;
pub use chat_membership::*;
mod operation;
pub use operation::*;
mod disband_mutation;
mod edit_ranks_mutation;
mod journal;
mod membership_mutation;
mod rank_mutation;
mod resign_mutation;
mod text_mutation;
use journal::*;
mod deadlines;
pub use deadlines::*;
mod disband;
mod disband_cleanup;
pub use disband::*;
mod disband_transaction;
pub use disband_transaction::*;

type Result<T> = std::result::Result<T, GuildError>;
