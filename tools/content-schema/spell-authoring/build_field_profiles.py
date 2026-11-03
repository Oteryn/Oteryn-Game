#!/usr/bin/env python3
"""Pinned native field condition recipes; XML order and C++ float ramp preserved."""
import argparse
import hashlib
import os
import json
from pathlib import Path
import struct
import subprocess
import xml.etree.ElementTree as ET

PINS={"canary":"99902524e052f37574194466c2949c576e4ab269","crystal":"ff7ede593c69d4c658b382c97443e8155926924a"}
PARSER="src/items/functions/item/item_parse.cpp"
CONDITION="src/creatures/combat/condition.cpp"
XML="data/items/items.xml"
CREATURE="src/creatures/creature.hpp"

# Immutable input digests also admit extracted source snapshots without a Git index.
INPUT_DIGESTS = {'canary': {'data/items/items.xml': 'bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85', 'src/items/functions/item/item_parse.cpp': '1c92c9c0716d83d4d79d81ffeab099ef526eea2473e984b85436b291ce90d07c', 'src/creatures/combat/condition.cpp': '3601f95a59a637d9b9722bb99eb4858de4ae119ba263dde658ccf0e277c3993f', 'src/creatures/creature.hpp': 'fb1ceaea332ac14cb649d1c24b432b7e079b344704aab4ce6a26389e370c9841'}, 'crystal': {'data/items/items.xml': 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb', 'src/items/functions/item/item_parse.cpp': '08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7', 'src/creatures/combat/condition.cpp': 'f5816ed3f546d9551facfcf655cf727f20de6ced808027d29da8b287514bf2e4', 'src/creatures/creature.hpp': 'df14779052a582d1fc4202ffb788d23310bd9d71ef50452a8bb9175c2b9c1fb6'}}

def f32(value):
    return struct.unpack("f",struct.pack("f",value))[0]

def ramp(amount,start):
    # ConditionDamage::generateDamageList: integer med; each sum/i arithmetic
    # division is float before promotion into the double fabs expression.
    total=0
    out=[]
    for current in range(start,0,-1):
        med=(start+1-current)*amount//start
        if not med:
            # The donor's do/while emits once before comparing infinities.
            total+=current
            out.append(current)
            continue
        while True:
            total+=current
            out.append(current)
            x1=abs(1.0-f32(f32(f32(total)+f32(current))/f32(med)))
            x2=abs(1.0-f32(f32(total)/f32(med)))
            if x1>=x2:
                break
            if len(out)>4096:
                raise ValueError("field ramp candidate bound")
    return out

def recipe(field):
    element=field.get("value","").lower()
    if element not in {"fire","energy","poison","drown","physical","agony"}:
        return {"kind":"unsupported","source_element":element}
    ticks=0
    count=1
    start=0
    entries=[]
    for node in field:
        key=node.get("key","").lower()
        value=node.get("value")
        if value is None:
            continue
        if key=="ticks":
            ticks=int(value)
        elif key=="count":
            count=max(1,int(value))
        elif key=="start":
            start=max(0,int(value))
        elif key=="damage":
            amount=int(value)
            if amount<0 or ticks<0:
                raise ValueError("healing/negative field outside damage candidate")
            values=ramp(amount,start) if start else [amount]*count
            for damage in values:
                # Preserve count compaction without altering application order.
                if entries and entries[-1]["amount"]==damage and entries[-1]["interval_ms"]==max(1000,ticks):
                    entries[-1]["repetitions"]+=1
                else:
                    entries.append({"amount":damage,"interval_ms":max(1000,ticks),"repetitions":1})
            start=0
    return {"kind":"damage","element":element,"entries":entries}

def git(repo,path,pin):
    return subprocess.check_output(["git","-C",str(repo),"show",f"{pin}:{path}"])

def source_root():
    return Path(os.environ.get("OTERYN_SPELL_SOURCE_ROOT", Path(__file__).resolve().parents[3] / "_sources"))

def build(root=None):
    root = Path(root) if root is not None else source_root()
    profiles=[]
    for server,pin in PINS.items():
        repo=root/server
        paths = (XML, PARSER, CONDITION, CREATURE)
        if all((repo / path).is_file() for path in paths):
            blobs = {path: (repo / path).read_bytes() for path in paths}
        else:
            actual=subprocess.check_output(["git","-C",str(repo),"rev-parse",f"{pin}^{{commit}}"],text=True).strip()
            if actual!=pin:
                raise ValueError(f"{server} is not pinned")
            blobs={path:git(repo,path,pin) for path in paths}
        if any(hashlib.sha256(raw).hexdigest() != INPUT_DIGESTS[server][path] for path, raw in blobs.items()):
            raise ValueError(f"{server} field source digest mismatch")
        records={}
        for item in ET.fromstring(blobs[XML]):
            fields=[node for node in item if node.get("key","").lower()=="field"]
            magic=any(node.get("key","").lower()=="type" and node.get("value")=="magicfield" for node in item)
            if not fields and not magic:
                continue
            condition=recipe(fields[-1]) if fields else {"kind":"none"}
            first=int(item.get("id",item.get("fromid","0")))
            last=int(item.get("id",item.get("toid",str(first))))
            for item_id in range(first,last+1):
                if item_id<=0:
                    raise ValueError("invalid item id")
                if fields or item_id not in records:
                    records[item_id]=condition
        for item_id,condition in sorted(records.items()):
            profiles.append({"source":{"server":server,"revision":pin,"items_xml_sha256":hashlib.sha256(blobs[XML]).hexdigest(),"parser_sha256":hashlib.sha256(blobs[PARSER]).hexdigest(),"condition_sha256":hashlib.sha256(blobs[CONDITION]).hexdigest(),"creature_sha256":hashlib.sha256(blobs[CREATURE]).hexdigest(),"server_item_id":item_id},"condition":condition})
    return {"schema":"OTERYN_NATIVE_FIELD_PROFILES/v1","profiles":profiles}

if __name__=="__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sources", type=Path, default=source_root())
    parser.add_argument("--out", type=Path, default=Path(__file__).parent / "samples/native-field-profiles.json")
    args = parser.parse_args()
    target = args.out
    target.write_text(json.dumps(build(args.sources),indent=2)+"\n")
    print(target)
