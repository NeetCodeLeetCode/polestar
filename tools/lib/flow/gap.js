'use strict';
// The route gap test: how far a guide's order, and the addon's live step reorder, are from the shortest walking
// order that keeps the rules. The inputs are a flow-audit walk's entries (tools/lib/flow/walk.js). The test
//
//   1. cuts the entries into segments: runs of movable goals (accepts, objectives, hand-ins) that lie between fixed
//      ones (flights, teleports, level goals, trainers, vendors, timed or escort quests), on one continent;
//   2. orders every segment four ways: the guide's order, the live reorder (Core/StepOrder.lua: the next 8 steps
//      ordered exactly, one step taken, then planned again), the best order of whole steps, and the best order of
//      single goals;
//   3. measures the straight-line yards walked, the areas entered again and the there-and-back trips of each order.
//
// The rules every order keeps: a quest is accepted before its objectives and handed in after them; a follow-up is
// accepted after the hand-in that opens it; the quest log stays under its cap; a goal is not done before the guide
// walk had the level its quest asks for. The best orders come from a local search (moves of one to three goals,
// restarts); on segments small enough for the exact search, the test also checks the local search finds the exact best.

const { OBJ_TYPES } = require('./walk');

const WINDOW = 8;                 // Core/StepOrder.lua MAX
const TAKE_YD = 20 * 7;           // Core/Engine.lua ORDER_TAKE (20 s) at HERE_SPEED (7 yd/s): the least saving that reorders
const LOG_CAP = 37;               // Own.logRoom: the log's 40 less LOG_SPARE 2, one free
const CLUSTER_YD = 100;           // goals this close are one area
const EXACT_MAX = 14;             // segments up to this many items also get the exact search
const MOVABLE = new Set(['accept', 'turnin', ...OBJ_TYPES]);

const dist = (a, b) => (a && b && a[0] === b[0] ? Math.hypot(a[1] - b[1], a[2] - b[2]) : 0);

// A small seeded generator, so a run repeats.
function rngOf(seed) {
	let s = (seed >>> 0) || 1;
	return () => {
		s ^= s << 13; s >>>= 0;
		s ^= s >>> 17;
		s ^= s << 5; s >>>= 0;
		return s / 4294967296;
	};
}

const isLocked = (e, locked) => e.goal.quest != null && locked.has(e.goal.quest);

// Segments of movable nodes: [{ start, end, nodes, carry }]. start and end are the positions of the fixed goals around
// it (null at the walk's ends); carry is the quests open when it starts.
function segmentsOf(entries, locked = new Set()) {
	const out = [];
	let cur = null;
	let last = null;
	const open = new Set();
	const close = (endPos) => {
		if (cur && cur.nodes.length) {
			cur.end = endPos;
			out.push(cur);
		}
		cur = null;
	};
	for (const e of entries) {
		const t = e.goal.t;
		const movable = e.pos && MOVABLE.has(t) && (e.mode === 'walk' || e.mode === undefined || e.mode === null) && !isLocked(e, locked) && !(e.goal.escort);
		if (movable) {
			if (cur && cur.nodes.length && cur.nodes[cur.nodes.length - 1].pos[0] !== e.pos[0]) close(e.pos);
			if (!cur) cur = { start: last, end: null, nodes: [], carry: new Set(open) };
			cur.nodes.push(e);
		} else if (e.pos || e.mode === 'tele' || e.mode === 'fly') {
			// a fixed goal with a place ends the run; one without (a note) does not
			if (e.pos) close(e.pos);
			if (e.pos) last = e.pos;
		}
		const q = e.goal.quest;
		if (t === 'accept' && q != null) open.add(q);
		if ((t === 'turnin' || t === 'abandon') && q != null) open.delete(q);
	}
	close(null);
	return out;
}

