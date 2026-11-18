#!/usr/bin/env python3 -I
"""Writes the download table of the READMEs (README.md, README.ru.md, README.de.md, README.zh.md) from the newest
published release, so the READMEs and a site that reads them always link the current files.

    python3 -I scripts/update-download-table.py RELEASE.json [REPO]

RELEASE.json is GitHub's answer to GET /repos/<repo>/releases/latest (fields tag_name, html_url, assets[].name,
assets[].browser_download_url), or the word `none` when nothing is published yet. The table goes between the lines
`<!-- downloads:start -->` and `<!-- downloads:end -->` of each README; the rest of the file is not touched. The 13
picked maps come from the data release `data-<scripts/DATA_VERSION>`. Standard library only; reads no network.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
START, END = "<!-- downloads:start -->", "<!-- downloads:end -->"

# (row key, file name pattern); <v> is the version part of the name
ROWS = [
    ("win-x64-setup", r"drahtseilakt-<v>-windows-x86_64-setup\.exe"),
    ("win-x64-zip", r"drahtseilakt-<v>-windows-x86_64\.zip"),
    ("win-arm-setup", r"drahtseilakt-<v>-windows-aarch64-setup\.exe"),
    ("win-arm-zip", r"drahtseilakt-<v>-windows-aarch64\.zip"),
    ("mac-arm-dmg", r"drahtseilakt-<v>-macos-aarch64\.dmg"),
    ("mac-x64-dmg", r"drahtseilakt-<v>-macos-x86_64\.dmg"),
    ("linux-x64-tar", r"drahtseilakt-<v>-linux-x86_64\.tar\.gz"),
    ("linux-arm-tar", r"drahtseilakt-<v>-linux-aarch64\.tar\.gz"),
    ("deb-x64", r"drahtseilakt_<v>_amd64\.deb"),
    ("deb-arm", r"drahtseilakt_<v>_arm64\.deb"),
    ("deb-data", r"drahtseilakt-data_<v>_all\.deb"),
    ("rpm-x64", r"drahtseilakt-<v>\.x86_64\.rpm"),
    ("rpm-arm", r"drahtseilakt-<v>\.aarch64\.rpm"),
    ("rpm-data", r"drahtseilakt-data-<v>\.noarch\.rpm"),
    ("freebsd", r"drahtseilakt-<v>-freebsd-x86_64\.tar\.gz"),
    ("sums", r"SHA256SUMS"),
]

TEXT = {
    "en": {
        "head": "| System | Download |\n|---|---|",
        "none": "No release is published yet. The builds appear here once the first one is.",
        "release": "Version **{tag}**, all files and notes: [{url}]({url}).",
        "levels": "The 13 picked maps, a separate download (about 1.1 GB, one zip each, unpack into the game's "
        "`assets/levels/drahtseilakt/`): [data-{dv}]({levels}).",
        "win-x64-setup": "Windows x64, installer", "win-x64-zip": "Windows x64, portable",
        "win-arm-setup": "Windows ARM, installer", "win-arm-zip": "Windows ARM, portable",
        "mac-arm-dmg": "macOS Apple Silicon (unsigned)", "mac-x64-dmg": "macOS Intel (unsigned)",
        "linux-x64-tar": "Linux x64, portable", "linux-arm-tar": "Linux ARM, portable",
        "deb-x64": "Debian/Ubuntu x64 (with the data package)", "deb-arm": "Debian/Ubuntu ARM (with the data package)",
        "deb-data": "Debian/Ubuntu: the data package", "rpm-x64": "Fedora/openSUSE x64 (with the data package)",
        "rpm-arm": "Fedora/openSUSE ARM (with the data package)", "rpm-data": "Fedora/openSUSE: the data package",
        "freebsd": "FreeBSD x64 (experimental)", "sums": "Checksums",
    },
    "ru": {
        "head": "| Система | Скачать |\n|---|---|",
        "none": "Релиз ещё не опубликован. Сборки появятся здесь, когда выйдет первый.",
        "release": "Версия **{tag}**, все файлы и описание: [{url}]({url}).",
        "levels": "13 избранных карт скачиваются отдельно (около 1,1 ГБ, по zip на карту, распаковать в "
        "`assets/levels/drahtseilakt/` игры): [data-{dv}]({levels}).",
        "win-x64-setup": "Windows x64, установщик", "win-x64-zip": "Windows x64, портативная",
        "win-arm-setup": "Windows ARM, установщик", "win-arm-zip": "Windows ARM, портативная",
        "mac-arm-dmg": "macOS Apple Silicon (без подписи)", "mac-x64-dmg": "macOS Intel (без подписи)",
        "linux-x64-tar": "Linux x64, портативная", "linux-arm-tar": "Linux ARM, портативная",
        "deb-x64": "Debian/Ubuntu x64 (вместе с пакетом данных)", "deb-arm": "Debian/Ubuntu ARM (вместе с пакетом данных)",
        "deb-data": "Debian/Ubuntu: пакет данных", "rpm-x64": "Fedora/openSUSE x64 (вместе с пакетом данных)",
        "rpm-arm": "Fedora/openSUSE ARM (вместе с пакетом данных)", "rpm-data": "Fedora/openSUSE: пакет данных",
        "freebsd": "FreeBSD x64 (экспериментально)", "sums": "Контрольные суммы",
    },
    "de": {
        "head": "| System | Download |\n|---|---|",
        "none": "Noch ist kein Release veröffentlicht. Die Builds erscheinen hier mit dem ersten.",
        "release": "Version **{tag}**, alle Dateien und Hinweise: [{url}]({url}).",
        "levels": "Die 13 ausgewählten Karten, ein eigener Download (etwa 1,1 GB, je Karte ein Zip, entpacken nach "
        "`assets/levels/drahtseilakt/` des Spiels): [data-{dv}]({levels}).",
        "win-x64-setup": "Windows x64, Installer", "win-x64-zip": "Windows x64, portabel",
        "win-arm-setup": "Windows ARM, Installer", "win-arm-zip": "Windows ARM, portabel",
        "mac-arm-dmg": "macOS Apple Silicon (nicht signiert)", "mac-x64-dmg": "macOS Intel (nicht signiert)",
        "linux-x64-tar": "Linux x64, portabel", "linux-arm-tar": "Linux ARM, portabel",
        "deb-x64": "Debian/Ubuntu x64 (mit dem Datenpaket)", "deb-arm": "Debian/Ubuntu ARM (mit dem Datenpaket)",
        "deb-data": "Debian/Ubuntu: das Datenpaket", "rpm-x64": "Fedora/openSUSE x64 (mit dem Datenpaket)",
        "rpm-arm": "Fedora/openSUSE ARM (mit dem Datenpaket)", "rpm-data": "Fedora/openSUSE: das Datenpaket",
        "freebsd": "FreeBSD x64 (experimentell)", "sums": "Prüfsummen",
    },
    "zh": {
        "head": "| 系统 | 下载 |\n|---|---|",
        "none": "尚未发布任何版本。第一个版本发布后，构建会出现在这里。",
        "release": "版本 **{tag}**，全部文件和说明：[{url}]({url})。",
        "levels": "13 张精选地图需单独下载（约 1.1 GB，每张一个 zip，解压到游戏的 `assets/levels/drahtseilakt/`）："
        "[data-{dv}]({levels})。",
        "win-x64-setup": "Windows x64，安装程序", "win-x64-zip": "Windows x64，便携版",
        "win-arm-setup": "Windows ARM，安装程序", "win-arm-zip": "Windows ARM，便携版",
        "mac-arm-dmg": "macOS Apple Silicon（未签名）", "mac-x64-dmg": "macOS Intel（未签名）",
        "linux-x64-tar": "Linux x64，便携版", "linux-arm-tar": "Linux ARM，便携版",
        "deb-x64": "Debian/Ubuntu x64（需同时安装数据包）", "deb-arm": "Debian/Ubuntu ARM（需同时安装数据包）",
        "deb-data": "Debian/Ubuntu：数据包", "rpm-x64": "Fedora/openSUSE x64（需同时安装数据包）",
        "rpm-arm": "Fedora/openSUSE ARM（需同时安装数据包）", "rpm-data": "Fedora/openSUSE：数据包",
        "freebsd": "FreeBSD x64（实验性）", "sums": "校验和",
    },
}
FILES = {"en": "README.md", "ru": "README.ru.md", "de": "README.de.md", "zh": "README.zh.md"}
# a download URL is only ever one of GitHub's, so a crafted release cannot put another link into the READMEs
URL_OK = re.compile(r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/releases/[A-Za-z0-9_./%+~-]+")


def block(lang, release, repo, data_version):
    t = TEXT[lang]
    levels = f"https://github.com/{repo}/releases/tag/data-{data_version}"
    lines = [START]
    if release is None:
        lines.append(t["none"])
    else:
        assets = {}
        for a in release.get("assets", []):
            name, url = str(a.get("name", "")), str(a.get("browser_download_url", ""))
            if URL_OK.fullmatch(url):
                assets[name] = url
        lines.append(t["release"].format(tag=release["tag_name"], url=release["html_url"]))
        lines.append("")
        lines.append(t["head"])
        for key, pattern in ROWS:
            rx = re.compile(pattern.replace("<v>", r"[0-9][0-9A-Za-z.~+-]*"))
            found = sorted(n for n in assets if rx.fullmatch(n))
            if found:
                lines.append(f"| {t[key]} | [{found[0]}]({assets[found[0]]}) |")
    lines.append("")
    lines.append(t["levels"].format(dv=data_version, levels=levels))
    lines.append(END)
    return "\n".join(lines)


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    repo = sys.argv[2] if len(sys.argv) > 2 else "eugene-vereshchagin/drahtseilakt"
    release = None
    if sys.argv[1] != "none":
        release = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
        if not URL_OK.fullmatch(str(release.get("html_url", ""))) or not re.fullmatch(
            r"[0-9A-Za-z.~+-]+", str(release.get("tag_name", ""))
        ):
            sys.exit("update-download-table: the release has no usable tag or page")
    data_version = (ROOT / "scripts" / "DATA_VERSION").read_text(encoding="utf-8").strip()
    if not data_version.isdigit():
        sys.exit("update-download-table: scripts/DATA_VERSION is not a number")
    for lang, name in FILES.items():
        path = ROOT / name
        text = path.read_text(encoding="utf-8")
        if text.count(START) != 1 or text.count(END) != 1:
            sys.exit(f"update-download-table: {name} needs one {START} and one {END} line")
        head, rest = text.split(START, 1)
        _, tail = rest.split(END, 1)
        path.write_text(head + block(lang, release, repo, data_version) + tail, encoding="utf-8")
        print(f"{name}: written")


if __name__ == "__main__":
    main()
