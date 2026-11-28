'use strict';
// The offline guide audit: the compiled guides against the public reference data, per goal and along each race's
// route. runAudit() returns merged findings; report.js writes them.

const { loadRef } = require('./refdata');
const { loadCompiled } = require('./compiled');
const goals = require('./checks/goals');
const order = require('./checks/order');
const walking = require('./checks/walking');

const CHECKS = { goals, order, walking };
const SEVERITY = ['error', 'warn', 'info'];

// One finding per (check, guide, step, goal, message); the branches (races) that meet it are listed.
function collector() {
	const map = new Map();
	const add = (f) => {
		const goal = f.goal || null;
		const key = [f.check, f.guide, f.step, goal && goal.t, goal && goal.quest, f.message].join('|');
		let row = map.get(key);
		if (!row) {
			row = {
				check: f.check, severity: f.severity, guide: f.guide, step: f.step, message: f.message, branches: new Set(),
				quest: goal && goal.quest != null ? goal.quest : null, goal: goal && goal.t, yards: f.yards || 0,
			};
			map.set(key, row);
		}
		if (f.branch) row.branches.add(f.branch);
	};
	return { add, rows: () => [...map.values()].map((r) => ({ ...r, branches: [...r.branches].sort() })) };
}

// Quest id -> the ids of the guides that hand it in (any category).
function providersOf(guides) {
	const m = new Map();
	for (const g of guides.values()) {
		for (const step of g.steps) for (const goal of step.goals) {
			if (goal.t !== 'turnin' || goal.quest == null) continue;
			if (!m.has(goal.quest)) m.set(goal.quest, new Set());
			m.get(goal.quest).add(g.id);
		}
	}
	return m;
}

function runAudit({ ref = loadRef(), guides = loadCompiled(), only = null } = {}) {
	const ctx = { ref, guides, providers: providersOf(guides) };
	const c = collector();
	for (const [name, mod] of Object.entries(CHECKS)) if (!only || only.includes(name)) mod.run(ctx, c.add);
	const rows = c.rows().sort((a, b) => SEVERITY.indexOf(a.severity) - SEVERITY.indexOf(b.severity) || a.check.localeCompare(b.check) || a.guide.localeCompare(b.guide) || a.step - b.step);
	return { ctx, findings: rows };
}

module.exports = { runAudit, CHECKS, SEVERITY };
