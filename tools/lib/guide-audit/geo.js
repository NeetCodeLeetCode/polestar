'use strict';
// Map percent <-> world yards, from the in-game maps' frames (forever-db ui_map: x runs north, y runs west, so a
// map's left edge is its largest y and its top edge its largest x).

// The frame of a ui map: { map (continent), left, right, top, bottom } or undefined.
function frameOf(ref, uiMap) {
	return ref.frames.get(uiMap);
}

// A goal's point (map percent) in world yards on the map's continent, or null.
function toWorld(ref, uiMap, x, y) {
	const f = frameOf(ref, uiMap);
	if (!f || x == null || y == null) return null;
	return { continent: f.map, x: f.top + (f.bottom - f.top) * (y / 100), y: f.left + (f.right - f.left) * (x / 100) };
}

// A spawn point (world yards) in map percent on a ui map, or null when it lies outside the map.
function toMap(ref, uiMap, s) {
	const f = frameOf(ref, uiMap);
	if (!f || f.map !== s.map) return null;
	const px = (f.left - s.y) / (f.left - f.right);
	const py = (f.top - s.x) / (f.top - f.bottom);
	if (px < -0.02 || px > 1.02 || py < -0.02 || py > 1.02) return null;
	return { x: px * 100, y: py * 100 };
}

function yards(a, b) {
	if (!a || !b || a.continent !== b.continent) return Infinity;
	return Math.hypot(a.x - b.x, a.y - b.y);
}

// The nearest spawn of an NPC or object to a goal's point: { yards, at: { x, y } in map percent on that ui map }.
// yards is Infinity when either is unknown.
function nearestSpawnAt(ref, kind, id, uiMap, x, y) {
	const here = toWorld(ref, uiMap, x, y);
	const list = ref.spawns[kind] && ref.spawns[kind].get(id);
	const out = { yards: Infinity, at: null };
	if (!here || !list) return out;
	for (const s of list) {
		const d = yards(here, { continent: s.map, x: s.x, y: s.y });
		if (d < out.yards) { out.yards = d; out.at = toMap(ref, uiMap, s); }
	}
	return out;
}


module.exports = { toWorld, toMap, yards, nearestSpawnAt, frameOf };
