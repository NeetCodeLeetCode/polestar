//! Engine.RoutingState (addon/Polestar/Core/Engine.lua): what the arrow that takes nearby objectives and the live step
//! reorder weighed and chose in their last polls, for the routing report (UI/RoutingReport.lua). Nothing is computed
//! when it is asked: the numbers are the polls'. Real Core manifest, stub world.
//!
//! Elwynn here is 3000 yd across (30 yd per map percent on x) and 2000 yd high (20 yd per percent on y).

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.level = 5
w.zone, w.subzone = "Elwynn Forest", ""
w.moveTo(1429, 47.6, 45.1)
w.onQuest(33, { 8 }, "Wolves Across the Border")
w.onQuest(7, { { required = 10, fulfilled = 8 } }, "Kobold Camp Cleanup")
"#;

const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
function flush() stub.advance(0.6) end
function at(x, y) w.moveTo(1429, x, y); stub.advance(1.2) end
-- Yards to the nearest ten: a stop is reached a few yards short of its point.
function tens(n) return math.floor(n / 10 + 0.5) * 10 end
"#;

// The wolves 800 yd south of the player, then the vermin 500 yd south on the way: the guide walks past them. The vermin
// are the guide's second step; `pad` goto steps at the wolves between them put them past the planner's lookahead.
fn guide(pad: usize) -> String {
	let on = r#"{ title = "Stay", goals = { { t = "goto", map = 1429, x = 47.6, y = 85, r = 10 } } },"#.repeat(pad);
	format!(
		r#"
Polestar.RegisterGuide({{
	id = "test/state", pack = "test", title = "State", minLevel = 1, maxLevel = 10,
	steps = {{
		{{ title = "Wolves", goals = {{ {{ t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 85 }} }} }},
		{on}
		{{ title = "Vermin", goals = {{ {{ t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, x = 47.6, y = 70 }} }} }},
	}},
}})
"#
	)
}

fn run(guide: &str, setup: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/state" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, setup, script].join("\n"),
		..Default::default()
	})
}

#[test]
fn both_off_by_default_and_nothing_pressed() {
	let out = run(
		&guide(0),
		"",
		r#"
		local s = E.RoutingState()
		emit({ s.reorder.on, s.reorder.why, s.aim.on, s.arrowFrom, s.recalibrated == nil, s.reorder.live })"#,
	);
	assert_eq!(out, vec![json!([false, "off", false, "guide", true, 1])]);
}

#[test]
fn the_reorder_says_which_step_it_put_first_and_what_it_saves() {
	let out = run(
		&guide(0),
		r#"Polestar.Set("behavior.stepReorder", true); flush()"#,
		r#"
		local r = E.RoutingState().reorder
		emit({ r.on, r.shown, r.live, r.why, r.plan })
		emit({ r.last.n, r.last.order, tens(r.last.base), tens(r.last.cost), r.last.saved >= r.take })
		emit({ #r.window, r.window[1].step, r.window[1].ready, r.window[1].quests, r.window[2].step })"#,
	);
	assert_eq!(out[0], json!([true, 2, 1, "showing the step that walks least", [2, 1]]));
	// The vermin first (500 yd), then the wolves (300 yd on): 800 yd, against the guide's 800 yd south and 300 back, 1100.
	assert_eq!(out[1][0], json!(2));
	assert_eq!(out[1][1], json!([2, 1]));
	assert_eq!(out[1][2], json!(1100));
	assert_eq!(out[1][3], json!(800));
	assert_eq!(out[1][4], json!(true));
	assert_eq!(out[2], json!([2, 1, true, 1, 2]));
}

#[test]
fn the_reorder_says_why_it_leaves_the_guides_order_alone() {
	// The wolves are the nearest and the way back is short: nothing saves the 20 s a step needs.
	let g = r#"
Polestar.RegisterGuide({ id = "test/state", pack = "test", title = "State", minLevel = 1, maxLevel = 10, steps = {
	{ title = "Vermin", goals = { { t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, x = 47.6, y = 50 } } },
	{ title = "Wolves", goals = { { t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 60 } } },
} })
"#;
	let out = run(g, r#"Polestar.Set("behavior.stepReorder", true); flush()"#, r#"local r = E.RoutingState().reorder; emit({ r.shown == nil, r.why })"#);
	assert_eq!(out, vec![json!([true, "the guide's own order walks least"])]);
}

#[test]
fn the_arrow_says_which_objective_it_took_and_how_it_scored_the_lines() {
	let out = run(
		&guide(3),
		r#"Polestar.Set("behavior.whileHereAim", true); flush()"#,
		r#"
		at(47.6, 62); at(47.6, 62)
		local s = E.RoutingState()
		local a = s.aim
		emit({ a.on, s.arrowFrom, a.held and a.held.quest, a.limit, a.trigger, a.why })
		emit({ a.last.n, a.last.lines[1].quest, a.last.lines[1].value > 0, a.last.lines[1].score ~= nil, a.last.lines[1].secs ~= nil })"#,
	);
	assert_eq!(out[0], json!([true, "auto", 7, 15, 3, "holding the line it took"]));
	assert_eq!(out[1], json!([1, 7, true, true, true]));
}

#[test]
fn the_arrow_says_why_it_took_nothing() {
	// The vermin 150 yd east of the way: a detour of about 20 s, over the 15 s limit, so the line is weighed or never shown.
	let g = guide(3).replace("x = 47.6, y = 70 }", "x = 52.6, y = 70 }");
	let out = run(&g, r#"Polestar.Set("behavior.whileHereAim", true); flush()"#, r#"at(47.6, 62); at(47.6, 62); local s = E.RoutingState(); emit({ s.arrowFrom, s.aim.held == nil, s.aim.why })"#);
	assert_eq!(out[0][0], json!("guide"));
	assert_eq!(out[0][1], json!(true));
	let why = out[0][2].as_str().unwrap();
	assert!(["no objective lines to weigh", "no path of lines fits the seconds limit", "the best path scores under the trigger"].contains(&why), "{why}");
}

#[test]
fn recalibrate_is_counted() {
	let out = run(
		&guide(0),
		"",
		r#"
		E.Recalibrate(); stub.advance(5); E.Recalibrate(); stub.advance(7)
		local s = E.RoutingState()
		emit({ s.recalibrated.count, s.recalibrated.ago })"#,
	);
	assert_eq!(out, vec![json!([2, 7])]);
}
