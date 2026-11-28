"""Complete an explicit World package with the native writer."""

from __future__ import annotations

import argparse
import json
import os
import stat
import subprocess
import tempfile
from collections import Counter
from pathlib import Path

from prebuilt import executable as prebuilt_executable

DOCUMENTS = (
    "assets/catalog.json",
    "content.lock.json",
    "definitions/declarations.json",
    "definitions/reference.json",
    "editor/author.json",
    "manifest.json",
    "presentations/bindings.json",
    "project.json",
    "provenance/imports.json",
    "provenance/sources.json",
    "worlds/world.json",
)
OWNED_DOCUMENTS = frozenset(
    {
        "content.lock.json",
        "definitions/declarations.json",
        "definitions/reference.json",
        "manifest.json",
        "project.json",
        "provenance/imports.json",
        "provenance/sources.json",
    }
)


def read_documents(root: Path, *, exact: bool = False) -> dict[str, bytes]:
    if root.is_symlink() or not root.is_dir():
        raise ValueError("World package must be an existing real directory")
    if exact:
        entries = list(root.rglob("*"))
        if any(path.is_symlink() for path in entries):
            raise ValueError("native output contains a symlink")
        files = {
            path.relative_to(root).as_posix() for path in entries if path.is_file()
        }
        if files != set(DOCUMENTS):
            raise ValueError(
                "native output must contain exactly the eleven managed documents"
            )
    result = {}
    for locator in DOCUMENTS:
        path = root
        for part in Path(locator).parts:
            path = path / part
            if path.is_symlink():
                raise ValueError(f"managed document path contains a symlink: {locator}")
        if not path.is_file():
            raise ValueError(f"managed document is absent: {locator}")
        result[locator] = path.read_bytes()
    return result


def install_trade_successors(repo_root: Path, package: Path) -> None:
    from crystal_npc_trade import install_crystal_npc_trade
    from import_cyclopedia_global import apply_trade_source, load_capture

    reference = json.loads((package / "definitions/reference.json").read_bytes())
    apply_trade_source(package, load_capture(repo_root), reference, Counter())
    install_crystal_npc_trade(repo_root, package)


def complete_world_project(
    repo_root: Path,
    input_root: Path,
    *,
    include_world_successors: bool = False,
    canonicalizer_manifest: Path | None = None,
    materializer: Path | None = None,
) -> dict[str, int | str]:
    from buddel_travel import install_buddel
    from crystal_creature_parity import PACKET as PARITY_PACKET
    from crystal_creature_parity import install_documents as install_parity
    from crystal_spawn_creatures import PATH as SPAWN_PACKET
    from crystal_spawn_creatures import install as install_spawn_creatures
    from questpc7_thaian_authoring import _install_documents
    from regenerate_content import source_cohort_extent
    from travel_route_corrections import project_routes

    repo_root = repo_root.resolve(strict=True)
    input_root = input_root.absolute()
    if canonicalizer_manifest is None:
        manifest = os.environ.get("OTERYN_WORLD_CANONICALIZER_MANIFEST")
        if manifest is not None:
            canonicalizer_manifest = Path(manifest)
    use_materializer = canonicalizer_manifest is None and materializer is not None
    if canonicalizer_manifest is not None:
        # An explicitly supplied artifact must qualify; never hide a stale or
        # corrupted producer behind a new, unreported fallback build.
        command = [
            str(prebuilt_executable("canonicalizer", canonicalizer_manifest, repo_root))
        ]
    elif use_materializer:
        command = [str(materializer.resolve(strict=True))]
    else:
        command = [
            "cargo",
            "run",
            "--locked",
            "-p",
            "oteryn-gameplay-content",
            "--bin",
            "oteryn-world-project-canonicalize",
            "--",
        ]
    original = read_documents(input_root)
    # Stage this caller's package, never substitute the repository's generated World.
    # A failed admission or writer must leave the caller's package untouched.
    with tempfile.TemporaryDirectory(prefix="oteryn-world-canonical-") as temporary:
        staging_root = Path(temporary) / "input"
        output_root = Path(temporary) / "output"
        for locator, data in original.items():
            path = staging_root / locator
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        _install_documents(repo_root, staging_root)
        if (repo_root / SPAWN_PACKET).is_file():
            install_spawn_creatures(repo_root, staging_root)
        if (repo_root / PARITY_PACKET).is_file():
            install_parity(repo_root, staging_root)
        if include_world_successors:
            from pema_display_name import install_pema_display_name
            from questpc7_world_seal import refresh
            from regional_npc_dialogue import install as install_dialogue
            from regional_npc_prices import install as install_prices

            install_buddel(repo_root, staging_root)
            project_routes(repo_root, staging_root)
            install_trade_successors(repo_root, staging_root)
            install_prices(repo_root, staging_root)
            install_dialogue(repo_root, staging_root)
            install_pema_display_name(staging_root)
            refresh(repo_root, staging_root)
        authored_extent = source_cohort_extent(staging_root)
        # Native capture enforces the existing bounded source limits before serialization.
        output_args = ["--output-root", str(output_root)]
        if use_materializer:
            writer_args = [*output_args, "--canonicalize-root", str(staging_root)]
        else:
            writer_args = ["--input-root", str(staging_root), *output_args]
        subprocess.run([*command, *writer_args], cwd=repo_root, check=True)
        canonical = read_documents(output_root, exact=True)
        if source_cohort_extent(output_root) != authored_extent:
            raise ValueError("native rewrite changed the authored source cohort")
        for locator in set(DOCUMENTS) - OWNED_DOCUMENTS:
            if canonical[locator] != original[locator]:
                raise ValueError(
                    f"native rewrite changed an unowned document: {locator}"
                )
        if read_documents(input_root) != original:
            raise ValueError("World input changed while completing the native package")

        # Prepare every replacement before changing any managed input. The four unowned
        # documents and all unmanaged siblings retain their original files and bytes.
        replacements = []
        try:
            for locator in sorted(OWNED_DOCUMENTS):
                if canonical[locator] == original[locator]:
                    continue
                destination = input_root / locator
                with tempfile.NamedTemporaryFile(
                    dir=destination.parent, delete=False
                ) as staged:
                    staged_path = Path(staged.name)
                    replacements.append((staged_path, destination))
                    staged.write(canonical[locator])
                staged_path.chmod(stat.S_IMODE(destination.stat().st_mode))
            for staged_path, destination in replacements:
                os.replace(staged_path, destination)
        finally:
            for staged_path, _ in replacements:
                staged_path.unlink(missing_ok=True)
    return authored_extent


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--input-root", type=Path, required=True)
    writer = parser.add_mutually_exclusive_group()
    writer.add_argument("--materializer", type=Path)
    writer.add_argument(
        "--canonicalizer-manifest",
        type=Path,
        help="same-head manifest from content-migration/prebuilt.py build",
    )
    args = parser.parse_args()
    extent = complete_world_project(
        args.repo_root,
        args.input_root,
        include_world_successors=True,
        canonicalizer_manifest=args.canonicalizer_manifest,
        materializer=args.materializer,
    )
    print(
        json.dumps(
            {"canonical_documents": len(DOCUMENTS), "source_cohort": extent},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
