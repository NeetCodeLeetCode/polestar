//! Scenario simulator for Fastest XP: a player that follows only the arrow plays the shipped guides on the stub
//! world with a clock, XP and levels (tests/lua/arrow_player.lua), under many seeded scenarios, and the run is
//! measured: XP for each minute against the guide's order, the arrow's reversals, stuck states, rules broken
//! (an accept taken before its prerequisite), and the CPU and memory of the updates. Every scenario is named by
//! its seed, so a failure is a repro: run the test with `SCENARIO_SEED=<n>`.

use super::engine_live_speed::Rng;
use forge_addontest::arrowplayer::ARROW_PLAYER;
use forge_addontest::{emitted, Addon, RunAddon};
use serde_json::Value;

/// One character, one guide, one set of settings and a list of events.
#[derive(Clone)]
pub(crate) struct Scene {
	pub faction: &'static str,
	pub race: &'static str,
	pub class: &'static str,
	pub level: u32,
	pub guide: &'static str,
	/// `key = value` Lua pairs written before the run: ("behavior.whileHereAim", "true").
	pub settings: Vec<(&'static str, &'static str)>,
	/// Lua table text of events, see tests/lua/arrow_player.lua.
	pub events: String,
	pub seconds: u32,
	pub stop_level: u32,
	pub hardcore: bool,
	pub mount: bool,
	/// Lua run after the settings, before the player starts.
	pub lua: String,
}

impl Scene {
	pub fn new(faction: &'static str, race: &'static str, class: &'static str, level: u32, guide: &'static str) -> Scene {
		Scene { faction, race, class, level, guide, settings: vec![], events: "{}".into(), seconds: 1800, stop_level: 0, hardcore: false, mount: false, lua: String::new() }
	}
	pub fn with(mut self, settings: &[(&'static str, &'static str)]) -> Scene {
		self.settings.extend_from_slice(settings);
		self
	}
}

pub(crate) const FASTEST: &[(&str, &str)] = &[("behavior.fastestXp", "true")];

// Death and a map-aware moveTo: the engine reads UnitIsDeadOrGhost when it loads, so the override is in `before`.
const PRELUDE: &str = r#"
DEAD = false
function UnitIsDeadOrGhost(unit) return unit == "player" and DEAD end
"#;

/// Plays the scene and returns the metrics table of ARROW.run.
pub(crate) fn play(s: &Scene) -> Value {
	let dir = forge_addontest::run::addons_dir();
	let guides = if s.faction == "Horde" { "Polestar_Guides_Horde" } else { "Polestar_Guides_Alliance" };
	let set: String = s.settings.iter().map(|(k, v)| format!("Polestar.Set(\"{k}\", {v}); stub.advance(0.3)\n")).collect();
	let stop = if s.stop_level > 0 { format!("level = {},", s.stop_level) } else { String::new() };
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: format!(
			r#"local w = stub.world
w.race, w.faction, w.class, w.className, w.level = "{}", "{}", "{}", "{}", {}
w.xp, w.xpMax = 0, 400
hardcoreOn = {}
C_GameRules = {{ IsHardcoreActive = function() return hardcoreOn end }}
{PRELUDE}"#,
			s.race, s.faction, s.class, s.class, s.level, s.hardcore
		),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"]), Addon::dir("Polestar_Guides", dir.join("Polestar_Guides")), Addon::dir(guides, dir.join(guides))],
		saved: vec![("Polestar".into(), "PolestarDB = { schema = 1, settings = { behavior = { suggestFrom = \"guides\" } } }".into())],
		script: format!(
			r#"
E = Polestar.Engine
for id, b in pairs(Polestar.MapFrames.era) do stub.world.addMap(id, b) end
do
	local w, move = stub.world, stub.world.moveTo
	w.moveTo = function(map, x, y, z)
		move(map, x, y, z)
		-- The zone text follows the map (era frames carry no names: the map id stands in), as the client's does.
		local m = w.maps[map]
		local name = m and m.name and m.name ~= "" and m.name or ("Map " .. tostring(map))
		if map ~= ZONEMAP then ZONEMAP = map; if name ~= w.zone then w.setZone(name, "") end end
	end
end
stub.login(); stub.advance(0.25)
Polestar.Set("behavior.guideEnd", "continue")
{set}
{lua}
E.SetGuide("{guide}"); stub.advance(0.25)
stub.fire("QUEST_LOG_UPDATE"); stub.advance(11)
do -- the player stands at the first place of the guide
	local g = E.GetGuide()
	for _, st in ipairs(g.steps) do
		local goal = st.goals[1]
		if goal and goal.map and goal.x then stub.world.moveTo(goal.map, goal.x, goal.y); break end
	end
	stub.advance(2)
end
{player}
emit(ARROW.run({{ seconds = {seconds}, {stop} mount = {mount}, events = {events} }}))
"#,
			guide = s.guide,
			lua = s.lua,
			player = ARROW_PLAYER.as_str(),
			seconds = s.seconds,
			mount = s.mount,
			events = s.events,
		),
		..Default::default()
	})
	.remove(0)
}

