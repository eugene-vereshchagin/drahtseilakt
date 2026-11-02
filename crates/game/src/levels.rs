// Where levels live (docs/drahtseilakt/levels.md): the game's data folder and its `levels/` with four folders, the
// original game (the player's copy), community maps, remastered originals and open-format levels. Each can be moved
// in the settings (`levels.<kind> PATH`); the default folder then keeps a README that points to it. Every folder
// gets a README in the player's language saying what belongs there. `--import` copies the original from the player's
// installed game.

use std::path::{Path, PathBuf};

pub use gamedata::profile::{KINDS, Kind, data_dir, default_dir, dir};

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
        // a folder moved back replaces the README that pointed away (ours, with its marker), and an earlier version
        // of ours is brought up to date; never anyone else's
        // read as a plain file of at most 4 KiB, never through a link
        let ours = gamedata::package::read_small(&here.join(README), 4096)
            .and_then(|b| String::from_utf8(b).ok())
            .is_some_and(|t| outdated(&t));
        write(&here, &about(language, kind), ours);
    }
}

/// A README of ours that a newer text replaces: one that pointed to a moved folder, or an earlier version's, untouched.
fn outdated(text: &str) -> bool {
    text.ends_with(MOVED_MARK) || SUPERSEDED.contains(&text)
}

