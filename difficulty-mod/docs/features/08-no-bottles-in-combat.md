# No bottles during combat

Setting `no_bottles_in_combat` (number 117), checkbox. Code: `crates/core/src/difficulty/flows.rs` (`no_bottles`, `refusal`).

## Request
"no bottles during combat (prevents using bottles while an enemy is aggro)"

After playtesting rc 1: "attempting to drink potion at any time doesn't work. also it just freezes me in place. it should play some sort of 'can't do that' animation. but most importantly: it should let me drink the potion if there's no nearby aggro enemy!! ... you can tell if 'in combat' by the music"

## Interpretation
While an enemy is after you, drinking from a bottle or letting out its contents is refused with a visible "no". Otherwise bottles work normally.

## Investigation
- Bottle use runs three flows: `Dm_SY_DrinkBottle`, `Dm_SY_OutBottle`, `Dm_SY_OutBottleFairy`.
- I found no "player is in battle" query. What exists: the game data flag `IsBossBattle`, and a sound query `EventQueryIsPlayingBgmName`. Ordinary enemies start `BGM_EnemyBattle` when they notice you.
- The sound query is not used by any shipped flow, so I read its parameter name (`BgmName`) out of the sound system's graph.
- **After your playtest, two bugs:**
  - True/false queries answer case 0 for true. I had it backwards, so bottles were blocked outside combat.
  - Ending the flow at its entry froze Link. Every vanilla path ends with `EventPlayerDrawnShieldAfterUseBottle`, so the refusal has to run it too.

## Implementation
Each entry of the three flows now starts with two checks: boss battle, then battle music. If either says "fighting", a refusal runs: start the event, play the head-shake animation (`EvtTalkNo`, the one Link uses to answer "no"), put the bottle away. Otherwise the vanilla flow runs.

## How I tested it
- Unit tests for the check order, case values and the three-step refusal.
- The game-file test stages all three real flows; `flow-loads` is clean.
- rc 1 in the game, by you: **inverted and froze.** Both fixed in rc 2.

## In the game
rc 2's fix has not been played. To test:
- With this on alone: drink a potion in a quiet place. Must work as normal.
- Get an enemy to chase you until the battle music starts, then try. Pass: head shake, bottle put away, you can move.
- Try in a boss fight, and try releasing a fairy or a bug in both situations.
- Things to report: any freeze, no animation at all, or bottles blocked for a long time after the fight (the music fading out).

## Notes
- Enemies that start no battle music (Stalfos, some minibosses) will not block bottles. You confirmed music is the right signal, so that gap follows from it.
- If you randomize music, the check still uses the name `BGM_EnemyBattle`; I believe the name is kept, but it is untested.
- The head shake is a guess at a "can't do that" gesture. If it looks wrong, the animation name is one constant, `REFUSE_AS`.

## Seen in the game
Not tested (batch 1 stopped early; no filled bottle was set up).
