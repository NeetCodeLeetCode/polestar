'use strict';
// "Skip slow quests" (behavior.skipSlow, off by default): the hand-written list data/codex/slow-quests.json, scored
// from forever-db, written to Core/SlowQuestList.lua by `slowquests build`. The addon side (Own.slowSkipped) is
// tested in forge-addontest's engine-slow-quests.rs.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const SQ = require('../tools/commands/slowquests');
const { loadDb, bandMedians, chainScore, questMinutes, isClassMask } = require('../tools/lib/slow-quests');

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

test('a class quest is never a row, and the score leaves class quests out of the medians', () => {
	// A quest open to some classes only, from the quest database's mask.
	let classQuest;
	for (const [q, info] of db.quests) if (isClassMask(info) && info.xp && questMinutes(db, q)) { classQuest = q; break; }
	assert.ok(classQuest, 'forever-db holds a scored class quest');
	const l = clone();
	l.slow = [row({ quests: [classQuest] })];
	assert.throws(() => SQ.buildSlowList(l, db, classQuests), /class quest/);
	// The same through a Class guide's quest.
	const l2 = clone();
	l2.slow = [row({ quests: [701] })];
	assert.throws(() => SQ.buildSlowList(l2, db, new Set([701])), /class quest/);
	// A made-up database: 3 quests at 50, 60 and 70 XP a minute, and a class quest at 5000. The class quest is no part of the median.
	const mk = (id, xp, classes = 0) => [id, { level: 10, xp, classes }];
	const fake = {
		quests: new Map([mk(1, 100), mk(2, 120), mk(3, 140), mk(4, 10000, 2)]),
		objectives: new Map([1, 2, 3, 4].map((q) => [q, [{ quest: q, kind: 'kill', count: 4 }]])),
		chance: new Map(),
	};
	assert.equal(questMinutes(fake, 1), 2);
	assert.equal([...bandMedians(fake).values()][0], 60, 'the class mask quest is left out');
	assert.equal([...bandMedians({ ...fake, quests: new Map([mk(1, 100), mk(2, 120), mk(3, 140), mk(4, 10000)]) }, new Set([4])).values()][0], 60, 'so is a quest of a Class guide');
	assert.equal(chainScore(fake, bandMedians(fake), [1]).ratio, 0.83);
});

test('the score overrules a low-drop row that the data says is fast, and no other code', () => {
	// Troll Witchery (205): 3,150 XP for 4 items at an 80 percent drop, 1.67 times its band's median.
	const fast = (code) => {
		const l = clone();
		l.slow = [row({ quests: [205], code })];
		l.keep = l.keep.filter((k) => !k.quests.includes(205));
		return SQ.buildSlowList(l, db, classQuests);
	};
	const low = fast('low-drop');
	assert.equal(low.rows[0].overruled, true);
	assert.deepEqual(low.codes, {}, 'it is not in the addon');
	assert.ok(low.rows[0].score.ratio >= 1);
	const respawn = fast('slow-respawn');
	assert.equal(respawn.rows[0].overruled, false, 'a respawn is not in the data');
	assert.deepEqual(respawn.codes, { 'slow-respawn': [205] });
});

test('a slow row of the list stays slow against the data', () => {
	const built = SQ.buildSlowList(LIST, db, classQuests);
	for (const r of built.rows) assert.equal(r.overruled, false, `${r.name} is overruled: ${JSON.stringify(r.score)}`);
	const medians = bandMedians(db, classQuests);
	for (const k of LIST.keep) assert.ok(chainScore(db, medians, k.quests), `${k.name} has a score`);
});

// The quests that turn slow as the character outlevels them (the list's `level`): from the quest database alone.
test('a quest the character outlevels turns slow from the level its pay falls far under its time, and never before the reward is cut', () => {
	const { slowFromLevels, levelMedians } = require('../tools/lib/slow-quests');
	// A level 14 quest asking 12 kills of level 12 mobs, and ten quests of level 20 asking 10 kills of level 20 mobs.
	const quests = new Map([[1, { level: 14, xp: 700, classes: 0 }]]);
	const objectives = new Map([[1, [{ quest: 1, kind: 'kill', target: 100, count: 12 }]]]);
	for (let q = 2; q < 12; q++) {
		quests.set(q, { level: 20, xp: 1500, classes: 0 });
		objectives.set(q, [{ quest: q, kind: 'kill', target: 101, count: 10 }]);
	}
	const fake = { quests, objectives, chance: new Map(), npcs: new Map([[100, { level: 12, elite: false }], [101, { level: 20, elite: false }]]) };
	const medians = levelMedians(fake);
	const from = slowFromLevels(fake, medians, 0.3, 6).get(1);
	assert.equal(from, 20, 'level 20: 6 levels over it, 80 percent of the reward and grey mobs');
	// Never before the reward is cut: the data alone calls it slow earlier (its mobs are grey from 19), and fromGap holds it.
	assert.ok(slowFromLevels(fake, medians, 0.3, 1).get(1) < 20, 'the rate alone is under the line sooner');
	assert.equal(slowFromLevels(fake, medians, 0.3, 7).get(1), 21, 'fromGap is the least level gap');
	// A class quest, and a quest the caller skips (a row, a keep), are never listed.
	const classy = { ...fake, quests: new Map([...quests, [1, { level: 14, xp: 700, classes: 2 }]]) };
	assert.equal(slowFromLevels(classy, levelMedians(classy), 0.3, 6).has(1), false);
	assert.equal(slowFromLevels(fake, medians, 0.3, 6, new Set([1])).has(1), false);
});

test('the list builds the outlevelled quests: none of a row, a keep row or a class quest, and each turns slow by the data', () => {
	const built = SQ.buildSlowList(LIST, db, classQuests);
	assert.ok(built.byLevel.size > 100, 'the data marks quests slow from some level');
	const rowQuests = new Set([...LIST.slow, ...LIST.keep].flatMap((r) => r.quests));
	const medians = require('../tools/lib/slow-quests').levelMedians(db, classQuests);
	const { barRate } = require('../tools/lib/slow-quests');
	for (const [q, level] of built.byLevel) {
		assert.ok(!rowQuests.has(q) && !classQuests.has(q) && !isClassMask(db.quests.get(q)), `${q} is a row, keep or class quest`);
		assert.ok(level - db.quests.get(q).level >= LIST.level.fromGap, `${q} turns slow ${level - db.quests.get(q).level} levels over it`);
		assert.ok(barRate(db, q, level) / medians.get(level) < LIST.level.threshold, `${q} at ${level}`);
	}
	// The user's case: a level 14 quest of level 12 mobs, at 20.
	assert.equal(built.byLevel.get(887), 20);
});

test('a bad level setting is refused', () => {
	for (const level of [undefined, { threshold: 2, fromGap: 6 }, { threshold: 0.3, fromGap: 0 }]) {
		const l = clone();
		l.level = level;
		assert.throws(() => SQ.buildSlowList(l, db, classQuests), /level\./);
	}
});
