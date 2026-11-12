"""Render batch-three evidence from independent, per-quest status inputs.

Normal operation needs only the status files present in this checkout. The
optional import command reconciles the original inventory with worker evidence
by (quest, step, complete donor array), never by the display name alone.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
AUTHORING = "tools/content-schema/quest-authoring/batch3_status"
EVIDENCE = "docs/agents/evidence/quests-3-20261102"
MAIN = "docs/agents/evidence/OTV2-20261102-quests-3-20261102.md"
SCHEMA = "OTERYN_QUESTS3_IMPLEMENTATION_STATUS/v1"
CRYSTAL = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:"
PR = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-"
ANSWERS = [PR + value for value in ("6589517205", "6589900671", "6590300355", "6590725297", "6591050910", "6598199611")]
QUESTS = (
    "a_pirates_tail_quest", "bigfoot_burden", "cradle_of_monsters", "demon_helmet",
    "fathers_burden", "heart_of_destruction", "hero_of_rathleton", "hot_cuisine",
    "in_service_of_yalahar", "marapur", "oramond", "thais_quest", "the_annihilator",
    "the_cursed_crystal", "the_hidden_city_of_beregar", "the_order_of_the_stag",
    "the_primal_ordeal", "the_shattered_isles_quest", "the_way_of_the_monk",
    "tower_defence_quest", "triangle_tower_quest",
)
GROUPS = {
    "a": ("bigfoot_burden", "fathers_burden", "hot_cuisine", "in_service_of_yalahar",
          "the_hidden_city_of_beregar", "the_cursed_crystal", "the_way_of_the_monk"),
    "b": ("a_pirates_tail_quest", "hero_of_rathleton", "marapur", "oramond", "the_shattered_isles_quest"),
    "c": ("cradle_of_monsters", "heart_of_destruction", "the_order_of_the_stag", "the_primal_ordeal", "tower_defence_quest"),
}
STATUS_FILES = (
    "group-a/implementation-status.json", "group-b/implementation-status.json",
    "group-c/status-after-answer.json", "root/implementation-status.json",
)
INVENTORY_FIELDS = ("quest", "step", "pattern", "trigger", "guard", "effect", "donor", "oteryn", "import_mode")
QUESTION_IDS = {
    "R-API": "6589101484", "R-MONK-VOCATION": "6589101991", "R-MONK-MANOP": "6589102441",
    "R-ANNIHILATOR-GROUP": "6589773424", "R-TRIANGLE-LEVER": "6590373823",
    "C-STAG-TARGET": "6589113974", "C-PRIMAL-BAG": "6589115035", "C-PRIMAL-OUTFIT": "6589115457",
    "B-Q01": "6589104570", "B-Q02": "6589105205", "B-Q03A": "6589105690",
    "B-Q03B": "6589106217", "B-Q03C": "6589106659", "B-Q04": "6589107159",
    "B-Q05": "6589107579", "B-Q06": "6589110214", "B-Q07": "6589110733",
    "B-Q08": "6589111177", "B-Q09": "6589111675",
}
E_HEADER = "## E. Implementacja i pozostały zakres"


# These decisions record accepted scope, not new runtime authority.
SCOPE_DECISIONS = [{'name': 'R-API',
  'question': '6589101484',
  'answer': '6589517205',
  'left_out': 'Nowe porównanie Global pozostaje NIEZNANE/API403 (wariant B); raport0 jest źródłem pośrednim. '
              'Brakuje dostępnego snapshotu MediaWiki z wikitextem i revid. Nie powstają elementy tylko Global.',
  'sources': ['data-global/scripts/quests/the_way_of_the_monk/actions_mainland_shrines.lua:58']},
 {'name': 'R-NPC-STATE',
  'question': '6589624211',
  'answer': '6589900671',
  'left_out': 'Brakuje produkcyjnego wykonania reguł NPC przez istniejący, niezmieniony adapter i writer. '
              'Zachowano pełne dane/dialogi; rozszerzenia adaptera rozmów, greetingów i zapisu stanu są Left out.',
  'sources': ['data-global/npc/strange_pipe.lua:59']},
 {'name': 'R-MAP-OWNER',
  'question': '6589772967',
  'answer': '6589900671',
  'left_out': 'Demon Helmet wymaga tworzenia/usuwania teleportu 1949, ewakuacji istot i ścian StepOut; Life Ring '
              'wymaga zmiany sześciu pól 4610/5770 i przeniesienia istot z tych pól. Brakuje operacji wspólnego '
              'właściciela mapy; pozostają Left out.',
  'sources': ['data-global/scripts/quests/demon_helmet/action-lever.lua:4-22',
              'data-global/scripts/quests/demon_helmet/movement-walls.lua:1-49',
              'data-global/scripts/quests/thais_quest/actions_life_ring_lever.lua:1-37']},
 {'name': 'R-ANNIHILATOR-GROUP',
  'question': '6589773424',
  'answer': '6589900671',
  'left_out': 'Brakuje atomowego admission dokładnie czterech graczy poziomu >=100, kontroli zajętości, '
              'teleportacji grupy, spawnu sześciu Angry Demon i zmiany dźwigni 2773. Brak źródłowego cooldownu 24 '
              'h; nie dodano takiego zamiennika.',
  'sources': ['data-global/scripts/quests/the_annihilator/lever.lua:1-83']},
 {'name': 'R-CHARGED-REWARDS',
  'question': '6589773836',
  'answer': '6589900671',
  'left_out': 'Brakuje atomowego wydania naładowanych nagród jako 3085 x1 / 200 charges oraz 3083 x1 / 150 '
              'charges. Pełne nagrody i ich pozostałe składniki pozostają w hold; liczba ładunków nie staje się '
              'liczbą przedmiotów.',
  'sources': ['data-global/startup/tables/chest.lua:1593-1599', 'data-global/startup/tables/chest.lua:1831-1838']},
 {'name': 'R-CALLBACK-SCHEMA',
  'question': '6589971017',
  'answer': '6590300355',
  'left_out': 'Brakuje pól storage.read_only, failure_teleport, tekstów odmowy per guard oraz '
              'success_message/success_say. Rozszerzenia parsera są Left out. Nie wolno zastąpić odczytu storage '
              'operacją SET, pominąć teleportu odmowy ani tekstu; TALKTYPE_MONSTER_SAY wymaga odrębnej '
              'kwalifikacji prezentacji.',
  'sources': ['data-global/scripts/quests/in_service_of_yalahar/actions_mechanism.lua:22-33',
              'data-global/scripts/quests/bigfoot_burden/movements_gnomebase_teleport.lua:78-94',
              'data-global/scripts/movements/teleport/turtles.lua:20-26']},
 {'name': 'R-ANNIHILATOR-DOOR',
  'question': '6590373502',
  'answer': '6590725297',
  'left_out': 'Brakuje obsługi odwróconego guardu Reward!=1, otwarcia 5113->5114 z teleportacją oraz zamknięcia z '
              'checkCreatureInsideDoor w istniejącym ownerze drzwi. Bez konkurencyjnego handlera Use.',
  'sources': ['data-global/scripts/quests/the_annihilator/door.lua:4-24']},
 {'name': 'R-TRIANGLE-LEVER',
  'question': '6590373823',
  'answer': '6590725297',
  'left_out': 'Brakuje source-literal mutacji świata: remove 1270, create 1025 i zmiany dźwigni. Asymetria ID nie '
              'została uznana za zatwierdzoną poprawkę błędu donora.',
  'sources': ['data-global/scripts/quests/triangle_tower_quest/actions_triangle_tower_lever.lua:3-18']},
 {'name': 'R-USE-ORDINAL',
  'question': '6590420434',
  'answer': '6590725297',
  'left_out': 'Brakuje dopasowania selektora pozycyjnego do rzeczywiście klikniętego obiektu ponad gruntem; '
              'obecny adapter wymaga ordinal==0. Dziewięć callbacków (Blue Valley 4, Cheesy exit 1, Stag 4) '
              'pozostaje held; owner użycia bez zmian.',
  'sources': ['data-global/scripts/quests/the_way_of_the_monk/actions_blue_valley_teleports.lua:15-25',
              'data-global/scripts/quests/a_pirates_tail_quest/actions_cheesy_key.lua:110-120',
              'data-global/scripts/quests/the_order_of_the_stag/isle_of_ada.lua:134']},
 {'name': 'R-KILL-ADAPTER',
  'question': '6590527568',
  'answer': '6590725297',
  'left_out': 'Dziesięć reguł śmierci pozostaje nieaktywnymi danymi. Brakuje utrwalenia właściwego '
              'rosteru/occurrence i rozliczenia przed zwolnieniem corpse-pin, z kwalifikacją graczy według damage '
              'map. Parser quest_batch3_c i sześć hooków zostały wycofane; cztery testy Tower nie dowodzą '
              'aktywnego runtime. Przyszły adapter musi też rozwiązać trwały ProgressionContextMismatch mapowany '
              'na Retry, który może blokować uporządkowaną kolejkę (character_progression_binding.rs:200-203; '
              'kill_reward.rs:2132-2134). Fire-Fighter grant/repair pozostaje Left out.',
  'sources': ['data/libs/functions/functions.lua:1075-1081',
              'data-global/scripts/quests/tower_defence_quest/creaturescripts_goblin_leader_kill.lua:3-8',
              'data-global/scripts/quests/the_primal_ordeal/creaturescripts_magma_bubble_death.lua:12-23',
              'data-global/scripts/quests/the_primal_ordeal/creaturescripts_the_primal_menace_killed.lua:4-18',
              'data-global/scripts/quests/heart_of_destruction/creaturescripts_heart_boss_death.lua:26-55',
              'data-global/scripts/quests/the_order_of_the_stag/court_warlock_fight.lua:323-329']},
 {'name': 'B-Q22',
  'question': '6590616025',
  'answer': '6590725297',
  'left_out': 'Brakuje wykonania StepIn przed natywną zmianą piętra. Item 39721 / NORTH2 na (33613,32759,9) '
              'przejmuje ruch przed źródłowym callbackiem do (33614,32759,8). Pełny callback pozostaje held.',
  'sources': ['data-global/scripts/quests/marapur/movements_ramp_to_north-temple.lua:9-16']},
 {'name': 'B-Q23',
  'question': '6590862560',
  'answer': '6591050910',
  'left_out': 'Pięć pełnych callbacków Timiry pozostaje held: brak kanonicznego Item/map object 35500 i jego '
              'kwalifikowanego bindingu. Klucz donor:crystalserver@00ce02a5:item/35500 jest provisional. Nie '
              'zastąpiono selektora, nie utworzono item.tibia.i35500; syntetyczna paleta PG nie kwalifikuje '
              'produkcyjnej mapy.',
  'sources': ['data-global/scripts/quests/marapur/actions_teleports_timira.lua:2',
              'data-global/scripts/quests/marapur/actions_teleports_timira.lua:38']},
 {'name': 'C-CRADLE-GROUND',
  'question': '6590915899',
  'answer': '6591050910',
  'left_out': 'Oba SimpleTeleport pozostają held: brak kwalifikacji gruntu 40236/40230/40198/40196 oraz jego '
              'bindingu w bundle. WorldObject 34111 nie jest gruntem; CallbackTeleport::check wymaga '
              'Ground::Walkable. Bez zmiany mapy, itemów ani syntetycznego PG jako dowodu bindingu.',
  'sources': ['data-global/scripts/actions/bosses_levers/the_monster.lua:26-28']},
 {'name': 'C-PRIMAL-COLLISION',
  'question': '6590916295',
  'answer': '6591050910',
  'left_out': 'Cztery przejścia Primal pozostają held: brak kwalifikowanego profilu kolizji statycznych itemów '
              '34111, 6383 i 3919 przez boot_bundle_items->BundleFacts::palette; bez profilu domyślnie blokują '
              'ruch. Bez nowych profili, zmian mapy lub obejścia odmowy teleportu. Aktywny pozostaje wyłącznie '
              'kandydat (33558,32754,14)->(33714,32799,14).',
  'sources': ['data-global/scripts/movements/teleport/gnomprona.lua:2-7',
              'data-global/scripts/movements/teleport/gnomprona.lua:30-33']},
 {'name': 'B-Q12',
  'question': '6589875169',
  'answer': '6598199611',
  'left_out': 'Brakuje wspólnego ownera całych encounterów Tentugly/Ratmiral: admission, uczestnicy, fazy i '
              'obiekty walki, obrażenia, cooldown, rozliczenie śmierci i cleanup. Reguły źródłowe pozostają '
              'danymi; podłączenie nastąpi w osobnym PR shared boss encounter.',
  'sources': ["data-global/scripts/quests/a_pirates_tail_quest/actions_tentugly's_head.lua:57",
              'data-global/scripts/quests/a_pirates_tail_quest/creaturescripts_tentugly.lua:257',
              'data-global/scripts/quests/a_pirates_tail_quest/actions_ratmiral_blackwhiskers.lua:41',
              'data-global/scripts/quests/a_pirates_tail_quest/creaturescripts_blue_shield.lua:1']},
 {'name': 'B-Q13',
  'question': '6589875594',
  'answer': '6598199611',
  'left_out': 'Brakuje wspólnego ownera czterech aren Rathleton: maszyny i stan świata, fale/etapy, wejście '
              'uczestników, portale, nagrody i cleanup. Istniejące ataki Maxxena nie wykonują całego encounteru.',
  'sources': ['data-global/scripts/quests/hero_of_rathleton/actions_machines_deep_terror.lua:1',
              'data-global/scripts/quests/hero_of_rathleton/movements_deep_terror.lua:1',
              'data-global/scripts/quests/hero_of_rathleton/movements_glooth_horror.lua:18',
              'data-global/scripts/quests/hero_of_rathleton/movements_lava.lua:11',
              'data-global/scripts/quests/hero_of_rathleton/movements_professor_maxxen.lua:1',
              'data-global/scripts/quests/hero_of_rathleton/creaturescripts_bosses_kill.lua:47']},
 {'name': 'B-Q14',
  'question': '6589875997',
  'answer': '6598199611',
  'left_out': 'Brakuje wspólnej sesji encounteru Timiry: fazy i limity uczestników, timer sparkling, '
              'wiadra/tarcza i żywe obiekty świata, claimy sesji i cleanup. Dane zachowano; istniejące ataki nie '
              'zastępują tego grafu.',
  'sources': ['data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:154',
              'data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:203',
              'data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:262',
              'data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:298',
              'data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:340',
              'data-global/scripts/quests/marapur/actions_boss_timira_fight.lua:368']},
 {'name': 'B-Q15',
  'question': '6589876455',
  'answer': '6598199611',
  'left_out': 'Brakuje losowego celu Probing per postać z trwałym zapisem oraz atomowego nadania '
              'nagrody/progresu; roots/pollen wymagają transformacji żywego obiektu świata z opóźnionym '
              'przywróceniem. Stan trzymany tylko w pamięci procesu nie zastępuje trwałego ownera.',
  'sources': ['data-global/scripts/quests/oramond/probing/actions_mono_detector_strange_probing_device.lua:1',
              'data-global/scripts/quests/oramond/probing/actions_mono_detector_strange_probing_device.lua:37',
              'data-global/scripts/quests/oramond/probing/actions_mono_detector_strange_probing_device.lua:77',
              'data-global/scripts/quests/oramond/to_take_roots/actions_to_take_roots.lua:9',
              'data-global/scripts/quests/oramond/the_powder_of_the_stars/actions_blue_pollen.lua:19']},
 {'name': 'B-Q16',
  'question': '6589876957',
  'answer': '6598199611',
  'left_out': 'Brakuje operacji create item on tile wraz z atomowym zapisem progresu: Rake tworzy pierścionek '
              '6093 na (32422,32770,1) i zmienia GovernorDaughter 1->2.',
  'sources': ['data-global/scripts/actions/tools/rake.lua:9', 'data-global/scripts/actions/tools/rake.lua:47']},
 {'name': 'B-Q17',
  'question': '6589877321',
  'answer': '6598199611',
  'left_out': 'Brakuje jednej atomowej operacji capacity, mint 6125 x1 i timestamp EggTime+24h z bezpiecznym '
              'retry; odmowa capacity nie może zapisać terminu ani oddać przedmiotu na ziemię.',
  'sources': ['data-global/scripts/quests/the_shattered_isles_quest/action_tortoise_egg_nargor.lua:1']},
 {'name': 'B-Q18',
  'question': '6589877720',
  'answer': '6598199611',
  'left_out': 'Duncan wymaga odczytu cudzego tracka IceIslands.Questline>=9 oraz Animal Activist w tej samej '
              'wymianie jaja/progresu. Brakuje tej operacji w niezmienionym wspólnym ownerze rozliczenia.',
  'sources': ['data-global/npc/duncan.lua:159']},
 {'name': 'B-Q19',
  'question': '6589878136',
  'answer': '6598199611',
  'left_out': 'Firebug zachowuje jedną źródłową kolejność wspólnego handlera: Blood Brothers z early return przed '
              'Nargor. Brakuje kwalifikacji tej całej operacji w istniejącym ownerze; nie zarejestrowano drugiego '
              'use5467 ani nie pominięto obcego guardu.',
  'sources': ['data-global/scripts/quests/others/actions_fire_bug.lua:32',
              'data-global/scripts/quests/others/actions_fire_bug.lua:80',
              'data-global/scripts/quests/others/actions_fire_bug.lua:110']},
 {'name': 'R-MAP-BINDINGS',
  'question': '6590278878',
  'answer': '6598199611',
  'left_out': 'Fizyczne uruchomienie 60 callbacków pozostaje Left out: brak kwalifikowanego importu oryginalnych '
              'UID/AID do mapy. Audyt 1208 regionów: 25 wartości UID i 32 AID, razem 60 żądań callbacków, zero '
              'dopasowań. Zachowano pełne literalne dane i bezpieczną odmowę; nie dopisano zgadywanych '
              'selektorów.',
  'sources': ['data-global/scripts/quests/in_service_of_yalahar/actions_mechanism.lua:2-15',
              'data-global/scripts/quests/the_hidden_city_of_beregar/moviments_elevator.lua:2-3',
              'data-global/scripts/quests/the_cursed_crystal/movements_StepIn_CursedCrystal.lua:25-35',
              'data-global/scripts/quests/heart_of_destruction/movements_teleport_heart.lua:2-14',
              'data-global/scripts/quests/heart_of_destruction/movements_teleport.lua:2-11',
              'data-global/scripts/movements/teleport/turtles.lua:41',
              'data-global/scripts/quests/the_shattered_isles_quest/action_nargor_maps.lua:36',
              'data-global/scripts/quests/a_pirates_tail_quest/actions_cheesy_key.lua:75',
              'data-global/scripts/quests/hero_of_rathleton/movements_exit.lua:16']},
 {'name': 'B-Q21',
  'question': '6590288251',
  'answer': '6598199611',
  'left_out': 'Pełny Familiar chess callback pozostaje held: natywny teleport Item 34111 (ordinal 2) na '
              '(33734,31273,7) wykonuje się przed callbackiem, więc guard TentuglyKilled/RatmiralKilled nigdy nie '
              'rusza. Brakuje rozwiązania kolejności w ownerze mapy/movement.',
  'sources': ['data-global/scripts/quests/a_pirates_tail_quest/actions_cheesy_key.lua:122-151']}]
QUALIFICATION_NOTE = 'Wyniki bramek końcowego kandydata i dokładny przetestowany SHA są zapisywane przez root w sekcji Validation [PR2843](https://github.com/Oteryn/Oteryn-Game/pull/2843). Ten dokument opisuje zakres i źródła; nie przypisuje historycznych ani syntetycznych wyników do końcowego head. Identyfikatora commita nie umieszcza się we własnym drzewie.'


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def encode(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(row):
    refs = row.get("donor", row.get("sources", row.get("source", [])))
    refs = [refs] if isinstance(refs, str) else refs
    return row["quest"], row.get("step", row.get("element")), tuple(refs)


def row_id(key):
    return digest(json.dumps(key, ensure_ascii=False, separators=(",", ":")).encode())


def known_corrections():
    """Four reviewed errata; no name-only or fuzzy reconciliation fallback."""
    shattered = "the_shattered_isles_quest"
    npc = CRYSTAL + "data-global/npc/"
    fire = CRYSTAL + "data-global/scripts/quests/others/actions_fire_bug.lua:"
    chest = CRYSTAL + "data-global/startup/tables/chest.lua:"
    return {
        (shattered, "Jack Fate: losowy shipwreck", (npc + "jack_fate.lua:87", npc + "jack_fate.lua:95")):
            ((shattered, "Jack Fate: losowy shipwreck", (npc + "jack_fate.lua:74",)),
             "Cytat dotyczy właściwej gałęzi podróży i losowania na liniach87/95, nie wcześniejszego dialogu74."),
        (shattered, "Ariella: 100 chlebów (item3600)", (npc + "ariella.lua:159", CRYSTAL + "data/items/items.xml:11458")):
            ((shattered, "Ariella: 100 exotic fruit", (npc + "ariella.lua:159",)),
             "Poprawiono etykietę: removeItem(3600,100) i items.xml11458 określają100 chlebów; ilość i ID nie zmieniły się."),
        (shattered, "Firebug: sabotaż Nargor", (fire + "32", fire + "80", fire + "110")):
            ((shattered, "Firebug: sabotaż Nargor", (fire + "110",)),
             "Uzupełniono wcześniejszą gałąź Blood Brothers i kolejność wspólnego handlera; własna gałąź pozostaje strzeżona."),
        ("triangle_tower_quest", "Triangle Tower chest", (chest + "1831",)):
            (("triangle_tower_quest", "Triangle Tower chest", (chest + "1832",)),
             "Początek źródłowego rekordu skrzyni jest na linii1831; skorygowano jedną omyłkę numeru linii."),
    }


def unique(values):
    return list(dict.fromkeys(value for value in values if value))


def links(owner, names):
    result = []
    for name in names:
        value = owner.get(name)
        result.extend(value if isinstance(value, list) else [value])
    return unique(value for value in result if isinstance(value, str) and value.startswith(PR))


def normalize_row(original, owner, ordinal, correction):
    key = identity(original)
    inventory = {name: original[name] for name in INVENTORY_FIELDS}
    if correction:
        inventory.update(quest=owner["quest"], step=owner.get("step", owner.get("element")), donor=list(identity(owner)[2]))
        for target, source in (("pattern", "patterns"), ("trigger", "trigger"), ("guard", "guard"),
                               ("effect", "effect"), ("oteryn", "oteryn")):
            inventory[target] = owner.get(source, owner.get(target, inventory[target]))
    status = owner.get("implementation_status", owner.get("status"))
    if not status:
        raise ValueError(f"missing implementation status: {key}")
    enabled = status in {"WORLD_CALLBACK_DATA", "callback_authored"}
    reward_qualified = status == "qualified_pg"
    existing = status.startswith("EXISTING_") or status in {"existing_data", "existing_qualified"}
    done = owner.get("done", "")
    left_out = owner.get("left_out", "")
    if "reason" in owner:
        if enabled:
            done = owner["reason"]
        else:
            left_out = owner["reason"]
            done = ("Pełny program danych NPC zachowany; bez nowego produkcyjnego adaptera rozmów."
                    if status == "NPC_CAPTURE_RUNTIME_LEFT_OUT" else
                    "Istniejące dane zachowano bez drugiego wykonawcy." if existing else
                    "Przypięte źródła i decyzja o zakresie zachowane w inwentaryzacji.")
    questions = links(owner, ("question_url", "question", "additional_questions", "related_questions"))
    answers = links(owner, ("specific_answer", "answer_url", "answer", "npc_answer", "schema_answer", "coordinator_answer"))
    if left_out and (not questions or not answers):
        raise ValueError(f"Left out without QUESTION/ANSWER: {key}")
    if not done and not left_out:
        raise ValueError(f"unclassified inventory row: {key}")
    result = {
        "id": row_id(key), "quest_row": ordinal, "inventory": inventory,
        "implementation_status": status,
        "classification": "DERIVED",
        "delivery": "runtime_callback" if enabled else "qualified_reward" if reward_qualified else "existing_data" if existing else "source_only_or_left_out",
        "new_runtime_callback": enabled, "done": done, "left_out": left_out,
        "question_urls": questions, "answer_urls": answers,
        "implementation_paths": owner.get("implementation_paths", []),
        "sources": unique(list(identity(original)[2]) + list(identity(owner)[2]) + owner.get("sources", [])),
    }
    if correction:
        result["inventory_correction"] = {"original": dict(zip(("quest", "step", "donor"), (key[0], key[1], list(key[2])), strict=True)), "reason": correction}
    evidence_keys = ("holds", "additional_questions", "related_questions", "existing_sources", "verification_evidence",
                     "callback_key", "source_compiler_proof", "test", "test_result", "test_evidence", "runtime_scope")
    result["evidence"] = {name: owner[name] for name in evidence_keys if name in owner}
    return result


def content_counts(root, quest):
    group = next((key for key, members in GROUPS.items() if quest in members), None)
    candidates = []
    if group:
        candidates.append((f"content/dialogues/quest-rules/quest-batch3-{group}-{quest}.json", "npc_rules_source_only", "rules"))
        if group == "c":
            candidates += [(f"tools/content-schema/quest-authoring/batch3_c/world/{quest}.json", "world_callbacks_enabled", "callbacks"),
                           (f"tools/content-schema/quest-authoring/batch3_c/held-world/{quest}.json", "world_callbacks_held", "callbacks"),
                           (f"tools/content-schema/quest-authoring/batch3_c/deaths/{quest}.json", "death_rules_source_only", "rules")]
        else:
            candidates += [(f"content/quests/world-callbacks/batch3-{group}-{quest}.json", "world_callbacks_enabled", "callbacks"),
                           (f"content/quests/world-callbacks-held/batch3-{group}-{quest}.json", "world_callbacks_held", "callbacks")]
    counts = dict.fromkeys(("npc_rules_source_only", "world_callbacks_enabled", "world_callbacks_held", "death_rules_source_only"), 0)
    inputs = []
    for name, field, collection in candidates:
        path = root / name
        if not path.is_file():
            continue
        raw = path.read_bytes()
        document = json.loads(raw)
        count = len(document[collection])
        if field == "death_rules_source_only" and document.get("runtime_enabled") is not False:
            raise ValueError(f"death source must remain inactive: {name}")
        counts[field] += count
        inputs.append({"path": name, "sha256": digest(raw), "count": count, "category": field})
    return counts, inputs


def import_statuses(root, work, selected):
    original_rows = read(work / "root/all-rows.json")
    original = {identity(row): row for row in original_rows}
    if len(original) != len(original_rows) or len(original_rows) != 883:
        raise ValueError("expected883 distinct full inventory identities")
    aliases = known_corrections()
    reconciled = {}
    origins = {}
    for name in STATUS_FILES:
        path = work / name
        if not path.is_file():
            continue
        for owner in read(path)["rows"]:
            if owner["quest"] not in selected:
                continue
            key = identity(owner)
            correction = None
            if key not in original:
                if key not in aliases:
                    raise ValueError(f"unknown inventory identity; no fuzzy merge: {key}")
                key, correction = aliases[key]
            if key not in original or key in reconciled:
                raise ValueError(f"missing or duplicate inventory owner: {key}")
            reconciled[key] = (owner, correction)
            origins[owner["quest"]] = {"worker_status": name, "sha256": digest(path.read_bytes())}
    result = {}
    for quest in selected:
        own = [row for row in original_rows if row["quest"] == quest]
        missing = [identity(row) for row in own if identity(row) not in reconciled]
        if missing:
            raise ValueError(f"missing status rows for {quest}: {missing}")
        rows = [normalize_row(row, reconciled[identity(row)][0], index + 1, reconciled[identity(row)][1])
                for index, row in enumerate(own)]
        counts, inputs = content_counts(root, quest)
        runtime_rows = sum(row["new_runtime_callback"] for row in rows)
        if runtime_rows != counts["world_callbacks_enabled"]:
            raise ValueError(f"runtime status/projection count drift for {quest}: {runtime_rows}/{counts['world_callbacks_enabled']}")
        result[quest] = {
            "schema": SCHEMA, "quest": quest, "source_revision": CRYSTAL.split("@")[1].rstrip(":"),
            "inventory_origin": {"document": MAIN, "commit": "f2b37d4c6b313952750189d75e6d19d99fde8641", "identity_fields": ["quest", "step", "donor"]},
            "normalization_provenance": origins[quest], "classification": "DERIVED",
            "authority": "PR comments govern scope; this file records evidence and does not authorize a runtime adapter.",
            "coordinator_answers": ANSWERS, "row_count": len(rows),
            "import_counts": dict(Counter(row["inventory"]["import_mode"] for row in rows)),
            "content_counts": counts, "content_inputs": inputs,
            "final_head_validation": {"status": "PENDING_ROOT", "tested_sha": None,
                                      "note": "Historical scoped results below do not attest the final publication head."},
            "rows": rows,
        }
    return result


def validate(root, document):
    if document.get("schema") != SCHEMA or document.get("quest") not in QUESTS:
        raise ValueError("batch-three status schema/quest")
    rows = document["rows"]
    if len(rows) != document["row_count"] or len({row["id"] for row in rows}) != len(rows):
        raise ValueError("status row count/identity collision")
    for row in rows:
        if row["inventory"]["quest"] != document["quest"]:
            raise ValueError("foreign quest row")
        key = identity(row.get("inventory_correction", {}).get("original", row["inventory"]))
        if row["id"] != row_id(key):
            raise ValueError("full inventory identity drift")
        if not row["sources"] or any(not re.fullmatch(re.escape(CRYSTAL) + r"[^:]+:[1-9][0-9]*(?:-[1-9][0-9]*)?", ref) for ref in row["sources"]):
            raise ValueError(f"unpinned donor evidence: {row['id']}")
        if row["left_out"] and (not row["question_urls"] or not row["answer_urls"]):
            raise ValueError("Left out requires a question and an answer")
        if not row["done"] and not row["left_out"]:
            raise ValueError("inventory row has no outcome")
    counts, inputs = content_counts(root, document["quest"])
    if counts != document["content_counts"] or inputs != document["content_inputs"]:
        raise ValueError(f"status content input drift: {document['quest']}; reimport after source freeze")


def cell(value):
    return str(value).replace("|", "\\|").replace("\n", "<br>")


def link_list(urls, label):
    return "; ".join(f"[{label} {url.rsplit('-', 1)[-1]}]({url})" for url in urls) or "—"


def historical_tests(document):
    return unique(row["evidence"].get("test_result") for row in document["rows"])


def render_quest(document):
    quest = document["quest"]
    counts = document["content_counts"]
    lines = [f"# {quest} — stan implementacji partii3", "",
             "<!-- Generated by tools/content-schema/quest-authoring/batch3_report.py; edit the per-quest batch3_status input. -->", "",
             f"Źródło statusu: `{AUTHORING}/{quest}.json`. Klasyfikacja: **DERIVED**. Decyzje zakresu pochodzą z komentarzy PR; dokument nie jest zgodą na zmianę wspólnego kontraktu.", "",
             "## Zakres i znaczenie liczb", "",
             f"Inwentaryzacja: **{document['row_count']} kroków**; AUTO {document['import_counts'].get('AUTO', 0)}, RĘCZNIE {document['import_counts'].get('RĘCZNIE', 0)}. Identyfikator kroku obejmuje quest, nazwę i całą tablicę źródeł donora; powtarzające się nazwy nie zostały scalone.", "",
             "| Rodzaj danych | Liczba | Znaczenie |", "| --- | ---: | --- |",
             f"| Callbacki w aktywnym źródle starego formatu | {counts['world_callbacks_enabled']} | Zakres pojedynczych interakcji; kompletność questa i końcowy head kwalifikuje się osobno. |",
             f"| Pełne callbacki held | {counts['world_callbacks_held']} | Zachowano parametry i źródła; wykluczone z aktywnego agregatu. |",
             f"| Reguły NPC source-only | {counts['npc_rules_source_only']} | Pełne teksty i reguły; bez nowego adaptera rozmów. |",
             f"| Reguły śmierci source-only | {counts['death_rules_source_only']} | runtime_enabled=false; parser i adapter C wycofane. |", "",
             "Te liczby dotyczą rekordów różnych katalogów; nie są liczbą grywalnych questów ani sumą kroków inwentaryzacji.", "",
             "## Walidacja", "",
             QUALIFICATION_NOTE + " Komenda kwalifikacji projekcji tego dokumentu:", "",
             f"```bash\npython tools/content-schema/quest-authoring/batch3_report.py --quest {quest} --check\n```", ""]
    for result in historical_tests(document):
        lines.append("- Historyczny, ograniczony przebieg: " + result)
    if historical_tests(document):
        lines.append("")
    lines += ["## Pełna kontrola kroków", "",
              "Każdy poniższy krok ma opis wykonanej pracy albo konkretny Left out. Sam zapis danych i test source-only nie oznaczają wywołania produkcyjnego.", ""]
    for row in document["rows"]:
        inv = row["inventory"]
        lines += [f"### {row['quest_row']:03d}. {inv['step']}", "", f"Identyfikator: `{row['id']}`. Status: `{row['implementation_status']}`. Rodzaj: `{row['delivery']}`.", "",
                  "| Pole | Wartość |", "| --- | --- |",
                  f"| Wzorzec / import | {cell(inv['pattern'])} / {inv['import_mode']} |",
                  f"| Wyzwalacz | {cell(inv['trigger'])} |", f"| Warunek | {cell(inv['guard'])} |",
                  f"| Efekt źródłowy | {cell(inv['effect'])} |",
                  f"| Zrobione | {cell(row['done']) or '—'} |", f"| Left out | {cell(row['left_out']) or '—'} |",
                  f"| QUESTION | {link_list(row['question_urls'], 'QUESTION')} |",
                  f"| ANSWER | {link_list(row['answer_urls'], 'ANSWER')} |", "", "Źródła:", ""]
        lines += [f"- `{ref}`" for ref in row["sources"]]
        if row["implementation_paths"]:
            lines += ["", "Pliki implementacji/danych:", ""] + [f"- `{path}`" for path in row["implementation_paths"]]
        if correction := row.get("inventory_correction"):
            lines += ["", "Korekta raportu etapu1: " + correction["reason"],
                      f"Pierwotny klucz zachowano: `{json.dumps(correction['original'], ensure_ascii=False, separators=(',', ':'))}`."]
        for hold in row["evidence"].get("holds", []):
            lines += ["", "Brakujące pola/operacje: " + ", ".join(f"`{name}`" for name in hold.get("missing_fields", [])) + ".",
                      "Dowód różnicy zachowania: " + hold.get("proof", "")]
        for field in ("test_result", "test_evidence", "source_compiler_proof"):
            if value := row["evidence"].get(field):
                lines += ["", f"{field}: " + (json.dumps(value, ensure_ascii=False) if not isinstance(value, str) else value)]
        lines.append("")
    lines += ["## Pliki stanowiące podstawę liczników", "", "| Plik | SHA256 | Rekordy |", "| --- | --- | ---: |"]
    lines += [f"| `{entry['path']}` | `{entry['sha256']}` | {entry['count']} |" for entry in document["content_inputs"]]
    lines += ["", "## Decyzje zakresu", "", link_list(document["coordinator_answers"], "ANSWER"), ""]
    return "\n".join(lines)


def refresh_decisions(document):
    """Record all six accepted answers without changing inventory identities."""
    document["coordinator_answers"] = ANSWERS
    document["final_head_validation"] = {
        "status": "EXTERNAL_PR_EVIDENCE",
        "tested_sha": None,
        "evidence_url": "https://github.com/Oteryn/Oteryn-Game/pull/2843",
        "note": "Exact frozen publication SHA and its gate results are recorded in PR Validation; this source document does not attest a final-head PASS.",
    }
    for row in document["rows"]:
        if "R-MAP-BINDINGS" in row["left_out"]:
            row["question_urls"] = unique(row["question_urls"] + [PR + "6590278878"])
        for decision in SCOPE_DECISIONS:
            if PR + decision["question"] not in row["question_urls"]:
                continue
            row["answer_urls"] = unique(row["answer_urls"] + [PR + decision["answer"]])
            if row["left_out"]:
                marker = "Decyzja " + decision["name"] + ": "
                if marker not in row["left_out"]:
                    row["left_out"] += " " + marker + decision["left_out"]
            row["sources"] = unique(row["sources"] + [CRYSTAL + ref for ref in decision["sources"]])
    return document


def scope_decision_lines():
    lines = ["", "### Left out — wiążące decyzje i brakujące funkcje", "",
             "Każde źródło poniżej jest przypięte do Crystal 201189f8. Pełna kontrola 883 kroków pozostaje w dokumentach per quest; tabela wyjaśnia wspólne ograniczenia bez deklaracji kompletnego runtime.", "",
             "| Decyzja | Brakująca funkcja / zachowany zakres | Crystal plik:linia | QUESTION / ANSWER |",
             "| --- | --- | --- | --- |"]
    for decision in SCOPE_DECISIONS:
        refs = "; ".join(chr(96) + CRYSTAL + ref + chr(96) for ref in decision["sources"])
        links = link_list([PR + decision["question"]], "QUESTION") + "; " + link_list([PR + decision["answer"]], "ANSWER")
        lines.append("| " + decision["name"] + " | " + cell(decision["left_out"]) + " | " + refs + " | " + links + " |")
    lines += ["",
              "Audyt selektorów mapy: \x60docs/agents/evidence/quests-3-20261102/map-bindings.json\x60; generator \x60batch3_map_bindings.py --check\x60. Liczba 60 dotyczy żądań callbacków, nie 60 różnych wartości UID/AID.",
              "",
              "Zatwierdzona korekta danych A-PALIMUTH-JOB: warunek job ma być \x60~=54\x60; pełny adapter NPC nadal pozostaje poza zakresem. Źródło: \x60" + CRYSTAL + "data-global/npc/palimuth.lua:65\x60; [QUESTION](" + PR + "6589698604), [ANSWER](" + PR + "6589900671). Pozostałe jawne korekty donora zachowują wskazane źródłowo klucze, wartości i cele.",
              "",
              "Left out: node-boot qualification owned by coordinator (merge-queue job node_boot, merge-group-gate.yml)",
              ""]
    return lines


def render_stage2(documents):
    totals = Counter()
    for document in documents:
        totals.update(document["content_counts"])
    lines = [E_HEADER, "", "<!-- Generated by tools/content-schema/quest-authoring/batch3_report.py --main-report. -->", "",
             "Stan etapu2 nie zmienia znaczenia historycznej inwentaryzacji A/B. Każdy jej wiersz ma osobny wynik w podlinkowanym dokumencie; source-only nie jest deklaracją grywalności.", "",
             f"Zakres danych: **{totals['world_callbacks_enabled']} callbacków w aktywnym źródle**, **{totals['world_callbacks_held']} pełnych callbacków held**, **{totals['npc_rules_source_only']} reguł NPC source-only** i **{totals['death_rules_source_only']} nieaktywnych reguł śmierci**. Liczniki pochodzą z plików i hashy zapisanych w sidecarach; te cztery kategorie nie są liczbą kroków i nie należy ich sumować.", "",
             "Nowy routing NPC, parser/adapter reguł śmierci C, rozszerzenia wspólnych właścicieli i callback-schema pozostają wycofane. Zachowane testowe kwalifikatory danych A/B nie mają produkcyjnego callera. Obowiązują decyzje: " + link_list(ANSWERS, "ANSWER") + ".", "",
             "| Quest | Kroki AUTO / RĘCZNIE | Zrobione w tym zakresie | Test / kwalifikacja | Left out |", "| --- | ---: | --- | --- | --- |"]
    for document in documents:
        quest = document["quest"]
        c = document["content_counts"]
        qualified = sum(row["delivery"] == "qualified_reward" for row in document["rows"])
        existing = sum(row["delivery"] == "existing_data" for row in document["rows"])
        unresolved = [row for row in document["rows"] if row["left_out"]]
        done = f"world {c['world_callbacks_enabled']}; NPC source-only {c['npc_rules_source_only']}; death source-only {c['death_rules_source_only']}; kwalifikacje nagród {qualified}; zachowane dane istniejące {existing}"
        tests = "Historyczne ograniczone PG; wyniki końcowego head w PR Validation" if historical_tests(document) else "Kontrola źródeł/projekcji; wyniki końcowego head w PR Validation"
        qlinks = unique(url for row in unresolved for url in row["question_urls"])
        left = f"{len(unresolved)} wierszy z ograniczeniem; held callbacks {c['world_callbacks_held']}; pełne funkcje i odnośniki w dokumencie"
        if qlinks:
            left += "; " + link_list(qlinks[:2], "QUESTION")
        lines.append(f"| [{quest}](quests-3-20261102/{quest}.md) | {document['import_counts'].get('AUTO', 0)} / {document['import_counts'].get('RĘCZNIE', 0)} | {done} | {tests} | {left} |")
    lines += scope_decision_lines()
    all_rows = [row for document in documents for row in document["rows"]]
    duplicate_names = sum(count - 1 for count in Counter((row["inventory"]["quest"], row["inventory"]["step"]) for row in all_rows).values() if count > 1)
    lines += ["", "### Kontrola końcowa raportu", "",
              f"- {len(documents)} questów i {len(all_rows)} odrębnych identyfikatorów kroków; zachowano {duplicate_names} dodatkowe wiersze o powtarzających się nazwach.",
              "- Każdy Left out ma nazwę brakującej operacji lub reguły, przypięte źródło oraz linki QUESTION/ANSWER w sidecarze.",
              "- Generator działa dla pojedynczego questa (`--quest`) i toleruje brak plików przyszłych questów. Tryb `--main-report` wymaga pełnych21/883 i nie kwalifikuje runtime.",
              "- " + QUALIFICATION_NOTE, ""]
    return "\n".join(lines)


def update_main(text, documents):
    if len(documents) != 21 or sum(document["row_count"] for document in documents) != 883:
        raise ValueError("main report requires21 quests and883 rows")
    text = text.split(E_HEADER, 1)[0].rstrip() + "\n"
    # Preserve A/B as the first-commit inventory, correcting only reviewed errata.
    lines = text.splitlines()
    for document in documents:
        for row in document["rows"]:
            if not (correction := row.get("inventory_correction")):
                continue
            original = correction["original"]
            inv = row["inventory"]
            matches = [index for index, line in enumerate(lines)
                       if (line.startswith(f"| {original['quest']} | {original['step']} |")
                           and all(ref in line for ref in original["donor"]))
                       or (line.startswith(f"| {inv['quest']} | {inv['step']} |")
                           and all(ref in line for ref in inv["donor"]))]
            if len(matches) != 1:
                raise ValueError(f"reviewed correction must select one inventory row: {original}")
            fields = [inv[name] for name in INVENTORY_FIELDS]
            fields[6] = "; ".join(fields[6])
            lines[matches[0]] = "| " + " | ".join(cell(value) for value in fields) + " |"
    text = "\n".join(lines) + "\n"
    for key in ("C-STAG-TARGET", "C-PRIMAL-BAG", "C-PRIMAL-OUTFIT"):
        text = text.replace(key + " — linkPR do uzupełnienia przez root", f"[{key}]({PR + QUESTION_IDS[key]}); [ANSWER]({ANSWERS[0]}) — zachowano Crystal")
    text = text.replace("R-MONK-VOCATION — link po utworzeniu PR", f"[R-MONK-VOCATION]({PR + QUESTION_IDS['R-MONK-VOCATION']}); [ANSWER]({ANSWERS[0]})")
    text = text.replace("Potrzebny snapshot API lub decyzja o ograniczeniu porównania; wspólne QUESTION R-API.",
                        f"[QUESTION R-API]({PR + QUESTION_IDS['R-API']}) rozstrzygnięto wariantemB: porównanie pozostaje NIEZNANE/API403, bez capture i bez elementów tylko Global; [ANSWER]({ANSWERS[0]}).")
    text = text.replace("4. Każde otwarte pytanie będzie komentarzem QUESTION na własnym DRAFT PR; odsyłacze w raporcie i Left out zostaną uzupełnione po jego otwarciu.",
                        "4. Pytania opublikowano na [DRAFT PR2843](https://github.com/Oteryn/Oteryn-Game/pull/2843). Obowiązujące odpowiedzi i szczegółowe Left out każdego kroku znajdują się w sekcjiE i dokumentach per quest.")
    text = text.replace("Odpowiednie źródła wA; zostaną osobno QUESTION, jeśli decyzji nie rozstrzyga repo.",
                        f"Źródła pozostają wA; [Manop]({PR + QUESTION_IDS['R-MONK-MANOP']}), [Triangle]({PR + QUESTION_IDS['R-TRIANGLE-LEVER']}) i [Annihilator]({PR + QUESTION_IDS['R-ANNIHILATOR-GROUP']}) mają opublikowane QUESTION i wiążące ANSWER opisane wE.")
    linked = []
    existing_lines = text.splitlines()
    for index, line in enumerate(existing_lines):
        linked.append(line)
        match = re.match(r"### QUESTION: ([A-Z0-9-]+) —", line)
        if match and match.group(1) in QUESTION_IDS:
            following = existing_lines[index + 2] if index + 2 < len(existing_lines) else ""
            if not following.startswith("[Komentarz PR]"):
                linked += ["", f"[Komentarz PR]({PR + QUESTION_IDS[match.group(1)]})."]
    text = "\n".join(linked) + "\n"
    # Verify the preserved inventory rather than accepting a silently reduced table.
    a = text.split("## A. Tabela kroków", 1)[1].split("## B.", 1)[0]
    b = text.split("## B. Podsumowanie per quest", 1)[1].split("## C.", 1)[0]
    count_a = sum(any(line.startswith("| " + quest + " |") for quest in QUESTS) for line in a.splitlines())
    count_b = sum(any(line.startswith("| " + quest + " |") for quest in QUESTS) for line in b.splitlines())
    if count_a != 883 or count_b != 21:
        raise ValueError("main inventory A/B count drift")
    return text.rstrip() + "\n\n" + render_stage2(documents)


def write_or_check(path, text, check):
    if check:
        if not path.is_file() or path.read_text(encoding="utf-8") != text:
            raise ValueError(f"stale generated batch-three evidence: {path}")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quest", choices=QUESTS, action="append", help="Render only this quest; repeatable.")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--main-report", action="store_true", help="Also update/check sectionE and reviewed errata; requires21/883.")
    parser.add_argument("--refresh-decisions", action="store_true", help="Reconcile the six accepted answers into existing status inputs without changing inventory identity.")
    parser.add_argument("--import-work-dir", type=Path, help="Import the four worker status snapshots and root/all-rows.json.")
    args = parser.parse_args()
    if args.refresh_decisions and args.check:
        parser.error("--refresh-decisions writes status inputs; run --check separately")
    selected = tuple(dict.fromkeys(args.quest or QUESTS))
    if args.import_work_dir:
        documents = import_statuses(ROOT, args.import_work_dir, selected)
        for quest, document in documents.items():
            refresh_decisions(document)
            write_or_check(ROOT / AUTHORING / (quest + ".json"), encode(document), args.check)
    else:
        documents = {quest: read(path) for quest in selected if (path := ROOT / AUTHORING / (quest + ".json")).is_file()}
    for quest, document in documents.items():
        if args.refresh_decisions:
            refresh_decisions(document)
            write_or_check(ROOT / AUTHORING / (quest + ".json"), encode(document), False)
        validate(ROOT, document)
        write_or_check(ROOT / EVIDENCE / (quest + ".md"), render_quest(document), args.check)
    if args.main_report:
        path = ROOT / MAIN
        ordered = [documents[quest] for quest in QUESTS if quest in documents]
        write_or_check(path, update_main(path.read_text(encoding="utf-8"), ordered), args.check)
    print(json.dumps({"quests": len(documents), "rows": sum(document["row_count"] for document in documents.values()),
                      "mode": "check" if args.check else "generate", "main_report": args.main_report}, sort_keys=True))


if __name__ == "__main__":
    main()
