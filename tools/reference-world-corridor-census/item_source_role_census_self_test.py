#!/usr/bin/env python3
from __future__ import annotations
import importlib.util
from pathlib import Path

MODULE=Path(__file__).with_name("item_source_role_census.py")
spec=importlib.util.spec_from_file_location("item_source_role_census",MODULE)
assert spec and spec.loader
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)

XML=b'''<?xml version="1.0" encoding="ISO-8859-1"?>
<items>
<item id="1" name="sword"><attribute key="primarytype" value="sword weapons"/><attribute key="attack" value="10"/></item>
<item fromid="2" toid="4" name="stone wall"><attribute key="primarytype" value="walls"/></item>
<item fromid="5" toid="8" name="RESERVED SPRITE"/>
<item fromid="10" toid="9" name="broken"/>
<item id="11" name="corpse"><attribute key="duration" value="10"/><attribute key="decayto" value="12"/></item>
<item id="12" name="used thing"><attribute key="transformonuse" value="13"/></item>
<item fromid="13" toid="15" name="visual variants"/>
<item id="16" name="plain"/>
<item id="17" name="simple"><attribute key="weight" value="100"/></item>
<item id="18" name="readable"><attribute key="writeable" value="1"/></item>
</items>'''
v=c.build_census(XML)
assert v["counts"]["source_xml_item_nodes"]==10
assert v["counts"]["reversed_ranges_excluded"]==1
assert v["counts"]["admitted_definition_nodes"]==9
assert v["counts"]["expanded_source_ids"]==16
assert v["role_buckets"]["PLAYER_CATALOG_STRONG"]["definition_nodes"]==1
assert v["role_buckets"]["WORLD_OBJECT_INTERACTION"]["expanded_source_ids"]==3
assert v["role_buckets"]["TECHNICAL_PLACEHOLDER"]["expanded_source_ids"]==4
assert v["role_buckets"]["STATE_VARIANT_DECAY"]["definition_nodes"]==1
assert v["role_buckets"]["STATE_VARIANT_TRANSFORM"]["definition_nodes"]==1
assert v["role_buckets"]["RANGE_VARIANT_NO_FIELDS"]["expanded_source_ids"]==3
assert v["role_buckets"]["DIRECT_SIMPLE_NO_FIELDS"]["definition_nodes"]==1
assert v["role_buckets"]["SIMPLE_PRESENTATION_CANDIDATE"]["definition_nodes"]==1
assert v["role_buckets"]["RESIDUAL_OTHER"]["definition_nodes"]==1
print("item-source-role-census self-test: PASS")
