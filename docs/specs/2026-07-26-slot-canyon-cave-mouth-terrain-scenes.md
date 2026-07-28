# Slot Canyon and Cave Mouth Terrain Scene Spike

## Status

**Completed (2026-07-26).** Both `slot-canyon` and `cave-mouth` were cut after three failed 4% visual reviews. The shipping change retains only `dunes`, `mesa`, `badlands`, and `glacier`. Cut reasons, iteration evidence, and Brigade receipt ids are recorded in `implementation-notes.md`.

## Goal and strict scope

Test two additions to Cloche's procedural terrain family:

- `slot-canyon`: asymmetric full-height sandstone walls with broad curved cross-bedding.
- `cave-mouth`: an asymmetric enclosing rock mass with a few large cave features on the perimeter.

Only scenes that remain legible when the centered screenshot hides most of the backdrop will ship.

**In scope:** two new `TerrainKind` variants in `src/terrain.rs`, profile helpers, structure painters, unit tests, visual QA sheets, and documentation updates only if a scene passes.

**Out of scope:** new dependencies, palettes, CLI flags, MCP surfaces, configuration fields, or backdrop families. `README.md` changes wait for an accepted scene. `implementation-notes.md` changes wait until visual QA accepts or cuts a scene. Pahoehoe lava texture is deferred to a future spike. Natural arches, basalt columns, and coastal sea stacks are cut from this spike.

## Why positive perimeter geometry beats centered subjects

A finished card is only about 4% backdrop by width. `PresentationStyle.padding` randomizes between 58 and 78 pixels at reference scale, and `compose_card` scales that padding with the capture window. The screenshot sits in the center, so the visible strip is the outer perimeter, not the middle of the canvas.

`src/terrain.rs` documents this constraint explicitly: every kind is judged on texture at the edges and corners, never on a centered subject. The existing `every_kind_carries_texture_at_the_edges_and_corners` test enforces variation along all four edge bands and across corners.

The Alpine and Hoodoos spike failed because both designs relied on signals that collapse or vanish behind the window:

- **Alpine (cut):** ridge and snow bands flattened into generic horizontal texture in the padding strip. Three iterations could not separate alpine from glacier or mesa at seeds 1, 7, 42, and 99 across all four palettes at 440×300.
- **Hoodoos (cut):** isolated narrow spires became T-shaped posts or a repeating fence. Caps were too small on finished cards. Three iterations never produced readable hoodoo identity in the outer bands.

Positive perimeter geometry places identifying structure on the side, top, or bottom bands where the padding strip actually shows it. Slot-canyon walls and cave-mouth enclosing rock are positive masses that occupy those bands directly. They do not depend on negative space in the hidden center or on distant horizon silhouettes.

Future visual QA must mask the center and inspect 4%, 6%, and 10% visible perimeter widths. A scene that only reads at the current generous 440×300 full-frame size is not acceptable.

Reference material:

