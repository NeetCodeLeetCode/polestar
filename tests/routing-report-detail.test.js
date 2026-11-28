'use strict';
// The routing report's detail sections: the arrow's source and chain (Nav/ArrowLog.lua, UI/RoutingReportArrow.lua),
// Next stop's weighed places (Core/NextStopLog.lua, UI/RoutingReportNextStop.lua) and the pace, Fastest XP and route
// advice lines (UI/RoutingReportPace.lua). Against the real Core, Theme, Nav and UI and the fakes of tests/lua/stub_ui.lua.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { runAddon } = require('./helpers/lua');

const FILES = ['Core/Core.xml', 'Theme/Theme.xml', 'Nav/Nav.xml', 'UI/UI.xml'];

function run(script) {
	return runAddon({
		stubs: ['stub_world.lua', 'stub_ui.lua'],
		saved: { Polestar: 'PolestarDB = { settings = {} }' },
		addons: [{ name: 'Polestar', files: FILES }],
		script: `uistub.install({ noNav = true })
stub.login()
local GF, UI, N = Polestar.UI.GuideFrame, Polestar.UI, Polestar.Nav
local function report() return table.concat(UI.RoutingReport.Lines(), "\\n") end
${script}`,
	}).emitted;
}

const lineWith = (text, needle) => text.split('\n').find((l) => l.includes(needle));

test('a Next stop trip reads as a trip with its age, not as the player\'s own waypoint', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		local t = N.SetTarget(1429, 50, 50, "The Barrens")
		N.StartTrip(t, "The Barrens", "Next stop")
		emit({ text = report(), arrow = UI.Report.Details().lines })`);
	assert.ok(lineWith(out.text, 'Arrow source:').includes('a trip: Next stop to The Barrens'), out.text);
	assert.ok(lineWith(out.text, 'Arrow chain 2').includes('a trip: Next stop to The Barrens'), out.text);
	assert.ok(lineWith(out.text, 'Arrow chain 2').includes('THE ARROW FOLLOWS THIS'));
	assert.ok(!lineWith(out.text, 'Arrow chain 5').includes('THE ARROW FOLLOWS THIS'), 'the guide\'s goal does not lead');
	assert.ok(lineWith(out.text, 'Trip: Next stop to The Barrens'), out.text);
	assert.ok(lineWith(out.text, 'Arrow:').includes('a trip: Next stop to The Barrens'), 'the bug report\'s Arrow line names the trip too');
	assert.ok(!lineWith(out.text, 'Arrow:').includes('your own waypoint'));
});

test('a point the player picked reads as their own waypoint, with the guide\'s own goal and its distance beside it', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		local guide = report()
		local t = N.SetTarget(1429, 50, 50, "Spot")
		t.mine = true
		emit({ guide = guide, own = report() })`);
	assert.ok(lineWith(out.guide, 'Guide goal: step 3 goal 3'), out.guide);
	assert.ok(lineWith(out.guide, 'Arrow chain 5').includes('step 3 goal 3'), out.guide);
	assert.ok(!out.guide.includes('THE ARROW FOLLOWS THIS'), 'no link holds the arrow when the guide has none');
	assert.ok(lineWith(out.own, 'Arrow source:').includes('your own waypoint'), out.own);
	assert.ok(lineWith(out.own, 'The arrow is NOT on the guide'), out.own);
});

test('the arrow log lists each change with its time and the event it followed', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		local AL = N.ArrowLog
		AL.Clear()
		local t = N.SetTarget(1429, 50, 50, "The Barrens")
		N.StartTrip(t, "The Barrens", "Next stop")
		stub.advance(30)
		N.EndTrip(true)
		stub.advance(1)
		emit({ n = #AL.Entries(), text = report(), first = AL.Entries()[1].source })`);
	assert.ok(out.n >= 2, `${out.n} entries`);
	assert.equal(out.first, 'trip', 'the point set for a trip is logged as the trip');
	const log = out.text.split('\n').filter((l) => l.startsWith('  ') && l.includes('because'));
	assert.ok(log.some((l) => l.includes('a trip started: Next stop to The Barrens')), out.text);
	assert.ok(log.some((l) => l.includes('the trip ended')), out.text);
});

test('Next stop: the report lists every place with its numbers, the winner and the reason each one lost', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		local NS, Log = Polestar.NextStop, Polestar.NextStopLog
		local here = { id = "hub:1:1", kind = "hub", name = "Razor Hill", zone = "Durotar", tripSecs = 0, quests = { { xp = 3, secs = 240 } } }
		local zone = { id = "zone:2", kind = "zone", name = "The Barrens", zone = "The Barrens", tripSecs = 505, quests = { { xp = 15, secs = 240 }, { xp = 15, secs = 240 } } }
		local near = { id = "hub:1:2", kind = "hub", name = "Sen'jin Village", zone = "Durotar", tripSecs = 120, quests = { { xp = 2, secs = 240 } } }
		local dungeon = { id = "dungeon:rfc", kind = "dungeon", name = "Ragefire Chasm", zone = "Orgrimmar", tripSecs = 300, quests = { { xp = 20, secs = 2400 } } }
		local empty = { id = "zone:3", kind = "zone", name = "Mulgore", zone = "Mulgore", tripSecs = 900, quests = {} }
		local places = { here, zone, near, dungeon, empty }
		local state = { level = 12, group = 1, dungeons = "skip", here = here }
		local r = NS.Rank(state, places)
		Log.Look("card shown", { level = 12, group = 1, dungeons = "skip", here = here, places = places, ranking = r, guideSecs = 400 }, true)
		emit({ text = report(), top = r.top.place.name })`);
	assert.equal(out.top, 'The Barrens');
	const t = out.text;
	assert.ok(lineWith(t, 'Next stop latest look:').includes('card shown'), t);
	assert.ok(lineWith(t, 'Next stop guide check:').includes('400 s away'), t);
	assert.ok(lineWith(t, 'Next stop weighed when the card showed').includes('standing at Razor Hill'), t);
	const rows = t.split('\n').filter((l) => /^ {2}\d+\. /.test(l));
	assert.equal(rows.length, 3, 'the hub, the zone and the near hub are ranked');
	assert.ok(rows[0].includes('The Barrens (zone') && rows[0].includes('chosen'), rows[0]);
	assert.ok(rows[0].includes('trip 505 s + work 480 s'), rows[0]);
	assert.ok(rows.some((l) => l.includes('Razor Hill') && l.includes('stay:')), t);
	assert.ok(rows.some((l) => l.includes("Sen'jin Village") && l.includes('lost:')), t);
	assert.ok(lineWith(t, 'Left out, dungeons: Ragefire Chasm').includes('needs a group of 3 (you are 1)'), t);
	assert.ok(lineWith(t, 'Left out, no quest pays XP').includes('Mulgore'), t);
});

test('Next stop: with nothing logged, the report says no look yet; a latest look that stopped early says why', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		local first = report()
		Polestar.NextStopLog.Look("no card: the guide has work here", { guideSecs = 20 })
		emit({ first = first, second = report() })`);
	assert.ok(lineWith(out.first, 'Next stop looks:').includes('none yet'), out.first);
	assert.ok(lineWith(out.second, 'Next stop latest look:').includes('the guide has work here'), out.second);
	assert.ok(lineWith(out.second, 'Next stop guide check:').includes('20 s away'), out.second);
});

