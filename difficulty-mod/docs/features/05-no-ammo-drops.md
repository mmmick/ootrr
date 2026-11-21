# Limited ammo (was "No ammo drops")

Setting `no_ammo_drops` (number 115), checkbox. Code: `crates/core/src/difficulty/drops.rs` and `difficulty/lottery.rs`.

## Request
"no ammo drops (checkbox)"

## Your later request (18 Nov, feature 27)
"Ammo should still drop in boss fights and anywhere else the player cannot flee and would otherwise have no option but to die." Also answers the review finding that no ammo drops plus shuffled shops can leave a seed with no arrow source.

## Interpretation
Random drops never give arrows, Deku Seeds, bombs, Bombchus, Deku Nuts or Deku Sticks, except from the drop tables the game itself made to stock a boss fight (list below). Magic jars and rupees stay. Ammo from shops, chests and shuffled items stays. The setting keeps its id `no_ammo_drops`, its number 115 and its place in the settings string; only the label and note changed to "Limited ammo".

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

## Limited ammo: how the boss exceptions work (feature 27)
Boss arenas are not separate maps with data we can tell apart (the real boss rooms are rooms inside each dungeon's map file; only the Boss Rush copies under `Banc/DreamWorld` are separate). Per-map or per-room edits would mean rewriting dungeon map files. The game already stocks its fights through named drop tables, so the exception is by table name:

| Table | Used by | Why kept |
|---|---|---|
| `DropTable_Enemy_RangedWeapon_02` | Gohma eggs and larvae | 80% Deku Seed (named actor); Gohma's room has no other seed source except two grass tufts that already name seeds in the map |
| `DropTable_ResourceSupply_FinalBattle` | rubble in the final battle | 15% arrow bundle |
| `DropTable_Enemy_RangedWeapon_01` | Poe Meg (a miniboss, sealed room) | 100% ammo for the weapon you hold |
| `Drop_ResourceSupply_BeforeBoss*` | the pots in the room before each boss door, and the pots in Ganondorf's room | these tables have no probability list (they pick the most needed of magic, ammo, bombs), so the old edit never touched them |

Everything else is cut as before: every ammo entry elsewhere gets 0%, "most needed" lists lose their ammo, the global low-ammo boosts are 0. **Changed from the first version:** the global file's ammo actor names (`ArrowActor`, `BombActor`, ...) are no longer pointed at the green rupee, because the boss tables resolve their ammo through them. Nothing else uses those names once every other ammo entry is at 0% with no boost.

Known losses and judgement calls, for you to decide:
- The pots before the boss doors are kept as boss supply, so about 25 pots across the dungeons can still give ammo. If you want them cut except in the boss room itself, that needs per-pot map edits (Ganondorf's room pots are placed with those tables).
- Boss help that only works through the low-ammo boost (the periodic remedy spawner in the Ganondorf and Ganon fights, the Bongo-Bongo hands' table) loses its ammo, because the boost is global and also feeds every other table.
- Bomb flowers in King Dodongo's room and the Flare Dancer's room are placed objects, not drops: untouched.
- Pots and grass that name an ammo actor directly in the map (the two seed tufts in Gohma's room) were never touched by this setting.

## How I tested it
- Unit tests for each case, including the "lowest on" list.
- The game-file test for the global file.
- rc 1 in the game, by you: **did not appear to work.** Same cause as hearts.
- Feature 27 (cargo, branch `feature/limited-ammo`): `boss_tables_keep_their_ammo_and_every_other_table_loses_it` (unit), `the_game_keeps_ammo_only_in_the_boss_tables` (reads every actor pack and the foliage pack from the game, applies the edit and asserts that the only tables with live ammo are the boss tables, including Gohma, Meg and the final battle, and that ordinary tables such as `DropTable_ResourceSupply_00` and `DropTable_Obj_Bomb` have none), and the lottery test now asserts the ammo actor names are unchanged. Not played in the game.

## In the game
rc 2's fix has not been played. To test:
- Turn this on alone. Empty your quiver or seed bag and bomb bag, then kill enemies and break pots.
- Pass: no ammo of any kind; magic jars still appear.
- Check a Deku Baba (drops sticks and nuts in vanilla) and a pot room before a boss.

## Scripted test plan (for a later agent; one game copy, windowed, captioned, see <playtest folder>\RECIPE.md)
Preset: `no_ammo_drops` on, plus `skip_intro`, `skip_cutscenes` "all", `faster_chests`, and the dungeon unlocks you need. Use `ootrr save-unlock` / `save-set` for a save with the slingshot or bow, bombs and full health, standing at the boss door (teleport ids only reach a map start). Never touch the user's saves. Caption: "Limited ammo: expect seeds from Gohma larvae, none from Deku Tree pots", then the result.
1. Control (Kokiri Forest or Hyrule Field): empty the seed bag (use the seeds), then cut 30 grass tufts and break 10 pots and kill a dozen enemies. Read the seed count from the give list or HUD after each batch. PASS: stays 0, only rupees and magic appear. FAIL: any ammo.
2. Gohma (Inside the Deku Tree, room before the boss door and the boss room): with 0 seeds, break the two pots/grass before the door (should be rupees or the supply the game gave) and then in the boss room kill Gohma larvae and break the grass. PASS: Deku Seeds drop from larvae/eggs (about 80% each) and from the two seed tufts. Record how many of 10 larvae gave seeds.
3. Meg (Forest Temple, Poe Meg room): defeat a Poe Meg copy with 0 ammo; PASS if it drops ammo for the held weapon.
4. Before-boss pots (any dungeon, for example Dodongo's Cavern room 19): with 0 ammo break the pots before the boss door. Record what comes out (expected: magic, hearts as set, ammo for the held weapon). This is the judgement call above.
5. Final battle rubble (Ganon's castle collapse, last fight): break rubble with 0 arrows; expected arrow bundle about 15%. Record only; reaching it needs a late save, so mark NOT TESTED if no save exists.
6. Ordinary table with a near-boss enemy: kill Keese and Octoroks with 0 ammo; no ammo.
Read: map name and the item counts from status/HUD; take a screenshot after each batch. Anything needing hands is NOT TESTED and goes to `docs\capture-playbook.md`.

## Notes
- The seed logic does not change. With this on you can run out of ammo with no way to refill except shops, so a seed may become very hard. The boss exceptions and the before-boss pots keep a source in every dungeon that has one; overworld seeds remain shop-or-chest only. I did not add logic for that.
- Deku Babas dropping sticks and nuts use named actors in their own tables; those are covered by the name rule (`Obj_DekuStick`, `Obj_DekuNut`).

## Seen in the game (18 Nov, batch 1, aborted)
Same mod as 04. Ammo read from the give list after bombs were used: bombs 20 to 9, sticks 10, nuts 20, seeds 30 unchanged during the walk. The number of things cut was not counted, so NOT TESTED.
