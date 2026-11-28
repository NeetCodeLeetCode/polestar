'use strict';
// Small shared helpers for the checks.

const { hasBit, RACES, CLASSES } = require('../game/races');

// Is a race (file name) allowed by a quest's race mask on a side? 0 means every race.
function raceAllowed(mask, raceFile, faction) {
	if (!mask) return true;
	const r = Object.values(RACES).find((x) => x.file === raceFile);
	const bit = r && r.sides[faction];
	return !!bit && hasBit(mask, bit);
}

function classAllowed(mask, classFile) {
	if (!mask) return true;
	const c = Object.values(CLASSES).find((x) => x.file === classFile);
	return !!c && hasBit(mask, c.bit);
}

// Quest side of a faction: 1 Alliance, 2 Horde (0 both).
const sideOk = (qside, faction) => !qside || qside === (faction === 'Alliance' ? 1 : 2);

// The classes each race can play at level 1 in Classic Era. Skyborne is Forever's own race: no list, every class.
const RACE_CLASSES = {
	Human: ['WARRIOR', 'PALADIN', 'ROGUE', 'PRIEST', 'MAGE', 'WARLOCK'],
	Dwarf: ['WARRIOR', 'PALADIN', 'HUNTER', 'ROGUE', 'PRIEST'],
	NightElf: ['WARRIOR', 'HUNTER', 'ROGUE', 'PRIEST', 'DRUID'],
	Gnome: ['WARRIOR', 'ROGUE', 'MAGE', 'WARLOCK'],
	Orc: ['WARRIOR', 'HUNTER', 'ROGUE', 'SHAMAN', 'WARLOCK'],
	Scourge: ['WARRIOR', 'ROGUE', 'PRIEST', 'MAGE', 'WARLOCK'],
	Tauren: ['WARRIOR', 'HUNTER', 'SHAMAN', 'DRUID'],
	Troll: ['WARRIOR', 'HUNTER', 'ROGUE', 'PRIEST', 'SHAMAN', 'MAGE'],
};
// Can this race be one of these classes? (A step for Warlocks is not played by a Dwarf.)
const raceCanBe = (raceFile, classes) => !classes || !RACE_CLASSES[raceFile] || classes.some((c) => RACE_CLASSES[raceFile].includes(c));

const norm = (s) => String(s || '').toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim();

// The `{kind:id}` tokens a guide text holds.
function tokensOf(text) {
	const out = [];
	for (const m of String(text || '').matchAll(/\{(npc|quest|object|item|area|zone):(\d+)\}/g)) out.push({ kind: m[1], id: Number(m[2]) });
	return out;
}

module.exports = { raceCanBe, raceAllowed, classAllowed, sideOk, norm, tokensOf };
