//! Accepted Platform update-origin contract; HTTPS transports bytes, TUF grants trust.
mod policy;
mod transport;
pub use policy::UpdatePolicy;
use std::path::Path;
use tough::{IntoVec, Repository, RepositoryLoader, TargetName, Transport};
pub use transport::OriginTransport;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    Configuration,
    Verification,
    Storage,
    Unavailable,
    Rollback,
}
impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Update refused: {self:?}")
    }
}
impl std::error::Error for UpdateError {}

pub struct UpdateConfig {
    channel: String,
    root: Vec<u8>,
    platform: String,
    architecture: String,
}
impl UpdateConfig {
    pub fn new(
        channel: &str,
        root: &[u8],
        platform: &str,
        architecture: &str,
    ) -> Result<Self, UpdateError> {
        if !matches!(channel, "stable" | "beta")
            || root.is_empty()
            || root.len() > 1_048_576
            || !matches!(platform, "windows" | "linux" | "macos")
            || !matches!(architecture, "x86_64" | "arm64" | "x86")
        {
            return Err(UpdateError::Configuration);
        }
        Ok(Self {
            channel: channel.into(),
            root: root.into(),
            platform: platform.into(),
            architecture: architecture.into(),
        })
    }
}
pub struct UpdateClient {
    repository: Repository,
    pub policy: UpdatePolicy,
    target: TargetName,
    pub length: u64,
}
impl UpdateClient {
    /// `state` is isolated by channel. The caller holds an exclusive install lock.
    pub async fn check(config: UpdateConfig, state: &Path) -> Result<Self, UpdateError> {
        Self::check_with_transport(config, state, OriginTransport::new()?).await
    }
    /// Injectable byte transport for fixture and deployment qualification; TUF checks are identical.
    pub async fn check_with_transport(
        config: UpdateConfig,
        state: &Path,
        transport: impl Transport + 'static,
    ) -> Result<Self, UpdateError> {
        let state = state.join(&config.channel);
        std::fs::create_dir_all(&state).map_err(|_| UpdateError::Storage)?;
        let anchor_path = state.join("anchor.json");
        let root_path = state.join("trusted-root.json");
        let trusted_root = if root_path.exists() {
            if std::fs::read(&anchor_path).map_err(|_| UpdateError::Storage)? != config.root {
                return Err(UpdateError::Configuration);
            }
            std::fs::read(&root_path).map_err(|_| UpdateError::Storage)?
        } else {
            config.root.clone()
        };
        let base = format!("https://account.oteryn.com/updates/{}/", config.channel);
        let repository = RepositoryLoader::new(
            &trusted_root,
            Url::parse(&(base.clone() + "metadata/")).map_err(|_| UpdateError::Configuration)?,
            Url::parse(&(base + "targets/")).map_err(|_| UpdateError::Configuration)?,
        )
        .datastore(&state)
        .transport(transport)
        .limits(tough::Limits {
            max_root_updates: 16,
            max_root_size: 1_048_576,
            max_targets_size: 1_048_576,
            max_snapshot_size: 1_048_576,
            max_timestamp_size: 1_048_576,
        })
        .load()
        .await
        .map_err(|_| UpdateError::Verification)?;
        let verified_root =
            serde_json::to_vec(repository.root()).map_err(|_| UpdateError::Verification)?;
        std::fs::write(&anchor_path, &config.root).map_err(|_| UpdateError::Storage)?;
        std::fs::write(state.join("root.next"), verified_root).map_err(|_| UpdateError::Storage)?;
        std::fs::rename(state.join("root.next"), root_path).map_err(|_| UpdateError::Storage)?;
        if !repository.root().signed.consistent_snapshot {
            return Err(UpdateError::Verification);
        }
        let top = &repository.targets().signed;
        let delegations = top.delegations.as_ref().ok_or(UpdateError::Verification)?;
        let role = repository
            .delegated_role(&config.channel)
            .ok_or(UpdateError::Verification)?;
        let scope = format!("channels/{}/*", config.channel);
        let valid_scope = matches!(&role.paths,tough::schema::PathSet::Paths(p) if p.len()==1 && p[0].value()==scope);
        if !top.targets.is_empty()
            || delegations.roles.len() != 1
            || !role.terminating
            || !valid_scope
        {
            return Err(UpdateError::Verification);
        }
        let policy_name = TargetName::new(format!("channels/{}/policy-v1.json", config.channel))
            .map_err(|_| UpdateError::Verification)?;
        let target = repository
            .all_targets()
            .find(|(name, _)| *name == &policy_name)
            .map(|(_, t)| t)
            .ok_or(UpdateError::Verification)?;
        if target.length > 262_144 {
            return Err(UpdateError::Verification);
        }
        let bytes = repository
            .read_target(&policy_name)
            .await
            .map_err(|_| UpdateError::Verification)?
            .ok_or(UpdateError::Verification)?
            .into_vec()
            .await
            .map_err(|_| UpdateError::Verification)?;
        let policy: UpdatePolicy =
            serde_json::from_slice(&bytes).map_err(|_| UpdateError::Verification)?;
        policy.validate(&config.channel)?;
        let delegated = repository
            .delegated_role(&config.channel)
            .and_then(|r| r.targets.as_ref())
            .ok_or(UpdateError::Verification)?;
        if delegated.signed.delegations.is_some()
            || delegated.signed.targets.len() != policy.artifacts.len() + 1
        {
            return Err(UpdateError::Verification);
        }
        let artifact = policy
            .artifacts
            .iter()
            .find(|a| a.platform == config.platform && a.architecture == config.architecture)
            .ok_or(UpdateError::Unavailable)?;
        let target =
            TargetName::new(&artifact.target_path).map_err(|_| UpdateError::Verification)?;
        let length = repository
            .all_targets()
            .find(|(name, _)| *name == &target)
            .map(|(_, t)| t.length)
            .filter(|l| *l > 0)
            .ok_or(UpdateError::Verification)?;
        let policy_path = state.join("policy.json");
        if policy_path.exists() {
            let old: UpdatePolicy = serde_json::from_slice(
                &std::fs::read(&policy_path).map_err(|_| UpdateError::Storage)?,
            )
            .map_err(|_| UpdateError::Storage)?;
            if policy.policy_revision < old.policy_revision
                || (policy.policy_revision == old.policy_revision
                    && bytes != std::fs::read(&policy_path).map_err(|_| UpdateError::Storage)?)
                || (policy.current_release_sequence < old.current_release_sequence
                    && policy.rollback_authorization != "explicit")
            {
                return Err(UpdateError::Rollback);
            }
        }
        std::fs::write(state.join("policy.next"), &bytes).map_err(|_| UpdateError::Storage)?;
        std::fs::rename(state.join("policy.next"), policy_path)
            .map_err(|_| UpdateError::Storage)?;
        Ok(Self {
            repository,
            policy,
            target,
            length,
        })
    }
    /// Streams a signed-length, signed-hash checked target; never buffers a package in memory.
    pub async fn download(&self, destination: &Path) -> Result<(), UpdateError> {
        use futures::StreamExt;
        use std::io::Write;
        let partial = destination.with_extension("partial");
        let result = async {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&partial)
                .map_err(|_| UpdateError::Storage)?;
            let mut stream = self
                .repository
                .read_target(&self.target)
                .await
                .map_err(|_| UpdateError::Verification)?
                .ok_or(UpdateError::Unavailable)?;
            while let Some(chunk) = stream.next().await {
                file.write_all(&chunk.map_err(|_| UpdateError::Verification)?)
                    .map_err(|_| UpdateError::Storage)?;
            }
            file.sync_all().map_err(|_| UpdateError::Storage)?;
            std::fs::rename(&partial, destination).map_err(|_| UpdateError::Storage)
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_file(partial);
        }
        result
    }
}
