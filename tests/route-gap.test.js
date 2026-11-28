'use strict';
// tools/lib/flow/gap.js, the route gap test: the exact search, the local search against it, the rules a best order
// keeps, the live reorder's window and the walking measures.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const gap = require('../tools/lib/flow/gap');

const node = (i, t, quest, x, y, extra = {}) => ({ i, gid: 'g', step: i, mode: 'walk', pos: [1, x, y], eff: 10, goal: { t, quest, ...(extra.goal || {}) }, ...extra });
const seg = (nodes, start = [1, 0, 0], end = null, carry = []) => ({ start, end, nodes, carry: new Set(carry) });

// Three quests laid out along a line: the guide walks out and back for each of them.
const line = () => seg([
	node(0, 'accept', 1, 0, 0), node(1, 'kill', 1, 1000, 0), node(2, 'turnin', 1, 0, 0),
	node(3, 'accept', 2, 0, 0), node(4, 'kill', 2, 1010, 0), node(5, 'turnin', 2, 0, 0),
	node(6, 'accept', 3, 0, 0), node(7, 'kill', 3, 1020, 0), node(8, 'turnin', 3, 0, 0),
]);

test('the best order of goals keeps the rules and walks less than the guide', () => {
	const s = line();
	const r = gap.gapOf(s, {}, { ms: 100 });
	assert.ok(r.goals.yd < r.guide.yd, `${r.goals.yd} < ${r.guide.yd}`);
	assert.ok(r.goals.yd <= 2040 + 1e-6, 'one trip out and back');
	const P = gap.problemOf(s, {}, 'goal');
	const best = gap.search(P, P.items.map((_, k) => k), gap.rngOf(1), 100);
	assert.ok(gap.feasible(P, best));
	const at = (t, q) => best.findIndex((k) => P.nodes[P.items[k].nodes[0]].goal.t === t && P.nodes[P.items[k].nodes[0]].goal.quest === q);
	for (const q of [1, 2, 3]) {
		assert.ok(at('accept', q) < at('kill', q) && at('kill', q) < at('turnin', q), `quest ${q} in order`);
	}
});

test('the search finds the exact best on a small segment', () => {
	const P = gap.problemOf(line(), {}, 'goal');
	const ex = gap.exact(P);
	const hs = gap.search(P, P.items.map((_, k) => k), gap.rngOf(3), 100);
	assert.ok(gap.feasible(P, ex));
	assert.ok(Math.abs(gap.costOf(P, ex) - gap.costOf(P, hs)) < 1e-6);
});

test('a follow-up is accepted after the hand-in that opens it', () => {
	const s = seg([node(0, 'accept', 1, 0, 0), node(1, 'turnin', 1, 500, 0), node(2, 'accept', 2, 500, 0), node(3, 'kill', 2, 0, 0)]);
	const r = gap.gapOf(s, { 2: { pre: [1] } }, { ms: 50 });
	assert.equal(Math.round(r.goals.yd), Math.round(r.guide.yd), 'no order is shorter');
});

test('a goal is not moved before the level its quest asks for', () => {
	const gated = node(2, 'kill', 9, 5000, 0, { eff: 10, goal: { t: 'kill', quest: 9, minLevel: 10 } });
	const s = seg([node(0, 'accept', 1, 0, 0, { eff: 5 }), node(1, 'kill', 1, 100, 0, { eff: 5 }), gated]);
	s.nodes[1].eff = 5;
	const P = gap.problemOf(s, { 9: {} }, 'goal');
	assert.equal(P.items[2].min, 2, 'the goal waits for the level the walk reached at its own place');
});

test('the live reorder takes a shorter first step only when it saves enough', () => {
	const far = seg([node(0, 'kill', 1, 0, 600), node(1, 'kill', 2, 20, 0), node(2, 'kill', 3, 40, 600)]);
	const P = gap.problemOf(far, {}, 'goal');
	assert.equal(gap.windowed(P)[0], 1, 'the near goal first: it saves 1140 yards');
	const close = seg([node(0, 'kill', 1, 0, 100), node(1, 'kill', 2, 0, 90)]);
	assert.deepEqual(gap.windowed(gap.problemOf(close, {}, 'goal')), [0, 1], 'ten yards is not worth a change');
});

test('walking measures: yards, areas entered again and there-and-back trips', () => {
	const seq = [node(0, 'kill', 1, 0, 0), node(1, 'kill', 2, 1000, 0), node(2, 'kill', 3, 10, 0), node(3, 'kill', 4, 2000, 0)];
	const f = gap.flowOf(seq, [1, 0, 0], null);
	assert.equal(Math.round(f.yd), 1000 + 990 + 1990);
	assert.equal(f.reentries, 1);
	assert.equal(f.trips, 1);
});

test('segments end at a flight and keep a timed quest where it is', () => {
	const fly = { i: 3, gid: 'g', step: 3, mode: 'fly', pos: [1, 100, 0], goal: { t: 'fly' } };
	const entries = [node(0, 'accept', 1, 0, 0), node(1, 'kill', 1, 50, 0), node(2, 'turnin', 7, 60, 0), fly, node(4, 'kill', 2, 200, 0)];
	const segs = gap.segmentsOf(entries, new Set([7]));
	assert.deepEqual(segs.map((x) => x.nodes.length), [2, 1]);
	assert.deepEqual(segs[1].start, [1, 100, 0]);
});