fn num(v: &Value, k: &str) -> f64 {
	v[k].as_f64().unwrap_or(0.0)
}

fn summary(name: &str, m: &Value) -> String {
	format!(
		"{name}: {}s, level {} (from {}), xp {} ({:.0}/min), {} yd, {} hand-ins, {} kills, {} jumps, {} reversals, stuck {}, breaks {}, cpu max {:.0} ms, mem {:+.0} KB, guide {} step {}",
		num(m, "seconds"),
		num(m, "level"),
		num(m, "level0"),
		num(m, "xp"),
		num(m, "xpPerMin"),
		num(m, "yards"),
		num(m, "handins"),
		num(m, "kills"),
		num(m, "jumps"),
		num(m, "reversals"),
		!m["stuck"].is_null(),
		m["breaks"].as_array().map_or(0, |a| a.len()),
		num(m, "cpuMax") * 1000.0,
		num(m, "mem"),
		m["guide"],
		m["step"],
	)
}

/// The characters: (name, faction, race, class, level, guide). Every start of the shipped guides, then a guide of each band.
pub(crate) const CHARS: &[(&str, &str, &str, &str, u32, &str)] = &[
	("orc-warrior-1", "Horde", "Orc", "WARRIOR", 1, "forever/horde/orc/01-durotar"),
	("tauren-druid-1", "Horde", "Tauren", "DRUID", 1, "forever/horde/tauren/01-mulgore"),
	("undead-mage-1", "Horde", "Undead", "MAGE", 1, "forever/horde/undead/01-tirisfal-glades"),
	("troll-priest-1", "Horde", "Troll", "PRIEST", 1, "forever/horde/troll/01-durotar"),
	("human-paladin-1", "Alliance", "Human", "PALADIN", 1, "forever/alliance/human/01-elwynn-forest"),
	("dwarf-hunter-1", "Alliance", "Dwarf", "HUNTER", 1, "forever/alliance/dwarf/01-dun-morogh"),
	("nightelf-rogue-1", "Alliance", "NightElf", "ROGUE", 1, "forever/alliance/nightelf/01-teldrassil"),
	("gnome-warlock-1", "Alliance", "Gnome", "WARLOCK", 1, "forever/alliance/gnome/01-dun-morogh"),
	("orc-shaman-12", "Horde", "Orc", "SHAMAN", 12, "forever/horde/02-the-barrens"),
	("human-warrior-15", "Alliance", "Human", "WARRIOR", 15, "forever/alliance/04-redridge-mountains"),
	("troll-hunter-22", "Horde", "Troll", "HUNTER", 22, "forever/horde/07-ashenvale"),
	("dwarf-priest-31", "Alliance", "Dwarf", "PRIEST", 31, "forever/alliance/30-40/01-hillsbrad-foothills"),
	("undead-warlock-33", "Horde", "Undead", "WARLOCK", 33, "forever/horde/30-40/03-stranglethorn-vale"),
	("human-mage-42", "Alliance", "Human", "MAGE", 42, "forever/alliance/40-50/04-tanaris"),
	("orc-rogue-52", "Horde", "Orc", "ROGUE", 52, "forever/horde/50-60/01-un-goro-crater"),
];

