//! G15 analyser item and party hunt queries (capability 10, commands 734 and 735).
//! 1. Admit both provisioned characters; wait until they see each other, then form a party
//!    (invite, accept) through the party command.
//! 2. Query the item catalogue (734) for the gold coin items and carried items: names come from
//!    the server catalogue and the coin worth must be exactly 1, 100 and 10000 for the three coins.
//! 3. Query the party hunt analyser (735) from both members with a price list: both members'
//!    names (read-only SQL) must appear in each reply.
//! 4. Leave the party and log both out. No kills are made, so loot tracker totals stay zero.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::EntityRef;
use oteryn_dev_client::client_runtime::party::party::{PartyDisposition, PartyIntent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{analyser_catalogue as items, analyser_party as hunt};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "analyser-item-and-party-queries-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G15.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    let result = client.session_mut().logout().await?.result;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={result:?}").into());
    }
    Ok(())
}

const ANALYSER: u32 = 10;
const COINS: [(u32, u64); 3] = [(3031, 1), (3035, 100), (3043, 10_000)];

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("step=admit observed=no-entity-view")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "step=admit observed=own-entity-absent".into())
}

fn seen(client: &DevClientSession, target: EntityRef) -> bool {
    client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|entry| entry.entity == target))
}

async fn pump(a: &mut DevClientSession, b: &mut DevClientSession) -> TestResult {
    a.service_liveness(Duration::from_millis(25)).await?;
    b.service_liveness(Duration::from_millis(25)).await?;
    a.take_events();
    b.take_events();
    Ok(())
}

/// Stages the peers once and requires both live entity views to contain the other player.
async fn see_each_other(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<EntityRef> {
    let leader = own(a)?;
    let target = own(b)?;
    let from = tile(a.world_spatial().actor_position)?;
    let peer = tile(b.world_spatial().actor_position)?;
    let world = full_e2ef7::qualification_world(clients)?;
    let path = route(world, from, |at| {
        at.floor == peer.floor
            && at != peer
            && at.x.abs_diff(peer.x).max(at.y.abs_diff(peer.y)) <= 1
    })
    .ok_or("step=admit observed=no-route-to-peer")?;
    let (done, mut walking) = tokio::sync::oneshot::channel::<()>();
    // Finish any peer frame in flight before giving the session back to the party journey.
    let (walked, serviced) = tokio::join!(
        async {
            // Dropping the sender also signals completion on the first failed step.
            let _done = done;
            for direction in path {
                let before = tile(a.world_spatial().actor_position)?;
                let expected = destination(world, before, direction)
                    .ok_or("step=peer-approach observed=no-planned-landing")?;
                let outcome = a.step(direction).await?;
                let actual = tile(a.world_spatial().actor_position)?;
                if outcome.disposition != StepDisposition::Moved || actual != expected {
                    return Err(format!(
                        "step=peer-approach direction={direction:?} from={before:?} expected={expected:?} actual={actual:?} observed={outcome:?}"
                    )
                    .into());
                }
                verdict::Harness::global().progress("walk");
                a.take_events();
            }
            TestResult::Ok(())
        },
        async {
            while matches!(
                walking.try_recv(),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty)
            ) {
                b.service_liveness(Duration::from_millis(25)).await?;
            }
            Ok::<(), oteryn_dev_client::DevClientError>(())
        },
    );
    walked?;
    serviced?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        if seen(a, target) && seen(b, leader) {
            return Ok(target);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "step=admit observed=other-player-not-in-view needed=two-players-in-view".into(),
            );
        }
        pump(a, b).await?;
    }
}

async fn name_of(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut connection)
    .await?)
}

async fn party(
    client: &mut DevClientSession,
    intent: Option<PartyIntent>,
) -> TestResult<(
    PartyDisposition,
    oteryn_dev_client::client_runtime::party::party::PartyResult,
)> {
    use oteryn_dev_client::client_runtime::party as runtime;
    let command = match intent {
        Some(intent) => {
            runtime::PartyCommand::intent(intent).map_err(|e| format!("step=party encode={e:?}"))?
        }
        None => runtime::PartyCommand::query(),
    };
    let reply = client
        .session_mut()
        .gated_command(
            runtime::party::CAPABILITY_PARTY_V1,
            command.kind,
            &command.payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=party command={} observed={:?} payload={:?}",
            command.kind, reply.status, reply.payload
        )
        .into());
    }
    let result = runtime::party::decode_party_result(&reply.payload)
        .map_err(|e| format!("step=party command={} decode={e:?}", command.kind))?;
    Ok((result.disposition, result))
}

