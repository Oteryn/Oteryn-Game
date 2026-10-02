"""Bounded external research parser: exact top-level own Infobox Item only."""
import re

def parts(text, separator=None):
    stack=[];links=0;start=0;i=0;out=[];spans=[]
    while i<len(text):
        if text.startswith('<!--',i):
            end=text.find('-->',i+4)
            if end<0:raise ValueError('UNTERMINATED_COMMENT')
            i=end+3;continue
        if text.startswith('{{{',i):stack.append(('}}}',i));i+=3;continue
        if text.startswith('{{',i):stack.append(('}}',i));i+=2;continue
        if stack and text.startswith(stack[-1][0],i):
            close,opened=stack.pop();i+=len(close)
            if not stack:spans.append((opened,i))
            continue
        if text.startswith('[[',i):links+=1;i+=2;continue
        if text.startswith(']]',i):
            if links:links-=1
            i+=2;continue
        if separator and text[i]==separator and not stack and not links:
            out.append(text[start:i]);start=i+1
        i+=1
    if stack or links:raise ValueError('UNBALANCED_TEMPLATE_OR_LINK')
    if separator:out.append(text[start:]);return out
    return spans

def own_fields(text):
    boxes=[]
    for start,end in parts(text):
        raw=text[start:end]
        if not raw.startswith('{{') or raw.startswith('{{{'):continue
        parameters=parts(raw[2:-2],'|')
        header=re.sub(r'[_\s]+',' ',parameters[0].strip()).casefold()
        if header!='infobox item':continue
        fields={}
        for parameter in parameters[1:]:
            if '=' not in parameter:continue
            key,value=parameter.split('=',1)
            key=key.strip().casefold()
            if not re.fullmatch(r'[a-z0-9_]+',key):continue
            fields.setdefault(key,[]).append(value.strip())
        boxes.append({'offset':start,'raw_sha_input':raw,'fields':fields})
    return boxes

def tests():
    def f(s):return own_fields(s)[0]['fields']
    assert f('{{Infobox_Item|empty=\n|classificacao=2\n|max_tier=2}}')['classificacao']==['2']
    assert f('{{Infobox_Item|notes={{Nested\n|max_tier=10}}\n|max_tier=2}}')['max_tier']==['2']
    assert own_fields('<!--{{Infobox_Item|max_tier=10}}-->')==[]
    assert len(f('{{Infobox_Item|max_tier=2|max_tier=2}}')['max_tier'])==2
    assert f('{{Infobox_Item|notes=[[File:Example|x=y]]|max_tier=2}}')['max_tier']==['2']
    assert len(own_fields('{{Infobox_Item|max_tier=2}}{{Infobox Item|max_tier=3}}'))==2
    assert f('{{Infobox_Item|max_tier=2<!--not scalar-->}}')['max_tier']==['2<!--not scalar-->']
    try:own_fields('{{Infobox_Item|max_tier=2')
    except ValueError:pass
    else:raise AssertionError('unterminated own box accepted')
    return 8

if __name__=='__main__':print('meaningful_parser_tests_PASS',tests())
