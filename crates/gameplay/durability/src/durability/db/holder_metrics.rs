use super::{DIAGNOSTIC_PROCESS, HOLDER_USAGE_WINDOW, Site, millis};
use oteryn_error_codes::{FieldValue, Level, Line};
use sqlx::postgres::PgQueryProfile;
use std::collections::HashMap;
use std::future::{Future, poll_fn};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(in crate::durability) enum Tag {
    Admission(Site),
    CharacterFence {
        operation: &'static str,
        caller: Site,
    },
    Soulpit(&'static str),
}

impl Tag {
    fn text(self) -> String {
        match self {
            Self::Admission(site) => format!("admission_journal:{site}"),
            Self::CharacterFence { operation, caller } => format!("{operation}:{caller}"),
            Self::Soulpit(driver) => format!("active_soulpits:{driver}"),
        }
    }
}

/// The checkout decision that blocked this pass. The public error stays RootUnavailable.
#[derive(Clone, Copy)]
pub(super) enum Refusal {
    QueueLock,
    OlderTicket,
    ForegroundPriority,
    PriorityLock,
    HolderBusy,
    NoLiveHolder,
}

pub(super) struct Hold {
    tag: Tag,
    checkout: Duration,
    profile: Arc<PgQueryProfile>,
    other_await: Duration,
    pub(super) return_time: Duration,
    pub(super) dispatch: Duration,
}

impl Hold {
    pub(super) fn new(tag: Tag, checkout: Duration) -> Self {
        Self {
            tag,
            checkout,
            profile: Arc::default(),
            other_await: Duration::ZERO,
            return_time: Duration::ZERO,
            dispatch: Duration::ZERO,
        }
    }

    pub(super) async fn observe<T>(&mut self, future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut pending_outside_sql = None;
        poll_fn(|cx| {
            if let Some(since) = pending_outside_sql.take() {
                self.other_await += Instant::now().duration_since(since);
            }
            let result = self.profile.in_poll(|| future.as_mut().poll(cx));
            if result.is_pending() && !self.profile.has_pending_statement() {
                pending_outside_sql = Some(Instant::now());
            }
            result
        })
        .await
    }
}

#[derive(Default)]
struct Usage {
    passes: u64,
    refused: [u64; 6],
    held: Duration,
    max: Duration,
    checkout: Duration,
    sql: Duration,
    statements: u64,
    other_await: Duration,
    return_time: Duration,
    dispatch: Duration,
    retries_on_passes: u64,
    retried_passes: u64,
    max_retries_per_pass: u64,
    retry_intervals: u64,
    retry_delay: Duration,
    min_retry_delay: Option<Duration>,
    max_retry_delay: Duration,
}

impl Usage {
    fn line(&self, event: &'static str, tag: Tag) -> Line<'static> {
        Line::new(Level::Info, "durability", event)
            .field("tag", FieldValue::Owned(tag.text()))
            .field("passes", FieldValue::U64(self.passes))
            .field("refused", FieldValue::U64(self.refused.iter().sum()))
            .field("held_ms", millis(self.held))
            .field("max_ms", millis(self.max))
    }

    fn window_line(&self, tag: Tag, elapsed: Duration) -> Line<'static> {
        let [
            queue_lock,
            older_ticket,
            foreground_priority,
            priority_lock,
            holder_busy,
            no_live_holder,
        ] = self.refused;
        let measured = self.sql + self.other_await + self.return_time + self.dispatch;
        self.line("holder_tag_window", tag)
            .field("window_ms", millis(elapsed))
            .field("wait_for_holder_ms", millis(self.checkout))
            .field("sql_ms", millis(self.sql))
            .field("sql_count", FieldValue::U64(self.statements))
            .field("other_await_ms", millis(self.other_await))
            .field("return_ms", millis(self.return_time))
            .field("dispatch_ms", millis(self.dispatch))
            .field("work_ms", millis(self.held.saturating_sub(measured)))
            .field("refused_queue_lock", FieldValue::U64(queue_lock))
            .field("refused_older_ticket", FieldValue::U64(older_ticket))
            .field(
                "refused_foreground_priority",
                FieldValue::U64(foreground_priority),
            )
            .field("refused_priority_lock", FieldValue::U64(priority_lock))
            .field("refused_holder_busy", FieldValue::U64(holder_busy))
            .field("refused_no_live_holder", FieldValue::U64(no_live_holder))
            .field("retried_passes", FieldValue::U64(self.retried_passes))
            .field("retries_on_passes", FieldValue::U64(self.retries_on_passes))
            .field(
                "max_retries_per_pass",
                FieldValue::U64(self.max_retries_per_pass),
            )
            .field("retry_intervals", FieldValue::U64(self.retry_intervals))
            .field("retry_delay_ms", millis(self.retry_delay))
            .field(
                "retry_delay_min_us",
                micros(self.min_retry_delay.unwrap_or_default()),
            )
            .field("retry_delay_max_us", micros(self.max_retry_delay))
    }
}

fn micros(duration: Duration) -> FieldValue<'static> {
    FieldValue::U64(u64::try_from(duration.as_micros()).unwrap_or(u64::MAX))
}

