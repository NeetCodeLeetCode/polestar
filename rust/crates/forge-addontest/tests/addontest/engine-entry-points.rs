//! Placement at the other entry points. A guide is placed from the quest log when it is loaded (login, reload, a new
//! tab, Reset guide progress); a tab that was open while the player played on elsewhere, and an open guide when the
//! player reaches a new zone, kept the step they had. Engine.SelectTab and ZONE_CHANGED_NEW_AREA now place the loaded
//! guide again (Own.placeIfMoved) when a quest event came since it was last placed. The player's own ticks and skips
//! stay, a placement the player undid stays undone, and a guide the quest log did not move is left as it is. Real Core
//! manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

// A level-20 Human in Elwynn with nothing done yet: Grey done (101) is handed in later, Grey not done (102) is never taken.
const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.moveTo(1429, 50, 50)
w.race, w.raceName, w.faction, w.class, w.className = "Human", "Human", "Alliance", "WARRIOR", "Warrior"
w.level = 20
"#;

// The route (test/low): 1 accept 101 · 2 a trip · 3 accept 102 · 4 hand-ins · 5 and 6 trips on. With 101 handed in,
// placement starts at step 5: it passes the trip to the farm and the quest 102, listed as skipped earlier.
// test/other is the guide the player plays in front of it.
const GUIDES: &str = r#"
Polestar.RegisterGuide({ id = "test/low", pack = "test", title = "Low", faction = "Alliance", minLevel = 1, maxLevel = 30, steps = {
	{ goals = { { t = "accept", quest = 101, title = "Grey done", qlvl = 5 } } },
	{ goals = { { t = "goto", map = 1429, x = 10, y = 10, label = "Farm" } } },
	{ goals = { { t = "accept", quest = 102, title = "Grey not done", qlvl = 6 } } },
	{ goals = { { t = "turnin", quest = 102, title = "Grey not done" }, { t = "turnin", quest = 101, title = "Grey done" } } },
	{ goals = { { t = "goto", map = 1429, x = 20, y = 20, label = "Town" } } },
	{ goals = { { t = "goto", map = 1429, x = 30, y = 30, label = "Camp" } } },
} })
Polestar.RegisterGuide({ id = "test/other", pack = "test", title = "Other", faction = "Alliance", minLevel = 1, maxLevel = 30, steps = {
	{ goals = { { t = "goto", map = 1429, x = 70, y = 70, label = "Mine" } } },
} })
"#;

const SETUP: &str = r#"
E = Polestar.Engine
function flush() stub.advance(0.25) end
function skips()
	local out = {}
	for k in pairs(Polestar.CharState().skipped["test/low"] or {}) do out[#out + 1] = tostring(k) end
	table.sort(out)
	return table.concat(out, " ")
end
function snapshot()
	return { guide = E.GetGuide().id, live = E.GetLiveStep(), line = E.PlacementLine() or false, skips = skips() }
end
-- The player plays on outside this guide: Grey done is handed in, and the quest log says so.
function plays_on()
	stub.world.complete({ 101 })
	stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)
end
stub.login()
stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)
"#;

// No catch-up and no guide picked at login: placement alone moves the player.
const SAVED: &str = r#"PolestarDB = { schema = 1, settings = { behavior = { skipGrey = false, autoSelectGuide = false } } }; PolestarCharDB = nil"#;

fn run(script: &str) -> Value {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), SAVED.into())],
		script: format!("{TEST_PACK}\n{TEST_SOURCE}\n{GUIDES}\n{SETUP}{script}"),
		..Default::default()
	})
	.remove(0)
}

const PLACED: &str = "Placed you at step 5: 1 step marked done, 1 quest skipped earlier.";

