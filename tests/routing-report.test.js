'use strict';
// The routing report (addon/Polestar/UI/RoutingReport.lua): the guide window's menu row and /polestar routing
// open the copy window on a report of what the arrow and the step order weighed, the bug report's details
// and the routing settings, selected for Ctrl+C, with no name that identifies the player. Against the real
// Core, Theme and UI and the fakes of tests/lua/stub_ui.lua.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { runAddon } = require('./helpers/lua');

const FILES = ['Core/Core.xml', 'Theme/Theme.xml', 'UI/UI.xml'];

// The copy window's box in view: the scrolling one for a long text, else the plain one.
const COPY = `
local function copyBox()
	local C = PolestarCopy
	if rawget(C, "ScrollWell") and C.ScrollWell:IsShown() then return C.ScrollBox, true end
	return C.Box, false
end
`;

function run(script, before = '') {
	return runAddon({
		stubs: ['stub_ui.lua'],
		before,
		addons: [{ name: 'Polestar', files: FILES }],
		script: `uistub.install()\nstub.login()\nlocal GF, UI = Polestar.UI.GuideFrame, Polestar.UI\n${COPY}\n${script}`,
	}).emitted;
}

const lineWith = (text, needle) => text.split('\n').find((l) => l.includes(needle));

// uistub's Engine is a fake: it has no RoutingState. This is the state of an Engine with the reorder and the
// arrow that takes nearby objectives off (the defaults); tests change it.
const STATE = `
local state = {
	reorder = { on = false, live = 3, take = 20, switch = 30, release = 5 },
	aim = { on = false, limit = 15, trigger = 3, release = 1.5, margin = 1.3, free = 3 },
	arrowFrom = "guide",
}
Polestar.Engine.RoutingState = function() return state end
`;

test('/polestar routing opens the copy window on the report, selected, with the routing lines before the bug report\'s details', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		${STATE}
		Polestar.RunCommand("routing")
		local C = PolestarCopy
		local box, scrolls = copyBox()
		emit({ shown = C:IsShown(), title = C.Title:GetText(), scrolls = scrolls, text = box:GetText(),
			focus = stub.focus == box, height = C:GetHeight(), width = C:GetWidth() })`);
	assert.equal(out.shown, true);
	assert.equal(out.title, 'Routing report');
	assert.equal(out.focus, true, 'the text is selected for Ctrl+C');
	assert.ok(out.height <= 720, `the window fits the screen (${out.height})`);
	assert.equal(out.width, 520);
	const lines = out.text.split('\n');
	assert.equal(lines[0], 'Polestar routing report');
	assert.equal(lines[1], 'Step reorder: off', 'off by default');
	assert.equal(lines[2], 'Take nearby objectives: off');
	assert.equal(lines[3], 'Arrow follows: the guide\'s own next goal');
	const settings = lines.findIndex((l) => l.startsWith('Routing settings: '));
	assert.equal(settings, 4);
	assert.ok(lines[settings + 1].startsWith('  behavior.'), 'the settings wrap into lines of about 100 characters');
	assert.ok(lines.slice(settings, settings + 3).every((l) => l.length <= 101), 'no line runs past the box');
	for (const s of ['behavior.stepReorder=off', 'behavior.whileHereAim=off', 'behavior.whileHereRadius=180', 'nav.routeStyle=balanced']) {
		assert.ok(out.text.includes(s), `${s} in the report`);
	}
	assert.ok(!out.text.includes('skipSlow'), 'a setting this Polestar does not have is left out');
	assert.ok(lines.includes('Guide: Elwynn Forest (forever/alliance/human/01-elwynn-forest)'), 'the bug report\'s details follow');
	assert.ok(lineWith(out.text, 'Arrow:').includes('yd'), out.text);
	assert.ok(lineWith(out.text, 'Where:').includes('Elwynn Forest'), out.text);
	assert.ok(!out.text.includes('!PSR1!'), 'the packed settings line is for the Discord bot');
});

test('the guide window\'s menu has a Copy routing report row that opens it', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		local W = Polestar.Widgets
		local got
		local show = W.ShowMenu
		W.ShowMenu = function(_, items) got = items end
		GF.OpenMenu(GF.menuButton)
		W.ShowMenu = show
		local row
		for _, it in ipairs(got) do if it.text == "Copy routing report" then row = it end end
		emit({ found = row ~= nil, enabled = row and row.disabled ~= true, tip = row and row.tip ~= nil })
		row.onClick()
		emit({ shown = PolestarCopy:IsShown(), title = PolestarCopy.Title:GetText() })`);
	assert.deepEqual(out, { found: true, enabled: true, tip: true });
});

