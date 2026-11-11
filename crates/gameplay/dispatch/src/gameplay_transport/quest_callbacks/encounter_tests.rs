use super::*;

#[test]
fn engenc_personal_encounter_pending_requires_original_command_and_physical_fences()
-> Result<(), String> {
    let (mut runtime, actor, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(71);
    runtime
        .initialize_first_entry_position(actor)
        .map_err(|e| format!("{e:?}"))?;
    let expected = runtime
        .read_actor_position(actor)
        .map_err(|e| format!("{e:?}"))?;
    let command = |n| crate::foundation::CommandId::new(n).map_err(|e| format!("{e:?}"));
    let cause = QuestCause::Use(crate::foundation::CommandRef::new(session, command(7)?));
    let original = Pending {
        actor,
        session,
        expected,
        callback: "oteryn:quest-callback/engenc/asura/s2/entry".into(),
        key: 41,
        revision: 3,
        cause,
        origin: None,
        context: None,
        ingress_settled: false,
        participants: Vec::new(),
        flame: None,
        flame_effect_sent: false,
        projection_target: None,
    };
    let allowed = |pending: &Pending| {
        pending_encounter_matches(
            pending,
            actor,
            session,
            expected,
            &original.callback,
            original.key,
            original.revision,
            cause,
        )
    };
    assert!(allowed(&original));
    let mut changed = original.clone();
    changed.cause = QuestCause::Use(crate::foundation::CommandRef::new(session, command(8)?));
    assert!(
        !allowed(&changed),
        "another command cannot consume retained work"
    );
    let (_, _, foreign_session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(72);
    let reservation = runtime
        .reserve_fresh_session(foreign_session)
        .map_err(|e| format!("{e:?}"))?;
    let foreign = runtime
        .commit_fresh_session(reservation)
        .map_err(|e| format!("{e:?}"))?;
    changed = original.clone();
    changed.actor = foreign;
    assert!(!allowed(&changed));
    changed = original.clone();
    changed.session = foreign_session;
    assert!(!allowed(&changed));
    changed = original.clone();
    changed.revision += 1;
    assert!(!allowed(&changed));
    changed = original.clone();
    changed.key += 1;
    assert!(!allowed(&changed));
    changed = original.clone();
    changed.callback.push_str("-foreign");
    assert!(!allowed(&changed));
    Ok(())
}
