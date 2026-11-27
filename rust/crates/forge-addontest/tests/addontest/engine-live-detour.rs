//! behavior.whileHereAim (addon/Polestar/Core/Engine.lua, Own.hereAutoAim): the arrow leads to a "While you're here"
//! objective by itself, as if the player had clicked the line, when it costs few seconds off the way the guide leads.
//! Off by default. The tests walk a simulated player that follows only the arrow, at 7 yd/s, so the walked yards
//! and the order of the arrow's stops are the measure. Real Core manifest, stub world.
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

// The arrow's stops in order, the yards walked and the seconds, for a player who only follows the arrow.
const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
QUEST = { ["1:1"] = 33, ["2:1"] = 7, ["3:1"] = 60 }
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
		if key ~= last then seen[#seen + 1], last = key, key end
		local dx, dy = (x.x - w.x) * 30, (x.y - w.y) * 20
		local d = math.sqrt(dx * dx + dy * dy)
		if d <= 5 then
			w.finish(QUEST[key])
			stub.advance(0.6)
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

// Step 1: the wolves, 800 yd south of the player. Step 2: the vermin at (vx, vy). `extra`: more steps.
fn guide(wolves_extra: &str, vermin: &str, extra: &str) -> String {
	format!(
		r#"
Polestar.RegisterGuide({{
	id = "test/detour", pack = "test", title = "Detour", minLevel = 1, maxLevel = 10,
	steps = {{
		{{ title = "Wolves", goals = {{ {{ t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 85{wolves_extra} }} }} }},
		{{ title = "Vermin", goals = {{ {{ t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, {vermin} }} }} }},
		{extra}
	}},
}})
"#
	)
}

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
	assert_eq!(out, vec![json!(false), json!(["1:1 2:1", 1090])]);
}

#[test]
fn an_objective_on_the_way_is_taken_and_the_walk_gets_shorter() {
	let out = run(&guide("", ON_THE_WAY, ""), ON, r#"emit(follow(300))"#);
	// The vermin first (500 yd), then the wolves (300 yd): 790 yd, not 1090.
	assert_eq!(out, vec![json!(["1:1 2:1 1:1", 790])]);
}

#[test]
fn a_detour_over_the_seconds_limit_stays_with_the_guide_and_a_higher_limit_takes_it() {
	let g = guide("", OFF_THE_WAY, "");
	let out = run(&g, ON, r#"emit(follow(300))"#);
	assert_eq!(out[0][0], json!("1:1 2:1"), "15 s is under the detour");
	let out = run(&g, &format!("{ON}\nPolestar.Set(\"behavior.whileHereAimSeconds\", 60); flush()"), r#"emit(follow(300))"#);
	assert_eq!(out[0][0], json!("1:1 2:1 1:1"), "60 s covers it");
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
	assert_eq!(out, vec![json!("2:1"), json!("1:1")]);
}

#[test]
fn a_line_the_player_clicked_is_not_replaced() {
	// The vermin (on the way, 0 s) and the candles (90 yd east, about 8 s): the nearest detour would win by itself.
	let candles = r#"{ title = "Candles", goals = { { t = "collect", item = 772, itemName = "Large Candle", count = 3, quest = 60, oi = 1, map = 1429, x = 50.6, y = 68 } } }"#;
	let g = guide("", ON_THE_WAY, candles);
	let alone = run(&g, ON, r#"at(47.6, 62); at(47.6, 62); emit(wp())"#);
	assert_eq!(alone, vec![json!("2:1")]);
	let clicked = run(
		&g,
		"",
		r#"
		at(47.6, 62)
		for i, v in ipairs(E.GetWhileHereList()) do if v.step == 3 then E.AimWhileHere(i) end end
		flush()
		emit(wp())
		Polestar.Set("behavior.whileHereAim", true); stub.advance(3)
		emit(wp())"#,
	);
	assert_eq!(clicked, vec![json!("3:1"), json!("3:1")]);
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
	assert_eq!(out, vec![json!("2:1"), json!("1:1"), json!("1:1")]);
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
	assert_eq!(out, vec![json!("1:1"), json!("2:1")]);
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
