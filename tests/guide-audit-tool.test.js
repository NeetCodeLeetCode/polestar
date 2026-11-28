'use strict';
// The offline guide audit (tools/guide-audit.js, tools/lib/guide-audit/): its checks on made-up guides and quest
// data, and the order checks held at zero on the shipped guides.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { runAudit } = require('../tools/lib/guide-audit');
const { loadRef } = require('../tools/lib/guide-audit/refdata');
const { loadCompiled } = require('../tools/lib/guide-audit/compiled');

const ALLIANCE = 4294967373;   // Human, Dwarf, Night Elf, Gnome, Skyborne
const HORDE = 8589934770;

// Quest rows { id, title, level, min_level, races, side, xp } and the tables keyed the way the checks read them.
function refOf(quests, { starts = {}, ends = {}, prereqs = {} } = {}) {
	const list = (o) => new Map(Object.entries(o).map(([k, v]) => [Number(k), v]));
	return {
		quests: new Map(quests.map((q) => [q.id, { level: 5, min_level: 1, races: ALLIANCE, side: 1, xp: 100, classes: 0, ...q }])),
		npcs: new Map([[10, { id: 10, name: 'Giver' }], [11, { id: 11, name: 'Other' }]]), objects: new Map(), frames: new Map(),
		spawns: { npc: new Map(), object: new Map() },
		starts: list(starts), ends: list(ends), prereqs: list(prereqs), objectives: new Map(), mechanics: new Map(), followers: new Map(),
	};
}

const goal = (t, quest, extra = {}) => ({ t, quest, title: `Quest ${quest}`, npc: 10, ...extra });
const guideOf = (steps, extra = {}) => new Map([['g/start', { id: 'g/start', category: 'Leveling', faction: 'Alliance', races: ['Human'], minLevel: 1, maxLevel: 10, steps: steps.map((goals) => ({ goals })), ...extra }]]);
const checks = (r) => r.findings.map((f) => f.check);

test('a prerequisite handed in later is an error', () => {
	const ref = refOf([{ id: 1 }, { id: 2 }]);
	const guides = guideOf([[goal('accept', 2, { pre: [1] })], [goal('accept', 1), goal('turnin', 1)], [goal('turnin', 2)]]);
	const r = runAudit({ ref, guides });
	assert.ok(checks(r).includes('prereq-later'));
	assert.equal(r.findings.find((f) => f.check === 'prereq-later').severity, 'error');
});

test('a hand-in before its accept is an error', () => {
	const ref = refOf([{ id: 1 }]);
	const guides = guideOf([[goal('turnin', 1)], [goal('accept', 1)]]);
	assert.ok(checks(runAudit({ ref, guides })).includes('turnin-before-accept'));
});

test('a quest taken again after its hand-in is an error', () => {
	const ref = refOf([{ id: 1 }]);
	const guides = guideOf([[goal('accept', 1), goal('turnin', 1)], [goal('accept', 1)]]);
	assert.ok(checks(runAudit({ ref, guides })).includes('accept-after-handin'));
});

test('a race the quest is closed to is an error, a goal narrowed to another race is not', () => {
	const ref = refOf([{ id: 1, races: HORDE, side: 2 }, { id: 2, races: HORDE, side: 2 }]);
	const guides = guideOf([[goal('accept', 1), goal('turnin', 1)], [goal('accept', 2, { only: { races: ['Orc'] } })]]);
	const found = runAudit({ ref, guides }).findings.filter((f) => f.check === 'race-closed' || f.check === 'wrong-side');
	assert.ok(found.some((f) => f.quest === 1));
	assert.ok(!found.some((f) => f.quest === 2 && f.check === 'race-closed'));
});

test('a giver the data does not list is a warning, a listed one is not', () => {
	const ref = refOf([{ id: 1 }, { id: 2 }], { starts: { 1: [{ kind: 'npc', id: 11 }], 2: [{ kind: 'npc', id: 10 }] } });
	const guides = guideOf([[goal('accept', 1), goal('turnin', 1)], [goal('accept', 2), goal('turnin', 2)]]);
	const found = runAudit({ ref, guides }).findings.filter((f) => f.check === 'giver-differs');
	assert.deepEqual(found.map((f) => f.quest), [1]);
});

test('a quest above the guide level is locked, a clean route has no errors', () => {
	const ref = refOf([{ id: 1, min_level: 20 }, { id: 2 }]);
	const locked = runAudit({ ref, guides: guideOf([[goal('accept', 1), goal('turnin', 1)]]) });
	assert.ok(checks(locked).includes('level-locked'));
	const clean = runAudit({ ref, guides: guideOf([[goal('accept', 2), goal('turnin', 2)]]) });
	assert.deepEqual(clean.findings.filter((f) => f.severity === 'error'), []);
});

test('corrections change what the audit reads: a race fix closes nothing it opened', () => {
	const ref = loadRef();
	const raw = loadRef(undefined, { corrections: false });
	// Creature of the Void (1473): the published rows say Undead only, the correction opens it to Orc, Undead and Troll.
	assert.equal(raw.quests.get(1473).races, 16);
	assert.equal(ref.quests.get(1473).races, 146);
});

// What no shipped guide may do any more. Each of these is zero today; a new one is a step a player cannot finish.
test('an arrow point the maintainers documented is not called a moved NPC', () => {
	const ref = loadRef();
	// Gann Stonespire (3341) walks the road; a guide fix names him by id. Eye of Kilrogg's point is a hand-set place.
	const roaming = guideOf([[goal('accept', 843, { npc: 3341, map: 1413, x: 46.09, y: 81.28 })]]);
	assert.ok(!checks(runAudit({ ref, guides: roaming, only: ['goals'] })).includes('npc-elsewhere'));
	const placed = guideOf([[goal('turnin', 98252, { npc: 271181, map: 1440, x: 38.2, y: 58.1 })]]);
	assert.ok(!checks(runAudit({ ref, guides: placed, only: ['goals'] })).includes('npc-elsewhere'));
});

test('the shipped guides keep the route-order and data checks at zero', () => {
	const r = runAudit({ ref: loadRef(), guides: loadCompiled() });
	const never = ['prereq-later', 'data-prereq-later', 'turnin-before-accept', 'level-locked', 'wrong-side', 'objective-differs', 'count-differs', 'text-unknown-id'];
	const hits = r.findings.filter((f) => never.includes(f.check)).map((f) => `${f.check} ${f.guide} step ${f.step}: ${f.message}`);
	assert.deepEqual(hits, []);
});
