'use strict';
// The public reference data the audit checks the guides against: forever-db/data/*.jsonl, which every clone has
// (no `forge data build`, no data/forever). Quests with their givers, enders, prerequisites and objectives; NPCs
// and objects with their spawn points; each in-game map's frame, to turn a point in world yards into map percent.

const fs = require('node:fs');
const path = require('node:path');
const { ROOT } = require('../paths');
const { loadPublicCorrections } = require('../data/corrections');
const { toWorld } = require('./geo');
const PLACES = path.join(ROOT, 'data', 'places.json');

const DIR = path.join(ROOT, 'forever-db', 'data');

function rows(name, dir) {
	const file = path.join(dir, `${name}.jsonl`);
	if (!fs.existsSync(file)) return [];
	const out = [];
	for (const line of fs.readFileSync(file, 'utf8').split('\n')) if (line) out.push(JSON.parse(line));
	return out;
}

function group(list, key) {
	const m = new Map();
	for (const r of list) {
		const k = r[key];
		if (!m.has(k)) m.set(k, []);
		m.get(k).push(r);
	}
	return m;
}

// The published tables lag the corrections (forever-db/corrections): some records are in the folder and not yet in a
// release (quest 1473's races, for one). The build applies every record, so the audit does too: what a build
// would read is what the guides were made from. Only the fields the checks read are applied.
function applyCorrections(ref, corr) {
	const rowsOf = (kind, quest, ids) => (ids || []).map((id) => ({ kind, id, quest }));
	for (const [key, e] of Object.entries(corr.quests || {})) {
		if (key.startsWith('_')) continue;
		const id = Number(key);
		if (e.remove) { ref.quests.delete(id); ref.starts.delete(id); ref.ends.delete(id); ref.prereqs.delete(id); continue; }
		const q = ref.quests.get(id);
		const s = e.set;
		if (!q || !s) continue;
		for (const [from, to] of [['title', 'title'], ['level', 'level'], ['minLevel', 'min_level'], ['races', 'races'], ['classes', 'classes'], ['xp', 'xp']]) if (from in s) q[to] = s[from];
		if (Array.isArray(s.objectives)) ref.objectives.set(id, s.objectives.map((o, i) => ({ quest: id, n: i + 1, kind: o.type, target: o.id, count: o.count })));
		for (const [field, table] of [['start', ref.starts], ['end', ref.ends]]) {
			if (!s[field]) continue;
			table.set(id, ['npc', 'object', 'item'].flatMap((k) => rowsOf(k, id, s[field][k])));
		}
		if ('pre' in s || 'preAny' in s || 'preAll' in s) {
			const old = ref.prereqs.get(id) || [];
			const crumbs = old.filter((p) => p.breadcrumb);
			const split = Array.isArray(s.preAll) || Array.isArray(s.preAny);
			const any = split ? s.preAny || [] : s.pre || [];
			const all = split ? s.preAll || [] : [];
			ref.prereqs.set(id, [...crumbs, ...any.map((pre) => ({ quest: id, pre, mode: 'any' })), ...all.map((pre) => ({ quest: id, pre, mode: 'all' }))]);
		}
	}
	for (const [key, e] of Object.entries(corr.npcs || {})) {
		if (key.startsWith('_')) continue;
		const id = Number(key);
		const spawns = (src) => Object.entries(src || {}).flatMap(([uiMap, v]) => v.pts.map(([x, y]) => toWorld(ref, Number(uiMap), x, y)).filter(Boolean).map((w) => ({ id, kind: 'npc', map: w.continent, x: w.x, y: w.y })));
		if (e.set && e.set.spawns) ref.spawns.npc.set(id, spawns(e.set.spawns));
		if (e.addSpawns) ref.spawns.npc.set(id, [...(ref.spawns.npc.get(id) || []), ...spawns(e.addSpawns)]);
	}
	return ref;
}

// Quests whose giver or ender point the maintainers set by hand (data/places.json): an NPC that shows elsewhere
// for the quest. The arrow there is meant to differ from the NPC's spawn.
function placedEnds() {
	const out = new Set();
	if (!fs.existsSync(PLACES)) return out;
	for (const p of JSON.parse(fs.readFileSync(PLACES, 'utf8')).places || []) if (p.objective === 'giver' || p.objective === 'ender') out.add(`${p.quest}:${p.objective}`);
	return out;
}

// The tables, indexed the way the checks ask. `maps` turns world yards into map percent and back.
function loadRef(dir = DIR, { corrections = true } = {}) {
	const quests = new Map(rows('quest', dir).map((q) => [q.id, q]));
	const npcs = new Map(rows('npc', dir).map((n) => [n.id, n]));
	const objects = new Map(rows('object', dir).map((o) => [o.id, o]));
	const frames = new Map(rows('ui_map', dir).map((m) => [m.ui_map, m]));
	const spawns = { npc: new Map(), object: new Map() };
	for (const s of rows('spawn', dir)) {
		const bucket = spawns[s.kind];
		if (!bucket) continue;
		if (!bucket.has(s.id)) bucket.set(s.id, []);
		bucket.get(s.id).push(s);
	}
	const ref = {
		quests, npcs, objects, frames, spawns, placed: placedEnds(),
		starts: group(rows('quest_start', dir), 'quest'),
		ends: group(rows('quest_end', dir), 'quest'),
		prereqs: group(rows('quest_prereq', dir), 'quest'),
		objectives: group(rows('quest_objective', dir), 'quest'),
		mechanics: group(rows('mechanic', dir), 'quest'),
		// Quests a quest unlocks: the reverse of the prerequisite table.
		followers: group(rows('quest_prereq', dir), 'pre'),
	};
	return corrections ? applyCorrections(ref, loadPublicCorrections()) : ref;
}

module.exports = { loadRef, applyCorrections, DIR };
