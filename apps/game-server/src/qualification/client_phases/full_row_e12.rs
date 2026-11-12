//! E12 full-row player scenario, derived from the E12 audit/matrix and the native owners.
//!
//! 1. Admit the node-boot character and send hireling-management `List`; require one owned
//!    hireling, its lamp in player custody, and the character's effective owned-house admission.
//! 2. Walk onto a non-door tile of that owned house, send command 220 `Summon` for the current
//!    tile, and require an accepted active reply plus the hireling NPC/entity projection delta.
//! 3. Talk to the projected hireling using ordinary NPC dialogue: exercise its bank service and
//!    buy a named cook meal with earned carried coins; require the dialogue reply and Inbox delta.
//! 4. Use the delivered food from the Inbox and require its authoritative condition/vitals delta.
//! 5. Send hireling-management `Outfit` for an entitled outfit and require `Committed`, followed
//!    by a changed projected appearance; then request an unowned outfit and require
//!    `Refused(OutfitNotOwned)` with no appearance change.
//! 6. Send `Dismiss`/`ReturnToLamp`; require an inactive reply, removal of the projected NPC, and
//!    the durable lamp returning to the Inbox. A repeated dismiss is the refusal path and must
//!    remain inactive without minting another lamp.
//! 7. Log out, obtain a fresh admission, and re-run `List`/`Query`; require the selected outfit,
//!    hireling identity and lamp custody to persist while no dismissed NPC is projected.
//!
//! The first command is deliberately the durable owner's native List rather than a source probe.
//! If node-boot content supplies neither an owned house nor a purchased hireling, the server
//! returns an accepted empty State. That observed prerequisite failure is reported and later
//! identity/house-bound commands cannot be truthfully fabricated by the test.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::hireling::{self, management};

const DEADLINE: Duration = Duration::from_secs(180);

#[derive(Debug)]
struct StepFailure {
    step: u8,
    observed: String,
    need: &'static str,
}

impl std::fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "step={};observed={};need={}",
            self.step,
            token(&self.observed),
            self.need
        )
    }
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:,".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

async fn management_command(
    client: &mut DevClientSession,
    request: management::Request,
) -> Result<(CommandStatus, Option<management::Reply>), StepFailure> {
    let response = client
        .session_mut()
        .gated_command(
            management::CAPABILITY_HIRELING_MANAGEMENT_V1,
            management::COMMAND_TYPE_HIRELING_MANAGEMENT,
            &request.encode(),
        )
        .await
        .map_err(|error| StepFailure {
            step: 1,
            observed: format!("transport-{error}"),
            need: "responsive-hireling-management-route",
        })?;
    let reply = management::Reply::decode(&response.payload);
    Ok((response.status, reply))
}

async fn talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> Result<oteryn_protocol_oteryn::npc_service::NpcConversation, StepFailure> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as wire;
    let disposition = npc_shop::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        },
    )
    .await
    .map_err(|error| StepFailure {
        step: 3,
        observed: format!("talk-{text}-transport-{error}"),
        need: "responsive-projected-hireling-dialogue",
    })?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(StepFailure {
            step: 3,
            observed: format!("talk-{text}-disposition-{disposition:?}"),
            need: "admitted-owner-hireling-service",
        });
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await
        .map_err(|error| StepFailure {
            step: 3,
            observed: format!("talk-{text}-delta-{error}"),
            need: "hireling-conversation-delta",
        })?;
    wire::decode_npc_conversation(&delta.payload).map_err(|error| StepFailure {
        step: 3,
        observed: format!("talk-{text}-invalid-delta-{error:?}"),
        need: "decodable-hireling-conversation-delta",
    })
}

