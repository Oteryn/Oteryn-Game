#!/usr/bin/env python3
"""Exercise the actual run.sh stage, traps and monitor with disposable commands."""

import os
import re
import selectors
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

# The workflows run this file only; it carries the node boot tool tests with it.
from test_node_boot_tools import (  # noqa: F401
    FailureSummaryTests,
    KnobTests,
    PgObserveTests,
    ReuseTests,
    SeamPassThroughTests,
    StageDumpTests,
    TimelineTests,
)

SOURCE = Path(__file__).resolve().with_name("run.sh")
# An explicit source path also runs the same fixture against a saved baseline.
if len(sys.argv) > 1 and sys.argv[1].endswith("run.sh"):
    SOURCE = Path(sys.argv.pop(1))
TEXT = SOURCE.read_text()


def function(name):
    ending = (
        r"^PY_RUNTIME_DIAGNOSTICS\n.*?^}"
        if name == "start_runtime_diagnostics"
        else r"^}"
    )
    found = re.search(rf"(?ms)^{name}\(\).*?{ending}", TEXT)
    return found.group(0) if found else ""


class RuntimeEvidenceTests(unittest.TestCase):
    def test_served_content_revision_is_the_characters_interpretation(self):
        # NBSEAM: the quest and Task Board owners refuse a Character whose stored progression
        # content differs from the served readiness.content_revision (ProgressionContextMismatch).
        interpretation = (
            re.search(r"^readonly INTERPRETATION=\(([^)]*)\)$", TEXT, re.MULTILINE)
            .group(1)
            .split()
        )
        readiness = TEXT.split("[readiness]\n", 1)[1].split("\n[", 1)[0]
        served = re.search(
            r'^content_revision = "([^"]*)"$', readiness, re.MULTILINE
        ).group(1)
        self.assertEqual(
            served.replace("${INTERPRETATION[2]}", interpretation[2]), interpretation[2]
        )
        harness = SOURCE.parents[3].joinpath("apps/game-server/src/qualification.rs")
        if harness.exists():
            rust = harness.read_text()
            content = re.search(
                r'const INTERPRETATION: \[&str; 4\] = \[\s*"[^"]*",\s*"[^"]*",\s*"([^"]*)"',
                rust,
            ).group(1)
            self.assertEqual(content, interpretation[2])
            self.assertNotRegex(rust, r'content_revision"?: "')

    def test_logout_failures_keep_only_closed_schema_stage_and_kind(self):
        monitor = function("start_runtime_diagnostics").split(
            "<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1
        )[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        lines = [
            (
                "oteryn-game-server event=session_durability_failed stage=logout_settle kind=holder_busy "
                "database_url=secret character=secret payload=secret"
            ),
            "oteryn-game-server event=logout_unknown stage=pvp_refresh kind=unknown",
            "oteryn-game-server event=session_durability_failed stage=secret kind=secret",
        ]
        script = prefix + "\n" + "\n".join(f"report({line!r})" for line in lines)
        result = subprocess.run(
            [sys.executable, "-c", script, "unused", "test", "1", "0"],
            capture_output=True,
            text=True,
            check=True,
        )
        self.assertIn(
            "event=session_durability_failed kind=holder_busy failure_stage=logout_settle",
            result.stdout,
        )
        self.assertIn(
            "event=logout_unknown kind=unknown failure_stage=pvp_refresh", result.stdout
        )
        self.assertNotIn("secret", result.stdout)

    def test_loss_blockers_keep_closed_stages_without_sensitive_fields(self):
        monitor = function("start_runtime_diagnostics").split(
            "<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1
        )[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        expected = [
            ("logout_busy", "foreground_demand"),
            ("logout_busy", "admitted_binding"),
            ("logout_busy", "activity_scope"),
            ("logout_busy", "zones_before_reconcile"),
            ("logout_busy", "newhaven_travel"),
            ("logout_busy", "spell_reconcile"),
            ("logout_busy", "zones_after_reconcile"),
            ("logout_busy", "release_fence"),
            ("logout_busy", "release_not_applicable"),
            ("logout_busy", "release_retry_lifted"),
            ("control_loss_reconcile_pending", "item_effects"),
            ("control_loss_reconcile_pending", "gourmet_checkpoint"),
            ("control_loss_reconcile_pending", "native_cast"),
            ("control_loss_reconcile_pending", "soul_war_source_use"),
            ("control_loss_reconcile_pending", "familiar_cast"),
            ("control_loss_reconcile_pending", "world_items"),
            ("control_loss_reconcile_pending", "parameters"),
            ("control_loss_reconcile_pending", "training_save"),
            ("control_loss_reconcile_pending", "pending_spell_commit"),
            ("control_loss_reconcile_pending", "actor_unreserved"),
            ("control_loss_reconcile_pending", "newhaven_travel"),
            ("session_durability_failed", "loss_session"),
            ("session_durability_failed", "loss_session_time"),
            ("session_durability_failed", "training_session"),
            ("session_durability_failed", "training_readback"),
            ("session_durability_failed", "stamina_session"),
            ("session_durability_failed", "hunting_stamina_readback"),
            ("session_durability_failed", "proficiency_readback"),
            ("session_durability_failed", "appearance_observation"),
            ("character_authority_check_failed", "recovery_fence"),
            ("character_authority_check_failed", "offline_training"),
            ("character_authority_check_failed", "npc_purchase"),
            ("character_authority_check_failed", "creation"),
            ("character_authority_check_failed", "bazaar"),
            ("character_authority_check_failed", "revision_chain"),
            ("character_authority_check_failed", "appearance"),
            ("character_authority_check_failed", "build_chain"),
            ("character_authority_check_failed", "proficiency_history"),
            ("character_authority_check_failed", "familiar_chain"),
            ("character_authority_check_failed", "spell_familiar_group"),
            ("character_authority_check_failed", "spell_item_chain"),
            ("character_authority_check_failed", "audit_payload"),
        ]
        lines = [
            f"oteryn-game-server event={event} stage={stage} "
            "account=secret transport=secret receipt=secret"
            for event, stage in expected
        ]
        lines.append("oteryn-game-server event=logout_busy stage=secret")
        script = prefix + "\n" + "\n".join(f"report({line!r})" for line in lines)
        result = subprocess.run(
            [sys.executable, "-c", script, "unused", "test", "1", "0"],
            capture_output=True,
            text=True,
            check=True,
        )
        for event, stage in expected:
            self.assertIn(f"event={event} failure_stage={stage}", result.stdout)
        self.assertNotIn("secret", result.stdout)

    def test_logout_release_retry_keeps_closed_prerequisite_stages(self):
        monitor = function('start_runtime_diagnostics').split("<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1)[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        expected = (
            'conditions', 'death', 'drome', 'familiar', 'gourmet', 'kill_rewards',
            'offline_training', 'pending_items', 'spell_training', 'timed_items',
            'wheel_or_monk', 'write_fence',
        )
        # Keep the cases separate from the per-event cap.
        for offset in range(0, len(expected), 8):
            batch = expected[offset:offset + 8]
            lines = [f'oteryn-game-server event=logout_release_retry stage={stage} account=secret'
                     for stage in (*batch, 'secret')]
            script = prefix + '\n' + '\n'.join(f'report({line!r})' for line in lines)
            result = subprocess.run([sys.executable, '-c', script, 'unused', 'test', '1', '0'],
                                    capture_output=True, text=True, check=True)
            for stage in batch:
                self.assertIn(f'event=logout_release_retry failure_stage={stage}', result.stdout)
            self.assertNotIn('secret', result.stdout)

    def test_loss_publication_and_grace_keep_only_fixed_results_and_stages(self):
        monitor = function("start_runtime_diagnostics").split(
            "<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1
        )[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        expected = [
            f"event=control_loss_result kind={kind}"
            for kind in (
                "recorded",
                "resumed_history",
                "terminal",
                "unknown",
                "not_applicable",
                "refused",
            )
        ]
        expected.extend(
            [
                "event=control_loss_publication stage=entry",
                "event=control_loss_publication kind=pending stage=loss_readback",
                "event=control_loss_publication kind=pending stage=native_publication",
                "event=control_loss_grace stage=entry",
            ]
        )
        expected.extend(
            f"event=control_loss_grace kind=pending stage={stage}"
            for stage in (
                "activity_successor",
                "staff_release",
                "terminal_session",
                "combat_hold",
                "foreground_demand",
                "death_save",
                "activity_return",
                "activity_publish",
                "item_effects",
                "release_fence",
                "familiar_save",
                "training_save",
                "wheel_gift",
                "monk_save",
                "gourmet_checkpoint",
                "condition_save",
                "kill_rewards",
                "release_not_applicable",
                "release_not_expired",
                "release_settle",
                "release_commit",
                "retire",
            )
        )
        # Separate schema coverage from the per-event cap exercised below.
        for offset in range(0, len(expected), 8):
            batch = expected[offset : offset + 8]
            lines = [
                f"oteryn-game-server {fields} account=secret character=secret "
                "transport=secret receipt=secret connection_generation=secret "
                "lease_generation=secret state=secret database_url=secret payload=secret"
                for fields in batch
            ]
            lines.append(
                "oteryn-game-server event=control_loss_grace stage=secret kind=secret"
            )
            script = prefix + "\n" + "\n".join(f"report({line!r})" for line in lines)
            result = subprocess.run(
                [sys.executable, "-c", script, "unused", "test", "1", "0"],
                capture_output=True,
                text=True,
                check=True,
            )
            for fields in batch:
                self.assertIn(fields.replace("stage=", "failure_stage="), result.stdout)
            self.assertNotIn("secret", result.stdout)

    def test_grace_deduplicates_before_numbers_and_keeps_event_and_total_caps(self):
        monitor = function("start_runtime_diagnostics").split(
            "<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1
        )[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        lines = [
            f"oteryn-game-server event=control_loss_grace stage=combat_hold kind=pending "
            f"ts={index} ms={index} server_sequence={index}"
            for index in range(200)
        ]
        lines.extend(
            f"oteryn-game-server event=control_loss_grace stage={stage} kind=pending"
            for stage in (
                "foreground_demand",
                "death_save",
                "activity_return",
                "activity_publish",
                "item_effects",
                "release_fence",
                "familiar_save",
                "training_save",
                "wheel_gift",
                "monk_save",
                "gourmet_checkpoint",
                "condition_save",
                "kill_rewards",
                "release_commit",
                "retire",
            )
        )
        # Exhausting grace's own budget leaves space for the lifecycle result.
        lines.append("oteryn-game-server event=control_loss_result kind=recorded")
        for event in (
            "serve_end",
            "serve_command_end",
            "command_guard_failed",
            "join_prepare_failed",
            "listener_poll_slow",
            "listener_connection_poll_slow",
            "pass_dropped",
            "step_rejected_dawnport",
            "step_rejected_source_map",
            "step_rejected_dead",
            "step_rejected_lane",
            "step_rejected_pvp_locks",
            "step_rejected_stale_position",
            "step_rejected_pending_spell",
            "step_rejected_drome",
            "step_rejected_movement",
        ):
            lines.extend(
                f"oteryn-game-server event={event} "
                f"source=crates/gameplay/example_{index}.rs:1:1"
                for index in range(32)
            )
        script = prefix + "\n" + "\n".join(f"report({line!r})" for line in lines)
        result = subprocess.run(
            [sys.executable, "-c", script, "unused", "test", "1", "0"],
            capture_output=True,
            text=True,
            check=True,
        )
        records = [
            line
            for line in result.stdout.splitlines()
            if line.startswith("NODE_BOOT_RUNTIME ")
        ]
        grace = [line for line in records if "event=control_loss_grace " in line]
        self.assertEqual(len(records), 128)
        self.assertEqual(len(grace), 16)
        self.assertEqual(sum("failure_stage=combat_hold " in line for line in grace), 2)
        self.assertIn("event=control_loss_grace records=16", result.stdout)
        self.assertIn("event=control_loss_result kind=recorded", result.stdout)

    def test_appearance_failures_keep_only_closed_classes(self):
        monitor = function("start_runtime_diagnostics").split(
            "<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1
        )[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        for event, classes in (
            (
                "appearance_initial_failed",
                ("observation", "mappings", "attach", "encode"),
            ),
            (
                "appearance_observation_failed",
                ("content", "fence", "revision", "state", "visibility", "conditions"),
            ),
        ):
            lines = [
                f"oteryn-game-server event={event} class={kind} "
                "account=secret character=secret session=secret role=secret "
                "error=secret payload=secret"
                for kind in classes
            ]
            lines.append(f"oteryn-game-server event={event} class=secret")
            script = prefix + "\n" + "\n".join(f"report({line!r})" for line in lines)
            result = subprocess.run(
                [sys.executable, "-c", script, "unused", "test", "1", "0"],
                capture_output=True,
                text=True,
                check=True,
            )
            for kind in classes:
                self.assertIn(f"event={event} class={kind}", result.stdout)
            self.assertNotIn("secret", result.stdout)

    def test_startup_integrity_failure_reports_only_closed_check_once(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            log = root / "node.log"
            lines = [
                (
                    "oteryn-game-server module=durability "
                    "event=character_authority_check_failed stage=creation "
                    "account=secret error=secret sql=secret"
                ),
                (
                    "oteryn-game-server module=durability "
                    "event=character_authority_check_failed stage=secret"
                ),
                (
                    "oteryn-game-server module=other "
                    "event=character_authority_check_failed stage=bazaar"
                ),
                "oteryn-game-server module=durability event=other stage=build_chain",
            ]
            log.write_text(
                "\n".join(lines * 30)
                + "\n"
                + ("x" * 4097)
                + lines[0]
                + "\n"
                + "oteryn-game-server module=durability "
                "event=character_authority_check_failed stage=audit_payload\n"
            )
            script = (
                f"node_log={log}\nNODE_BOOT_WORLD=1\n"
                + function("report_character_authority_failures")
                + "\nreport_character_authority_failures superseding_replacement\n"
            )
            result = subprocess.run(
                ["bash", "-c", script], capture_output=True, text=True, check=True
            )
            self.assertEqual(
                result.stdout.splitlines(),
                [
                    (
                        "NODE_BOOT_CHARACTER_AUTHORITY_CHECK stage=superseding_replacement "
                        "world_bundle=1 check=creation"
                    ),
                    (
                        "NODE_BOOT_CHARACTER_AUTHORITY_CHECK stage=superseding_replacement "
                        "world_bundle=1 check=audit_payload"
                    ),
                ],
            )
            self.assertNotIn("secret", result.stdout)

    def test_step_rejections_keep_only_closed_schema_fields(self):
        monitor = function("start_runtime_diagnostics").split(
            "<<'PY_RUNTIME_DIAGNOSTICS' &\n", 1
        )[1]
        prefix = monitor.split("print(f'NODE_BOOT_RUNTIME_MONITOR", 1)[0]
        events = [
            f"step_rejected_{branch}"
            for branch in (
                "dawnport",
                "source_map",
                "dead",
                "lane",
                "pvp_locks",
                "stale_position",
                "pending_spell",
                "drome",
                "movement",
                "holder_busy",
            )
        ]
        lines = [
            f"oteryn-game-server event={event} kind=Actor character=secret "
            "position=secret database_url=secret"
            for event in events
        ]
        lines.append("oteryn-game-server event=step_rejected_movement kind=secret")
        script = prefix + "\n" + "\n".join(f"report({line!r})" for line in lines)
        result = subprocess.run(
            [sys.executable, "-c", script, "unused", "test", "1", "0"],
            capture_output=True,
            text=True,
            check=True,
        )
        for event in events:
            self.assertIn(f"event={event} ", result.stdout)
        self.assertIn("kind=Actor", result.stdout)
        self.assertNotIn("secret", result.stdout)

    def exercise(self, signum):
        inbound_phases = (
            "frame",
            "frame_error",
            "held",
            "step_due",
            "probe",
            "server_message",
            "serene",
            "quest_log",
            "chat",
            "visibility",
            "combat",
            "chase",
            "target_move",
            "npc",
        )
        authority_stages = (
            "recovery_fence",
            "offline_training",
            "npc_purchase",
            "creation",
            "bazaar",
            "revision_chain",
            "appearance",
            "build_chain",
            "proficiency_history",
            "familiar_chain",
            "spell_familiar_group",
            "spell_item_chain",
            "audit_payload",
        )
        # Cover every label across both signal fixtures without exceeding the
        # monitor's limit of sixteen records for this event.
        authority_stages = (
            authority_stages[::2]
            if signum == signal.SIGTERM
            else authority_stages[1::2]
        )
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            work = root / "work"
            work.mkdir()
            (work / "vars").touch()
            (work / "node.log").touch()
            definitions = "\n".join(
                function(name)
                for name in [
                    "start_runtime_diagnostics",
                    "stop_runtime_diagnostics",
                    "stop_pg_sampler",
                    "pg_observability_dump",
                    "interrupted",
                    "cleanup",
                    "stage",
                ]
            )
            trap_lines = "\n".join(re.findall(r"^trap .+$", TEXT, re.MULTILINE))
            script = f"""#!/usr/bin/env bash
set -Eeuo pipefail
WORK={work}
BASE={root / "base"}
VARS="$WORK/vars"
node_log="$WORK/node.log"
NODE_PID=""
RUNTIME_DIAGNOSTICS_PID=""
PG_SAMPLER_PID=""
ACTIVE_STAGE=setup
NODE_BOOT_WORLD=1
NODE_BOOT_SPELLS=0
NODE_BOOT_PROFILE=sorcerer
SERVICE_USER=fixture
PG_CONTAINER=fixture
result=NODE_BOOT_FAIL
declare -A STATUS=()
ORDER=()
evidence() {{ printf 'NODE_BOOT_EVIDENCE %s\\n' "$*"; }}
compose() {{ :; }}
docker() {{ :; }}
sudo() {{ :; }}
{definitions}
{trap_lines}
blocked() {{
  printf 'FIXTURE_RUNNING\\n'
  while [[ ! -e "$WORK/release" ]]; do sleep 1; done
}}
echo 'NODE_BOOT_RESULT=NODE_BOOT_PASS'
stage seam_stages -- blocked
"""
            harness = root / "harness.sh"
            harness.write_text(script)
            output = bytearray()
            process = subprocess.Popen(
                ["bash", str(harness)],
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)

            def until(marker, seconds):
                deadline = time.monotonic() + seconds
                while marker not in output and time.monotonic() < deadline:
                    for key, _ in selector.select(max(0, deadline - time.monotonic())):
                        block = os.read(key.fd, 65536)
                        if not block:
                            return False
                        output.extend(block)
                return marker in output

            try:
                self.assertTrue(until(b"FIXTURE_RUNNING", 3), output.decode())
                # On the old script this is deliberately absent; the later assertions
                # then demonstrate the lost diagnostics rather than a fixture failure.
                until(b"NODE_BOOT_RUNTIME_MONITOR state=watching", 2)
                with (work / "node.log").open("a") as log:
                    # Slow-pass noise must not consume the slots for the later cause.
                    for index in range(2000):
                        log.write(
                            f"DURABILITY_PASS_SLOW caller=crates/gameplay/caller_{index}.rs:1:1 "
                            "held_ms=6000 deadline_exceeded=false\n"
                        )
                    log.write(
                        "DURABILITY_PASS_RETIRED caller=crates/gameplay/example.rs:7:2 reason=no_evidence\n"
                    )
                    log.write(
                        "oteryn-game-server event=listener_connection_poll_slow ms=5650 ready=false\n"
                    )
                    log.write(
                        "oteryn-game-server event=serve_end reason=control_lost path=liveness_missed "
                        "source=crates/gameplay/serve.rs:8:3 next_command_id=17 server_sequence=23 "
                        "password=do-not-print-this-secret\n"
                    )
                    for phase in inbound_phases:
                        log.write(
                            f"oteryn-game-server event=serve_end reason=disconnected phase={phase} "
                            "source=crates/gameplay/serve.rs:9:3 next_command_id=18 server_sequence=24\n"
                        )
                    log.write(
                        "oteryn-game-server event=serve_end reason=disconnected "
                        "phase=unapproved-phase-secret "
                        "source=crates/gameplay/serve.rs:10:3 next_command_id=19 server_sequence=25\n"
                    )
                    for stage in authority_stages:
                        log.write(
                            "oteryn-game-server ts=2 level=warn module=durability "
                            f"event=character_authority_check_failed stage={stage}\n"
                        )
                    log.write(
                        "oteryn-game-server ts=3 level=warn module=durability "
                        "event=character_authority_check_failed stage=unapproved-stage-secret\n"
                    )
                    log.write(
                        "oteryn-game-server ts=1 level=error module=test event=admission_refused "
                        'error="permission denied for table secret-table" '
                        "database_url=postgres://fixture:do-not-print-this-secret@db/fixture\n"
                    )
                # Deliver INT/TERM to the fixture group while its stage waits on tee.
                # GitHub entry-shell cancellation is a separate signal boundary.
                until(b"NODE_BOOT_RUNTIME_LIMIT", 3)
                os.killpg(process.pid, signum)
                remainder, _ = process.communicate(timeout=5)
                output.extend(remainder)
            finally:
                selector.close()
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            text = output.decode()
            self.assertEqual(process.returncode, 128 + signum, text)
            self.assertEqual(text.count("cleanup=complete"), 1, text)
            self.assertIn(
                "event=serve_end reason=control_lost path=liveness_missed", text
            )
            for phase in inbound_phases:
                self.assertIn(
                    f"event=serve_end reason=disconnected phase={phase} ", text
                )
            self.assertIn(
                "event=serve_end reason=disconnected source=crates/gameplay/serve.rs:10:3 ",
                text,
            )
            self.assertNotIn("unapproved-phase-secret", text)
            for stage in authority_stages:
                self.assertIn(
                    "event=character_authority_check_failed "
                    f"failure_stage={stage} check_stage={stage} ",
                    text,
                )
            self.assertIn("event=character_authority_check_failed ts=3 ", text)
            self.assertNotIn("unapproved-stage-secret", text)
            self.assertIn("event=durability_pass_retired", text)
            self.assertIn(
                "event=listener_connection_poll_slow ready=false ms=5650", text
            )
            self.assertIn("signature=permission_denied", text)
            self.assertNotIn("do-not-print-this-secret", text)
            self.assertNotIn("secret-table", text)
            self.assertLess(
                text.index("event=serve_end"), text.index("cleanup=complete")
            )
            results = re.findall(r"^NODE_BOOT_RESULT=(.+)$", text, re.MULTILINE)
            self.assertEqual(results, ["NODE_BOOT_PASS", "NODE_BOOT_FAIL"], text)
            records = [
                line
                for line in text.splitlines()
                if line.startswith("NODE_BOOT_RUNTIME ")
            ]
            self.assertLessEqual(len(records), 128, text)
            self.assertIn("NODE_BOOT_RUNTIME_LIMIT", text)
            self.assertIn("world_bundle=1", text)
            self.assertFalse(work.exists(), text)

    def test_term_preserves_bounded_safe_evidence_before_cleanup(self):
        self.exercise(signal.SIGTERM)

    def test_int_preserves_bounded_safe_evidence_before_cleanup(self):
        self.exercise(signal.SIGINT)


class KnownFailRowsTests(unittest.TestCase):
    """A FAIL row fails its stage unless known_fail_rows.txt lists it."""

    def rows(self, log, status, known):
        helpers = "\n".join(
            re.search(rf"(?m)^{name}\(\).*$", TEXT).group(0)
            for name in ("stage_rows", "known_rows")
        )
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, "stage.log").write_text(log)
            Path(tmp, "known.txt").write_text(known)
            script = (
                helpers
                + '\nstage_rows "$1/stage.log" "$2" | grep -vxFf <(known_rows "$1/known.txt") | paste -sd, - || true\n'
                'stage_rows "$1/stage.log" "$2" | grep -xFf <(known_rows "$1/known.txt") | paste -sd, - || true\n'
            )
            out = subprocess.run(
                ["bash", "-c", "set -Eeuo pipefail\n" + script, "x", tmp, status],
                capture_output=True,
                text=True,
                check=True,
            ).stdout.split("\n")
        return out[0], out[1]

    def test_new_failure_is_not_hidden_and_known_failure_is_listed(self):
        log = (
            "SEAM_EVIDENCE e2e_all scenario=C01.full status=FAIL seconds=1 cause=assertion\n"
            "SEAM_EVIDENCE e2e_all scenario=Q01a.full status=FAIL seconds=1 cause=assertion\n"
            "SEAM_EVIDENCE e2e_all scenario=C03.full status=PASS seconds=1\n"
        )
        known = (
            "# comment\n\nC01.full assertion at C01.full/admission\nC03.full flaky\n"
        )
        self.assertEqual(self.rows(log, "FAIL", known), ("Q01a.full", "C01.full"))
        self.assertEqual(self.rows(log, "PASS", known)[1], "C03.full")

    def test_missing_list_hides_nothing(self):
        log = "SEAM_EVIDENCE e2e_all scenario=C01.full status=FAIL seconds=1\n"
        self.assertEqual(self.rows(log, "FAIL", ""), ("C01.full", ""))

    def test_w19a_actor_failure_fails_the_seam_stage(self):
        log = (
            "e2e_all scenario=W19a.full status=FAIL evidence=Messenger_absent\n"
            "e2e_all scenario=W19a.client status=FAIL evidence=Messenger_absent\n"
        )
        known = SOURCE.with_name("known_fail_rows.txt").read_text()
        self.assertEqual(self.rows(log, "FAIL", known), ("W19a.client,W19a.full", ""))

    def test_w19a_shards_require_the_expected_live_verdict(self):
        helpers = "\n".join(
            re.search(rf"(?m)^{name}\(\).*$", TEXT).group(0)
            for name in ("stage_rows", "known_rows")
        )
        script = "\n".join(
            (
                "set -Eeuo pipefail",
                "WORK=$1; VARS=/dev/null; GAME_SOURCE=$2",
                "OTERYN_E2E_ROW_BATCH=$3; NODE_BOOT_E2E_ONLY=rows",
                "NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0",
                "declare -A STATUS=(); ORDER=()",
                "evidence() { :; }; start_runtime_diagnostics() { :; }",
                "stop_runtime_diagnostics() { :; }; pg_observability_dump() { :; }",
                helpers,
                function("node_boot_row_selection"),
                function("w19a_rows"),
                function("stage"),
                "stage seam_stages -- printf '%s\\n' \"$4\"",
                'echo "verdict=${STATUS[seam_stages]}"',
            )
        )
        full = "e2e_all scenario=W19a.full status=PASS"
        client = "e2e_all scenario=W19a.client status=CAPTURED"
        cases = (
            ("12:1", full, "PASS"),
            ("12:1", client, "FAIL(rows)"),
            ("216:1", client, "PASS"),
            ("216:1", full, "FAIL(rows)"),
            ("+12:1", "", "FAIL(rows)"),
            ("+12:+1", full, "PASS"),
            ("0:18446744073709551615", "", "FAIL(rows)"),
            ("0:18446744073709551615", full, "FAIL(rows)"),
            ("0:18446744073709551615", full + "\n" + client, "PASS"),
            ("18446744073709551628:1", "", "PASS"),
            ("0:18446744073709551616", "", "PASS"),
            ("all", full + "\n" + client, "PASS"),
            ("all", full, "FAIL(rows)"),
            ("13:1", "", "PASS"),
        )
        with tempfile.TemporaryDirectory() as work:
            for batch, log, verdict in cases:
                with self.subTest(batch=batch, log=log):
                    result = subprocess.run(
                        [
                            "bash",
                            "-c",
                            script,
                            "w19a",
                            work,
                            Path(__file__).resolve().parents[3],
                            batch,
                            log,
                        ],
                        capture_output=True,
                        text=True,
                        check=True,
                    )
                    self.assertEqual(
                        result.stdout.splitlines()[-1], f"verdict={verdict}"
                    )

    def test_seed_list_has_one_reason_per_row(self):
        listed = SOURCE.with_name("known_fail_rows.txt").read_text().splitlines()
        rows = [
            line.split(None, 1)
            for line in listed
            if line.strip() and not line.startswith("#")
        ]
        self.assertTrue(all(len(row) == 2 and row[0].endswith(".full") for row in rows))
        self.assertEqual(len({row[0] for row in rows}), len(rows))


class ChristmasFixtureTests(unittest.TestCase):
    def test_world_seam_passes_normalized_row_batch_to_journeys(self):
        script = r"""
set -Eeuo pipefail
GAME_SOURCE=$PWD; NODE_BOOT_LIBTEST=$1
WORK=$PWD; node_log=$WORK/node.log
export OTERYN_E2E_ROW_BATCH=$2
NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; NODE_BOOT_LIFECYCLE=native_reference
BASE=/x; MAP_REVISION=m; WP5_PKI=/x; ADMIN_URL=u; WORLD_ID=w; CHANNEL_ID=c; WP5_PORT=1
ACCOUNT_ID=a; SECOND_ACCOUNT_ID=b; FRESH_KEY_ID=f; FRESH_SEED_HEX=s
RECOVERY_KEY_ID=r; RECOVERY_SEED_HEX=t
PACKET_ACCOUNT_ID=p; SECOND_PACKET_ACCOUNT_ID=q; LIGHT_ACCOUNT_ID=l
W10_ACCOUNT_ID=w10; C16C_ACCOUNT_ID=c16c; G17_ACCOUNT=g17; G01_ACCOUNT=g01
EXTRA_WORLD_ACCOUNTS=(e); NODE_GAMEPLAY_ENV=(); NODE_CLOCK_FEATURES=(); NODE_SPAWN_FEATURES=()
declare -A STATUS=()
sleep() { :; }
"""
        script += function("node_boot_row_selection") + "\n" + function("seam_stages")
        script += "\nseam_stages\n"
        accounts = dict(re.findall(
            r"^readonly ([A-Z0-9_]+_ACCOUNT(?:_ID)?)=([0-9a-f-]+)$", TEXT, re.MULTILINE
        ))
        with tempfile.TemporaryDirectory() as work:
            package = Path(work, "apps", "game-server")
            package.mkdir(parents=True)
            executable = Path(work, "test-binary")
            executable.write_text(
                '#!/bin/bash\ncase "$1" in\n'
                '  *qualification_spawn_transaction*) echo "test result: ok. 1 passed; 0 failed; 0 ignored;" ;;\n'
                '  *) echo "batch=${OTERYN_E2E_ROW_BATCH-unset}" ;;\nesac\n'
            )
            executable.chmod(0o700)
            for batch, expected in (
                ("+12:+1", "12:1"),
                ("0:18446744073709551615", "0:18446744073709551615"),
            ):
                with self.subTest(batch=batch):
                    result = subprocess.run(
                        ["bash", "-c", script, "w19a", executable, batch],
                        cwd=work,
                        env={**os.environ, **accounts},
                        capture_output=True,
                        text=True,
                        check=True,
                    )
                    self.assertEqual(
                        result.stdout.splitlines()[-1], f"batch={expected}"
                    )

    def test_assignment_waits_for_rendered_christmas_epoch(self):
        script = r"""
set -Eeuo pipefail
GAME_SOURCE=$1; node_log=$2
NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; NODE_BOOT_UNPRIVILEGED=0
NODE_BOOT_E2E_ONLY=rows; OTERYN_E2E_ROW_BATCH=12:1
BASE=$GAME_SOURCE; SERVICE_UID=123; WORLD_ID=world; CHANNEL_ID=channel
WORLD_DIGEST=digest; MAP_REVISION=revision; NODE_ACTIVATION_WAIT_SECONDS=1
node_field() { echo value; }
remember() { printf '%s=%s\n' "$1" "$2"; }
ops() { case "$*" in *ungranted.json*|*content-stale.json*) return 6 ;; esac; }
node_deployment_grants() { :; }
assignment_report_args() { :; }
psql_admin() {
    case "$(cat)" in
        "PREPARE node_boot_christmas"*) echo 1 ;;
        "PREPARE node_boot_world_epoch"*) printf 'f\nf\n' ;;
        "SELECT count(*)"*) echo 1 ;;
        *) return 1 ;;
    esac
}
sudo() { echo '123 600'; }
evidence() { :; }
fail() { echo "$1" >&2; return 1; }
seq() { echo 1; }
sleep() { :; }
"""
        # Captured from oteryn_error_codes::Line::render with and without a boot trace.
        calendar = (
            "oteryn-game-server ts=0 level=info module=node event=seasonal_calendar "
        )
        christmas = r'detail="reset_epoch=1 active=[\"christmas\"]"'
        script += "\n".join(
            function(name)
            for name in (
                "node_boot_row_selection",
                "w19a_rows",
                "await_log",
                "node_complete_assignment",
            )
        )
        script += '\nnode_complete_assignment "$SECONDS"\n'
        with tempfile.TemporaryDirectory() as work:
            log = Path(work, "node.log")
            for detail, ready in (
                (christmas, True),
                ("trace=1234 " + christmas, True),
                (r'detail="reset_epoch=0 active=[]"', False),
                (r'detail="reset_epoch=1 active=[\"not_christmas\"]"', False),
            ):
                with self.subTest(detail=detail):
                    log.write_text(
                        "event=content_activated detail=activation_sequence=1 \n"
                        "event=readiness detail=ready=true\n" + calendar + detail + "\n"
                    )
                    result = subprocess.run(
                        [
                            "bash",
                            "-c",
                            script,
                            "w19a",
                            Path(__file__).resolve().parents[3],
                            log,
                        ],
                        capture_output=True,
                        text=True,
                        check=False,
                    )
                    self.assertEqual(
                        result.returncode, 0 if ready else 1, result.stderr
                    )
                    if ready:
                        self.assertIn(
                            "node_seasonal_fixture=christmas_epoch_1", result.stdout
                        )
                    else:
                        self.assertIn("missing node event", result.stderr)
                        self.assertNotIn("node_seasonal_fixture=", result.stdout)

    def test_reused_world_checks_christmas_before_running_seam(self):
        dispatch = re.search(
            r"(?ms)^seam_stage_name=seam_stages\n.*?(?=^else\n)", TEXT
        ).group(0)
        script = "\n".join(
            (
                "set -Eeuo pipefail",
                "NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; NODE_BOOT_REUSE_STACK=1",
                "NODE_BOOT_E2E_ONLY=rows; OTERYN_E2E_ROW_BATCH=$1",
                '[[ -z "$2" ]] || kept_seasonal_fixture=$2',
                "trap 'echo \"${result:-}\"' EXIT",
                'stage() { echo "ran $1"; }',
                function("node_boot_row_selection"),
                function("w19a_rows"),
                dispatch,
                "fi",
            )
        )
        for batch, fixture, allowed in (
            ("0:1", "", True),
            ("13:1", "", True),
            ("12:1", "", False),
            ("216:1", "", False),
            ("+12:1", "", False),
            ("0:18446744073709551615", "", False),
            ("12:1", "christmas_epoch_1", True),
            ("216:1", "christmas_epoch_1", True),
        ):
            with self.subTest(batch=batch, fixture=fixture):
                result = subprocess.run(
                    ["bash", "-c", script, "w19a", batch, fixture],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(
                    result.returncode,
                    0 if allowed else 2,
                    result.stdout + result.stderr,
                )
                if allowed:
                    self.assertIn("ran seam_stages", result.stdout)
                else:
                    self.assertEqual(
                        result.stdout.strip(),
                        "BLOCKED reason=kept_stack_requires_christmas_epoch_1",
                    )


if __name__ == "__main__":
    unittest.main()
