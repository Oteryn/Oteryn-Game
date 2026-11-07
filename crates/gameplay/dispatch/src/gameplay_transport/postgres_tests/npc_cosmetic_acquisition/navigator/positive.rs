//! Historical owned-operand qualification, never a natural Soul Net drop claim.
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::generation_item_facts;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    actor: crate::foundation::ExactActorRef,
    source_entry: bool,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native source pin")?;
    let definition = |key: &str| TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    let anchor = definition("oteryn:item.tibia.i14019");
    let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/navigator'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        before, 0,
        "the actual story begins with absent progress; no stage3 seed"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("actual initial character fence")?;
    let request = RewardClaimMintRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(1000).map_err(debug)?,
        ),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "fixture:reward.srv7-historical-owned-small-golden-anchor".into(),
            revision_ref: "definition-r1".into(),
        },
        source_placement:
            "fixture:historically-owned-anchor-rule-qualification-not-soul-net-producer".into(),
        item: generation_item_facts(native, &anchor).map_err(debug)?,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        backpack: generation_item_facts(native, &definition("oteryn:item.tibia.i2854"))
            .map_err(debug)?,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    // Genuine mature Root MINT + audit/custody proof. Only its historical
    // acquisition provenance is isolated; no donor drop odds are asserted.
    let mut candidate = owner
        .root
        .freeze_reward_claim_mint(owner.character, owner.holder, fence.item_fence(), request)
        .await
        .map_err(debug)?;
    let uuid = *candidate.item_instance_id();
    let mint = *candidate.transaction_id();
    owner
        .root
        .commit_reward_claim_mint(
            owner.character,
            owner.holder,
            fence.item_fence(),
            &mut candidate,
        )
        .await
        .map_err(debug)?;
    let proven:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts r JOIN game_item_instances i USING(item_instance_id) JOIN game_item_audit_outbox a USING(transaction_id,event_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.minted_transaction_id=encode($2,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i14019' AND i.lifecycle=1 AND i.quantity=1 AND r.envelope_sha256=a.envelope_sha256 AND a.envelope_sha256=sha256(a.envelope))").bind(uuid.as_slice()).bind(mint.as_slice()).fetch_one(&h.pool).await?;
    assert!(
        proven,
        "historical anchor has a genuine Root MINT/audit receipt"
    );
    let mut first = 1u64;
    if source_entry {
        use crate::map::view::MapFacts as _;
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let at = crate::gameplay_transport::bundle_map_position::bundle_map_position(
                &mut runtime,
                owner.movement_cells,
                actor,
            )
            .ok_or("real source bundle position")?;
            let tile = crate::map::view::compose_tile(
                &at.map.overlay,
                &at.map.facts.pinned(),
                crate::map::overlay::TilePos {
                    x: 33504,
                    y: 31252,
                    floor: -11,
                },
            )
            .map_err(debug)?
            .ok_or("actual Golden Anchor tile")?;
            let reference = native
                .item_index()
                .ok_or("native index")?
                .definition_ref("Item", "oteryn:item.tibia.i13941", "definition-r1")
                .ok_or("Golden Anchor ref")?;
            let entry = tile
                .entries
                .iter()
                .find(|entry| {
                    entry.facts.definition
                        == oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference)
                })
                .ok_or("actual13941 source placement")?;
            let crate::map::view::EntrySource::Base { placement_key, .. } = entry.source else {
                return Err("Golden Anchor must remain original source map placement".into());
            };
            let mut placement = at.map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&placement_key.to_be_bytes());
            WorldObjectTarget {
                placement,
                expected_revision: at.map.facts.object_revision(placement_key),
            }
        };
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("initial source selector")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target)),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let source_ref = native
            .item_index()
            .ok_or("native index")?
            .definition_ref("Item", "oteryn:item.tibia.i14019", "definition-r1")
            .ok_or("anchor source ref")?;
        let (next, result, _) = super::super::super::item_craft::exchange(
            owner,
            *admitted,
            1,
            &payload,
            Some(source_ref),
        )
        .await?;
        *admitted = next;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "native same-anchor USE on actual13941 must enter the Navigator room; no test teleport fallback"
        );
        let position = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            (position.x, position.y, position.floor),
            (33640, 31379, -9),
            "exact source anchor destination"
        );
        let kept:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND minted_transaction_id=encode($2,'hex')::uuid AND lifecycle=1 AND quantity=1)").bind(uuid.as_slice()).bind(mint.as_slice()).fetch_one(&h.pool).await?;
        assert!(kept, "room access retains the same anchor UUID");
        first = 2;
    }
    let text = [
        "hi", "flou", "explain", "yes", "no", "yes", "yes", "yes", "helmet", "yes",
    ];
    let commands: Vec<_> = text
        .iter()
        .enumerate()
        .map(|(i, text)| Ok((first + u64::try_from(i)?, *text)))
        .collect::<Result<_, std::num::TryFromIntError>>()?;
    super::super::super::grizzly_npc::wire::talk_paced(owner, admitted, npc, &commands).await?;
    let receipt:(String,String,i64)=sqlx::query_as("SELECT transaction_id::text,cost_transaction_id::text,committed_character_revision::bigint FROM game_npc_cosmetic_receipts WHERE source_key='oteryn:npc-cosmetic/navigator/deepling-helmet' AND character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let consumed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND minted_transaction_id=encode($2,'hex')::uuid AND lifecycle<>1 AND last_transaction_id=$3::uuid)").bind(uuid.as_slice()).bind(mint.as_slice()).bind(&receipt.1).fetch_one(&h.pool).await?;
    assert!(
        consumed,
        "the final source exchange consumes that same historically owned anchor"
    );
    let state:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:appearance-grant/npc/navigator/deepling/addon2'),(SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/navigator' AND value=4)").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        state,
        (1, 1),
        "actual NPC grants helmet addon and acknowledges source4 once"
    );
    // Resolve the durable source grant through the production appearance reader.
    // This is not a claim that the unrelated Deepling base quest was completed.
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let account = crate::domain::AccountId::from_bytes(*account.as_bytes()).map_err(debug)?;
    let mut connection = h.pool.acquire().await?;
    let resolved = crate::durability::character_appearance::read_account_appearance_unlocks(
        &mut connection,
        account,
        &["oteryn:appearance-grant/npc/navigator/deepling/addon2".into()],
    )
    .await?;
    assert_eq!(
        resolved.len(),
        1,
        "production appearance owner resolves actual source grant"
    );
    assert_eq!(resolved[0].granting_character_id.as_bytes(), &id(41));
    assert_eq!(
        resolved[0].source_event_id.as_slice(),
        sqlx::types::Uuid::parse_str(&receipt.0)?.as_bytes().as_slice()
    );
    drop(connection);
    let final_command = first + 9;
    super::super::super::grizzly_npc::wire::talk_paced(
        owner,
        admitted,
        npc,
        &[(final_command, "yes")],
    )
    .await?;
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_cosmetic_receipts WHERE source_key='oteryn:npc-cosmetic/navigator/deepling-helmet' AND character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(count, 1, "replay must not add a second grant/cost/parent");
    Ok(())
}
