//! ns.NextStop (addon/Polestar/Core/NextStop.lua): ranks hubs, zones and dungeons by XP for each minute, trip and work
//! together. Plain numbers: finish the hub before leaving it (the next place must be 1.3 times as good), a dungeon only
//! for a group of 3 or a route that takes dungeons, the Hardcore filter forced. Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const HELPERS: &str = r#"
NS = Polestar.NextStop
-- A place of one quest worth `xp` percent of a level for `secs` seconds of work, `trip` seconds away.
function place(id, kind, trip, xp, secs, extra)
	local q = { xp = xp, secs = secs, level = 20 }
	for k, v in pairs(extra or {}) do q[k] = v end
	return { id = id, kind = kind, name = id, tripSecs = trip, quests = { q } }
end
function ids(r)
	local out = {}
	for _, e in ipairs(r.ranked) do out[#out + 1] = e.place.id .. ":" .. e.reason end
	return table.concat(out, " ")
end
"#;

fn run(script: &str) -> Value {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		script: [TEST_PACK, TEST_SOURCE, HELPERS, "stub.login()", script].join("\n"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn a_place_is_ranked_by_xp_for_each_minute_of_trip_and_work() {
	let out = run(r#"
		local near = place("near", "zone", 60, 10, 540)  -- 10 percent in 10 minutes
		local far = place("far", "zone", 600, 20, 600)   -- 20 percent in 20 minutes
		local r = NS.Rank({ level = 20 }, { far, near })
		emit({ order = ids(r), rate = r.top.rate, minutes = r.top.minutes })
		"#);
	assert_eq!(out["order"], "near:best far:next");
	assert_eq!(out["rate"], 1.0);
	assert_eq!(out["minutes"], 10.0);
}

#[test]
fn the_hub_is_finished_before_leaving_unless_the_next_place_is_30_percent_better() {
	let out = run(r#"
		local here = place("here", "hub", 0, 10, 600)           -- 1.00 for each minute
		local a = place("a", "zone", 0, 12.9, 600)               -- 1.29: not enough
		local b = place("b", "zone", 0, 13.0, 600)               -- 1.30: enough
		local r1 = NS.Rank({ level = 20, here = here }, { a })
		local r2 = NS.Rank({ level = 20, here = here }, { a, b })
		emit({ stay = ids(r1), leave = ids(r2), top = r2.top.place.id })
		"#);
	assert_eq!(out["stay"], "here:stay a:next");
	assert_eq!(out["leave"], "b:leave here:stay a:next");
	assert_eq!(out["top"], "b");
}

#[test]
fn a_hub_with_nothing_left_is_not_a_reason_to_stay() {
	let out = run(r#"
		local here = { id = "here", kind = "hub", tripSecs = 0, quests = { { xp = 0, secs = 60, level = 5 } } } -- grey
		local r = NS.Rank({ level = 20, here = here }, { place("next", "zone", 120, 5, 300) })
		emit({ order = ids(r), hasStay = r.stay ~= nil })
		"#);
	assert_eq!(out, json!({ "order": "next:best", "hasStay": false }));
}

#[test]
fn a_dungeon_needs_a_group_of_three_or_a_route_that_takes_dungeons() {
	let out = run(r#"
		local dungeon = place("dungeon", "dungeon", 300, 40, 1500)
		local zone = place("zone", "zone", 60, 3, 600)
		local function listed(state) return ids(NS.Rank(state, { zone, dungeon })) end
		emit({
			alone = listed({ level = 20, group = 1, dungeons = "skip" }),
			two = listed({ level = 20, group = 2, dungeons = "skip" }),
			three = listed({ level = 20, group = 3, dungeons = "skip" }),
			route = listed({ level = 20, group = 1, dungeons = "route" }),
		})
		"#);
	assert_eq!(out["alone"], "zone:best");
	assert_eq!(out["two"], "zone:best");
	assert_eq!(out["three"], "dungeon:best zone:next");
	assert_eq!(out["route"], "dungeon:best zone:next");
}

#[test]
fn hardcore_leaves_out_escorts_and_quests_over_two_levels_above_the_player() {
	let out = run(r#"
		local mixed = { id = "mixed", kind = "zone", tripSecs = 0, quests = {
			{ xp = 5, secs = 300, level = 20 },
			{ xp = 5, secs = 300, level = 22 },
			{ xp = 5, secs = 300, level = 23 },
			{ xp = 5, secs = 300, level = 20, escort = true },
		} }
		local normal = NS.Rank({ level = 20 }, { mixed }).top
		local hard = NS.Rank({ level = 20, hardcore = true }, { mixed }).top
		emit({ normal = normal.quests, normalXp = normal.xp, hard = hard.quests, hardXp = hard.xp })
		"#);
	assert_eq!(out, json!({ "normal": 4, "normalXp": 20, "hard": 2, "hardXp": 10 }));
}

#[test]
fn a_place_with_no_paying_quest_is_not_ranked_and_a_tie_goes_to_the_shorter_trip_then_the_id() {
	let out = run(r#"
		local empty = place("empty", "zone", 0, 0, 100)
		local b = place("b", "zone", 60, 6, 300)    -- 1 for each minute
		local a = place("a", "zone", 60, 6, 300)
		local c = place("c", "zone", 0, 6, 360)     -- 1 for each minute, no trip
		emit({ order = ids(NS.Rank({ level = 20 }, { b, empty, a, c })) })
		"#);
	assert_eq!(out["order"], "c:best a:next b:next");
}

#[test]
fn the_top_is_first_whatever_the_rank_of_staying() {
	let out = run(r#"
		local here = place("here", "hub", 0, 10, 600)
		local far = place("far", "zone", 60, 30, 540)
		local r = NS.Rank({ level = 20, here = here }, { far })
		emit({ first = r.ranked[1].place.id, top = r.top.place.id, stay = r.stay.place.id, reason = r.top.reason })
		"#);
	assert_eq!(out, json!({ "first": "far", "top": "far", "stay": "here", "reason": "leave" }));
}
