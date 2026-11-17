//! Selected Ferumbras route and atomic world rewards over existing fenced owners.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_ferumbras_full_native_seals_and_reward_bundle() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-ferumbras", || {
        super::quest_world_callbacks::scenario(
            "questpc7-ferumbras",
            Position {
                x: 33275,
                y: 32390,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FERUMBRAS POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
const TRACK: &str = "oteryn:quest-progress/questpc7/ferumbras/programme";
const PORTALS: &str = "oteryn:quest-progress/questpc7/ferumbras/passages";
type ArenaPosition = (u16, u16, i8);
const ARRIVALS: [(u16, u16, i8); 8] = [
    (33173, 31504, -13),
    (33644, 32760, -11),
    (33584, 32693, -14),
    (33419, 32467, -14),
    (33479, 32339, -13),
    (33459, 32848, -11),
    (33485, 32791, -13),
    (33392, 31479, -14),
];
const ARENAS: [(&str, ArenaPosition, ArenaPosition, u16); 8] = [
    ("plagirath", (33172, 31501, -13), (33174, 31504, -13), 10534),
    ("zamulosh", (33643, 32756, -11), (33645, 32760, -11), 20714),
    ("mazoran", (33584, 32688, -14), (33583, 32693, -14), 7063),
    ("razzagorn", (33422, 32465, -14), (33420, 32467, -14), 10548),
    ("ragiaz", (33481, 32336, -13), (33479, 32340, -13), 20375),
    ("tarbaz", (33459, 32844, -11), (33460, 32848, -11), 10549),
    ("shulgrax", (33485, 32786, -13), (33486, 32791, -13), 19273),
    (
        "ferumbras_mortal_shell",
        (33392, 31473, -14),
        (33393, 31479, -14),
        9857,
    ),
];
fn pos(p: (u16, u16, i8)) -> Position {
    Position {
        x: p.0,
        y: p.1,
        floor: p.2,
    }
}
pub(super) fn objects() -> Vec<(Position, u16)> {
    let mut rows = vec![
        (pos((33275, 32388, -8)), 1949),
        (pos((33388, 31409, -14)), 4073),
        (pos((33388, 31410, -14)), 409),
    ];
    for (_, center, portal, appearance) in ARENAS {
        rows.push((pos(center), appearance));
        for x in portal.0 - 2..=portal.0 + 2 {
            for y in portal.1 - 2..=portal.1 + 2 {
                rows.push((
                    Position {
                        x,
                        y,
                        floor: portal.2,
                    },
                    appearance,
                ));
            }
        }
    }
    rows
}
pub(super) async fn value(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    key: &str,
) -> TestResult<i64> {
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, f.character_id)
        .await
        .map_err(debug)?;
    Ok(state.tracks().get(key).copied().unwrap_or(-1))
}
pub(super) async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    d: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let n = a.continuity.next_command_id;
    let r = super::quest_world_callbacks::wire(
        owner,
        a,
        n,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(d),
    )
    .await?;
    assert_eq!(
        decode_step_result(&r).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
pub(super) async fn walk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    target: Position,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    for _ in 0..32 {
        let p = owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        if p == super::quest_world_callbacks::local(target) {
            return Ok(());
        }
        if p.floor != -i16::from(target.floor) {
            return Err("wrong native destination floor".into());
        }
        let d = if p.x < i32::from(target.x) {
            StepDirection::East
        } else if p.x > i32::from(target.x) {
            StepDirection::West
        } else if p.y < i32::from(target.y) {
            StepDirection::South
        } else {
            StepDirection::North
        };
        let mut expected = p;
        match d {
            StepDirection::East => expected.x += 1,
            StepDirection::West => expected.x -= 1,
            StepDirection::South => expected.y += 1,
            StepDirection::North => expected.y -= 1,
            StepDirection::NorthEast => {
                expected.x += 1;
                expected.y -= 1;
            }
            StepDirection::SouthEast => {
                expected.x += 1;
                expected.y += 1;
            }
            StepDirection::SouthWest => {
                expected.x -= 1;
                expected.y += 1;
            }
            StepDirection::NorthWest => {
                expected.x -= 1;
                expected.y -= 1;
            }
        }
        step(owner, a, d).await?;
        // Every passage displacement, including a short same-floor teleport,
        // ends this walk; the story separately asserts its exact destination.
        let p = owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        if p != expected {
            return Ok(());
        }
    }
    Err("bounded walk".into())
}
pub(super) async fn use_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    p: Position,
    appearance: u16,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let payload = super::questpc7_hoist_story::target(owner, p, appearance)?;
    let n = a.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, a, n, 2, payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
pub(super) async fn guardian(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    p: Position,
) -> TestResult<Option<crate::foundation::ExactActorRef>> {
    let r = owner.runtime.lock().await;
    let found: Vec<_> = r
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .filter(|(_, at, s)| s.is_none() && at.position() == super::quest_world_callbacks::local(p))
        .map(|(a, _, _)| a)
        .collect();
    assert!(found.len() <= 1);
    Ok(found.first().copied())
}
async fn move_essence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    h: &Harness,
    command: u64,
    pickup: bool,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    let rows:Vec<(sqlx::types::Uuid,Option<Vec<u8>>)>=sqlx::query_as("SELECT i.item_instance_id,g.spatial_position FROM game_item_instances i LEFT JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i6499' ORDER BY i.item_instance_id LIMIT 2").fetch_all(&h.pool).await?;
    assert_eq!(rows.len(), 1, "the native merge has one live essence stack");
    let (id, cell) = &rows[0];
    assert_eq!(cell.is_some(), pickup);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let map = owner.bundle_map().ok_or("map")?;
    let (digest, frame) = {
        let r = owner.runtime.lock().await;
        (
            r.content_pin().server_artifact_digest(),
            r.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let destination = if pickup {
        LoopDestination::MainBackpack
    } else {
        LoopDestination::Ground(LoopGround {
            spatial_position: crate::durability::map_item_mint_audit::map_item_spatial_position(
                33275, 32389, -8,
            ),
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: revision,
            placement_context: frame.to_vec(),
        })
    };
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &f.item_fence(),
            crate::foundation::CommandRef::new(
                a.game_session_id,
                crate::foundation::CommandId::new(command).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: *id.as_bytes(),
                count: None,
                pickup_cell: cell.clone(),
                destination,
            },
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "native pickup/drop: {result:?}");
    Ok(())
}
async fn facts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    let tokens = super::questpc7_outfit_items::quantity(h, 22516).await?;
    let appearance:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:outfit.rift_warrior/quest-grant/%'").fetch_one(&h.pool).await?;
    let achievement:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/hat_hunter'").fetch_one(&h.pool).await?;
    Ok((tokens, appearance, achievement))
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_FERUMBRAS POSTGRES_STARTED version={version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let actor = a.runtime_actor.ok_or("actor")?;
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            a.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(f.character_id.as_bytes()).map_err(debug)?,
                f.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert_eq!(value(owner, a, TRACK).await?, -1, "no quest progress seeds");
    step(owner, a, StepDirection::East).await?;
    assert_eq!(value(owner, a, TRACK).await?, 1);
    walk(owner, a, pos((33275, 32389, -8))).await?;
    assert_eq!(
        use_at(owner, a, pos((33275, 32388, -8)), 1949).await?,
        UseDisposition::Rejected,
        "ten real essences required"
    );
    let offering = |count| oteryn_gameplay_quests::quest::world_callbacks::Offering {
        position: pos((33275, 32389, -8)),
        item: "oteryn:item.tibia.i6499".into(),
        count,
    };
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(owner, f.item_fence(), &offering(9), 15000, false)
        .await?;
    assert_eq!(
        use_at(owner, a, pos((33275, 32388, -8)), 1949).await?,
        UseDisposition::Rejected
    );
    assert_eq!(
        super::questpc7_outfit_items::quantity(h, 6499).await?,
        9,
        "short offering is not charged"
    );
    move_essence(owner, a, h, 18000, true).await?;
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(owner, f.item_fence(), &offering(1), 15001, true)
        .await?;
    assert_eq!(
        super::questpc7_outfit_items::quantity(h, 6499).await?,
        10,
        "ordinary carried merge gathers the complete offering"
    );
    move_essence(owner, a, h, 18001, false).await?;
    assert_eq!(
        use_at(owner, a, pos((33275, 32388, -8)), 1949).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::questpc7_outfit_items::quantity(h, 6499).await?, 0);
    assert_eq!(value(owner, a, TRACK).await?, 2);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(pos(ARRIVALS[0]))
    );
    for (i, (name, center, portal, _)) in ARENAS.iter().enumerate() {
        owner.refresh_quest_session(a.game_session_id, false).await;
        let target = guardian(owner, pos(*center))
            .await?
            .ok_or("native seal boss")?;
        if let Some(next) = ARENAS.get(i + 1) {
            assert_eq!(
                guardian(owner, pos(next.1)).await?,
                None,
                "unearned next arena is inactive"
            );
        }
        super::questpc7_native_deaths::lethal_creature(
            owner,
            a,
            target,
            16000 + u64::try_from(i)?,
            &format!("oteryn:creature.{name}"),
        )
        .await?;
        assert_eq!(
            value(
                owner,
                a,
                &format!("oteryn:quest-progress/questpc7/ferumbras/seal_{i}")
            )
            .await?,
            1
        );
        walk(owner, a, pos(*portal)).await?;
        assert_eq!(value(owner, a, PORTALS).await?, i64::try_from(i + 1)?);
        let destination = if i < 7 {
            pos(ARRIVALS[i + 1])
        } else {
            pos((33388, 31410, -14))
        };
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(destination),
            "earned native passage destination"
        );
    }
    let chest = pos((33388, 31409, -14));
    assert_eq!(facts(h).await?, (0, 0, 0));
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, a, chest, 4073).await?,
        UseDisposition::Rejected
    );
    owner.premium_coordinator = premium;
    assert_eq!(
        facts(h).await?,
        (0, 0, 0),
        "fresh Premium refusal grants nothing"
    );
    let occupied:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries e JOIN game_item_container_slots s ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id WHERE s.character_id=encode($1,'hex')::uuid").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
    for ordinal in occupied..20 {
        let f = owner
            .current_quest_fence(a.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            f.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: pos((33388, 31410, -14)),
                item: format!(
                    "oteryn:item.tibia.i{}",
                    if ordinal == 19 { 3578 } else { 2854 }
                ),
                count: 1,
            },
            17000 + u64::try_from(ordinal)?,
            true,
        )
        .await?;
    }
    assert_eq!(
        use_at(owner, a, chest, 4073).await?,
        UseDisposition::Rejected,
        "full backpack refuses the entire value bundle"
    );
    assert_eq!(facts(h).await?, (0, 0, 0));
    assert_eq!(value(owner, a, TRACK).await?, 2);
    let sources:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key='oteryn:quest-callback/questpc7-ferumbras-reward'").fetch_one(&h.pool).await?;
    assert_eq!(sources, 0);
    super::questpc7_outfit_items::use_token(owner, a, 3578).await?;
    let claim_command = a.continuity.next_command_id;
    let claim_payload = super::questpc7_hoist_story::target(owner, chest, 4073)?;
    assert_eq!(
        use_at(owner, a, chest, 4073).await?,
        UseDisposition::Committed
    );
    let replay =
        super::quest_world_callbacks::wire(owner, a, claim_command, 2, claim_payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed,
        "exact native command replay"
    );
    assert_eq!(facts(h).await?, (5, 3, 1));
    assert_eq!(value(owner, a, TRACK).await?, 3);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.ferumbras_ascension_quest'").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    assert_eq!(
        use_at(owner, a, chest, 4073).await?,
        UseDisposition::Rejected
    );
    assert_eq!(facts(h).await?, (5, 3, 1));
    let observed = owner
        .appearance_observation(actor, a.game_session_id, vec![])
        .await
        .ok_or("appearance")?;
    assert_eq!(
        observed
            .catalogue
            .outfits
            .iter()
            .find(|o| o.key == "oteryn:outfit.rift_warrior")
            .ok_or("earned Rift Warrior")?
            .permitted_addons,
        3
    );
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: "oteryn:outfit.rift_warrior".into(),
            colours: [0; 4],
            addons: 3,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let command = a.continuity.next_command_id;
    let reply =
        super::quest_world_callbacks::wire(owner, a, command, 25, change.encode().map_err(debug)?)
            .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    Ok(())
}
