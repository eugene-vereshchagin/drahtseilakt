"""Encodes what build.py, music.py and rails.py rendered into a sound theme's folder layout, under the game's names:
Sounds/<Name>.ogg (mono effects, Vorbis 96 kb/s) and Music/<Name>.ogg (stereo music, Vorbis 128 kb/s), as the shipped
neuro theme has them. It writes into the folder given (by default a fresh one beside this file, never the theme
itself), so a rebuild can be listened to and compared before it is copied into assets/sounds/neuro/.

usage: python3 to_theme.py [OUT] [--rails RAILS]
  OUT     the folder to write (default: theme-out/ beside this file)
  RAILS   the folder rails.py wrote: its Roll_Wood_Metal_C and Roll_Stone_Metal_A (the owner's choices by ear) become
          Roll_Wood_Metal and Roll_Stone_Metal, replacing build.py's older rail roles
Needs ffmpeg. Effects without a rendered file (not rendered yet, or not made here, as Hit_Sack) are listed, not made."""
import os, shutil, subprocess, sys

H = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, H)
import build  # noqa: E402  (its ROLES: role -> the game's name)
import music  # noqa: E402

RAIL_PICKS = {'Roll_Wood_Metal': 'Roll_Wood_Metal_C', 'Roll_Stone_Metal': 'Roll_Stone_Metal_A'}
END_MUSIC = ['Music_EndCheckpoint', 'Music_Final', 'Music_LastFinal']


def encode(src, dst, rate):
    subprocess.run(['ffmpeg', '-loglevel', 'error', '-y', '-i', src, '-c:a', 'libvorbis', '-b:a', rate, dst],
                   check=True)


def main():
    args = sys.argv[1:]
    rails = None
    if '--rails' in args:
        k = args.index('--rails')
        rails = args[k + 1]
        del args[k:k + 2]
    out = args[0] if args else os.path.join(H, 'theme-out')
    os.makedirs(os.path.join(out, 'Sounds'), exist_ok=True)
    os.makedirs(os.path.join(out, 'Music'), exist_ok=True)
    missing = []
    for role, (_, name, *_rest) in build.ROLES.items():
        if rails and name in RAIL_PICKS:
            continue
        # the atmospheres lie in Music/, as in the shipped theme; every other role of build.py is an effect in Sounds/
        folder = 'Music' if name.startswith('Music_Atmo') else 'Sounds'
        src = os.path.join(H, 'generated', f'{role}.wav')
        if os.path.isfile(src):
            encode(src, os.path.join(out, folder, f'{name}.ogg'), '96k')
        else:
            missing.append(name)
    for name in list(music.SPEC) + END_MUSIC:
        src = os.path.join(H, 'generated', 'music', f'{name}.wav')
        if os.path.isfile(src):
            encode(src, os.path.join(out, 'Music', f'{name}.ogg'), '128k')
        else:
            missing.append(name)
    if rails:
        for name, pick in RAIL_PICKS.items():
            src = os.path.join(rails, f'{pick}.ogg')
            if os.path.isfile(src):
                shutil.copyfile(src, os.path.join(out, 'Sounds', f'{name}.ogg'))
            else:
                missing.append(name)
    print(f'written into {out}')
    if missing:
        print('not rendered (left out):', ' '.join(sorted(missing)))


if __name__ == '__main__':
    main()
