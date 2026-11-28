'use strict';
// node tools/polestar.js slowquests build|check|report
//
// The quests "Skip slow quests" (behavior.skipSlow) leaves out: the hand-written list data/codex/slow-quests.json
// names chains that cost far more time than they pay, each with 2 or more agreeing sources and a review date. A
// `low-drop` row is scored from forever-db (tools/lib/slow-quests.js): a row whose XP per minute reaches `margin`
// times the median of its level band is overruled and left out of the addon. build writes
// addon/Polestar/Core/SlowQuestList.lua; check exits 1 when the committed file differs; report prints each row's
// score and age.

const fs = require('node:fs');
const path = require('node:path');
const { parseArgs } = require('node:util');
const { header, reportSync } = require('../lib/generated');
const { ROOT } = require('../lib/paths');
const { loadDb, bandMedians, chainScore, isClassMask, levelMedians, slowFromLevels } = require('../lib/slow-quests');
const { classGuides } = require('./classquests');

const usage = `usage: node tools/polestar.js slowquests build [--list <file>]
       node tools/polestar.js slowquests check [--list <file>]
       node tools/polestar.js slowquests report [--list <file>] [--today <yyyy-mm-dd>]

build: writes addon/Polestar/Core/SlowQuestList.lua from the list and forever-db. check: exits 1 when the committed
file differs. report: each row's score against its level band's median, whether it is overruled, and its age.

  --list <file>   the list (default: data/codex/slow-quests.json)
  --today <date>  the day report counts a row's age from (default: today)`;

const LIST = path.join(ROOT, 'data', 'codex', 'slow-quests.json');
const OUT_DIR = path.join(ROOT, 'addon', 'Polestar', 'Core');
const OUT_NAME = 'SlowQuestList.lua';
// Why a row is slow. Only the codes in SCORED are set against the data: a drop rate is in it, a respawn is not.
const CODES = ['low-drop', 'slow-respawn', 'slow-xp'];
const SCORED = ['low-drop'];
const MIN_SOURCES = 2;
// A row older than this many days is due for a review (drop rates change in hotfixes).
const STALE_DAYS = 30;
const DAY = /^\d{4}-\d{2}-\d{2}$/;

// Every quest of a Class guide (guides/ with `category Class`): a class chain, never left out by this setting.
function classQuestSet(guidesDir = path.join(ROOT, 'guides')) {
	const set = new Set();
	for (const g of classGuides(guidesDir)) for (const q of g.quests) set.add(q);
	return set;
}

