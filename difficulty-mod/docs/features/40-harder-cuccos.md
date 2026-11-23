# 40. Harder cucco game (Talon, Lon Lon Ranch)

## Requested

"When you toss a cucco, it doesn't stay in-place for a while. Also there's 50% more cuccos and 50% as much time." This is Talon's game at Lon Lon Ranch (child), where cuccos are scattered in the house and you find his 3 super cuccos before time runs out, not Anju's game in Kakariko. More cuccos means more ordinary decoys, not more super cuccos. The time was set to 20 seconds (two thirds) after 15 seconds proved too hard by hand.

## Setting

`harder_talon_cuccos`, number 155, "Harder cucco game at Lon Lon Ranch", off by default. Difficulty page, section Minigames (next to the Bombchu Bowling grass). Default settings string and seeds unchanged.

Tip: "In Talon's game there are more cuccos to dig through, they don't stay put after they land, and you get 20 seconds instead of 30. Anju's cuccos are not changed."

It does not touch Anju's cuccos, so it has no conflict with `anju_one_cucco`. Both can be on.

## How it works

Talon's game is `Minigame_FindOutCucco` (Anju's is `Minigame_CatchCucco`). `difficulty/talon.rs` edits:

1. **Time.** `Event/EventFlow/Minigame_FindOutCucco.bfevfl.zs`, event `Event27`: `ChallengeSystemActor.EventTriggerMiniGameTimerSetting {IsCountDown=true, TimerLimit=1800}` (60 frames per second, 30 s). The setting sets it to 1200 (20 s). A flow without exactly one such event is refused.
2. **Not staying put.** The pack `Pack/Actor/Animal_Cucco_FindOutCucco.pack.zs` has its own wander file, `Game/BSA/ActionSeqContainer/Animal_Cucco_FindOutCucco_Wander`, with `MoveIntervalMin` 100, `MoveIntervalMax` 600 and `NoMoveTime` 60 frames: a landed cucco waits 2 to 10 seconds. These become 5, 40 and 5 frames. A file without the vanilla numbers, or a change of size, is refused.
3. **More cuccos.** `Banc/HyruleField/HyruleField_Child_Static.bcett.byml.zs` holds Talon (`Npc_Hylia_Talon_Child`), 10 `Animal_Cucco_FindOutCucco` and 3 `Animal_SuperCucco`. Five more cuccos are copied from the first with new names (`Animal_Cucco_11` to `_15`), fresh hashes, positions inside the room and no link to Talon. The flow does not name them, so they are plain decoys that are never reset or hidden.
4. **Talon's spoken number.** Talon says "30 seconds" in two lines of `EventFlowMsg/Npc_Hylia_Talon_Child.msbt` (before the game and on a retry). In every language archive, only the whole number token `30` inside plain text becomes `20`. The rupee lines are untouched.

Talon is only at the ranch once the castle quest is past a certain step (his placement has a presence flag), so test with `open_kakariko` open.

All edits are data (a flow number, three floats, five placements, two message lines). Nothing needs a code patch; it works wherever OOTRR works.

## Tested

- `cargo test -p ootrr-core difficulty`, `settings` and the module's own tests pass with the game files: the limit reads 1800 and becomes 1200; the wander file changes only its three numbers and is refused with other numbers; the map gains exactly five cuccos with unique hashes and nothing else changes; a second run is refused; the stage returns the three files and only the wander file changes in the pack; every language archive changes exactly Talon's two lines and keeps the rupee lines.
- By script (test game, setting on, `open_kakariko` open, rupees given): the mod loads and the child field loads; the ranch house is full of cuccos; Talon starts the game; the timer counts down; the cuccos move about; at 00:00 Talon says time is up and Link walks again (no freeze).
- By hand (22 Nov): Talon says 20 seconds before the game and on a retry, the timer is 20 seconds, and the game was won (all three super cuccos lifted in time).

## Not tested

- Languages other than English in the game (the unit test covers every archive).
- The old rest time compared side by side with a vanilla build.
- Other runtimes.

## How to test it in the game

Preset: `harder_talon_cuccos` on, `open_kakariko` open. Go to Lon Lon Ranch as a child with 10 rupees and talk to Talon in the house. Expect his line to say 20 seconds, more cuccos than usual, a 20 second timer, and cuccos that you throw starting to move again almost at once. Lift the three super cuccos for the reward. Retry and read the number again.
