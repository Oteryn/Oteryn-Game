#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Receipt(Option<CharacterRevision>);

    impl SequencedOutcome for Receipt {
        fn committed_revision(&self) -> Option<CharacterRevision> {
            self.0
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Failure {
        Mismatch,
        Unknown,
        Refused,
    }

    impl SequencedError for Failure {
        fn is_mismatch(&self) -> bool {
            *self == Self::Mismatch
        }
        fn is_unknown(&self) -> bool {
            *self == Self::Unknown
        }
        fn rejected() -> Self {
            Self::Refused
        }
        fn unavailable(_: DurabilityError) -> Self {
            Self::Unknown
        }
    }

    fn character(seed: u8) -> CharacterId {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        CharacterId::from_bytes(bytes).expect("UUIDv7 bytes")
    }

    fn revision(value: u64) -> CharacterRevision {
        CharacterRevision::new(value).expect("non-zero revision")
    }

    fn fence(character_id: CharacterId) -> CurrentCharacterGameplayFence {
        use crate::foundation::{
            ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
            ScopeOwnershipGeneration, WorldId,
        };
        let bytes = character_id.as_bytes();
        CurrentCharacterGameplayFence {
            character_id,
            game_session_id: GameSessionId::decode(bytes).expect("session"),
            connection_generation: ConnectionGeneration::new(1).expect("generation"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(bytes).expect("world"),
                ChannelId::decode(bytes).expect("channel"),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("generation"),
            expected_character_revision: revision(1),
        }
    }

    /// Both futures to completion, polled in turn on one task (no `tokio::join!` here).
    async fn join<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
        use std::task::Poll;
        let (mut a, mut b) = (std::pin::pin!(a), std::pin::pin!(b));
        let (mut left, mut right) = (None, None);
        std::future::poll_fn(|context| {
            if left.is_none()
                && let Poll::Ready(output) = a.as_mut().poll(context)
            {
                left = Some(output);
            }
            if right.is_none()
                && let Poll::Ready(output) = b.as_mut().poll(context)
            {
                right = Some(output);
            }
            match (left.take(), right.take()) {
                (Some(l), Some(r)) => Poll::Ready((l, r)),
                (l, r) => {
                    (left, right) = (l, r);
                    Poll::Pending
                }
            }
        })
        .await
    }

    /// The Character root of one test: the durable revision a write is fenced against.
    struct Durable(AtomicU64);

    impl Durable {
        fn load(
            &self,
        ) -> impl FnMut(
            CharacterId,
        )
            -> std::future::Ready<Result<CharacterRevision, CharacterAuthorityError>>
        + '_ {
            |_| std::future::ready(Ok(revision(self.0.load(Ordering::SeqCst))))
        }

        /// A revision-advancing write: commits one revision when `expected` is current, as the
        /// root row lock does.
        async fn write(&self, expected: CharacterRevision) -> Result<Receipt, Failure> {
            tokio::task::yield_now().await;
            let next = expected.get() + 1;
            self.0
                .compare_exchange(expected.get(), next, Ordering::SeqCst, Ordering::SeqCst)
                .map_err(|_| Failure::Mismatch)?;
            tokio::task::yield_now().await;
            Ok(Receipt(Some(revision(next))))
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(future)
    }

    #[test]
    fn concurrent_writers_of_one_character_commit_in_sequence_without_mismatch() {
        let durable = Durable(AtomicU64::new(7));
        let sequencer = CharacterRevisionSequencer::new();
        let id = character(1);
        let writer = || async {
            let mut slot = sequencer.acquire(id).await;
            slot.sequenced(
                durable.load(),
                fence(id),
                Expect::Cursor(OnMismatch::FailClosed),
                |fence| durable.write(fence.expected_character_revision),
            )
            .await
        };
        // An XP award and a charm command arriving together, interleaved at every await.
        let (xp, charm) = block_on(join(writer(), writer()));
        let mut committed = [xp.expect("XP commits"), charm.expect("charm commits")]
            .map(|receipt| receipt.0.expect("revision").get());
        committed.sort_unstable();
        assert_eq!(committed, [8, 9]);
        assert_eq!(durable.0.load(Ordering::SeqCst), 9);
    }

    #[test]
    fn unsequenced_writers_race_into_a_mismatch() {
        // The defect class the sequencer removes: both read revision 7, one loses.
        let durable = Durable(AtomicU64::new(7));
        let (first, second) =
            block_on(join(durable.write(revision(7)), durable.write(revision(7))));
        assert!(first.is_ok() != second.is_ok());
    }

    #[test]
    fn a_composition_holds_the_slot_for_its_chain() {
        let durable = Durable(AtomicU64::new(3));
        let sequencer = CharacterRevisionSequencer::new();
        let id = character(2);
        let interloper_saw = Rc::new(Cell::new(0));
        block_on(async {
            let chain = async {
                let mut slot = sequencer.acquire(id).await;
                let xp = slot
                    .sequenced(
                        durable.load(),
                        fence(id),
                        Expect::Cursor(OnMismatch::FailClosed),
                        |fence| durable.write(fence.expected_character_revision),
                    )
                    .await
                    .expect("XP commits");
                tokio::task::yield_now().await;
                let bestiary_expected = Cell::new(None);
                slot.sequenced(
                    durable.load(),
                    fence(id),
                    Expect::Cursor(OnMismatch::RetryOnce),
                    |fence| {
                        bestiary_expected.set(Some(fence.expected_character_revision));
                        durable.write(fence.expected_character_revision)
                    },
                )
                .await
                .expect("Bestiary commits");
                // Bestiary takes the revision XP committed.
                assert_eq!(bestiary_expected.get(), xp.0);
            };
            let interloper = async {
                tokio::task::yield_now().await;
                let _slot = sequencer.acquire(id).await;
                interloper_saw.set(durable.0.load(Ordering::SeqCst));
            };
            join(chain, interloper).await;
        });
        // The waiting request ran only after the whole chain.
        assert_eq!(interloper_saw.get(), 5);
    }

    #[test]
    fn a_bestiary_mismatch_reloads_the_cursor_and_retries_once() {
        let id = character(3);
        let sequencer = CharacterRevisionSequencer::new();
        let durable = Durable(AtomicU64::new(10));
        let attempts = Cell::new(0);
        let result = block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(9)); // a bypass moved the root under the cursor
            slot.sequenced(
                durable.load(),
                fence(id),
                Expect::Cursor(OnMismatch::RetryOnce),
                |fence| {
                    attempts.set(attempts.get() + 1);
                    durable.write(fence.expected_character_revision)
                },
            )
            .await
        });
        assert_eq!(attempts.get(), 2);
        assert_eq!(result, Ok(Receipt(Some(revision(11)))));

        // A second mismatch is not retried again.
        let attempts = Cell::new(0);
        let result = block_on(async {
            let mut slot = sequencer.acquire(id).await;
            slot.sequenced(
                durable.load(),
                fence(id),
                Expect::Cursor(OnMismatch::RetryOnce),
                |_| {
                    attempts.set(attempts.get() + 1);
                    std::future::ready(Err::<Receipt, _>(Failure::Mismatch))
                },
            )
            .await
        });
        assert_eq!((attempts.get(), result), (2, Err(Failure::Mismatch)));
    }

    #[test]
    fn a_revision_bound_mismatch_fails_closed_without_retry() {
        let id = character(4);
        let sequencer = CharacterRevisionSequencer::new();
        let durable = Durable(AtomicU64::new(10));
        let attempts = Cell::new(0);
        block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(9));
            let result = slot
                .sequenced(
                    durable.load(),
                    fence(id),
                    Expect::Cursor(OnMismatch::FailClosed),
                    |fence| {
                        attempts.set(attempts.get() + 1);
                        durable.write(fence.expected_character_revision)
                    },
                )
                .await;
            assert_eq!(result, Err(Failure::Mismatch));
            // The cursor is dropped, so the next write reloads it from the root.
            assert_eq!(*slot.cursor, None);
        });
        assert_eq!(attempts.get(), 1);
        assert_eq!(durable.0.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn a_replay_uses_its_original_revision_once() {
        let id = character(11);
        let sequencer = CharacterRevisionSequencer::new();
        let seen = Cell::new(None);
        let attempts = Cell::new(0);
        let result = block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(9));
            slot.sequenced(
                |_| std::future::ready(Ok(revision(9))),
                fence(id),
                Expect::Replay(revision(4)),
                |fence| {
                    attempts.set(attempts.get() + 1);
                    seen.set(Some(fence.expected_character_revision));
                    std::future::ready(Ok::<_, Failure>(Receipt(Some(revision(5)))))
                },
            )
            .await
        });
        assert_eq!(result, Ok(Receipt(Some(revision(5)))));
        assert_eq!((attempts.get(), seen.get()), (1, Some(revision(4))));
    }

    #[test]
    fn historical_replay_with_unknown_cursor_reloads_before_fresh_write() {
        let id = character(5);
        let sequencer = CharacterRevisionSequencer::new();
        block_on(async {
            let mut slot = sequencer.acquire(id).await;
            let replay = slot.sequenced(
                |_| std::future::ready(Ok(revision(10))),
                fence(id),
                Expect::Replay(revision(4)),
                |fence| {
                    assert_eq!(fence.expected_character_revision, revision(4));
                    std::future::ready(Ok::<_, Failure>(Receipt(Some(revision(5)))))
                },
            ).await;
            assert_eq!(replay, Ok(Receipt(Some(revision(5)))));
            let fresh = slot.sequenced(
                |_| std::future::ready(Ok(revision(10))),
                fence(id),
                Expect::Cursor(OnMismatch::FailClosed),
                |fence| std::future::ready(if fence.expected_character_revision == revision(10) {
                    Ok(Receipt(Some(revision(11))))
                } else {
                    Err(Failure::Mismatch)
                }),
            ).await;
            assert_eq!(fresh, Ok(Receipt(Some(revision(11)))));
        });
    }

    #[test]
    fn historical_party_receipt_keeps_unknown_cursor_unknown() {
        use crate::durability::character_progression::{CommittedExperienceAward, ExperienceRewardOccurrence};
        use oteryn_simulation_determinism::ExactI64;
        let id = character(5);
        let sequencer = CharacterRevisionSequencer::new();
        block_on(async {
            let mut slot = sequencer.acquire(id).await;
            slot.observe_party_experience(&Ok(ExperienceCommitOutcome::AlreadyCommitted(CommittedExperienceAward {
                occurrence: ExperienceRewardOccurrence::from_bytes(*id.as_bytes()).expect("occurrence"),
                character_id: id,
                original_character_revision: revision(4),
                committed_character_revision: revision(5),
                level_before: 1,
                level_after: 1,
                experience_before: ExactI64::new(0),
                experience_after: ExactI64::new(1),
                experience_awarded: ExactI64::new(1),
            })));
            assert_eq!(*slot.cursor, None);
        });
    }

    #[test]
    fn the_cursor_moves_only_on_receipts_and_never_back() {
        let id = character(5);
        let sequencer = CharacterRevisionSequencer::new();
        block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(6));
            slot.settle::<Receipt, Failure>(&Ok(Receipt(Some(revision(4)))));
            assert_eq!(*slot.cursor, Some(revision(6)), "a replayed older receipt");
            slot.settle::<Receipt, Failure>(&Ok(Receipt(None)));
            assert_eq!(*slot.cursor, Some(revision(6)), "nothing written");
            slot.settle::<Receipt, Failure>(&Err(Failure::Refused));
            assert_eq!(*slot.cursor, Some(revision(6)), "a refusal wrote nothing");
            slot.settle::<Receipt, Failure>(&Ok(Receipt(Some(revision(7)))));
            assert_eq!(*slot.cursor, Some(revision(7)));
            slot.settle::<Receipt, Failure>(&Err(Failure::Unknown));
            assert_eq!(*slot.cursor, None, "an unknown outcome");
        });
    }

    #[test]
    fn a_slot_refuses_another_characters_fence() {
        let sequencer = CharacterRevisionSequencer::new();
        let result = block_on(async {
            let mut slot = sequencer.acquire(character(6)).await;
            slot.sequenced(
                |_| std::future::ready(Ok(revision(1))),
                fence(character(7)),
                Expect::Cursor(OnMismatch::FailClosed),
                |_| std::future::ready(Ok::<_, Failure>(Receipt(None))),
            )
            .await
        });
        assert_eq!(result, Err(Failure::Refused));
    }

    #[test]
    fn idle_slots_are_dropped() {
        let sequencer = CharacterRevisionSequencer::new();
        block_on(async {
            drop(sequencer.acquire(character(8)).await);
            let _held = sequencer.acquire(character(9)).await;
            drop(sequencer.acquire(character(10)).await);
        });
        let slots = sequencer.slots.lock().expect("slots");
        assert!(slots.contains_key(&character(9)) && slots.len() <= 2);
    }
}
