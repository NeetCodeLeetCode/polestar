//! Sync with my quests (Engine.SyncPlacement). It places the loaded guide again from the quest log as it is now: a guide open
//! in the middle of a zone, after the player played on outside it, or after an undo. What an earlier placement marked is taken
//! back and worked out afresh; the player's own ticks and skips stay. Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

// A level-20 Human; the quests done are set by each test.
const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.moveTo(1429, 50, 50)
w.race, w.raceName, w.faction, w.class, w.className = "Human", "Human", "Alliance", "WARRIOR", "Warrior"
w.level = 20
"#;

// 1 accept 101 · 2 a trip · 3 accept 102 (never taken) · 4 hand-ins · 5 and 6 trips on. With 101 handed in, placement starts
// at step 5: it passes the trip to the farm and the quest 102, listed as skipped earlier.
const GUIDE: &str = r#"
Polestar.RegisterGuide({ id = "test/low", pack = "test", title = "Low", faction = "Alliance", minLevel = 1, maxLevel = 30, steps = {
	{ goals = { { t = "accept", quest = 101, title = "Grey done", qlvl = 5 } } },
	{ goals = { { t = "goto", map = 1429, x = 10, y = 10, label = "Farm" } } },
	{ goals = { { t = "accept", quest = 102, title = "Grey not done", qlvl = 6 } } },
	{ goals = { { t = "turnin", quest = 102, title = "Grey not done" }, { t = "turnin", quest = 101, title = "Grey done" } } },
	{ goals = { { t = "goto", map = 1429, x = 20, y = 20, label = "Town" } } },
	{ goals = { { t = "goto", map = 1429, x = 30, y = 30, label = "Camp" } } },
} })
"#;

const SETUP: &str = r#"
E = Polestar.Engine
function flush() stub.advance(0.25) end
-- Saved marks of the guide, as their keys in one string.
function keys(t)
	local out = {}
	for k in pairs(t or {}) do out[#out + 1] = tostring(k) end
	table.sort(out)
	return table.concat(out, " ")
end
function marks(field) return keys(Polestar.CharState()[field]["test/low"]) end
function recorded(field)
	local rec = Polestar.CharState().placementUndo["test/low"]
	return keys(rec and rec.put and rec.put[field])
end
function snapshot()
	return { at = E.GetStepIndex(), manual = marks("manual"), skipped = marks("skipped"), earlier = marks("earlier") }
end
stub.login()
stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)
"#;

// No catch-up and no guide picked at login: placement alone moves the player.
const SAVED: &str = r#"PolestarDB = { schema = 1, settings = { behavior = { skipGrey = false, autoSelectGuide = false } } }; PolestarCharDB = nil"#;

