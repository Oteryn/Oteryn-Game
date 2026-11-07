"""Strict literal source registration parser; data only, never Lua execution.

Unsupported/dynamic registration syntax is rejected rather than called empty.
The callback list retains the parent monster.events order and original indices.
"""
from __future__ import annotations
from dataclasses import dataclass
import hashlib
import re

@dataclass(frozen=True)
class Token:
    kind: str
    value: str
    line: int

def tokens(text: str) -> list[Token]:
    result=[]; index=0; line=1
    while index<len(text):
        char=text[index]
        if char.isspace():
            line+=char=='\n'; index+=1; continue
        if text.startswith('--',index):
            long=re.match(r'--\[(=*)\[',text[index:])
            if long:
                close=']'+long[1]+']'; end=text.find(close,index+len(long[0]))
                if end<0: raise ValueError('unterminated Lua long comment')
                end+=len(close)
            else:
                end=text.find('\n',index)
                if end<0:end=len(text)
            line+=text[index:end].count('\n'); index=end;continue
        if char in "\"'":
            quote=char; start=index; index+=1
            while index<len(text):
                if text[index]=='\\':index+=2;continue
                if text[index]==quote:break
                index+=1
            if index>=len(text):raise ValueError('unterminated Lua string')
            raw=text[start+1:index]
            # Decode only ordinary escaped quotes/backslashes. Registration
            # names needing other Lua string escapes require an explicit parser.
            value=re.sub(r'\\([\\\"\'])',r'\1',raw)
            result.append(Token('string',value,line));line+=text[start:index+1].count('\n');index+=1;continue
        long=re.match(r'\[(=*)\[',text[index:])
        if long:
            close=']'+long[1]+']'; end=text.find(close,index+len(long[0]))
            if end<0:raise ValueError('unterminated Lua long string')
            value=text[index+len(long[0]):end]
            result.append(Token('string',value,line));end+=len(close)
            line+=text[index:end].count('\n');index=end;continue
        ident=re.match(r'[A-Za-z_][A-Za-z0-9_]*',text[index:])
        if ident:
            value=ident[0];result.append(Token('identifier',value,line));index+=len(value);continue
        result.append(Token('punctuation',char,line));index+=1
    return result

def values(stream: list[Token],index:int,count:int)->list[str]:
    return [token.value for token in stream[index:index+count]]

def block_depths(stream:list[Token])->list[int]:
    depths=[];blocks=[]
    for token in stream:
        depths.append(len(blocks))
        if token.kind!='identifier':continue
        word=token.value
        if word in ('function','if','repeat'):
            blocks.append([word,False])
        elif word in ('for','while'):
            blocks.append([word,True])
        elif word=='do':
            if blocks and blocks[-1][0] in ('for','while') and blocks[-1][1]:
                blocks[-1][1]=False
            else:blocks.append(['do',False])
        elif word=='end':
            if not blocks or blocks[-1][0]=='repeat':raise ValueError('unsupported Lua block structure')
            blocks.pop()
        elif word=='until':
            if not blocks or blocks[-1][0]!='repeat':raise ValueError('unsupported Lua repeat block')
            blocks.pop()
    if blocks:raise ValueError('unterminated Lua block')
    return depths

def monster_registrations(text: str, require_registration:bool=False) -> list[dict]:
    stream=tokens(text); assignments=[];depths=block_depths(stream)
    registration_calls=[]
    for index,token in enumerate(stream):
        if token.kind=='identifier' and values(stream,index+1,5)==[':','register','(','monster',')']:
            if depths[index]!=0:raise ValueError('conditional/nested MonsterType registration')
            registration_calls.append(index)
    if len(registration_calls)>1:raise ValueError('multiple MonsterType register calls require explicit engine model')
    if require_registration and not registration_calls:raise ValueError('missing literal MonsterType register call')
    registration_at=registration_calls[0] if registration_calls else len(stream)
    for index,token in enumerate(stream):
        if values(stream,index,4)==['monster','.','events','=']:
            if depths[index]!=0:raise ValueError('conditional/nested monster events assignment')
            assignments.append(index)
    if any(values(stream,index,5)==['monster','[','events',']','='] for index in range(len(stream))):
        raise ValueError('unsupported bracket monster events assignment')
    if not assignments:
        if any(token.kind=='identifier' and token.value=='events' for token in stream):
            raise ValueError('nonliteral/unsupported monster events assignment')
        return []
    effective=[]
    for assignment in assignments:
        index=assignment+4
        if values(stream,index,1)!=['{']:raise ValueError('dynamic monster events assignment')
        index+=1;registrations=[];need_value=True
        while index<len(stream):
            token=stream[index]
            if token.value=='}':
                if assignment<registration_at:effective=registrations
                break
            if (not need_value or token.kind!='string'
                or not re.fullmatch(r"[A-Za-z0-9_ '\-]+",token.value)):
                raise ValueError('nonliteral monster events array')
            registrations.append({'registration_index':len(registrations),'name':token.value,'line':token.line})
            index+=1;need_value=False
            if index<len(stream) and stream[index].value in [',',';']:
                index+=1;need_value=True
        else:raise ValueError('unterminated monster events array')
    # Lua table assignment replaces the prior table. All assignments above
    # must be static, unconditional literals; dynamic/mixed input is refused.
    return effective

def registered_callbacks(text:str,name:str)->set[str]:
    stream=tokens(text); variables=[]
    for index,token in enumerate(stream):
        if (token.kind=='identifier' and values(stream,index+1,3)==['=','CreatureEvent','(']
            and index+5<len(stream) and stream[index+4].kind=='string'
            and stream[index+4].value==name and stream[index+5].value==')'):
            variables.append(token.value)
    if len(variables)!=1:raise ValueError('event source must declare exactly one literal named constructor: '+name)
    var=variables[0]
    constructors=sum(values(stream,index,4)==[var,'=','CreatureEvent','('] for index in range(len(stream)))
    if constructors!=1:raise ValueError('rebound event constructor variable requires explicit parser')
    if not any(values(stream,i,5)==[var,':','register','(',')'] for i in range(len(stream))):
        raise ValueError('event source lacks literal register: '+name)
    callbacks=set()
    for index in range(len(stream)):
        if values(stream,index,3)==['function',var,'.']:
            if index+4>=len(stream) or stream[index+4].value!='(':
                raise ValueError('unsupported callback declaration')
            callbacks.add(stream[index+3].value)
        if values(stream,index,2)==[var,'.'] and index+3<len(stream) and stream[index+3].value=='=':
            if stream[index+2].value.startswith('on'):
                if values(stream,index+4,2)!=['function','(']:raise ValueError('dynamic callback assignment')
                callbacks.add(stream[index+2].value)
    return callbacks

def ordered_death_events(monster:bytes,sources:dict[str,tuple[str,bytes]])->dict:
    text=monster.decode('utf-8'); registrations=monster_registrations(text)
    effective=[];seen=set()
    for row in registrations:
        name=row['name']
        if name in seen:continue # Creature::registerCreatureEvent rejects a duplicate pointer.
        seen.add(name)
        if name not in sources:raise ValueError('missing registered event source: '+name)
        path,body=sources[name]
        if 'onDeath' in registered_callbacks(body.decode('utf-8'),name):
            effective.append({**row,'source_path':path,'source_sha256':hashlib.sha256(body).hexdigest()})
    return {'monster_sha256':hashlib.sha256(monster).hexdigest(),
            'registered_events':registrations,'on_death':effective}
