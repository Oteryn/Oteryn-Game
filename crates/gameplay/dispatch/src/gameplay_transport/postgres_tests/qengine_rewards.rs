//! QENGINE-2: source rewards through the production NPC command dispatcher.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qengine_rewards_pirate_outfit() -> TestResult {
    scenario("qengine-rewards-pirate")
}
#[test]
fn qengine_rewards_knight_outfit() -> TestResult {
    scenario("qengine-rewards-knight")
}
#[test]
fn qengine_rewards_deepling_outfit() -> TestResult {
    scenario("qengine-rewards-deepling")
}
#[test]
fn qengine_rewards_hallowed_ground_exchange() -> TestResult {
    scenario("qengine-rewards-hallowed")
}
#[test]
fn qengine_rewards_outfox_exchange() -> TestResult {
    scenario("qengine-rewards-outfox")
}
#[test]
fn wevents_postgres_spring_merchant_reward() -> TestResult {
    scenario("qengine-rewards-spring-claim")
}
#[test]
fn wevents_postgres_spring_merchant_purchase() -> TestResult {
    scenario("qengine-rewards-spring-buy")
}

#[test]
fn qengine_rewards_backpack_exchange() -> TestResult {
    scenario("qengine-rewards-backpack")
}

#[test]
fn qengine_l1_warmaster_crest() -> TestResult {
    scenario("qengine-rewards-l1-warmaster")
}
#[test]
fn qengine_l1_wayfarer_cape() -> TestResult {
    scenario("qengine-rewards-l1-wayfarer")
}
#[test]
fn qengine_l1_yalaharian_addon_sage() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian")
}

#[test]
fn qengine_l1_warmaster_tribal() -> TestResult {
    scenario("qengine-rewards-l1-warmaster-tribal")
}
#[test]
fn qengine_l1_wayfarer_hat() -> TestResult {
    scenario("qengine-rewards-l1-wayfarer-hat")
}
#[test]
fn qengine_l1_yalaharian_base_palimuth() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-base-palimuth")
}
#[test]
fn qengine_l1_yalaharian_base_palimuth_accepts_villain() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-base-palimuth-villain")
}
#[test]
fn qengine_l1_yalaharian_base_yalahari() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-base-yalahari")
}
#[test]
fn qengine_l1_yalaharian_addon_villain() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-villain")
}

#[test]
fn qengine_l1_legacy_decaying_entitlement_remains_selectable() -> TestResult {
    scenario("qengine-rewards-l1-legacy-decaying")
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    id: u64,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown, "NPC command must have a known outcome");
    Ok(outcome.disposition)
}

fn offer_words(key: &str) -> &'static [&'static str] {
    match key.rsplit('/').next().unwrap_or(key) {
        "knight-sword" => &["forge"],
        "l1-warmaster-crest" => &["crest"],
        "l1-wayfarer-cape" => &["cape"],
        "l1-yalaharian-sage" | "l1-yalaharian-villain" => &["addon"],
        "l1-warmaster-tribal" => &["tribal crest"],
        "l1-wayfarer-hat" => &["hat"],
        "hallowed-holy-water" => &["holy water"],
        "sam-backpack" => &["old backpack"],
        "deepling-helmet" => &[
            "flou", "explain", "yes", "no", "yes", "yes", "yes", "helmet",
        ],
        _ => &[],
    }
}

async fn acknowledge_probe(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    payload: &[u8],
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    let probe =
        oteryn_protocol_oteryn::decode_liveness_probe(payload).map_err(|e| format!("{e:?}"))?;
    let frame = oteryn_protocol_oteryn::encode_liveness_ack(generation, probe, 0)
        .map_err(|e| format!("{e:?}"))?;
    client
        .write_all(
            &u32::try_from(frame.len())
                .map_err(|e| e.to_string())?
                .to_be_bytes(),
        )
        .await
        .map_err(|e| e.to_string())?;
    client.write_all(&frame).await.map_err(|e| e.to_string())
}

