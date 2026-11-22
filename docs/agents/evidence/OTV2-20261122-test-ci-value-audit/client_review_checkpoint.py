"""Build an explicit, reproducible read-coverage checkpoint; does not edit the repository."""
from __future__ import annotations

import csv
import hashlib
import json
import subprocess
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

import argparse
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
SCRATCH = args.output.resolve()
REPO = args.repo.resolve()
SCRATCH.mkdir(parents=True, exist_ok=True)
BASE = '8e9f0e16631aab1beb8416f8d5ca707aa3031083'
PREFIXES = [
    'apps/client/', 'crates/client-runtime/', 'crates/client-ui/',
    'crates/client-ui-bevy/', 'crates/client-domain/', 'crates/client-simulation/',
    'crates/client-assets/', 'crates/input-actions/', 'crates/input-platform/',
    'crates/renderer/', 'crates/synthetic-assets/', 'tests/pre-native-client/',
    'tools/synthetic-client-harness/',
]

# Entries below record actual complete semantic reads, including fixture setup and assertions.
# Merely scanning symbols or obtaining a truncated tool result does not qualify.
FULL = {
    'tests/pre-native-client/Cargo.toml': 'Workspace package identity and dependency.',
    'tests/pre-native-client/src/lib.rs': 'Keep no-config fail-closed API guard; compare overlapping app unit.',
    'crates/client-runtime/src/features/0017-attack/tests/protocol.rs': 'Exact mana refusal text for projected combat result.',
    'crates/client-runtime/src/features/local-spell-action-bar/tests/state.rs': 'Action bar feedback branches; keep small UX contract.',
    'crates/client-domain/src/movement.rs': 'Eight directions and atomic checked-overflow movement.',
    'crates/client-runtime/src/features/0016-quest-log/tests/e2e_all.rs': 'Captured bytes or synthetic fallback; real reducer replay.',
    'tools/synthetic-client-harness/tests/native_login.rs': 'Login form/settings component manually injects completion; not live admission.',
    'apps/client/src/features/0149-raid-schedule/tests/e2e_w04.rs': 'Captured/synthetic reducer replay; malformed-state retention assertion is meaningful.',
    'crates/client-runtime/src/features/0064-target-move/tests/e2e_m01.rs': 'Useful receipt decoder checks; final local String retention equality is tautological.',
    'crates/client-ui/src/features/0081-npc-shop/tests/e2e_w05c.rs': 'Conversation/window reducer replay with actual mutation rollback checks.',
    'crates/input-platform/tests/native_winit.rs': 'Valuable public native ingress test; feature-only file cfg omits default Windows invocation.',
    'crates/client-simulation/src/scene/tests.rs': 'Painter order, displacement, missing floor and lookup error contracts.',
    'crates/client-ui-bevy/tests/frames.rs': 'Real Bevy computed layout/nine-slice geometry; keep.',
    'crates/client-ui-bevy/tests/presentation.rs': 'Real layout constraints with fixture glyph metrics; keep, not raster screenshot coverage.',
    'crates/client-ui-bevy/tests/widgets.rs': 'Pressed/checked/disabled widget projection and control sizing; keep.',
    'crates/client-ui/tests/registrations.rs': 'Independent registry guards plus same-generator build-output parity; narrow oracle limitation.',
    'crates/client-ui/tests/house_furniture_confirmation.rs': 'Exact item/destination/revision confirmation; keep irreversible intent boundary.',
    'crates/client-ui/tests/localization_audit.rs': 'Mix of full prose goldens and stronger numerical/consent/spell semantic checks.',
    'crates/client-runtime/tests/queued_retries.rs': 'Actual admission loop against controlled attempts; retry, expiry, cancellation invariants.',
    'crates/synthetic-assets/src/lib.rs': 'Only test checks valid fixture width and hash length; bounded/deterministic claim is broader than oracle.',
    'apps/client/src/tests.rs': 'No-config rejection overlaps external pre-native package; public input re-export smoke.',
    'tools/synthetic-client-harness/src/main.rs': 'Default synthetic CLI logs action/entity outputs without validating them; explicit --live switch.',
    'crates/client-assets/tests/boss_area_metadata.rs': 'Pinned manifest tamper and invalid/duplicate identity rejection.',
    'crates/renderer/src/tests.rs': 'Transactional surface state transitions, generation, overflow, bounded recovery; keep.',
    'tools/synthetic-client-harness/tests/native_vitals.rs': 'Controlled Stepper drives real async play/view/HUD; component test.',
    'apps/client/src/features/0010-analyser/tests/e2e_n07c.rs': 'Captured/synthetic replay with metadata checks and actual outbox/reducer/reset/logout assertions.',
    'crates/client-runtime/tests/admission_progress.rs': 'Progress publication and cancellation of actual admission loop against controlled peer.',
    'crates/client-simulation/src/lib.rs': 'Single upsert test name overclaims determinism; useful narrow projection smoke.',
    'crates/input-actions/src/lib.rs': '11 meaningful router/lifecycle/privacy checks; priority test does not exercise identifier tie named in title.',
    'apps/client/src/features/local-reference-preferences/character_scope.rs': 'No-assert test fails through bounded polling error; meaningful outgoing save feedback regression.',
    'tools/synthetic-client-harness/src/native_floor_transition.rs': 'No-assert test checks floor/sprite outcome through timeout errors; scripted duplex peer, real Session/view.',
    'apps/client/src/bevy_shell/mod.rs': 'Real shell composition and startup parser test; production event loop explicitly outside headless tests.',
    'apps/client/src/bevy_shell/owner/admission_tests.rs': 'Scripted TLS/HTTP stack to real admission/login UI; queue text/cancellation/no extra attempt assertions.',
    'apps/client/src/bevy_shell/login/tests/text_input.rs': 'Native Bevy editor systems, paste/submission ordering and settings conflict preservation.',
    'crates/client-assets/tests/real_assets.rs': 'Pinned asset acceptance, corruption, patterns, cache bounds, decode timeout; keep real data checks.',
    'crates/input-platform/src/audit_regressions.rs': 'Adapter-to-router modifier side/order and chord lifecycle; keep seam coverage.',
    'crates/client-ui/build.rs': 'Traced same scan/render functions that registration parity test reuses.',
    'crates/input-platform/Cargo.toml': 'No default feature; native-winit opt-in, Windows winit dependency.',
    'tools/synthetic-client-harness/Cargo.toml': 'Synthetic-only tool identity; production crates and test seams.',
    'apps/client/build.rs': 'Windows default or explicit bevy-shell emits cfg(bevy_shell).',
    'apps/client/Cargo.toml': 'Feature and target-dependent native input wiring; validates differing test configurations.',
    'crates/input-platform/src/lib.rs': 'Public winit adapter enabled by any(windows, feature=native-winit).',
    'crates/input-platform/src/winit_reset_tests.rs': 'Real WindowEvent transient/focus resets; keep internal coverage distinct from public ingress test.',
}
PARTIAL = {
    'apps/client/src/lib.rs': {'lines': [[80, 210]], 'note': 'Bootstrap configuration, availability and gameplay entry path only.'},
    'tools/synthetic-client-harness/tests/qualification_player_trade.rs': {'lines': [[1, 175]], 'note': 'Explicit scripted TLS qualification seam; rest not semantically read.'},
    'tools/synthetic-client-harness/src/live/cli.rs': {'lines': [[1, 140]], 'note': 'Live CLI inputs and bounds only; execution/session remainder not fully reviewed.'},
    'apps/client/src/native_entry/test_support.rs': {'lines': [[1, 300]], 'note': 'Local scripted Platform/Gateway/TLS Stack and generated configuration; end of output truncated, no full-read claim.'},
}

