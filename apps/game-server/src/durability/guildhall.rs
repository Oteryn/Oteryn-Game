//! GUILDHALL-1 private, uninstalled candidate storage. No runtime registration or schema install.
//! Financial writers require the independently qualified Guild bank participant and native
//! house/auction owner; the bid below is custody data, never current guild/leader authority.

/// The single proposed HELD escrow source consumed by GUILD0-RL-19/RL-20 headroom guards.
/// Apply only after the independently proposed Guild prerequisite; do not install on main.
pub const BID_SOURCE_SCHEMA: &str = r#"
CREATE TABLE game_guildhall_bids (
 bid_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(bid_id)),
 guild_id UUID NOT NULL REFERENCES game_guilds(guild_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 auction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(auction_id)),
 funding_account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 funding_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 max_gold BIGINT NOT NULL CHECK(max_gold>=0),
 rent_gold BIGINT NOT NULL CHECK(rent_gold>=0),
 escrow_guild_gold BIGINT NOT NULL CHECK(escrow_guild_gold BETWEEN 0 AND 999999999999),
 escrow_account_gold BIGINT NOT NULL CHECK(escrow_account_gold BETWEEN 0 AND 9000000000000000),
 state TEXT NOT NULL CHECK(state IN('HELD','WON','RELEASED')),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 last_operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(last_operation_id)),
 CHECK((state='HELD' AND escrow_guild_gold::numeric+escrow_account_gold::numeric=max_gold::numeric+rent_gold::numeric)
    OR(state<>'HELD' AND escrow_guild_gold=0 AND escrow_account_gold=0))
);
CREATE UNIQUE INDEX game_guildhall_one_held_bid ON game_guildhall_bids(guild_id) WHERE state='HELD';
CREATE FUNCTION game_guildhall_bid_binding() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id)
 OR NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.funding_character_id
    AND c.account_id=NEW.funding_account_id AND c.world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'guildhall bid has inconsistent World/funding subject' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' AND NEW.revision<>1 THEN
  RAISE EXCEPTION 'guildhall bid requires initial revision' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND (OLD.state<>'HELD' OR NEW.revision<>OLD.revision+1
 OR (NEW.bid_id,NEW.guild_id,NEW.world_id,NEW.house_key,NEW.auction_id,NEW.funding_account_id,NEW.funding_character_id,NEW.rent_gold)
 IS DISTINCT FROM (OLD.bid_id,OLD.guild_id,OLD.world_id,OLD.house_key,OLD.auction_id,OLD.funding_account_id,OLD.funding_character_id,OLD.rent_gold)
 OR NEW.last_operation_id=OLD.last_operation_id) THEN
  RAISE EXCEPTION 'guildhall bid requires exact immutable-source successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guildhall_bid_binding BEFORE INSERT OR UPDATE ON game_guildhall_bids
 FOR EACH ROW EXECUTE FUNCTION game_guildhall_bid_binding();
REVOKE ALL ON game_guildhall_bids FROM PUBLIC;
"#;

