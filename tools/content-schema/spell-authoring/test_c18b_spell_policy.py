"""C18b registration, formula, and presentation-boundary regressions."""

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from complete_player_sounds import synchronize_native_profiles
from native_source_projection import project_native_candidate
from test_native_source_projection import CATALOG, ROOT, entry, source
from validate_spell import evaluate, validate

SAMPLES = Path(__file__).with_name("samples")
CRYSTAL = {
    "repository": "zimbadev/crystalserver",
    "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
}
GLOBAL_HEADERS = (
    ("Death Echo", "costs", "mana", 155, 150, "S15:", "mana", 1),
    ("Divine Grenade", "requirements", "level", 300, 0, "S22:", "level", 93),
    ("Divine Grenade", "targeting", "range_tiles", 7, 4, "S3:", "spellrange", 1),
)
BARRAGE_HEADERS = (
    ("Divine Barrage", "targeting", "needs_target", False, True, "needTarget", 32),
    (
        "Divine Barrage",
        "targeting",
        "cast_at_position",
        False,
        True,
        "needPosition",
        31,
    ),
    ("Ethereal Barrage", "targeting", "needs_target", False, True, "needTarget", 34),
    (
        "Ethereal Barrage",
        "targeting",
        "cast_at_position",
        False,
        True,
        "needPosition",
        33,
    ),
)


