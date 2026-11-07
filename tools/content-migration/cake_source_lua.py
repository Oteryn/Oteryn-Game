import ctypes as C
from pathlib import Path
import re
L=C.CDLL('liblua5.4.so.0')
def sig(name,args,ret):
 f=getattr(L,name);f.argtypes=args;f.restype=ret;return f
P=C.c_void_p;I=C.c_int
new=sig('luaL_newstate',[],P);opens=sig('luaL_openlibs',[P],None);close=sig('lua_close',[P],None)
load=sig('luaL_loadbufferx',[P,C.c_char_p,C.c_size_t,C.c_char_p,C.c_char_p],I);call=sig('lua_pcallk',[P,I,I,I,C.c_longlong,P],I)
typeof=sig('lua_type',[P,I],I);top=sig('lua_gettop',[P],I);settop=sig('lua_settop',[P,I],None);get=sig('lua_getglobal',[P,C.c_char_p],I)
boolean=sig('lua_toboolean',[P,I],I);integer=sig('lua_isinteger',[P,I],I);tonum=sig('lua_tonumberx',[P,I,P],C.c_double);toint=sig('lua_tointegerx',[P,I,P],C.c_longlong);tostr=sig('lua_tolstring',[P,I,C.POINTER(C.c_size_t)],P)
pushnil=sig('lua_pushnil',[P],None);next_=sig('lua_next',[P,I],I)
def value(s,i):
 t=typeof(s,i)
 if t==0:return None
 if t==1:return bool(boolean(s,i))
 if t==3:return toint(s,i,None) if integer(s,i) else tonum(s,i,None)
 if t==4:
  size=C.c_size_t();p=tostr(s,i,C.byref(size));return C.string_at(p,size.value).decode()
 if t==5:
  idx=i if i>0 else top(s)+i+1;rows={};pushnil(s)
  while next_(s,idx):
   key=value(s,-2);rows[key]=value(s,-1);settop(s,-2)
  array=[v for k,v in sorted((k,v) for k,v in rows.items() if isinstance(k,int))]
  named={k:v for k,v in rows.items() if not isinstance(k,int)}
  if named:
   if array:named['_list']=array
   return named
  return array
 raise ValueError(('unsupported Lua value',t))
def execute(s,source):
 raw=source.encode();code=load(s,raw,len(raw),b'@pinned-source',None)
 if not code:code=call(s,0,0,0,0,None)
 if code:raise ValueError(value(s,-1))
def capture(path,prelude,config):
 s=new();opens(s)
 try:
  execute(s,prelude.replace('return registered, callbacks, capture_error','__registered=registered; __callbacks=callbacks'))
  items=re.search(r'CakeQuest.Items\s*=\s*(\{)',config,re.S)
  if not items:raise ValueError('exact pinned Items table missing')
  start=items.start(1);depth=0;end=None
  for i,char in enumerate(config[start:],start):
   depth+=(char=='{')-(char=='}')
   if depth==0:end=i+1;break
  if end is None:raise ValueError('unclosed source Items table')
  execute(s,'CakeQuest={}\nCakeQuest.Items='+config[start:end])
  execute(s,path.read_text())
  execute(s,'__name=__registered.name; __monster=__registered.monster')
  get(s,b'__name');name=value(s,-1);settop(s,0);get(s,b'__monster');monster=value(s,-1);settop(s,0);get(s,b'__callbacks');callbacks=value(s,-1)
  return name,monster, {} if callbacks==[] else callbacks
 finally:close(s)
