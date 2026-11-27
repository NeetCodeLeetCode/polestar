//! A vendor, a buy, an inn or a trainer that a guide leaves to the nearest candidate (`vendor`, `buy`,
//! `sethome nearest`, `train <profession>`, `train class`) is the one on the guide's way, not the one nearest the
//! player (addon/Polestar/Core/Engine.lua, pickFor and Own.routeFirst). Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.race, w.raceName, w.faction, w.class, w.className = "Human", "Human", "Alliance", "MAGE", "Mage"
w.level = 10
w.moveTo(1429, 50, 20)
"#;

// The player stands at y 20. The north candidate is the nearest (8 points of y, 160 yards); the south one is 12
// points away (240 yards). Each point of y is 20 yards on this map.
const NORTH: &str = r#"{ npc = 1, npcName = "North", map = 1429, x = 50, y = 12, faction = "A" }"#;
const SOUTH: &str = r#"{ npc = 2, npcName = "South", map = 1429, x = 50, y = 32, faction = "A" }"#;

fn goal(kind: &str, candidates: &[&str]) -> String {
	let head = match kind {
		"vendor" => r#"t = "vendor""#,
		"buy" => r#"t = "buy", item = 2901, itemName = "Mining Pick""#,
		"sethome" => r#"t = "sethome", location = "Goldshire""#,
		"train" => r#"t = "train", skill = "Mining""#,
		"train class" => r#"t = "train", class = true"#,
		_ => unreachable!("{kind}"),
	};
	format!("{{ {head}, candidates = {{ {} }} }}", candidates.join(", "))
}

