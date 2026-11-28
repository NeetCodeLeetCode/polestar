'use strict';
// Fastest XP (addon/Polestar/Core/FastestXP.lua) writes settings other modules own, by key. When the maintainer
// renames or removes one of them, this test names it, so the fix is one line in FX.LEVERS.
// Against the real addon's modules and the fakes of tests/lua/stub_ui.lua.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { runAddon } = require('./helpers/lua');

const ALL = ['Core/Core.xml', 'Theme/Theme.xml', 'Nav/Nav.xml', 'Quests/Quests.xml', 'Gear/Gear.xml', 'Maps/Maps.xml',
	'QoL/QoL.xml', 'Lookup/Lookup.xml', 'Dungeons/Dungeons.xml', 'UI/UI.xml', 'Rares/Rares.xml', 'Alts/Alts.xml'];

// An empty Lua table reads back as {}.
const list = (v) => (Array.isArray(v) ? v : Object.values(v || {}));

let cached;
function survey() {
	if (cached) return cached;
	cached = runAddon({
		stubs: ['stub_world.lua', 'stub_ui.lua'],
		saved: { Polestar: 'PolestarDB = { settings = {} }' },
		addons: [{ name: 'Polestar', files: ALL }],
		script: `uistub.install()
			stub.login()
			local FX = Polestar.FastestXP
			local missing, wrongType, repeated, seen = {}, {}, {}, {}
			for _, lv in ipairs(FX.LEVERS) do
				local default = Polestar.GetBaseDefault(lv.key)
				if default == nil then missing[#missing + 1] = lv.key
				elseif type(default) ~= type(lv.value) then wrongType[#wrongType + 1] = lv.key end
				if seen[lv.key] then repeated[#repeated + 1] = lv.key end
				seen[lv.key] = true
			end
			local values = 0
			for _ in pairs(FX.Values()) do values = values + 1 end
			emit({ levers = #FX.LEVERS, values = values, missing = missing, wrongType = wrongType, repeated = repeated,
				row = Polestar.UI.QuickSettings.ROWS[1].key, button = Polestar.UI.QuickSettings.ROWS[2].id })`,
	});
	return cached;
}

test('every setting the mode writes is registered by a module, with a value of its own type', () => {
	const out = survey().emitted[0];
	assert.deepEqual(list(out.missing), [], 'a lever names a setting no module registers: fix FX.LEVERS');
	assert.deepEqual(list(out.wrongType), [], 'a lever value is not of its setting\'s type');
	assert.deepEqual(list(out.repeated), []);
});

test('with every module loaded, every lever is in force', () => {
	const out = survey().emitted[0];
	assert.equal(out.values, out.levers);
});

test('Quick settings lists the switch first and the arrow\'s Recalibrate under it', () => {
	assert.equal(survey().emitted[0].row, 'behavior.fastestXp');
	assert.equal(survey().emitted[0].button, 'recalibrate');
});
