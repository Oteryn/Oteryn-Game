#!/usr/bin/env python3
"""Stage NPC dialogue (greet/farewell/walkaway/send_trade, keyword tree, voice lines) as
WorldProject/v2 Dialogue declarations, one per admitted NPC candidate that has recorded text.

Pure: reads the committed promotion candidates and the two source text-bundle directories, writes one
canonical JSON packet. Sources are compared per NPC; agreeing two-source NPCs and single-source NPCs are
staged, disagreeing two-source NPCs are held for review (DIALOGUE_CONFLICT). No engine writes.

Optional rule D10 (`--transcripts DIR --transcripts-revision SHA`, both required together): tie-break
DIALOGUE_CONFLICT NPCs only (never TEXT_BUNDLE_UNVERIFIED, never agreeing or single-source NPCs) against
recorded Tibia Global in-game NPC transcripts under DIR. Each side's texts that differ between the two
built dialogues are matched, as normalized full-line regexes, against that NPC's own spoken transcript
lines; the side with strictly more matches (and more than zero) wins and is staged with provenance
[winner, 'transcript'] and recorded in a new top-level `resolved` list. Everything else is unaffected,
and a run without `--transcripts` produces byte-identical output to a build without D10 support.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

# Fixed repository root: this tool runs from a scratchpad copy, not from inside the repository
# checkout, so it cannot rely on a relative-to-__file__ walk to find the repo root.
ROOT = Path(__file__).resolve().parents[2]
CANDIDATES = ROOT / 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'
CANARY_REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
CRYSTAL_REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
REVISION = 'definition-r1'
NPC_PREFIX = 'oteryn:npc.'
SLUG = re.compile(r'[a-z0-9]+(?:_[a-z0-9]+)*')
MAX_DEPTH = 8
MESSAGE_KEYS = ('greet', 'farewell', 'walkaway', 'send_trade')
DIALOGUE_FIELDS = ('greet', 'farewell', 'walkaway', 'send_trade', 'keywords', 'voices')
CONTROL_CHARS = frozenset(chr(c) for c in range(0x20) if c != 0x0a) | {chr(0x7f)}


CENSUS = {'canary': ROOT / 'tools/content-schema/npc-authoring/samples/census-canary-47dfd51f.json',
          'crystal': ROOT / 'tools/content-schema/npc-authoring/samples/census-crystal-ff7ede59.json'}


class StageError(Exception):
    pass


def canonical(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def clean_text(text: str) -> str:
    text = text.replace('\r\n', '\n').replace('\r', '\n')
    text = ''.join(ch for ch in text if ch == '\n' or (ord(ch) >= 0x20 and ord(ch) != 0x7f))
    return text.strip()


def has_control_chars(text: str) -> bool:
    return any(ch in CONTROL_CHARS for ch in text)


def leaf_texts(textref: Any) -> list[str]:
    """Flatten a text reference into its ordered leaf strings, un-cleaned."""
    if not isinstance(textref, dict):
        return []
    if 'parts' in textref:
        out: list[str] = []
        for part in textref['parts']:
            out.extend(leaf_texts(part))
        return out
    text = textref.get('text')
    if not isinstance(text, str):
        raise StageError('text reference carries no text: stage bundles converted with --include-text')
    if hashlib.sha256(text.encode('utf-8')).hexdigest() != textref.get('sha256'):
        raise StageError('text does not match its text-reference digest')
    return [text]


def text_parts_cleaned(textref: Any) -> list[str]:
    """Ordered, cleaned, trimmed, non-empty message/reply parts (control chars other than \\n stripped)."""
    parts = [clean_text(t) for t in leaf_texts(textref)]
    return [p for p in parts if p]


def normalize_trigger(keyword: Any) -> str | None:
    if not isinstance(keyword, str):
        return None
    trigger = keyword.strip().lower()
    if not trigger:
        return None
    if has_control_chars(trigger):
        return None
    if len(trigger.encode('utf-8')) > 64:
        return None
    return trigger


def slug_of_trigger(trigger: str) -> str:
    slug = re.sub(r'[^a-z0-9]+', '_', trigger).strip('_')
    if len(slug.encode('utf-8')) > 64:
        slug = slug.encode('utf-8')[:64].decode('utf-8', 'ignore').strip('_')
    return slug or 'k'


def make_unique_key(base: str, used: set[str]) -> str:
    if base not in used:
        used.add(base)
        return base
    index = 2
    while True:
        suffix = f'_{index}'
        budget = max(64 - len(suffix.encode('utf-8')), 1)
        candidate = base.encode('utf-8')[:budget].decode('utf-8', 'ignore').strip('_') + suffix
        if candidate not in used:
            used.add(candidate)
            return candidate
        index += 1


def keyword_words(node: dict) -> set[str]:
    return {str(k).strip().lower() for k in (node.get('keywords') or [])}


def shadowed_by_omitted_sibling(node: dict, omitted: list) -> bool:
    """The engine answers with the first sibling whose keywords match (and whose predicate passes), so an
    earlier sibling that is not emitted here (gated, scripted, an action keyword, or itself dropped) and can
    match the same message decides what this reply really answers; convert.py marks the same shadowing
    for services. An empty keyword matches every message."""
    words = keyword_words(node)
    for previous in omitted:
        other = keyword_words(previous)
        if '' in words or '' in other or words & other:
            return True
    return False


def build_keyword_nodes(nodes: list, depth: int, stats: dict) -> list[dict]:
    if depth > MAX_DEPTH or not nodes:
        return []
    prepared = []  # source order, one entry per node that survives per-node validation
    omitted = []  # earlier siblings that are not emitted
    for node in nodes:
        if node.get('kind') != 'say' or node.get('gate') != 'NONE' or node.get('effect') != 'NONE':
            stats['dropped_non_say_gated'] += 1
            omitted.append(node)
            continue
        if shadowed_by_omitted_sibling(node, omitted):
            stats['dropped_shadowed_by_omitted_sibling'] += 1
            omitted.append(node)
            continue

        reply = text_parts_cleaned(node.get('text'))
        if not reply:
            stats['dropped_no_text'] += 1
            omitted.append(node)
            continue

        flags_src = node.get('flags') or {}
        only_focus = bool(flags_src.get('only_focus'))
        only_unfocus = bool(flags_src.get('only_unfocus'))
        reset = bool(flags_src.get('reset'))
        ungreet = bool(flags_src.get('ungreet'))
        if only_focus and only_unfocus:
            stats['dropped_conflicting_focus_flags'] += 1
            omitted.append(node)
            continue

        move_up_raw = flags_src.get('move_up')
        move_up = None
        if move_up_raw is not None:
            if isinstance(move_up_raw, bool) or not isinstance(move_up_raw, int) or not (1 <= move_up_raw <= depth):
                stats['dropped_bad_move_up'] += 1
                omitted.append(node)
                continue
            move_up = move_up_raw

        raw_keywords = node.get('keywords') or []
        non_empty_raw = [k for k in raw_keywords if not (isinstance(k, str) and k.strip() == '')]
        has_empty = any(isinstance(k, str) and k.strip() == '' for k in raw_keywords)
        fallback = False
        triggers: list[str] = []
        if non_empty_raw:
            triggers = sorted({t for t in (normalize_trigger(k) for k in non_empty_raw) if t})
        elif has_empty:
            fallback = True
        if not triggers and not fallback:
            stats['dropped_no_triggers'] += 1
            omitted.append(node)
            continue

        children = build_keyword_nodes(node.get('children') or [], depth + 1, stats)
        base_key = 'fallback' if fallback else slug_of_trigger(triggers[0])
        prepared.append({
            'base_key': base_key, 'fallback': fallback, 'triggers': triggers, 'reply': reply,
            'only_focus': only_focus, 'only_unfocus': only_unfocus, 'reset': reset, 'ungreet': ungreet,
            'move_up': move_up, 'children': children,
        })

    seen_fallback = False
    filtered = []
    for item in prepared:
        if item['fallback']:
            if seen_fallback:
                stats['dropped_extra_fallback'] += 1
                continue
            seen_fallback = True
        filtered.append(item)

    used: set[str] = set()
    result = []
    for item in filtered:
        entry: dict[str, Any] = {'key': make_unique_key(item['base_key'], used)}
        if item['fallback']:
            entry['fallback'] = True
        else:
            entry['triggers'] = item['triggers']
        entry['reply'] = item['reply']
        if item['only_focus']:
            entry['only_focus'] = True
        if item['only_unfocus']:
            entry['only_unfocus'] = True
        if item['reset']:
            entry['reset'] = True
        if item['ungreet']:
            entry['ungreet'] = True
        if item['move_up'] is not None:
            entry['move_up'] = item['move_up']
        if item['children']:
            entry['children'] = item['children']
        result.append(entry)
        stats['keyword_nodes'] += 1
    return result


def build_voice_entries(profile: dict, stats: dict) -> list[dict]:
    entries = []
    for line in profile.get('lines') or []:
        textref = line.get('text')
        if not isinstance(textref, dict) or 'parts' in textref:
            stats['dropped_voice_lines_invalid'] += 1
            continue
        raw, = leaf_texts(textref)  # requires the text and checks it against its digest
        if has_control_chars(raw):
            stats['dropped_voice_lines_invalid'] += 1
            continue
        text = raw.strip()
        if not text:
            stats['dropped_voice_lines_invalid'] += 1
            continue
        entries.append({'text': text, 'mode': 'Yell' if line.get('yell') else 'Say'})
    return entries


def build_voices(bundle: dict, stats: dict) -> dict | None:
    profile = bundle.get('voices')
    if not profile:
        return None
    interval_ms = profile.get('interval_ms')
    chance_percent = profile.get('chance_percent')
    lines = profile.get('lines') or []
    if not interval_ms or chance_percent is None or not lines:
        stats['voices_dropped_no_cadence'] += 1
        return None
    entries = build_voice_entries(profile, stats)
    if not entries:
        return None
    return {'interval_ms': interval_ms, 'chance_ppm': chance_percent * 10000, 'entries': entries}


def build_dialogue(bundle: dict, stats: dict) -> dict | None:
    dialogue = bundle.get('dialogue') or {}
    messages = dialogue.get('messages') or {}
    result: dict[str, Any] = {}
    for key in MESSAGE_KEYS:
        parts = text_parts_cleaned(messages.get(key))
        if parts:
            result[key] = parts
    keywords = build_keyword_nodes(dialogue.get('keywords') or [], 1, stats)
    if keywords:
        result['keywords'] = keywords
    voices = build_voices(bundle, stats)
    if voices:
        result['voices'] = voices
    return result or None


def count_nodes(nodes: list) -> int:
    return sum(1 + count_nodes(node.get('children', [])) for node in nodes)


def diff_fields(a: dict | None, b: dict | None) -> list[str]:
    a = a or {}
    b = b or {}
    return [field for field in DIALOGUE_FIELDS if a.get(field) != b.get(field)]


# ---------------------------------------------------------------------------
# D10: transcript tie-break for DIALOGUE_CONFLICT NPCs (Tibia Global in-game transcripts).
# Applied only to NPCs that would otherwise be held with reason DIALOGUE_CONFLICT; never changes
# any other admission logic (agreeing NPCs, single-source NPCs, or TEXT_BUNDLE_UNVERIFIED holds).
# ---------------------------------------------------------------------------

STEM_NORMALIZE = re.compile(r'[^a-z0-9]+')


def normalize_stem(text: str) -> str:
    return STEM_NORMALIZE.sub('_', text.lower()).strip('_')


def normalize_transcript_text(text: str) -> str:
    return re.sub(r'\s+', ' ', text).strip().casefold()


class TranscriptIndex:
    """Maps a normalized transcript-file stem to the (unique) file(s) sharing that stem."""

    def __init__(self, root: Path):
        self.root = root
        by_stem: dict[str, list[Path]] = {}
        for path in sorted(root.rglob('*.txt')):
            by_stem.setdefault(normalize_stem(path.stem), []).append(path)
        self.by_stem = by_stem

    def find(self, slug: str) -> Path | None:
        candidates = self.by_stem.get(slug)
        if candidates and len(candidates) == 1:
            return candidates[0]
        base = slug.split('_', 1)[0]
        if base != slug:
            candidates = self.by_stem.get(base)
            if candidates and len(candidates) == 1:
                return candidates[0]
        return None


def load_transcript_lines(path: Path) -> list[str]:
    """Normalized NPC-spoken lines from a transcript file (lines whose speaker prefix normalizes
    to the file's own stem; 'Player:' lines and any other speaker are excluded)."""
    stem_norm = normalize_stem(path.stem)
    text = path.read_text(encoding='utf-8', errors='replace')
    lines: list[str] = []
    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        if ':' not in line:
            continue
        speaker, _, rest = line.partition(':')
        speaker = speaker.strip()
        if not speaker or normalize_stem(speaker) != stem_norm:
            continue
        lines.append(normalize_transcript_text(rest))
    return lines


def text_matches_transcript(text: str, transcript_lines: list[str]) -> bool:
    """True if `text` (which may contain the literal '|PLAYERNAME|' placeholder) matches, as a
    full-line regex (placeholder -> '.+?', the rest escaped), some normalized transcript line."""
    normalized = normalize_transcript_text(text)
    parts = normalized.split('|playername|')
    pattern = '.+?'.join(re.escape(part) for part in parts)
    regex = re.compile('^' + pattern + '$')
    return any(regex.fullmatch(line) for line in transcript_lines)


def flatten_keyword_nodes(nodes: list) -> list[dict]:
    """Every keyword node in the tree (any depth), each with its own attributes but without its
    'children' key, so identity comparison is per-node rather than per-subtree."""
    out: list[dict] = []

    def walk(items: list) -> None:
        for node in items:
            out.append({k: v for k, v in node.items() if k != 'children'})
            walk(node.get('children') or [])

    walk(nodes)
    return out


def extra_nodes(nodes_a: list[dict], nodes_b: list[dict]) -> list[dict]:
    """Nodes in `nodes_a` with no identical (multiset) counterpart in `nodes_b`."""
    remaining: dict[bytes, int] = {}
    for node in nodes_b:
        key = canonical(node)
        remaining[key] = remaining.get(key, 0) + 1
    extra = []
    for node in nodes_a:
        key = canonical(node)
        if remaining.get(key, 0) > 0:
            remaining[key] -= 1
        else:
            extra.append(node)
    return extra


def differing_texts(canary_dialogue: dict | None, crystal_dialogue: dict | None) -> tuple[list[str], list[str]]:
    """Every text that differs between the two candidate dialogues, split by which source it
    belongs to: message parts and voice entry texts for fields that differ wholesale, and
    keyword reply parts for individual keyword nodes without an identical counterpart."""
    a = canary_dialogue or {}
    b = crystal_dialogue or {}
    canary_texts: list[str] = []
    crystal_texts: list[str] = []
    for key in MESSAGE_KEYS:
        av, bv = a.get(key), b.get(key)
        if av != bv:
            canary_texts.extend(av or [])
            crystal_texts.extend(bv or [])
    a_voices, b_voices = a.get('voices'), b.get('voices')
    if a_voices != b_voices:
        canary_texts.extend(entry['text'] for entry in (a_voices or {}).get('entries', []))
        crystal_texts.extend(entry['text'] for entry in (b_voices or {}).get('entries', []))
    a_flat = flatten_keyword_nodes(a.get('keywords') or [])
    b_flat = flatten_keyword_nodes(b.get('keywords') or [])
    for node in extra_nodes(a_flat, b_flat):
        canary_texts.extend(node.get('reply') or [])
    for node in extra_nodes(b_flat, a_flat):
        crystal_texts.extend(node.get('reply') or [])
    return canary_texts, crystal_texts


def score_texts(texts: list[str], transcript_lines: list[str]) -> dict:
    matches = sum(1 for text in texts if text_matches_transcript(text, transcript_lines))
    return {'matches': matches, 'misses': len(texts) - matches, 'total': len(texts)}


def resolve_conflict(slug: str, canary_dialogue: dict | None, crystal_dialogue: dict | None,
                      transcripts: TranscriptIndex) -> dict | None:
    """Apply rule D10 for one held-conflict NPC. Returns None when no transcript is found for it
    (nothing to add); otherwise a dict with 'path', 'transcript' (path relative to the transcripts
    root), 'canary_score', 'crystal_score', and, when one source strictly wins (more matches, and
    more than zero), 'winner'."""
    path = transcripts.find(slug)
    if path is None:
        return None
    transcript_lines = load_transcript_lines(path)
    canary_texts, crystal_texts = differing_texts(canary_dialogue, crystal_dialogue)
    canary_score = score_texts(canary_texts, transcript_lines)
    crystal_score = score_texts(crystal_texts, transcript_lines)
    relative = path.relative_to(transcripts.root).as_posix()
    info = {'path': path, 'transcript': relative, 'canary_score': canary_score, 'crystal_score': crystal_score}
    cm, km = canary_score['matches'], crystal_score['matches']
    if cm != km and max(cm, km) > 0:
        info['winner'] = 'canary' if cm > km else 'crystal'
    return info


def transcripts_used_digest(transcripts_root: Path, used: set[Path]) -> str:
    """sha256 over sorted `<relative path>:<sha256 of file bytes>` lines of every transcript file
    that was actually used in a D10 decision (resolved or scored)."""
    lines = [f'{path.relative_to(transcripts_root).as_posix()}:{hashlib.sha256(path.read_bytes()).hexdigest()}'
             for path in sorted(used)]
    return hashlib.sha256('\n'.join(lines).encode()).hexdigest()


def build_declaration(slug: str, dialogue: dict) -> dict:
    declaration: dict[str, Any] = {'kind': 'Dialogue',
                                   'identity': {'key': f'oteryn:dialogue.npc.{slug}', 'revision': REVISION}}
    for key in MESSAGE_KEYS:
        if key in dialogue:
            declaration[key] = dialogue[key]
    if 'keywords' in dialogue:
        declaration['keywords'] = dialogue['keywords']
    if 'voices' in dialogue:
        declaration['voices'] = dialogue['voices']
    declaration['fields'] = []
    return declaration


def slug_of(key: str) -> str:
    if not key.startswith(NPC_PREFIX):
        raise StageError(f'{key}: expected prefix {NPC_PREFIX}')
    slug = key[len(NPC_PREFIX):]
    if not SLUG.fullmatch(slug) or len(slug) > 64:
        raise StageError(f'{key}: slug is not a production slug')
    return slug


SOURCE_IDENTITY = {'canary': ('opentibiabr/canary', CANARY_REVISION),
                   'crystal': ('zimbadev/crystalserver', CRYSTAL_REVISION)}


def load_bundle(bundles_dir: Path, source: str, provenance: dict) -> dict | None:
    """Loads a converted bundle and binds it to the candidate: its schema, key, pinned repository and
    revision, and the source-file digest the candidate records must all match."""
    source_key = provenance['key']
    stem = source_key.split(':npc/', 1)[1]
    path = bundles_dir / f'{stem}.json'
    if not path.exists():
        return None
    bundle = json.loads(path.read_text())
    repository, revision = SOURCE_IDENTITY[source]
    origin = bundle.get('source') or {}
    if (bundle.get('schema') != 'OTERYN_NPC_AUTHORING_CANDIDATE/v1' or bundle.get('key') != source_key
            or origin.get('repository') != repository or origin.get('revision') != revision
            or origin.get('sha256') != provenance.get('sha256')):
        raise StageError(f'{source} bundle {path.name} does not match {source_key} at the pinned revision')
    return bundle


def strip_text(value: Any) -> Any:
    """The reference-only form of an --include-text bundle: every text reference without its text."""
    if isinstance(value, dict):
        return {k: strip_text(v) for k, v in value.items() if not (k == 'text' and 'sha256' in value)}
    if isinstance(value, list):
        return [strip_text(v) for v in value]
    return value


def census_bound_reference(reference_dir: Path, source: str) -> tuple[Path, str]:
    """The reference-only bundle directory must reproduce the committed census bundle_digest (sorted
    `<file name>:<sha256 of file>` lines), which authenticates every reference bundle."""
    census_digest = json.loads(CENSUS[source].read_text())['bundle_digest']
    lines = [f'{path.name}:{hashlib.sha256(path.read_bytes()).hexdigest()}'
             for path in sorted(reference_dir.glob('*.json'))]
    if hashlib.sha256('\n'.join(lines).encode()).hexdigest() != census_digest:
        raise StageError(f'{source} reference bundles do not reproduce the committed census bundle_digest')
    return reference_dir, census_digest


def text_bundle_verified(bundle: dict, reference_dir: Path, source_key: str) -> bool:
    """An --include-text bundle is authenticated when, without its text, it equals the census-bound
    reference bundle; the text itself is bound by the per-reference digests checked in leaf_texts."""
    reference = reference_dir / f"{source_key.split(':npc/', 1)[1]}.json"
    return reference.exists() and json.loads(reference.read_text()) == strip_text(bundle)


def stage(report: dict, canary_dir: Path, crystal_dir: Path, canary_reference: Path,
          crystal_reference: Path, transcripts: 'TranscriptIndex | None' = None,
          transcripts_revision: str | None = None) -> dict:
    if report['schema'] != 'OTERYN_NPC_PROMOTION_CANDIDATES/v1':
        raise StageError('promotion candidate report drifted')
    for bundles_dir in (canary_dir, crystal_dir):
        index_path = bundles_dir.parent / 'index.json'
        if not index_path.exists() or json.loads(index_path.read_text()).get('include_text') is not True:
            raise StageError(f'{bundles_dir} is not a convert.py --include-text output (its index.json)')
    references = {'canary': census_bound_reference(canary_reference, 'canary'),
                  'crystal': census_bound_reference(crystal_reference, 'crystal')}
    candidates = sorted(report['candidates'], key=lambda c: c['identity']['key'])
    stats = {
        'dropped_non_say_gated': 0, 'dropped_no_text': 0, 'dropped_no_triggers': 0, 'keyword_nodes': 0,
        'dropped_extra_fallback': 0, 'dropped_bad_move_up': 0, 'dropped_conflicting_focus_flags': 0,
        'voices_dropped_no_cadence': 0, 'dropped_voice_lines_invalid': 0, 'dropped_shadowed_by_omitted_sibling': 0,
    }
    dialogues, held, resolved = [], [], []
    used_transcripts: set[Path] = set()
    voice_line_total = greet_total = farewell_total = walkaway_total = send_trade_total = 0
    for candidate in candidates:
        npc_key = candidate['identity']['key']
        slug = slug_of(npc_key)
        provenance = candidate['provenance']
        by_source: dict[str, dict | None] = {}
        unverified = []
        for source, bundles_dir in (('canary', canary_dir), ('crystal', crystal_dir)):
            if source not in provenance:
                continue
            bundle = load_bundle(bundles_dir, source, provenance[source])
            if bundle is None:
                raise StageError(f'missing {source} bundle for {npc_key}: {provenance[source]["key"]}')
            if not text_bundle_verified(bundle, references[source][0], provenance[source]['key']):
                unverified.append(source)
                continue
            by_source[source] = build_dialogue(bundle, stats)
        if unverified:
            # e.g. a script that registers keywords in Lua pairs() order converts differently per run
            held.append({'npc': npc_key, 'reason': 'TEXT_BUNDLE_UNVERIFIED', 'sources': unverified})
            continue
        if not by_source:
            continue
        if len(by_source) == 1:
            (source_name, dialogue), = by_source.items()
            if dialogue is None:
                continue
            taken_provenance = [source_name]
        else:
            canary_dialogue = by_source.get('canary')
            crystal_dialogue = by_source.get('crystal')
            if canary_dialogue != crystal_dialogue:
                resolution = resolve_conflict(slug, canary_dialogue, crystal_dialogue, transcripts) \
                    if transcripts is not None else None
                if resolution is not None:
                    used_transcripts.add(resolution['path'])
                if resolution is not None and 'winner' in resolution:
                    winner = resolution['winner']
                    dialogue = canary_dialogue if winner == 'canary' else crystal_dialogue
                    if dialogue is None:
                        continue
                    resolved.append({
                        'npc': npc_key, 'rule': 'D10_TRANSCRIPT', 'chosen': winner,
                        'score': {'canary': resolution['canary_score']['matches'],
                                  'crystal': resolution['crystal_score']['matches']},
                        'transcript': resolution['transcript'],
                    })
                    dialogues.append({'npc': npc_key, 'provenance': [winner, 'transcript'],
                                      'declaration': build_declaration(slug, dialogue)})
                    if 'greet' in dialogue:
                        greet_total += 1
                    if 'farewell' in dialogue:
                        farewell_total += 1
                    if 'walkaway' in dialogue:
                        walkaway_total += 1
                    if 'send_trade' in dialogue:
                        send_trade_total += 1
                    voice_line_total += len(dialogue.get('voices', {}).get('entries', []))
                    continue
                held_entry = {'npc': npc_key, 'reason': 'DIALOGUE_CONFLICT',
                               'diff': diff_fields(canary_dialogue, crystal_dialogue)}
                if transcripts is not None:
                    if resolution is not None:
                        held_entry['transcript_scores'] = {
                            'transcript': resolution['transcript'],
                            'canary': resolution['canary_score'], 'crystal': resolution['crystal_score'],
                        }
                    else:
                        held_entry['transcript'] = None
                held.append(held_entry)
                continue
            dialogue = canary_dialogue
            if dialogue is None:
                continue
            taken_provenance = ['canary', 'crystal']
        dialogues.append({'npc': npc_key, 'provenance': taken_provenance,
                          'declaration': build_declaration(slug, dialogue)})
        if 'greet' in dialogue:
            greet_total += 1
        if 'farewell' in dialogue:
            farewell_total += 1
        if 'walkaway' in dialogue:
            walkaway_total += 1
        if 'send_trade' in dialogue:
            send_trade_total += 1
        voice_line_total += len(dialogue.get('voices', {}).get('entries', []))
    dialogues.sort(key=lambda entry: entry['npc'])
    held.sort(key=lambda entry: entry['npc'])
    resolved.sort(key=lambda entry: entry['npc'])
    keyword_node_total = sum(count_nodes(entry['declaration'].get('keywords', [])) for entry in dialogues)
    source: dict[str, Any] = {'canary_revision': CANARY_REVISION, 'crystal_revision': CRYSTAL_REVISION,
                               'candidates_sha256': hashlib.sha256(CANDIDATES.read_bytes()).hexdigest(),
                               'canary_census_bundle_digest': references['canary'][1],
                               'crystal_census_bundle_digest': references['crystal'][1]}
    counts: dict[str, Any] = {'npcs_with_dialogue': len(dialogues),
                               'held_dialogue_conflict': sum(h['reason'] == 'DIALOGUE_CONFLICT' for h in held),
                               'held_text_bundle_unverified': sum(h['reason'] == 'TEXT_BUNDLE_UNVERIFIED' for h in held),
                               'keyword_nodes': keyword_node_total, 'voice_lines': voice_line_total,
                               'greet': greet_total, 'farewell': farewell_total, 'walkaway': walkaway_total,
                               'send_trade': send_trade_total,
                               'dropped_non_say_gated_nodes': stats['dropped_non_say_gated'],
                               'dropped_no_text_nodes': stats['dropped_no_text'],
                               'dropped_no_triggers_nodes': stats['dropped_no_triggers'],
                               'dropped_extra_fallback_nodes': stats['dropped_extra_fallback'],
                               'dropped_bad_move_up_nodes': stats['dropped_bad_move_up'],
                               'dropped_conflicting_focus_flags_nodes': stats['dropped_conflicting_focus_flags'],
                               'voices_dropped_no_cadence': stats['voices_dropped_no_cadence'],
                               'dropped_voice_lines_invalid': stats['dropped_voice_lines_invalid'],
                               'dropped_shadowed_by_omitted_sibling_nodes': stats['dropped_shadowed_by_omitted_sibling']}
    packet: dict[str, Any] = {'schema': 'OTERYN_NPC_DIALOGUE_STAGED/v1', 'source': source, 'counts': counts,
                               'dialogues': dialogues, 'held': held}
    # Only present at all when --transcripts was given, so a run without it stays byte-identical
    # to a build of this tool without D10 support.
    if transcripts is not None:
        source['transcripts_revision'] = transcripts_revision
        source['transcripts_digest'] = transcripts_used_digest(transcripts.root, used_transcripts)
        counts['resolved_by_transcript'] = len(resolved)
        packet['resolved'] = resolved
    return packet


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary-bundles', type=Path, required=True)
    parser.add_argument('--crystal-bundles', type=Path, required=True)
    parser.add_argument('--canary-reference-bundles', type=Path, required=True,
                        help='reference-only convert.py output that reproduces the committed Canary census')
    parser.add_argument('--crystal-reference-bundles', type=Path, required=True,
                        help='reference-only convert.py output that reproduces the committed Crystal census')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--transcripts', type=Path, default=None,
                        help='Optional root of Tibia Global in-game NPC transcript .txt files '
                             '(rule D10: tie-break DIALOGUE_CONFLICT NPCs against them). '
                             'Requires --transcripts-revision.')
    parser.add_argument('--transcripts-revision', type=str, default=None,
                        help='Commit of the transcripts source, recorded as source.transcripts_revision. '
                             'Requires --transcripts.')
    args = parser.parse_args()
    if bool(args.transcripts) != bool(args.transcripts_revision):
        print('npc dialogue stage: --transcripts and --transcripts-revision are required together',
              file=sys.stderr)
        return 1
    transcripts = TranscriptIndex(args.transcripts) if args.transcripts is not None else None
    report = json.loads(CANDIDATES.read_text())
    try:
        packet = stage(report, args.canary_bundles, args.crystal_bundles, args.canary_reference_bundles,
                       args.crystal_reference_bundles, transcripts, args.transcripts_revision)
    except StageError as error:
        print(f'npc dialogue stage: {error}', file=sys.stderr)
        return 1
    args.out.write_bytes(canonical(packet))
    print(json.dumps(packet['counts'], sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
