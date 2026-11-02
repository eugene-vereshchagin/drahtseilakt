// Where levels live (docs/drahtseilakt/levels.md): the game's data folder and its `levels/` with four folders, the
// original game (the player's copy), community maps, remastered originals and open-format levels. Each can be moved
// in the settings (`levels.<kind> PATH`); the default folder then keeps a README that points to it. Every folder
// gets a README in the player's language saying what belongs there. `--import` copies the original from the player's
// installed game.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Original,
    Community,
    Remastered,
    Open,
}

pub const KINDS: [Kind; 4] = [
    Kind::Original,
    Kind::Community,
    Kind::Remastered,
    Kind::Open,
];

impl Kind {
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Original => "original",
            Kind::Community => "community",
            Kind::Remastered => "remastered",
            Kind::Open => "open",
        }
    }
}

/// The game's data folder: XDG_DATA_HOME or ~/.local/share on Linux, %APPDATA% on Windows,
/// ~/Library/Application Support on macOS; the current folder when none is known.
pub fn data_dir() -> PathBuf {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        // a relative XDG_DATA_HOME is ignored, as the XDG spec says
        var("XDG_DATA_HOME")
            .filter(|p| p.is_absolute())
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("drahtseilakt")
}

/// Where a kind's folder is by default.
pub fn default_dir(kind: Kind) -> PathBuf {
    data_dir().join("levels").join(kind.folder())
}

/// Where a kind's levels are: the folder the settings name (`levels.<kind>`), else the default.
pub fn dir(kind: Kind) -> PathBuf {
    crate::app::setting(&format!("levels.{}", kind.folder()))
        .filter(|p| !p.is_empty())
        .map_or_else(|| default_dir(kind), |p| resolve(&p))
}

/// A folder named in the settings: `~/` is the home folder, and a relative path is taken from the data folder (never
/// from wherever the game was started).
fn resolve(p: &str) -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let path = match (p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")), home) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(p),
    };
    if path.is_absolute() {
        path
    } else {
        data_dir().join(path)
    }
}

const README: &str = "README.txt";
/// The last line of a README that points to a moved folder, so that it alone is ever replaced.
const MOVED_MARK: &str = "[drahtseilakt: moved]\n";

/// Makes the folders and writes their READMEs in `language` where they have none yet; a moved folder's default place
/// gets a README that says where it went. Problems are reported, never fatal: the game plays without these folders.
pub fn prepare(language: &str) {
    for kind in KINDS {
        let (here, default) = (dir(kind), default_dir(kind));
        if here != default {
            write(&default, &moved(language, &here), true);
        }
        // a folder moved back replaces the README that pointed away (ours, with its marker), never anyone else's
        let readme = here.join(README);
        let small = std::fs::metadata(&readme).is_ok_and(|m| m.len() < 4096);
        let pointed =
            small && std::fs::read_to_string(&readme).is_ok_and(|t| t.ends_with(MOVED_MARK));
        write(&here, &about(language, kind), pointed);
    }
}

