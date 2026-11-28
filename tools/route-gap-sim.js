#!/usr/bin/env node
'use strict';
// The route gap test on simulated runs (docs/ROUTE-FEEL.md, tools/lib/flow/gap.js): the campaign's JSON lines
// (engine_scenarios::campaign with SCENARIO_OUT) carry the places each run did its goals. For every run this prints the
// yards walked between goals against the best order of the same goals, and the areas entered again.
//
//   node tools/route-gap-sim.js <campaign.jsonl> [--data DIR] [--ms N] [--out FILE]

const fs = require('node:fs');
const { parseArgs } = require('./lib/cli');
const { DATA } = require('./lib/paths');
const { loadData, timedQuests } = require('./lib/flow/data');
const gap = require('./lib/flow/gap');

const USAGE = 'usage: node tools/route-gap-sim.js <campaign.jsonl> [--data DIR] [--ms N] [--out FILE]';
const median = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.floor(s.length / 2)] : 0; };

function main() {
	const { opts, positional, error } = parseArgs(process.argv.slice(2), { flags: [], options: ['--data', '--ms', '--out'] });
	if (error || opts.help || positional.length !== 1) {
		console.error(error ? `route-gap-sim: ${error}\n${USAGE}` : USAGE);
		process.exit(opts.help ? 0 : 2);
	}
	const dataDir = opts.data || DATA;
	const data = loadData(dataDir);
	const locked = new Set(timedQuests(dataDir, data.quests).keys());
	const rows = fs.readFileSync(positional[0], 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
	const out = [];
	for (const r of rows) {
		if (!Array.isArray(r.visits) || !r.visits.length) continue;
		const g = gap.visitsGap(r.visits, data.quests, locked, { seed: r.seed || 1, ms: Number(opts.ms || 400) });
		out.push({ name: r.name, family: r.family, seed: r.seed, arm: r.arm, goals: r.visits.length, handins: r.handins, seconds: r.seconds, sim_yards: r.yards,
			actual: g.actual, guide: g.guide, best: g.best });
	}
	if (opts.out) fs.writeFileSync(opts.out, `${JSON.stringify(out)}\n`);
	const lines = ['| character | arm | runs | goals | yd between goals | guide order of the same goals | best order yd | excess over best | re-entries now / best | there-and-back now / best |', '|---|---|---|---|---|---|---|---|---|---|'];
	const by = new Map();
	for (const o of out) {
		const k = `${o.name}|${o.arm}`;
		if (!by.has(k)) by.set(k, []);
		by.get(k).push(o);
	}
	const tot = { D: [], F: [] };
	for (const [k, rs] of by) {
		const [name, arm] = k.split('|');
		const sum = (f) => rs.reduce((s, o) => s + f(o), 0);
		const a = sum((o) => o.actual.yd), b = sum((o) => o.best.yd);
		tot[arm].push(...rs);
		lines.push(`| ${name} | ${arm} | ${rs.length} | ${Math.round(sum((o) => o.goals) / rs.length)} | ${Math.round(a / rs.length)} | ${Math.round(sum((o) => o.guide.yd) / rs.length)} | ${Math.round(b / rs.length)} | ${(100 * (a - b) / b).toFixed(1)}% | ${(sum((o) => o.actual.reentries) / rs.length).toFixed(1)} / ${(sum((o) => o.best.reentries) / rs.length).toFixed(1)} | ${(sum((o) => o.actual.trips) / rs.length).toFixed(1)} / ${(sum((o) => o.best.trips) / rs.length).toFixed(1)} |`);
	}
	for (const arm of ['D', 'F']) {
		const rs = tot[arm];
		if (!rs.length) continue;
		const ex = rs.map((o) => 100 * (o.actual.yd - o.best.yd) / Math.max(o.best.yd, 1));
		const wasted = rs.map((o) => (o.actual.yd - o.best.yd) / 7 / 60);
		lines.push('', `${arm === 'D' ? 'Guide order' : 'Fastest XP'}: ${rs.length} runs, excess over the best order of the same goals: median ${median(ex).toFixed(1)}%, worst ${Math.max(...ex).toFixed(1)}%; walking minutes above the best order: median ${median(wasted).toFixed(1)}, worst ${Math.max(...wasted).toFixed(1)}; re-entries median ${median(rs.map((o) => o.actual.reentries))}, there-and-back median ${median(rs.map((o) => o.actual.trips))}.`);
	}
	process.stdout.write(`${lines.join('\n')}\n`);
}

main();
