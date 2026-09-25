## Feature

- Removed the unused Tauri integration and dependency from the GPUI engine, along with an unused direct engine dependency in the native app. Pruned the Cargo lockfile to the active workspace dependency graph.

- Compared all Incarnation Tree supplementary notes with HS Helper and added The Ripper’s missing dagger requirement. Preserved verified Temporal Echo and Manafury explanations; documented remaining stat-line differences in `docs/incarnation-helper-comparison.md`.

- Separated Fireball and Storm Bolt's rank/flat, generic and own-subtree damage stages, including the generic helper's rounding and total spell damage pool. Fireball's independent damage rolls now affect its average separately from an ordinary hit. Fixed Orbital Fire's 4/8/12 primary counts and fractional extra Storm Bolt casts; secondary and rebound damage no longer inflate primary hits, and calculation details identify secondary DPS that still needs a contact model.
- Forwarded scoped skill stats through both direct damage command APIs, preserving skill-specific bonuses and hybrid calculation inputs.

- Corrected Fireball and Storm Bolt's neutral damage coefficients from their runtime cast paths. Winter's Bite now works across all classes and uses its explicit skill level without adding rank bonuses again.
- Separated Circle of Slugs' projectile and wave counts, applied its 5.25-second cooldown, and made the single-target estimate of one contact per wave explicit in calculation details.
- Corrected normal skill cooldown recovery to 0.5% per effective Skill Haste point, including Blender, while retaining Cloud of Sand's no-haste exception. Cost/sustain calculations now respect weapon and cooldown cadence. Bone Altar adds 8 seconds to the inherited 0.25-second cooldown, and Blood Demons uses player recasts instead of sentry attacks.

- Corrected Arrow Rampage's Exploding Arrowhead area and Marked For Destruction duration coefficients, plus Wounding Paw's Slashing Momentum attack speed. Expanded subskill coefficient evidence to 180 source-address checks.

- Corrected Blender's weapon coefficient to 175% + 15% per effective rank, removed unsupported flat physical additions and separated its synergy/tag bonus from the base weapon and subtree multipliers. Nanoblender tags now take precedence over Blenderang. Isolated triggered spell calculations from the main skill's subtree while preserving their own modifiers and projectile counts.
- Generic projectile damage from items now affects Projectile-tagged skills, including Blenderang, while respecting Nanoblender tag precedence.

- Added expected double damage from Fireball's Critical Burn without inflating hit count; preserved minimum/maximum chance ranges. Corrected Fire Nova's Traveling Flames area bonus and added binary-backed coefficient checks for all Blender and Fireball subnodes.

- Added reproducible all-class skill coverage and static game-metadata audit tools. Fixed entity/cooldown cadence, targeted proc trigger rates, signed physical/elemental subtree damage, per-projectile noncritical DPS and per-contact ailment inputs. Corrected 13 additional cooldown/duration fields from the supplied executable and repaired the Blood Demons proc target. The full game-mechanics audit remains in progress.

- Fixed shared attack calculations to include physical/tagged skill damage, tag-based rank bonuses and attack-rating bonuses across classes. Corrected Blender cooldown to the game metadata value of 8 seconds.

- Fixed Unholy Form: 18.5% + 1.5% × effective rank damage (20% at rank 1, verified against game rank indexing), functional global damage bonuses including item-granted procs, and percentage-based life replenish/life steal penalties.

- Added an explicit average-contact model for Butcher Blender and all 14 subskills: separate Nanoblender groups/blades, Blenderang travel estimates, physical and conditional bonuses, and non-recursive Microblade proc DPS. Calculation details expose positioning assumptions and movement/life-drain effects.

- Replaced whole-build profiles with independent incarnation tree, ether tree, gear and skills loadouts. Each category supports switching, duplication, renaming, deletion and undo; saves and share codes retain every loadout. Existing native profiles are converted with their original snapshots retained for recovery.
- Integrated compact loadout selectors into each view’s existing header or tree toolbar, with secondary actions grouped in a menu.
- Made the loadout actions button more visible and gave its three-dot icon an explicit, unclipped frame.
- Added an immediately applied, saved interface language setting for English, Korean, Russian and Simplified Chinese, with translations maintained in separate JSON catalogs.
- Added Page Up / Page Down scrolling to planner panels (including the virtualized Stats view), the build library and notes preview.
