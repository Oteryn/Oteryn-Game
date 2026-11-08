#[test]
fn atelier_controls_send_only_intent_and_current_revision() -> Result<(), Error> {
    for (intent, mutation) in [
        (Intent::Reveal, atelier::Mutation::Reveal { quality: 2 }),
        (
            Intent::SwitchDomain,
            atelier::Mutation::SwitchDomain { gem: [17; 16] },
        ),
        (
            Intent::Dismantle,
            atelier::Mutation::Dismantle { gem: [17; 16] },
        ),
        (
            Intent::GradeUp,
            atelier::Mutation::GradeUp { kind: 1, mod_id: 5 },
        ),
    ] {
        let mut panel = panel();
        panel.quality = 2;
        panel.modifier = Some((1, 5));
        let command = request(&mut panel, intent, occurrence(7)).ok_or("mutation")?;
        assert_eq!(command.kind, 424);
        assert_eq!(
            wire(atelier::decode_mutate(&command.payload))?,
            atelier::MutateRequest {
                occurrence: occurrence(7),
                expected_revision: 4,
                mutation,
            }
        );
    }
    let mut panel = panel();
    let command = request(&mut panel, Intent::ToggleLock, occurrence(6)).ok_or("lock")?;
    assert_eq!(command.kind, 309);
    assert_eq!(
        wire(atelier::decode_set_lock(&command.payload))?,
        atelier::SetLockRequest {
            occurrence: occurrence(6),
            expected_revision: 4,
            gem: [17; 16],
            locked: true,
        }
    );
    Ok(())
}
