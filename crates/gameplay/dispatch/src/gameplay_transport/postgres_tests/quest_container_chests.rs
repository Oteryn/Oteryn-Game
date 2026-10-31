//! Pinned source selectors -> literal USE -> existing once-claim owner -> full
//! container custody, key/achievement facts, replay and atomic refusal.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world, source_world_choices, wire_use_targets, wire_uses};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[derive(Clone, Copy)]
enum Case { Bear, Key, Choice, Text, RootText, CrystalKey, FullCharge, Full, Weight }

fn execute(case: Case, marker: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()) }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join().map_err(|_| "reward container scenario panicked")??;
    println!("{marker} POSTGRES_CASE_COMPLETED");
    Ok(())
}

#[test]
fn quest_container_chests_bear_bag_once_and_replay() -> TestResult {
    execute(Case::Bear, "W15B_BEAR_CONTAINER")
}
#[test]
fn quest_container_chests_key_attributes_are_atomic() -> TestResult {
    execute(Case::Key, "W15B_KEY_CONTAINER")
}
#[test]
fn quest_container_chests_annihilator_choices_share_storage_and_achievement() -> TestResult {
    execute(Case::Choice, "W15B_STORAGE_CHOICE")
}
#[test]
fn quest_container_chests_authored_book_text_survives_replay() -> TestResult {
    execute(Case::Text, "W15B_TEXT_CONTAINER")
}
#[test]
fn quest_container_chests_leaf_text_is_frozen_and_replayed() -> TestResult {
    execute(Case::RootText, "W15B_LEAF_TEXT")
}
#[test]
fn quest_container_chests_crystal_key3002_is_minted_once() -> TestResult {
    execute(Case::CrystalKey, "W15B_CRYSTAL_KEY")
}
#[test]
fn quest_container_chests_full_definition_charges_mint_one_instance() -> TestResult {
    execute(Case::FullCharge, "W15B_FULL_CHARGES")
}
#[test]
fn quest_container_chests_full_backpack_writes_no_pending_claim() -> TestResult {
    execute(Case::Full, "W15B_CONTAINER_FULL")
}
#[test]
fn quest_container_chests_capacity_counts_every_child_before_reservation() -> TestResult {
    execute(Case::Weight, "W15B_CONTAINER_WEIGHT")
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef { family: "Item".into(), production_key: key.into(), revision_ref: "definition-r1".into() }
}

