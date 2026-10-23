//! HOUSE-1a transaction participants. The caller retains the Premium gate's
//! transaction, recovery/admission locks and its commit/rollback responsibility.
//! Opening awaits D1383's qualified catalogue producer: a VACANT row grants nothing.

mod bid;
pub(super) mod eligibility;
mod premium;
mod settlement;
pub(super) mod value;

use super::DurabilityError;
use oteryn_gameplay_housing::house_auction::AuctionError;
use sqlx::{Postgres, Row, Transaction};

pub use bid::HouseBidRequest;
pub use settlement::HouseSettlementRequest;

#[derive(Debug)]
pub enum HouseOwnershipError {
    InvalidInput,
    AuthorityRejected,
    NotPremium,
    CatalogueUnavailable,
    AuctionClosed,
    AuctionFull,
    Ineligible,
    BidConflict,
    RequestConflict,
    ConcurrentChange,
    Value(AuctionError),
    Unavailable(DurabilityError),
}

impl From<sqlx::Error> for HouseOwnershipError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl From<DurabilityError> for HouseOwnershipError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<AuctionError> for HouseOwnershipError {
    fn from(error: AuctionError) -> Self {
        match error {
            AuctionError::Closed => Self::AuctionClosed,
            AuctionError::Full => Self::AuctionFull,
            error => Self::Value(error),
        }
    }
}
impl From<super::bank::BankError> for HouseOwnershipError {
    fn from(error: super::bank::BankError) -> Self {
        match error {
            super::bank::BankError::Unavailable(error) => Self::Unavailable(error),
            _ => Self::AuthorityRejected,
        }
    }
}

type Result<T> = std::result::Result<T, HouseOwnershipError>;
type Tx<'a> = Transaction<'a, Postgres>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseOwnershipOutcome {
    Applied,
    Replayed,
    NotDue,
}

async fn account_lock(tx: &mut Tx<'_>, account: &[u8; 16], world: &[u8; 16]) -> Result<()> {
    // The account/world slot may be absent. This is its named lock, acquired
    // after the property and before bids, slots and balances are changed.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||encode($2,'hex'),143))",
    )
    .bind(account.as_slice())
    .bind(world.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn clock(tx: &mut Tx<'_>) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}

async fn lock_property(
    tx: &mut Tx<'_>,
    world: &[u8; 16],
    house: &str,
) -> Result<sqlx::postgres::PgRow> {
    sqlx::query("SELECT state,revision::text,ownership_generation::text,content_digest FROM game_house_property_lock(encode($1,'hex')::uuid,$2)")
        .bind(world.as_slice()).bind(house).fetch_optional(&mut **tx).await?
        .ok_or(HouseOwnershipError::CatalogueUnavailable)
}

async fn held(tx: &mut Tx<'_>, auction: &[u8; 16]) -> Result<Vec<sqlx::postgres::PgRow>> {
    let rows = sqlx::query("SELECT uuid_send(account_id) AS account,uuid_send(character_id) AS character,maximum,first_rent,escrow_gold,reached_order FROM game_house_bids_lock(encode($1,'hex')::uuid)")
        .bind(auction.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > oteryn_gameplay_housing::house_auction::BIDDERS_MAX {
        return Err(HouseOwnershipError::AuctionFull);
    }
    Ok(rows)
}

fn uuid(row: &sqlx::postgres::PgRow, column: &str) -> Result<[u8; 16]> {
    row.try_get::<Vec<u8>, _>(column)?
        .try_into()
        .map_err(|_| HouseOwnershipError::InvalidInput)
}
fn gold(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64> {
    u64::try_from(row.try_get::<i64, _>(column)?).map_err(|_| HouseOwnershipError::InvalidInput)
}