// The rules of a segment as items. unit: 'step' (a step's goals move together, in their order) or 'goal'.
function itemsOf(seg, quests, unit) {
	const nodes = seg.nodes;
	const acc = new Map();
	for (let i = 0; i < nodes.length; i++) {
		const g = nodes[i].goal;
		if (g.t === 'accept' && g.quest != null && !acc.has(g.quest)) acc.set(g.quest, i);
	}
	// node-level preds
	const npred = nodes.map(() => new Set());
	const turn = new Map();
	for (let i = 0; i < nodes.length; i++) {
		const g = nodes[i].goal;
		if (g.t === 'turnin' && g.quest != null && !turn.has(g.quest)) turn.set(g.quest, i);
	}
	for (let i = 0; i < nodes.length; i++) {
		const g = nodes[i].goal;
		const q = g.quest;
		if (q == null) continue;
		if (g.t !== 'accept' && acc.has(q) && acc.get(q) !== i) npred[i].add(acc.get(q));
		if (g.t === 'turnin') {
			for (let j = 0; j < nodes.length; j++) {
				const o = nodes[j].goal;
				if (j !== i && o.quest === q && OBJ_TYPES.has(o.t)) npred[i].add(j);
			}
		}
		if (g.t === 'accept' && acc.get(q) === i) {
			const pre = (quests[String(q)] || {}).pre || [];
			for (const p of pre) if (turn.has(p)) npred[i].add(turn.get(p));
		}
	}
	// groups of nodes
	const groups = [];
	if (unit === 'step') {
		const byKey = new Map();
		nodes.forEach((n, i) => {
			const k = `${n.gid}#${n.step}`;
			if (!byKey.has(k)) {
				byKey.set(k, []);
				groups.push(byKey.get(k));
			}
			byKey.get(k).push(i);
		});
	} else {
		nodes.forEach((_, i) => groups.push([i]));
	}
	const of = new Map();
	groups.forEach((gr, k) => gr.forEach((i) => of.set(i, k)));
	const items = groups.map((gr) => ({
		nodes: gr,
		entry: nodes[gr[0]].pos,
		exit: nodes[gr[gr.length - 1]].pos,
		pred: new Set(),
		acc: gr.filter((i) => nodes[i].goal.t === 'accept').length,
		trn: gr.filter((i) => nodes[i].goal.t === 'turnin').length,
		lvl: Math.max(0, ...gr.map((i) => nodes[i].goal.minLevel || 0)),
	}));
	for (let i = 0; i < nodes.length; i++) {
		for (const p of npred[i]) if (of.get(p) !== of.get(i)) items[of.get(i)].pred.add(of.get(p));
	}
	// level gate: the earliest place of an item is where the guide's own walk had the level its goals ask for
	items.forEach((it, k) => {
		it.min = 0;
		if (it.lvl > 1) {
			const f = items.findIndex((o) => nodes[o.nodes[o.nodes.length - 1]].eff >= it.lvl);
			it.min = f < 0 ? k : Math.min(k, f);
		}
	});
	return items;
}

// A segment's problem: the items, the places around it, and the log's room.
function problemOf(seg, quests, unit) {
	const items = itemsOf(seg, quests, unit);
	let open = seg.carry.size, max = open;
	for (const it of items) {
		open += it.acc - it.trn;
		if (open > max) max = open;
	}
	return { items, start: seg.start, end: seg.end, carry: seg.carry.size, cap: Math.max(LOG_CAP, max), nodes: seg.nodes };
}

function costOf(P, order) {
	let c = 0, at = P.start;
	for (const k of order) {
		const it = P.items[k];
		c += dist(at, it.entry);
		at = it.exit;
	}
	return c + dist(at, P.end);
}

// Is the order a legal one?
function feasible(P, order) {
	const at = new Array(order.length);
	for (let p = 0; p < order.length; p++) at[order[p]] = p;
	let open = P.carry;
	for (let p = 0; p < order.length; p++) {
		const it = P.items[order[p]];
		if (p < it.min) return false;
		for (const q of it.pred) if (at[q] > p) return false;
		open += it.acc - it.trn;
		if (open > P.cap) return false;
	}
	return true;
}