class C18bSpellPolicyTests(unittest.TestCase):
    def assert_manifest_identity(self, row):
        manifest = row["manifest"]
        encoded = (
            json.dumps(
                manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":")
            )
            + "\n"
        ).encode()
        for identity in row["source_identities"]:
            self.assertEqual(identity["sources"], manifest["sources"])
            self.assertEqual(
                identity["manifest_sha256"], hashlib.sha256(encoded).hexdigest()
            )

    def test_global_fields_keep_their_source_proofs(self):
        source_markers = (
            ("kind", "official_capture"),
            ("revision", "99902524e052f37574194466c2949c576e4ab269"),
            ("revision_id", 436982),
        )
        for case, (source_key, source_value) in zip(
            GLOBAL_HEADERS, source_markers, strict=True
        ):
            name, section, field, _, expected, rule, source_field, line = case
            with self.subTest(name=name, field=field):
                row = entry(name)
                self.assertEqual(row["bundle"]["spell"][section][field], expected)
                (proof,) = [
                    proof
                    for proof in row["manifest"]["entries"]
                    if proof.get("destination") == f"/spell/spell/{section}/{field}"
                ]
                self.assertTrue(proof["resolution"].startswith(rule))
                self.assertEqual(
                    (proof["source_field"], proof["source_line"]), (source_field, line)
                )
                self.assertEqual(
                    row["manifest"]["sources"][proof["source_index"]][source_key],
                    source_value,
                )
                self.assert_manifest_identity(row)

    def test_crystal_fields_have_current_provenance(self):
        for name, section, field, _, expected, source_field, line in BARRAGE_HEADERS:
            with self.subTest(name=name, field=field):
                row = entry(name)
                spell = row["bundle"]["spell"]
                self.assertEqual(spell[section][field], expected)
                destination = f"/spell/spell/{section}/{field}"
                proofs = [
                    p
                    for p in row["manifest"]["entries"]
                    if p.get("destination") == destination
                ]
                self.assertEqual(len(proofs), 1)
                proof = proofs[0]
                self.assertEqual(
                    row["manifest"]["sources"][proof["source_index"]], CRYSTAL
                )
                path = (
                    f"data/scripts/spells/attack/{name.lower().replace(' ', '_')}.lua"
                )
                self.assertEqual(
                    (proof["source_file"], proof["source_field"], proof["source_line"]),
                    (path, source_field, line),
                )
                self.assertTrue(proof["resolution"].startswith("C18b:"))
                self.assertTrue(spell["targeting"]["cast_at_position"])
                self.assert_manifest_identity(row)

    def test_source_projection_requires_proof_for_global_native_header_values(self):
        for name, section, field, old, expected, *_ in GLOBAL_HEADERS:
            with self.subTest(name=name, field=field):
                row = entry(name)
                bundle, dependencies, catalog = source(row)
                bundle["spell"][section][field] = old
                identity = row["bundle"]["spell"]["identity"]
                projected, _, receipt = project_native_candidate(
                    bundle, dependencies, row, identity, catalog
                )
                self.assertEqual(projected["spell"][section][field], expected)
                pointer = f"/{section}/{field}"
                self.assertTrue(
                    any(
                        proof["field"] == pointer
                        for proof in receipt["field_provenance"]
                    )
                )
                row["manifest"]["entries"] = [
                    proof
                    for proof in row["manifest"]["entries"]
                    if proof.get("destination") != f"/spell/spell{pointer}"
                ]
                with self.assertRaisesRegex(
                    ValueError, f"SOURCE_CONFLICT_KEPT:{pointer}"
                ):
                    project_native_candidate(
                        bundle, dependencies, row, identity, catalog
                    )

    def test_sound_sync_preserves_matching_legacy_gameplay(self):
        catalog = copy.deepcopy(CATALOG)
        profiles = json.loads((SAMPLES / "native-spell-profiles.json").read_bytes())
        for name, section, field, old, *_ in GLOBAL_HEADERS:
            spell = next(
                row["bundle"]["spell"]
                for row in catalog["bundles"]
                if row["bundle"]["spell"]["name"] == name
            )
            profile = next(row for row in profiles["profiles"] if row["name"] == name)
            spell[section][field] = profile["spell"][section][field] = old
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profiles.json"
            path.write_text(json.dumps(profiles))
            result, changes = synchronize_native_profiles(catalog, path)
        self.assertEqual(changes, [])
        self.assertEqual(result, profiles)

    def test_registered_spell_projections_keep_the_authoring_schema(self):
        sample = json.loads((SAMPLES / "executable-spell-catalog.json").read_bytes())
        names = {name for name, *_ in (*GLOBAL_HEADERS, *BARRAGE_HEADERS)}
        for label, catalog in (("production", CATALOG), ("sample", sample)):
            for row in catalog["bundles"]:
                name = row["bundle"]["spell"]["name"]
                if name not in names:
                    continue
                with self.subTest(catalog=label, name=name):
                    for expected_name, section, field, _, expected, *_ in (
                        *GLOBAL_HEADERS,
                        *BARRAGE_HEADERS,
                    ):
                        if expected_name == name:
                            self.assertEqual(
                                row["bundle"]["spell"][section][field], expected
                            )
                    self.assertEqual(
                        validate(
                            row["bundle"],
                            row["dependencies"],
                            row["catalog"],
                            row["manifest"],
                        ),
                        [],
                    )

    def test_registered_damage_matches_crystal_callbacks(self):
        collection = json.loads(
            (
                ROOT / "content/abilities/formulas/player-spell-formulas.json"
            ).read_bytes()
        )
        profiles = json.loads((SAMPLES / "native-spell-profiles.json").read_bytes())[
            "profiles"
        ]
        for name, power, expected in (
            ("Death Echo", 75, (180, 240)),
            ("Divine Barrage", 130, (389, 495)),
            ("Ethereal Barrage", 40, (204, 250)),
        ):
            with self.subTest(name=name):
                row = entry(name)
                spell = row["bundle"]["spell"]
                env = {
                    "level": 300,
                    "magic_level": 50,
                    "attack_skill": 100,
                    "attack_value": 40,
                    "base_power": spell["base_power"],
                }
                if name == "Death Echo":
                    params = spell["execution"]["native_behavior"]["parameters"]
                    formula = params["formula"]
                    native = next(p for p in profiles if p["name"] == name)
                    self.assertEqual(native["spell"], spell)
                    self.assertEqual(params["base_power"], spell["base_power"])
                else:
                    (formula,) = row["dependencies"]["formulas"]
                    self.assertIn(formula, collection["records"])
                bounds = tuple(
                    int(evaluate(formula[bound], env))
                    for bound in ("minimum", "maximum")
                )
                self.assertEqual((spell["base_power"], bounds), (power, expected))
                if name == "Ethereal Barrage":
                    env["attack_value"] = 20
                    self.assertEqual(
                        tuple(
                            int(evaluate(formula[bound], env))
                            for bound in ("minimum", "maximum")
                        ),
                        (132, 162),
                    )