fn run(done: &str, script: &str) -> Value {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: format!("{WORLD}\nstub.world.complete({{ {done} }})"),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), SAVED.into())],
		script: format!("{TEST_PACK}\n{TEST_SOURCE}\n{GUIDE}\n{SETUP}{script}"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn a_guide_open_before_the_quests_moved_on_is_placed_where_they_are_now() {
	let out = run(
		"",
		r#"
		E.SetGuide("test/low"); flush(); stub.advance(11)
		local opened = snapshot()
		stub.world.complete({ 101 }); stub.fire("QUEST_LOG_UPDATE"); stub.advance(30)
		local drifted = snapshot()
		local line, marked = E.SyncPlacement(); flush()
		emit({ opened = opened, drifted = drifted, synced = snapshot(), line = line, marked = marked })"#,
	);
	assert_eq!(out["opened"]["at"], 1, "{out}");
	// Playing on outside the guide never moves a guide that is placed already: only the trip's step is passed.
	assert_eq!(out["drifted"]["at"], 2, "{out}");
	assert_eq!(out["synced"], json!({ "at": 5, "manual": "2:1", "skipped": "", "earlier": "102" }), "{out}");
	assert_eq!(out["line"], "Placed you at step 5: 1 step marked done, 1 quest skipped earlier.", "{out}");
	assert_eq!(out["marked"], true, "{out}");
}

#[test]
fn the_players_own_tick_stays_and_the_record_holds_only_the_new_placement() {
	let out = run(
		"101",
		r#"
		E.SetGuide("test/low"); flush()
		E.ToggleGoal(5, 1); flush()                  -- the player's own tick, after placement
		local before = { snap = snapshot(), put = recorded("manual") }
		E.SyncPlacement(); flush()
		local once = { snap = snapshot(), put = recorded("manual") }
		E.SyncPlacement(); flush()
		emit({ before = before, once = once, twice = { snap = snapshot(), put = recorded("manual") } })"#,
	);
	let same = json!({ "snap": { "at": 6, "manual": "2:1 5:1", "skipped": "", "earlier": "102" }, "put": "2:1" });
	assert_eq!(out["before"], same, "{out}");
	assert_eq!(out["once"], same, "a second placement marks the same and keeps the player's tick: {out}");
	assert_eq!(out["twice"], same, "and does not pile marks up: {out}");
}

#[test]
fn the_players_own_skip_stays_and_is_never_the_start() {
	let out = run(
		"101",
		r#"
		E.SetGuide("test/low"); flush()
		E.Skip(); flush()                            -- the player's own skip of step 5, the step placement chose
		local before = snapshot()
		E.SyncPlacement(); flush()
		emit({ before = before, synced = snapshot() })"#,
	);
	let same = json!({ "at": 6, "manual": "2:1", "skipped": "5", "earlier": "102" });
	assert_eq!(out["before"], same, "{out}");
	assert_eq!(out["synced"], same, "{out}");
}

#[test]
fn a_mark_placement_made_earlier_goes_when_the_quest_log_no_longer_shows_it() {
	let out = run(
		"101",
		r#"
		E.SetGuide("test/low"); flush()
		local placed = snapshot()
		-- The player abandoned the work: quest 101 is no longer handed in, so the trip to the farm is no mark placement can vouch for.
		stub.world.completed[101] = nil; stub.fire("QUEST_LOG_UPDATE"); flush()
		E.SyncPlacement(); flush()
		emit({ placed = placed, synced = snapshot() })"#,
	);
	assert_eq!(out["placed"], json!({ "at": 5, "manual": "2:1", "skipped": "", "earlier": "102" }), "{out}");
	assert_eq!(out["synced"], json!({ "at": 1, "manual": "", "skipped": "", "earlier": "" }), "{out}");
}

#[test]
fn sync_places_a_guide_whose_placement_was_undone_and_the_undo_still_works() {
	let out = run(
		"101",
		r#"
		E.SetGuide("test/low"); flush()
		E.UndoPlacement(); flush()
		local undone = snapshot()
		E.SyncPlacement(); flush()
		local synced = snapshot()
		local rec = Polestar.CharState().placementUndo["test/low"]
		emit({ undone = undone, synced = synced, undoneFlag = rec and rec.undone or false, undoAgain = E.UndoPlacement() })"#,
	);
	assert_eq!(out["undone"]["at"], 2, "{out}");
	assert_eq!(out["synced"], json!({ "at": 5, "manual": "2:1", "skipped": "", "earlier": "102" }), "{out}");
	assert_eq!(out["undoneFlag"], false, "{out}");
	assert_eq!(out["undoAgain"], true, "{out}");
}

#[test]
fn a_guide_that_needs_no_change_says_so_and_moves_nothing() {
	let out = run(
		"",
		r#"
		E.SetGuide("test/low"); flush(); stub.advance(11)
		local line, marked = E.SyncPlacement(); flush()
		emit({ at = E.GetStepIndex(), line = line, marked = marked, snap = snapshot(), none = E.PlacementReport() or false })"#,
	);
	assert_eq!(out["at"], 1, "{out}");
	assert_eq!(out["line"], "Already in sync: your quests need no change in this guide.", "{out}");
	assert_eq!(out["marked"], false, "{out}");
	assert_eq!(out["none"], false, "{out}");
}

#[test]
fn there_is_nothing_to_sync_without_a_guide() {
	let out = run("", r#"emit({ sync = E.SyncPlacement() or false })"#);
	assert_eq!(out["sync"], false, "{out}");
}

// The full addon: the menu row, the command and the banner.
fn run_window(script: &str) -> Value {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: format!("{WORLD}\nstub.world.complete({{ 101 }})"),
		addons: vec![Addon::name("Polestar")],
		saved: vec![("Polestar".into(), SAVED.into())],
		script: [
			TEST_PACK,
			TEST_SOURCE,
			GUIDE,
			r#"E = Polestar.Engine; GF = Polestar.UI.GuideFrame; W = Polestar.Widgets; function flush() stub.advance(0.5) end"#,
			"stub.login()",
			r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#,
			script,
		]
		.join("\n"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn the_menu_row_syncs_and_the_banner_offers_the_undo() {
	let out = run_window(
		r#"
		E.SetGuide("test/low"); stub.advance(3)
		GF.TakeBanner(); flush()                      -- the player undid the placement: back at the trip
		local at = E.GetStepIndex()
		local items
		local show = W.ShowMenu
		W.ShowMenu = function(_, list) items = list end
		GF.OpenMenu(GF.menuButton)
		W.ShowMenu = show
		local row
		for _, it in ipairs(items or {}) do if it.text == "Sync with my quests" then row = it end end
		local found, disabled = row ~= nil, row and row.disabled or false
		if row then row.onClick() end
		flush()
		emit({ found = found, disabled = disabled, at = at, synced = E.GetStepIndex(), banner = GF.BannerNotice() })"#,
	);
	assert_eq!(out, json!({ "found": true, "disabled": false, "at": 2, "synced": 5, "banner": "placed" }), "{out}");
}

#[test]
fn the_sync_command_prints_what_it_marked() {
	let out = run_window(
		r#"
		E.SetGuide("test/low"); stub.advance(3)
		E.UndoPlacement(); flush()
		local from = #stub.messages
		SlashCmdList.POLESTAR("sync"); flush()
		emit({ printed = table.concat(stub.messages, "\n", from + 1), at = E.GetStepIndex() })"#,
	);
	let printed = out["printed"].as_str().unwrap_or("");
	assert!(printed.contains("Placed you at step 5:") && printed.contains("1 quest skipped earlier."), "{out}");
	assert_eq!(out["at"], 5, "{out}");
}
