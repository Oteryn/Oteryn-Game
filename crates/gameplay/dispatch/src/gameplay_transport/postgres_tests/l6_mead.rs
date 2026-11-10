//! Native room proof and literal USE of the conservatively selected eastern barrel.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    self,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const NAME: &str = "l6/waterskin-of-mead";
#[test]
fn l6_mead_native_barrel_requires_mission_and_mints_complete_filled_bag() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                NAME,
                Position {
                    x: 32215,
                    y: 31060,
                    floor: -10,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("Mead fixture panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z10-x125-y121.b3"
            )),
            region: (10, 125, 121),
            bounds: (32213, 31059, 32218, 31064),
            start: (32215, 31061),
            expected: vec![
                ExpectedTile {
                    position: (32215, 31060),
                    ids: vec![6684, 7107, 2523],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32214, 31060),
                    ids: vec![6684, 7114, 2523],
                    counted_entry: None,
                },
            ],
        },
    )
}
/// Retain the production callback and all of its exact gates. The external
/// accepted-mission prerequisite is arranged separately as historical fixture data.
pub(super) fn catalogue(
    original: &quest::QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<quest::QuestStateCatalogue> {
    let tracks = callback
        .storage
        .iter()
        .map(|s| original.track(&s.track).cloned().ok_or("Mead track"))
        .collect::<Result<Vec<_>, _>>()?;
    quest::QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .ok_or("Mead transition")?
                .clone(),
            quest::world_callbacks::completion_transition(callback).ok_or("child completion")?,
        ],
    )
    .map_err(debug)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &Callback,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY {NAME} version={version}");
    let checksum: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=2060 AND success")
            .fetch_one(&h.pool)
            .await?;
    use sha2::{Digest, Sha384};
    assert_eq!(
        checksum,
        Sha384::digest(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/2060_quest_world_child_completion.sql"
        )))
        .as_slice()
    );

    let map_digest = owner
        .bundle_map()
        .ok_or("native room")?
        .overlay
        .base()
        .digest()
        .to_vec();
    let payload = |x| -> TestResult<Vec<u8>> {
        let mut placement = map_digest.clone();
        placement.extend_from_slice(
            &oteryn_world_bundle::bundle::placement_key(-10, x, 31060, 2)
                .ok_or("barrel key")?
                .to_be_bytes(),
        );
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
        .map_err(debug)
    };
    let east = payload(32215)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, east.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(before, 0);
    let prerequisites = callback
        .storage
        .iter()
        .filter(|s| s.preserve)
        .map(|s| (s.track.clone(), s.maximum))
        .collect::<Vec<_>>();
    super::qengine_outfit_stories::seed_quest_prerequisites(h, owner, admitted, &prerequisites, 92)
        .await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(32214)?).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::NothingToUse
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    // A full backpack must leave the claim, flag and value relations unchanged.
    let slots: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool)
        .await?;
    for n in slots..20 {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &quest::world_callbacks::Offering {
                position: Position {
                    x: 32215,
                    y: 31061,
                    floor: -10,
                },
                item: "oteryn:item.tibia.i2901".into(),
                count: 1,
            },
            100 + u64::try_from(n)?,
            true,
        )
        .await?;
    }
    let full = counts(h).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 3, 2, east.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        full,
        "no-room rolls back the entire source reward"
    );
    let source:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2901' ORDER BY i.item_instance_id LIMIT 1").fetch_one(&h.pool).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let map = owner.bundle_map().ok_or("map")?;
    let (content_revision, placement_context, context) = {
        let runtime = owner.runtime.lock().await;
        let content_revision = format!(
            "sha256:{}",
            runtime
                .content_pin()
                .server_artifact_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let placement_context = runtime.content_pin().frame_binding_digest().to_vec();
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let context = crate::durability::quest_world_callback::QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: content_revision.clone(),
            placement_context: placement_context.clone(),
            reset_epoch: owner.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .revision(),
        };
        (content_revision, placement_context, context)
    };
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    let dropped = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(950).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: source.try_into().map_err(|_| "source identity")?,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            32215, 31061, -10,
                        ),
                    map_revision: context.map_revision.clone(),
                    content_revision,
                    placement_context,
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(
        dropped.is_ok(),
        "make room through native custody: {dropped:?}"
    );
    let session = admitted.game_session_id;
    let request = |id| -> TestResult<_> {
        Ok(QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Use(CommandRef::new(session, CommandId::new(id).map_err(debug)?)),
        })
    };
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone();
    let before = counts(h).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    assert!(matches!(
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request(951)?,
            catalogue.clone()
        )
        .await,
        Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
    ));
    let child =
        quest::world_callbacks::completion_transition(callback).ok_or("child transition")?;
    assert!(
        matches!(
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: child.key.clone(),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(954).map_err(debug)?
                    ))
                },
                catalogue.clone(),
            )
            .await,
            Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
        ),
        "child completion cannot be invoked without its native reward"
    );
    let mut alias = child;
    alias.key = "oteryn:quest-transition/fixture-mead-child-alias".into();
    let alias_key = alias.key.clone();
    let forged = quest::QuestStateCatalogue::new(
        catalogue.content_revision(),
        callback
            .storage
            .iter()
            .map(|s| catalogue.track(&s.track).cloned().ok_or("parent track"))
            .collect::<Result<Vec<_>, _>>()?,
        vec![
            catalogue
                .transition(&callback.key)
                .ok_or("primary")?
                .clone(),
            alias,
        ],
    )
    .map_err(debug)?;
    assert!(
        matches!(
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: alias_key,
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(956).map_err(debug)?
                    ))
                },
                std::sync::Arc::new(forged),
            )
            .await,
            Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
        ),
        "renaming a child completion cannot evade its native owner"
    );
    let weights = NativeItemWeights(native).with_wheel(owner.premium_coordinator);
    let admission = crate::durability::reward_claim_mint::QuestRewardAdmission {
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,

        native,
        weights: &weights,
        fee_payment: None,
    };
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(matches!(
        slot.commit_world_callback_with_items(
            owner.root,
            owner.character,
            owner.holder,
            stale,
            request(952)?,
            catalogue.clone(),
            &permit,
            context.clone(),
            Some(native),
            Some(admission),
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
    ));
    // The same valid source and mission refuse when the live Premium participant is absent.
    assert!(matches!(
        slot.commit_world_callback_with_items(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request(953)?,
            catalogue.clone(),
            &permit,
            context.clone(),
            Some(native),
            Some(crate::durability::reward_claim_mint::QuestRewardAdmission {
                actor_conditions: None,
                carried_use: None,
                creature_use: None,
                peer_use: None,

                native,
                weights: &NativeItemWeights(native),
                fee_payment: None,
            }),
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    let without_child = quest::QuestStateCatalogue::new(
        catalogue.content_revision(),
        callback
            .storage
            .iter()
            .map(|s| catalogue.track(&s.track).cloned().ok_or("parent track"))
            .collect::<Result<Vec<_>, _>>()?,
        vec![
            catalogue
                .transition(&callback.key)
                .ok_or("primary")?
                .clone(),
        ],
    )
    .map_err(debug)?;
    assert!(
        matches!(
            slot.commit_world_callback_with_items(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request(955)?,
                std::sync::Arc::new(without_child),
                &permit,
                context.clone(),
                Some(native),
                Some(admission),
                &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
            )
            .await
            .map_err(debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "missing child admission must roll back the already prepared reward"
    );
    assert_eq!(counts(h).await?, before);
    drop(slot);
    drop(permit);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 4, 2, east.clone()).await?;

    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let completed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.waterskin_of_mead_quest' AND completed_character_revision IS NOT NULL)")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        completed,
        "native mead reward must complete its standalone quest atomically"
    );
    let parent_completed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&callback.quest).fetch_one(&h.pool).await?;
    assert!(
        !parent_completed,
        "the side reward must not complete Ice Islands"
    );
    let bags:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i7343'").fetch_one(&h.pool).await?;
    assert_eq!(
        bags, 1,
        "valid native barrel must atomically mint the complete reward"
    );
    let children:Vec<(String,i64,Option<i16>)>=sqlx::query_as("SELECT i.definition_production_key,i.quantity,f.fluid_subtype FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_instances p ON p.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_fluid_states f ON f.item_instance_id=i.item_instance_id WHERE p.definition_production_key='oteryn:item.tibia.i7343' ORDER BY i.definition_production_key").fetch_all(&h.pool).await?;
    assert_eq!(
        children,
        vec![
            ("oteryn:item.tibia.i2901".into(), 1, Some(16)),
            ("oteryn:item.tibia.i3031".into(), 82, None),
            ("oteryn:item.tibia.i7439".into(), 1, None)
        ]
    );
    let bag:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i7343'").fetch_one(&h.pool).await?;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    let observation = owner
        .observe_container(
            admitted.runtime_actor.ok_or("actor")?,
            Some(fence.item_fence()),
            crate::gameplay_transport::item_view::ItemKey::Instance(
                bag.try_into().map_err(|_| "bag identity")?,
            ),
        )
        .await
        .ok_or("normal native bag observation")?;
    let crate::gameplay_transport::container_view::ContainerObservation::Container {
        capacity,
        entries,
        ..
    } = observation
    else {
        return Err("native bag is not visible as a container".into());
    };
    assert_eq!((capacity, entries.len()), (8, 3));
    let vessel_ref = native
        .item_index()
        .ok_or("item index")?
        .definition_ref("Item", "oteryn:item.tibia.i2901", "definition-r1")
        .ok_or("waterskin definition")?;
    assert!(
        entries
            .iter()
            .any(|item| item.item_definition_ref == vessel_ref
                && item.count.get() == 1
                && item.sub_type == 16),
        "ordinary container projection must render the waterskin as mead"
    );
    let atomic:(i64,i64)=sqlx::query_as("WITH root AS (SELECT item_instance_id FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7343'), chosen AS (SELECT item_instance_id FROM root UNION ALL SELECT e.item_instance_id FROM game_item_nested_entries e JOIN root r ON r.item_instance_id=e.parent_item_instance_id), evidence AS (SELECT xmin::text AS xid FROM game_item_instances WHERE item_instance_id IN (SELECT item_instance_id FROM chosen) UNION ALL SELECT xmin::text FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1 UNION ALL SELECT xmin::text FROM game_character_quest_receipts WHERE transition_key=$1 UNION ALL SELECT xmin::text FROM game_character_quest_tracks WHERE track_key=$2 UNION ALL SELECT xmin::text FROM game_item_fluid_states WHERE item_instance_id IN (SELECT item_instance_id FROM chosen) UNION ALL SELECT xmin::text FROM game_item_fluid_reward_mints WHERE item_instance_id IN (SELECT item_instance_id FROM chosen)) SELECT (SELECT count(*) FROM chosen),count(DISTINCT xid) FROM evidence").bind(&callback.key).bind(&callback.storage.last().ok_or("claim flag")?.track).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic,
        (4, 1),
        "bag, three children, fluid and quest receipt share one PostgreSQL transaction"
    );
    let same_transaction: bool = sqlx::query_scalar("SELECT count(DISTINCT xid)=1 AND count(*)=3 FROM (SELECT xmin::text AS xid FROM game_quest_world_completions WHERE quest_key='oteryn:quest.authored.waterskin_of_mead_quest' UNION ALL SELECT xmin::text FROM game_character_quest_states WHERE quest_key='oteryn:quest.authored.waterskin_of_mead_quest' UNION ALL SELECT xmin::text FROM game_character_quest_receipts WHERE transition_key=$1 AND cause_ordinal=4) evidence")
        .bind(&callback.key).fetch_one(&h.pool).await?;
    assert!(
        same_transaction,
        "child state/proof and native parent receipt share one transaction"
    );
    let vessel:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_instances p ON p.item_instance_id=e.parent_item_instance_id WHERE p.definition_production_key='oteryn:item.tibia.i7343' AND i.definition_production_key='oteryn:item.tibia.i2901'").fetch_one(&h.pool).await?;
    let vessel: [u8; 16] = vessel.try_into().map_err(|_| "filled vessel identity")?;
    let transaction: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
        .fetch_one(&h.pool)
        .await?;
    let definition = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2901".into(),
        revision_ref: "definition-r1".into(),
    };
    assert_eq!(
        owner
            .root
            .commit_item_fluid(
                owner.character,
                owner.holder,
                fence.item_fence(),
                crate::durability::item_use::fluid::ItemFluidRequest {
                    command_id: 960,
                    transaction_id: transaction.try_into().map_err(|_| "fluid transaction")?,
                    change: crate::durability::item_use::fluid::ItemFluidChange {
                        item_instance_id: vessel,
                        definition,
                        fluid_before: 16,
                        fluid_after: 0
                    },
                    other: None
                }
            )
            .await
            .map_err(debug)?,
        crate::durability::item_use::fluid::ItemFluidOutcome::Written
    );
    let committed = counts(h).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 4, 2, east.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 5, 2, east.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::StaleState
    );
    assert_eq!(
        counts(h).await?,
        committed,
        "stale physical target writes nothing"
    );
    let mut current =
        oteryn_protocol_oteryn::world_object::decode_use_intent(&east).map_err(debug)?;
    current.expected_revision = 1;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace repeat USE")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        6,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&current).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );
    let permit = owner.spell_lane_permit().await.ok_or("replay lane")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let replay = slot
        .commit_world_callback_with_items(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request(4)?,
            catalogue,
            &permit,
            context,
            Some(native),
            Some(admission),
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
        "durable exact receipt: {replay:?}"
    );
    assert_eq!(counts(h).await?, committed);
    let fluid:Option<i16>=sqlx::query_scalar("SELECT fluid_subtype FROM game_item_fluid_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(vessel.as_slice()).fetch_optional(&h.pool).await?;
    assert_eq!(
        fluid, None,
        "historical world receipt cannot refill a used waterskin"
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    for gate in &callback.storage {
        assert_eq!(
            state.tracks().get(&gate.track),
            Some(&if gate.preserve { gate.maximum } else { 1 })
        );
    }
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('instances',(SELECT count(*) FROM game_item_instances),'mints',(SELECT count(*) FROM game_reward_claim_mint_receipts),'reservations',(SELECT count(*) FROM game_reward_claim_mint_reservations),'fluids',(SELECT count(*) FROM game_item_fluid_states),'proofs',(SELECT count(*) FROM game_item_fluid_reward_mints),'audits',(SELECT count(*) FROM game_item_audit_outbox),'quest_receipts',(SELECT count(*) FROM game_character_quest_receipts),'quest_states',(SELECT jsonb_agg(to_jsonb(s)) FROM game_character_quest_states s),'world_completions',(SELECT jsonb_agg(to_jsonb(c)) FROM game_quest_world_completions c),'roots',(SELECT jsonb_agg(to_jsonb(r)) FROM game_character_roots r),'tracks',(SELECT jsonb_agg(to_jsonb(t)) FROM game_character_quest_tracks t))").fetch_one(&h.pool).await?)
}
