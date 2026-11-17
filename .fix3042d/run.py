import subprocess,sys,pathlib,time,re
p=pathlib.Path('.fix3042d')
phase=sys.argv[1]
for name in (sys.argv[2:] or (p/'names').read_text().splitlines()):
 print('START '+name,flush=True)
 cmd=['/home/mole/oteryn-cache/codex-bin/cargo','test','--offline','--locked','-p','oteryn-gameplay-dispatch','--all-features','--lib',name,'--','--nocapture']
 t=time.monotonic()
 lines=[]
 with (p/(phase+'-'+name+'.log')).open('w') as log:
  r=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  for line in r.stdout:
   line=re.sub(r'postgres(?:ql)?://[^\s]+','[REDACTED]',line)
   lines.append(line);log.write(line);log.flush()
  r.wait()
 text=''.join(lines)
 print(name+' exit='+str(r.returncode)+' seconds='+str(round(time.monotonic()-t)),flush=True)
 print('\n'.join(text.splitlines()[-24:]),flush=True)
