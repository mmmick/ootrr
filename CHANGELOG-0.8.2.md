# OOTRR 0.8.2

The dungeon logic is rebuilt for most dungeons, and beating a temple (or reaching a later part of its story) before an earlier step no longer locks you out of checks. The Gerudo Fortress now follows the Gerudo Membership Card instead of the Spirit Temple. Also new: Classic triggers for the Nocturne and the Prelude, a new colour editor on the Cosmetics page, a debug option that names each check's location, more Difficulty options, and fixes for saves and softlocks after Zelda's escape and the Nocturne.

## Dungeon logic overhaul

- The logic of the **Forest, Fire, Water and Shadow Temples and the Bottom of the Well** was rebuilt from a 100% walkthrough of the remake. It now follows the remake's own rooms, doors and keys where they differ from the original game, instead of the original game's layout.
  - **Water Temple:** 4 small keys and 4 locked doors, opened in a fixed order. The Dark Link room opens onto the river through bars the Longshot reaches past; the original game's Song of Time block is gone.
  - **Fire Temple:** its locked doors follow the remake's order, from the lobby to the Boss Key loop. The loop door sits behind a Hammer pillar, so the logic keeps one key spare for it.
  - **Forest Temple:** the courtyards, gates, switches and wells follow the remake's layout. The east courtyard gate has no eye switch in the remake.
  - **Shadow Temple:** 5 keys and 5 locked doors in a line, as in the original game. The falling spikes chests now need the Strength upgrade.
  - **Bottom of the Well:** 2 small keys and 2 locked doors, and either door can be opened first. The east rooms past the fake walls are open, with no key spent.
- Pots, Gold Skulltula names and chest locations in these dungeons were updated to match.
- The Ice Cavern, the Gerudo Training Ground and the Spirit Temple keep the previous logic for now.
- The write-up of each dungeon is in `docs/dungeon-logic`.

## Gerudo Fortress

- **The fortress follows the Gerudo Membership Card.**
  - **Without the card:** the fortress is hostile.
  - **With the card in your inventory:** the guards are friendly, the fortress looks cleared, the bridge in Gerudo Valley is repaired and horseback archery is open.
  - Beating the Spirit Temple, or reaching the Desert Colossus or the Spirit Temple first, no longer changes any of this.
- **The carpenters can always be rescued.**
  - **Before:** reaching the Spirit Temple first left the carpenters in their cells talking as if they were already free, and the thieves who guard them never appeared.
  - **Now:** they stay jailed until you free them, each one still calls his thief, and freeing all four still plays the Membership Card scene with its randomized item.
  - This works with or without the card. A card found somewhere else does not empty the cells.
- **Open fortress:** the guard who gives the Membership Card check now offers it when it is still owed. Before, she would only replay the scene after it had already played.

## Checks that stay available

- **Link the Goron (Goron City, adult):**
  - His gift is no longer lost if you go to the Death Mountain Crater first or beat the Fire Temple first. In that case he no longer rolls around; talk to him and he gives you his item.
  - He is now also there when you enter Goron City from the Crater.
- **King Zora's thaw gift (Zora's Domain, adult):** if you beat the Water Temple before melting the red ice, King Zora is free when you return. Talk to him and he gives you the check, whether or not you already own a Zora Tunic.

## Quests no longer go backwards

Some places in the game moved a story quest back to an earlier step when you visited them late, which could undo progress in a temple. They now leave the quest where it is:

- **Spirit Temple:**
  - talking to the carpenter boss in Gerudo Valley
  - talking to the guard at the Haunted Wasteland gate
  - arriving at the Desert Colossus through the Haunted Wasteland
  - getting Requiem of Spirit after you have already made progress in the Spirit Temple
- **Fire Temple:** Link the Goron's first scene, stopping him and receiving his gift, if the Fire Temple quest is already further along.

## New: Nocturne and Prelude triggers

Two new settings in the **Logic** section of the **Logic / Goal** page. Both default to **Vanilla**. They are untested in the game.

- **Nocturne Trigger:** what makes Kakariko burn, where Sheik teaches the Nocturne of Shadow to adult Link.
  - **Vanilla:** the Forest, Fire and Water Temples are cleared, as in the remake.
  - **Classic:** hold the Forest, Fire and Water Medallions, as in the original game.
