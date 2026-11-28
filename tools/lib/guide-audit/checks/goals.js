'use strict';
// Checks of one goal against the public reference data, guide by guide (no route needed): is the quest known, does
// its giver and ender match, does the arrow point where the NPC stands, do the levels, prerequisites, side and
// race match, and does a kill or collect goal name what the quest asks for.

const { nearestSpawnAt } = require('../geo');
const { raceAllowed, raceCanBe, classAllowed, sideOk, norm, tokensOf } = require('../ids');
const { RACES } = require('../../game/races');

const QUEST_VERBS = new Set(['accept', 'turnin', 'complete', 'kill', 'collect', 'interact', 'use', 'talk']);
const MOVED_WARN = 60;     // yards between the arrow's point and the nearest spawn of the giver
const MOVED_ERROR = 250;

// Races a goal plays: its own `only`, else its step's, else the guide's (a start guide), else every race of the
// faction (of both factions in a shared guide).
function racesOf(goal, step, guide) {
	if (goal.only && goal.only.races) return goal.only.races;
	if (step.only && step.only.races) return step.only.races;
	if (guide.races) return guide.races;
	return [...new Set(Object.values(RACES).map((r) => r.file))];
}
// The sides a goal plays: its own `only`, else its step's, else the guide's, else both.
function sidesOf(goal, step, guide) {
	return (goal.only && goal.only.factions) || (step.only && step.only.factions) || (guide.faction ? [guide.faction] : ['Alliance', 'Horde']);
}
const classesOf = (goal, step, guide) => (goal.only && goal.only.classes) || (step.only && step.only.classes) || guide.classes || null;

// The races a goal plays that its quest's race mask closes, on the sides the goal plays (a race on a side it cannot
// join is no combination).
function closedRaces(q, goal, step, guide) {
	const sides = sidesOf(goal, step, guide);
	// A shared guide that does not narrow a goal to a side or a race plays it for whoever can take the quest.
	if (sides.length > 1 && !guide.races && !(goal.only && goal.only.races) && !(step.only && step.only.races)) return [];
	const out = [];
	for (const file of racesOf(goal, step, guide)) {
		if (!raceCanBe(file, classesOf(goal, step, guide))) continue;
		const r = Object.values(RACES).find((x) => x.file === file);
		const live = sides.filter((s) => r && r.sides[s]);
		if (live.length && !live.some((s) => raceAllowed(q.races, file, s))) out.push(file);
	}
	return out;
}

function goalChecks(ctx, guide, step, index, goal, add) {
	const { ref } = ctx;
	const here = { guide: guide.id, step: index, goal };
	const q = goal.quest != null ? ref.quests.get(goal.quest) : null;

	if (goal.quest != null && QUEST_VERBS.has(goal.t)) {
		if (!q) {
			add({ ...here, check: 'quest-unknown', severity: 'warn', message: `quest ${goal.quest} (${goal.title || 'no title'}) is not in the public quest data` });
			return;
		}
		if (goal.title && norm(goal.title) !== norm(q.title)) {
			add({ ...here, check: 'title-differs', severity: 'info', message: `title "${goal.title}" in the guide, "${q.title}" in the data` });
		}
		const sides = sidesOf(goal, step, guide);
		if (!sides.some((s) => sideOk(q.side, s))) {
			add({ ...here, check: 'wrong-side', severity: 'error', message: `quest ${goal.quest} "${q.title}" is for ${q.side === 1 ? 'Alliance' : 'Horde'} but the goal plays for ${sides.join(' and ')}` });
		}
		// A step that waits on a quest state (`when onquest 96136`) is for whoever is in that state: no race verdict.
		const stateGated = (step.when || []).some((w) => w.k === 'onquest' || w.k === 'turnedin');
		if ((goal.t === 'accept' || goal.t === 'turnin') && !stateGated) {
			const bad = closedRaces(q, goal, step, guide);
			// Skyborne is Forever's own race: its bit is missing from many quest masks until a player's cache fixes them
			// (quests 26 and 29 were). Only-Skyborne rows are their own check, so a real closed race stands out.
			const skyOnly = bad.length === 1 && bad[0] === 'Skyborne';
			if (bad.length) add({ ...here, check: skyOnly ? 'skyborne-mask' : 'race-closed', severity: skyOnly ? 'warn' : 'error', message: `quest ${goal.quest} "${q.title}" is closed to ${bad.join(', ')}, which this step plays` });
			const classes = classesOf(goal, step, guide);
			if (classes) {
				const cbad = classes.filter((c) => !classAllowed(q.classes, c));
				if (cbad.length) add({ ...here, check: 'class-closed', severity: 'error', message: `quest ${goal.quest} "${q.title}" is closed to ${cbad.join(', ')}, which this step plays` });
			}
		}
	}

	if (goal.t === 'accept' && q) acceptChecks(ctx, here, goal, q, add);
	if (goal.t === 'turnin' && q) turninChecks(ctx, here, goal, q, add);
	if ((goal.t === 'kill' || goal.t === 'collect' || goal.t === 'interact' || goal.t === 'talk' || goal.t === 'use') && q && goal.oi) objectiveChecks(ctx, here, goal, q, add);
}

