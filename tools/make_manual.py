#!/usr/bin/env python3 -I
"""Builds the player's manual: local HTML pages from the player's manual sources (docs/manual/<lang>/*.md), in en, ru, de and zh.

    python3 -I tools/make_manual.py [OUTDIR]        (default: assets/manual)

Needs only the standard library and the `markdown` module. Reads no network and writes none of its references
outside OUTDIR: a page links to its own style.css and fonts, to the other pages, and (as plain links the player may
click) to outside sites. Output layout: OUTDIR/index.html (language choice), OUTDIR/<lang>/<page>.html,
OUTDIR/style.css, OUTDIR/fonts/. The game opens OUTDIR/<lang>/index.html (and <lang>/levels.html from the levels hint).
"""
import html
import os
import re
import shutil
import sys
from pathlib import Path

import markdown
from markdown.extensions.toc import slugify_unicode

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
FONTS = ROOT / "assets" / "fonts"

# (code, name in its own language, source folder)
LANGS = [
    ("en", "English", DOCS / "manual" / "en"),
    ("ru", "Русский", DOCS / "manual" / "ru"),
    ("de", "Deutsch", DOCS / "manual" / "de"),
    ("zh", "简体中文", DOCS / "manual" / "zh"),
]

# Sections and pages (slug = source file name without .md); the order is the sidebar's.
SECTIONS = [
    ("play", ["playing", "levels", "looks", "toolbox", "mods", "sound", "help"]),
]

UI = {
    "en": {
        "title": "Drahtseilakt manual",
        "home": "Home",
        "play": "Contents",
        "lang": "Language",
        "untranslated": "This page is not translated into your language yet; it is shown in English.",
        "lead": "A new engine for Ballance, the 2004 game of a ball on a rail over an abyss. It plays the original "
        "levels from your own copy, the open level format, and community maps.",
        "keys_h": "Controls at a glance",
        "keys": [
            ("Arrow keys", "roll the ball, relative to the camera"),
            ("Shift + Left / Right", "turn the camera"),
            ("Space", "lift the camera over the level"),
            ("Esc", "pause menu"),
            ("F11", "full screen"),
            ("F1", "this manual, anywhere in the menu"),
            ("Gamepad", "left stick or cross rolls, shoulders turn the camera, lower face button lifts, Start pauses"),
        ],
        "keys_n": "Other keys can be bound in Options, Controls.",
        "pages_h": "Pages",
        "note_h": "Manual",
    },
    "ru": {
        "title": "Руководство Drahtseilakt",
        "home": "Главная",
        "play": "Содержание",
        "lang": "Язык",
        "untranslated": "Эта страница ещё не переведена на ваш язык, она показана по-английски.",
        "lead": "Новый движок для Ballance, игры 2004 года о шаре на рельсе над пропастью. Играет в оригинальные "
        "уровни из вашей копии, в открытый формат уровней и в карты сообщества.",
        "keys_h": "Управление вкратце",
        "keys": [
            ("Стрелки", "катят шар относительно камеры"),
            ("Shift + влево / вправо", "поворот камеры"),
            ("Пробел", "поднять камеру над уровнем"),
            ("Esc", "меню паузы"),
            ("F11", "полный экран"),
            ("F1", "это руководство, в любом месте меню"),
            ("Геймпад", "левый стик или крестовина катят, плечевые кнопки поворачивают камеру, нижняя кнопка "
             "поднимает, Start ставит паузу"),
        ],
        "keys_n": "Другие клавиши назначаются в Настройках, Управление.",
        "pages_h": "Страницы",
        "note_h": "Руководство",
    },
    "de": {
        "title": "Drahtseilakt-Anleitung",
        "home": "Start",
        "play": "Inhalt",
        "lang": "Sprache",
        "untranslated": "Diese Seite ist noch nicht in Ihre Sprache übersetzt; sie wird auf Englisch gezeigt.",
        "lead": "Eine neue Engine für Ballance, das Spiel von 2004 über eine Kugel auf einer Schiene über dem Abgrund. "
        "Sie spielt die Originallevel aus Ihrer eigenen Kopie, das offene Levelformat und Community-Karten.",
        "keys_h": "Steuerung auf einen Blick",
        "keys": [
            ("Pfeiltasten", "rollen die Kugel, relativ zur Kamera"),
            ("Umschalt + Links / Rechts", "Kamera drehen"),
            ("Leertaste", "Kamera über das Level heben"),
            ("Esc", "Pausenmenü"),
            ("F11", "Vollbild"),
            ("F1", "diese Anleitung, überall im Menü"),
            ("Gamepad", "linker Stick oder Steuerkreuz rollt, Schultertasten drehen die Kamera, untere Taste hebt, "
             "Start pausiert"),
        ],
        "keys_n": "Andere Tasten lassen sich unter Optionen, Steuerung belegen.",
        "pages_h": "Seiten",
        "note_h": "Anleitung",
    },
    "zh": {
        "title": "Drahtseilakt 手册",
        "home": "首页",
        "play": "目录",
        "lang": "语言",
        "untranslated": "此页尚未翻译成您的语言，以英文显示。",
        "lead": "Ballance（2004 年的游戏，一只球在深渊上的轨道上滚动）的新引擎。可玩您自己拷贝中的原版关卡、"
        "开放关卡格式和社区地图。",
        "keys_h": "操作一览",
        "keys": [
            ("方向键", "相对于镜头滚动球"),
            ("Shift + 左 / 右", "转动镜头"),
            ("空格", "将镜头抬到关卡上方"),
            ("Esc", "暂停菜单"),
            ("F11", "全屏"),
            ("F1", "本手册，菜单中任何位置"),
            ("手柄", "左摇杆或十字键滚动，肩键转动镜头，下方按键抬升，Start 暂停"),
        ],
        "keys_n": "其他按键可在 选项、控制 中设置。",
        "pages_h": "页面",
        "note_h": "手册",
    },
}

