# Changes since 0.6.1 (main)

Everything on `feature/Fixes` that is not on `main` (v0.6.1): 99 commits, including upstream PRs #16 to #33. Items marked *untested* have not been played in the game yet.

## New settings and options

- **Dungeon Rewards** (was "Stones and Medallions") is now a choice:
  - **Vanilla**: each boss and Rauru give their usual Stone or Medallion.
  - **End of dungeons**: the 3 Stones and 6 Medallions are shuffled only among the 8 bosses and Rauru.
  - **Anywhere**: the old shuffle.
  - Old settings strings still load (off reads as Vanilla, on reads as Anywhere).
- **Rainbow Bridge** now works with **Vanilla** and **Dungeon rewards** (0 to 9 Stones and Medallions). The bridge can appear before the Light Arrows scene. The default is now Vanilla, which is how the old default already played. *Untested.*
- **Ganon's Boss Key: LACS - Dungeon Rewards**: the Light Arrows Cutscene plays once you hold the set number (1 to 9) of Stones and Medallions, and Zelda gives Ganon's Boss Key. The Light Arrows join the item pool. Needs Story and NPC rewards and dungeon chests shuffled. *Untested.*
- **Shopsanity** uses the N64 choices: Off, 0 items (shuffled goods), 1 to 4 items per shop, Random.
  - **Shopsanity Prices** sets the price range by wallet: Affordable (10), Starting Wallet (10 to 200), Adult Wallet (10 to 500), Giant's Wallet (10 to 995).
  - Special deals can sell unique items, upgrades, songs (when songs go anywhere), and ammo, refills and hearts. The ammo, refills and hearts only go to the Bazaars and the Bombchu Shop.
- **Shuffle Merchants** (Medigoron, Bean Merchant), **Scrubs Prices**, **Merchant Prices** and **Scrub Hint Text** (shuffled scrubs name a Mystery Item or the real item, with their price).
- **Harp colour by song** (on by default): a song found as an item shows Sheik's harp in that song's ocarina menu colour.
- **Shuffle Cows** is now one setting that covers all eight cows.
- **Freestanding Items** always shuffles Pieces of Heart. Dungeons and Overworld choices are added (greyed out until tested).
- **Removed: shop deal gear** (Bow, Slingshot, Bomb Bag and Magic on deals). Old strings that carry it load with a warning.

## Settings menu

- Reorganized into Logic / Goal, World, Dungeons and Shuffle, then Gameplay and a new In Dev heading. Rows follow Ship of Harkinian wording.
- Options the build cannot apply yet are greyed out with "Not implemented yet". Settings that never worked are hidden but still load from old strings.
- **New defaults**: boss Heart Containers, More NPC rewards and Return with Navi everywhere are on. The Deku Tree bridge is off.
- The Stone of Agony is listed as "Echo Gem (Stone of Agony)", the remake's name.

## Item pool and placement

