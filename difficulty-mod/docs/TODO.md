# Difficulty settings: what does not work yet

Newest state first. "Seen" means seen in the running game. Items move to the changelog when they work.

## Broken or unsafe

Nothing known at the moment.

## To confirm by hand

| What | State | What to do |
|---|---|---|
| More enemies slider, rc 15 rules | Played at 10x in rc 14; the rules that came out of it (peaceful places, perched Keese, big enemies apart, Iron Knuckle cap, nothing in lava, no added Dodongos in Dodongo's Cavern) are built, not all seen | Play child Hyrule Field, Kokiri Forest, Kakariko and Dodongo's Cavern at 10x; then 2x and 5x; a room whose door waits for its enemies; die in a dungeon and check the retry is safe |
| Enemies call for backup, lesser kind | Built (rc 15), not played | Fight a Stalfos (a Stalchild should come), a Wolfos (two Keese), a Lizalfos (a Keese). Eight callers called nobody in an open-field script (Stalchild, Freezard, Skulltula, Wallmaster, Armos, Spike, Tailpasaran, Biri): check some in their homes |
| Does a kill free one of the 12 spawn slots? | Unknown | Spawn 12, kill some, spawn again |
| Boss adds in the other rooms | Seen with Gohma and Phantom Ganon (by hand) and Twinrova (by script) | Barinade, Volvagia, Morpha, Bongo Bongo: walk in, the adds come with the fight |
| Shadow Temple with "Bosses spawn minions" | Black screen before the rc 16 fix; the fix is not loaded in the game yet | Load the Shadow Temple with the setting on, then fight Bongo Bongo |
| Fireball storms, other stretches | Middle road and castle garden seen | Hyrule Field south-central and west road, child and adult |
| Gates with dungeon entrance shuffle | Gates 9 present and 2 absent (script) | Gates 1 and 6 absent |
| Jabu-Jabu water spawns | Positions fine (script) | Look at the scrub room pool at 10x: no Octorok under a ceiling |
| Zora's Domain ambush | Map loads, Skulltulas present | Walk the cave |
| The held-back gate 7 | Never tried | Walk the Lost Woods east passage, kill the guard, see the barrier drop |
| Dead Hand in the Hyrule Field hole | Rises near the entrance; left as is by decision | None |
| Fake pots | Never seen | Break pots until one flies |

## Not built yet (asked for)

- The Gerudo fighter as a placed enemy (she plays a fortress scene when beaten), and Volvagia or other bosses outside their rooms.
- Enemy speed: most enemies ignore the setting (see enemy-speed.md). Making the others follow it is open.
- Limited ammo; Mido as a blocker; lose-heart surviving a quit.

## Gates

Eleven of twelve are on, all played by hand (the Death Mountain Trail one with its Dodongo guard: "perfect"). Gate 7 (Lost Woods, east passage) is held back until someone tries it.

## Works, with a known limit

- **Lose heart on death:** the loss lasts for the play session; quitting and continuing restores the hearts.
- **No ammo drops with shuffled shops:** a seed can end up with no source of arrows. Planned change: "limited ammo", where ammo still drops in boss fights.
- **Dead Hand in the Hyrule Field hole** rises right by the entrance, although he is placed at the back of the room.
- **Pots in the Market guard house** give no surprises.
- **Bosses spawn minibosses:** King Dodongo is left out; adding a miniboss there hung his dungeon.
- **Fireball storms:** child Hyrule Field ran at about 30 to 37 frames a second with 5x more enemies plus the storm.

## Never seen in the game

Enemy speed by hand (measured by script: most enemies ignore it, see enemy-speed.md), enemies respawn sooner, Zora's Domain ambush, the ten Like Likes at Lake Hylia, fake pots.

## Ideas saved for later

- Grass that normally gives bugs joins the surprise pool.
- The Gerudo fighter as a gate guard.
- Mido as a visible blocker.
- A bomb-proof, glowing wall was replaced by the purple barrier; nothing further planned.