rows = list(csv.DictReader((SCRATCH / 'test_value_inventory.tsv').open(), delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n'))
scoped = [r for r in rows if any(r['path'].startswith(p) for p in PREFIXES)]
entries = []
for row in scoped:
    path = row['path']
    entry = dict(row)
    for key in ('bytes', 'rust_test_attributes', 'python_test_definitions', 'python_test_classes', 'ignored_attributes'):
        entry[key] = int(entry[key])
    if path in FULL:
        entry.update(semantic_review_status='FULL_FILE_READ', review_note=FULL[path])
    elif path in PARTIAL:
        entry.update(semantic_review_status='PARTIAL_FILE_READ', reviewed_line_ranges=PARTIAL[path]['lines'], review_note=PARTIAL[path]['note'])
    else:
        entry['semantic_review_status'] = 'STATIC_INDEX_ONLY'
    entry['sha256'] = hashlib.sha256((REPO / path).read_bytes()).hexdigest()
    entries.append(entry)

indexed = {e['path'] for e in entries}
context = []
for path in sorted((set(FULL) | set(PARTIAL)) - indexed):
    data = (REPO / path).read_bytes()
    entry = {'path': path, 'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
    if path in FULL:
        entry.update(semantic_review_status='FULL_FILE_READ', review_note=FULL[path])
    else:
        entry.update(semantic_review_status='PARTIAL_FILE_READ', reviewed_line_ranges=PARTIAL[path]['lines'], review_note=PARTIAL[path]['note'])
    context.append(entry)

changed = subprocess.check_output(['git', 'diff', '--name-only', BASE, '--', *PREFIXES], cwd=REPO, text=True).splitlines()
assert not changed, changed
counts = Counter(e['semantic_review_status'] for e in entries)
env_refs = []
for prefix in PREFIXES:
    for path in (REPO / prefix).rglob('*.rs'):
        if 'E2E_ALL_CAPTURE_DIR' in path.read_text():
            env_refs.append(str(path.relative_to(REPO)))

document = {
    'schema': 'client-test-review-v1',
    'created_utc': datetime.now(timezone.utc).isoformat(),
    'repository': 'Oteryn/Oteryn-Game',
    'source_sha': BASE,
    'worktree_matches_source_sha_for_scoped_paths': not changed,
    'method': 'Read-only static index plus deliberate complete/partial semantic source reads; no Rust tests executed in this lane.',
    'count_caveat': 'Literal test attributes and test-bearing source files are inventory metrics, not compiled test IDs, executed cases, or branch coverage.',
    'scope_prefixes': PREFIXES,
    'summary': {
        'indexed_test_bearing_files': len(entries),
        'literal_rust_test_attributes': sum(e['rust_test_attributes'] for e in entries),
        'ignored_attributes_in_static_index': sum(e['ignored_attributes'] for e in entries),
        'test_bearing_file_status_counts': dict(counts),
        'full_read_indexed_literal_attributes': sum(e['rust_test_attributes'] for e in entries if e['semantic_review_status'] == 'FULL_FILE_READ'),
        'fully_read_context_files': sum(e['semantic_review_status'] == 'FULL_FILE_READ' for e in context),
        'partially_read_context_files': sum(e['semantic_review_status'] == 'PARTIAL_FILE_READ' for e in context),
        'capture_environment_reference_files': len(env_refs),
    },
    'capture_environment_reference_files': sorted(env_refs),
    'files': entries,
    'additional_context_files': context,
    'workflow_context_reads': [{'path': '.github/workflows/merge-group-gate.yml', 'line_ranges': [[635, 745], [1290, 1405]], 'status': 'PARTIAL_FILE_READ_IN_CLIENT_LANE', 'note': 'Full CI lane inventory/report is separate.'}],
    'unknowns': [
        'Compiled per-feature/target case inventories and exact repeated case IDs.',
        'CI runtime, failure/flake history and mutation-test survival for client cases.',
        'Windows test selection execution result and real OS/GPU behavior.',
        'Semantic value of all STATIC_INDEX_ONLY and unread portions of PARTIAL_FILE_READ files.',
        'Pinned production asset data execution in this sparse checkout.',
    ],
}
(SCRATCH / 'client_test_review_manifest.json').write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n')
with (SCRATCH / 'client_test_reviewed_files.tsv').open('w') as out:
    fields = ['path', 'semantic_review_status', 'sha256', 'review_note']
    writer = csv.DictWriter(out, fields, delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n', extrasaction='ignore')
    writer.writeheader()
    writer.writerows(e for e in entries + context if e['semantic_review_status'] != 'STATIC_INDEX_ONLY')
print(json.dumps(document['summary'], indent=2))