/// Private financial participant journal. This is not the shared House command/event journal.
/// Entry identities are fixed before retry; NULL means that source has no value change.
pub const OPERATION_SCHEMA: &str = r#"
CREATE TABLE game_guildhall_operations (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 occurrence_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(occurrence_id)),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 kind TEXT NOT NULL CHECK(kind IN('BID_RESERVE','BID_LOWER','BID_RELEASE')),
 bid_id UUID NOT NULL REFERENCES game_guildhall_bids(bid_id) DEFERRABLE INITIALLY DEFERRED,
 guild_id UUID NOT NULL REFERENCES game_guilds(guild_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 auction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(auction_id)),
 funding_account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 funding_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 guild_balance_before BIGINT NOT NULL CHECK(guild_balance_before BETWEEN 0 AND 999999999999),
 guild_balance_after BIGINT NOT NULL CHECK(guild_balance_after BETWEEN 0 AND 999999999999),
 account_balance_before BIGINT NOT NULL CHECK(account_balance_before BETWEEN 0 AND 9000000000000000),
 account_balance_after BIGINT NOT NULL CHECK(account_balance_after BETWEEN 0 AND 9000000000000000),
 escrow_guild_before BIGINT NOT NULL CHECK(escrow_guild_before BETWEEN 0 AND 999999999999),
 escrow_guild_after BIGINT NOT NULL CHECK(escrow_guild_after BETWEEN 0 AND 999999999999),
 escrow_account_before BIGINT NOT NULL CHECK(escrow_account_before BETWEEN 0 AND 9000000000000000),
 escrow_account_after BIGINT NOT NULL CHECK(escrow_account_after BETWEEN 0 AND 9000000000000000),
 guild_entry_id UUID NULL CHECK(guild_entry_id IS NULL OR game_character_is_uuid_v7(guild_entry_id)),
 account_entry_id UUID NULL CHECK(account_entry_id IS NULL OR game_character_is_uuid_v7(account_entry_id)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(guild_balance_before::numeric+escrow_guild_before::numeric=guild_balance_after::numeric+escrow_guild_after::numeric),
 CHECK(account_balance_before::numeric+escrow_account_before::numeric=account_balance_after::numeric+escrow_account_after::numeric),
 CHECK((kind='BID_RESERVE' AND escrow_guild_after>=escrow_guild_before AND escrow_account_after>=escrow_account_before
    AND escrow_guild_after::numeric+escrow_account_after::numeric>escrow_guild_before::numeric+escrow_account_before::numeric)
 OR(kind IN('BID_LOWER','BID_RELEASE') AND escrow_guild_after<=escrow_guild_before AND escrow_account_after<=escrow_account_before
    AND(escrow_guild_after::numeric+escrow_account_after::numeric<escrow_guild_before::numeric+escrow_account_before::numeric
    OR(kind='BID_RELEASE' AND escrow_guild_before=0 AND escrow_account_before=0)))),
 CHECK(kind<>'BID_RELEASE' OR(escrow_guild_after=0 AND escrow_account_after=0))
);
REVOKE ALL ON game_guildhall_operations FROM PUBLIC;
"#;

/// Apply after the exact Guild bank proposal and its Hall ledger extension. No runtime install.
/// These guards prove custody/ledger consistency, not current leader or an auction decision.
pub const HALL_GUARDS_SCHEMA: &str = r#"
CREATE UNIQUE INDEX game_guildhall_one_change_per_transaction ON game_guildhall_operations(bid_id,created_xact_id);
CREATE FUNCTION game_guildhall_custody_successor() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guildhall_operations;
BEGIN
 SELECT * INTO o FROM game_guildhall_operations WHERE transaction_id=NEW.last_operation_id;
 IF NOT FOUND OR o.created_xact_id<>pg_current_xact_id()
 OR(o.bid_id,o.guild_id,o.world_id,o.house_key,o.auction_id,o.funding_account_id,o.funding_character_id)
 IS DISTINCT FROM(NEW.bid_id,NEW.guild_id,NEW.world_id,NEW.house_key,NEW.auction_id,NEW.funding_account_id,NEW.funding_character_id)
 OR(o.escrow_guild_after,o.escrow_account_after) IS DISTINCT FROM(NEW.escrow_guild_gold,NEW.escrow_account_gold)
 OR NEW.state<>(CASE WHEN o.kind='BID_RELEASE' THEN 'RELEASED' ELSE 'HELD' END) THEN
 RAISE EXCEPTION 'bid needs exact same-transaction custody operation' USING ERRCODE='23514';END IF;
 IF TG_OP='INSERT' THEN
 IF o.kind<>'BID_RESERVE' OR o.escrow_guild_before<>0 OR o.escrow_account_before<>0 THEN
 RAISE EXCEPTION 'initial escrow must reserve from zero' USING ERRCODE='23514';END IF;
 ELSE
 IF(o.escrow_guild_before,o.escrow_account_before) IS DISTINCT FROM(OLD.escrow_guild_gold,OLD.escrow_account_gold)
 OR(o.kind='BID_RESERVE' AND NEW.max_gold<=OLD.max_gold)
 OR(o.kind='BID_LOWER' AND NEW.max_gold>=OLD.max_gold) THEN
 RAISE EXCEPTION 'bid needs its exact escrow predecessor' USING ERRCODE='23514';END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guildhall_custody_successor BEFORE INSERT OR UPDATE ON game_guildhall_bids
 FOR EACH ROW EXECUTE FUNCTION game_guildhall_custody_successor();