// A conversation spans several commands on ONE connection. Re-opening the
// one-command movement helper correctly clears conversation engagement.
async fn wire_grant(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    callback: &RewardCallback,
) -> TestResult<(
    NpcIntentDisposition,
    Vec<oteryn_protocol_oteryn::appearance::Catalogue>,
    u64,
)> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let words: Vec<_> = std::iter::once("hi")
        .chain(offer_words(&callback.key).iter().copied())
        .chain(std::iter::once(callback.keywords[0].as_str()))
        .collect();
    let last = admitted.continuity.next_command_id + u64::try_from(words.len())? - 1;
    let generation = admitted.continuity.connection_generation;
    let frames = words
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let payload =
                oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                })
                .map_err(debug)?;
            oteryn_protocol_oteryn::encode_client_command(
                generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: admitted.continuity.next_command_id + u64::try_from(i)?,
                    command_type: 7,
                    payload: &payload,
                },
            )
            .map_err(debug)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let expected_outfit = callback.outfit.clone();
    let peer = tokio::spawn(async move {
        let mut catalogues = Vec::new();
        let mut result = None;
        let mut window = tokio::time::Instant::now();
        for (index, frame) in frames.iter().enumerate() {
            // Drive the actual NPC0-RL-04 admission window, rather than
            // disabling its limiter for Navigator's ten-message conversation.
            if index != 0 && index % 4 == 0 {
                tokio::time::sleep_until(window + std::time::Duration::from_secs(1)).await;
                window = tokio::time::Instant::now();
            }
            client
                .write_all(
                    &u32::try_from(frame.len())
                        .map_err(|e| e.to_string())?
                        .to_be_bytes(),
                )
                .await
                .map_err(|e| e.to_string())?;
            client.write_all(frame).await.map_err(|e| e.to_string())?;
            loop {
                let mut prefix = [0; 4];
                client
                    .read_exact(&mut prefix)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut bytes = vec![0; u32::from_be_bytes(prefix) as usize];
                client
                    .read_exact(&mut bytes)
                    .await
                    .map_err(|e| e.to_string())?;
                let envelope = decode_wire_envelope(&bytes).map_err(|e| format!("{e:?}"))?;
                if envelope.message_type() == MessageType::LivenessProbe {
                    acknowledge_probe(&mut client, generation, envelope.payload()).await?;
                }
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(|e| format!("{e:?}"))?;
                    if delta.domain_id == 21
                        && delta.delta_type
                            == oteryn_protocol_oteryn::appearance::DELTA_TYPE_APPEARANCE_CATALOGUE
                    {
                        catalogues.push(
                            oteryn_protocol_oteryn::appearance::Catalogue::decode(delta.payload)
                                .map_err(|e| format!("{e:?}"))?,
                        );
                    }
                }
                if envelope.message_type() == MessageType::CommandResult {
                    let command = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(|e| format!("{e:?}"))?;
                    let disposition =
                        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
                            command.payload,
                        )
                        .map_err(|e| format!("{e:?}"))?;
                    if command.command_id == last {
                        result = Some(disposition);
                    } else {
                        assert_eq!(
                            disposition,
                            NpcIntentDisposition::Ok,
                            "conversation command {}",
                            command.command_id
                        );
                    }
                    if (index + 1) % 4 == 0 {
                        window = tokio::time::Instant::now();
                    }
                    break;
                }
            }
        }
        let has_outfit = |catalogues: &[oteryn_protocol_oteryn::appearance::Catalogue]| {
            expected_outfit.as_ref().is_none_or(|expected| {
                catalogues.iter().any(|c| {
                    c.outfits.iter().any(|o| {
                        o.key == expected.outfit
                            && o.permitted_addons & expected.addons == expected.addons
                    })
                })
            })
        };
        let mut shut = has_outfit(&catalogues);
        if shut {
            client.shutdown().await.map_err(|e| e.to_string())?;
        }
        // Keep the real connection alive until the normal appearance refresh
        // publishes the earned choice, then drain effects through EOF.
        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let mut prefix = [0; 4];
            match tokio::time::timeout_at(drain_deadline, client.read_exact(&mut prefix))
                .await
                .map_err(|_| "earned appearance catalogue did not arrive".to_owned())?
            {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.to_string()),
            }
            let mut bytes = vec![0; u32::from_be_bytes(prefix) as usize];
            client
                .read_exact(&mut bytes)
                .await
                .map_err(|e| e.to_string())?;
            let envelope = decode_wire_envelope(&bytes).map_err(|e| format!("{e:?}"))?;
            if !shut && envelope.message_type() == MessageType::LivenessProbe {
                acknowledge_probe(&mut client, generation, envelope.payload()).await?;
            }
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(|e| format!("{e:?}"))?;
                if delta.domain_id == 21
                    && delta.delta_type
                        == oteryn_protocol_oteryn::appearance::DELTA_TYPE_APPEARANCE_CATALOGUE
                {
                    catalogues.push(
                        oteryn_protocol_oteryn::appearance::Catalogue::decode(delta.payload)
                            .map_err(|e| format!("{e:?}"))?,
                    );
                }
            }
            if !shut && has_outfit(&catalogues) {
                client.shutdown().await.map_err(|e| e.to_string())?;
                shut = true;
            }
        }
        Ok::<_, String>((result.ok_or("no reward result")?, catalogues, last))
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    peer.await?.map_err(Into::into)
}

