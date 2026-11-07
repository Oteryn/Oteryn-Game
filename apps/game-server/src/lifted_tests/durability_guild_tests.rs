//! Actual native Root qualification with privileged historical fixtures. Guild snapshot rows
//! below are NOT a formation/Premium writer or an external membership authority. Every native
//! Character guard remains enabled, and current authority is independently reconciled afterwards.
#![allow(clippy::expect_used, clippy::unwrap_used)]
use crate::domain::CharacterId;
use crate::domain::guild::{
    DEFAULT_RANKS, FORMATION_LIFETIME_MS, GUILD_INVITATIONS_MAX, GuildId, GuildName, GuildRank,
    GuildState, MEMBERS_MAX, TARGET_INVITATIONS_MAX,
};
use crate::domain::premium::PremiumStatus;
use crate::durability::DurabilityRoot;
use crate::durability::guild::*;
use crate::durability::guild_audit::{
    GuildAuditProposal, GuildDeclineAuditProposal, GuildEditRanksAuditProposal,
    GuildRankAuditProposal, GuildResignAuditProposal, GuildTextAuditProposal,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::CommandRef;
use crate::foundation::{CommandId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use crate::item_transfer_postgres_cases::{
    self as fixture, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};
use sha2::{Digest, Sha256};
use sqlx::Executor;
use sqlx::Row;

use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use crate::premium::{PremiumConsumer, TrustedNow};
use std::sync::Arc;

#[path = "durability_guild_tests/support.rs"]
mod support;
use support::*;
#[path = "durability_guild_tests/chat_tests.rs"]
mod chat_tests;
#[path = "durability_guild_tests/deadline_tests.rs"]
mod deadline_tests;
#[path = "durability_guild_tests/decline_tests.rs"]
mod decline_tests;
#[path = "durability_guild_tests/disband_tests.rs"]
mod disband_tests;
#[path = "durability_guild_tests/edit_ranks_tests.rs"]
mod edit_ranks_tests;
#[path = "durability_guild_tests/found_tests.rs"]
mod found_tests;
#[path = "durability_guild_tests/membership_tests.rs"]
mod membership_tests;
#[path = "durability_guild_tests/rank_tests.rs"]
mod rank_tests;
#[path = "durability_guild_tests/resign_tests.rs"]
mod resign_tests;
#[path = "durability_guild_tests/text_tests.rs"]
mod text_tests;

#[path = "durability_guild_tests/world_job_tests.rs"]
mod world_job_tests;

#[path = "durability_guild_tests/terminal_core_tests.rs"]
mod terminal_core_tests;

#[path = "durability_guild_disband_terminal_tests.rs"]
mod terminal_tests;

#[path = "durability_guild_tests/event_failure.rs"]
mod event_failure;

#[path = "durability_guild_tests/disband_cleanup_tests.rs"]
mod disband_cleanup_tests;

#[path = "durability_guild_tests/event_guard_tests.rs"]
mod event_guard_tests;
