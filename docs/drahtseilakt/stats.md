# By the numbers

How Drahtseilakt was made, counted on 2026-11-03 (`scripts/stats.py`). Every line of code, every page of these
docs and every generated sound was written by Claude (Anthropic's AI model, in Claude Code). Veresk, who leads the
project, wrote none of it: he set the goals, made the decisions, played, listened and said what was wrong.

## Time

| | |
|---|---|
| Work began | 2026-10-30, on the fork of OpenBallance (2026-10-01) |
| Calendar time | 4 days |
| Claude's working time, main session | 34.6 hours (intervals between actions under 15 minutes) |
| Claude's working time, all agents summed | 47.8 hours (up to six agents worked in parallel) |

## Code

| | Lines |
|---|---|
| Rust, 6 crates | 44,108 |
| — game (window, menu, sound, simulation rules) | 22,882 |
| — ivp (the physics engine, ported) | 14,080 |
| — gamedata, level, nmo, cdimage (files, formats, the CD) | 7,146 |
| Shaders (WGSL) | 296 |
| The toolbox (its own repository, Rust) | 3,874 |
| Sound synthesis (Python) | 1,234 |
| Tests | 249 |
| Commits | 243 in the game since the fork, 19 in the toolbox |

What it replaces: the C code of OpenBallance it began from, 27,144 lines in 83 files, kept only as reference until
it leaves.

## Docs

Four languages (English, Russian, German, Simplified Chinese), 11 pages each, about 1,200 lines per language.

## Sound

The generated sound theme is made entirely by code: 51 sounds and pieces of music — every ball on every surface,
the fans, the thunder, the atmospheres, fifteen plucked-string phrases and the music of the finish — synthesized
from measurements of the original's sounds, without a single sample of it.

## Tokens

| | |
|---|---|
| Model calls | 7,472 (Claude Opus 5.5: 6,368, Claude Haiku 5.5: 1,104) |
| Tokens written by the model | 6.7 million |
| Tokens read, fresh | 15.6 thousand |
| Tokens read from the prompt cache | 3.30 billion |
| Tokens written to the prompt cache | 25.0 million |
| All tokens processed | 3.33 billion |

Most of the reading is the same working context read again on every step: a long session over a large code base.
