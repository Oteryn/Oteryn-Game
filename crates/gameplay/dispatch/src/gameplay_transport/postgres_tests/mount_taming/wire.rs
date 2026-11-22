use super::*;
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::{UseTarget, encode_item_use_intent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MUSIC_BOX: &str = "oteryn:item.tibia.i16244";
const TARGET_TILE: TilePos = TilePos {
    x: 101,
    y: 100,
    floor: -7,
};
const VOICES: [(&str, &str); 9] = [
    ("oteryn:creature.dragonling", "FI?"),
    ("oteryn:creature.draptor", "Screeeeeeeeech"),
    ("oteryn:creature.enraged_white_deer", "*bell*"),
    ("oteryn:creature.ironblight", "Plinngggg"),
    ("oteryn:creature.magma_crawler", "ZzzZzzZzzzZz"),
    ("oteryn:creature.midnight_panther", "Purrrrrrr"),
    ("oteryn:creature.panda", "Rrrrr... Grrrr"),
    ("oteryn:creature.wailing_widow", "Sssssssss"),
    ("oteryn:creature.wild_horse", "*snort*"),
];

#[test]
fn music_box_committed_tame_publishes_target_voice_and_red_effect_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_music_box_cues(admin, false)
}

#[test]
fn music_box_refused_or_replayed_tame_emits_no_new_cues() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_music_box_cues(admin, true)
}

fn run_music_box_cues(admin: String, check_refusal: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            postgres_fixture::runtime()
                .map_err(debug)?
                .block_on(music_box_cues_scenario(admin, check_refusal))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "music box presentation scenario panicked")?
        .map_err(Into::into)
}

async fn music_box_cues_scenario(admin: String, check_refusal: bool) -> TestResult {
    use super::super::{Dependencies, fixture::Harness, item_look_fixtures::seed_main_backpack};

    let h = Harness::create(admin, "music_box_cues", true).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = crate::content::native_gameplay::test_support::cosmetics_fixture::activated_with_optional_cosmetics(world, None, Some(&cosmetics()?));
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let index = native.item_index().ok_or("item index")?;
    let map = crate::map::spell_tiles::tests::movement_arena::rectangle((99, 100), (101, 100), -7);
    let entry = pin.qualified_room();
    let room = entry
        .bundle_view(
            map.movement_cells(entry.movement_cells()).map_err(debug)?,
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .ok_or("bundle room")?;
    let channel_pin = crate::foundation::ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (100, 100, 7),
    );
    let deps = Dependencies::with_content(
        channel_pin,
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
    }
    seed_main_backpack(&h, &[(MUSIC_BOX, i64::try_from(VOICES.len())?)]).await?;
    runtime_role(&h.root).await?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    deps.initialize(&owner).await;
    let session = deps.admitted.game_session_id;
    let mut retained = admitted(&owner, deps.actor, session)?;
    let source = index
        .definition_ref(
            "Item",
            MUSIC_BOX,
            index.revision(MUSIC_BOX).ok_or("source revision")?,
        )
        .ok_or("Music Box definition")?;
    let use_box = |entity| {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(source),
            use_with: Some(entity),
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)
    };

    let cues = &owner.bundle_map().ok_or("bundle presentation")?.facts;
    let mut cursor = 0;
    let cases = if check_refusal {
        &VOICES[..1]
    } else {
        &VOICES[..]
    };
    for &(species, text) in cases {
        let (target, entity, name) = spawn_speaking_creature(&owner, deps.actor, species).await?;
        let payload = use_box(entity)?;
        let command = retained.continuity.next_command_id;
        let (result, next) = exchange(&owner, retained, std::slice::from_ref(&payload)).await?;
        assert_eq!(result, [UseDisposition::Committed], "{species}");
        assert!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_identity(target)
                .is_err()
        );
        let effects = cues.movement_effects_since(cursor);
        let speech = cues.movement_speech_since(cursor);
        assert_eq!(effects.len(), 1, "{species}");
        assert_eq!((effects[0].1, effects[0].2), (TARGET_TILE, 20));
        assert_eq!(speech.len(), 1, "{species}");
        let voice = &speech[0].1;
        assert_eq!(
            (voice.speaker, voice.position, voice.talk_type),
            (target, TARGET_TILE, 36)
        );
        assert_eq!(
            voice
                .captured_name
                .as_ref()
                .and_then(|name| name.as_str().ok()),
            Some(name.as_str())
        );
        assert_eq!(voice.text, text);
        cursor = effects[0].0.max(speech[0].0);
        retained = next;
        if check_refusal {
            let mut replay = retained;
            replay.continuity.next_command_id = command;
            let (result, _) = exchange(&owner, replay, &[payload]).await?;
            assert_eq!(result, [UseDisposition::Committed]);
            assert!(cues.movement_effects_since(cursor).is_empty());
            assert!(cues.movement_speech_since(cursor).is_empty());

            let (target, entity, _) = spawn_speaking_creature(&owner, deps.actor, species).await?;
            let (result, _) = exchange(&owner, retained, &[use_box(entity)?]).await?;
            assert_eq!(result, [UseDisposition::RequirementNotMet]);
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_identity(target)
                    .is_ok()
            );
            assert!(cues.movement_effects_since(cursor).is_empty());
            assert!(cues.movement_speech_since(cursor).is_empty());
        }
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn spawn_speaking_creature(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    species: &str,
) -> TestResult<(
    ExactActorRef,
    oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    String,
)> {
    let (target, entity) = spawn(owner, actor, species, 1).await?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .install_creature_policy(target, species)
        .map_err(debug)?;
    let creature = runtime.companion_snapshot(target).map_err(debug)?;
    Ok((target, entity, creature.state.policy.display_name.clone()))
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
) -> TestResult<(Vec<UseDisposition>, AdmittedSession)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (command_id, payload) in (admitted.continuity.next_command_id..).zip(payloads) {
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload,
            },
        )?;
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
    }
    client.shutdown().await?;
    let end = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        &WireOwner(owner),
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    )
    .await;
    let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(retained) =
        end
    else {
        return Err(format!("{end:?}").into());
    };
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let frame =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
        if frame.message_type() == crate::foundation::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            results.push(
                oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                    .map_err(debug)?,
            );
        }
        cursor += 4 + length;
    }
    Ok((results, retained))
}
