//! P12c: real attack refresh -> fenced snapshot -> command 388 -> durable read-back.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::character_authority::CharacterAuthorityError;
use crate::durability::cyclopedia_character::combat::CombatSnapshot;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, ConnectionGeneration};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::combat::{CombatStat as Stat, CombatStats};
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, CharacterTab,
};
use sqlx::Row;

#[test]
fn p12c_cyclopedia_combat_postgres_dispatch_snapshot_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "P12c combat scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "p12c_combat", true, false).await?;
        eprintln!("POSTGRES_STARTED P12C cyclopedia_combat_snapshot");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::equipment_mods::activate(deps.admitted.world_id)?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        deps.room = pin.qualified_room().clone();
        deps.combat_room = Some(deps.room.clone());
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            deps.room.map_revision_digest(),
            (2, 11, -7),
        );
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.activation_sequence().to_string())
            .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
            .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let maximum_health = u64::try_from(
            runtime
                .companion_policy("oteryn:creature.rat")
                .map_err(debug)?
                .maximum_health,
        )?;
        runtime
            .realize_activation_spawns(vec![(
                vec![(1, 11, -7)],
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                maximum_health,
            )])
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.admitted.runtime_actor = Some(deps.actor);
        seed(&h).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let effects = owner
            .read_admission_effects(deps.admitted.game_session_id)
            .await
            .map_err(|_| "read effects")?
            .ok_or("effects")?;
        let armor = effects.1;
        {
            let runtime = deps.runtime.lock().await;
            owner
                .install_admission_suppressions(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    Some(effects),
                )
                .await
                .ok_or("install equipment")?;
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let tab = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, tab, &[8, 0, 0, 1])
                .await?
                .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            row_count(&h).await?,
            0,
            "unselected capability cannot populate statistics"
        );
        let offers = [
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 17,
                requires: &[6],
            },
            OfferedCapability {
                id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                requires: &[],
            },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[6, 17, CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("capabilities")?;
        let target = {
            let runtime = deps.runtime.lock().await;
            let visible = runtime.visible_entities();
            let row = visible.creatures.first().ok_or("rat")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: row.actor.placement_identity(),
                generation: row.generation,
            }
        };
        let intent = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&target))
            .map_err(debug)?;
        let (status, result) = exchange(&owner, &mut admitted, 2, 11, &intent).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            result,
            oteryn_protocol_oteryn::attack::encode_attack_intent_result(
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            )
        );
        owner
            .refresh_attack_weapon(deps.actor, admitted.game_session_id)
            .await;
        assert_eq!(
            row_count(&h).await?,
            1,
            "the production weapon refresh persists statistics before the first tab query"
        );
        // The following connection exercises only the tab capability, so an independent
        // periodic attack observation cannot obscure the command-replay write count.
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("tab capability")?;
        let (status, offence_bytes) =
            exchange(&owner, &mut admitted, 3, tab, &[8, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let offence = CombatStats::decode(&offence_bytes).map_err(debug)?;
        assert_eq!(offence.get(Stat::CriticalChance), Some(1_050_000));
        assert_eq!(offence.get(Stat::LifeLeech), Some(500_000));
        assert_eq!(offence.get(Stat::Cleave), Some(1_000_000));
        assert_eq!(offence.get(Stat::PerfectShot1), Some(42));
        assert_eq!(offence.get(Stat::OnslaughtEquipment), Some(90_500));
        assert_eq!(offence.get(Stat::AttackTotal), None);
        let version = sample_revision(&h).await?;
        let (status, replay) = exchange(&owner, &mut admitted, 3, tab, &[8, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(offence_bytes, replay);
        assert_eq!(
            sample_revision(&h).await?,
            version,
            "wire replay cannot sample or write twice"
        );
        let (status, bytes) = exchange(&owner, &mut admitted, 4, tab, &[9, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let defence = CombatStats::decode(&bytes).map_err(debug)?;
        assert_eq!(defence.get(Stat::Armor), Some(i64::from(armor)));
        assert_eq!(defence.get(Stat::PhysicalReflection), Some(99));
        assert_eq!(defence.get(Stat::Dodge), None);
        assert_eq!(defence.get(Stat::Mitigation), None);
        let (status, bytes) = exchange(&owner, &mut admitted, 5, tab, &[10, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let misc = CombatStats::decode(&bytes).map_err(debug)?;
        assert_eq!(misc.get(Stat::Amplification), Some(574_000));
        assert_eq!(misc.get(Stat::Blessings), Some(2));
        assert_eq!(misc.get(Stat::MaximumBlessings), Some(7));
        assert!(
            misc.get(Stat::WealthDuplexSeconds)
                .is_some_and(|value| value > 0 && value <= 1800)
        );
        assert_eq!(misc.get(Stat::Momentum), None);
        assert_eq!(misc.get(Stat::OtherConcoctions), None);
        let version = sample_revision(&h).await?;
        let mut foreign = vec![8, 0, 0, 1];
        foreign.extend_from_slice(&id(60));
        for (command, bytes) in [
            (6, foreign.as_slice()),
            (7, &[11, 0, 0, 1][..]),
            (8, &[8, 0, 0][..]),
        ] {
            assert_eq!(
                exchange(&owner, &mut admitted, command, tab, bytes)
                    .await?
                    .0,
                CommandStatus::Rejected
            );
        }
        let mut stale_admitted = admitted;
        let mut stale_fence = fence;
        stale_fence.character_lease_generation -= 1;
        stale_admitted.item_fence = Some(stale_fence);
        assert!(
            owner
                .cyclopedia_character_wire(stale_admitted, tab, &[8, 0, 0, 1])
                .await
                .is_err()
        );
        assert_eq!(
            sample_revision(&h).await?,
            version,
            "malformed, unconsented and stale requests write nothing"
        );

        // An owner restart reads the committed bytes, including their observation time and
        // revisions. Exact retries use the snapshot receipt rather than re-counting misc facts.
        let row = sqlx::query("SELECT observed_owner_micros::text,offence,defence,misc FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(
            &owner,
            admitted.game_session_id,
        )
        .await
        .ok_or("equipment")?;
        let snapshot = CombatSnapshot {
            observed_owner_micros: row
                .try_get::<String, _>("observed_owner_micros")?
                .parse::<u64>()?
                + 1,
            equipment,
            offence: CombatStats::decode(&row.try_get::<Vec<u8>, _>("offence")?).map_err(debug)?,
            defence: CombatStats::decode(&row.try_get::<Vec<u8>, _>("defence")?).map_err(debug)?,
            misc: CombatStats::decode(&row.try_get::<Vec<u8>, _>("misc")?).map_err(debug)?,
        };
        let receipt = h
            .root
            .record_cyclopedia_combat(&authority, &h.node, fence, snapshot.clone())
            .await
            .map_err(debug)?;
        assert!(!receipt.replayed);
        let replay = h
            .root
            .record_cyclopedia_combat(&authority, &h.node, fence, snapshot.clone())
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(replay.sample_revision, receipt.sample_revision);
        let mut changed = snapshot.clone();
        changed
            .offence
            .set(Stat::CriticalChance, 1)
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .record_cyclopedia_combat(&authority, &h.node, fence, changed)
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut older = snapshot.clone();
        older.observed_owner_micros -= 1;
        assert!(matches!(
            h.root
                .record_cyclopedia_combat(&authority, &h.node, fence, older)
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut changed_equipment = snapshot.clone();
        changed_equipment.equipment.revision += 1;
        changed_equipment.offence.equipment_revision += 1;
        changed_equipment.defence.equipment_revision += 1;
        changed_equipment.misc.equipment_revision += 1;
        assert!(matches!(
            h.root
                .record_cyclopedia_combat(&authority, &h.node, fence, changed_equipment)
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut stale_connection = fence;
        stale_connection.connection_generation = ConnectionGeneration::new(999).map_err(debug)?;
        let mut foreign_fence = fence;
        foreign_fence.character_id =
            crate::domain::CharacterId::from_bytes(id(60)).map_err(debug)?;
        for refused in [stale_fence, stale_connection, foreign_fence] {
            assert!(
                h.root
                    .record_cyclopedia_combat(&authority, &h.node, refused, snapshot.clone())
                    .await
                    .is_err()
            );
        }
        assert_eq!(
            sample_revision(&h).await?,
            i64::try_from(receipt.sample_revision)?
        );
        assert_eq!(row_count(&h).await?, 1);
        let read = h
            .root
            .read_cyclopedia_character_tab(
                &authority,
                &h.node,
                fence,
                fence.character_id,
                CharacterTab::Offence,
                oteryn_protocol_oteryn::character_info::PageRequest::new(0, 1).ok_or("page")?,
                i64::try_from(snapshot.offence.observed_at_unix_ms)?,
            )
            .await
            .map_err(debug)?
            .encode()
            .map_err(debug)?;
        assert_eq!(CombatStats::decode(&read).map_err(debug)?, snapshot.offence);

        // A consented foreign target skips the self-refresh writer, so the read itself must
        // prove the viewer's current lease/connection. Reuse source-produced snapshot bytes
        // for this privileged peer fixture; the request never supplies a combat observation.
        super::fixture::pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_cyclopedia_combat(character_id,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc) SELECT encode($2,'hex')::uuid,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (status, foreign_read) = exchange(&owner, &mut admitted, 9, tab, &foreign).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            CombatStats::decode(&foreign_read).map_err(debug)?,
            snapshot.offence
        );
        for refused in [stale_fence, stale_connection] {
            let mut stale_viewer = admitted;
            stale_viewer.item_fence = Some(refused);
            // Keep the admitted envelope internally consistent: the durable current fence,
            // rather than a local field mismatch, must reject this foreign read.
            stale_viewer.continuity.connection_generation = refused.connection_generation.get();
            assert!(
                owner
                    .cyclopedia_character_wire(stale_viewer, tab, &foreign)
                    .await
                    .is_err()
            );
        }
        assert_eq!(
            sample_revision(&h).await?,
            i64::try_from(receipt.sample_revision)?
        );
        let peer_revision: i64 = sqlx::query_scalar("SELECT sample_revision FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(peer_revision, i64::try_from(receipt.sample_revision)?);
        assert_eq!(
            row_count(&h).await?,
            2,
            "foreign inspection never writes a sample"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        eprintln!("P12C POSTGRES_CASE_COMPLETED cyclopedia_combat_snapshot");
        h.cleanup().await
    })
}

async fn seed(h: &Harness) -> TestResult {
    // Only custody/effect provisioning is fixture SQL. Every observed value passes through the
    // native content, equipment and combat owners before the new writer persists it.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    for (seed, key, slot) in [
        (112_u8, "oteryn:item.tibia.i3264", 6_i16),
        (114, "oteryn:item.tibia.i6529", 4),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,10)",
        )
        .bind(id(seed).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    for key in [
        "oteryn:blessing.spiritual_shielding",
        "oteryn:blessing.embrace_of_tibia",
        "oteryn:blessing.twist_of_fate",
    ] {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'fixture:p12c') ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).bind(key).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_concoction_effects(transaction_id,character_id,concoction,started_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'wealth_duplex',now()-interval '30 minutes',now()+interval '30 minutes')")
        .bind(id(115).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn row_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_cyclopedia_combat")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn sample_revision(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT sample_revision FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
