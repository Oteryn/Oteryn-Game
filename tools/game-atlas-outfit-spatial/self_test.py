#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile

HERE=Path(__file__).resolve().parent
SPEC=importlib.util.spec_from_file_location('outfit_spatial',HERE/'export.py')
assert SPEC and SPEC.loader
subject=importlib.util.module_from_spec(SPEC);sys.modules[SPEC.name]=subject;SPEC.loader.exec_module(subject)


def v(value:int)->bytes:
    out=bytearray()
    while True:
        byte=value&0x7f;value>>=7;out.append(byte|(0x80 if value else 0))
        if not value:return bytes(out)

def vi(field:int,value:int)->bytes:return v(field<<3)+v(value)
def vb(field:int,payload:bytes)->bytes:return v((field<<3)|2)+v(len(payload))+payload

def fixture()->bytes:
    shift=vi(1,8)+vi(2,4)
    flags=vb(26,shift)+vi(29,1)+vi(51,1)
    outfit=vi(1,128)+vb(3,flags)
    plain=vi(1,129)
    return vb(2,outfit)+vb(2,plain)


def main()->int:
    product=subject.build_from_bytes(fixture(),{'zip_sha256':'fixture'})
    assert product['manifest']['statistics']['outfits']==2
    assert product['manifest']['statistics']['shift_flag_records']==1
    assert product['manifest']['statistics']['nonzero_displacement_records']==1
    assert product['manifest']['statistics']['animate_always_records']==1
    assert product['manifest']['statistics']['reverse_addons_true']['south']==1
    first=product['records'][0]
    assert first['displacement']=={'x':8,'y':4}
    assert first['reverse_addons']['south'] is True
    assert first['anchor_policy']==subject.ANCHOR_POLICY
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp);subject.write_product(product,root)
        manifest,index=subject.load_index(root)
        assert manifest['product_root']==product['manifest']['product_root']
        assert index[128]['spatial_record_id']==first['spatial_record_id']
        path=root/'outfits.jsonl';path.write_text(path.read_text()+' ')
        try:subject.load_index(root)
        except (subject.SpatialError,ValueError):pass
        else:raise AssertionError('corrupt spatial product accepted')
    print('game-atlas outfit spatial self-test: PASS')
    return 0

if __name__=='__main__':raise SystemExit(main())
