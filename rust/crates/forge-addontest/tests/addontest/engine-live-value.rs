//! behavior.stepReorderXp (addon/Polestar/Core/Engine.lua, Own.orderFill; Core/StepOrder.lua): with the step reorder
//! on, the window shows first the step that pays XP soonest, counting the walk and the work of each step, and it
//! never takes longer in all than the guide's own order. The planner looks at up to 8 steps with an exact search over
//! the sets of steps done. Off by default. Real Core manifest, stub world.
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
function shown() return E.GetStepIndex() end
"#;

fn run(guide: &str, setup: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/value" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, setup, script].join("\n"),
		..Default::default()
	})
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

fn turnin(title: &str, quest: u32, x: f64, y: f64) -> String {
	format!(r#"{{ title = "{title}", goals = {{ {{ t = "turnin", quest = {quest}, title = "{title}", npc = {quest}, npcName = "Giver {quest}", map = 1429, x = {x}, y = {y} }} }} }},"#)
}

fn guide(steps: &str) -> String {
	format!(r#"Polestar.RegisterGuide({{ id = "test/value", pack = "test", title = "Value", minLevel = 1, maxLevel = 10, steps = {{ {steps} }} }})"#)
}

// The client's XP for the two hand-ins.
const XP: &str = r#"w.finish(33); w.finish(7)
XP = { [33] = 100, [7] = 5000 }; GetQuestLogRewardXP = function(id) return XP[id] end"#;
const REORDER: &str = r#"Polestar.Set("behavior.stepReorder", true); flush()"#;
const BY_XP: &str = r#"Polestar.Set("behavior.stepReorderXp", true); flush(); flush()"#;

// The search is exact: it finds what trying every order finds, for the walk and for each first step.
#[test]
fn the_search_finds_what_every_order_finds() {
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local seed = 12345
		local function rnd() seed = (seed * 1103515245 + 12345) % 2147483648; return seed / 2147483648 end
		-- The walk of one order, or nil when it breaks a `need`.
		local function walk(n, order, px, py)
			local done, cost, lx, ly = {}, 0, px, py
			for k = 1, n do
				local i = order[k]
				for j = 1, n do if O.need[i][j] and not done[j] then return nil end end
				cost = cost + math.sqrt((lx - O.ex[i]) ^ 2 + (ly - O.ey[i]) ^ 2)
				lx, ly, done[i] = O.ox[i], O.oy[i], true
			end
			if O.tx then cost = cost + math.sqrt((lx - O.tx) ^ 2 + (ly - O.ty) ^ 2) end
			return cost
		end
		local best, firsts
		local function permute(n, order, used, k, px, py)
			if k > n then
				local c = walk(n, order, px, py)
				if c then
					if not best or c < best then best = c end
					if not firsts[order[1]] or c < firsts[order[1]] then firsts[order[1]] = c end
				end
				return
			end
			for i = 1, n do
				if not used[i] then used[i], order[k] = true, i; permute(n, order, used, k + 1, px, py); used[i] = false end
			end
		end
		local bad = 0
		for round = 1, 60 do
			local n = 2 + round % 5
			for i = 1, n do
				O.ex[i], O.ey[i] = rnd() * 1000, rnd() * 1000
				O.ox[i], O.oy[i] = O.ex[i] + rnd() * 100, O.ey[i] + rnd() * 100
				O.need[i] = {}
				for j = 1, i - 1 do O.need[i][j] = rnd() < 0.25 end
			end
			if round % 3 == 0 then O.tx, O.ty = rnd() * 1000, rnd() * 1000 else O.tx, O.ty = nil, nil end
			local px, py = rnd() * 1000, rnd() * 1000
			best, firsts = nil, {}
			permute(n, {}, {}, 1, px, py)
			O.Best(n, px, py)
			if math.abs(O.cost - best) > 1e-6 then bad = bad + 1 end
			for i = 1, n do
				if (O.first[i] == nil) ~= (firsts[i] == nil) or (firsts[i] and math.abs(O.first[i] - firsts[i]) > 1e-6) then bad = bad + 1 end
			end
			local seen, ok = {}, true
			for k = 1, n do seen[O.order[k]] = true end
			for i = 1, n do if not seen[i] then ok = false end end
			if not ok or walk(n, O.order, px, py) == nil then bad = bad + 1 end
		end
		emit(bad)"#,
	);
	assert_eq!(out, vec![json!(0)]);
}

#[test]
fn it_looks_at_eight_steps_not_five() {
	// Eight steps along a line, the player at the far end: the last step first is the shortest way.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		for i = 1, 8 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i] = i * 100, 0, i * 100, 0, {} end
		local first = O.Best(8, 900, 0)
		emit({ O.MAX, first, O.order[1], O.order[8], math.floor(O.base + 0.5), math.floor(O.cost + 0.5) })"#,
	);
	// The guide's order walks 800 yd to step 1 and on east to step 8 (700 yd): 1500; the other way is 100 + 700.
	assert_eq!(out, vec![json!([8, 8, 8, 1, 1500, 800])]);
}

