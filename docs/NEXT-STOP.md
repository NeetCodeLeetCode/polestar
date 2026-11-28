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

## Not built yet

- The card that shows the top place, with a Go button (the arrow never moves without a tap).
- The places themselves: quest XP at the player's level, trip time from `Nav/Travel.lua` and the dungeon list.
- The check against the shipped guides: for each level, the top 2 places contain the zone of the route.
