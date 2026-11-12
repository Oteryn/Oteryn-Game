//! Six repeat chests must use the dedicated durable cycle owner, never once claims.
use super::TestResult;

type DeliveredCycle = (i64, Vec<u8>, i16, Option<i64>, String, String, i64);

#[test]
fn falcon_chests_exact_six_sources_and_seven_rewards() -> TestResult {
    use oteryn_gameplay_quests::quest::falcon::{CHESTS, REWARDS, claim_key, draw_index};
    assert_eq!(CHESTS.len(), 6);
    assert_eq!(
        REWARDS,
        [
            (9081, 1),
            (28821, 1),
            (28823, 1),
            (9058, 1),
            (6299, 1),
            (3052, 1),
            (3035, 10)
        ]
    );
    for chest in CHESTS {
        assert!(claim_key(&chest.key()).is_some());
    }
    let mut seen = std::collections::BTreeSet::new();
    for value in 0..=255 {
        let entropy = [value; 32];
        let index = draw_index(&entropy);
        assert!(index < 7);
        assert_eq!(index, draw_index(&entropy));
        seen.insert(index);
    }
    assert_eq!(seen.len(), 7);
    Ok(())
}

use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world, wire_uses};
use super::{Dependencies, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_gameplay_quests::quest::falcon::{CHESTS, Chest, REWARDS, draw_index};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
pub(super) fn falcon_chests_postgres_real_use_frozen_draw_refusal_restart_and_fences() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            let world = crate::foundation::WorldId::decode(&id(42))
                .map_err(debug)
                .map_err(|e| e.to_string())?;
            let activation =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")
                    .map_err(|e| e.to_string())?;
            for chest in CHESTS {
                let started = std::time::Instant::now();
                scenario(chest, &activation).map_err(|e| format!("{}: {e}", chest.suffix))?;
                println!(
                    "FALCON_CHEST POSTGRES_CASE_COMPLETED {} duration={:?}",
                    chest.suffix,
                    started.elapsed()
                );
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "Falcon scenario panicked")??;
    Ok(())
}
fn definition(id: u16) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    }
}
fn scenario(
    chest: Chest,
    activation: &super::quest_catalogue_chests::CatalogueActivation,
) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "falcon_chests", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) = activation;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native generation")?;
        for (id, _) in REWARDS {
            generation_item_facts(native, &definition(id)).map_err(debug)?;
        }
        let world = source_world(chest.position, chest.unique, chest.appearance, 0)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(chest.position.0) - 1,
                i32::from(chest.position.1),
                -i16::from(chest.position.2),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(chest.position.0) - 1,
                    y: i32::from(chest.position.1),
                    floor: -i16::from(chest.position.2),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: definition(2854),
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(&authority, &h.node, &fence, &weights, &template)
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key = oteryn_world_bundle::bundle::placement_key(
            chest.position.2,
            chest.position.0,
            chest.position.1,
            1,
        )
        .ok_or("placement")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let full = chest.unique == 6105;
        if full {
            for index in 0..20 {
                let request = RewardClaimMintRequest {
                    backpack_outputs: false,
                    command: CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(100 + index).map_err(debug)?,
                    ),
                    claim: TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        production_key: format!("fixture:falcon-burden.{index}"),
                        revision_ref: "definition-r1".into(),
                    },
                    source_placement: "fixture:falcon-burden".into(),
                    item: generation_item_facts(native, &definition(3031)).map_err(debug)?,
                    quantity: 1,
                    contents: Vec::new(),
                    root_text: None,
                    root_fluid_subtype: None,
                    root_charges_remaining: None,
                    backpack: generation_item_facts(native, &template.definition).map_err(debug)?,
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                    achievement: None,
                    quest_transition: None,
                    key_number: None,
                };
                let mut candidate = h
                    .root
                    .freeze_reward_claim_mint(&authority, &h.node, fence, request)
                    .await
                    .map_err(debug)?;
                h.root
                    .commit_reward_claim_mint(&authority, &h.node, fence, &mut candidate)
                    .await
                    .map_err(debug)?;
            }
        }
        let mut frozen_draw = None;
        {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut stale = admitted;
            let mut stale_fence = fence;
            stale_fence.character_lease_generation += 1;
            stale.item_fence = Some(stale_fence);
            assert_eq!(
                wire_uses(&owner, stale, &[1], &target).await?,
                [UseDisposition::Rejected]
            );
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_falcon_chest_cycles")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(count, 0, "stale authority must not freeze a draw");
        }
        if full {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, &[1, 2], &target).await?,
                [UseDisposition::Rejected, UseDisposition::Rejected]
            );
            let draw: (Vec<u8>, i16, Option<i64>) = sqlx::query_as(
                "SELECT entropy,selected_index,claimed_at FROM game_character_falcon_chest_cycles",
            )
            .fetch_one(&h.pool)
            .await?;
            assert!(draw.2.is_none());
            frozen_draw = Some((draw.0, draw.1));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations WHERE claim_production_key LIKE '%falcon_bastion%'").fetch_one(&h.pool).await?;
            assert_eq!(count, 0, "refusal writes only the cycle draw");
            free_one(&owner, fence, chest, 500).await?;
        }
        // Recreate the composed owner: neither refusal draw nor mint replay can
        // depend on an in-memory RNG value or a connection's terminal-result cache.
        for cold in [false, true] {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, &[1, 1, 2], &target).await?,
                [
                    UseDisposition::Committed,
                    UseDisposition::Committed,
                    UseDisposition::NothingToUse
                ],
                "cold={cold}"
            );
        }
        let rows:Vec<DeliveredCycle>=sqlx::query_as("SELECT c.cycle,c.entropy,c.selected_index,c.claimed_at,r.claim_production_key,i.definition_production_key,r.quantity FROM game_character_falcon_chest_cycles c JOIN game_reward_claim_mint_receipts r ON r.transaction_id=c.transaction_id JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE c.chest_key=$1")
            .bind(chest.key()).fetch_all(&h.pool).await?;
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.0, 1);
        assert!(row.3.is_some());
        let entropy: [u8; 32] = row.1.as_slice().try_into()?;
        assert_eq!(usize::try_from(row.2)?, draw_index(&entropy));
        if let Some(draw) = frozen_draw {
            assert_eq!(
                (&row.1, row.2),
                (&draw.0, draw.1),
                "refusal must never reroll"
            );
        }
        let (reward, count) = REWARDS[draw_index(&entropy)];
        assert_eq!(row.4, format!("{}:cycle:1", chest.key()));
        assert_eq!(row.5, format!("oteryn:item.tibia.i{reward}"));
        assert_eq!(row.6, i64::from(count));
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_falcon_chest_cycles")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);
        // This isolated database owns its helper definition. Advance only the
        // server DB-clock seam: receipts and all guards remain active and unchanged.
        let claimed = row.3.ok_or("claimed time")?;
        for (now, expected) in [
            (claimed + 86_400_000 - 1, UseDisposition::NothingToUse),
            (claimed + 86_400_000, UseDisposition::Committed),
        ] {
            // The only interpolation is an i64 obtained from this fixture receipt.
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE OR REPLACE FUNCTION game_falcon_now_ms() RETURNS BIGINT LANGUAGE SQL VOLATILE AS 'SELECT {now}::bigint'")))
                .execute(&h.pool).await?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            if full && expected == UseDisposition::Committed {
                free_one(&owner, fence, chest, 501).await?;
            }
            assert_eq!(
                wire_uses(&owner, admitted, &[1, 2], &target).await?,
                [UseDisposition::Committed, expected],
                "repeat boundary time={now}"
            );
        }
        {
            // Replaying the old command after the second cycle must still
            // return its original receipt, never consume another cycle.
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, &[1], &target).await?,
                [UseDisposition::Committed]
            );
        }
        let cycles: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT cycle,claimed_at FROM game_character_falcon_chest_cycles ORDER BY cycle",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(cycles, [(1, claimed), (2, claimed + 86_400_000)]);
        let minted:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key LIKE '%falcon_bastion%'").fetch_one(&h.pool).await?;
        assert_eq!(
            minted, 2,
            "new cycle mints once, old command replays its original cycle"
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn free_one(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    chest: Chest,
    command_id: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    let filler = bag
        .entries
        .iter()
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i3031")
        .ok_or("filler")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let permit = owner.spell_lane_permit().await.ok_or("lane permit")?;
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            CommandRef::new(
                fence.game_session_id,
                CommandId::new(command_id).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            crate::durability::item_loop_move::LoopMoveRequest {
                item: filler.item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Ground(
                    crate::durability::item_loop_move::LoopGround {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                chest.position.0 - 1,
                                chest.position.1,
                                chest.position.2,
                            ),
                        map_revision: crate::map::overlay::map_revision(
                            owner.bundle_map().ok_or("bundle map")?.overlay.base(),
                        ),
                        content_revision: revision,
                        placement_context: frame.to_vec(),
                    },
                ),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "free capacity: {moved:?}");
    Ok(())
}
