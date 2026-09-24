#!/usr/bin/env python3
"""Fetch two exact GitHub Actions artifacts by immutable ID and verify their bytes."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import sys
import urllib.error
import urllib.request
import zipfile
from urllib.parse import urlparse

REPOSITORY = "Oteryn/Oteryn-Game"
API_ROOT = "https://api.github.com/repos/Oteryn/Oteryn-Game/actions/artifacts"
ARTIFACTS = {
    "g1": {
        "id": 10798668295,
        "name": "full-content-source-discovery-f1d7dbd6577b033c53545d8650ffba05aafc9480",
        "size_in_bytes": 1147357,
        "digest": "sha256:bf08a2715891d138b5db4ed4be0a69034770315e8d67f3e83867f22b9afc9863",
        "run_id": 35977349690,
        "head_sha": "f1d7dbd6577b033c53545d8650ffba05aafc9480",
        "members": ("source-universe.json", "manifest.json"),
    },
    "item": {
        "id": 10778892407,
        "name": "item-wiki-first-identity-crosswalk-61d051a13329c51ae04d8a011655e279664c334c",
        "size_in_bytes": 246686,
        "digest": "sha256:834c10d3dfb24b8857ffd666046743c96ee4093e7e09ef170dcd477ffea91b43",
        "run_id": 35925860576,
        "head_sha": "61d051a13329c51ae04d8a011655e279664c334c",
        "members": ("crosswalk-first.json", "manifest-first.json"),
    },
}
EXPECTED = {
    "g1_full_output_sha256": "3291bf4ac148a3c28921be34ccec2db977e8ebf5dde82928263b93ddfc1a2ae5",
    "g1_stable_sha256": "17f72a8f63861244b3193e639c3e33a530b641b77a7539653cc066a70c093c6c",
    "item_crosswalk_output_sha256": "f5bb9724bff51b99c35789dea986635e73fa9bdc6df77bc7f4e4a105768bed80",
    "item_wiki_stable_sha256": "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a",
    "item_classification_sha256": "004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d",
    "crystal_items_sha256": "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb",
    "crystal_revision": "ff7ede593c69d4c658b382c97443e8155926924a",
}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def api_json(url: str, token: str) -> dict:
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "Oteryn-G2-Source-Overlap/1.0",
    })
    with urllib.request.urlopen(request, timeout=30) as response:
        value = json.load(response)
    if not isinstance(value, dict):
        raise ValueError("GitHub artifact metadata was not an object")
    return value


def redirect_url(zip_api_url: str, token: str) -> str:
    request = urllib.request.Request(zip_api_url, headers={
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "Oteryn-G2-Source-Overlap/1.0",
    })
    opener = urllib.request.build_opener(NoRedirect())
    try:
        opener.open(request, timeout=30)
    except urllib.error.HTTPError as error:
        if error.code != 302:
            raise RuntimeError(f"artifact ZIP endpoint returned HTTP {error.code}") from error
        location = error.headers.get("Location")
        if not location or urlparse(location).scheme != "https":
            raise RuntimeError("artifact ZIP redirect had no HTTPS Location") from error
        return location
    raise RuntimeError("artifact ZIP endpoint did not redirect")


def verify_successful_run(spec: dict, token: str) -> None:
    run = api_json(f"https://api.github.com/repos/{REPOSITORY}/actions/runs/{spec['run_id']}", token)
    if run.get("id") != spec["run_id"] or run.get("head_sha") != spec["head_sha"]:
        raise ValueError(f"workflow run {spec['run_id']} ID/head mismatch")
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        raise ValueError(f"workflow run {spec['run_id']} did not complete successfully")


def download_archive(spec: dict, token: str, destination: Path) -> str:
    verify_successful_run(spec, token)
    meta = api_json(f"{API_ROOT}/{spec['id']}", token)
    expected_fields = ("id", "name", "size_in_bytes", "digest")
    for field in expected_fields:
        if meta.get(field) != spec[field]:
            raise ValueError(f"artifact {spec['id']} metadata mismatch: {field}")
    run = meta.get("workflow_run", {})
    if meta.get("expired") is not False:
        raise ValueError(f"artifact {spec['id']} is expired or expiry state is unknown")
    if run.get("id") != spec["run_id"] or run.get("head_sha") != spec["head_sha"]:
        raise ValueError(f"artifact {spec['id']} workflow run/head mismatch")

    location = redirect_url(f"{API_ROOT}/{spec['id']}/zip", token)
    # Deliberately issue the second request without the GitHub bearer token.
    with urllib.request.urlopen(urllib.request.Request(location), timeout=120) as response:
        hasher = hashlib.sha256()
        byte_count = 0
        with destination.open("wb") as output:
            while block := response.read(1024 * 1024):
                byte_count += len(block)
                if byte_count > spec["size_in_bytes"]:
                    raise ValueError(f"artifact {spec['id']} ZIP exceeded expected size")
                hasher.update(block)
                output.write(block)
    if byte_count != spec["size_in_bytes"]:
        raise ValueError(f"artifact {spec['id']} ZIP size mismatch: {byte_count}")
    digest = "sha256:" + hasher.hexdigest()
    if digest != spec["digest"]:
        raise ValueError(f"artifact {spec['id']} ZIP digest mismatch: {digest}")
    return hasher.hexdigest()


def extract_exact_members(archive_path: Path, spec: dict, out_dir: Path, stem: str) -> list[Path]:
    outputs = []
    with zipfile.ZipFile(archive_path) as archive:
        names = archive.namelist()
        if sorted(names) != sorted(spec["members"]):
            raise ValueError(f"artifact {spec['id']} member list mismatch: {names!r}")
        for member in spec["members"]:
            value = json.loads(archive.read(member))
            if not isinstance(value, dict):
                raise ValueError(f"artifact member is not a JSON object: {member}")
            destination = out_dir / f"{stem}-{member}"
            destination.write_text(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
            outputs.append(destination)
    return outputs


def verify_embedded_inputs(out_dir: Path) -> None:
    g1 = json.loads((out_dir / "g1-source-universe.json").read_text(encoding="utf-8"))
    g1m = json.loads((out_dir / "g1-manifest.json").read_text(encoding="utf-8"))
    item = json.loads((out_dir / "item-crosswalk-first.json").read_text(encoding="utf-8"))
    itemm = json.loads((out_dir / "item-manifest-first.json").read_text(encoding="utf-8"))
    if g1m.get("full_output", {}).get("sha256") != EXPECTED["g1_full_output_sha256"]:
        raise ValueError("G1 full-output digest mismatch")
    if g1m.get("full_output", {}).get("stable_without_retrieval_timestamp_sha256") != EXPECTED["g1_stable_sha256"]:
        raise ValueError("G1 stable digest mismatch")
    if g1m.get("counts", {}).get("live_unique_pages") != 9373:
        raise ValueError("G1 live page count mismatch")
    if set(g1m.get("hard_exclusions", [])) != {"Kalkulatory", "Narzędzie do nasycania", "Dostawca"}:
        raise ValueError("G1 hard-exclusion manifest mismatch")
    if g1m.get("invariants", {}).get("hard_exclusions_absent") is not True:
        raise ValueError("G1 hard-exclusion absence not proven")
    item_lanes = [lane for lane in g1.get("sealed_lanes", []) if lane.get("root_id") == "items-protected"]
    if len(item_lanes) != 1 or item_lanes[0].get("stable_digest") != EXPECTED["item_wiki_stable_sha256"]:
        raise ValueError("G1 sealed Item digest mismatch")
    if itemm.get("schema") != "OTERYN_ITEM_WIKI_FIRST_IDENTITY_CROSSWALK_MANIFEST/v1":
        raise ValueError("unexpected Item crosswalk manifest schema")
    if itemm.get("full_output", {}).get("sha256") != EXPECTED["item_crosswalk_output_sha256"]:
        raise ValueError("Item crosswalk output digest mismatch")
    digests = itemm.get("input_digests", {})
    for field, expected in (
        ("wiki_first_census_stable_sha256", EXPECTED["item_wiki_stable_sha256"]),
        ("classification_crosswalk_sha256", EXPECTED["item_classification_sha256"]),
        ("crystal_items_sha256", EXPECTED["crystal_items_sha256"]),
        ("crystal_revision", EXPECTED["crystal_revision"]),
    ):
        if digests.get(field) != expected:
            raise ValueError(f"Item crosswalk embedded input digest mismatch: {field}")
    if itemm.get("counts", {}).get("source_pages") != 6918 or len(item.get("records", [])) != 6918:
        raise ValueError("protected Item population mismatch")


def main() -> None:
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        raise SystemExit("GITHUB_TOKEN is required")
    if len(sys.argv) != 2:
        raise SystemExit("usage: global_source_overlap_fetch_artifacts.py OUT_DIR")
    out_dir = Path(sys.argv[1])
    out_dir.mkdir(parents=True, exist_ok=True)
    archive_dir = out_dir / "zip"
    archive_dir.mkdir()
    id_digests = {}
    for stem, spec in ARTIFACTS.items():
        archive = archive_dir / f"{stem}-{spec['id']}.zip"
        id_digests[stem] = download_archive(spec, token, archive)
        extract_exact_members(archive, spec, out_dir, stem)
    verify_embedded_inputs(out_dir)
    verified = {}
    for key, spec in ARTIFACTS.items():
        verified[f"{key}_artifact_id"] = spec["id"]
        verified[f"{key}_run_id"] = spec["run_id"]
        verified[f"{key}_head_sha"] = spec["head_sha"]
        verified[f"{key}_archive_sha256"] = id_digests[key]
    (out_dir / "verified-archive-digests.json").write_text(
        json.dumps(verified, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(verified, sort_keys=True))


if __name__ == "__main__":
    main()