#[test]
fn a_tab_brought_to_the_front_is_placed_again_after_play_in_another_guide() {
	let out = run(r#"
		E.OpenTab("test/low", true); stub.advance(11)
		local opened = snapshot()
		E.OpenTab("test/other", true); stub.advance(11)
		plays_on()
		E.SelectTab(1); flush()
		emit({ opened = opened, selected = snapshot() })"#);
	assert_eq!(out["opened"], json!({ "guide": "test/low", "live": 1, "line": false, "skips": "" }), "{out}");
	assert_eq!(out["selected"], json!({ "guide": "test/low", "live": 5, "line": PLACED, "skips": "" }), "{out}");
}

#[test]
fn a_new_zone_places_the_open_guide_again_after_play_outside_it() {
	let out = run(r#"
		E.OpenTab("test/low", true); stub.advance(11)
		plays_on()
		local before = snapshot()
		stub.fire("ZONE_CHANGED_NEW_AREA"); stub.advance(1)
		emit({ before = before, after = snapshot() })"#);
	assert_eq!(out["before"]["live"], 2, "playing on outside the guide leaves it held at the trip: {out}");
	assert_eq!(out["after"], json!({ "guide": "test/low", "live": 5, "line": PLACED, "skips": "" }), "{out}");
}

#[test]
fn a_subzone_change_does_not_place_the_guide() {
	let out = run(r#"
		E.OpenTab("test/low", true); stub.advance(11)
		plays_on()
		stub.fire("ZONE_CHANGED"); stub.advance(1)
		emit(snapshot())"#);
	assert_eq!(out["live"], 2, "only a new zone places again: {out}");
}

#[test]
fn a_guide_the_quest_log_did_not_move_is_left_as_it_is() {
	let out = run(r#"
		E.OpenTab("test/low", true); stub.advance(11)
		E.OpenTab("test/other", true); stub.advance(11)
		plays_on()
		E.SelectTab(1); flush()
		local placed = snapshot()
		-- Away and back, a new zone: no quest event came, so nothing is placed again.
		E.SelectTab(2); flush()
		E.SelectTab(1); flush()
		stub.fire("ZONE_CHANGED_NEW_AREA"); stub.advance(1)
		emit({ placed = placed, again = snapshot(), report = E.PlacementReport() })"#);
	assert_eq!(out["again"], out["placed"], "{out}");
	assert_eq!(out["report"]["steps"].as_array().map(Vec::len), Some(1), "one step marked, once: {out}");
}

#[test]
fn the_players_own_skip_stays_when_the_guide_is_placed_again() {
	let out = run(r#"
		E.OpenTab("test/low", true); stub.advance(11)
		plays_on()
		stub.fire("ZONE_CHANGED_NEW_AREA"); stub.advance(1)
		E.Skip(); flush()                            -- the player passes step 5 by hand
		local skipped = snapshot()
		stub.world.complete({ 102 })
		stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)
		stub.fire("ZONE_CHANGED_NEW_AREA"); stub.advance(1)
		emit({ skipped = skipped, after = snapshot() })"#);
	assert_eq!(out["skipped"]["live"], 6, "{out}");
	assert_eq!(out["after"]["live"], 6, "the skipped step does not come back: {out}");
	assert_eq!(out["after"]["skips"], "5", "{out}");
}

#[test]
fn a_placement_the_player_undid_stays_undone_in_a_new_zone_and_a_tab_switch() {
	let out = run(r#"
		E.OpenTab("test/low", true); stub.advance(11)
		E.OpenTab("test/other", true); stub.advance(11)
		plays_on()
		E.SelectTab(1); flush()
		E.UndoPlacement(); flush()
		local undone = snapshot()
		stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)   -- a quest event since the placement
		stub.fire("ZONE_CHANGED_NEW_AREA"); stub.advance(1)
		E.SelectTab(2); flush()
		E.SelectTab(1); flush()
		emit({ undone = undone, after = snapshot() })"#);
	assert_eq!(out["undone"]["live"], 2, "{out}");
	assert_eq!(out["after"]["live"], out["undone"]["live"], "{out}");
	assert_eq!(out["after"]["line"], false, "{out}");
}
