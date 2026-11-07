//! The selected quest's two source chests must award the real Ring and full Bronze Amulet once.
use super::TestResult;
pub(super) const RING_CLAIM: &str = "oteryn:reward-claim.kv.powerring1";

#[test]
fn questpc5_power_ring_complete_source_route() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_power_ring",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED power_ring_selected_reward {:?}",
            started.elapsed()
        );
        let mut rewards = Vec::new();
        for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
            let doc: serde_json::Value = serde_json::from_str(shard)?;
            for row in doc["records"].as_array().ok_or("claim records")? {
                let d = &row["definition"];
                if d["identity"]["key"] == RING_CLAIM {
                    rewards.push(d["placements"][0]["reward"]["items"][0]["item"]["key"].clone());
                }
            }
        }
        assert_eq!(
            rewards,
            [serde_json::json!("oteryn:item.tibia.i3050")],
            "the chosen Power Ring quest still projects the donor gold reward instead of its selected Ring"
        );
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED power_ring_selected_reward {:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::power_ring_route_cohort()
}

use super::fixture::debug;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    actor: ExactActorRef,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("item fence")?;
    let before = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert_eq!(before.entries.len(), 1);
    assert_eq!(
        before.entries[0].item.definition.production_key,
        "oteryn:item.tibia.i3050"
    );
    assert_eq!(before.entries[0].item.quantity, 1);
    let ring = before.entries[0].item.item_instance_id;
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        actor,
        Position {
            x: 32600,
            y: 31776,
            floor: -9,
        },
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-9, 32601, 31776, 1).ok_or("amulet source")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    for (command, expected) in [
        (3, UseDisposition::Committed),
        (4, UseDisposition::NothingToUse),
    ] {
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            2,
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            expected
        );
    }
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert_eq!(bag.entries.len(), 2);
    assert!(bag.entries.iter().any(|e| e.item.item_instance_id == ring
        && e.item.definition.production_key == "oteryn:item.tibia.i3050"
        && e.item.quantity == 1));
    let amulets: Vec<_> = bag
        .entries
        .iter()
        .filter(|e| e.item.definition.production_key == "oteryn:item.tibia.i3056")
        .collect();
    assert_eq!(amulets.len(), 1);
    assert_eq!(amulets[0].item.quantity, 1);
    let native = owner
        .active_generation
        .ok_or("generation")?
        .native_gameplay()
        .ok_or("native")?;
    let policy = native
        .item_policy("oteryn:item.tibia.i3056", "definition-r1")
        .ok_or("Bronze Amulet")?;
    use crate::content::ReferenceItemField::Known;
    assert!(
        matches!(&policy.record().semantics.charges,Known(charges) if charges.count==Known(200)),
        "exact default two hundred charges"
    );
    let active_ring = native
        .item_policy("oteryn:item.tibia.i3087", "definition-r1")
        .ok_or("active Power Ring")?;
    assert!(
        matches!(&active_ring.record().semantics.temporal,
        Known(temporal) if matches!(&temporal.duration,Known(ms) if ms.0==1_800_000)
            && temporal.stop_duration==Known(true)),
        "the admitted active ring supplies the pinned thirty-minute equipped-time default"
    );
    eprintln!(
        "POSTGRES_CASE_OBSERVED power_ring_active_default_ms=1800000 stop_while_unequipped=true"
    );
    let states:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(ring.as_slice()).bind(amulets[0].item.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        states, 0,
        "fresh exact definitions provide full defaults, with no fabricated partial timer/charge state"
    );
    let claims:Vec<(String,i64)>=sqlx::query_as("SELECT claim_production_key,count(*) FROM game_reward_claim_mint_receipts GROUP BY claim_production_key ORDER BY claim_production_key").fetch_all(&h.pool).await?;
    assert_eq!(
        claims,
        [
            (RING_CLAIM.into(), 1),
            ("oteryn:reward-claim.kv.powerring2".into(), 1)
        ]
    );
    let gold:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(
        gold, 0,
        "selected Ring replaces the donor leaf rather than granting a parallel gold reward"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED power_ring_actual_two_chests_full_defaults_once {:?}",
        started.elapsed()
    );
    Ok(())
}
