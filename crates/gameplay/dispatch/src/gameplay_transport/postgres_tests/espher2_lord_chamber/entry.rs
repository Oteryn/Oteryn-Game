use super::{TestResult, debug, local, source_lever};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CharacterId, CharacterLease, CommandId, CommandRef};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority, UseCommand},
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, world_callbacks,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const ENTRY: &str = "oteryn:quest-callback/elemental-spheres/lord-entry";
const PREPARE: &str = "oteryn:quest-transition/espher2-prepare-chamber";

pub(super) fn callback() -> TestResult<world_callbacks::Callback> {
    world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == ENTRY)
        .ok_or_else(|| "Lord entry callback".into())
}

pub(super) async fn exercise(
    h: &super::Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    callback: &world_callbacks::Callback,
) -> TestResult {
    let party = callback.effects.party_transfer.as_ref().ok_or("party")?;
    let guard = callback.storage.first().ok_or("entry track")?;
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: PREPARE.into(),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: guard.track.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(guard.minimum),
                }],
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let formula = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("training formula")?;
    let mut players = Vec::new();
    let mut missing_sphere = None;
    for (index, station) in party.stations.iter().enumerate() {
        let member = u8::try_from(index)?;
        let session = if Some(station.from) == party.initiator_at {
            admitted.game_session_id
        } else {
            let character = super::super::party_peer_bootstrap::create_peer(
                h,
                owner.character,
                owner.evidence,
                member,
            )
            .await?;
            super::super::activity_deadline_paging::admit_created_member(
                h,
                owner.evidence,
                member,
                character,
            )
            .await?
        };
        let actor = if session == admitted.game_session_id {
            admitted.runtime_actor.ok_or("initiator actor")?
        } else {
            let mut runtime = owner.runtime.lock().await;
            let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(actor, local(station.from))
                .map_err(debug)?;
            actor
        };
        let mut peer = admitted;
        peer.game_session_id = session;
        peer.runtime_actor = Some(actor);
        owner.admit_progression_session(&mut peer).await;
        let fence = fence(owner, session).await?;
        let build = convert_vocation(formula, &DurableBuildState::default(), &station.vocation)
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .commit_character_build(
                    owner.character,
                    &h.node,
                    fence,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(super::id(190 + member))
                            .map_err(debug)?,
                        cause: BuildCause::VocationChoice,
                        before: DurableBuildState::default(),
                        after: build.clone(),
                        pruned_stance: None,
                    },
                    formula,
                )
                .await
                .map_err(debug)?,
            BuildCommitOutcome::Committed(_)
        ));
        let fence = self::fence(owner, session).await?;
        {
            let mut revision = owner.revision_sequencer.acquire(fence.character_id).await;
            assert!(matches!(
                revision
                    .commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        QuestTransitionRequest {
                            transition_key: PREPARE.into(),
                            cause: QuestCause::Command(CommandRef::new(
                                session,
                                CommandId::new(100).map_err(debug)?,
                            )),
                        },
                        catalogue.clone(),
                    )
                    .await
                    .map_err(debug)?,
                QuestTransitionOutcome::Committed(_)
            ));
        }
        let item = station.carried_item.as_deref().ok_or("carried sphere")?;
        let character = *fence.character_id.as_bytes();
        let bag = 70 + member * 4;
        if index == 0 {
            missing_sphere = Some((character, bag, item));
        } else {
            super::super::item_look::seed_character_main_backpack(h, character, bag, &[(item, 1)])
                .await?;
        }
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            let vocation = match station.vocation.as_str() {
                "druid" => crate::spell::Vocation::Druid,
                "knight" => crate::spell::Vocation::Knight,
                "paladin" => crate::spell::Vocation::Paladin,
                "sorcerer" => crate::spell::Vocation::Sorcerer,
                _ => return Err("unexpected chamber vocation".into()),
            };
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        vocation,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("spell state")?;
            assert!(states.load_owned_build(&runtime, actor, session, &build));
        }
        owner.refresh_quest_session(session, false).await;
        players.push((actor, local(station.from), local(station.to)));
    }

    let position = callback.selector.position.ok_or("entry lever tile")?;
    let map = owner.bundle_map().ok_or("entry map")?;
    let key = source_lever(
        map,
        crate::map::overlay::TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        },
    )?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = fence(owner, admitted.game_session_id).await?;
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let actor = admitted.runtime_actor.ok_or("initiator")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 101,
        item_fence: Some(fence.item_fence()),
    };
    let refused = owner.use_object(actor, command, target.clone()).await;
    assert_eq!(refused.disposition, UseDisposition::Rejected);
    assert_eq!(
        map.facts.object_revision(key),
        0,
        "a missing sphere must leave the lever ready"
    );
    {
        let runtime = owner.runtime.lock().await;
        for (actor, origin, _) in &players {
            assert_eq!(
                runtime
                    .read_actor_position(*actor)
                    .map_err(debug)?
                    .position(),
                *origin
            );
        }
    }
    let (character, bag, item) = missing_sphere.ok_or("withheld non-initiator sphere")?;
    super::super::item_look::seed_character_main_backpack(h, character, bag, &[(item, 1)]).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("USE gate clock")?;
    let outcome = owner
        .use_object(
            actor,
            UseCommand {
                command_id: 102,
                ..command
            },
            target,
        )
        .await;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    let retained: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
         WHERE i.definition_production_key IN ('oteryn:item.tibia.i942','oteryn:item.tibia.i946',
             'oteryn:item.tibia.i947','oteryn:item.tibia.i948') AND i.quantity=1 AND i.lifecycle=1",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        retained, 4,
        "admission checks the four carried spheres without consuming them"
    );
    let runtime = owner.runtime.lock().await;
    for (actor, _, destination) in players {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            destination
        );
    }
    Ok(())
}

async fn fence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<crate::durability::character_progression::CurrentCharacterGameplayFence> {
    owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "chamber entry fence")?
        .ok_or_else(|| "chamber entry fence".into())
}