#[test]
fn the_step_that_pays_most_comes_first_when_the_walk_is_the_same() {
	// Steps 100 yd east (no XP) and 100 yd west (1000 XP) of the player: the walk is 300 yd either way. By walk the
	// guide's order stays; by XP the west one comes first and the XP waits less.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local xs, xp = { 100, -100 }, { 0, 1000 }
		for i = 1, 2 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i], O.xp[i], O.work[i] = xs[i], 0, xs[i], 0, {}, xp[i], 0 end
		O.per, O.byXp = 0.2, false
		local walk = O.Best(2, 0, 0)
		local a = { walk, O.mode }
		O.byXp = true
		local byXp = O.Best(2, 0, 0)
		emit({ a, { byXp, O.mode, math.floor((O.base - O.cost) * O.per + 0.5) } })"#,
	);
	// By XP: the west step is done after 100 yd (20 s), the guide's order after 300 yd (60 s): 40 s sooner.
	assert_eq!(out, vec![json!([[1, "walk"], [2, "xp", 40]])]);
}

#[test]
fn the_work_of_a_step_counts_as_time_before_the_next_pays() {
	// The same two steps, each paying 1000 XP, the east one taking 600 s of work and the west one 10 s: the quick
	// one first, the XP of both comes soonest that way.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local xs, work = { 100, -100 }, { 600, 10 }
		for i = 1, 2 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i], O.xp[i], O.work[i] = xs[i], 0, xs[i], 0, {}, 1000, work[i] end
		O.per, O.byXp = 0.2, true
		emit({ O.Best(2, 0, 0), O.mode })"#,
	);
	assert_eq!(out, vec![json!([2, "xp"])]);
}

#[test]
fn it_never_takes_longer_in_all_than_the_guides_order() {
	// East 100 yd (10 XP) and west 300 yd (5000 XP): the west one first pays sooner but walks 700 yd against the
	// guide's 500, so it is not taken. The guide's order stays, scored by XP: (10 * 100 + 5000 * 500) / 5010 yd.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local xs, xp = { 100, -300 }, { 10, 5000 }
		for i = 1, 2 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i], O.xp[i], O.work[i] = xs[i], 0, xs[i], 0, {}, xp[i], 0 end
		O.per, O.byXp = 0.2, true
		emit({ O.Best(2, 0, 0), O.mode, math.floor(O.cost + 0.5) })"#,
	);
	assert_eq!(out, vec![json!([1, "xp", 499])]);
}

#[test]
fn with_no_xp_anywhere_the_walk_decides() {
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local ys = { 500, 100, 900 }
		for i = 1, 3 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i], O.xp[i], O.work[i] = 0, ys[i], 0, ys[i], {}, 0, 30 end
		O.per, O.byXp = 0.2, true
		emit({ O.Best(3, 0, 0), O.mode, math.floor(O.cost + 0.5) })"#,
	);
	assert_eq!(out, vec![json!([2, "walk", 900])]);
}

#[test]
fn a_step_that_follows_another_keeps_its_place_whatever_it_pays() {
	// Step 2 needs step 1 (the same quest): it pays most, and still comes second.
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local xs, xp = { 100, -100 }, { 0, 1000 }
		for i = 1, 2 do O.ex[i], O.ey[i], O.ox[i], O.oy[i], O.need[i], O.xp[i], O.work[i] = xs[i], 0, xs[i], 0, {}, xp[i], 0 end
		O.need[2] = { true }
		O.per, O.byXp = 0.2, true
		emit({ O.Best(2, 0, 0), O.order[2] })"#,
	);
	assert_eq!(out, vec![json!([1, 2])]);
}

// The engine. The player stands between a hand-in 600 yd east (100 XP) and one 600 yd west (5000 XP): the walk is the
// same either way.
fn between() -> String {
	guide(&format!("{}{}", turnin("East", 33, 67.6, 45.1), turnin("West", 7, 27.6, 45.1)))
}

#[test]
fn with_the_setting_off_the_guides_order_stays_and_on_the_richer_hand_in_comes_first() {
	let out = run(&between(), XP, &format!("{REORDER}\nemit(shown())\n{BY_XP}\nemit(shown())"));
	assert_eq!(out, vec![json!(1), json!(2)]);
}