// Local search: relocate runs of one to three items, with restarts. Returns the best order found.
function search(P, from, rng, ms) {
	const n = from.length;
	let best = from.slice(), bestC = costOf(P, best);
	if (n < 3) return best;
	const t0 = Date.now();
	const descend = (order) => {
		let c = costOf(P, order);
		for (let improved = true; improved;) {
			improved = false;
			for (let len = 1; len <= 3 && !improved; len++) {
				for (let i = 0; i + len <= n && !improved; i++) {
					const run = order.slice(i, i + len);
					const rest = order.slice(0, i).concat(order.slice(i + len));
					for (let j = 0; j <= rest.length && !improved; j++) {
						if (j === i) continue;
						const cand = rest.slice(0, j).concat(run, rest.slice(j));
						const cc = costOf(P, cand);
						if (cc < c - 1e-9 && feasible(P, cand)) {
							order = cand;
							c = cc;
							improved = true;
						}
					}
				}
			}
		}
		return [order, c];
	};
	let [cur, curC] = descend(best);
	if (curC < bestC) { best = cur; bestC = curC; }
	const b = beam(P, n > 40 ? 300 : 800);
	if (b && feasible(P, b)) {
		[cur, curC] = descend(b);
		if (curC < bestC) { best = cur; bestC = curC; }
	}
	while (Date.now() - t0 < ms) {
		let cand = best.slice();
		const kicks = 2 + Math.floor(rng() * 4);
		for (let k = 0; k < kicks; k++) {
			const len = 1 + Math.floor(rng() * 4);
			const i = Math.floor(rng() * Math.max(1, n - len + 1));
			const run = cand.slice(i, i + len);
			const rest = cand.slice(0, i).concat(cand.slice(i + len));
			const j = Math.floor(rng() * (rest.length + 1));
			const moved = rest.slice(0, j).concat(run, rest.slice(j));
			if (feasible(P, moved)) cand = moved;
		}
		[cur, curC] = descend(cand);
		if (curC < bestC - 1e-9) { best = cur; bestC = curC; }
	}
	return best;
}

// Beam search over the places of the order, one layer for each place: the states of a layer (the items done, the last
// one) are cut to the `width` cheapest by the yards so far and the walk to the nearest item still to do. Two states with
// the same items done and the same last item are one (the cheaper stays).
function beam(P, width = 600) {
	const n = P.items.length;
	if (!n) return [];
	const pred = P.items.map((it) => [...it.pred]);
	const key1 = P.items.map((_, k) => ((k + 1) * 2654435761) >>> 0);
	const key2 = P.items.map((_, k) => ((k + 7) * 40503 * 2246822519) >>> 0);
	let layer = [{ done: new Uint8Array(n), last: -1, cost: 0, open: P.carry, h1: 0, h2: 0, parent: null, item: -1, at: P.start, score: 0 }];
	for (let p = 0; p < n; p++) {
		const seen = new Map();
		const kids = [];
		for (const st of layer) {
			let near = Infinity;
			const cand = [];
			for (let k = 0; k < n; k++) {
				if (st.done[k]) continue;
				const it = P.items[k];
				if (p < it.min) continue;
				let ok = true;
				for (const q of pred[k]) if (!st.done[q]) { ok = false; break; }
				if (!ok) continue;
				const open = st.open + it.acc - it.trn;
				if (open > P.cap) continue;
				cand.push([k, open, dist(st.at, it.entry)]);
			}
			for (const [k, open, d] of cand) {
				const it = P.items[k];
				const cost = st.cost + d;
				const h1 = (st.h1 + key1[k]) >>> 0, h2 = (st.h2 + key2[k]) >>> 0;
				const id = `${h1}.${h2}.${k}`;
				const old = seen.get(id);
				if (old && old.cost <= cost) continue;
				// the walk on from this item to the nearest one left (its lower bound of the next leg)
				near = Infinity;
				for (let j = 0; j < n; j++) {
					if (j === k || st.done[j]) continue;
					const dd = dist(it.exit, P.items[j].entry);
					if (dd < near) near = dd;
				}
				const kid = { parent: st, item: k, done: null, last: k, cost, open, h1, h2, at: it.exit, score: cost + (near === Infinity ? dist(it.exit, P.end) : near) };
				seen.set(id, kid);
				if (old) old.dead = true;
				kids.push(kid);
			}
		}
		const live = kids.filter((x) => !x.dead).sort((a, b) => a.score - b.score).slice(0, width);
		for (const kid of live) {
			kid.done = kid.parent.done.slice();
			kid.done[kid.item] = 1;
		}
		layer = live;
		if (!layer.length) return null;
	}
	let best = null, bc = Infinity;
	for (const st of layer) {
		const c = st.cost + dist(st.at, P.end);
		if (c < bc) { bc = c; best = st; }
	}
	const order = [];
	for (let st = best; st && st.item >= 0; st = st.parent) order.push(st.item);
	return order.reverse();
}

