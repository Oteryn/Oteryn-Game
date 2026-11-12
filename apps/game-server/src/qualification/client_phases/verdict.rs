//! One verdict shape for every qualification row.
//!
//! A failing row names one cause and one root, `<row>/<check|precondition>`.
//! Every FAIL line keeps its prefix and fields and appends
//! ` cause=<kind> root=<root>`; checks that never ran after the first failure
//! point at that root with `cause=blocked-by`. Rows declare what they need (a
//! released fixture character, an earlier row). Once a need has failed, a later
//! row needing it is reported FAIL at once instead of waiting its own deadline,
//! and a row without progress for `NODE_BOOT_NO_PROGRESS_S` is aborted.
//! `NODE_BOOT_NO_FAILFAST=1` keeps the earlier run-everything behaviour.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

/// Why a row failed; `kind` is the wire token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    Deadline {
        phase: &'static str,
    },
    NoProgress {
        since_last_pass_s: u64,
        phase: &'static str,
    },
    Precondition {
        name: String,
    },
    Transport {
        code: Option<u32>,
        trace: Option<String>,
        phase: Option<&'static str>,
        last_frame: Option<String>,
    },
    ServerRefused {
        code: u32,
    },
    Assertion,
    BlockedBy {
        root: String,
    },
}

impl Cause {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Deadline { .. } => "deadline",
            Self::NoProgress { .. } => "no-progress",
            Self::Precondition { .. } => "precondition",
            Self::Transport { .. } => "transport",
            Self::ServerRefused { .. } => "server-refused",
            Self::Assertion => "assertion",
            Self::BlockedBy { .. } => "blocked-by",
        }
    }

    /// The suffix every FAIL line of the row carries.
    pub(crate) fn suffix(&self, root: &str) -> String {
        format!(" cause={} root={root}", self.kind())
    }

    /// Kind-specific fields of the one `QUALIFICATION_CAUSE` line. Free-text
    /// error detail stays on the existing `_GAP` lines.
    fn fields(&self) -> String {
        match self {
            Self::Deadline { phase } => format!(" phase={phase}"),
            Self::NoProgress {
                since_last_pass_s,
                phase,
            } => format!(" phase={phase} since_last_pass_s={since_last_pass_s}"),
            Self::Precondition { name } => format!(" precondition={name}"),
            Self::Transport {
                code,
                trace,
                phase,
                last_frame,
            } => format!(
                " code={} trace={} phase={} last_frame={}",
                code.map_or_else(|| "none".to_owned(), |code| format!("E{code:04}")),
                trace.as_deref().unwrap_or("none"),
                phase.unwrap_or("none"),
                last_frame
                    .as_deref()
                    .map_or_else(|| "none".to_owned(), |frame| frame.replace(' ', "_")),
            ),
            Self::ServerRefused { code } => format!(" code={code}"),
            Self::Assertion | Self::BlockedBy { .. } => String::new(),
        }
    }
}

/// A harness-made failure that keeps its cause through `TestResult`.
#[derive(Debug)]
pub(super) struct HarnessError {
    cause: Cause,
    message: String,
}

