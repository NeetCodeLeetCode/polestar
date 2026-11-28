'use strict';
// The audit as a report: a summary table by check, then each check's findings grouped by guide. Plain text, no
// colour, so it pastes into an issue or a pull request.

const { SEVERITY } = require('./index');

// What each check means and where a fix goes, in one line each.
const ABOUT = {
	'accept-after-handin': 'An accept that comes after the route handed the quest in. The engine counts it done, so the line is dead for that race.',
	'accept-twice': 'The same quest accepted in two steps while it is still in the log. The engine counts the second line done.',
	'never-handed-in': 'A quest the route accepts and no guide hands in.',
	'prereq-later': 'An accept whose prerequisite the route hands in later.',
	'prereq-outside-route': 'An accept whose prerequisite the route never hands in (the note says if another guide does).',
	'turnin-before-accept': 'A hand-in that comes before its accept on the route.',
	'turnin-without-accept': 'A hand-in with no accept on the route.',
	'level-locked': 'A quest that needs a higher level than the guide is for.',
	'grey-quest': 'A quest that pays half XP or less from the guide\'s first level, and nothing later needs it.',
	'race-closed': 'A quest the data closes to a race the step plays.',
	'skyborne-mask': 'A quest whose race mask lacks Skyborne, in a step that plays Skyborne. Forever\'s own race: players\' caches fixed quests 26 and 29 this way.',
	'class-closed': 'A quest the data closes to a class the step plays.',
	'wrong-side': 'A quest for the other faction.',
	'giver-differs': 'The guide takes a quest from an NPC the data does not list as its giver.',
	'ender-differs': 'The guide hands a quest in to an NPC the data does not list as its ender.',
	'giver-unknown': 'The data has no giver for the quest.',
	'ender-unknown': 'The data has no turn-in for the quest.',
	'npc-elsewhere': 'The arrow\'s point for an NPC is far from every spawn the data holds for it.',
	'prereq-drift': 'The data needs a quest first that the compiled guide does not list.',
	'level-drift': 'The data\'s minimum level differs from the compiled guide\'s.',
	'objective-differs': 'A kill, collect or interact goal aims at a target the quest\'s objective does not name.',
	'objective-missing': 'A goal names an objective number the quest does not have.',
	'count-differs': 'A collect goal wants another count than the quest.',
	'quest-unknown': 'The public quest data does not know the quest (content added by Forever, or a missing row).',
	'title-differs': 'The guide\'s title for a quest differs from the data\'s.',
	'text-unknown-id': 'A text names an NPC, quest or object id the public data does not know.',
	'round-trip': 'A step between two steps at one place: a walk out and back (a lead, not a verdict).',
	'quest-too-hard': 'A quest five levels or more above the guide\'s top level that is not a group quest.',
	'long-leg': 'A walk of 2,500 yd or more between two steps of one guide, with no flight or boat (a lead).',
	'objective-no-data': 'A goal tied to a quest objective, and the public data holds no objectives for the quest.',
	'prereq-extra': 'The compiled guide holds a quest behind a prerequisite the data does not list.',
	'data-prereq-later': 'A prerequisite the data lists (and the compiled guide does not) that the route hands in later.',
	'split-visit': 'The route stands with an NPC twice, far apart, when the later quest was open at the first visit (a lead).',
};

function where(f) {
	const b = f.branches.length > 3 ? `${f.branches.length} races` : f.branches.join(', ');
	return `${f.guide} step ${f.step}${b ? ` (${b})` : ''}`;
}

// markdown(result, { limit, minSeverity }) -> text. `limit` caps the rows listed under each check (0: all).
function markdown({ findings }, { limit = 15, minSeverity = 'info' } = {}) {
	const keep = findings.filter((f) => SEVERITY.indexOf(f.severity) <= SEVERITY.indexOf(minSeverity));
	const checks = [...new Set(keep.map((f) => f.check))];
	const byCheck = (c) => keep.filter((f) => f.check === c);
	const out = ['# Guide audit', '', `| Check | ${SEVERITY.join(' | ')} |`, `|---|${SEVERITY.map(() => '---').join('|')}|`];
	for (const c of checks) {
		const l = byCheck(c);
		out.push(`| ${c} | ${SEVERITY.map((sv) => l.filter((f) => f.severity === sv).length || '').join(' | ')} |`);
	}
	for (const c of checks) {
		const l = byCheck(c);
		out.push('', `## ${c} (${l.length})`, '', ABOUT[c] || '');
		l.sort((a, b) => SEVERITY.indexOf(a.severity) - SEVERITY.indexOf(b.severity));
		for (const f of (limit ? l.slice(0, limit) : l)) out.push(`- ${f.severity === 'info' ? '' : `${f.severity}: `}${where(f)}: ${f.message}`);
		if (limit && l.length > limit) out.push(`- ... ${l.length - limit} more (run with --limit 0)`);
	}
	return out.join('\n') + '\n';
}

module.exports = { markdown, ABOUT };