// The exact best order (subset search) of up to EXACT_MAX items; `from` an order the rules allow.
function exact(P, only = null, start = P.start, end = P.end, done = 0, carry = P.carry) {
	const idx = only || P.items.map((_, k) => k);
	const n = idx.length;
	const predMask = idx.map((k) => {
		let m = 0;
		for (const q of P.items[k].pred) {
			const j = idx.indexOf(q);
			if (j >= 0) m |= 1 << j;
		}
		return m;
	});
	const full = (1 << n) - 1;
	const pop = (m) => { let c = 0; for (; m; m &= m - 1) c++; return c; };
	const D = new Float64Array((full + 1) * n).fill(Infinity);
	const from = new Int16Array((full + 1) * n).fill(-1);
	const openAt = (m) => {
		let o = carry;
		for (let j = 0; j < n; j++) if (m & (1 << j)) o += P.items[idx[j]].acc - P.items[idx[j]].trn;
		return o;
	};
	for (let j = 0; j < n; j++) {
		const it = P.items[idx[j]];
		if (predMask[j] === 0 && done >= it.min && carry + it.acc - it.trn <= P.cap) D[(1 << j) * n + j] = dist(start, it.entry);
	}
	for (let m = 1; m <= full; m++) {
		const p = pop(m);
		for (let l = 0; l < n; l++) {
			const c = D[m * n + l];
			if (c === Infinity) continue;
			for (let j = 0; j < n; j++) {
				if (m & (1 << j) || (predMask[j] & m) !== predMask[j]) continue;
				const it = P.items[idx[j]];
				if (done + p < it.min) continue;
				if (openAt(m | (1 << j)) > P.cap) continue;
				const v = c + dist(P.items[idx[l]].exit, it.entry);
				const at = (m | (1 << j)) * n + j;
				if (v < D[at]) { D[at] = v; from[at] = l; }
			}
		}
	}
	let bc = Infinity, bl = -1;
	for (let l = 0; l < n; l++) {
		const v = D[full * n + l] + dist(P.items[idx[l]].exit, end);
		if (v < bc) { bc = v; bl = l; }
	}
	if (bl < 0) return null;
	const order = [];
	for (let m = full, l = bl; l >= 0;) {
		order.push(idx[l]);
		const pl = from[m * n + l];
		m &= ~(1 << l);
		l = pl;
	}
	return order.reverse();
}

// The live reorder as Core/StepOrder.lua and the engine run it: the next WINDOW steps of what is left, in the guide's
// order, are ordered exactly (ending toward the step after them); the first is taken when it saves ORDER_TAKE.
function windowed(P) {
	const left = P.items.map((_, k) => k);
	const order = [];
	let at = P.start, open = P.carry;
	while (left.length) {
		const w = left.slice(0, WINDOW);
		const target = left.length > WINDOW ? P.items[left[WINDOW]].entry : P.end;
		const sub = Object.assign({}, P, { start: at, end: target });
		const base = costOf(sub, w);
		const best = exact(sub, w, at, target, order.length, open) || w;
		const bestC = costOf(sub, best);
		const pick = base - bestC >= TAKE_YD && best[0] !== w[0] ? best[0] : w[0];
		order.push(pick);
		left.splice(left.indexOf(pick), 1);
		at = P.items[pick].exit;
		open += P.items[pick].acc - P.items[pick].trn;
	}
	return order;
}

// The goals of an order of items, in walking order.
const nodesOf = (P, order) => order.flatMap((k) => P.items[k].nodes.map((i) => P.nodes[i]));

