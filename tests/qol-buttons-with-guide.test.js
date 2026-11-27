'use strict';
// The action buttons and the guide window (QoL/Buttons.lua, Buttons.Shown):
// "Hide the buttons with the guide" (qol.hideButtonsWithGuide, on by default)
// takes the target, bag and the other action buttons with the window; off, they
// stay while the window is closed or the guide waits out a dungeon. Hiding
// Polestar takes them either way (#4403, #4290).

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { runAddon } = require('./helpers/lua');

const FILES = { name: 'Polestar', files: ['Core/Init.lua', 'Core/Compat.lua', 'Theme/Theme.xml', 'QoL/QoL.xml'] };

// A step with an NPC to target, and a bag that is full with one junk item to
// delete: the target and bag buttons have something to show.
const SETUP = String.raw`
	local q = stub.qol
	local Space = Polestar.QoL.Space
	Space.Low = function() return true end
	Space.Spare = function() return nil end
	Space.Junk = function() return { id = 900 } end
	Space.Free = function() return 1, 16 end
	Space.Armed = function() return false end
	-- The target button alone (the list shows its names in rows instead).
	Polestar.Set("qol.targetSimple", true); stub.runTimers()
	q.items[900] = { name = "Worn Boots", quality = 0, price = 1 }
	q.step({ { t = "talk", npcName = "Madame Eva", map = 9000, x = 50, y = 50 } })
	Polestar.QoL.Buttons.Refresh()
	local function shown()
		Polestar.QoL.Buttons.Refresh(); stub.runTimers()
		return { target = PolestarTargetButton:IsShown() and true or false, bag = PolestarBagButton:IsShown() and true or false }
	end
`;

function run(script) {
	return runAddon({ stubs: ['stub_qol.lua'], addons: [FILES], login: true, script: SETUP + script }).emitted;
}

test('by default the buttons go with the guide window and come back with it', () => {
	const [on, off, back] = run(`
		emit(shown())
		Polestar.Set("ui.shown", false); emit(shown())
		Polestar.Set("ui.shown", true); emit(shown())
	`);
	assert.deepEqual(on, { target: true, bag: true });
	assert.deepEqual(off, { target: false, bag: false });
	assert.deepEqual(back, { target: true, bag: true });
});

test('with the setting off, closing the guide window keeps the buttons', () => {
	const [off] = run(`
		Polestar.Set("qol.hideButtonsWithGuide", false)
		Polestar.Set("ui.shown", false); emit(shown())
	`);
	assert.deepEqual(off, { target: true, bag: true });
});

test('inside a dungeon the guide pauses and the buttons stay only with the setting off', () => {
	const [byDefault, kept, left] = run(`
		Polestar.SetPaused(true); q.enterInstance(100); emit(shown())
		Polestar.Set("qol.hideButtonsWithGuide", false); emit(shown())
		Polestar.SetPaused(false); q.leaveInstance(); emit(shown())
	`);
	assert.deepEqual(byDefault, { target: false, bag: false });
	assert.deepEqual(kept, { target: true, bag: true });
	assert.deepEqual(left, { target: true, bag: true });
});

test('hiding Polestar takes the buttons even with the setting off, and they come back', () => {
	const [hidden, back] = run(`
		Polestar.Set("qol.hideButtonsWithGuide", false)
		Polestar.Set("ui.shown", false)
		Polestar.SetHidden(true); stub.runTimers(); emit(shown())
		Polestar.SetHidden(false); stub.runTimers(); emit(shown())
	`);
	assert.deepEqual(hidden, { target: false, bag: false });
	assert.deepEqual(back, { target: true, bag: true });
});