test('a section that fails is named in the report and the rest still print', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		UI.RoutingReport.AddSection("broken", function() error("boom") end, "before")
		emit(report())`);
	assert.ok(/Section broken failed: .*boom/.test(out), out);
	assert.ok(out.includes('Routing settings:'), 'the settings still print');
});

test('the report keeps the player\'s name out of the new lines too', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		UnitName = function(unit) if unit == "player" then return "Testchar" end end
		local t = N.SetTarget(1429, 50, 50, "Walk to Testchar")
		N.StartTrip(t, "Testchar's camp", "Next stop")
		UI.ShowRoutingReport()
		emit(PolestarCopy.ScrollBox and PolestarCopy.ScrollBox:GetText() or PolestarCopy.Box:GetText())`);
	assert.ok(!out.toLowerCase().includes('testchar'), out);
	assert.ok(out.includes('<character>'), out);
});

test('the pace lines give the XP rate, the session and the guide\'s pace numbers', () => {
	const [out] = run(`
		uistub.setGuide(uistub.elwynn, 3)
		Polestar.QoL = Polestar.QoL or {}
		Polestar.QoL.GetXPStats = function()
			return { level = 12, percent = 45.2, rollingRate = 23400, sessionRate = 19800, timeToLevel = 600, restedPercent = 0,
				sessionSeconds = 2280, sessionXP = 12000, sessionQuests = 12, sessionKills = 87, levelSeconds = 370, avgKillXP = 120.4, avgQuestXP = 410 }
		end
		Polestar.Levels.Pace = function() return { target = 17, step = 58, reach = 15.3, bars = 34, short = 1.7, xp = 5400, kind = "goal", live = 55 } end
		emit(report())`);
	assert.ok(lineWith(out, 'Level pace:').includes('level 12, 45.2% into it'), out);
	assert.ok(lineWith(out, 'Level pace:').includes('23400 XP/h now, 19800 for the session'), out);
	assert.ok(lineWith(out, 'Session:').includes('12 quests') && lineWith(out, 'Session:').includes('87 kills'), out);
	assert.ok(lineWith(out, 'Averages:').includes('a kill pays 120 XP'), out);
	assert.ok(lineWith(out, 'Guide pace numbers:').includes('target level 17 at step 58'), out);
	assert.ok(lineWith(out, 'Guide pace numbers:').includes('1.70 levels short (34 bars, 5400 XP)'), out);
	assert.ok(lineWith(out, 'Fastest XP: '), out);
	assert.ok(lineWith(out, 'Route advice: off'), out);
});

test('RouteAdvice.Why names the rule that keeps an entry from drawing advice, and the judge agrees with it', () => {
	const [out] = run(`
		local RA = Polestar.RouteAdvice
		local cases = {
			{ kind = "off", seconds = 100, age = 9999 },
			{ kind = "off", seconds = 500, age = 9999 },
			{ kind = "off", seconds = 500, age = 9999, ready = true },
			{ kind = "off", seconds = 500, age = 10 },
			{ kind = "off", seconds = 500, age = 9999, kept = true },
			{ kind = "off", seconds = nil, age = 9999 },
			{ kind = "missed", seconds = 900, needed = true },
			{ kind = "missed", seconds = 900 },
			{ kind = "listed", age = 9999, progress = 0.8 },
			{ kind = "listed", age = 9999, progress = 0.1 },
		}
		local res = {}
		for i, e in ipairs(cases) do
			e.quest = i
			res[i] = { why = RA.Why(e, 10) or false, judged = #RA.Judge({ e }, 10) }
		end
		emit(res)`);
	const why = out.map((r) => r.why);
	assert.deepEqual(why, ['its detour is within the limit', false, 'finished: it is handed in, never abandoned', 'taken less than 600 s ago',
		'you chose Keep it', 'no detour measured', 'a later accept of the guide needs it', false, 'more than half done', false]);
	assert.deepEqual(out.map((r) => r.judged), [0, 1, 0, 0, 0, 0, 0, 1, 0, 1], 'advice is due exactly where there is no reason against it');
});
