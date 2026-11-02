# The neuro sound theme

Author: Claude (Anthropic), for Veresk — every sound and piece here was designed and synthesized by Claude, an AI
model by Anthropic, working for the project's owner.

Every file in `Sounds/` and `Music/` was synthesized for Drahtseilakt by code: oscillators, noise, filters and modal synthesis
(banks of decaying resonators). The original's sounds were only measured (duration, loudness envelope, spectral
centroid, peak frequencies and decay) to set the synthesis parameters; no sample of the original was resampled,
cut, filtered or otherwise copied into them. They are part of the game and under its licence (GPL-3.0-only).

The theme holds every role the game asks for, so it takes nothing from another theme (`fallback none` in
theme.txt); a test checks this.

How the paper ball and the iron posts are made (redone after listening in game): the paper ball is a granular
crackle — thousands of millisecond noise bursts, each band-passed at its own frequency, over a soft rustle with slow
swells as the crumpled ball turns; the ball against an iron post excites the inharmonic bending modes of a steel
beam (ratios 1 : 2.76 : 5.40 : 8.93 …), each a close pair so it beats slightly, ringing for about half a second.

Redone a second time after the owner listened again (2026-11-02), each with a different model: the fall is a gusting
wind of repeated upward sweeps (a noise band with a few harmonics gliding up half an octave or more and cut off), the
gusts coming faster and starting higher as the ball drops away; the paper ball landing is a dense, almost white
scrunch of overlapping friction grains, one crunch and a few settling rustles; the stone ball against an iron post is
a short heavy clank — a dense field of short metal partials struck by a chatter of five or six impacts; rolling on
iron rails is a low hum with a grinding clatter of bright bursts; rolling on wood is a smooth deep rumble of dense
soft contacts in hollow planks whose colour drifts, without knocks.

The level music (`Music/Music_Theme_<n>_<part>.ogg`, five themes of three parts) is synthesized the same way: short,
quiet East-Asian plucked-string phrases, each composed by its own seeded rules (a pentatonic contour arching up and
coming home to the mode's root, slides into notes, press-bends and the slow vibrato of the left hand, the tremolo of
long notes, sweeps up the strings, a low drum in the fifth theme), played on an additive guzheng-like string in a soft
synthesized room. Of the original's parts only numbers were measured: length, loudness, the pitch classes that sound
most, the onset grid and density, and how long a phrase plays before its tail.

The end music is made the same way. `Music_EndCheckpoint`, which loops after the last checkpoint near the finish,
keeps the original's length and grid (64 eighth notes at 136 a minute, 14.11 s): a muted, driving pentatonic
ostinato in D over a low drone that builds bar by bar (more notes, a drum on the beats, a higher answering line, a
run back into the start), rendered so its tails and room wrap round and it loops without a seam. `Music_Final` is a
struck opening, a sweep up the strings and slow ringing notes in A; `Music_LastFinal` is a bright high chime over a
soft low rumble.

The roles the system theme used to fill (made 2026-11-03, for a theme complete on its own): the ball on the big dome
rings the shell's low modes (stone: a 390/440 Hz pair over a 98 Hz body; wood: a deep 43 Hz boom with a 255 Hz ring)
after a short knock; the level loading is a struck high bell (2.67 kHz with a slow beat) under thin tones sweeping
upward; the thunder is a dry crack and a long rumble of overlapping low rolls; the three atmospheres are a quiet
swell and fade of glassy sustained tones, of a breathing band of wind, and of a breathy pad with faint tones in it.
