//! Shared E2E-WORLD-1 setup and durable observations.
pub(super) use super::fixture::{Harness, debug, id, run};
pub(super) use super::{Dependencies, TestResult, prepare_current_session, reset, track};
pub(super) use crate::content::channel_content_pin::ChannelContentPin;
pub(super) use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
pub(super) use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
pub(super) use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
pub(super) use crate::durability::item_transfer::{ItemTransferDestination, ItemTransferRequest};
pub(super) use crate::foundation::{
    CharacterLease, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementFacing,
    MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
pub(super) use crate::gameplay_transport::ComposedFreshAdmission;
pub(super) use crate::gameplay_transport::capabilities::SelectedCapabilities;
pub(super) use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, UseCommand,
};
pub(super) use crate::gameplay_transport::item_ref_admission::{
    NativeItemWeights, generation_item_facts,
};
pub(super) use crate::gameplay_transport::item_view::{
    ItemKey, ItemViewContinuity, SessionItemView,
};
pub(super) use crate::gameplay_transport::kill_reward::DrainOutcome;
pub(super) use crate::gameplay_transport::last_position::AdmissionPosition;
pub(super) use crate::gameplay_transport::npc_talk::NpcTalkBook;
pub(super) use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
pub(super) use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
pub(super) use crate::map::facts::ItemDefinition;
pub(super) use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
pub(super) use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
pub(super) use oteryn_protocol_oteryn::depot::*;
pub(super) use oteryn_protocol_oteryn::logout::LogoutResult;
pub(super) use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
pub(super) use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
pub(super) use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
pub(super) use std::sync::Arc;
pub(super) use std::time::Duration;

pub(super) const GOLD: &str = "oteryn:item.tibia.i3031";
pub(super) const BACKPACK: &str = "oteryn:item.tibia.i2854";
/// A generated-reply merchant whose Service sells cheap goods for plain coin.
pub(super) const NPC: &str = "oteryn:npc.asphota";
/// The pinned entry start; two paced West steps reach the trade cell.
pub(super) const ENTRY: (i32, i32) = (10, 0);
pub(super) const TRADE_CELL: MovementLocalPosition = MovementLocalPosition {
    x: 8,
    y: 0,
    floor: 7,
};
/// Beside the depot locker the bundle fixture places at x 6.
pub(super) const LOCKER_CELL: MovementLocalPosition = MovementLocalPosition {
    x: 7,
    y: 0,
    floor: 7,
};
pub(super) const NPC_CELL: (u16, u16) = (9, 1);
pub(super) const RAT_CELL: MovementLocalPosition = MovementLocalPosition {
    x: 8,
    y: 1,
    floor: 7,
};
pub(super) const DEPOT_BOX: u8 = 17;

/// Every durable value the chain can move, as one comparable row set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Ledger {
    /// `(production key, quantity, instance)` of the main backpack's entries, ordered.
    pub(super) backpack: Vec<(String, i64, [u8; 16])>,
    pub(super) npc_trades: i64,
    pub(super) npc_trade_lines: i64,
    pub(super) xp_receipts: i64,
    pub(super) experience: i64,
    pub(super) corpse_mints: i64,
    pub(super) corpse_entries: i64,
    pub(super) transfer_receipts: i64,
    pub(super) depot_receipts: i64,
    pub(super) depot_counter: Option<(String, i32)>,
    pub(super) live_items: i64,
    pub(super) live_quantity: i64,
}

pub(super) async fn ledger(h: &Harness) -> TestResult<Ledger> {
    let backpack: Vec<(String, i64, Vec<u8>)> = sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity::bigint, decode(replace(i.item_instance_id::text,'-',''),'hex') \
         FROM game_item_container_entries e JOIN game_item_instances i USING (item_instance_id) \
         WHERE e.character_id = encode($1,'hex')::uuid ORDER BY e.placement_ordinal",
    )
    .bind(id(41).as_slice())
    .fetch_all(&h.pool)
    .await?;
    let count = |sql: &'static str| async move {
        Ok::<i64, Box<dyn std::error::Error>>(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
    };
    Ok(Ledger {
        backpack: backpack
            .into_iter()
            .map(|(key, quantity, id)| Ok((key, quantity, id.as_slice().try_into()?)))
            .collect::<TestResult<_>>()?,
        npc_trades: count("SELECT count(*) FROM game_npc_trades").await?,
        npc_trade_lines: count("SELECT count(*) FROM game_npc_trade_lines").await?,
        xp_receipts: count("SELECT count(*) FROM game_character_xp_receipts").await?,
        experience: count("SELECT coalesce(sum(total_experience),0)::bigint FROM game_character_progression_state")
            .await?,
        corpse_mints: sqlx::query_scalar(
            "SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key = $1 \
             AND materialized_at IS NOT NULL",
        )
        .bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY)
        .fetch_one(&h.pool)
        .await?,
        corpse_entries: count("SELECT count(*) FROM game_item_corpse_container_entries").await?,
        transfer_receipts: count("SELECT count(*) FROM game_item_transfer_receipts").await?,
        depot_receipts: count(
            "SELECT count(*) FROM game_item_loop_move_receipts WHERE depot_town=0 AND depot_box=17",
        )
        .await?,
        depot_counter: sqlx::query_as(
            "SELECT next_ordinal::text, committed FROM game_character_depot_box_counters \
             WHERE town_id=0 AND box_number=17",
        )
        .fetch_optional(&h.pool)
        .await?,
        live_items: count("SELECT count(*) FROM game_item_instances WHERE lifecycle = 1").await?,
        live_quantity: count(
            "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle = 1",
        )
        .await?,
    })
}

