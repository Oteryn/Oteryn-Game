"""Character progression content: formula, canonical JSON, revisions and evidence check.

ARCH-PROGRESSION-SOURCE-0 section 1.1-1.3. Standard library only.
"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXPERIENCE_DIR = ROOT / "rulesets/character/experience"
DEATH_DIR = ROOT / "rulesets/character/death"
EVIDENCE_DIR = ROOT / "docs/reference/experience-table-20261005"

TABLE_LEVELS = 2000
# Last level covered by the private tibia.com capture (section 1.1). It must equal
# evidence.json's last_level and the table's evidence_coverage.
EXPERIENCE_EVIDENCE_LAST_LEVEL = 2000
# Reference formula 50/3 * (L^3 - 6L^2 + 17L - 12), as integer coefficients.
FORMULA_NUMERATOR = 50
FORMULA_DENOMINATOR = 3
FORMULA_COEFFICIENTS = (1, -6, 17, -12)
SIMULATION_REVISION = "oteryn-simulation-determinism-exact-i64-v1"
MAX_SAFE_INTEGER = 2**53 - 1
SECTION_SCHEMA = "OTERYN_NATIVE_PROGRESSION/v1"
TABLE_SCHEMA = "OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1"
DEATH_SCHEMA = "OTERYN_GAME_CHARACTER_DEATH_POLICY/v1"
REWARD_SCHEMA = "OTERYN_GAME_CHARACTER_REWARD_POLICY/v1"
DIFFERENCES_SCHEMA = "OTERYN_GAME_CHARACTER_PROGRESSION_DIFFERENCES/v1"
EVIDENCE_SCHEMA = "OTERYN_GAME_CHARACTER_EXPERIENCE_EVIDENCE/v1"
PREFIX = {
    "table": "character-experience-v1-",
    "death": "character-death-v1-",
    "reward": "character-reward-v1-",
    "differences": "character-progression-differences-v1-",
    "policy": "character-progression-policy-v1-",
    "evidence": "experience-table-evidence-20261005-",
}
PATHS = {
    "table": EXPERIENCE_DIR / "experience-table.json",
    "reward": EXPERIENCE_DIR / "reward-policy.json",
    "differences": EXPERIENCE_DIR / "declared-differences.json",
    "death": DEATH_DIR / "death-policy.json",
    "evidence": EVIDENCE_DIR / "evidence.json",
}


class ProgressionError(ValueError):
    pass


# ---------------------------------------------------------------- canonical JSON
def _refuse(kind):
    def hook(*_args):
        raise ProgressionError(f"canonical profile refuses {kind}")

    return hook


def _pairs(pairs):
    out = {}
    for name, value in pairs:
        if name in out:
            raise ProgressionError("duplicate member name")
        if not name.isascii():
            raise ProgressionError("non-ASCII member name")
        out[name] = value
    return out


def _int(text):
    if text.startswith("-"):
        raise ProgressionError("canonical profile refuses a negative number")
    value = int(text)
    if value > MAX_SAFE_INTEGER:
        raise ProgressionError("integer above 2^53-1")
    return value


def _check(value):
    if value is None:
        raise ProgressionError("canonical profile refuses null")
    if isinstance(value, bool):
        return
    if isinstance(value, int):
        if not 0 <= value <= MAX_SAFE_INTEGER:
            raise ProgressionError("integer outside 0..=2^53-1")
    elif isinstance(value, str):
        try:
            value.encode("utf-8")
        except UnicodeEncodeError:
            raise ProgressionError("unpaired surrogate") from None
    elif isinstance(value, list):
        for item in value:
            _check(item)
    elif isinstance(value, dict):
        for name, item in value.items():
            if not name.isascii():
                raise ProgressionError("non-ASCII member name")
            _check(item)
    else:
        raise ProgressionError("canonical profile refuses this value")


def strict_loads(data):
    """Parse bytes or text under the canonical profile; refuses before any map is built."""
    if isinstance(data, bytes):
        try:
            data = data.decode("utf-8")
        except UnicodeDecodeError:
            raise ProgressionError("not UTF-8") from None
    try:
        value = json.loads(
            data,
            object_pairs_hook=_pairs,
            parse_constant=_refuse("a constant"),
            parse_float=_refuse("a float"),
            parse_int=_int,
        )
    except json.JSONDecodeError as exc:
        raise ProgressionError("invalid JSON") from exc
    _check(value)
    return value


def canonical(value):
    _check(value)
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def sha32(data):
    return hashlib.sha256(data).hexdigest()[:32]


# ---------------------------------------------------------------- formula and capture
def experience_for_level(level):
    a, b, c, d = FORMULA_COEFFICIENTS
    numerator = FORMULA_NUMERATOR * (((a * level + b) * level + c) * level + d)
    if numerator % FORMULA_DENOMINATOR:
        raise ProgressionError("formula is not integral")
    return numerator // FORMULA_DENOMINATOR


def levels_csv(last_level):
    """The normalized capture bytes the formula yields for levels 1..=last_level."""
    return "".join(
        f"{level},{experience_for_level(level)}\n" for level in range(1, last_level + 1)
    ).encode("ascii")


def check_capture(data, last_level):
    """Refuse a capture that is not exactly the normalized rows 1..=last_level."""
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError:
        raise ProgressionError("capture is not ASCII") from None
    if text.startswith("﻿") or "\r" in text or not text.endswith("\n"):
        raise ProgressionError("capture line ends are not LF with a final LF")
    rows = text[:-1].split("\n")
    if len(rows) != last_level:
        raise ProgressionError("capture row count is not K")
    for index, row in enumerate(rows, start=1):
        level, sep, experience = row.partition(",")
        if (
            not sep
            or not level.isascii()
            or not level.isdigit()
            or not experience.isascii()
            or not experience.isdigit()
            or level != str(int(level))
            or experience != str(int(experience))
            or int(level) != index
        ):
            raise ProgressionError("capture rows are not exactly 1..=K in order")
    # Not a duplicate check by itself: rows are positional, so a duplicate or a
    # missing row already moves a level and fails above.
    return True


def verify_levels_hash(evidence, last_level=EXPERIENCE_EVIDENCE_LAST_LEVEL):
    expected = hashlib.sha256(levels_csv(last_level)).hexdigest()
    if evidence["levels_sha256"] != expected:
        raise ProgressionError("levels_sha256 differs from the formula for 1..=K")


def verify_capture_file(path, evidence):
    data = Path(path).read_bytes()
    check_capture(data, evidence["last_level"])
    if hashlib.sha256(data).hexdigest() != evidence["levels_sha256"]:
        raise ProgressionError("capture SHA-256 differs from levels_sha256")


# ---------------------------------------------------------------- documents
def evidence_revision(evidence):
    return PREFIX["evidence"] + sha32(canonical(evidence))


def _without_revision(document):
    return {k: v for k, v in document.items() if k != "revision"}


def document_revision(kind, document):
    return PREFIX[kind] + sha32(canonical(_without_revision(document)))


def policy_revision(table_revision, death_revision):
    return PREFIX["policy"] + sha32(
        canonical({"death_policy": death_revision, "experience_table": table_revision})
    )


def build_evidence(last_level, source_url, captured_on, extraction_method, levels_sha256):
    return {
        "schema": EVIDENCE_SCHEMA,
        "source_url": source_url,
        "captured_on": captured_on,
        "extraction_method": extraction_method,
        "last_level": last_level,
        "levels_sha256": levels_sha256,
    }


def build_table(evidence):
    last = evidence["last_level"]
    table = {
        "schema": TABLE_SCHEMA,
        "evidence_revision": evidence_revision(evidence),
        "evidence_coverage": {"first_level": 1, "last_level": last},
        "levels": [
            {"level": level, "minimum_experience": str(experience_for_level(level))}
            for level in range(1, TABLE_LEVELS + 1)
        ],
        "terminal_exclusive_experience": str(experience_for_level(TABLE_LEVELS + 1)),
    }
    table["revision"] = document_revision("table", table)
    return table


def build_death():
    doc = {
        "schema": DEATH_SCHEMA,
        "loss_numerator": 1,
        "loss_denominator": 1,
        "rounding": "floor",
    }
    doc["revision"] = document_revision("death", doc)
    return doc


def build_reward():
    doc = {"schema": REWARD_SCHEMA}
    doc["revision"] = document_revision("reward", doc)
    return doc


def build_differences(last_level):
    records = [
        {
            "id": "low-level-experience-bonus",
            "reference": "characters.md 5.1.1: a qualitative experience bonus applies below level 50; no formula is in evidence.",
            "oteryn": "Not modelled: kill experience carries no low-level bonus.",
        }
    ]
    if last_level < TABLE_LEVELS:
        records.append(
            {
                "id": "formula-derived-levels-above-evidence",
                "reference": f"The captured table covers levels 1..={last_level}.",
                "oteryn": f"Levels {last_level + 1}..={TABLE_LEVELS} and the terminal threshold are derived from the Reference formula without captured evidence.",
            }
        )
    records.append(
        {
            "id": "stamina-and-experience-boosts",
            "reference": "Stamina and experience boosts change the experience a kill awards.",
            "oteryn": "Not modelled: the base experience is awarded unchanged.",
        }
    )
    doc = {"schema": DIFFERENCES_SCHEMA, "records": records}
    doc["revision"] = document_revision("differences", doc)
    return doc


def revisions(docs):
    table_rev = docs["table"]["revision"]
    death_rev = docs["death"]["revision"]
    return {
        "policy_revision": policy_revision(table_rev, death_rev),
        "experience_table_revision": table_rev,
        "death_policy_revision": death_rev,
        "reward_revision": docs["reward"]["revision"],
        "declaration": docs["differences"]["revision"],
        "evidence": evidence_revision(docs["evidence"]),
        "simulation": SIMULATION_REVISION,
    }


def verify_documents(docs):
    """Refuse a document set whose stated revisions or coverage disagree with its content."""
    evidence, table = docs["evidence"], docs["table"]
    if evidence.get("schema") != EVIDENCE_SCHEMA:
        raise ProgressionError("evidence schema")
    last = evidence["last_level"]
    if not (isinstance(last, int) and 1 <= last <= TABLE_LEVELS):
        raise ProgressionError("evidence last_level out of range")
    if last != EXPERIENCE_EVIDENCE_LAST_LEVEL:
        raise ProgressionError("EXPERIENCE_EVIDENCE_LAST_LEVEL differs from evidence.json")
    if table["evidence_coverage"] != {"first_level": 1, "last_level": last}:
        raise ProgressionError("table evidence_coverage differs from evidence.json")
    verify_levels_hash(evidence, last)
    if table["evidence_revision"] != evidence_revision(evidence):
        raise ProgressionError("table evidence_revision differs from the evidence record")
    for kind in ("table", "death", "reward", "differences"):
        if docs[kind]["revision"] != document_revision(kind, docs[kind]):
            raise ProgressionError(f"{kind} revision differs from its content")
    expected = {
        "table": build_table(evidence),
        "death": build_death(),
        "reward": build_reward(),
        "differences": build_differences(last),
    }
    for kind, doc in expected.items():
        if docs[kind] != doc:
            raise ProgressionError(f"{kind} differs from the producer output")


def build_section(docs):
    verify_documents(docs)
    return {
        "schema": SECTION_SCHEMA,
        "experience_table": docs["table"],
        "death_policy": docs["death"],
        "reward_policy": docs["reward"],
        "declared_differences": docs["differences"],
        "revisions": revisions(docs),
    }


def load_docs(paths=PATHS):
    return {kind: strict_loads(Path(path).read_bytes()) for kind, path in paths.items()}


def pretty(value):
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")
