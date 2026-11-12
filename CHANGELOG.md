## Season 11 (partial patch notes)

- Skill data follows the first Season 11 patch notes: skill damage, synergies and Intelligence scaling, subskill values and maximum ranks for every class with listed numbers, the swapped subskill positions (icons move with their nodes) and the new Spell/Projectile/Ranged tags. Bug fixes the planner already assumed (nodes that "never worked") need no change; Brutalizing Cleave's 225% per rank, Donuts' 50% per rank, Dune Warrior's corrected damage and Penetrating Trail without the extra size follow the fixes. Unholy Form, Awakening Fury and Insatiable Hunger keep the game's starting values (18.5 + 1, 13 + 5 and 0.75 + 0.15 per rank).
- Incarnation tree: Gigantus, Furious Smash, Rapid Mending, Vital Evasion, Vital Heart, the Life Steal big node, Light In The Dark, Vital Resistance, Vital Suppression, Manahunger, Spacial Conversion (50%), Colossus (0.5% per stack), Citadel (40% per stack) and Missile Explosion (any explosion) use the new values. Ether tree: Back to Basics 30%, Ready to Pop 15%, Reaper's Toll -85%.
- Items, runewords and item skills: the listed nerfs and buffs (Fury of Tarethiel, Eternity, Rakhul's Legion Crusher, Dragon's Blessing, Grief, Commander's Sentry Blaster, Tactical Flak Jacket, St. Aaron's Eternal Rage and the rest). St. Neri's Rainbow Lance converts 20% of Lightning Resistance into Lightning Skill Damage. Celestial Might converts half of each elemental skill damage bonus to arcane. Light Radius and Intelligence Added as Magic Skill Damage no longer scale with star levels, and Spectral Scatter no longer gains star ranks.
- Mechanics: Magic Find diminishes with a power of 0.8 (was 0.9); Physical and Magic Damage Reduction diminish past 40% (power 0.85); DoT frequency diminishes past 50% (power 0.75); Jagged Gauntlet lowers critical chance by 25% instead of halving it; Blender gets a third of every other area of effect bonus (Increased and More) while its own Industrial Sized counts fully; Maelstrom of Frost hits every 0.8 seconds.
- Not in yet: new items, the Meteor, Homing Missile and Benediction subskill trees, new Incarnation and Ether nodes, rearranged subskill trees and changes the notes give without numbers (e.g. Shockwave's area scaling, projectile speeds, St. Soloyolo's Holy Bow), Lightning Fury's spell critical strikes and Blazing Trail's spell duration.

## Interface

- When HSPlanner crashes, a window opens with the crash log (error, file and line, backtrace), saved as crash.log in the data folder: Copy log puts it on the clipboard for a bug report and Restart starts HSPlanner again.
- The item Text edit editor numbers its lines and marks every line with a problem in the gutter and on the scrollbar: amber for warnings, red for errors.
- The Skills panel shows a Preview of one use against the training dummy for 40 skills whose motion was read from the game, such as Fireball, Meteor, Armageddon, Hydra, Storm Bolt, Asteroid, Tsunami and Holy Nova: the game's own sprites play on the Act 1 town floor, the training dummy reacts to the hit, and each hitbox shows while it can hit. The use plays in a loop, framed to fit the skill, and rests while the window is in the background.

## Calculations

