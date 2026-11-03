"""Read immutable Git objects at the prescribed donor pin, never a dirty worktree."""

import subprocess
from pathlib import Path

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
COHORTS = {
    "dark_trails": [
        "jacob.lua",
        "barnabas_dee.lua",
        "doubleday.lua",
        "terrence.lua",
        "a_beggar.lua",
        "roswitha.lua",
        "the_beggar_king.lua",
        "jondrin.lua",
        "elliott.lua",
        "marvin.lua",
        "ezebeth.lua",
        "quandons_ghost.lua",
        "sholley.lua",
        "barazbaz.lua",
    ],
    "spike_tasks": [
        "gnomilly.lua",
        "gnombold.lua",
        "a_lost_thrower.lua",
        "a_dragon_lord.lua",
        "gnomargery.lua",
        "a_wyrm.lua",
        "a_vulcongra.lua",
        "a_lost_basher.lua",
        "gnome_trooper.lua",
        "a_drillworm.lua",
        "a_behemoth.lua",
        "a_nightmare_scion.lua",
        "a_lost_husher.lua",
    ],
    "what_a_foolish_quest": [
        "carina.lua",
        "lorbas.lua",
        "bozo.lua",
        "mirabell.lua",
        "ariella.lua",
        "nah_bob.lua",
        "markwin.lua",
        "yaman.lua",
        "the_orc_king.lua",
        "hairycles.lua",
        "hjaern.lua",
        "avar_tar.lua",
        "kazzan.lua",
        "sam.lua",
        "theodore_loveless.lua",
        "simon_the_beggar.lua",
        "wyda.lua",
    ],
}
MARKERS = {
    "dark_trails": "Storage.Quest.U10_50.DarkTrails",
    "spike_tasks": "Storage.Quest.U10_20.SpikeTaskQuest",
    "what_a_foolish_quest": "Storage.Quest.U8_1.WhatAFoolishQuest",
}


class Source:
    def __init__(self, root):
        self.root = Path(root).resolve()
        self._read_cache: dict[str, str] = {}
        self._cohort_cache: dict[str, tuple[str, ...]] = {}
        actual = self.git("rev-parse", "--verify", PIN + "^{commit}").strip()
        if actual != PIN:
            raise ValueError("Crystal commit does not match the prescribed pin")

    def git(self, *arguments):
        return subprocess.run(
            ["git", "-C", str(self.root), *arguments], check=True, capture_output=True
        ).stdout.decode("utf-8")

    def read(self, path):
        if path in self._read_cache:
            return self._read_cache[path]
        if not path.startswith(("data-global/", "data/")) or ".." in Path(path).parts:
            raise ValueError("Donor path outside the reviewed data roots")
        self._read_cache[path] = self.git("show", PIN + ":" + path)
        return self._read_cache[path]

    def cohort(self, quest):
        if quest in self._cohort_cache:
            return self._cohort_cache[quest]
        found = self.git(
            "grep", "-l", "-F", MARKERS[quest], PIN, "--", "data-global/npc"
        ).splitlines()
        names = {
            Path(path.split(":", 1)[1]).name for path in found if path.endswith(".lua")
        }
        expected = COHORTS[quest]
        if names != set(expected):
            raise ValueError(
                f"{quest}: pinned NPC census differs: {sorted(names ^ set(expected))}"
            )
        self._cohort_cache[quest] = tuple(expected)
        return self._cohort_cache[quest]
