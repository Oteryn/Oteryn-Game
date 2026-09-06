#!/usr/bin/env python3
"""Build native libFuzzer/ASan targets directly; never normalize product TOML."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,tomllib
SOURCE='7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3'
NIGHTLY='nightly-2026-09-05'
def main():
    target=sys.argv[1]
    if target not in {'wire','content'}: raise ValueError('unknown target')
    root=Path.cwd();out=root/'evidence';out.mkdir(exist_ok=True)
    if subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()!=SOURCE: raise ValueError('wrong product source')
    def call(args,log,check=True,env=None):
        with (out/log).open('w') as stream:
            result=subprocess.run(args,stdout=stream,stderr=subprocess.STDOUT,check=False,env=env)
        if check and result.returncode: raise RuntimeError(f'{log}: exit {result.returncode}')
        return result.returncode
    call(['rustup','toolchain','install',NIGHTLY,'--profile','minimal'],'nightly-install.log')
    call(['rustup','toolchain','install','1.94.0','--profile','minimal'],'stable-install.log')
    call(['rustc','+'+NIGHTLY,'-vV'],'compiler.txt')
    fuzz=root/'fuzz';(fuzz/'fuzz_targets').mkdir(parents=True,exist_ok=True)
    for kind in ('wire','content'):
        (fuzz/'corpus'/kind).mkdir(parents=True,exist_ok=True)
        shutil.copyfile(root/'audit-packet/docs/agents/reports/repository-audit-20260906/probes'/('fuzz_'+kind+'.rs'),fuzz/'fuzz_targets'/(kind+'.rs'))
    (fuzz/'Cargo.toml').write_text('''[package]
name = "oteryn-audit-fuzz"
version = "0.0.0"
publish = false
edition = "2024"
[package.metadata]
cargo-fuzz = true
[dependencies]
libfuzzer-sys = "=0.4.13"
oteryn-game-server = { path = "../apps/game-server" }
sha2 = { version = "=0.11.0", default-features = false }
[workspace]
members = ["."]
[[bin]]
name = "wire"
path = "fuzz_targets/wire.rs"
test = false
doc = false
bench = false
[[bin]]
name = "content"
path = "fuzz_targets/content.rs"
test = false
doc = false
bench = false
''')
    shutil.copyfile(root/'Cargo.lock',fuzz/'Cargo.lock')
    (fuzz/'corpus/wire/valid-protocol-error').write_bytes(bytes([8,14,34,0]))
    (fuzz/'corpus/wire/framed-protocol-error').write_bytes(bytes([0,0,0,4,8,14,34,0]))
    if target=='content':
        shutil.copyfile(root/'audit-packet/docs/agents/reports/repository-audit-20260906/probes/fuzz_seed_writer.rs',root/'apps/game-server/tests/audit_fuzz_seed.rs')
        call(['cargo','+1.94.0','test','--locked','-p','oteryn-game-server','--test','audit_fuzz_seed'],'seeds.log')
        assert len(list((fuzz/'corpus/content').iterdir()))==2
    call(['cargo','+'+NIGHTLY,'metadata','--manifest-path',str(fuzz/'Cargo.toml'),'--format-version','1'],'metadata.json')
    old=tomllib.loads((root/'Cargo.lock').read_text())['package'];new=tomllib.loads((fuzz/'Cargo.lock').read_text())['package']
    originals={(p['name'],p['version']):p.get('checksum') for p in old if 'source' in p};names={n for n,v in originals}
    changed=[p for p in new if p['name'] in names and ((p['name'],p['version']) not in originals or originals[(p['name'],p['version'])]!=p.get('checksum'))]
    (out/'lock-comparison.json').write_text(json.dumps({'changed_original_external_packages':changed},indent=2));assert not changed
    shutil.copyfile(fuzz/'Cargo.lock',out/'fuzz-Cargo.lock')
    subprocess.run(['git','diff','--exit-code'],check=True)
    # cargo-fuzz0.12 parses the root with an older TOML library even when its
    # fuzz-dir is explicit. Use Cargo's native parser and standard libFuzzer
    # sanitizer-coverage instrumentation, retaining both earlier failed attempts.
    flags='-Cpasses=sancov-module -Cllvm-args=-sanitizer-coverage-level=4 -Cllvm-args=-sanitizer-coverage-inline-8bit-counters -Cllvm-args=-sanitizer-coverage-pc-table -Cllvm-args=-sanitizer-coverage-trace-compares -Cllvm-args=-sanitizer-coverage-stack-depth -Zsanitizer=address -Cdebug-assertions --cfg fuzzing -Ccodegen-units=1'
    env=dict(os.environ,RUSTFLAGS=flags,ASAN_OPTIONS='detect_odr_violation=0',CXX='clang++')
    triple='x86_64-unknown-linux-gnu';target_dir=fuzz/'target'
    (out/'instrumentation.json').write_text(json.dumps(dict(driver='direct Cargo plus native libfuzzer-sys',rustflags=flags,ASAN_OPTIONS=env['ASAN_OPTIONS'],target=triple,original_tracked_source_changes=False),indent=2))
    call(['cargo','+'+NIGHTLY,'build','--locked','--manifest-path',str(fuzz/'Cargo.toml'),'--release','--target',triple,'--target-dir',str(target_dir),'--bin',target],'build.log',env=env)
    executable=target_dir/triple/'release'/target
    artifacts=fuzz/'artifacts'/target;artifacts.mkdir(parents=True,exist_ok=True)
    result=call([str(executable),str(fuzz/'corpus'/target),'-artifact_prefix='+str(artifacts)+'/', '-seed=60906','-max_total_time=300','-max_len=1048576','-rss_limit_mb=2048','-timeout=5','-print_final_stats=1'],'run.log',False,env)
    rows=[dict(path=str(p.relative_to(root)),bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest()) for p in (fuzz/'corpus').rglob('*') if p.is_file()]
    (out/'corpus-inventory.json').write_text(json.dumps(rows,indent=2))
    (out/'result.json').write_text(json.dumps(dict(target=target,exit=result,source=SOURCE,binary_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),seconds_budget=300,max_input_bytes=1048576,scope='coverage-guided native libFuzzer/ASan with cfg(fuzzing); not exhaustive or release-equivalent execution'),indent=2))
    subprocess.run(['git','diff','--exit-code'],check=True)
    return result
if __name__=='__main__': raise SystemExit(main())
