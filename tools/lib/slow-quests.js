'use strict';
// The score behind "Skip slow quests" (tools/commands/slowquests.js): how many XP a quest pays for each minute its
// work takes, from forever-db, set against the median of quests of the same level band. It counts only the work the
// data knows: kills and the kills an item drop needs. Walks, respawns and crowds are not in the data, so the score
// can say a quest is not slow (its drop is fine) but never that it is.

const fs = require('node:fs');
const path = require('node:path');
const { ROOT } = require('./paths');
const xp = require('./game/xp');

const DB_DIR = path.join(ROOT, 'forever-db', 'data');
// Blizzard's aim for Forever: an ordinary mob dies in 10 to 15 seconds.
const KILL_MINUTES = 0.25;
// The talk at each end of a quest.
const BASE_MINUTES = 1;
// Quests of 3 levels share one median.
const BAND = 3;
// The quests a character of level L does on level: a quest of level L - 3 to L + 2 (the route's own pace at L).
const WINDOW_BELOW = 3;
const WINDOW_ABOVE = 2;

function readJsonl(file) {
	return fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line));
}

// -> { quests: Map(id -> { level, xp }), objectives: Map(id -> [objective]), chance: Map(item -> best drop percent) }
function loadDb(dir = DB_DIR) {
	const quests = new Map();
	for (const r of readJsonl(path.join(dir, 'quest.jsonl'))) quests.set(r.id, { level: r.level, xp: r.xp || 0, classes: r.classes || 0 });
	const objectives = new Map();
	for (const r of readJsonl(path.join(dir, 'quest_objective.jsonl'))) {
		if (!objectives.has(r.quest)) objectives.set(r.quest, []);
		objectives.get(r.quest).push(r);
	}
	const chance = new Map();
	for (const r of readJsonl(path.join(dir, 'item_source.jsonl'))) {
		if (r.kind !== 'npc' || !(r.chance > 0)) continue;
		if (!(chance.get(r.item) >= r.chance)) chance.set(r.item, r.chance);
	}
	// A mob's level for the sums: the middle of its range, and whether it is an elite.
	const npcs = new Map();
	for (const r of readJsonl(path.join(dir, 'npc.jsonl'))) {
		const lo = r.level_min || 1;
		npcs.set(r.id, { level: Math.floor((lo + (r.level_max || lo)) / 2), elite: (r.rank || 0) >= 1 });
	}
	return { quests, objectives, chance, npcs };
}

// The minutes the work of quest q takes, or null when the data holds no kill and no known drop for it. An item
// objective takes count over the best drop chance of its item, in kills.
function questMinutes(db, q) {
	let kills = 0;
	let known = false;
	for (const o of db.objectives.get(q) || []) {
		if (o.kind === 'kill') {
			kills += o.count || 1;
			known = true;
		} else if (o.kind === 'item') {
			const percent = db.chance.get(o.target);
			if (!percent) continue;
			kills += (o.count || 1) / (percent / 100);
			known = true;
		}
	}
	return known ? kills * KILL_MINUTES + BASE_MINUTES : null;
}

// Whether the quest database's class mask makes quest `info` a class quest: open to some classes only (the engine's
// Own.classQuest reads the same mask, 1503 being every class).
const ALL_CLASSES = 1503;
function isClassMask(info) {
	return info.classes !== 0 && (info.classes & ALL_CLASSES) !== ALL_CLASSES;
}

function median(list) {
	const s = [...list].sort((a, b) => a - b);
	const m = s.length >> 1;
	return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}

// Each band's median XP per minute over the quests that can be scored, class quests left out (`classQuests`: more
// quest ids to leave out, the Class guides'). -> Map(band -> median)
function bandMedians(db, classQuests = new Set()) {
	const bands = new Map();
	for (const [q, info] of db.quests) {
		if (isClassMask(info) || classQuests.has(q)) continue;
		const minutes = questMinutes(db, q);
		if (!minutes || !info.xp) continue;
		const band = Math.floor(info.level / BAND);
		if (!bands.has(band)) bands.set(band, []);
		bands.get(band).push(info.xp / minutes);
	}
	return new Map([...bands].map(([band, list]) => [band, median(list)]));
}

