//! ns.RouteAdvice (addon/Polestar/Core/RouteAdvice.lua): the judge of the route advice cards. Plain numbers: a log
//! quest off the route is told to be abandoned when it adds more than 240 seconds (120 with a log of 36), a missed
//! pickup is taken when near and left when far (never one a later accept needs), a quest of the hand-written list is
//! told once, and a held, kept, finished, mostly done or fresh quest is left alone. Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const HELPERS: &str = r#"
RA = Polestar.RouteAdvice
function off(q, seconds, extra)
	local e = { kind = "off", quest = q, title = "Q" .. q, seconds = seconds, ready = false, progress = 0, age = 3600 }
	for k, v in pairs(extra or {}) do e[k] = v end
	return e
end
function missed(q, seconds, extra)
	local e = { kind = "missed", quest = q, title = "Q" .. q, seconds = seconds }
	for k, v in pairs(extra or {}) do e[k] = v end
	return e
end
function listed(q, extra)
	local e = { kind = "listed", quest = q, title = "Q" .. q, code = "low-drop", ready = false, progress = 0, age = 3600 }
	for k, v in pairs(extra or {}) do e[k] = v end
	return e
end
function verdicts(entries, log)
	local out = {}
	for _, a in ipairs(RA.Judge(entries, log or 10)) do out[#out + 1] = a.entry.quest .. ":" .. a.verdict end
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
fn a_log_quest_is_off_course_only_above_the_limit() {
	let out = run(r#"
		emit({ below = verdicts({ off(1, 240) }), above = verdicts({ off(1, 241) }), unknown = verdicts({ off(1, nil) }) })"#);
	assert_eq!(out, json!({ "below": "", "above": "1:abandon", "unknown": "" }));
}

#[test]
fn a_log_of_36_quests_halves_the_limit() {
	let out = run(r#"
		emit({ limit = tostring(RA.Limit(10)), crowded = tostring(RA.Limit(36)), few = verdicts({ off(1, 150) }, 35), many = verdicts({ off(1, 150) }, 36) })"#);
	assert_eq!(out, json!({ "limit": "240", "crowded": "120", "few": "", "many": "1:abandon" }));
}

#[test]
fn a_held_kept_finished_mostly_done_or_fresh_quest_is_left_alone() {
	let out = run(r#"
		emit({
			class = verdicts({ off(1, 900, { hold = "class" }) }),
			kept = verdicts({ off(1, 900, { kept = true }) }),
			ready = verdicts({ off(1, 900, { ready = true }) }),
			half = verdicts({ off(1, 900, { progress = 0.5 }) }),
			most = verdicts({ off(1, 900, { progress = 0.51 }) }),
			fresh = verdicts({ off(1, 900, { age = 599 }) }),
			old = verdicts({ off(1, 900, { age = 600 }) }),
			unknownAge = verdicts({ off(1, 900, { age = false }) }),
		})"#);
	assert_eq!(out["class"], "");
	assert_eq!(out["kept"], "");
	assert_eq!(out["ready"], "");
	assert_eq!(out["half"], "1:abandon");
	assert_eq!(out["most"], "");
	assert_eq!(out["fresh"], "");
	assert_eq!(out["old"], "1:abandon");
}

#[test]
fn a_missed_pickup_is_taken_when_near_and_left_when_far_but_never_when_a_later_accept_needs_it() {
	let out = run(r#"
		emit({
			near = verdicts({ missed(1, 30) }),
			far = verdicts({ missed(1, 600) }),
			needed = verdicts({ missed(1, 600, { needed = true }) }),
			neededNear = verdicts({ missed(1, 30, { needed = true }) }),
			held = verdicts({ missed(1, 30, { hold = "timed" }) }),
			unknown = verdicts({ missed(1, nil) }),
		})"#);
	assert_eq!(out, json!({ "near": "1:take", "far": "1:leave", "needed": "", "neededNear": "1:take", "held": "", "unknown": "" }));
}

#[test]
fn a_listed_quest_is_told_unless_it_is_held_kept_finished_or_mostly_done() {
	let out = run(r#"
		emit({
			plain = verdicts({ listed(1) }),
			kept = verdicts({ listed(1, { kept = true }) }),
			ready = verdicts({ listed(1, { ready = true }) }),
			class = verdicts({ listed(1, { hold = "class" }) }),
			most = verdicts({ listed(1, { progress = 0.9 }) }),
		})"#);
	assert_eq!(out, json!({ "plain": "1:listed", "kept": "", "ready": "", "class": "", "most": "" }));
}

#[test]
fn the_dearest_detour_comes_first_then_the_list_then_pickups_nearest_first() {
	let out = run(r#"
		emit(verdicts({ missed(5, 600), missed(4, 90), missed(3, 30), listed(2), off(1, 300), off(6, 700) }))"#);
	assert_eq!(out, json!("6:abandon 1:abandon 2:listed 3:take 4:take 5:leave"));
}

#[test]
fn a_card_waits_ten_minutes_and_at_most_two_an_hour() {
	let out = run(r#"
		local s = { times = {} }
		local r = { first = RA.Allowed(s, 0) }
		RA.Record(s, 0)
		r.at599 = RA.Allowed(s, 599)
		r.at600 = RA.Allowed(s, 600)
		RA.Record(s, 600)
		r.third = RA.Allowed(s, 1500)
		r.afterAnHour = RA.Allowed(s, 3601)
		emit(r)"#);
	assert_eq!(out, json!({ "first": true, "at599": false, "at600": true, "third": false, "afterAnHour": true }));
}
