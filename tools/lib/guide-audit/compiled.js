'use strict';
// The compiled guides as the player's addon holds them (addon/Polestar_Guides*/Guides.lua, what `forge guide compile`
// writes): every step with the quest, giver, place and level the build resolved. The audit reads these, not the
// .guide files, because they are what a player plays, and the build's data is what drifts from forever-db.

const fs = require('node:fs');
const path = require('node:path');
const { ROOT } = require('../paths');
const { luaToJson } = require('../lua');

const PACKS = ['Polestar_Guides', 'Polestar_Guides_Alliance', 'Polestar_Guides_Horde'];

// Collects the guides a Guides.lua registers; a guide's steps are a table or a string (`stepsSrc`) to run.
// Any other Polestar call a pack makes (RetireGuides, ...) is ignored.
const PRELUDE = `AUDIT_GUIDES = {}
Polestar = setmetatable({ RegisterGuide = function(g) AUDIT_GUIDES[#AUDIT_GUIDES + 1] = g end }, { __index = function() return function() end end })`;
const EXPR = `
local out = {}
for _, g in ipairs(AUDIT_GUIDES) do
	if g.steps == nil and type(g.stepsSrc) == "string" then
		local f = loadstring(g.stepsSrc)
		if f then g.steps = f() end
		g.stepsSrc = nil
	end
	out[#out + 1] = g
end
return out`;

function loadCompiled(addonDir = path.join(ROOT, 'addon')) {
	const guides = [];
	for (const pack of PACKS) {
		const file = path.join(addonDir, pack, 'Guides.lua');
		if (!fs.existsSync(file)) continue;
		for (const g of luaToJson({ files: [file], prelude: PRELUDE, expr: EXPR })) {
			if (!g.steps) g.steps = [];
			g.pack = pack;
			guides.push(g);
		}
	}
	return new Map(guides.map((g) => [g.id, g]));
}

module.exports = { loadCompiled };
