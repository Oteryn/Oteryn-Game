//! M23 chosen personal trial: real native B3 + framed USE/STEP, canonical journal.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

fn data() -> TestResult<serde_json::Value> {
    Ok(serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/last-creep-native.json"
    )))?)
}
fn pos(v: &serde_json::Value) -> TestResult<Position> {
    Ok(serde_json::from_value(v.clone())?)
}
pub(super) fn start() -> Position {
    Position {
        x: 33262,
        y: 32365,
        floor: -7,
    }
}

#[test]
fn m23_last_creep_eight_disguises_matching_portals_and_returns_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let begin = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m23-last-creep", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-m23-last-creep",
            Position {
                x: 33261,
                y: 32365,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "M23_LAST_CREEP_PG_COMPLETE duration={:?} success={}",
        begin.elapsed(),
        result.is_ok()
    );
    if result.is_ok() {
        assert!(begin.elapsed() > std::time::Duration::from_millis(10));
    }
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let d = data()?;
    let tile = |p: Position, ids: Vec<u32>| ExpectedTile {
        position: (p.x, p.y),
        ids,
        counted_entry: None,
    };
    let mut expected = vec![
        tile(pos(&d["central"])?, vec![409]),
        tile(pos(&d["safe"])?, vec![409]),
    ];
    for key in ["return_portal", "exit_portal"] {
        expected.push(tile(pos(&d[key])?, vec![409, 1949]));
    }
    let mut appearances = vec![12251];
    for form in d["forms"].as_array().ok_or("forms")? {
        let id = u16::try_from(form["appearance"].as_u64().ok_or("appearance")?)?;
        appearances.push(id);
        expected.push(tile(pos(&form["portal"])?, vec![409, 1949]));
        expected.push(tile(pos(&form["marker"])?, vec![409, u32::from(id)]));
    }
    native_scene_with_appearances(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y126.b3"
                )),
                region: (7, 129, 126),
                bounds: (33261, 32365, 33265, 32368),
                start: (33262, 32365),
                expected: vec![
                    tile(start(), vec![231, 1703]),
                    tile(pos(&d["start"])?, vec![231, 12047]),
                    tile(pos(&d["outside"])?, vec![231]),
                    tile(
                        Position {
                            x: 33264,
                            y: 32367,
                            floor: -7,
                        },
                        vec![231],
                    ),
                    tile(
                        Position {
                            x: 33263,
                            y: 32367,
                            floor: -7,
                        },
                        vec![231, 1709],
                    ),
                    tile(
                        Position {
                            x: 33262,
                            y: 32367,
                            floor: -7,
                        },
                        vec![231, 1706],
                    ),
                    tile(
                        Position {
                            x: 33262,
                            y: 32366,
                            floor: -7,
                        },
                        vec![231, 1703],
                    ),
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z15-x080-y080.b3"
                )),
                region: (15, 80, 80),
                bounds: (20480, 20480, 20512, 20512),
                start: (20494, 20492),
                expected,
            },
        ],
        &[],
        &appearances,
    )
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Position> {
    let runtime = owner.runtime.lock().await;
    let p = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok(Position {
        x: u16::try_from(p.x).map_err(debug)?,
        y: u16::try_from(p.y).map_err(debug)?,
        floor: -i8::try_from(p.floor)?,
    })
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult<(u64, Vec<u8>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(500))
        .ok_or("clock")?;
    let id = admitted.continuity.next_command_id;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        id,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        payload.clone(),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok((id, payload))
}
async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
) -> TestResult {
    for _ in 0..96 {
        let p = position(owner, admitted).await?;
        if p == target {
            return Ok(());
        }
        assert_eq!(
            p.floor, target.floor,
            "walking never substitutes a teleport"
        );
        let dir = if p.x < target.x {
            StepDirection::East
        } else if p.x > target.x {
            StepDirection::West
        } else if p.y < target.y {
            StepDirection::South
        } else {
            StepDirection::North
        };
        step(owner, admitted, dir).await?;
    }
    Err("walk did not reach native target".into())
}
async fn use_marker(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
) -> TestResult<UseDisposition> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn exchange_shapechanger(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<(u64, Vec<u8>, UseDisposition)> {
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i12251", "definition-r1"))
        .ok_or("Shapechanger native reference")?;
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
    let id = admitted.continuity.next_command_id;
    let (end, result, actual) =
        super::rust_lottery::exchange(owner, *admitted, id, &payload, Some(reference), None)
            .await?;
    *admitted = end;
    Ok((id, actual, result))
}
async fn use_shapechanger(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<(u64, Vec<u8>)> {
    let (id, actual, result) = exchange_shapechanger(owner, admitted).await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "eligible central-floor Shapechanger USE must install this canonical form"
    );
    Ok((id, actual))
}
async fn journal(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT coalesce(jsonb_object_agg(track_key,value),'{}'::jsonb) FROM game_character_quest_tracks WHERE quest_key='oteryn:quest.authored.last_creep_standing'").fetch_one(&h.pool).await?)
}
async fn item(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i12251'").fetch_one(&h.pool).await?)
}
async fn assert_form(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    id: u64,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .ok_or("player")?;
    let expected = format!("oteryn:item.tibia.i{id}");
    assert!(
        matches!(state.owned_conditions().displayed_temporary_appearance_at(owner.owner_now().get()),Some(oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance::Item(item)) if item.definition_key()==expected)
    );
    let saved = state
        .owned_conditions()
        .persistent_snapshot(owner.owner_now().get())
        .map_err(debug)?;
    let restored=oteryn_gameplay_foundation::foundation::condition::ConditionStore::<String>::from_persistent_snapshot(&saved,0).map_err(debug)?;
    assert!(
        restored
            .get(crate::domain::condition::ConflictKey::Outfit)
            .is_none(),
        "temporary trial disguise is not logout-persistent"
    );
    drop(states);
    drop(runtime);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let entity = oteryn_protocol_oteryn::appearance::EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    };
    let expected_ref = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|index| index.definition_ref("Item", &expected, "definition-r1"))
        .ok_or("qualified form reference")?;
    let shown = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("projected form")?;
    assert!(
        matches!(&shown.bindings.first().ok_or("form binding")?.look, oteryn_protocol_oteryn::appearance::Look::Item {definition_ref} if *definition_ref == expected_ref.get()),
        "each canonical form reaches the client appearance projection"
    );
    Ok(())
}

