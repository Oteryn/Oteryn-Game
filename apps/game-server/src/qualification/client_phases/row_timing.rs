//! Wall-clock accounting for qualification work. Explicit waits outrank setup
//! when nested inside admission, so the three categories never double count.
use std::cell::RefCell;
use std::future::Future;
use std::time::{Duration, Instant};

tokio::task_local! {
    static RUN: RefCell<Run>;
}

#[derive(Default)]
struct Run {
    row: Option<Row>,
    last_end: Option<(&'static str, Instant)>,
}

struct Row {
    name: &'static str,
    start: Instant,
    changed: Instant,
    elapsed: [Duration; 3],
    depth: [usize; 3],
}

impl Row {
    fn sample(&mut self, now: Instant) {
        let slot = self.depth.iter().rposition(|&depth| depth > 0).unwrap_or(0);
        self.elapsed[slot] += now.duration_since(self.changed);
        self.changed = now;
    }
}

pub(super) async fn run<T>(work: impl Future<Output = T>) -> T {
    RUN.scope(
        RefCell::new(Run {
            last_end: Some(("run-start", Instant::now())),
            ..Run::default()
        }),
        async {
            let result = work.await;
            RUN.with(|context| {
                if let Some((previous, end)) = context.borrow().last_end {
                    println!(
                        "NODE_BOOT_ROW_GAP after={previous} before=run-end s={:.3}",
                        end.elapsed().as_secs_f64()
                    );
                }
            });
            result
        },
    )
    .await
}

pub(super) struct RowGuard(bool);

pub(super) fn enter(name: &'static str) -> RowGuard {
    let started = RUN
        .try_with(|context| {
            let mut context = context.borrow_mut();
            if context.row.is_some() {
                return false;
            }
            let now = Instant::now();
            if let Some((previous, end)) = context.last_end {
                println!(
                    "NODE_BOOT_ROW_GAP after={previous} before={name} s={:.3}",
                    now.duration_since(end).as_secs_f64()
                );
            }
            context.row = Some(Row {
                name,
                start: now,
                changed: now,
                elapsed: [Duration::ZERO; 3],
                depth: [0; 3],
            });
            true
        })
        .unwrap_or(false);
    RowGuard(started)
}

impl Drop for RowGuard {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        let _ = RUN.try_with(|context| {
            let mut context = context.borrow_mut();
            if let Some(mut row) = context.row.take() {
                let now = Instant::now();
                row.sample(now);
                let [action, setup, wait] = row.elapsed;
                println!(
                    "NODE_BOOT_ROW_TIMING row={} total_s={:.3} wait_s={:.3} setup_s={:.3} action_s={:.3}",
                    row.name,
                    now.duration_since(row.start).as_secs_f64(),
                    wait.as_secs_f64(),
                    setup.as_secs_f64(),
                    action.as_secs_f64()
                );
                context.last_end = Some((row.name, now));
            }
        });
    }
}

struct Span(usize, bool);

impl Span {
    fn enter(slot: usize) -> Self {
        let entered = RUN
            .try_with(|context| {
                if let Some(row) = context.borrow_mut().row.as_mut() {
                    row.sample(Instant::now());
                    row.depth[slot] += 1;
                    return true;
                }
                false
            })
            .unwrap_or(false);
        Self(slot, entered)
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.1 {
            let _ = RUN.try_with(|context| {
                if let Some(row) = context.borrow_mut().row.as_mut() {
                    row.sample(Instant::now());
                    row.depth[self.0] -= 1;
                }
            });
        }
    }
}

pub(super) async fn setup<T>(work: impl Future<Output = T>) -> T {
    let _span = Span::enter(1);
    work.await
}

pub(super) async fn wait<T>(work: impl Future<Output = T>) -> T {
    let _span = Span::enter(2);
    work.await
}