- Storm Cloud hits 17 times per cast instead of 16, as in the game: its fade starts 2 frames after its 8 seconds run out, which leaves time for a 17th tick. Skill Duration now stretches it as well: +25% gives 20 hits per cloud, where before only Supercell Storm added hits. Destructive Cloudburst, Thunder's Roar, Storm Front and Apocalyptic Event, which trigger on its tick hits, grow with it.
- Blade Barrier's blades last 6 seconds instead of 5 when Skill Duration or Barrier Extension stretches them, as in the game, so their extra hits grow with the bonus: +50% gives 1.5 times the 35 hits instead of 1.47 times. Its Hit interval shows 0.17 s, the time between two blade passes, instead of the 0.33 s that only Body And Soul's stuck blades use.
- Comet's extra comets fall on the same spot as the first one, as in the game, so each of them hits: +1 projectile gives 2 hits per cast instead of 1.63.
- Volcano hits 8 times per cast instead of 9, as in the game: its last tick comes while it shrinks away from the dummy. Skill Duration now stretches it: +25% gives 10 hits.
- Mine Field reaches the dummy with three mines at rank 1, as in the game: two ring mines land below the cursor within the smaller blast's reach, so rank 1 gives 1.2 hits instead of 0.4.
- Colossus Hammer and Orbital Light widen every Holy Hammer, as in the game: each rank adds about 10 % (Colossus Hammer) or 21 % (Orbital Light) to a hammer's expected hits.
- Cannon Scatter no longer adds hits on a single dummy, as in the game: the cannonball's chain skips the enemy the ball hit, so a longer chain finds no target, and its pop-up bomb lands the second hit instead.
- Temporal Heroes' Marksman fires one lobbed arrow per attack, as in the game, not a fan of six, so a hero's attack averages 1.67 hits instead of 2.05.
- Holy Hammer's hammers are full size, as in the game: the cast sets their scale to 1.0 over the 0.75 they are created with, so each hammer is expected to hit 0.9 times per cast instead of 0.74.
- Sand Gush hits once per cast instead of 32.8, as in the game: its 18 extra waves and their tornadoes come only with Sandstorm or Sand Tornado, which keep the 32.8 hits. Gushing Winds, Desert Cyclones, Desert Storm and Dust Devils add tornado hits only together with them.
- Sand Vortex hits 13 times per cast instead of 12.5, as in the game: it keeps hitting while it fades.
- Lunar Orbit casts one moon instead of three, as in the game, so it hits 2.6 times per cast instead of 6.8. Lunar Architect still makes 3 + 3 moons per rank, and Extrasolar Moons adds moons only together with it.
- Black Hole hits 12 times per cast instead of 11, as in the game: it keeps hitting while it shrinks away.
- Circle of Guardians lands about twice as many strikes: 0.94 per attack instead of 0.47, because a guardian's strike reaches 86 px with an 80 by 48 hitbox, as in the game.
- Skill Duration no longer adds Holy Bolt hits: the base bolt hits once, and only Divine Assbeating gives it a tick, as in the game. Holy Bolt and Divine Storm no longer show a Hit interval, because only their sub-skills give them a tick.
- Skill Duration no longer adds Miasma or Toxic Flask hits: their ticking clouds come only with sub-skills, as in the game.
- Skill Duration no longer adds Shuriken Throw hits: each shuriken hits once as it flies past, as in the game.
- Skill Duration no longer adds Lightning Surge hits: the base ray hits once, and only Concurrent Current makes the ticking tether, as in the game.
- Skill Duration no longer adds Torrent hits: its two explosions hit once each, and only Whirlpool makes the ticking vortex, as in the game.
- Torrent's extra projectiles add 0.5 hits each instead of 2, as in the game: with more than one marker, all of them land on a 70 px ring around the cursor, where only their second, larger explosion reaches the dummy.
- Multinet Throw no longer adds Retiarius Net hits: its extra net lands 64 px past the aim point, out of the dummy's reach, as in the game.
- Skill Duration no longer adds Shadow Bolt or Black Mass hits: their hits land while the bolt or the cultists pass the dummy, long before their life ends, as in the game.
- Shredder Trap gains the right number of ticks from Skill Duration: its 4-second life now stretches to 15 hits at +50% instead of 13.3, as in the game.
- Bone Barrage hits 19 times per cast instead of 8.6, and once more per extra bone: as in the game, the bones fly at -12, -6, then 0 and +6 degrees, and their 38 px tall mask catches the dummy on every line but the first.
- Plague of Rats hits 1.48 times per cast instead of 2, and 0.39 per extra rat instead of 0.6: the rats are smaller than assumed (24 px), so the side rats miss more often, as in the game.
