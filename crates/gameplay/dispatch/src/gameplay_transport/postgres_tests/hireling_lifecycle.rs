//! E12 native lamp custody, exact House authority and immutable replay.
//! Item creation uses the production native Inbox mint owner. Only the independent
//! Store entitlement is seeded; Store fulfilment has its own integration suite.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::durability::character_inbox::{RewardClaimInboxError, RewardClaimInboxMintOutput,
    RewardClaimInboxMintRequest, RewardClaimInboxOutcome, RewardClaimInboxPlan};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_hireling::{HirelingLampInput, HirelingLampPlacement, HirelingLampReturn, HirelingReturnInput, HIRELING_LAMP_KEY};
use crate::durability::house_scope_handoff::{HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{CurrentCharacterItemFence, ItemDefinitionFacts,
    ItemStackClass, ItemWeightLookup, ItemWeightRefused};
use crate::durability::item_use::{ItemUseFlaskPlan, ItemUseKind, ItemUseRequest};
use crate::durability::runtime_scope_assignment::{BootstrapSecret, ControlActor,
    NodeIncarnationProof, NodeRegistrationFact, OperationKey};
use crate::foundation::{ChannelContentPin, CharacterId, CharacterLease, CommandId,
    CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::hireling::management::{Refusal, Reply, Request};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
#[path = "hireling_lifecycle/legacy.rs"]
mod legacy;
#[path = "hireling_lifecycle/bank.rs"]
mod bank;
#[path = "hireling_lifecycle/acl.rs"]
mod acl;
#[path = "hireling_lifecycle/wire.rs"]
mod wire;
#[path = "hireling_lifecycle/legacy_wire.rs"]
mod legacy_wire;
#[path = "hireling_lifecycle/trader.rs"]
mod trader;
#[path = "hireling_lifecycle/gourmet.rs"]
mod gourmet;
#[path = "hireling_lifecycle/cook.rs"]
mod cook;
#[path = "hireling_lifecycle/steward.rs"]
mod steward;
#[path = "hireling_lifecycle/furniture.rs"]
mod furniture;
#[path = "hireling_lifecycle/lamp_pending.rs"]
mod lamp_pending;
#[path = "hireling_lifecycle/position.rs"]
mod position;

const OTHER_HOUSE: &str = "oteryn:content.house.admiral_s_avenue_2";
#[derive(Clone, Copy)]
enum Case { Lifecycle, InboxFull, LegacyConversion, Bank, GuestAcl, Wire, LegacyWire, Trader, Gourmet, Cook, CookBank, Steward, Furniture, LampPending, Store, PositionCapture, PositionRelog, PositionProcessLoss, PositionAcl, PositionOccupied }

#[test]
fn hireling_postgres_native_inbox_lamp_fences_replay_and_return() -> TestResult {
    run_case(Case::Lifecycle)
}
#[test]
fn hireling_postgres_full_inbox_refusal_then_offline_recall_and_replay() -> TestResult {
    run_case(Case::InboxFull)
}
#[test]
fn hireling_postgres_0405_upgrade_preserves_owned_banker_and_real_provenance() -> TestResult {
    run_case(Case::LegacyConversion)
}
#[test]
fn hireling_postgres_scoped_bank_deposit_withdraw_and_replay() -> TestResult {
    run_case(Case::Bank)
}
#[test]
fn hireling_postgres_guest_service_rechecks_acl_and_house_content_fence() -> TestResult {
    run_case(Case::GuestAcl)
}
#[test]
fn hireling_postgres_client_use_management_durable_replay_and_lost_ingress_refusal() -> TestResult {
    run_case(Case::Wire)
}
#[test]
fn hireling_postgres_command220_durable_replay_and_lost_ingress_refusal() -> TestResult {
    run_case(Case::LegacyWire)
}
#[test]
fn hireling_postgres_trader_maximum_price_no_room_and_replay() -> TestResult {
    run_case(Case::Trader)
}
#[test]
fn hireling_postgres_gourmet_use_audit_cooldown_and_persistent_conditions() -> TestResult {
    run_case(Case::Gourmet)
}
#[test]
fn hireling_postgres_cook_carrot_purchase_package_and_replay() -> TestResult {
    run_case(Case::Cook)
}
#[test]
fn hireling_postgres_cook_bank_only_payment_without_backpack() -> TestResult {
    run_case(Case::CookBank)
}
#[test]
fn hireling_postgres_steward_native_stash_replay_and_revoked_acl() -> TestResult {
    run_case(Case::Steward)
}
#[test]
fn hireling_postgres_trader_wrapped_furniture_native_house_roundtrip() -> TestResult {
    run_case(Case::Furniture)
}
#[test]
fn hireling_postgres_lamp_pending_native_commit_and_refusal_recovery() -> TestResult {
    run_case(Case::LampPending)
}
#[test]
fn hireling_postgres_store_named_house_lamp_entitlement_and_replay() -> TestResult {
    run_case(Case::Store)
}
#[test]
fn hireling_postgres_house_position_ordered_capture_and_same_actor_resume() -> TestResult {
    run_case(Case::PositionCapture)
}
#[test]
fn hireling_postgres_house_logout_final_capture_and_authenticated_fresh_relog() -> TestResult {
    run_case(Case::PositionRelog)
}
#[test]
fn hireling_postgres_house_process_loss_refuses_started_namespace_then_fresh_restores() -> TestResult {
    run_case(Case::PositionProcessLoss)
}
#[test]
fn hireling_postgres_house_acl_loss_gourmet_logout_and_fresh_refusal_cleanup() -> TestResult {
    run_case(Case::PositionAcl)
}
#[test]
fn hireling_postgres_house_fresh_restore_refuses_current_hireling_collision() -> TestResult {
    run_case(Case::PositionOccupied)
}
fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20).spawn(move || {
        run(async move |admin| {
            // The fresh-relog composition case obtains its progression from the
            // actual native admission owner, not the generic level-50 row seed.
            let initialized = !matches!(case, Case::PositionRelog);
            let h = Harness::create_with_admission_seed(admin, "hireling_lifecycle", initialized, false).await?;
            let result = scenario(&h, case).await;
            h.cleanup().await?;
            result?;
            println!("HIRELING_LIFECYCLE POSTGRES_CASE_COMPLETED");
            Ok(())
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "hireling lifecycle thread panicked")??;
    Ok(())
}

async fn scenario(h: &Harness, case: Case) -> TestResult {
    super::reset();
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let world = hireling_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    // Qualify movement under the very same full native input as the active
    // Item provider. The small Dependencies::new entry artifact has another
    // digest and cannot authorize a STEP against this full generation.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let mut input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(&manifest)
        .map_err(debug)?;
    // This scenario boots its genuine bounded BundleWorld separately.
    input.source_world = None;
    if matches!(case, Case::PositionCapture | Case::PositionRelog | Case::PositionProcessLoss
        | Case::PositionAcl | Case::PositionOccupied) {
        // Use the same qualified training fixture as the existing native
        // death/relog tests: source formula bytes are unchanged, while its
        // binding names this fixture's actual Character interpretation.
        input.build_training.as_mut().ok_or("manifest training formula")?
            .content_revision = "content-1".into();
    }
    let room = crate::content::qualify_selected_native_gameplay_room(base.admitted.world_id, &input)
        .map_err(debug)?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id: base.admitted.world_id,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    crate::content::activate_native_entry_room_with_gameplay(&mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        base.admitted.world_id, &issuance, &input).map_err(debug)?;
    let generation = controller.active().ok_or("active content")?;
    assert_eq!(room.compiled().server_digest(), generation.identity().server_artifact_digest(),
        "fixture movement and Item facts must share one qualified native generation");
    // Publish the same qualified artifact through the existing control-plane
    // owner. Native House equipment/STEP reads require the durable current
    // activation; an in-memory provider alone does not authorize that read.
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(base.admitted.world_id.as_bytes().as_slice())
        .bind(base.admitted.channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    assert!(h.root.record_content_activation(
        &crate::durability::content_activation::ContentActivationRequest {
            world_id: base.admitted.world_id,
            channel_id: base.admitted.channel_id,
            activation_sequence: 1,
            previous_sequence: None,
            server_artifact_digest: room.compiled().server_digest(),
            client_artifact_digest: room.compiled().client_digest(),
            frame_binding_digest: room.frame_binding().digest(),
        },
    ).await.map_err(debug)?);
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    use sha2::Digest as _;
    let map_digest = sha2::Sha256::digest(cells.scope().map_revision.as_str().as_bytes()).into();
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        map_digest,
        (32309, 32805, 7),
    );
    let deps = Dependencies::with_content(pin, room, base.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(generation);
    let house_store = matches!(case, Case::Store).then(||
        super::hireling_store::install_house_store(&mut owner, deps.admitted.world_id.as_bytes())
    );
    // This fixture boots the bounded command set it will carry through the real handoff.
    const OFFERS: &[crate::gameplay_transport::capabilities::OfferedCapability] = &[
        crate::gameplay_transport::capabilities::OfferedCapability { id: 3, requires: &[] },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 6,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 15, requires: &[4] },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 21, requires: &[] },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 37, requires: &[4] },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 49,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 51, requires: &[] },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 62, requires: &[37] },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 86,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 154, requires: &[] },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 165,
            requires: &[3, 51],
        },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 207, requires: &[49] },
        crate::gameplay_transport::capabilities::OfferedCapability { id: 250, requires: &[165] },
    ];
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
    ).map_err(debug)?;

    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    // Genuine empty House cell for source-wrapped furniture. The fixture uses
    // the same spatial-position bytes as the native House wrap PostgreSQL test.
    let furniture_position: Vec<u8> = [32311_i32, 32804, 7].into_iter().flat_map(i32::to_be_bytes).collect();
    let mut tiles = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tiles).await?;
    sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
        .bind(id(42).as_slice()).bind(HOUSE).bind(furniture_position).execute(&mut *tiles).await?;
    tiles.commit().await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    // Reuse the genuine initial FreshAdmission receipt, before leaving its
    // Channel scope, to initialize the native equipment owner's empty seed.
    // The House STEP reader deliberately never creates missing equipment state.
    let admission = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    let equipment = h.root.initialize_admission_equipment(&authority, &h.node,
        &admitted.item_fence.ok_or("source admission equipment fence")?,
        generation.identity().server_artifact_digest(), admission.operation())
        .await.map_err(debug)?;
    assert!(equipment.items.is_empty());
    if matches!(case, Case::PositionCapture | Case::PositionRelog | Case::PositionProcessLoss
        | Case::PositionAcl | Case::PositionOccupied) {
        // This source is really positioned in its Channel. Use the ordinary
        // native login settlement and runtime install before the House handoff.
        // Other service fixtures deliberately retain their revision-one seed.
        if matches!(case, Case::PositionRelog) {
            owner.admit_progression_session(&mut admitted).await;
            assert!(owner.progression_binding(admitted.game_session_id).is_some(),
                "the genuine initial source binds the production progression policy");
        }
        assert!(owner.admit_hazard(admitted).await,
            "the initial Channel source admits its genuine Hazard selection before House recovery reads it");
        owner.admit_loop_training(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
    }
    // As in the existing Store PostgreSQL fixture, capability154 is offered
    // only by this test producer. It is selected in the genuine source session
    // against the same boot offer already bound above, before the native handoff.
    let offers = if matches!(case, Case::Store) { OFFERS } else { owner.offered_capabilities() };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(offers, &[3, 4, 6, 15, 21, 37, 49, 51, 62, 86, 154, 165, 207, 250])
            .ok_or("hireling offer")?;
    let source_selection = admitted.continuity.selected_capabilities;

    let mut transfer = vec![8, 1, 18, HOUSE.len() as u8];
    transfer.extend(HOUSE.as_bytes());
    let (inside, _) = owner.transfer_house(admitted, 1, &transfer).await.ok_or("house transfer outcome")?;
    let mut admitted = inside.ok_or("house transfer admission")?;
    assert_eq!(admitted.continuity.selected_capabilities, source_selection,
        "the native House handoff retains the complete authenticated source selection");
    if !matches!(case, Case::Wire) {
        wire::enter_interior(&owner, &mut admitted, deps.actor).await?;
    }
    if matches!(case, Case::Store) {
        return super::hireling_store::house_scenario(
            h, &owner, admitted, house_store.ok_or("House Store producer")?,
        ).await;
    }
    if matches!(case, Case::PositionCapture) {
        return position::capture(h, &owner, admitted, deps.actor).await;
    }
    if matches!(case, Case::PositionRelog) {
        return position::logout_and_relog(h, &owner, admitted, deps.actor).await;
    }
    if matches!(case, Case::PositionAcl) {
        return position::revoked_access(h, &owner, admitted, deps.actor).await;
    }
    if matches!(case, Case::PositionProcessLoss) {
        let (admitted, saved) = position::before_process_loss(h, &owner, admitted, deps.actor).await?;
        drop(owner);
        // Replace the entire carrier and process-local owner, exactly as the
        // native House transfer process-loss fixture does. The committed node
        // assignment and durable current position remain intact.
        let pin = deps.runtime.lock().await.content_pin().clone();
        *deps.runtime.lock().await = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id, deps.admitted.channel_id, h.node.fact().node_id(),
            h.node.fact().registration_revision(), 1, 1, "runtime-scope-assignment:1", 2, pin,
        ).map_err(debug)?;
        let mut restarted = deps.owner(h, &authority);
        restarted.movement_cells = &cells;
        restarted.evidence = &evidence.evidence;
        restarted.active_generation = Some(generation);
        restarted.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
        ).map_err(debug)?;
        return position::after_process_loss(h, &restarted, admitted, saved).await;
    }
    let current = admitted.item_fence.ok_or("current House item fence")?;
    if matches!(case, Case::LegacyConversion) {
        let converted = legacy::upgrade(h, current).await?;
        return legacy::recall_after_session_retirement(h, &authority, current, converted).await;
    }
    let origin = admitted.channel_id;
    if matches!(case, Case::GuestAcl) {
        return acl::scenario(h, &authority, current, origin).await;
    }
    if matches!(case, Case::Wire) {
        let replay = wire::scenario(h, &authority, &owner, admitted, deps.actor).await?;
        drop(owner);
        let mut restarted = deps.owner(h, &authority);
        restarted.movement_cells = &cells;
        restarted.evidence = &evidence.evidence;
        restarted.active_generation = Some(generation);
        restarted.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
        ).map_err(debug)?;
        return wire::replay(h, &restarted, replay).await;
    }
    if matches!(case, Case::LegacyWire) {
        let replay = legacy_wire::scenario(h, &authority, &owner, admitted, deps.actor).await?;
        drop(owner);
        let mut restarted = deps.owner(h, &authority);
        restarted.movement_cells = &cells;
        restarted.evidence = &evidence.evidence;
        restarted.active_generation = Some(generation);
        restarted.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
        ).map_err(debug)?;
        let replay = legacy_wire::replay_after_ingress_loss(h, &restarted, replay).await?;
        drop(restarted);
        let mut final_owner = deps.owner(h, &authority);
        final_owner.movement_cells = &cells;
        final_owner.evidence = &evidence.evidence;
        final_owner.active_generation = Some(generation);
        final_owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
        ).map_err(debug)?;
        return legacy_wire::replay_on_second_owner(h, &final_owner, replay).await;
    }
    let hireling = id(141);
    let lamp = id(142);
    assert_eq!(h.root.query_hirelings(&authority, &h.node, current,
        *origin.as_bytes(), 100).await.map_err(debug)?, Reply::State(Vec::new()));
    let source = inbox_source(current, origin, 101, 143, vec![lamp])?;
    let mut mint = mint_request(vec![lamp_output(0)]);
    if matches!(case, Case::LampPending) {
        // This scenario enters the actual wire definition path, so its initial
        // Store Inbox lamp must carry the qualified canonical definition too.
        let definition = {
            let runtime = owner.runtime.lock().await;
            let index = crate::gameplay_transport::item_ref_admission::pinned_index(
                &runtime, owner.active_generation).ok_or("native lamp index")?;
            TypedDefinitionRef { family: "Item".into(), production_key: HIRELING_LAMP_KEY.into(),
                revision_ref: index.revision(HIRELING_LAMP_KEY)
                    .ok_or("canonical lamp definition")?.into() }
        };
        let native = owner.active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or("native lamp gameplay")?;
        let item = crate::gameplay_transport::item_ref_admission::generation_inbox_item_facts(
            native, &definition).map_err(debug)?;
        mint.claim = definition;
        mint.outputs = vec![RewardClaimInboxMintOutput {
            source_line: 0, item, quantity: 1, key_number: None,
        }];
    }
    assert!(matches!(h.root.prepare_reward_claim_inbox_mint(&authority, &h.node,
        current, &source, mint.clone()).await, Err(RewardClaimInboxError::AuthorityRejected)),
        "unrelated generic Inbox consumers must retain their Channel-only authority");
    native_mint(h, &authority, current, &source, mint).await?;
    assert_eq!(inbox_count(h).await?, 1);
    seed_identity(h, hireling, lamp).await?;
    if matches!(case, Case::LampPending) {
        return lamp_pending::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }

    let state = owner.runtime.lock().await.house_actor_state(deps.actor,
        admitted.game_session_id).map_err(debug)?;
    let position = (u16::try_from(state.position.x)?, u16::try_from(state.position.y)?,
        -i8::try_from(state.position.floor)?);
    let placement = placement(current, origin, hireling, lamp, position, 200)?;
    let mut wrong_house = current;
    wrong_house.runtime_scope = HouseId::new(admitted.world_id, OTHER_HOUSE)
        .ok_or("other House id")?.runtime_scope().map_err(debug)?;
    assert_eq!(h.root.commit_hireling_lamp(&authority, &h.node, wrong_house,
        placement.clone()).await.map_err(debug)?, Reply::Refused(Refusal::Stale));
    let wrong_node = NodeIncarnationProof::new(NodeRegistrationFact::new(
        h.node.fact().node_id(), h.node.fact().registration_revision() + 1),
        BootstrapSecret::from_bytes([0x91; 32]));
    assert_eq!(h.root.commit_hireling_lamp(&authority, &wrong_node, current,
        placement.clone()).await.map_err(debug)?, Reply::Refused(Refusal::Stale));
    let mut stale = current;
    stale.character_lease_generation += 1;
    assert_eq!(h.root.commit_hireling_lamp(&authority, &h.node, stale,
        placement.clone()).await.map_err(debug)?, Reply::Refused(Refusal::Stale));
    assert_eq!(inbox_count(h).await?, 1, "refused proofs cannot consume the lamp");
    assert_eq!(h.root.commit_hireling_lamp(&authority, &h.node, current,
        placement.clone()).await.map_err(debug)?, Reply::Committed);
    assert_eq!(inbox_count(h).await?, 0);
    let used: (i16, Option<Vec<u8>>, i64, i64, String) = sqlx::query_as(
        "SELECT source_kind,uuid_send(source_parent_item_instance_id),source_quantity_before::bigint,source_quantity_after::bigint,source_house_key \
         FROM game_item_use_records WHERE source_item_instance_id=encode($1,'hex')::uuid")
        .bind(lamp.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(used, (2, None, 1, 0, HOUSE.into()), "the use owner must prove the actual Inbox and House source");
    let envelope: Vec<u8> = sqlx::query_scalar(
        "SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_use_records u USING(transaction_id) WHERE u.source_item_instance_id=encode($1,'hex')::uuid",
    ).bind(lamp.as_slice()).fetch_one(&h.pool).await?;
    let (_, audit) = crate::durability::item_use_audit::decode_item_use_envelope(&envelope).map_err(debug)?;
    assert_eq!(audit.house_key.as_deref(), Some(HOUSE));
    assert_eq!(audit.channel_id, origin.as_bytes().to_vec(), "Channel is recorded only as origin provenance");
    assert_eq!(h.root.commit_hireling_lamp(&authority, &h.node, current,
        placement.clone()).await.map_err(debug)?, Reply::Committed);
    assert_eq!(uses(h).await?, 1, "replayed use burns exactly once");
    if matches!(case, Case::PositionOccupied) {
        return position::occupied_saved_cell(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::Trader) {
        return trader::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::Gourmet) {
        return gourmet::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::Cook) {
        return cook::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::CookBank) {
        return cook::bank_only_scenario(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::Steward) {
        return steward::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::Furniture) {
        return furniture::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }
    if matches!(case, Case::Bank) {
        return bank::scenario(h, &owner, admitted, deps.actor, hireling).await;
    }

    if matches!(case, Case::InboxFull) {
        let items = (160_u8..180).map(id).collect::<Vec<_>>();
        let filler = inbox_source(current, origin, 201, 180, items)?;
        let outputs = (0..20).map(lamp_output).collect();
        native_mint(h, &authority, current, &filler, mint_request(outputs)).await?;
        assert_eq!(inbox_count(h).await?, 20);
        let returned = return_plan(current, origin, hireling, 202, 181, 182)?;
        assert_eq!(h.root.return_hireling_to_lamp(&authority, &h.node, current,
            returned.clone(), &LampWeights(1000)).await.map_err(debug)?,
            Reply::Refused(Refusal::InventoryFull));
        assert_eq!(h.root.return_hireling_to_lamp(&authority, &h.node, current,
            returned, &LampWeights(1000)).await.map_err(debug)?,
            Reply::Refused(Refusal::InventoryFull));
        assert_eq!(inbox_count(h).await?, 20);
        assert!(active(h, hireling).await?);
        let absent: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid)")
            .bind(id(182).as_slice()).fetch_one(&h.pool).await?;
        assert!(absent, "refused native mint must roll back its fresh identity and audit");
        assert!(h.root.pending_hireling_recalls(&authority, &h.node, admitted.world_id)
            .await.map_err(debug)?.is_empty(), "current ownership prevents forced recall");
        seed_released_house_and_offline_session(h, current).await?;
        let pending = h.root.pending_hireling_recalls(&authority, &h.node, admitted.world_id)
            .await.map_err(debug)?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].hireling.id, hireling);
        assert_eq!(pending[0].source.command.game_session_id(), current.game_session_id);
        assert_eq!(pending[0].source.command.command_id().get(), 200);
        let recall = pending[0].authority.clone();
        let forced = HirelingLampReturn { hireling, input: HirelingReturnInput::Management, source: pending[0].source.clone(),
            mint: mint_request(vec![lamp_output(0)]) };
        let mut stale_recall = recall.clone();
        stale_recall.scope_ownership_generation += 1;
        assert!(h.root.recall_hireling_to_lamp(&authority, &h.node, stale_recall,
            forced.clone()).await.is_err());
        assert!(h.root.recall_hireling_to_lamp(&authority, &wrong_node, recall.clone(),
            forced.clone()).await.is_err());
        let mut missing_provenance = forced.clone();
        missing_provenance.source.command = CommandRef::new(current.game_session_id,
            CommandId::new(999).map_err(debug)?);
        assert!(h.root.recall_hireling_to_lamp(&authority, &h.node, recall.clone(),
            missing_provenance).await.is_err());
        assert_eq!(inbox_count(h).await?, 20);
        // No reconnect session remains: recall must use independent current House
        // authority and the original placement only as historical provenance.
        h.root.recall_hireling_to_lamp(&authority, &h.node, recall.clone(),
            forced.clone()).await.map_err(debug)?;
        assert_inbox_scope(h, forced.source.transaction_id, 2,
            recall.scope_ownership_generation).await?;
        assert_eq!(inbox_count(h).await?, 21, "ownership-loss recall bypasses donor Inbox20 cap");
        assert!(!active(h, hireling).await?);
        h.root.recall_hireling_to_lamp(&authority, &h.node, recall, forced)
            .await.map_err(debug)?;
        assert_eq!(inbox_count(h).await?, 21);
        assert!(h.root.pending_hireling_recalls(&authority, &h.node, admitted.world_id)
            .await.map_err(debug)?.is_empty());
        return Ok(());
    }

    let denied = h.root.set_hireling_outfit(&authority, &h.node, current,
        *origin.as_bytes(), 201, Request::Outfit { hireling, look_type: 1110,
            colors: [1, 2, 3, 4] }).await.map_err(debug)?;
    assert_eq!(denied, Reply::Refused(Refusal::OutfitNotOwned));
    assert_eq!(h.root.set_hireling_outfit(&authority, &h.node, current,
        *origin.as_bytes(), 202, Request::Outfit { hireling, look_type: 1108,
            colors: [1, 2, 3, 4] }).await.map_err(debug)?, Reply::Committed);
    let heavy = return_plan(current, origin, hireling, 203, 144, 145)?;
    assert_eq!(h.root.return_hireling_to_lamp(&authority, &h.node, current,
        heavy, &LampWeights(u32::MAX)).await.map_err(debug)?, Reply::Refused(Refusal::Capacity));
    assert!(active(h, hireling).await?);
    assert_eq!(inbox_count(h).await?, 0);
    let returned = return_plan(current, origin, hireling, 204, 146, 147)?;
    assert_eq!(h.root.return_hireling_to_lamp(&authority, &h.node, current,
        returned.clone(), &LampWeights(1000)).await.map_err(debug)?, Reply::Committed);
    assert!(!active(h, hireling).await?);
    assert_eq!(inbox_count(h).await?, 1);
    assert_inbox_scope(h, returned.source.transaction_id, 1,
        current.scope_ownership_generation.get()).await?;
    assert_eq!(h.root.return_hireling_to_lamp(&authority, &h.node, current,
        returned, &LampWeights(1000)).await.map_err(debug)?, Reply::Committed);
    // Old placement replay returns its original answer without restoring obsolete custody.
    assert_eq!(h.root.commit_hireling_lamp(&authority, &h.node, current,
        placement).await.map_err(debug)?, Reply::Committed);
    assert!(!active(h, hireling).await?);
    assert_eq!(uses(h).await?, 1);
    assert_eq!(inbox_count(h).await?, 1);
    let transitions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_hireling_lamp_transitions WHERE hireling_id=encode($1,'hex')::uuid")
        .bind(hireling.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(transitions, 2);
    let snapshot = h.root.query_hirelings(&authority, &h.node, current,
        *origin.as_bytes(), 205).await.map_err(debug)?;
    let Reply::State(entries) = snapshot else { return Err("Hireling list state".into()); };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].hireling, hireling);
    assert_eq!(entries[0].look_type, 1108);
    assert_eq!(entries[0].colors, [1, 2, 3, 4]);
    assert_eq!(entries[0].position, None);
    Ok(())
}

