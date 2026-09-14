# Embedded Inter and JetBrains Mono

`Inter-{Regular,Medium,SemiBold,Bold,Italic}.ttf` come from `@fontsource/inter`
5.3.0: weights 400, 500, 600 and 700 plus the regular italic (used for notes and
empty sockets; GPUI does not synthesize italics). Latin and Latin Extended subsets
are merged, so Polish characters are included. `OFL.txt` is the SIL Open Font License.

`JetBrainsMono-{Regular,Medium,SemiBold}.ttf` come from `@fontsource/jetbrains-mono`:
weights 400, 500 and 600, Latin plus Latin Extended, family name unified under
`JetBrains Mono`. Used for the uppercase tracked labels in the top and bottom bars.
`OFL-JetBrainsMono.txt` is its SIL Open Font License.

GPUI's native font loader needs SFNT files, so the WOFF subsets were merged with
`fonttools` into TTFs with family/style metadata unified under `Inter`, letting
native font selection resolve Medium and SemiBold by weight. The application embeds
these files and never downloads fonts at runtime.
