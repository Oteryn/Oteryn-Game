//! Production session, graph/native callback methods and the unchanged161 save/reset writer.
use super::*;
use crate::play::feature_0148_wheel_allocation::input::wheel_presentation::{
    Action, activate, query,
};
use oteryn_client_runtime::{
    raw_feature::{FeatureCommand, GatedCommandResult},
    wheel_presentation as graph,
};
use oteryn_client_ui::wheel_allocation::diagram;
struct Live(Peer);
fn reply(request: graph::Request) -> graph::Reply {
    graph::Reply {
        request,
        catalogue: "server-qualified".into(),
        vocation: "knight".into(),
        available: 300,
        unlock_points: 300,
        removal_permitted: true,
        eligible: true,
        draft_valid: true,
        reason: String::new(),
        nodes: (0..36)
            .map(|_| graph::Node {
                domain: 0,
                capacity: 50,
                minimum_points: 0,
                unlock_from: Vec::new(),
                name: "Server-configured perk".into(),
                per_point: vec!["Maximum mana: +1".into()],
                full_slot: vec!["Maximum mana: +50".into()],
            })
            .collect(),
        revelations: (0..4)
            .map(|domain| graph::Revelation {
                domain,
                name: "Server revelation".into(),
                stages: [250, 500, 1000]
                    .into_iter()
                    .map(|minimum_points| graph::Stage {
                        minimum_points,
                        effects: Vec::new(),
                    })
                    .collect(),
            })
            .collect(),
        current: Vec::new(),
        draft: Vec::new(),
    }
}
impl Stepper for Live {
    fn wheel_selected(&self) -> bool {
        true
    }
    fn feature_selected(&self, c: u32) -> bool {
        c == 148
    }
    async fn wheel(&mut self, c: &WheelCommand) -> Result<WheelResponse, SessionError> {
        self.0.wheel(c).await
    }
    async fn feature(&mut self, c: &FeatureCommand) -> Result<GatedCommandResult, SessionError> {
        assert_eq!((c.capability, c.kind), (148, 6119));
        let r = graph::Request::decode(&c.payload).ok_or(SessionError::SessionUnusable)?;
        assert_eq!(r.wheel_revision, self.0.wheel.revision);
        assert_eq!(r.atelier_revision, self.0.atelier.revision);
        if self.0.wheel.revision == 5 && r.points == [0; 36] {
            assert_eq!(
                r.vessels[0],
                Some([17; 16]),
                "reset preview must use committed A, not unsaved B"
            );
        }
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply(r).encode().ok_or(SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        self.0.serve(d).await
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, d: StepDirection) -> Result<StepOutcome, SessionError> {
        self.0.step(d).await
    }
    async fn combat(&mut self, c: CombatCommand) -> Result<CombatReport, SessionError> {
        self.0.combat(c).await
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        self.0.logout().await
    }
}
async fn preview(link: &PlayLink) -> Result<(), Error> {
    query(
        &mut crate::play::mailbox::lock(&link.mailbox),
        &link.commands,
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        while diagram::current(&link.wheel_panel()).is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn wheel_presentation_actual_session_graph_selection_exact_save_and_reset() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let rt = oteryn_client_runtime::ClientRuntime::new()?;
            rt.block_on(async {
                let recorded = Arc::new(Mutex::new(Vec::new()));
                let peer = Live(Peer {
                    wheel: wheel::WheelAllocationReply {
                        refused: false,
                        revision: 4,
                        points: [0; 36],
                        vessels: None,
                    },
                    atelier: atelier::AtelierReply {
                        refused: false,
                        revision: 3,
                        gems: vec![
                            atelier::GemView {
                                id: [17; 16],
                                quality: 2,
                                domain: 0,
                                locked: false,
                                vessel: true,
                                modifiers: vec![31, 0, 5],
                            },
                            atelier::GemView {
                                id: [18; 16],
                                quality: 2,
                                domain: 0,
                                locked: false,
                                vessel: false,
                                modifiers: vec![31, 0, 5],
                            },
                        ],
                        grades: Vec::new(),
                        quality_quotes: Vec::new(),
                        grade_quotes: Vec::new(),
                    },
                    commands: recorded.clone(),
                });
                let (link, rx, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, rx, outbox));
                ready(&link, None).await?;
                link.wheel_request(Intent::Refresh);
                ready(&link, Some((4, 3))).await?;
                preview(&link).await?;
                let nonce = diagram::metadata(&link.wheel_panel())
                    .ok_or("metadata")?
                    .request
                    .nonce;
                let (x, y) = diagram::graph::center(14).ok_or("node")?;
                let index = diagram::graph::hit(x, y).ok_or("hit")?;
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Slot { nonce, index },
                );
                assert_eq!(link.wheel_panel().slot, 14);
                assert_eq!(
                    diagram::graph::rgba(&link.wheel_panel())
                        .ok_or("actual CPU graph")?
                        .len(),
                    384 * 384 * 4
                );
                assert!(
                    diagram::view::text(&link.wheel_panel()).contains("Server-configured perk")
                );
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Points(50),
                );
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Apply,
                );
                assert!(
                    link.wheel_panel().pending.is_none(),
                    "changed draft must await its exact server preview"
                );
                preview(&link).await?;
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Apply,
                );
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Apply,
                );
                ready(&link, Some((5, 3))).await?;
                assert_eq!(
                    link.wheel_panel()
                        .confirmed_wheel
                        .as_ref()
                        .ok_or("wheel")?
                        .points[14],
                    50
                );
                preview(&link).await?;
                {
                    let mut m = crate::play::mailbox::lock(&link.mailbox);
                    m.wheel.select_gem(Some([18; 16]));
                    assert!(m.wheel.socket_selected());
                    assert_eq!(
                        m.wheel.vessels[0],
                        Some([18; 16]),
                        "B is an unsaved vessel edit"
                    );
                    assert_eq!(
                        m.wheel.confirmed_vessels().ok_or("committed vessels")?[0],
                        Some([17; 16])
                    );
                }
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::ResetDraft,
                );
                assert_eq!(link.wheel_panel().vessels[0], Some([17; 16]));
                assert!(link.wheel_panel().pending.is_none());
                assert!(link.wheel_panel().diagram.reset_requested);
                preview(&link).await?;
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Apply,
                );
                ready(&link, Some((6, 3))).await?;
                let changes = recorded
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .iter()
                    .filter_map(|c| wheel::decode_wheel_request(&c.payload).ok())
                    .filter(|r| r.action != 0)
                    .collect::<Vec<_>>();
                assert_eq!(changes.len(), 2);
                assert_eq!((changes[0].action, changes[1].action), (1, 2));
                assert_eq!(
                    (changes[0].expected_revision, changes[1].expected_revision),
                    (4, 5)
                );
                assert_eq!(changes[0].points[14], 50);
                assert_eq!(changes[1].points, [0; 36]);
                assert_eq!(
                    changes[1].vessels, None,
                    "legacy reset preserves committed vessels"
                );
                assert_eq!(
                    link.wheel_panel()
                        .confirmed_vessels()
                        .ok_or("vessels after reset")?[0],
                    Some([17; 16])
                );
                for r in &changes {
                    assert_eq!(r.occurrence[6] >> 4, 7);
                }
                assert_ne!(changes[0].occurrence, changes[1].occurrence);
                crate::play::mailbox::lock(&link.mailbox).cancel_activity_requests();
                assert!(link.wheel_panel().diagram.reply.is_none());
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Wheel presentation session thread panicked")?
}