test('with the reorder and the objective-taking arrow on, the report says what each weighed and chose', () => {
	const out = run(`
		uistub.setGuide(uistub.elwynn, 3)
		${STATE}
		state.reorder = { on = true, live = 3, shown = 5, why = "showing the step that walks least", rest = 0, take = 20, switch = 30, release = 5,
			plan = { 5, 3, 4 }, last = { ago = 2, n = 3, order = { 5, 3, 4 }, base = 800, cost = 520, saved = 40.04 },
			window = { { step = 3, ready = true, quests = 1 }, { step = 4, ready = false, quests = 2 }, { step = 5, ready = true, quests = 1 } } }
		state.aim = { on = true, limit = 15, trigger = 3, release = 1.5, margin = 1.3, free = 3, held = { quest = 783, step = 3 },
			why = "holding the line it took", ago = 1,
			last = { n = 2, first = 2, score = 12.34, secs = 9, later = 4.2, held = 2, lines = {
				{ quest = 7, value = 5, score = 3.2, secs = 14 }, { quest = 783, value = 8.46, score = 12.34, secs = 9 } } } }
		state.arrowFrom = "auto"
		state.recalibrated = { count = 2, ago = 40 }
		Polestar.Set("behavior.stepReorder", true)
		Polestar.Set("behavior.whileHereAim", true)
		UI.ShowRoutingReport()
		local box, scrolls = copyBox()
		local C = PolestarCopy
		emit({ text = box:GetText(), scrolls = scrolls, well = C.ScrollWell:GetHeight(), window = C:GetHeight(), plain = rawget(C, "Box") ~= nil and C.Box:IsShown() })
		-- The invite after it: the plain box again, at the window's own width.
		UI.ShowDiscord()
		emit({ plain = C.Box:IsShown(), well = C.ScrollWell:IsShown(), width = C:GetWidth(), text = C.Box:GetText() })`);
	assert.equal(out[0].scrolls, true, 'a report of more than 16 lines scrolls in a well');
	assert.equal(out[0].plain, false);
	assert.ok(out[0].well <= 16 * 12 + 16, `the well shows at most 16 lines (${out[0].well}px)`);
	assert.ok(out[0].window <= 720, `the window fits the screen (${out[0].window})`);
	assert.deepEqual(out[1], { plain: true, well: false, width: 360, text: 'https://discord.gg/polestar' }, 'the copy window is itself again for the next text');
	const lines = out[0].text.split('\n');
	assert.equal(lines[1], 'Step reorder: on \u00b7 live step 3 \u00b7 on screen step 5 \u00b7 showing the step that walks least');
	assert.equal(lines[2], "Reorder walk: best order 5 3 4 saves 40.0 s \u00b7 520 yd against the guide's 800 yd \u00b7 measured 2 s ago");
	assert.equal(lines[3], 'Reorder needs: a saving of 20 s to show a step, 30 s more to switch, under 5 s to go back');
	assert.equal(lines[4], 'Reorder plan: 5 3 4');
	assert.equal(lines[5], 'Reorder window: step 3, 1 quest; step 4 (waits for a hand-in or a pick-up), 2 quests; step 5, 1 quest');
	assert.equal(lines[6], 'Take nearby objectives: on \u00b7 up to 15 s of extra walking \u00b7 holding the line of quest 783 \u00b7 holding the line it took (1 s ago)');
	assert.equal(lines[7], 'Arrow follows: an objective it took by itself');
	assert.equal(lines[8], 'Objective 1: quest 7 \u00b7 worth 5.0% of a level \u00b7 3.2 a minute, 14.0 s extra');
	assert.equal(lines[9], 'Objective 2: quest 783 \u00b7 worth 8.5% of a level \u00b7 12.3 a minute, 9.0 s extra \u00b7 held');
	assert.equal(lines[10], 'Best path: starts with objective 2 \u00b7 scores 12.3 a minute, 9.0 s extra \u00b7 4.2 after a later stop \u00b7 takes at 3.0, lets go under 1.5, a swap needs 1.3 times as much');
	assert.equal(lines[11], 'Recalibrate: pressed 2 times this session \u00b7 last 40 s ago');
	assert.ok(lines[12].startsWith('Routing settings: '), lines[12]);
	assert.ok(out[0].text.includes('behavior.stepReorder=on') && out[0].text.includes('behavior.whileHereAim=on'));
});

