//! G38: Windows touch joystick and action hotbar (#3074, O035 and O056). The feature is
//! `apps/client/src/features/0233-touch-controls` fed by winit touch events in
//! `apps/client/src/windows_shell/application_handler.rs`; it maps touches to the ordinary
//! step and action commands the server already serves.
//! 1. Admit the node-boot character and send the step the joystick would emit; the server
//!    answers it like any step (the disposition is not judged: the touch layer is the subject).
//! 2. Step back to the admission tile, require it, then log out and require Accepted.
//!
//! The dev client has no touch or window layer and cannot synthesize a touch event, so the
//! joystick and hotbar themselves are unreachable: the row ends MISSING-CLIENT.
use super::*;

const SOURCE: &str = "source:apps/client/src/features/0233-touch-controls/input.rs:1";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(60), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("MISSING-CLIENT", SOURCE.to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G38.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xe9).await?;
    let start = client.world_spatial().actor_position;
    let outcome = client.step_retrying(StepDirection::North).await?;
    if outcome.disposition == StepDisposition::Moved {
        // Walk the shared character back so the next journey is admitted at its fixture tile.
        let back = client.step_retrying(StepDirection::South).await?;
        if back.disposition != StepDisposition::Moved {
            return Err(format!("step=restore-position;observed={:?}", back.disposition).into());
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let at = client.world_spatial().actor_position;
        if at.x == start.x && at.y == start.y && at.floor == start.floor {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=restore-position;observed=not-at-start".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_token_has_no_spaces() {
        assert!(SOURCE.starts_with("source:") && !SOURCE.contains(' '));
        assert_eq!(token("a\nb"), "a-b");
    }
}