impl HarnessError {
    pub(super) fn boxed(cause: Cause, message: impl Into<String>) -> Box<dyn Error> {
        Box::new(Self {
            cause,
            message: message.into(),
        })
    }
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HarnessError {}

/// Classify a row error by walking its source chain. A client failure keeps
/// its phase, last frame, server code and connection trace, so the client
/// line can be joined with the server's `transport_close` line.
pub(crate) fn classify(error: &(dyn Error + 'static)) -> Cause {
    use oteryn_dev_client::DevClientError;
    let mut context = None;
    let mut next = Some(error);
    while let Some(current) = next {
        if let Some(harness) = current.downcast_ref::<HarnessError>() {
            return harness.cause.clone();
        }
        if let Some(client) = current.downcast_ref::<DevClientError>() {
            match client {
                DevClientError::Context {
                    phase, last_frame, ..
                } => {
                    context.get_or_insert((*phase, last_frame.clone()));
                }
                DevClientError::Session(session) => {
                    return session_cause(&session.to_string(), context);
                }
                _ => return transport(None, None, context),
            }
        } else if current.is::<std::io::Error>() {
            return transport(None, None, context);
        } else if current.is::<tokio::time::error::Elapsed>() {
            return Cause::Deadline {
                phase: "inner-timeout",
            };
        }
        next = current.source();
    }
    match context {
        Some(_) => transport(None, None, context),
        None => Cause::Assertion,
    }
}

fn transport(
    code: Option<u32>,
    trace: Option<String>,
    context: Option<(&'static str, String)>,
) -> Cause {
    let (phase, last_frame) = context.unzip();
    Cause::Transport {
        code,
        trace,
        phase,
        last_frame,
    }
}

// `oteryn_session::SessionError` is not re-exported by the dev client, so the
// server code and trace come from its stable Display text (#3090).
fn session_cause(text: &str, context: Option<(&'static str, String)>) -> Cause {
    let number = |tail: &str| {
        tail.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<u32>()
            .ok()
    };
    if let Some((_, tail)) = text.split_once("server closed the connection with E") {
        let trace = tail
            .split_once("(trace ")
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(trace, _)| trace.to_owned())
            .filter(|trace| trace.chars().all(|c| c.is_ascii_hexdigit()));
        return transport(number(tail), trace, context);
    }
    for refusal in [
        "admission refused with code ",
        "admission queued with code ",
    ] {
        if let Some(code) = text.split_once(refusal).and_then(|(_, tail)| number(tail)) {
            return Cause::ServerRefused { code };
        }
    }
    transport(None, None, context)
}

/// A precondition a row declares before it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Need {
    /// Fixture character `i` is terminally released and can be admitted.
    Fixture(usize),
    /// An earlier row produced the state this row starts from.
    Row(&'static str),
}

struct Tracker {
    last: tokio::time::Instant,
    phase: &'static str,
    stall: Option<Cause>,
    not_run: bool,
}

/// Progress and stall state of a running row, shared with its watchdog.
#[derive(Clone)]
pub(super) struct Watch {
    row: &'static str,
    needs: Vec<Need>,
    harness: &'static Harness,
    tracker: Arc<Mutex<Tracker>>,
}

impl Watch {
    fn new(row: &'static str, harness: &'static Harness) -> Self {
        Self {
            row,
            needs: Vec::new(),
            harness,
            tracker: Arc::new(Mutex::new(Tracker {
                last: tokio::time::Instant::now(),
                phase: "start",
                stall: None,
                not_run: false,
            })),
        }
    }

    fn state(&self) -> MutexGuard<'_, Tracker> {
        self.tracker.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn touch(&self, phase: &'static str) {
        let mut state = self.state();
        state.last = tokio::time::Instant::now();
        state.phase = phase;
    }

    fn stall(&self, cause: Cause, message: String) -> Box<dyn Error> {
        self.state().stall = Some(cause.clone());
        HarnessError::boxed(cause, message)
    }

    /// Count admission and cleanup outside `bound` towards this row.
    pub(super) fn enter(&self) -> Active<'_> {
        Active::enter(self)
    }

    /// Run one row journey under its wall-clock budget and the no-progress
    /// watchdog. A row whose declared need already failed is not run.
    pub(super) async fn bound<T>(
        &self,
        budget: Duration,
        deadline: impl Into<String>,
        journey: impl Future<Output = TestResult<T>>,
    ) -> TestResult<T> {
        if let Some(root) = self.harness.blocked(&self.needs) {
            let mut state = self.state();
            state.not_run = true;
            state.stall = Some(Cause::BlockedBy { root: root.clone() });
            return Err(HarnessError::boxed(
                Cause::BlockedBy { root: root.clone() },
                format!("not run: blocked by {root}"),
            ));
        }
        let _active = Active::enter(self);
        self.touch("start");
        let watchdog = async {
            let Some(limit) = self.harness.no_progress else {
                return std::future::pending().await;
            };
            loop {
                let (last, phase) = {
                    let state = self.state();
                    (state.last, state.phase)
                };
                let idle = last.elapsed();
                if idle >= limit {
                    return (idle, phase);
                }
                tokio::time::sleep(limit - idle).await;
            }
        };
        tokio::select! {
            biased;
            result = tokio::time::timeout(budget, journey) => match result {
                Ok(result) => result,
                Err(_) => {
                    let phase = self.state().phase;
                    Err(self.stall(Cause::Deadline { phase }, deadline.into()))
                }
            },
            (idle, phase) = watchdog => {
                let since_last_pass_s = idle.as_secs();
                Err(self.stall(
                    Cause::NoProgress { since_last_pass_s, phase },
                    format!("no progress for {since_last_pass_s}s after {phase}; remaining actions unproven"),
                ))
            }
        }
    }
}

/// Marks the running row so shared helpers (admission, walking) count as its
/// progress and name it in a precondition root.
pub(super) struct Active<'a> {
    harness: &'a Harness,
    previous: Option<Watch>,
}

impl<'a> Active<'a> {
    fn enter(watch: &'a Watch) -> Self {
        let previous = watch.harness.active().replace(watch.clone());
        Self {
            harness: watch.harness,
            previous,
        }
    }
}

impl Drop for Active<'_> {
    fn drop(&mut self) {
        *self.harness.active() = self.previous.take();
    }
}

