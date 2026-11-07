"""Read-only bounded metadata from actual protocol envelopes; never encodes frames."""
def varint(data, cursor):
    value=0
    for shift in range(0,70,7):
        if cursor>=len(data):raise ValueError('truncated actual varint')
        b=data[cursor];cursor+=1;value|=(b&127)<<shift
        if b<128:return value,cursor
    raise ValueError('oversized actual varint')
def fields(data):
    result={};cursor=0
    while cursor<len(data):
        key,cursor=varint(data,cursor);field=key>>3;wire=key&7
        if wire==0:value,cursor=varint(data,cursor)
        elif wire==2:
            size,cursor=varint(data,cursor);value=data[cursor:cursor+size]
            if len(value)!=size:raise ValueError('actual field length')
            cursor+=size
        else:raise ValueError('unexpected actual envelope wire type')
        result.setdefault(field,[]).append(value)
    return result
def envelopes(raw):
    cursor=0
    while cursor<len(raw):
        if cursor+4>len(raw):raise ValueError('actual prefix')
        size=int.from_bytes(raw[cursor:cursor+4],'big');cursor+=4
        value=raw[cursor:cursor+size]
        if not size or len(value)!=size:raise ValueError('actual body')
        cursor+=size;yield fields(value)
def metadata(raw):
    last=0;palette=None
    for envelope in envelopes(raw):
        last=max(last,envelope.get(3,[0])[0])
        if envelope.get(1)==[11]:
            last=max(last,fields(envelope[4][0]).get(4,[0])[0])
        if envelope.get(1)!=[12]:continue
        chunk=fields(envelope[4][0]);body=fields(chunk[3][0])
        for entry in body.get(1,[]):
            entry=fields(entry)
            if entry.get(1)!=[21]:continue
            appearance=fields(entry[4][0]);catalogue=fields(appearance[1][0]);rgb=catalogue[5][0]
            if not rgb or len(rgb)>768 or len(rgb)%3:raise ValueError('actual bounded appearance palette')
            palette=[list(rgb[i:i+3]) for i in range(0,len(rgb),3)]
    return last,palette
