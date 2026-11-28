# Next stop

`ns.NextStop` (`addon/Polestar/Core/NextStop.lua`) ranks places by XP for each minute.
A place is a hub, a zone or a dungeon. The file holds plain numbers: no game calls, no settings read.

## Rank

`NS.Rank(state, places)` returns `{ ranked, top, stay }`.
Each entry holds the place, its XP (percent of a level), its minutes (trip and work), its rate and a reason.
The reasons are `stay`, `leave`, `best` and `next`.

## Rules

- Finish the hub before leaving it. The top is another place only when its rate is 1.3 times the rate of staying, or more.
- If nothing is left to do here, any place with a rate beats staying.
- A dungeon is a candidate with a group of 3 or more, or when "Dungeons in my route" is not "Skip them".
- A Hardcore character gets no escort and no quest more than 2 levels above the player.
- A tie goes to the shorter trip, then to the id.

The numbers (1.3, 3 players, 2 levels) are first guesses. The speed test and the in-game test set them.

## The card

`UI/NextStopCard.lua` shows the top place on the guide window's card, with Go and Turn these off.
`Core/NextStopPlaces.lua` builds the places from the Codex, a slice a frame, once for each level.

- Fastest XP must be on, in a level-range guide, and "Suggest the next stop" (`behavior.nextStop`, on by default) must be on.
- The guide's own arrow must be 5 minutes away or more. Nearer, the guide has work here.
- The card shows once for each place and level. Never in combat or while Polestar is hidden.
- Go starts a trip to the place's hub (`Nav.StartTrip`). "Back to the guide" ends it. The card never moves the arrow by itself.
- A zone is a place with all its quests for the level. In the zone you stand in, each hub is a place.
- Work is 4 minutes for each quest: the Codex has no times. This is a first guess.
- The Codex has no escort flag, so the Hardcore escort filter does nothing here. The level filter works.

## Dungeons

A dungeon is a place when the maintainer's dungeon list (`Dungeons.List`) has one in range for the character and
`Dungeons.QuestXP` says quests inside are ready to take or in the log. Its XP is those quests, in levels, so kills are not counted:
the number is a floor. A run is 40 minutes of work, a first guess. Rank offers a dungeon only to a group of 3 or more,
or when Guide, Automation, "Dungeons in my route" is not "Skip them". The card then says "Go do (dungeon) now".
The dungeon files are read, not changed.

## Not built yet

- The check against the shipped guides: for each level, the top 2 places contain the zone of the route.