pub(super) fn gold(ledger: &Ledger) -> i64 {
    ledger
        .backpack
        .iter()
        .filter(|(key, _, _)| key == GOLD)
        .map(|(_, quantity, _)| quantity)
        .sum()
}

pub(super) async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

pub(super) async fn step_west(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<MovementLocalPosition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    let (outcome, _) = owner.paced_step(actor, session, StepDirection::West).await;
    assert_eq!(outcome.disposition, StepDisposition::Moved, "{outcome:?}");
    position(owner, actor).await
}

/// The successor admission of the fixture Character, as the loop-training reload does: refresh
/// the fixture Platform sources through their owner API, then commit the next fresh admission.
pub(super) async fn relogin(h: &Harness) -> TestResult<GameSessionId> {
    relogin_as(h, id(41), id(40), id(110)).await
}

pub(super) async fn relogin_as(
    h: &Harness,
    character: [u8; 16],
    account: [u8; 16],
    successor: [u8; 16],
) -> TestResult<GameSessionId> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityGuardStateV1 as State,
        AdmissionAuthorityPublicationV1, AdmissionPublicationPreconditionV1,
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let template = super::fresh_fixture::fresh::Source::new(now)?;
    let current = crate::foundation::fnd04_verifier::FreshCurrentEvidence {
        account_id: crate::gameplay_transport::canonical_uuid(&account),
        character_id: crate::foundation::CharacterId::decode(&character).map_err(debug)?,
        ..template.current
    };
    let mut source = super::fresh_fixture::fresh::Source::with_current(now, current.clone())?;
    let keys = source
        .rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current source publications")?;
    let mut refresh = super::fresh_fixture::fresh::Source::with_current(now, current)?;
    refresh.rows = source
        .rows
        .iter()
        .filter(|row| matches!(row.key, Key::Account { .. } | Key::SigningTrust { .. }))
        .cloned()
        .collect();
    for row in &mut refresh.rows {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: row.publication_revision,
        };
        row.publication_revision += 1;
        row.source.source_revision += 1;
        row.source.decision_identity = format!("full-world-source-{}", row.source.source_revision);
        row.source.source_observed_at = now;
        if let State::Account { security, .. } = &mut row.state {
            security.provenance.publication_revision = row.publication_revision;
            security.provenance.source_revision = row.source.source_revision;
            security.provenance.accepted_source_revision = row.source.source_revision;
            security.provenance.decision_identity = row.source.decision_identity.clone();
            security.provenance.accepted_decision_identity = row.source.decision_identity.clone();
            security.provenance.source_observed_at = now;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&refresh, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("refreshed current publications")?;
    source.current.character_lease_generation = 2;
    let next = if character == id(41) {
        source.appearance_observer_request(
            &crate::gameplay_transport::canonical_uuid(&character),
            now,
        )?
    } else {
        source.request_with_nonce(
            GameSessionId::decode(&successor).map_err(debug)?,
            crate::foundation::AuthenticatedTransportRefV1::decode(&id(112)).map_err(debug)?,
            Some((
                &source.current.account_id,
                &crate::gameplay_transport::canonical_uuid(&character),
            )),
            now,
            Some([91; 32]),
        )?
    };
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&next).await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    // A replayed commit of the same request admits no third lease.
    let _ = store.commit(&next).await;
    let next_session = GameSessionId::decode(&successor).map_err(debug)?;
    assert_eq!(
        store
            .current_session(next_session)
            .await?
            .current_character_lease()
            .generation(),
        3
    );
    Ok(next_session)
}