function acceptChecks(ctx, here, goal, q, add) {
	const { ref } = ctx;
	const starts = ref.starts.get(q.id) || [];
	if (goal.npc != null && starts.length && !starts.some((s) => s.kind === 'npc' && s.id === goal.npc)) {
		const names = starts.map((s) => `${s.kind} ${s.id}`).join(', ');
		add({ ...here, check: 'giver-differs', severity: 'warn', message: `quest ${q.id} "${q.title}" is taken from NPC ${goal.npc} ${goal.npcName || ''} in the guide; the data lists ${names}` });
	}
	if (goal.npc != null && !starts.length && !ref.starts.has(q.id)) {
		add({ ...here, check: 'giver-unknown', severity: 'info', message: `the data has no giver for quest ${q.id} "${q.title}" (the guide uses NPC ${goal.npc} ${goal.npcName || ''})` });
	}
	placeCheck(ctx, here, goal, 'giver', add);
	if (goal.minLevel != null && q.min_level != null && goal.minLevel !== q.min_level) {
		add({ ...here, check: 'level-drift', severity: 'info', message: `quest ${q.id} needs level ${q.min_level} in the data; the compiled guide says ${goal.minLevel}` });
	}
	// Prerequisites that block in the data (corrections applied) against what the build wrote on the goal.
	const blocking = (p) => ref.quests.has(p) && (ref.starts.get(p) || []).length > 0;
	const wanted = (ref.prereqs.get(q.id) || []).filter((p) => !p.breadcrumb && blocking(p.pre)).map((p) => p.pre);
	const had = new Set([...(goal.pre || []), ...(goal.preAll || [])]);
	const missing = wanted.filter((p) => !had.has(p));
	const extra = [...had].filter((p) => !wanted.includes(p) && !(ref.prereqs.get(q.id) || []).some((x) => x.pre === p));
	const name = (p) => `${p} "${(ref.quests.get(p) || {}).title || '?'}"`;
	if (missing.length) add({ ...here, check: 'prereq-drift', severity: 'info', message: `the data needs ${missing.map(name).join(', ')} first for quest ${q.id} "${q.title}"; the compiled guide does not list it` });
	if (extra.length) add({ ...here, check: 'prereq-extra', severity: 'info', message: `the compiled guide holds quest ${q.id} "${q.title}" behind ${extra.map(name).join(', ')}; the data lists no such prerequisite` });
}

function turninChecks(ctx, here, goal, q, add) {
	const { ref } = ctx;
	const ends = ref.ends.get(q.id) || [];
	if (goal.npc != null && ends.length && !ends.some((e) => e.kind === 'npc' && e.id === goal.npc)) {
		add({ ...here, check: 'ender-differs', severity: 'warn', message: `quest ${q.id} "${q.title}" is handed in to NPC ${goal.npc} ${goal.npcName || ''} in the guide; the data lists ${ends.map((e) => `${e.kind} ${e.id}`).join(', ')}` });
	}
	if (goal.npc != null && !ends.length) {
		add({ ...here, check: 'ender-unknown', severity: 'info', message: `the data has no turn-in for quest ${q.id} "${q.title}" (the guide uses NPC ${goal.npc} ${goal.npcName || ''})` });
	}
	placeCheck(ctx, here, goal, 'ender', add);
}

