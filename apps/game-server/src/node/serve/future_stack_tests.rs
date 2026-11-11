use super::*;

// Determine the actual production future type without constructing or polling it. This
// neither loads configuration nor starts a runtime, socket, database or scheduler job.
fn future_state_bytes<A, F: Future>(_: impl FnOnce(A) -> F) -> usize {
    std::mem::size_of::<F>()
}

#[test]
fn node_control_futures_leave_stack_for_gameplay() {
    // Each raw control state gets at most 1/64 of the existing 64 MiB node stack.
    // This bounds inlined state growth; runtime qualification still checks the poll chain.
    const MAX_CONTROL_FUTURE_BYTES: usize = 1 << 20;
    let entry = future_state_bytes(|()| {
        run_with_npc_data_project(Path::new("unread-node-config.toml"), None)
    });
    let supervisor = future_state_bytes::<supervision::Serving<'static, 'static, 'static>, _>(
        supervision::supervise,
    );
    for (name, bytes) in [("node entry", entry), ("supervisor", supervisor)] {
        assert!(
            bytes <= MAX_CONTROL_FUTURE_BYTES,
            "{name} retains {bytes} bytes; child jobs must not grow the inline control state"
        );
    }
}

#[test]
fn joined_jobs_keep_poll_and_cancellation_ownership() {
    use std::cell::{Cell, RefCell};
    use std::pin::Pin;
    use std::task::{Context, Waker};

    struct Child<'a> {
        id: u8,
        ready: &'a Cell<bool>,
        polls: &'a RefCell<Vec<u8>>,
        drops: &'a Cell<u8>,
        completed: bool,
    }
    impl Future for Child<'_> {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            assert!(!self.completed, "a finished child must not be polled again");
            self.polls.borrow_mut().push(self.id);
            if self.ready.get() {
                self.completed = true;
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
    impl Drop for Child<'_> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    for finish_peer in [false, true] {
        let ready = [Cell::new(true), Cell::new(false)];
        let polls = RefCell::new(Vec::new());
        let drops = Cell::new(0);
        let child = |id: u8| Child {
            id,
            ready: &ready[usize::from(id)],
            polls: &polls,
            drops: &drops,
            completed: false,
        };
        let mut joined = Box::pin(both(child(0), child(1)));
        assert!(polls.borrow().is_empty(), "construction must not poll jobs");
        let mut context = Context::from_waker(Waker::noop());
        assert!(joined.as_mut().poll(&mut context).is_pending());
        assert_eq!(*polls.borrow(), [0, 1]);
        assert_eq!(
            drops.get(),
            0,
            "the completed child's ownership is retained"
        );
        if finish_peer {
            ready[1].set(true);
            assert!(joined.as_mut().poll(&mut context).is_ready());
            assert_eq!(*polls.borrow(), [0, 1, 1]);
        }
        drop(joined);
        assert_eq!(drops.get(), 2, "completion or cancellation drops both jobs");
    }
}