#[derive(Default)]
struct TagUsage {
    window: Usage,
    run: Usage,
    dirty: bool,
}

struct Report {
    since: Instant,
    by_tag: HashMap<Tag, TagUsage>,
}

static REPORT: LazyLock<Mutex<Report>> = LazyLock::new(|| {
    Mutex::new(Report {
        since: Instant::now(),
        by_tag: HashMap::new(),
    })
});

fn update(tag: Tag, apply: impl Fn(&mut Usage)) {
    let Ok(mut report) = REPORT.lock() else {
        return;
    };
    let entry = report.by_tag.entry(tag).or_default();
    apply(&mut entry.window);
    apply(&mut entry.run);
    entry.dirty = true;
    let now = Instant::now();
    if now.duration_since(report.since) >= HOLDER_USAGE_WINDOW {
        let (elapsed, window) = take_window(&mut report, now);
        drop(report);
        emit_window(elapsed, window);
    }
}

fn take_window(report: &mut Report, now: Instant) -> (Duration, Vec<(Tag, Usage)>) {
    let elapsed = now.duration_since(report.since);
    report.since = now;
    let window = report
        .by_tag
        .iter_mut()
        .filter_map(|(&tag, entry)| {
            if !std::mem::take(&mut entry.dirty) {
                return None;
            }
            Some((tag, std::mem::take(&mut entry.window)))
        })
        .collect();
    (elapsed, window)
}

fn emit_window(elapsed: Duration, window: Vec<(Tag, Usage)>) {
    for (tag, usage) in window {
        oteryn_error_codes::emit(DIAGNOSTIC_PROCESS, &usage.window_line(tag, elapsed));
    }
}

pub(super) fn summarize() {
    let Ok(mut report) = REPORT.lock() else {
        return;
    };
    let (elapsed, window) = take_window(&mut report, Instant::now());
    let summary: Vec<_> = report
        .by_tag
        .iter()
        .map(|(&tag, entry)| entry.run.line("holder_tag_summary", tag))
        .collect();
    drop(report);
    emit_window(elapsed, window);
    for line in summary {
        oteryn_error_codes::emit(DIAGNOSTIC_PROCESS, &line);
    }
}

pub(super) fn refused(tag: Tag, elapsed: Duration, refusal: Refusal) {
    update(tag, |usage| {
        usage.checkout += elapsed;
        usage.refused[refusal as usize] += 1;
    });
}

pub(super) fn released(hold: &Hold, elapsed: Duration) {
    let (statements, sql) = hold.profile.totals();
    update(hold.tag, |usage| {
        usage.passes += 1;
        usage.checkout += hold.checkout;
        usage.held += elapsed;
        usage.max = usage.max.max(elapsed);
        usage.statements += statements;
        usage.sql += sql;
        usage.other_await += hold.other_await;
        usage.return_time += hold.return_time;
        usage.dispatch += hold.dispatch;
    });
}

/// Tracks a single retry driver across checkout attempts, independent of other tasks using it.
pub struct SoulpitHolderRetry {
    driver: &'static str,
    last_refusal: Option<Instant>,
    refusals: u64,
}

impl SoulpitHolderRetry {
    pub fn new(driver: &'static str) -> Self {
        Self {
            driver,
            last_refusal: None,
            refusals: 0,
        }
    }

    pub(in crate::durability) fn tag(&self) -> Tag {
        Tag::Soulpit(self.driver)
    }

    pub(in crate::durability) fn attempt(&self) {
        if let Some(last_refusal) = self.last_refusal {
            let delay = last_refusal.elapsed();
            update(self.tag(), |usage| {
                usage.retry_intervals += 1;
                usage.retry_delay += delay;
                usage.min_retry_delay =
                    Some(usage.min_retry_delay.map_or(delay, |old| old.min(delay)));
                usage.max_retry_delay = usage.max_retry_delay.max(delay);
            });
        }
    }

    pub(in crate::durability) fn outcome(&mut self, acquired: bool) {
        if !acquired {
            self.last_refusal = Some(Instant::now());
            self.refusals += 1;
        } else if self.refusals != 0 {
            let retries = std::mem::take(&mut self.refusals);
            self.last_refusal = None;
            update(self.tag(), |usage| {
                usage.retried_passes += 1;
                usage.retries_on_passes += retries;
                usage.max_retries_per_pass = usage.max_retries_per_pass.max(retries);
            });
        }
    }
}
