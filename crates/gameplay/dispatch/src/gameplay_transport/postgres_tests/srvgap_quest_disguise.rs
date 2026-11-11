//! Source crate USE borrows existing carried quest and appearance owners without progress.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn srvgap1108_quest_item_disguise_fenced_use_restores_on_exact_exit_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srvgap1108-disguise", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-srvgap-disguise",
            Position {
                x: 33322,
                y: 31080,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "SRVGAP1108_DISGUISE_PG_COMPLETE duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let entity = oteryn_protocol_oteryn::appearance::EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    };
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let track = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
    let key = "oteryn:quest-transition/srvgap-disguise-fixture-stage";
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let content_revision = canonical.content_revision().to_owned();
    let definition = canonical
        .track(track)
        .cloned()
        .ok_or("source quest track")?;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            vec![definition.clone()],
            vec![
                QuestTransition {
                    key: key.into(),
                    quest: definition.quest.clone(),
                    effects: vec![QuestEffect {
                        track: track.into(),
                        from: QuestComparison::Eq(definition.initial),
                        effect: QuestEffectKind::Set(2),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
                QuestTransition {
                    key: format!("{key}-leave-stage"),
                    quest: definition.quest.clone(),
                    effects: vec![QuestEffect {
                        track: track.into(),
                        from: QuestComparison::Eq(2),
                        effect: QuestEffectKind::Set(3),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            ],
        )
        .map_err(debug)?,
    ));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    {
        // The shared qengine fixture positions an actor; native appearance reads
        // additionally require the admitted Character lease, as real first entry does.
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33321,
                y: 31080,
                floor: -7,
            },
            item: "oteryn:item.tibia.i11328".into(),
            count: 1,
        },
        710,
        true,
    )
    .await?;
    super::qengine_outfit_stories::commit_fixture(owner, admitted, key, 9800).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let before:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'quest',(SELECT jsonb_agg(to_jsonb(q) ORDER BY track_key) FROM game_character_quest_tracks q),'receipts',(SELECT count(*) FROM game_character_quest_receipts))").fetch_one(&h.pool).await?;
    let normal = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("normal appearance")?;
    let source_id:sqlx::types::Uuid=sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i11328'").fetch_one(&h.pool).await?;
    let intent = oteryn_protocol_oteryn::world_object::UseIntent {
        target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
            std::num::NonZeroU64::new(1).ok_or("handle")?,
        ),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let mut stale = fence.item_fence();
    stale.character_lease_generation += 1;
    let rejected = owner
        .use_event_item(
            actor,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 9900,
                item_fence: Some(stale),
            },
            intent,
            Some(crate::gameplay_transport::item_view::ItemKey::Instance(
                *source_id.as_bytes(),
            )),
            false,
        )
        .await;
    assert!(
        !matches!(rejected,Ok(Some(outcome)) if outcome.disposition==UseDisposition::Committed),
        "stale lease cannot install a disguise"
    );
    {
        // Incoming permanent source costume replaces a finite existing costume.
        let member = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.spell_appearances())
            .and_then(|a| a.for_look_type(1593))
            .ok_or("prior finite outfit")?;
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?;
        crate::spell::actor_conditions::apply_appearance(
            state,
            member.selection(),
            member.look_type(),
            600000,
            owner.owner_now().get() / 1000,
        )
        .map_err(debug)?;
        state.advance_batch_revision().map_err(debug)?;
    }
    let (use_command, use_bytes) = use_crate(owner, admitted).await?;
    let (end, replayed, _) =
        super::rust_lottery::exchange(owner, *admitted, use_command, &use_bytes, None, None)
            .await?;
    *admitted = end;
    assert_eq!(replayed, UseDisposition::Committed);

    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?;
        assert!(
            matches!(player.owned_conditions().displayed_temporary_appearance_at(owner.owner_now().get()),Some(oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance::Item(item)) if item.definition_key()=="oteryn:item.tibia.i11328")
        );
        let saved = player
            .owned_conditions()
            .persistent_snapshot(owner.owner_now().get())
            .map_err(debug)?;
        let restored=oteryn_gameplay_foundation::foundation::condition::ConditionStore::<String>::from_persistent_snapshot(&saved,0).map_err(debug)?;
        assert!(
            restored
                .get(crate::domain::condition::ConflictKey::Outfit)
                .is_none(),
            "source permanent outfit is not persistent across logout"
        );
    }
    let after:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'quest',(SELECT jsonb_agg(to_jsonb(q) ORDER BY track_key) FROM game_character_quest_tracks q),'receipts',(SELECT count(*) FROM game_character_quest_receipts))").fetch_one(&h.pool).await?;
    assert_eq!(
        before, after,
        "source retains crate, quest stage and receipt count"
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(
        map.facts
            .movement_speech_since(0)
            .iter()
            .filter(|(_, line)| line.text.starts_with("You remove its top"))
            .count(),
        1
    );
    assert!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .any(|(_, _, effect)| *effect == 3)
    );
    // A fresh USE while already disguised still has source speech/effect.
    use_crate(owner, admitted).await?;
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_speech_since(0)
            .iter()
            .filter(|(_, line)| line.text.starts_with("You remove its top"))
            .count(),
        2
    );
    let option = normal.catalogue.outfits.first().ok_or("base outfit")?;
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: normal.catalogue.context,
        expected_root_revision: normal.catalogue.root_revision,
        prior: normal.catalogue.selected.clone(),
        content_revision,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: option.key.clone(),
            colours: [1, 2, 3, 4],
            addons: 0,
            mount_key: None,
        },
    };
    let changed = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        oteryn_protocol_oteryn::appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&changed).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    let transformed = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("item appearance")?;
    assert!(
        matches!(
            transformed.bindings[0].look,
            oteryn_protocol_oteryn::appearance::Look::Item { .. }
        ),
        "changing base does not remove crate disguise"
    );
    for x in [33322, 33323] {
        if x == 33323 {
            // Source exit clears the entire Outfit slot, including a replacement.
            let member = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .and_then(|n| n.spell_appearances())
                .and_then(|a| a.for_look_type(1593))
                .ok_or("qualified replacement")?;
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, admitted.game_session_id)
                .ok_or("player")?;
            let before = state.owned_conditions().clone();
            let mut next = before.clone();
            let definition = crate::foundation::ConditionDefinition::new_permanent_outfit(
                "test.replacement.outfit",
                1,
                member.look_type(),
            )
            .and_then(|d| d.with_appearance(member.selection()))
            .ok_or("replacement definition")?;
            let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([7; 32]);
            let facts = crate::foundation::ApplicationFacts {
                now: owner.owner_now().get(),
                base_speed: 1,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [8; 16],
                ),
            };
            next.apply(
                &definition,
                None,
                crate::foundation::ConditionSourceKind::SelfUse,
                &[],
                &facts,
            )
            .map_err(debug)?;
            state
                .apply_batch_conditions(&before, &next)
                .map_err(debug)?;
            state.advance_batch_revision().map_err(debug)?;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(500))
            .ok_or("clock")?;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            ),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?;
        assert_eq!(
            player
                .owned_conditions()
                .displayed_temporary_appearance_at(owner.owner_now().get())
                .is_some(),
            x != 33323,
            "only exact exit restores appearance"
        );
    }
    let restored = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("restored appearance")?;
    assert!(
        matches!(
            restored.bindings[0].look,
            oteryn_protocol_oteryn::appearance::Look::Outfit {
                colours: [1, 2, 3, 4],
                ..
            }
        ),
        "exit reveals the changed base selection"
    );
    let speeches = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_speech_since(0);
    assert_eq!(
        speeches
            .iter()
            .filter(|(_, line)| line.text
                == "On reaching the watchtower you remove your temporary disguise.")
            .count(),
        1
    );
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        &format!("{key}-leave-stage"),
        9801,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let effects = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0);
    use_crate(owner, admitted).await?;
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_speech_since(0),
        speeches,
        "wrong quest stage acknowledges without source speech"
    );
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0),
        effects,
        "wrong quest stage has no effect"
    );
    let observed = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("stage noop appearance")?;
    assert_eq!(
        observed.bindings[0].look, restored.bindings[0].look,
        "wrong quest stage cannot install outfit"
    );
    assert_expired_replay(owner, admitted, use_command, &use_bytes).await?;
    let observed = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("replay observation")?;
    assert_eq!(
        observed.bindings[0].look, restored.bindings[0].look,
        "expired ingress replay cannot reinstall the disguise"
    );
    Ok(())
}

