#!/usr/bin/env node
'use strict';
// The route gap test (tools/lib/flow/gap.js): how much walking, and how many trips back to an area already visited,
// the guide's order and the live step reorder cost over the shortest order that keeps the rules.
//
//   node tools/route-gap.js --addon <dir> [--data DIR] [--faction F] [--start LIST] [--class LIST] [--dynamic 042]
//                           [--ms N] [--seed N] [--out FILE] [--md FILE]
//
// --addon, --data, --faction, --start, --class, --dynamic read as in tools/flow-audit.js. --ms is the local search's
// time for each segment and order (default 600).

const fs = require('node:fs');
const { parseArgs } = require('./lib/cli');
const { DATA } = require('./lib/paths');
const { loadCompiled, loadData, timedQuests } = require('./lib/flow/data');
const { Walk, Router } = require('./lib/flow/walk');
const { branchesOf } = require('./lib/flow/audit');
const gap = require('./lib/flow/gap');

const USAGE = 'usage: node tools/route-gap.js --addon <dir> [--data DIR] [--faction F] [--start LIST] [--class LIST] [--dynamic 042] [--ms N] [--seed N] [--out FILE] [--md FILE]';
const listOf = (s) => (s ? String(s).split(',').map((x) => x.trim().toLowerCase()).filter(Boolean) : []);
const ORDERS = ['guide', 'live', 'steps', 'goals'];

function walkOf(b, G, compiled, data, dynamic, timed) {
	const common = { race: b.race, cls: b.cls, faction: b.faction, guides: G, legs: compiled.legs, data, longFly: 2500, dungeons: '', dungeonQuests: 'all', levelOffset: 0 };
	const w0 = new Walk({ ...common, chain: new Router(b.first, [], '', 'all', { guides: G, faction: b.faction }) });
	if (!dynamic) return w0;
	const future = new Map([...w0.turninAt].map(([q, i]) => [q, w0.entries[i].lv]));
	return new Walk({ ...common, chain: w0.chain, dyn: dynamic, future, timed, take: 'near', followups: true });
}

const pct = (a, b) => (b ? `${(100 * (a - b) / b).toFixed(1)}%` : '-');
const min = (yd) => (yd / 7 / 60).toFixed(1);

function render(res) {
	const lines = ['# Route gap test', '', `Branches: ${res.branches.join(', ')}. Yards are straight lines between goals; minutes at 7 yd/s.`, ''];
	const head = '| guide | goals | guide yd | live yd | best steps yd | best goals yd | live vs best goals | guide vs best goals | re-entries guide / live / best | there-and-back guide / live / best |';
	lines.push(head, `|${head.split('|').slice(1, -1).map(() => '---').join('|')}|`);
	const row = (name, t) => `| ${name} | ${t.guide.goals} | ${Math.round(t.guide.yd)} | ${Math.round(t.live.yd)} | ${Math.round(t.steps.yd)} | ${Math.round(t.goals.yd)} | ${pct(t.live.yd, t.goals.yd)} | ${pct(t.guide.yd, t.goals.yd)} | ${t.guide.reentries} / ${t.live.reentries} / ${t.goals.reentries} | ${t.guide.trips} / ${t.live.trips} / ${t.goals.trips} |`;
	for (const [g, t] of Object.entries(res.guides)) lines.push(row(g, t));
	lines.push(row('**all**', res.total), '');
	lines.push(`Walking minutes saved by the best order of goals: ${min(res.total.guide.yd - res.total.goals.yd)} over the guide's order, ${min(res.total.live.yd - res.total.goals.yd)} over the live reorder.`);
	if (res.check.n) lines.push(`Search against the exact best on ${res.check.n} small segments: equal on ${res.check.equal}, worst excess ${(100 * res.check.worst).toFixed(2)}%.`);
	return `${lines.join('\n')}\n`;
}

function main() {
	const { opts, error } = parseArgs(process.argv.slice(2), { flags: [], options: ['--addon', '--data', '--faction', '--start', '--class', '--dynamic', '--ms', '--seed', '--out', '--md'] });
	if (error || opts.help || !opts.addon) {
		console.error(error ? `route-gap: ${error}\n${USAGE}` : USAGE);
		process.exit(error ? 2 : opts.help ? 0 : 2);
	}
	const dataDir = opts.data || DATA;
	const data = loadData(dataDir);
	const compiled = loadCompiled(opts.addon);
	const G = new Map(compiled.guides.filter((g) => g.category === 'Leveling').map((g) => [g.id, g]));
	const dynamic = opts.dynamic === 'off' ? null : (opts.dynamic || '042');
	const timed = timedQuests(dataDir, data.quests);
	const locked = new Set(timed.keys());
	const starts = listOf(opts.start), classes = (opts.class || '').split(',').map((x) => x.trim()).filter(Boolean);
	const res = { branches: [], guides: {}, total: Object.fromEntries(ORDERS.map((o) => [o, gap.ZERO()])), check: { n: 0, equal: 0, worst: 0 }, segments: [] };
	const seed = Number(opts.seed || 1), ms = Number(opts.ms || 600);
	for (const b of branchesOf(G, data.races, { faction: opts.faction || '', starts, classes })) {
		res.branches.push(`${b.start}/${b.cls}`);
		const w = walkOf(b, G, compiled, data, dynamic, timed);
		for (const seg of gap.segmentsOf(w.entries, locked)) {
			const r = gap.gapOf(seg, data.quests, { seed, ms });
			const gid = seg.nodes[0].gid.replace(/^forever\//, '');
			if (!res.guides[gid]) res.guides[gid] = Object.fromEntries(ORDERS.map((o) => [o, gap.ZERO()]));
			for (const o of ORDERS) {
				res.guides[gid][o] = gap.add(res.guides[gid][o], r[o]);
				res.total[o] = gap.add(res.total[o], r[o]);
			}
			if (r.check) {
				res.check.n++;
				const ex = (r.check.search - r.check.exact) / Math.max(r.check.exact, 1);
				if (ex < 1e-9) res.check.equal++;
				res.check.worst = Math.max(res.check.worst, ex);
			}
			res.segments.push({ gid, goals: seg.nodes.length, first: seg.nodes[0].i, ...Object.fromEntries(ORDERS.map((o) => [o, r[o]])) });
		}
	}
	if (opts.out) fs.writeFileSync(opts.out, `${JSON.stringify(res)}\n`);
	const md = render(res);
	if (opts.md) fs.writeFileSync(opts.md, md);
	else process.stdout.write(md);
}

main();