fn scenario(case: Case) -> TestResult {
    let (claim, chest, unique, root_key, expected) = match case {
        Case::RootText => ("oteryn:reward-claim.kv.powerbolt2",
            (32814, 32281, -8), 6112, "oteryn:item.tibia.i2821", vec![]),
        Case::CrystalKey => ("oteryn:reward-claim.quest.key.id3002",
            (32802, 31576, -7), 6093, "oteryn:item.tibia.i2969", vec![]),
        Case::FullCharge => ("oteryn:reward-claim.kv.madmageroom3",
            (32572, 32195, -14), 6082, "oteryn:item.tibia.i3081", vec![]),
        Case::Key => ("oteryn:reward-claim.quest.u6_1.emperors_cookies.rewards.cookies",
            (32648, 31905, -3), 5004, "oteryn:item.tibia.i2853",
            vec![("oteryn:item.tibia.i2970", 1_i64, 3801_i32), ("oteryn:item.tibia.i3598", 27, 0)]),
        Case::Choice => ("oteryn:reward-claim.quest.u7_24.the_annihilator.reward",
            (33233, 31656, -13), 6088, "oteryn:item.tibia.i2856",
            vec![("oteryn:item.tibia.i3213", 1, 0)]),
        Case::Text => ("oteryn:reward-claim.quest.u7_9.dreamers_challenge.bplong_sword",
            (32860, 32249, -9), 6195, "oteryn:item.tibia.i2854",
            vec![("oteryn:item.tibia.i2816", 1, 0), ("oteryn:item.tibia.i3285", 1, 0),
                 ("oteryn:item.tibia.i3352", 1, 0), ("oteryn:item.tibia.i3558", 1, 0)]),
        _ => ("oteryn:reward-claim.kv.bear_chest1", (32146, 32097, -11), 6114,
            "oteryn:item.tibia.i2853",
            vec![("oteryn:item.tibia.i3447", 12, 0), ("oteryn:item.tibia.i3031", 40, 0)]),
    };
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "quest_container", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id, "content-1")?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = if matches!(case, Case::Choice) {
            source_world_choices(chest, &[(33227,6085),(33229,6086),(33231,6087),(33233,6088)], 2472)?
        } else {
            let appearance = match case {
                Case::Text | Case::RootText => 4240,
                Case::CrystalKey => 2474,
                _ => 2472,
            };
            source_world(chest, unique, appearance, 0)?
        };
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(world.base().as_ref(), &|_| true)?;
        assert!(report.served.iter().any(|row| row.key.as_str() == claim), "source claim did not bind");
        let cells = world.movement_cells(deps.room.movement_cells()).map_err(debug)?;
        let pin = ChannelContentPin::from_activation(deps.admitted.world_id, 1,
            generation.identity().server_artifact_digest(), generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(), world.map_revision_digest(),
            (i32::from(chest.0)-1,i32::from(chest.1),-i16::from(chest.2)));
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id,
            deps.admitted.channel_id, crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,1,1,"runtime-scope-assignment:1",2,pin).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(deps.actor, MovementLocalPosition {
            x:i32::from(chest.0)-1,y:i32::from(chest.1),floor:-i16::from(chest.2),
        }).map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(deps.admitted.game_session_id).await?.0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness.root.initialize_admission_equipment(&authority,&harness.node,&fence,
            generation.identity().server_artifact_digest(),fresh.operation()).await.map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition:definition("oteryn:item.tibia.i2854"),
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        harness.root.grant_starter_items(&authority,&harness.node,&fence,&weights,&template).await.map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let target_at = |x| -> TestResult<WorldObjectTarget> {
            let key = oteryn_world_bundle::bundle::placement_key(chest.2,x,chest.1,1).ok_or("placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget { placement, expected_revision:0 })
        };
        let target = target_at(chest.0)?;
        {
            let mut owner = deps.owner(&harness,&authority);
            owner.active_generation = Some(generation); owner.movement_cells = &cells;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut stale = admitted; let mut stale_fence = fence;
            stale_fence.character_lease_generation += 1; stale.item_fence = Some(stale_fence);
            assert_eq!(wire_uses(&owner,stale,&[1],&target).await?,[UseDisposition::Rejected]);
            let count:i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
                .fetch_one(&harness.pool).await?;
            assert_eq!(count,0,"stale authority cannot reserve any part of the reward");
        }
        if matches!(case,Case::Full|Case::Weight) {
            // Establish real durable before-state through the existing leaf
            // owner. The tested action remains the literal native chest USE.
            let backpack = generation_item_facts(native,&template.definition).map_err(debug)?;
            let coin = generation_item_facts(native,&definition("oteryn:item.tibia.i3031")).map_err(debug)?;
            let burdens = if matches!(case,Case::Full) {
                vec![(coin,1);20]
            } else {
                let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
                let vocation: Option<String> = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&harness.pool).await?;
                let capacity = oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(
                    u32::try_from(level)?,vocation.as_deref().unwrap_or("none"),
                ).map_err(debug)?;
                let root = generation_item_facts(native,&definition(root_key)).map_err(debug)?;
                let root_weight = u64::from(root.weight.ok_or("native root weight")?);
                let backpack_weight = u64::from(backpack.weight.ok_or("native backpack weight")?);
                let mut child_weight = 0_u64;
                for (key, count, _) in &expected {
                    let child = generation_item_facts(native,&definition(key)).map_err(debug)?;
                    child_weight += u64::from(child.weight.ok_or("native child weight")?) * u64::try_from(*count)?;
                }
                // This admitted chain helmet is already in the native chest
                // closure; the giant sword has no native profile in this pin.
                let helmet = generation_item_facts(native,&definition("oteryn:item.tibia.i3352")).map_err(debug)?;
                let helmet_weight = u64::from(helmet.weight.ok_or("native burden weight")?);
                let coin_weight = u64::from(coin.weight.ok_or("native coin weight")?);
                assert!(helmet_weight > 0 && coin_weight > 0 && child_weight > 0);
                let budget = capacity.checked_sub(backpack_weight + root_weight).ok_or("capacity for root")?;
                let helmets = budget / helmet_weight;
                let mut coins = (budget % helmet_weight) / coin_weight;
                let carried = backpack_weight + helmets * helmet_weight + coins * coin_weight;
                assert!(carried <= capacity, "the before-state itself must fit");
                assert!(carried + root_weight <= capacity, "the empty root must fit");
                assert!(carried + root_weight + child_weight > capacity,
                    "only including the actual source children must cause refusal");
                let maximum = match coin.stack {
                    crate::durability::item_transfer::ItemStackClass::Stackable { proven_maximum: Some(maximum) }
                        if (1..=100).contains(&maximum) => u64::from(maximum),
                    _ => return Err("native coin stack maximum".into()),
                };
                let mut burdens = vec![(helmet,1);usize::try_from(helmets)?];
                while coins > 0 {
                    let count = coins.min(maximum);
                    burdens.push((coin.clone(),u32::try_from(count)?));
                    coins -= count;
                }
                assert!(burdens.len() < usize::try_from(backpack.container_capacity.ok_or("backpack capacity")?)?,
                    "the incoming root still needs a free direct slot");
                println!("W15B_CONTAINER_WEIGHT_BOUNDARY capacity={capacity} before={carried} root={root_weight} children={child_weight} entries={}",burdens.len());
                burdens
            };
            for (index,(item,quantity)) in burdens.into_iter().enumerate() {
                let request = RewardClaimMintRequest {
                    command:CommandRef::new(admitted.game_session_id,CommandId::new(100+u64::try_from(index)?).map_err(debug)?),
                    claim:TypedDefinitionRef { family:"RewardClaim".into(),
                        production_key:format!("fixture:reward.container-burden.{index}"),revision_ref:"definition-r1".into() },
                    source_placement:"fixture:placement.container-burden".into(), item,quantity,
                    contents:Vec::new(),
                    root_text: None,backpack:backpack.clone(),content_revision:"content-1".into(),
                    ruleset_revision:"ruleset-1".into(),sim_revision:"sim-1".into(),achievement:None,
                    quest_transition:None,key_number:None,
                };
                let mut candidate = harness.root.freeze_reward_claim_mint(&authority,&harness.node,fence,request).await.map_err(debug)?;
                harness.root.commit_reward_claim_mint(&authority,&harness.node,fence,&mut candidate).await.map_err(debug)?;
            }
            {
                let mut owner = deps.owner(&harness,&authority);
                owner.active_generation=Some(generation);owner.movement_cells=&cells;
                owner.admit_progression_session(&mut admitted).await;deps.initialize(&owner).await;
                assert_eq!(wire_use_targets(&owner,admitted,
                    &[(1,&target),(1,&target),(2,&target)]).await?,[
                        (CommandStatus::Rejected,UseDisposition::Rejected),
                        (CommandStatus::DuplicateReplay,UseDisposition::Rejected),
                        (CommandStatus::Rejected,UseDisposition::Rejected),
                    ]);
            }
            let counts:(i64,i64,i64) = sqlx::query_as("SELECT \
                (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), \
                (SELECT count(*) FROM game_reward_claim_mint_reservations WHERE claim_production_key=$1), \
                (SELECT count(*) FROM game_item_nested_entries)")
                .bind(claim).fetch_one(&harness.pool).await?;
            assert_eq!(counts,(0,0,0),"room/weight refusal must leave no partial or pending reward");
        } else {
            let alternate = if matches!(case,Case::Choice) { target_at(33231)? } else { target.clone() };
            let mut first_root = None;
            // One live stream starts at 1. Replay its retained result before
            // command 2 replaces the single terminal-record slot. A new owner
            // then starts at 1 again, proving replay from the durable receipt
            // without an in-memory command-result cache.
            for cold_replay in [false,true] {
                let mut owner = deps.owner(&harness,&authority);
                owner.active_generation=Some(generation);owner.movement_cells=&cells;
                owner.admit_progression_session(&mut admitted).await;deps.initialize(&owner).await;
                if cold_replay {
                    assert_eq!(wire_use_targets(&owner,admitted,&[(1,&target)]).await?,[
                        (CommandStatus::Accepted,UseDisposition::Committed),
                    ]);
                } else {
                    assert_eq!(wire_use_targets(&owner,admitted,
                        &[(1,&target),(1,&target),(2,&alternate)]).await?,[
                            (CommandStatus::Accepted,UseDisposition::Committed),
                            (CommandStatus::DuplicateReplay,UseDisposition::Committed),
                            (CommandStatus::Accepted,UseDisposition::NothingToUse),
                        ]);
                }
                let bag = harness.root.read_character_backpack(&authority,fence.character_id).await.map_err(debug)?.ok_or("backpack")?;
                assert_eq!(bag.entries.len(),1);let root=&bag.entries[0].item;
                assert_eq!(root.quantity,1);assert_eq!(root.definition.production_key,root_key);
                if let Some(original)=first_root { assert_eq!(root.item_instance_id,original) }
                else { first_root=Some(root.item_instance_id) }
                let children:Vec<(String,i64,i32)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity,COALESCE(k.key_number,0) \
                    FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) \
                    LEFT JOIN game_item_keys k USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal")
                    .bind(root.item_instance_id.as_slice()).fetch_all(&harness.pool).await?;
                assert_eq!(children,expected.iter().map(|(key,count,number)|((*key).into(),*count,*number)).collect::<Vec<_>>());
                let counts:(i64,i64,i64,i64) = sqlx::query_as("SELECT \
                    (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), \
                    (SELECT count(*) FROM game_reward_claim_mint_receipts), \
                    (SELECT count(*) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid)), \
                    (SELECT count(DISTINCT created_xact_id::text) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid))")
                    .bind(claim).bind(root.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(counts,(1,1,1+expected.len() as i64,1));
                if matches!(case, Case::CrystalKey) {
                    let key: (i32,bool,bool) = sqlx::query_as("SELECT k.key_number, \
                        k.minted_transaction_id=r.transaction_id,k.created_xact_id=r.created_xact_id \
                        FROM game_item_keys k JOIN game_reward_claim_mint_receipts r USING(item_instance_id) \
                        WHERE k.item_instance_id=encode($1,'hex')::uuid")
                        .bind(root.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(key,(3002,true,true));
                }
                if matches!(case, Case::FullCharge) {
                    use crate::content::ReferenceItemField::Known;
                    let policy = native.item_policy(root_key,"definition-r1").ok_or("native charge profile")?;
                    assert!(matches!(&policy.record().semantics.charges,
                        Known(charges) if charges.count == Known(5)), "native full charges must match source five");
                    let states: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
                        .bind(root.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(states,0,"the existing timed owner interprets absent state as full definition values");
                }
                if matches!(case, Case::RootText) {
                    let text: (String,i64,i64,bool,bool,bool) = sqlx::query_as("SELECT \
                        encode(sha256(convert_to(t.text,'UTF8')),'hex'),octet_length(t.text)::bigint, \
                        t.text_revision::bigint,t.writer_character_id IS NULL, \
                        t.written_at=to_timestamp(r.occurred_at::double precision/1000.0), \
                        t.text=r.root_text AND r.root_text=v.root_text AND x.last_revision=1 \
                          AND r.contents_capacity IS NULL AND r.contents_plan='[]'::jsonb \
                        FROM game_item_texts t JOIN game_item_text_revisions x USING(item_instance_id) \
                        JOIN game_reward_claim_mint_receipts r USING(item_instance_id) \
                        JOIN game_reward_claim_mint_reservations v ON v.transaction_id=r.transaction_id \
                        WHERE t.item_instance_id=encode($1,'hex')::uuid")
                        .bind(root.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(text,("9ea894f92d1b6e4cf60555cd08f5e552b8c5ce59c11c13eb6d35f7ff07abb036".into(),
                        1505,1,true,true,true),"existing source LF normalization is part of the exact frozen body");
                    let writes: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_text_write_receipts")
                        .fetch_one(&harness.pool).await?;
                    assert_eq!(writes,0);
                }
                if matches!(case, Case::Text) {
                    // This certifies the source book's durable Text grant, not
                    // a READ/TextWindow transport that this repository lacks.
                    let texts: Vec<(String,String,i64,i64,bool,bool,bool)> = sqlx::query_as("SELECT \
                        i.definition_production_key,encode(sha256(convert_to(t.text,'UTF8')),'hex'), \
                        octet_length(t.text)::bigint,t.text_revision::bigint,t.writer_character_id IS NULL, \
                        t.written_at=to_timestamp(r.occurred_at::double precision/1000.0), \
                        t.text=r.contents_plan->0->>'text' AND x.last_revision=1 \
                        FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) \
                        JOIN game_item_texts t USING(item_instance_id) JOIN game_item_text_revisions x USING(item_instance_id) \
                        JOIN game_reward_claim_mint_receipts r ON r.item_instance_id=e.parent_item_instance_id \
                        WHERE e.parent_item_instance_id=encode($1,'hex')::uuid")
                        .bind(root.item_instance_id.as_slice()).fetch_all(&harness.pool).await?;
                    assert_eq!(texts, vec![("oteryn:item.tibia.i2816".into(),
                        "484f70e524471dc25e2dc695deae9e697aa11497a1d6d7d6d32b2e79f3d7f476".into(),
                        478,1,true,true,true)]);
                    let writes: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_text_write_receipts")
                        .fetch_one(&harness.pool).await?;
                    assert_eq!(writes,0,"authored text must not forge a player rewrite receipt");
                }
            }
            let achievements:i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/annihilator'")
                .fetch_one(&harness.pool).await?;
            assert_eq!(achievements,i64::from(matches!(case,Case::Choice)));
            if matches!(case,Case::Text) {
                let valid: (bool,bool,bool,bool,bool,bool) = sqlx::query_as("SELECT \
                    game_reward_claim_contents_plan_valid(contents_plan,contents_capacity), \
                    game_reward_claim_contents_plan_valid(contents_plan#-'{0,text}',contents_capacity), \
                    game_reward_claim_contents_plan_valid(jsonb_set(contents_plan,'{0,text}','42'),contents_capacity), \
                    game_reward_claim_contents_plan_valid(jsonb_set(contents_plan,'{0,text}',to_jsonb(repeat('x',7997))),contents_capacity), \
                    game_reward_claim_contents_plan_valid(jsonb_set(contents_plan,'{0,text}',to_jsonb(repeat('x',4001))),contents_capacity), \
                    game_reward_claim_contents_plan_valid(jsonb_set(contents_plan,'{0,quantity}','2'),contents_capacity) \
                    FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1")
                    .bind(claim).fetch_one(&harness.pool).await?;
                assert_eq!(valid,(true,false,false,false,false,false),"text plan must remain complete, bounded and singleton");
            }
            // A later transaction cannot truncate the frozen plan to make a
            // partial reward look complete. The immutable owner guard rejects it.
            let mutation = if expected.is_empty() {
                "UPDATE game_reward_claim_mint_reservations SET root_text=COALESCE(root_text,'')||'tamper' WHERE claim_production_key=$1"
            } else {
                "UPDATE game_reward_claim_mint_reservations SET contents_plan=contents_plan-0 WHERE claim_production_key=$1"
            };
            let changed = sqlx::query(mutation)
                .bind(claim).execute(&harness.pool).await;
            assert!(matches!(changed,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514")));
        }
        drop(authority);drop(seal);harness.cleanup().await
    })
}
