import importlib.util,json,subprocess,select,time,os
from pathlib import Path
p=Path('/workspace/scratch/weapon-proficiency-execution/prof1-sql');spec=importlib.util.spec_from_file_location('q',p/'qualify_semantics.py');q=importlib.util.module_from_spec(spec);spec.loader.exec_module(q)
report=json.loads((p/'semantic-results.json').read_text());source=report['database'];q.DB=q.DB+'_race';q.must('CREATE DATABASE '+q.DB+' TEMPLATE '+source+';', 'postgres');q.must('BEGIN;'+q.SEED+'COMMIT;')
cmd=['docker','--host=unix:///var/run/docker.sock','exec','-i',q.CONTAINER,'psql','-X','-U','oteryn_test_admin','-d',q.DB,'-v','ON_ERROR_STOP=1','-v','VERBOSITY=verbose','-At']
a=subprocess.Popen(cmd,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=q.env)
a.stdin.write('BEGIN;'+q.training()+"SET CONSTRAINTS ALL IMMEDIATE;SELECT 'A_READY';\n");a.stdin.flush()
ready=False;deadline=time.monotonic()+10;ready_output=b''
while time.monotonic()<deadline:
 if select.select([a.stdout],[],[],0.1)[0]:
  ready_output+=os.read(a.stdout.fileno(),65536)
  if b'A_READY' in ready_output:ready=True;break
assert ready,'A failed to reach fully validated locked transaction'
b=subprocess.Popen(cmd,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=q.env)
b.stdin.write('BEGIN;'+q.advance()+'COMMIT;\n');b.stdin.close()
time.sleep(.2);blocked=b.poll() is None
assert blocked,'B must wait on A Character root lock'
a.stdin.write('COMMIT;\n');a.stdin.close();a.wait(timeout=10);b.wait(timeout=10)
aerr=a.stderr.read();berr=b.stderr.read();assert a.returncode==0,aerr
assert b.returncode!=0 and '23514' in berr and 'exact revision successor' in berr,berr
counts=q.must("SELECT (SELECT character_revision FROM game_character_roots),(SELECT character_revision FROM game_character_progression_state),(SELECT count(*) FROM game_character_proficiency_receipts),(SELECT count(*) FROM game_character_proficiency_receipt_lines),(SELECT count(*) FROM game_character_proficiency);").strip();assert counts=='2|2|1|1|1',counts
result=dict(database=q.DB,candidate_sha256=report['candidate_sha256'],case='concurrent_absolute_successor_replay',blocked_before_winner_commit=blocked,winner_committed=True,loser_sqlstate='23514',loser_error=berr.strip(),committed_counts=counts,status='PASS')
(p/'concurrency-results.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