fn coin_worth(index: &[(u32, u32)], item: u32) -> Option<u64> {
    index
        .iter()
        .find(|(reference, _)| *reference == item)
        .map(|(_, worth)| u64::from(*worth))
}

fn idle_accounting(members: &[hunt::Member], names: &[&str; 2]) -> bool {
    let [first, second] = members else {
        return false;
    };
    first.handle != second.handle
        && names
            .iter()
            .all(|name| members.iter().any(|member| member.name == *name))
        && members.iter().all(|member| {
            member.available
                && !member.incomplete
                && member.needed_prices.is_empty()
                && [
                    member.kills,
                    member.xp,
                    member.raw_xp,
                    member.loot_items,
                    member.loot_value,
                    member.supplies,
                    member.supply_cost,
                    member.unpriced_loot,
                    member.unpriced_supplies,
                    member.damage,
                    member.healing,
                ] == [0; 11]
        })
}

fn invitation_handle(
    reply: PartyDisposition,
    view: &oteryn_protocol_oteryn::party::PartyResult,
    inviter: &str,
    now: u64,
    expires: u64,
) -> Option<u64> {
    let [invite] = view.invitations.as_slice() else {
        return None;
    };
    (reply == PartyDisposition::Ok
        && view.disposition == PartyDisposition::Ok
        && view.members.is_empty()
        && invite.name == inviter
        && invite.expires_at > now
        && invite.expires_at == expires)
        .then_some(invite.handle)
}

fn catalogue_matches(rows: &[items::ItemInfo], expected: &[items::ItemInfo]) -> bool {
    rows.len() == expected.len() && expected.iter().all(|item| rows.contains(item))
}

async fn members(
    client: &mut DevClientSession,
    prices: &[hunt::Price],
) -> TestResult<Vec<hunt::Member>> {
    let payload = hunt::encode_request(prices)?;
    let reply = accepted(
        client,
        ANALYSER,
        hunt::COMMAND_TYPE_ANALYSER_PARTY_QUERY,
        &payload,
    )
    .await?;
    Ok(hunt::decode_reply(&reply)?)
}