/// Writes `text` as the folder's README, making the folder; an existing README is replaced only when `replace`.
fn write(folder: &Path, text: &str, replace: bool) {
    let path = folder.join(README);
    if path.exists() && !replace {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(folder).and_then(|()| std::fs::write(&path, text)) {
        eprintln!("could not write {}: {e}", path.display());
    }
}

/// The README of a moved folder's default place.
fn moved(language: &str, to: &Path) -> String {
    let to = to.display();
    let text = match language {
        "ru" => {
            format!("Уровни этой папки теперь лежат здесь:\n{to}\n(настройка в settings.txt)\n")
        }
        "de" => format!(
            "Die Level dieses Ordners liegen jetzt hier:\n{to}\n(Einstellung in settings.txt)\n"
        ),
        "zh" => format!("此文件夹的关卡现在位于：\n{to}\n（在 settings.txt 中设置）\n"),
        _ => format!("The levels of this folder now live here:\n{to}\n(set in settings.txt)\n"),
    };
    text + MOVED_MARK
}

/// A folder's README: what belongs there and where to get it.
fn about(language: &str, kind: Kind) -> String {
    let text = match (language, kind) {
        ("ru", Kind::Original) => {
            "Оригинальная Ballance — ваша собственная копия.\n\nСкопируйте сюда из установленной игры base.cmo и папки \
             3D Entities, Textures и Sounds, или выполните:\n  drahtseilakt --import <папка Ballance>\nДля 13-го уровня \
             сначала установите дополнение SuDu в свою игру, как сказано в его readme.\n"
        }
        ("ru", Kind::Community) => {
            "Любительские карты (.nmo), играются в расширенном режиме (Options, Extended).\n\nГде брать: каталог \
             github.com/pseudoacetyl/BallanceCustomMaps (список карт и ссылки на файлы), Ballance International \
             Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nКарты запускаются как данные; моды и DLL игра никогда \
             не выполняет.\n"
        }
        ("ru", Kind::Remastered) => {
            "Оригинальные уровни в открытом формате, с современными текстурами (4K–8K).\n\nДелаются на вашей машине \
             из папки original скриптами игры (план работ, фазы 6 и 7) и никуда не распространяются.\n"
        }
        ("ru", Kind::Open) => {
            "Уровни в открытом формате: созданные в нём или сконвертированные любительские карты; каждый уровень — \
             папка с level.gltf.\n\nЧасть поставляется с игрой, с разрешения и под свободной лицензией авторов \
             (LICENSE в папке уровня).\n"
        }
        ("de", Kind::Original) => {
            "Das Original-Ballance — Ihre eigene Kopie.\n\nKopieren Sie aus dem installierten Spiel base.cmo und die \
             Ordner 3D Entities, Textures und Sounds hierher, oder führen Sie aus:\n  drahtseilakt --import \
             <Ballance-Ordner>\nFür Level 13 installieren Sie zuerst das SuDu-Add-on in Ihr Spiel, wie sein Readme \
             sagt.\n"
        }
        ("de", Kind::Community) => {
            "Community-Karten (.nmo), gespielt im erweiterten Modus (Options, Extended).\n\nWo es sie gibt: der \
             Katalog github.com/pseudoacetyl/BallanceCustomMaps (Kartenliste und Links zu den Dateien), der Ballance \
             International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nKarten laufen als Daten; Mods und DLLs \
             führt das Spiel nie aus.\n"
        }
        ("de", Kind::Remastered) => {
            "Die Original-Level im offenen Format, mit modernen Texturen (4K–8K).\n\nAuf Ihrem Rechner aus dem Ordner \
             original von den Skripten des Spiels erzeugt (Fahrplan, Phasen 6 und 7) und nie weitergegeben.\n"
        }
        ("de", Kind::Open) => {
            "Level im offenen Format: darin gebaut oder umgewandelte Community-Karten; jedes Level ist ein Ordner mit \
             level.gltf.\n\nEinige kommen mit dem Spiel, mit Erlaubnis und unter freier Lizenz ihrer Autoren (LICENSE \
             im Ordner des Levels).\n"
        }
        ("zh", Kind::Original) => {
            "原版 Ballance —— 你自己的游戏副本。\n\n从已安装的游戏中把 base.cmo 以及 3D Entities、Textures、Sounds \
             文件夹复制到这里，或者运行：\n  drahtseilakt --import <Ballance 文件夹>\n如需第 13 关，请先按照 SuDu \
             扩展包的 readme 把它安装到你的游戏中。\n"
        }
        ("zh", Kind::Community) => {
            "自制地图（.nmo），在扩展模式下游玩（Options，Extended）。\n\n获取途径：目录 \
             github.com/pseudoacetyl/BallanceCustomMaps（地图列表和文件链接）、Ballance International Discord \
             （#new）、百度贴吧 平衡球吧、哔哩哔哩。\n地图只作为数据运行；游戏从不执行模组或 DLL。\n"
        }
        ("zh", Kind::Remastered) => {
            "开放格式的原版关卡，配有现代纹理（4K–8K）。\n\n由游戏的脚本在你的电脑上从 original 文件夹生成（路线图第 6、7 \
             阶段），不会对外分发。\n"
        }
        ("zh", Kind::Open) => {
            "开放格式的关卡：直接以该格式制作的关卡或转换后的自制地图；每个关卡是一个含有 level.gltf 的文件夹。\n\n\
             部分关卡随游戏一起提供，需经作者许可并采用自由许可证（关卡文件夹中的 LICENSE）。\n"
        }
        (_, Kind::Original) => {
            "The original Ballance - your own copy.\n\nCopy base.cmo and the folders 3D Entities, Textures and Sounds of \
             the installed game here, or run:\n  drahtseilakt --import <Ballance folder>\nFor level 13, first install \
             the SuDu add-on into your game as its readme says.\n"
        }
        (_, Kind::Community) => {
            "Community maps (.nmo), played in the extended mode (Options, Extended).\n\nWhere to get them: the \
             catalogue github.com/pseudoacetyl/BallanceCustomMaps (the list of maps and links to their files), the \
             Ballance International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nMaps run as data; the game \
             never runs mods or DLLs.\n"
        }
        (_, Kind::Remastered) => {
            "The original levels in the open format, with modern textures (4K-8K).\n\nMade on your machine from the \
             original folder by the game's scripts (roadmap, phases 6 and 7) and never distributed.\n"
        }
        (_, Kind::Open) => {
            "Levels in the open format: made in it, or converted community maps; each level is a folder with \
             level.gltf.\n\nSome come with the game, with their authors' permission and under a free licence (LICENSE \
             in the level's folder).\n"
        }
    };
    text.to_owned()
}

/// What `--import` copies from an installed game: the files and folders the game reads.
const IMPORTED: [&str; 4] = ["base.cmo", "3D Entities", "Textures", "Sounds"];
/// Files one import copies at most, and folder depth it follows.
const MAX_FILES: usize = 100_000;
const MAX_DEPTH: usize = 16;

/// Copies the original game into the original levels' folder, from the installed copy at `from` or, when `from` is a
/// file, from the game's CD image (a .cue, .bin or .iso): what the game reads. Returns how many files were written.
pub fn import(from: &Path) -> Result<usize, String> {
    if from.is_file() {
        return import_cd(from);
    }
    let to = dir(Kind::Original);
    // neither inside the other, checked before anything is made: a copy into the game it copies from would nest
    let (a, b) = (canonical(from)?, canonical_to_be(&to));
    if a != b && (a.starts_with(&b) || b.starts_with(&a)) {
        return Err(format!(
            "{} and {} are inside one another",
            from.display(),
            to.display()
        ));
    }
    let found = |name: &str| crate::assets::find(from, name);
    if found("3D Entities").is_none() {
        return Err(format!(
            "{}: no Ballance here (no 3D Entities folder)",
            from.display()
        ));
    }
    let mut copied = 0usize;
    let mut seen = 0usize;
    for name in IMPORTED {
        let Some(src) = found(name) else {
            eprintln!("{}: no {name}", from.display());
            continue;
        };
        // the game's own folders may be links (some installs link them): those are followed, nothing below them
        let src = std::fs::canonicalize(&src).map_err(|e| format!("{}: {e}", src.display()))?;
        copy(&src, &to.join(name), 0, &mut seen, &mut copied)?;
    }
    Ok(copied)
}

/// A folder that may not exist yet as it will be: its nearest existing parent made canonical, the rest added.
fn canonical_to_be(p: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = p;
    loop {
        if let Ok(c) = std::fs::canonicalize(at) {
            return rest
                .iter()
                .rev()
                .fold(c, |acc: PathBuf, part| acc.join(part));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                at = parent;
            }
            _ => return p.to_path_buf(),
        }
    }
}