fn lamp_definition() -> TypedDefinitionRef {
    TypedDefinitionRef { family: "Item".into(), production_key: HIRELING_LAMP_KEY.into(),
        revision_ref: "hireling-lamp-1".into() }
}
fn lamp_output(source_line: u32) -> RewardClaimInboxMintOutput {
    RewardClaimInboxMintOutput { source_line, quantity: 1, key_number: None,
        item: ItemDefinitionFacts { definition: lamp_definition(), weight: Some(1000),
            stack: ItemStackClass::NonStackable, container_capacity: None,
            container_slot_equip_pattern: false } }
}
fn mint_request(outputs: Vec<RewardClaimInboxMintOutput>) -> RewardClaimInboxMintRequest {
    RewardClaimInboxMintRequest { claim: lamp_definition(), content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(), sim_revision: "simulation-1".into(), outputs }
}
fn inbox_source(current: CurrentCharacterItemFence, channel: crate::foundation::ChannelId,
    command: u64, transaction: u8, items: Vec<[u8; 16]>) -> TestResult<RewardClaimInboxPlan> {
    Ok(RewardClaimInboxPlan { command: CommandRef::new(current.game_session_id,
        CommandId::new(command).map_err(debug)?), character_id: current.character_id,
        world_id: current.runtime_scope.world_id(), channel_id: channel,
        transaction_id: id(transaction), source_binding: [transaction; 32], item_instance_ids: items })
}
fn return_plan(current: CurrentCharacterItemFence, channel: crate::foundation::ChannelId,
    hireling: [u8; 16], command: u64, transaction: u8, item: u8) -> TestResult<HirelingLampReturn> {
    let input = HirelingReturnInput::Management;
    let mut source = inbox_source(current, channel, command, transaction, vec![id(item)])?;
    source.source_binding = input.binding(hireling);
    Ok(HirelingLampReturn { hireling, input, source, mint: mint_request(vec![lamp_output(0)]) })
}
fn placement(_current: CurrentCharacterItemFence, channel: crate::foundation::ChannelId,
    hireling: [u8; 16], lamp: [u8; 16], position: (u16, u16, i8), command: u64,
) -> TestResult<HirelingLampPlacement> {
    Ok(HirelingLampPlacement { hireling, input: HirelingLampInput::ItemInstance(lamp), house: HOUSE.into(), origin_channel: *channel.as_bytes(),
        position, item_use: ItemUseRequest { kind: ItemUseKind::HirelingLamp { origin_channel: *channel.as_bytes() },
            command_id: command, content_revision: "content-1".into(), ruleset_revision: "ruleset-1".into(),
            sim_revision: "simulation-1".into(), transaction_id: id(148), event_id: id(149),
            occurred_at_unix_ms: 1, server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.into(),
            source_item_instance_id: lamp, source_definition: lamp_definition(), source_quantity_before: 1,
            flask: ItemUseFlaskPlan::None } })
}
async fn native_mint(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    current: CurrentCharacterItemFence, source: &RewardClaimInboxPlan,
    request: RewardClaimInboxMintRequest) -> TestResult {
    let mut candidate = h.root.prepare_hireling_inbox_mint(authority, &h.node, current,
        source, request).await.map_err(debug)?;
    let mut state = ();
    let result = h.root.commit_hireling_inbox_mint(authority, &h.node, current,
        &mut candidate, &mut state, |_, _| Box::pin(async { Ok(()) })).await.map_err(debug)?;
    assert!(matches!(result, RewardClaimInboxOutcome::Committed { .. }));
    assert_inbox_scope(h, source.transaction_id, 1, current.scope_ownership_generation.get()).await?;
    Ok(())
}
async fn assert_inbox_scope(h: &Harness, transaction: [u8; 16], kind: u32,
    generation: u64) -> TestResult {
    let envelope: Vec<u8> = sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_reward_claim_inbox_mint_receipts r USING(transaction_id) WHERE r.transaction_id=encode($1,'hex')::uuid")
        .bind(transaction.as_slice()).fetch_one(&h.pool).await?;
    let (_, minted) = crate::durability::reward_claim_mint_audit::decode_reward_claim_mint_envelope(&envelope).map_err(debug)?;
    let inbox = minted.inbox.ok_or("native Inbox audit")?;
    assert_eq!(inbox.house_key.as_deref(), Some(HOUSE));
    assert_eq!(inbox.house_scope_ownership_generation, Some(generation));
    assert_eq!(inbox.house_authority_kind, Some(kind));
    let stored: (String,String,i16) = sqlx::query_as("SELECT house_key,scope_ownership_generation::text,authority_kind FROM game_hireling_inbox_scope_receipts WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(transaction.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(stored, (HOUSE.into(), generation.to_string(), i16::try_from(kind)?));
    Ok(())
}
async fn seed_identity(h: &Harness, hireling: [u8; 16], lamp: [u8; 16]) -> TestResult {
    // Test-only Store grant. Actual item custody was already written by native mint above.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_hirelings(hireling_id,world_id,owner_character_id,name,sex,look_type,active,house_key,x,y,floor,lamp_item_instance_id,legacy_grant) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'Fixture Hireling',1,1108,FALSE,NULL,0,0,0,encode($4,'hex')::uuid,FALSE)")
        .bind(hireling.as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(lamp.as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn insert_character_root_copy(connection: &mut sqlx::PgConnection, record: &str) -> TestResult {
    // System-owned identifiers only. Generated columns (notably name_key) must
    // be computed from this imported historical Character rather than supplied.
    let columns: String = sqlx::query_scalar(
        "SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute \
         WHERE attrelid='game_character_roots'::regclass AND attnum>0 AND NOT attisdropped \
           AND attgenerated='' AND attidentity=''",
    ).fetch_one(&mut *connection).await?;
    let statement = format!("INSERT INTO game_character_roots({columns}) \
        SELECT {columns} FROM jsonb_populate_record(NULL::game_character_roots,$1::text::jsonb)");
    sqlx::query(sqlx::AssertSqlSafe(statement)).bind(record).execute(connection).await?;
    Ok(())
}
async fn insert_reconnect_session_copy(connection: &mut sqlx::PgConnection, record: &str) -> TestResult {
    // The historical session is copied across migration stages. Native
    // generated fields, including training_session_sequence, belong to the
    // destination schema and must be computed there from the copied authority.
    let columns: String = sqlx::query_scalar(
        "SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute \
         WHERE attrelid='game_durability_reconnect_sessions'::regclass AND attnum>0 \
           AND NOT attisdropped AND attgenerated='' AND attidentity=''",
    ).fetch_one(&mut *connection).await?;
    let statement = format!("INSERT INTO game_durability_reconnect_sessions({columns}) \
        SELECT {columns} FROM jsonb_populate_record(NULL::game_durability_reconnect_sessions,$1::text::jsonb)");
    sqlx::query(sqlx::AssertSqlSafe(statement)).bind(record).execute(connection).await?;
    Ok(())
}
async fn seed_released_house_and_offline_session(h: &Harness,
    current: CurrentCharacterItemFence) -> TestResult {
    // Administrative historical fixture: the already completed release outcome.
    // Production release is covered by HOUSE-1b; this test starts its offline recall.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_account_housing_slots WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_properties SET state=1,owner_character_id=NULL,owner_account_id=NULL,acquisition_receipt_ref=NULL,paid_until=NULL,ownership_generation=ownership_generation+1,revision=revision+1 WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(current.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn inbox_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn uses(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_use_records WHERE cause_kind=470")
        .fetch_one(&h.pool).await?)
}
async fn active(h: &Harness, hireling: [u8; 16]) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT active FROM game_house_hirelings WHERE hireling_id=encode($1,'hex')::uuid")
        .bind(hireling.as_slice()).fetch_one(&h.pool).await?)
}
struct LampWeights(u32);
impl ItemWeightLookup for LampWeights {
    fn generation_digest(&self) -> [u8; 32] { [1; 32] }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        if *definition == lamp_definition() { Ok(self.0) } else { Err(ItemWeightRefused::Unknown) }
    }
}

fn hireling_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    for tile in &mut sector.tiles {
        if tile.x == 7 {
            tile.house = 3437;
            tile.items.truncate(1);
        }
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    let mut furniture_cell = entrance.clone();
    furniture_cell.x = 32311;
    furniture_cell.y = 32804;
    furniture_cell.house = 3437;
    furniture_cell.items.truncate(1);
    sector.tiles.push(furniture_cell);
    entrance.x = 32309;
    entrance.y = 32805;
    sector.tiles.push(entrance);
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        fixture::door_items,
    )
    .map_err(debug)
}