pub(super) async fn seed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    callback: &RewardCallback,
    ordinal: u64,
) -> TestResult {
    let canonical = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let tracks = callback
        .storage
        .iter()
        .map(|s| canonical.track(&s.track).cloned().ok_or("track"))
        .collect::<Result<Vec<_>, _>>()?;
    let setup = "oteryn:quest-transition/qengine-rewards-fixture-ready";
    let mut transitions = vec![QuestTransition {
        key: setup.into(),
        quest: callback.quest.clone(),
        effects: callback
            .storage
            .iter()
            .map(|s| QuestEffect {
                track: s.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(s.minimum),
            })
            .collect(),
        completes: false,
        experience: None,
    }];
    transitions.push(
        canonical
            .transition(&callback.key)
            .cloned()
            .ok_or("reward transition")?,
    );
    let catalogue = Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: setup.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(|e| format!("reward fixture seed: {e:?}"))?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_account_appearance_unlock_receipts)").fetch_one(&h.pool).await?)
}

async fn into_bag(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: [u8; 16],
    bag: [u8; 16],
    ordinal: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(ordinal).map_err(debug)?,
            ),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            crate::durability::item_loop_move::LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Container(bag),
            },
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "move into closed bag: {result:?}");
    Ok(())
}

async fn capacity_refusal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: EntityRef,
    command: u64,
    callback: &RewardCallback,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let bag_source = oteryn_gameplay_quests::quest::world_callbacks::Offering {
        position: Position {
            x: 33293,
            y: 31677,
            floor: -15,
        },
        item: "oteryn:item.tibia.i2854".into(),
        count: 1,
    };
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &bag_source, 300, true)
        .await?;
    let bag:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2854' LIMIT 1").fetch_one(&h.pool).await?;
    let input:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1 LIMIT 1").bind(&callback.take[0].item).fetch_one(&h.pool).await?;
    into_bag(owner, admitted, *input.as_bytes(), *bag.as_bytes(), 8000).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
        native,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: bag_source.item.clone(),
            revision_ref: "definition-r1".into(),
        },
    )
    .map_err(debug)?;
    let capacity = facts.container_capacity.ok_or("capacity")?;
    for n in 1..capacity {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                item: "oteryn:item.tibia.i3031".into(),
                count: 100,
                ..bag_source.clone()
            },
            300 + u64::from(n),
            true,
        )
        .await?;
    }
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        occupied,
        i64::from(capacity),
        "the refusal fixture must fill every main backpack slot"
    );
    let before = counts(h).await?;
    let quantity: i64 =
        sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=$1")
            .bind(input)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        talk(owner, admitted, npc, command, &callback.keywords[0]).await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        before,
        "capacity refusal rolls back inputs, output and source"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT quantity FROM game_item_instances WHERE item_instance_id=$1"
        )
        .bind(input)
        .fetch_one(&h.pool)
        .await?,
        quantity
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=$1"
        )
        .bind(input)
        .fetch_one(&h.pool)
        .await?,
        1
    );
    let coin:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3031' ORDER BY i.item_instance_id LIMIT 1").fetch_one(&h.pool).await?;
    into_bag(owner, admitted, *coin.as_bytes(), *bag.as_bytes(), 8001).await?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    if name == "qengine-rewards-l1-legacy-decaying" {
        return legacy_decaying(owner, admitted, h).await;
    }
    let key = match name {
        "qengine-rewards-l1-warmaster-tribal" => "l1-warmaster-tribal",
        "qengine-rewards-l1-wayfarer-hat" => "l1-wayfarer-hat",
        "qengine-rewards-l1-yalaharian-base-palimuth"
        | "qengine-rewards-l1-yalaharian-base-palimuth-villain" => "l1-yalaharian-base-palimuth",
        "qengine-rewards-l1-yalaharian-base-yalahari" => "l1-yalaharian-base-yalahari",
        "qengine-rewards-l1-yalaharian-villain" => "l1-yalaharian-villain",
        "qengine-rewards-l1-warmaster" => "l1-warmaster-crest",
        "qengine-rewards-l1-wayfarer" => "l1-wayfarer-cape",
        "qengine-rewards-l1-yalaharian" => "l1-yalaharian-sage",
        "qengine-rewards-pirate" => "pirate-sabre",
        "qengine-rewards-knight" => "knight-sword",
        "qengine-rewards-deepling" => "deepling-helmet",
        "qengine-rewards-hallowed" => "hallowed-holy-water",
        "qengine-rewards-outfox" => "outfox-helmet",
        "qengine-rewards-backpack" => "sam-backpack",
        "qengine-rewards-spring-claim" => "spring-nest-claim",
        "qengine-rewards-spring-buy" => "spring-nest-buy",
        _ => return Err("unknown reward scenario".into()),
    };
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(key))
        .ok_or("callback")?;
    let mut setup = callback.clone();
    if name == "qengine-rewards-l1-yalaharian-base-palimuth-villain" {
        // Palimuth's source completion at questline 52 does not test the side.
        // Keep the actual callback unchanged while arranging the opposite side.
        setup
            .storage
            .retain(|state| !state.track.ends_with("/side_decision"));
        for state in &mut setup.storage {
            if state.track.ends_with("/door_to_reward") || state.track.ends_with("/mission10") {
                state.minimum = -1;
            }
        }
        setup
            .storage
            .push(oteryn_gameplay_quests::quest::world_callbacks::Storage {
                track: "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/side_decision"
                    .into(),
                minimum: 2,
                maximum: 2,
                set: 2,
                preserve: true,
            });
    }
    if key.starts_with("spring-nest-") {
        spring_fixture(owner, admitted, h, true).await?;
    }
    seed(owner, admitted, &setup, 9000).await?;
    let [npc] = super::classic_quests::install(owner, admitted, [callback.npc.as_str()]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert!(admitted.continuity.selected_capabilities.contains(3));
    assert!(admitted.continuity.selected_capabilities.contains(6));
    assert!(admitted.continuity.selected_capabilities.contains(25));
    let first = admitted.continuity.next_command_id;
    assert_eq!(
        talk(owner, admitted, npc, first, "hi").await?,
        NpcIntentDisposition::Ok
    );
    let mut command = admitted.continuity.next_command_id;
    let offers = offer_words(key);
    for offer in offers {
        assert_eq!(
            talk(owner, admitted, npc, command, offer).await?,
            NpcIntentDisposition::Ok
        );
        command += 1;
    }
    let before = counts(h).await?;
    // Missing inputs must refuse without even a consumption receipt or progress change.
    if !callback.take.is_empty() || !callback.present.is_empty() {
        assert_eq!(
            talk(owner, admitted, npc, command, &callback.keywords[0]).await?,
            NpcIntentDisposition::Rejected
        );
        command += 1;
        assert_eq!(counts(h).await?, before);
        for s in &callback.storage {
            assert_eq!(
                super::classic_quests::progress(owner, admitted, authority, &s.track).await?,
                s.minimum
            );
        }
    }
    let at = Position {
        x: 33293,
        y: 31677,
        floor: -15,
    };
    for (i, input) in callback.take.iter().chain(&callback.present).enumerate() {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        // Two separate stacks prove aggregation rather than a single-stack >= check.
        let portions = if input.count > 1 {
            vec![input.count / 2, input.count - input.count / 2 + 7]
        } else {
            vec![input.count]
        };
        for (j, count) in portions.into_iter().enumerate() {
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position: at,
                    item: input.item.clone(),
                    count,
                },
                200 + u64::try_from(i * 2 + j)?,
                true,
            )
            .await?;
        }
        if i + 1 < callback.take.len() + callback.present.len() {
            assert_eq!(
                talk(owner, admitted, npc, command, &callback.keywords[0]).await?,
                NpcIntentDisposition::Rejected
            );
            command += 1;
            assert_eq!(
                counts(h).await?,
                before,
                "one present input cannot be partially consumed"
            );
        }
    }
    if key == "outfox-helmet" || key == "spring-nest-buy" {
        capacity_refusal(owner, admitted, h, npc, command, &callback).await?;
        command += 1;
    }
    if key == "sam-backpack" {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: at,
                item: "oteryn:item.tibia.i3031".into(),
                count: 1,
            },
            245,
            true,
        )
        .await?;
        let bag: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1 LIMIT 1")
            .bind(&callback.take[0].item).fetch_one(&h.pool).await?;
        let coin: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3031' ORDER BY item_instance_id LIMIT 1")
            .fetch_one(&h.pool).await?;
        into_bag(owner, admitted, *coin.as_bytes(), *bag.as_bytes(), 8100).await?;
        let before = counts(h).await?;
        assert_eq!(
            talk(owner, admitted, npc, command, &callback.keywords[0]).await?,
            NpcIntentDisposition::Rejected
        );
        assert_eq!(
            counts(h).await?,
            before,
            "a filled hand-in container and its contents stay intact"
        );
        let main: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        into_bag(owner, admitted, *coin.as_bytes(), *main.as_bytes(), 8101).await?;
    }
    let mut held_evidence = Vec::new();
    for evidence in &callback.present {
        let item: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1 ORDER BY item_instance_id LIMIT 1")
            .bind(&evidence.item).fetch_one(&h.pool).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: at,
                item: "oteryn:item.tibia.i2854".into(),
                count: 1,
            },
            300,
            true,
        )
        .await?;
        let bag: sqlx::types::Uuid = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2854' ORDER BY i.item_instance_id LIMIT 1")
            .fetch_one(&h.pool).await?;
        into_bag(owner, admitted, *item.as_bytes(), *bag.as_bytes(), 8200).await?;
        held_evidence.push(item);
    }
    let original = counts(h).await?;
    let (disposition, catalogues, committed_command) =
        wire_grant(owner, admitted, npc, &callback).await?;
    command = committed_command;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    if let Some(outfit) = &callback.outfit {
        assert!(
            catalogues.iter().any(|catalogue| catalogue
                .outfits
                .iter()
                .any(|option| option.key == outfit.outfit
                    && option.permitted_addons & outfit.addons == outfit.addons)),
            "the real connection must publish the earned outfit/addon in its client catalogue delta"
        );
    }
    let after = counts(h).await?;
    assert_eq!(after.0, original.0 + 1);
    assert_eq!(after.2, original.2 + i64::from(callback.give.is_some()));
    if callback.outfit.is_some() {
        assert!(after.3 > original.3);
    }
    for s in &callback.storage {
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, &s.track).await?,
            if callback.repeatable {
                s.minimum + s.set
            } else {
                s.set
            }
        );
    }
    for input in &callback.take {
        let consumed:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1 AND before_item->>'definition_production_key'=$2").bind(&callback.key).bind(&input.item).fetch_one(&h.pool).await?;
        assert_eq!(consumed, i64::from(input.count));
        if input.count > 1 {
            let remaining: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
                .bind(&input.item).fetch_one(&h.pool).await?;
            assert_eq!(remaining, 7, "only the required stack count is removed");
        }
    }
    for item in held_evidence {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=$1"
            )
            .bind(item)
            .fetch_one(&h.pool)
            .await?,
            1,
            "the exact original crest stays in its closed nested custody"
        );
    }
    for evidence in &callback.present {
        let remaining: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(&evidence.item).fetch_one(&h.pool).await?;
        assert_eq!(remaining, i64::from(evidence.count));
        let consumed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1",
        )
        .bind(&callback.key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            consumed, 0,
            "possession keeps the original crest and custody receipt"
        );
    }
    if let Some(reward) = &callback.give {
        let minted:(String,i64)=sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE m.claim_production_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(minted, (reward.item.clone(), i64::from(reward.count)));
    }
    if let Some(achievement) = &callback.achievement {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1"
            )
            .bind(achievement)
            .fetch_one(&h.pool)
            .await?,
            1
        );
    }
    // Literal transport replay cannot take items or repeat either value grant.
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
            &super::quest_world_callbacks::wire(
                owner,
                admitted,
                command,
                7,
                oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: callback.keywords[0].clone()
                })
                .map_err(debug)?
            )
            .await?
        )
        .map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(counts(h).await?, after);
    if name == "qengine-rewards-l1-yalaharian-base-palimuth-villain" {
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/side_decision",
            )
            .await?,
            2,
            "Palimuth's base reward must preserve the chosen side"
        );
    }
    if let Some(outfit) = &callback.outfit {
        let observation = owner
            .appearance_observation(
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
                vec![],
            )
            .await
            .ok_or("appearance observation")?;
        let option = observation
            .catalogue
            .outfits
            .iter()
            .find(|o| o.key == outfit.outfit)
            .ok_or("earned outfit absent from client catalogue")?;
        assert_eq!(option.permitted_addons & outfit.addons, outfit.addons);
        if key.starts_with("l1-yalaharian-") {
            assert_eq!(
                option.permitted_addons, outfit.addons,
                "Palimuth and Azerus sides must never grant each other's addon"
            );
        }
        let bytes = observation.catalogue.encode().map_err(debug)?;
        assert_eq!(
            oteryn_protocol_oteryn::appearance::Catalogue::decode(&bytes).map_err(debug)?,
            observation.catalogue
        );
        let expected_look = match key {
            "pirate-sabre" => 151,
            "knight-sword" => 139,
            "l1-warmaster-crest" | "l1-warmaster-tribal" => 336,
            "l1-wayfarer-cape" | "l1-wayfarer-hat" => 366,
            "l1-yalaharian-sage"
            | "l1-yalaharian-villain"
            | "l1-yalaharian-base-palimuth"
            | "l1-yalaharian-base-yalahari" => 324,
            _ => 464,
        };
        assert_eq!(
            option.look_type, expected_look,
            "the durable creation sex selects its donor look"
        );
        // A client can select the earned choice through its normal command, and
        // the appearance owner then publishes that exact look and addon mask.
        let change = oteryn_protocol_oteryn::appearance::Change {
            context: observation.catalogue.context,
            expected_root_revision: observation.catalogue.root_revision,
            prior: observation.catalogue.selected.clone(),
            selection: oteryn_protocol_oteryn::appearance::Selection {
                outfit_key: outfit.outfit.clone(),
                colours: [0; 4],
                addons: outfit.addons,
                mount_key: None,
            },
            content_revision: owner
                .appearance_context()
                .ok_or("cosmetics")?
                .1
                .content_revision()
                .into(),
        };
        let next = admitted.continuity.next_command_id;
        let result = super::quest_world_callbacks::wire(
            owner,
            admitted,
            next,
            25,
            change.encode().map_err(debug)?,
        )
        .await?;
        let change_result =
            oteryn_protocol_oteryn::appearance::ChangeResult::decode(&result).map_err(debug)?;
        assert!(
            matches!(
                change_result,
                oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
            ),
            "earned outfit selection: {change_result:?}"
        );
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let current = owner
            .appearance_observation(
                actor,
                admitted.game_session_id,
                vec![EntityRef {
                    identity: actor.placement_identity(),
                    generation: actor.actor_local_generation(),
                }],
            )
            .await
            .ok_or("selected look")?;
        assert!(current.bindings.iter().any(|b|matches!(b.look,oteryn_protocol_oteryn::appearance::Look::Outfit {look_type,addons,..} if look_type==expected_look && addons==outfit.addons)));
    }
    // A new command after completion cannot create a second source receipt.
    let next = admitted.continuity.next_command_id;
    let _ = talk(owner, admitted, npc, next, &callback.keywords[0]).await?;
    assert_eq!(counts(h).await?, after);
    if key == "pirate-sabre" {
        // Arrange a prior account entitlement, as when a sibling Character
        // already finished this quest. Progress can still advance without
        // replacing either immutable appearance receipt.
        seed(owner, admitted, &callback, 9001).await?;
        let (disposition, _, _) = wire_grant(owner, admitted, npc, &callback).await?;
        assert_eq!(disposition, NpcIntentDisposition::Ok);
        let retained = counts(h).await?;
        assert_eq!(retained.0, after.0 + 1);
        assert_eq!(retained.3, after.3);
        for s in &callback.storage {
            assert_eq!(
                super::classic_quests::progress(owner, admitted, authority, &s.track).await?,
                s.set
            );
        }
    }
    Ok(())
}

