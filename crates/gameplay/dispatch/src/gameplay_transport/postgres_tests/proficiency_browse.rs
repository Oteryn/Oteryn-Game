//! CL7: the production read owner exposes the complete pinned catalogue and live XP.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use oteryn_protocol_oteryn::{self as protocol, weapon_proficiency::ProficiencyPage};
use tokio::io::AsyncWriteExt;

#[test]
fn proficiency_browse_postgres_pinned_catalogue_live_xp_and_session_fence() -> TestResult {
    std::thread::Builder::new()
        .name("proficiency-browse".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency browse panicked")?
        .map_err(Into::into)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let harness =
            Harness::create_with_admission_seed(admin, "proficiency_browse", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let frame_digest = pin.frame_binding().digest();
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        owner.proficiency_catalogue = Some(catalogue.clone());
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let sword = "oteryn:item.tibia.i3264";
        let sword_definition = generation
            .native_gameplay()
            .ok_or("native")?
            .item_index()
            .ok_or("index")?
            .definition_ref("Item", sword, "definition-r1")
            .ok_or("sword reference")?;
        {
            let mut states = owner.states.proficiencies.lock().map_err(|_| "states")?;
            let state = states.entry(session).or_default();
            let mut track = catalogue.empty_track(sword).ok_or("sword")?;
            track.progress = 12345;
            state.install_track(sword, track);
        }
        let mut offset = 0;
        let mut keys = Vec::new();
        let mut found = false;
        loop {
            let page =
                FreshAdmissionAuthority::weapon_proficiency_browse(&owner, actor, session, offset)
                    .await
                    .ok_or("page")?;
            assert_eq!(
                ProficiencyPage::decode(&page.encode().ok_or("encode")?),
                Some(page.clone())
            );
            assert_eq!(page.offset, offset);
            assert!(page.rows.len() <= 20);
            for row in &page.rows {
                if row.definition == sword_definition {
                    assert_eq!(row.experience, 12345);
                    found = true;
                } else {
                    assert_eq!(row.experience, 0);
                }
                keys.push(row.definition.get());
            }
            offset += u32::try_from(page.rows.len())?;
            if offset == page.total {
                break;
            }
            assert!(!page.rows.is_empty());
        }
        assert!(found);
        assert!(keys.len() > 20);
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            FreshAdmissionAuthority::weapon_proficiency_browse(&owner, actor, session, u32::MAX)
                .await
                .is_none()
        );
        let wrong_session = crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
        assert!(
            FreshAdmissionAuthority::weapon_proficiency_browse(&owner, actor, wrong_session, 0)
                .await
                .is_none()
        );
        let sword_offset = u32::try_from(
            keys.iter()
                .position(|reference| *reference == sword_definition.get())
                .ok_or("sword index")?,
        )?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        admitted.item_fence = Some(
            owner
                .current_quest_fence(session)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?
                .item_fence(),
        );
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 6, 12, 14, 195],
            )
            .ok_or("capabilities")?;
        assert!(
            super::proficiency_select_perk::connection(&owner, admitted, async |client| {
                let page = wire_query(
                    client,
                    1,
                    &sword_offset.to_be_bytes(),
                    protocol::CommandStatus::Accepted,
                )
                .await?;
                assert_eq!(page.as_ref().map(|p| p.total), Some(offset));
                assert_eq!(
                    page.and_then(|p| p.rows.first().map(|row| row.experience)),
                    Some(12345)
                );
                assert!(
                    wire_query(client, 2, &[0], protocol::CommandStatus::Rejected)
                        .await?
                        .is_none()
                );
                assert!(
                    wire_query(
                        client,
                        3,
                        &u32::MAX.to_be_bytes(),
                        protocol::CommandStatus::Rejected
                    )
                    .await?
                    .is_none()
                );
                Ok(())
            })
            .await?
        );
        eprintln!(
            "POSTGRES_CASE_COMPLETED proficiency_browse: {} weapons, {:?}",
            keys.len(),
            started.elapsed()
        );
        Ok(())
    })
}

async fn wire_query(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    payload: &[u8],
    status: protocol::CommandStatus,
) -> TestResult<Option<ProficiencyPage>> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id,
            command_type: 641,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = super::item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.status, status);
            if status == protocol::CommandStatus::Accepted {
                return Ok(Some(
                    ProficiencyPage::decode(result.payload).ok_or("wire page")?,
                ));
            }
            assert!(result.payload.is_empty());
            return Ok(None);
        }
    }
}