/// Life: random events for `secs` seconds from `seed`, a gap of about 10 minutes between them. `kinds` picks from
/// "die", "wander", "afk", "abandon", "level", "recalibrate", "toggle".
pub(crate) fn events(seed: u64, secs: u32, kinds: &[&str]) -> String {
	let mut rng = Rng(seed * 2654435761 + 99);
	let mut at = 120.0;
	let mut out = String::from("{");
	while at < secs as f64 && !kinds.is_empty() {
		let kind = kinds[(rng.next() * kinds.len() as f64) as usize % kinds.len()];
		let n = (rng.next() * 100.0) as u32;
		let ev = match kind {
			"die" => format!("kind = \"die\", yards = {}, angle = {:.2}", rng.between(300.0, 900.0) as u32, rng.between(0.0, 6.28)),
			"wander" => format!("kind = \"wander\", yards = {}, secs = {}, angle = {:.2}", rng.between(200.0, 900.0) as u32, rng.between(20.0, 180.0) as u32, rng.between(0.0, 6.28)),
			"shift" => format!("kind = \"shift\", yards = {}, angle = {:.2}", rng.between(100.0, 700.0) as u32, rng.between(0.0, 6.28)),
			"afk" => format!("kind = \"afk\", secs = {}", rng.between(30.0, 300.0) as u32),
			"abandon" => format!("kind = \"abandon\", n = {n}"),
			"level" => "kind = \"level\", n = 1".to_string(),
			"recalibrate" => format!("kind = \"recalibrate\", n = {}, gap = 0.1", 1 + n % 25),
			"toggle" => {
				let keys = ["behavior.fastestXp", "behavior.whileHereAim", "behavior.stepReorder", "behavior.skipGrey", "behavior.skipSlow", "behavior.whileHere"];
				let key = keys[(rng.next() * keys.len() as f64) as usize % keys.len()];
				format!("kind = \"set\", key = \"{key}\", value = {}", if rng.next() < 0.5 { "true" } else { "false" })
			}
			other => panic!("unknown event kind {other}"),
		};
		out += &format!("{{ at = {}, {ev} }}, ", at as u32);
		at += rng.between(300.0, 900.0);
	}
	out + "}"
}

/// One scenario played twice, on the same seed: the guide's order (defaults) and Fastest XP.
pub(crate) struct Pair {
	pub name: String,
	pub seed: u64,
	pub d: Value,
	pub f: Value,
}

