## Interface

- Fix version badge to match with version stream: Stable for stable stream, opt-in beta for beta stream and dev for local
## Calculations

- "+X to Melee, Ranged, Area of Effect and Orbit Skills" on items now raise the ranks of skills with that tag; before, they did nothing.
- Mana Costs Decreased on items and the Manaleak–Manahunger affixes now reduce mana costs instead of doing nothing or adding flat Mana.
- Gurag's Soulgem gives Life Replenish; its stat was misnamed and did nothing.
- Large negative damage stats (Belt of Skulls' -666% Magic Skill Damage, Lucifer's Crown's damage per resistance point with -300% all resistances) bring a hit down to zero instead of making damage and DPS negative.
- Satanic crystals on satanic, heroic, angelic and unholy charms now count; before, only crystals on equipment did, so a Flat Life crystal on a charm left Life unchanged. Potions no longer offer a crystal slot, as they cannot take one (#214).
- Powerfunnel and Powerfunneled Lifespan convert 1% and 7.5% of the Attack Damage stat to Increased Life; they used to read the weapon's damage after Enhanced Damage and came out about seven times too strong (#215).
- Citadel's Increased Life Replenish per Colossus stack adds to the same pool as other Increased Life Replenish (e.g. Angel's Vibrance Aura) instead of multiplying it again (#216).
