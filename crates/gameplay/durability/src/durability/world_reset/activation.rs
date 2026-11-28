//! Final reset composition; one commit owns epoch, calendar and Channel issuance.
use super::*;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_decay_retire::DecayRetireError;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use oteryn_gameplay_content::content::QualifiedNativeEntryRoom;

pub(super) struct ResetContent<'a> {
    pub(super) map: &'a str,
    pub(super) server: [u8; 32],
    pub(super) client: [u8; 32],
    pub(super) frame: [u8; 32],
}

impl<'a> ResetContent<'a> {
    fn new(room: &QualifiedNativeEntryRoom, map: &'a str) -> Self {
        Self {
            map,
            server: room.compiled().server_digest(),
            client: room.compiled().client_digest(),
            frame: room.frame_binding().digest(),
        }
    }
}

impl DurabilityRoot {
    /// Complete a sealed reset only with the exact qualified old/new content and
    /// proven absence of Houses. None means the fresh-generation drain must wait.
    /// An unknown commit is retried with this same binding; historical replay
    /// returns the epoch without acquiring new runtime authority.
    pub async fn finish_world_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        reset: &RetiringWorldReset,
        source: &QualifiedNativeEntryRoom,
        target: &QualifiedNativeEntryRoom,
    ) -> std::result::Result<Option<u64>, DecayRetireError> {
        if reset.target != target.compiled().server_digest()
            || [source, target].iter().any(|room| {
                let cells = room.movement_cells();
                cells.scope().world_id != reset.world || !cells.house_tiles().is_known_empty()
            })
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let source = ResetContent::new(source, source.movement_cells().scope().map_revision.as_str());
        let target = ResetContent::new(target, target.movement_cells().scope().map_revision.as_str());
        self.finish_qualified_reset(authority, node, reset, (source, target), &[])
            .await
    }

    /// Finish a Server Save for the same checked bundle on both sides of the reset.
    /// The artifact owns the map and House tiles. The separately qualified native
    /// content owns the issued server, client and frame identities.
    pub async fn finish_bundle_world_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        reset: &RetiringWorldReset,
        rooms: (&QualifiedNativeEntryRoom, &QualifiedNativeEntryRoom),
        catalogue: &BundleHouseCatalogue,
    ) -> std::result::Result<Option<u64>, DecayRetireError> {
        let (source, target) = rooms;
        if reset.target != catalogue.digest
            || reset.target == target.compiled().server_digest()
            || [source, target]
                .iter()
                .any(|room| room.movement_cells().scope().world_id != reset.world)
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let source = ResetContent::new(source, &catalogue.map_revision);
        let target = ResetContent::new(target, &catalogue.map_revision);
        self.finish_qualified_reset(authority, node, reset, (source, target), catalogue.tiles())
            .await
    }

    async fn finish_qualified_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        reset: &RetiringWorldReset,
        (source, target): (ResetContent<'_>, ResetContent<'_>),
        house_tiles: &[bundle_houses::HouseTile],
    ) -> std::result::Result<Option<u64>, DecayRetireError> {
        content_preflight::check(self, *reset, &source, &target, house_tiles).await?;
        // Reconcile a completed commit first; historical evidence grants no
        // current authority. Empty resets can finish in this same transaction.
        if let Some(epoch) = activate(self, node, *reset, &target).await? {
            return Ok(Some(epoch));
        }
        if !self
            .retire_world_reset_ground(authority, node, reset)
            .await?
        {
            return Ok(None);
        }
        Ok(activate(self, node, *reset, &target).await?)
    }
}

async fn activate(
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    reset: RetiringWorldReset,
    target: &ResetContent<'_>,
) -> Result<Option<u64>> {
    let world_changes = crate::durability::world_event::prepare_catalogue()?;
    let owner = root.clone();
    let node = node.clone();
    let map = target.map.to_owned();
    let server = target.server;
    let client = target.client;
    let frame = target.frame;
    root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
        let mut tx = begin_semantic_transaction(holder, deadline).await?;
        crate::durability::db::lock_admission_relations(&mut tx).await?;
        let phase: Option<String> = sqlx::query_scalar("SELECT phase FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric FOR UPDATE")
            .bind(reset.world.as_bytes().as_slice()).bind(reset.epoch.to_string()).fetch_optional(&mut *tx).await?;
        if phase.as_deref() != Some("ACTIVATED") {
            match census::drained_channels(&owner, &mut tx, &node, reset).await? {
                census::WorldResetDrain::Drained(_) => {},
                census::WorldResetDrain::NotCurrent => return Ok(None),
                census::WorldResetDrain::CapacityExceeded => return Err(DurabilityError::InvalidStoredState),
            }
        }
        if phase.as_deref()!=Some("ACTIVATED") {
            crate::durability::cake_event::before_reset(&mut tx,reset.world).await?;
        }
        let fact = node.fact();
        let outcome: Option<String> = sqlx::query_scalar("SELECT game_world_reset_activate(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6,$7,$8,encode($9,'hex')::uuid,$10::text::numeric)")
            .bind(reset.world.as_bytes().as_slice()).bind(reset.epoch.to_string())
            .bind(reset.binding.as_slice()).bind(reset.target.as_slice()).bind(map)
            .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice())
            .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
            .fetch_one(&mut *tx).await?;
        let epoch = match outcome.as_deref() {
            Some("NEW_EPOCH" | "HISTORICAL") => Some(reset.epoch.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?),
            None => None,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        if let Some(epoch) = epoch {
            // Same transaction as epoch, calendar and content issuance. Historical
            // replay verifies the immutable decision; it never redraws at channel boot.
            crate::durability::world_event::record_reset(&mut tx, reset.world, epoch, outcome.as_deref() == Some("NEW_EPOCH"), &world_changes).await?;
            crate::durability::cake_event::after_reset(&mut tx,reset.world,epoch).await?;
        }
        commit_semantic_transaction(tx, deadline).await?;
        Ok(epoch)
    })).await
}
