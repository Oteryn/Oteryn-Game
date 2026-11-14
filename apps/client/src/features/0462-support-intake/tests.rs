use crate::support_delivery::{Evidence, EvidenceOwner};
#[test]
fn support_evidence_window_shows_server_data_only() {
    let mut owner = EvidenceOwner::default();
    owner.observe(Evidence::new(
        "approved-crash.txt".into(),
        6144,
        "exact owner evidence".into(),
    ));
    assert_eq!(owner.rows()[0].text, "exact owner evidence");
    assert!(!owner.rows()[0].pending);
}
#[test]
fn support_evidence_actions_are_bound_and_confirmed() {
    let mut owner = EvidenceOwner::default();
    owner.observe(Evidence::new(
        "approved-crash.txt".into(),
        6144,
        "exact evidence".into(),
    ));
    let expected = owner.rows()[0].clone();
    assert!(owner.submit(&expected, false, false, |_, _| true).is_none());
    assert!(owner.submit(&expected, true, false, |_, _| true).is_some());
    assert!(owner.rows()[0].pending);
    assert!(owner.rows()[0].result.is_none());
    assert!(owner.submit(&expected, true, false, |_, _| true).is_none());
}
#[test]
fn support_evidence_rejected_when_session_replaced_or_target_changed() {
    let mut owner = EvidenceOwner::default();
    let evidence = Evidence::new("approved-crash.txt".into(), 6144, "exact evidence".into());
    owner.observe(evidence.clone());
    assert!(owner.submit(&evidence, true, true, |_, _| true).is_none());
    let mut changed = evidence.clone();
    changed.text = "changed".into();
    assert!(owner.submit(&changed, true, false, |_, _| true).is_none());
    assert_eq!(owner.rows(), &[evidence]);
}

#[test]
fn support_evidence_real_dispatch_is_exact_and_closed_sessions_reject()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("u52-bound-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("approved-crash.txt");
    std::fs::write(&path, "exact evidence")?;
    let (link, mut requests, outbox) = crate::play::play_channel();
    let evidence = Evidence::new(path.clone(), 6144, "exact evidence".into());
    crate::support_delivery::with_owner(link.support_evidence_key(), &link.commands, |owner| {
        owner.observe(evidence.clone())
    });
    assert!(!link.support_evidence_submit(&evidence, false, false));
    assert!(!link.support_evidence_submit(&evidence, true, true));
    assert!(requests.try_recv().is_err());
    std::fs::write(&path, "changed target")?;
    assert!(!link.support_evidence_submit(&evidence, true, false));
    std::fs::write(&path, "exact evidence")?;
    assert!(link.support_evidence_submit(&evidence, true, false));
    assert!(!link.support_evidence_submit(&evidence, true, false));
    let crate::combat_input::PlayCommand::SupportDelivery(command, queued_path) =
        requests.try_recv()?
    else {
        return Err("support command missing".into());
    };
    assert_eq!(queued_path, path);
    assert_eq!(command.capability, 462);
    assert_eq!(
        oteryn_client_runtime::support_intake::decode_submission(&command.payload)
            .map_err(|_| "decode")?,
        "exact evidence"
    );
    assert!(requests.try_recv().is_err());
    let result = oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 2,
        payload: vec![1],
    };
    outbox.support_evidence_reply(&path, &command, &result);
    crate::support_delivery::with_owner(link.support_evidence_key(), &link.commands, |owner| {
        assert_eq!(owner.rows()[0].result, Some(result.clone()))
    });
    let (replacement, _replacement_requests, _replacement_outbox) = crate::play::play_channel();
    crate::support_delivery::with_owner(
        replacement.support_evidence_key(),
        &replacement.commands,
        |owner| owner.observe(evidence.clone()),
    );
    outbox.support_evidence_reply(&path, &command, &result);
    crate::support_delivery::with_owner(
        replacement.support_evidence_key(),
        &replacement.commands,
        |owner| {
            assert!(owner.rows()[0].result.is_none());
            assert!(!owner.rows()[0].pending);
        },
    );
    requests.close();
    assert!(!link.support_evidence_submit(&evidence, true, false));
    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn support_evidence_pending_preview_requires_consent_before_saving_or_sending()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("u52-pending-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let pending = directory.join("pending-crash.txt");
    let approved = directory.join("approved-crash.txt");
    std::fs::write(&pending, "retained crash evidence")?;
    let mut owner = EvidenceOwner::default();
    owner.refresh(&directory);
    let expected = owner.rows()[0].clone();
    assert!(expected.text.ends_with("\nretained crash evidence"));
    owner.refresh(&directory);
    assert_eq!(owner.rows(), std::slice::from_ref(&expected));
    assert!(owner.submit(&expected, false, false, |_, _| true).is_none());
    assert!(pending.exists());
    assert!(!approved.exists());
    let mut sent = None;
    assert!(
        owner
            .submit(&expected, true, false, |command, path| {
                sent = Some((command, path));
                true
            })
            .is_some()
    );
    let (command, path) = sent.ok_or("no delivery")?;
    assert_eq!(path, approved);
    assert_eq!(std::fs::read_to_string(&approved)?, expected.text);
    assert_eq!(
        oteryn_client_runtime::support_intake::decode_submission(&command.payload)
            .map_err(|_| "decode")?,
        expected.text
    );
    assert!(!pending.exists());
    owner.refresh(&directory);
    assert_eq!(
        owner.rows().len(),
        1,
        "approved file must not become a second request"
    );
    assert!(owner.rows()[0].pending);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
