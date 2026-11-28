//! behavior.fastestXp "Fastest XP" (addon/Polestar/Core/FastestXP.lua): one switch that writes the fastest value into
//! each real setting, keeps what it replaced, reads "customized" after a hand change, and puts back only what the
//! player did not change since. Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.moveTo(1429, 48, 42)
w.titles[701] = "Guile of the Raptor"
"#;

// A listed slow quest (701), so the engine's reaction to the written settings shows.
const GUIDE: &str = r#"
Polestar.RegisterGuide({
	id = "test/fast", pack = "test", title = "Fast", minLevel = 30, maxLevel = 40, steps = {
		{ title = "Tor'gan", goals = {
			{ t = "accept", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 },
		} },
	},
})
"#;

const HELPERS: &str = r#"
E = Polestar.Engine
FX = Polestar.FastestXP
function flush() stub.advance(0.25) end
function status() return E.GetStepView(1).goals[1].status end
-- Every value the mode writes, as the settings hold them.
function held()
	local out = {}
	for key in pairs(FX.Values()) do out[key] = Polestar.Get(key) end
	return out
end
"#;

fn run(before: &str, script: &str) -> Value {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: format!("{WORLD}{before}"),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: [("Polestar".to_string(), r#"PolestarCharDB = { schema = 1, guide = "test/fast" }"#.to_string())].into_iter().collect(),
		script: [TEST_PACK, TEST_SOURCE, GUIDE, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, "flush()", script].join("\n"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn it_is_off_by_default_and_changes_nothing_until_turned_on() {
	let out = run("", r#"emit({ on = FX.On(), state = FX.State(), reorder = Polestar.Get("behavior.stepReorder"), slow = Polestar.Get("behavior.skipSlow") })"#);
	assert_eq!(out, json!({ "on": false, "state": "off", "reorder": false, "slow": false }));
}

#[test]
fn turning_it_on_writes_the_value_of_every_lever() {
	let out = run(
		"",
		r#"
		Polestar.Set("behavior.fastestXp", true)
		local want, bad = FX.Values(), {}
		for key, value in pairs(want) do if Polestar.Get(key) ~= value then bad[#bad + 1] = key end end
		emit({ state = FX.State(), bad = #bad, n = (function() local n = 0 for _ in pairs(want) do n = n + 1 end return n end)(),
			aim = Polestar.Get("behavior.whileHereAim"), reorder = Polestar.Get("behavior.stepReorder"),
			slow = Polestar.Get("behavior.skipSlow"), rare = Polestar.Get("behavior.rareDrops"), death = Polestar.Get("behavior.deathskips") })"#,
	);
	assert_eq!(out["bad"], 0, "each setting holds the mode's value: {out}");
	assert_eq!(out["state"], "on");
	assert!(out["n"].as_u64().unwrap() >= 17, "the levers of the Core manifest are all in force: {out}");
	assert_eq!(out["aim"], true);
	assert_eq!(out["reorder"], true);
	assert_eq!(out["slow"], true);
	assert_eq!(out["rare"], "hide");
	assert_eq!(out["death"], true);
}

#[test]
fn a_setting_the_player_changed_by_hand_reads_customized_and_the_mode_stays_on() {
	let out = run(
		"",
		r#"
		Polestar.Set("behavior.fastestXp", true)
		Polestar.Set("behavior.whileHereTurninSeconds", 40)
		local changed = {}
		for _, lv in ipairs(FX.Changed()) do changed[#changed + 1] = lv.key end
		emit({ on = FX.On(), state = FX.State(), changed = changed })"#,
	);
	assert_eq!(out, json!({ "on": true, "state": "customized", "changed": ["behavior.whileHereTurninSeconds"] }));
}

#[test]
fn turning_it_off_puts_back_what_it_replaced_and_keeps_a_hand_change() {
	let out = run(
		"",
		r#"
		Polestar.Set("behavior.whileHereRadius", 250)
		Polestar.Set("behavior.skipGrey", false)
		Polestar.Set("behavior.fastestXp", true)
		local during = { radius = Polestar.Get("behavior.whileHereRadius"), grey = Polestar.Get("behavior.skipGrey") }
		Polestar.Set("behavior.stepReorder", false) -- the player turns one off by hand
		Polestar.Set("behavior.fastestXp", false)
		emit({ during = during, radius = Polestar.Get("behavior.whileHereRadius"), grey = Polestar.Get("behavior.skipGrey"),
			reorder = Polestar.Get("behavior.stepReorder"), aim = Polestar.Get("behavior.whileHereAim"), slow = Polestar.Get("behavior.skipSlow"),
			rare = Polestar.Get("behavior.rareDrops"), state = FX.State(), undo = Polestar.GetStored("fastest.undo") })"#,
	);
	assert_eq!(out["during"], json!({ "radius": 180, "grey": true }), "on: the mode's values");
	assert_eq!(out["radius"], 250, "the value the player had comes back");
	assert_eq!(out["grey"], false, "so does a setting they had turned off");
	assert_eq!(out["reorder"], false, "the setting they changed by hand is theirs");
	assert_eq!(out["aim"], false);
	assert_eq!(out["slow"], false);
	assert_eq!(out["rare"], "optional");
	assert_eq!(out["state"], "off");
	assert_eq!(out["undo"], Value::Null, "nothing is kept once it is off");
}

#[test]
fn restore_writes_the_values_again_and_the_old_ones_still_come_back_at_the_end() {
	let out = run(
		"",
		r#"
		Polestar.Set("behavior.whileHereRadius", 250)
		Polestar.Set("behavior.fastestXp", true)
		Polestar.Set("behavior.whileHereRadius", 100)
		local custom = FX.State()
		local done = FX.Reapply()
		local after = { state = FX.State(), radius = Polestar.Get("behavior.whileHereRadius"), n = #done }
		Polestar.Set("behavior.fastestXp", false)
		emit({ custom = custom, after = after, radius = Polestar.Get("behavior.whileHereRadius") })"#,
	);
	assert_eq!(out["custom"], "customized");
	assert_eq!(out["after"], json!({ "state": "on", "radius": 180, "n": 1 }));
	assert_eq!(out["radius"], 250, "what the player had before the mode, not the value it wrote");
}

#[test]
fn a_hardcore_character_keeps_the_arrow_and_deathskips_as_they_are() {
	let out = run(
		"C_GameRules = { IsHardcoreActive = function() return true end }",
		r#"
		Polestar.Set("behavior.fastestXp", true)
		local values = FX.Values()
		emit({ aim = Polestar.Get("behavior.whileHereAim"), seconds = Polestar.Get("behavior.whileHereAimSeconds"), death = Polestar.Get("behavior.deathskips"),
			reorder = Polestar.Get("behavior.stepReorder"), slow = Polestar.Get("behavior.skipSlow"),
			left = values["behavior.whileHereAim"] == nil and values["behavior.deathskips"] == nil, state = FX.State() })"#,
	);
	assert_eq!(out["aim"], false, "no arrow that leaves the guide by itself");
	assert_eq!(out["death"], false);
	assert_eq!(out["seconds"], 15);
	assert_eq!(out["reorder"], true, "the other levers apply");
	assert_eq!(out["slow"], true);
	assert_eq!(out["left"], true, "neither is among the values it writes");
	assert_eq!(out["state"], "on", "left alone is not a hand change");
}

#[test]
fn a_profile_switch_or_a_batch_does_not_apply_or_release_it() {
	let out = run(
		"",
		r#"
		Polestar.Fire("SETTING_CHANGED", "", nil)
		local first = Polestar.Get("behavior.stepReorder")
		Polestar.Fire("SETTING_CHANGED", "behavior", nil)
		emit({ first = first, reorder = Polestar.Get("behavior.stepReorder"), on = FX.On() })"#,
	);
	assert_eq!(out, json!({ "first": false, "reorder": false, "on": false }));
}

#[test]
fn the_engine_follows_the_written_settings() {
	let out = run(
		"",
		r#"
		local before = status()
		Polestar.Set("behavior.fastestXp", true); flush()
		local on = status()
		Polestar.Set("behavior.fastestXp", false); flush()
		emit({ before = before, on = on, off = status() })"#,
	);
	assert_ne!(out["before"], "na", "offered: {out}");
	assert_eq!(out["on"], "na", "Skip slow quests, written by the mode, leaves the listed quest out: {out}");
	assert_eq!(out["off"], out["before"], "and turning the mode off brings it back");
}

#[test]
fn the_command_turns_it_on_and_off_and_says_its_state() {
	let out = run(
		"",
		r#"
		Polestar.RunCommand("fast on")
		local on = FX.On()
		Polestar.Set("behavior.whileHereRadius", 100)
		Polestar.RunCommand("fast restore")
		local radius = Polestar.Get("behavior.whileHereRadius")
		Polestar.RunCommand("fast")
		Polestar.RunCommand("fast off")
		emit({ on = on, radius = radius, off = FX.On() })"#,
	);
	assert_eq!(out, json!({ "on": true, "radius": 180, "off": false }));
}

// Rare-drop quests hidden by the mode are for level-range guides only: a Class guide keeps its optional start.
const DROPS: &str = r#"
Polestar.RegisterGuide({
	id = "test/fast", pack = "test", title = "Drops", CATEGORY minLevel = 1, maxLevel = 30,
	steps = {
		{ title = "Woods", goals = {
			{ t = "kill", target = "Kobold Vermin", npc = 6, count = 10, map = 1429, x = 47.6, y = 35.1 },
			{ t = "collect", item = 2794, itemName = "An Old History Book", count = 1, optional = true, map = 1429, x = 48.5, y = 40.5 },
			{ t = "accept", quest = 337, title = "An Old History Book", item = 2794, itemName = "An Old History Book", optional = true },
		} },
	},
})
"#;

fn drops(category: &str) -> Value {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: [("Polestar".to_string(), r#"PolestarCharDB = { schema = 1, guide = "test/fast" }"#.to_string())].into_iter().collect(),
		script: [
			TEST_PACK,
			TEST_SOURCE,
			&DROPS.replace("CATEGORY", category),
			HELPERS,
			"stub.login()",
			r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#,
			r#"Polestar.Set("behavior.fastestXp", true); flush()"#,
			r#"local out = {} for i, g in ipairs(E.GetStepView(1).goals) do out[i] = g.status end
			emit({ out, E.IsLeveling(), Polestar.Get("behavior.rareDrops") })"#,
		]
		.join("\n"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn the_mode_hides_rare_drop_quests_in_a_leveling_guide_only() {
	let leveling = drops("");
	let class = drops(r#"category = "Class","#);
	assert_eq!(leveling[0][1], "na", "hidden until the item drops: {leveling}");
	assert_eq!(leveling[0][2], "na");
	assert_eq!(leveling[1], true);
	assert_ne!(class[0][1], "na", "a Class guide keeps its optional start: {class}");
	assert_ne!(class[0][2], "na");
	assert_eq!(class[1], false);
	assert_eq!(class[2], "hide", "the setting itself is still written");
}