// The score of a chain of quests: its XP over its minutes (the quests with no known work add nothing to either),
// and that over the median of the band of its mean level. -> { xpm, median, ratio } or null with no known work.
function chainScore(db, medians, quests) {
	let xp = 0;
	let minutes = 0;
	let level = 0;
	for (const q of quests) {
		const info = db.quests.get(q);
		const m = questMinutes(db, q);
		level += info.level;
		if (m) {
			xp += info.xp;
			minutes += m;
		}
	}
	if (!minutes) return null;
	const mid = medians.get(Math.floor(level / quests.length / BAND));
	if (!mid) return null;
	const xpm = xp / minutes;
	return { xpm: Math.round(xpm), median: Math.round(mid), ratio: Math.round((xpm / mid) * 100) / 100 };
}

// What a quest pays a character of `level` for each minute of its work, as a share of that level's XP bar: its
// reward at the share Forever pays for the level gap (xp.js questXpShare) and the XP of the kills its objectives
// ask (xp.js mobXp, nothing from a mob that is grey for the character), over its minutes. null when the data holds
// no kill and no known drop for it, or no reward. The walk is not in it (the data holds no route), so the rate is
// an upper bound: a quest it calls slow is slow.
function barRate(db, q, level) {
	const info = db.quests.get(q);
	const minutes = questMinutes(db, q);
	if (!(minutes && info && info.xp && xp.LEVEL_XP[level])) return null;
	let gained = xp.questXpAt(info.xp, level, info.level);
	for (const o of db.objectives.get(q) || []) {
		const mob = o.kind === 'kill' && db.npcs && db.npcs.get(o.target);
		if (mob) gained += (o.count || 1) * xp.mobXp(level, mob.level, mob.elite);
	}
	return gained / xp.LEVEL_XP[level] / minutes;
}

// Each player level's median bar rate over the quests it does on level, class quests left out (as bandMedians).
// -> Map(level -> median)
function levelMedians(db, classQuests = new Set()) {
	const out = new Map();
	for (let level = 1; level < xp.MAX_LEVEL; level++) {
		const rates = [];
		for (const [q, info] of db.quests) {
			if (isClassMask(info) || classQuests.has(q)) continue;
			if (info.level < level - WINDOW_BELOW || info.level > level + WINDOW_ABOVE) continue;
			const r = barRate(db, q, level);
			if (r) rates.push(r);
		}
		if (rates.length) out.set(level, median(rates));
	}
	return out;
}

// The quests that turn slow as the character outlevels them: each one's lowest level at which its bar rate is
// under `threshold` times the median of its level, while at its own level it was not (a quest slow at its own
// level is in a guide for another reason: on the way, a chain, a hand-in), and only from `minGap` levels above the
// quest (6: where Forever starts to cut the reward). `skip`: quest ids never listed (class
// quests, the hand-written list's). -> Map(quest -> level)
function slowFromLevels(db, medians, threshold, minGap, skip = new Set()) {
	const out = new Map();
	for (const [q, info] of db.quests) {
		if (isClassMask(info) || skip.has(q)) continue;
		const own = barRate(db, q, info.level);
		const ownMedian = medians.get(info.level);
		if (own === null || !ownMedian || own / ownMedian < threshold) continue;
		for (let level = info.level + minGap; level < xp.MAX_LEVEL; level++) {
			const mid = medians.get(level);
			const r = barRate(db, q, level);
			if (mid && r !== null && r / mid < threshold) {
				out.set(q, level);
				break;
			}
		}
	}
	return out;
}

module.exports = { barRate, levelMedians, slowFromLevels, WINDOW_BELOW, WINDOW_ABOVE, ALL_CLASSES, isClassMask, DB_DIR, KILL_MINUTES, BASE_MINUTES, BAND, loadDb, questMinutes, bandMedians, chainScore };
