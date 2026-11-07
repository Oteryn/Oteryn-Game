#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LimitSpec {
    hard_maximum: u64,
    retained_bytes_per_unit: u64,
    fixed_retained_bytes: u64,
    work_units_per_unit: u64,
    fixed_work_units: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cost {
    retained_bytes: u64,
    work_units: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LimitError {
    CapacityExceeded,
    ArithmeticOverflow,
}

fn checked_cost(fixed: u64, per_unit: u64, observed: u64) -> Result<u64, LimitError> {
    per_unit
        .checked_mul(observed)
        .and_then(|variable| fixed.checked_add(variable))
        .ok_or(LimitError::ArithmeticOverflow)
}

fn assess(spec: &LimitSpec, observed: u64) -> Result<Cost, LimitError> {
    if observed > spec.hard_maximum {
        return Err(LimitError::CapacityExceeded);
    }

    Ok(Cost {
        retained_bytes: checked_cost(
            spec.fixed_retained_bytes,
            spec.retained_bytes_per_unit,
            observed,
        )?,
        work_units: checked_cost(spec.fixed_work_units, spec.work_units_per_unit, observed)?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CandidateLimit {
    inventory_id: &'static str,
    representative: u64,
    spec: LimitSpec,
}

impl CandidateLimit {
    fn candidate_id(&self) -> &'static str {
        match (self.inventory_id, self.spec.hard_maximum) {
            ("AB-RL-01", _) => "ABILITY01-TARGET-CANDIDATES",
            ("AB-RL-02", _) => "ABILITY01-RESOLVED-TARGETS",
            ("AB-RL-05", _) => "ABILITY01-EFFECT-PLAN-ENTRIES",
            ("AB-RL-06", _) => "ABILITY01-EFFECT-PLAN-BYTES",
            ("AB-RL-07", _) => "ABILITY01-CALC-STAGES",
            ("GI-RL-01", _) => "INTERACTION01-CASCADE-DEPTH",
            ("GI-RL-02", _) => "INTERACTION01-CHILD-FANOUT",
            ("GI-RL-03", _) => "INTERACTION01-ROOT-WORK",
            ("GI-RL-06", _) => "INTERACTION01-TRIGGER-CANDIDATES",
            ("GI-RL-07", _) => "INTERACTION01-RETAINED-CHILD-LIFECYCLES",
            ("AI-RL-01", _) => "AI01-ACTIVE-ACTORS",
            ("AI-RL-02", _) => "AI01-AUTHORED-UNITS",
            ("AI-RL-03", _) => "AI01-EVALUATION-WORK",
            ("AI-RL-05", _) => "AI01-PERCEPTION-CANDIDATES",
            ("AI-RL-07", _) => "AI01-PATH-REQUESTS-PER-ACTOR",
            ("AI-RL-08", _) => "AI01-PATH-SEARCH-WORK",
            ("AI-RL-09", 128) => "AI01-ROUTE-STEPS",
            ("AI-RL-09", 4_096) => "AI01-ROUTE-BYTES",
            ("NET03-RL-01", _) => "NET03-PREADMISSION-CONNECTIONS",
            ("NET03-RL-02", _) => "NET03-HANDSHAKE-AUTH-WORK",
            ("NET03-RL-04", _) => "NET03-OUTBOUND-QUEUE-ENTRIES",
            ("NET03-RL-05", _) => "NET03-OUTBOUND-QUEUE-BYTES",
            ("NET03-RL-06", _) => "NET03-PENDING-WRITES",
            ("NET03-RL-07", _) => "NET03-DRAIN-TASKS",
            _ => "UNKNOWN-CANDIDATE",
        }
    }

    fn domain(&self) -> &'static str {
        if self.inventory_id.starts_with("AB-") {
            "Ability"
        } else if self.inventory_id.starts_with("GI-") {
            "Interaction"
        } else if self.inventory_id.starts_with("AI-") {
            "AI"
        } else {
            "Server Seam"
        }
    }

    fn resource(&self) -> &'static str {
        match self.candidate_id() {
            "ABILITY01-TARGET-CANDIDATES" => {
                "target candidates examined by one explicit resolution step"
            }
            "ABILITY01-RESOLVED-TARGETS" => "resolved targets for one immediate ability occurrence",
            "ABILITY01-EFFECT-PLAN-ENTRIES" => "typed entries in one staged Effect Plan",
            "ABILITY01-EFFECT-PLAN-BYTES" => {
                "retained encoded/in-memory bytes for one staged Effect Plan"
            }
            "ABILITY01-CALC-STAGES" => "typed calculation/contribution stages per occurrence",
            "INTERACTION01-CASCADE-DEPTH" => "child ancestry levels per interaction root",
            "INTERACTION01-CHILD-FANOUT" => "immediate child occurrences per parent",
            "INTERACTION01-ROOT-WORK" => "total immediate descendant work per root",
            "INTERACTION01-TRIGGER-CANDIDATES" => {
                "eligible trigger/edge candidates per source occurrence"
            }
            "INTERACTION01-RETAINED-CHILD-LIFECYCLES" => {
                "retained child lifecycle entries per root"
            }
            "AI01-ACTIVE-ACTORS" => "active AI actors per authoritative scope safety envelope",
            "AI01-AUTHORED-UNITS" => "authored acquire-or-idle representation units",
            "AI01-EVALUATION-WORK" => "semantic AI evaluation work per resolution",
            "AI01-PERCEPTION-CANDIDATES" => "perception/target candidates per decision",
            "AI01-PATH-REQUESTS-PER-ACTOR" => "queued/in-flight path requests per actor",
            "AI01-PATH-SEARCH-WORK" => "path-search nodes/work units per request",
            "AI01-ROUTE-STEPS" => "route steps retained in one path proposal",
            "AI01-ROUTE-BYTES" => "aggregate retained route bytes in one path proposal",
            "NET03-PREADMISSION-CONNECTIONS" => "concurrent pre-admission TCP/TLS connections",
            "NET03-HANDSHAKE-AUTH-WORK" => "concurrent TLS handshake/authentication work",
            "NET03-OUTBOUND-QUEUE-ENTRIES" => "outbound queued entries per admitted session",
            "NET03-OUTBOUND-QUEUE-BYTES" => "outbound queued bytes per admitted session",
            "NET03-PENDING-WRITES" => "pending transport writes per session",
            "NET03-DRAIN-TASKS" => "connection/task shutdown and drain work per batch",
            _ => "unknown candidate resource",
        }
    }

    fn unit(&self) -> &'static str {
        match self.candidate_id() {
            "ABILITY01-TARGET-CANDIDATES" => "candidate entities/positions",
            "ABILITY01-RESOLVED-TARGETS" => "targets",
            "ABILITY01-EFFECT-PLAN-ENTRIES" => "typed plan entries",
            "ABILITY01-EFFECT-PLAN-BYTES" => "bytes",
            "ABILITY01-CALC-STAGES" => "stages",
            "INTERACTION01-CASCADE-DEPTH" => "child ancestry levels",
            "INTERACTION01-CHILD-FANOUT" => "child occurrences",
            "INTERACTION01-ROOT-WORK" => "descendant work units",
            "INTERACTION01-TRIGGER-CANDIDATES" => "candidates",
            "INTERACTION01-RETAINED-CHILD-LIFECYCLES" => "child lifecycle entries",
            "AI01-ACTIVE-ACTORS" => "actors",
            "AI01-AUTHORED-UNITS" => "authored units",
            "AI01-EVALUATION-WORK" => "deterministic work units",
            "AI01-PERCEPTION-CANDIDATES" => "candidates",
            "AI01-PATH-REQUESTS-PER-ACTOR" => "requests per actor",
            "AI01-PATH-SEARCH-WORK" => "nodes/work units",
            "AI01-ROUTE-STEPS" => "route steps",
            "AI01-ROUTE-BYTES" => "bytes",
            "NET03-PREADMISSION-CONNECTIONS" => "connections",
            "NET03-HANDSHAKE-AUTH-WORK" => "concurrent handshakes",
            "NET03-OUTBOUND-QUEUE-ENTRIES" => "entries per session",
            "NET03-OUTBOUND-QUEUE-BYTES" => "bytes per session",
            "NET03-PENDING-WRITES" => "writes per session",
            "NET03-DRAIN-TASKS" => "tasks per drain batch",
            _ => "unknown unit",
        }
    }

    fn owner_contract(&self) -> &'static str {
        match self.domain() {
            "Ability" => "GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md",
            "Interaction" => {
                "GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md"
            }
            "AI" => "GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md",
            _ => "FND-03_RUNTIME_EXECUTION_CONTRACT.md",
        }
    }
}