# where the other languages' pages of the sidebar get their titles when the page has no translation: the English H1
CSS = r"""
@font-face { font-family: "Noto Sans"; font-weight: 400; src: url("../fonts/NotoSans-Regular.ttf") format("truetype"); font-display: swap; }
@font-face { font-family: "Noto Sans"; font-weight: 700; src: url("../fonts/NotoSans-Bold.ttf") format("truetype"); font-display: swap; }
@font-face { font-family: "Tektur"; font-weight: 400 900; src: url("../fonts/Tektur-Variable.ttf") format("truetype"); font-display: swap; }
@font-face { font-family: "Noto Sans SC"; font-weight: 400; src: url("../fonts/NotoSansSC-Regular.otf") format("opentype"); font-display: swap;
  unicode-range: U+2E80-9FFF, U+3000-303F, U+FF00-FFEF; }

:root {
  --bg: #f5f2ec; --panel: #ffffff; --side: #ebe6dc; --text: #232830; --muted: #5d6572;
  --accent: #b5530a; --accent-soft: #f3dfc9; --line: #d9d2c4; --code-bg: #efeae0; --ok: #1f6f4a;
  --shadow: 0 1px 2px rgba(0,0,0,.06), 0 8px 24px rgba(40,30,10,.06);
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #101319; --panel: #171b23; --side: #0c0f14; --text: #e6e8ec; --muted: #9aa3b0;
    --accent: #ffa245; --accent-soft: #33281a; --line: #272d38; --code-bg: #1e232d; --ok: #5ccf9a;
    --shadow: 0 1px 2px rgba(0,0,0,.4), 0 8px 24px rgba(0,0,0,.35);
  }
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0; background: var(--bg); color: var(--text); line-height: 1.65; font-size: 17px;
  font-family: "Noto Sans", "Noto Sans SC", system-ui, "Segoe UI", sans-serif;
}
.layout { display: grid; grid-template-columns: 17rem minmax(0, 1fr); min-height: 100vh; }
aside {
  background: var(--side); border-right: 1px solid var(--line); padding: 1.6rem 1.2rem; position: sticky; top: 0;
  height: 100vh; overflow-y: auto;
}
.brand { display: block; font-family: "Tektur", "Noto Sans SC", sans-serif; font-weight: 800; font-size: 1.45rem;
  letter-spacing: .02em; color: var(--text); text-decoration: none; margin-bottom: .2rem; }
.brand b { color: var(--accent); }
.tag { color: var(--muted); font-size: .8rem; margin-bottom: 1.4rem; }
aside h2 { font-family: "Tektur", "Noto Sans SC", sans-serif; font-size: .72rem; text-transform: uppercase;
  letter-spacing: .14em; color: var(--muted); margin: 1.5rem 0 .4rem; font-weight: 600; }
aside nav a { display: block; padding: .38rem .7rem; border-radius: .5rem; color: var(--text); text-decoration: none;
  font-size: .94rem; line-height: 1.35; }
aside nav a:hover { background: var(--accent-soft); }
aside nav a.cur { background: var(--accent-soft); color: var(--accent); font-weight: 700; box-shadow: inset 3px 0 0 var(--accent); }
.langs { display: flex; flex-wrap: wrap; gap: .4rem; margin-top: .3rem; }
.langs a { padding: .22rem .65rem; border: 1px solid var(--line); border-radius: 2rem; color: var(--text);
  text-decoration: none; font-size: .85rem; }
.langs a:hover { border-color: var(--accent); }
.langs a.cur { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 700; }
main { padding: 2.2rem clamp(1.2rem, 4vw, 3.5rem) 4rem; }
article { max-width: 52rem; margin: 0 auto; background: var(--panel); border: 1px solid var(--line);
  border-radius: 1rem; padding: 2rem clamp(1.1rem, 3.5vw, 2.8rem) 2.4rem; box-shadow: var(--shadow); }
h1, h2, h3, h4 { font-family: "Tektur", "Noto Sans", "Noto Sans SC", sans-serif; line-height: 1.25; font-weight: 700; }
h1 { font-size: 2.1rem; margin: 0 0 1rem; letter-spacing: .01em; }
h1::after { content: ""; display: block; width: 4rem; height: 4px; margin-top: .7rem; background: var(--accent); border-radius: 2px; }
h2 { font-size: 1.45rem; margin: 2.4rem 0 .7rem; padding-bottom: .3rem; border-bottom: 1px solid var(--line); }
h3 { font-size: 1.15rem; margin: 1.8rem 0 .4rem; }
h4 { font-size: 1rem; margin: 1.4rem 0 .3rem; color: var(--muted); }
a { color: var(--accent); text-underline-offset: 2px; }
a.ext::after { content: " \2197"; font-size: .8em; }
p, li { overflow-wrap: anywhere; }
li { margin: .2rem 0; }
code, pre, kbd { font-family: ui-monospace, "Cascadia Mono", "DejaVu Sans Mono", Menlo, Consolas, monospace; font-size: .88em; }
code { background: var(--code-bg); padding: .08em .35em; border-radius: .3rem; }
pre { background: var(--code-bg); border: 1px solid var(--line); border-radius: .6rem; padding: .9rem 1.1rem; overflow-x: auto; line-height: 1.5; }
pre code { background: none; padding: 0; font-size: .88rem; }
blockquote { margin: 1.2rem 0; padding: .6rem 1.1rem; background: var(--accent-soft); border-left: 4px solid var(--accent);
  border-radius: 0 .6rem .6rem 0; }
blockquote p { margin: .4rem 0; }
.tablewrap { overflow-x: auto; margin: 1.2rem 0; border: 1px solid var(--line); border-radius: .6rem; }
table { border-collapse: collapse; width: 100%; font-size: .93rem; }
th, td { padding: .5rem .8rem; text-align: left; vertical-align: top; border-bottom: 1px solid var(--line); }
th { background: var(--code-bg); font-family: "Tektur", "Noto Sans", sans-serif; font-weight: 600; white-space: nowrap; }
tr:last-child td { border-bottom: 0; }
hr { border: 0; border-top: 1px solid var(--line); margin: 2rem 0; }
.untranslated { background: var(--accent-soft); border: 1px solid var(--accent); border-radius: .6rem; padding: .55rem .9rem;
  font-size: .9rem; margin: 0 0 1.4rem; }
.lead { font-size: 1.1rem; color: var(--muted); margin: 0 0 1.6rem; }
.lead p { margin: 0; }
.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr)); gap: .8rem; margin: 1rem 0 0; }
.cards a { display: block; padding: .9rem 1.1rem; border: 1px solid var(--line); border-radius: .8rem; text-decoration: none;
  color: var(--text); background: var(--bg); font-weight: 700; font-family: "Tektur", "Noto Sans", "Noto Sans SC", sans-serif; }
.cards a:hover { border-color: var(--accent); color: var(--accent); }
.keys { width: 100%; }
.keys td:first-child { white-space: nowrap; font-weight: 700; width: 1%; }
.pick { max-width: 30rem; margin: 18vh auto 0; text-align: center; }
.pick .brand { font-size: 2.6rem; }
.foot { max-width: 52rem; margin: 1.2rem auto 0; color: var(--muted); font-size: .8rem; text-align: center; }
@media (max-width: 820px) {
  .layout { grid-template-columns: 1fr; }
  aside { position: static; height: auto; border-right: 0; border-bottom: 1px solid var(--line); }
  main { padding: 1.2rem 1rem 3rem; }
  article { padding: 1.3rem 1.1rem 1.6rem; }
  h1 { font-size: 1.7rem; }
}
@media print { aside, .foot { display: none; } .layout { display: block; } article { border: 0; box-shadow: none; } }
"""


