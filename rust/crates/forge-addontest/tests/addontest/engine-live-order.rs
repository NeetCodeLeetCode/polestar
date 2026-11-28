//! behavior.stepReorder (addon/Polestar/Core/Engine.lua, Own.orderTick; Core/StepOrder.lua): when the next few
//! steps of the guide can be done in an order that walks less, the window shows the one to do first. The guide, its
//! steps and its progress do not change: the step shown is a look at a later one (the pin), and the live step comes
//! back when that one is done. Off by default. The tests walk a simulated player that follows only the arrow, at
//! 7 yd/s. Real Core manifest, stub world.
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
w.onQuest(60, { { required = 5, fulfilled = 1 } }, "Candles")
"#;

// QUEST maps a step's "step:goal" to the quest whose work is done there.
const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
function flush() stub.advance(0.6) end
function at(x, y) w.moveTo(1429, x, y); stub.advance(1.2) end
-- The step on screen, and the one the guide is live on.
function shown() return E.GetStepIndex() end
-- Walk to what the arrow leads to, 3.5 yd every 0.5 s (7 yd/s); at its place the quest's work is done.
function follow(limit)
	local secs, yards, seen, last = 0, 0, {}, nil
	while secs < limit do
		local x = E.GetWaypoint()
		if not x then break end
		local key = x.stepIndex .. ":" .. x.goalIndex
		if key ~= last and QUEST[key] then seen[#seen + 1] = key end
		last = key
		local dx, dy = (x.x - w.x) * 30, (x.y - w.y) * 20
		local d = math.sqrt(dx * dx + dy * dy)
		if d <= 5 then
			if QUEST[key] then w.finish(QUEST[key]) end
			stub.advance(0.6)
			secs = secs + 0.6
		else
			local step = math.min(d, 3.5)
			w.moveTo(1429, w.x + dx / d * step / 30, w.y + dy / d * step / 20)
			stub.advance(0.5)
			yards, secs = yards + step, secs + 0.5
		end
	end
	return { table.concat(seen, " "), math.floor(yards / 10 + 0.5) * 10 }
end
"#;

fn kill(title: &str, quest: u32, npc: u32, x: f64, y: f64) -> String {
	format!(r#"{{ title = "{title}", goals = {{ {{ t = "kill", target = "{title}", npc = {npc}, count = 8, quest = {quest}, oi = 1, map = 1429, x = {x}, y = {y} }} }} }},"#)
}

fn guide(steps: &str, quests: &str) -> String {
	format!(
		r#"
Polestar.RegisterGuide({{ id = "test/order", pack = "test", title = "Order", minLevel = 1, maxLevel = 10, steps = {{ {steps} }} }})
QUEST = {{ {quests} }}
"#
	)
}

fn run(guide: &str, setup: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/order" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, setup, script].join("\n"),
		..Default::default()
	})
}

const ON: &str = r#"Polestar.Set("behavior.stepReorder", true); flush()"#;

