// Transport-only relocation proof; reuse the existing connection driver and step fixture.
struct RelocatingNpcAuthority {
    ordinary: StepAuthority,
    moved: std::cell::Cell<bool>,
}

impl FreshAdmissionAuthority for RelocatingNpcAuthority {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.ordinary.admit(attempt).await
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        if self.moved.get() {
            Some(at(32472))
        } else {
            self.ordinary.observe(actor).await
        }
    }

    async fn npc_talk(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: NpcTalkIntent,
    ) -> super::super::npc_command::NpcTalkOutcome {
        self.moved.set(true);
        super::super::npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Ok)
    }
}

#[test]
fn tutorial_npc_relocation_sends_the_actual_spatial_delta_after_the_talk_result()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = RelocatingNpcAuthority {
            ordinary: StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            },
            moved: std::cell::Cell::new(false),
        };
        let mut admitted = positioned()?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            super::super::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[3],
        )
        .ok_or("selection")?;
        let intent = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: WireEntityRef {
                identity: Default::default(),
                generation: 1,
            },
            text: "yes".into(),
        })
        .map_err(|error| format!("{error:?}"))?;
        let mut payload = Vec::new();
        scalar(&mut payload, 1, 1);
        scalar(&mut payload, 2, u64::from(COMMAND_TYPE_NPC_TALK_INTENT));
        bytes(&mut payload, 4, &intent);
        let (end, frames) =
            drive_session(&authority, admitted, &[envelope(7, 1, &payload)]).await?;
        let mut saw_result = false;
        let mut relocated = false;
        for frame in frames {
            let wire = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
            if wire.message_type() == MessageType::CommandResult {
                saw_result = true;
            } else if wire.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(wire.payload())?;
                if delta.domain_id == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                    assert!(saw_result);
                    assert_eq!(delta.payload, encode_world_spatial(&at(32472)));
                    relocated = true;
                }
            }
        }
        assert!(relocated);
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        Ok(())
    })
}
