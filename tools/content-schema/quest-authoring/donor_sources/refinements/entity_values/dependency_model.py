"""Pinned external LuaJIT Source bodies; no executable dependency admission."""
import pathlib,json,hashlib,re
ROOT=pathlib.Path(__file__).parent/'dependency'
def digest(b):return hashlib.sha256(b).hexdigest()
def dependency_catalog(root=ROOT):
 root=pathlib.Path(root);m=json.loads((root/'luajit-acquisition.json').read_bytes());ports=json.loads((root/'vcpkg-acquisition.json').read_bytes());texts={};rows={}
 for r in m['files']+ports:
  if 'sha256' not in r:continue
  raw=(root/r['local_file']).read_bytes()
  if digest(raw)!=r['sha256'] or len(raw)!=r['byte_count']:raise ValueError('Pinned dependency Source mismatch')
  texts[(r['donor'],r['path'])]=raw.decode();rows[(r['donor'],r['path'])]=r
 def witness(d,path,start,end):
  r=rows[d,path];t=texts[d,path];return {k:r[k] for k in ('repository','revision','path','sha256','byte_count','url')}|{'start_char':start,'end_char_exclusive':end,'slice_sha256':digest(t[start:end].encode()),'source_text':t[start:end],'transport':'NORMAL_HTTPS_PUBLIC_SOURCE','archive_sha512_verified':False}
 def region(d,path,pattern):
  t=texts[d,path];a=re.search(pattern,t,re.S)
  if not a:raise ValueError('Expected pinned library Source body missing')
  return witness(d,path,a.start(),a.end())
 result={}
 for mapping in m['mapping']:
  d=mapping['donor'];revision=mapping['luajit_revision'];port=texts[d,'ports/luajit/portfile.cmake'];match=re.search(r'REF\s+([0-9a-f]{40})',port)
  if not match or match[1]!=revision:raise ValueError('Pinned port REF mismatch')
  b=region(d,'src/lj_buf.c',r'SBuf \* LJ_FASTCALL lj_buf_putstr_lower\(SBuf \*sb, GCstr \*s\)\s*\{.*?\n\}')
  if "c >= 'A' && c <= 'Z'" not in b['source_text'] or '0x20' not in b['source_text']:raise ValueError('Unexpected lower byte algorithm')
  defs=[b,region(d,'src/lib_string.c',r'LJLIB_ASM_\(string_lower\)[^\n]*'),region(d,'src/vm_x64.dasc',r'\|\.macro ffstring_op, name.*?\|ffstring_op lower'),region(d,'src/lib_string.c',r'LUALIB_API int luaopen_string\(lua_State \*L\).*?\n\}'),region(d,'src/lib_init.c',r'LUALIB_API void luaL_openlibs\(lua_State \*L\).*?\n\}')]
  result[d]={'lower':{'method':'lower','implementation_language':'PINNED_EXTERNAL_LUAJIT','dependency_pin':revision,'port_ref_witness':witness(d,'ports/luajit/portfile.cmake',0,len(port)),'definitions':defs,'semantics':{'operation':'BYTE_ASCII_CASE_MAP','input':'SOURCE_STRING_BYTES','byte_count':'UNCHANGED','mapping':'BYTES_65_THROUGH_90_ADD_32_OTHER_BYTES_UNCHANGED','unicode_or_locale_mapping':False,'allocation':'SOURCE_BUFFER_AND_STRING_ALLOCATION_MAY_FAIL','fast_path':'STRING_TYPE_CHECK_THEN_BUFFER_LOWER_THEN_STRING_RESULT','other_types':'SOURCE_FAST_FUNCTION_FALLBACK_UNPROVEN_LIVE_COERCION','string_method_registration':'STRING_BASE_METATABLE_INDEX_TO_LIBRARY_TABLE','result':'STRING','live_value':'UNKNOWN'},'dispatch':'UNPROVEN','deployed_library_version':'UNPROVEN','metatable_or_global_mutation':'UNPROVEN','native_admission':False}}
 return result
