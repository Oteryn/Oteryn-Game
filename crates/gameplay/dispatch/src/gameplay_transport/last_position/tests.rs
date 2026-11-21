#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn world(seed: u8) -> WorldId {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        WorldId::decode(&bytes).expect("uuid v7")
    }

    fn stored(world_id: WorldId, frame: [u8; 32]) -> StoredLastPosition {
        StoredLastPosition {
            record: LastPositionRecord {
                rest: None,
                world_id,
                cell: LastPositionCell {
                    x: 5,
                    y: 6,
                    floor: 7,
                },
                coordinate_frame_digest: frame,
                map_revision_digest: [2; 32],
                server_artifact_digest: [3; 32],
                vitals: None,
            },
            key: crate::durability::character_last_position::LastPositionKey {
                character_lease_generation: 1,
                write_sequence: 1,
            },
        }
    }

    /// `CHARPOS0-RL-01`: the interval is within the registered 60..=300 seconds, so a changed
    /// position is written within 300 s.
    #[test]
    fn write_interval_is_within_the_registered_range() {
        assert!(LAST_POSITION_WRITE_INTERVAL <= Duration::from_secs(300));
        assert!(LAST_POSITION_WRITE_INTERVAL >= Duration::from_secs(60));
        // The writer wakes at the deadline itself; the slack is left for the pass.
        assert_eq!(NEXT_DUE + WRITE_PASS_SLACK, LAST_POSITION_WRITE_INTERVAL);
        assert!(WRITE_PASS_SLACK >= Duration::from_secs(30));
        assert!(WRITER_MIN_PAUSE < WRITE_PASS_SLACK && WRITER_MAX_SLEEP < WRITE_PASS_SLACK);
        assert!(!NEXT_DUE.is_zero());
    }

    /// `CHARPOS0-RL-01` under load: every tracked session of a Channel due at once is captured
    /// and written within the slack, each batch costing two passes bounded by the pass deadline,
    /// however slow the store is short of failing a pass. The bound is the reserved player
    /// slots, which exceed the listener's connection limit (Codex 4248578094).
    #[test]
    fn a_pass_of_every_tracked_session_fits_in_the_slack() {
        assert_eq!(MAX_TRACKED_SESSIONS, 500);
        assert_eq!(MAX_WRITE_BATCHES, 8);
        assert_eq!(MAX_BATCH_WORK, Duration::from_secs(4));
        assert!(MAX_WRITE_PASS <= WRITE_PASS_SLACK);
        assert!(MAX_WRITE_PASS < MAX_WRITE_LAG);
        assert!(MAX_WRITE_LAG <= WRITE_PASS_SLACK);
        assert!(NEXT_DUE + MAX_WRITE_LAG <= LAST_POSITION_WRITE_INTERVAL);
    }

    /// §3.2 "Order": each capture orders after every earlier one of the process.
    #[test]
    fn sequences_strictly_increase_from_the_wall_clock() {
        let positions = LastPositions::default();
        let first = positions.next_sequence();
        let second = positions.next_sequence();
        assert!(first > 1_600_000_000_000_000, "wall clock microseconds");
        assert!(second > first);
        positions.sequence.store(u64::MAX - 1, Ordering::Relaxed);
        assert_eq!(positions.next_sequence(), u64::MAX);
    }

    /// §3.3 "valid": another World or coordinate frame places at `entry_start`; another map or
    /// artifact digest is re-checked, not refused.
    #[test]
    fn a_stored_position_applies_only_in_its_world_and_frame() {
        let here = world(1);
        assert!(stored_position_applies(
            &stored(here, [1; 32]),
            here,
            [1; 32]
        ));
        assert!(!stored_position_applies(
            &stored(world(2), [1; 32]),
            here,
            [1; 32]
        ));
        assert!(!stored_position_applies(
            &stored(here, [9; 32]),
            here,
            [1; 32]
        ));
    }

    /// DEATH-RESTART-SAFETY-1 with §3.3 step 1: the in-session respawn upserts exactly the cell
    /// it places the actor at; any other cell refuses the placement.
    #[test]
    fn an_in_session_respawn_writes_only_the_cell_it_places() {
        let placed = FinalLastPosition {
            record: stored(world(1), [1; 32]).record,
            write_sequence: 1,
        };
        assert!(respawn_cell_recorded(
            MovementLocalPosition {
                x: 5,
                y: 6,
                floor: 7
            },
            &placed
        ));
        for other in [
            MovementLocalPosition {
                x: 4,
                y: 6,
                floor: 7,
            },
            MovementLocalPosition {
                x: 5,
                y: 5,
                floor: 7,
            },
            MovementLocalPosition {
                x: 5,
                y: 6,
                floor: 6,
            },
        ] {
            assert!(!respawn_cell_recorded(other, &placed), "{other:?}");
        }
    }

    /// Unchanged cells are not rewritten, and the interval gates the periodic write.
    #[test]
    fn tracks_are_due_after_the_interval_and_forgotten_on_retire() {
        let positions = LastPositions::default();
        let session = GameSessionId::decode(&{
            let mut bytes = [7; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        })
        .expect("uuid v7");
        let channel = crate::foundation::ChannelId::decode(session.as_bytes()).expect("uuid");
        let actor = ExactActorRef::transport_fixture(world(1), channel);
        positions.track(
            session,
            actor,
            None,
            CharacterRevision::new(1).expect("revision"),
        );
        let now = Instant::now();
        assert!(positions.due(now, false).is_empty());
        assert_eq!(positions.due(now, true).len(), 1);
        let deadline = positions.next_due().expect("tracked");
        assert!(deadline >= now + NEXT_DUE - Duration::from_secs(1));
        assert!(deadline <= Instant::now() + NEXT_DUE);
        assert_eq!(
            positions
                .due(now + NEXT_DUE + Duration::from_secs(1), false)
                .len(),
            1
        );
        let cell = LastPositionCell {
            x: 1,
            y: 2,
            floor: 7,
        };
        // The next deadline is measured from the capture, not from when its write settled.
        let captured = Instant::now();
        let saved = Saved {
            rest: None,
            cell,
            vitals: LastVitals::new(40, 3),
        };
        positions.settled(session, actor, Some(saved), captured);
        let track = positions.tracked(session).expect("tracked");
        assert_eq!(track.written, Some(saved));
        assert_eq!(track.due(), captured + NEXT_DUE);
        // An older capture never moves the deadline back.
        positions.settled(session, actor, None, now);
        assert_eq!(
            positions.tracked(session).map(|track| track.due()),
            Some(captured + NEXT_DUE)
        );
        // An in-session respawn's placement records its respawn cell.
        let respawn = LastPositionCell {
            x: 9,
            y: 9,
            floor: 7,
        };
        positions.respawned(session, respawn);
        assert_eq!(
            positions.tracked(session).and_then(|track| track.written),
            Some(Saved {
                rest: None,
                cell: respawn,
                vitals: None
            })
        );
        positions.forget(session);
        assert!(positions.tracked(session).is_none());
    }

    /// CHAR-VITALS-1: a periodic write is due when the cell or the vitals changed, and only
    /// then; a capture is compared with the cell and vitals of the row it last wrote or read.
    #[test]
    fn a_vitals_change_alone_makes_the_capture_changed() {
        let row = stored(world(1), [1; 32]).record;
        let mut wounded = row;
        wounded.vitals = LastVitals::new(40, 3);
        let mut moved = row;
        moved.cell.x += 1;
        assert_ne!(Saved::of(&wounded), Saved::of(&row));
        assert_ne!(Saved::of(&moved), Saved::of(&row));
        let mut healed = wounded;
        healed.vitals = LastVitals::new(41, 3);
        assert_ne!(Saved::of(&healed), Saved::of(&wounded));
        // The digests are the pinned content's; they never make a capture changed.
        let mut other_map = wounded;
        other_map.map_revision_digest = [9; 32];
        assert_eq!(Saved::of(&other_map), Saved::of(&wounded));
    }

    /// CHAR-VITALS-1: only a stored row's vitals are restored at admission; a respawn, a first
    /// login, a row without vitals and a row of another World start the actor at its maxima.
    /// In its World the vitals apply whatever the stored cell (another frame still restores
    /// them).
    #[test]
    fn only_a_stored_rows_vitals_are_restored() {
        let mut row = stored(world(1), [9; 32]);
        assert_eq!(
            admitted_vitals(&AdmissionPosition::Stored(row), world(1)),
            None
        );
        row.record.vitals = LastVitals::new(40, 0);
        assert_eq!(
            admitted_vitals(&AdmissionPosition::Stored(row), world(1)),
            Some((40, 0))
        );
        assert_eq!(
            admitted_vitals(&AdmissionPosition::Stored(row), world(2)),
            None
        );
        assert_eq!(
            admitted_vitals(&AdmissionPosition::FirstLogin, world(1)),
            None
        );
    }

    fn at(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    fn v7(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    /// The first-entry owner step's state: a runtime of `world(1)` whose pinned entry start is
    /// (3, 4, 7) and frame digest `[3; 32]`, with one committed player not yet positioned.
    fn admitted() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        use {
            crate::foundation::{ChannelId, NodeId},
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
        };
        let world_id = world(1);
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            ChannelId::decode(&v7(0x61)).expect("channel"),
            NodeId::decode(&v7(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::from_activation(
                world_id,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (3, 4, 7),
            ),
        )
        .expect("runtime");
        let session = GameSessionId::decode(&v7(0x21)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let player = runtime.commit_fresh_session(reservation).expect("commit");
        (runtime, player, session)
    }

    fn saved(world_id: WorldId, frame: [u8; 32], cell: MovementLocalPosition) -> AdmissionPosition {
        let mut row = stored(world_id, frame);
        row.record.cell = cell_of(cell);
        AdmissionPosition::Stored(row)
    }

    fn position(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> MovementLocalPosition {
        runtime
            .read_actor_position(actor)
            .expect("positioned")
            .position()
    }

    /// P20: a relocated login enters at the temple, keeps the row's vitals and rest of this
    /// World only; the admission commits the actual placement with citizenship.
    #[test]
    fn a_relocated_login_enters_at_the_temple_and_writes_it() {
        let temple = at(2, 11);
        let mut row = stored(world(1), [3; 32]);
        row.record.vitals = LastVitals::new(40, 3);
        row.record.rest = Some([1, 2, 3, 4]);
        let relocated = AdmissionPosition::Stored(row).relocated(temple, world(1));
        assert_eq!(
            first_entry_input(&relocated, world(1), [9; 32], |_| true),
            FirstEntryInput::Recorded(Some(temple))
        );
        assert_eq!(admitted_vitals(&relocated, world(1)), Some((40, 3)));
        assert_eq!(
            super::super::bed_regen::admitted_rest(&relocated, world(1)),
            Some([1, 2, 3, 4])
        );
        let mut other = stored(world(2), [3; 32]);
        other.record.vitals = LastVitals::new(40, 3);
        let other = AdmissionPosition::Stored(other).relocated(temple, world(1));
        assert_eq!(admitted_vitals(&other, world(1)), None);
        let first = AdmissionPosition::FirstLogin.relocated(temple, world(1));
        assert_eq!(
            first_entry_input(&first, world(1), [3; 32], |_| false),
            FirstEntryInput::Recorded(Some(temple))
        );
        let (mut runtime, player, session) = admitted();
        let input = first_entry_input(&relocated, world(1), [3; 32], |_| false);
        initialize_first_entry_at(&mut runtime, player, session, input, |_| true)
            .expect("first entry");
        assert_eq!(position(&runtime, player), temple);
    }

    #[test]
    fn relocation_overrides_pending_respawn_and_arrival_cells() {
        let temple = at(2, 11);
        let respawn = AdmissionPosition::Respawn(ConsumedRespawn {
            occurrence: crate::durability::character_death::PlayerDeathOccurrence::from_bytes(v7(
                0x31,
            ))
            .expect("death occurrence"),
            respawn_position: vec![1, 2, 3],
        });
        let vitals = LastVitals::new(40, 3);
        let rest = Some([1, 2, 3, 4]);
        let arrival = AdmissionPosition::Arrival(
            crate::durability::npc_travel::PendingArrival {
                occurrence: crate::durability::npc_travel::TravelOccurrence::from_bytes(v7(0x32))
                    .expect("travel occurrence"),
                world_id: world(1),
                destination: cell_of(at(14, 11)),
            },
            vitals,
            rest,
        );
        for (decided, expected_vitals, expected_rest) in
            [(respawn, None, None), (arrival, Some((40, 3)), rest)]
        {
            let relocated = decided.relocated(temple, world(1));
            assert_eq!(
                first_entry_input(&relocated, world(1), [3; 32], |_| false),
                FirstEntryInput::Recorded(Some(temple))
            );
            assert_eq!(admitted_vitals(&relocated, world(1)), expected_vitals);
            assert_eq!(
                super::super::bed_regen::admitted_rest(&relocated, world(1)),
                expected_rest
            );
        }
    }

    /// Codex 4249667405, §3.3 "it replaces `entry_start` as the input to
    /// `initialize_position`": a valid saved cell other than the entry start is the cell the
    /// first-entry owner step leaves the actor at, so the owners that run after that step (party
    /// presence, the familiar's login placement) read the saved cell, never `entry_start`.
    #[test]
    fn the_saved_cell_replaces_the_entry_start_in_the_first_entry_step() {
        let (mut runtime, player, session) = admitted();
        let decided = saved(world(1), [3; 32], at(20, 20));
        let input = first_entry_input(&decided, world(1), [3; 32], |_| false);
        assert_eq!(input, FirstEntryInput::Recorded(Some(at(20, 20))));
        let placed = initialize_first_entry_at(&mut runtime, player, session, input, |_| true)
            .expect("first entry");
        assert!(matches!(
            placed,
            FirstEntryPosition::Initialized(snapshot) if snapshot.position() == at(20, 20)
        ));
        assert_eq!(position(&runtime, player), at(20, 20));
        // An initialization that had already completed moves nothing.
        let again = initialize_first_entry_at(
            &mut runtime,
            player,
            session,
            FirstEntryInput::Recorded(Some(at(30, 30))),
            |_| true,
        )
        .expect("reconciled");
        assert!(matches!(again, FirstEntryPosition::Reconciled(_)));
        assert_eq!(position(&runtime, player), at(20, 20));
    }

    /// §3.3 "Occupied or blocked tile": the NPC-0 §6.1 fallback (north first), and with no
    /// admissible free cell within 3, step 4, the entry start; all within the owner step.
    #[test]
    fn a_blocked_saved_cell_uses_the_radius_fallback_then_the_entry_start() {
        let (mut runtime, player, session) = admitted();
        let input = FirstEntryInput::Recorded(Some(at(20, 20)));
        initialize_first_entry_at(&mut runtime, player, session, input, |cell| {
            cell != at(20, 20)
        })
        .expect("fallback");
        assert_eq!(position(&runtime, player), at(20, 19));
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(&mut runtime, player, session, input, |_| false)
            .expect("entry start");
        assert_eq!(position(&runtime, player), at(3, 4));
    }

    /// §3.3 "valid": a saved house tile goes straight to step 4, as do another World or frame
    /// and a first login (no row); the entry start is then the step's own input.
    #[test]
    fn a_house_tile_another_frame_or_no_row_enters_at_the_entry_start() {
        let house = |cell: MovementLocalPosition| cell == at(20, 20);
        for decided in [
            saved(world(1), [3; 32], at(20, 20)),
            saved(world(2), [3; 32], at(21, 21)),
            saved(world(1), [9; 32], at(21, 21)),
            AdmissionPosition::FirstLogin,
        ] {
            assert_eq!(
                first_entry_input(&decided, world(1), [3; 32], house),
                FirstEntryInput::EntryStart,
                "{decided:?}"
            );
        }
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(
            &mut runtime,
            player,
            session,
            FirstEntryInput::EntryStart,
            |_| true,
        )
        .expect("entry start");
        assert_eq!(position(&runtime, player), at(3, 4));
    }

    /// §3.3 step 1: a pending respawn's recorded cell is the first-entry input, even when it is
    /// a house tile; an undecodable record enters at step 4.
    #[test]
    fn a_pending_respawn_is_the_first_entry_input() {
        let occurrence =
            crate::durability::character_death::PlayerDeathOccurrence::from_bytes(v7(0x31))
                .expect("occurrence");
        let mut recorded = Vec::new();
        recorded.extend_from_slice(&5_i32.to_be_bytes());
        recorded.extend_from_slice(&6_i32.to_be_bytes());
        recorded.extend_from_slice(&7_i16.to_be_bytes());
        let pending = AdmissionPosition::Respawn(ConsumedRespawn {
            occurrence,
            respawn_position: recorded,
        });
        let input = first_entry_input(&pending, world(1), [3; 32], |_| true);
        assert_eq!(input, FirstEntryInput::Recorded(Some(at(5, 6))));
        // CHAR-VITALS-1: the respawned actor starts at its maxima (DEATH-2 §4.5).
        assert_eq!(admitted_vitals(&pending, world(1)), None);
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(&mut runtime, player, session, input, |_| true).expect("respawn");
        assert_eq!(position(&runtime, player), at(5, 6));
        let undecodable = AdmissionPosition::Respawn(ConsumedRespawn {
            occurrence,
            respawn_position: vec![1, 2, 3],
        });
        let input = first_entry_input(&undecodable, world(1), [3; 32], |_| false);
        assert_eq!(input, FirstEntryInput::Recorded(None));
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(&mut runtime, player, session, input, |_| true).expect("step 4");
        assert_eq!(position(&runtime, player), at(3, 4));
    }
}