/// Writes `text` as the folder's README, making the folder; an existing README is replaced only when `replace` and it
/// is a plain file. A README that is a link (even a dangling one) is never written through or replaced. The text is
/// written beside and renamed into place, so a README is never half written.
fn write(folder: &Path, text: &str, replace: bool) {
    let path = folder.join(README);
    match std::fs::symlink_metadata(&path) {
        Ok(m) if m.file_type().is_symlink() => {
            eprintln!("{}: a link, not written", path.display());
            return;
        }
        Ok(m) if !replace || !m.is_file() => return,
        _ => {}
    }
    let tmp = folder.join(format!(".{README}.{}.tmp", std::process::id()));
    let result = std::fs::create_dir_all(folder).and_then(|()| {
        // a leftover of an earlier run (a file or a link) is removed itself, never followed
        if std::fs::symlink_metadata(&tmp).is_ok() {
            std::fs::remove_file(&tmp)?;
        }
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        std::io::Write::write_all(&mut f, text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &path)
    });
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
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
            "Любительские карты, играются в расширенном режиме (Options, Extended).\n\nКаждый уровень — папка здесь \
             с одним уровнем: один файл .nmo или уровень в открытом формате (level.gltf, level.bin, textures/ — его \
             легко поправить). Рядом по желанию: level.txt со строками title = ..., author = ..., version = ... \
             (в меню показывается title, до 40 знаков; без него — имя папки), README и картинка preview.png или preview.jpg. \
             Одиночный .nmo прямо в этой папке — тоже уровень. Вложенные папки, прочие файлы и папки без уровня или с \
             несколькими меню не показывает. Скачанную коллекцию раскладывает так: drahtseilakt-toolbox import-maps \
             <папка>.\n\nГде брать: каталог github.com/pseudoacetyl/BallanceCustomMaps (список карт и ссылки на \
             файлы), Ballance International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nКарты запускаются как \
             данные; моды и DLL игра никогда не выполняет.\n"
        }
        ("ru", Kind::Remastered) => {
            "Оригинальные уровни в открытом формате, с современными текстурами (4K–8K).\n\nДелает их на вашей \
             машине из папки original программа drahtseilakt-toolbox (remaster); никуда не распространяются.\n"
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
            "Community-Karten, gespielt im erweiterten Modus (Options, Extended).\n\nJedes Level ist hier ein Ordner \
             mit genau einem Level: einer .nmo-Datei oder einem Level im offenen Format (level.gltf, level.bin, \
             textures/ - leicht nachzubessern). Daneben nach Wunsch: level.txt mit Zeilen title = ..., author = ..., \
             version = ... (das Menü zeigt title, bis 40 Zeichen; ohne ihn den Ordnernamen), ein README und ein Bild \
             preview.png oder preview.jpg. Eine einzelne .nmo direkt in diesem Ordner ist auch ein Level. Unterordner, \
             andere Dateien und Ordner ohne oder mit mehreren Levels zeigt das Menü nicht. Eine geladene Sammlung \
             sortiert drahtseilakt-toolbox import-maps <Ordner> so ein.\n\nWo es sie gibt: der Katalog \
             github.com/pseudoacetyl/BallanceCustomMaps (Kartenliste und Links zu den Dateien), der Ballance \
             International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nKarten laufen als Daten; Mods und DLLs \
             führt das Spiel nie aus.\n"
        }
        ("de", Kind::Remastered) => {
            "Die Original-Level im offenen Format, mit modernen Texturen (4K–8K).\n\nAuf Ihrem Rechner aus dem Ordner \
             original vom Programm drahtseilakt-toolbox (remaster) erzeugt und nie weitergegeben.\n"
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
            "自制地图，在扩展模式下游玩（Options，Extended）。\n\n每个关卡是这里的一个文件夹，只含一个关卡：一个 \
             .nmo 文件，或一个开放格式的关卡（level.gltf、level.bin、textures/，便于修改）。可选：level.txt，每行 \
             title = ...、author = ...、version = ...（菜单显示 title，最多 40 个字符；没有时显示文件夹名），README，\
             以及预览图 preview.png 或 preview.jpg。直接放在本文件夹里的单个 .nmo 也算一个关卡。子文件夹、其他文件、\
             没有关卡或有多个关卡的文件夹不会显示。下载的合集可以用 drahtseilakt-toolbox import-maps <文件夹> \
             整理成这样。\n\n获取途径：目录 github.com/pseudoacetyl/BallanceCustomMaps（地图列表和文件链接）、\
             Ballance International Discord（#new）、百度贴吧 平衡球吧、哔哩哔哩。\n地图只作为数据运行；游戏从不执行\
             模组或 DLL。\n"
        }
        ("zh", Kind::Remastered) => {
            "开放格式的原版关卡，配有现代纹理（4K–8K）。\n\n由 drahtseilakt-toolbox（remaster）在你的电脑上从 original \
             文件夹生成，不会对外分发。\n"
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
            "Community maps, played in the extended mode (Options, Extended).\n\nEach level is a folder here holding \
             one level: one .nmo file, or a level in the open format (level.gltf, level.bin, textures/ - easy to fix \
             up). Beside it, if you like: level.txt with lines title = ..., author = ..., version = ... (the menu \
             shows the title, up to 40 characters; without one, the folder's name), a README and a picture preview.png or \
             preview.jpg. A single .nmo put straight into this folder is a level too. Folders inside a level's folder, \
             other files, and folders with no level or several are not shown. drahtseilakt-toolbox import-maps \
             <folder> sorts a downloaded collection into this shape.\n\nWhere to get them: the catalogue \
             github.com/pseudoacetyl/BallanceCustomMaps (the list of maps and links to their files), the Ballance \
             International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nMaps run as data; the game never runs \
             mods or DLLs.\n"
        }
        (_, Kind::Remastered) => {
            "The original levels in the open format, with modern textures (4K-8K).\n\nMade on your machine from the \
             original folder by the program drahtseilakt-toolbox (remaster) and never distributed.\n"
        }
        (_, Kind::Open) => {
            "Levels in the open format: made in it, or converted community maps; each level is a folder with \
             level.gltf.\n\nSome come with the game, with their authors' permission and under a free licence (LICENSE \
             in the level's folder).\n"
        }
    };
    text.to_owned()
}