CREATE FUNCTION game_guildhall_operation_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
 OR(NEW.guild_entry_id IS NULL)<>(NEW.guild_balance_before=NEW.guild_balance_after)
 OR(NEW.account_entry_id IS NULL)<>(NEW.account_balance_before=NEW.account_balance_after)
 OR NOT EXISTS(SELECT 1 FROM game_guildhall_bids b WHERE b.bid_id=NEW.bid_id AND b.last_operation_id=NEW.transaction_id
 AND(b.guild_id,b.world_id,b.house_key,b.auction_id,b.funding_account_id,b.funding_character_id,b.escrow_guild_gold,b.escrow_account_gold)
 IS NOT DISTINCT FROM(NEW.guild_id,NEW.world_id,NEW.house_key,NEW.auction_id,NEW.funding_account_id,NEW.funding_character_id,NEW.escrow_guild_after,NEW.escrow_account_after))
 OR(NEW.guild_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guild_bank_entries e WHERE e.entry_id=NEW.guild_entry_id AND e.guildhall_transaction_id=NEW.transaction_id AND e.guild_id=NEW.guild_id AND e.balance_before=NEW.guild_balance_before AND e.balance_after=NEW.guild_balance_after AND e.created_xact_id=pg_current_xact_id()))
 OR(NEW.account_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.entry_id=NEW.account_entry_id AND e.guildhall_transaction_id=NEW.transaction_id AND e.account_id=NEW.funding_account_id AND e.world_id=NEW.world_id AND e.balance_before=NEW.account_balance_before AND e.balance_after=NEW.account_balance_after AND e.created_xact_id=pg_current_xact_id())) THEN
 RAISE EXCEPTION 'guildhall operation requires conserved custody and both exact ledger participants' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guildhall_operation_complete AFTER INSERT ON game_guildhall_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guildhall_operation_complete();
CREATE TRIGGER game_guildhall_operation_immutable BEFORE UPDATE OR DELETE ON game_guildhall_operations
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
CREATE TRIGGER game_guildhall_bid_retained BEFORE DELETE ON game_guildhall_bids
 FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable();
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_guildhall_bids','game_guildhall_operations'] LOOP
 EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',t||'_no_truncate',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_guildhall_bid_binding','game_guildhall_custody_successor','game_guildhall_operation_complete'] LOOP
 EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',t,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',t);
 END LOOP;
END $$;
GRANT SELECT,INSERT ON game_guildhall_operations TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_guildhall_bids TO oteryn_game_runtime;
GRANT SELECT ON game_guildhall_operations,game_guildhall_bids TO oteryn_game_control;
"#;

/// Minimal proposed read contract of the absent House auction owner. This deliberately has
/// no catalogue/admission/settlement writer. Isolated fixture rows are DATA, not production
/// authority; CP must bind the actual auction owner's decision before registering this module.
pub const AUCTION_PREREQUISITE_SCHEMA_PROPOSAL: &str = r#"
CREATE TABLE game_house_auctions(
 auction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(auction_id)),
 world_id UUID NOT NULL,house_key TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind='GUILDHALL'),
 rent_gold BIGINT NOT NULL CHECK(rent_gold>=0),
 current_price_gold BIGINT NOT NULL CHECK(current_price_gold>=0),
 ends_at TIMESTAMPTZ NOT NULL,
 phase TEXT NOT NULL CHECK(phase IN('OPEN','RELEASING')),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key));
CREATE UNIQUE INDEX game_house_one_auction ON game_house_auctions(world_id,house_key);
CREATE TABLE game_guild_house_bans(
 guild_id UUID PRIMARY KEY REFERENCES game_guilds(guild_id),banned_until TIMESTAMPTZ NOT NULL);
