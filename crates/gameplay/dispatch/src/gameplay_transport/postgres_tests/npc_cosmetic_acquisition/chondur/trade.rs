//! Final real NPC composition after original-source Stampor deaths and actual TAKE.
//! No material fixtures, task flags or appearance unlocks may be seeded.
use super::*;
const SOURCE: &str = "oteryn:npc-cosmetic/chondur/stampor-spirit";
const GRANT: &str = "oteryn:appearance-grant/npc/chondur/stampor/mount";
fn lines(frames: &[Vec<u8>]) -> TestResult<Vec<String>> {
    let mut result = Vec::new();
    for frame in frames {
        let envelope = crate::foundation::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != crate::foundation::MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id == oteryn_protocol_oteryn::npc_service::STATE_DOMAIN_NPC_CONVERSATION {
            result.extend(
                oteryn_protocol_oteryn::npc_service::decode_npc_conversation(delta.payload)
                    .map_err(debug)?
                    .lines,
            );
        }
    }
    Ok(result)
}
pub(super) async fn trade(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    // This is a prerequisite assertion, not a forged source producer. The
    // preceding natural acquisition loop must have killed unchanged1200HP/
    //780XP source Stampors and moved these actual MINT items with corpse TAKE.
    for (item, minimum) in [(12312, 50), (12314, 30), (12313, 100)] {
        let count: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i \
            JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id \
            WHERE i.definition_production_key=$1 AND r.loot_table_family='Loot' \
            AND r.loot_table_production_key='oteryn:loot.creature.stampor'",
        )
        .bind(format!("oteryn:item.tibia.i{item}"))
        .fetch_one(&h.pool)
        .await?;
        assert!(
            count >= minimum,
            "natural source material {item}: {count}/{minimum}"
        );
    }
    let prior: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key=$1",
    )
    .bind(GRANT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(prior, 0, "no mount entitlement fixture");
    let mut before_materials = Vec::new();
    for item in [12312, 12314, 12313] {
        let amount:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        before_materials.push(amount);
    }
    let first = admitted.continuity.next_command_id;
    let frames = super::super::super::grizzly_npc::wire::talk_appearance(
        owner,
        admitted,
        npc,
        &[(first, "hi"), (first + 1, "mount"), (first + 2, "yes")],
    )
    .await?;
    // First behavioral assertion: no new adapter/table API is queried before
    // the genuine source caller is required to award its actual mount.
    let granted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key=$1",
    )
    .bind(GRANT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        granted, 1,
        "real Chondur trade awards source Stampor mount11"
    );
    let spoken = lines(&frames)?;
    for exact in [
        "Ohhhhh Mmmmmmmmmmmm Ammmmmgggggggaaaaaaa ...",
        "Aaaaaaaaaahhmmmm Mmmaaaaaaaaaa Kaaaaaamaaaa ...",
        "Brrt! I think it worked! It's a male stampor. I linked this spirit to yours. You can probably already summon him to you ...",
        "So, since we are done here... I need to prepare another ritual, so please let me work, child.",
    ] {
        assert!(
            spoken.iter().any(|line| line == exact),
            "actual source ritual line: {exact}"
        );
    }
    let source: (i64, i64) = sqlx::query_as(
        "SELECT count(*),count(cost_transaction_id) FROM game_npc_cosmetic_receipts \
        WHERE character_id=encode($1,'hex')::uuid AND source_key=$2",
    )
    .bind(id(41).as_slice())
    .bind(SOURCE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        source,
        (1, 1),
        "one physical multi-operand payment and one source grant"
    );
    for (index, (item, cost)) in [(12312, 50), (12314, 30), (12313, 100)]
        .into_iter()
        .enumerate()
    {
        let amount:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        assert_eq!(
            before_materials[index] - amount,
            cost,
            "exact physical source cost for {item}"
        );
    }
    let observed = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("current player")?,
            admitted.game_session_id,
            Vec::new(),
        )
        .await
        .ok_or("actual earned native appearance")?;
    assert!(
        observed
            .catalogue
            .mounts
            .iter()
            .any(|m| m == "oteryn:content.mount.stampor"),
        "real server appearance codec publishes earned Stampor"
    );
    let replay = admitted.continuity.next_command_id;
    let frames = super::super::super::grizzly_npc::wire::talk_appearance(
        owner,
        admitted,
        npc,
        &[(replay, "mount"), (replay + 1, "yes")],
    )
    .await?;
    assert!(
        lines(&frames)?
            .iter()
            .any(|line| line == "You already have stampor mount."),
        "actual owned-mount source refusal"
    );
    let duplicate: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key=$1),\
        (SELECT count(*) FROM game_npc_cosmetic_receipts WHERE source_key=$2)",
    )
    .bind(GRANT)
    .bind(SOURCE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        duplicate,
        (1, 1),
        "later source dialogue cannot repay/regrant an owned mount"
    );
    Ok(())
}
