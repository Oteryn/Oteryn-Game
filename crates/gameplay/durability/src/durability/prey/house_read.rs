//! Existing Prey observations for an independently current named-House resident.
//! This read never initializes a slot or produces a receipt, payment or clock write.
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::{CharacterProgressionError as Error, uuid_text};
use crate::durability::character_wheel::WheelPremiumGate;
use crate::durability::db::{begin_type2_transaction, lock_admission_relations};
use crate::durability::house_hireling::fence::assert_house_inventory_fence;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::ChannelId;
use oteryn_protocol_oteryn::prey::Reply;

impl DurabilityRoot {
    pub async fn read_house_prey_state<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        origin_channel: ChannelId,
        gate: &G,
    ) -> Result<(Reply, [bool; 3]), Error> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate, move |holder, deadline, gate| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // Preserve the original Prey/Premium account and Character lock order.
            let account: String = sqlx::query_scalar(
                "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let premium = gate.allocation_permitted(&mut tx, uuid_text(&account)?).await?;
            if !assert_house_inventory_fence(&mut tx, &node, &fence, *origin_channel.as_bytes()).await? {
                return Ok(Err(Error::AuthorityRejected));
            }
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id=$1::text::uuid FOR UPDATE")
                .bind(&account).fetch_one(&mut **tx).await?;
            let world = fence.runtime_scope.world_id();
            let mut permanent = [false; 3];
            for target in sqlx::query_scalar::<_, i16>(
                "SELECT u.target FROM game_account_prey_unlocks u JOIN game_character_roots r ON r.character_id=u.claiming_character_id WHERE u.account_id=$1::text::uuid AND u.world_id=encode($2,'hex')::uuid AND u.account_id=r.account_id AND u.world_id=r.world_id AND u.unlock_kind='PREY_SLOT_PERMANENT' ORDER BY u.target LIMIT 2",
            ).bind(&account).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await? {
                let index = usize::try_from(target).ok().and_then(|value| value.checked_sub(1))
                    .filter(|value| *value < permanent.len()).ok_or(DurabilityError::InvalidStoredState)?;
                permanent[index] = true;
            }
            let character = fence.character_id.as_bytes();
            let vocation: Option<String> = sqlx::query_scalar(
                "SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid",
            ).bind(character.as_slice()).fetch_optional(&mut **tx).await?;
            let eligible = vocation.as_deref().is_some_and(|value| !value.is_empty() && value != "none");
            let mut access = [eligible, eligible && premium, false];
            for (index, permanent) in permanent.iter().enumerate() {
                access[index] |= eligible && *permanent;
            }
            let balance: i16 = sqlx::query_scalar(
                "SELECT balance FROM game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE",
            ).bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.unwrap_or(0);
            let balance = u8::try_from(balance).map_err(|_| DurabilityError::InvalidStoredState)?;
            let reply = super::storage::load(&mut tx, character, &access, balance).await?;
            tx.commit(deadline).await?;
            Ok(Ok((reply, permanent)))
        })).await?
    }
}
