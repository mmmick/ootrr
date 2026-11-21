# Difficulty features: index

One file per feature, each with your request, my interpretation, the investigation, the implementation, how it was tested, how to test it in the game, and notes. Start with [00-making-new-features.md](00-making-new-features.md).

All 19 are in rc 4 (`<builds>\rc-4\`).

| # | Feature | Setting | Played in the game? |
|---|---|---|---|
| 01 | [Enemy health](01-enemy-health.md) | `enemy_health` | Yes, works (rc 1) |
| 02 | [Enemy damage](02-enemy-damage.md) | `enemy_damage` | No |
| 03 | [Enemy speed](03-enemy-speed.md) | `enemy_speed` | No |
| 04 | [No heart drops](04-no-heart-drops.md) | `no_heart_drops` | Failed in 001, fix unplayed |
| 05 | [No ammo drops](05-no-ammo-drops.md) | `no_ammo_drops` | Failed in 001, fix unplayed |
| 06 | [No gossip fairies](06-no-gossip-fairies.md) | `no_gossip_fairies` | No |
| 07 | [Lose heart on death](07-lose-heart-on-death.md) | `lose_heart_on_death` | Failed in 001, fix unplayed |
| 08 | [No bottles during combat](08-no-bottles-in-combat.md) | `no_bottles_in_combat` | Failed in 001, fix unplayed |
| 09 | [Enemies regen health](09-enemy-regen.md) | `enemy_regen` | No |
| 10 | [More enemies](10-more-enemies.md) | `more_enemies` | No |
| 11 | [Bosses spawn minibosses](11-boss-minibosses.md) | `boss_minibosses` | No |
| 12 | [Bosses spawn minions](12-boss-minions.md) | `boss_minions` | No |
| 13 | [No dupe items](13-no-dupe-items.md) | `no_dupe_items` | No |
| 14 | [Enemies respawn sooner](14-enemies-respawn-sooner.md) | `enemies_respawn_sooner` | No |
| 15 | [No Z-targeting](15-no-z-targeting.md) | `no_z_targeting` | No |
| 16 | [Fairies help enemies too](16-fairies-help-enemies.md) | `fairy_helps_enemies` | No (respawn half not possible) |
| 17 | [Enemies call for backup](17-enemies-call-for-backup.md) | `enemy_backup` | No |
| 18 | [Zora's Domain ambush](18-zora-ambush.md) | `zora_ambush` | No |
| 19 | [Bean Seller's sales pitch](19-bean-sellers-pitch.md) | `bean_pitch` | Retired (setting kept off) |
| 20 | [Enemies block the way](20-enemies-block-the-way.md) | `enemy_gates` | Yes, by hand |
| 22 | [Special encounters](22-special-encounters.md) | part of `more_enemies` | Partly |
| 23 | [Grass and pot surprises](23-grass-and-pot-surprises.md) | part of `more_enemies` | Yes, by hand |
| 24 | [Cuccos take revenge at once](24-cucco-revenge.md) | `cucco_revenge` | Yes |
| 25 | [Spawn enemies](25-spawn-enemies-combo.md) | `enemy_spawn_combo` | Yes, by hand |
| 27 | [Unbreakable Bombchu Bowling grass](27-unbreakable-bowling-grass.md) (idea by Sabriel) | `unbreakable_bowling_grass` | Yes, by hand |
| 29 | [Fireball storms](29-fireball-storms.md) | `fireball_storms` | Yes, by hand (two of four spots) |

Features 01 to 08 I built directly. Features 09 to 19 were built by sub-agents; what those files say about investigation and testing comes from the agents' reports, which I did not re-run except where noted.
