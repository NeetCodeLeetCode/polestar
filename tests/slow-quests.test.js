'use strict';
// "Skip slow quests" (behavior.skipSlow, off by default): the hand-written list data/codex/slow-quests.json, checked
// against forever-db and written to Core/SlowQuestList.lua by `slowquests build`. The build judges no kill count,
// drop chance or time. The addon side (Own.slowSkipped) is tested in forge-addontest's engine-slow-quests.rs.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const SQ = require('../tools/commands/slowquests');
const { loadDb, isClassMask } = require('../tools/lib/slow-quests');

const ROOT = path.join(__dirname, '..');
const LIST = JSON.parse(fs.readFileSync(path.join(ROOT, 'data', 'codex', 'slow-quests.json'), 'utf8'));
const db = loadDb();
const classQuests = SQ.classQuestSet(path.join(ROOT, 'guides'));
const clone = () => JSON.parse(JSON.stringify(LIST));
const row = (extra) => ({ quests: [701], name: 'x', code: 'low-drop', why: 'why', reviewed: '2026-11-28', sources: [{ kind: 'forum', date: '2026-11-23', note: 'a' }, { kind: 'guide', date: '2026-10-26', note: 'b' }], ...extra });

test('the committed list is the one data/codex/slow-quests.json builds (slowquests check)', () => {
	const lua = SQ.toLua(SQ.buildSlowList(LIST, db, classQuests));
	assert.equal(fs.readFileSync(path.join(ROOT, 'addon', 'Polestar', 'Core', 'SlowQuestList.lua'), 'utf8'), lua, 'run node tools/polestar.js slowquests build');
});

test('every slow row has 2 sources, a review date and a reason, and is no class quest', () => {
	assert.ok(LIST.slow.length > 0);
	for (const r of LIST.slow) {
		assert.ok(r.sources.length >= SQ.MIN_SOURCES, `${r.name}: sources`);
		assert.match(r.reviewed, /^\d{4}-\d{2}-\d{2}$/, `${r.name}: reviewed`);
		assert.ok(r.why.trim(), `${r.name}: why`);
		for (const q of r.quests) {
			assert.ok(!classQuests.has(q), `${r.name}: ${q} is in a Class guide`);
			assert.ok(!isClassMask(db.quests.get(q)), `${r.name}: ${q} has a class mask`);
		}
	}
});

test('a row the list builds is refused when a quest is unknown, doubled, in both lists, or the row is thin', () => {
	const bad = (mutate, re) => {
		const l = clone();
		mutate(l);
		assert.throws(() => SQ.buildSlowList(l, db, classQuests), re);
	};
	bad((l) => l.slow.push(row({ quests: [99999999] })), /not in forever-db/);
	bad((l) => l.slow.push(row({ quests: [701] })), /also in/);
	bad((l) => l.keep.push({ quests: [701], name: 'k', why: 'w', reviewed: '2026-11-28' }), /also in/);
	bad((l) => (l.slow[0].sources = [l.slow[0].sources[0]]), /at least 2 needed/);
	bad((l) => (l.slow[0].code = 'whatever'), /code "whatever"/);
	bad((l) => (l.slow[0].reviewed = 'last week'), /reviewed must be/);
});

test('a class quest is never a row', () => {
	// A quest open to some classes only, from the quest database's mask.
	let classQuest;
	for (const [q, info] of db.quests) if (isClassMask(info)) { classQuest = q; break; }
	assert.ok(classQuest, 'forever-db holds a class quest');
	const l = clone();
	l.slow = [row({ quests: [classQuest] })];
	assert.throws(() => SQ.buildSlowList(l, db, classQuests), /class quest/);
	// The same through a Class guide's quest.
	const l2 = clone();
	l2.slow = [row({ quests: [701] })];
	assert.throws(() => SQ.buildSlowList(l2, db, new Set([701])), /class quest/);
});

test('the list judges no kill count, drop chance or time: a row stays whatever the data says', () => {
	// Troll Witchery (205) pays 3,150 XP for 4 items at an 80 percent drop: fast by any score. As a row it is still on the list.
	for (const code of ['low-drop', 'slow-respawn', 'slow-xp']) {
		const l = clone();
		l.slow = [row({ quests: [205], code })];
		l.keep = l.keep.filter((k) => !k.quests.includes(205));
		assert.deepEqual(SQ.buildSlowList(l, db, classQuests).codes, { [code]: [205] });
	}
	// And the list names no score, level or margin setting.
	for (const key of ['margin', 'level']) assert.equal(key in LIST, false, `${key} is gone`);
	const lua = fs.readFileSync(path.join(ROOT, 'addon', 'Polestar', 'Core', 'SlowQuestList.lua'), 'utf8');
	assert.equal(/SlowAt|outlevelled/.test(lua), false, 'the build writes no level table');
});

test('the report names each row with its age and when a review is due', () => {
	const out = require('node:child_process').execFileSync(process.execPath, [path.join(ROOT, 'tools', 'polestar.js'), 'slowquests', 'report', '--today', '2026-12-30'], { encoding: 'utf8' });
	assert.match(out, /Guile of the Raptor \(701 702 847\) \| low-drop \| 32 days \(review due\)/);
	assert.match(out, /keep Consumed by Hatred/);
});