/// Process-wide health of declared needs and the run's fail-fast settings.
pub(super) struct Harness {
    failfast: bool,
    no_progress: Option<Duration>,
    health: Mutex<BTreeMap<Need, String>>,
    active: Mutex<Option<Watch>>,
}

impl Harness {
    fn new(failfast: bool, no_progress: Option<Duration>) -> Self {
        Self {
            failfast,
            no_progress,
            health: Mutex::new(BTreeMap::new()),
            active: Mutex::new(None),
        }
    }

    fn from_env(no_failfast: Option<&str>, no_progress: Option<&str>) -> Result<Self, String> {
        let failfast = no_failfast != Some("1");
        let seconds = match no_progress {
            None => 90,
            Some(value) => value
                .trim()
                .parse::<u64>()
                .map_err(|_| format!("invalid NODE_BOOT_NO_PROGRESS_S {value:?}"))?,
        };
        // The escape hatch runs every row to its own deadline, as before.
        let no_progress = (failfast && seconds > 0).then(|| Duration::from_secs(seconds));
        Ok(Self::new(failfast, no_progress))
    }

    pub(super) fn global() -> &'static Self {
        static GLOBAL: OnceLock<Harness> = OnceLock::new();
        GLOBAL.get_or_init(|| {
            let value = |name| std::env::var(name).ok();
            Self::from_env(
                value("NODE_BOOT_NO_FAILFAST").as_deref(),
                value("NODE_BOOT_NO_PROGRESS_S").as_deref(),
            )
            .unwrap_or_else(|error| {
                // Fail closed to the earlier behaviour; the line names the bad value.
                evidence(&format!(
                    "QUALIFICATION_HARNESS {error}; fail-fast disabled"
                ));
                Self::new(false, None)
            })
        })
    }

    fn health(&self) -> MutexGuard<'_, BTreeMap<Need, String>> {
        self.health.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn active(&self) -> MutexGuard<'_, Option<Watch>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn blocked(&self, needs: &[Need]) -> Option<String> {
        if !self.failfast {
            return None;
        }
        let health = self.health();
        needs.iter().find_map(|need| health.get(need).cloned())
    }

    /// Record a failed need; the first failure stays the root.
    fn fail(&self, need: Need, root: String) {
        self.health().entry(need).or_insert(root);
    }

    /// Count a shared step (admission, a walked tile) as progress of the running row.
    pub(super) fn progress(&self, phase: &'static str) {
        if let Some(watch) = self.active().as_ref() {
            watch.touch(phase);
        }
    }

    fn running_row(&self) -> &'static str {
        self.active().as_ref().map_or("core", |watch| watch.row)
    }

    /// Fail at once when an earlier wait for this fixture already timed out.
    pub(super) fn fixture_gate(&self, index: usize) -> TestResult {
        match self.blocked(&[Need::Fixture(index)]) {
            Some(root) => Err(HarnessError::boxed(
                Cause::BlockedBy { root: root.clone() },
                format!("fixture {index} release blocked by {root}"),
            )),
            None => Ok(()),
        }
    }

    /// The running row becomes the root of every later row needing this fixture.
    pub(super) fn fixture_failed(&self, index: usize, error: &dyn Error) -> Box<dyn Error> {
        let name = format!("fixture{index}-released");
        self.fail(
            Need::Fixture(index),
            format!("{}/{name}", self.running_row()),
        );
        HarnessError::boxed(Cause::Precondition { name }, error.to_string())
    }
}

