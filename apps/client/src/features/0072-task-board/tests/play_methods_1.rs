impl Stepper for Peer {
    fn task_board_selected(&self) -> bool {
        true
    }
    async fn task_board(
        &mut self,
        command: &TaskBoardCommand,
    ) -> Result<TaskBoardResponse, SessionError> {
        let bytes = task_board::encode_task_board_request(&command.0)
            .map_err(|_| SessionError::SessionUnusable)?;
        let request = task_board::decode_task_board_request(&bytes)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(command.clone());
        let reply = task_board::TaskBoardReply {
            refused: request.action == 2,
            selected: 1,
            kills: 4,
            points: 12,
            bounty_points: 7,
            talisman_levels: [0; 4],
            talisman_bonuses: [0; 4],
            talisman_next_costs: [10; 4],
            tasks: vec![task_board::TaskBoardTask {
                race: "oteryn:dragon".into(),
                required: 20,
                claimed_at_unix: 0,
            }],
        };
        let payload = task_board::encode_task_board_reply(&reply)
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(TaskBoardResponse {
            status: CommandStatus::Accepted,
            reply: task_board::decode_task_board_reply(&payload)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        assert_eq!((command.capability, command.kind), (72, 89));
        let request = task_board::full::decode_request(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.full_commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request.clone());
        let mut reply = full_reply();
        reply.refusal = if request.action == 6 { 13 } else { 0 };
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: task_board::full::encode_reply(&reply)
                .map_err(|_| SessionError::SessionUnusable)?,
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
