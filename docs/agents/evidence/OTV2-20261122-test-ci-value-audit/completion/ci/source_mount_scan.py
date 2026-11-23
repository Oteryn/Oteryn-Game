"""Possible Rust source mounts, with cfg and generated-source uncertainty visible.

This is a source graph, not cargo/rustc test discovery or proof of execution.
Unreachable files require investigation; they are never automatically obsolete.
"""
import argparse
import collections
import csv
import hashlib
import json
import pathlib
import re
import subprocess
import tomllib

from tree_sitter import Language, Parser
import tree_sitter_rust
from source_value_scan import preceding_attributes, walk


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('--repo',type=pathlib.Path,required=True)
    args.add_argument('--output',type=pathlib.Path,required=True)
    opts=args.parse_args()
    root=opts.repo.resolve()
    parser=Parser(Language(tree_sitter_rust.language()))
    paths=subprocess.check_output(['git','ls-files','--','*.rs'],cwd=root,text=True).splitlines()
    edges,unresolved=[],[]
    def rel(path):
        absolute=path.resolve()
        return str(absolute.relative_to(root)) if absolute.is_relative_to(root) else None
    def add(source,target,kind,attributes=(),line=None):
        target=rel(target)
        if target and (root/target).is_file():
            edges.append({'source':source,'target':target,'kind':kind,
                          'attributes':list(attributes),'line':line})
            return True
        return False
    for path in paths:
        file=root/path
        if not file.is_file():continue
        source=file.read_bytes()
        tree=parser.parse(source)
        for node in walk(tree.root_node):
            if node.type=='mod_item' and node.child_by_field_name('body') is None:
                name=node.child_by_field_name('name').text.decode()
                attrs=preceding_attributes(node,source)
                ancestors=[]
                parent=node.parent
                allattrs=list(attrs)
                while parent is not None:
                    if parent.type=='mod_item':
                        ancestors.append(parent.child_by_field_name('name').text.decode())
                        allattrs+=preceding_attributes(parent,source)
                    parent=parent.parent
                ancestors.reverse()
                explicit=[m.group(1) for a in attrs for m in re.finditer(r'\bpath\s*=\s*"([^"]+)"',a)]
                base=file.parent if file.name in {'lib.rs','main.rs','mod.rs'} else file.parent/file.stem
                bases=[base.joinpath(*ancestors)]
                if explicit:
                    bases=[file.parent] if not ancestors else [base.joinpath(*ancestors),file.parent.joinpath(*ancestors)]
                    candidates=[b/p for b in bases for p in explicit]
                else:
                    candidates=[p for b in bases for p in [b/(name+'.rs'),b/name/'mod.rs']]
                found=[]
                for candidate in candidates:
                    target=rel(candidate)
                    if target not in found and add(path,candidate,'path_attribute' if explicit else 'mod',allattrs,node.start_point.row+1):
                        found.append(target)
                if not found:
                    unresolved.append({'source':path,'line':node.start_point.row+1,'kind':'unresolved_mod',
                                       'name':name,'attributes':allattrs,
                                       'candidates':[rel(c) for c in candidates]})
            elif node.type=='macro_invocation':
                macro=node.child_by_field_name('macro')
                if macro is None or macro.text!=b'include':continue
                text=source[node.start_byte:node.end_byte].decode()
                match=re.fullmatch(r'include!\s*\(\s*"([^"]+)"\s*\)',text)
                if match and add(path,file.parent/match[1],'literal_include',line=node.start_point.row+1):continue
                manifest_include=re.fullmatch(r'include!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"]+)"\s*,?\s*\)\s*\)',text)
                if manifest_include:
                    owner=file.parent
                    while owner.is_relative_to(root) and not (owner/'Cargo.toml').is_file():owner=owner.parent
                    if owner.is_relative_to(root) and add(path,owner/manifest_include[1].lstrip('/'),'manifest_literal_include',line=node.start_point.row+1):continue
                unresolved.append({'source':path,'line':node.start_point.row+1,'kind':'generated_or_unresolved_include','expression':text})
    evidence=root/'docs/agents/evidence/OTV2-20261122-test-ci-value-audit'
    manifests=list(csv.DictReader((evidence/'test_value_workspace_members.tsv').open(),delimiter='\t'))
    roots,manifest_errors=[],[]
    for member in manifests:
        package=root/member['path']
        manifest=package/'Cargo.toml'
        try:value=tomllib.loads(manifest.read_text())
        except (OSError,tomllib.TOMLDecodeError) as error:
            manifest_errors.append({'manifest':str(manifest.relative_to(root)),'error':str(error)})
            continue
        entries=[]
        lib=value.get('lib',{})
        entries.append(('lib',package/lib.get('path','src/lib.rs'),lib.get('required-features',[])))
        for kind in ['bin','test','example','bench']:
            for target in value.get(kind,[]):
                name=target.get('name','')
                default={'bin':'src/bin/'+name+'.rs','test':'tests/'+name+'.rs','example':'examples/'+name+'.rs','bench':'benches/'+name+'.rs'}[kind]
                entries.append((kind,package/target.get('path',default),target.get('required-features',[])))
        if value.get('package',{}).get('autobins',True):
            entries.append(('auto_bin',package/'src/main.rs',[]))
            entries += [('auto_bin',p,[]) for p in (package/'src/bin').glob('*.rs')]
            entries += [('auto_bin',p,[]) for p in (package/'src/bin').glob('*/main.rs')]
        for kind,setting in [('tests','autotests'),('examples','autoexamples'),('benches','autobenches')]:
            if value.get('package',{}).get(setting,True):
                entries += [('auto_'+kind,p,[]) for p in (package/kind).glob('*.rs')]
                entries += [('auto_'+kind,p,[]) for p in (package/kind).glob('*/main.rs')]
        for kind,path,features in entries:
            if path.is_file():
                roots.append({'path':rel(path),'package':member['package'],'manifest':rel(manifest),
                              'kind':kind,'required_features':features})
    # Manual source trace: build.rs -> build_support/play_features.rs -> concerns
    # and tests/play.rs for exactly the directories containing play.json. This
    # adds possible source mounts; it does not expand or execute the generator.
    generated=[]
    features=root/'apps/client/src/features'
    for fragment in sorted(features.glob('*/play.json')):
        document=json.loads(fragment.read_text())
        base=fragment.parent
        test=base/'tests/play.rs'
        if add('apps/client/src/play/tests.rs',test,'audited_play_feature_generator'):
            generated.append(rel(test))
        for concern in document.get('concerns',[]):
            add('apps/client/src/play.rs',base/(concern+'.rs'),'audited_play_feature_generator')
    # Explicit path mounts and include! establish a source-directory context.
    # Add this alternative only for files with such an observed mount, retaining
    # the evidence kind. This is possible selection, not a rustc resolution claim.
    explicit_targets={e['target'] for e in edges if e['kind'] in {
        'path_attribute','literal_include','manifest_literal_include','audited_play_feature_generator'}}
    remaining=[]
    for item in unresolved:
        if item['kind']=='unresolved_mod' and item['source'] in explicit_targets:
            base=(root/item['source']).parent
            found=False
            for candidate in [base/(item['name']+'.rs'),base/item['name']/'mod.rs']:
                found=add(item['source'],candidate,'explicit_mount_directory_candidate',item['attributes'],item['line']) or found
            if found:continue
        remaining.append(item)
    unresolved=remaining
    adjacency=collections.defaultdict(list)
    for edge in edges:adjacency[edge['source']].append(edge)
    parent={}
    queue=collections.deque()
    for entry in roots:
        if entry['path'] not in parent:
            parent[entry['path']]=None;queue.append(entry['path'])
    while queue:
        source=queue.popleft()
        for edge in adjacency[source]:
            if edge['target'] not in parent:
                parent[edge['target']]=edge
                queue.append(edge['target'])
    assigned=[]
    for assignment in sorted((evidence/'completion/ci').glob('full_scope_assignment_*.json')):
        data=json.loads(assignment.read_text())
        for file in data['files']:
            if not file['path'].endswith('.rs'):continue
            path=file['path'];chain=[];cursor=path
            while cursor in parent and parent[cursor] is not None:
                edge=parent[cursor];chain.append(edge);cursor=edge['source']
            assigned.append({'path':path,'owner':data['owner'],'source_sha256':hashlib.sha256((root/path).read_bytes()).hexdigest(),
                             'possible_mount_found':path in parent,'source_mount_chain':list(reversed(chain)),
                             'entry':cursor if cursor in parent else None})
    result={'method':__doc__,'scanner_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),
            'scope_limits':['All cfg alternatives are possible; no default/platform/feature execution is asserted.',
                            'Roots use the baseline workspace-member inventory and current existing manifests.',
                            'Other generated sources and unsupported module layouts remain unresolved, not obsolete.',
                            'Only apps/client play-feature generator has a separately audited deterministic mount rule.'],
            'summary':{'rust_files_parsed':len(paths),'roots':len(roots),'edges':len(edges),
                       'assigned_rust_paths':len(assigned),'assigned_possible_mounts':sum(x['possible_mount_found'] for x in assigned),
                       'assigned_unresolved_mounts':sum(not x['possible_mount_found'] for x in assigned),
                       'generated_play_tests':len(generated),'unresolved_edges':len(unresolved)},
            'manifest_errors':manifest_errors,'roots':roots,'edges':edges,'unresolved_edges':unresolved,
            'assigned_paths':assigned}
    opts.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result['summary'],sort_keys=True))


if __name__=='__main__':main()
