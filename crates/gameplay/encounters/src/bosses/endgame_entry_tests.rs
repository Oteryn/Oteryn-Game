use super::*;
use std::future::{Future, ready};
use std::task::{Context, Poll, Waker};

fn fact(id: u8) -> EntryFacts {
    EntryFacts {
        character: [id; 16],
        character_revision: 9,
        level: 500,
        access: true,
        chain_ready: true,
    }
}
fn poll<T>(future: impl Future<Output = T>) -> Poll<T> {
    let mut future = std::pin::pin!(future);
    future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn endgame_entry_passes_all_participants_to_shared_owner() -> Result<(), EntryRefusal> {
    let encounter = Encounter::from_boss_key("oteryn:creature.goshnar_s_megalomania_green")
        .ok_or(EntryRefusal::InvalidInput)?;
    let participants = [fact(1), fact(2)];
    let called = std::cell::Cell::new(false);
    let actual = poll(enter(encounter, &participants, |qualified| {
        called.set(true);
        assert_eq!(qualified.encounter(), encounter);
        assert_eq!(qualified.participants(), participants);
        ready(Ok(37))
    }));
    assert_eq!(actual, Poll::Ready(Ok(37)));
    assert!(called.get());
    Ok(())
}

#[test]
fn endgame_entry_refusals_never_call_shared_owner() -> Result<(), EntryRefusal> {
    let final_boss =
        Encounter::from_boss_key("oteryn:creature.bakragore").ok_or(EntryRefusal::InvalidInput)?;
    let mut low = fact(2);
    low.level = 249;
    let mut no_access = fact(2);
    no_access.access = false;
    let mut no_chain = fact(2);
    no_chain.chain_ready = false;
    for (members, refusal) in [
        (vec![], EntryRefusal::InvalidInput),
        (vec![fact(1), fact(1)], EntryRefusal::InvalidInput),
        ((1..=6).map(fact).collect(), EntryRefusal::InvalidInput),
        (vec![fact(1), low], EntryRefusal::Level),
        (vec![fact(1), no_access], EntryRefusal::QuestAccess),
        (vec![fact(1), no_chain], EntryRefusal::BossChain),
    ] {
        let called = std::cell::Cell::new(false);
        let actual = poll(enter(final_boss, &members, |_| {
            called.set(true);
            ready(Ok(()))
        }));
        assert_eq!(actual, Poll::Ready(Err(refusal)));
        assert!(!called.get());
    }
    Ok(())
}

#[test]
fn endgame_entry_keeps_w02_content_unavailable_typed() -> Result<(), EntryRefusal> {
    let encounter = Encounter::from_boss_key("oteryn:creature.the_primal_menace")
        .ok_or(EntryRefusal::InvalidInput)?;
    assert_eq!(
        poll(enter(encounter, &[fact(1)], |_| ready::<
            Result<(), EntryRefusal>,
        >(Err(
            EntryRefusal::ContentUnavailable
        )))),
        Poll::Ready(Err(EntryRefusal::ContentUnavailable))
    );
    Ok(())
}

#[test]
fn endgame_entry_only_applies_sourced_quest_gates() -> Result<(), EntryRefusal> {
    let primal = Encounter::from_boss_key("oteryn:creature.the_primal_menace")
        .ok_or(EntryRefusal::InvalidInput)?;
    let mut member = fact(1);
    member.access = false;
    member.chain_ready = false;
    assert_eq!(
        poll(enter(primal, &[member], |_| ready(Ok(())))),
        Poll::Ready(Ok(()))
    );
    member.level = 499;
    assert_eq!(
        poll(enter(primal, &[member], |_| ready(Ok(())))),
        Poll::Ready(Err(EntryRefusal::Level))
    );
    let mini = Encounter::from_boss_key("oteryn:creature.goshnar_s_spite")
        .ok_or(EntryRefusal::InvalidInput)?;
    member.level = 250;
    member.access = true;
    assert_eq!(
        poll(enter(mini, &[member], |_| ready(Ok(())))),
        Poll::Ready(Ok(()))
    );
    assert!(Encounter::from_boss_key("Goshnar's Spite").is_none());
    assert!(Encounter::from_boss_key("oteryn:creature.goshnar_s_fake").is_none());
    assert!(Encounter::from_boss_key("oteryn:creature.echo_of_bakragore").is_none());
    Ok(())
}

#[test]
fn endgame_entry_forwards_canonical_rotten_lever_and_retry_policy() -> Result<(), EntryRefusal> {
    for (key, lever, retry_seconds) in [
        ("oteryn:creature.ichgahal", (32979, 32333, 15), 72_000),
        ("oteryn:creature.murcion", (32979, 32365, 15), 72_000),
        ("oteryn:creature.vemiath", (33079, 32333, 15), 72_000),
        ("oteryn:creature.chagorz", (33079, 32367, 15), 72_000),
        ("oteryn:creature.bakragore", (33015, 32392, 15), 244_800),
    ] {
        let encounter = Encounter::from_boss_key(key).ok_or(EntryRefusal::InvalidInput)?;
        let actual = poll(enter(encounter, &[fact(1)], |qualified| {
            assert_eq!(
                qualified.source_policy(),
                Some(SourceEntryPolicy {
                    lever,
                    retry_seconds
                })
            );
            // The same server-authored request reaches the shared owner. The
            // unqualified duplicate Bakragore lever is never a selected policy.
            assert_ne!(
                qualified.source_policy().map(|policy| policy.lever),
                Some((33079, 32398, 15))
            );
            ready(Ok(()))
        }));
        assert_eq!(actual, Poll::Ready(Ok(())));
    }
    Ok(())
}

#[test]
fn endgame_entry_forwards_primal_soul_policy_and_canonical_final_phase() -> Result<(), EntryRefusal>
{
    for (key, lever, retry_seconds) in [
        (
            "oteryn:creature.the_primal_menace",
            (33547, 32752, 14),
            72_000,
        ),
        (
            "oteryn:creature.goshnar_s_malice",
            (33678, 31599, 14),
            72_000,
        ),
        (
            "oteryn:creature.goshnar_s_spite",
            (33773, 31634, 14),
            72_000,
        ),
        (
            "oteryn:creature.goshnar_s_greed",
            (33775, 31665, 14),
            72_000,
        ),
        (
            "oteryn:creature.goshnar_s_hatred",
            (33772, 31601, 14),
            72_000,
        ),
        (
            "oteryn:creature.goshnar_s_cruelty",
            (33853, 31854, 6),
            72_000,
        ),
        (
            "oteryn:creature.goshnar_s_megalomania_purple",
            (33675, 31634, 14),
            259_200,
        ),
    ] {
        let encounter = Encounter::from_boss_key(key).ok_or(EntryRefusal::InvalidInput)?;
        assert_eq!(
            poll(enter(encounter, &[fact(1)], |qualified| {
                assert_eq!(
                    qualified.source_policy(),
                    Some(SourceEntryPolicy {
                        lever,
                        retry_seconds
                    })
                );
                ready(Ok(()))
            })),
            Poll::Ready(Ok(()))
        );
    }
    for alias in [
        "oteryn:creature.goshnar_s_megalomania_blue",
        "oteryn:creature.goshnar_s_megalomania_green",
    ] {
        let encounter = Encounter::from_boss_key(alias).ok_or(EntryRefusal::InvalidInput)?;
        assert_eq!(
            poll(enter(encounter, &[fact(1)], |qualified| {
                assert_eq!(
                    qualified.encounter().boss_key(),
                    "oteryn:creature.goshnar_s_megalomania_purple"
                );
                assert_eq!(
                    qualified.source_policy(),
                    Some(SourceEntryPolicy {
                        lever: (33675, 31634, 14),
                        retry_seconds: 259_200,
                    })
                );
                ready(Ok(()))
            })),
            Poll::Ready(Ok(()))
        );
    }
    Ok(())
}
