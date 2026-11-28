//! behavior.skipSlow "Skip slow quests" (addon/Polestar/Core/Engine.lua, Own.slowSkipped, Own.slowNeeds): the
//! quests of Core/SlowQuestList.lua (here Guile of the Raptor, 701, 702 and 847) are left out of a leveling guide,
//! every goal of them, unless the quest is in the log or done, or a quest not on the list needs it. Real Core
//! manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.moveTo(1429, 48, 42)
w.titles[701] = "Guile of the Raptor"
w.titles[702] = "Guile of the Raptor"
w.titles[7] = "Kobold Camp Cleanup"
w.titles[9] = "A Quest Needing Guile"
"#;

// Step 1: the first part of the chain (accept, the kills it asks, hand-in). Step 2: its follow-up, also on the list.
// Step 3: a quest that is not on the list.
const PLAIN: &str = r#"
Polestar.RegisterGuide({
	id = "test/slow", pack = "test", title = "Slow", minLevel = 30, maxLevel = 40, steps = {
		{ title = "Tor'gan", goals = {
			{ t = "accept", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 },
			{ t = "kill", target = "Raptor", npc = 2561, count = 12, quest = 701, map = 1429, x = 47.6, y = 35.1 },
			{ t = "turnin", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 },
		} },
		{ title = "Gor'mul", goals = {
			{ t = "accept", quest = 702, title = "Guile of the Raptor", npc = 2562, npcName = "Gor'mul", map = 1429, x = 50.2, y = 40.1, pre = { 701 } },
		} },
		{ title = "Marshal McBride", goals = {
			{ t = "accept", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 },
		} },
	},
})
"#;

// The same chain, with a quest that is not on the list and needs the first part.
const NEEDED: &str = r#"
Polestar.RegisterGuide({
	id = "test/slow", pack = "test", title = "Slow", minLevel = 30, maxLevel = 40, steps = {
		{ title = "Tor'gan", goals = {
			{ t = "accept", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 },
			{ t = "turnin", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 },
		} },
		{ title = "Needing it", goals = {
			{ t = "accept", quest = 9, title = "A Quest Needing Guile", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6, pre = { 701 } },
		} },
	},
})
"#;

// A Class guide holding quest 701, and a leveling guide whose goal for 701 is for one class and whose goal for 702
// is for everyone.
const CLASS_GUIDE: &str = r#"
Polestar.RegisterGuide({
	id = "test/slow", pack = "test", title = "Class chain", category = "Class", classes = { "WARRIOR" }, steps = {
		{ title = "Trainer", goals = {
			{ t = "accept", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 },
		} },
	},
})
"#;

const CLASS_GOAL: &str = r#"
Polestar.RegisterGuide({
	id = "test/slow", pack = "test", title = "Slow", minLevel = 30, maxLevel = 40, steps = {
		{ title = "Trainer", goals = {
			{ t = "accept", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6, only = { classes = { "WARRIOR" } } },
			{ t = "accept", quest = 702, title = "Guile of the Raptor", npc = 2562, npcName = "Gor'mul", map = 1429, x = 50.2, y = 40.1 },
		} },
	},
})
"#;

const HELPERS: &str = r#"
E = Polestar.Engine
function flush() stub.advance(0.25) end
-- The status of each goal of step s.
function statuses(s)
	local out = {}
	for i, g in ipairs(E.GetStepView(s).goals) do out[i] = g.status end
	return out
end
"#;

fn run(guide: &str, on: bool, before: &str, script: &str) -> Value {
	let set = if on { r#"Polestar.Set("behavior.skipSlow", true); flush()"# } else { "" };
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: format!("{WORLD}{before}"),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: [("Polestar".to_string(), r#"PolestarCharDB = { schema = 1, guide = "test/slow" }"#.to_string())].into_iter().collect(),
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, "flush()", set, script].join("\n"),
		..Default::default()
	})
	.remove(0)
}

const ALL_NA: [&str; 3] = ["na", "na", "na"];

#[test]
fn the_setting_is_off_by_default() {
	let out = run(PLAIN, false, "", "emit({ statuses(1), statuses(2) })");
	assert!(out[0].as_array().unwrap().iter().all(|s| s != "na"), "the chain is offered: {out}");
}

#[test]
fn on_it_leaves_out_every_goal_of_the_listed_quests() {
	let out = run(PLAIN, true, "", "emit({ statuses(1), statuses(2), statuses(3) })");
	assert_eq!(out[0], json!(ALL_NA), "the first part: accept, kills and hand-in");
	assert_eq!(out[1], json!(["na"]), "its follow-up, also on the list");
	assert_ne!(out[2], json!(["na"]), "a quest that is not on the list stays");
}

#[test]
fn a_quest_in_the_log_stays() {
	let out = run(PLAIN, true, "stub.world.onQuest(701, {})", "emit(statuses(1))");
	assert!(out.as_array().unwrap().iter().all(|s| s != "na"), "the player took it: {out}");
}

#[test]
fn a_quest_a_quest_off_the_list_needs_stays() {
	let out = run(NEEDED, true, "", "emit({ statuses(1), statuses(2) })");
	assert!(out[0].as_array().unwrap().iter().all(|s| s != "na"), "the prerequisite stays: {out}");
	assert_ne!(out[1], json!(["na"]), "and so does the quest that needs it");
}

#[test]
fn turning_it_off_brings_the_quests_back() {
	let out = run(PLAIN, true, "", r#"Polestar.Set("behavior.skipSlow", false); flush(); emit(statuses(1))"#);
	assert!(out.as_array().unwrap().iter().all(|s| s != "na"), "back in the route: {out}");
}

#[test]
fn the_guide_window_says_why_it_left_the_quest_out() {
	let out = run(
		PLAIN,
		true,
		"",
		r#"
		local list = E.LeftOutHere(1, true)
		local found
		for _, o in ipairs(list) do if o.quest == 701 then found = o end end
		emit({ n = #list, kind = found and found.kind, text = found and found.text })
	"#,
	);
	assert_eq!(out["kind"], json!("slow"));
	assert!(out["text"].as_str().unwrap().contains("Skip slow quests"), "the text names the setting: {out}");
}

#[test]
fn a_class_guide_is_never_left_out() {
	let out = run(CLASS_GUIDE, true, "", "emit(statuses(1))");
	assert_ne!(out, json!(["na"]), "a Class guide's quest stays: {out}");
}

#[test]
fn a_goal_for_one_class_is_never_left_out_while_the_same_quest_for_everyone_is() {
	let off = run(CLASS_GOAL, false, "", "emit(statuses(1))");
	let on = run(CLASS_GOAL, true, "", "emit(statuses(1))");
	assert_eq!(on[0], off[0], "the class goal is as it was with the setting off");
	assert_ne!(on[0], json!("na"), "and it is not left out: {on}");
	assert_eq!(on[1], json!("na"), "the goal for everyone is left out");
}