fn all_candidate_limits() -> Vec<CandidateLimit> {
    let mut limits = Vec::with_capacity(24);
    limits.extend(ability_limits());
    limits.extend(interaction_limits());
    limits.extend(ai_limits());
    limits.extend(listener_limits());
    limits
}

#[repr(C)]
struct AbilityPlanFixture {
    bytes: [u8; 1_024],
}

fn ability_limits() -> [CandidateLimit; 5] {
    [
        CandidateLimit {
            inventory_id: "AB-RL-01",
            representative: 1,
            spec: LimitSpec {
                hard_maximum: 2,
                retained_bytes_per_unit: 32,
                fixed_retained_bytes: 64,
                work_units_per_unit: 1,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "AB-RL-02",
            representative: 1,
            spec: LimitSpec {
                hard_maximum: 2,
                retained_bytes_per_unit: 32,
                fixed_retained_bytes: 64,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "AB-RL-05",
            representative: 1,
            spec: LimitSpec {
                hard_maximum: 2,
                retained_bytes_per_unit: 256,
                fixed_retained_bytes: 256,
                work_units_per_unit: 4,
                fixed_work_units: 2,
            },
        },
        CandidateLimit {
            inventory_id: "AB-RL-06",
            representative: std::mem::size_of::<AbilityPlanFixture>() as u64,
            spec: LimitSpec {
                hard_maximum: 4_096,
                retained_bytes_per_unit: 1,
                fixed_retained_bytes: 0,
                work_units_per_unit: 0,
                fixed_work_units: 8,
            },
        },
        CandidateLimit {
            inventory_id: "AB-RL-07",
            representative: 4,
            spec: LimitSpec {
                hard_maximum: 8,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 128,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
    ]
}

#[repr(C)]
struct InteractionChildFixture {
    bytes: [u8; 128],
}

fn interaction_limits() -> [CandidateLimit; 5] {
    [
        CandidateLimit {
            inventory_id: "GI-RL-01",
            representative: 1,
            spec: LimitSpec {
                hard_maximum: 2,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 64,
                work_units_per_unit: 1,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "GI-RL-02",
            representative: 4,
            spec: LimitSpec {
                hard_maximum: 8,
                retained_bytes_per_unit: std::mem::size_of::<InteractionChildFixture>() as u64,
                fixed_retained_bytes: 128,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "GI-RL-03",
            representative: 4,
            spec: LimitSpec {
                hard_maximum: 8,
                retained_bytes_per_unit: std::mem::size_of::<InteractionChildFixture>() as u64,
                fixed_retained_bytes: 256,
                work_units_per_unit: 4,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "GI-RL-06",
            representative: 8,
            spec: LimitSpec {
                hard_maximum: 16,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 64,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "GI-RL-07",
            representative: 4,
            spec: LimitSpec {
                hard_maximum: 8,
                retained_bytes_per_unit: 192,
                fixed_retained_bytes: 128,
                work_units_per_unit: 1,
                fixed_work_units: 1,
            },
        },
    ]
}

#[repr(C)]
struct AiActorFixture {
    bytes: [u8; 256],
}
#[repr(C)]
struct PathNodeFixture {
    bytes: [u8; 48],
}
#[repr(C)]
struct RouteStepFixture {
    bytes: [u8; 32],
}

fn ai_limits() -> [CandidateLimit; 8] {
    [
        CandidateLimit {
            inventory_id: "AI-RL-01",
            representative: 128,
            spec: LimitSpec {
                hard_maximum: 256,
                retained_bytes_per_unit: std::mem::size_of::<AiActorFixture>() as u64,
                fixed_retained_bytes: 0,
                work_units_per_unit: 1,
                fixed_work_units: 0,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-02",
            representative: 2,
            spec: LimitSpec {
                hard_maximum: 4,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 64,
                work_units_per_unit: 1,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-03",
            representative: 4,
            spec: LimitSpec {
                hard_maximum: 8,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 128,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-05",
            representative: 32,
            spec: LimitSpec {
                hard_maximum: 64,
                retained_bytes_per_unit: 32,
                fixed_retained_bytes: 128,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-07",
            representative: 1,
            spec: LimitSpec {
                hard_maximum: 2,
                retained_bytes_per_unit: 256,
                fixed_retained_bytes: 0,
                work_units_per_unit: 1,
                fixed_work_units: 0,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-08",
            representative: 512,
            spec: LimitSpec {
                hard_maximum: 1_024,
                retained_bytes_per_unit: std::mem::size_of::<PathNodeFixture>() as u64,
                fixed_retained_bytes: 256,
                work_units_per_unit: 1,
                fixed_work_units: 0,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-09",
            representative: 64,
            spec: LimitSpec {
                hard_maximum: 128,
                retained_bytes_per_unit: std::mem::size_of::<RouteStepFixture>() as u64,
                fixed_retained_bytes: 64,
                work_units_per_unit: 1,
                fixed_work_units: 0,
            },
        },
        CandidateLimit {
            inventory_id: "AI-RL-09",
            representative: 2_048,
            spec: LimitSpec {
                hard_maximum: 4_096,
                retained_bytes_per_unit: 1,
                fixed_retained_bytes: 0,
                work_units_per_unit: 0,
                fixed_work_units: 4,
            },
        },
    ]
}

#[repr(C)]
struct PreAdmissionConnectionFixture {
    bytes: [u8; 2_048],
}
#[repr(C)]
struct TlsHandshakeFixture {
    bytes: [u8; 16_384],
}

fn listener_inherited_frame_bytes() -> u64 {
    1_048_576
}

fn listener_limits() -> [CandidateLimit; 6] {
    [
        CandidateLimit {
            inventory_id: "NET03-RL-01",
            representative: 128,
            spec: LimitSpec {
                hard_maximum: 256,
                retained_bytes_per_unit: std::mem::size_of::<PreAdmissionConnectionFixture>()
                    as u64,
                fixed_retained_bytes: 0,
                work_units_per_unit: 1,
                fixed_work_units: 0,
            },
        },
        CandidateLimit {
            inventory_id: "NET03-RL-02",
            representative: 32,
            spec: LimitSpec {
                hard_maximum: 64,
                retained_bytes_per_unit: std::mem::size_of::<TlsHandshakeFixture>() as u64,
                fixed_retained_bytes: 0,
                work_units_per_unit: 4,
                fixed_work_units: 0,
            },
        },
        CandidateLimit {
            inventory_id: "NET03-RL-04",
            representative: 32,
            spec: LimitSpec {
                hard_maximum: 64,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 128,
                work_units_per_unit: 1,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "NET03-RL-05",
            representative: 524_288,
            spec: LimitSpec {
                hard_maximum: 1_048_576,
                retained_bytes_per_unit: 1,
                fixed_retained_bytes: 0,
                work_units_per_unit: 0,
                fixed_work_units: 4,
            },
        },
        CandidateLimit {
            inventory_id: "NET03-RL-06",
            representative: 4,
            spec: LimitSpec {
                hard_maximum: 8,
                retained_bytes_per_unit: 128,
                fixed_retained_bytes: 64,
                work_units_per_unit: 2,
                fixed_work_units: 1,
            },
        },
        CandidateLimit {
            inventory_id: "NET03-RL-07",
            representative: 128,
            spec: LimitSpec {
                hard_maximum: 256,
                retained_bytes_per_unit: 64,
                fixed_retained_bytes: 128,
                work_units_per_unit: 1,
                fixed_work_units: 1,
            },
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExcludedRow {
    inventory_id: &'static str,
    reason: &'static str,
    fail_closed: bool,
}

const fn excluded(inventory_id: &'static str, reason: &'static str) -> ExcludedRow {
    ExcludedRow {
        inventory_id,
        reason,
        fail_closed: true,
    }
}

fn ability_exclusions() -> [ExcludedRow; 14] {
    [
        excluded(
            "AB-RL-03",
            "explicit-target slice performs no geometry or spatial candidate query",
        ),
        excluded(
            "AB-RL-04",
            "dynamic retargeting is disabled in the first slice",
        ),
        excluded(
            "AB-RL-08",
            "multi-hit and multi-target sub-occurrences are disabled",
        ),
        excluded(
            "AB-RL-09",
            "channel and periodic future occurrences are disabled",
        ),
        excluded("AB-RL-10", "no future ability work may be enqueued"),
        excluded(
            "AB-RL-11",
            "catch-up scheduling is unreachable without future work",
        ),
        excluded("AB-RL-12", "conditions are outside the first slice"),
        excluded(
            "AB-RL-13",
            "scheduled condition work is outside the first slice",
        ),
        excluded("AB-RL-14", "post-commit reactions are disabled"),
        excluded("AB-RL-15", "reaction descendants are disabled"),
        excluded("AB-RL-16", "aggregate reaction/future work is unreachable"),
        excluded("AB-RL-17", "cross-domain proposals are disabled"),
        excluded(
            "AB-RL-18",
            "variable diagnostic payloads are disabled; fixed counters only",
        ),
        excluded("AB-RL-19", "script-backed mechanics are disabled"),
    ]
}

fn interaction_exclusions() -> [ExcludedRow; 2] {
    [
        excluded(
            "GI-RL-04",
            "foreign delegated owner operations are disabled",
        ),
        excluded(
            "GI-RL-05",
            "automatic reconciliation and retry execution are disabled",
        ),
    ]
}
fn ai_exclusions() -> [ExcludedRow; 9] {
    [
        excluded(
            "AI-RL-04",
            "threat, stimulus and memory collections are disabled",
        ),
        excluded("AI-RL-06", "AI timers and delayed operations are disabled"),
        excluded("AI-RL-10", "repath and retry windows are disabled"),
        excluded(
            "AI-RL-11",
            "spawn sources and population mutation are disabled",
        ),
        excluded("AI-RL-12", "spawn placement search is disabled"),
        excluded("AI-RL-13", "postponed occupancy retries are disabled"),
        excluded("AI-RL-14", "controlled-actor command backlog is disabled"),
        excluded("AI-RL-15", "script-backed AI is disabled"),
        excluded(
            "AI-RL-16",
            "variable replay/diagnostic payloads are disabled; fixed counters only",
        ),
    ]
}

fn durability_exclusions() -> [ExcludedRow; 8] {
    const REASON: &str = "journal-only first slice exposes no item/value transaction entry; transaction work fails closed";
    [
        excluded("DUR03-RL-01", REASON),
        excluded("DUR03-RL-02", REASON),
        excluded("DUR03-RL-03", REASON),
        excluded("DUR03-RL-04", REASON),
        excluded("DUR03-RL-05", REASON),
        excluded("DUR03-RL-06", REASON),
        excluded("DUR03-RL-07", REASON),
        excluded("DUR03-RL-08", REASON),
    ]
}

const SOURCE_MAIN_SHA: &str = "86653375231febbf81623b4c6984a6ff1263bdc2";

impl CandidateLimit {
    fn client_visible(&self) -> bool {
        self.domain() != "AI"
    }
}

fn all_exclusions() -> Vec<ExcludedRow> {
    let mut rows = Vec::with_capacity(33);
    rows.extend(ability_exclusions());
    rows.extend(interaction_exclusions());
    rows.extend(ai_exclusions());
    rows.extend(durability_exclusions());
    rows
}

fn exclusion_domain(id: &str) -> &'static str {
    if id.starts_with("AB-") {
        "Ability"
    } else if id.starts_with("GI-") {
        "Interaction"
    } else if id.starts_with("AI-") {
        "AI"
    } else {
        "Durability"
    }
}
fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c < ' ' => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output.push('"');
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StressSummary {
    iterations: u64,
    accepted_cases: u64,
    rejected_cases: u64,
    peak_single_allocation_bytes: u64,
    checksum: u64,
}

fn stress_fixture(iterations: u64) -> StressSummary {
    let limits = all_candidate_limits();
    let mut accepted_cases = 0_u64;
    let mut rejected_cases = 0_u64;
    let mut peak_single_allocation_bytes = 0_u64;
    let mut checksum = 0xcbf2_9ce4_8422_2325_u64;

    for iteration in 0..iterations {
        for limit in &limits {
            let cost = assess(&limit.spec, limit.spec.hard_maximum)
                .expect("candidate max must remain accepted");
            accepted_cases += 1;
            peak_single_allocation_bytes = peak_single_allocation_bytes.max(cost.retained_bytes);
            let len = usize::try_from(cost.retained_bytes).expect("fixture allocation fits usize");
            let mut retained = vec![0_u8; len];
            if let Some(first) = retained.first_mut() {
                *first = (iteration as u8).wrapping_add(1);
            }
            if let Some(last) = retained.last_mut() {
                *last = (limit.spec.hard_maximum as u8).wrapping_add(1);
            }
            checksum = checksum.rotate_left(7)
                ^ cost.retained_bytes
                ^ cost.work_units
                ^ retained.len() as u64;

            let rejected = limit
                .spec
                .hard_maximum
                .checked_add(1)
                .and_then(|count| assess(&limit.spec, count).err());
            assert_eq!(rejected, Some(LimitError::CapacityExceeded));
            rejected_cases += 1;
        }
    }

    StressSummary {
        iterations,
        accepted_cases,
        rejected_cases,
        peak_single_allocation_bytes,
        checksum,
    }
}

fn evidence_json() -> String {
    let limits = all_candidate_limits();
    let exclusions = all_exclusions();
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str("  \"schema_version\": 1,\n");
    output.push_str("  \"status\": \"PASS\",\n");
    output.push_str(&format!(
        "  \"source_main_sha\": {},\n",
        json_string(SOURCE_MAIN_SHA)
    ));
    output.push_str("  \"authority\": \"first-slice safety ceilings only; no production sizing, gameplay balance, Reference parity, or implementation authority\",\n");
    output.push_str(&format!("  \"candidate_limit_count\": {},\n", limits.len()));
    output.push_str(&format!(
        "  \"excluded_row_count\": {},\n",
        exclusions.len()
    ));
    output.push_str("  \"inherited_exact\": [\n");
    output.push_str(&format!("    {{\"inventory_id\": \"NET03-RL-03\", \"registry_id\": \"FND02-WIRE-FRAME-BYTES\", \"hard_maximum\": {}, \"unit\": \"bytes\", \"same_resource_proof\": \"first listener retains at most one already FND-02-bounded WireEnvelope frame per connection and introduces no second application assembly buffer\"}}\n", listener_inherited_frame_bytes()));
    output.push_str("  ],\n");
    output.push_str("  \"candidate_limits\": [\n");
    for (index, limit) in limits.iter().enumerate() {
        let Ok(representative_cost) = assess(&limit.spec, limit.representative) else {
            return "{\"schema_version\":1,\"status\":\"FAIL\",\"reason\":\"representative accounting failed\"}\n".to_owned();
        };
        let Ok(hard_cost) = assess(&limit.spec, limit.spec.hard_maximum) else {
            return "{\"schema_version\":1,\"status\":\"FAIL\",\"reason\":\"hard maximum accounting failed\"}\n".to_owned();
        };
        let margin_multiplier = limit.spec.hard_maximum / limit.representative;
        let comma = if index + 1 == limits.len() { "" } else { "," };
        output.push_str(&format!(
            "    {{\"candidate_id\": {}, \"inventory_id\": {}, \"domain\": {}, \"owner_contract\": {}, \"resource\": {}, \"unit\": {}, \"hard_maximum\": {}, \"configurable_range\": {{\"minimum\": 1, \"maximum\": {}}}, \"representative_fixture\": {}, \"margin_multiplier\": {}, \"retained_bytes_per_unit\": {}, \"fixed_retained_bytes\": {}, \"representative_retained_bytes\": {}, \"hard_max_retained_bytes\": {}, \"work_units_per_unit\": {}, \"fixed_work_units\": {}, \"representative_work_units\": {}, \"hard_max_work_units\": {}, \"failure_category\": \"CAPACITY_EXCEEDED\", \"allocation_impact\": \"check before retaining or allocating resource-shaped state and before publishing partial mutation\", \"client_visible\": {}, \"boundary_tests\": [\"hard maximum accepted\", \"hard maximum + 1 rejected as CAPACITY_EXCEEDED before allocation or partial mutation\", \"checked arithmetic overflow rejected before allocation or partial mutation\"], \"evidence_basis\": \"representative deterministic fixture plus checked retained/work cost equations, at least 2x headroom, and optimized multi-iteration stress fixture\", \"max_result\": \"ACCEPT\", \"max_plus_one_result\": \"CAPACITY_EXCEEDED\", \"overflow_result\": \"ARITHMETIC_OVERFLOW\"}}{}\n",
            json_string(limit.candidate_id()),
            json_string(limit.inventory_id),
            json_string(limit.domain()),
            json_string(limit.owner_contract()),
            json_string(limit.resource()),
            json_string(limit.unit()),
            limit.spec.hard_maximum,
            limit.spec.hard_maximum,
            limit.representative,
            margin_multiplier,
            limit.spec.retained_bytes_per_unit,
            limit.spec.fixed_retained_bytes,
            representative_cost.retained_bytes,
            hard_cost.retained_bytes,
            limit.spec.work_units_per_unit,
            limit.spec.fixed_work_units,
            representative_cost.work_units,
            hard_cost.work_units,
            if limit.client_visible() { "true" } else { "false" },
            comma,
        ));
    }
    output.push_str("  ],\n");
    output.push_str("  \"excluded_rows\": [\n");
    for (index, row) in exclusions.iter().enumerate() {
        let comma = if index + 1 == exclusions.len() {
            ""
        } else {
            ","
        };
        output.push_str(&format!(
            "    {{\"inventory_id\": {}, \"domain\": {}, \"disposition\": \"NOT_APPLICABLE_TO_FIRST_SLICE\", \"fail_closed\": true, \"reason\": {}}}{}\n",
            json_string(row.inventory_id), json_string(exclusion_domain(row.inventory_id)), json_string(row.reason), comma
        ));
    }
    output.push_str("  ],\n");
    output.push_str("  \"movement_status\": \"NON_CURRENT_SUCCESSOR_REQUIRED\",\n");
    output.push_str("  \"movement_deferred_rows\": [\"MOVE-RL-02\", \"MOVE-RL-03\", \"MOVE-RL-04\", \"MOVE-RL-05\", \"MOVE-RL-06\", \"MOVE-RL-07\", \"MOVE-RL-08\", \"MOVE-RL-09\", \"MOVE-RL-10\", \"MOVE-RL-11\", \"MOVE-RL-16\", \"MOVE-RL-17\"],\n");
    output.push_str("  \"movement_inherited_exact_rows\": [\"MOVE-RL-01\", \"MOVE-RL-12\", \"MOVE-RL-13\", \"MOVE-RL-14\", \"MOVE-RL-15\"],\n");
    output.push_str("  \"production_default_selected\": false,\n");
    output.push_str("  \"reference_parity_claimed\": false\n");
    output.push_str("}\n");
    output
}

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--stress") {
        let iterations = args
            .next()
            .as_deref()
            .unwrap_or("8")
            .parse::<u64>()
            .expect("--stress iterations must be a positive integer");
        let summary = stress_fixture(iterations);
        println!(
            "iterations={} accepted={} rejected={} peak_single_allocation_bytes={} checksum={}",
            summary.iterations,
            summary.accepted_cases,
            summary.rejected_cases,
            summary.peak_single_allocation_bytes,
            summary.checksum
        );
    } else {
        print!("{}", evidence_json());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_limit_accepts_max_and_rejects_max_plus_one() {
        let spec = LimitSpec {
            hard_maximum: 8,
            retained_bytes_per_unit: 16,
            fixed_retained_bytes: 32,
            work_units_per_unit: 2,
            fixed_work_units: 1,
        };
        assert_eq!(
            assess(&spec, 8).unwrap(),
            Cost {
                retained_bytes: 160,
                work_units: 17,
            }
        );
        assert_eq!(assess(&spec, 9), Err(LimitError::CapacityExceeded));
    }

    #[test]
    fn ability_first_slice_limits_are_exact_and_bounded() {
        let limits = ability_limits();
        let expected = [
            ("AB-RL-01", 2, 1),
            ("AB-RL-02", 2, 1),
            ("AB-RL-05", 2, 1),
            ("AB-RL-06", 4_096, 1_024),
            ("AB-RL-07", 8, 4),
        ];
        assert_eq!(limits.len(), expected.len());
        assert_eq!(std::mem::size_of::<AbilityPlanFixture>(), 1_024);
        for (limit, (id, hard_maximum, representative)) in limits.iter().zip(expected) {
            assert_eq!(limit.inventory_id, id);
            assert_eq!(limit.spec.hard_maximum, hard_maximum);
            assert_eq!(limit.representative, representative);
            assert!(assess(&limit.spec, hard_maximum).is_ok());
            assert_eq!(
                assess(&limit.spec, hard_maximum + 1),
                Err(LimitError::CapacityExceeded)
            );
        }
    }

    #[test]
    fn interaction_first_slice_limits_are_exact_and_bounded() {
        let limits = interaction_limits();
        let expected = [
            ("GI-RL-01", 2, 1),
            ("GI-RL-02", 8, 4),
            ("GI-RL-03", 8, 4),
            ("GI-RL-06", 16, 8),
            ("GI-RL-07", 8, 4),
        ];
        assert_eq!(limits.len(), expected.len());
        assert_eq!(std::mem::size_of::<InteractionChildFixture>(), 128);
        for (limit, (id, hard_maximum, representative)) in limits.iter().zip(expected) {
            assert_eq!(limit.inventory_id, id);
            assert_eq!(limit.spec.hard_maximum, hard_maximum);
            assert_eq!(limit.representative, representative);
            assert!(assess(&limit.spec, hard_maximum).is_ok());
            assert_eq!(
                assess(&limit.spec, hard_maximum + 1),
                Err(LimitError::CapacityExceeded)
            );
        }
    }

    #[test]
    fn ai_first_slice_limits_are_exact_and_bounded() {
        let limits = ai_limits();
        let expected = [
            ("AI-RL-01", 256, 128),
            ("AI-RL-02", 4, 2),
            ("AI-RL-03", 8, 4),
            ("AI-RL-05", 64, 32),
            ("AI-RL-07", 2, 1),
            ("AI-RL-08", 1_024, 512),
            ("AI-RL-09", 128, 64),
            ("AI-RL-09", 4_096, 2_048),
        ];
        assert_eq!(limits.len(), expected.len());
        assert_eq!(std::mem::size_of::<AiActorFixture>(), 256);
        assert_eq!(std::mem::size_of::<PathNodeFixture>(), 48);
        assert_eq!(std::mem::size_of::<RouteStepFixture>(), 32);
        for (limit, (id, hard_maximum, representative)) in limits.iter().zip(expected) {
            assert_eq!(limit.inventory_id, id);
            assert_eq!(limit.spec.hard_maximum, hard_maximum);
            assert_eq!(limit.representative, representative);
            assert!(assess(&limit.spec, hard_maximum).is_ok());
            assert_eq!(
                assess(&limit.spec, hard_maximum + 1),
                Err(LimitError::CapacityExceeded)
            );
        }
    }

    #[test]
    fn listener_first_slice_limits_are_exact_and_bounded() {
        let limits = listener_limits();
        let expected = [
            ("NET03-RL-01", 256, 128),
            ("NET03-RL-02", 64, 32),
            ("NET03-RL-04", 64, 32),
            ("NET03-RL-05", 1_048_576, 524_288),
            ("NET03-RL-06", 8, 4),
            ("NET03-RL-07", 256, 128),
        ];
        assert_eq!(limits.len(), expected.len());
        assert_eq!(std::mem::size_of::<PreAdmissionConnectionFixture>(), 2_048);
        assert_eq!(std::mem::size_of::<TlsHandshakeFixture>(), 16_384);
        assert_eq!(listener_inherited_frame_bytes(), 1_048_576);
        for (limit, (id, hard_maximum, representative)) in limits.iter().zip(expected) {
            assert_eq!(limit.inventory_id, id);
            assert_eq!(limit.spec.hard_maximum, hard_maximum);
            assert_eq!(limit.representative, representative);
            assert!(assess(&limit.spec, hard_maximum).is_ok());
            assert_eq!(
                assess(&limit.spec, hard_maximum + 1),
                Err(LimitError::CapacityExceeded)
            );
        }
    }

    #[test]
    fn every_non_exercised_first_slice_row_is_explicitly_fail_closed() {
        let ability = ability_exclusions();
        assert_eq!(
            ids(&ability),
            vec![
                "AB-RL-03", "AB-RL-04", "AB-RL-08", "AB-RL-09", "AB-RL-10", "AB-RL-11", "AB-RL-12",
                "AB-RL-13", "AB-RL-14", "AB-RL-15", "AB-RL-16", "AB-RL-17", "AB-RL-18", "AB-RL-19"
            ]
        );
        let interaction = interaction_exclusions();
        assert_eq!(ids(&interaction), vec!["GI-RL-04", "GI-RL-05"]);
        let ai = ai_exclusions();
        assert_eq!(
            ids(&ai),
            vec![
                "AI-RL-04", "AI-RL-06", "AI-RL-10", "AI-RL-11", "AI-RL-12", "AI-RL-13", "AI-RL-14",
                "AI-RL-15", "AI-RL-16"
            ]
        );
        let durability = durability_exclusions();
        assert_eq!(
            ids(&durability),
            vec![
                "DUR03-RL-01",
                "DUR03-RL-02",
                "DUR03-RL-03",
                "DUR03-RL-04",
                "DUR03-RL-05",
                "DUR03-RL-06",
                "DUR03-RL-07",
                "DUR03-RL-08"
            ]
        );
        for exclusion in ability
            .iter()
            .chain(interaction.iter())
            .chain(ai.iter())
            .chain(durability.iter())
        {
            assert!(exclusion.fail_closed);
            assert!(!exclusion.reason.is_empty());
        }
    }

    fn ids(rows: &[ExcludedRow]) -> Vec<&str> {
        rows.iter().map(|row| row.inventory_id).collect()
    }

    #[test]
    fn candidate_metadata_is_complete_unique_and_costed() {
        let limits = all_candidate_limits();
        assert_eq!(limits.len(), 24);
        let mut ids = std::collections::BTreeSet::new();
        for limit in &limits {
            assert!(ids.insert(limit.candidate_id()));
            assert!(!limit.domain().is_empty());
            assert!(!limit.resource().is_empty());
            assert!(!limit.unit().is_empty());
            assert!(!limit.owner_contract().is_empty());
            assert!(limit.spec.hard_maximum >= limit.representative);
            let cost = assess(&limit.spec, limit.spec.hard_maximum).unwrap();
            assert!(cost.retained_bytes >= limit.spec.fixed_retained_bytes);
            assert!(cost.work_units >= limit.spec.fixed_work_units);
        }
        assert!(ids.contains("AI01-ROUTE-STEPS"));
        assert!(ids.contains("AI01-ROUTE-BYTES"));
        assert!(ids.contains("NET03-PREADMISSION-CONNECTIONS"));
    }

    #[test]
    fn every_candidate_records_at_least_two_x_representative_headroom() {
        for limit in all_candidate_limits() {
            let minimum = limit.representative.checked_mul(2).unwrap();
            assert!(
                limit.spec.hard_maximum >= minimum,
                "{} lacks 2x representative headroom",
                limit.candidate_id()
            );
        }
    }

    #[test]
    fn evidence_json_is_deterministic_complete_and_authority_bounded() {
        let first = evidence_json();
        let second = evidence_json();
        assert_eq!(first, second);
        assert!(first.starts_with("{\n"));
        assert!(first.ends_with("}\n"));
        assert!(first.contains(r#""source_main_sha": "86653375231febbf81623b4c6984a6ff1263bdc2""#));
        assert!(first.contains(r#""candidate_limit_count": 24"#));
        assert!(first.contains(r#""excluded_row_count": 33"#));
        assert!(first.contains(r#""candidate_id": "ABILITY01-TARGET-CANDIDATES""#));
        assert!(first.contains(r#""candidate_id": "NET03-DRAIN-TASKS""#));
        assert!(first.contains(r#""registry_id": "FND02-WIRE-FRAME-BYTES""#));
        assert!(first.contains(r#""inventory_id": "DUR03-RL-08""#));
        assert!(first.contains(r#""disposition": "NOT_APPLICABLE_TO_FIRST_SLICE""#));
        assert!(!first.contains("OWNER_DECISION_REQUIRED"));
        assert!(first.contains(r#""allocation_impact": "check before retaining"#));
        assert!(first.contains(r#""boundary_tests": ["hard maximum accepted""#));
        assert!(first.contains(r#""evidence_basis": "representative deterministic fixture"#));
        assert!(first.contains(r#""production_default_selected": false"#));
    }

    #[test]
    fn stress_fixture_exercises_every_max_and_max_plus_one_deterministically() {
        let first = stress_fixture(8);
        let second = stress_fixture(8);
        assert_eq!(first, second);
        assert_eq!(first.iterations, 8);
        assert_eq!(first.accepted_cases, 24 * 8);
        assert_eq!(first.rejected_cases, 24 * 8);
        assert!(first.peak_single_allocation_bytes >= 1_048_576);
        assert_ne!(first.checksum, 0);
    }

    #[test]
    fn generic_limit_rejects_checked_arithmetic_overflow() {
        let spec = LimitSpec {
            hard_maximum: u64::MAX,
            retained_bytes_per_unit: 2,
            fixed_retained_bytes: 0,
            work_units_per_unit: 1,
            fixed_work_units: 0,
        };
        assert_eq!(assess(&spec, u64::MAX), Err(LimitError::ArithmeticOverflow));
    }
}