async fn body(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    let a_name = name_of(clients, 0).await?;
    let b_name = name_of(clients, 1).await?;
    for client in [&mut *a, &mut *b] {
        assert_solo(client).await?;
    }
    let b_seen = see_each_other(a, b, clients).await?;
    let (reply, _) = party(a, Some(PartyIntent::Invite(b_seen))).await?;
    if reply != PartyDisposition::Ok {
        return Err(format!("step=invite observed={reply:?}").into());
    }
    let (reply, invitation) = party(b, None).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let (now, expires): (i64, i64) = sqlx::query_as(
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint,
                ceil(extract(epoch FROM i.expires_at))::bigint
         FROM game_party_invitations i JOIN game_parties p USING (party_id)
         WHERE p.leader_character_id = encode($1,'hex')::uuid
           AND i.invitee = encode($2,'hex')::uuid
           AND i.expires_at > clock_timestamp()",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.characters[1].as_slice())
    .fetch_one(&mut db)
    .await?;
    drop(db);
    let handle = invitation_handle(
        reply,
        &invitation,
        &a_name,
        now.try_into()?,
        expires.try_into()?,
    )
    .ok_or_else(|| {
        format!("step=invite observed={reply:?}/{invitation:?} now={now} expected-expiry={expires}")
    })?;
    let (reply, _) = party(b, Some(PartyIntent::Accept(handle))).await?;
    if reply != PartyDisposition::Ok {
        return Err(format!("step=accept observed={reply:?}").into());
    }
    for client in [&mut *a, &mut *b] {
        let (reply, view) = party(client, None).await?;
        if reply != PartyDisposition::Ok
            || !view.invitations.is_empty()
            || view.members.len() != 2
            || !view
                .members
                .iter()
                .any(|entry| entry.name == a_name && entry.leader)
            || !view
                .members
                .iter()
                .any(|entry| entry.name == b_name && !entry.leader)
        {
            return Err(format!("step=party observed={reply:?}/{view:?}").into());
        }
    }

    let index = provisioning::item_index()?;
    let mut worth = Vec::new();
    let mut ids = Vec::new();
    for (tibia, value) in COINS {
        let reference = provisioning::item_reference(&index, tibia)?.get();
        worth.push((reference, u32::try_from(value)?));
        ids.push(reference);
    }
    let inventory = a
        .inventory()
        .ok_or("step=items observed=no-inventory-view")?;
    let carried: std::collections::BTreeSet<u32> = inventory
        .entries
        .iter()
        .map(|entry| entry.item_definition_ref.get())
        .filter(|reference| !ids.contains(reference))
        .collect();
    if carried.is_empty() {
        return Err("step=items observed=no-carried-non-currency-items".into());
    }
    ids.extend(carried);
    use crate::content::ReferenceItemField::Known;
    use crate::content::native_gameplay::ItemProfilesDocument;
    let profiles: ItemProfilesDocument = serde_json::from_slice(
        &native_input()?
            .item_profiles
            .as_ref()
            .ok_or("missing pinned item profiles")?
            .bytes,
    )?;
    let expected = ids
        .iter()
        .map(|id| -> TestResult<items::ItemInfo> {
            let (key, revision) = index
                .definition_at(std::num::NonZeroU32::try_from(*id)?)
                .ok_or("missing expected item identity")?;
            let profile = profiles
                .records
                .iter()
                .find(|profile| {
                    let definition = &profile.production_definition;
                    definition.production_key == key && definition.revision_ref == revision
                })
                .ok_or_else(|| format!("missing pinned profile {key}/{revision}"))?;
            let Known(presentation) = &profile.semantics.presentation else {
                return Err(format!("missing pinned presentation {key}").into());
            };
            let Known(name) = &presentation.name else {
                return Err(format!("missing pinned name {key}").into());
            };
            Ok(items::ItemInfo {
                item_type: *id,
                name: name.clone(),
                coin_worth: coin_worth(&worth, *id).unwrap_or(0),
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    let payload = items::encode_request(&ids)?;
    let reply = accepted(
        a,
        ANALYSER,
        items::COMMAND_TYPE_ANALYSER_ITEM_QUERY,
        &payload,
    )
    .await?;
    let rows = items::decode_reply(&reply)?;
    if !catalogue_matches(&rows, &expected) {
        return Err(format!("step=items observed={rows:?} expected={expected:?}").into());
    }

    let prices: Vec<hunt::Price> = ids
        .iter()
        .map(|item| hunt::Price {
            item: *item,
            loot: Some(1),
            supply: Some(1),
        })
        .collect();
    for (who, client) in [("leader", &mut *a), ("member", &mut *b)] {
        let rows = members(client, &prices).await?;
        if !idle_accounting(&rows, &[a_name.as_str(), b_name.as_str()]) {
            return Err(format!(
                "step=party-analyser observed={who}-names:{:?} needed={a_name}+{b_name}",
                rows
            )
            .into());
        }
    }
    Ok(())
}

async fn assert_solo(client: &mut DevClientSession) -> TestResult {
    let (reply, view) = party(client, None).await?;
    if reply != PartyDisposition::Ok || !view.members.is_empty() || !view.invitations.is_empty() {
        return Err(format!("step=solo observed={reply:?}/{view:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut a = connect_character(clients, 0x78, 0).await?;
    let mut b = match connect_character(clients, 0x79, 1).await {
        Ok(b) => b,
        Err(error) => {
            verdict::record_error(error.as_ref());
            finish(&mut a).await.ok();
            return Err(error);
        }
    };
    let before = a.world_spatial().actor_position;
    let outcome = verdict::observe(async {
        body(&mut a, &mut b, clients).await?;
        let (reply, _) = party(&mut b, Some(PartyIntent::Leave)).await?;
        if reply != PartyDisposition::Ok {
            return Err(format!("step=leave observed={reply:?}").into());
        }
        assert_solo(&mut b).await?;
        assert_solo(&mut a).await?;
        let after = a.world_spatial().actor_position;
        if after == before {
            return Err("step=position observed=unchanged".into());
        }
        TestResult::Ok(after)
    })
    .await;
    let member_position = b.world_spatial().actor_position;
    let closed_b = verdict::observe(finish(&mut b)).await;
    let closed_a = verdict::observe(finish(&mut a)).await;
    let expected = outcome?;
    closed_b?;
    closed_a?;
    for (index, tag, position) in [(0, 0x7a, expected), (1, 0x7b, member_position)] {
        let mut relogged = connect_character(clients, tag, index).await?;
        let restored = verdict::observe(async {
            let actual = relogged.world_spatial().actor_position;
            if actual != position {
                return Err(format!(
                    "step=relog player={index} position={actual:?} expected={position:?}"
                )
                .into());
            }
            assert_solo(&mut relogged).await?;
            let rows = members(&mut relogged, &[]).await?;
            if !rows.is_empty() {
                return Err(format!("step=relog player={index} stale-accounting={rows:?}").into());
            }
            TestResult::Ok(())
        })
        .await;
        let closed = verdict::observe(finish(&mut relogged)).await;
        restored?;
        closed?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn idle_accounting_rejects_false_positives() {
        let member = |name: &str| hunt::Member {
            handle: if name == "Alpha" { 1 } else { 2 },
            name: name.to_owned(),
            available: true,
            incomplete: false,
            elapsed_seconds: 0,
            kills: 0,
            xp: 0,
            raw_xp: 0,
            loot_items: 0,
            loot_value: 0,
            supplies: 0,
            supply_cost: 0,
            unpriced_loot: 0,
            unpriced_supplies: 0,
            damage: 0,
            healing: 0,
            needed_prices: Vec::new(),
        };
        let rows = [member("Alpha"), member("beta")];
        let names = ["Alpha", "beta"];
        assert!(idle_accounting(&rows, &names));
        assert!(!idle_accounting(
            &[rows[0].clone(), rows[0].clone()],
            &names
        ));
        assert!(!idle_accounting(
            &[rows[0].clone(), rows[1].clone(), member("extra")],
            &names
        ));
        for corrupt in [
            |m: &mut hunt::Member| m.available = false,
            |m: &mut hunt::Member| m.incomplete = true,
            |m: &mut hunt::Member| m.loot_value = 1,
            |m: &mut hunt::Member| m.supply_cost = 1,
        ] {
            let mut bad = rows.clone();
            corrupt(&mut bad[0]);
            assert!(!idle_accounting(&bad, &names));
        }
    }

    #[test]
    fn catalogue_rejects_wrong_names_and_missing_carried_items() {
        let expected = [
            items::ItemInfo {
                item_type: 1,
                name: "gold coin".into(),
                coin_worth: 1,
            },
            items::ItemInfo {
                item_type: 2,
                name: "rope".into(),
                coin_worth: 0,
            },
        ];
        assert!(catalogue_matches(&expected, &expected));
        assert!(!catalogue_matches(&expected[..1], &expected));
        let mut bad = expected.clone();
        bad[1].name = "shovel".into();
        assert!(!catalogue_matches(&bad, &expected));
    }

    #[test]
    fn invitation_rejects_refused_queries_and_bogus_expiry() {
        use oteryn_protocol_oteryn::party::{PartyEntry, PartyResult};
        let mut view = PartyResult::refused(PartyDisposition::Ok);
        view.invitations.push(PartyEntry {
            list: vec![],
            handle: 1,
            name: "Leader".into(),
            leader: false,
            expires_at: 400,
            party_spell_remaining_ms: None,
            channel: None,
        });
        assert_eq!(
            invitation_handle(PartyDisposition::Ok, &view, "Leader", 100, 400),
            Some(1)
        );
        assert_eq!(
            invitation_handle(PartyDisposition::Rejected, &view, "Leader", 100, 400),
            None
        );
        assert_eq!(
            invitation_handle(PartyDisposition::Ok, &view, "Leader", 400, 400),
            None
        );
        assert_eq!(
            invitation_handle(PartyDisposition::Ok, &view, "Leader", 100, 401),
            None
        );
    }

    #[test]
    fn coin_worth_reads_the_index() {
        assert_eq!(coin_worth(&[(7, 100)], 7), Some(100));
        assert_eq!(coin_worth(&[(7, 100)], 8), None);
    }
}
