'use strict';
// The route one character plays: its start guide, then each guide's `next` (the faction's side where a guide
// offers both), with the steps that character plays. A branch is a race; class-only steps stay in (a class never
// loses the quests its own steps give).

const side = (f) => String(f || '').toLowerCase();

// The guide a character goes on to from `g`: the `nextChoices` entry of its faction, else `next`.
function nextOf(g, faction, guides) {
	const pick = (g.nextChoices || []).find((id) => id.includes(`/${side(faction)}/`) && guides.has(id));
	const id = pick || g.next;
	return id && guides.has(id) ? id : null;
}

// Start guides: a Leveling guide with a `races` list that no other start guide points into.
function startsOf(guides) {
	const leveling = [...guides.values()].filter((g) => g.category === 'Leveling');
	const pointed = new Set(leveling.filter((g) => g.races).map((g) => g.next));
	const out = [];
	for (const g of leveling) {
		if (!g.races || pointed.has(g.id)) continue;
		for (const race of g.races) out.push({ race, faction: g.faction, start: g.id, label: `${race}-${g.faction}` });
	}
	return out;
}

// An `only` (races, factions, classes) admits a branch (a race on a side); classes never narrow a branch.
const onlyOk = (only, branch) => !only || ((!only.races || only.races.includes(branch.race)) && (!only.factions || only.factions.includes(branch.faction)));
const playsRace = (only, branch) => onlyOk(only, branch);
// A goal can carry its own `only`, narrower than its step's.
const playsGoal = (goal, branch) => onlyOk(goal.only, branch);

// The guide ids of a branch in order, each once (a cycle ends the walk).
function chainOf(branch, guides) {
	const ids = [];
	let id = branch.start;
	while (id && !ids.includes(id)) {
		ids.push(id);
		id = nextOf(guides.get(id), branch.faction, guides);
	}
	return ids;
}

// The steps of a branch in play order: { guide, index (1-based), step, conditional }. `when` makes a step
// conditional (a dungeon, a skill, a quest state): checks that need the order skip those.
function routeOf(branch, guides) {
	const out = [];
	for (const id of chainOf(branch, guides)) {
		const g = guides.get(id);
		g.steps.forEach((step, i) => {
			if (!playsRace(step.only, branch)) return;
			out.push({ guide: g, index: i + 1, step, conditional: !!step.when });
		});
	}
	return out;
}

// The guide's own levels for a faction (shared guides carry one range per side).
function levelsOf(g, faction) {
	const fl = g.factionLevels && g.factionLevels[faction];
	return { min: fl ? fl.min : g.minLevel, max: fl ? fl.max : g.maxLevel };
}

module.exports = { startsOf, chainOf, routeOf, nextOf, playsRace, playsGoal, levelsOf };
