//! Replay the live Team Finder commands through the native panel entry points.
use super::{DevClientSession, LeaderListing, TestResult, expect, tf};
use oteryn_dev_client::{
    client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse},
    client_ui::{team_finder as members, team_finder_leader as leader},
};

#[derive(Clone, Copy)]
pub(super) enum Action {
    Query(tf::TeamFinderQuery),
    Member(tf::MemberAction),
    Leader(Option<LeaderListing>),
}

struct Exchange {
    action: Action,
    command: ServerMechanicCommand,
    reply: ServerMechanicResponse,
}

/// Owned by one live connection. A relog always constructs an empty capture.
pub(super) struct Client {
    pub(super) wire: DevClientSession,
    exchanges: Vec<Exchange>,
    joined: (u64, u64),
    logged_out: Option<(u64, u64)>,
}

impl Client {
    pub(super) fn new(wire: DevClientSession) -> Self {
        Self {
            joined: (wire.next_command_id(), wire.last_server_sequence()),
            wire,
            exchanges: Vec::new(),
            logged_out: None,
        }
    }

    pub(super) async fn record(
        &mut self,
        action: Action,
        capability: u32,
        command_type: u32,
        payload: Vec<u8>,
    ) -> TestResult<ServerMechanicResponse> {
        let reply = self
            .wire
            .session_mut()
            .gated_command(capability, command_type, &payload)
            .await?;
        self.exchanges.push(Exchange {
            action,
            command: ServerMechanicCommand {
                capability,
                command_type,
                payload,
            },
            reply: reply.clone(),
        });
        Ok(reply)
    }

    pub(super) fn seal(&mut self, command_id: u64, server_sequence: u64) {
        self.logged_out = Some((command_id, server_sequence));
    }

    pub(super) fn replay(self, leader_name: &str) -> TestResult {
        let selected = self.wire.selected_capabilities();
        replay(
            self.exchanges,
            self.joined,
            self.logged_out
                .ok_or("step=native-replay;observed=missing-logout-bound")?,
            leader_name,
            selected.contains(&tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1),
            selected.contains(&leader::CAPABILITY_TEAM_FINDER_LEADER_V1),
        )
    }
}

