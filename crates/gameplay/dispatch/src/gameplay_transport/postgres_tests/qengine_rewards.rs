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

#[path = "qengine_rewards/m39_any_topic.rs"]
mod m39_any_topic;

pub(super) fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            if name.starts_with("qengine-rewards-qworld-phase-") {
                super::qworld_phase_steps::SOURCE
            } else if name.starts_with("qengine-rewards-questpc4-paradox-upper-") {
                super::l6_paradox_upper::start(name)
            } else if name.starts_with("qengine-rewards-questpc4-paradox-story-") {
                super::l6_paradox_story::start()
            } else if name.starts_with("qengine-rewards-questpc4-native-tower-") {
                super::l6_paradox_native::trigger(name)
            } else if name == "qengine-rewards-m56-wax" {
                Position { x: 31942, y: 32936, floor: -10 }
            } else if matches!(name, "qengine-rewards-m56-wax-corner" | "qengine-rewards-m56-wax-edge-outside") {
                super::m56_cursed_crystal_wax::trigger(name)
            } else if name.starts_with("qengine-rewards-questpc4-admission-") {
                super::l6_world_admission::trigger(name)
            } else {
                Position {
                    x: 33294,
                    y: 31677,
                    floor: -15,
                }
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

pub(super) async fn talk(
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
    if key.contains("/l2-cael-tome-") { return &["tome"]; }
    if key.ends_with("/l2-zalamon-tome") { return &["mission"]; }
    if key.ends_with("/l2-bone-start") {
        return &["mission"];
    }

    match key.rsplit('/').next().unwrap_or(key) {
        "m09-rookie-mission11" => &["yes"],
        "m09-tower" => &["mission"],
        "knight-sword" => &["forge"],
        "qnpcfx-koshei-start" => &["amulet"],
        "l1-warmaster-crest" => &["crest"],
        "l1-wayfarer-cape" => &["cape"],
        "l1-yalaharian-sage" | "l1-yalaharian-villain" => &["addon"],
        "l1-warmaster-tribal" => &["tribal crest"],
        "l1-wayfarer-hat" => &["hat"],
        "l2-wizard-start" => &["outfit", "yes"],
        "l2-wizard-shield" => &["shield"],
        "l2-wizard-mail" => &["mail"],
        "l2-wizard-legs" => &["legs"],
        "l2-wizard-skull" => &["ring"],
        "l2-wizard-flowers-start" => &["addon", "yes"],
        "l2-wizard-flowers" => &["orchid"],
        "l2-battle-mage-book" | "l2-battle-mage-epaulettes" => &["addon", "yes", "yes"],
        "l2-elementalist-start" | "l2-elementalist-base" => &["slime", "yes"],
        "l2-festive-mask-start" | "l2-festive-feather-start" => &["addons"],
        "l3-demon-base" | "l3-demon-shield" | "l3-demon-helmet" => &["outfit", "yes"],
        "l4-nobleman-first-start" => &["addon", "yes", "dress", "yes", "yes"],
        "l4-nobleman-second-start" => &["addon", "yes", "hat", "yes", "yes"],
        "l4-nobleman-first" | "l4-nobleman-second" => &["addon"],
        "l4-royal-costume-base" => &["outfit", "yes", "armor"],
        "l4-royal-costume-shield" => &["outfit", "yes", "shield"],
        "l4-royal-costume-crown" => &["outfit", "yes", "crown"],
        "l6-citizen-rook-backpack-start" => &["addon", "backpack"],
        "l6-citizen-rook-backpack-materials" => &["backpack"],
        "l6-citizen-rook-hat-start" | "l6-citizen-rook-hat" => &["hat"],
        "l7-knight-helmet-start" => &["task", "yes"],
        "l7-knight-fangs" => &["behemoth fang"],
        "l7-knight-helmet" => &["ramsay the reckless helmet"],
        "l7-knight-sweat" => &["sweat"],
        "l7-knight-steel" => &["royal steel"],
        "l7-druid-body" => &["paws"],
        "l7-druid-hat-start" => &["addon", "faolan", "yes"],
        "l7-druid-griffinclaw" => &["griffinclaw"],
        "l7-druid-water-start" | "l7-druid-dust-start" | "l7-druid-chain-start" => &["task"],
        "l7-druid-water" => &["waterskin"],
        "l7-druid-dust" => &["dust"],
        "l7-druid-chain" => &["chain"],
        "l7-druid-hat" => &["addon"],
        "l7-assassin-base-start" => &["sober", "potion", "yes"],
        "l7-assassin-eyes" => &["30 bonelord eyes"],
        "l7-assassin-scales" => &["10 red dragon scales"],
        "l7-assassin-lizard" => &["30 lizard scales"],
        "l7-assassin-fins" => &["20 fish fins"],
        "l7-assassin-vampire" => &["20 vampire dust"],
        "l7-assassin-demon" => &["10 demon dust"],
        "l7-assassin-sweat" => &["warrior's sweat"],
        "l7-assassin-first-start" => &["addon", "yes"],
        "l7-assassin-blue" => &["50 blue cloth"],
        "l7-assassin-green" => &["50 green cloth"],
        "l7-assassin-red" => &["50 red cloth"],
        "l7-assassin-brown" => &["50 brown cloth"],
        "l7-assassin-yellow" => &["50 yellow cloth"],
        "l7-assassin-white" => &["50 white cloth"],
        "l7-assassin-yarn" => &["10 spools of yarn"],
        "l11-makeshift-base" => &["salvage"],
        "l11-dream-warrior-first" | "l11-dream-warrior-second" => &["addon", "yes", "yes"],
        "l10-brotherhood-join" | "l10-nightmare-join" => &["join", "yes"],
        "l10-brotherhood-officer"
        | "l10-nightmare-officer"
        | "l10-brotherhood-leader"
        | "l10-nightmare-leader" => &["advancement"],
        "l10-golden-base" => &["outfit", "yes", "armor"],
        "l10-golden-boots" => &["outfit", "yes", "boots"],
        "l10-golden-helmet" => &["outfit", "yes", "helmet"],
        "l9-soil-guardian-base" => &["basic soil guardian outfit"],
        "l9-soil-guardian-first" => &["addition to the soil guardian outfit"],
        "l9-crystal-warlord-base" => &["basic crystal warlord outfit"],
        "l9-crystal-warlord-first" => &["addition to the crystal warlord armor outfit"],
        "l9-gnomish-supply" => &["supply"],
        "l9-afflicted-pouch" => &["medicine"],
        "l8-hunter-start" => &["addon", "task", "yes"],
        "l8-hunter-crossbow" => &["crossbow"],
        "l8-hunter-leather" => &["leather"],
        "l8-hunter-wings" => &["chicken wing"],
        "l8-hunter-hat" => &["steel"],
        "l8-hunter-gloves" => &["sniper gloves"],
        "l8-mage-male-wand-start" => &["angelina", "yes"],
        "l8-mage-male-wands" => &["wand"],
        "l8-mage-male-sulphur" => &["sulphur"],
        "l8-mage-male-soul" => &["soul stone"],
        "l8-mage-male-ankhs" => &["ankh"],
        "l8-mage-male-hat" => &["proof"],
        "l8-mage-female-belt-start" => &["vial", "yes"],
        "l8-mage-female-belt" => &["prize"],
        "l8-mage-female-hat-start" => &["outfit", "tiara", "yes"],
        "l8-dream-warden-mask" => &["dream warden mask"],
        "l8-dream-warden-claw" => &["dream warden claw"],
        "l8-mage-female-wings" => &["bat wings"],
        "l8-mage-female-cloth" => &["red cloth"],
        "l8-mage-female-fur" => &["ape fur"],
        "l8-mage-female-orchids" => &["holy orchids"],
        "l8-mage-female-yarn" => &["yarn"],
        "l8-mage-female-lizard" => &["lizard scales"],
        "l8-mage-female-scales" => &["red dragon scales"],
        "l8-mage-female-sulphur" => &["sulphur"],
        "l8-mage-female-vampire" => &["vampire dust"],
        "l7-assassin-second-start" => &["addon", "yes"],
        "l6-beggar-start" => &["outfit", "yes"],
        "l6-beggar-cloth" => &["20 brown pieces of cloth"],
        "l6-beggar-leather" => &["50 minotaur leathers"],
        "l6-beggar-wings" => &["10 bat wings"],
        "l6-beggar-blossoms" => &["30 heaven blossoms"],
        "l6-beggar-base" => &["outfit"],
        "l6-beggar-first-start" => &["outfit", "yes"],
        "l6-beggar-first-materials" => &["100 ape fur"],
        "l6-beggar-staff-start" => &["addon", "staff", "yes"],
        "l6-beggar-staff" => &["staff"],
        "l6-druid-rook-body" => &["paws"],
        "l5-norseman-first-shards" | "l5-norseman-second-shards" => &["shard"],
        "l5-oriental-female-first-start" => &["outfit", "yes"],
        "l5-oriental-female-comb" => &["comb"],
        "l5-oriental-female-second-start" => &["task", "yes"],
        "l5-oriental-female-ape-fur" => &["ape fur"],
        "l5-oriental-female-fish-fins" => &["fish fins"],
        "l5-oriental-female-wings" => &["enchanted chicken wings"],
        "l5-oriental-female-blue-cloth" => &["blue cloth"],
        "l4-pirate-sabre-start" => &["task", "yes"],
        "l4-pirate-eye-patches" => &["eye patches"],
        "l4-pirate-peg-legs" => &["peg legs"],
        "l4-pirate-hooks" => &["hooks"],
        "l4-pirate-hat-start" => &["addon", "task", "yes"],
        "l4-pirate-hat" => &["task"],
        "hallowed-holy-water" => &["holy water"],
        "sam-backpack" => &["old backpack"],
        "deepling-helmet" => &[
            "flou", "explain", "yes", "no", "yes", "yes", "yes", "helmet",
        ],
        "qnpcfx-oldrak-begin" => &["demon oak"],
        "qnpcfx-oldrak-hallowed-axe" => &["axe"],
        "qnpcfx-inquisition-join" => &["join"],
        "qnpcfx-inquisition-report" => &["mission"],
        "qnpcfx-inquisition-kulag_guard" | "qnpcfx-inquisition-grof_guard" => &["trouble"],
        "qnpcfx-inquisition-tim_guard" | "qnpcfx-inquisition-walter_guard" => &["trouble", "authorities", "avoided"],
        "qnpcfx-inquisition-miles_guard" => &["trouble", "foresight of the authorities", "also for the gods"],
        _ => &[],
    }
}

pub(super) async fn acknowledge_probe(
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
) -> TestResult<(NpcIntentDisposition, Vec<oteryn_protocol_oteryn::appearance::Catalogue>, u64)> {
    wire_reward(owner, admitted, npc, callback, true).await
}

pub(super) async fn wire_reward(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    callback: &RewardCallback,
    expect_outfit: bool,
) -> TestResult<(NpcIntentDisposition, Vec<oteryn_protocol_oteryn::appearance::Catalogue>, u64)> {
    let words: Vec<_> = std::iter::once("hi")
        .chain(offer_words(&callback.key).iter().copied())
        .chain(std::iter::once(callback.keywords[0].as_str()))
        .collect();
    wire_conversation(owner, admitted, npc, callback, expect_outfit, words).await
}

pub(super) async fn wire_conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    callback: &RewardCallback,
    expect_outfit: bool,
    words: Vec<&str>,
) -> TestResult<(NpcIntentDisposition, Vec<oteryn_protocol_oteryn::appearance::Catalogue>, u64)> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
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
    let expected_outfits = if expect_outfit && !callback.outfits.is_empty() {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let state = owner
            .root
            .read_current_character_appearance_state(owner.character, owner.holder, fence, vec![])
            .await
            .map_err(debug)?;
        callback
            .outfits
            .iter()
            .filter(|target| {
                target.sex.is_none_or(|sex| {
                    matches!(
                        (sex, state.sex),
                        (
                            oteryn_gameplay_quests::quest::reward_callbacks::CreationSex::Female,
                            crate::domain::appearance::Sex::Female
                        ) | (
                            oteryn_gameplay_quests::quest::reward_callbacks::CreationSex::Male,
                            crate::domain::appearance::Sex::Male
                        )
                    )
                })
            })
            .cloned()
            .collect::<Vec<_>>()
    } else if expect_outfit {
        callback.outfit.iter().cloned().collect()
    } else {
        vec![]
    };
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
            expected_outfits.iter().all(|expected| {
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
        world_change: None,
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

pub(super) async fn into_bag(
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
    if name == "qengine-rewards-m09-rookie" {
        return super::m09_rookie_reward::exercise(owner, admitted, h, authority).await;
    }
    if name == "qengine-rewards-m37-peter" {
        return super::questpc8_npc_teleport_story::exercise(owner, admitted, h, authority).await;
    }
    if name.starts_with("qengine-rewards-m56-wax") {
        return super::m56_cursed_crystal_wax::exercise(owner, admitted, h, name.ends_with("-outside")).await;
    }
    if name.starts_with("qengine-rewards-m09-tower-") {
        return super::m09_multi_rewards::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-qworld-phase-") {
        return super::qworld_phase_steps::exercise(owner, admitted, h, authority, name).await;
    }
    if name == "qengine-rewards-questpc5-personal-map-query" {
        return super::questpc5_personal_map_marks::exercise(owner, admitted, h).await;
    }
    if name == "qengine-rewards-m23-last-creep" {
        return super::m23_last_creep::exercise(owner, admitted, h).await;
    }
    if name == "qengine-rewards-srvgap-disguise" {
        return super::srvgap_quest_disguise::exercise(owner, admitted, h).await;
    }
    if name.starts_with("qengine-rewards-qworld-phase-") {
        return super::qworld_phase_steps::exercise(owner, admitted, h, authority, name).await;
    }
    if name == "qengine-rewards-m39-any-topic" {
        return m39_any_topic::exercise(owner, admitted, h).await;
    }
    if name.starts_with("qengine-rewards-m39-tomes") {
        return super::m39_tomes_journey::exercise(owner, admitted, h, name).await;
    }
    if name.starts_with("qengine-rewards-m39-bone") {
        return super::m39_bone_journey::exercise(owner, admitted, h, authority, name).await;
    }
    if name == "qengine-rewards-m39-any-topic" {
        return m39_any_topic::exercise(owner, admitted, h).await;
    }
    if name == "qengine-rewards-authored-imported-talk-group" {
        return super::qengine_m01a_imported_talk::exercise_group(owner, admitted, h).await;
    }
    if name == "qengine-rewards-authored-imported-talk" {
        return super::qengine_m01a_imported_talk::exercise(owner, admitted, h).await;
    }
    if name == "qengine-rewards-m01c-collection" {
        return super::m01c_collection::exercise(owner, admitted, h).await;
    }
    if name.starts_with("qengine-rewards-qnpcfx-m10-") {
        return super::qnpcfx_m10::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-qnpcfx-koshei") {
        return super::qnpcfx_koshei::exercise(owner, admitted, h, authority, name.ends_with("-bank")).await;
    }
    if name.starts_with("qengine-rewards-questpc4-paradox-upper-") {
        return super::l6_paradox_upper::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-questpc4-paradox-story-") {
        return super::l6_paradox_story::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-questpc4-native-tower-") {
        return super::l6_paradox_native::exercise(owner, admitted, h, name).await;
    }
    if name.starts_with("qengine-rewards-questpc4-admission-") {
        return super::l6_world_admission::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-questpc4-puzzle-") {
        return super::l6_world_puzzles::exercise(owner, admitted, h, name).await;
    }
    if name.starts_with("qengine-rewards-questpc4-paradox") {
        return super::l6_paradox::exercise(owner, admitted, h, authority, name).await;
    }
    if name == "qengine-rewards-questpc4-spirit" {
        return super::l6_spirit::exercise(owner, admitted, h, authority).await;
    }
    if name.starts_with("qengine-rewards-l12-") {
        return super::qengine_kill_stories::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-l11-") {
        return super::qengine_level_stories::exercise(owner, admitted, h, authority, name).await;
    }
    if name.starts_with("qengine-rewards-l10-documents-") {
        return super::qengine_document_stories::exercise(owner, admitted, h, authority, name)
            .await;
    }
    if name.starts_with("qengine-rewards-l9-") {
        return super::qengine_voucher_stories::exercise(owner, admitted, h, name).await;
    }
    if name.starts_with("qengine-rewards-l10-")
        || name.starts_with("qengine-rewards-l8-")
        || name.starts_with("qengine-rewards-l7-")
        || name.starts_with("qengine-rewards-l2-")
        || name.starts_with("qengine-rewards-l3-")
        || name.starts_with("qengine-rewards-l4-")
        || name.starts_with("qengine-rewards-l5-")
        || name.starts_with("qengine-rewards-l6-")
    {
        return super::qengine_outfit_stories::exercise(owner, admitted, h, authority, name).await;
    }
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
                at_least: false,
                increment: false,

                track: "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/side_decision"
                    .into(),
                minimum: 2,
                maximum: 2,
                set: 2,
                preserve: true,
            });
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
    if key == "outfox-helmet" {
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
            s.set
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
    assert!(
        observed
            .catalogue
            .outfits
            .iter()
            .any(|entry| entry.key == "oteryn:outfit.decaying_defender")
    );
    // A pre-merge NX-5 selection must remain valid under its original key too.
    let outfit = "oteryn:content.outfit.decaying_defender_outfits";
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
