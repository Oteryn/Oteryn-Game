"""Closed SOURCE-only UID/item-type record rewards; never Native admission.

This supplement leaves the digest-bound historical compiler untouched. Existing
outer guards and UNKNOWN fallback remain byte/canonically identical.
"""
import copy
import re
import lua_tables
from lua_writers import static_name_is_immutable, builtin_binding_is_pristine, unconditional_prefix
from ots_interactions import LiteralParser, split_args, scalar_leaf_paths, CALLBACK, pristine_constructor

ENV = {'_G','_ENV','rawset','setmetatable','getfenv','setfenv','load','loadstring','loadfile','dofile','require','debug'}


def duplicate_free(table):
    if not isinstance(table,dict) or 'fields' not in table:return True
    keys=[]
    for f in table['fields']:
        if f['key'] in keys or not duplicate_free(f['value']):return False
        keys.append(f['key'])
    return True


def record_reward(script, number):
    raw=script.raw(number);call=re.fullmatch(r'(\w+):addItem\((.*)\)',raw)
    if not call or not script.proven_player(call[1],number):return None
    args=split_args(call[2])
    if len(args)!=2:return None
    field=re.fullmatch(r'(\w+)\.(\w+)',args[0])
    if not field:return None
    alias,idfield=field.groups();declarations=[]
    for n,line in enumerate(script.lines,1):
        m=re.fullmatch(r'\s*local\s+'+re.escape(alias)+r'\s*=\s*(\w+)\[(\w+)\.(uid|itemid)\]\s*(?:--.*)?',line)
        if m:declarations.append((n,m))
    if len(declarations)!=1:return None
    line,m=declarations[0];root,selector,kind=m.groups();scope=script.line_scopes.get(line);current=script.line_scopes.get(number)
    if line>=number or scope is None or current is None or current[:len(scope)]!=scope or any(s[0]=='opaque'for s in scope):return None
    if script.roles.get(selector)!='source':return None
    body=[script.lines[n-1]for n in script.line_scopes]
    if not builtin_binding_is_pristine(body,selector) or not builtin_binding_is_pristine(body,call[1]):return None
    # A record captured before an Item mutation cannot be guarded by its later live UID/type.
    for method in re.findall(r'\b'+re.escape(selector)+r':(\w+)\s*\(', '\n'.join(script.code_lines)):
        if method not in ('getId','getUniqueId','getActionId'):return None
    table_lines=[n for n in unconditional_prefix(script.lines)if re.match(r'\s*local\s+'+re.escape(root)+r'\s*=\s*\{',script.lines[n-1])]
    if len(table_lines)!=1 or table_lines[0]>=line:return None
    text='\n'.join(script.lines[table_lines[0]-1:]);text=text[text.index('{'):]
    try:
        parsed=LiteralParser(text).table()
        if not duplicate_free(parsed):return None
        values=lua_tables.as_python(parsed)
    except (lua_tables.LuaError,ValueError,IndexError,TypeError):return None
    if not isinstance(values,dict)or not 1<=len(values)<=128:return None
    resolved={}
    countfield=re.fullmatch(re.escape(alias)+r'\.(\w+)',args[1]);literal=re.fullmatch(r'\d+',args[1])
    if not countfield and not literal:return None
    for key,value in values.items():
        if type(key)is not int or not 0<key<=65535 or not isinstance(value,dict):return None
        ident=value.get(idfield);count=value.get(countfield[1])if countfield else int(args[1])
        if type(ident)is not int or not 0<ident<=2147483647 or type(count)is not int or not 0<count<=2147483647:return None
        # Other metadata can be literal scalars or symbolic storage values only;
        # executable expressions/table/callback constructors are never admitted.
        if any(type(v)not in (int,str,bool,type(None))and not(isinstance(v,dict)and set(v)=={'expr'}and re.fullmatch(r'Storage\.[A-Za-z0-9_.]+',v['expr']))for v in value.values()):return None
        resolved[key]=(ident,count)
    tokens=lua_tables.tokenize('\n'.join(script.lines))
    if any(k=='name'and v in ENV for k,v,_ in tokens):return None
    # Remove only exact alias capture from root escape analysis; independently
    # prove the selected record cannot mutate/escape through that alias.
    alias_fields={alias+'.'+k for v in values.values()for k in v}
    # A missing field of every duplicate-free plain literal record is nil.
    # Admit its scalar read, while the unchanged immutable proof rejects writes,
    # escapes, shadowing, metatables and dynamic member selection.
    referenced_fields=set(re.findall(r'\b'+re.escape(alias)+r'\.(\w+)', '\n'.join(script.code_lines)))
    alias_fields.update(alias+'.'+field for field in referenced_fields
                        if all(field not in record for record in values.values()))
    if not static_name_is_immutable(script.lines,alias,references=True,scalar_paths=alias_fields):return None
    proof=script.lines[:];proof[line-1]='local '+alias+' = 0'
    # Allow only three-line read-only UID registration via pairs(root).
    for n,l in enumerate(proof):
        if re.search(r'\bpairs\(\s*'+re.escape(root)+r'\s*\)',l):
            h=re.fullmatch(r'\s*for\s+(\w+)\s*,\s*\w+\s+in\s+pairs\('+re.escape(root)+r'\)\s+do\s*',l)
            if not h or n+2>=len(proof)or not re.fullmatch(r'\s*\w+:(uid|id)\('+h[1]+r'\)\s*',proof[n+1])or proof[n+2].strip()!='end':return None
            proof[n]=proof[n+1]=proof[n+2]=''
    if not pristine_constructor(script.lines,'pairs'):return None
    if not static_name_is_immutable(proof,root,references=True,scalar_paths=scalar_leaf_paths(values,root)):return None
    # Shared immutable proof is complemented with compound-LHS rejection.
    visible='\n'.join(script.code_lines)
    if re.search(r'\b(?:'+re.escape(root)+'|'+re.escape(alias)+r')(?:\.[\w]+|\[[^\]]+\])*\s*,[^\n]*=(?!=)',visible):return None
    return {'selector':selector,'selector_kind':kind,'record_alias':alias,'table':root,'table_line':table_lines[0],'alias_line':line,'rewards':resolved}


def apply(script, graph):
    graph=copy.deepcopy(graph);changes=[]
    def walk(rules, pointer):
        for i,child in enumerate(rules):
            if 'branch'in child:
                for bindex,b in enumerate(child['branch']):walk(b['then'],pointer+'/'+str(i)+'/branch/'+str(bindex)+'/then')
                walk(child.get('otherwise',[]),pointer+'/'+str(i)+'/otherwise');continue
            if child.get('owner')!='Item'or child.get('request')!='hand_out'or 'value_source_line'not in child:continue
            line=child['value_source_line'];data=record_reward(script,line)
            if not data:continue
            branches=[]
            for key,(ident,count)in sorted(data['rewards'].items()):
                condition={'object':{'role':'source','field':'unique_id','op':'==','value':key}}if data['selector_kind']=='uid'else{'object':{'role':'source','field':'item_type','op':'==','item':{'family':'Item','key':script.namespace+':item/'+str(key),'revision':graph['identity']['revision']}}}
                condition['negate']=False
                branches.append({'when':condition,'then':[{'owner':'Item','request':'hand_out','item':{'family':'Item','key':script.namespace+':item/'+str(ident),'revision':graph['identity']['revision']},'count':count}]})
            rules[i]={'branch':branches,'otherwise':[copy.deepcopy(child)]};changes.append({'line':line,**data,'fallback_preserved':True,'baseline_child_pointer':pointer+'/'+str(i)})
    walk(graph['rules'],'/rules');return graph,changes
