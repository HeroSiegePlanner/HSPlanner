## Unreleased

- Native installer builds now receive the existing bug-report destination from GitHub Actions secrets. CI tests the full workspace with native OCR, and the OCR example declares its required feature so engine-only builds remain supported.

- Restored Aurora's Might's level 12–28 Lunar Aura in Granted Skill Effects and calculations, corrected the aura's per-rank bonuses, and removed an unrelated +14 Energy bonus.

- Linked all 105 relic skills to Granted Skill Effects using [HS Helper abilities](https://hero-siege-helper.vercel.app/abilities), including 29 triggered skills and the missing Snowball, Thief's Glove and Soul Box abilities. Skill descriptions, rank ranges and formulas now appear together; the separate green proc text is hidden on relics. Triggered effects retain their proc conditions in the data without becoming permanent stat bonuses.

- Added native problem reporting with optional screenshots and build attachment, validation and retryable errors. Pinned stat sources now preview equipped items and Incarnation nodes using the current breakdown snapshot.

- Restored Incarnation Jewelry Sockets: right-click an allocated socket or use Inspect to insert gems, runes and jewels, or craft an Uncut Jewel with up to four distinct affix families and editable tiers/rolls. Socket contents appear in tooltips, contribute to stats while allocated, and support save/share and undo/redo.

- Gear, affix, socketable, skill and Stash pickers now render visible rows on demand; the Stash panel and Incarnation suggestion list are virtualized too. Search/sort results and Stats row layouts are retained between redraws. Build-library filtering no longer copies every profile, and folder counts are computed in one pass.

- Unified item resistance penetration as Ignore Fire/Cold/Lightning/Poison/Arcane Resistance. Ignore All Resistance now adds to all five elemental ignore stats, including source breakdowns and rune bonuses; older saves and tooltip imports remain readable.
- Incarnation suggestions now compare full paths and competing allocations, including setup nodes and skill synergies, within the requested point budget. Existing allocations are preserved; search runs in the background and can be cancelled.
- Refreshed all 155 relics from [HS Helper](https://hero-siege-helper.vercel.app/relics), including tier ranges, granted skills and proc descriptions. Added Snowball, Thief’s Glove and Soul Box; removed unrelated bonuses assigned to the wrong relics.
- The in-app changelog and release packaging now use this single `CHANGELOG.md` file, available offline in the installed app.

## Native desktop application

- HSPlanner now runs on pure `Rust`
- Incarnation and Ether trees keep their familiar design, with smooth navigation and responsive node previews.
- Net Change uses compact rows showing the absolute change and percentage and renders it aprox 4x faster
- Stats include source breakdowns and pinnable calculation details.
- Notes use Markdown with a formatted preview.

## Installing this version

Download the installer from GitHub. I can't figure it out how to update from webview to pure rust so you need to update it manually. From this version on, the native app checks GitHub for newer releases and can install them from the footer.