/// The game's files from its CD: the InstallShield archive in SETUP (DATA1.HDR and its volumes DATA<n>.CAB), the
/// files under the folders the game reads and base.cmo, each written to its place under the original levels' folder.
fn import_cd(image: &Path) -> Result<usize, String> {
    use cdimage::{iscab, iso};
    let to = dir(Kind::Original);
    let fail = |e: cdimage::Error| format!("{}: {e}", image.display());
    let mut img = cdimage::Image::open(image).map_err(fail)?;
    let hdr = iso::find(&mut img, "SETUP/DATA1.HDR")
        .map_err(fail)?
        .ok_or(format!(
            "{}: no SETUP/DATA1.HDR (not the Ballance CD?)",
            image.display()
        ))?;
    let cab = iscab::parse(&iso::read(&mut img, &hdr).map_err(fail)?).map_err(fail)?;
    let wanted = |parts: &[String]| match parts {
        [name] => name.eq_ignore_ascii_case("base.cmo"),
        [first, ..] => IMPORTED.iter().any(|w| w.eq_ignore_ascii_case(first)),
        [] => false,
    };
    let mut volume: Option<(u16, Vec<u8>)> = None;
    let mut written = 0usize;
    let mut total = 0u64;
    let mut files: Vec<&iscab::File> = cab.files.iter().filter(|f| f.stored()).collect();
    // by volume, so each volume is read once
    files.sort_by_key(|f| (f.volume, f.offset));
    for f in files {
        let Some(parts) = f.parts().filter(|p| wanted(p)) else {
            continue;
        };
        if volume.as_ref().is_none_or(|(n, _)| *n != f.volume) {
            let name = format!("SETUP/DATA{}.CAB", f.volume);
            let e = iso::find(&mut img, &name)
                .map_err(fail)?
                .ok_or(format!("{}: no {name}", image.display()))?;
            volume = Some((f.volume, iso::read(&mut img, &e).map_err(fail)?));
        }
        let Some((_, data)) = &volume else { continue };
        total = total.saturating_add(f.expanded);
        if total > iscab::TOTAL_MAX {
            return Err(format!(
                "{}: more data than the game has; not imported further",
                image.display()
            ));
        }
        let bytes = iscab::extract(f, data).map_err(fail)?;
        let dst = parts.iter().fold(to.clone(), |p, part| p.join(part));
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        // a link where the file goes is removed, never written through
        if std::fs::symlink_metadata(&dst).is_ok_and(|m| m.file_type().is_symlink()) {
            std::fs::remove_file(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        }
        std::fs::write(&dst, bytes).map_err(|e| format!("{}: {e}", dst.display()))?;
        written += 1;
    }
    if written == 0 {
        return Err(format!("{}: no game files in its archive", image.display()));
    }
    Ok(written)
}

fn canonical(p: &Path) -> Result<PathBuf, String> {
    std::fs::canonicalize(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn copy(
    src: &Path,
    dst: &Path,
    depth: usize,
    seen: &mut usize,
    copied: &mut usize,
) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", src.display());
    // links below the copy's top folders are not followed; one is reported
    let meta = std::fs::symlink_metadata(src).map_err(err)?;
    if meta.file_type().is_symlink() {
        eprintln!("{}: a link, not copied", src.display());
        return Ok(());
    }
    if meta.is_dir() {
        if depth >= MAX_DEPTH {
            return Err(format!("{}: folders nested too deep", src.display()));
        }
        std::fs::create_dir_all(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        let mut entries: Vec<_> = std::fs::read_dir(src).map_err(err)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            copy(&e.path(), &dst.join(e.file_name()), depth + 1, seen, copied)?;
        }
    } else if meta.is_file() {
        *seen += 1;
        if *seen > MAX_FILES {
            return Err(format!(
                "more than {MAX_FILES} files: not a Ballance folder"
            ));
        }
        // the same file: same size and not older than the source
        let dst_meta = std::fs::symlink_metadata(dst).ok();
        let newer = |d: &std::fs::Metadata| match (d.modified(), meta.modified()) {
            (Ok(d), Ok(s)) => d >= s,
            _ => true,
        };
        let same = dst_meta
            .as_ref()
            .is_some_and(|d| d.is_file() && d.len() == meta.len() && newer(d));
        if !same {
            // a link where the copy goes is removed, never written through
            if dst_meta.is_some_and(|d| d.file_type().is_symlink()) {
                std::fs::remove_file(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            std::fs::copy(src, dst)
                .map_err(|e| format!("{} -> {}: {e}", src.display(), dst.display()))?;
            *copied += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn every_folder_has_a_readme_in_every_language() {
        for l in ["en", "ru", "de", "zh", "fr"] {
            for k in KINDS {
                assert!(about(l, k).len() > 40, "{l} {k:?}");
            }
            assert!(moved(l, Path::new("/x")).contains("/x"));
        }
        assert_ne!(about("zh", Kind::Open), about("en", Kind::Open));
    }

    #[test]
    fn settings_paths_never_depend_on_where_the_game_started() {
        assert!(resolve("relative/maps").starts_with(data_dir()));
        assert!(resolve("/abs/maps") == Path::new("/abs/maps"));
        if let Some(h) = std::env::var_os("HOME") {
            assert_eq!(resolve("~/maps"), PathBuf::from(h).join("maps"));
        }
    }

    #[test]
    fn only_our_pointer_is_marked() {
        for l in ["en", "ru", "de", "zh"] {
            assert!(moved(l, Path::new("/x")).ends_with(MOVED_MARK));
            for k in KINDS {
                assert!(!about(l, k).ends_with(MOVED_MARK));
            }
        }
    }

    #[test]
    fn an_import_copies_what_the_game_reads_and_nothing_else() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-import-{}", std::process::id()));
        let game = root.join("game");
        std::fs::create_dir_all(game.join("3D Entities/Level")).unwrap();
        std::fs::create_dir_all(game.join("Bin")).unwrap();
        std::fs::write(game.join("base.cmo"), b"cmo").unwrap();
        std::fs::write(game.join("3D Entities/Level/Level_01.NMO"), b"nmo").unwrap();
        std::fs::write(game.join("Bin/Player.exe"), b"exe").unwrap();
        let to = root.join("original");
        let (mut seen, mut copied) = (0, 0);
        for name in IMPORTED {
            if let Some(src) = crate::assets::find(&game, name) {
                copy(&src, &to.join(name), 0, &mut seen, &mut copied).unwrap();
            }
        }
        assert_eq!(copied, 2);
        assert!(to.join("3D Entities/Level/Level_01.NMO").exists());
        assert!(!to.join("Bin").exists());
        let (mut seen, mut again) = (0, 0);
        copy(
            &game.join("base.cmo"),
            &to.join("base.cmo"),
            0,
            &mut seen,
            &mut again,
        )
        .unwrap();
        assert_eq!(again, 0, "the same file is not copied again");
        let _ = std::fs::remove_dir_all(&root);
    }
}