def title_of(path: Path) -> str:
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return path.stem


def shipped_pages():
    return [p for _, ps in SECTIONS for p in ps]


def convert(source: str, lang: str, page: str, titles: dict) -> tuple[str, list[str]]:
    """Markdown -> HTML body, links rewritten. Returns the body and the unresolved-link notes."""
    notes = []
    md = markdown.Markdown(
        extensions=["tables", "fenced_code", "toc", "sane_lists", "attr_list"],
        extension_configs={"toc": {"slugify": slugify_unicode, "permalink": False}},
    )
    body = md.convert(source)
    pages = set(shipped_pages())

    def fix(m):
        attrs, href, text = m.group(1), m.group(2), m.group(3)
        href = html.unescape(href)
        if re.match(r"https?://", href):
            return f'<a{attrs} href="{html.escape(href)}" class="ext" rel="noopener">{text}</a>'
        if href.startswith("#"):
            return f'<a{attrs} href="{html.escape(href)}">{text}</a>'
        if href.startswith("mailto:"):
            return m.group(0)
        path, _, frag = href.partition("#")
        slug = re.sub(r"\.(md|html)$", "", path.rstrip("/").rsplit("/", 1)[-1])
        # a link to a page of ours: ./x.md, x.md, ../drahtseilakt/x.md, /drahtseilakt/x, /ru/drahtseilakt/x
        if slug in pages and (path.endswith(".md") or path.startswith("/drahtseilakt") or "drahtseilakt/" in path):
            tail = f"#{frag}" if frag else ""
            return f'<a{attrs} href="{slug}.html{tail}">{text}</a>'
        notes.append(f"{lang}/{page}: link to {href} is not shipped, shown as text")
        return text

    body = re.sub(r"<a([^>]*?) href=\"([^\"]*)\"[^>]*>(.*?)</a>", fix, body, flags=re.S)
    # wide tables scroll inside their box on a narrow screen
    body = body.replace("<table>", '<div class="tablewrap"><table>').replace("</table>", "</table></div>")
    return body, notes