#[test]
fn it_is_off_by_default() {
	let out = run(&between(), XP, r#"emit(Polestar.Get("behavior.stepReorderXp")); emit(shown())"#);
	assert_eq!(out, vec![json!(false), json!(1)]);
}

#[test]
fn it_does_nothing_without_the_step_reorder() {
	let out = run(&between(), XP, &format!("{BY_XP}\nemit(shown())"));
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn a_class_guide_is_never_reordered_by_xp() {
	let g = format!(
		r#"Polestar.RegisterGuide({{ id = "test/value", pack = "test", title = "Value", category = "Class", classes = {{ "WARRIOR" }}, minLevel = 1, maxLevel = 10, steps = {{ {}{} }} }})"#,
		turnin("East", 33, 67.6, 45.1),
		turnin("West", 7, 27.6, 45.1)
	);
	let out = run(&g, &format!("{XP}\nw.class, w.className, w.classID = \"WARRIOR\", \"Warrior\", 1"), &format!("{REORDER}\n{BY_XP}\nemit(shown())"));
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn a_hand_in_that_costs_more_in_all_is_not_taken_first() {
	// The rich hand-in is 1200 yd west, the poor one 300 yd east: the rich one first walks 2700 yd against 1800.
	let g = guide(&format!("{}{}", turnin("East", 33, 57.6, 45.1), turnin("West", 7, 7.6, 45.1)));
	let out = run(&g, XP, &format!("{REORDER}\n{BY_XP}\nemit(shown())"));
	assert_eq!(out, vec![json!(1)]);
}

#[test]
fn eight_steps_are_in_the_window_and_the_last_one_can_come_first() {
	// Eight hand-ins in a row eastwards, the player at the east end: the guide's order walks to the far west end
	// first, and back. The window holds all eight, so the east one comes first.
	let mut steps = String::new();
	let mut log = String::new();
	let mut xp = String::new();
	for i in 1..=8u32 {
		steps += &turnin(&format!("Step {i}"), 100 + i, 10.0 + 8.0 * i as f64, 45.1);
		log += &format!("w.onQuest({}, {{ 1 }}, \"Step {i}\"); w.finish({})\n", 100 + i, 100 + i);
		xp += &format!("[{}] = 1000, ", 100 + i);
	}
	let setup = format!("{log}w.moveTo(1429, 80.0, 45.1)\nXP = {{ {xp} }}; GetQuestLogRewardXP = function(id) return XP[id] end");
	let out = run(&guide(&steps), &setup, &format!("{REORDER}\nemit(shown())"));
	assert_eq!(out, vec![json!(8)]);
}

// With XP: when the order that pays soonest takes no longer in all than the guide's, the search finds it.
#[test]
fn the_search_for_xp_finds_what_every_order_finds() {
	let out = planner(
		r#"
		local O = Polestar.StepOrder
		local seed = 777
		local function rnd() seed = (seed * 1103515245 + 12345) % 2147483648; return seed / 2147483648 end
		local function leg(ax, ay, bx, by) return math.sqrt((ax - bx) ^ 2 + (ay - by) ^ 2) end
		-- Seconds in all and the XP-weighted score of an order; nil when it breaks a `need`.
		local function measure(n, order, px, py)
			local done, t, lx, ly, score, total = {}, 0, px, py, 0, 0
			for i = 1, n do total = total + O.xp[i] end
			local left = total
			for k = 1, n do
				local i = order[k]
				for j = 1, n do if O.need[i][j] and not done[j] then return nil end end
				t = t + leg(lx, ly, O.ex[i], O.ey[i]) * O.per + leg(O.ex[i], O.ey[i], O.ox[i], O.oy[i]) * O.per + O.work[i]
				score = score + t * O.xp[i]
				lx, ly, done[i] = O.ox[i], O.oy[i], true
			end
			return t, score / total
		end
		local best, bestTook, own
		local function permute(n, order, used, k, px, py)
			if k > n then
				local t, sc = measure(n, order, px, py)
				if t and (not best or sc < best) then best, bestTook = sc, t end
				return
			end
			for i = 1, n do
				if not used[i] then used[i], order[k] = true, i; permute(n, order, used, k + 1, px, py); used[i] = false end
			end
		end
		local bad, checked = 0, 0
		for round = 1, 80 do
			local n = 2 + round % 6
			for i = 1, n do
				O.ex[i], O.ey[i] = rnd() * 1000, rnd() * 1000
				O.ox[i], O.oy[i] = O.ex[i] + rnd() * 100, O.ey[i] + rnd() * 100
				O.xp[i], O.work[i] = rnd() < 0.3 and 0 or math.floor(rnd() * 3000), math.floor(rnd() * 120)
				O.need[i] = {}
				for j = 1, i - 1 do O.need[i][j] = rnd() < 0.2 end
			end
			O.xp[1] = O.xp[1] + 100
			O.tx, O.ty, O.per, O.byXp, O.keep = nil, nil, 0.2, true, nil
			local px, py = rnd() * 1000, rnd() * 1000
			best = nil
			permute(n, {}, {}, 1, px, py)
			local ident = {}
			for i = 1, n do ident[i] = i end
			local ownTook, ownScore = measure(n, ident, px, py)
			O.Best(n, px, py)
			local gotTook, gotScore = measure(n, O.order, px, py)
			if gotTook > ownTook + 1e-6 then bad = bad + 1 end
			if bestTook <= ownTook + 1e-6 then
				checked = checked + 1
				-- The best order is within the guide's time: the search finds it.
				if math.abs(gotScore - best) > 1e-4 or O.mode ~= "xp" or math.abs(O.cost * O.per - best) > 1e-4 then bad = bad + 1 end
			end
		end
		emit({ bad, checked > 10 })"#,
	);
	assert_eq!(out, vec![json!([0, true])]);
}
