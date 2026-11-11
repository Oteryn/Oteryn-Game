#!/usr/bin/env python3
"""Generate the bounded W11 Rotten Blood callback witness from canonical native source inputs.

No WorldProject is fabricated. The actual native loader's complete source digest
binds the bounded records to the full canonical project; exact typed equality at
runtime binds every consumed Creature/Behavior/Ability/Effect/Formula record.
Crystal callback bytes are read-only compatibility evidence, never executed.
Physical profiles retain the canonical Canary bindings and the existing accepted
Tibia Global boss HP overrides (official-balance.json; re-audit2026-10-28:195).
The accepted charge explosion is flat1250, not Crystal maxHealth*0.4.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
CANARY = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
KEYS = ('bakragore', 'bloated_man_maggot', 'chagorz', 'converter', 'darklight_construct', 'darklight_emitter', 'darklight_matter', 'darklight_source', 'darklight_striker', 'echo_of_chagorz', 'echo_of_ichgahal', 'echo_of_murcion', 'echo_of_vemiath', 'elder_bloodjaw', 'ichgahal', 'meandering_mushroom', 'murcion', 'mushroom', 'mycobiontic_beetle', 'oozing_carcass', 'pillar_of_dark_energy', 'rotten_man_maggot', 'sopping_carcass', 'sopping_corpus', 'vemiath', 'walking_pillar', 'wandering_pillar')
CALLBACKS = {'data-global/monster/quests/rotten_blood_quest/bakragore.lua': '974df9389e734ca947111cd41af9adec44f531adf0cc53233d5a7a94ab190178', 'data-global/monster/quests/rotten_blood_quest/bloated_man-maggot.lua': 'eabbd1e4861932f2deb515b5ddae29d7a5b5a7430dc600785f89df31fe05b608', 'data-global/monster/quests/rotten_blood_quest/chagorz.lua': '297fc5a9fc5e58ade139dd3c3e60e10fdd2f0d6745c05dc9137c0f44d6a60732', 'data-global/monster/quests/rotten_blood_quest/converter.lua': 'b623d1095491f1d2343e00188e9aeb4c9322b3b74f56dcf3edbc64928a85f770', 'data-global/monster/quests/rotten_blood_quest/darklight_construct.lua': 'b618a40b1cdb8b8a6ecc9da4a3eb62e5ddb8ed21225ae863c18a3fcc8d4643c6', 'data-global/monster/quests/rotten_blood_quest/darklight_emitter.lua': 'f38064d6c9e783d6b81732c4f8d083ad36e9aa84b432e7400ac9f8f7bd52b982', 'data-global/monster/quests/rotten_blood_quest/darklight_matter.lua': '839417a9740f60435ae06bd5b860be0abc7185298b5cf1288f2781bf1feaa80d', 'data-global/monster/quests/rotten_blood_quest/darklight_source.lua': '240390c67c4a9b7d9027e95069befbe1aed34dccc0c2a8cdd6413041fd810de6', 'data-global/monster/quests/rotten_blood_quest/darklight_striker.lua': 'e24f098f15111d0aff2df33b6217767f177f4d3daa63e12fae6d1d8235b4ead9', 'data-global/monster/quests/rotten_blood_quest/echo_of_chagorz.lua': 'afe650288c1f975736796cedb73a0caf4f643fe1c3f10d7f69daf7f70794ee6b', 'data-global/monster/quests/rotten_blood_quest/echo_of_ichgahal.lua': 'cd2f9984f24c81f870a2e564b15c98a9bc2e960c010a6355035c184a301b365d', 'data-global/monster/quests/rotten_blood_quest/echo_of_murcion.lua': 'd5fab0b33974b5be0cb3bea1ea239c157f875812f8eb8c23162f6524167de609', 'data-global/monster/quests/rotten_blood_quest/echo_of_vemiath.lua': '97b2c4485310911f65c8af5ba87af68a78a36c04d29ff70a37eed04367a81bca', 'data-global/monster/quests/rotten_blood_quest/elder_bloodjaw.lua': '483c813d4e44181a72325e2468307b3d8cd36611276703cf1f0e8668e3eed1c6', 'data-global/monster/quests/rotten_blood_quest/ichgahal.lua': '237c4b4a8743fc2f2868f0a0ce183fa9c1a0d66cc2037345419fe70cd3eb852d', 'data-global/monster/quests/rotten_blood_quest/meandering_mushroom.lua': '7e4d1a4012565575e60dec6e9af9e0eda2c71a3ed63c69df5b88225abb32390b', 'data-global/monster/quests/rotten_blood_quest/murcion.lua': 'cdd4e39e5119d6c1952f3850ba4bfd19877f608716f8b166fe39a6f25ca1801a', 'data-global/monster/quests/rotten_blood_quest/mushroom.lua': '450850113ccaef42caf0e86d517b25769fd1e81271e6924901e459042843e9a4', 'data-global/monster/quests/rotten_blood_quest/mycobiontic_beetle.lua': '585bf2c2f52828cbdb44bfd094ab3e469a6ae8e0ded3ed930ab95b4c218f2341', 'data-global/monster/quests/rotten_blood_quest/oozing_carcass.lua': '94485724d5f67dfd17cd9de7317725d33ccf9bb058b23c78c79544d3b2be6c70', 'data-global/monster/quests/rotten_blood_quest/pillar of dark energy.lua': '7337c13a36b75afd65eae1fe0075a950f7b940413a24b74e0e7d5a04a857c041', 'data-global/monster/quests/rotten_blood_quest/rotten_man-maggot.lua': 'eff91724e78c31f624303ab1a75e6c7ba504769a30d39ff1cfa607c584be969d', 'data-global/monster/quests/rotten_blood_quest/sopping_carcass.lua': '35ed87d0dbe1044d434680d2842db1923bb7cc0354f7640f59c3f195cf073185', 'data-global/monster/quests/rotten_blood_quest/sopping_corpus.lua': '442c75a802efab55d831ddf7f329d9757aecaef46b520c8c4656b26936aa969f', 'data-global/monster/quests/rotten_blood_quest/vemiath.lua': 'd0b477325e4d3e058490023870075dd6e1b159cd614f82610d122510f7c7a8e7', 'data-global/monster/quests/rotten_blood_quest/walking_pillar.lua': 'f3dfa1b24bb0079823107da3da2de5a173405348d7947e98b307563626dcc6cd', 'data-global/monster/quests/rotten_blood_quest/wandering_pillar.lua': '41ad052dd11f855420e8e8180d68fea54419f102c852cc916dc7b445431fa96c', 'data-global/scripts/quests/rotten_blood_quest/monster_onthink_area_icons.lua': 'c1ca5e85ae2f0ea102909502f65d3586ad44e9adf97bf3601f90331595142aad', 'data-global/scripts/quests/rotten_blood_quest/rotten_blood_boss_mechanics.lua': '1743fdb5bc1ea75932af6dcfe8ebd13e0f6e4529d8af09dc34e47c289183773d', 'data-global/scripts/quests/rotten_blood_quest/teleport_step_event.lua': '2da0ced1fd77b54003addc3a3513c9fdac7556cd3e32dda95a9554615b7c392b'}
REVIEWED_PROFILES = {'oteryn:creature.echo_of_chagorz': 'db8b72f09a49c357fbfdf24d9d8f52947cb18e508489874bcab88f4455dcf513', 'oteryn:creature.echo_of_ichgahal': '3d1a9712a4650f711f925df8e55b2cb460efdb23cb6ca1697c600d640a9f7401', 'oteryn:creature.echo_of_murcion': '8f5ddff79a90398e1a9f451dd835b9e07238a36e1adacc139ec7ad134d686bd4', 'oteryn:creature.echo_of_vemiath': '521031b2b36a84fd42250bdd98ba968251b07bf60f032262ffe41347be89d39f', 'oteryn:creature.mushroom': '50e7ef475c5eae0f72210a0649ea17f87d4e7a2aa3dc162355acf89209e470fc', 'oteryn:creature.sopping_carcass': 'ef01be192cd6c037c5373292bbecb30545151809442301326fc472a146cb4a85', 'oteryn:creature.bakragore': '68cd092f90ff6749e50cc609d4d92d7f9eaea8002d65673717bf7d9082d0059e', 'oteryn:creature.bloated_man_maggot': '5c121654babb73f5e299c860a40381cf7ccdf59151ba63cf8ab7aa6f9e662d2e', 'oteryn:creature.chagorz': '57809986af30f4f2964d84ca88783374f0bad6fd891f7aee47d53c89ea847c95', 'oteryn:creature.converter': 'f9309732673b75be529e64e50dfb74844748137cef6bb12d4eaa646c61f6aec0', 'oteryn:creature.darklight_construct': 'f3869762da26af7e1ae0a956d642dc1ec5bbaacb56cce9cd67344132ed410de1', 'oteryn:creature.darklight_emitter': '5911165996bd9ce19fe75542b465ea9a636877bfa968e6856939373a12b9291f', 'oteryn:creature.darklight_matter': 'b651ad3b888b449c3d78c13bbccbba7773aad8cefbc7ecd79e36476bdb8d3a2a', 'oteryn:creature.darklight_source': '1f8ad689ef2fb92f77516467aa239fb51b7c6e0c7c8b7bf2d25e56ab1ef6f0aa', 'oteryn:creature.darklight_striker': 'cb0a3f440018cf186103e05dc1902e373b8e8898e445a287840c9c0851e10eec', 'oteryn:creature.elder_bloodjaw': 'c97eb782df03ee71b67215988804ab7a87f987d79f3cf9d902430abc103aecb1', 'oteryn:creature.ichgahal': '039baaa90fc9f6908c1afd59c2e9e774c697c90fdd1e1a65fb8971593391747a', 'oteryn:creature.meandering_mushroom': '65c1a32f7b44444e3287525d8f43028c099c4f48a06d81e7226e73c3195a9ee3', 'oteryn:creature.murcion': '2487654c9975756f4f82ed495e6275f4cb8ad3d296a4904e453edcce8df500b2', 'oteryn:creature.mycobiontic_beetle': 'b8e5ee41db87d496fd96fefd59447ec1b709f24a7bf14ecd0ee3829dd5aec28d', 'oteryn:creature.oozing_carcass': '2e93cd1dfc68dbc2bac1b6c0a290254867c3c4c842e66f1e04f55eef5dc565ba', 'oteryn:creature.pillar_of_dark_energy': 'c194d5528d5404933cecf18096cad9b77a16e47b52e481c1612eca8f9f7db589', 'oteryn:creature.rotten_man_maggot': '0e0bfbe1e0343016c30d65da7c99f17c71b9f20837c7630163312dd192167060', 'oteryn:creature.sopping_corpus': '240d6805189dee2c5239150533d7f31f28c1d507eb937bbe4e44c4d2acbcc69a', 'oteryn:creature.vemiath': '56714617bd30f2dd820647642946ad713a1c33878c26307a4bf49c303c769e31', 'oteryn:creature.walking_pillar': '269c691f830e1759e2aeecba7975155c79b3c74cf98830f66ac9d2b816fb8f4f', 'oteryn:creature.wandering_pillar': '2140db453838349152246dd15cb31412aa189ba18d9bee8b82268c91ccadf014'}
CALLBACKS['data/libs/systems/monster_ai.lua'] = 'fa25b4ac8dacd53cefee06382801e13ce592a8fac91356dd6200a60edec7aee3'
OUTPUT = ROOT / "tools/content-schema/native-gameplay/srx3-rotten-blood-witness.json"

# Actual Item-use, movement and physical-corpse callback source closure.
CALLBACKS.update({'data-global/monster/quests/rotten_blood_quest/rottenEssences.lua': '2d8f31fca1793fed900e5baf121f04d126acdb64b174649ec22ea6c842b9ca02', 'data-global/scripts/quests/rotten_blood_quest/movements_fields.lua': '3ed5050ea68ba182e3bfed783ddce5198d154bf48537500ff96ac19de7c0b9e3', 'data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua': '21ba33eb58d1b5a4a88ca0c950d49a8369b34f3a11fe39a7e549a8b3ab154b46'})


def build():
    source = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_bytes())
    digest = source["source_definitions_sha256"]
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("native full source digest missing")
    records = []
    bindings = []
    for slug in KEYS:
        key = "oteryn:creature." + slug
        matches = [row for row in source["records"] if row["profile"]["target"]["key"] == key]
        if len(matches) != 1:
            raise ValueError("missing or duplicated native participant: " + key)
        record = matches[0]
        actual = hashlib.sha256(json.dumps(record, sort_keys=True, separators=(",", ":"),
                                           ensure_ascii=False).encode()).hexdigest()
        if actual != REVIEWED_PROFILES[key]:
            raise ValueError("native Rotten Blood body changed; source qualification review required: " + key)
        if record["profile"]["target"]["revision"] != "definition-r1" or not record.get("behavior"):
            raise ValueError("changed native source participant: " + key)
        canonical = [row for path in (ROOT / "content/creatures/definitions").glob("creatures-*.json")
                     for row in json.loads(path.read_bytes())["records"]
                     if row["definition"]["identity"]["key"] == key]
        if len(canonical) != 1:
            raise ValueError("canonical participant absent or duplicated: " + key)
        exact = [binding for binding in canonical[0]["source_bindings"]
                 if ((slug != "pillar_of_dark_energy"
                      and binding["source_key"] == "oteryn:source.canary"
                      and binding["source_revision"] == CANARY
                      and binding["identity_namespace"] == "canary/monster-file")
                     or (slug == "pillar_of_dark_energy"
                         and binding["source_key"] == "oteryn:source.crystalserver"
                         and binding["source_revision"] == "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f"
                         and binding["identity_namespace"] == "crystalserver/monster-file"
                         and binding["external_id"] == "data-global/monster/quests/rotten_blood_quest/pillar of dark energy.lua"))
                 and binding["disposition"] == "EXACT"
                 and binding["target"] == record["profile"]["target"]]
        if len(exact) != 1:
            raise ValueError("source binding changed: " + key)
        wanted = exact[0]
        bindings.append(wanted)
        records.append(record)
    return {"schema": "OTERYN_SRX3_ROTTEN_BLOOD_CALLBACK_WITNESS/v1",
            "mechanics_source_revision": PIN, "source_definitions_sha256": digest,
            "source_bindings": bindings, "callbacks": CALLBACKS, "records": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-root", type=Path)
    args = parser.parse_args()
    if args.source_root is not None:
        for path, expected in CALLBACKS.items():
            if hashlib.sha256((args.source_root / path).read_bytes()).hexdigest() != expected:
                raise ValueError("pinned callback changed: " + path)
    raw = (json.dumps(build(), sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("stale W11 Rotten Blood source witness; run this generator")
    else:
        OUTPUT.write_bytes(raw)


if __name__ == "__main__":
    main()
