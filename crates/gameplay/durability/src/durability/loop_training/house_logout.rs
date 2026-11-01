//! Q16: the existing explicit logout marker under actual named-House custody.
//! This proof cannot authorize training, vocation, experience or activity writes.
use super::TrainingLogout;
use super::super::character_authority::ReconciledCharacterAuthority;
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, FencedCharacterRoot, numeric_u64,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, RuntimeScopeRefV1};
use sqlx::{Postgres, Row, Transaction};

/// Only TrainingLogout can construct this borrowed capability. Every new build
/// rechecks its exact current custody and root inside the deciding transaction.
#[derive(Clone, Copy)]
pub(in crate::durability) struct HouseLogout<'a> {
    owner: &'a TrainingLogout,
}

impl<'a> HouseLogout<'a> {
    pub(super) const fn of(owner: &'a TrainingLogout) -> Self {
        Self { owner }
    }

    pub(in crate::durability) const fn fence(self) -> CurrentCharacterGameplayFence {
        self.owner.fence
    }

    pub(in crate::durability) async fn fenced_root(
        self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Result<FencedCharacterRoot, CharacterProgressionError>, DurabilityError> {
        let Some(origin) = self.owner.origin_channel else {
            return Ok(Err(CharacterProgressionError::InvalidInput));
        };
        // This is an actor-end checkpoint, not a service or entry grant. ACL
        // revocation must not prevent the resident from releasing its claims.
        if super::super::house_hireling::fence::assert_house_custody_fence(
            tx, &self.owner.node, &self.owner.fence.item_fence(), *origin.as_bytes(),
        ).await?.is_none() {
            return Ok(Err(CharacterProgressionError::AuthorityRejected));
        }
        let row = sqlx::query(
            "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision \
             FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid \
             AND lifecycle=1 FOR UPDATE",
        ).bind(self.owner.fence.character_id.as_bytes().as_slice())
         .fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(Err(CharacterProgressionError::AuthorityRejected));
        };
        let revision = numeric_u64(&row, "character_revision")?;
        if revision != self.owner.fence.expected_character_revision.get() {
            return Ok(Err(CharacterProgressionError::CharacterRevisionMismatch));
        }
        Ok(Ok(FencedCharacterRoot {
            revision,
            profile_revision: row.try_get("profile_revision")?,
            ruleset_revision: row.try_get("ruleset_revision")?,
            content_revision: row.try_get("content_revision")?,
        }))
    }
}

impl TrainingLogout {
    /// Preserve the original logout participant with the actual House fence.
    /// `origin_channel` remains provenance and never replaces that scope.
    pub fn new_house(
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        context: super::TrainingContext,
        selected: Option<usize>,
        origin_channel: ChannelId,
    ) -> Result<Self, DurabilityError> {
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. }) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut owner = Self::new(root, authority, node, fence, context, selected)?;
        owner.origin_channel = Some(origin_channel);
        Ok(owner)
    }
}