/// The READMEs earlier versions of the game wrote (community and remastered, every language): one of these, untouched,
/// is replaced by the current text; any other README is the player's and stays.
const SUPERSEDED: [&str; 8] = [
    "Любительские карты (.nmo), играются в расширенном режиме (Options, Extended).\n\nГде брать: каталог \
             github.com/pseudoacetyl/BallanceCustomMaps (список карт и ссылки на файлы), Ballance International \
             Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nКарты запускаются как данные; моды и DLL игра никогда \
             не выполняет.\n",
    "Оригинальные уровни в открытом формате, с современными текстурами (4K–8K).\n\nДелаются на вашей машине \
             из папки original скриптами игры (план работ, фазы 6 и 7) и никуда не распространяются.\n",
    "Community-Karten (.nmo), gespielt im erweiterten Modus (Options, Extended).\n\nWo es sie gibt: der \
             Katalog github.com/pseudoacetyl/BallanceCustomMaps (Kartenliste und Links zu den Dateien), der Ballance \
             International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nKarten laufen als Daten; Mods und DLLs \
             führt das Spiel nie aus.\n",
    "Die Original-Level im offenen Format, mit modernen Texturen (4K–8K).\n\nAuf Ihrem Rechner aus dem Ordner \
             original von den Skripten des Spiels erzeugt (Fahrplan, Phasen 6 und 7) und nie weitergegeben.\n",
    "自制地图（.nmo），在扩展模式下游玩（Options，Extended）。\n\n获取途径：目录 \
             github.com/pseudoacetyl/BallanceCustomMaps（地图列表和文件链接）、Ballance International Discord \
             （#new）、百度贴吧 平衡球吧、哔哩哔哩。\n地图只作为数据运行；游戏从不执行模组或 DLL。\n",
    "开放格式的原版关卡，配有现代纹理（4K–8K）。\n\n由游戏的脚本在你的电脑上从 original 文件夹生成（路线图第 6、7 \
             阶段），不会对外分发。\n",
    "Community maps (.nmo), played in the extended mode (Options, Extended).\n\nWhere to get them: the \
             catalogue github.com/pseudoacetyl/BallanceCustomMaps (the list of maps and links to their files), the \
             Ballance International Discord (#new), Baidu Tieba (平衡球吧), Bilibili.\nMaps run as data; the game \
             never runs mods or DLLs.\n",
    "The original levels in the open format, with modern textures (4K-8K).\n\nMade on your machine from the \
             original folder by the game's scripts (roadmap, phases 6 and 7) and never distributed.\n",
];

/// What a folder or file found on the machine is, if it is a Ballance to import.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    /// An installed game (a `3D Entities` folder).
    Installed,
    /// The game's CD, in its drive or copied off it (`SETUP/DATA1.HDR`).
    Disc,
    /// An image of the CD (`.cue` or `.iso`).
    Image,
}

pub fn what(p: &Path) -> Option<Found> {
    let ext = |e: &str| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(e));
    if p.is_file() {
        return (ext("cue") || ext("iso") || ext("bin")).then_some(Found::Image);
    }
    if crate::assets::find(p, "3D Entities").is_some() {
        return Some(Found::Installed);
    }
    let setup = crate::assets::find(p, "SETUP").and_then(|d| crate::assets::find(&d, "DATA1.HDR"));
    setup.map(|_| Found::Disc)
}

/// Where a Ballance may be on this machine: installed games (in the usual folders of Windows, of Wine prefixes and of
/// the GOG, Heroic and Lutris launchers), discs in their drives, and CD images in the home and download folders.
pub fn candidates() -> Vec<(PathBuf, Found)> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let mut parents: Vec<PathBuf> = Vec::new();
    let mut discs: Vec<PathBuf> = Vec::new();
    let subdirs = |d: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(d)
            .map(|r| {
                r.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .take(64)
                    .collect()
            })
            .unwrap_or_default()
    };
    if cfg!(windows) {
        for drive in 'C'..='Z' {
            let root = PathBuf::from(format!("{drive}:\\"));
            if !root.exists() {
                continue;
            }
            discs.push(root.clone());
            for p in ["Program Files", "Program Files (x86)", "GOG Games", "Games"] {
                parents.push(root.join(p));
            }
        }
    } else {
        if let Some(h) = &home {
            for prefix in [".wine/drive_c", "Games/Heroic/Prefixes/default/drive_c"] {
                for p in ["Program Files", "Program Files (x86)", "GOG Games"] {
                    parents.push(h.join(prefix).join(p));
                }
            }
            parents.push(h.join("Games"));
            parents.push(h.join("Games/Heroic"));
            parents.push(h.join("GOG Games"));
        }
        let user = std::env::var("USER").unwrap_or_default();
        for mounts in ["/media", "/mnt", "/Volumes"]
            .map(PathBuf::from)
            .into_iter()
            .chain([
                PathBuf::from("/media").join(&user),
                PathBuf::from("/run/media").join(&user),
            ])
        {
            discs.extend(subdirs(&mounts));
        }
    }
    let mut out: Vec<(PathBuf, Found)> = Vec::new();
    let mut add = |p: PathBuf| {
        if let Some(f) = what(&p)
            && !out.iter().any(|(q, _)| *q == p)
            && out.len() < 64
        {
            out.push((p, f));
        }
    };
    for parent in parents {
        for name in [
            "Ballance",
            "Atari/Ballance",
            "GOG Galaxy/Games/Ballance",
            "ballance",
        ] {
            add(parent.join(name));
        }
    }
    for d in discs {
        add(d);
    }
    if let Some(h) = &home {
        for d in [h.clone(), h.join("Downloads"), h.join("Desktop")] {
            let files = std::fs::read_dir(&d)
                .map(|r| r.flatten().map(|e| e.path()).take(4096).collect::<Vec<_>>());
            for f in files.unwrap_or_default() {
                let name = f
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                // a .bin is listed only without a cue sheet next to it (the sheet names it)
                let bin = f.extension().is_some_and(|e| e.eq_ignore_ascii_case("bin"));
                let sheet = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cue"));
                let has_cue = bin
                    && std::fs::read_dir(&d)
                        .is_ok_and(|r| r.flatten().take(4096).any(|e| sheet(&e.path())));
                if name.contains("ballance") && !has_cue {
                    add(f);
                }
            }
        }
    }
    out
}

