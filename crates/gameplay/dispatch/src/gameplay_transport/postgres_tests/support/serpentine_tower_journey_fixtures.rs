//! The source-world journey also qualifies the catalogued White Pearl claim.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks_fixtures::wire_use_targets_retained;
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, SessionContinuity,
};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::last_position::AdmissionPosition;
use crate::gameplay_transport::quest_log::QuestLogLines;
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state, log::loader::load_embedded_quest_log,
};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_LOG_QUERY, QuestLogQuery, QuestLogView, decode_quest_log,
    encode_quest_log_query,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection::{self, East, North, South, West};
use std::{sync::Arc, time::Duration};

const QUEST: &str = "oteryn:quest.serpentine_tower_quest";
const TRACK: &str = "oteryn:quest-progress/authored/serpentine-tower/stage";
const PEARL: &str = "oteryn:item.tibia.i3026";
const CLAIM: &str = "oteryn:reward-claim.quest.u7_3.serpentine_tower.white_pearl";
const START: TilePos = cell(33147, 32864, -7);
const CHEST: TilePos = cell(33150, 32862, -7);
const LEVER: TilePos = cell(33152, 32866, -8);

const fn cell(x: u16, y: u16, floor: i8) -> TilePos {
    TilePos { x, y, floor }
}

