//! A hand-in that is not ready never leads the arrow while a line of its quest-log entry is open (#3381;
//! addon/Polestar/Core/Engine.lua, Own.objectiveAim). The guide's own goal for the line leads, wherever it stands
//! (a step the player skipped, an optional goal, a later step than the hand-in); with no goal for the line in the
//! guide, one built from the quest database does. Real Core manifest, stub world, the fixture quest database.

use forge_addontest::questdb::{build_db, fixture_dir, QuestDb};
use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.moveTo(1429, 48, 42)
w.titles[7] = "Kobold Camp Cleanup"
"#;

const ACCEPT: &str = r#"{ t = "accept", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 }"#;
const KILL: &str = r#"{ t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, x = 47.6, y = 35.1 OPT }"#;
const TURNIN: &str = r#"{ t = "turnin", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 }"#;
const LEVEL: &str = r#"{ t = "level", level = 6 }"#;

fn guide(steps: &[String]) -> String {
	format!(r#"Polestar.RegisterGuide({{ id = "test/ofirst", pack = "test", title = "OF", minLevel = 1, maxLevel = 6, steps = {{ {} }} }})"#, steps.join(",\n"))
}

fn step(title: &str, goals: &[&str]) -> String {
	format!(r#"{{ title = "{}", goals = {{ {} }} }}"#, title, goals.join(", "))
}

// Accept, the kills (OPT: extra fields of the goal), the hand-in, the end.
fn three_steps(opt: &str) -> String {
	guide(&[step("McBride", &[ACCEPT]), step("Kobolds", &[&KILL.replace("OPT", opt)]), step("Back", &[TURNIN]), step("End", &[LEVEL])])
}

const HELPERS: &str = r#"
E = Polestar.Engine
function flush() stub.advance(0.25) end
-- "<step>:<goal>:<verb>" of the goal the arrow leads to.
function wp()
	local w = E.GetWaypoint()
	local g = w and E.GetGuide().steps[w.stepIndex].goals[w.goalIndex]
	return w and (w.stepIndex .. ":" .. w.goalIndex .. ":" .. g.t) or "none"
end
"#;

fn fixture() -> QuestDb {
	build_db(&fixture_dir())
}

fn run_with(db: Option<&QuestDb>, g: &str, script: &str) -> Vec<Value> {
	let before = db.map_or(String::new(), |db| format!("stub.quests.dbFiles = {}\n", db.lua_files()));
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua", "stub_quests.lua"],
		before: format!("{before}{WORLD}"),
		addons: vec![Addon::name("Polestar")],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/ofirst" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, g, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, script].join("\n"),
		..Default::default()
	})
}

fn run(g: &str, script: &str) -> Vec<Value> {
	run_with(None, g, script)
}

// The guide's order: the kills lead, and the hand-in leads once the quest is ready.
#[test]
fn the_work_leads_the_arrow_and_the_hand_in_leads_once_the_quest_is_ready() {
	let out = run(
		&three_steps(""),
		r#"
		stub.world.accept(7, { 10 }); flush()
		emit({ E.GetLiveStep(), wp() })
		stub.world.progress(7, 1, 10); flush()
		emit({ E.GetLiveStep(), wp() })
	"#,
	);
	assert_eq!(out, vec![json!([2, "2:1:kill"]), json!([3, "3:1:turnin"])]);
}

// The player skipped the work's step: the quest is at 0/10 and the live step is the hand-in. The arrow leads to the
// kills of the skipped step; at 10/10 it leads to the hand-in.
#[test]
fn a_skipped_work_step_leaves_the_arrow_on_the_kills_until_the_quest_is_ready() {
	let out = run(
		&three_steps(""),
		r#"
		stub.world.accept(7, { 10 }); flush()
		E.Skip(); flush()
		emit({ E.GetLiveStep(), wp() })
		stub.world.progress(7, 1, 10); flush()
		emit({ E.GetLiveStep(), wp() })
	"#,
	);
	assert_eq!(out, vec![json!([3, "2:1:kill"]), json!([3, "3:1:turnin"])]);
}

