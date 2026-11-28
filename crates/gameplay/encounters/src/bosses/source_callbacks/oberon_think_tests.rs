use super::*;

#[test]
fn oberon_debates_at_red_health_and_accepts_only_the_matching_reply() {
    let mut owner = OberonThinkOwner::default();
    assert_eq!(owner.think(1, 3001, 30000, |n| n), Ok(None));
    assert_eq!(owner.limit_damage(30000, 30000, 30000), 27000);
    assert_eq!(
        owner.think(1, 3000, 30000, |_| 0),
        Ok(Some(OberonPhase {
            taunt: OBERON_DEBATE[0].0,
            summon: OBERON_SUMMONS[0],
        }))
    );
    assert_eq!(owner.limit_damage(30000, 30000, 700), 0);
    assert_eq!(owner.reply(OBERON_DEBATE[1].1), Some("HAHAHAHA!"));
    assert_eq!(owner.limit_damage(30000, 30000, 700), 0);
    let hotkey = format!("{} {}", OBERON_DEBATE[1].1, OBERON_DEBATE[0].1);
    assert_eq!(owner.reply(&hotkey), Some("GRRRAAANNGH!"));
    assert_eq!(owner.limit_damage(30000, 30000, 700), 700);
    assert_eq!(owner.reply(&hotkey), None);
}

#[test]
fn oberon_remains_vulnerable_after_the_third_correct_reply() -> Result<(), OberonRefusal> {
    let mut owner = OberonThinkOwner::default();
    for (sequence, index) in [(1, 0), (2, 1), (3, 2)] {
        let phase = owner.think(sequence, 3000, 30000, |_| 0)?;
        assert_eq!(phase.map(|phase| phase.taunt), Some(OBERON_DEBATE[index].0));
        assert_eq!(owner.think(1, 3000, 30000, |n| n), Ok(None));
        assert_eq!(owner.reply(OBERON_DEBATE[index].1), Some("GRRRAAANNGH!"));
        assert_eq!(owner.think(sequence, 3000, |n| n), Ok(None));
    }
    assert_eq!(owner.think(1, 3000, 30000, |n| n), Ok(None));
    assert_eq!(owner.limit_damage(3000, 30000, 5000), 5000);
    Ok(())
}

#[test]
fn invalid_draws_preserve_the_current_phase() {
    let mut owner = OberonThinkOwner::default();
    assert_eq!(
        owner.think(1, 3000, 30000, |n| n),
        Err(OberonRefusal::InvalidDraw)
    );
    assert_eq!(
        owner.think(1, 3000, 30000, |n| if n == 2 { n } else { 0 }),
        Err(OberonRefusal::InvalidDraw)
    );
    assert_eq!(owner, OberonThinkOwner::default());
}
