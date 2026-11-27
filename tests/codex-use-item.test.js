'use strict';
// A kill the mechanics do by using an item on that NPC keeps useItem on the
// Codex objective (tools/lib/codex/build.js). Lookup reads it and says Use.
// The same kill without an item, a tame, and an item for a different NPC stay
// as they were. Not one quest id (#2468).

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { mergeCodex } = require('../tools/lib/codex/build');
const { emitPack } = require('../tools/lib/codex/pack');
const fs = require('node:fs');
const path = require('node:path');

function inputs(mechanics) {
	const quest = (id, extra = {}) => ({ id, title: 'Q' + id, start: { npc: [] }, end: { npc: [] }, objectives: [{ type: 'kill', id: extra.npc || 1, count: 1, text: 'Mob' }], ...extra });
	return {
		forever: {
			zones: {},
			factions: {},
			mapFrames: { era: {}, forever: {} },
			meta: {},
			npcs: {},
			objects: {},
			items: {},
			instances: {},
			dungeons: null,
			quests: {
				88001: quest(88001, { npc: 6498 }),
				88002: quest(88002, { npc: 1 }),
				88003: quest(88003, { npc: 1, objectiveText: 'Use the Taming Rod to tame a bear' }),
				88004: quest(88004, { npc: 2 }),
			},
		},
		atlasloot: null,
		rarescanner: null,
		client: null,
		curated: [],
		entrances: [],
		mechanics,
	};
}

const mechanics = {
	quests: {
		88001: { kinds: [{ kind: 'use-item-on-mob', confidence: 'high' }], params: [{ kind: 'use-item-on-mob', item: 22432, npcs: [6498] }] },
		88002: { kinds: [{ kind: 'use-item-on-mob', confidence: 'high' }], params: [{ kind: 'use-item-on-mob', item: 9, npcs: [99] }] },
		88003: { kinds: [{ kind: 'use-item-on-mob', confidence: 'high' }], params: [{ kind: 'use-item-on-mob', item: 9, npcs: [1] }] },
		88004: { kinds: [{ kind: 'use-item-on-mob', confidence: 'high' }], params: [] },
	},
};

test('a kill done with an item keeps useItem; a plain kill, a tame and a kind with no item do not (#2468)', () => {
	const withItem = mergeCodex(inputs(mechanics)).quests;
	assert.equal(withItem[88001].objectives[0].useItem, 22432);
	assert.equal(withItem[88001].objectives[0].tame, undefined);
	assert.equal(withItem[88002].objectives[0].useItem, undefined);
	assert.equal(withItem[88003].objectives[0].tame, true);
	assert.equal(withItem[88003].objectives[0].useItem, undefined);
	assert.equal(withItem[88004].objectives[0].useItem, undefined);
	const plain = mergeCodex(inputs(null)).quests[88001].objectives[0];
	assert.equal(plain.useItem, undefined);
	assert.equal(plain.type, 'kill');
});

// The pack carries it to the addon (Lookup/Codex.lua reads `a<item>` after the objective).
test('the pack writes useItem after its objective, and nothing for a plain kill (#2468)', () => {
	const quests = mergeCodex(inputs(mechanics)).quests;
	const text = Object.values(emitPack({ meta: { schema: 1 }, items: {}, npcs: {}, objects: {}, quests: { 88001: quests[88001], 88002: quests[88002] }, instances: {}, zones: {}, factions: {}, spells: {}, frames: {} }, { version: '0', build: 'test' })).join('\n');
	assert.match(text, /k6498,1,Mob;a22432\b/);
	assert.doesNotMatch(text, /k1,1,Mob;a/);
});

// #4346: All Along the Watchtowers marks each of Andorhal's four towers with the Beacon Torch (12815); the game
// credits a mark as a kill of the tower, so the Lookup guide worded each one "Kill Andorhal Tower Three". The public
// mechanics say the kills are a use of the torch, for both sides' quest (5097 Alliance, 5098 Horde).
test("the public mechanics make All Along the Watchtowers' tower kills a use of the Beacon Torch (#4346)", () => {
	const real = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'data', 'mechanics-public.json'), 'utf8'));
	const towers = [10902, 10903, 10904, 10905];
	const tower = (id) => ({ type: 'kill', id, count: 1, text: 'Tower' });
	const base = inputs(real);
	for (const id of [5097, 5098]) {
		base.forever.quests[id] = { id, title: 'All Along the Watchtowers', start: { npc: [] }, end: { npc: [] }, objectives: towers.map(tower) };
	}
	const quests = mergeCodex(base).quests;
	for (const id of [5097, 5098]) {
		assert.deepEqual(quests[id].objectives.map((o) => o.id), towers);
		assert.deepEqual(quests[id].objectives.map((o) => o.useItem), towers.map(() => 12815), `quest ${id}: each tower is a use of the Beacon Torch`);
	}
});