// The work goal is optional: it never holds the hand-in's step, but the quest is not ready, so the arrow leads to it.
#[test]
fn an_optional_work_goal_leads_the_arrow_before_its_quest_s_hand_in() {
	let out = run(
		&three_steps(", optional = true"),
		r#"
		stub.world.accept(7, { 10 }); flush()
		emit({ E.GetLiveStep(), wp() })
		stub.world.progress(7, 1, 10); flush()
		emit({ E.GetLiveStep(), wp() })
	"#,
	);
	assert_eq!(out, vec![json!([3, "2:1:kill"]), json!([3, "3:1:turnin"])]);
}

// The guide writes the hand-in before the work (a writer's order slip): the work's goal, a later step's, leads.
#[test]
fn a_hand_in_written_before_its_work_leads_the_arrow_to_the_work_ahead() {
	let g = guide(&[step("McBride", &[ACCEPT]), step("Back", &[TURNIN]), step("Kobolds", &[&KILL.replace("OPT", "")]), step("End", &[LEVEL])]);
	let out = run(
		&g,
		r#"
		stub.world.accept(7, { 10 }); flush()
		emit({ E.GetLiveStep(), wp() })
		stub.world.progress(7, 1, 10); flush()
		emit({ E.GetLiveStep(), wp() })
	"#,
	);
	assert_eq!(out, vec![json!([2, "3:1:kill"]), json!([2, "2:1:turnin"])]);
}

// The guide has no goal for the quest's work at all (#3381: the quest data lists none, or the quest was carried in):
// without a quest database the arrow has nowhere else to lead, so it stays on the hand-in; with one, it leads to the
// objective's spawns, built from the database.
#[test]
fn a_guide_without_the_work_leads_to_the_database_s_spawns_when_it_has_them() {
	let g = guide(&[step("Back", &[TURNIN]), step("End", &[LEVEL])]);
	let script = r#"
		stub.world.accept(7, { 10 }); flush()
		local w = E.GetWaypoint()
		emit({ E.GetLiveStep(), w and w.kind, w and w.map, w and math.floor(w.y) })
	"#;
	assert_eq!(run(&g, script), vec![json!([1, "turnin", 1429, 41])], "no quest database: the hand-in");
	let db = fixture();
	let out = run_with(Some(&db), &g, script);
	assert_eq!(out, vec![json!([1, "kill", 1429, 35])], "the kobolds of the fixture database, north of the hand-in at y 41");
}

// A quest the player ticked by hand (#1135) keeps its hand-in waiting and takes no focus: the arrow does not lead
// back to the work the player left, nor to the hand-in.
#[test]
fn a_ticked_work_goal_still_holds_the_hand_in_and_the_arrow_does_not_return_to_it() {
	let out = run(
		&three_steps(""),
		r#"
		stub.world.accept(7, { 10 }); flush()
		E.ToggleGoal(2, 1); flush()
		emit({ E.GetLiveStep(), wp() })
	"#,
	);
	assert_eq!(out, vec![json!([3, "none"])]);
}

// Work and hand-in in one step: the work first holds the hand-in, as before.
#[test]
fn work_and_hand_in_in_one_step_keep_their_order() {
	let g = guide(&[step("McBride", &[ACCEPT]), step("Kobolds and back", &[&KILL.replace("OPT", ""), TURNIN]), step("End", &[LEVEL])]);
	let out = run(
		&g,
		r#"
		stub.world.accept(7, { 10 }); flush()
		emit({ E.GetLiveStep(), wp() })
		stub.world.progress(7, 1, 10); flush()
		emit({ E.GetLiveStep(), wp() })
	"#,
	);
	assert_eq!(out, vec![json!([2, "2:1:kill"]), json!([2, "2:2:turnin"])]);
}
