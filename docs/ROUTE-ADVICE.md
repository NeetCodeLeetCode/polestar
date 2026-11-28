# Route advice

Route advice is a card that speaks up when the quest log and the guide drift apart.
The setting is `behavior.routeAdvice` (off by default; Fastest XP turns it on).
It only advises. It never abandons, takes or leaves a quest by itself.

The judge is `ns.RouteAdvice` (`addon/Polestar/Core/RouteAdvice.lua`): plain numbers, no game calls.
The card is `UI/RouteAdviceCard.lua`. It gathers the entries, asks the judge and shows one card.

## What it reads

One measure: route distance. The seconds a stop adds to the guide's next stops (the next 8 steps).
The engine gives it as `Engine.QuestDetour(q)` for a log quest and `Engine.PickupDetour(q)` for a giver.
Both take the cheapest leg to leave and rejoin the route, so "to here, to there, to here" counts in full.
The route planner prices the two nearest spots of an objective; the other spots use straight lines.

The addon never judges kill counts, drop chances or the time a quest takes.
A quest is "slow" only when the hand-written list names it (`data/codex/slow-quests.json`).

## The three cards

| Card | When | Buttons |
|---|---|---|
| Off course | A quest in the log that no guide of the route uses adds more than 240 seconds (120 with 36 quests in the log). | Abandon, Keep it |
| Missed pickup | A quest placement passed as skipped earlier. Its giver adds 240 seconds or less: "We recommend taking it". More: "We recommend leaving it". | Take it, Leave it |
| Slow quest | A quest in the log that the hand-written list names. | Abandon, Keep it |

Abandon asks the game's own confirmation (`Compat.AskAbandonQuest`).
Take it and Leave it use `Engine.TakeEarlier` and `Engine.DismissEarlier`, the same as the guide window's list.
Keep it is saved for the character (`CharState().routeAdvice`).

## When it stays quiet

- A class quest, a timed quest, an escort, or a dungeon's quest (`Engine.AdviceHold`).
- A quest a chain needs: `Plan.OffRoute` leaves out quests that the guide's chain uses, and a missed pickup that a later accept needs is never told to be left (`Engine.AdviceNeeded`).
- A finished quest. It is handed in, never abandoned.
- A quest more than half done, or taken in the last 10 minutes.
- A quest with no spot in the quest database. Its detour is unknown, so there is no card.
- A quest you chose to keep.

## Pacing

- A look happens on a level up, a new zone, Recalibrate, a log of 36 quests and once after login. There is no timer.
- One card at a time, 10 minutes apart, 2 an hour. Never in combat or while Polestar is hidden.
- A quest gets one card for each level, whatever the verdict. Closing the card is not an answer.

## Not built yet

- A card for a quest the arrow is about to lead to far away.
- Group quests that are not in the guide: the guides hold the group flag, so an off-route group quest looks like any other.
- A second recommendation after the card ("go here next"). The guide's own arrow stays the next step.
- The limit of 240 seconds is a first guess. Play notes set it.
