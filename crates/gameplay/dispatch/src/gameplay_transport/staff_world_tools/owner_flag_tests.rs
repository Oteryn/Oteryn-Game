#[test]
fn nx1_staff_world_proof_refuses_same_session_control_aba() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut runtime, actor, _, session) =
        crate::gameplay_transport::spell_presentations::tests::runtime();
    let character = crate::foundation::CharacterId::decode(session.as_bytes())?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            crate::foundation::CharacterLease::new(character, 1)?,
        )
        .map_err(|e| format!("{e:?}"))?;
    let proof = super::StaffActorProof {
        actor,
        session,
        control_revision: runtime
            .player_control_revision(actor, session)
            .map_err(|e| format!("{e:?}"))?,
    };
    assert!(proof.current(&runtime));
    let position = runtime
        .read_actor_position(actor)
        .map_err(|e| format!("{e:?}"))?;
    runtime
        .record_control_loss(
            actor,
            session,
            oteryn_gameplay_entities::carrier::ControlLossMark {
                epoch: 1,
                grace_deadline: 100,
            },
        )
        .map_err(|e| format!("{e:?}"))?;
    assert!(!proof.current(&runtime));
    runtime
        .restore_control(actor, session, 1)
        .map_err(|e| format!("{e:?}"))?;
    // Actor and session are unchanged, but the old pending-effect proof stays invalid.
    assert!(!proof.current(&runtime));
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(|e| format!("{e:?}"))?,
        position
    );
    let fresh = super::StaffActorProof {
        control_revision: runtime
            .player_control_revision(actor, session)
            .map_err(|e| format!("{e:?}"))?,
        ..proof
    };
    assert!(fresh.current(&runtime));
    assert_eq!(fresh.control_revision, proof.control_revision + 2);
    Ok(())
}
#[test]
fn nx1_staff_ghost_runtime_flags_change_revision_and_physical_occupancy()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut runtime, gm, other, session) =
        crate::gameplay_transport::spell_presentations::tests::runtime();
    let before = runtime
        .read_actor_position(gm)
        .map_err(|e| format!("{e:?}"))?;
    let other_at = runtime
        .read_actor_position(other)
        .map_err(|e| format!("{e:?}"))?
        .position();
    assert!(
        runtime
            .position_occupied_by_other(gm, other_at)
            .map_err(|e| format!("{e:?}"))?
    );
    runtime
        .set_staff_visibility(gm, session, true, false)
        .map_err(|e| format!("{e:?}"))?;
    let after = runtime
        .read_actor_position(gm)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(after.position(), before.position());
    assert!(after.revision() > before.revision());
    assert!(
        !runtime
            .position_occupied_by_other(gm, other_at)
            .map_err(|e| format!("{e:?}"))?
    );
    assert!(
        !runtime
            .position_occupied_by_other(other, before.position())
            .map_err(|e| format!("{e:?}"))?
    );
    assert_eq!(
        runtime
            .actor_staff_visibility(gm)
            .map_err(|e| format!("{e:?}"))?,
        (true, false, 1)
    );
    runtime
        .set_staff_visibility(gm, session, false, true)
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        runtime
            .position_occupied_by_other(gm, other_at)
            .map_err(|e| format!("{e:?}"))?
    );
    assert_eq!(
        runtime
            .actor_staff_visibility(gm)
            .map_err(|e| format!("{e:?}"))?,
        (false, true, 2)
    );
    assert!(
        runtime
            .set_staff_visibility(
                gm,
                runtime
                    .positioned_actor_census()
                    .map_err(|e| format!("{e:?}"))?
                    .into_iter()
                    .find_map(|(actor, _, session)| (actor == other).then_some(session).flatten())
                    .ok_or("other session")?,
                true,
                true
            )
            .is_err()
    );
    Ok(())
}
