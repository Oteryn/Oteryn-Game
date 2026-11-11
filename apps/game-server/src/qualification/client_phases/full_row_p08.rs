//! P08 full-row native-client journey.
//!
//! Scenario derived from the P08 matrix/audit row and its production dispatch owner:
//! 1. Browse the admitted proficiency catalogue and current tracks; expect bounded native reply
//!    pages whose definition, XP, level, selections, thresholds and perk descriptions agree.
//! 2. Walk to a compiled-world rat, select it with the ordinary attack command and finish the
//!    fight; expect the server to credit weapon proficiency XP only for an equipped bound weapon.
//! 3. Select an unlocked perk; expect an accepted selection reply and a matching browse delta.
//! 4. Exercise modify, rank-up, orb-rank, reshape-offer, reshape-choose and clear with the
//!    committed revision; expect typed replies and the corresponding slot transitions/costs.
//! 5. Send a stale-revision modification as the refusal path; expect `StaleRevision` and no
//!    change to the track page.
//! 6. Log out and obtain a fresh admission; expect XP, selections and shaped slots/catalyst
//!    consumption to remain durable. A server/content refusal is reported with the exact step
//!    and observed typed reply, but only after all mechanic commands have actually been sent.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use std::num::NonZeroU32;

#[derive(Default)]
struct Observations {
    failures: Vec<String>,
}

impl Observations {
    fn require(&mut self, step: &str, condition: bool, observed: impl std::fmt::Debug) {
        if !condition {
            self.failures.push(format!("step-{step}:{observed:?}"));
        }
    }
}

async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_WEAPON_PROFICIENCY_V1, kind, payload)
        .await?)
}

async fn browse(client: &mut DevClientSession) -> TestResult<wire::ProficiencyPage> {
    let reply = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &0_u32.to_be_bytes(),
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-browse:status={:?}", reply.status).into());
    }
    wire::ProficiencyPage::decode(&reply.payload)
        .ok_or_else(|| "step-browse:malformed proficiency page".into())
}

async fn catalogue(client: &mut DevClientSession) -> TestResult<wire::ProficiencyCataloguePage> {
    let reply = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &0_u32.to_be_bytes(),
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-catalogue:status={:?}", reply.status).into());
    }
    wire::ProficiencyCataloguePage::decode(&reply.payload)
        .ok_or_else(|| "step-catalogue:malformed catalogue page".into())
}

async fn select(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult<wire::SelectPerkReply> {
    let payload = wire::SelectPerkRequest {
        definition,
        level: 0,
        perk: Some(0),
    }
    .encode();
    let reply = command(client, wire::COMMAND_TYPE_PROFICIENCY_SELECT_PERK, &payload).await?;
    wire::SelectPerkReply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step-select:status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into()
    })
}

async fn modify(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    operation: wire::ModificationOperation,
    revision: u64,
) -> TestResult<wire::ModificationReply> {
    let request = wire::ModificationRequest {
        definition,
        slot: 1,
        operation,
        expected_revision: revision,
    };
    let reply = command(client, operation.command_type(), &request.encode()).await?;
    wire::ModificationReply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step-modify-{operation:?}:status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into()
    })
}

