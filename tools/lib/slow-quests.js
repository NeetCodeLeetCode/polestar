'use strict';
// What "Skip slow quests" (tools/commands/slowquests.js) reads from forever-db: which quests exist, and which of them
// the quest database opens to some classes only. The list of slow quests is hand-written (data/codex/slow-quests.json):
// the build judges no kill count, drop chance or time.

const fs = require('node:fs');
const path = require('node:path');
const { ROOT } = require('./paths');

const DB_DIR = path.join(ROOT, 'forever-db', 'data');

function readJsonl(file) {
	return fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line));
}

// -> { quests: Map(id -> { level, xp, classes }) }
function loadDb(dir = DB_DIR) {
	const quests = new Map();
	for (const r of readJsonl(path.join(dir, 'quest.jsonl'))) quests.set(r.id, { level: r.level, xp: r.xp || 0, classes: r.classes || 0 });
	return { quests };
}

// Whether the quest database's class mask makes quest `info` a class quest: open to some classes only (the engine's
// Own.classQuest reads the same mask, 1503 being every class).
const ALL_CLASSES = 1503;
function isClassMask(info) {
	return info.classes !== 0 && (info.classes & ALL_CLASSES) !== ALL_CLASSES;
}

module.exports = { ALL_CLASSES, isClassMask, DB_DIR, loadDb };