pub(super) fn journey() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "serpentine_tower", false, false).await?;
        let result = play(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn play(h: &Harness) -> TestResult {
    let lowered = load_embedded_quest_state("content-1").map_err(debug)?;
    let catalogue = Arc::new(lowered.catalogue().clone());
    assert!(catalogue.track(TRACK).is_some(), "embedded tower journey");
    let log = load_embedded_quest_log(&lowered)?.content;
    let quests = log.active().quests();
    let hidden = quests.iter().position(|quest| quest.key == QUEST);
    assert!(hidden.is_none_or(|index| !quests[index].shown_in_quest_log));

    super::prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
    super::tutorial_vitals_fixture::publish_native_activation(h, &activation).await?;
    let generation = controller.active().ok_or("production generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let world = super::source_world_crop::compiled_region(
        cell(33143, 32861, -9),
        cell(33152, 32871, -7),
        START,
        native,
    )?;
    let cells = world
        .movement_cells(activation.qualified_room().movement_cells())
        .map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        activation.frame_binding().digest(),
        world.map_revision_digest(),
        (
            i32::from(START.x),
            i32::from(START.y),
            -i16::from(START.floor),
        ),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let reserved = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);

    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, purchase_fixture,
    };
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(catalogue);
    owner.quest_log_content = Some(QuestLogLines::new(log).ok_or("quest log bounds")?);
    owner.premium_coordinator = Some(&premium);

    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 13, 15, 16])
            .ok_or("tower capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "gameplay fence")?
        .ok_or("session")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.item_fence = Some(fence.item_fence());
    let admission = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            admission.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_premium(id(40), admitted.game_session_id);
    owner.admit_quest_session(&admitted).await;
    deps.initialize(&owner).await;
    assert!(owner.track_restored(
        admitted.game_session_id,
        deps.actor,
        &AdmissionPosition::FirstLogin,
        fence
    ));

    progress(&owner, &admitted, h, 0).await?;
    super::annihilator_route_fixtures::step(&mut owner, &mut admitted, East).await?;
    assert_at(&owner, &admitted, cell(33151, 32864, -7)).await?;
    progress(&owner, &admitted, h, 1).await?;
    walk(&mut owner, &mut admitted, h, &[North, North], 1).await?;
    assert_at(&owner, &admitted, cell(33151, 32862, -7)).await?;
    use_object(
        &mut owner,
        &mut admitted,
        CHEST,
        1,
        UseDisposition::Rejected,
    )
    .await?;
    progress(&owner, &admitted, h, 1).await?;
    let torch_wall = cell(33151, 32866, -8);
    assert!(!hidden_placement(&owner, torch_wall, 1)?);
    use_object(
        &mut owner,
        &mut admitted,
        cell(33151, 32861, -7),
        2,
        UseDisposition::Committed,
    )
    .await?;
    assert!(hidden_placement(&owner, torch_wall, 1)?);
    progress(&owner, &admitted, h, 2).await?;

    // Return portal, library door, then two retained floors of stairs and passages.
    walk(
        &mut owner,
        &mut admitted,
        h,
        &[
            South, South, West, North, West, West, West, West, South, South, South, East, East,
            South,
        ],
        2,
    )
    .await?;
    assert_at(&owner, &admitted, cell(33145, 32867, -7)).await?;
    use_object(
        &mut owner,
        &mut admitted,
        cell(33145, 32868, -7),
        1,
        UseDisposition::Committed,
    )
    .await?;
    progress(&owner, &admitted, h, 2).await?;
    walk(
        &mut owner,
        &mut admitted,
        h,
        &[
            South, South, East, South, South, East, East, East, North, East, West, North, West,
        ],
        2,
    )
    .await?;
    assert_at(&owner, &admitted, cell(33148, 32870, -8)).await?;
    use_object(
        &mut owner,
        &mut admitted,
        cell(33148, 32869, -8),
        1,
        UseDisposition::Committed,
    )
    .await?;
    progress(&owner, &admitted, h, 2).await?;
    walk(
        &mut owner,
        &mut admitted,
        h,
        &[North, North, North, North, East, East, East],
        2,
    )
    .await?;
    assert_at(&owner, &admitted, torch_wall).await?;

    let walls = [
        cell(33148, 32867, -9),
        cell(33148, 32868, -9),
        cell(33149, 32867, -9),
        cell(33149, 32868, -9),
    ];
    for wall in walls {
        assert!(!hidden_placement(&owner, wall, 1)?);
    }
    use_object(
        &mut owner,
        &mut admitted,
        LEVER,
        1,
        UseDisposition::Committed,
    )
    .await?;
    for wall in walls {
        assert!(hidden_placement(&owner, wall, 1)?);
    }
    progress(&owner, &admitted, h, 3).await?;

    for open in [false, true] {
        use_object(&mut owner, &mut admitted, LEVER, 1, UseDisposition::Committed).await?;
        for wall in walls {
            assert_eq!(hidden_placement(&owner, wall, 1)?, open);
        }
        progress(&owner, &admitted, h, 3).await?;
    }

    walk(
        &mut owner,
        &mut admitted,
        h,
        &[West, South, West, West, West, West, South, East, East],
        3,
    )
    .await?;
    assert_at(&owner, &admitted, cell(33148, 32869, -9)).await?;
    walk(&mut owner, &mut admitted, h, &[North], 4).await?;
    assert_at(&owner, &admitted, cell(33148, 32868, -9)).await?;

    walk(
        &mut owner,
        &mut admitted,
        h,
        &[
            South, West, West, North, North, East, East, South, South, South, South, East, East,
            West, South, South, West, West, West, North, North, West, North, North, North, West,
            West, North, North, North, East, East, East, East, South, East, North,
        ],
        4,
    )
    .await?;
    assert_at(&owner, &admitted, cell(33151, 32863, -7)).await?;
    let chest = target(&owner, CHEST, 1)?;
    let command = admitted.continuity.next_command_id;
    advance_clock(&mut owner)?;
    assert_eq!(
        wire_use_targets_retained(
            &owner,
            &mut admitted,
            &[(command, &chest), (command, &chest)]
        )
        .await?,
        [
            (CommandStatus::Accepted, UseDisposition::Committed),
            (CommandStatus::DuplicateReplay, UseDisposition::Committed)
        ]
    );
    progress(&owner, &admitted, h, 5).await?;
    let command = admitted.continuity.next_command_id;
    let payload = encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?;
    let reply = super::quest_world_callbacks_fixtures::wire(
        &owner,
        &mut admitted,
        command,
        COMMAND_TYPE_QUEST_LOG_QUERY,
        payload,
    )
    .await?;
    let journal = decode_quest_log(&reply).map_err(debug)?;
    let Some(QuestLogView::List(entries)) = journal.view else {
        return Err("production quest log did not return a list".into());
    };
    assert!(hidden.is_none_or(|index| {
        entries
            .iter()
            .all(|entry| entry.quest_index as usize != index)
    }));
    progress(&owner, &admitted, h, 5).await?;

    let mut successor = reconnect(&owner, admitted, h).await?;
    deps.initialize_player(
        &owner,
        successor.runtime_actor.ok_or("successor actor")?,
        successor.game_session_id,
    )
    .await;
    assert_at(&owner, &successor, cell(33151, 32863, -7)).await?;
    use_object(
        &mut owner,
        &mut successor,
        CHEST,
        1,
        UseDisposition::Rejected,
    )
    .await?;
    progress(&owner, &successor, h, 5).await
}