async fn journey(clients: &SeamClients<'_>, observed: &mut Observations) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa2).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let initial_session = client.join_snapshot().game_session_id;
    let catalogue = catalogue(&mut client).await?;
    let before = browse(&mut client).await?;
    let definition = catalogue
        .entries
        .iter()
        .find_map(|entry| NonZeroU32::new(entry.definition))
        .ok_or("step-catalogue:no admitted weapon definition")?;
    let catalogue_entry = catalogue
        .entries
        .iter()
        .find(|entry| entry.definition == definition.get())
        .ok_or("step-1:chosen definition missing from catalogue")?;
    let browse_row = before
        .rows
        .iter()
        .find(|row| row.definition == definition)
        .ok_or("step-1:chosen definition missing from browse page")?;
    let catalogue_level = u8::try_from(
        catalogue_entry
            .thresholds
            .iter()
            .take_while(|threshold| catalogue_entry.experience >= **threshold)
            .count(),
    )?;
    observed.require(
        "1-catalogue-browse-agreement",
        browse_row.name == catalogue_entry.name
            && browse_row.experience == catalogue_entry.experience
            && browse_row.level == catalogue_level
            && browse_row.selections
                == catalogue_entry
                    .levels
                    .iter()
                    .map(|level| level.selected)
                    .collect::<Vec<_>>(),
        (catalogue_entry, browse_row),
    );

    // This is a real player fight. With the node-boot starter's empty hands it also proves that
    // an unrelated kill cannot manufacture a weapon track.
    let combat = async {
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(&mut client).await
    }
    .await;
    observed.require("combat", combat.is_ok(), &combat);
    let after_kill = browse(&mut client).await?;
    observed.require(
        "2-xp-delta",
        after_kill
            .rows
            .iter()
            .zip(&before.rows)
            .any(|(after, before)| {
                after.definition == before.definition && after.experience > before.experience
            }),
        (&before.rows, &after_kill.rows),
    );
    observed.require(
        "2-equipped-bound-weapon",
        false,
        "browse exposes XP but no equipped/bound weapon state; the journey did not equip one",
    );

    let selected = select(&mut client, definition).await?;
    observed.require(
        "select",
        selected.result == wire::SelectPerkResult::Accepted,
        &selected,
    );
    let after_select = browse(&mut client).await?;
    let selected_row = after_select
        .rows
        .iter()
        .find(|row| row.definition == definition);
    observed.require(
        "3-selection-browse-delta",
        selected.result == wire::SelectPerkResult::Accepted
            && selected.selections.first() == Some(&Some(0))
            && selected_row.is_some_and(|row| {
                row.selections == selected.selections
                    && after_kill
                        .rows
                        .iter()
                        .find(|before| before.definition == definition)
                        != Some(row)
            }),
        (&after_kill, &selected, &after_select),
    );
    let operations = [
        wire::ModificationOperation::Modify { level: 0 },
        wire::ModificationOperation::RankUp,
        wire::ModificationOperation::OrbRank,
        wire::ModificationOperation::ReshapeOffer,
        wire::ModificationOperation::ReshapeChoose { choice: Some(0) },
        wire::ModificationOperation::Clear,
    ];
    let mut previous_slot: Option<wire::ModifiedSlot> = None;
    for (index, operation) in operations.into_iter().enumerate() {
        let reply = modify(&mut client, definition, operation, index as u64 + 1).await?;
        observed.require(
            "modification",
            reply.result == wire::ModificationResult::Accepted,
            (operation, reply),
        );
        let transition_matches = match (operation, previous_slot, reply.slot) {
            (wire::ModificationOperation::Modify { level: 0 }, None, Some(after)) => {
                after.level == 0 && after.rank == 0 && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::RankUp, Some(before), Some(after)) => {
                after.level == before.level
                    && after.entry_index == before.entry_index
                    && after.rank == before.rank + 1
                    && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::OrbRank, Some(before), Some(after)) => {
                after.level == before.level
                    && after.entry_index == before.entry_index
                    && after.rank == 10
                    && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::ReshapeOffer, Some(before), Some(after)) => {
                after.level == before.level
                    && after.entry_index == before.entry_index
                    && after.rank == before.rank
                    && after.pending_offer.is_some()
            }
            (
                wire::ModificationOperation::ReshapeChoose { choice: Some(0) },
                Some(before),
                Some(after),
            ) => {
                after.level == before.level
                    && after.rank == before.rank
                    && after.entry_index
                        == before
                            .pending_offer
                            .map(|offer| offer[0])
                            .unwrap_or(u8::MAX)
                    && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::Clear, Some(_), None) => true,
            _ => false,
        };
        observed.require(
            "4-slot-transition",
            transition_matches,
            (operation, previous_slot, reply),
        );
        observed.require(
            "4-cost-delta",
            false,
            format!(
                "operation={operation:?} reply={reply:?}; no player inventory/currency state is exposed by the native proficiency replies"
            ),
        );
        previous_slot = reply.slot;
    }
    let stable = browse(&mut client).await?;
    let refused = modify(
        &mut client,
        definition,
        wire::ModificationOperation::RankUp,
        u64::MAX,
    )
    .await?;
    observed.require(
        "stale-refusal",
        refused.result == wire::ModificationResult::StaleRevision,
        refused,
    );
    observed.require(
        "refusal-no-delta",
        browse(&mut client).await? == stable,
        &stable,
    );

    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-logout:refused".into());
    }
    row_needs_logout_0 = false;
    if row_needs_logout_0 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??; row_needs_logout_0 = false; }
    let mut fresh = connect_world(clients, 0xa3).await?;
    let mut row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    observed.require(
        "fresh-session",
        fresh.join_snapshot().game_session_id != initial_session,
        fresh.join_snapshot().game_session_id,
    );
    observed.require(
        "durable-state",
        browse(&mut fresh).await? == stable,
        &stable,
    );
    observed.require(
        "6-shaped-slot-catalyst-durability",
        false,
        "fresh browse matches XP/selections, but exposes neither shaped slots nor catalyst inventory consumption",
    );
    tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??; row_needs_logout_1 = false;
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??; }
    row_result

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let mut observed = Observations::default();
    let result = row_complete(journey(clients, &mut observed)).await;
    match result {
        Ok(Err(error)) => observed.failures.push(error.to_string()),
        Err(_) => observed.failures.push("step-deadline:timeout".into()),
        Ok(Ok(())) => {}
    }
    if observed.failures.is_empty() {
        evidence(&format!(
            "e2e_all scenario=P08.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        ));
    } else {
        let detail = observed.failures.join(";").replace(' ', "_");
        evidence(&format!(
            "e2e_all scenario=P08.full status=FAIL seconds={:.3} evidence={detail}",
            started.elapsed().as_secs_f64()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_verdict_is_derived_from_failed_steps() {
        let mut observations = Observations::default();
        observations.require("accepted", true, "accepted");
        assert!(observations.failures.is_empty());
        observations.require("refused", false, wire::ModificationResult::NoOrb);
        assert_eq!(observations.failures, ["step-refused:NoOrb"]);
    }
}