async fn journey(clients: &SeamClients<'_>) -> Result<(), StepFailure> {
    let mut client = connect_world(clients, 0xe1)
        .await
        .map_err(|error| StepFailure {
            step: 1,
            observed: format!("admission-{error}"),
            need: "fresh-player-admission",
        })?;
    if !client
        .selected_capabilities()
        .contains(&management::CAPABILITY_HIRELING_MANAGEMENT_V1)
    {
        return Err(StepFailure {
            step: 1,
            observed: "capability-250-not-selected".into(),
            need: "activated-hireling-capability",
        });
    }

    let (status, reply) = management_command(&mut client, management::Request::List).await?;
    let entries = match (status, reply) {
        (CommandStatus::Accepted, Some(management::Reply::State(entries)))
            if !entries.is_empty() =>
        {
            entries
        }
        (status, reply) => {
            return Err(StepFailure {
                step: 1,
                observed: format!("status-{status:?}-reply-{reply:?}"),
                need: "player-obtainable-owned-house-and-hireling-lamp",
            });
        }
    };
    let owned = &entries[0];
    let position = client.world_spatial().actor_position;
    let summon = hireling::Request::Summon {
        x: u16::try_from(position.x).map_err(|_| StepFailure {
            step: 2,
            observed: format!("invalid-x-{}", position.x),
            need: "house-position-in-wire-range",
        })?,
        y: u16::try_from(position.y).map_err(|_| StepFailure {
            step: 2,
            observed: format!("invalid-y-{}", position.y),
            need: "house-position-in-wire-range",
        })?,
        floor: i8::try_from(position.floor).map_err(|_| StepFailure {
            step: 2,
            observed: format!("invalid-floor-{}", position.floor),
            need: "house-position-in-wire-range",
        })?,
    };
    let response = client
        .session_mut()
        .gated_command(
            hireling::CAPABILITY_HIRELING_V1,
            hireling::COMMAND_TYPE_HIRELING,
            &summon.encode(),
        )
        .await
        .map_err(|error| StepFailure {
            step: 2,
            observed: format!("transport-{error}"),
            need: "responsive-hireling-summon-route",
        })?;
    if response.status != CommandStatus::Accepted
        || hireling::decode_reply(&response.payload) != Some(true)
    {
        return Err(StepFailure {
            step: 2,
            observed: format!(
                "status-{:?}-payload-{:?}",
                response.status, response.payload
            ),
            need: "walkable-owned-house-non-door-tile",
        });
    }

    // The remaining lifecycle is reachable only after the real summon. Refuse to claim the
    // projected identity until the server publishes the exact durable hireling binding.
    let (status, projection) =
        management_command(&mut client, management::Request::Projection).await?;
    let npc = match projection {
        Some(management::Reply::Projection(bindings))
            if status == CommandStatus::Accepted
                && bindings
                    .iter()
                    .any(|binding| binding.hireling == owned.hireling) =>
        {
            bindings
                .into_iter()
                .find(|binding| binding.hireling == owned.hireling)
                .map(|binding| binding.entity)
                .ok_or_else(|| StepFailure {
                    step: 2,
                    observed: "projection-lost-matching-binding".into(),
                    need: "summoned-hireling-entity-projection",
                })?
        }
        reply => {
            return Err(StepFailure {
                step: 2,
                observed: format!("projection-status-{status:?}-reply-{reply:?}"),
                need: "summoned-hireling-entity-projection",
            });
        }
    };

    for word in ["hi", "food", "specific", "carrot pie"] {
        let conversation = talk(&mut client, npc, word).await?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) || conversation.lines.is_empty() {
            return Err(StepFailure {
                step: 3,
                observed: format!("talk-{word}-conversation-{conversation:?}"),
                need: "cook-service-dialogue-and-delivery",
            });
        }
        if word == "food"
            && conversation
                .lines
                .iter()
                .any(|line| line.contains("do not have mastery"))
        {
            return Err(StepFailure {
                step: 3,
                observed: format!("talk-food-reply-{:?}", conversation.lines),
                need: "player-obtainable-cook-hireling-skill",
            });
        }
        if word == "carrot pie"
            && !conversation
                .lines
                .iter()
                .any(|line| line.contains("delivered to your Inbox"))
        {
            return Err(StepFailure {
                step: 3,
                observed: format!("talk-carrot-pie-reply-{:?}", conversation.lines),
                need: "earned-90000-gold-and-inbox-capacity",
            });
        }
    }

    Err(StepFailure {
        step: 4,
        observed: "food-delivered-awaiting-player-inbox-use".into(),
        need: "native-inbox-item-use-route-in-e12-journey",
    })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(DEADLINE, verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=E12.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E12.full status=FAIL seconds={:.3} evidence={error}",
            started.elapsed().as_secs_f64()
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=E12.full status=FAIL seconds={:.3} evidence=step=runtime;observed=timeout;need=responsive-hireling-owner",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::token;

    #[test]
    fn evidence_token_has_no_report_delimiters() {
        let encoded = token("State([]) no house");
        assert!(encoded.starts_with("State"));
        assert!(encoded.ends_with("no_house"));
        assert!(!encoded.contains(char::is_whitespace));
    }
}
