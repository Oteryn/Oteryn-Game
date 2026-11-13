"""The shared Ground census must fit the unchanged SQLx WP3 wire profile."""
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[3]
TRANSACTION = "crates/gameplay/durability/src/durability/spell_item_transaction.rs"
PREFLIGHT = "vendor/sqlx-postgres-0.9.0/src/message/row_description.rs"


def projection_columns(sql):
    """Split the outer SELECT, retaining nested arrays/subqueries as one field."""
    if not sql.startswith("SELECT "):
        raise ValueError("expected the actual Ground SELECT")
    columns = []
    start = 7
    depth = brackets = 0
    quoted = False
    for index, char in enumerate(sql[7:], 7):
        if char == "'":
            quoted = not quoted
        if quoted:
            continue
        depth += (char == "(") - (char == ")")
        brackets += (char == "[") - (char == "]")
        if depth == brackets == 0:
            if char == ",":
                columns.append(sql[start:index].strip())
                start = index + 1
            elif sql[index:index + 6] == " FROM ":
                columns.append(sql[start:index].strip())
                return columns
    raise ValueError("Ground SELECT outer FROM not found")


def function_body(source, marker):
    start = source.index(marker)
    end = source.index("{", start) + 1
    depth = 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


class GroundWireProjectionTests(unittest.TestCase):
    def columns(self):
        source = (ROOT / TRANSACTION).read_text()
        literal = re.search(r'const TILE_ROWS: &str = ("[^\n]*");', source)
        self.assertIsNotNone(literal, "retain an inspectable literal census query")
        return projection_columns(json.loads(literal.group(1)))

    def test_actual_ground_projection_stays_inside_unchanged_wire_budget(self):
        preflight = (ROOT / PREFLIGHT).read_text()
        guard = re.search(r"if count > (\d+) \{", preflight)
        self.assertIsNotNone(guard)
        self.assertEqual(int(guard.group(1)), 32, "do not widen the WP3 profile")
        self.assertLessEqual(len(self.columns()), 32)

    def test_actual_projection_passes_real_preflight_and_added_column_cannot(self):
        rustc = shutil.which("rustc")
        if rustc is None:
            self.skipTest("rustc is required for the exact std-only wire preflight probe")
        columns = self.columns()
        names = []
        for column in columns:
            aliases = re.findall(r"\bAS ([a-z_]+)\b", column)
            names.append(aliases[-1] if aliases else column.rsplit(".", 1)[-1].split("::")[0])
        body = function_body((ROOT / PREFLIGHT).read_text(),
                             "pub(crate) fn validate_wp3_first_slice_body(")
        # Compile the actual preflight unchanged. Only its Error/macro descriptors
        # are supplied here; this grants no SQL, gameplay or runtime authority.
        harness = '''type Error = String;
macro_rules! err_protocol { ($($arg:tt)*) => { format!($($arg)*) }; }
''' + body + '''
fn description(names: &[&str]) -> Vec<u8> {
    let mut bytes = (names.len() as u16).to_be_bytes().to_vec();
    for name in names { bytes.extend_from_slice(name.as_bytes()); bytes.push(0); bytes.extend_from_slice(&[0;18]); }
    bytes
}
fn main() {
    let names: &[&str] = &''' + json.dumps(names) + ''';
    assert!(validate_wp3_first_slice_body(&description(names)).is_ok());
    let overflow = vec!["field"; 33];
    assert!(validate_wp3_first_slice_body(&description(&overflow)).unwrap_err().contains("field count exceeds 32"));
    let old_ground = vec!["field"; 35];
    assert!(validate_wp3_first_slice_body(&description(&old_ground)).unwrap_err().contains("field count exceeds 32"));
}
'''
        with tempfile.TemporaryDirectory(prefix="ground-wire-preflight-") as directory:
            source = Path(directory) / "probe.rs"
            binary = Path(directory) / "probe"
            source.write_text(harness)
            subprocess.run([rustc, "--edition=2024", "-D", "warnings", str(source),
                            "-o", str(binary)], check=True, capture_output=True, text=True)
            subprocess.run([str(binary)], check=True, capture_output=True, text=True)

    def test_projection_counter_keeps_nested_and_quoted_commas_inside_fields(self):
        self.assertEqual(projection_columns(
            "SELECT ARRAY['a,b',NULL], (SELECT count(*) FROM nested), i.id FROM items i"),
            ["ARRAY['a,b',NULL]", "(SELECT count(*) FROM nested)", "i.id"])


if __name__ == "__main__":
    unittest.main()