// Shared lexical fixture: the controller, authority and owner borrow each other. Keeping
// their setup in one macro preserves those lifetimes without copying setup into each case.
macro_rules! world_scenario {
    ($tag:literal, |$h:ident, $request:ident, $seal:ident, $authority:ident, $world_id:ident, $native:ident, $world:ident, $cells:ident, $room:ident, $pin:ident, $deps:ident, $actor:ident, $session:ident, $owner:ident, $fence:ident, $admitted:ident| $body:block) => {{
        use $crate::gameplay_transport::monk_save::postgres_tests::full_world_support::*;
        run(async |admin| {
        reset();
        let $h = Harness::create_with_admission_seed(admin, $tag, false, false).await?;
        let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&$h.pool).await?;
        eprintln!("POSTGRES_DATABASE_STARTED {} version={version}", $tag);
        let $request = prepare_current_session(&$h).await?;
        let $seal = $h.recovery.seal_current().map_err(debug)?;
        let $authority = $h
            .root
            .open_character_authority(&$seal)
            .await
            .map_err(debug)?;
        let $world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;

        // One production generation (spells manifest with loot tables) for items, NPC offers,
        // the rat and its loot. Its training formula uses the fixture's durable content revision.
        let (controller, activated) =
            $crate::gameplay_transport::monk_save::postgres_tests::death_ml_loss_composed::production_generation_with_pin($world_id, "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let $native = active.native_gameplay().ok_or("native gameplay")?;
        let index = $native.item_index().ok_or("item index")?;
        let typed = |key: &str| {
            crate::gameplay_transport::item_look::typed_definition(
                index,
                index.definition_ref("Item", key, "definition-r1")?,
            )
        };
        let backpack_definition = typed(BACKPACK).ok_or("backpack definition")?;
        let gold_definition = typed(GOLD).ok_or("gold definition")?;
        let backpack_facts = generation_item_facts($native, &backpack_definition).map_err(debug)?;
        let gold_facts = generation_item_facts($native, &gold_definition).map_err(debug)?;
        let wire_id = |key: &str| -> TestResult<u32> {
            Ok(index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("wire id")?
                .get()
                - 1)
        };

        // The depot bundle fixture (locker at x 6), widened with open grass for the walk, the
        // NPC and the rat, through the production writer, loader and boot.
        let (bytes, mut bundle_pins) = bundle_fixture::bundle_with_item_ids(
            (BACKPACK, wire_id(BACKPACK)?),
            (GOLD, wire_id(GOLD)?),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        // The fixture's `terrain:grass` carries no Tibia source id, which leaves its tiles'
        // spell facts unknown and refuses every attack on them (BUNDLE-TILES-1): route the
        // ground through a Tibia ground key, as the PvP bundle fixture does.
        bundle.manifest.palette.first_mut().ok_or("ground palette")?.key =
            "oteryn:item.tibia.i100".into();
        let sector = bundle.sectors.first_mut().ok_or("bundle sector")?;
        sector
            .tiles
            .iter_mut()
            .find(|tile| tile.x == 6)
            .ok_or("locker tile")?
            .items
            .get_mut(1)
            .ok_or("locker item")?
            .attrs
            .depot = Some(0);
        let grass = |x, y| oteryn_world_bundle::sector::Tile {
            x,
            y,
            flags: 0,
            house: 0,
            zones: vec![],
            items: vec![oteryn_world_bundle::sector::Item {
                palette: 0,
                depth: 0,
                attrs: oteryn_world_bundle::sector::Attrs::default(),
            }],
        };
        sector.tiles.extend((8..=12).map(|x| grass(x, 0)));
        sector.tiles.extend((6..=12).map(|x| grass(x, 1)));
        sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        bundle_pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &bundle_pins).map_err(debug)?;
        let $world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: bundle_pins,
                start: crate::map::overlay::TilePos {
                    x: u16::try_from(ENTRY.0)?,
                    y: u16::try_from(ENTRY.1)?,
                    floor: -7,
                },
            },
            $world_id,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            |key| {
                Some(ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key == BACKPACK),
                    blocks_projectile: false,
                    pickupable: key == GOLD,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let $cells = $world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let $room = activated
            .qualified_room()
            .bundle_view(
                $cells.clone(),
                $world.frame_binding_digest($world_id),
                $world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: ENTRY.0,
                    y: ENTRY.1,
                    floor: 7,
                },
            )
            .ok_or("bundle combat room")?;
        let $pin = ChannelContentPin::from_activation(
            $world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            $room.frame_binding().digest(),
            $room.map_revision_digest(),
            (ENTRY.0, ENTRY.1, 7),
        );
        let mut $deps =
            Dependencies::with_content($pin.clone(), $room.clone(), $native.spell_book().clone())?;
        $deps.combat_room = Some($room.clone());
        // The Channel holds the player, the merchant, the rat and the successor's $actor, as the
        // autowalk case widens its runtime: the fixture default seats two.
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            $deps.$admitted.$world_id,
            $deps.$admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            $pin.clone(),
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session($deps.$admitted.game_session_id)
            .map_err(debug)?;
        $deps.$actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        $native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        $deps.runtime = tokio::sync::Mutex::new(runtime);
        let $actor = $deps.$actor;
        let $session = $deps.$admitted.game_session_id;

        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&$h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&$h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice())
            .execute(&$h.pool).await?;

        let mut $owner = $deps.$owner(&$h, &$authority);
        $owner.active_generation = Some(active);
        $owner.movement_cells = (&$cells).into();
        $owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(600))
            .ok_or("a monotonic clock older than ten minutes")?;

        // 1. Login and admission: the committed fresh $session enters the Channel on the pinned
        // entry start with its Character lease bound, and its $session is tracked for logout.
        let $fence = crate::gameplay_transport::item_fence_of(
            FreshAdmissionStore::from_root($h.root.clone())
                .current_session_at($session)
                .await?
                .0,
        )
        .ok_or("item fence")?;
        {
            let mut runtime = $deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position($actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    $actor,
                    $session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut states = $owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(
                        &runtime,
                        $actor,
                        $session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            // Minimum fist damage stays above the rat's defence and armor.
                            level: 50,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        $owner.owner_now(),
                    )
                    .is_some()
            );
        }
        track(&$owner, &$deps)?;
        let mut $admitted = $deps.$admitted;
        $admitted.first_entry = FirstEntryOutcome::Positioned;
        $admitted.item_fence = Some($fence);
        $owner.admit_progression_session(&mut $admitted).await;
        assert!($owner.progression_binding($session).is_some());
        $owner.admit_loop_training(&mut $admitted).await;
        assert_eq!($admitted.first_entry, FirstEntryOutcome::Positioned);
        assert_eq!(
            position(&$owner, $actor).await?,
            MovementLocalPosition {
                x: ENTRY.0,
                y: ENTRY.1,
                floor: 7
            }
        );

        // The purse: a backpack and 100 gold through the existing MINT and TRANSFER owners.
        for (ordinal, definition, quantity) in [
            (0_u64, &backpack_definition, 1_u32),
            (1, &gold_definition, 100_u32),
        ] {
            let mut seed = $h
                .root
                .freeze_item_mint(
                    &$authority,
                    &$h.node,
                    ItemMintRequest {
                        cause: ItemMintCause::for_test(
                            $world_id,
                            $deps.$admitted.channel_id,
                            $fence.scope_ownership_generation,
                            u32::try_from(ordinal + 1)?,
                            1,
                            TypedDefinitionRef {
                                family: "LootTable".into(),
                                production_key: "fixture:loot.full-world-purse".into(),
                                revision_ref: "definition-r1".into(),
                            },
                            "fixture:purpose.full-world-purse".into(),
                            0,
                        ),
                        item: definition.clone(),
                        quantity,
                        ground: GroundPlacement {
                            spatial_position:
                                crate::durability::map_item_mint_audit::map_item_spatial_position(
                                    u16::try_from(ENTRY.0)?, u16::try_from(ENTRY.1)?, -7,
                                ),
                            corpse_ref: id(3).to_vec(),
                            map_revision: $world.map_revision().into(),
                            content_revision: "content-1".into(),
                            native_room_placement_context: $world.base().digest().to_vec(),
                        },
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            let permit = $owner.spell_lane_permit().await.ok_or("lane")?;
            let minted = match $h
                .root
                .commit_item_mint(&permit, &$authority, &$h.node, &mut seed)
                .await
                .map_err(debug)?
            {
                ItemMintOutcome::Committed(item) => item.item_instance_id,
                other => return Err(debug(other)),
            };
            let mut transfer = $h
                .root
                .freeze_item_transfer(
                    &$authority,
                    &$h.node,
                    $fence,
                    ItemTransferRequest {
                        weight_generation_digest: $native.source_digest(),
                        command: CommandRef::new(
                            $session,
                            CommandId::new(900 + ordinal).map_err(debug)?,
                        ),
                        source_item_instance_id: minted,
                        destination: if ordinal == 0 {
                            ItemTransferDestination::ContainerSlot
                        } else {
                            ItemTransferDestination::MainBackpack
                        },
                        item: if ordinal == 0 {
                            backpack_facts.clone()
                        } else {
                            gold_facts.clone()
                        },
                        backpack: (ordinal != 0).then(|| backpack_facts.clone()),
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                    &NativeItemWeights($native),
                )
                .await
                .map_err(debug)?;
            $h.root
                .commit_item_transfer(
                    &permit,
                    &$authority,
                    &$h.node,
                    $fence,
                    &mut transfer,
                    &NativeItemWeights($native),
                )
                .await
                .map_err(debug)?;
        }

            $body
        })
    }};
}
pub(super) use world_scenario;