- **Prelude Trigger:** what makes Sheik teach the Prelude of Light in the Temple of Time.
  - **Vanilla:** the Forest Temple is cleared.
  - **Classic:** hold the Forest Medallion.

## New: Cosmetics colour editor

The **Cosmetics** page has one colour editor in place of Colour mode and the two part sections.

- **Grouped parts:** parts sit in groups that fold, all open at first.
- **Per-part controls:** each part has a colour swatch, a die to randomize it, a reset button and a lock.
- **Group buttons:** **Randomize unlocked**, **Shades of one colour** and **Reset unlocked** leave locked parts alone.
- **Colour picker:** the new picker is drawn in the page's own theme. It has a colour square, a hue bar, a hex box, your recent colours and **Use original**. It works with the keyboard and updates live.
- **Re-roll:** with re-roll on, the build draws new colours for the unlocked parts and keeps the locked ones.
- Settings saved by older versions still load.

## New: Show location names (Debug)

A new **Debug** page at the bottom of the menu, off by default and experimental. It is for checking the location data.

- **Show location names:** after an item get, a text box shows the internal name of the location the item came from, such as `[KF Midos Top Left Chest]`.
- **What it covers:** chests, NPC, song and event rewards, Gold Skulltulas, cows, Medigoron, one-time scrub sales and items on the ground.
- **Gold Skulltulas:** they show their name after the next item get.
- **Items on the ground:** they need a button press to pick up while this is on, so swimming does not grab them silently. A placed item missing from the location list says so.

## Difficulty (experimental)

New options and changes on the **Difficulty** page. Everything there is still off by default.

**New options:**
- **Fireball storms:** burning rocks rain down on a few quiet stretches of Hyrule Field and the child Hyrule Castle garden, as on Death Mountain.
- **Unbreakable Bombchu Bowling grass:** the lane obstacles can't be broken, so you have to steer your Bombchus around them. Idea by Sabriel.

**Changes:**
- **Bosses spawn minibosses** and **Bosses spawn minions:** the extra enemies now arrive when the fight starts (Gohma: after you look up at her) and leave when the boss is beaten.
- **Healing enemies:** when an enemy regenerates or a fairy heals it, it now glows with Farore's Wind's green effect. A call for backup shows a white flash.
- **Backup calls:** harmless scrubs (business, stage and leaf scrubs, the Deku Scrub brothers) never call for backup. The fighting Deku Scrub still does.
- **Enemies block the way:**
  - As a child, the barriers only appear once you have the Kokiri Sword.
  - The barriers no longer vanish when you walk up close.
  - With dungeon entrance shuffle on, they stay on except where a dungeon door is beyond the wall.
- **More enemies:**
  - In Jabu-Jabu's Belly, enemies that spawn in water only appear in open pools near a shore.
  - The Zora's Domain ambush is now part of **More enemies** (once you have a sword) instead of its own setting.
- **Retired:** the Bean Seller's sales pitch joke is gone.
- **Tooltips:** they are shorter and plainer, and say what was tried in the game.
- **Notes:** `difficulty-mod/docs/CHANGELOG.md` lists what was played and what is known not to work.

## Fixes

- **Zelda's escape / Ocarina of Time:** after the escape, the game could refuse to save or warp, and leaving the drawbridge area said "Cannot exit right now". The Door of Time quest now moves past that step. The moat Ocarina is a plain pickup, so its two checks stand on their own.
- **Song of Storms:** getting the Song of Storms at the windmill now also slows the windmill and moves its story on, as the original scene did.
- **Nocturne of Shadow:**
  - After the burning village, Link could be left unable to move or save. The lesson now waits for the loading screen and fades in first.
  - Its item is now given during the burning event, before the burned village loads, so Link no longer freezes before the item window shows.

## Known limits

- **Gerudo Valley bridge:** logic still expects the bridge to be repaired once the carpenters are rescued. In the game it is now repaired once you hold the Membership Card. This only matters for walking back across the valley.
- **Link the Goron and King Zora:** these new ways to get their checks are extra. The logic still asks for the usual items: something to stop Link the Goron, and blue fire for King Zora.
