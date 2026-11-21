"""Extract factual Tools metadata; never infer absent UI/server values."""
import argparse
import hashlib
import json
from pathlib import Path


def varint(data, offset):
    value = shift = 0
    while True:
        byte = data[offset]
        offset += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, offset
        shift += 7
        if shift > 70:
            raise ValueError('oversized varint')


def fields(data):
    offset = 0
    while offset < len(data):
        key, offset = varint(data, offset)
        number, wire = key >> 3, key & 7
        if wire == 0:
            value, offset = varint(data, offset)
        elif wire in (1, 2, 5):
            if wire == 2:
                size, offset = varint(data, offset)
            else:
                size = 8 if wire == 1 else 4
            value = data[offset:offset + size]
            offset += size
            if offset > len(data):
                raise ValueError('truncated protobuf')
        else:
            raise ValueError('unsupported wire')
        yield number, wire, value


def extract(raw, census):
    digest = hashlib.sha256(raw).hexdigest()
    if census['source_sha256'] != digest:
        raise ValueError('census belongs to another appearance source')
    frames = {r['appearance_object_id']: r for r in census['objects']}
    rows = []
    for number, wire, record in fields(raw):
        if number != 1:
            continue
        obj = list(fields(record))
        get = lambda n: next((v for k, w, v in obj if k == n), None)
        flags = list(fields(get(3) or b''))
        market = next((dict((n, v) for n, w, v in fields(v)) for n, w, v in flags if n == 36), {})
        if market.get(1) != 14:
            continue
        appearance_id = get(1)
        record_hash = hashlib.sha256(record).hexdigest()
        frame = frames[appearance_id]
        if frame['record_sha256'] != record_hash:
            raise ValueError('frame/census record substitution')
        vendors = []
        npc_names = {1:'name', 2:'location', 3:'sale_price', 4:'buy_price', 5:'currency_object_type_id', 6:'currency_quest_flag_display_name'}
        for n, w, v in flags:
            if n == 40:
                vendors.append({npc_names[k]: x.decode('utf-8') if k in (1,2,6) else x for k, ww, x in fields(v)})
        slot = next((dict((k,x) for k,ww,x in fields(v)).get(1) for n,w,v in flags if n == 34), None)
        description = get(5)
        rows.append({'appearance_object_id':appearance_id, 'record_sha256':record_hash,
            'name':get(4).decode('utf-8') if get(4) else None,
            'description':description.decode('utf-8') if description else None,
            'market':{'category':market[1], 'trade_as_object_id':market.get(2), 'show_as_object_id':market.get(3)},
            'npc_rows':vendors, 'clothes_slot':slot,
            'present_flag_numbers':[n for n,w,v in flags],
            'unmapped_flag_numbers':[n for n,w,v in flags if n not in range(1,65)],
            'sprite_ids':sorted({s for f in frame['frame_groups'] for s in f['sprite_ids'] if s}),
            'frame_record_source':'Existing Atlas structural decoder; icon reference, not visual parity',
            'weight':{'status':'UNKNOWN_NOT_IN_APPEARANCE_SCHEMA','value':None},
            'description_presence':'PRESENT' if description else 'ABSENT_IN_THIS_RECORD_NOT_UI_PROOF',
            'body_position_presence':'SOURCE_CLOTHES_SLOT_PRESENT' if slot is not None else 'ABSENT_SOURCE_FLAG_NOT_UI_PROOF',
            'canonical_item_binding':'UNQUALIFIED_APPEARANCE_ID_NOT_SERVER_ID'})
    return {'schema':'OTERYN_TOOLS_LOCAL_FIRST/v1', 'appearance_sha256':digest, 'records':rows}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--appearances', required=True)
    parser.add_argument('--census', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    result = extract(Path(args.appearances).read_bytes(), json.loads(Path(args.census).read_text(encoding='utf-8')))
    Path(args.output).write_text(json.dumps(result, indent=2, ensure_ascii=False)+'\n', encoding='utf-8')
    print(json.dumps({'tools':len(result['records']), 'npc_tools':sum(bool(r['npc_rows']) for r in result['records']),
        'npc_rows':sum(len(r['npc_rows']) for r in result['records']), 'descriptions':sum(bool(r['description']) for r in result['records'])}))
