'use strict';
// Checks along a branch's route (a race's guides in play order): prerequisites handed in before an accept, a hand-in
// after its accept, a quest taken twice or never handed in, and the level fit of each accept against the level the
// guide is for. Findings repeat across races; the report merges them and lists the branches.

const { levelsOf, routeOf, startsOf, playsGoal } = require('../route');

// XP a quest pays at `level`: 100% up to quest level + 5, 80% at +6, 60% at +7, 50% from +8 (docs: tools/lib/game/xp.js).
function xpShare(qlevel, level) {
	const over = level - qlevel;
	return over <= 5 ? 1 : over === 6 ? 0.8 : over === 7 ? 0.6 : 0.5;
}

function branchChecks(ctx, branch, add) {
	const { ref, guides } = ctx;
	const route = routeOf(branch, guides);
	const where = (e, goal) => ({ guide: e.guide.id, step: e.index, goal, branch: branch.label });

	// Where each quest is accepted and handed in on this route (first and last unconditional position).
	const at = { accept: new Map(), turnin: new Map() };
	route.forEach((e, pos) => {
		// Conditional steps count here: a hand-in behind `when onquest` is still the quest's hand-in.
		for (const goal of e.step.goals) {
			if (!playsGoal(goal, branch)) continue;
			if (goal.quest == null || (goal.t !== 'accept' && goal.t !== 'turnin')) continue;
			const m = at[goal.t];
			if (!m.has(goal.quest)) m.set(goal.quest, pos);
		}
	});
	const inRoute = (qid) => at.accept.has(qid) || at.turnin.has(qid);

	const accepted = new Set();
	const turned = new Set();
	const abandoned = new Set();
	route.forEach((e, pos) => {
		const { min, max } = levelsOf(e.guide, branch.faction);
		for (const goal of e.step.goals) {
			if (!playsGoal(goal, branch) || goal.quest == null) continue;
			const q = ref.quests.get(goal.quest);
			if (goal.t === 'abandon') abandoned.add(goal.quest);
			if (e.conditional || goal.optional) {
				if (goal.t === 'accept') accepted.add(goal.quest);
				if (goal.t === 'turnin') turned.add(goal.quest);
				continue;
			}
			if (goal.t === 'accept') {
				if (turned.has(goal.quest) && !(q && q.repeatable)) add({ ...where(e, goal), check: 'accept-after-handin', severity: 'warn', message: `quest ${goal.quest} "${goal.title}" is accepted after the route handed it in` });
				else if (accepted.has(goal.quest) && !(q && q.repeatable)) add({ ...where(e, goal), check: 'accept-twice', severity: 'info', message: `quest ${goal.quest} "${goal.title}" is accepted a second time while it is still in the log (the engine counts the second line done)` });
				accepted.add(goal.quest);
				prereqCheck(ctx, e, goal, q, turned, at, pos, where, add);
				levelCheck(e, goal, q, min, max, where, add, ref, at, pos);
			} else if (goal.t === 'turnin') {
				if (!goal.direct && !accepted.has(goal.quest) && !turned.has(goal.quest)) {
					const later = at.accept.get(goal.quest);
					if (later != null && later > pos) add({ ...where(e, goal), check: 'turnin-before-accept', severity: 'error', message: `quest ${goal.quest} "${goal.title}" is handed in here but accepted later (guide ${route[later].guide.id} step ${route[later].index})` });
					else if (!ref.quests.has(goal.quest) || !q) add({ ...where(e, goal), check: 'turnin-without-accept', severity: 'info', message: `quest ${goal.quest} "${goal.title}" is handed in but never accepted on this route` });
					else if (later == null) add({ ...where(e, goal), check: 'turnin-without-accept', severity: 'warn', message: `quest ${goal.quest} "${goal.title}" is handed in but never accepted on this route (a carried quest, or a missing accept)` });
				}
				turned.add(goal.quest);
			}
		}
	});
	// Taken and never handed in, on the whole route.
	for (const [qid, pos] of at.accept) {
		if (at.turnin.has(qid) || abandoned.has(qid)) continue;
		const e = route[pos];
		const goal = e.step.goals.find((g) => g.quest === qid && g.t === 'accept' && playsGoal(g, branch));
		if (goal && goal.optional) continue;   // "take it only if you pass by": the guide never promises the hand-in
		if (e.conditional) continue;           // a dungeon or group quest, handed in where the group is
		const q = ref.quests.get(qid);
		if (goal && goal.carriedTo) continue;
		const other = ctx.providers.get(qid);
		if (other && [...other].some((id) => id !== e.guide.id && !/^forever\/(alliance|horde)\//.test(id))) continue;   // a class or reputation guide hands it in
		add({ ...where(e, goal), check: 'never-handed-in', severity: 'warn', message: `quest ${qid} "${(q && q.title) || goal.title}" is accepted and never handed in on the route` });
	}
}

// An accept whose prerequisites (the goal's own: `pre` one of them, `preAll` every one) are not handed in before it.
function prereqCheck(ctx, e, goal, q, turned, at, pos, where, add) {
	const { ref } = ctx;
	if (!q) return;
	const name = (p) => `${p} "${(ref.quests.get(p) || {}).title || '?'}"`;
	const check = (list, every) => {
		if (!list || !list.length) return;
		const open = every ? list.filter((p) => !turned.has(p)) : (list.some((p) => turned.has(p)) ? [] : list);
		if (!open.length) return;
		const later = open.filter((p) => at.turnin.get(p) > pos);
		const joined = open.map(name).join(every ? ' and ' : ' or ');
		if (later.length === open.length || (every && later.length)) {
			add({ ...where(e, goal), check: 'prereq-later', severity: 'error', message: `quest ${q.id} "${q.title}" needs ${joined} first, which this route hands in later` });
			return;
		}
		const others = open.filter((p) => ctx.providers.has(p));
		const from = others.length ? ` (another guide hands in ${others.map((p) => [...ctx.providers.get(p)][0]).join(', ')})` : '';
		add({ ...where(e, goal), check: 'prereq-outside-route', severity: others.length ? 'info' : 'warn', message: `quest ${q.id} "${q.title}" needs ${joined} first, which this route never hands in${from}` });
	};
	check(goal.preAll, true);
	check(goal.pre, false);
	// What the data adds to the goal's own list (a build that predates a data fix): in the route's order too?
	const own = new Set([...(goal.pre || []), ...(goal.preAll || [])]);
	const blocking = (ref.prereqs.get(q.id) || []).filter((p) => !p.breadcrumb && !own.has(p.pre) && ref.quests.has(p.pre) && (ref.starts.get(p.pre) || []).length);
	for (const p of blocking) {
		if (turned.has(p.pre)) continue;
		const later = at.turnin.get(p.pre) > pos;
		if (later) add({ ...where(e, goal), check: 'data-prereq-later', severity: 'warn', message: `the data needs ${name(p.pre)} first for quest ${q.id} "${q.title}" (the compiled guide does not list it), and this route hands it in later` });
	}
}

// The quest against the guide's level range. `max + 1` allows the level gained inside the guide's last step.
function levelCheck(e, goal, q, min, max, where, add, ref, at, pos) {
	if (!q || min == null || max == null) return;
	if (q.min_level > max + 1) {
		add({ ...where(e, goal), check: 'level-locked', severity: 'error', message: `quest ${q.id} "${q.title}" needs level ${q.min_level}; the guide is for ${min}-${max}` });
	}
	// Five levels or more above the guide's top level is a red quest (a group quest says so itself).
	if (q.level != null && q.level - max >= 5 && !(ref.mechanics.get(q.id) || []).some((m) => /group|elite/.test(m.kind))) {
		add({ ...where(e, goal), check: 'quest-too-hard', severity: 'info', message: `quest ${q.id} "${q.title}" is level ${q.level}; the guide is for ${min}-${max}` });
	}
	// Half-paid from the guide's first step on, and nothing later on the route needs it.
	const needed = (ref.followers.get(q.id) || []).some((f) => at.accept.has(f.quest));
	if (!needed && q.xp && q.level != null && xpShare(q.level, min) <= 0.5) {
		add({ ...where(e, goal), check: 'grey-quest', severity: 'info', message: `quest ${q.id} "${q.title}" is level ${q.level}; at the guide's first level (${min}) it pays half XP or less` });
	}
}

function run(ctx, add) {
	for (const branch of startsOf(ctx.guides)) branchChecks(ctx, branch, add);
}

module.exports = { run, xpShare };