- **Boss Heart Containers**: the shuffled item now appears where the heart would, after the boss falls. Only an item with no ground pickup falls back to a chest.
- **Darunia's Joy** is shuffled. All three Strength upgrades are in the pool.
- **Bolero of Fire** is shuffled with the other songs.
- **Bows, Bomb Bags and Slingshots** now have every progressive copy in the pool (before, there was only one of each).
- **Reward events take any item a chest can.** NPC, minigame, Skulltula, cutscene, Great Fairy and dungeon reward events can give rupees, ammo, shields, Heart Containers, filled bottles, tokens and dungeon items. Dungeon items still credit their dungeon.
- **The Ocarina can be anywhere**: on the ground, in pots, at Skulltulas, and at NPC gifts. At an NPC gift it is handed out as a pickup, equipped, and can be played at once. *Untested.*
- Songs stay away from NPC gifts that hold the item up.
- The items tied to Kak 20 Skulltulas, Biggoron and Medigoron (Biggoron Sword, Giant's Knife) can go anywhere.
- **Shop upgrades**: a shelf sells the next tier you don't have yet, decided when you buy, once per shelf. This replaces giving the top tier at once (for example 50 seeds from a Slingshot with no bag). *Untested.*
- **Chest size matches contents**: keys and boss keys found outside their own dungeon now count as major and get big chests.

## Cutscenes and quality of life

- **Standard item get** instead of the long scenes for:
  - the Spiritual Stone and Medallion scenes, and Rauru's check
  - Saria's gift and the Light Arrows check
  - the song lessons (Impa, Malon, Saria, the Royal Family's Tomb, the Ocarina of Time, the windmill)
  - Sheik's Minuet, Bolero, Serenade, Nocturne and Prelude, given from the sage flows
- **Great Fairies**: once paid, the fairy gives her reward straight away (no rising or leaving) and gives it only once.
- More cutscenes skipped, including Nocturne of Shadow and Saria's gift.
- **Epona**: after the first Epona's Song as adult (where riding is allowed), Epona becomes your horse. No ranch visit or race is needed.
- **Gerudo Card**: holding it makes the fortress guards passive and lets the Gerudo treat you as a member (talk, valley gate, Training Ground, horseback archery), even before the carpenter rescue.
- **Map and minimap** show up in areas whose intro scene was skipped.
- **Seed pickups** (freestanding, pots, Skulltula drops) turn in place, glint like the Heart Piece and stand off the ground. *Untested.*
- **Title screen**: a "Randomizer" line under the logo. Confirmed in the game, and the mod is about 107 MB instead of 190 MB.

## Bug fixes

- **Crashes**:
  - The Ocarina given by an NPC (Anju)
  - Sheik's Bolero trigger
  - Loading the Gerudo Fortress
- **Duplicated items**: pots, crates, beehives, rocks and grass give their item once and stay used. An item left there and not picked up is lost.
- **Great Fairies**: a fountain no longer refuses to talk, and no longer gives its shuffled item on every visit.
- **Fire Temple**: a softlock on clearing it is fixed.
- **Darunia** dances once Dodongo's Cavern is opened or beaten. Late story events no longer move the Fire quest backwards.
- **Epona's Song** no longer makes a new Epona each time you take out the Ocarina.
- **Maps**: the map no longer stays locked. Navi warps from boss and miniboss rooms no longer break saving.
- **Kokiri Forest rock ring** is the same at every point in the story for each age. Items no longer hide under rocks that never spawn.
- **Builds**:
  - No longer fail with "places N copies of Bombs (20) but its chests hold M".
  - Scrub prices and hint text no longer fail when menu scrubs are one-time sales.
  - Shop deals no longer fall back to no upgrades when Saria's gift is shuffled.
- **Cosmetics**:
  - Music shuffle no longer silences the game.
  - Sound effect shuffle no longer breaks sounds. Short sounds are shuffled too.
  - The tunic colour randomizer now colours every tunic, with full RGB colours.
- **Small fixes**: Skulltula token behaviour, random items dropped by Skulltulas, the potion shop tip text, Market Poe reward names matching the remake, and items that went into progressive upgrades (including magic).

## Logic

- **Deku Tree**:
  - The Compass Room needs the Slingshot or Boomerang plus a weapon for the big Skulltula.
  - The scrubs need a shield, Slingshot, Boomerang or Bow.
  - The basement webs also open to explosives, and the Boomerang hits the eye switch.
- **Dodongo's Cavern**:
  - The upper Lizalfos room can't be entered from the far bridge.
  - The Scarecrow Skulltula needs Scarecrow's Song.
  - Adult needs Fire Arrows, the Bow or Din's Fire for the torch room.
  - The Hammer opens the Mouth from the far bridge.
- With shops shuffled, the child Bombchu Shop opens at night from the start.

## Tracker

- **Live game memory**: the tracker can read the running Longshot game every 100 ms instead of waiting for a save. Turn it on with "Read game memory".
- **Exact reward flags**: NPC rewards, boss hearts and boss rewards are detected from the exact flag, not "maybe".
- **Pop-ups**: each new check shows its own pop-up naming the item found.
- **Fixes**: Lake Hylia and Lon Lon Ranch rows have their own names, and two Skulltula rows pointed at the wrong spot.
- **`ootrr mem-watch`** prints every flag the running game changes.

## For developers

- **New layout**:
  - `randomizer/` holds the crates `game`, `logic`, `cosmetics`, `core`, `cli` and `mem`.
  - `web/` holds the page and the tracker.
  - The overworld logic is split into one file per area.
- `tools/repath_patch.py` moves old patches onto the new layout.
- **Placement rules, build stages and map passes** are ordered lists (`place_rules.rs`, `stages.rs`, `map_passes.rs`). Settings are read by their own modules.
- **Seed pins** live in `data/pins/*.json`. Re-pin with `OOTRR_UPDATE_PINS=1`.
- **Speed**:
  - The test suite runs about 4 times faster (29 to 7.5 minutes).
  - Builds with pickup packs run about 4 times faster.
  - The flow simulator is faster and uses less memory (2.5 GB to 330 MB).
- **Known test failures** on the branch:
  - 3 tests still expect the old bridge default.
  - The seed pins were not updated after the Deku Tree / Dodongo's Cavern logic change.
  - Several boss heart, flow and seed pack tests fail after the last commit.
