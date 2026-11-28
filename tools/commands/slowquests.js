'use strict';
// node tools/polestar.js slowquests build|check|report
//
// The quests "Skip slow quests" (behavior.skipSlow) leaves out: the hand-written list data/codex/slow-quests.json
// names chains that cost far more time than they pay, each with 2 or more agreeing sources and a review date. The
// build judges no kill count, drop chance or time: a row is slow because people agree it is. build writes
// addon/Polestar/Core/SlowQuestList.lua; check exits 1 when the committed file differs; report prints each row's age.

const fs = require('node:fs');
const path = require('node:path');
const { parseArgs } = require('node:util');
const { header, reportSync } = require('../lib/generated');
const { ROOT } = require('../lib/paths');
const { loadDb, isClassMask } = require('../lib/slow-quests');
const { classGuides } = require('./classquests');

const usage = `usage: node tools/polestar.js slowquests build [--list <file>]
       node tools/polestar.js slowquests check [--list <file>]
       node tools/polestar.js slowquests report [--list <file>] [--today <yyyy-mm-dd>]

build: writes addon/Polestar/Core/SlowQuestList.lua from the list and forever-db. check: exits 1 when the committed
file differs. report: each row's age, and whether a review is due.

  --list <file>   the list (default: data/codex/slow-quests.json)
  --today <date>  the day report counts a row's age from (default: today)`;

const LIST = path.join(ROOT, 'data', 'codex', 'slow-quests.json');
const OUT_DIR = path.join(ROOT, 'addon', 'Polestar', 'Core');
const OUT_NAME = 'SlowQuestList.lua';
// Why a row is slow: what its sources say.
const CODES = ['low-drop', 'slow-respawn', 'slow-xp'];
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

// The list checked against forever-db. `classQuests`: the quests of the Class guides. A class quest (one of those, or
// one the quest database opens to some classes only) is never a row.
// -> { rows, keep, codes: { code -> sorted quest ids }, time }
// Throws on a quest forever-db does not know, a quest in two rows or in both slow and keep, an unknown code, a row
// with no reason or review date, or one with fewer than MIN_SOURCES sources.
function buildSlowList(list, db, classQuests = new Set()) {
	const errors = [];
	const time = list.time || {};
	for (const k of ['killMinutes', 'baseMinutes']) if (!(typeof time[k] === 'number' && time[k] > 0)) errors.push(`time.${k} must be a number of minutes over 0`);
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

	const codes = {};
	for (const row of list.slow || []) (codes[row.code] = codes[row.code] || []).push(...row.quests);
	for (const code of Object.keys(codes)) codes[code].sort((a, b) => a - b);
	return { rows: list.slow || [], keep: list.keep || [], codes, time: list.time };
}

// One string of ids per code, read with string.find (Polestar's core at login has a memory budget).
function toLua({ codes, time }) {
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
		'-- why: low-drop, slow-respawn, slow-xp. The list is hand-written: no kill count, drop chance or time is judged.',
		'-- Read by Core/Engine.lua (Own.slowSkipped).',
		'',
		'local ADDON, ns = ...',
		'local find = string.find',
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
		'-- The time model of the step reorder (data/codex/slow-quests.json, `time`): minutes a kill takes, minutes of',
		'-- talk at both ends of a quest. Core/StepOrder.lua counts a step\'s work with them. Skip slow quests does not.',
		`List.KILL_MINUTES, List.BASE_MINUTES = ${time.killMinutes}, ${time.baseMinutes}`,
		'',
		'ns.SlowQuestList = List',
		'',
	].join('\n');
}

function ageDays(from, today) {
	return Math.floor((Date.parse(today) - Date.parse(from)) / 86400000);
}

function reportText({ rows, keep }, today) {
	const lines = ['row | code | age'];
	for (const r of rows) {
		const age = ageDays(r.reviewed, today);
		lines.push(`slow ${r.name} (${r.quests.join(' ')}) | ${r.code} | ${age} days${age > STALE_DAYS ? ' (review due)' : ''}`);
	}
	for (const r of keep) {
		const age = ageDays(r.reviewed, today);
		lines.push(`keep ${r.name} (${r.quests.join(' ')}) | - | ${age} days${age > STALE_DAYS ? ' (review due)' : ''}`);
	}
	return lines.join('\n');
}

module.exports = {
	name: 'slowquests',
	summary: 'Build the list of slow quests behavior.skipSlow leaves out (Core/SlowQuestList.lua)',
	usage,
	CODES,
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
			console.log(reportText(data, values.today || new Date().toISOString().slice(0, 10)));
			return 0;
		}
		return reportSync({ [OUT_NAME]: toLua(data) }, OUT_DIR, { check: sub === 'check', command: 'slowquests build' });
	},
};