// The wolves 800 yd south of the player, then the vermin 500 yd south on the way: the guide walks past them.
fn way_back() -> String {
	guide(&format!("{}{}", kill("Wolves", 33, 299, 47.6, 85.0), kill("Vermin", 7, 6, 47.6, 70.0)), r#"["1:1"] = 33, ["2:1"] = 7"#)
}

#[test]
fn off_by_default_the_guide_is_walked_in_its_own_order() {
	let out = run(&way_back(), "", r#"emit(Polestar.Get("behavior.stepReorder")); emit(follow(300))"#);
	// 800 yd south to the wolves, 300 yd back north to the vermin (each stop is reached 5 yd short of its point).
	assert_eq!(out, vec![json!(false), json!(["1:1 2:1", 1090])]);
}

#[test]
fn a_step_on_the_way_is_shown_first_and_the_walk_gets_shorter() {
	let out = run(&way_back(), ON, r#"emit(shown()); emit(follow(300))"#);
	// The vermin first (500 yd), then the wolves (300 yd): 790 yd, not 1090.
	assert_eq!(out, vec![json!(2), json!(["2:1 1:1", 790])]);
}

#[test]
fn a_step_that_shares_a_quest_with_an_earlier_one_keeps_its_place() {
	// The vermin step takes the same quest as the wolves step: it comes after it, however far the way back.
	let g = guide(&format!("{}{}", kill("Wolves", 33, 299, 47.6, 85.0), kill("Wolves again", 33, 299, 47.6, 70.0)), r#"["1:1"] = 33"#);
	let out = run(&g, ON, r#"emit(shown()); emit(follow(300)[1])"#);
	assert_eq!(out[0], json!(1));
}

#[test]
fn a_step_whose_quest_is_not_in_the_log_waits_for_the_steps_before_it() {
	// Quest 99 is not in the log: nothing in the window takes it, so its step cannot be done first.
	let g = guide(&format!("{}{}", kill("Wolves", 33, 299, 47.6, 85.0), kill("Vermin", 99, 6, 47.6, 70.0)), r#"["1:1"] = 33"#);
	let out = run(&g, ON, r#"emit(shown())"#);
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn a_flight_between_steps_is_a_wall_nothing_moves_across() {
	let steps = format!("{}{}{}", kill("Wolves", 33, 299, 47.6, 85.0), r#"{ title = "Fly", goals = { { t = "fly", location = "Stormwind City" } } },"#, kill("Vermin", 7, 6, 47.6, 70.0));
	let out = run(&guide(&steps, r#"["1:1"] = 33, ["3:1"] = 7"#), ON, r#"emit(shown())"#);
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn when_the_guides_order_is_the_shortest_nothing_changes() {
	// The vermin 800 yd south, past the wolves at 500: the guide's order is the walk that costs least.
	let g = guide(&format!("{}{}", kill("Wolves", 33, 299, 47.6, 70.0), kill("Vermin", 7, 6, 47.6, 85.0)), r#"["1:1"] = 33, ["2:1"] = 7"#);
	let out = run(&g, ON, r#"emit(shown()); emit(follow(300))"#);
	assert_eq!(out, vec![json!(1), json!(["1:1 2:1", 790])]);
}

#[test]
fn a_step_the_player_looks_at_is_theirs_and_stays() {
	let g = guide(&format!("{}{}", kill("Wolves", 33, 299, 47.6, 70.0), kill("Vermin", 7, 6, 47.6, 85.0)), r#"["1:1"] = 33, ["2:1"] = 7"#);
	let out = run(&g, ON, r#"E.Goto(2); flush(); at(47.6, 45.1); at(47.6, 46); emit(shown())"#);
	assert_eq!(out, vec![json!(2)]);
}

#[test]
fn the_step_is_held_and_handed_back_for_a_walk_away_and_recalibrate_takes_it_again() {
	let out = run(
		&way_back(),
		ON,
		r#"
		emit(shown())
		-- Far off to the east: more than 60 yd past the nearest the player got to the step shown. It goes, and rests.
		at(90.0, 10.0)
		emit(shown())
		stub.advance(35)
		emit(shown())
		at(47.6, 45.1)
		emit(shown())
		at(90.0, 10.0)
		emit(shown())
		E.Recalibrate()
		emit(shown())"#,
	);
	// Taken, handed back for the walk away, taken again after the rest; kept on the walk back; handed back again:
	// Recalibrate takes it at once, with no rest.
	assert_eq!(out, vec![json!(2), json!(1), json!(2), json!(2), json!(1), json!(2)]);
}

#[test]
fn recalibrate_leaves_the_players_step_alone_with_the_setting_off() {
	let out = run(&way_back(), "", r#"E.Goto(2); flush(); E.Recalibrate(); emit(shown())"#);
	assert_eq!(out, vec![json!(2)]);
}

#[test]
fn with_the_arrow_that_takes_objectives_on_too_the_walk_is_no_longer() {
	let out = run(&way_back(), ON, r#"Polestar.Set("behavior.whileHereAim", true); flush(); emit(follow(300))"#);
	assert_eq!(out, vec![json!(["2:1 1:1", 790])]);
}

fn planner(script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		script: [TEST_PACK, TEST_SOURCE, "stub.login()", script].join("\n"),
		..Default::default()
	})
}

#[test]
fn the_planner_finds_the_shortest_order_and_keeps_the_guides_on_a_tie() {
	// From (0, 0): steps at 500, 100 and 900 yd along a line, left where they are entered. The guide's order is
	// 500 + 400 + 800 = 1700 yd; the 100, 500, 900 order is 900.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local ys = { 500, 100, 900 }
		for i = 1, 3 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i] = 0, ys[i], 0, ys[i], {} end
		local first = O.Best(3, 0, 0)
		local free = { first, O.order[1], O.order[2], O.order[3], math.floor(O.cost), math.floor(O.base) }
		-- Step 2 has to come after step 1: the guide's order is the shortest left (a tie goes to it).
		O.need[2] = { true }
		local held = O.Best(3, 0, 0)
		emit({ free, { held, O.order[1], O.order[2], O.order[3], math.floor(O.cost) } })"#,
	);
	assert_eq!(out, vec![json!([[2, 2, 1, 3, 900, 1700], [1, 1, 2, 3, 1700]])]);
}

#[test]
fn the_planner_counts_the_walk_on_to_the_steps_after_the_window() {
	// Steps 100 yd east and 100 yd west of the player: either first is 300 yd, so the guide's order stays. With the
	// guide going on to a point 200 yd east after them, the west step first and the east one last is the shorter.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local xs = { 100, -100 }
		for i = 1, 2 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i] = xs[i], 0, xs[i], 0, {} end
		local a = O.Best(2, 0, 0)
		O.tx, O.ty = 200, 0
		local b = O.Best(2, 0, 0)
		emit({ a, b })"#,
	);
	assert_eq!(out, vec![json!([1, 2])]);
}

// Class quests. A Class guide is done in its own order; a leveling guide's class steps (`only` a class) belong to
// that class alone, a class trainer or a quest chain between two steps is a wall or keeps the order.
const WARRIOR: &str = r#"w.class, w.className, w.classID = "WARRIOR", "Warrior", 1"#;
const DRUID: &str = r#"w.class, w.className, w.classID = "DRUID", "Druid", 11"#;

fn class_kill(title: &str, class: &str, quest: u32, x: f64, y: f64) -> String {
	format!(
		r#"{{ title = "{title}", only = {{ classes = {{ "{class}" }} }}, goals = {{ {{ t = "kill", target = "{title}", npc = 6, count = 8, quest = {quest}, oi = 1, map = 1429, x = {x}, y = {y} }} }} }},"#
	)
}

#[test]
fn a_class_guide_is_never_reordered() {
	let steps = format!("{}{}", kill("Wolves", 33, 299, 47.6, 85.0), kill("Vermin", 7, 6, 47.6, 70.0));
	let g = format!(
		r#"Polestar.RegisterGuide({{ id = "test/order", pack = "test", title = "Order", category = "Class", classes = {{ "WARRIOR" }}, minLevel = 1, maxLevel = 10, steps = {{ {steps} }} }})
QUEST = {{ ["1:1"] = 33, ["2:1"] = 7 }}"#
	);
	let out = run(&g, &format!("{WARRIOR}\n{ON}"), r#"emit(shown()); emit(follow(300))"#);
	// The same two steps as the leveling guide that is reordered: here the guide's order stays.
	assert_eq!(out, vec![json!(1), json!(["1:1 2:1", 1090])]);
}

#[test]
fn a_class_step_is_the_players_own_and_moves_only_for_that_class() {
	let steps = format!("{}{}", kill("Wolves", 33, 299, 47.6, 85.0), class_kill("Vermin", "WARRIOR", 7, 47.6, 70.0));
	let g = guide(&steps, r#"["1:1"] = 33, ["2:1"] = 7"#);
	// The Warrior's step is on the way: it is shown first. For a Druid the step is not in the guide at all.
	let warrior = run(&g, &format!("{WARRIOR}\n{ON}"), r#"emit(shown()); emit(follow(300))"#);
	assert_eq!(warrior, vec![json!(2), json!(["2:1 1:1", 790])]);
	let druid = run(&g, &format!("{DRUID}\n{ON}"), r#"emit(shown()); emit(follow(300))"#);
	assert_eq!(druid, vec![json!(1), json!(["1:1", 790])]);
}

#[test]
fn a_class_trainer_between_two_steps_is_a_wall() {
	let steps = format!(
		"{}{}{}",
		kill("Wolves", 33, 299, 47.6, 85.0),
		r#"{ title = "Train", goals = { { t = "train", class = true, npc = 911, npcName = "Llane Beshere", map = 1429, x = 47.6, y = 60.0 } } },"#,
		kill("Vermin", 7, 6, 47.6, 70.0)
	);
	let g = guide(&steps, r#"["1:1"] = 33, ["3:1"] = 7"#);
	let out = run(&g, &format!("{WARRIOR}\n{ON}"), r#"emit(shown())"#);
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn a_chain_keeps_its_order_a_follow_up_waits_for_its_quest_and_an_unrelated_step_may_go_first() {
	// The Druid's chain: take 6361 far south (step 1), work it (step 2), hand it in (step 3). A follow-up (step 4)
	// needs 6361 handed in first and sits nearer than anything. The wolves (step 5) are independent and on the way.
	let steps = format!(
		"{}{}{}{}{}",
		r#"{ title = "Take", only = { classes = { "DRUID" } }, goals = { { t = "accept", quest = 6361, title = "The Chain", npc = 3033, map = 1429, x = 47.6, y = 85.0 } } },"#,
		r#"{ title = "Work", only = { classes = { "DRUID" } }, goals = { { t = "kill", target = "Work", npc = 6, count = 8, quest = 6361, oi = 1, map = 1429, x = 47.6, y = 86.0 } } },"#,
		r#"{ title = "Hand in", only = { classes = { "DRUID" } }, goals = { { t = "turnin", quest = 6361, title = "The Chain", npc = 3033, map = 1429, x = 47.6, y = 85.0 } } },"#,
		r#"{ title = "Next", only = { classes = { "DRUID" } }, goals = { { t = "accept", quest = 6362, title = "The Next", pre = { 6361 }, npc = 3034, map = 1429, x = 47.6, y = 50.0 } } },"#,
		kill("Wolves", 33, 299, 47.6, 70.0)
	);
	let g = guide(&steps, r#"["5:1"] = 33"#);
	let out = run(&g, &format!("{DRUID}\n{ON}"), r#"emit(shown()); at(47.6, 45.1); emit(shown())"#);
	// The wolves (step 5, on the way south) come first; the chain's steps never go ahead of each other, and the
	// follow-up (step 4) never goes ahead of the chain it follows.
	assert_eq!(out[0], json!(5));
	let seq = run(
		&g,
		&format!("{DRUID}\n{ON}"),
		r#"
		local seen, ticked = {}, {}
		for _ = 1, 2500 do
			local s = shown()
			if s and seen[#seen] ~= s then seen[#seen + 1] = s end
			local x = E.GetWaypoint()
			if not x then break end
			local dx, dy = (x.x - w.x) * 30, (x.y - w.y) * 20
			local d = math.sqrt(dx * dx + dy * dy)
			if d <= 5 then
				-- At its place the quest's work is done in the game: taken, finished, handed in.
				local key = x.stepIndex .. ":" .. x.goalIndex
				if not ticked[key] then
					ticked[key] = true
					local at = x.stepIndex
					if at == 1 then w.onQuest(6361, { { required = 8, fulfilled = 0 } }, "The Chain")
					elseif at == 2 then w.finish(6361)
					elseif at == 3 then w.turnIn(6361)
					elseif at == 4 then w.onQuest(6362, { { required = 1, fulfilled = 0 } }, "The Next")
					elseif at == 5 then w.finish(33) end
					stub.fire("QUEST_LOG_UPDATE")
				end
				stub.advance(0.6)
			else
				local st = math.min(d, 3.5)
				w.moveTo(1429, w.x + dx / d * st / 30, w.y + dy / d * st / 20)
				stub.advance(0.5)
			end
		end
		emit(seen)"#,
	);
	let seen: Vec<i64> = seq[0].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
	let pos = |s: i64| seen.iter().position(|&x| x == s);
	assert!(pos(1) < pos(2) && pos(2) < pos(3), "the chain keeps its order: {seen:?}");
	assert!(pos(3) < pos(4), "the follow-up waits for its quest: {seen:?}");
	assert_eq!(seen, vec![5, 1, 2, 3, 4], "the wolves on the way first, then the chain, then its follow-up");
}

#[test]
fn a_warriors_follow_up_waits_for_the_quest_it_follows_however_near_it_is() {
	// The follow-up (step 2) is 300 yd away, the wolves it needs done first 800 yd: the way back would be shorter
	// with the follow-up first, but its accept needs the quest handed in.
	let steps = format!(
		"{}{}",
		class_kill("Wolves", "WARRIOR", 33, 47.6, 85.0),
		r#"{ title = "Next", only = { classes = { "WARRIOR" } }, goals = { { t = "accept", quest = 1686, title = "The Next", pre = { 33 }, npc = 3034, map = 1429, x = 47.6, y = 60.0 } } },"#
	);
	let g = guide(&steps, r#"["1:1"] = 33"#);
	let out = run(&g, &format!("{WARRIOR}\n{ON}"), r#"emit(shown())"#);
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn a_second_tab_keeps_its_own_step_and_the_first_keeps_its_reorder() {
	let other = r#"Polestar.RegisterGuide({ id = "test/other", pack = "test", title = "Other", minLevel = 1, maxLevel = 10, steps = {
		{ title = "Only", goals = { { t = "kill", target = "Vermin", npc = 6, count = 8, quest = 7, oi = 1, map = 1429, x = 47.6, y = 70.0 } } },
		{ title = "Second", goals = { { t = "kill", target = "Wolves", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 85.0 } } },
	} })"#;
	let out = run(
		&format!("{}{other}", way_back()),
		ON,
		r#"
		emit(shown())
		E.OpenTab("test/other", true); flush(); at(47.6, 45.1)
		emit(shown())
		E.SelectTab(1); flush(); at(47.6, 45.1)
		emit(shown())"#,
	);
	// The first guide shows its second step first; the other tab's guide is on the way already, so it keeps step 1;
	// back on the first tab, its reorder is where it was.
	assert_eq!(out[0], json!(2));
	assert_eq!(out[1], json!(1));
	assert_eq!(out[2], json!(2));
}
