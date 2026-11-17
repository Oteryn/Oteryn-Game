use crate::combat_input::{CombatCommand, CombatReport, PlayCommand};
use crate::play::PlayView;

impl PlayView {
    /// Shows the server's answer to a combat command.
    pub fn apply_combat(&mut self, report: CombatReport) {
        self.combat_line = report.text().map(oteryn_client_ui::localization::tr);
        self.action_line = None;
        self.logout_line = None;
    }
}

impl crate::play::Outbox {
    pub(in crate::play) async fn battle_run<T: crate::play::Stepper>(
        &self,
        session: &mut T,
        request: PlayCommand,
    ) -> Result<(), oteryn_session::SessionError> {
        let (command, panel_command) = match request {
            PlayCommand::Battle(command) => (command, Some(command)),
            PlayCommand::Combat(command) => (command, None),
            _ => return Err(oteryn_session::SessionError::SessionUnusable),
        };
        let wire = crate::play::mailbox::lock(&self.0).wire_combat(command);
        let report = session.combat(wire).await?;
        self.battle_result(report, session.battle_state(), panel_command);
        Ok(())
    }

    pub(in crate::play) fn battle_result(
        &self,
        report: CombatReport,
        state: Option<oteryn_session::ActorCombatState>,
        panel_command: Option<CombatCommand>,
    ) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        mailbox.battle.combat = state;
        mailbox.battle_feedback(report, panel_command);
        mailbox.combat_report(report);
    }

    pub(crate) fn battle_publish(
        &self,
        state: Option<oteryn_session::ActorCombatState>,
        own_floor: Option<i16>,
        entities: &[oteryn_session::WorldSpatialEntity],
    ) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        mailbox.battle.combat = state;
        let frame = mailbox.wire_frame();
        let entities = mailbox.native_entities(entities);
        mailbox
            .battle
            .replace_entities(&entities, own_floor.map(|floor| frame.floor_to_native(floor)));
    }
}

impl crate::play::mailbox::Mailbox {
    /// Cursor-aimed casts name their tile in the wire frame.
    fn wire_combat(&self, command: CombatCommand) -> CombatCommand {
        let wire = |position: oteryn_session::SpellTargetPosition| {
            let wire = self.wire_position(oteryn_session::ActorPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            });
            oteryn_session::SpellTargetPosition {
                x: wire.x,
                y: wire.y,
                floor: wire.floor,
            }
        };
        match command {
            CombatCommand::CastCursor {
                spell,
                position,
                direction,
            } => CombatCommand::CastCursor {
                spell,
                position: wire(position),
                direction,
            },
            CombatCommand::CastAt { spell, position } => CombatCommand::CastAt {
                spell,
                position: wire(position),
            },
            other => other,
        }
    }

    pub(in crate::play) fn battle_feedback(
        &mut self,
        report: CombatReport,
        panel_command: Option<CombatCommand>,
    ) {
        if matches!(report, CombatReport::Cast(_)) {
            return;
        }
        let matching_kind = matches!(
            (panel_command, report),
            (Some(CombatCommand::Attack(_)), CombatReport::Attack(_))
                | (Some(CombatCommand::Stop), CombatReport::Stop(_))
                | (Some(CombatCommand::Modes(_)), CombatReport::Modes(_))
        );
        if matching_kind && self.battle.pending_command == panel_command {
            self.battle.pending = false;
            self.battle.pending_command = None;
        }
        self.battle.feedback = report.text().map(oteryn_client_ui::localization::tr);
    }

    pub(in crate::play) fn battle_end(&mut self) {
        self.battle.shutdown = true;
        self.battle.open = false;
    }
}
