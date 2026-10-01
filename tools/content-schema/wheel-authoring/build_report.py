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
<style>body{font:15px system-ui;background:#101924;color:#e6edf4;margin:24px auto;padding:16px;max-width:1450px}p{line-height:1.5}select,input{padding:10px;margin:8px;background:#223346;color:white}table{width:100%;border-collapse:collapse}td,th{padding:12px;text-align:left;vertical-align:top;border-bottom:1px solid #324356}details{padding:14px;margin:8px 0;background:#1b2939}summary{cursor:pointer}pre{white-space:pre-wrap}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px}a{color:#83c7ff}.icon{display:inline-block;position:relative;overflow:hidden;vertical-align:middle;margin:4px;min-width:24px;min-height:24px}.icon img{position:absolute;top:0;max-width:none}.icon-pending{font-size:11px;color:#a9bbce}</style>
<h1>Wheel of Destiny — schemat referencyjny</h1>
<p>5 profesji · 180 pól · Dedication, Conviction, Revelation · Gem Atelier.
Dane są kandydatem do authoringu. Wdrożenie efektów w silniku i assety klienta wymagają osobnej integracji.</p>
<label>Profesja <select id="vocation"></select></label><label>Szukaj <input id="search"></label>
<div id="revelations" class="grid"></div><h2>Pola koła</h2>
<table><thead><tr><th>Slot / domena / limit</th><th>Dedication za punkt</th><th>Conviction po wypełnieniu</th><th>Ikony</th></tr></thead><tbody id="slots"></tbody></table>
<p>Ikony ładowane są bezpośrednio z CDN Tibii. Jeśli dostęp jest zablokowany, pokazujemy identyfikator; sam identyfikator nie potwierdza wyglądu.</p><h2>Gemy profesji i koszty Atelier</h2><div id="gems"></div>
<details><summary>Źródła i granice potwierdzenia</summary><pre id="sources"></pre></details>
<p><a href="samples/wheel-candidate.json">Pełne dane JSON</a> · <a href="wheel.schema.json">JSON Schema</a></p>
<script id="data" type="application/json">CANDIDATE_DATA</script><script>
function icon(reference){const box=el('span','ID '+reference.source_index);box.className='icon icon-pending';box.title=reference.sprite+' / '+reference.source_index;const img=new Image();img.onload=()=>{const size=img.naturalHeight;if(!size||img.naturalWidth<(reference.source_index+1)*size)return;box.textContent='';box.style.width=size+'px';box.style.height=size+'px';img.style.left=(-reference.source_index*size)+'px';box.append(img)};img.onerror=()=>{box.textContent='ID '+reference.source_index+' (CDN niedostępny)'};img.src=reference.asset_url;return box}
const data=JSON.parse(document.querySelector('#data').textContent),voc=document.querySelector('#vocation');
function el(tag,text){const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e}
function detail(label,value,reference){const d=el('details'),summary=el('summary',label);if(reference)summary.prepend(icon(reference));d.append(summary,el('pre',typeof value==='string'?value:JSON.stringify(value,null,2)));return d}
for(const v of Object.keys(data.vocations))voc.add(new Option(v,v));
function render(){const d=data.vocations[voc.value],query=document.querySelector('#search').value.toLowerCase(),revs=document.querySelector('#revelations');revs.replaceChildren();
for(const r of d.revelations){const box=el('div');box.append(icon(r.icon),el('h3',r.domain+' · '+r.name));for(const s of r.stages)box.append(detail('Stage '+s.stage+' / '+s.minimum_domain_points+' punktów',s));revs.append(box)}
const rows=document.querySelector('#slots');rows.replaceChildren();for(const s of d.slots){const t=data.topology[s.state_slot-1];if(!JSON.stringify([s,t]).toLowerCase().includes(query))continue;const tr=el('tr');tr.append(el('td',s.state_slot+' / '+t.domain+' / '+t.capacity),el('td',s.dedication.map(e=>'+'+e.value_per_point+' '+e.unit+' '+e.stat).join('; ')));const c=el('td');c.append(el('b',s.conviction.name),detail('Efekty i warunki',s.conviction),detail('Odblokowanie',t));const icons=el('td');icons.append(icon(s.dedication_icon),icon(s.conviction.icon));tr.append(c,icons);rows.append(tr)}
const gems=document.querySelector('#gems');gems.replaceChildren(detail('Zasady koła i punkty',data.progression),detail('Rodzina i przedmioty',d.gem_family),detail('Jakości',data.gems.qualities),detail('Atelier',data.gems.atelier),detail('Koszty stopni',data.gems.grade_costs),detail('Loot: hipoteza OTS',data.gems.loot_reference));
for(const id of d.supreme_mods){const m=data.gems.supreme_mods.find(x=>x.source_id===id);gems.append(detail(m.summary_name+' / ID '+id,m.grades,m.icon))}
const basic=data.gems.basic_mods.filter(m=>d.basic_mods_position_1.includes(m.source_id)||d.basic_mods_position_2.includes(m.source_id));for(const m of basic)gems.append(detail(m.effects.map(e=>e.name).join(' + ')+' / ID '+m.source_id,m.effects.map(e=>({name:e.name,unit:e.unit,grades:e.values_by_vocation[voc.value]})),m.icon));
}
document.querySelector('#sources').textContent=JSON.stringify({sources:data.sources,verification:data.verification,corrections:data.gems.reference_corrections,icons:data.icon_evidence},null,2);
voc.onchange=render;document.querySelector('#search').oninput=render;render();
</script></html>'''
    return template.replace('CANDIDATE_DATA', embedded)


if __name__ == '__main__':
    (ROOT / 'wheel-comparison.html').write_text(build_report())
