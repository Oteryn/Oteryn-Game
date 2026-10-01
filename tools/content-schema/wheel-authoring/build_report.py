"""Build a standalone, offline comparison from the validated authoring candidate."""
import json
from pathlib import Path
from wheel_authoring import ROOT, read, validate


def build_report():
    candidate = read(ROOT / 'samples/wheel-candidate.json')
    validate(candidate)
    # Escape HTML script terminators; render every data string with textContent.
    embedded = json.dumps(candidate, ensure_ascii=False).replace('<', '\\u003c')
    template = '''<!doctype html><html lang="pl"><meta charset="utf-8">
<meta name="viewport" content="width=device-width"><title>Wheel of Destiny — schemat</title>
<style>body{font:15px system-ui;background:#101924;color:#e6edf4;margin:24px auto;padding:16px;max-width:1450px}p{line-height:1.5}select,input{padding:10px;margin:8px;background:#223346;color:white}table{width:100%;border-collapse:collapse}td,th{padding:12px;text-align:left;vertical-align:top;border-bottom:1px solid #324356}details{padding:14px;margin:8px 0;background:#1b2939}summary{cursor:pointer}pre{white-space:pre-wrap}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px}a{color:#83c7ff}</style>
<h1>Wheel of Destiny — schemat referencyjny</h1>
<p>5 profesji · 180 pól · Dedication, Conviction, Revelation · Gem Atelier.
Dane są kandydatem do authoringu. Wdrożenie efektów w silniku i assety klienta wymagają osobnej integracji.</p>
<label>Profesja <select id="vocation"></select></label><label>Szukaj <input id="search"></label>
<div id="revelations" class="grid"></div><h2>Pola koła</h2>
<table><thead><tr><th>Slot / domena / limit</th><th>Dedication za punkt</th><th>Conviction po wypełnieniu</th><th>Ikony</th></tr></thead><tbody id="slots"></tbody></table>
<h2>Gemy profesji i koszty Atelier</h2><div id="gems"></div>
<details><summary>Źródła i granice potwierdzenia</summary><pre id="sources"></pre></details>
<p><a href="samples/wheel-candidate.json">Pełne dane JSON</a> · <a href="wheel.schema.json">JSON Schema</a></p>
<script id="data" type="application/json">CANDIDATE_DATA</script><script>
const data=JSON.parse(document.querySelector('#data').textContent),voc=document.querySelector('#vocation');
function el(tag,text){const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e}
function detail(label,value){const d=el('details');d.append(el('summary',label),el('pre',typeof value==='string'?value:JSON.stringify(value,null,2)));return d}
for(const v of Object.keys(data.vocations))voc.add(new Option(v,v));
function render(){const d=data.vocations[voc.value],query=document.querySelector('#search').value.toLowerCase(),revs=document.querySelector('#revelations');revs.replaceChildren();
for(const r of d.revelations){const box=el('div');box.append(el('h3',r.domain+' · '+r.name));for(const s of r.stages)box.append(detail('Stage '+s.stage+' / '+s.minimum_domain_points+' punktów',s));revs.append(box)}
const rows=document.querySelector('#slots');rows.replaceChildren();for(const s of d.slots){const t=data.topology[s.state_slot-1];if(!JSON.stringify([s,t]).toLowerCase().includes(query))continue;const tr=el('tr');tr.append(el('td',s.state_slot+' / '+t.domain+' / '+t.capacity),el('td',s.dedication.map(e=>'+'+e.value_per_point+' '+e.unit+' '+e.stat).join('; ')));const c=el('td');c.append(el('b',s.conviction.name),detail('Efekty i warunki',s.conviction),detail('Odblokowanie',t));tr.append(c,el('td','Dedication '+s.dedication_icon.source_index+'; Conviction '+s.conviction.icon.source_index));rows.append(tr)}
const gems=document.querySelector('#gems');gems.replaceChildren(detail('Rodzina i przedmioty',d.gem_family),detail('Jakości',data.gems.qualities),detail('Atelier',data.gems.atelier),detail('Koszty stopni',data.gems.grade_costs));
for(const id of d.supreme_mods){const m=data.gems.supreme_mods.find(x=>x.source_id===id);gems.append(detail(m.summary_name+' / ID '+id,m.grades))}
const basic=data.gems.basic_mods.filter(m=>d.basic_mods_position_1.includes(m.source_id)||d.basic_mods_position_2.includes(m.source_id));gems.append(detail('Basic mods dla profesji',basic));
}
document.querySelector('#sources').textContent=JSON.stringify({sources:data.sources,verification:data.verification,corrections:data.gems.reference_corrections},null,2);
voc.onchange=render;document.querySelector('#search').oninput=render;render();
</script></html>'''
    return template.replace('CANDIDATE_DATA', embedded)


if __name__ == '__main__':
    (ROOT / 'wheel-comparison.html').write_text(build_report())
