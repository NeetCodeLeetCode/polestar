'use strict';
// Walking checks along a branch's route, from the places the build gave each goal (same continent only; a flight
// or a boat ends a leg). These are leads for the route's owner, not verdicts: a quest hub is often visited twice
// on purpose. Findings: a round trip to one step and back (A, B, A), and an NPC visited in two separate steps when
// the later quest could have been taken on the first visit.

const { toWorld, yards } = require('../geo');
const { routeOf, startsOf, playsGoal } = require('../route');

const ROUND_TRIP_YD = 1000;   // extra yards a detour step adds
const SAME_PLACE_YD = 200;    // the steps before and after it are at about one place
const FAR_YD = 600;           // a step in between is this far away
const TELEPORT_YD = 6000;     // farther than this is a flight, a boat or a teleport, not a walk
const LONG_LEG_YD = 2500;     // a walk between two steps this long is listed

function placeOf(ref, goal) {
	if (goal.map == null || goal.x == null) return null;
	return toWorld(ref, goal.map, goal.x, goal.y);
}

// The step's first and last positioned goal, in world yards.
function span(ref, step, branch) {
	const pts = step.goals.filter((g) => !g.optional && g.t !== 'note' && playsGoal(g, branch)).map((g) => placeOf(ref, g)).filter(Boolean);
	return pts.length ? { first: pts[0], last: pts[pts.length - 1] } : null;
}

function branchWalk(ctx, branch, add) {
	const { ref, guides } = ctx;
	const route = routeOf(branch, guides).filter((e) => !e.conditional);
	const spans = route.map((e) => span(ref, e.step, branch));
	const where = (e, goal) => ({ guide: e.guide.id, step: e.index, goal, branch: branch.label });

	// A, B, A: step i+1 sits away from steps i and i+2, which are close together.
	for (let i = 0; i + 2 < route.length; i++) {
		const [a, b, c] = [spans[i], spans[i + 1], spans[i + 2]];
		if (!a || !b || !c || route[i].guide !== route[i + 2].guide) continue;
		const back = yards(a.last, c.first);
		const out = yards(a.last, b.first) + yards(b.last, c.first);
		const there = yards(a.last, b.first);
		if (back <= SAME_PLACE_YD && out - back >= ROUND_TRIP_YD && there >= FAR_YD && there <= TELEPORT_YD && yards(b.last, c.first) <= TELEPORT_YD) {
			add({ ...where(route[i + 1]), check: 'round-trip', severity: 'info', yards: Math.round(out - back), message: `a round trip of about ${Math.round(out - back)} yd: step ${route[i].index} and step ${route[i + 2].index} are ${Math.round(back)} yd apart, this step is in between` });
		}
	}

	// Long walks between two steps of one guide: where a step's first point is far from the last one's, and no flight, boat or hearth leads there.
	for (let i = 0; i + 1 < route.length; i++) {
		const [a, b] = [spans[i], spans[i + 1]];
		if (!a || !b || route[i].guide !== route[i + 1].guide) continue;
		const d = yards(a.last, b.first);
		const travel = route[i].step.goals.concat(route[i + 1].step.goals).some((g) => g.t === 'fly' || g.t === 'sethome');
		if (d >= LONG_LEG_YD && d <= TELEPORT_YD && !travel) add({ ...where(route[i + 1]), check: 'long-leg', severity: 'info', yards: Math.round(d), message: `a walk of about ${Math.round(d)} yd from step ${route[i].index} to this step` });
	}

	// The same NPC in two steps of one guide with a far step between, for a quest that was open at the first visit.
	const seen = new Map();   // npc -> { pos, turned: Set at that time }
	const turned = new Set();
	route.forEach((e, pos) => {
		const mine = e.step.goals.filter((g) => playsGoal(g, branch));
		for (const goal of mine) {
			if (goal.t === 'turnin' && goal.quest != null) turned.add(goal.quest);
		}
		const npcs = new Set(mine.filter((g) => g.t === 'accept' && g.npc != null).map((g) => g.npc));
		for (const npc of npcs) {
			const prev = seen.get(npc);
			if (prev && prev.guide === e.guide.id && pos - prev.pos >= 2) {
				const between = spans.slice(prev.pos + 1, pos).filter(Boolean);
				const here = spans[prev.pos];
				const away = here && between.length && Math.min(...between.map((s) => yards(here.last, s.first))) >= FAR_YD;
				if (away) {
					for (const g of mine.filter((x) => x.t === 'accept' && x.npc === npc)) {
						const q = ref.quests.get(g.quest);
						const pres = (ref.prereqs.get(g.quest) || []).filter((p) => p.mode === 'all' && !p.breadcrumb).map((p) => p.pre);
						const ok = q && pres.every((p) => prev.turned.has(p)) && (q.min_level || 0) <= (prev.level || 0);
						if (ok) add({ ...where(e, g), check: 'split-visit', severity: 'info', message: `quest ${g.quest} "${g.title}" is taken from ${g.npcName} here, but the route stood with ${g.npcName} at step ${prev.index} and nothing needed first was missing then` });
					}
				}
			}
			seen.set(npc, { pos, guide: e.guide.id, index: e.index, turned: new Set(turned), level: e.guide.minLevel });
		}
	});
}

function run(ctx, add) {
	for (const branch of startsOf(ctx.guides)) branchWalk(ctx, branch, add);
}

module.exports = { run };