def sidebar(lang: str, cur: str, titles: dict, ui: dict) -> str:
    out = [f'<a class="brand" href="index.html">Draht<b>seil</b>akt</a><div class="tag">{html.escape(ui["note_h"])}</div>']
    out.append("<nav>")
    out.append(f'<a href="index.html"{" class=cur" if cur == "index" else ""}>{html.escape(ui["home"])}</a>')
    for sec, pages in SECTIONS:
        out.append(f"<h2>{html.escape(ui[sec])}</h2>")
        for p in pages:
            c = ' class="cur"' if p == cur else ""
            out.append(f'<a{c} href="{p}.html">{html.escape(titles[lang][p])}</a>')
    out.append("</nav>")
    out.append(f'<h2>{html.escape(ui["lang"])}</h2><div class="langs">')
    for code, name, _ in LANGS:
        c = ' class="cur"' if code == lang else ""
        out.append(f'<a{c} href="../{code}/{cur}.html" lang="{code}">{html.escape(name)}</a>')
    out.append("</div>")
    return "\n".join(out)


def shell(lang: str, cur: str, title: str, content: str, titles: dict) -> str:
    ui = UI[lang]
    return f"""<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>{html.escape(title)} - {html.escape(ui["title"])}</title>
<link rel="stylesheet" href="../style.css">
</head>
<body>
<div class="layout">
<aside>
{sidebar(lang, cur, titles, ui)}
</aside>
<main>
<article>
{content}
</article>
<div class="foot">Drahtseilakt</div>
</main>
</div>
</body>
</html>
"""


