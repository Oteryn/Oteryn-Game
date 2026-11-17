//! INVCAP-1: the real connection sends the own character's capacity, read through the
//! session's item fence with the move capacity rules, and the client panel shows Tibia's Cap.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::{Dependencies, TestResult, id, prepare_current_session};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::{ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_dev_client::native_session as client;
use oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity;
use oteryn_protocol_oteryn::{
    MessageType, ServerAcceptedValue, decode_wire_envelope, encode_server_accepted,
};
use std::time::Duration;

fn select(capabilities: &[u32]) -> TestResult<SelectedCapabilities> {
    Ok(SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: 4,
                requires: &[6],
            },
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 508,
                requires: &[4, 6],
            },
        ],
        capabilities,
    )
    .ok_or("capacity capability selection")?)
}

#[test]
fn inventory_capacity_real_connection_postgres() -> TestResult {
    run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "inventory_capacity", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            world,
            "content-1",
        )?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.controller.as_mut().ok_or("controller")?.transport =
            current.current_transport().ok_or("transport")?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 3)]).await?;
        // Without capability 508 the owner reads nothing.
        admitted.continuity.selected_capabilities = select(&[6])?;
        assert_eq!(owner.observe_inventory_capacity(admitted).await, None);
        admitted.continuity.selected_capabilities = select(&[4, 6, 508])?;
        let expected = owner
            .observe_inventory_capacity(admitted)
            .await
            .ok_or("fenced capacity read")?;
        // The same derivation the item-move owner checks: capacity and the carried tree.
        let fence = admitted.item_fence.ok_or("item fence")?;
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native gameplay")?;
        let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
            .with_wheel(owner.premium_coordinator);
        let probe = ObservationLockProbe {
            weights: &weights,
            options: h.pool.connect_options().as_ref().clone(),
            root: Default::default(),
            carried: Default::default(),
        };
        let (capacity, carried) = h
            .root
            .read_inventory_capacity(
                &authority,
                &h.node,
                fence,
                None,
                &probe,
            )
            .await
            .map_err(debug)?
            .ok_or("durable capacity")?;
        assert_eq!(probe.root.get(), Some(&Ok(())), "observation locked the root");
        assert_eq!(
            probe.carried.get(),
            Some(&Ok(())),
            "observation locked carried items"
        );
        assert!(carried > 0, "the lock probe must read actual carried items");
        assert!(capacity > 0);
        assert_eq!(
            Some(expected),
            InventoryCapacity::from_carried(capacity, carried)
        );
        // Exercise the snapshot's database fence, beyond the connection's early check.
        let mut outdated = fence;
        outdated.connection_generation = crate::foundation::ConnectionGeneration::new(
            fence.connection_generation.get() + 1,
        ).map_err(debug)?;
        assert_eq!(
            h.root.read_inventory_capacity(&authority, &h.node, outdated, None, &weights)
                .await.map_err(debug)?,
            None
        );
        // A stale item fence (another connection generation) reads nothing.
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert_eq!(owner.observe_inventory_capacity(stale).await, None);
        real_connection(&owner, admitted, expected).await?;
        // The index-bounded observation still rejects a scope whose row a newer receipt
        // supersedes (a successor's row lost, or a rolled-back row).
        let fact = h.node.fact();
        let node_id = *fact.node_id().as_bytes();
        let scope_key: Vec<u8> = sqlx::query_scalar(
            "SELECT scope_key FROM game_runtime_scope_assignments \
             WHERE holder_node_id = encode($1,'hex')::uuid AND state = 1 ORDER BY scope_key LIMIT 1",
        )
        .bind(node_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let observation_valid = "SELECT game_runtime_scope_assignment_observation_valid($1)";
        let mut tx = h.pool.begin().await?;
        assert!(
            sqlx::query_scalar::<_, bool>(observation_valid)
                .bind(&scope_key)
                .fetch_one(&mut *tx)
                .await?,
            "the current scope observes as valid"
        );
        sqlx::query(
            "UPDATE game_runtime_scope_assignment_writer \
             SET source_revision_high_water = source_revision_high_water + 1 WHERE writer_id = 1",
        )
        .execute(&mut *tx)
        .await?;
        // Retain the granted command's kind and actor: the scope-grant trigger decodes
        // these fields even for this synthetic successor used to model a lost scope row.
        sqlx::query(
            "INSERT INTO game_runtime_scope_assignment_receipts (operation_key, command, scope_key, \
             ownership_generation, state, holder_node_id, holder_registration_revision, \
             source_revision, decision_identity, decided_at) \
             SELECT sha256(a.operation_key), r.command, a.scope_key, a.ownership_generation + 1, \
             a.state, a.holder_node_id, a.holder_registration_revision, \
             w.source_revision_high_water, 'invcap-superseded-assignment', a.decided_at \
             FROM game_runtime_scope_assignments a \
             JOIN game_runtime_scope_assignment_receipts r ON r.operation_key = a.operation_key, \
             game_runtime_scope_assignment_writer w \
             WHERE a.scope_key = $1 AND w.writer_id = 1",
        )
        .bind(&scope_key)
        .execute(&mut *tx)
        .await?;
        assert!(
            !sqlx::query_scalar::<_, bool>(observation_valid)
                .bind(&scope_key)
                .fetch_one(&mut *tx)
                .await?,
            "a superseded assignment row must not observe as valid"
        );
        tx.rollback().await?;
        // An ended (revoked) registration of the proven node reads nothing.
        assert!(
            sqlx::query_scalar::<_, bool>(
                "SELECT game_node_end_registration(encode($1,'hex')::uuid, \
                 $2::text::numeric(20,0), 2::smallint, NULL)",
            )
            .bind(node_id.as_slice())
            .bind(fact.registration_revision().to_string())
            .fetch_one(&h.pool)
            .await?
        );
        assert_eq!(
            h.root
                .read_inventory_capacity(&authority, &h.node, fence, None, &weights)
                .await
                .map_err(debug)?,
            None,
            "an ended registration must not observe as current"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

// A second connection probes while the production observer's transaction is active. The
// lookup still delegates to the admitted content; no production test hook or guessed sleep.
struct ObservationLockProbe<'a> {
    weights: &'a dyn crate::durability::item_transfer::ItemWeightLookup,
    options: sqlx::postgres::PgConnectOptions,
    root: std::sync::OnceLock<Result<(), String>>,
    carried: std::sync::OnceLock<Result<(), String>>,
}

impl ObservationLockProbe<'_> {
    fn probe(&self) -> Result<(), String> {
        let options = self.options.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime.block_on(async {
                use sqlx::Connection;
                let mut connection = sqlx::PgConnection::connect_with(&options).await?;
                let mut tx = connection.begin().await?;
                for (sql, required) in [
                    ("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT", true),
                    ("SELECT 1 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT", true),
                    ("SELECT 1 FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT", false),
                ] {
                    let rows = sqlx::query(sql)
                        .bind(id(41).as_slice())
                        .fetch_all(&mut *tx)
                        .await?;
                    // An unassigned vocation legitimately has no build row.
                    if required {
                        assert!(!rows.is_empty(), "lock probe needs the fixture row");
                    }
                }
                sqlx::query("SELECT 1 FROM game_item_instances WHERE world_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT")
                    .bind(id(42).as_slice()).fetch_all(&mut *tx).await?;
                tx.rollback().await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            })
            .map_err(|error| error.to_string())
        })
        .join()
        .map_err(|_| "capacity lock probe thread panicked".to_owned())?
    }
}

