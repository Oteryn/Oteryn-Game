/* Verify the complete source capture against a pinned TibiaPal checkout, offline. */
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
const repository = process.argv[2];
if (!repository) throw new Error('Usage: node verify_planner.cjs /path/to/pinned/TibiaPal');
const raw = JSON.parse(fs.readFileSync(path.join(__dirname, 'samples/source-wheel-reference.json')));
const actualSha = execFileSync('git', ['-C', repository, 'rev-parse', 'HEAD'], {encoding:'utf8'}).trim();
assert.equal(actualSha, raw.source.commit, 'Reference checkout SHA');
const strings = JSON.parse(fs.readFileSync(path.join(repository, 'data/wheel-planner/SkillwheelStringsJsonLibrary.json')));
const context = {console, WebAssembly, setTimeout, clearTimeout, URL, Uint8Array, TextDecoder};
vm.createContext(context);
vm.runInContext(fs.readFileSync(path.join(repository, 'scripts/wod-assets/skillgrid.js'), 'utf8'), context);
const values = vector => Array.from({length:vector.size()}, (_, i) => vector.get(i));
const names = enumeration => Object.fromEntries(Object.entries(enumeration)
  .filter(([,value]) => value && typeof value.value === 'number').map(([name,value]) => [value.value,name]));
(async () => {
  const module = await context.createModule();
  const tiles = names(module.EGridTile), quarters = names(module.EQuarter), categories = names(module.EMediumPerkCategory);
  let slots = 0, revelations = 0;
  for (const vocation of ['Knight','Paladin','Sorcerer','Druid','Monk']) {
    const planner = new module.SkillwheelPlanner(module.EActiveVocation[vocation]);
    const reference = raw.vocations[vocation.toLowerCase()];
    const actualSlots = values(planner.getSkillParameters());
    assert.equal(actualSlots.length, 36);
    for (let i = 0; i < actualSlots.length; i++) {
      const actual = actualSlots[i], captured = reference.slots[i];
      assert.equal(tiles[actual.id.value], captured.tile);
      assert.equal(actual.maxSkillPoints, captured.capacity);
      assert.equal(actual.smallPerkId, captured.dedication.id);
      assert.equal(actual.smallPerkPrimaryEffectIncrease, captured.dedication.effects_per_point[0].raw_value);
      if (actual.smallPerkHasSecondaryEffect)
        assert.equal(actual.smallPerkSecondaryEffectIncrease, captured.dedication.effects_per_point[1].raw_value);
      assert.deepEqual(strings.SmallPerkInfos[actual.smallPerkId], captured.dedication.info);
      assert.equal(actual.mediumPerkId, captured.conviction.id);
      assert.equal(actual.mediumPerkValue, captured.conviction.value);
      assert.equal(categories[actual.mediumPerkCategory.value], captured.conviction.category);
      assert.deepEqual(strings.MediumPerkInfos[actual.mediumPerkId], captured.conviction.info);
      slots++;
    }
    for (const [i, actual] of values(planner.getCornerParameters()).entries()) {
      const captured = reference.revelations[i];
      assert.equal(quarters[actual.id.value], captured.quarter);
      assert.equal(actual.largePerkId, captured.id);
      assert.deepEqual(strings.LargePerkInfos[actual.largePerkId], captured.info);
      revelations++;
    }
    for (const [method, field] of [['getAvailableBasicModsPos1','basic_mods_position_1'],['getAvailableBasicModsPos2','basic_mods_position_2']])
      assert.deepEqual(values(planner[method]()).map(x => x.id), reference.gems[field].map(x => x.id));
    assert.deepEqual(values(planner.getAvailableSupremeMods()), reference.gems.supreme_mod_ids);
    for (let round = 0; round < 10; round++)
      for (const slot of reference.slots) planner.fillSkill(module.EGridTile[slot.tile]);
    assert.equal(planner.getSpentSkillPoints(), 4000);
    const fullSlots = values(planner.getSkillParameters());
    for (let i = 0; i < fullSlots.length; i++) {
      assert.equal(fullSlots[i].smallPerkPrimaryEffectValue, reference.slots[i].dedication.primary_value_full_raw);
      assert.equal(fullSlots[i].smallPerkSecondaryEffectValue, reference.slots[i].dedication.secondary_value_full_raw);
    }
    planner.delete();
  }
  assert.deepEqual(strings.BasicModConfig, raw.gem_library.basic_mod_config);
  assert.deepEqual(strings.BasicModEffectInfos, raw.gem_library.basic_mod_effects);
  assert.deepEqual(strings.SupremeModInfos, raw.gem_library.supreme_mods);
  assert.deepEqual(strings.VesselInfos, raw.gem_library.vessels);
  console.log(JSON.stringify({state:'PASS',source_commit:actualSha,slots,revelations,
    gem_catalogues:'all entries and vocation lists',full_point_states:5,live_site_verified:false}));
})().catch(error => { console.error(error); process.exitCode = 1; });
