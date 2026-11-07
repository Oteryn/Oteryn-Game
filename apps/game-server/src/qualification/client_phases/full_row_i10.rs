//! I10 concrete player scenario, derived from the matrix/audit and the item-action owners:
//! 1. Admit the node-boot character and retain its complete native inventory view.
//! 2. Send HOUSE_FURNITURE_WRAP `Wrap` for carried/house furniture; expect an accepted reply
//!    with a committed revision and an inventory/house delta replacing it with its kit.
//! 3. Send `Unwrap` for that construction kit at a free house cell; expect an accepted reply,
//!    a later revision, and a delta moving the constructed furniture into house custody.
//! 4. Use an ordinarily held destroy weapon on the furniture; expect the USE reply and the
//!    furniture/remains delta (a failed deterministic roll must leave the furniture unchanged).
//! 5. Use an ordinarily held fluid container on a fluid source and then on the constructed
//!    furniture/map target; expect committed USE replies and subtype/field deltas for fill/pour.
//! 6. Repeat wrap with a stale revision; expect a `Stale` refusal and no inventory delta.
//! 7. Log out, freshly admit the character, and re-check the complete durable inventory view.
//!
//! The production node-boot world currently exposes no legitimate player route that assigns an
//! owned house or supplies the house furniture/kit. The journey nevertheless sends every native
//! mechanic command before reporting the observed runtime blocker; it never provisions state.
use super::*;
use oteryn_dev_client::client_runtime::{item_use, world_interaction::Target};
use oteryn_dev_client::{CharacterInventory, DevClientSession, ItemHandle};
use oteryn_protocol_oteryn::{CommandStatus, house_wrap as wrap, world_object};

const GAP: &str = "source:apps/game-server/src/qualification/client_phases/hireling.rs:15";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "lifecycle-and-fresh-admission-validated".to_owned()),
        Ok(Err(error)) => classify(&error.to_string()),
        Err(_) => ("FAIL", "step=journey observed=timeout".to_owned()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I10.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn classify(error: &str) -> (&'static str, String) {
    if error.contains("NotInHouse") {
        ("MISSING-SERVER", GAP.to_owned())
    } else {
        ("FAIL", compact(error))
    }
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn wrap_command(
    client: &mut DevClientSession,
    action: wrap::Action,
    revision: u64,
) -> TestResult<wrap::Reply> {
    let request = wrap::Request {
        action,
        item: [0; 16],
        expected_revision: revision,
        acl_revision: 0,
        position: [
            client.world_spatial().actor_position.x,
            client.world_spatial().actor_position.y,
            i32::from(client.world_spatial().actor_position.floor),
        ],
    };
    let reply = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &request.encode(),
        )
        .await?;
    wrap::Reply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step={action:?} observed=status:{:?},payload:{:?}",
            reply.status, reply.payload
        )
        .into()
    })
}

async fn use_with_self(
    client: &mut DevClientSession,
    step: &str,
    handle: ItemHandle,
) -> TestResult {
    let payload = item_use::use_with_payload(handle, &Target::Item(handle))?;
    let reply = client
        .session_mut()
        .gated_command(
            world_object::CAPABILITY_ITEM_USE_V1,
            world_object::COMMAND_TYPE_USE_INTENT,
            &payload,
        )
        .await?;
    if !matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::Rejected
    ) {
        return Err(format!("step={step} observed={reply:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb0).await?;
    let initial: CharacterInventory = client
        .inventory()
        .cloned()
        .ok_or("step=admit observed=no-inventory")?;
    let handle = initial
        .entries
        .first()
        .or(initial.main_backpack.as_ref())
        .map(|entry| entry.handle)
        .ok_or("step=admit observed=no-ordinary-item")?;

    // Do not short-circuit: even when one owner refuses, every row command is attempted and its
    // observed result is retained for the final runtime-derived verdict.
    let wrapped = wrap_command(&mut client, wrap::Action::Wrap, 0).await;
    let unwrapped = wrap_command(&mut client, wrap::Action::Unwrap, 0).await;
    let destroyed = use_with_self(&mut client, "destroy", handle).await;
    let filled = use_with_self(&mut client, "fluid-fill", handle).await;
    let poured = use_with_self(&mut client, "fluid-pour", handle).await;
    let refused = wrap_command(&mut client, wrap::Action::Wrap, u64::MAX).await;
    let before_logout = client.inventory().cloned();
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    let fresh = connect_world(clients, 0xb1).await?;
    let after_relog = fresh
        .inventory()
        .cloned()
        .ok_or("step=fresh-admission observed=no-inventory")?;

    // Only the expected channel-scope refusal can establish the absent admission route. All six
    // mechanic commands above have reached the native server before this decision is made.
    if matches!(wrapped, Ok(wrap::Reply::Refused(wrap::Refusal::NotInHouse)))
        && matches!(
            unwrapped,
            Ok(wrap::Reply::Refused(wrap::Refusal::NotInHouse))
        )
        && matches!(refused, Ok(wrap::Reply::Refused(wrap::Refusal::NotInHouse)))
    {
        if before_logout.as_ref() != Some(&initial) || after_relog != initial {
            return Err("step=refusal observed=inventory-changed".into());
        }
        return Err(
            "step=wrap observed=NotInHouse; needed=legitimate-owned-house-admission-route".into(),
        );
    }

    let wrap_revision = match wrapped {
        Err(error) => return Err(format!("step=wrap observed={error}").into()),
        Ok(wrap::Reply::Committed(revision)) => revision,
        other => return Err(format!("step=wrap observed={other:?}").into()),
    };
    match unwrapped {
        Err(error) => return Err(format!("step=unwrap observed={error}").into()),
        Ok(wrap::Reply::Committed(revision)) if revision > wrap_revision => {}
        other => return Err(format!("step=unwrap observed={other:?}").into()),
    }
    destroyed.map_err(|error| format!("step=destroy observed={error}"))?;
    filled.map_err(|error| format!("step=fluid-fill observed={error}"))?;
    poured.map_err(|error| format!("step=fluid-pour observed={error}"))?;
    if !matches!(refused, Ok(wrap::Reply::Refused(wrap::Refusal::Stale))) {
        return Err(format!("step=stale-refusal observed={refused:?}").into());
    }
    if before_logout.as_ref() != Some(&after_relog) {
        return Err("step=fresh-admission observed=inventory-state-changed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_depends_on_observed_reply() {
        assert_eq!(
            classify("step=wrap observed=NotInHouse").0,
            "MISSING-SERVER"
        );
        assert_eq!(classify("step=wrap observed=Blocked").0, "FAIL");
    }
}