/// What `--import` copies from an installed game: the files and folders the game reads.
const IMPORTED: [&str; 4] = ["base.cmo", "3D Entities", "Textures", "Sounds"];
/// Files one import copies at most, and folder depth it follows.
const MAX_FILES: usize = 100_000;
const MAX_DEPTH: usize = 16;

/// Copies the original game into the original levels' folder, from the installed copy at `from` or, when `from` is a
/// file, from the game's CD image (a .cue, .bin or .iso): what the game reads. Returns how many files were written.
pub fn import(from: &Path) -> Result<usize, String> {
    let n = import_from(from)?;
    // what was copied must play: a folder with an empty 3D Entities, or an archive cut short, is not the game
    let level = ["3D Entities", "Level", "Level_01.NMO"]
        .iter()
        .try_fold(dir(Kind::Original), |d, p| crate::assets::find(&d, p));
    if level.is_none() {
        return Err(format!(
            "{}: no level 1 came out of it; not the game",
            from.display()
        ));
    }
    Ok(n)
}

fn import_from(from: &Path) -> Result<usize, String> {
    if from.is_file() {
        return import_cd(from);
    }
    // a CD in its drive, or a copy of its files: the archive in SETUP/, not an installed game
    let setup =
        crate::assets::find(from, "SETUP").and_then(|d| crate::assets::find(&d, "DATA1.HDR"));
    if crate::assets::find(from, "3D Entities").is_none() && setup.is_some() {
        return import_disc_folder(from);
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
    use cdimage::iso;
    let fail = |e: cdimage::Error| format!("{}: {e}", image.display());
    let mut img = cdimage::Image::open(image).map_err(fail)?;
    let mut read = |name: &str| -> Result<Option<Vec<u8>>, String> {
        match iso::find(&mut img, name).map_err(fail)? {
            Some(e) => iso::read(&mut img, &e).map(Some).map_err(fail),
            None => Ok(None),
        }
    };
    import_archive(&mut read, image)
}

/// The game's files from a CD's own folders (a disc in its drive, or its files copied off it).
fn import_disc_folder(disc: &Path) -> Result<usize, String> {
    let mut read = |name: &str| -> Result<Option<Vec<u8>>, String> {
        let Some(path) = name
            .split('/')
            .try_fold(disc.to_path_buf(), |d, part| crate::assets::find(&d, part))
        else {
            return Ok(None);
        };
        // read at most one byte past the cap: a file larger than a CD's is refused, however it grows meanwhile
        let cap = cdimage::image::READ_MAX as u64;
        let mut bytes = Vec::new();
        let file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::io::Read::read_to_end(&mut std::io::Read::take(file, cap + 1), &mut bytes)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if bytes.len() as u64 > cap {
            return Err(format!(
                "{}: too big for a file of the game's CD",
                path.display()
            ));
        }
        Ok(Some(bytes))
    };
    import_archive(&mut read, disc)
}

/// Gives a file of the disc by its path, `None` when the disc has none.
type DiscReader<'a> = dyn FnMut(&str) -> Result<Option<Vec<u8>>, String> + 'a;

/// The game's files from the InstallShield archive of its CD, `read` giving a file of the disc by its path
/// ("SETUP/DATA1.HDR"): those under the folders the game reads and base.cmo, each written to its place under the
/// original levels' folder.
fn import_archive(read: &mut DiscReader<'_>, disc: &Path) -> Result<usize, String> {
    use cdimage::iscab;
    let to = dir(Kind::Original);
    let image = disc;
    let fail = |e: cdimage::Error| format!("{}: {e}", image.display());
    let hdr = read("SETUP/DATA1.HDR")?.ok_or(format!(
        "{}: no SETUP/DATA1.HDR (not the Ballance CD?)",
        image.display()
    ))?;
    let cab = iscab::parse(&hdr).map_err(fail)?;
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
            let data = read(&name)?.ok_or(format!("{}: no {name}", image.display()))?;
            volume = Some((f.volume, data));
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
    use gamedata::profile::resolve;
    use super::*;

    #[test]
    fn what_is_found_is_told_by_its_contents() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-what-{}", std::process::id()));
        std::fs::create_dir_all(root.join("game/3D Entities")).unwrap();
        std::fs::create_dir_all(root.join("disc/Setup")).unwrap();
        std::fs::write(root.join("disc/Setup/data1.hdr"), b"").unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();
        std::fs::write(root.join("b.CUE"), b"").unwrap();
        std::fs::write(root.join("b.txt"), b"").unwrap();
        assert_eq!(what(&root.join("game")), Some(Found::Installed));
        assert_eq!(what(&root.join("disc")), Some(Found::Disc));
        assert_eq!(what(&root.join("b.CUE")), Some(Found::Image));
        assert_eq!(what(&root.join("other")), None);
        assert_eq!(what(&root.join("b.txt")), None);
        let _ = std::fs::remove_dir_all(&root);
    }

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
        // absolute as each system writes it ("/abs" has no drive on Windows: not absolute there)
        let abs = if cfg!(windows) {
            "C:\\abs\\maps"
        } else {
            "/abs/maps"
        };
        assert!(resolve(abs) == Path::new(abs));
        if let Some(h) = std::env::var_os("HOME") {
            assert_eq!(resolve("~/maps"), PathBuf::from(h).join("maps"));
        }
    }

    #[test]
    fn earlier_readmes_are_brought_up_to_date_and_players_ones_kept() {
        for l in ["en", "ru", "de", "zh"] {
            for k in KINDS {
                assert!(!outdated(&about(l, k)), "{l} {k:?}: the current text is not replaced again");
            }
        }
        assert!(SUPERSEDED.iter().all(|t| outdated(t) && t.len() < 4096));
        assert!(SUPERSEDED.iter().any(|t| t.contains("game's scripts")));
        assert!(!outdated("my own notes\n"));
        assert!(!outdated(&format!("{}extra\n", SUPERSEDED[0])));
        assert!(about("en", Kind::Community).contains("level.txt"));
        assert!(about("en", Kind::Remastered).contains("drahtseilakt-toolbox"));
    }

    #[cfg(unix)]
    #[test]
    fn a_readme_that_is_a_link_is_never_written_through() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-readme-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (folder, outside) = (root.join("levels"), root.join("outside"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        // a dangling link: writing through it would make a file outside
        std::os::unix::fs::symlink(outside.join("made.txt"), folder.join(README)).unwrap();
        write(&folder, "text", true);
        assert!(!outside.join("made.txt").exists());
        assert!(std::fs::symlink_metadata(folder.join(README)).unwrap().file_type().is_symlink());
        // a link to an existing file: neither it nor its target is changed
        std::fs::write(outside.join("target.txt"), "theirs").unwrap();
        std::fs::remove_file(folder.join(README)).unwrap();
        std::os::unix::fs::symlink(outside.join("target.txt"), folder.join(README)).unwrap();
        write(&folder, "text", true);
        assert_eq!(std::fs::read_to_string(outside.join("target.txt")).unwrap(), "theirs");
        // a plain README: written atomically, replaced only when asked, no temporary file left
        let plain = root.join("plain");
        write(&plain, "one", false);
        write(&plain, "two", false);
        assert_eq!(std::fs::read_to_string(plain.join(README)).unwrap(), "one");
        write(&plain, "three", true);
        assert_eq!(std::fs::read_to_string(plain.join(README)).unwrap(), "three");
        assert_eq!(std::fs::read_dir(&plain).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&root);
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
