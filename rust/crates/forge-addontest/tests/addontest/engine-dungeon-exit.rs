//! The route after a dungeon run (addon/Polestar/Core/Engine.lua): the quest log is the truth. A quest finished
//! or handed in inside the instance leaves the route when the player is out again, however the way out is
//! signalled, and a nearby objective the arrow held for such a quest goes with it. Class quests and their chains keep their place: a class step stays the player's own, the hand-in
//! after the run is the live step, and another class passes them all. Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.level = 5
w.zone, w.subzone = "Elwynn Forest", ""
w.moveTo(1429, 48, 42)
w.titles[7] = "Kobold Camp Cleanup"; w.titles[33] = "Wolves Across the Border"; w.titles[783] = "A Threat Within"
w.titles[1500] = "The Warrior's Trial"; w.titles[1501] = "The Warrior's Reward"
"#;

// Three open-world steps, each a quest in the log, then the hand-ins and a follow-up.
const GENERAL: &str = r#"
w.onQuest(7, { { required = 10, fulfilled = 3 } }, "Kobold Camp Cleanup")
w.onQuest(33, { 8 }, "Wolves Across the Border")
Polestar.RegisterGuide({
	id = "test/general", pack = "test", title = "General", minLevel = 1, maxLevel = 10,
	steps = {
		{ title = "Vermin", goals = { { t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, x = 47.6, y = 60 } } },
		{ title = "Wolves", goals = { { t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 85 } } },
		{ title = "McBride", goals = { { t = "turnin", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 } } },
		{ title = "Willem", goals = { { t = "accept", quest = 783, title = "A Threat Within", npc = 823, npcName = "Deputy Willem", map = 1429, x = 48.2, y = 42.0 } } },
	},
})
"#;

// A Warrior's class chain: 1500 (its work is in a dungeon), then 1501 at the same trainer, and a plain quest after.
const CLASS: &str = r#"
Polestar.RegisterGuide({
	id = "test/class", pack = "test", title = "Class", minLevel = 1, maxLevel = 10,
	steps = {
		{ title = "Trial", only = { classes = { "WARRIOR" } }, goals = { { t = "accept", quest = 1500, title = "The Warrior's Trial", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6, only = { classes = { "WARRIOR" } } } } },
		{ title = "Boss", only = { classes = { "WARRIOR" } }, goals = { { t = "kill", target = "Deadmines Boss", npc = 647, count = 1, quest = 1500, oi = 1, map = 1429, x = 47.6, y = 60, only = { classes = { "WARRIOR" } } } } },
		{ title = "Hand in", only = { classes = { "WARRIOR" } }, goals = { { t = "turnin", quest = 1500, title = "The Warrior's Trial", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6, only = { classes = { "WARRIOR" } } } } },
		{ title = "Reward", only = { classes = { "WARRIOR" } }, goals = { { t = "accept", quest = 1501, title = "The Warrior's Reward", npc = 823, npcName = "Deputy Willem", map = 1429, x = 48.2, y = 42.0, pre = { 1500 }, only = { classes = { "WARRIOR" } } } } },
		{ title = "Wolves", goals = { { t = "kill", target = "Young Wolf", npc = 299, count = 8, map = 1429, x = 47.6, y = 85 } } },
	},
})
"#;

const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
function flush() stub.advance(0.6) end
function wp() local x = E.GetWaypoint(); return x and (x.stepIndex .. ":" .. x.goalIndex) or "none" end
local inside = false
function IsInInstance() return inside, inside and "party" or "none" end
function enter() inside = true; stub.fire("ZONE_CHANGED_NEW_AREA"); stub.fire("PLAYER_ENTERING_WORLD"); flush() end
-- The way out: with the zone event too, or the loading screen alone.
function leave(zoneEvent) inside = false; if zoneEvent ~= false then stub.fire("ZONE_CHANGED_NEW_AREA") end; stub.fire("PLAYER_ENTERING_WORLD"); flush() end
function where() return { E.GetLiveStep(), wp() } end
"#;

fn run(guide: &str, id: &str, before: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: format!("{WORLD}\n{before}"),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), format!(r#"PolestarCharDB = {{ schema = 1, guide = "{id}" }}"#))],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, script].join("\n"),
		..Default::default()
	})
}

fn general(script: &str) -> Vec<Value> {
	run(&format!("w = stub.world\n{GENERAL}"), "test/general", "", script)
}

fn class(before: &str, script: &str) -> Vec<Value> {
	run(CLASS, "test/class", before, script)
}

#[test]
fn a_quest_finished_inside_the_instance_leaves_the_route_when_the_player_is_out() {
	let out = general(r#"emit(where()); enter(); w.finish(7); flush(); leave(); emit(where())"#);
	assert_eq!(out, vec![json!([1, "1:1"]), json!([2, "2:1"])], "the vermin are done: the wolves are next");
}

#[test]
fn a_quest_handed_in_inside_leaves_the_route_with_the_zone_event_or_without_it() {
	for zone_event in ["true", "false"] {
		let out = general(&format!(r#"enter(); w.finish(7); w.turnIn(7); flush(); leave({zone_event}); emit(where())"#));
		assert_eq!(out, vec![json!([2, "2:1"])], "zone event {zone_event}");
	}
}

#[test]
fn a_nearby_objective_the_arrow_held_goes_when_its_quest_is_finished_inside() {
	let out = general(
		r#"
		Polestar.Set("behavior.whileHereAim", true); flush()
		w.moveTo(1429, 47.6, 55); stub.advance(2)
		emit(where())
		enter(); w.finish(7); flush(); leave()
		emit(where())"#,
	);
	assert_eq!(out, vec![json!([1, "1:1"]), json!([2, "2:1"])]);
}

#[test]
fn a_class_quest_finished_in_a_dungeon_leads_to_its_hand_in_and_the_chain_goes_on() {
	let out = class(
		r#"w.onQuest(1500, { 1 }, "The Warrior's Trial")"#,
		r#"
		emit(where())
		enter(); w.finish(1500); flush(); leave()
		emit(where())
		w.turnIn(1500); flush()
		emit(where())"#,
	);
	assert_eq!(out[0], json!([2, "2:1"]), "in the log, not done: the class boss is the live step");
	assert_eq!(out[1], json!([3, "3:1"]), "finished inside: the hand-in leads, the class steps are not passed over");
	assert_eq!(out[2], json!([4, "4:1"]), "handed in: the chain's next quest is next");
}

#[test]
fn a_class_quest_not_taken_yet_stays_the_players_through_a_dungeon_trip() {
	let out = class("", r#"emit(where()); enter(); leave(); emit(where())"#);
	assert_eq!(out, vec![json!([1, "1:1"]), json!([1, "1:1"])]);
}

#[test]
fn another_class_passes_the_class_chain_and_a_dungeon_trip_changes_nothing() {
	let out = class(r#"w.class, w.className = "MAGE", "Mage""#, r#"emit(where()); enter(); leave(); emit(where())"#);
	assert_eq!(out, vec![json!([5, "5:1"]), json!([5, "5:1"])]);
}

#[test]
fn deathskips_do_not_touch_class_steps() {
	let out = class(
		r#"w.onQuest(1500, { 1 }, "The Warrior's Trial")"#,
		r#"Polestar.Set("behavior.deathskips", true); flush(); emit(where()); w.level = 11; stub.fire("PLAYER_LEVEL_UP", 11); flush(); emit(where())"#,
	);
	assert_eq!(out, vec![json!([2, "2:1"]), json!([2, "2:1"])]);
}