pub(crate) fn pair(c: &(&'static str, &'static str, &'static str, &'static str, u32, &'static str), seed: u64, secs: u32, kinds: &[&str]) -> Pair {
	let mut s = Scene::new(c.1, c.2, c.3, c.4, c.5);
	s.seconds = secs;
	s.events = events(seed, secs, kinds);
	let d = play(&s);
	let f = play(&s.clone().with(FASTEST));
	Pair { name: c.0.to_string(), seed, d, f }
}

fn flag(m: &Value) -> String {
	let mut v = Vec::new();
	if !m["stuck"].is_null() {
		v.push(format!("STUCK@{} step {}", m["stuck"]["at"], m["stuck"]["step"]));
	}
	if num(m, "grindXp") > 0.25 * num(m, "xp") {
		v.push(format!("grind {:.0}%", 100.0 * num(m, "grindXp") / num(m, "xp").max(1.0)));
	}
	if num(m, "reversals") > 0.0 {
		v.push(format!("{} reversals", num(m, "reversals")));
	}
	if m["breaks"].as_array().is_some_and(|a| !a.is_empty()) {
		v.push(format!("breaks {}", m["breaks"]));
	}
	v.join(", ")
}

#[test]
#[ignore = "campaign"]
fn campaign_calm() {
	let secs: u32 = std::env::var("SCENARIO_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(1800);
	let seeds: u64 = std::env::var("SCENARIO_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
	let kinds: Vec<&str> = std::env::var("SCENARIO_KINDS").ok().map(|v| v.split(',').filter(|k| !k.is_empty()).map(|k| &*Box::leak(k.to_string().into_boxed_str())).collect()).unwrap_or_default();
	let only = std::env::var("SCENARIO_ONLY").unwrap_or_default();
	let mut ratio = Vec::new();
	for c in CHARS.iter().filter(|c| only.is_empty() || c.0.contains(&only)) {
		for seed in 1..=seeds {
			let p = pair(c, seed, secs, &kinds);
			let (xd, xf) = (num(&p.d, "xp"), num(&p.f, "xp"));
			ratio.push(xf / xd.max(1.0));
			eprintln!(
				"{:<18} seed {seed}: D {:>6.0} xp lv{} {:>5} yd {:>3} hand-ins | F {:>6.0} xp lv{} {:>5} yd {:>3} hand-ins | F/D {:.2} | cpu {:.0}/{:.0} ms | D: {} | F: {}",
				p.name,
				xd,
				num(&p.d, "level"),
				num(&p.d, "yards"),
				num(&p.d, "handins"),
				xf,
				num(&p.f, "level"),
				num(&p.f, "yards"),
				num(&p.f, "handins"),
				xf / xd.max(1.0),
				num(&p.d, "cpuMax") * 1000.0,
				num(&p.f, "cpuMax") * 1000.0,
				flag(&p.d),
				flag(&p.f)
			);
		}
	}
	ratio.sort_by(|a, b| a.partial_cmp(b).unwrap());
	eprintln!("F/D XP ratio over {} scenarios: min {:.2}, median {:.2}, max {:.2}", ratio.len(), ratio[0], ratio[ratio.len() / 2], ratio[ratio.len() - 1]);
}

/// One run printed whole, with the stuck state: SCENARIO_ONLY=<name> SCENARIO_FAST=1 for Fastest XP.
#[test]
#[ignore = "probe"]
fn probe_one() {
	let only = std::env::var("SCENARIO_ONLY").unwrap_or_default();
	let seed: u64 = std::env::var("SCENARIO_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
	let secs: u32 = std::env::var("SCENARIO_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(1800);
	let kinds: Vec<&str> = std::env::var("SCENARIO_KINDS").ok().map(|v| v.split(',').filter(|k| !k.is_empty()).map(|k| &*Box::leak(k.to_string().into_boxed_str())).collect()).unwrap_or_default();
	let c = CHARS.iter().find(|c| c.0 == only).expect("SCENARIO_ONLY names a character");
	let mut s = seeded(c, seed, secs, &kinds);
	if let Ok(lua) = std::env::var("SCENARIO_CALL") {
		// SCENARIO_CALL="<seconds>:<Lua body>": run Lua at that second, its prints go to the trace.
		let (at, body) = lua.split_once(':').expect("SCENARIO_CALL is <seconds>:<Lua>");
		let extra = format!("{{ at = {at}, kind = \"call\", fn = function() {body} end }}");
		s.events = if s.events == "{}" { format!("{{ {extra} }}") } else { s.events.replacen('{', &format!("{{ {extra}, "), 1) };
	}
	if std::env::var("SCENARIO_FAST").is_ok() {
		s = s.with(FASTEST);
	}
	let mut m = play(&s);
	eprintln!("{}", summary(&only, &m));
	let trace = if m["stuck"].is_null() { m["trace"].clone() } else { m["stuck"]["trace"].clone() };
	eprintln!("grind: {}", m["grindWhy"]);
	if let Some(o) = m.as_object_mut() {
		o.remove("trace");
		if let Some(st) = o.get_mut("stuck").and_then(|v| v.as_object_mut()) {
			st.remove("trace");
		}
	}
	if let Some(t) = trace.as_array() {
		for l in t.iter().rev().take(std::env::var("SCENARIO_TRACE").ok().and_then(|v| v.parse().ok()).unwrap_or(25)).rev() {
			eprintln!("  trace: {}", l.as_str().unwrap_or(""));
		}
	}
	eprintln!("{}", serde_json::to_string_pretty(&m).unwrap().chars().take(6000).collect::<String>());
}

/// The mode with each lever group put back to its default, one at a time, for one character.
#[test]
#[ignore = "probe"]
fn probe_ablation() {
	let only = std::env::var("SCENARIO_ONLY").unwrap_or_default();
	let secs: u32 = std::env::var("SCENARIO_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(7200);
	let seed: u64 = std::env::var("SCENARIO_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
	let kinds = family_kinds(&std::env::var("SCENARIO_FAMILY").unwrap_or_else(|_| "calm".into()));
	for c in CHARS.iter().filter(|c| c.0.contains(&only)) {
		let base = {
			let mut s = Scene::new(c.1, c.2, c.3, c.4, c.5);
			s.seconds = secs;
			s.stop_level = target_level(c.4);
			s.events = events(seed, secs, &kinds);
			s
		};
		let mut arms: Vec<(String, Scene)> = vec![("defaults".into(), base.clone()), ("mode".into(), base.clone().with(FASTEST))];
		for group in ["here", "aim", "reorder", "automation", "skips", "deathskips", "travel"] {
			let mut s = base.clone();
			s.lua = super::engine_live_ablation::without(group);
			arms.push((format!("mode minus {group}"), s));
		}
		for key in ["behavior.skipGrey", "behavior.skipSlow", "behavior.rareDrops"] {
			let mut s = base.clone();
			s.lua = format!("Polestar.Set(\"behavior.fastestXp\", true); stub.advance(0.6)\nPolestar.Set(\"{key}\", Polestar.GetDefault(\"{key}\")); stub.advance(0.6)\n");
			arms.push((format!("mode minus {}", key.trim_start_matches("behavior.")), s));
		}
		let mut xp = base.clone().with(FASTEST);
		xp.lua = "Polestar.Set(\"behavior.stepReorderXp\", true); stub.advance(0.6)\n".into();
		arms.push(("mode plus XP order".into(), xp));
		for (name, s) in arms {
			let m = play(&s);
			eprintln!(
				"{:<18} {:<24} {:>5.0}s lv{} xp {:>7.0} yd {:>6.0} hand-ins {:>3.0} kills {:>4.0} grind {:>5.0} xp  {}",
				c.0,
				name,
				num(&m, "seconds"),
				num(&m, "level"),
				num(&m, "xp"),
				num(&m, "yards"),
				num(&m, "handins"),
				num(&m, "kills"),
				num(&m, "grindXp"),
				flag(&m)
			);
		}
	}
}

/// The kinds of events a family of scenarios draws from.
fn family_kinds(family: &str) -> Vec<&'static str> {
	match family {
		"calm" => vec![],
		"life" => vec!["die", "wander", "afk", "abandon"],
		"levels" => vec!["level", "die"],
		"toggle" => vec!["toggle", "recalibrate"],
		"chaos" => vec!["die", "wander", "afk", "abandon", "level", "toggle", "recalibrate"],
		other => panic!("unknown family {other}"),
	}
}

/// The level a character plays to: ten for a new one, three more for the others.
fn target_level(level: u32) -> u32 {
	if level == 1 {
		10
	} else {
		level + 3
	}
}

/// One character's scenario on a seed: the events of the family, and for the seeds after the first, a start somewhere
/// else in the zone (seed 1 starts where the guide starts).
pub(crate) fn seeded(c: &(&'static str, &'static str, &'static str, &'static str, u32, &'static str), seed: u64, secs: u32, kinds: &[&str]) -> Scene {
	let mut s = Scene::new(c.1, c.2, c.3, c.4, c.5);
	s.seconds = secs;
	s.stop_level = target_level(c.4);
	let mut ev = events(seed, secs, kinds);
	if seed > 1 {
		let shift = format!("{{ at = 1, kind = \"shift\", yards = {}, angle = {} }}, ", 100 + seed * 130 % 600, seed as f64 * 1.3);
		ev = ev.replacen('{', &format!("{{ {shift}"), 1);
	}
	s.events = ev;
	s
}

/// Every character, `seeds` seeds each, the guide's order against Fastest XP, one JSON line a run in $SCENARIO_OUT.
/// SCENARIO_FAMILY=calm|life|levels|toggle|chaos, SCENARIO_SEEDS=1-3, SCENARIO_ONLY=<part of a name>, SCENARIO_SECONDS.
#[test]
#[ignore = "campaign"]
fn campaign() {
	let family = std::env::var("SCENARIO_FAMILY").unwrap_or_else(|_| "calm".into());
	let (lo, hi) = {
		let v = std::env::var("SCENARIO_SEEDS").unwrap_or_else(|_| "1-2".into());
		let (a, b) = v.split_once('-').unwrap_or((&v, &v));
		(a.parse::<u64>().unwrap(), b.parse::<u64>().unwrap())
	};
	let secs: u32 = std::env::var("SCENARIO_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(7200);
	let only = std::env::var("SCENARIO_ONLY").unwrap_or_default();
	let out = std::env::var("SCENARIO_OUT").ok();
	let kinds = family_kinds(&family);
	for c in CHARS.iter().filter(|c| only.is_empty() || c.0.contains(&only)) {
		for seed in lo..=hi {
			let s = seeded(c, seed, secs, &kinds);
			let d = play(&s);
			let f = play(&s.clone().with(FASTEST));
			let (sd, sf) = (num(&d, "seconds"), num(&f, "seconds"));
			eprintln!(
				"{:<18} {family} seed {seed}: D {:>5.0}s lv{} {:>4.0} hand-ins {:>6.0} yd | F {:>5.0}s lv{} {:>4.0} hand-ins {:>6.0} yd | F/D time {:.2} | D: {} | F: {}",
				c.0,
				sd,
				num(&d, "level"),
				num(&d, "handins"),
				num(&d, "yards"),
				sf,
				num(&f, "level"),
				num(&f, "handins"),
				num(&f, "yards"),
				sf / sd.max(1.0),
				flag(&d),
				flag(&f)
			);
			if let Some(path) = &out {
				use std::io::Write;
				let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
				for (arm, m) in [("D", &d), ("F", &f)] {
					let mut row = m.clone();
					if let Some(o) = row.as_object_mut() {
						o.remove("trace");
						o.insert("name".into(), c.0.into());
						o.insert("family".into(), family.clone().into());
						o.insert("seed".into(), seed.into());
						o.insert("arm".into(), arm.into());
						o.insert("target".into(), s.stop_level.into());
					}
					writeln!(file, "{row}").unwrap();
				}
			}
		}
	}
}
