//! Account deletion finalization v2. Unknown owner policies are durable PENDING,
//! never successful dispositions. The root remains private quarantine backing
//! state; active ownership is fenced by the closure membership relation.
use super::*;

mod publish;
mod state;
mod wire;
pub use publish::AuditDelivery;
pub use wire::{ClosureState, Intent, ResultData, decode, encode_receipt};

/// A decision belonging to a domain owner, not to the deletion worker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerPolicy {
    #[default]
    Pending,
    /// Only an independently established empty domain can prepare.
    VacantOnly,
    /// Owner has explicitly declared reversible disposition impossible.
    Refuse,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleSource {
    #[default]
    Pending,
    /// Independently reconciled, expiring authenticated observations. Runtime
    /// never reconstructs current evidence from an intent.
    PlatformPull,
    ControlObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub epoch_fence_file: std::path::PathBuf,
    pub lifecycle_source: LifecycleSource,
    pub house: OwnerPolicy,
    pub guild: OwnerPolicy,
    pub market: OwnerPolicy,
    pub bank: OwnerPolicy,
    /// Preserve exact in-place inventory/stash/Inbox custody; disabling stays pending.
    pub preserve_custody: bool,
    /// No permanent disposition until its owner accepts retirement.
    pub permit_retirement: bool,
    /// Name release defaults to thirty days; physical erasure remains unapproved.
    pub name_release_after_seconds: Option<u64>,
    pub physical_erasure_after_seconds: Option<u64>,
    pub proof_retention_seconds: Option<u64>,
    pub restoration_operator: Option<String>,
}

pub const STALE_GENERATION: &str = "CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION";
pub const OBLIGATION: &str = "CHAR_CMD_ACCOUNT_CLOSE_OBLIGATION";
pub const RESOLUTION_CONFLICT: &str = "CHAR_CMD_ACCOUNT_CLOSE_RESOLUTION_CONFLICT";
pub const EVENT_TYPE: i64 = 7;
pub const AUDIT_PROFILE: &str = "PLAT3_AUTHORITY_AUDIT_RETENTION_V1";

fn pending() -> Option<Option<Stored>> {
    // A durable job may have advanced without a terminal receipt. The empty
    // internal sentinel commits that bounded pass, but is never published.
    Some(Some(Stored {
        receipt: Vec::new(),
        result_code: String::new(),
    }))
}

pub(crate) async fn operation_exists(
    tx: &mut Tx<'_>,
    operation: &[u8; 16],
) -> Result<bool, DurabilityError> {
    let installed: bool =
        sqlx::query_scalar("SELECT to_regclass('game_account_closure_jobs') IS NOT NULL")
            .fetch_one(&mut **tx)
            .await?;
    if !installed {
        return Ok(false);
    }
    Ok(sqlx::query(
        "SELECT 1 FROM game_account_closure_jobs WHERE operation_id=encode($1,'hex')::uuid",
    )
    .bind(operation.as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .is_some())
}

pub(super) async fn decide(
    tx: &mut Tx<'_>,
    request: &Request<'_>,
    input: &Owned,
    binding: &[u8],
    now: u64,
) -> Result<Option<Option<Stored>>, DurabilityError> {
    state::decide(tx, request, input, binding, now).await
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            epoch_fence_file: std::path::PathBuf::from(
                "/var/lib/oteryn/account-closure/epoch-fence",
            ),
            lifecycle_source: LifecycleSource::PlatformPull,
            house: OwnerPolicy::VacantOnly,
            guild: OwnerPolicy::VacantOnly,
            market: OwnerPolicy::VacantOnly,
            bank: OwnerPolicy::VacantOnly,
            preserve_custody: true,
            permit_retirement: true,
            name_release_after_seconds: Some(2_592_000),
            physical_erasure_after_seconds: None,
            proof_retention_seconds: None,
            restoration_operator: None,
        }
    }
}