// The arrow's point against where the NPC stands.
function placeCheck(ctx, here, goal, role, add) {
	if (goal.npc == null || goal.map == null || goal.x == null) return;
	if (ctx.ref.placed && ctx.ref.placed.has(`${goal.quest}:${role}`)) return;   // set by hand in data/places.json
	const { yards: d, at } = nearestSpawnAt(ctx.ref, 'npc', goal.npc, goal.map, goal.x, goal.y);
	if (d === Infinity || d < MOVED_WARN) return;
	const spawns = (ctx.ref.spawns.npc.get(goal.npc) || []).length;
	const where = at ? `; the data's spawn is at ${at.x.toFixed(1)},${at.y.toFixed(1)}` : '';
	add({
		...here, check: 'npc-elsewhere', severity: d >= MOVED_ERROR && spawns === 1 ? 'error' : 'warn',
		message: `the arrow's point (${goal.x},${goal.y}, map ${goal.map}) for the ${role} ${goal.npc} ${goal.npcName || ''} is ${Math.round(d)} yd from its nearest spawn in the data${where}${spawns > 1 ? ` (${spawns} spawns: it may roam)` : ''}`,
	});
}

// A kill, collect, interact or talk goal tied to quest objective `oi`: does the data's objective ask for that target?
function objectiveChecks(ctx, here, goal, q, add) {
	const objs = ctx.ref.objectives.get(q.id);
	if (!objs) {
		add({ ...here, check: 'objective-no-data', severity: 'info', message: `${goal.t} goal for quest ${q.id} "${q.title}" objective ${goal.oi}: the public data holds no objectives for the quest` });
		return;
	}
	// A `use` goal names the item that is used, which is not always the one the objective counts: only the kinds that match are compared.
	if (goal.t === 'use') return;
	const o = objs.find((x) => x.n === goal.oi);
	if (!o) {
		add({ ...here, check: 'objective-missing', severity: 'warn', message: `quest ${q.id} "${q.title}" has no objective ${goal.oi} in the data (it has ${objs.length})` });
		return;
	}
	const target = { kill: goal.npc, collect: goal.item, interact: goal.object, talk: goal.npc }[goal.t];
	const kinds = { kill: ['kill'], collect: ['item'], interact: ['object', 'event'], talk: ['talk', 'kill'] }[goal.t];
	if (target != null && kinds.includes(o.kind) && o.target !== target) {
		add({ ...here, check: 'objective-differs', severity: 'warn', message: `${goal.t} goal for quest ${q.id} "${q.title}" objective ${goal.oi} aims at ${target}; the data asks for ${o.kind} ${o.target}` });
	}
	if (goal.t === 'collect' && goal.count && o.kind === 'item' && o.count && goal.count !== o.count) {
		add({ ...here, check: 'count-differs', severity: 'warn', message: `collect goal for quest ${q.id} "${q.title}" wants ${goal.count}; the data asks for ${o.count}` });
	}
}

// Texts name NPCs, quests, objects and items by id: each must exist, or the player reads a blank or a raw id.
function textChecks(ctx, guide, step, index, add) {
	const { ref } = ctx;
	const texts = [[step.note, null], [step.title, null], ...step.goals.map((g) => [g.note, g])];
	for (const [text, goal] of texts) {
		for (const t of tokensOf(text)) {
			const known = { npc: ref.npcs, quest: ref.quests, object: ref.objects }[t.kind];
			if (known && !known.has(t.id) && !(guide.names && guide.names[`${t.kind}:${t.id}`])) {
				add({ guide: guide.id, step: index, goal, check: 'text-unknown-id', severity: 'warn', message: `a text names ${t.kind} ${t.id}, which the public data does not know` });
			}
		}
	}
}

function run(ctx, add) {
	for (const guide of ctx.guides.values()) {
		guide.steps.forEach((step, i) => {
			for (const goal of step.goals) goalChecks(ctx, guide, step, i + 1, goal, add);
			textChecks(ctx, guide, step, i + 1, add);
		});
	}
}

module.exports = { run, racesOf };
