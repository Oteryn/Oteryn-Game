use super::*;
use crate::foundation::{
    ActorLocalGeneration, ActorLocalId, ActorRef, ChannelId, ExactActorRef, WorldId,
};
use oteryn_gameplay_content::content::ProductionKey;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(n: u8) -> [u8; 16] {
    [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
}

fn selected_boss() -> TestResult<SelectedRaid> {
    let catalogue = RaidCatalogue::embedded()?;
    let key = ProductionKey::new("raid:candia.sugarmommy")?;
    let selected = catalogue
        .get(&key)
        .ok_or("Sugar Mommy missing")?
        .select([7; 32], id(4))?;
    assert!(!selected.boss_entries.is_empty());
    Ok(selected)
}

fn status(wave: u16, entry: u16, index: u16, alive: bool) -> TestResult<RaidCreatureStatus> {
    let actor = ExactActorRef(ActorRef::from_owner_parts(
        WorldId::decode(&id(1))?,
        ChannelId::decode(&id(2))?,
        ScopeOwnershipGeneration::new(1)?,
        ActorLocalId(u32::from(index)),
        ActorLocalGeneration(1),
    ));
    Ok(RaidCreatureStatus {
        identity: RaidSpawnIdentity {
            firing_id: id(4),
            wave,
            entry,
            index,
        },
        outcome: RaidSpawnOutcome::Placed(actor),
        alive,
        boss: true,
    })
}

#[test]
fn raids_global_boss_end_requires_actual_death_and_all_terminal_requests() -> TestResult {
    let mut selected = selected_boss()?;
    let &(wave, entry) = selected.boss_entries.first().ok_or("boss entry")?;
    selected.definition.waves[usize::from(wave)].entries[usize::from(entry)].count = 2;
    assert_eq!(dead_bosses(&selected, &[]), None);
    let first = status(wave, entry, 0, false)?;
    assert_eq!(dead_bosses(&selected, &[first]), None);
    let mut second = status(wave, entry, 1, true)?;
    assert_eq!(dead_bosses(&selected, &[first, second]), None);
    second.alive = false;
    assert_eq!(
        dead_bosses(&selected, &[first, second]),
        Some(vec![
            creature_identity(first.identity),
            creature_identity(second.identity),
        ])
    );
    Ok(())
}

#[test]
fn raids_global_failed_boss_placement_is_not_a_boss_death() -> TestResult {
    let mut selected = selected_boss()?;
    let &(wave, entry) = selected.boss_entries.first().ok_or("boss entry")?;
    selected.definition.waves[usize::from(wave)].entries[usize::from(entry)].count = 2;
    let mut first = status(wave, entry, 0, false)?;
    let mut second = status(wave, entry, 1, false)?;
    first.outcome = RaidSpawnOutcome::Failed;
    second.outcome = RaidSpawnOutcome::Failed;
    assert_eq!(dead_bosses(&selected, &[first, second]), None);
    second = status(wave, entry, 1, false)?;
    assert_eq!(
        dead_bosses(&selected, &[first, second]),
        Some(vec![creature_identity(second.identity)])
    );
    Ok(())
}

#[test]
fn raids_global_pending_later_boss_wave_prevents_premature_end() -> TestResult {
    let mut selected = selected_boss()?;
    let &(wave, entry) = selected.boss_entries.first().ok_or("boss entry")?;
    selected.definition.waves[usize::from(wave)].entries[usize::from(entry)].count = 1;
    let next = u16::try_from(selected.definition.waves.len())?;
    selected
        .definition
        .waves
        .push(selected.definition.waves[usize::from(wave)].clone());
    selected.boss_entries.insert((next, entry));
    let first = status(wave, entry, 0, false)?;
    assert_eq!(dead_bosses(&selected, &[first]), None);
    let second = status(next, entry, 0, false)?;
    assert_eq!(
        dead_bosses(&selected, &[first, second]),
        Some(vec![
            creature_identity(first.identity),
            creature_identity(second.identity),
        ])
    );
    Ok(())
}

#[test]
fn raids_global_regular_wave_deaths_do_not_end_a_boss_raid() -> TestResult {
    let selected = selected_boss()?;
    let &(wave, entry) = selected.boss_entries.first().ok_or("boss entry")?;
    let mut ordinary = status(wave, entry, 0, false)?;
    ordinary.boss = false;
    assert_eq!(dead_bosses(&selected, &[ordinary]), None);
    let catalogue = RaidCatalogue::embedded()?;
    let badger = catalogue
        .get(&ProductionKey::new("raid:badger")?)
        .ok_or("Badger missing")?
        .select([7; 32], id(5))?;
    assert!(badger.boss_entries.is_empty());
    assert_eq!(dead_bosses(&badger, &[ordinary]), None);
    Ok(())
}

#[test]
fn raids_global_source_abort_suppresses_boss_cleanup_and_retains_its_proof_until_bosses_end()
-> TestResult {
    let selected = selected_boss()?;
    let &(wave, entry) = selected.boss_entries.first().ok_or("boss entry")?;
    let firing = id(4);
    let mut progress = RaidEffectProgress::default();
    progress.source_aborted.insert(firing);
    progress.settled_ends.insert(firing);
    assert!(!progress.boss_end_open(firing));
    let alive = status(wave, entry, 0, true)?;
    assert!(!progress.may_compact(firing, &selected, &[alive]));
    let dead = status(wave, entry, 0, false)?;
    assert!(dead_bosses(&selected, &[dead]).is_some());
    assert!(!progress.boss_end_open(firing));
    let mut survivor = status(wave + 1, 0, 0, true)?;
    survivor.boss = false;
    assert!(progress.may_compact(firing, &selected, &[dead, survivor]));
    Ok(())
}

#[test]
fn raids_global_partial_boss_death_keeps_evidence_but_all_failed_bosses_allow_compaction()
-> TestResult {
    let mut selected = selected_boss()?;
    let &(wave, entry) = selected.boss_entries.first().ok_or("boss entry")?;
    selected.definition.waves[usize::from(wave)].entries[usize::from(entry)].count = 2;
    let firing = id(4);
    let mut progress = RaidEffectProgress::default();
    let first = status(wave, entry, 0, false)?;
    let alive = status(wave, entry, 1, true)?;
    assert!(!progress.may_compact(firing, &selected, &[first]));
    progress.settled_ends.insert(firing);
    assert!(!progress.may_compact(firing, &selected, &[first, alive]));
    let mut failed = first;
    failed.outcome = RaidSpawnOutcome::Failed;
    let mut second_failed = alive;
    second_failed.outcome = RaidSpawnOutcome::Failed;
    second_failed.alive = false;
    assert!(progress.may_compact(firing, &selected, &[failed, second_failed]));
    assert!(progress.boss_end_open(firing));
    assert_eq!(dead_bosses(&selected, &[failed, second_failed]), None);
    Ok(())
}

#[test]
fn raids_global_death_between_observation_and_compaction_keeps_its_receipt() -> TestResult {
    let first = status(0, 0, 0, false)?;
    let second = status(0, 0, 1, true)?;
    let mut recorded = BTreeSet::from([first.identity]);
    assert!(!has_unrecorded_death(&[first, second], &recorded));
    let mut later_death = second;
    later_death.alive = false;
    assert!(has_unrecorded_death(&[first, later_death], &recorded));
    recorded.insert(later_death.identity);
    assert!(!has_unrecorded_death(&[first, later_death], &recorded));
    let mut failed = second;
    failed.outcome = RaidSpawnOutcome::Failed;
    failed.alive = false;
    assert!(!has_unrecorded_death(&[failed], &BTreeSet::new()));
    Ok(())
}

fn numbered_firing(number: usize) -> TestResult<[u8; 16]> {
    let mut firing = id(9);
    firing[..4].copy_from_slice(&u32::try_from(number)?.to_be_bytes());
    Ok(firing)
}

fn terminal_step(firing: [u8; 16], ordinal: u32, cleanup: bool) -> TestResult<CommittedRaidStep> {
    Ok(CommittedRaidStep {
        identity: RaidStepIdentity {
            firing_id: firing,
            channel: ChannelId::decode(&id(2))?,
            generation: ScopeOwnershipGeneration::new(1)?,
            ordinal,
        },
        action: CommittedRaidAction::Complete { cleanup },
        replayed: false,
    })
}

#[test]
fn raids_global_uncertain_terminal_ack_survives_a_missing_sql_page_and_reconciles_exact_body()
-> TestResult {
    let key = ProductionKey::new("raid:badger")?;
    let firing = id(4);
    let mut progress = RaidEffectProgress::default();
    progress.begin(firing, &key).map_err(|()| "begin")?;
    let terminal = terminal_step(firing, 3, false)?;
    progress.retain_terminal(&terminal).map_err(|()| "retain")?;
    // The terminal ACK committed, but the response or a later reconciliation
    // failed. SQL now returns no row; the next poll still owns the exact work.
    assert!(next_page_firing(&mut VecDeque::new(), &BTreeSet::new(), &progress).is_none());
    assert_eq!(progress.retry_work(), vec![(firing, key.clone())]);
    progress.begin(firing, &key).map_err(|()| "retry")?;
    assert_eq!(progress.terminal(firing), Some(terminal.clone()));
    let changed_body = terminal_step(firing, 3, true)?;
    assert!(progress.retain_terminal(&changed_body).is_err());
    assert_eq!(progress.terminal(firing), Some(terminal));
    // A persistent run may later receive a distinct boss-cleanup terminal.
    let boss_end = terminal_step(firing, 4, true)?;
    progress
        .retain_terminal(&boss_end)
        .map_err(|()| "later terminal")?;
    assert_eq!(progress.terminal(firing), Some(boss_end.clone()));
    assert!(
        progress
            .retain_terminal(&terminal_step(firing, 2, false)?)
            .is_err()
    );
    assert_eq!(progress.terminal(firing), Some(boss_end));
    Ok(())
}

#[test]
fn raids_global_retry_capacity_keeps_failed_identity_and_unprocessed_page_rows() -> TestResult {
    let key = ProductionKey::new("raid:badger")?;
    let mut progress = RaidEffectProgress::default();
    for number in 0..MAX_CANDIDATES {
        progress
            .begin(numbered_firing(number)?, &key)
            .map_err(|()| "fill")?;
    }
    let failed = numbered_firing(0)?;
    let terminal = terminal_step(failed, 1, true)?;
    progress.retain_terminal(&terminal).map_err(|()| "retain")?;
    let next = numbered_firing(MAX_CANDIDATES)?;
    assert!(progress.begin(next, &key).is_err());
    progress
        .begin(failed, &key)
        .map_err(|()| "existing retry at capacity")?;
    let mut page = VecDeque::from([(next, key.clone())]);
    assert!(next_page_firing(&mut page, &BTreeSet::new(), &progress).is_none());
    assert_eq!(page.front(), Some(&(next, key.clone())));
    assert_eq!(progress.terminal(failed), Some(terminal));
    // Reconciliation frees one slot, but never evicts the failed firing to
    // make room. The page resumes at the unprocessed row on the next poll.
    progress.retries.remove(&numbered_firing(1)?);
    assert_eq!(
        next_page_firing(&mut page, &BTreeSet::new(), &progress),
        Some((next, key.clone()))
    );
    progress.begin(next, &key).map_err(|()| "new row")?;
    assert_eq!(progress.retries.len(), MAX_CANDIDATES);
    assert!(progress.retries.contains_key(&failed));
    assert!(page.is_empty());
    Ok(())
}

#[test]
fn raids_global_saved_page_advances_after_retry_uses_part_of_the_poll_budget() -> TestResult {
    let key = ProductionKey::new("raid:badger")?;
    let mut progress = RaidEffectProgress::default();
    let failed = numbered_firing(0)?;
    progress.begin(failed, &key).map_err(|()| "begin")?;
    let mut page = VecDeque::new();
    for number in 1..=MAX_CANDIDATES {
        page.push_back((numbered_firing(number)?, key.clone()));
    }
    // One retry consumes one poll slot. Leave the final SQL row in the saved
    // page instead of re-fetching the same prefix and starving that row.
    let mut attempted = BTreeSet::from([failed]);
    for _ in 0..MAX_CANDIDATES - 1 {
        let (firing, _) = next_page_firing(&mut page, &attempted, &progress).ok_or("row")?;
        assert_ne!(firing, failed);
        attempted.insert(firing);
    }
    assert_eq!(page.len(), 1);
    let last = numbered_firing(MAX_CANDIDATES)?;
    assert_eq!(
        next_page_firing(&mut page, &BTreeSet::from([failed]), &progress),
        Some((last, key))
    );
    assert!(page.is_empty());
    Ok(())
}

#[test]
fn raids_global_completed_compaction_prunes_only_its_firing_and_keeps_retry_until_reconciliation()
-> TestResult {
    let key = ProductionKey::new("raid:badger")?;
    let firing = id(4);
    let other = id(5);
    let mut progress = RaidEffectProgress::default();
    for current in [firing, other] {
        progress.begin(current, &key).map_err(|()| "begin")?;
        progress
            .retain_terminal(&terminal_step(current, 1, true)?)
            .map_err(|()| "retain")?;
        progress.seen_dead.insert(RaidSpawnIdentity {
            firing_id: current,
            wave: 0,
            entry: 0,
            index: 0,
        });
        progress.closed_bosses.insert(current);
        progress.source_aborted.insert(current);
        progress.settled_ends.insert(current);
    }
    progress.completed(firing);
    assert!(
        progress
            .seen_dead
            .iter()
            .all(|identity| identity.firing_id == other)
    );
    assert_eq!(progress.closed_bosses, BTreeSet::from([other]));
    assert_eq!(progress.source_aborted, BTreeSet::from([other]));
    assert_eq!(progress.settled_ends, BTreeSet::from([other]));
    // The drive wrapper releases the retry entry only once every post-ACK
    // operation has returned successfully, not merely on ACK or compaction.
    assert!(progress.terminal(firing).is_some());
    progress.retries.remove(&firing);
    assert_eq!(progress.retry_work(), vec![(other, key)]);
    Ok(())
}
