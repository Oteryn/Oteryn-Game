use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::exiva_settings::{PlayerList, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    list: PlayerList,
    commands: Arc<Mutex<Vec<Vec<u8>>>>,
}
impl Stepper for Peer {
    fn exiva_selected(&self) -> bool {
        true
    }
    async fn exiva(&mut self, payload: &[u8]) -> Result<GatedCommandResult, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(payload.to_vec());
        match payload {
            [6, rest @ ..] => {
                assert_eq!(rest, b"Al");
                self.list.players.push(([3; 16], "Al".into()));
            }
            [2, rest @ ..] => {
                assert_eq!(rest, &[3; 16]);
                self.list.players.clear();
            }
            [5] => {}
            _ => return Err(SessionError::SessionUnusable),
        }
        Ok(GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: if payload == [5] {
                self.list.encode().ok_or(SessionError::SessionUnusable)?
            } else {
                vec![1]
            },
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}
async fn ready(link: &PlayLink, count: Option<usize>) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.exiva_panel();
            if panel.selected
                && !panel.pending
                && count.is_none_or(|n| panel.list.as_ref().is_some_and(|l| l.players.len() == n))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn exiva_native_input_runs_edits_and_server_refresh_through_the_session_queue() -> Result<(), Error>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    list: PlayerList {
                        modes: 2,
                        players: vec![],
                    },
                    commands: Arc::clone(&commands),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, None).await?;
                let open = NormalizedInputEvent::Key {
                    code: KeyCode::new(8)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control),
                    repeat: false,
                };
                assert!(link.exiva_input(&[open]));
                ready(&link, Some(0)).await?;
                assert!(link.exiva_panel().open);
                link.exiva_request(vec![6, b'A', b'l']);
                ready(&link, Some(1)).await?;
                assert!(panel_text(&link.exiva_panel()).contains("Al"));
                let mut remove = vec![2];
                remove.extend([3; 16]);
                link.exiva_request(remove.clone());
                ready(&link, Some(0)).await?;
                assert_eq!(
                    *commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![vec![5], vec![6, b'A', b'l'], vec![5], remove, vec![5]]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Exiva UI test thread panicked")?
}

#[test]
fn e2e_p17_replays_live_exiva_results() -> Result<(), Error> {
    use oteryn_session::CommandStatus;

    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = std::path::Path::new(&root);
    let expected = std::fs::read_to_string(root.join("p17-expected.txt"))?;
    let field = |key: &str| -> Result<&str, Error> {
        expected
            .lines()
            .filter_map(|line| line.split_once('='))
            .find_map(|(name, value)| (name == key).then_some(value))
            .ok_or_else(|| format!("expected line {key} absent").into())
    };
    let binding = std::env::var("E2E_P17_BINDING")?;
    assert_eq!(field("binding")?, binding);
    let fields: Vec<_> = binding.split(':').collect();
    let [original, deny, allow, refuse, relog, retain, remove] = fields.as_slice() else {
        return Err("expected two sessions and five server sequences".into());
    };
    assert_ne!(original, relog);
    let (deny, allow, refuse) = (
        deny.parse::<u64>()?,
        allow.parse::<u64>()?,
        refuse.parse::<u64>()?,
    );
    let (retain, remove) = (retain.parse::<u64>()?, remove.parse::<u64>()?);
    assert!(0 < deny && deny < allow && allow < refuse);
    assert!(0 < retain && retain < remove);

    let result = |name: &str, status, server_sequence| -> Result<GatedCommandResult, Error> {
        let command_id = field(&format!("{name}_command"))?.parse()?;
        assert_ne!(command_id, 0);
        Ok(GatedCommandResult {
            command_id,
            status,
            server_sequence,
            payload: std::fs::read(root.join(format!("p17-{name}.bin")))?,
        })
    };
    let (id_hex, name) = field("player")?
        .split_once(' ')
        .ok_or("missing named entry")?;
    assert_eq!(id_hex.len(), 32);
    let mut id = [0; 16];
    for (index, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&id_hex[index * 2..index * 2 + 2], 16)?;
    }
    let listed = PlayerList {
        modes: 2,
        players: vec![(id, name.to_owned())],
    };
    let (link, _commands, outbox) = play_channel();
    outbox.exiva_initialize(true);
    outbox.exiva_result(&result("denied", CommandStatus::Accepted, deny)?);
    assert_eq!(
        link.exiva_panel().list,
        Some(PlayerList {
            modes: 0,
            players: vec![]
        })
    );
    assert_eq!(
        panel_text(&link.exiva_panel()),
        "Exiva allow list (0/100)\r\n"
    );
    outbox.exiva_result(&result("allowed", CommandStatus::Accepted, allow)?);
    assert_eq!(link.exiva_panel().list.as_ref(), Some(&listed));
    assert_eq!(
        panel_text(&link.exiva_panel()),
        format!("Exiva allow list (1/100)\r\n{name}")
    );
    assert_eq!(link.exiva_panel().error, None);

    link.exiva_request(vec![5]);
    assert!(link.exiva_panel().pending);
    let status = match field("refused_status")? {
        "Rejected" => CommandStatus::Rejected,
        other => return Err(format!("expected Rejected, captured {other}").into()),
    };
    let refused = result("refused", status, refuse)?;
    assert_eq!(refused.payload, [0]);
    outbox.exiva_result(&refused);
    let panel = link.exiva_panel();
    assert_eq!(panel.error, Some("Exiva request refused; refresh the list"));
    assert!(!panel.pending);
    assert_eq!(panel.list.as_ref(), Some(&listed));

    let (link, _commands, outbox) = play_channel();
    outbox.exiva_initialize(true);
    outbox.exiva_result(&result("retained", CommandStatus::Accepted, retain)?);
    assert_eq!(
        link.exiva_panel().list,
        Some(listed),
        "relogin lost named permissions"
    );
    assert_eq!(link.exiva_panel().error, None);
    outbox.exiva_result(&result("removed", CommandStatus::Accepted, remove)?);
    assert_eq!(
        link.exiva_panel().list,
        Some(PlayerList {
            modes: 2,
            players: vec![]
        })
    );
    assert_eq!(
        panel_text(&link.exiva_panel()),
        "Exiva allow list (0/100)\r\n"
    );
    Ok(())
}