// Arrange an entitlement written by the accepted pre-callback owner, then use
// the real client appearance command. No new reward grant is used in this case.
async fn legacy_decaying(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::appearance::{Change, ChangeResult, Selection};
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let mut receipt_id = [0x91; 16];
    receipt_id[6] = 0x81;
    let receipt_id = sqlx::types::Uuid::from_bytes(receipt_id);
    let key = "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base";
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,'owner-pr2784-decaying-defender-base-v1',encode($4,'hex')::uuid,'quest',$5)")
        .bind(receipt_id).bind(account).bind(key).bind(fence.character_id.as_bytes().as_slice())
        .bind(b"qengine-l1-existing-entitlement".as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)")
        .bind(account).bind(key).bind(receipt_id).execute(&h.pool).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
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
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("appearance capability")?;
    let observed = owner
        .appearance_observation(actor, admitted.game_session_id, vec![])
        .await
        .ok_or("legacy catalogue")?;
    let outfit = "oteryn:outfit.decaying_defender";
    let option = observed
        .catalogue
        .outfits
        .iter()
        .find(|entry| entry.key == outfit)
        .ok_or("legacy Decaying Defender entitlement is missing from the native catalogue")?;
    assert_eq!((option.look_type, option.permitted_addons), (1663, 0));
    let change = Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: Selection {
            outfit_key: outfit.into(),
            colours: [0; 4],
            addons: 0,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let next = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(
        owner,
        admitted,
        next,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        ChangeResult::decode(&result).map_err(debug)?,
        ChangeResult::Committed(_)
    ));
    let retained: sqlx::types::Uuid = sqlx::query_scalar("SELECT grant_occurrence_id FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=$2")
        .bind(account).bind(key).fetch_one(&h.pool).await?;
    assert_eq!(
        retained, receipt_id,
        "selection preserves the original immutable grant receipt"
    );
    Ok(())
}