REVOKE ALL ON game_house_auctions,game_guild_house_bans FROM PUBLIC;
GRANT SELECT ON game_house_properties,game_house_auctions,game_guild_house_bans TO oteryn_game_runtime;
GRANT UPDATE(world_id) ON game_house_properties TO oteryn_game_runtime;
GRANT UPDATE(auction_id) ON game_house_auctions TO oteryn_game_runtime;
GRANT UPDATE(guild_id) ON game_guild_house_bans TO oteryn_game_runtime;
CREATE FUNCTION game_guildhall_context_read_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN IF current_user='oteryn_game_runtime' THEN RAISE EXCEPTION 'auction context is read/lock only' USING ERRCODE='42501';END IF;RETURN NEW;END $$;
CREATE TRIGGER game_guildhall_property_world_read_only BEFORE UPDATE OF world_id ON game_house_properties FOR EACH ROW EXECUTE FUNCTION game_guildhall_context_read_only();
CREATE TRIGGER game_guildhall_auction_context_read_only BEFORE UPDATE ON game_house_auctions FOR EACH ROW EXECUTE FUNCTION game_guildhall_context_read_only();
CREATE TRIGGER game_guildhall_ban_context_read_only BEFORE UPDATE ON game_guild_house_bans FOR EACH ROW EXECUTE FUNCTION game_guildhall_context_read_only();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_guildhall_context_read_only() SET search_path=%I,pg_temp',current_schema());END $$;
REVOKE ALL ON FUNCTION game_guildhall_context_read_only() FROM PUBLIC;
"#;

use super::bank::fee_payer_is_junior;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_item_only_fence, numeric_u64, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::guild_bank::{GuildBankError, apply_guildhall_escrow_in_transaction};
use super::house_scope_handoff::HouseId;
use super::item_mint_audit::check_uuid_v7;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::RuntimeScopeRefV1;
use crate::guild::guildhall::{EscrowParts, SplitError};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, GuildhallEscrowError>;

#[derive(Debug)]
pub enum GuildhallEscrowError {
    InvalidInput,
    NotCurrent,
    NotLeader,
    GuildIneligible,
    AuctionContextUnavailable,
    StaleBid,
    FundingBindingChanged,
    AlreadyReleased,
    ConflictingOccurrence,
    Split(SplitError),
    BankParticipant(GuildBankError),
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for GuildhallEscrowError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for GuildhallEscrowError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl From<SplitError> for GuildhallEscrowError {
    fn from(error: SplitError) -> Self {
        Self::Split(error)
    }
}
impl std::fmt::Display for GuildhallEscrowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "guildhall escrow: {self:?}")
    }
}
impl std::error::Error for GuildhallEscrowError {}

/// All identities fixed before first attempt. These are identifiers, never admission proofs.
#[derive(Clone, Debug)]
pub struct GuildhallEscrowIdentity {
    pub occurrence_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub guild_entry_id: [u8; 16],
    pub account_entry_id: [u8; 16],
}
#[derive(Clone, Debug)]
pub struct GuildhallEscrowStep {
    pub identity: GuildhallEscrowIdentity,
    pub bid_id: [u8; 16],
    pub guild_id: [u8; 16],
    pub house: HouseId,
    pub auction_id: [u8; 16],
    /// None only for a new custody row; a successor must name its exact revision.
    pub expected_bid_revision: Option<u64>,
    /// Some(max) resizes custody; None releases all custody for a World job.
    pub max_gold: Option<u64>,
}
/// A financial outcome only: no auction admission, owner grant, settlement or event claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildhallEscrowReceipt {
    pub transaction_id: [u8; 16],
    pub bid_id: [u8; 16],
    pub parts: EscrowParts,
    pub replayed: bool,
}
impl GuildhallEscrowStep {
    fn validate(&self) -> Result<()> {
        let ids = [
            self.identity.occurrence_id,
            self.identity.transaction_id,
            self.identity.guild_entry_id,
            self.identity.account_entry_id,
            self.bid_id,
            self.guild_id,
            self.auction_id,
        ];
        if ids.iter().any(|id| check_uuid_v7(id).is_err())
            || ids[..4]
                .iter()
                .enumerate()
                .any(|(n, id)| ids[..n].contains(id))
            || self.expected_bid_revision == Some(0)
            || self.max_gold.is_some_and(|max| i64::try_from(max).is_err())
            || (self.max_gold.is_none() && self.expected_bid_revision.is_none())
        {
            return Err(GuildhallEscrowError::InvalidInput);
        }
        Ok(())
    }
}