// The goal's step, then `ahead` (goals of the steps after it), then the end.
fn guide(kind: &str, candidates: &[&str], ahead: &[&str]) -> String {
	let mut steps = vec![format!(r#"{{ title = "Pick", goals = {{ {} }} }}"#, goal(kind, candidates))];
	for (i, g) in ahead.iter().enumerate() {
		steps.push(format!(r#"{{ title = "Ahead {i}", goals = {{ {g} }} }}"#));
	}
	steps.push(r#"{ goals = { { t = "level", level = 60 } } }"#.into());
	format!(r#"Polestar.RegisterGuide({{ id = "test/pick", pack = "test", title = "Pick", minLevel = 1, maxLevel = 60, steps = {{ {} }} }})"#, steps.join(",\n"))
}

fn goto(y: u32) -> String {
	format!(r#"{{ t = "goto", map = 1429, x = 50, y = {y} }}"#)
}

fn run(g: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::name("Polestar")],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/pick" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, g, "E = Polestar.Engine", "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, script].join("\n"),
		..Default::default()
	})
}

// The chosen candidate's name, the arrow's place and how many others the goal offers.
const PICK: &str = r#"
	local v = E.GetStepView(1).goals[1]
	local wp = E.GetWaypoint()
	emit({ pick = v.npcName, arrow = wp and wp.y, others = v.alternatives })
"#;

fn pick(kind: &str, candidates: &[&str], ahead: &[&str]) -> Value {
	run(&guide(kind, candidates, ahead), PICK).remove(0)
}

const KINDS: [&str; 5] = ["vendor", "buy", "sethome", "train", "train class"];

// The report: with the route going south the arrow chose the off-route vendor in the north, and with the route
// going north it chose the same one.
#[test]
fn the_candidate_on_the_way_wins_over_a_nearer_one_off_it() {
	for kind in KINDS {
		let south = pick(kind, &[NORTH, SOUTH], &[&goto(90)]);
		assert_eq!(south["pick"], "South", "{kind}, the way goes south: {south}");
		assert_eq!(south["others"], 1, "{kind}: {south}");
		let north = pick(kind, &[NORTH, SOUTH], &[&goto(2)]);
		assert_eq!(north["pick"], "North", "{kind}, the way goes north: {north}");
	}
}

// The listing order of the guide's candidates does not decide.
#[test]
fn the_order_of_the_candidates_in_the_guide_does_not_decide() {
	for kind in KINDS {
		assert_eq!(pick(kind, &[SOUTH, NORTH], &[&goto(90)])["pick"], "South", "{kind}");
		assert_eq!(pick(kind, &[SOUTH, NORTH], &[&goto(2)])["pick"], "North", "{kind}");
	}
}

// The arrow leads to the chosen one.
#[test]
fn the_arrow_leads_to_the_pick_on_the_way() {
	let out = pick("vendor", &[NORTH, SOUTH], &[&goto(90)]);
	assert_eq!(out["arrow"], 32.0, "{out}");
	let out = pick("vendor", &[NORTH, SOUTH], &[&goto(2)]);
	assert_eq!(out["arrow"], 12.0, "{out}");
}

// A guide with no route ahead of the pick keeps the nearest.
#[test]
fn with_no_route_ahead_the_nearest_stays() {
	for kind in KINDS {
		assert_eq!(pick(kind, &[SOUTH, NORTH], &[])["pick"], "North", "{kind}");
	}
}

// Two on the way: the detour is no different (within Own.ROUTE_TIE), so the nearer one stays.
#[test]
fn two_on_the_way_keep_the_nearer() {
	let near = r#"{ npc = 3, npcName = "Near", map = 1429, x = 50, y = 26, faction = "A" }"#;
	assert_eq!(pick("vendor", &[SOUTH, near], &[&goto(90)])["pick"], "Near");
}

// The route reads the first open place of each step ahead, not a step of its own whose place is a pick too:
// a second vendor ahead does not move the first one's route.
#[test]
fn a_candidate_goal_ahead_is_no_point_of_the_route() {
	let ahead_vendor = goal("vendor", &[NORTH, SOUTH]);
	assert_eq!(pick("vendor", &[NORTH, SOUTH], &[&ahead_vendor, &goto(90)])["pick"], "South");
	assert_eq!(pick("vendor", &[NORTH, SOUTH], &[&ahead_vendor, &goto(2)])["pick"], "North");
}

// A candidate the character cannot use never wins, on the way or not.
#[test]
fn a_candidate_of_the_other_faction_never_wins() {
	let horde = r#"{ npc = 4, npcName = "Horde", map = 1429, x = 50, y = 32, faction = "H" }"#;
	assert_eq!(pick("vendor", &[NORTH, horde], &[&goto(90)])["pick"], "North");
}

// Cycling still goes through the ranking: the pick on the way first, then the other.
#[test]
fn cycling_moves_from_the_pick_on_the_way_to_the_other() {
	let out = run(
		&guide("vendor", &[NORTH, SOUTH], &[&goto(90)]),
		r#"
		local before = E.GetStepView(1).goals[1].npcName
		E.CycleCandidate(1, 1)
		stub.advance(0.3)
		emit({ before, E.GetStepView(1).goals[1].npcName })
	"#,
	);
	assert_eq!(out, vec![json!(["South", "North"])]);
}

// The reminder's and Nearby's class trainer stays the one nearest the player.
#[test]
fn the_nearest_class_trainer_stays_by_place() {
	let trainers = r#"
		Polestar.RegisterClassTrainers({
			{ npc = 1, npcName = "North", map = 1429, x = 50, y = 12, faction = "A", classes = { "MAGE" } },
			{ npc = 2, npcName = "South", map = 1429, x = 50, y = 32, faction = "A", classes = { "MAGE" } },
		})
		emit(E.NearestClassTrainer().npcName)
	"#;
	let out = run(&guide("vendor", &[NORTH, SOUTH], &[&goto(90)]), trainers);
	assert_eq!(out, vec![json!("North")]);
}

// Price: a zone change picks again; one candidate in the tier plans no route, two plan their legs, and a pick
// is kept until the next zone change or step start.
#[test]
fn route_plans_are_made_only_to_choose_between_candidates() {
	let count = r#"
		local plans = 0
		local cost = Polestar.TravelCost
		Polestar.TravelCost = function(...) plans = plans + 1; return cost(...) end
		stub.fire("ZONE_CHANGED_NEW_AREA"); stub.advance(0.3)
		E.GetStepView(1)
		local first = plans
		E.GetStepView(1); E.GetStepView(1); E.GetWaypoint()
		emit({ first = first, again = plans - first })
	"#;
	let one = run(&guide("vendor", &[SOUTH], &[&goto(90)]), count).remove(0);
	let two = run(&guide("vendor", &[NORTH, SOUTH], &[&goto(90)]), count).remove(0);
	assert_eq!(one["again"], 0, "a pick is kept: {one}");
	assert_eq!(two["again"], 0, "a pick is kept: {two}");
	// Both rank from where the player stands (one plan each); only two candidates price legs, three plans each.
	assert_eq!(two["first"].as_u64().unwrap() - one["first"].as_u64().unwrap(), 1 + 2 * 3, "{one} {two}");
}
