#!/usr/bin/env node
'use strict';
// The offline guide audit: the compiled guides (addon/Polestar_Guides*) against the public reference data
// (forever-db), per goal and along each race's route. It needs no data/forever and no game client.
//
//   node tools/guide-audit.js [--only goals,order,walking] [--severity error|warn|info] [--check NAME[,NAME]]
//                             [--guide ID-PART] [--limit N] [--json FILE] [--md FILE]
//
// Exit 1 when an error is found (--strict), else 0. The checks and what each means: tools/lib/guide-audit/report.js.

const fs = require('node:fs');
const { parseArgs } = require('./lib/cli');
const { runAudit, SEVERITY } = require('./lib/guide-audit');
const { markdown } = require('./lib/guide-audit/report');

const USAGE = 'usage: node tools/guide-audit.js [--only goals,order,walking] [--severity error|warn|info] [--check LIST] [--guide TEXT] [--limit N] [--json FILE] [--md FILE] [--strict]';
const list = (s) => (s ? String(s).split(',').map((x) => x.trim()).filter(Boolean) : null);

function main(argv) {
	const { opts: args, error } = parseArgs(argv, { flags: ['--strict'], options: ['--only', '--severity', '--check', '--guide', '--limit', '--json', '--md'] });
	if (error) throw new Error(error);
	if (args.help) return console.log(USAGE);
	const result = runAudit({ only: list(args.only) });
	const checks = list(args.check);
	const sev = args.severity || 'info';
	if (!SEVERITY.includes(sev)) throw new Error(`--severity: ${SEVERITY.join(', ')}`);
	result.findings = result.findings.filter((f) => (!checks || checks.includes(f.check)) && (!args.guide || f.guide.includes(args.guide)));
	if (args.json) fs.writeFileSync(args.json, JSON.stringify(result.findings, null, '\t') + '\n');
	const text = markdown(result, { limit: args.limit == null ? 15 : Number(args.limit), minSeverity: sev });
	if (args.md) fs.writeFileSync(args.md, text);
	else process.stdout.write(text);
	if (args.strict && result.findings.some((f) => f.severity === 'error')) process.exitCode = 1;
}

if (require.main === module) {
	try { main(process.argv.slice(2)); } catch (e) { console.error(e.message); console.error(USAGE); process.exit(2); }
}
module.exports = { main };
