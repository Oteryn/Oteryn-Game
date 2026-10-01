"""Build a standalone, offline comparison from the validated authoring candidate."""
import json
import sys
from pathlib import Path
from wheel_authoring import ROOT, read, validate
from client_icons import validate_manifest, validate_selection


def build_report():
    candidate = read(ROOT / 'samples/wheel-candidate.json')
    validate(candidate)
    manifest = read(ROOT / 'samples/client-icon-manifest.json')
    validate_manifest(manifest, candidate)
    validate_selection(candidate)
    # Escape HTML script terminators; render every data string with textContent.
    embedded = json.dumps(candidate, ensure_ascii=False, allow_nan=False).replace('<', '\\u003c')
    icon_data = json.dumps(manifest, ensure_ascii=False, allow_nan=False).replace('<', '\\u003c')
    selection_data = json.dumps(read(ROOT / 'samples/reference-selection.json'), ensure_ascii=False, allow_nan=False).replace('<', '\\u003c')
    template = '''<!doctype html><html lang="pl"><meta charset="utf-8">
<meta name="viewport" content="width=device-width"><title>Wheel of Destiny — schemat</title>
<style>body{font:15px system-ui;background:#101924;color:#e6edf4;margin:24px auto;padding:16px;max-width:1450px}p{line-height:1.5}select,input{padding:10px;margin:8px;background:#223346;color:white}table{width:100%;border-collapse:collapse}td,th{padding:12px;text-align:left;vertical-align:top;border-bottom:1px solid #324356}details{padding:14px;margin:8px 0;background:#1b2939}summary{cursor:pointer}pre{white-space:pre-wrap}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px}a{color:#83c7ff}.icon{display:inline-block;position:relative;overflow:hidden;vertical-align:middle;margin:4px;}.icon img{position:absolute;top:0;max-width:none}.icon-pending{min-width:24px;min-height:24px;font-size:11px;color:#a9bbce}</style>
<h1>Wheel of Destiny — schemat referencyjny</h1>
<p>5 profesji · 180 pól · Dedication, Conviction, Revelation · Gem Atelier.
Dane są kandydatem do authoringu. Wdrożenie efektów w silniku i assety klienta wymagają osobnej integracji.</p>
<label>Profesja <select id="vocation"></select></label><label>Szukaj <input id="search"></label>
<div id="revelations" class="grid"></div><h2>Pola koła</h2>
<table><thead><tr><th>Slot / domena / limit</th><th>Dedication za punkt</th><th>Conviction po wypełnieniu</th><th>Ikony</th></tr></thead><tbody id="slots"></tbody></table>
<p>Ikony korzystają z kompletnego manifestu referencyjnego: arkusz, SHA-256 i wycinek dla każdego ID. Podgląd pobiera publiczne arkusze przypięte do rewizji źródła; bez sieci pokazuje ID. Jest to materiał referencyjny, bez dopuszczenia assetów do dystrybucji klienta.</p><details><summary>Manifest wszystkich 205 ikon</summary><div id="icon-catalogue"></div></details><h2>Gemy profesji i koszty Atelier</h2><div id="gems"></div>
<details><summary>Źródła i granice potwierdzenia</summary><pre id="sources"></pre></details>
<p><a href="samples/wheel-candidate.json">Pełne dane JSON</a> · <a href="wheel.schema.json">JSON Schema</a> · <a href="samples/client-icon-manifest.json">Manifest ikon</a> · <a href="samples/reference-selection.json">Wybór źródeł</a></p>
<script id="data" type="application/json">CANDIDATE_DATA</script><script id="icons-data" type="application/json">ICON_DATA</script><script id="selection-data" type="application/json">SELECTION_DATA</script><script>
const manifest=JSON.parse(document.querySelector('#icons-data').textContent);
function icon(reference){const key=reference.sprite+':'+reference.source_index,entry=manifest.icons[key],sheet=manifest.sheets[entry.sheet],box=el('span',key);box.className='icon icon-pending';box.dataset.iconKey=key;box.title=key+' / '+sheet.sha256;const img=new Image();img.onload=()=>{if(img.naturalWidth!==sheet.width||img.naturalHeight!==sheet.height){box.textContent=key+' (inny arkusz)';box.dataset.loaded='false';return}const [x,y,w,h]=entry.rect;box.textContent='';box.className='icon';box.style.width=w+'px';box.style.height=h+'px';img.style.left=(-x)+'px';img.style.top=(-y)+'px';box.append(img);box.dataset.loaded='true'};img.onerror=()=>{box.textContent=key+' (brak podglądu)';box.dataset.loaded='false'};img.src=sheet.reference_url;return box}
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
document.querySelector('#sources').textContent=JSON.stringify({sources:data.sources,verification:data.verification,corrections:data.gems.reference_corrections,icons:data.icon_evidence,selection:JSON.parse(document.querySelector('#selection-data').textContent)},null,2);
for(const [key,entry] of Object.entries(manifest.icons)){const row=el('span');row.append(icon({sprite:entry.sheet,source_index:entry.source_index}),el('span',key+' '));document.querySelector('#icon-catalogue').append(row)}
voc.onchange=render;document.querySelector('#search').oninput=render;render();
</script></html>'''
    return template.replace('CANDIDATE_DATA', embedded).replace('ICON_DATA', icon_data).replace('SELECTION_DATA', selection_data)


if __name__ == '__main__':
    path = ROOT / 'wheel-comparison.html'
    report = build_report()
    if '--check' in sys.argv:
        if path.read_text() != report:raise ValueError('REPORT_REBUILD_DRIFT')
    else:path.write_text(report)