fn advance_clock(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    Ok(())
}

fn placement(at: TilePos, ordinal: u8) -> TestResult<u64> {
    crate::map::view::placement_key(at, ordinal).ok_or_else(|| "production placement key".into())
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: TilePos,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("tower map")?;
    let key = placement(at, ordinal)?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

fn hidden_placement(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: TilePos,
    ordinal: u8,
) -> TestResult<bool> {
    Ok(owner
        .bundle_map()
        .ok_or("tower map")?
        .facts
        .base_hidden(placement(at, ordinal)?))
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: TilePos,
    ordinal: u8,
    expected: UseDisposition,
) -> TestResult {
    advance_clock(owner)?;
    let target = target(owner, at, ordinal)?;
    let command = admitted.continuity.next_command_id;
    let replies = wire_use_targets_retained(owner, admitted, &[(command, &target)]).await?;
    assert_eq!(
        replies.first().map(|(_, disposition)| *disposition),
        Some(expected),
        "USE at {at:?}"
    );
    Ok(())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    directions: &[StepDirection],
    stage: i64,
) -> TestResult {
    for &direction in directions {
        super::annihilator_route_fixtures::step(owner, admitted, direction).await?;
        progress(owner, admitted, h, stage).await?;
    }
    Ok(())
}

async fn assert_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: TilePos,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("tower actor")?;
    let position = super::full_world_support::position(owner, actor).await?;
    assert_eq!(
        position,
        MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor)
        }
    );
    Ok(())
}

async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    stage: i64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "current gameplay fence")?
        .ok_or("tower character")?;
    let state = h
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        state.tracks().get(TRACK).copied(),
        (stage > 0).then_some(stage)
    );
    assert_eq!(
        state.states().get(QUEST).is_some_and(|row| row.completed),
        stage == 5
    );
    let inventory: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity::bigint FROM game_item_container_entries e
         JOIN game_item_instances i USING (item_instance_id) WHERE e.character_id = encode($1,'hex')::uuid
         ORDER BY e.placement_ordinal",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claims WHERE character_id = encode($1,'hex')::uuid AND claim_production_key = $2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(CLAIM).fetch_one(&h.pool).await?;
    if stage == 5 {
        assert_eq!(inventory, [(PEARL.to_owned(), 1)]);
        assert_eq!(claims, 1);
    } else {
        assert!(
            inventory.is_empty(),
            "inventory at stage {stage}: {inventory:?}"
        );
        assert_eq!(claims, 0);
    }
    Ok(())
}

async fn reconnect(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    old: AdmittedSession,
    h: &Harness,
) -> TestResult<AdmittedSession> {
    assert_eq!(
        owner.logout_session(old).await,
        LogoutReply::Result(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    );
    let session = super::full_world_support::relogin(h).await?;
    assert_ne!(session, old.game_session_id);
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(session)
        .await
        .map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "successor fence")?
        .ok_or("successor session")?;
    let decided = owner
        .read_admission_position(fence)
        .await
        .map_err(|()| "successor position")?;
    assert!(matches!(decided, AdmissionPosition::Stored(_)));
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        owner
            .initialize_admitted_position(&mut runtime, actor, session, &decided)
            .await
            .map_err(debug)?;
        actor
    };
    assert!(owner.track_restored(session, actor, &decided, fence));
    let mut successor = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("successor controller")?,
            account_id: id(40),
        }),
        item_fence: Some(fence.item_fence()),
        continuity: SessionContinuity {
            connection_generation: current.current_connection_generation().get(),
            selected_capabilities: old.continuity.selected_capabilities,
            ..SessionContinuity::FRESH
        },
        ..old
    };
    owner.admit_progression_session(&mut successor).await;
    owner.admit_premium(id(40), session);
    owner.admit_quest_session(&successor).await;
    Ok(successor)
}
