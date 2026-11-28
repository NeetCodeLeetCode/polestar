//! behavior.whileHereAim (addon/Polestar/Core/Engine.lua, Own.hereAutoAim): the arrow leads to a "While you're here"
//! objective by itself, as if the player had clicked the line, when the walk is worth it: its quest's XP for each
//! minute of extra walking off the way the guide leads, through the guide's next stops (Core/StopPlanner.lua scores
//! the orders of the lines shown). Off by default. The tests walk a simulated player that follows only the arrow, at
//! 7 yd/s, so the walked yards and the order of the arrow's stops are the measure. Real Core manifest, stub world.
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
w.onQuest(60, { { text = "Large Candle: 1/3", type = "item", required = 3, fulfilled = 1 } }, "Candles")
"#;

// The arrow's stops in order (the quest steps only), the yards walked and the seconds, for a player who only follows
// the arrow. QUEST (set by the guide) maps a step's "step:goal" to the quest whose work is done there.
const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
function flush() stub.advance(0.6) end
function at(x, y) w.moveTo(1429, x, y); stub.advance(1.2) end
function wp()
	local x = E.GetWaypoint()
	return x and (x.stepIndex .. ":" .. x.goalIndex) or "none"
end
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

// Step 1: the wolves, 800 yd south of the player. Then `pad` goto steps at the wolves (the guide's next stops), then
// the vermin at (vx, vy). `extra`: more steps. With pad 3 the vermin are past the stops the planner looks through;
// with pad 0 the guide goes to them right after the wolves.
fn guide_with(pad: usize, wolves_extra: &str, vermin: &str, extra: &str) -> String {
	let steps = 2 + pad;
	let on = r#"{ title = "Stay", goals = { { t = "goto", map = 1429, x = 47.6, y = 85, r = 10 } } },"#.repeat(pad);
	format!(
		r#"
Polestar.RegisterGuide({{
	id = "test/detour", pack = "test", title = "Detour", minLevel = 1, maxLevel = 10,
	steps = {{
		{{ title = "Wolves", goals = {{ {{ t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 85{wolves_extra} }} }} }},
		{on}
		{{ title = "Vermin", goals = {{ {{ t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, {vermin} }} }} }},
		{extra}
	}},
}})
QUEST = {{ ["1:1"] = 33, ["{steps}:1"] = 7, ["{next}:1"] = 60 }}
"#,
		next = steps + 1
	)
}

// The vermin at step 5, past the wolves' next stops: only what they are worth against the walking counts.
fn guide(wolves_extra: &str, vermin: &str, extra: &str) -> String {
	guide_with(3, wolves_extra, vermin, extra)
}

// The vermin at step 2: the guide goes to them right after the wolves, so doing them first saves the way back.
fn near(wolves_extra: &str, vermin: &str, extra: &str) -> String {
	guide_with(0, wolves_extra, vermin, extra)
}

// The vermin step and the extra step of `guide`.
const V: &str = "5:1";
const X: &str = "6:1";

// The vermin 500 yd south of the player, on the way to the wolves.
const ON_THE_WAY: &str = "x = 47.6, y = 70";
// The vermin 150 yd east of the way: a detour of about 33 s at its nearest, 20 s when it first shows.
const OFF_THE_WAY: &str = "x = 52.6, y = 70";

fn run(guide: &str, setup: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/detour" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, setup, script].join("\n"),
		..Default::default()
	})
}

const ON: &str = r#"Polestar.Set("behavior.whileHereAim", true); flush()"#;