#[derive(Clone, Copy)]
enum Unproven {
    /// `<family>_CHECK <row> <action> FAIL assertion-not-completed`
    CheckFail,
    /// `<family>_GAP <row> <action> unproven`
    Gap,
}

/// One full row: its required actions, PASS lines and final verdict.
pub(super) struct Row {
    pub(super) id: &'static str,
    family: &'static str,
    unproven: Unproven,
    pub(super) required: &'static [&'static str],
    pub(super) passed: BTreeSet<&'static str>,
    invalid: Vec<&'static str>,
    started: std::time::Instant,
    evidence: Option<(&'static str, &'static str)>,
    /// A known absent server route turns an incomplete row into MISSING-SERVER.
    pub(super) missing_server: Option<&'static str>,
    watch: Watch,
}

impl Row {
    pub(super) fn new(
        family: &'static str,
        id: &'static str,
        required: &'static [&'static str],
    ) -> Self {
        Self::with_harness(family, id, required, Harness::global())
    }

    fn with_harness(
        family: &'static str,
        id: &'static str,
        required: &'static [&'static str],
        harness: &'static Harness,
    ) -> Self {
        Self {
            id,
            family,
            unproven: Unproven::CheckFail,
            required,
            passed: BTreeSet::new(),
            invalid: Vec::new(),
            started: std::time::Instant::now(),
            evidence: None,
            missing_server: None,
            watch: Watch::new(id, harness),
        }
    }

    pub(super) fn needs(mut self, needs: &[Need]) -> Self {
        self.watch.needs.extend_from_slice(needs);
        self
    }

    /// Unproven actions are `_GAP ... unproven` lines instead of CHECK FAIL lines.
    pub(super) fn gap_lines(mut self) -> Self {
        self.unproven = Unproven::Gap;
        self
    }

    /// The `evidence=` field of the `e2e_all` line on PASS and on failure.
    pub(super) fn evidence(mut self, pass: &'static str, fail: &'static str) -> Self {
        self.evidence = Some((pass, fail));
        self
    }

    /// Rows proven by one journey share its progress, stall and needs.
    pub(super) fn sharing(mut self, other: &Self) -> Self {
        self.watch.tracker = Arc::clone(&other.watch.tracker);
        self.watch.needs.clone_from(&other.watch.needs);
        self
    }

    pub(super) fn watch(&self) -> Watch {
        self.watch.clone()
    }

    /// Emit a PASS line for a required action; a repeated or unknown action
    /// invalidates the row.
    pub(super) fn pass(&mut self, action: &'static str, observation: impl fmt::Display) {
        if !self.required.contains(&action) || !self.passed.insert(action) {
            self.invalid.push(action);
            return;
        }
        evidence(&format!(
            "{}_CHECK {} {action} PASS {observation}",
            self.family, self.id
        ));
        self.watch.touch(action);
    }

    /// For journeys that observe the same action repeatedly: only the first
    /// observation emits a line, each one counts as progress.
    pub(super) fn pass_once(&mut self, action: &'static str, observation: impl fmt::Display) {
        if self.required.contains(&action) && self.passed.insert(action) {
            evidence(&format!(
                "{}_CHECK {} {action} PASS {observation}",
                self.family, self.id
            ));
        }
        if self.required.contains(&action) {
            self.watch.touch(action);
        }
    }

    pub(super) fn invalidate(&mut self, action: &'static str) {
        self.invalid.push(action);
    }

    pub(super) fn complete(&self) -> bool {
        !self.required.is_empty()
            && self.invalid.is_empty()
            && self
                .required
                .iter()
                .all(|action| self.passed.contains(action))
    }

    fn unproven(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.required
            .iter()
            .copied()
            .filter(|action| !self.passed.contains(action))
    }

    /// The row's cause and root. A stall recorded by `bound` wins over the
    /// error, which cleanup code may have turned into text.
    fn cause(&self, error: Option<&(dyn Error + 'static)>) -> (Cause, String) {
        let cause = self
            .watch
            .state()
            .stall
            .clone()
            .or_else(|| error.map(classify))
            .unwrap_or(Cause::Assertion);
        let check = self
            .unproven()
            .next()
            .or_else(|| self.invalid.first().copied())
            .unwrap_or("lifecycle");
        let root = match &cause {
            Cause::BlockedBy { root } => root.clone(),
            Cause::Precondition { name } => format!("{}/{name}", self.id),
            _ => format!("{}/{check}", self.id),
        };
        (cause, root)
    }

    fn report_unproven(&self, cause: &Cause, root: &str) {
        for (index, action) in self.unproven().enumerate() {
            let suffix = if index == 0 {
                cause.suffix(root)
            } else {
                Cause::BlockedBy {
                    root: root.to_owned(),
                }
                .suffix(root)
            };
            evidence(&match self.unproven {
                Unproven::CheckFail => format!(
                    "{}_CHECK {} {action} FAIL assertion-not-completed{suffix}",
                    self.family, self.id
                ),
                Unproven::Gap => {
                    format!("{}_GAP {} {action} unproven{suffix}", self.family, self.id)
                }
            });
        }
    }

    pub(super) fn finish(self, result: &TestResult) {
        let complete = result.is_ok() && self.complete();
        let failure = (!complete).then(|| self.cause(result.as_ref().err().map(|error| &**error)));
        if let Some((cause, root)) = &failure {
            self.report_unproven(cause, root);
        }
        for action in &self.invalid {
            evidence(&format!(
                "{}_GAP {} duplicate-or-unknown-action:{action}",
                self.family, self.id
            ));
        }
        if let Err(error) = result {
            evidence(&format!("{}_GAP {} {error}", self.family, self.id));
        }
        let (status, field) = match (&failure, self.missing_server) {
            (None, _) => ("PASS", self.evidence.map(|(pass, _)| pass)),
            (Some(_), Some(source)) => ("MISSING-SERVER", Some(source)),
            (Some(_), None) => ("FAIL", self.evidence.map(|(_, fail)| fail)),
        };
        self.report(status, field, failure);
    }

    /// The row is incomplete because a known server route is absent.
    pub(super) fn finish_missing_server(self, source: &str) {
        let (cause, root) = self.cause(None);
        self.report_unproven(&cause, &root);
        let field = format!("source:{source}");
        self.report("MISSING-SERVER", Some(field.as_str()), Some((cause, root)));
    }

    fn report(&self, status: &str, field: Option<&str>, failure: Option<(Cause, String)>) {
        let seconds = if self.watch.state().not_run {
            0.0
        } else {
            self.started.elapsed().as_secs_f64()
        };
        let field = field.map_or_else(String::new, |field| format!(" evidence={field}"));
        let mut suffix = String::new();
        if let Some((cause, root)) = failure {
            if status == "FAIL" {
                suffix = cause.suffix(&root);
                evidence(&format!(
                    "QUALIFICATION_CAUSE row={} cause={} root={root}{}",
                    self.id,
                    cause.kind(),
                    cause.fields()
                ));
            }
            self.watch.harness.fail(Need::Row(self.id), root);
        }
        evidence(&format!(
            "e2e_all scenario={}.full status={status} seconds={seconds:.3}{field}{suffix}",
            self.id
        ));
    }
}

