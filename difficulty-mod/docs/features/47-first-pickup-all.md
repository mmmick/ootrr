# 47. Skip first pickup text: every pickup

Follow-up to upstream's "Skip first pickup text" (setting 136, `skip_first_pickup_text`, pull request #53). No new setting; default off, settings strings and default seeds unchanged. Code: `randomizer/core/src/first_pickups.rs`, `slice/stages.rs`.

## Requested

A player report: the first Deku Nut pickup animation still plays with the option on. The user then asked for the first Deku Nut and Deku Stick to be picked up like any other pickup (no freeze, no "You got" box).

## How it works

The game keeps the "get event played" flag under the name of the item the pickup gives, `HavePlayedGetEvent.<PouchActor>`, not under the world actor's name. For the Deku Nut pickup (`Obj_DekuNut`) that is `Item_DekuNut`. The same holds for bombs, bombchus and the seed bundle (`Item_DekuSeed`). The setting now also sets `Item_DekuSeed`, `Item_Bomb`, `Item_Bombchu`, `Item_DekuNut` and `Item_DekuStick` as new-game start flags.

Nuts and sticks need one more step. The first get is also what hands out the bag: the flag alone gave a 0/0 count. So with the setting on, a new game also starts with the small nut bag and the small stick bag, empty, written the way the starting items write a bag (`IsGet.Item_NutBag_Small`, its stock, the bag's maximum 20 or 10, and `CurrentExpansionActor_DekuNut` or `_DekuStick`). Then the first nut or stick takes the game's normal path. If the starting items already give a nut or stick bag, `bag_sets` leaves that bag alone.

Randomizer items and logic are unaffected: the progressive nut and stick families list only the Medium and Large bags, and the generator's logic counts Deku Nut Capacity from offset 1 (small bag assumed).

Data only: start flags and start-item writes. No flow, actor or message edit, no code patch.

Left out on purpose: gold and silver rupees, heart pieces and containers, bomb, bombchu and stick bundle items, shop and chest items, fairies and Poes. Because the flag belongs to the item, a bomb or bombchu first received from a chest is also silent.

## Tested

- Unit tests with the game files (`cargo test -p ootrr-core first_pickups`): flag list; each world pickup's flag equals the item its pouch file names; the bag writes are skipped for a bag the starting items already give; they are accepted as new-game defaults by the real flag list.
- By hand, 22 Nov: first nut and first stick picked up in a new game, "felt normal" (no box, no freeze, count popup).
- By script (Longshot, fresh save): new game gives both small bags, nut maximum 20, stick maximum 10, both counts 0, `HavePlayedGetEvent.Item_DekuNut` 1. First nut: no event, popup 1/20. First stick: popup 0/10 to 1/10. Capacity items by `give`: Big Nut Bag maximum 30 and Biggest 40; Big Stick Bag 20 and Biggest 30; the small bag flag stays 1.
- Not tested: a nut or stick from an enemy or a shop, the item menu before the first nut or stick, bombs and bombchus as world pickups.

## How to test in the game

1. Build with `skip_first_pickup_text` on, `skip_intro`, `skip_cutscenes` all and `faster_chests`.
2. New game: the item menu already has an empty nut bag and stick bag (0/20, 0/10).
3. Pick up a Deku Nut (a Deku Scrub or a nut plant) and a Deku Stick: no "You got" box, no freeze, the count popup shows.
4. Open a chest: its box still shows.
