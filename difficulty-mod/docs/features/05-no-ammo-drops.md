# No ammo drops

Setting `no_ammo_drops` (number 115), checkbox. Code: `crates/core/src/difficulty/drops.rs` and `difficulty/lottery.rs`.

## Request
"no ammo drops (checkbox)"

## Interpretation
Random drops never give arrows, Deku Seeds, bombs, Bombchus, Deku Nuts or Deku Sticks. Magic jars and rupees stay. Ammo from shops, chests and shuffled items stays.

## Investigation
Same files as no heart drops. The ammo types, identified by hashing guessed names and by which tables use them:

| Type hash | What |
|---|---|
| 3237216321 | Bomb |
| 2211636230 | Deku Nut |
| 4192551701 | Deku Stick |
| 1113099761 | ranged ammo for your current weapon |
| 798871343 | arrows |
| 2565786591 | Bombchu |

One more type, 2121460961, means "whatever you are lowest on" and carries a list of the above.

## Implementation
- Ammo entries in every drop table get a chance of 0.
- "Whatever you are lowest on" entries have the ammo types removed from their list, leaving magic.
- In the global lottery file the low-ammo boosts are set to 0 and the eight ammo actor names are pointed at the green rupee.

## How I tested it
- Unit tests for each case, including the "lowest on" list.
- The game-file test for the global file.
- rc 1 in the game, by you: **did not appear to work.** Same cause as hearts.

## In the game
rc 2's fix has not been played. To test:
- Turn this on alone. Empty your quiver or seed bag and bomb bag, then kill enemies and break pots.
- Pass: no ammo of any kind; magic jars still appear.
- Check a Deku Baba (drops sticks and nuts in vanilla) and a pot room before a boss.

## Notes
- The seed logic does not change. With this on you can run out of ammo with no way to refill except shops, so a seed may become very hard. I did not add logic for that.
- Deku Babas dropping sticks and nuts use named actors in their own tables; those are covered by the name rule (`Obj_DekuStick`, `Obj_DekuNut`).

## Seen in the game (18 Nov, batch 1, aborted)
Same mod as 04. Ammo read from the give list after bombs were used: bombs 20 to 9, sticks 10, nuts 20, seeds 30 unchanged during the walk. The number of things cut was not counted, so NOT TESTED.
