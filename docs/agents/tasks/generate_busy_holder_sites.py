"""Generate the busy-holder review inventory from the pinned Rust source."""
import csv
import re
import subprocess
import sys
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import PurePosixPath

STAGING = "43a8b036a0e3d9e05f130b6932b533b4cf7c7573"
C17B = "342c099bcd197f10ecb69baecaaa0c6300a1b267"
GAME = "crates/gameplay/dispatch/src/gameplay_transport"
DB = "crates/gameplay/durability/src/durability"
SERVE = f"{GAME}/connection/serve"
ROUTING = f"{GAME}/connection/routing"
CHECKOUT = r"([.]|::)(try_acquire_(background_)?ready|try_issue_(background_)?semantic_pass(_by)?|try_issue_root(_pass)?)[[:space:]]*\("
END = (
    r"ServeEnd::(Disconnected|ControlLost|Closed|LoggedOut)|"
    r"ConnectionEnd::(AdmittedThenDisconnected|AdmittedThenClosed|AdmittedThenControlLost|"
    r"AdmittedThenLoggedOut|ResumeUnavailable|AdmissionRefused)|"
    r"\b(close_admitted|join_unavailable|send_join_unavailable|close)\(|\bend\.finish(_with_phase)?\("
)
PRODUCER = (
    r"_outcome_unavailable|holder_busy|\bStaleState\b|"
    r"\b(Unavailable|None)[[:space:]]*=>[[:space:]]*(None|return None)\b|\breturn None\b"
)
INDIRECT = (
    r"\.await[?]|\.await[.]ok[(]|[.]ok[(][)][?]|[.]ok_or(_else)?[(]|"
    r"\bNone\b|\b(RootUnavailable|RootPassDeadlineExceeded|[[:alnum:]_]+::Unavailable|"
    r"HolderBusy|Unknown|Disconnect|StaleState)\b"
)
PROJECTION = r'ServeEnd::server\("state_projection_unavailable"\)'
FN = re.compile(r"\bfn\s+([a-zA-Z_]\w*)\s*[<(]")
TEST_PATH = re.compile(r"(^|/)(tests|postgres_tests|lifted_tests|\w+_tests)(/|\.rs$)")


def git(*args):
    result = subprocess.run(["git", *args], capture_output=True, text=True, check=False)
    if result.returncode and (args[0] != "grep" or result.returncode != 1):
        raise SystemExit(result.stderr)
    return result.stdout


@lru_cache(maxsize=None)
def source_context(ref, path):
    function = "<module>"
    functions = {}
    lines = git("show", f"{ref}:{path}").splitlines()
    for number, line in enumerate(lines, 1):
        if match := FN.search(line.split("//", 1)[0]):
            function = match[1]
        functions[number] = function
    return functions, lines


def scan(sites, ref, label, pattern, *paths, production=False):
    for hit in git("grep", "-nI", "-E", pattern, ref, "--", *paths).splitlines():
        _, path, number, source = hit.split(":", 3)
        if production and ("/tests/" in path or path.endswith("tests.rs")):
            continue
        sites[ref, path, int(number), source].add(label)