// Yards walked through goals, and areas: re-entries (entering an area again after leaving it) and out-and-back trips
// (one or two goals in another area between two visits of one area).
function flowOf(seq, start, end) {
	let yd = 0, at = start;
	for (const n of seq) {
		yd += dist(at, n.pos);
		at = n.pos;
	}
	yd += dist(at, end);
	const centres = [];
	const area = seq.map((n) => {
		let k = centres.findIndex((c) => c[0] === n.pos[0] && Math.hypot(c[1] - n.pos[1], c[2] - n.pos[2]) <= CLUSTER_YD);
		if (k < 0) { centres.push(n.pos); k = centres.length - 1; }
		return k;
	});
	const runs = [];
	for (const a of area) {
		if (runs.length && runs[runs.length - 1].a === a) runs[runs.length - 1].n++;
		else runs.push({ a, n: 1 });
	}
	const reentries = runs.length - new Set(area).size;
	let trips = 0;
	for (let i = 1; i + 1 < runs.length; i++) if (runs[i - 1].a === runs[i + 1].a && runs[i].n <= 2) trips++;
	return { yd, reentries, trips, goals: seq.length };
}

const ZERO = () => ({ yd: 0, reentries: 0, trips: 0, goals: 0 });
const add = (a, b) => ({ yd: a.yd + b.yd, reentries: a.reentries + b.reentries, trips: a.trips + b.trips, goals: a.goals + b.goals });

// The four orders of one segment: { guide, live, steps, goals } flows, and the exact check on small ones.
function gapOf(seg, quests, { seed = 1, ms = 600 } = {}) {
	const out = {};
	const rng = rngOf(seed);
	const S = problemOf(seg, quests, 'step');
	const G = problemOf(seg, quests, 'goal');
	const ident = (P) => P.items.map((_, k) => k);
	out.guide = flowOf(seg.nodes, seg.start, seg.end);
	out.live = flowOf(nodesOf(S, windowed(S)), seg.start, seg.end);
	out.steps = flowOf(nodesOf(S, search(S, ident(S), rng, ms)), seg.start, seg.end);
	out.goals = flowOf(nodesOf(G, search(G, ident(G), rng, ms)), seg.start, seg.end);
	out.check = null;
	if (G.items.length <= EXACT_MAX && G.items.length >= 3) {
		const ex = exact(G);
		const hs = search(G, ident(G), rngOf(seed + 7), ms);
		if (ex) out.check = { n: G.items.length, exact: costOf(G, ex), search: costOf(G, hs) };
	}
	return out;
}

// One simulated run's visits (tests/lua/arrow_player.lua: [kind, quest, map, a, b, second, level, step, goal, guide] for
// each goal the player did): the run's own order, the guide's order of the same goals, and the best order of them under
// the same rules.
function visitsGap(visits, quests, locked = new Set(), { seed = 1, ms = 600 } = {}) {
	const guides = [];
	const entries = visits.map((v, i) => {
		if (v[9] != null && !guides.includes(v[9])) guides.push(v[9]);
		return {
			i, gid: v[9] || 'sim', step: v[7] == null ? i : v[7], gi: v[8] == null ? 0 : v[8], gk: guides.indexOf(v[9]), mode: 'walk', pos: [v[2], v[3], v[4]], eff: v[6],
			goal: { t: v[0], quest: v[1] || null, minLevel: v[1] ? (quests[String(v[1])] || {}).minLevel || 0 : 0 },
		};
	});
	const out = { actual: ZERO(), guide: ZERO(), best: ZERO(), segments: 0 };
	const rng = rngOf(seed);
	for (const seg of segmentsOf(entries, locked)) {
		// the run starts where the player stood: the first goal's place is the start
		const from = seg.start || seg.nodes[0].pos;
		const sg = { ...seg, start: from };
		const G = problemOf(sg, quests, 'goal');
		const best = search(G, G.items.map((_, k) => k), rng, ms);
		const inGuide = sg.nodes.slice().sort((x, y) => x.gk - y.gk || x.step - y.step || x.gi - y.gi || x.i - y.i);
		out.actual = add(out.actual, flowOf(sg.nodes, from, sg.end));
		out.guide = add(out.guide, flowOf(inGuide, from, sg.end));
		out.best = add(out.best, flowOf(nodesOf(G, best), from, sg.end));
		out.segments++;
	}
	return out;
}

module.exports = { visitsGap, beam,  WINDOW, TAKE_YD, LOG_CAP, CLUSTER_YD, EXACT_MAX, segmentsOf, itemsOf, problemOf, costOf, feasible, search, exact, windowed, flowOf, gapOf, nodesOf, add, ZERO, rngOf };