/// Replace only the existing ephemeral Outfit slot under its real live owners.
/// Both halves remain real qualified content; no precomputed gate/proof is injected.
async fn replace_fixture_disguise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    condition: &str,
    appearance: u64,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("condition lane")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "condition fence")?
        .ok_or("condition fence")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let key = format!("oteryn:item.tibia.i{appearance}");
    native
        .spell_appearances()
        .and_then(|registry| registry.for_item(&key, "definition-r1"))
        .ok_or("qualified replacement item look")?;
    let runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let lease = runtime
        .bound_attacker_lease(actor, admitted.game_session_id)
        .map_err(debug)?;
    assert_eq!(
        lease.character_id().as_bytes(),
        fence.character_id.as_bytes()
    );
    assert_eq!(lease.generation(), fence.character_lease_generation);
    assert_eq!(
        runtime.binding().runtime_scope(),
        fence.item_fence().runtime_scope
    );
    assert_eq!(
        runtime.binding().scope_generation(),
        fence.item_fence().scope_ownership_generation
    );
    let mut states = owner.states.spell_states.lock().await;
    let current = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("player")?;
    let mut next = current.clone();
    let mut conditions = current.owned_conditions().clone();
    let definition =
        crate::foundation::ConditionDefinition::new_permanent_item_outfit(condition, 1)
            .and_then(|definition| {
                definition.with_item_appearance(&key, "definition-r1", native.source_digest())
            })
            .ok_or("replacement condition")?;
    let decision =
        oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(native.source_digest());
    let facts = crate::foundation::ApplicationFacts {
        now: owner.owner_now().get(),
        base_speed: 1,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &decision,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([8; 16]),
    };
    conditions
        .apply(
            &definition,
            None,
            crate::foundation::ConditionSourceKind::SelfUse,
            &[],
            &facts,
        )
        .map_err(debug)?;
    next.apply_batch_conditions(current.owned_conditions(), &conditions)
        .map_err(debug)?;
    next.advance_batch_revision().map_err(debug)?;
    assert!(states.commit(&runtime, actor, admitted.game_session_id, next));
    states.owner_wake.notify_one();
    Ok(())
}
async fn assert_condition_key(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    expected: &str,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let player = states
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .ok_or("player")?;
    let outfit = player
        .owned_conditions()
        .get(crate::domain::condition::ConflictKey::Outfit)
        .ok_or("outfit")?;
    assert_eq!(outfit.definition().key(), expected);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let d = data()?;
    let forms = d["forms"].as_array().ok_or("forms")?;
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
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let revision = canonical.content_revision().to_owned();
    let prefix = "oteryn:quest-progress/authored/last_creep_standing/s";
    let definitions = (1..=12)
        .filter_map(|i| canonical.track(&format!("{prefix}{i}")).cloned())
        .collect::<Vec<_>>();
    assert!(definitions.len() >= 9);
    let s1 = canonical
        .track(&format!("{prefix}1"))
        .ok_or("canonical Casper stage")?
        .clone();
    let fixture = "oteryn:quest-transition/m23-last-creep-fixture-casper";
    let mut transitions = forms
        .iter()
        .map(|f| {
            let key = f["callback"].as_str().ok_or("callback")?;
            canonical
                .transition(key)
                .cloned()
                .ok_or("canonical portal transition")
        })
        .collect::<Result<Vec<_>, _>>()?;
    transitions.push(
        canonical
            .transition("oteryn:quest-callback/last-creep-native-start")
            .cloned()
            .ok_or("start transition")?,
    );
    transitions.push(QuestTransition {
        key: fixture.into(),
        quest: s1.quest.clone(),
        effects: vec![QuestEffect {
            track: s1.key.clone(),
            from: QuestComparison::Eq(s1.initial),
            effect: QuestEffectKind::Set(1),
        }],
        completes: false,
        experience: None,
        world_change: None,
    });
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(&revision, definitions, transitions).map_err(debug)?,
    ));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    {
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
    // M01a owns TALK/acquisition. The fixture uses its ordinary durable writer and MINT.
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: start(),
            item: "oteryn:item.tibia.i12251".into(),
            count: 1,
        },
        710,
        true,
    )
    .await?;
    assert_eq!(
        use_marker(owner, admitted, pos(&d["start"])?).await?,
        UseDisposition::Rejected,
        "retained tool cannot bypass canonical Casper TALK"
    );
    assert_eq!(position(owner, admitted).await?, start());
    super::qengine_outfit_stories::commit_fixture(owner, admitted, fixture, 9800).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let before = item(h).await?;
    let before_journal = journal(h).await?;
    let (_, _, outside_use) = exchange_shapechanger(owner, admitted).await?;
    assert_ne!(
        outside_use,
        UseDisposition::Committed,
        "an eligible form still requires the central white tile"
    );
    assert_eq!(position(owner, admitted).await?, start());
    assert_eq!(journal(h).await?, before_journal);
    assert_eq!(item(h).await?, before);
    assert_eq!(
        use_marker(owner, admitted, pos(&d["start"])?).await?,
        UseDisposition::Committed
    );
    let central = pos(&d["central"])?;
    assert_eq!(position(owner, admitted).await?, central);
    let normal = owner
        .appearance_observation(actor, admitted.game_session_id, vec![entity])
        .await
        .ok_or("normal appearance")?;
    let source_id:sqlx::types::Uuid=sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i12251'").fetch_one(&h.pool).await?;
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
            oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
                    std::num::NonZeroU64::new(1).ok_or("handle")?,
                ),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
            Some(crate::gameplay_transport::item_view::ItemKey::Instance(
                *source_id.as_bytes(),
            )),
            false,
        )
        .await;
    assert!(
        !matches!(rejected,Ok(Some(outcome)) if outcome.disposition==UseDisposition::Committed)
    );
    for (i, form) in forms.iter().enumerate() {
        let (id, payload) = use_shapechanger(owner, admitted).await?; // intended semantic RED without Rule12251 registration
        let appearance = form["appearance"].as_u64().ok_or("form appearance")?;
        assert_form(owner, admitted, appearance).await?;
        if i == 0 {
            let callback = form["callback"].as_str().ok_or("callback")?;
            assert!(
                super::qengine_outfit_stories::commit_fixture(owner, admitted, callback, 9799)
                    .await
                    .is_err(),
                "generic quest writer cannot bypass the native STEP and disguise proof"
            );
            assert_eq!(journal(h).await?, before_journal);
        }
        let (end, replayed, _) =
            super::rust_lottery::exchange(owner, *admitted, id, &payload, None, None).await?;
        *admitted = end;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_form(owner, admitted, appearance).await?;
        if i == 0 {
            let option = normal.catalogue.outfits.first().ok_or("base outfit")?;
            let change = oteryn_protocol_oteryn::appearance::Change {
                context: normal.catalogue.context,
                expected_root_revision: normal.catalogue.root_revision,
                prior: normal.catalogue.selected.clone(),
                content_revision: revision.clone(),
                selection: oteryn_protocol_oteryn::appearance::Selection {
                    outfit_key: option.key.clone(),
                    colours: [1, 2, 3, 4],
                    addons: 0,
                    mount_key: None,
                },
            };
            let reply = super::quest_world_callbacks::wire(
                owner,
                admitted,
                admitted.continuity.next_command_id,
                oteryn_protocol_oteryn::appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
                change.encode().map_err(debug)?,
            )
            .await?;
            assert!(matches!(
                oteryn_protocol_oteryn::appearance::ChangeResult::decode(&reply).map_err(debug)?,
                oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
            ));
            assert_form(owner, admitted, appearance).await?;
            let wrong = pos(&forms[1]["portal"])?;
            walk(
                owner,
                admitted,
                Position {
                    x: wrong.x,
                    y: wrong.y + 1,
                    ..wrong
                },
            )
            .await?;
            // The callback gate may roll back a spatially accepted STEP. Its
            // security contract is the final position and durable state.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(500))
                .ok_or("clock")?;
            let wrong_reply = super::quest_world_callbacks::wire(
                owner,
                admitted,
                admitted.continuity.next_command_id,
                oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North),
            )
            .await?;
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&wrong_reply)
                .map_err(debug)?;
            assert_eq!(
                position(owner, admitted).await?,
                Position {
                    x: wrong.x,
                    y: wrong.y + 1,
                    ..wrong
                }
            );
            assert_eq!(
                journal(h).await?,
                before_journal,
                "wrong form/out-of-order portal never advances canonical stages"
            );
        }
        if i == 0 {
            // Keep canonical s1=1/s2=0 eligible: independently corrupt each
            // appearance/condition-key half and try the CURRENT matching portal.
            let correct_condition = form["condition"].as_str().ok_or("condition")?;
            let other_condition = forms[1]["condition"].as_str().ok_or("other condition")?;
            let other_appearance = forms[1]["appearance"].as_u64().ok_or("other appearance")?;
            let matching = pos(&form["portal"])?;
            let approach = Position {
                y: matching.y + 1,
                ..matching
            };
            for (look, condition) in [
                (appearance, other_condition),
                (other_appearance, correct_condition),
            ] {
                replace_fixture_disguise(owner, admitted, condition, look).await?;
                assert_form(owner, admitted, look).await?;
                assert_condition_key(owner, admitted, condition).await?;
                walk(owner, admitted, approach).await?;
                let unchanged = journal(h).await?;
                assert_eq!(unchanged, before_journal, "current phase remains eligible");
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_millis(500))
                    .ok_or("clock")?;
                let reply = super::quest_world_callbacks::wire(
                    owner,
                    admitted,
                    admitted.continuity.next_command_id,
                    oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North),
                )
                .await?;
                oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?;
                assert_eq!(
                    position(owner, admitted).await?,
                    approach,
                    "eligible portal rejects a mismatched ItemOutfit pair"
                );
                assert_eq!(
                    journal(h).await?,
                    unchanged,
                    "condition refusal never advances the eligible journal stage"
                );
                assert_eq!(
                    item(h).await?,
                    before,
                    "condition refusal retains the actual tool"
                );
                assert_form(owner, admitted, look).await?;
                assert_condition_key(owner, admitted, condition).await?;
                walk(owner, admitted, central).await?;
                use_shapechanger(owner, admitted).await?;
                assert_form(owner, admitted, appearance).await?;
                assert_condition_key(owner, admitted, correct_condition).await?;
            }
        }
        if i == 0 {
            // Exercise the exit with a live disguise, then physically walk around
            // the source rocks to reenter. Never replace movement with a position write.
            let exit = pos(&d["exit_portal"])?;
            walk(
                owner,
                admitted,
                Position {
                    y: exit.y - 1,
                    ..exit
                },
            )
            .await?;
            step(owner, admitted, StepDirection::South).await?;
            let outside = pos(&d["outside"])?;
            assert_eq!(position(owner, admitted).await?, outside);
            let restored = owner
                .appearance_observation(actor, admitted.game_session_id, vec![entity])
                .await
                .ok_or("exit appearance")?;
            assert!(matches!(
                restored.bindings[0].look,
                oteryn_protocol_oteryn::appearance::Look::Outfit {
                    colours: [1, 2, 3, 4],
                    ..
                }
            ));
            assert_eq!(
                journal(h).await?,
                before_journal,
                "exit never advances a form"
            );
            walk(
                owner,
                admitted,
                Position {
                    y: 32367,
                    ..outside
                },
            )
            .await?;
            walk(
                owner,
                admitted,
                Position {
                    x: 33262,
                    y: 32367,
                    floor: -7,
                },
            )
            .await?;
            walk(owner, admitted, start()).await?;
            assert_eq!(
                use_marker(owner, admitted, pos(&d["start"])?).await?,
                UseDisposition::Committed
            );
            assert_eq!(position(owner, admitted).await?, central);
            use_shapechanger(owner, admitted).await?;
            assert_form(owner, admitted, appearance).await?;
        }
        let portal = pos(&form["portal"])?;
        walk(
            owner,
            admitted,
            Position {
                x: portal.x,
                y: portal.y + 1,
                ..portal
            },
        )
        .await?;
        step(owner, admitted, StepDirection::North).await?;
        assert_eq!(position(owner, admitted).await?, pos(&d["safe"])?);
        let fresh = journal(h).await?;
        for (j, f) in forms.iter().enumerate() {
            let key = f["stage"].as_str().ok_or("stage")?;
            assert_eq!(
                fresh.get(key).and_then(|v| v.as_i64()).unwrap_or(0),
                i64::from(j <= i)
            );
        }
        walk(
            owner,
            admitted,
            Position {
                y: 20505,
                ..pos(&d["safe"])?
            },
        )
        .await?;
        let (return_id, return_payload) = step(owner, admitted, StepDirection::North).await?;
        assert_eq!(position(owner, admitted).await?, central);
        let restored = owner
            .appearance_observation(actor, admitted.game_session_id, vec![entity])
            .await
            .ok_or("restored")?;
        assert!(matches!(
            restored.bindings[0].look,
            oteryn_protocol_oteryn::appearance::Look::Outfit {
                colours: [1, 2, 3, 4],
                ..
            }
        ));
        let journal_before_replay = journal(h).await?;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            return_id,
            oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            return_payload,
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(position(owner, admitted).await?, central);
        assert_eq!(journal(h).await?, journal_before_replay);
        assert_eq!(
            item(h).await?,
            before,
            "Shapechanger remains unchanged across all trial stages"
        );
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let final_journal = journal(h).await?;
    for i in 10..=12 {
        let key = format!("{prefix}{i}");
        assert_eq!(
            final_journal.get(&key),
            before_journal.get(&key),
            "reward stages stay with their owner"
        );
    }
    let (consumptions,completed):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-callback/last-creep-native-%'),(SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.authored.last_creep_standing' AND completed_character_revision IS NOT NULL)").fetch_one(&h.pool).await?;
    assert_eq!((consumptions, completed), (0, 0));
    for form in forms {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(form["callback"].as_str().ok_or("callback")?)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(count, 1);
    }
    Ok(())
}
