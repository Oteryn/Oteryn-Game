#!/usr/bin/env python3
"""Pinned native field condition recipes; XML order and C++ float ramp preserved."""
import hashlib
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
            raise ValueError("source ramp division by zero")
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
    if element not in {"fire","energy","poison","drown","physical"}:
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

def git(repo,path):
    return subprocess.check_output(["git","-C",str(repo),"show",f"HEAD:{path}"])

def build(root=Path("/workspace/spell-sources")):
    profiles=[]
    for server,pin in PINS.items():
        repo=root/server
        actual=subprocess.check_output(["git","-C",str(repo),"rev-parse","HEAD"],text=True).strip()
        if actual!=pin:
            raise ValueError(f"{server} is not pinned")
        blobs={path:git(repo,path) for path in (XML,PARSER,CONDITION,CREATURE)}
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
    target=Path(__file__).parent/"samples/native-field-profiles.json"
    target.write_text(json.dumps(build(),indent=2)+"\n")
    print(target)