// The list checked against forever-db and scored. `classQuests`: the quests of the Class guides. A class quest (one
// of those, or one the quest database opens to some classes only) is never a row, and the medians leave them out.
// -> { rows: [{ ...row, score, overruled }], codes: { code -> sorted quest ids kept slow } }
// Throws on a quest forever-db does not know, a quest in two rows or in both slow and keep, an unknown code, a row
// with no reason or review date, or one with fewer than MIN_SOURCES sources.
function buildSlowList(list, db, classQuests = new Set()) {
	const errors = [];
	const margin = list.margin;
	if (!(typeof margin === 'number' && margin > 0)) errors.push('margin must be a number over 0');
	const level = list.level || {};
	if (!(typeof level.threshold === 'number' && level.threshold > 0 && level.threshold < 1)) errors.push('level.threshold must be a number between 0 and 1');
	if (!(Number.isInteger(level.fromGap) && level.fromGap >= 1)) errors.push('level.fromGap must be a whole number of levels, 1 or more');
	const seen = new Map();
	const claim = (kind, row, i) => {
		const where = `${kind}[${i}] ${row.name || ''}`.trim();
		if (!Array.isArray(row.quests) || !row.quests.length) errors.push(`${where}: no quests`);
		for (const q of row.quests || []) {
			if (!db.quests.has(q)) errors.push(`${where}: quest ${q} is not in forever-db`);
			else if (kind === 'slow' && (classQuests.has(q) || isClassMask(db.quests.get(q)))) errors.push(`${where}: quest ${q} is a class quest, and class quests are never left out`);
			if (seen.has(q)) errors.push(`${where}: quest ${q} is also in ${seen.get(q)}`);
			else seen.set(q, where);
		}
		if (typeof row.why !== 'string' || !row.why.trim()) errors.push(`${where}: no reason (why)`);
		return where;
	};
	(list.slow || []).forEach((row, i) => {
		const where = claim('slow', row, i);
		if (!CODES.includes(row.code)) errors.push(`${where}: code "${row.code}" is none of ${CODES.join(', ')}`);
		if (!DAY.test(row.reviewed || '')) errors.push(`${where}: reviewed must be a yyyy-mm-dd date`);
		const sources = row.sources || [];
		if (sources.length < MIN_SOURCES) errors.push(`${where}: ${sources.length} sources, at least ${MIN_SOURCES} needed`);
		for (const s of sources) if (!s.kind || !s.date || !s.note) errors.push(`${where}: a source needs kind, date and note`);
	});
	(list.keep || []).forEach((row, i) => {
		const where = claim('keep', row, i);
		if (!DAY.test(row.reviewed || '')) errors.push(`${where}: reviewed must be a yyyy-mm-dd date`);
	});
	(list.covered || []).forEach((row, i) => {
		for (const q of row.quests || []) if (!db.quests.has(q)) errors.push(`covered[${i}] ${row.name}: quest ${q} is not in forever-db`);
	});
	if (errors.length) throw new Error(`slow-quests.json:\n  ${errors.join('\n  ')}`);

	const medians = bandMedians(db, classQuests);
	const rows = [];
	const codes = {};
	for (const row of list.slow) {
		const score = chainScore(db, medians, row.quests);
		const overruled = SCORED.includes(row.code) && score !== null && score.ratio >= margin;
		rows.push({ ...row, score, overruled });
		if (!overruled) (codes[row.code] = codes[row.code] || []).push(...row.quests);
	}
	for (const code of Object.keys(codes)) codes[code].sort((a, b) => a - b);
	// The quests that turn slow as the character outlevels them: from the data alone, never a quest of a row, a
	// `keep` row or a class quest.
	const skip = new Set([...classQuests, ...seen.keys()]);
	const byLevel = slowFromLevels(db, levelMedians(db, classQuests), level.threshold, level.fromGap, skip);
	return { rows, keep: list.keep || [], codes, byLevel };
}

// The `quest:level` pairs, in quest order, as chunks of one string.
function levelChunks(byLevel) {
	const pairs = [...(byLevel || [])].sort((a, b) => a[0] - b[0]).map(([q, l]) => `${q}:${l}`);
	if (!pairs.length) return ['\t" "'];
	const lines = [];
	for (let i = 0; i < pairs.length; i += 12) lines.push(pairs.slice(i, i + 12).join(' '));
	return lines.map((l, i) => `\t${JSON.stringify((i === 0 ? ' ' : '') + l + ' ')}${i < lines.length - 1 ? ' ..' : ''}`);
}