test('the report holds no name that identifies the player: character, realm, guild, party, raid and the account\'s other characters', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		${STATE}
		UnitName = function(unit)
			if unit == "player" then return "Testchar" end
			if unit == "party1" then return "Partyfriend" end
			if unit == "raid7" then return "Raidbuddy" end
			if unit == "pet" then return "Fluffy" end
		end
		GetGuildInfo = function() return "Night Owls", "Officer", 1 end
		Polestar.DB().alts = { chars = { ["Altone-Test Realm"] = { name = "Altone", realm = "Test Realm" } } }
		-- Every name the player could carry into a reason, an error or a label.
		state.reorder.on = true
		state.reorder.why = "testchar walked to Partyfriend with Raidbuddy, Fluffy and Altone of <Night Owls> on Test Realm and TestRealm"
		UI.ShowRoutingReport()
		emit((copyBox()):GetText())`);
	for (const name of ['Testchar', 'Partyfriend', 'Raidbuddy', 'Fluffy', 'Altone', 'Night Owls', 'Test Realm', 'TestRealm']) {
		assert.ok(!out.toLowerCase().includes(name.toLowerCase()), `${name} is kept out: ${out}`);
	}
	assert.ok(out.includes('<character>') && out.includes('<party member>') && out.includes('<pet>') && out.includes('<realm>') && out.includes('<guild>'), out);
});

test('Scrub swaps names for placeholders as whole words only, in any case, and clears links, tags, addresses and folders', () => {
	const out = run(String.raw`
		local RR = Polestar.UI.RoutingReport
		local names = { { "Bob", "<character>" }, { "Bobby Tables", "<party member>" } }
		emit(RR.Scrub("bob and BOB, Bobcat, Bobby Tables, (Bob), Bob-Test", names))
		emit(RR.Scrub("|Hplayer:Someone-Realm:12|h[Someone]|h said hi; Name#12345 wrote me@example.com", {}))
		emit(RR.Scrub("C:\\Program Files (x86)\\World of Warcraft\\_classic_\\Interface\\AddOns\\Polestar\\Core\\Engine.lua:12: oops", {}))
		emit(RR.Scrub("C:\\Users\\jdoe\\Desktop\\x /home/jdoe/x /Users/jdoe/x", {}))
		emit(RR.Scrub("WTF/Account/MYACCOUNT/Realm/Char/SavedVariables/Polestar.lua", {}))
		emit(RR.Scrub("quest 783 at map 1429 @47.60,35.10 live", {}))`);
	assert.equal(out[0], '<character> and <character>, Bobcat, <party member>, (<character>), <character>-Test', 'whole words only: Bobcat stays');
	assert.equal(out[1], '<player> said hi; <battletag> wrote <email>');
	assert.equal(out[2], String.raw`Interface/AddOns\Polestar\Core\Engine.lua:12: oops`, 'the drive and the install go; the addon path stays');
	assert.equal(out[3], '<user folder>\\Desktop\\x <user folder>/x <user folder>/x');
	assert.equal(out[4], 'WTF/Account/<account>');
	assert.equal(out[5], 'quest 783 at map 1429 @47.60,35.10 live', 'ids and coordinates stay: they are what the report is for');
});
