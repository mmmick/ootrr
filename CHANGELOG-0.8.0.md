# OOTRR 0.8.0

The biggest update since 0.7. You can now play the randomizer straight from your browser, pick how the Door of Time opens, choose how many of Ganon's Trials you need, make the game harder on purpose, and restyle Link with a new cosmetic editor. Plenty of fixes too.

**Heads up:** seeds made with 0.7.x will not come out the same in 0.8.0. Finish any race or shared seed on the version it was made with.

## New: play from the browser

- OOTRR now runs as a website. Pick your game folder (or a .nup / .nx file) and you get the finished mod as a zip. Everything happens on your own computer: nothing is uploaded, and the site holds no game files.
- The browser version builds exactly the same mod as the desktop app.

## New settings

- **Door of Time**: choose what opens the door.
  - **Vanilla**: clear Dodongo's Cavern and Jabu-Jabu's Belly, like the original.
  - **Song Only**: play the Song of Time at the altar.
  - **Open**: the door starts open. The Ocarina of Time then waits in the moat.
  - You can also decide whether Zelda's escape scene has to happen first.
- **Ganon's Trials**: **Skip** starts with every trial cleared. **Set Number** lets you pick how many you need, and *any* trials count, not specific ones.
- **Skip Epona's Race** is back and **on by default**. Epona's Song alone is enough to get Epona as adult, and the logic knows it. Horseback archery then only needs the Gerudo Membership Card, Epona's Song and the Bow.
- **Fortress Carpenters**: new **Fast** (three carpenters start free) and **Open** (start with the Gerudo Membership Card and the bridge built; the card's own check is still shuffled) options.
- **Great Fairy Prices**: the Great Fairies can ask a cheap or fixed price instead of 25 to 500 rupees. The price shows correctly in their text.
- **Kokiri Sword shuffle**: the Kokiri Sword can be shuffled into the item pool.
- **Potion Shop shuffle**: the Potion Shop's goods can be randomized.
- **Ocarina shuffle** now includes the Ocarina of Time.
- **Difficulty page** (under Gameplay): optional ways to make the game harder. All off by default. Highlights:
  - Tougher enemies: more health, more damage, faster, health regen, calling for backup, respawning sooner, and up to 10x more enemies.
  - Fewer helpers: no heart drops, no ammo drops, no gossip fairies, no bottles during combat, no Z-targeting.
  - Lose a heart container on each game over (never below three).
  - Bosses can bring minibosses or minions, enemies can block the way, and cuccos take revenge right away.
  - Just for fun: spawn enemies with L + R + D-pad.

## New: cosmetic editor

- Link's look is split into parts, each with its own colour picker.
- Hair colour can now be changed too.

## Always on now (quality of life)

- **Short item text**: getting an item just says "You got X!" in one box, without the long description. Gold Skulltula tokens still show your count, and small keys still show how many you have.
- **Shorter story cutscenes**.
- **Bombchu Bowling and the Bombchu Shop** are open from the start, without beating Dodongo's Cavern first.
- **Water Temple gate**: the gate switch in Lake Hylia can be hit with the Hookshot without doing the Ice Cavern first.
- **Fire Temple**: the jailed Gorons are gone. Their cells still open with the switches, and the chests inside open without talking to a Goron first.

## Fixes

- Progressive Slingshot and Bow now also give you the Seed Bag and Quiver.
- Bomb drops no longer hand out a Bomb Bag.
- Child Link can learn Epona's Song from Malon no matter how far the Epona quest is.
- The horseback archery Gerudo now talks to you when you have the Gerudo Membership Card.
- Link is no longer turned back toward the river after Zelda's escape.
- Fixed the logic for the Dodongo's Cavern heart piece and a rupee in the Lakeside Laboratory pool.
- Adjusted logic for several child-only areas.
- The update banner in the app is now shorter and points to the right download page.