// One string of ids per code, read with string.find (Polestar's core at login has a memory budget).
function toLua({ codes, byLevel }) {
	const chunks = (ids) => {
		const lines = [];
		for (let i = 0; i < ids.length; i += 16) lines.push(ids.slice(i, i + 16).join(' '));
		return lines.map((l, i) => `\t\t${JSON.stringify((i === 0 ? ' ' : '') + l + ' ')}${i < lines.length - 1 ? ' ..' : ','}`);
	};
	const body = [];
	for (const code of CODES) {
		if (!codes[code]) continue;
		body.push(`\t[${JSON.stringify(code)}] =`, ...chunks(codes[code]));
	}
	return [
		header('lua', 'slowquests build', 'data/codex/slow-quests.json and forever-db'),
		'-- The quests behavior.skipSlow ("Skip slow quests") leaves out of the leveling guides, by the code that says',
		'-- why: low-drop, slow-respawn, slow-xp. A row the data overrules is not here (tools/commands/slowquests.js).',
		'-- Read by Core/Engine.lua (Own.slowSkipped).',
		'',
		'local ADDON, ns = ...',
		'local find, match = string.find, string.match',
		'',
		'local List = {}',
		'local codes = {',
		...body,
		'}',
		'',
		'-- The code that makes quest `q` slow, or nil when it is not on the list.',
		'function List.Slow(q)',
		'\tif type(q) ~= "number" then return nil end',
		'\tlocal key = " " .. q .. " "',
		'\tfor code, ids in pairs(codes) do',
		'\t\tif find(ids, key, 1, true) then return code end',
		'\tend',
		'\treturn nil',
		'end',
		'',
		'-- Quests that turn slow as the character outlevels them: " quest:level " (the lowest level at which what the',
		'-- quest pays for its time is far under what a quest of that level pays; tools/lib/slow-quests.js).',
		'local outlevelled =',
		...levelChunks(byLevel),
		'',
		'-- The code that makes quest `q` slow for a character of `level`: its row (List.Slow), else slow-xp once the',
		'-- character is at the level the quest turned slow at; nil when it is not slow.',
		'function List.SlowAt(q, level)',
		'\tlocal code = List.Slow(q)',
		'\tif code or type(q) ~= "number" or type(level) ~= "number" then return code end',
		'\tlocal from = tonumber(match(outlevelled, " " .. q .. ":(%d+) "))',
		'\tif from and level >= from then return "slow-xp" end',
		'\treturn nil',
		'end',
		'',
		'ns.SlowQuestList = List',
		'',
	].join('\n');
}

function ageDays(from, today) {
	return Math.floor((Date.parse(today) - Date.parse(from)) / 86400000);
}

function reportText({ rows, keep }, db, today, classQuests) {
	const medians = bandMedians(db, classQuests);
	const lines = ['row | code | score | overruled | age'];
	for (const r of rows) {
		const s = r.score ? `${r.score.xpm} XP/min, ${r.score.ratio} x median ${r.score.median}` : 'not scored';
		const age = ageDays(r.reviewed, today);
		lines.push(`slow ${r.name} (${r.quests.join(' ')}) | ${r.code} | ${s} | ${r.overruled ? 'yes: left out of the addon' : 'no'} | ${age} days${age > STALE_DAYS ? ' (review due)' : ''}`);
	}
	for (const r of keep) {
		const s = chainScore(db, medians, r.quests);
		const age = ageDays(r.reviewed, today);
		lines.push(`keep ${r.name} (${r.quests.join(' ')}) | - | ${s ? `${s.xpm} XP/min, ${s.ratio} x median ${s.median}` : 'not scored'} | - | ${age} days${age > STALE_DAYS ? ' (review due)' : ''}`);
	}
	return lines.join('\n');
}

module.exports = {
	name: 'slowquests',
	summary: 'Build the list of slow quests behavior.skipSlow leaves out (Core/SlowQuestList.lua)',
	usage,
	CODES,
	SCORED,
	MIN_SOURCES,
	STALE_DAYS,
	buildSlowList,
	classQuestSet,
	toLua,
	run(argv) {
		let values, positionals;
		try {
			({ values, positionals } = parseArgs({ args: argv, options: { list: { type: 'string' }, today: { type: 'string' } }, allowPositionals: true, strict: true }));
		} catch (err) {
			console.error(`${err.message}\n\n${usage}`);
			return 2;
		}
		const [sub, ...extra] = positionals;
		if (!['build', 'check', 'report'].includes(sub) || extra.length) {
			console.error(usage);
			return 2;
		}
		const list = JSON.parse(fs.readFileSync(path.resolve(values.list || LIST), 'utf8'));
		const db = loadDb();
		const classQuests = classQuestSet();
		let data;
		try {
			data = buildSlowList(list, db, classQuests);
		} catch (err) {
			console.error(err.message);
			return 1;
		}
		if (sub === 'report') {
			console.log(reportText(data, db, values.today || new Date().toISOString().slice(0, 10), classQuests));
			return 0;
		}
		return reportSync({ [OUT_NAME]: toLua(data) }, OUT_DIR, { check: sub === 'check', command: 'slowquests build' });
	},
};
