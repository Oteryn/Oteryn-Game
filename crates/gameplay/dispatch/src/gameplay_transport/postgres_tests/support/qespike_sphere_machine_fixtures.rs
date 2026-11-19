use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, atomic::Ordering};

pub(super) const NAME: &str = "qespike-sphere-machine";
pub(super) const BELOW_THRESHOLD: &str = "qespike-sphere-machine-below-threshold";
pub(super) const LOST_ACK: &str = "qespike-sphere-machine-lost-ack";
pub(super) const LOST_ACK_SINGLE_TARGET: &str = "qespike-sphere-machine-lost-ack-single-target";
const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const COUNTER: &str = "oteryn:quest-progress/elemental_spheres/machine_gem_count";
pub(super) const LEFT: Position = Position {
    x: 33268,
    y: 31830,
    floor: -10,
};
const RIGHT: Position = Position { x: 33269, ..LEFT };
const KNIGHT: Position = Position {
    x: 33087,
    y: 32096,
    floor: -13,
};

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/qespike/sphere-machine-left-funded",
        "quest":QUEST, "trigger":"use",
        "selector":{"position":LEFT,"appearance":842},
        "reversible":false,"offerings":[],"return_on_failure":false,
        "passages":[{"position":RIGHT,"appearance":843,"replacement":847,"if_present":true}],
        "requires_world_admission":true,
        "storage":[{"track":COUNTER,"minimum":20,"maximum":100,"set":-20,"increment":true}],
        "effects":{
            "transform":846, "transform_requires_current":true,"teleport_effect":11,"teleport_effect_repetitions":2,
            "vocation_teleports":{
                "sorcerer":{"x":33183,"y":32197,"floor":-13},
                "druid":{"x":33331,"y":32076,"floor":-13},
                "paladin":{"x":33265,"y":32202,"floor":-13},
                "knight":KNIGHT
            }
        },
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_machine1.lua:2-27"]
    }))?)
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    // Model the verified B3 machine positions and appearances in a fixture
    // palette. Content owns admission of the four machine definitions.
    super::quest_callbacks_fixtures::source_world_spatial(
        (LEFT.x, LEFT.y + 1, LEFT.floor),
        &[
            (LEFT.x, LEFT.y, LEFT.floor, 0, 842),
            (RIGHT.x, RIGHT.y, RIGHT.floor, 0, 843),
            (RIGHT.x + 1, RIGHT.y, RIGHT.floor, 0, 846),
            (RIGHT.x + 2, RIGHT.y, RIGHT.floor, 0, 847),
            (KNIGHT.x, KNIGHT.y, KNIGHT.floor, 0, 231),
        ],
    )
}

fn catalogue(revision: &str, funded: &Callback, initial: i64) -> TestResult<QuestStateCatalogue> {
    let mut below_threshold = funded.clone();
    below_threshold.key =
        "oteryn:quest-callback/qespike/sphere-machine-left-below-threshold".into();
    let guard = &mut below_threshold.storage[0];
    guard.minimum = -1;
    guard.maximum = 19;
    guard.set = 0;
    guard.increment = false;
    guard.preserve = true;
    below_threshold.effects.vocation_teleports.clear();
    below_threshold.effects.teleport_effect = None;
    below_threshold.effects.teleport_effect_repetitions = None;

    let mut close = below_threshold.clone();
    close.key = "oteryn:quest-callback/qespike/sphere-machine-left-off".into();
    close.selector.appearance = Some(846);
    close.storage[0].maximum = 100;
    close.effects.transform = Some(842);
    close.passages[0].appearance = 847;
    close.passages[0].replacement = Some(843);

    let stages = [funded, &below_threshold, &close].map(|callback| {
        json!({"key":callback.key,"completes":false,"trigger":{
            "kind":"map_use","callback":callback
        }})
    });
    let recipe: String = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let authored = Bindings::parse(
        serde_json::to_string(&json!({"schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":recipe,"sources":funded.sources,
            "tracks":[{"key":COUNTER,"initial":initial,"minimum":-1,"maximum":100}],"stages":stages
        }]}))?
        .as_bytes(),
    )?;
    let base = QuestStateCatalogue::empty(revision).ok_or("sphere catalogue")?;
    Ok(authored.attach(&base)?)
}

async fn other_operator(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    funded: &Callback,
    single_target: bool,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    let mut callbacks = owner
        .quest_catalogue
        .as_ref()
        .and_then(|catalogue| catalogue.authored_world_callbacks())
        .ok_or("sphere bindings")?
        .to_vec();
    let mut right = funded.clone();
    right.selector.position = Some(RIGHT);
    right.requires_world_admission = false;
    right.storage.clear();
    right.effects.vocation_teleports.clear();
    right.effects.teleport_effect = None;
    right.effects.teleport_effect_repetitions = None;
    right.passages[0].position = LEFT;
    // This player operates the right half without a debit or destination.
    for (before, after, left_before, left_after) in [(843, 847, 842, 846), (847, 843, 846, 842)] {
        right.key = format!("oteryn:quest-callback/qespike/sphere-machine-right-{before}");
        right.selector.appearance = Some(before);
        right.effects.transform = Some(after);
        right.passages[0].appearance = left_before;
        right.passages[0].replacement = Some(left_after);
        let mut row = right.clone();
        if single_target {
            row.passages.clear();
        }
        callbacks.push(row);
    }
    owner
        .states
        .quest_callbacks
        .fixture
        .set(callbacks)
        .map_err(|_| "sphere map bindings")?;
    let session = GameSessionId::decode(&id(51)).map_err(debug)?;
    let mut runtime = owner.runtime.lock().await;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(actor, super::quest_world_callbacks_fixtures::local(RIGHT))
        .map_err(debug)?;
    Ok((actor, session))
}