/// Journey groups `NODE_BOOT_E2E_ONLY` selects, each with the groups whose
/// state it starts from. No group currently consumes another group's state:
/// each row owns its admissions and cleanup.
const GROUPS: &[(&str, &[&str])] = &[
    ("core", &[]),
    ("f4", &[]),
    ("f5", &[]),
    ("f6", &[]),
    ("f7add", &[]),
    ("f7clock", &[]),
    ("f7cont", &[]),
    ("f7forge", &[]),
    ("f7run", &[]),
];

/// The selected journey groups; `None` runs every group.
pub(super) struct Groups(Option<BTreeSet<&'static str>>);

impl Groups {
    pub(super) fn from_env() -> TestResult<Self> {
        let selected = Self::parse(std::env::var("NODE_BOOT_E2E_ONLY").ok().as_deref(), GROUPS)?;
        if let Some(groups) = &selected.0 {
            let names = groups.iter().copied().collect::<Vec<_>>().join(",");
            evidence(&format!("e2e_groups selected={names}"));
        }
        Ok(selected)
    }

    fn parse(
        value: Option<&str>,
        registry: &[(&'static str, &'static [&'static str])],
    ) -> TestResult<Self> {
        let Some(list) = value.filter(|list| !list.trim().is_empty()) else {
            return Ok(Self(None));
        };
        let mut selected = BTreeSet::new();
        let mut pending = list
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        while let Some(name) = pending.pop() {
            let (group, after) = registry
                .iter()
                .find(|(group, _)| *group == name)
                .ok_or_else(|| format!("unknown NODE_BOOT_E2E_ONLY group {name}"))?;
            if selected.insert(*group) {
                pending.extend(after.iter().copied());
            }
        }
        Ok(Self(Some(selected)))
    }

    pub(super) fn runs(&self, group: &str) -> bool {
        self.0
            .as_ref()
            .is_none_or(|selected| selected.contains(group))
    }
}

#[cfg(test)]
mod verdict_tests {
    use super::*;

    fn harness(failfast: bool, no_progress: Option<u64>) -> &'static Harness {
        Box::leak(Box::new(Harness::new(
            failfast,
            no_progress.map(Duration::from_secs),
        )))
    }

    fn paused() -> TestResult<tokio::runtime::Runtime> {
        Ok(tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?)
    }

    #[test]
    fn cause_suffix_and_fields_use_the_shared_wire_tokens() {
        let cases = [
            (Cause::Deadline { phase: "walk" }, "deadline", " phase=walk"),
            (
                Cause::NoProgress {
                    since_last_pass_s: 90,
                    phase: "earned-level-eight",
                },
                "no-progress",
                " phase=earned-level-eight since_last_pass_s=90",
            ),
            (
                Cause::Precondition {
                    name: "fixture1-released".into(),
                },
                "precondition",
                " precondition=fixture1-released",
            ),
            (
                Cause::Transport {
                    code: Some(17),
                    trace: Some("00ff".into()),
                    phase: Some("join_snapshot"),
                    last_frame: Some("server_sequence=3".into()),
                },
                "transport",
                " code=E0017 trace=00ff phase=join_snapshot last_frame=server_sequence=3",
            ),
            (
                Cause::ServerRefused { code: 1101 },
                "server-refused",
                " code=1101",
            ),
            (Cause::Assertion, "assertion", ""),
            (
                Cause::BlockedBy {
                    root: "S16a/fixture1-released".into(),
                },
                "blocked-by",
                "",
            ),
        ];
        for (cause, kind, fields) in cases {
            assert_eq!(cause.kind(), kind);
            assert_eq!(cause.suffix("C20a/x"), format!(" cause={kind} root=C20a/x"));
            assert_eq!(cause.fields(), fields);
        }
    }

    #[test]
    fn classify_keeps_harness_causes_and_reads_session_codes_and_traces() {
        let harness = HarnessError::boxed(Cause::Deadline { phase: "walk" }, "deadline");
        assert_eq!(
            classify(harness.as_ref()),
            Cause::Deadline { phase: "walk" }
        );
        let plain: Box<dyn Error> = "town step changed position".into();
        assert_eq!(classify(plain.as_ref()), Cause::Assertion);
        let io: Box<dyn Error> = Box::new(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        assert_eq!(classify(io.as_ref()), transport(None, None, None));
        assert_eq!(
            session_cause(
                "server closed the connection with E0042 (trace 0a1b2c)",
                Some(("join_snapshot", "ServerAccepted".into()))
            ),
            Cause::Transport {
                code: Some(42),
                trace: Some("0a1b2c".into()),
                phase: Some("join_snapshot"),
                last_frame: Some("ServerAccepted".into()),
            }
        );
        assert_eq!(
            session_cause("server closed the connection with E0042", None),
            transport(Some(42), None, None)
        );
        assert_eq!(
            session_cause("admission refused with code 1103", None),
            Cause::ServerRefused { code: 1103 }
        );
    }

    #[test]
    fn first_unproven_check_is_the_root_and_later_checks_are_blocked_by_it() {
        const ACTIONS: &[&str] = &["admission", "cast", "relog"];
        let mut row = Row::with_harness("E2EF6", "S16a", ACTIONS, harness(true, None));
        row.pass("admission", "observed");
        let error: Box<dyn Error> = "cast refused".into();
        let (cause, root) = row.cause(Some(error.as_ref()));
        assert_eq!((cause, root.as_str()), (Cause::Assertion, "S16a/cast"));
        assert_eq!(
            row.unproven().collect::<Vec<_>>(),
            ["cast", "relog"],
            "relog is reported blocked-by S16a/cast"
        );
        let precondition = HarnessError::boxed(
            Cause::Precondition {
                name: "fixture1-released".into(),
            },
            "not released",
        );
        assert_eq!(
            row.cause(Some(precondition.as_ref())).1,
            "S16a/fixture1-released"
        );
    }

    #[test]
    fn a_failed_fixture_blocks_later_rows_without_running_them() -> TestResult {
        let harness = harness(true, None);
        paused()?.block_on(async {
            let root_row =
                Row::with_harness("E2EF6", "S16a", &["a"], harness).needs(&[Need::Fixture(1)]);
            let watch = root_row.watch();
            let result: TestResult = watch
                .bound(Duration::from_secs(160), "deadline", async {
                    let timeout: Box<dyn Error> = "not terminally released".into();
                    Err(harness.fixture_failed(1, timeout.as_ref()))
                })
                .await;
            assert_eq!(
                root_row.cause(result.as_ref().err().map(|error| &**error)),
                (
                    Cause::Precondition {
                        name: "fixture1-released".into()
                    },
                    "S16a/fixture1-released".into()
                )
            );
            assert!(harness.fixture_gate(1).is_err());
            assert!(harness.fixture_gate(0).is_ok());

            let blocked =
                Row::with_harness("E2EF6", "C20a", &["a", "b"], harness).needs(&[Need::Fixture(1)]);
            let mut ran = false;
            let result: TestResult = blocked
                .watch()
                .bound(Duration::from_secs(1_100), "deadline", async {
                    ran = true;
                    Ok(())
                })
                .await;
            assert!(!ran, "a row whose need failed must not run");
            let (cause, root) = blocked.cause(result.as_ref().err().map(|error| &**error));
            assert_eq!(cause.kind(), "blocked-by");
            assert_eq!(root, "S16a/fixture1-released");
            assert!(blocked.watch.state().not_run);
            Ok(())
        })
    }

    #[test]
    fn failed_rows_block_rows_that_need_them_and_keep_the_original_root() {
        let harness = harness(true, None);
        harness.fail(Need::Row("S16a"), "S16a/fixture1-released".into());
        assert_eq!(
            harness
                .blocked(&[Need::Fixture(0), Need::Row("S16a")])
                .as_deref(),
            Some("S16a/fixture1-released")
        );
        harness.fail(Need::Row("S16a"), "S16a/later".into());
        assert_eq!(
            harness.blocked(&[Need::Row("S16a")]).as_deref(),
            Some("S16a/fixture1-released")
        );
    }

    #[test]
    fn no_failfast_runs_every_row_and_disables_the_watchdog() -> TestResult {
        let off = Harness::from_env(Some("1"), Some("5"))?;
        assert!(!off.failfast && off.no_progress.is_none());
        off.fail(Need::Fixture(1), "S16a/fixture1-released".into());
        assert!(off.fixture_gate(1).is_ok());
        assert_eq!(off.blocked(&[Need::Fixture(1)]), None);
        let on = Harness::from_env(None, None)?;
        assert_eq!(
            (on.failfast, on.no_progress),
            (true, Some(Duration::from_secs(90)))
        );
        let disabled = Harness::from_env(None, Some("0"))?;
        assert_eq!(disabled.no_progress, None);
        assert!(Harness::from_env(None, Some("ninety")).is_err());
        Ok(())
    }

    #[test]
    fn watchdog_aborts_a_row_without_progress_but_not_a_progressing_one() -> TestResult {
        let harness = harness(true, Some(90));
        paused()?.block_on(async {
            let mut stuck = Row::with_harness("E2EF6", "S08", &["a", "b"], harness);
            stuck.pass("a", "observed");
            let watch = stuck.watch();
            let started = tokio::time::Instant::now();
            let result: TestResult = watch
                .bound(
                    Duration::from_secs(1_800),
                    "deadline",
                    std::future::pending(),
                )
                .await;
            assert_eq!(started.elapsed(), Duration::from_secs(90));
            assert_eq!(
                stuck.cause(result.as_ref().err().map(|error| &**error)),
                (
                    Cause::NoProgress {
                        since_last_pass_s: 90,
                        phase: "start"
                    },
                    "S08/b".into()
                )
            );

            let progressing = Row::with_harness("E2EF6", "P21", &["a"], harness);
            let watch = progressing.watch();
            let inner = watch.clone();
            let started = tokio::time::Instant::now();
            let result: TestResult = watch
                .bound(Duration::from_secs(1_800), "deadline", async {
                    for _ in 0..10 {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        harness.progress("walk");
                    }
                    inner.touch("walk");
                    Ok(())
                })
                .await;
            assert!(result.is_ok());
            assert_eq!(started.elapsed(), Duration::from_secs(600));

            let slow = Row::with_harness("E2EF6", "W20a", &["a"], harness);
            let result: TestResult = slow
                .watch()
                .bound(Duration::from_secs(120), "world-clock deadline", async {
                    loop {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        harness.progress("clock-delta");
                    }
                })
                .await;
            let (cause, root) = slow.cause(result.as_ref().err().map(|error| &**error));
            assert_eq!((cause.kind(), root.as_str()), ("deadline", "W20a/a"));
            assert_eq!(
                result.err().map(|error| error.to_string()).as_deref(),
                Some("world-clock deadline")
            );
            Ok(())
        })
    }

    #[test]
    fn stall_cause_survives_errors_rewritten_as_text() -> TestResult {
        let harness = harness(true, Some(5));
        paused()?.block_on(async {
            let row = Row::with_harness("E2EF7", "P19", &["a"], harness).gap_lines();
            let result: TestResult = row
                .watch()
                .bound(Duration::from_secs(60), "deadline", std::future::pending())
                .await;
            let rewritten: Box<dyn Error> = format!("P19 cleanup=ok;journey={result:?}").into();
            assert_eq!(row.cause(Some(rewritten.as_ref())).0.kind(), "no-progress");
            Ok(())
        })
    }

    #[test]
    fn group_selection_includes_declared_prerequisites_and_rejects_unknown_names() -> TestResult {
        let all = Groups::parse(None, GROUPS)?;
        assert!(GROUPS.iter().all(|(group, _)| all.runs(group)));
        assert!(Groups::parse(Some(" "), GROUPS)?.runs("core"));
        let some = Groups::parse(Some("f6, f7run"), GROUPS)?;
        assert!(some.runs("f6") && some.runs("f7run"));
        assert!(!some.runs("core") && !some.runs("f7clock"));
        assert!(Groups::parse(Some("f6,nope"), GROUPS).is_err());
        const CHAIN: &[(&str, &[&str])] = &[("a", &["b"]), ("b", &["c"]), ("c", &[]), ("d", &[])];
        let chained = Groups::parse(Some("a"), CHAIN)?;
        assert!(chained.runs("a") && chained.runs("b") && chained.runs("c") && !chained.runs("d"));
        Ok(())
    }
}