fn replay(
    exchanges: Vec<Exchange>,
    (mut next_command_id, mut server_sequence): (u64, u64),
    logged_out: (u64, u64),
    leader_name: &str,
    members_selected: bool,
    leader_selected: bool,
) -> TestResult {
    let mut members_panel = members::Panel {
        selected: members_selected,
        ..Default::default()
    };
    let mut leader_panel = leader::Panel {
        selected: leader_selected,
        ..Default::default()
    };
    expect(!exchanges.is_empty(), "native-replay", "empty-capture")?;
    for exchange in exchanges {
        let reply = &exchange.reply;
        expect(
            reply.server_sequence > server_sequence && reply.command_id == next_command_id,
            "native-replay-sequence",
            (reply.server_sequence, reply.command_id),
        )?;
        server_sequence = reply.server_sequence;
        next_command_id = reply
            .command_id
            .checked_add(1)
            .ok_or("step=native-replay;observed=command-id-overflow")?;

        let command = match exchange.action {
            Action::Query(request) => members::query(
                &mut members_panel,
                matches!(request, tf::TeamFinderQuery::Applicants { .. }),
                false,
            ),
            Action::Member(action) => {
                let Some(tf::TeamFinderResult::Browse { listings, .. }) = &members_panel.result
                else {
                    return Err("step=native-replay;observed=no-browse-before-member-action".into());
                };
                members_panel.selected_row = listings
                    .iter()
                    .position(|row| row.name == leader_name)
                    .ok_or("step=native-replay;observed=leader-not-selectable")?;
                members::member(
                    &mut members_panel,
                    matches!(action, tf::MemberAction::Cancel { .. }),
                )
            }
            Action::Leader(listing) => leader::protocol::request(&mut leader_panel, listing),
        };
        expect(
            command.as_ref() == Some(&exchange.command),
            "native-replay-command",
            command,
        )?;

        match exchange.action {
            Action::Query(_) => {
                expect(
                    members::apply_result(&mut members_panel, reply),
                    "native-replay-page",
                    &members_panel.feedback,
                )?;
                let expected = tf::decode_result(&reply.payload)
                    .map_err(|error| format!("step=native-replay-decode;observed={error:?}"))?;
                let names_and_statuses: Vec<_> = match &expected {
                    tf::TeamFinderResult::Browse { listings, .. } => {
                        listings.iter().map(|row| (&row.name, row.status)).collect()
                    }
                    tf::TeamFinderResult::Applicants { members, .. } => {
                        members.iter().map(|row| (&row.name, row.status)).collect()
                    }
                };
                let visible = members_panel.visible_names();
                expect(
                    members_panel.result.as_ref() == Some(&expected)
                        && visible.len() == names_and_statuses.len()
                        && visible
                            .iter()
                            .zip(names_and_statuses)
                            .all(|(label, (name, status))| {
                                label.strip_prefix(name).is_some_and(|suffix| {
                                    suffix.starts_with(" — ")
                                        && suffix.ends_with(&format!("{status:?}"))
                                })
                            })
                        && members_panel.pending.is_none(),
                    "native-replay-visible-page",
                    &members_panel,
                )?;
            }
            Action::Member(_) => {
                let changed_page = members::apply_result(&mut members_panel, reply);
                expect(
                    reply.status == super::CommandStatus::Accepted
                        && reply.payload.is_empty()
                        && !changed_page
                        && members_panel.pending.is_none()
                        && members_panel.result.is_none(),
                    "native-replay-member",
                    &members_panel,
                )?;
            }
            Action::Leader(listing) => {
                leader::protocol::apply_result(&mut leader_panel, reply);
                expect(
                    reply.status == super::CommandStatus::Accepted
                        && reply.payload.is_empty()
                        && !leader_panel.pending
                        && leader_panel.confirmed == listing,
                    "native-replay-leader",
                    &leader_panel,
                )?;
            }
        }
    }
    expect(
        next_command_id == logged_out.0 && server_sequence < logged_out.1,
        "native-replay-logout-bound",
        (next_command_id, server_sequence, logged_out),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_rejects_empty_stale_malformed_or_truncated_captures() -> TestResult {
        assert!(replay(Vec::new(), (1, 0), (1, 3), "Leader", true, false).is_err());
        let action = tf::TeamFinderQuery::Browse { after: None };
        let command = ServerMechanicCommand {
            capability: tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
            command_type: tf::COMMAND_TYPE_TEAM_FINDER_QUERY,
            payload: tf::encode_query(action).map_err(|error| format!("{error:?}"))?,
        };
        let empty = tf::encode_result(&tf::TeamFinderResult::Browse {
            listings: Vec::new(),
            next: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let exchange = |payload| Exchange {
            action: Action::Query(action),
            command: command.clone(),
            reply: ServerMechanicResponse {
                command_id: 1,
                status: super::super::CommandStatus::Accepted,
                server_sequence: 2,
                payload,
            },
        };
        assert!(
            replay(
                vec![exchange(empty.clone())],
                (1, 2),
                (2, 3),
                "Leader",
                true,
                false
            )
            .is_err()
        );
        assert!(
            replay(
                vec![exchange(vec![])],
                (1, 1),
                (2, 3),
                "Leader",
                true,
                false
            )
            .is_err()
        );
        assert!(
            replay(
                vec![exchange(empty.clone()), exchange(empty.clone())],
                (1, 1),
                (3, 4),
                "Leader",
                true,
                false
            )
            .is_err()
        );
        assert!(
            replay(
                vec![exchange(empty.clone())],
                (1, 1),
                (3, 4),
                "Leader",
                true,
                false
            )
            .is_err()
        );
        replay(vec![exchange(empty)], (1, 1), (2, 3), "Leader", true, false)
    }
}
