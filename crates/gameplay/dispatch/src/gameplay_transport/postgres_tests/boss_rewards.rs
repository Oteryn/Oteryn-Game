//! Real boss settlement -> personal browse/take/collect-all dispatch -> guarded Item custody.
use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::boss_encounter::reward_chest::{
    BossRewardChestPlan, BossRewardContributor, BossRewardDeath, BossRewardItem,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::guild_intent::GuildReply;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_gameplay_encounters::bosses::reward_chest::RewardParticipation;
use oteryn_protocol_oteryn::boss_rewards::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

#[path = "boss_rewards/native.rs"]
mod native;
use native::native_death;

#[path = "boss_rewards/paged_count.rs"]
mod paged_count;

#[path = "boss_rewards/movement.rs"]
mod movement;

#[path = "boss_rewards/privacy.rs"]
mod privacy;

#[path = "boss_rewards/concurrent.rs"]
mod concurrent;

#[derive(Clone, Copy)]
enum Case {
    Collection,
    Expiry,
    NativeDeath,
    OfflineReward,
    LivePrivacy,
    ConcurrentCollection,
}

#[test]
fn boss_rewards_postgres_dispatch_collection_replay_capacity_and_custody_guards() -> TestResult {
    run_case(Case::Collection)
}

#[test]
fn boss_rewards_postgres_dispatch_seven_day_expiry_private_scope_and_stale_fences() -> TestResult {
    run_case(Case::Expiry)
}

#[test]
fn boss_rewards_postgres_native_lethal_settlement_and_later_replay() -> TestResult {
    run_case(Case::NativeDeath)
}

#[test]
fn boss_rewards_postgres_disconnected_contributor_keeps_earned_entitlement() -> TestResult {
    run_case(Case::OfflineReward)
}

#[test]
fn boss_rewards_postgres_live_rewards_are_private_between_current_characters() -> TestResult {
    run_case(Case::LivePrivacy)
}

#[test]
fn boss_rewards_postgres_concurrent_collectors_on_two_roots_transfer_once() -> TestResult {
    run_case(Case::ConcurrentCollection)
}

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BOSS_REWARDS POSTGRES_CASE_SKIPPED: OTERYN_TEST_POSTGRES_ADMIN_URL absent");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss rewards PostgreSQL scenario panicked")??;
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "boss_rewards", true, false)
            .await
            .map_err(|error| format!("W03 fixture harness: {error:?}"))?;
        prepare_current_session(&h)
            .await
            .map_err(|error| format!("W03 fixture current session: {error:?}"))?;
        let seal = h
            .recovery
            .seal_current()
            .map_err(|error| format!("W03 fixture recovery seal: {error:?}"))?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("W03 fixture Character authority: {error:?}"))?;
        let mut deps = Dependencies::new()
            .map_err(|error| format!("W03 fixture native dependencies: {error:?}"))?;
        let (controller, pin) = if matches!(case, Case::NativeDeath) {
            crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                deps.admitted.world_id,
                None,
            )
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            )
        };
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("Item index")?;
        let reference = |key: &str| {
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or_else(|| format!("W03 fixture Item index lacks {key}@definition-r1"))
        };
        let reward_chest = "oteryn:item.tibia.i19250";
        let gold = "oteryn:item.tibia.i3031";
        let (bytes, pins) = movement::bundle_with_reward_corridor(
            (reward_chest, reference(reward_chest)?.get() - 1),
            (gold, reference(gold)?.get() - 1),
        )?;
        let base = crate::map::load(&bytes, &pins)
            .map_err(|error| format!("W03 fixture bundle load: {error:?}"))?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                if key == reward_chest {
                    // Crystal201189f data/items/items.xml:39545-39552 identifies
                    // i19250 as the immovable, non-pickupable reward chest. The
                    // production Item index resolves it, but current native Item
                    // profiles do not contain its physical policy. This test map
                    // explicitly authors an obstructing static chest at x=6; it
                    // supplies no production policy fallback or custody admission.
                    return Some(crate::map::facts::ItemDefinition {
                        reference: reference(key).ok()?,
                        solid: Some(true),
                        blocks_projectile: true,
                        pickupable: false,
                        door: false,
                    });
                }
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key).ok()?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(|error| {
            format!("W03 fixture physical bundle boot: {error:?}; palette={reward_chest},{gold}")
        })?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(|error| format!("W03 fixture bundle movement binding: {error:?}"))?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (7, 0, 7),
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
            content_pin,
        )
        .map_err(|error| format!("W03 fixture Channel runtime: {error:?}"))?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(|error| format!("W03 fixture actor position: {error:?}"))?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = item_fence_of(current).ok_or("current item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let existing = if matches!(case, Case::Collection) {
            vec![(gold, 1); 17]
        } else {
            vec![]
        };
        super::item_look::seed_main_backpack(&h, &existing)
            .await
            .map_err(|error| format!("W03 fixture backpack seed: {error:?}"))?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        // ITEM_VIEW_MOVE_V1 (4) requires WORLD_SPATIAL_ENTITIES (6). Omitting 6
        // legitimately prunes both 4 and its dependent BOSS_REWARDS_V1 (224).
        let supported = [4, 6, 117, CAPABILITY_BOSS_REWARDS_V1];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &supported)
                .ok_or("boss rewards offered capability closure")?;
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            supported.as_slice(),
            "the physical reward fixture must negotiate every required capability"
        );
        if matches!(case, Case::NativeDeath) {
            native_death(&h, &owner, &mut admitted, fence).await?;
            eprintln!("BOSS_REWARDS POSTGRES_NATIVE_DEATH_COMPLETED");
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        let death = id(94);
        let definition =
            crate::gameplay_transport::item_look::typed_definition(index, reference(gold)?)
                .ok_or("gold definition")?;
        let minted = match case {
            Case::Collection
            | Case::NativeDeath
            | Case::OfflineReward
            | Case::LivePrivacy
            | Case::ConcurrentCollection => now,
            Case::Expiry => now - 604_800_000 + 1_000,
        };
        let contribution = RewardParticipation {
            damage_dealt: 100,
            damage_received: 20,
            healing_others: 10,
        };
        // The concurrent case isolates one live ordinal; all original cases retain four.
        let item_count = if matches!(case, Case::ConcurrentCollection) { 1 } else { 4 };
        let plan = BossRewardDeath {
            death,
            boss_key: "oteryn:boss.reward_fixture".into(),
            minted_at_unix_ms: u64::try_from(minted)?,
            contributors: vec![BossRewardContributor {
                character_id: fence.character_id,
                session_id: fence.game_session_id,
                lease_generation: fence.character_lease_generation,
                contribution,
            }],
            chests: vec![BossRewardChestPlan {
                character_id: fence.character_id,
                session_id: fence.game_session_id,
                lease_generation: fence.character_lease_generation,
                contribution,
                items: vec![
                    BossRewardItem {
                        definition,
                        quantity: 1
                    };
                    item_count
                ],
            }],
        };
        let mut wrong_source_lease = plan.clone();
        wrong_source_lease.contributors[0].lease_generation += 1;
        wrong_source_lease.chests[0].lease_generation += 1;
        assert!(
            !h.root
                .settle_boss_reward_chests(
                    &authority,
                    &h.node,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                    wrong_source_lease
                )
                .await?,
            "membership alone cannot prove a different captured source lease"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_deaths")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        if matches!(case, Case::OfflineReward) {
            // The server records a real ended controller. No session state or
            // issuance row is fabricated to simulate an offline participant.
            assert_eq!(
                FreshAdmissionAuthority::lose_control(
                    &owner,
                    admitted,
                    std::time::Duration::ZERO,
                    None
                )
                .await,
                crate::gameplay_transport::connection::ControlLossResult::Recorded
            );
            let retained =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0;
            assert!(retained.current_transport().is_none());
            assert!(retained.current_control_loss_epoch().is_some());
            assert_eq!(
                retained.current_character_lease().generation(),
                fence.character_lease_generation
            );
        }
        assert!(
            h.root
                .settle_boss_reward_chests(
                    &authority,
                    &h.node,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                    plan.clone()
                )
                .await?
        );
        assert!(
            h.root
                .settle_boss_reward_chests(
                    &authority,
                    &h.node,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                    plan.clone()
                )
                .await?
        );
        let frozen: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths),(SELECT count(*) FROM game_boss_reward_contributors),(SELECT count(*) FROM game_boss_reward_items)").fetch_one(&h.pool).await?;
        assert_eq!(frozen, (1, 1, i64::try_from(item_count)?));
        let mut conflict = plan;
        conflict.chests[0].items[0].quantity = 2;
        assert!(
            h.root
                .settle_boss_reward_chests(
                    &authority,
                    &h.node,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                    conflict
                )
                .await
                .is_err()
        );
        match case {
            Case::Collection => collection(&h, &owner, &mut admitted, fence, death)
                .await
                .map_err(|error| format!("W03 collection scenario: {error}"))?,
            Case::Expiry => expiry(
                &h,
                &owner,
                &mut admitted,
                fence,
                death,
                native.source_digest(),
            )
            .await
            .map_err(|error| format!("W03 expiry scenario: {error}"))?,
            Case::NativeDeath => return Err("native scenario already returned".into()),
            Case::OfflineReward => {
                assert_eq!(
                    sqlx::query_scalar::<_, i32>(
                        "SELECT item_count FROM game_character_boss_reward_chests"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    4
                );
                eprintln!("BOSS_REWARDS_OFFLINE_ENTITLEMENT POSTGRES_CASE_COMPLETED");
            }
            Case::LivePrivacy => {
                privacy::live_other_character(&h, &deps, &owner, &mut admitted, fence, death)
                    .await
                    .map_err(|error| format!("W03 live private rewards: {error}"))?;
            }
            Case::ConcurrentCollection => {
                concurrent::collect_on_two_roots(&h, &owner, fence, death)
                    .await
                    .map_err(|error| format!("W03 concurrent collection: {error}"))?;
            }
        }
        eprintln!("BOSS_REWARDS POSTGRES_CASE_COMPLETED");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn collection(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterItemFence,
    death: [u8; 16],
) -> TestResult {
    let browse = encode_browse_request(&BrowseRequest {
        death,
        after_ordinal: 0,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 1, COMMAND_TYPE_BOSS_REWARD_BROWSE, &browse).await?;
    let page = checked_browse_reply("initial browse", &bytes)?;
    assert_eq!(page.status, BossRewardStatus::Ok);
    assert_eq!(page.rows.len(), 4);
    // The role cannot consume, expire, retarget or rewrite a frozen reward without its proofs.
    assert!(
        sqlx::query("UPDATE game_boss_reward_items SET expired=true WHERE item_ordinal=1")
            .execute(&h.pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("UPDATE game_boss_reward_items SET collected_transaction_id=game_character_uuid_v7() WHERE item_ordinal=1").execute(&h.pool).await.is_err());
    assert!(
        sqlx::query("UPDATE game_boss_reward_contributors SET healing_others=0")
            .execute(&h.pool)
            .await
            .is_err()
    );
    let take = encode_take_request(&TakeRequest { death, ordinal: 1 }).map_err(debug)?;
    let (status, bytes) =
        exchange(owner, admitted, 2, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        checked_collect_reply("take or expired collection", &bytes)?,
        CollectReply {
            status: BossRewardStatus::Ok,
            moved: 1,
            remaining: 0
        }
    );
    let (status, replay) =
        exchange(owner, admitted, 2, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(status, CommandStatus::DuplicateReplay);
    assert_eq!(bytes, replay);
    let all =
        encode_collect_all_request(&CollectAllRequest { death: Some(death) }).map_err(debug)?;
    let (_, collected) = exchange(
        owner,
        admitted,
        3,
        COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &all,
    )
    .await?;
    assert_eq!(
        checked_collect_reply("partial collect-all", &collected)?,
        CollectReply {
            status: BossRewardStatus::Ok,
            moved: 2,
            remaining: 1
        }
    );
    let (_, full) = exchange(
        owner,
        admitted,
        4,
        COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &all,
    )
    .await?;
    assert_eq!(
        checked_collect_reply("full backpack collect-all", &full)?,
        CollectReply {
            status: BossRewardStatus::NoRoom,
            moved: 0,
            remaining: 1
        }
    );
    let count: (i64, i64, i32, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_item_collections),(SELECT count(*) FROM game_item_container_entries),(SELECT item_count FROM game_character_boss_reward_chests),(SELECT count(*) FROM game_boss_reward_collection_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(count, (3, 20, 1, 3));
    let audits: Vec<Vec<u8>> = sqlx::query_scalar("SELECT a.envelope FROM game_boss_reward_item_collections c JOIN game_item_audit_outbox a USING(event_id) ORDER BY c.item_ordinal").fetch_all(&h.pool).await?;
    assert_eq!(audits.len(), 3);
    for bytes in audits {
        let (envelope, mint) = crate::durability::reward_claim_mint_audit::decode_reward_claim_mint_envelope(&bytes)
            .map_err(|error| format!("W03 op4 audit decode ({} bytes): {error:?}", bytes.len()))?;
        assert_eq!(
            envelope.channel_id.as_deref(),
            Some(admitted.channel_id.as_bytes().as_slice())
        );
        assert!(envelope.instance_id.is_none());
        let source = mint.source.ok_or("source")?;
        assert!(source.reward_claim.is_none());
        assert_eq!(source.boss_reward.ok_or("boss source")?.death, death);
    }
    // A new owner call with the original command reads its durable receipt even after moving
    // away. No ephemeral row ordinal, capacity or physical-reach change can apply it again.
    movement::walk_corridor(owner, *admitted, [0, 1, 2]).await?;
    let GuildReply::Payload(replayed) = owner
        .boss_rewards_command(*admitted, 3, COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL, &all)
        .await
    else {
        return Err("durable replay payload".into());
    };
    assert_eq!(replayed, collected);
    let (_, away) = exchange(owner, admitted, 5, COMMAND_TYPE_BOSS_REWARD_BROWSE, &browse).await?;
    assert_eq!(
        checked_browse_reply("moved-away browse", &away)?.status,
        BossRewardStatus::OutOfReach
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_item_collections")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining, 3);
    cached_receipt_fences(h, owner, fence, death, &all)
        .await
        .map_err(|error| format!("W03 cached receipt fences: {error}"))?;
    let mut stale = *admitted;
    let mut stale_fence = fence;
    stale_fence.character_lease_generation += 1;
    stale.item_fence = Some(stale_fence);
    let GuildReply::Payload(refused) = owner
        .boss_rewards_command(stale, 6, COMMAND_TYPE_BOSS_REWARD_TAKE, &take)
        .await
    else {
        return Err("stale refusal payload".into());
    };
    assert_eq!(
        checked_collect_reply("stale collection refusal", &refused)?.status,
        BossRewardStatus::Rejected
    );
    paged_count::all_groups(h, owner, admitted, fence)
        .await
        .map_err(|error| format!("W03 group paging: {error}"))?;
    Ok(())
}

async fn cached_receipt_fences(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    death: [u8; 16],
    payload: &[u8],
) -> TestResult {
    use crate::durability::reward_claim_mint::boss_reward::{
        BossRewardCollectRequest, BossRewardCollectionStatus, BossRewardSelection,
    };
    use crate::durability::runtime_scope_assignment::{
        BootstrapSecret, NodeIncarnationProof, NodeRegistrationFact,
    };
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("current spell lane")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native pin")?;
    let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    let mut intent = COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL.to_be_bytes().to_vec();
    intent.extend_from_slice(payload);
    let request = BossRewardCollectRequest {
        command: crate::foundation::CommandRef::new(
            fence.game_session_id,
            crate::foundation::CommandId::new(3).map_err(debug)?,
        ),
        selection: BossRewardSelection::Group { death },
        intent,
        source_placement: String::new(),
        content_revision: String::new(),
        ruleset_revision: String::new(),
        sim_revision: String::new(),
    };
    let retained = h
        .root
        .collect_boss_rewards(
            &permit,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            &content,
            true,
        )
        .await?
        .ok_or("retained receipt")?;
    assert_eq!(retained.status, BossRewardCollectionStatus::Collected);
    assert_eq!((retained.moved, retained.remaining), (2, 1));
    let mut lease = fence;
    lease.character_lease_generation += 1;
    let mut connection = fence;
    connection.connection_generation =
        crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
            .map_err(debug)?;
    for stale in [lease, connection] {
        let rejected = h
            .root
            .collect_boss_rewards(
                &permit,
                owner.character,
                owner.holder,
                stale,
                request.clone(),
                &content,
                true,
            )
            .await?
            .ok_or("explicit stale receipt refusal")?;
        assert_eq!(rejected.status, BossRewardCollectionStatus::Rejected);
    }
    let fact = h.node.fact();
    let stale_node = NodeIncarnationProof::new(
        NodeRegistrationFact::new(fact.node_id(), fact.registration_revision() + 1),
        BootstrapSecret::from_bytes([241; 32]),
    );
    let rejected = h
        .root
        .collect_boss_rewards(
            &permit,
            owner.character,
            &stale_node,
            fence,
            request,
            &content,
            true,
        )
        .await?
        .ok_or("explicit stale incarnation refusal")?;
    assert_eq!(rejected.status, BossRewardCollectionStatus::Rejected);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_item_collections")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count, 3,
        "stale cached calls cannot mutate custody or expose a success receipt"
    );
    Ok(())
}

async fn expiry(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterItemFence,
    death: [u8; 16],
    digest: [u8; 32],
) -> TestResult {
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_items WHERE NOT expired")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(before, 4);
    // A genuine wall-clock boundary, with immutable minted_at/expiry; no metadata rewrite.
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let browse = encode_browse_request(&BrowseRequest {
        death,
        after_ordinal: 0,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 1, COMMAND_TYPE_BOSS_REWARD_BROWSE, &browse).await?;
    let page = checked_browse_reply("initial browse", &bytes)?;
    assert_eq!(page.status, BossRewardStatus::Expired);
    assert!(page.rows.is_empty());
    let after: (i64, i32) = sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_items WHERE expired),(SELECT item_count FROM game_character_boss_reward_chests)").fetch_one(&h.pool).await?;
    assert_eq!(after, (4, 0));
    let all =
        encode_collect_all_request(&CollectAllRequest { death: Some(death) }).map_err(debug)?;
    let (_, bytes) = exchange(
        owner,
        admitted,
        2,
        COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &all,
    )
    .await?;
    assert_eq!(
        checked_collect_reply("take or expired collection", &bytes)?.status,
        BossRewardStatus::Expired
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_item_collections")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let mut other = fence;
    other.character_id = crate::domain::CharacterId::from_bytes(id(96)).map_err(debug)?;
    assert!(
        h.root
            .read_boss_reward_items(owner.character, owner.holder, other, death, 0, digest)
            .await?
            .is_none()
    );
    let mut stale = fence;
    stale.connection_generation =
        crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
            .map_err(debug)?;
    assert!(
        h.root
            .read_boss_reward_items(owner.character, owner.holder, stale, death, 0, digest)
            .await?
            .is_none()
    );
    let mut forged = fence;
    forged.runtime_scope = crate::foundation::RuntimeScopeRefV1::instance(
        admitted.world_id,
        *admitted.channel_id.as_bytes(),
    )
    .map_err(debug)?;
    assert!(
        h.root
            .read_boss_reward_items(owner.character, owner.holder, forged, death, 0, digest)
            .await?
            .is_none()
    );
    let missing = encode_browse_request(&BrowseRequest {
        death: id(97),
        after_ordinal: 0,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(
        owner,
        admitted,
        3,
        COMMAND_TYPE_BOSS_REWARD_BROWSE,
        &missing,
    )
    .await?;
    assert_eq!(
        checked_browse_reply("unknown death browse", &bytes)?.status,
        BossRewardStatus::Empty
    );
    Ok(())
}

fn checked_browse_reply(stage: &str, bytes: &[u8]) -> TestResult<BrowseReply> {
    decode_browse_reply(bytes).map_err(|error| {
        format!(
            "W03 {stage}: browse reply decode {error:?}; len={} prefix={:?}",
            bytes.len(),
            &bytes[..bytes.len().min(8)],
        )
        .into()
    })
}

fn checked_collect_reply(stage: &str, bytes: &[u8]) -> TestResult<CollectReply> {
    decode_collect_reply(bytes).map_err(|error| {
        format!(
            "W03 {stage}: collection reply decode {error:?}; len={} prefix={:?}",
            bytes.len(),
            &bytes[..bytes.len().min(8)],
        )
        .into()
    })
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, id);
                // Command 340 retains its existing 64-group metadata page bound;
                // W03 commands 428-430 retain their separate 1 KiB reply bound.
                // This helper exercises both protocols, so check the owning one.
                let result_bound = match kind {
                    oteryn_protocol_oteryn::boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN => {
                        oteryn_protocol_oteryn::boss_encounter::MAX_RESULT_BYTES
                    }
                    COMMAND_TYPE_BOSS_REWARD_BROWSE
                    | COMMAND_TYPE_BOSS_REWARD_TAKE
                    | COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL => MAX_RESULT_BYTES,
                    _ => return Err(format!("unsupported reward fixture command {kind}").into()),
                };
                assert!(
                    result.payload.len() <= result_bound,
                    "command {kind} result {} exceeds its bound {result_bound}",
                    result.payload.len()
                );
                if !matches!(
                    result.status,
                    CommandStatus::Accepted | CommandStatus::DuplicateReplay
                ) {
                    return Err(format!(
                        "W03 outer command result id={id} kind={kind} next={} status={:?}; payload_len={} prefix={:?}",
                        before.continuity.next_command_id, result.status, result.payload.len(),
                        &result.payload[..result.payload.len().min(8)],
                    ).into());
                }
                break (result.status, result.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("boss reward command {id} ended {ended:?}: {error}").into())
}