def home(lang: str, titles: dict, intro: str = "", rest: str = "") -> str:
    ui = UI[lang]
    rows = "".join(
        f"<tr><td>{html.escape(k)}</td><td>{html.escape(v)}</td></tr>" for k, v in ui["keys"]
    )
    cards = []
    for sec, pages in SECTIONS:
        for p in pages:
            cards.append(f'<a href="{p}.html">{html.escape(titles[lang][p])}</a>')
    return (
        f'<h1>{html.escape(ui["title"])}</h1>\n<div class="lead">{intro or html.escape(ui["lead"])}</div>\n'
        f'<h2>{html.escape(ui["keys_h"])}</h2>\n<div class="tablewrap"><table class="keys">{rows}</table></div>\n'
        f'<p>{html.escape(ui["keys_n"])}</p>\n{rest}\n'
        f'<h2>{html.escape(ui["pages_h"])}</h2>\n<div class="cards">{"".join(cards)}</div>\n'
    )


def pick_page() -> str:
    links = "".join(f'<a href="{c}/index.html" lang="{c}">{html.escape(n)}</a>' for c, n, _ in LANGS)
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>Drahtseilakt</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
<div class="pick"><span class="brand">Draht<b>seil</b>akt</span>
<div class="langs" style="justify-content:center;margin-top:1.5rem">{links}</div></div>
</body>
</html>
"""


def copy_fonts(out: Path, zh_text: str):
    dst = out / "fonts"
    dst.mkdir(parents=True, exist_ok=True)
    for name in ["NotoSans-Regular.ttf", "NotoSans-Bold.ttf", "Tektur-Variable.ttf",
                 "OFL-NotoSans.txt", "OFL-Tektur.txt", "OFL.txt"]:
        shutil.copy2(FONTS / name, dst / name)
    sc = "NotoSansSC-Regular.otf"
    try:  # the Chinese pages use a few thousand glyphs: subset when fontTools is there (the file is 8 MB)
        from fontTools import subset
        opts = subset.Options()
        opts.layout_features = ["*"]
        font = subset.load_font(str(FONTS / sc), opts)
        sub = subset.Subsetter(opts)
        sub.populate(text=zh_text + "".join(chr(c) for c in range(0x20, 0x7F)))
        sub.subset(font)
        subset.save_font(font, str(dst / sc), opts)
    except Exception:
        shutil.copy2(FONTS / sc, dst / sc)


def build(out: Path) -> list[str]:
    out = Path(out)
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    notes: list[str] = []
    titles = {}
    for code, _, folder in LANGS:
        titles[code] = {}
        for p in shipped_pages():
            src = folder / f"{p}.md"
            if not src.exists():
                src = LANGS[0][2] / f"{p}.md"
            titles[code][p] = title_of(src)
    zh_text = ""
    for code, _, folder in LANGS:
        (out / code).mkdir()
        for p in shipped_pages():
            src, own = folder / f"{p}.md", True
            if not src.exists():
                src, own = LANGS[0][2] / f"{p}.md", False
            text = src.read_text(encoding="utf-8")
            body, n = convert(text, code, p, titles)
            notes += n
            if not own:
                body = f'<p class="untranslated">{html.escape(UI[code]["untranslated"])}</p>\n' + body
            if code == "zh":
                zh_text += text
            (out / code / f"{p}.html").write_text(shell(code, p, titles[code][p], body, titles), encoding="utf-8")
        isrc = folder / "index.md"
        if not isrc.exists():
            isrc = LANGS[0][2] / "index.md"
        itext = isrc.read_text(encoding="utf-8")
        head, _, tail = itext.partition("\n## ")
        ihead, n = convert(head, code, "index", titles)
        irest, n2 = convert("## " + tail, code, "index", titles)
        notes += n + n2
        h = home(code, titles, ihead, irest)
        if code == "zh":
            zh_text += itext + home("zh", titles)
        (out / code / "index.html").write_text(shell(code, "index", UI[code]["home"], h, titles), encoding="utf-8")
    (out / "index.html").write_text(pick_page(), encoding="utf-8")
    css = CSS.replace('url("../fonts/', 'url("fonts/')
    (out / "style.css").write_text(css, encoding="utf-8")
    zh_text += "".join(UI["zh"][k] for k in ("title", "home", "play", "lang", "untranslated", "lead",
                                             "keys_h", "keys_n", "pages_h", "note_h"))
    zh_text += "".join(a + b for a, b in UI["zh"]["keys"])
    zh_text += "".join(n for _, n, _ in LANGS)
    copy_fonts(out, zh_text)
    return notes


def main(argv):
    out = Path(argv[1]) if len(argv) > 1 else ROOT / "assets" / "manual"
    notes = build(out)
    for n in sorted(set(notes)):
        print("note:", n, file=sys.stderr)
    print(f"manual written to {out}")


if __name__ == "__main__":
    main(sys.argv)