def disposition(ref, path, function, code, following, labels):
    file = PurePosixPath(path).stem
    if TEST_PATH.search(path) or code.lstrip().startswith("//") or (
        ref == C17B and file == "mod" and function.startswith(("frame_", "serve_end", "server_end"))
    ) or (
        file == "qualification_fixture" and function.startswith(("exercise_", "qualification_spawn_"))
    ):
        return "unrelated", "Fixture or source reference for the production boundary."
    if re.search(r'frame_(encode_failed|write_refused)|sequence_exhausted|command_continuity_invalid', code):
        return "keep", "Encoding, transport or sequence failure retains its existing outcome."
    if re.search(r"\b(StaleConnectionGeneration|StaleState|ProtocolViolation|ControlLost|LoggedOut|CommitOutcomeUnknown)\b", code):
        return "keep", "Keep stale-state, fence, terminal or reconciliation evidence distinct."
    if ("reconcil" in function or re.search(r"pending|Unknown|Disconnect|\.unknown\b", code)
            and "holder checkout" not in labels):
        return "keep", "Preserve pending or historical outcome evidence and its reconciliation."
    if "Ok(None)" in code or "=> None" in code and "Unavailable" not in code:
        return "keep", "Preserve ordinary absence and inapplicable branch semantics."
    if re.search(r"map_err\([^\n]*(encode|write)|write_frame\(", code):
        return "keep", "Frame encoding and writing retain their existing failure handling."
    if "holder checkout" in labels:
        if file == "db":
            return "keep", "Track the primitive checkout and observed return with FIFO wake."
        if (file == "item_forge_live" and function == "execute_item_forge"
                and ".run_with_context" in following):
            return "keep", "A Forge reservation and attempt charge may already be committed."
        if "reconcil" in function or "post_commit" in function:
            return "keep", "Keep receipt lookup and issued work on the existing reconciliation path."
        if ("try_issue_background_semantic_pass" in code
                or file in {"character_chat_name", "current_appearance", "highscore_snapshot"}
                or function == "read_character_progression"
                or function in {"refresh_item_fluid_splashes", "expire_item_fluid_splashes"}):
            return "degrade", "Advisory or scheduled use defers on busy, subject to the caller's authority role."
        if file == "ready_admission":
            return "keep", "Bound readiness and checkout to the shared join deadline and preserve E1051."
        if file == "qualification_fixture":
            return "retry-then-refuse", "Retry the proven unissued checkout within the existing operation budget."
        if path.startswith((f"{DB}/item_use", f"{DB}/item_transfer", f"{DB}/item_forge_live")) or (
            file in {"character_appearance", "terminal_release"} and function.startswith(("commit_", "release", "read_"))
        ) or (file == "pvp_expert" and function == "expert_context"):
            return "retry-then-refuse", "Carry proven pre-issue busy to the owner after resolving existing receipts."
        return "keep", "Track the checkout and preserve the existing caller policy and authority checks here."
    if file in {"setup", "connection"} and (
        "1051" in code or "join_unavailable" in code or "AdmissionRefused" in code
        or "join_snapshot_unavailable" in code or "ResumeUnavailable" in code
    ):
        return "keep", "Retry within the shared join deadline, preserving E1051 and transport close."
    if path.endswith(("/appearance_change/observation.rs", "/entity_names.rs", "/training_save.rs")) or (
        path.endswith("/chat_intent.rs") and "speaker" in function
    ) or (path.startswith(SERVE) and function in {
        "visibility_inner", "chat", "typing", "pvp_expert", "analyser"
    } and re.search(r"[Uu]navailable|HolderBusy", code)):
        return "degrade", "Defer only the busy projection or checkpoint and preserve other failure causes."
    if (file == "command" and re.search(r"Dispatch::(Appearance|House|Vip)\(None\)|holder_busy", code)
            or file == "forge_preview" and "Outcome::Unavailable" in code
            or file in {"item_forge", "appearance_change", "combat_intent", "use_item", "item_move", "equipment_move", "loop_move", "logout"}
            and re.search(r"RootUnavailable|HolderBusy|Err\(\w+Error::Unavailable", code)):
        return "retry-then-refuse", "Split the provenance-tagged pre-issue busy branch and preserve every other outcome."
    if labels == {"indirect producer"} and not re.search(r"[Uu]navailable|[Bb]usy|return None|\.await", code):
        return "unrelated", "Local matching expression supplies no contention or session-end evidence."
    return "keep", "Preserve the expression until its producer proves pre-issue busy or advisory contention."


def main():
    sites = defaultdict(set)
    scan(sites, STAGING, "holder checkout", CHECKOUT, "apps/*.rs", "crates/*.rs")
    scan(sites, STAGING, "holder checkout", r"\bself[.]pool[.](try_acquire|acquire)\(", f"{DB}/db.rs")
    scan(sites, STAGING, "session end", END, f"{GAME}/connection.rs", f"{GAME}/connection/*.rs", production=True)
    scan(sites, STAGING, "route producer", PRODUCER, f"{ROUTING}/*.rs", f"{SERVE}/*.rs", production=True)
    # Routes use Option propagation as well as explicit ends. Include owner adapters
    # whose errors feed those routes and the direct ServeEnd sinks.
    scan(sites, STAGING, "indirect producer", INDIRECT,
         f"{ROUTING}/*.rs", f"{SERVE}/*.rs",
         *(f"{GAME}/{module}" for module in (
             "appearance_change.rs", "appearance_change/observation.rs", "chat_intent.rs",
             "composed_admission/admission.rs", "composed_admission/items.rs",
             "composed_admission/release.rs", "composed_admission/resume.rs", "connection.rs",
             "connection/authority/composed.rs", "connection/routing.rs",
             "entity_names.rs", "equipment_move.rs", "forge_preview.rs", "item_forge.rs",
             "item_move.rs", "item_use.rs", "logout.rs", "loop_move.rs", "pvp_expert.rs",
             "rune_use_bridge.rs", "training_save.rs",
         )), production=True)
    scan(sites, C17B, "direct server close", r"ServeEnd::server\(", f"{SERVE}/*.rs")
    scan(sites, C17B, "projection close (71 filter)", PROJECTION, f"{SERVE}/*.rs")

    scan_totals = Counter(label for labels in sites.values() for label in labels)
    assert scan_totals["direct server close"] == 413, scan_totals
    assert scan_totals["projection close (71 filter)"] == 71, scan_totals
    writer = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n",
                        quoting=csv.QUOTE_NONE, quotechar=None)
    writer.writerow(("site", "matched pattern", "consumer", "disposition", "reason"))
    totals = Counter()
    for (ref, path, number, code), labels in sorted(sites.items()):
        functions, lines = source_context(ref, path)
        function = functions[number]
        following = lines[number] if number < len(lines) else ""
        action, reason = disposition(ref, path, function, code, following, labels)
        totals[action] += 1
        matched = f"{ref[:8]} {' + '.join(sorted(labels))} | {' '.join(code.split())}"
        writer.writerow((f"{path}:{number}", matched, function, action, reason))
    for name, count in sorted((totals | scan_totals).items()):
        print(f"{name}\t{count}", file=sys.stderr)
    print(f"total rows\t{len(sites)}", file=sys.stderr)


if __name__ == "__main__":
    main()