#[test]
fn off_by_default_the_arrow_walks_the_guide_and_comes_back_for_the_vermin() {
	let out = run(&guide("", ON_THE_WAY, ""), "", r#"emit(Polestar.Get("behavior.whileHereAim")); emit(follow(300))"#);
	// 800 yd south to the wolves, then 300 yd back north to the vermin passed on the way (each stop is reached
	// 5 yd short of its point).
	assert_eq!(out, vec![json!(false), json!(["1:1 5:1", 1090])]);
}

#[test]
fn an_objective_on_the_way_is_taken_and_the_walk_gets_shorter() {
	let out = run(&guide("", ON_THE_WAY, ""), ON, r#"emit(follow(300))"#);
	// The vermin first (500 yd), then the wolves (300 yd): 790 yd, not 1090.
	assert_eq!(out, vec![json!(["1:1 5:1 1:1", 790])]);
}

#[test]
fn a_detour_over_the_seconds_limit_stays_with_the_guide_and_a_higher_limit_takes_it() {
	let g = guide("", OFF_THE_WAY, "");
	let out = run(&g, ON, r#"emit(follow(300))"#);
	assert_eq!(out[0][0], json!("1:1 5:1"), "15 s is under the detour");
	let out = run(&g, &format!("{ON}\nPolestar.Set(\"behavior.whileHereAimSeconds\", 60); flush()"), r#"emit(follow(300))"#);
	assert_eq!(out[0][0], json!("1:1 5:1 1:1"), "60 s covers it");
}

#[test]
fn the_arrow_goes_back_to_the_guide_when_the_objective_is_done() {
	let out = run(
		&guide("", ON_THE_WAY, ""),
		ON,
		r#"
		at(47.6, 62); at(47.6, 62)
		emit(wp())
		w.finish(7); flush(); flush()
		emit(wp())"#,
	);
	assert_eq!(out, vec![json!("5:1"), json!("1:1")]);
}

#[test]
fn a_line_the_player_clicked_is_not_replaced() {
	// The vermin (on the way, 0 s) and the candles (90 yd east, about 8 s): the nearest detour would win by itself.
	let candles = r#"{ title = "Candles", goals = { { t = "collect", item = 772, itemName = "Large Candle", count = 3, quest = 60, oi = 1, map = 1429, x = 50.6, y = 68 } } }"#;
	let g = guide("", ON_THE_WAY, candles);
	let alone = run(&g, ON, r#"at(47.6, 62); at(47.6, 62); emit(wp())"#);
	assert_eq!(alone, vec![json!("5:1")]);
	let clicked = run(
		&g,
		"",
		r#"
		at(47.6, 62)
		for i, v in ipairs(E.GetWhileHereList()) do if v.step == 6 then E.AimWhileHere(i) end end
		flush()
		emit(wp())
		Polestar.Set("behavior.whileHereAim", true); stub.advance(3)
		emit(wp())"#,
	);
	assert_eq!(clicked, vec![json!("6:1"), json!("6:1")]);
}

#[test]
fn a_dismissed_line_is_not_taken_again() {
	let out = run(
		&guide("", ON_THE_WAY, ""),
		ON,
		r#"
		at(47.6, 62); at(47.6, 62)
		emit(wp())
		E.DismissWhileHere(1); flush()
		emit(wp())
		stub.advance(10)
		emit(wp())"#,
	);
	assert_eq!(out, vec![json!("5:1"), json!("1:1"), json!("1:1")]);
}

#[test]
fn nothing_is_taken_in_combat_until_it_ends() {
	let out = run(
		&guide("", ON_THE_WAY, ""),
		ON,
		r#"
		stub.inCombat = true
		at(47.6, 62); at(47.6, 62)
		emit(wp())
		stub.inCombat = false
		stub.advance(2)
		emit(wp())"#,
	);
	assert_eq!(out, vec![json!("1:1"), json!("5:1")]);
}

#[test]
fn an_escort_on_the_live_step_holds_every_line() {
	let out = run(&guide(", escort = true", ON_THE_WAY, ""), ON, r#"at(47.6, 62); at(47.6, 62); stub.advance(2); emit(wp())"#);
	assert_eq!(out, vec![json!("1:1")]);
}

#[test]
fn mobs_above_the_players_level_are_not_taken() {
	let out = run(&guide("", &format!("{ON_THE_WAY}, lvl = 9"), ""), ON, r#"at(47.6, 62); at(47.6, 62); stub.advance(2); emit(wp())"#);
	assert_eq!(out, vec![json!("1:1")]);
}

#[test]
fn a_pick_up_is_never_taken() {
	let pickup = r#"{ title = "Maybell", goals = { { t = "accept", quest = 106, title = "Young Lovers", npc = 251, npcName = "Maybell Maclure", qlvl = 5, map = 1429, x = 47.6, y = 70 } } }"#;
	let g = guide("", "x = 47.6, y = 20", pickup);
	let out = run(&g, ON, r#"at(47.6, 62); at(47.6, 62); stub.advance(2); emit(wp())"#);
	assert_eq!(out, vec![json!("1:1")]);
}

// What a line is worth, the order of several, and when the arrow lets one go. The detour of OFF_THE_WAY is 20 s when its
// line first shows; the wolves' hunt is 800 yd south. The client's XP for a quest is set with GetQuestLogRewardXP; the
// level bar is 400 XP, so 4 XP is 1 percent of a level.
const XP: &str = r#"XP = {}; GetQuestLogRewardXP = function(id) return XP[id] end"#;
const WIDE: &str = "Polestar.Set(\"behavior.whileHereAimSeconds\", 60); flush()";

fn planner(script: &str) -> Vec<Value> {
	run(&guide("", ON_THE_WAY, ""), "", script)
}

#[test]
fn the_planner_orders_stops_along_the_way_and_gives_each_first_stop_its_best_path() {
	// The player at (0, 0) heads for (0, 1000). Stops 1 and 2 stand 800 yd and 400 yd along the way: the nearer one
	// first and then the other costs nothing, the farther one first walks back 400 yd.
	let out = planner(
		r#"
		local P = Polestar.StopPlanner
		P.x[1], P.y[1], P.value[1] = 0, 800, 5
		P.x[2], P.y[2], P.value[2] = 0, 400, 5
		P.gx[1], P.gy[1], P.gown[1] = 0, 1000, 0
		local first, score, secs = P.Best(2, 1, 0, 0, 1.25 / 7, 10, 60)
		emit({ first, math.floor(secs + 0.5), P.score[2] > P.score[1], P.secs[1] })"#,
	);
	// Stop 1 first can only visit itself (stop 2 after it is 142 s of walking back); stop 2 first visits both.
	assert_eq!(out, vec![json!([2, 0, true, 0])]);
}

#[test]
fn the_planner_weighs_value_against_walking_and_drops_what_passes_the_limit() {
	// Stop 1 is 14 s out of the way and worth a little, stop 2 is 50 s out and worth a lot. Visiting both is 68 s.
	let out = planner(
		r#"
		local P = Polestar.StopPlanner
		P.x[1], P.y[1], P.value[1] = 200, 500, 0.25
		P.x[2], P.y[2], P.value[2] = 400, 500, 10
		P.gx[1], P.gy[1], P.gown[1] = 0, 1000, 0
		local a = P.Best(2, 1, 0, 0, 1.25 / 7, 10, 60)
		local b = P.Best(2, 1, 0, 0, 1.25 / 7, 10, 20)
		local c = P.Best(2, 1, 0, 0, 1.25 / 7, 10, 5)
		emit({ a, b, c or "none", P.score[1] == nil })"#,
	);
	// 60 s: the worthwhile stop first. 20 s: only the cheap stop fits. 5 s: nothing fits.
	assert_eq!(out, vec![json!([2, 1, "none", true])]);
}

#[test]
fn the_planner_sees_a_stop_that_costs_less_after_the_guides_next_stop() {
	// Heading for (0, 300), then (400, 300): the stop at (200, 300) is on the second leg. Done now it is 46 s of
	// walking; after the first stop it is free.
	let out = planner(
		r#"
		local P = Polestar.StopPlanner
		P.x[1], P.y[1], P.value[1] = 200, 300, 5
		P.gx[1], P.gy[1], P.gown[1] = 0, 300, 0
		P.gx[2], P.gy[2], P.gown[2] = 400, 300, 0
		local first, score, secs, later = P.Best(1, 2, 0, 0, 1.25 / 7, 10, 60)
		emit({ first, math.floor(secs + 0.5), later > score * 1.3 })"#,
	);
	assert_eq!(out, vec![json!([1, 47, true])]);
}

#[test]
fn the_planner_leaves_out_the_guides_stop_that_a_visited_stop_does_the_work_of() {
	// The guide heads for (0, 800) and then (0, 300); stop 1 is the work at (0, 300). Visiting it first and going on
	// to (0, 800) is shorter than the guide's own way (800 yd, then 500 back): no extra walking.
	let out = planner(
		r#"
		local P = Polestar.StopPlanner
		P.x[1], P.y[1], P.value[1] = 0, 300, 5
		P.gx[1], P.gy[1], P.gown[1] = 0, 800, 0
		P.gx[2], P.gy[2], P.gown[2] = 0, 300, 1
		local first, score, secs = P.Best(1, 2, 0, 0, 1.25 / 7, 10, 60)
		emit({ first, secs })"#,
	);
	assert_eq!(out, vec![json!([1, 0])]);
}

#[test]
fn a_quest_worth_little_is_not_worth_a_detour_and_one_worth_more_is() {
	let g = guide("", OFF_THE_WAY, "");
	let little = run(&g, &format!("{ON}\n{WIDE}\n{XP}; XP[7] = 1"), r#"emit(follow(300))"#);
	assert_eq!(little[0][0], json!("1:1 5:1"), "a quarter of a percent of a level for 20 s of walking");
	let more = run(&g, &format!("{ON}\n{WIDE}\n{XP}; XP[7] = 40"), r#"emit(follow(300))"#);
	assert_eq!(more[0][0], json!("1:1 5:1 1:1"), "ten percent of a level for the same walk");
}

#[test]
fn a_line_on_the_way_is_taken_whatever_it_is_worth() {
	let out = run(&guide("", ON_THE_WAY, ""), &format!("{ON}\n{XP}; XP[7] = 1"), r#"emit(follow(300))"#);
	assert_eq!(out, vec![json!(["1:1 5:1 1:1", 790])]);
}

#[test]
fn a_full_quest_log_makes_the_line_that_finishes_a_quest_worth_more() {
	// 1.25 percent of a level: 2.5 for each minute of the 20 s walk, under the 3 it takes; with the log nearly full the
	// hand-in frees a slot and the line counts half as much again.
	let g = guide("", OFF_THE_WAY, "");
	let setup = format!("{ON}\n{WIDE}\n{XP}; XP[7] = 5");
	let roomy = run(&g, &setup, r#"emit(follow(300))"#);
	assert_eq!(roomy[0][0], json!("1:1 5:1"));
	let fillers = "for q = 1000, 1032 do w.onQuest(q, { { text = 'Other: 0/1', type = 'monster', required = 1, fulfilled = 0 } }, 'Other ' .. q) end; flush()";
	let full = run(&g, &format!("{setup}\n{fillers}"), r#"emit(follow(300))"#);
	assert_eq!(full[0][0], json!("1:1 5:1 1:1"));
}

#[test]
fn a_quest_whose_follow_up_the_guide_takes_next_is_worth_more() {
	// The guide's step 6 takes quest 8, which needs quest 7: finishing the vermin now opens it on that pass.
	let follow_up = r#"{ title = "More", goals = { { t = "accept", quest = 8, title = "More Vermin", npc = 12, pre = { 7 }, map = 1429, x = 47.6, y = 30 } } }"#;
	let setup = format!("{ON}\n{WIDE}\n{XP}; XP[7] = 5");
	let plain = run(&guide("", OFF_THE_WAY, ""), &setup, r#"emit(follow(300))"#);
	assert_eq!(plain[0][0], json!("1:1 5:1"));
	let chained = run(&guide("", OFF_THE_WAY, follow_up), &setup, r#"emit(follow(300))"#);
	assert!(chained[0][0].as_str().unwrap().starts_with("1:1 5:1 1:1"), "{chained:?}");
}

#[test]
fn a_stop_the_guide_comes_back_for_saves_the_trip_back_whatever_it_is_worth() {
	// The vermin are the guide's next stop after the wolves: 150 yd off the way, the guide goes south and comes back.
	// Doing them first is shorter than that, so the arrow takes them even for a quest worth almost nothing.
	let g = near("", OFF_THE_WAY, "");
	let off = run(&g, "", r#"emit(follow(300))"#);
	let on = run(&g, &format!("{ON}\n{XP}; XP[7] = 1"), r#"emit(follow(300))"#);
	assert_eq!(off[0][0], json!("1:1 2:1"));
	assert_eq!(on[0][0], json!("1:1 2:1 1:1"));
	assert!(on[0][1].as_i64() < off[0][1].as_i64(), "{on:?} against {off:?}");
}

// 340 yd along the way to the wolves, the vermin 155 yd off it: a detour of about 27 s.
const ABREAST: &str = "at(47.6, 68); at(47.6, 68)";

#[test]
fn of_two_lines_the_arrow_starts_with_the_one_worth_more_not_the_nearer() {
	// The vermin 150 yd east, the candles 90 yd east and farther south (the cheaper detour): the quest worth more leads.
	let candles = r#"{ title = "Candles", goals = { { t = "collect", item = 772, itemName = "Large Candle", count = 3, quest = 60, oi = 1, map = 1429, x = 50.6, y = 74 } } }"#;
	let g = guide("", OFF_THE_WAY, candles);
	let script = format!("flush(); {ABREAST}; emit(wp())");
	let out = run(&g, &format!("{ON}\n{WIDE}\n{XP}; XP[7] = 80; XP[60] = 1"), &script);
	assert_eq!(out, vec![json!(V)]);
	let flipped = run(&g, &format!("{ON}\n{WIDE}\n{XP}; XP[7] = 1; XP[60] = 80"), &script);
	assert_eq!(flipped, vec![json!(X)]);
}

#[test]
fn the_arrow_holds_a_line_while_it_pays_and_lets_it_go_below_the_release_score() {
	let out = run(
		&guide("", OFF_THE_WAY, ""),
		&format!("{ON}\n{WIDE}\n{XP}; XP[7] = 40"),
		&format!(
			r#"
		{ABREAST}
		emit(wp())
		-- Worth 1.25 percent of a level now: 2 for each minute, under the 3 it takes but over the 1.5 it holds.
		XP[7] = 5; stub.advance(6)
		emit(wp())
		-- A quarter of a percent: under the 1.5, so the arrow goes back to the guide.
		XP[7] = 1; stub.advance(6)
		emit(wp())"#
		),
	);
	assert_eq!(out, vec![json!(V), json!(V), json!("1:1")]);
}

#[test]
fn lowering_the_seconds_limit_hands_the_line_back() {
	let out = run(
		&guide("", OFF_THE_WAY, ""),
		&format!("{ON}\n{WIDE}\n{XP}; XP[7] = 40"),
		&format!(
			r#"
		{ABREAST}
		emit(wp())
		Polestar.Set("behavior.whileHereAimSeconds", 20); stub.advance(3)
		emit(wp())"#
		),
	);
	assert_eq!(out, vec![json!(V), json!("1:1")]);
}

#[test]
fn walking_away_from_a_held_line_hands_it_back_and_it_rests() {
	// With a wide reach, the player walks 75 yd back from the vermin after getting to 155 yd of them: past the 60 yd
	// the arrow tolerates, so it goes back to the guide and does not take the line again for 30 s.
	let out = run(
		&guide("", OFF_THE_WAY, ""),
		&format!("{ON}\n{WIDE}\nPolestar.Set(\"behavior.whileHereRadius\", 400); flush()\n{XP}; XP[7] = 400"),
		&format!(
			r#"
		{ABREAST}
		emit(wp())
		at(45.0, 68); at(45.0, 68)
		emit(wp())
		stub.advance(10)
		emit(wp())
		stub.advance(30)
		emit(wp())"#
		),
	);
	assert_eq!(out, vec![json!(V), json!("1:1"), json!("1:1"), json!(V)]);
}

#[test]
fn recalibrate_makes_the_arrow_choose_again_at_once() {
	let out = run(
		&guide("", OFF_THE_WAY, ""),
		&format!("{ON}\n{WIDE}\n{XP}; XP[7] = 40"),
		&format!(
			r#"
		{ABREAST}
		emit(wp())
		-- The limit drops under the detour: Recalibrate lets the line go in the same call, with no poll between.
		Polestar.Set("behavior.whileHereAimSeconds", 5)
		local line = E.Recalibrate()
		emit(wp())
		emit(type(line))
		-- It picks again from scratch: with the limit back up, the same call takes the line.
		Polestar.Set("behavior.whileHereAimSeconds", 60)
		E.Recalibrate()
		emit(wp())"#
		),
	);
	assert_eq!(out, vec![json!(V), json!("1:1"), json!("string"), json!(V)]);
}

#[test]
fn recalibrate_without_a_guide_says_nothing_and_with_the_setting_off_leaves_the_guides_arrow() {
	let out = run(&guide("", OFF_THE_WAY, ""), "", r#"at(47.6, 68); at(47.6, 68); E.Recalibrate(); emit(wp())"#);
	assert_eq!(out, vec![json!("1:1")]);
}

// Class quests: a goal for one class is the arrow's detour only for that class.
const CLASS_ONLY: &str = r#"x = 47.6, y = 70, only = { classes = { "WARRIOR" } }"#;

#[test]
fn a_warrior_objective_is_a_detour_for_a_warrior_and_never_for_a_druid() {
	let g = guide("", CLASS_ONLY, "");
	let warrior = run(&g, &format!("w.class, w.className, w.classID = \"WARRIOR\", \"Warrior\", 1\n{ON}"), r#"emit(follow(300))"#);
	assert_eq!(warrior, vec![json!(["1:1 5:1 1:1", 790])]);
	let druid = run(&g, &format!("w.class, w.className, w.classID = \"DRUID\", \"Druid\", 11\n{ON}"), r#"emit(follow(300))"#);
	// Only the wolves: the Druid walks 800 yd (5 yd short of them) and no objective of the Warrior's is aimed.
	assert_eq!(druid, vec![json!(["1:1", 790])]);
}