pub(super) async fn spring_fixture(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    hatched: bool,
) -> TestResult {
    use crate::durability::world_event::changes::timed::{self, EventKind, Input};
    let at = 1_934_078_400_000_i64;
    let world = owner.world_id;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'spring-reward-fixture',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000),'2031-04')")
        .bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).bind(at).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    owner.reset_epoch = 1;
    // The World transaction locks admission relations. Read the Character fence
    // first instead of recursively entering the Root while those locks are held.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "spring fixture fence read failed")?
        .ok_or("spring fixture session terminal")?;
    let mut tx = h.pool.begin().await?;
    let mut state = timed::poll(&mut tx, world, 1, EventKind::SpringIntoLife, at)
        .await?
        .ok_or("spring reward season")?;
    if hatched {
        // Native placement ACK is independently proved by the owner/actor cases.
        // This fixture starts at that committed precondition to exercise real MINT.
        state.note_spawn(1, true).map_err(debug)?;
        state
            .advance(EventKind::SpringIntoLife, at + 3_600_000, Input::Tick, 0)
            .map_err(debug)?;
        sqlx::query("UPDATE game_world_timed_events SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND event_key='spring_into_life'")
        .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(&state)?).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_timed_participants(world_id,event_key,cycle_id,character_id,credited,reward_ready) SELECT world_id,event_key,cycle_id,encode($2,'hex')::uuid,1,true FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key='spring_into_life'")
        .bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner
        .root
        .set_world_event_clock_for_test(Some(at + if hatched { 3_600_000 } else { 0 }))
        .map_err(debug)?;
    Ok(())
}