async fn use_right_half(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    (actor, session): (ExactActorRef, GameSessionId),
    command_id: u64,
    key: u64,
) -> TestResult<UseDisposition> {
    let map = owner.bundle_map().ok_or("sphere map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let outcome = owner
        .use_world_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: None,
            },
            WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await;
    Ok(outcome.disposition)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    scenario: &str,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    let funded = scenario != BELOW_THRESHOLD;
    let lost_ack = matches!(scenario, LOST_ACK | LOST_ACK_SINGLE_TARGET);
    let initial = if funded { 24 } else { 19 };
    owner.quest_catalogue = Some(Arc::new(catalogue(revision, callback, initial)?));
    let other = if lost_ack {
        Some(other_operator(owner, callback, scenario == LOST_ACK_SINGLE_TARGET).await?)
    } else {
        None
    };
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actor = admitted.runtime_actor.ok_or("machine user")?;
    let (facts, build) =
        super::party_xp_fixtures::initialize_build(owner, admitted.game_session_id, "knight", 139)
            .await?;
    {
        let runtime = owner.runtime.lock().await;
        let mut spells = owner.states.spell_states.lock().await;
        let mut state = crate::spell::cast::PlayerSpellState::new(facts, 0, 0).ok_or("knight")?;
        state.load_owned_build(&build).map_err(debug)?;
        state.make_playable(owner.owner_now()).map_err(debug)?;
        assert!(spells.replace_owned_player_state_test_fixture(
            &runtime,
            actor,
            admitted.game_session_id,
            state
        ));
    }
    let map = owner.bundle_map().ok_or("machine map")?;
    let object = |position: Position, appearance: u16| -> TestResult<_> {
        use crate::map::{overlay::TilePos, view::placement_key};
        let pos = TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        let tile = map
            .overlay
            .base()
            .tile(pos.x, pos.y, pos.floor)
            .ok_or("machine cell")?;
        tile.ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .find_map(|(ordinal, (item, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                (map.facts.base_entry(pos, ordinal, *item)?.appearance_id == appearance)
                    .then(|| placement_key(pos, ordinal))
                    .flatten()
            })
            .ok_or_else(|| format!("machine appearance {appearance}").into())
    };
    let left = object(LEFT, 842)?;
    let right = object(RIGHT, 843)?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&left.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let command = admitted.continuity.next_command_id;
    if let Some(other) = other {
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, Ordering::Release);
        assert!(
            super::quest_callbacks_fixtures::wire_use_targets_retained(
                owner,
                admitted,
                &[(command, &target)]
            )
            .await
            .is_err(),
            "a committed debit with an unpublished travel has no acknowledgement"
        );
        assert!(owner.has_pending_quest_callback(admitted.game_session_id));
        assert_eq!(balance(h).await?, Some(4));
        assert_eq!(object(LEFT, 842)?, left);
        assert_eq!(object(RIGHT, 843)?, right);
        assert_eq!(
            use_right_half(owner, other, 1, right).await?,
            UseDisposition::Exhausted,
            "the uncertain debit reserves both halves for its original command"
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            origin
        );
        assert!(
            owner
                .reconcile_quest_callbacks(admitted.game_session_id)
                .await
        );
        assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    }
    let result = super::quest_callbacks_fixtures::wire_use_targets_retained(
        owner,
        admitted,
        &[(command, &target)],
    )
    .await?;
    assert_eq!(result[0].1, UseDisposition::Committed);
    assert_eq!(object(LEFT, 846)?, left);
    assert_eq!(object(RIGHT, 847)?, right);
    if !funded {
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            origin
        );
        let target = WorldObjectTarget {
            expected_revision: map.facts.object_revision(left),
            ..target
        };
        let command = admitted.continuity.next_command_id;
        let result = super::quest_callbacks_fixtures::wire_use_targets_retained(
            owner,
            admitted,
            &[(command, &target)],
        )
        .await?;
        assert_eq!(result[0].1, UseDisposition::Committed);
        assert_eq!(object(LEFT, 842)?, left);
        assert_eq!(object(RIGHT, 843)?, right);
    }
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .filter(|(_, p, effect)| {
                (p.x, p.y, p.floor, *effect) == (KNIGHT.x, KNIGHT.y, KNIGHT.floor, 11)
            })
            .count(),
        if funded { 2 } else { 0 }
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        if funded {
            super::quest_world_callbacks_fixtures::local(KNIGHT)
        } else {
            origin
        }
    );
    let balance = balance(h).await?;
    if funded {
        assert_eq!(balance, Some(4));
    } else {
        assert_eq!(balance.unwrap_or(initial), 19);
    }
    if let Some(other) = other {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(201))
            .ok_or("clock")?;
        assert_eq!(
            use_right_half(owner, other, 2, right).await?,
            UseDisposition::Committed,
            "reconciliation releases the opposite half"
        );
    }
    Ok(())
}

async fn balance(h: &Harness) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(COUNTER)
            .fetch_optional(&h.pool)
            .await?,
    )
}