async fn use_crate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<(u64, Vec<u8>)> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let reference = native
        .item_index()
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i11328", "definition-r1"))
        .ok_or("item reference")?;
    let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(
        &oteryn_protocol_oteryn::world_object::UseIntent {
            target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
                std::num::NonZeroU64::new(1).ok_or("handle")?,
            ),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        },
    )
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (end, result, actual) =
        super::rust_lottery::exchange(owner, *admitted, command, &payload, Some(reference), None)
            .await?;
    *admitted = end;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source-qualified carried crate USE must install its disguise"
    );
    Ok((command, actual))
}

async fn assert_expired_replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_id: u64,
    payload: &[u8],
) -> TestResult {
    use crate::foundation::{FoundationProtocolError, MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let frame = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: 2,
            payload,
        },
    )?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = Box::pin(serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS)).await;
    let ConnectionEnd::AdmittedThenClosed(current, FoundationProtocolError::CommandOutcomeExpired) =
        end
    else {
        return Err(format!("expected expired replay closure, got {end:?}").into());
    };
    *admitted = current;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = output.as_slice();
    let mut expired = false;
    while !cursor.is_empty() {
        let length = usize::try_from(u32::from_be_bytes(cursor[..4].try_into()?))?;
        let envelope = decode_wire_envelope(&cursor[4..4 + length]).map_err(debug)?;
        assert_ne!(envelope.message_type(), MessageType::CommandResult);
        if envelope.message_type() == MessageType::ProtocolError {
            let error =
                oteryn_protocol_oteryn::decode_protocol_error(envelope.payload()).map_err(debug)?;
            assert_eq!(
                error.error_code,
                FoundationProtocolError::CommandOutcomeExpired as u32
            );
            assert_eq!(error.related_command_id, command_id);
            expired = true;
        }
        cursor = &cursor[4 + length..];
    }
    assert!(expired, "expired replay emits the exact protocol error");
    Ok(())
}