impl crate::durability::item_transfer::ItemWeightLookup for ObservationLockProbe<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.weights.generation_digest()
    }

    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.root.get_or_init(|| self.probe());
        self.weights.wheel_premium_current(account)
    }

    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        self.carried.get_or_init(|| self.probe());
        self.weights.weight_of(definition)
    }
}

// The admission and lease are the fixture's committed facts; the serve loop, the sidecar
// delta, the client session decoder and the client panel reducer are production code.
async fn real_connection(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    expected: InventoryCapacity,
) -> TestResult {
    let (mut server, stream) = tokio::io::duplex(256 * 1024);
    let serving = async {
        let bytes = read_frame(&mut server).await?;
        assert_eq!(
            decode_wire_envelope(&bytes).map_err(debug)?.message_type(),
            MessageType::ClientBootstrap
        );
        let accepted = encode_server_accepted(&ServerAcceptedValue {
            game_session_id: admitted.game_session_id,
            world_id: admitted.world_id,
            channel_id: admitted.channel_id,
            connection_generation: admitted.continuity.connection_generation,
            current_server_sequence: admitted.continuity.server_sequence,
            next_command_id: admitted.continuity.next_command_id,
            schema_revision: 1,
            selected_capabilities: admitted.continuity.selected_capabilities.as_slice(),
        })
        .map_err(debug)?;
        write_frame(&mut server, &accepted).await?;
        Ok::<_, Box<dyn std::error::Error>>(
            serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await,
        )
    };
    let driving = async {
        let mut session = client::Session::admit(
            stream,
            client::Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: oteryn_protocol_oteryn::CharacterId::decode(&id(41))
                    .map_err(debug)?,
                admission_material: b"committed fixture admission",
                client_build_id: "inventory-capacity-e2e",
                supported_capabilities: client::CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(10),
            },
        )
        .await?;
        let received = loop {
            let pushed = session.take_events().into_iter().find_map(|event| match event {
                client::SessionEvent::InventoryCapacity(delta) => Some(delta.value),
                _ => None,
            });
            if let Some(capacity) = pushed {
                break capacity;
            }
            session.service_liveness(Duration::from_millis(20)).await?;
        };
        assert_eq!(received, expected);
        let mut panel = oteryn_client_ui::inventory::InventoryPanel::default();
        oteryn_client_ui::inventory_capacity::apply(&mut panel, received);
        assert_eq!(
            oteryn_client_ui::inventory_capacity::cap_text(&panel),
            Some(format!("Cap: {}", expected.free / 100))
        );
        // An unchanged capacity is not sent again.
        session.service_liveness(Duration::from_millis(500)).await?;
        assert!(!session.take_events().iter().any(|event| matches!(
            event,
            client::SessionEvent::InventoryCapacity(_)
        )));
        drop(session);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, ()) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(serving, driving)
    })
    .await
    .map_err(|_| "inventory capacity real connection timed out")??;
    assert!(
        matches!(
            end,
            ConnectionEnd::AdmittedThenDisconnected(_) | ConnectionEnd::AdmittedThenControlLost(_)
        ),
        "{end:?}"
    );
    Ok(())
}
