## Feature

- Incarnation tree nodes can be excluded from Suggest Nodes: Alt-click a node to mark it with a red ring, and the optimizer will neither allocate it nor path through it. Alt-click again to allow it. Exclusions are saved with the build (#164).

- Equipping a relic whose proc grants a buff (e.g. Skull Axe, Shiv) turns that buff on in Config. Editing the same relic keeps your own setting (#157).

- Buffs cast by relic procs appear under Item Blessings in Config and apply when toggled. Shiv's Live by the Sword adds 28% + 1% per level Attack Speed and Movement Speed, and Skull Axe's Demon Form can now be toggled too (#158).

- Boots can roll the game's "% of Movement Speed Converted to Total Damage Dealt" prefix (Walkers to Dominating, 1-4% up to 9-16%), so items such as Grimwalkers can be rebuilt. The stat is shown on the item but not yet included in damage (#174).

- Signet of Shadows rolls its three Unholy stats from its own pool, as in the game: attribute conversions to Magic Skill or Attack Damage, Total Faster Cast Rate, Total Attack Speed, sentry and guardian stats and more, instead of the generic Unholy pool (#195).

- Crematorium's Pledge gives -30% to Enemy Fire Resistance, as in the game, instead of Ignore Lightning Resistance (#194).

- Maelstrom of Frost lasts its 30 s duration and ticks every second (Intense Temperature shortens the tick), as in the game, instead of a single 0.5 s tick per cast.

- Black Hole lasts its 4.5 s duration (Mass Density extends it) and ticks every 0.4 s, as in the game, instead of 1 s at 0.5 s ticks. In the game Time != Space shortens that tick (to at least 0.1 s).

- Sand Vortex ticks every 0.33 s for 4 s as in the game (Lasting Whirlpool extends the lifetime), instead of every 0.25 s for 5 s. The skill script replaces the vortex's default 0.25 s tick with 0.33 s; Accelerated Sand shortens the tick and cuts the lifetime to a third.

- Soul Spurn now hits each enemy once per cast. Its area has no damage tick: the 0.1 s timer it was modelled with is the Purgatory life-drain timer, and ticks every 1.25 s exist only while the Purgatory node keeps the area alive.

- Subtree cooldown reductions (e.g. Timeless Tundra, Ephemeral Flurry, Blenderang) now reduce the cooldown itself as the game's GetTalentCooldown does (base × (1 − reduction%)), instead of acting as skill haste. Negative subtree cooldown changes are kept, and a reduced cooldown under 0.5 s from a longer base becomes 0.51 s.

- Shuriken Throw's mana cost is multiplied by 1.5 while Twin Shurikens has a rank, before mana cost reduction, as in the game's LoadManaCost.

- Enemy-side damage follows CalculateEndDamage: subtree "enemy damage taken" (enemy buff 13, e.g. Weakening Blast) joins the elemental break sum instead of multiplying separately and no longer affects physical hits; the low-life bonus is a separate multiplier for physical damage only; resistance penetration is no longer clamped at 100%.

- Skill critical strikes now use the pool of the hit's damage type, as the game does: physical skills crit on attack critical chance/damage and elemental skills on spell critical chance/damage, regardless of the Spell tag. Attacks keep attack crit on their converted elemental share.

- Spell projectile counts now include Extra Spell Projectiles % and Additional Spell Projectile for the 44 skills that use the game's ReturnExtraSpellProjectiles (e.g. Frozen Boulder, Orb of Frost, Charged Bolts): floor(count × (1 + %/100)), then the flat bonus.

- Strength now grants 1% Attack Damage per point (the game's StatAttackDamage), instead of Enhanced Damage. Unarmed Wargod still removes it.

- Defensive caps follow the game's Stat scripts: maximum resistance stops at 90% even with +max resistance, physical and magic damage reduction cap at 75% (raised by +max reduction), and weapon-panel critical chance caps at 95%.

- Attack and cast speed now follow the game's formulas. Increased attack speed adds to the weapon rate on at most 0.75 attacks per second (diminishing above 400%, power 0.85), and faster cast rate adds FCR/100 casts per second to each skill's own cast speed (mostly 1.75, diminishing above 350%, power 0.75). Attack-path subtree speed bonuses (e.g. Sun Gods Chosen, Marksmanship) multiply that skill's final attack rate instead of adding to attack speed.

- Attribute conversions now follow the game's Stat scripts: 6.5 life per Vitality, 4.5 mana per Energy and 0.1% critical damage per Dexterity. Life and mana replenish gain 0.125 per Vitality and 0.175 per Energy without per-attribute flooring. Life, mana and other floored stat totals are now floored even without percentage bonuses, as the game does.

- Fixed the main window opening partly off-screen at 125% display scaling on Windows, which hid the title bar and close button. The app now starts maximized and restores to a size that fits the monitor's work area.

- Hellsent, Killing Spree and Rage Trance now require their explicit active temporary buffs instead of granting permanent subtree stats. Hellsent enables phasing conditions; Rage Trance adds separately to physical and magic reduction. Configuration, summaries, saves, share codes and skill loadouts retain each buff's own state.

- The Real Monster now keeps its life and mana recovery on Chain Rip instead of increasing passive regeneration. Calculation details show nominal life recovery and the gradual mana-recovery portion, with blocking, resource caps and unresolved contact totals stated separately.

- Prey on the Weak now requires its explicit active five-second buff and benefits all skills against stunned targets. Configuration and the character summary include the buff, its saved/loadout state is preserved, and the unsupported permanent critical-damage bonus was removed.

- Corrected With 2 Hands to retain a separate attack-time repeat chance instead of increasing the primary hit. For the Weak now reports its fractional Strength-dependent melee size offset separately from generic range and damage.

- Corrected Bucthering Skill and Sharp Edges to multiply only their own skill's intrinsic attack-rating bonus. Bucthering Skill's critical chance now enters its local attack pool, preserving global stats and other attacks.

- Corrected Concussive Blow and Hookstorm to retain their damage and size/range coefficients for separate effects. Their bonuses no longer increase the primary Hook or Chain Swing hit; calculation details show the verified payload formulas and the remaining contact-model limits.

- Added Fuel to Fire's consumed-at-cast count beside its buff toggle. Each consumed object scales only that buff's damage bonus by 1%; the saved profile and complete share code retain the count.

- Corrected Enraged Mania's size recipient: its additive melee attack size offset now stays separate from generic area and floors after aggregation. Attack speed and existing area-based conversions retain their own behavior.

- Awakening Fury now contributes only while its learned four-second buff is active, with its attack-hit trigger shown in configuration. Hunger for Blood now applies only against bleeding targets; its global bonus is floored after aggregation, while three verified Butcher subtree bonuses retain their local fractional values.

- Corrected Holy Form recovery to scale existing regeneration and life per hit in the MORE pool, with its separate flat regeneration bonus. Insatiable Hunger now uses the executable's actual form values and overrides ordinary forms only when its stored damage value is positive; form-dependent passives share that condition.

- Corrected five Butcher subtree critical bonuses to add only to their own attack after global critical scaling. One Swift Strike, Skullsplitter and Obsidian Blades no longer pass through the global critical multiplier; Immaculate Butcher and Entry Frenzy now reach their physical hits.

- Fixed Butcher's conditional primary damage: Blade Uppercut changes Furious Strike's weapon bonus on its successful attack roll, while Meet Daisy applies its damage and local critical bonus together on every third Brutalizing Slash attack. Daisy's life-dependent size is shown separately in calculation details.

- Ending Fate's regeneration now requires its learned, active six-second buff. The configuration view includes its existing-buff toggle and kill chance; the unsupported permanent attack-damage bonus was removed. Its explosion contacts remain under audit.

- Corrected the seven other Butcher primary attacks' weapon coefficients and removed unsupported flat damage. Twenty verified subtree bonuses now enter their own additive pool at the appropriate weapon stage; Precision Equals Power scales Brutalizing Slash's weapon bonus from Dexterity instead of adding flat damage. Conditional and secondary effects remain under audit.

- Fixed Sacrilegious Scorn and Spiritual Duality granting both benefits without a form. Their Holy and Unholy benefits now require the matching active form; active Insatiable Hunger enables both without duplicating either bonus.

- Fixed increased mana cost and mana-cost reduction combining as separate multipliers. They now share the verified signed pool, preserve fractional reduction, floor increased cost and apply the game's subtraction order before final rounding.

- Added a reusable Ghidra headless helper for static Hero Siege decompilation by function name or address, with executable fingerprint checks and local analysis projects.

- Corrected 47 active skills' raw mana coefficients and full-rank indexing from the executable. Costs retain fractional values through modifiers and round once at the end; skill previews show the rounded neutral cost.

- Fixed physical and hybrid triggered skills losing weapon damage. Proc targets now use their own attack calculation and explicit cast rank; Warbringer correctly triggers rank-8 Trigger Finger on qualifying attacks without triggering itself.

- Corrected Blade Strike's primary hit to 100% weapon damage without invented flat damage. Its intrinsic critical damage scales with rank, while Blade's Edge and Giant Slayer multiply only that local bonus; neither changes global critical damage or ordinary hit damage. Restored the intrinsic critical bonuses in the skill preview.

- Fixed Blade Master applying a shared Attack Rating bonus. Its 10% per rank now multiplies only Blade Strike's intrinsic attack-rating bonus, preserving other skills and effective-rank ranges.

- Corrected three Defense affix families that were all contributing flat Defense: Ignore Target Defense no longer scales with stars, Increased Defense uses its 3% per-star rule, and Defense vs Missiles stays separate. Corrected the Increased Defense crystal's stat target as well.

- Fixed selected implicit roll pins in the item editor: resolved class pins now match calculation and tooltip values, raw pins retain precedence, and resetting a roll clears both forms.

- Fixed Ferocious Swipe's critical chance entering global character stats. Its verified 5 points per rank now apply only to Blade Strike alongside that skill's intrinsic chance, preserving ordinary hits and effective-rank ranges.

- Fixed Blade Strike's intrinsic critical chance leaking into other skills. Its 50-point bonus now applies only to its own attack, after global stat scaling and before the game's 95% cap; preserved effective-rank ranges and direct damage API parity.

- Added executable-backed activation metadata for all 432 class skills and separated global on-cast/on-attack proc eligibility from damage type and tags. Enabled Dragon Knight's Helmet's previously ignored rank-125 Breath of Fire proc for qualifying attacks; preserved owner-scoped subskill triggers and player recast cadence for summoned entities.

- Removed the unused Tauri integration and dependency from the GPUI engine, along with an unused direct engine dependency in the native app. Pruned the Cargo lockfile to the active workspace dependency graph.

- Compared all Incarnation Tree supplementary notes with HS Helper and added The Ripper’s missing dagger requirement. Preserved verified Temporal Echo and Manafury explanations; documented remaining stat-line differences in `docs/incarnation-helper-comparison.md`.

- Fixed shield minimum/maximum damage nodes1504–1513: flat bonuses no longer become Attack Damage percentages, minimum and maximum stay separate, and shield-gated flat/percentage bonuses reach spells and weapon damage in their own damage type. Added localized stats and conversion regressions.
- Temporal Echo now contributes to the shared ailment replacement estimate: the latest application replaces the prior one rather than adding a second simultaneous DoT. Uses Echo's own rank and cadence without repeated multicast/projectile damage; calculation details disclose the contact-rate approximation.

- Clarified Manafury (1327) per the confirmed game behavior: only Mana Replenish is disabled; other mana recovery remains available. Corrected the node note and added a regression preserving leech, on-kill/on-hit recovery and mana recoup. Recorded the user-deferred Incarnation mechanics separately from verified work.

- Restored the game's Leap tag on ten skills, including Storm Dash, Fast Slices, Vault and Link of Sand, so Incarnation Leap bonuses can reach their intended damage. Fixed flat Leap damage incorrectly increasing weapon physical damage; added real-node regressions, translated tags and executable evidence.

- Fixed Elemental Break stacking: universal and matching elemental Break now add before multiplying damage, including converted weapon damage and Critical Break averages. Hybrid spells such as Blade Barrier correctly use the on-hit bonus when their damage includes a physical component; added executable evidence and real-node regressions.

- Fixed all five Incarnation Critical Break nodes (1768/1782/1796/1810/1824): their previously ignored chance and damage now affect the matching active Break contribution in average damage, including physical-to-element conversion. Preserved ordinary hit values and independent critical/projectile multipliers; added localized stats and executable evidence.

- Fixed Death Over Time (813/825) against fully DoT-immune targets using the supplied executable: one node restores 50% ailment damage, two restore 100%. Added a saved, localized enemy condition for 100% DoT immunity; ordinary hit damage and the separate non-immune bonus are preserved.

- Corrected Temporal Echo’s note to every sixth qualifying spell, following the supplied executable and user confirmation. Added direct Echo DPS for ordinary spells using its own effective rank and repeated-cast synergies, with no extra mana or recursive multicast. Entity/weapon casts and secondary effects remain explicitly under audit.

- Fixed Wargod (737): the planner now enforces 75 skill weapon requirements and Wargod removes those restrictions while retaining its total damage penalty. Invalid casts produce no DPS, mana expenditure or proc trigger contacts. Includes Wand classification, Zeal/Berserk subskill exceptions and Shield Slam's extra melee requirement, verified against the supplied game executable. Fixed the metadata extractor's missing int64 assignment helper.

- Restricted Hercules Grip's two-handed dual wielding to the Sword, Mace and Axe families named in its note; Claw and Polearm combinations were incorrectly accepted. Added coverage for wand permission, both dual-wield speed penalties and Black Hole allocation shortcuts.

- Added configurable Ramping Pulse stacks and their verified radius bonus (5% per stack, ten stacks from node 1160). The bonus is reported separately from global radius used by damage conversions; per-skill size and radius scaling remains under audit.

- Fixed Agitation (608) to grant movement speed per configured stack, up to the game's verified ten-stack limit; it previously had no effect on movement speed.

- Matched Colossus and Combat Mitigation stacks to the supplied game executable: each active mechanic has one base stack of capacity, Colossus grants total damage per stack, and Combat Mitigation now reaches EHP with a configurable stack count. Citadel recovery uses the corrected Colossus limit.

- Fixed Incarnation aura effectiveness: damage bonuses from Holy Aura (including Pearlescent Dream), class auras and externally granted auras use Damaging Aura Effectiveness; other aura bonuses use Buffing Aura Effectiveness. Mixed auras scale each output separately. Power of Weakness now increases generic damage, including spells, rather than weapon Enhanced Damage.

- Fixed Raging Titan to increase generic damage per Rage stack and the omitted half-value DoT bonus from immunity-shatter nodes against non-immune targets.

- Fixed Incarnation physical-to-element conversions (1080–1109): they add elemental damage from the physical skill hit, apply elemental bonuses and target resistance, preserve physical damage and stack beyond 100%. Converted components now reach attack DPS, calculation details and the elemental hit display; ordinary spells remain unaffected.

- Fixed another set of Incarnation Tree calculations: resistance conversions and the 50% resistance cap, full/low-life and phasing conditions, regeneration restrictions and multipliers, combat stack counts, unarmed/dual-axe/shield/staff bonuses, flat ranged physical damage from Energy, Charge attribute scaling, global Defense bonuses, weapon-scoped Melee/Ranged Enhanced Damage, mana-limited Mind Over Matter EHP, Wizard’s Wrath overheat states, AoE radius conversion and melee projectile damage, increased mana costs, bleeding execution and Mechanical Engineering's self-damage restriction. Added regression coverage, localized controls/stat labels and an audit checkpoint documenting unresolved mechanics; the full-tree audit remains in progress.
- Fixed incarnation tree conversions that add life or mana after the multiplier pass. Vital Power now increases final Maximum Life, converted mana feeds mana-threshold notes, and Strength To Life grants Maximum Life for each complete 5 Strength per allocated node. Powerfunneled Lifespan now converts physical attack damage after the weapon's Enhanced Damage. Asphyxiating Touch now removes hit and proc DPS while retaining hits as the source of ailments and adds 140% skill damage to their base damage.
- Fixed rainbow sockets to floor each amplified stat before aggregation, including transformed socket bonuses and tooltip totals (six Ist runes now grant 132% Magic Find). Fixed starred affix value editing, locked item-skill rank ranges, pinned mercenary aura ranks and duplicate resolved implicit overrides. Matched the game's star-scaling operation order to prevent floating-point loss of whole stat points.
- Corrected star upgrades for Summon Skills, Sentry Skills and maximum Sentries to add flat ranks/counts instead of percentages.
- Fixed random elemental skill affixes to receive the same star upgrades as the selected skill element; the placeholder stat previously skipped scaling in calculations and item tooltips.
- Fixed item text export to apply star upgrades to the selected random elemental skill bonus, matching the tooltip and calculations. Raw and resolved implicit overrides now export once with the same precedence, including zero-value suppression.

- Fixed the gear picker close button so it dismisses an unsaved selection like Cancel.

- Removed the skill metadata evidence tests that depended on missing reference JSON files.

- Corrected Amazon's elemental coefficients, verified weapon bonus formulas, own-subtree damage pools and movement transformations. Added weapon requirements, elemental-only Powerstasis averages, third-attack weighting, Storm Orbs cooldown and an explicit one-contact-per-wave estimate. Secondary payloads no longer inflate primary damage; calculation details identify effects and proc/physical models that still need verification. The full class audit remains in progress.

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