- [NPS geodiversity atlas: Zion National Park slot canyons](https://www.nps.gov/articles/nps-geodiversity-atlas-zion-national-park.htm)
- [NPS Mammoth Cave: stalactites, stalagmites, and cave formations](https://www.nps.gov/maca/learn/nature/stalactites-stalagmites-and-cave-formations.htm)

## Alternatives considered

### Slot canyon plus cave mouth

Recommended and approved. The pair adds one vertical wall scene and one overhead enclosure scene. Both anchor positive structure on the perimeter bands that survive center masking.

### Pahoehoe lava

Deferred. Ropy lava crust is strong edge texture, but it reads as surface pattern rather than landform silhouette. It overlaps the pattern family and needs a separate spike to avoid reopening palette or family scope here.

### Natural arches

Cut. The defining inner curve sits behind the centered screenshot. The visible side strips would show only generic cliff faces, the same failure mode as the canyon alternative rejected in the Alpine spike.

### Basalt columns

Cut. Vertical repeats risk the Hoodoos fence failure. Column facets also overlap the geometric pattern family.

### Coastal sea stacks

Cut. Isolated narrow pillars repeat the Hoodoos post failure. Stacks that sit in the lower center are hidden behind the capture window.

## Existing contract

Terrain palettes own color and `TerrainKind` owns structure, the same composition as `--palette` plus `--terrain`. `src/polish.rs` defines four terrain palettes with stops ordered as sky, horizon, and ground. `glow_a` is the lit accent and `glow_b` is the shadow accent. `terrain.rs` relies on that ordering.

Every accepted scene must work with all four existing terrain palettes:

- `dunes`
- `mesa`
- `badlands`
- `glacier`

The style seed controls every free parameter through `style.seed ^ TERRAIN_SEED_SALT`. The same `--style-seed` must reproduce identical pixels. Rendering stays hand-rolled on the existing `image` and `rand` dependencies with private copies of `to_f32`, `mix3`, `smoothstep`, `quantize`, grain, and `warped_fbm`.

Pinned terrain uses `PresentationStyle.terrain` and `TerrainKind::from_name`. Public terrain names derive from `TerrainKind::NAMES`. No separate CLI or MCP enumeration is added.

## Design

### Slot canyon (`slot-canyon`)

Model a narrow canyon cross-section with **positive sandstone walls**, not a centered river void. Zion slot canyons are defined by water-carved walls with curved cross-beds and asymmetric narrowing. The identifying signal is wall material in the side bands, not water in the hidden center.

**Wall placement:** generate one dominant wall anchored to the left or right outer band and a secondary wall on the opposite side. The dominant wall must reach from near the sky band through the ground band so the full-height side strip shows continuous rock. Anchor centers partly off-frame so the capture window's center padding still reads as canyon wall, following the mesa lane pattern.

**Asymmetry:** dominant and secondary walls differ in horizontal extent, relief depth, and lit face direction. Seeds must not produce mirror-symmetric twins.

**Cross-bedding:** apply broad curved strata on both walls using low-frequency warped fields and gentle sinusoidal bends across the wall face. Strata curve with the wall surface rather than as horizontal barcode lines. Use the horizon color at low opacity for bedding tints.

**Floor:** a low continuous floor profile may connect the walls across the bottom band. A river, pool, or central negative slot is **not required** and must not be the scene's primary identifier. Avoid a bright horizontal waterline that reads as generic bedrock stripe, the Hoodoos iteration 1 failure.

**Lighting:** shade the wall interior faces and lift the outer rim with `glow_a`. Sample the local wall silhouette slope for face lighting, similar to badlands slope lighting but confined to wall masses.

**Horizon:** keep the shared sky and ground ramp. Local horizon modulation may follow the wall tops only where it does not paint a vertical column through the sky, the mesa ramp guard.

### Cave mouth (`cave-mouth`)

Model the **enclosing rock mass** around a cave opening, not the void inside. Mammoth Cave entrances and decorated chambers are read from ceiling rock, side walls, and large dripstone forms on the perimeter.

**Enclosure:** generate an asymmetric rock mass that wraps the top edge, both side edges, and the lower corners. The mass must be visible in the top band, side bands, and bottom corner pixels after center masking. The hidden center may stay darker or emptier, but identity must not depend on that void.

**Drapery and stalactites:** place 3 or 4 large features per seed, not a teeth fence. Each feature is either a broad drapery sheet or a single stalactite with a tapered tip. Features vary in width, length, and horizontal position. Minimum feature width at 440×300 must exceed one pixel. Cap or tip highlights use `glow_a`. Shadow under each form uses `glow_b`.

**No repeated teeth:** gap between adjacent features must vary by at least 0.04 canvas width between the smallest and largest gap at 440×300. Do not place evenly spaced icicle columns. This directly guards against the Hoodoos fence and cave-teeth failure modes.

**Rock texture:** low-amplitude warped fbm on the enclosing mass for fracture and bedding. Keep amplitude small so the mass reads as rock, not smoke blobs.

**Lighting:** light from off-frame side or above so rim highlights appear on the outer enclosure edge and on dripstone tips facing the light.

## Code boundaries

Primary implementation stays in `src/terrain.rs`:

- add `SlotCanyon` and `CaveMouth` to `TerrainKind`.
- add both public names `slot-canyon` and `cave-mouth` to `TerrainKind::NAMES` and `from_name`.
- add generation ranges for horizon, coverage, `features_across`, and light in `Terrain::generate`.
- add isolated profile helpers and structure painters for each scene.
- route new variants through `base_layer` horizon handling and `apply_structure`.

`src/polish.rs` needs no palette changes. Public enums and option lists that iterate `TerrainKind::NAMES` pick up the new names automatically. If either scene is accepted, update the hard-coded terrain examples in `src/cli.rs` and the hard-coded terrain descriptions in `src/mcp.rs` to list every retained kind. These are text updates to existing surfaces, not new surfaces.

If either scene is accepted after visual QA:

- update terrain documentation in `README.md`.
- record visual tradeoffs and iteration count in `implementation-notes.md`.

No new dependency, palette, command, configuration field, or rendering family is in scope.

## Data flow

1. `PresentationStyle` supplies the palette stops, glow colors, style seed, and optional pinned terrain.
2. `render` seeds `StdRng` from `style.seed ^ TERRAIN_SEED_SALT` and calls `Terrain::generate`.
3. `Terrain::generate` picks or pins the kind and rolls horizon, coverage, `features_across`, light, and `noise_seed` once.
4. `base_layer` paints the shared sky and ground ramps with smoothstep horizon cross-fade and atmospheric haze.
5. A scene-specific profile helper supplies local wall or enclosure geometry where required.
6. A scene-specific structure painter adds wall faces, cross-bedding, enclosure mass, or dripstone forms through `StructureCtx`.
7. Shared grain and quantization produce opaque final pixels.

Profile and mask helpers remain pure functions of canvas fractions, terrain parameters, and seeds so silhouette behavior can be tested without image comparison.

## Automated acceptance

Tests are written before implementation and observed failing.

Shared requirements for both scenes:

- both names parse and appear in `TerrainKind::NAMES`.
- rendering is deterministic and seed-sensitive.
- zero-width and zero-height rendering remains safe.
- every outer edge band and corner contains variation, existing `every_kind_carries_texture_at_the_edges_and_corners` extended to new names.
- existing dunes, mesa, badlands, and glacier tests remain unchanged and green.
- new kinds render differently from each other and from all four existing kinds at the same seed.

### Slot canyon profile requirements

Test at 440×300 with seeds 1, 7, 42, and 99.

- **Outer band wall mass:** the dominant wall's rock mask reaches at least 0.40 canvas height in the first or last 12.5% horizontal band.
- **Asymmetry:** dominant wall peak relief exceeds secondary wall peak relief by at least 0.03 canvas height.
- **Full-height side coverage:** in the dominant wall band, rock mask exceeds 0.25 for at least 60% of samples from sky to ground.
- **Cross-bedding signal:** curved strata modulation varies by at least 0.02 in normalized luminance across 16 vertical samples on the dominant wall face.
- **No column cliff:** adjacent horizontal samples on the wall top profile cannot drop more than 0.80 of local relief in one step, the mesa column guard adapted for walls.
- **No required river:** center 50% horizontal band average rock mask stays below 0.35 so the scene is not a solid fill.

### Cave mouth profile requirements

Test at 440×300 with seeds 1, 7, 42, and 99.

- **Top band enclosure:** rock mass mask in the top 8% of canvas height reaches at least 0.30 average across the width.
- **Side band enclosure:** left or right 12.5% band carries rock mass mask peak at least 0.35.
- **Lower corner mass:** bottom 15% combined with outer 12.5% bands shows rock mask peak at least 0.30 on both lower corners.
- **Feature count:** 3 or 4 dripstone or drapery features with peak mask at least 0.20 and width at least 0.02 canvas width at 440×300.
- **Gap variation:** smallest to largest center-to-center gap between features differs by at least 0.04 canvas width.
- **No teeth fence:** no more than one feature peak per 0.08 canvas width in the top and side bands combined.

## Visual acceptance

Render full backdrops and finished light and dark cards for:

- scenes: `slot-canyon`, `cave-mouth`.
- palettes: all four terrain palettes.
- seeds: 1, 7, 42, and 99.
- review size: 440×300.

For each scene, produce contact sheets and inspect all 16 palette-by-seed combinations for:

- recognition from visible side, top, and bottom strips after center masking.
- no dependence on a centered subject or central void.
- no flat outer band.
- no fence, barcode, smoke, icicle teeth, or generic noise reading.
- enough contrast behind both light and dark screenshots.
- clear distinction from dunes, mesa, badlands, glacier, and the other new scene.

### Masked perimeter review

For each accepted render, mask the center rectangle and inspect only the visible perimeter at three widths:

| Visible perimeter width | Mask center horizontal fraction |
| ----------------------- | --------------------------------- |
| 4% each side            | keep outer 4% left and right      |
| 6% each side            | keep outer 6% left and right      |
| 10% each side           | keep outer 10% left and right     |

Apply the same mask fractions to top and bottom bands. A scene passes masking only if the identifying structure remains readable at 4% and does not depend on the 10% band alone.

Finished card review uses `compose_card` with both light and dark window styles so padding and shadow move with the card.

## Iteration and cut rule

Allow at most **three** implementation-and-render iterations per scene. An iteration means one code adjustment followed by the full 16-render visual sheet at 440×300 plus masked perimeter checks.

If a scene still fails visual acceptance after its third sheet, **cut that scene**:

- remove its `TerrainKind` variant, profile helpers, structure painter, and tests from the shipping change.
- record why in `implementation-notes.md` with Brigade verify receipt ids.
- do not update `README.md` for a cut scene.
- continue with any scene that passed.

Both scenes may be cut. The shipping change may retain only dunes, mesa, badlands, and glacier, the same outcome as the Alpine and Hoodoos spike.

## Verification and delivery

After the last tracked edit, run:

```bash
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

An accepted change must have:

- the full verification entrypoint green.
- accepted visual sheets and masked perimeter passes for every retained scene.
- no ignored tests, lint allowances, debug output, or new dependency.
- a memory handoff recording durable profile and visual-QA lessons.
