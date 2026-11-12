//! LOGDBG-C: the waiter side of holder contention and the connection trace on holder lines.

use super::*;

const TRACE: [u8; 16] = [
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
];
const TRACE_TEXT: &str = "00112233-4455-6677-8899-aabbccddeeff";

#[track_caller]
fn site() -> Site {
    std::panic::Location::caller()
}

fn current_thread() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()
}

#[test]
fn a_long_wait_reports_its_attempts_and_the_owner_that_refused_it_most() {
    let (waiter, owner, other) = (site(), site(), site());
    let start = Instant::now();
    let mut waits = Vec::new();
    note_wait_refused(&mut waits, waiter, Some(owner), start);
    note_wait_refused(&mut waits, waiter, Some(other), start + Duration::from_millis(200));
    note_wait_refused(&mut waits, waiter, Some(owner), start + Duration::from_millis(400));
    let end = start + Duration::from_millis(650);
    let wait = finish_wait(&mut waits, waiter, end).ok_or("a 650 ms wait is reported");
    assert!(waits.is_empty());
    let Ok(wait) = wait else {
        return;
    };
    let text = holder_wait_line(&wait, end.duration_since(wait.first)).render("p", 0);
    for expected in [
        "event=holder_wait".to_owned(),
        format!("waiter={waiter}"),
        "waited_ms=650".to_owned(),
        "attempts=3".to_owned(),
        format!("dominant_owner={owner}"),
    ] {
        assert!(text.contains(&expected), "{expected} missing in {text}");
    }
}

#[test]
fn a_short_or_abandoned_wait_is_not_reported() {
    let (waiter, owner) = (site(), site());
    let start = Instant::now();
    let mut waits = Vec::new();
    note_wait_refused(&mut waits, waiter, Some(owner), start);
    assert!(finish_wait(&mut waits, waiter, start + Duration::from_millis(499)).is_none());
    // Refused long ago, then nothing until a much later checkout: the waiter gave up.
    note_wait_refused(&mut waits, waiter, None, start);
    let later = start + HOLDER_WAIT_STALE + Duration::from_secs(1);
    assert!(finish_wait(&mut waits, waiter, later).is_none());
    // A refusal after a stale one starts a new wait.
    note_wait_refused(&mut waits, waiter, None, start);
    note_wait_refused(&mut waits, waiter, None, later);
    let fresh = finish_wait(&mut waits, waiter, later + Duration::from_millis(500));
    assert_eq!(fresh.map(|wait| wait.attempts), Some(1));
}

#[test]
fn refusals_tally_by_owner_and_an_unowned_holder_is_named_none() {
    let owner = site();
    let mut counts = Vec::new();
    assert_eq!(dominant(&counts), None);
    tally(&mut counts, None);
    tally(&mut counts, Some(owner));
    tally(&mut counts, None);
    assert_eq!(counts.len(), 2);
    assert_eq!(site_text(dominant(&counts).flatten()), "none");
    tally(&mut counts, Some(owner));
    tally(&mut counts, Some(owner));
    assert_eq!(site_text(dominant(&counts).flatten()), owner.to_string());
}

#[test]
fn the_trace_text_matches_the_transport_connection_trace() {
    assert_eq!(format_trace(TRACE), TRACE_TEXT);
}

#[test]
fn slow_and_stuck_lines_name_the_caller_and_held_time() {
    let caller = site();
    let slow = pass_slow_line(caller, Duration::from_millis(1_250), true).render("p", 0);
    for expected in [
        "event=pass_slow".to_owned(),
        format!("caller={caller}"),
        "held_ms=1250".to_owned(),
        "deadline_exceeded=true".to_owned(),
    ] {
        assert!(slow.contains(&expected), "{expected} missing in {slow}");
    }
    let stuck = holder_stuck_line(Some((caller, Duration::from_millis(5_001)))).render("p", 0);
    assert!(stuck.contains("event=holder_stuck") && stuck.contains("held_ms=5001"));
    let none = holder_stuck_line(None).render("p", 0);
    assert!(none.contains("caller=none") && !none.contains("held_ms"));
}

#[test]
fn a_dropped_pass_and_its_waiter_carry_the_issuing_connection_trace() -> Result<(), std::io::Error> {
    let runtime = current_thread()?;
    let slot = Arc::new(StdMutex::new(None));
    let waiter = site();
    let lines = oteryn_error_codes::capture(|| {
        runtime.block_on(with_holder_trace(TRACE, async {
            // A waiter refused 600 ms ago gets the holder now.
            if let Ok(mut waits) = HOLDER_WAITS.lock() {
                note_wait_refused(
                    &mut waits,
                    waiter,
                    None,
                    Instant::now() - Duration::from_millis(600),
                );
            }
            let owner = PassOwner::checked_out(&slot, waiter, Instant::now());
            assert!(
                slot.lock()
                    .is_ok_and(|held| held.is_some_and(|(_, _, trace)| trace == Some(TRACE)))
            );
            drop(owner);
        }));
    });
    let wait = lines.iter().find(|line| line.contains("event=holder_wait"));
    let dropped = lines.iter().find(|line| line.contains("event=pass_dropped"));
    for line in [wait, dropped] {
        assert!(
            line.is_some_and(|line| line.contains(&format!("trace={TRACE_TEXT}"))),
            "{lines:?}"
        );
    }
    Ok(())
}

#[test]
fn a_root_task_inherits_the_trace_of_the_task_that_spawned_it() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = current_thread()?;
    runtime.block_on(async {
        // Lazy: the fixture establishes no connection.
        let root = DurabilityRoot::connect_test_runtime("postgres://oteryn@127.0.0.1:1/oteryn")?;
        let traced =
            with_holder_trace(TRACE, async { root.spawn_task(async { holder_trace() }).await })
                .await?;
        assert_eq!(traced, Some(TRACE));
        assert_eq!(root.spawn_task(async { holder_trace() }).await?, None);
        Ok(())
    })
}