impl DurabilityRoot {
    /// PRIVATE UNINSTALLED financial-component wrapper. The fixture-only auction read contract
    /// is not a native House admission/control proof. Do not register this as guildhall_bid:
    /// CP must compose real catalogue/auction decision+anti-sniping and required admitted events.
    pub async fn guildhall_escrow_financial_step(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        actor: Option<CurrentCharacterItemFence>,
        request: GuildhallEscrowStep,
    ) -> Result<GuildhallEscrowReceipt> {
        request.validate()?;
        if actor.is_some() != request.max_gold.is_some() {
            return Err(GuildhallEscrowError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildhallEscrowError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match financial_step_in_transaction(&mut tx, &node, actor, &request).await {
                        Ok(receipt) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Err(GuildhallEscrowError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

fn request_binding(
    request: &GuildhallEscrowStep,
    actor: Option<CurrentCharacterItemFence>,
) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(b"GUILDHALL_FINANCIAL_COMPONENT_V1\0");
    hash.update(request.identity.transaction_id);
    hash.update(request.identity.guild_entry_id);
    hash.update(request.identity.account_entry_id);
    hash.update(request.bid_id);
    hash.update(request.guild_id);
    hash.update(request.house.world_id().as_bytes());
    hash.update((request.house.house_key().len() as u64).to_be_bytes());
    hash.update(request.house.house_key().as_bytes());
    hash.update(request.auction_id);
    hash.update([u8::from(request.max_gold.is_some())]);
    hash.update(request.max_gold.unwrap_or(0).to_be_bytes());
    hash.update(request.expected_bid_revision.unwrap_or(0).to_be_bytes());
    hash.update(actor.map(|f| *f.character_id.as_bytes()).unwrap_or([0; 16]));
    hash.finalize().to_vec()
}
async fn financial_step_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    actor: Option<CurrentCharacterItemFence>,
    request: &GuildhallEscrowStep,
) -> Result<GuildhallEscrowReceipt> {
    // GUILD0-LO-01: recovery/admission already held; occurrence precedes Character/Guild.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guildhall-financial:'||encode($1,'hex'),0))")
        .bind(request.identity.occurrence_id.as_slice()).execute(&mut **tx).await?;
    let binding = request_binding(request, actor);
    if let Some(row)=sqlx::query("SELECT transaction_id::text,bid_id::text,request_binding,escrow_guild_after,escrow_account_after FROM game_guildhall_operations WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(request.identity.occurrence_id.as_slice()).fetch_optional(&mut **tx).await? {
        if row.try_get::<Vec<u8>,_>("request_binding")?!=binding {return Err(GuildhallEscrowError::ConflictingOccurrence)}
        return Ok(GuildhallEscrowReceipt{transaction_id:uuid_text(&row.try_get::<String,_>("transaction_id")?)?,bid_id:uuid_text(&row.try_get::<String,_>("bid_id")?)?,
            parts:EscrowParts{guild:value(&row,"escrow_guild_after")?,account:value(&row,"escrow_account_after")?},replayed:true});
    }
    let world = request.house.world_id();
    let account = if let Some(actor) = actor {
        if !matches!(actor.runtime_scope,RuntimeScopeRefV1::Channel{world_id,..} if world_id==world)
            || assert_item_only_fence(tx, &actor, node).await?.is_err()
        {
            return Err(GuildhallEscrowError::NotCurrent);
        }
        if fee_payer_is_junior(actor.character_id) {
            return Err(GuildhallEscrowError::GuildIneligible);
        }
        let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(actor.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        Some(uuid_text(&account)?)
    } else {
        // World jobs have no session or Character-root lock, but require a current incarnation
        // actually holding a Channel of this World, independently from the persisted bid.
        if !prove_current_incarnation(tx, node).await? {
            return Err(GuildhallEscrowError::NotCurrent);
        }
        let fact = node.fact();
        if sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 AND state=1 AND holder_node_id=encode($2,'hex')::uuid AND holder_registration_revision=$3::text::numeric ORDER BY scope_key LIMIT 1 FOR SHARE")
            .bind(world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_optional(&mut **tx).await?.is_none() {return Err(GuildhallEscrowError::NotCurrent)}
        None
    };
    let guild_sql = if actor.is_some() {
        "SELECT world_id::text,state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE"
    } else {
        "SELECT world_id::text,state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR SHARE"
    };
    let guild = sqlx::query(guild_sql)
        .bind(request.guild_id.as_slice())
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(GuildhallEscrowError::GuildIneligible)?;
    if uuid_text(&guild.try_get::<String, _>("world_id")?)? != *world.as_bytes()
        || (actor.is_some() && guild.try_get::<i16, _>("state")? != 2)
        || (actor.is_none() && guild.try_get::<i16, _>("state")? == 4)
    {
        return Err(GuildhallEscrowError::GuildIneligible);
    }
    if let Some(actor) = actor {
        let rank:Option<i16>=sqlx::query_scalar("SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid AND guild_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(actor.character_id.as_bytes().as_slice()).bind(request.guild_id.as_slice()).fetch_optional(&mut **tx).await?;
        if rank != Some(1) {
            return Err(GuildhallEscrowError::NotLeader);
        }
    }
    // Serializing on the guild row prevents a new owner/held bid racing the eligibility read.
    if actor.is_some() && sqlx::query("SELECT 1 FROM game_house_properties WHERE owner_guild_id=encode($1,'hex')::uuid AND state IN('OWNED','MOVE_OUT_PENDING','DISPOSITION') LIMIT 1")
        .bind(request.guild_id.as_slice()).fetch_optional(&mut **tx).await?.is_some() {return Err(GuildhallEscrowError::GuildIneligible)}
    let property=sqlx::query("SELECT state,content_fence_operation::text FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(request.house.house_key()).fetch_optional(&mut **tx).await?.ok_or(GuildhallEscrowError::AuctionContextUnavailable)?;
    if actor.is_some()
        && (property.try_get::<String, _>("state")? != "AUCTION"
            || property
                .try_get::<Option<String>, _>("content_fence_operation")?
                .is_some())
    {
        return Err(GuildhallEscrowError::AuctionContextUnavailable);
    }
    let auction=sqlx::query("SELECT kind,rent_gold,current_price_gold,phase,ends_at>clock_timestamp() AS before_close FROM game_house_auctions WHERE auction_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND house_key=$3 FOR UPDATE")
        .bind(request.auction_id.as_slice()).bind(world.as_bytes().as_slice()).bind(request.house.house_key()).fetch_optional(&mut **tx).await?.ok_or(GuildhallEscrowError::AuctionContextUnavailable)?;
    if auction.try_get::<String, _>("kind")? != "GUILDHALL"
        || if actor.is_some() {
            auction.try_get::<String, _>("phase")? != "OPEN"
                || !auction.try_get::<bool, _>("before_close")?
        } else {
            auction.try_get::<String, _>("phase")? != "RELEASING"
        }
    {
        return Err(GuildhallEscrowError::AuctionContextUnavailable);
    }
    let old=sqlx::query("SELECT guild_id::text,world_id::text,house_key,auction_id::text,funding_account_id::text,funding_character_id::text,rent_gold,escrow_guild_gold,escrow_account_gold,state,revision::text FROM game_guildhall_bids WHERE bid_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.bid_id.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(row) = &old {
        if uuid_text(&row.try_get::<String, _>("guild_id")?)? != request.guild_id
            || uuid_text(&row.try_get::<String, _>("world_id")?)? != *world.as_bytes()
            || row.try_get::<String, _>("house_key")? != request.house.house_key()
            || uuid_text(&row.try_get::<String, _>("auction_id")?)? != request.auction_id
        {
            return Err(GuildhallEscrowError::StaleBid);
        }
        if row.try_get::<String, _>("state")? != "HELD" {
            return Err(GuildhallEscrowError::AlreadyReleased);
        }
        if request.expected_bid_revision != Some(numeric_u64(row, "revision")?) {
            return Err(GuildhallEscrowError::StaleBid);
        }
    } else if request.expected_bid_revision.is_some() || actor.is_none() {
        return Err(GuildhallEscrowError::StaleBid);
    }
    if actor.is_some() {
        if sqlx::query("SELECT 1 FROM game_guildhall_bids WHERE guild_id=encode($1,'hex')::uuid AND state='HELD' AND bid_id<>encode($2,'hex')::uuid LIMIT 1")
            .bind(request.guild_id.as_slice()).bind(request.bid_id.as_slice()).fetch_optional(&mut **tx).await?.is_some() {return Err(GuildhallEscrowError::GuildIneligible)}
        if sqlx::query("SELECT 1 FROM game_guild_house_bans WHERE guild_id=encode($1,'hex')::uuid AND banned_until>clock_timestamp() FOR SHARE")
            .bind(request.guild_id.as_slice()).fetch_optional(&mut **tx).await?.is_some() {return Err(GuildhallEscrowError::GuildIneligible)}
    }
    let (funding_account, funding_character, parts, rent, revision) = if let Some(row) = &old {
        (
            uuid_text(&row.try_get::<String, _>("funding_account_id")?)?,
            uuid_text(&row.try_get::<String, _>("funding_character_id")?)?,
            EscrowParts {
                guild: value(row, "escrow_guild_gold")?,
                account: value(row, "escrow_account_gold")?,
            },
            value(row, "rent_gold")?,
            numeric_u64(row, "revision")?
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?,
        )
    } else {
        (
            account.ok_or(GuildhallEscrowError::InvalidInput)?,
            *actor
                .ok_or(GuildhallEscrowError::InvalidInput)?
                .character_id
                .as_bytes(),
            EscrowParts::default(),
            value(&auction, "rent_gold")?,
            1,
        )
    };
    let total = if let Some(max) = request.max_gold {
        if max < value(&auction, "current_price_gold")? {
            return Err(GuildhallEscrowError::InvalidInput);
        }
        max.checked_add(rent)
            .ok_or(GuildhallEscrowError::InvalidInput)?
    } else {
        0
    };
    sqlx::query("INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(encode($1,'hex')::uuid,0) ON CONFLICT DO NOTHING")
        .bind(request.guild_id.as_slice()).execute(&mut **tx).await?;
    let gb:i64=sqlx::query_scalar("SELECT balance FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.guild_id.as_slice()).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0) ON CONFLICT DO NOTHING")
        .bind(funding_account.as_slice()).bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    let ab:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(funding_account.as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let guild_balance = u64::try_from(gb).map_err(|_| DurabilityError::InvalidStoredState)?;
    let account_balance = u64::try_from(ab).map_err(|_| DurabilityError::InvalidStoredState)?;
    let change = if actor.is_none() {
        parts.release(guild_balance, account_balance)?
    } else {
        parts.resize(total, guild_balance, account_balance)?
    };
    // GUILD0 §6.2 fixes the original Account as the refund source, including after Resign.
    // Extra Account funding after leader change awaits CP/architect disposition: retain the
    // existing safe refusal. A raise covered entirely by Guild gold needs no Account choice;
    // lowering and release continue returning each original contributor's custody.
    if change.after.account > parts.account && account != Some(funding_account) {
        return Err(GuildhallEscrowError::FundingBindingChanged);
    }
    let kind = if actor.is_none() {
        "BID_RELEASE"
    } else if total > parts.total()? {
        "BID_RESERVE"
    } else {
        "BID_LOWER"
    };
    let ge = (change.guild_balance_before != change.guild_balance_after)
        .then_some(request.identity.guild_entry_id);
    let ae = (change.account_balance_before != change.account_balance_after)
        .then_some(request.identity.account_entry_id);
    sqlx::query("INSERT INTO game_guildhall_operations(transaction_id,occurrence_id,request_binding,kind,bid_id,guild_id,world_id,house_key,auction_id,funding_account_id,funding_character_id,guild_balance_before,guild_balance_after,account_balance_before,account_balance_after,escrow_guild_before,escrow_guild_after,escrow_account_before,escrow_account_after,guild_entry_id,account_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,encode($11,'hex')::uuid,$12,$13,$14,$15,$16,$17,$18,$19,encode($20,'hex')::uuid,encode($21,'hex')::uuid)")
        .bind(request.identity.transaction_id.as_slice()).bind(request.identity.occurrence_id.as_slice()).bind(binding).bind(kind).bind(request.bid_id.as_slice()).bind(request.guild_id.as_slice()).bind(world.as_bytes().as_slice()).bind(request.house.house_key()).bind(request.auction_id.as_slice()).bind(funding_account.as_slice()).bind(funding_character.as_slice())
        .bind(change.guild_balance_before as i64).bind(change.guild_balance_after as i64).bind(change.account_balance_before as i64).bind(change.account_balance_after as i64)
        .bind(parts.guild as i64).bind(change.after.guild as i64).bind(parts.account as i64).bind(change.after.account as i64).bind(ge.as_ref().map(|id|id.as_slice())).bind(ae.as_ref().map(|id|id.as_slice())).execute(&mut **tx).await?;
    if old.is_none() {
        sqlx::query("INSERT INTO game_guildhall_bids(bid_id,guild_id,world_id,house_key,auction_id,funding_account_id,funding_character_id,max_gold,rent_gold,escrow_guild_gold,escrow_account_gold,state,revision,last_operation_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,'HELD',1,encode($12,'hex')::uuid)")
            .bind(request.bid_id.as_slice()).bind(request.guild_id.as_slice()).bind(world.as_bytes().as_slice()).bind(request.house.house_key()).bind(request.auction_id.as_slice()).bind(funding_account.as_slice()).bind(funding_character.as_slice()).bind(request.max_gold.unwrap_or(0) as i64).bind(rent as i64).bind(change.after.guild as i64).bind(change.after.account as i64).bind(request.identity.transaction_id.as_slice()).execute(&mut **tx).await?;
    } else {
        sqlx::query("UPDATE game_guildhall_bids SET max_gold=coalesce($2,max_gold),escrow_guild_gold=$3,escrow_account_gold=$4,state=$5,revision=$6::text::numeric,last_operation_id=encode($7,'hex')::uuid WHERE bid_id=encode($1,'hex')::uuid")
            .bind(request.bid_id.as_slice()).bind(request.max_gold.map(|v|v as i64)).bind(change.after.guild as i64).bind(change.after.account as i64).bind(if actor.is_none(){"RELEASED"}else{"HELD"}).bind(revision.to_string()).bind(request.identity.transaction_id.as_slice()).execute(&mut **tx).await?;
    }
    apply_guildhall_escrow_in_transaction(tx, request.identity.transaction_id)
        .await
        .map_err(|error| match error {
            GuildBankError::Unavailable(error) => GuildhallEscrowError::Unavailable(error),
            error => GuildhallEscrowError::BankParticipant(error),
        })?;
    Ok(GuildhallEscrowReceipt {
        transaction_id: request.identity.transaction_id,
        bid_id: request.bid_id,
        parts: change.after,
        replayed: false,
    })
}
fn value(row: &sqlx::postgres::PgRow, name: &str) -> Result<u64> {
    u64::try_from(row.try_get::<i64, _>(name)?)
        .map_err(|_| GuildhallEscrowError::Unavailable(DurabilityError::InvalidStoredState))
}

#[cfg(test)]
#[path = "../guild/guildhall_tests.rs"]
mod tests;
