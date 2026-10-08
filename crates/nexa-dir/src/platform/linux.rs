//! Linux 구현(T-50·T-51 A): 셸 탐지 · 열기/보기 · **freedesktop 휴지통**(`$XDG_DATA_HOME/Trash` · `.trashinfo` · std만) · **드라이브 용량**(`statvfs` 수동 extern).
//! 파일 클립보드 = `X11Files`(`clipboard_x11` uri-list · gnome-copied-files · kde cut) · inotify 감시 = `linuxwatch.rs` · PTY = `unixpty.rs` · XDND는 T-53 잔여.

use super::*;

pub(super) struct NativeShell;

impl Shell for NativeShell {
    /// `$SHELL` → `/bin/bash` → `/bin/sh`(SKEL-425).
    fn candidates(&self) -> Vec<ShellSpec> {
        let mut names: Vec<(String, &[&str], &str)> = Vec::new();
        if let Some(sh) = std::env::var_os("SHELL") {
            names.push((sh.to_string_lossy().into_owned(), &["-l"], "$SHELL"));
        }
        names.push(("/bin/bash".into(), &["-l"], "bash"));
        names.push(("/bin/sh".into(), &[], "sh"));
        let owned: Vec<(&str, &[&str], &str)> =
            names.iter().map(|(n, a, l)| (n.as_str(), *a, *l)).collect();
        let mut v = shell_from_names(&owned);
        v.dedup_by(|a, b| a.program == b.program);
        v
    }
}

/// `xdg-open`(보기는 부모 폴더 열기 — 파일 관리자별 선택 지원은 T-53).
pub(super) fn opener() -> CommandOpener {
    CommandOpener {
        open: ["xdg-open", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        reveal: ["xdg-open", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

/// freedesktop Trash 규격 — `files/<이름>` + `info/<이름>.trashinfo`(원래 경로 · 삭제 시각).
///
/// 휴지통 자리(규격 §"Trash directories" · 10-08 Linux GAP): 홈 휴지통(`$XDG_DATA_HOME/Trash`)과 **같은 파일 시스템**이면 거기로 ·
/// 다른 파일 시스템(tmpfs `/tmp` · 다른 디스크 · USB)이면 그 마운트 최상위의 `$topdir/.Trash/$uid`(관리자가 만든 sticky 공용) 또는
/// `$topdir/.Trash-$uid`(없으면 만든다). rename은 같은 장치 안에서만 되므로(EXDEV) 이 분기가 없으면 `Invalid cross-device link`로 실패했다
/// (10-08 이 PC의 자가 점검 trash 2건 FAIL · CI 러너는 /tmp가 같은 fs라 못 잡았다). 그래도 못 옮기면 `Failed`(복사 삭제는 ops 몫).
pub(super) struct FreedesktopTrash {
    base: PathBuf,
    /// 시험용 — `Some`이면 마운트 최상위 탐색 대신 이 폴더를 topdir로 본다(홈과 다른 장치처럼 다룬다).
    #[cfg(test)]
    fake_topdir: Option<PathBuf>,
}

/// 홈 휴지통인가 · topdir 휴지통인가 — 순수 판정(MC/DC 시험): 같은 장치면 홈 · 아니면 공용 `.Trash/$uid`가 쓸 수 있을 때 그것 ·
/// 아니면 `.Trash-$uid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TrashSite {
    Home,
    Shared,
    PerUser,
}

pub(super) fn trash_site(same_dev_as_home: bool, shared_dot_trash_ok: bool) -> TrashSite {
    if same_dev_as_home {
        TrashSite::Home
    } else if shared_dot_trash_ok {
        TrashSite::Shared
    } else {
        TrashSite::PerUser
    }
}

extern "C" {
    fn getuid() -> u32;
}

fn dev_of(p: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(p).ok().map(|m| m.dev())
}

/// `path` 또는 가장 가까운 있는 조상의 장치.
fn dev_nearest(path: &Path) -> Option<u64> {
    let mut p = path;
    loop {
        if let Some(d) = dev_of(p) {
            return Some(d);
        }
        p = p.parent()?;
    }
}

/// `path`가 놓인 파일 시스템의 마운트 최상위 — 부모로 올라가며 st_dev가 같은 마지막 폴더(규격의 `$topdir`).
pub(super) fn topdir_of(path: &Path) -> PathBuf {
    let mut start = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map_or_else(|| PathBuf::from("/"), Path::to_path_buf)
    };
    while !start.exists() {
        match start.parent() {
            Some(p) => start = p.to_path_buf(),
            None => return start,
        }
    }
    let Some(dev) = dev_of(&start) else {
        return start;
    };
    let mut cur = start;
    while let Some(parent) = cur.parent() {
        if dev_of(parent) != Some(dev) {
            break;
        }
        cur = parent.to_path_buf();
    }
    cur
}

impl FreedesktopTrash {
    pub(super) fn new() -> Self {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("Trash");
        FreedesktopTrash {
            base,
            #[cfg(test)]
            fake_topdir: None,
        }
    }

    /// 시험용 — 임시 폴더를 휴지통으로.
    #[cfg(test)]
    fn at(base: PathBuf) -> Self {
        FreedesktopTrash {
            base,
            fake_topdir: None,
        }
    }

    /// 홈 휴지통이 놓인 장치(폴더가 아직 없으면 가장 가까운 있는 부모).
    fn home_dev(&self) -> Option<u64> {
        let mut p = self.base.as_path();
        loop {
            if let Some(d) = dev_of(p) {
                return Some(d);
            }
            p = p.parent()?;
        }
    }

    /// `path`를 버릴 휴지통 폴더(`files/` · `info/`의 부모)와, topdir 휴지통이면 그 topdir(trashinfo의 `Path=`를 상대로 쓴다).
    fn site_for(&self, path: &Path) -> (PathBuf, Option<PathBuf>) {
        #[cfg(test)]
        if let Some(top) = &self.fake_topdir {
            return (
                top.join(format!(".Trash-{}", unsafe { getuid() })),
                Some(top.clone()),
            );
        }
        // 되돌릴 때는 원래 경로가 이미 없다 — 가장 가까운 있는 조상의 장치로 판정한다.
        let same = match (self.home_dev(), dev_nearest(path)) {
            (Some(h), Some(d)) => h == d,
            _ => true,
        };
        if same {
            return (self.base.clone(), None);
        }
        let top = topdir_of(path);
        let uid = unsafe { getuid() };
        // 공용 `.Trash`는 규격대로 심볼릭 링크가 아니고 sticky 비트가 있는 폴더일 때만 쓴다.
        let shared = top.join(".Trash");
        let shared_ok = std::fs::symlink_metadata(&shared).is_ok_and(|m| {
            use std::os::unix::fs::PermissionsExt;
            m.is_dir() && !m.file_type().is_symlink() && m.permissions().mode() & 0o1000 != 0
        });
        match trash_site(false, shared_ok) {
            TrashSite::Home => (self.base.clone(), None),
            TrashSite::Shared => (shared.join(uid.to_string()), Some(top)),
            TrashSite::PerUser => (top.join(format!(".Trash-{uid}")), Some(top)),
        }
    }

    /// 되돌릴 때 뒤질 휴지통들 — 홈 + 원래 경로들의 topdir 휴지통(중복 제거).
    fn restore_sites(&self, original: &[PathBuf]) -> Vec<(PathBuf, Option<PathBuf>)> {
        let mut out = vec![(self.base.clone(), None)];
        for p in original {
            let site = self.site_for(p);
            if !out.iter().any(|s| s.0 == site.0) {
                out.push(site);
            }
        }
        out
    }

    /// 한 휴지통에서 `wanted`를 되돌린다(찾은 것은 `wanted`에서 뺀다) — 되돌린 수.
    fn restore_in(base: &Path, topdir: Option<&Path>, wanted: &mut Vec<PathBuf>) -> usize {
        let files = base.join("files");
        let info = base.join("info");
        let Ok(rd) = std::fs::read_dir(&info) else {
            return 0;
        };
        let mut n = 0;
        for e in rd.flatten() {
            if wanted.is_empty() {
                break;
            }
            let ip = e.path();
            let Some(stem) = ip
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|f| f.strip_suffix(".trashinfo"))
            else {
                continue;
            };
            let Ok(body) = std::fs::read_to_string(&ip) else {
                continue;
            };
            let Some(orig) = body.lines().find_map(|l| l.strip_prefix("Path=")) else {
                continue;
            };
            let orig = PathBuf::from(percent_decode(orig.trim()));
            // topdir 휴지통의 `Path=`는 topdir 기준 상대 경로(규격) — 절대 경로로 적힌 것도 받아 준다.
            let orig = match topdir {
                Some(top) if orig.is_relative() => top.join(orig),
                _ => orig,
            };
            let Some(idx) = wanted.iter().position(|w| *w == orig) else {
                continue;
            };
            if orig.exists() {
                wanted.remove(idx);
                continue;
            }
            let src = files.join(stem);
            if std::fs::rename(&src, &orig).is_ok() {
                let _ = std::fs::remove_file(&ip);
                wanted.remove(idx);
                n += 1;
            }
        }
        n
    }
}

/// `.trashinfo`의 Path 값 — RFC 2396 식 퍼센트 인코딩(`/`는 그대로).
/// `.trashinfo` `Path=` 디코드(percent-encoding · 손상 = 그대로).
pub(super) fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(super) fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl Trash for FreedesktopTrash {
    /// `info/*.trashinfo`의 `Path=`가 원래 경로와 같은 항목을 `files/`에서 되돌린다(원위치에 이미 있으면 건너뜀) · info 삭제.
    /// 홈 휴지통과 원래 경로가 놓인 장치의 topdir 휴지통을 차례로 뒤진다.
    fn restore(&self, original: &[PathBuf]) -> Result<usize, PlatformError> {
        let mut wanted: Vec<PathBuf> = original.to_vec();
        let mut n = 0;
        for (base, top) in self.restore_sites(original) {
            if wanted.is_empty() {
                break;
            }
            n += Self::restore_in(&base, top.as_deref(), &mut wanted);
        }
        Ok(n)
    }

    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
        let mut n = 0;
        for p in paths {
            let abs = if p.is_absolute() {
                p.clone()
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            };
            let (base, top) = self.site_for(&abs);
            let files = base.join("files");
            let info = base.join("info");
            std::fs::create_dir_all(&files).map_err(|e| PlatformError::Failed(e.to_string()))?;
            std::fs::create_dir_all(&info).map_err(|e| PlatformError::Failed(e.to_string()))?;
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| PlatformError::Failed(format!("no file name: {}", p.display())))?;
            // 이름 충돌 = 숫자 꼬리(규격 권고).
            let mut target = name.clone();
            let mut k = 1;
            while files.join(&target).exists() || info.join(format!("{target}.trashinfo")).exists()
            {
                k += 1;
                target = format!("{name}.{k}");
            }
            let when = crate::filelist::format_iso_utc(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64),
            );
            // topdir 휴지통의 `Path=`는 topdir 기준 상대 경로(규격 · 다른 파일 관리자와 호환).
            let recorded = match &top {
                Some(t) => abs.strip_prefix(t).map_or(abs.clone(), Path::to_path_buf),
                None => abs.clone(),
            };
            let body = format!(
                "[Trash Info]\nPath={}\nDeletionDate={when}\n",
                percent_encode(&recorded.to_string_lossy())
            );
            std::fs::write(info.join(format!("{target}.trashinfo")), body)
                .map_err(|e| PlatformError::Failed(e.to_string()))?;
            if let Err(e) = std::fs::rename(p, files.join(&target)) {
                let _ = std::fs::remove_file(info.join(format!("{target}.trashinfo")));
                return Err(PlatformError::Failed(format!("{}: {e}", p.display())));
            }
            n += 1;
        }
        Ok(n)
    }
}

/// glibc x86_64/aarch64 `struct statvfs`(64비트 필드 · `__f_spare[6]`).
#[repr(C)]
struct StatVfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    _spare: [i32; 6],
}

extern "C" {
    fn statvfs(path: *const std::os::raw::c_char, buf: *mut StatVfs) -> i32;
}

/// X11 CLIPBOARD 파일 목록(T-53 · docs/port/19 §4-4): 소유자 스레드가 `text/uri-list`·`x-special/gnome-copied-files`·`application/x-kde-cutselection`을
/// 같이 게시 · 읽기는 gnome → uri-list(+kde). 설정 `clipboard.x11_native`가 꺼졌거나 X 연결이 없으면(Wayland 전용 세션) `Unsupported`(앱 내 사본만).
/// Linux 우클릭 메뉴 항목 공급(1차 · 사용자 10-03 "리눅스의 우클릭 기능을 윈도우처럼 통합"): 노틸러스 메뉴 자체는 가져올 수
/// 없어, 같은 표준 정보로 같은 항목을 만든다 — **기본 앱으로 열기 · 다른 앱으로 열기 ▸ · 압축… · 속성**(해석 = `xdgapps.rs`).
/// 표는 처음 물을 때 한 번 읽는다. 폴더의 "열기"는 앱 안 이동이 따로 있으므로 폴더에는 기본 앱 항목을 내지 않는다.
#[derive(Default)]
pub(super) struct XdgMenu {
    state: std::cell::OnceCell<XdgState>,
}

struct XdgState {
    mime: nexa_fs::icontheme::MimeDb,
    apps: super::xdgapps::MimeApps,
    /// `.desktop`을 찾을 폴더들(`applications/` — 앞이 우선).
    app_dirs: Vec<PathBuf>,
    /// 압축 도구 `(프로그램, 앞 인자)` — 설치된 첫 번째.
    archiver: Option<(PathBuf, &'static [&'static str])>,
    /// 터미널 에뮬레이터(T-131 2차 · `$TERMINAL` → `x-terminal-emulator` → 알려진 것 순) — "터미널에서 열기".
    terminal: Option<PathBuf>,
    /// `xdg-email`(전자메일로 보내기) · `xdg-open`(파일 관리자에서 열기) 유무.
    has_email: bool,
    has_open: bool,
}

/// 터미널 에뮬레이터를 `dir`에서 여는 인자(순수 · T-131): 프로그램 이름별 작업 폴더 옵션 — 모르는 것은 옵션 없이(호출자가
/// `current_dir`로 대신한다 · xterm 등).
pub(super) fn terminal_argv(prog: &Path, dir: &Path) -> Vec<String> {
    let name = prog
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let d = dir.to_string_lossy().into_owned();
    let mut v = vec![prog.to_string_lossy().into_owned()];
    match name.as_str() {
        "gnome-terminal" | "xfce4-terminal" | "mate-terminal" | "tilix" | "terminator"
        | "alacritty" | "foot" => {
            v.push(format!("--working-directory={d}"));
        }
        "konsole" | "qterminal" => {
            v.push("--workdir".into());
            v.push(d);
        }
        "kitty" | "ghostty" => {
            v.push(format!("--directory={d}"));
        }
        "wezterm" => {
            v.push("start".into());
            v.push("--cwd".into());
            v.push(d);
        }
        "ptyxis" => {
            v.push("--working-directory".into());
            v.push(d);
        }
        _ => {}
    }
    v
}

/// "다른 앱으로 열기" 하위 메뉴에 넣는 앱 수 상한.
const OPEN_WITH_MAX: usize = 12;

fn xdg_data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut v = Vec::new();
    if let Some(d) = std::env::var_os("XDG_DATA_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".local/share")))
    {
        v.push(d);
    }
    let sys = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    v.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    v
}

impl XdgMenu {
    fn state(&self) -> &XdgState {
        self.state.get_or_init(|| {
            let data = xdg_data_dirs();
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
            let read_all = |name: &str| -> String {
                data.iter()
                    .filter_map(|d| std::fs::read_to_string(d.join("mime").join(name)).ok())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let mime = nexa_fs::icontheme::MimeDb::parse(
                &read_all("globs2"),
                &read_all("icons"),
                &read_all("generic-icons"),
            );
            let app_dirs: Vec<PathBuf> = data.iter().map(|d| d.join("applications")).collect();
            // mimeapps.list 우선순위: 사용자 설정 → 각 applications 폴더의 데스크톱별 목록 → 일반 목록.
            let desktops: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .split(':')
                .filter(|s| !s.is_empty())
                .map(str::to_lowercase)
                .collect();
            let mut lists: Vec<String> = Vec::new();
            let mut push = |p: PathBuf| {
                if let Ok(t) = std::fs::read_to_string(p) {
                    lists.push(t);
                }
            };
            for dir in config.iter().chain(app_dirs.iter()) {
                for d in &desktops {
                    push(dir.join(format!("{d}-mimeapps.list")));
                }
                push(dir.join("mimeapps.list"));
            }
            let caches: Vec<String> = app_dirs
                .iter()
                .filter_map(|d| std::fs::read_to_string(d.join("mimeinfo.cache")).ok())
                .collect();
            let archiver = [
                ("file-roller", &["--add"][..]),
                ("ark", &["--add", "--dialog"][..]),
                ("engrampa", &["--add"][..]),
                ("xarchiver", &["--compress"][..]),
            ]
            .into_iter()
            .find_map(|(name, args)| which(name).map(|p| (p, args)));
            let terminal = std::env::var_os("TERMINAL")
                .filter(|t| !t.is_empty())
                .and_then(|t| {
                    let p = PathBuf::from(&t);
                    if p.is_absolute() {
                        p.is_file().then_some(p)
                    } else {
                        which(&t.to_string_lossy())
                    }
                })
                .or_else(|| {
                    [
                        "x-terminal-emulator",
                        "gnome-terminal",
                        "ptyxis",
                        "konsole",
                        "xfce4-terminal",
                        "kitty",
                        "alacritty",
                        "foot",
                        "wezterm",
                        "tilix",
                        "mate-terminal",
                        "qterminal",
                        "xterm",
                    ]
                    .into_iter()
                    .find_map(which)
                });
            XdgState {
                mime,
                apps: super::xdgapps::MimeApps::build(&lists, &caches),
                app_dirs,
                archiver,
                terminal,
                has_email: which("xdg-email").is_some(),
                has_open: which("xdg-open").is_some(),
            }
        })
    }

    /// `.desktop` id → 앱(이름 · 실행 줄) — 숨김 항목은 `None`.
    fn app(&self, id: &str) -> Option<super::xdgapps::DesktopApp> {
        let st = self.state();
        let lang = ndir_i18n::current_code(); // 앱 이름도 이 프로그램의 표시 언어로(`Name[ko]`)
        st.app_dirs
            .iter()
            .find_map(|d| std::fs::read_to_string(d.join(id)).ok())
            .and_then(|t| super::xdgapps::parse_desktop_app(&t, &lang))
            .filter(|a| !a.hidden)
    }

    /// 경로의 MIME(폴더 = `inode/directory` · 이름으로 판정 · 모르면 일반 파일).
    fn mime_of(&self, path: &Path) -> String {
        if path.is_dir() {
            return "inode/directory".to_string();
        }
        path.file_name()
            .and_then(|n| {
                self.state()
                    .mime
                    .mime_of(&n.to_string_lossy())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "application/octet-stream".to_string())
    }
}

/// `PATH`에서 실행 파일 찾기.
fn which(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn spawn_detached(argv: &[String]) -> Result<(), PlatformError> {
    let Some((prog, rest)) = argv.split_first() else {
        return Err(PlatformError::Failed("empty command".into()));
    };
    std::process::Command::new(prog)
        .args(rest)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| PlatformError::Failed(format!("{prog}: {e}")))
}

impl ContextMenuProvider for XdgMenu {
    fn items(&self, paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
        let Some(first) = paths.first() else {
            return Ok(Vec::new());
        };
        let item = |id: String, label: String| ShellMenuItem {
            id,
            label,
            enabled: true,
            ..Default::default()
        };
        let mime = self.mime_of(first);
        let is_dir = mime == "inode/directory";
        let apps: Vec<(String, String)> = self
            .state()
            .apps
            .apps_for(&mime)
            .iter()
            .filter_map(|id| self.app(id).map(|a| (id.clone(), a.name)))
            .take(OPEN_WITH_MAX + 1)
            .collect();
        let mut out = Vec::new();
        // 파일: 기본 앱 = 첫 항목("‹앱›(으)로 열기") · 나머지 = "다른 앱으로 열기 ▸". 폴더: 전부 하위 메뉴(열기 = 앱 안 이동).
        let mut rest = apps.as_slice();
        if !is_dir {
            if let Some(((id, name), others)) = apps.split_first() {
                out.push(item(
                    format!("xdg.app:{id}"),
                    ndir_i18n::trf("ctx.openWithApp", &[name]),
                ));
                rest = others;
            }
        }
        if !rest.is_empty() {
            out.push(ShellMenuItem {
                id: "xdg.openwith".into(),
                label: ndir_i18n::tr("ctx.openWith"),
                enabled: true,
                children: rest
                    .iter()
                    .take(OPEN_WITH_MAX)
                    .map(|(id, name)| item(format!("xdg.app:{id}"), name.clone()))
                    .collect(),
                ..Default::default()
            });
        }
        if !out.is_empty() {
            out.push(ShellMenuItem {
                separator: true,
                ..Default::default()
            });
        }
        // T-131 2차: 폴더 1개 = 터미널에서 열기 · 파일만 = 전자메일로 보내기(xdg-email) · 항상 = 파일 관리자에서 보기(FileManager1.ShowItems).
        let st = self.state();
        if is_dir && paths.len() == 1 && st.terminal.is_some() {
            out.push(item(
                "xdg.terminal".into(),
                ndir_i18n::tr("ctx.openTerminal"),
            ));
        }
        if st.has_email && paths.iter().all(|p| p.is_file()) {
            out.push(item("xdg.email".into(), ndir_i18n::tr("ctx.sendEmail")));
        }
        out.push(item(
            "xdg.showin".into(),
            ndir_i18n::tr("ctx.showInFileManager"),
        ));
        if st.archiver.is_some() {
            out.push(item("xdg.compress".into(), ndir_i18n::tr("ctx.compress")));
        }
        out.push(item("xdg.props".into(), ndir_i18n::tr("ctx.properties")));
        Ok(out)
    }

    /// 폴더 배경 메뉴(T-131 2차): 터미널에서 열기 · 파일 관리자에서 열기(xdg-open) · 속성.
    fn bg_items(&self, _dir: &Path) -> Result<Vec<ShellMenuItem>, PlatformError> {
        let item = |id: &str, label: String| ShellMenuItem {
            id: id.into(),
            label,
            enabled: true,
            ..Default::default()
        };
        let st = self.state();
        let mut out = Vec::new();
        if st.terminal.is_some() {
            out.push(item("xdg.terminal", ndir_i18n::tr("ctx.openTerminal")));
        }
        if st.has_open {
            out.push(item(
                "xdg.filemanager",
                ndir_i18n::tr("ctx.openFileManager"),
            ));
        }
        out.push(item("xdg.props", ndir_i18n::tr("ctx.properties")));
        Ok(out)
    }

    fn invoke_bg(&self, id: &str, dir: &Path) -> Result<Option<PathBuf>, PlatformError> {
        match id {
            "xdg.terminal" => self.open_terminal(dir).map(|()| None),
            "xdg.filemanager" => {
                spawn_detached(&["xdg-open".into(), dir.to_string_lossy().into_owned()])
                    .map(|()| None)
            }
            "xdg.props" => self
                .show_properties(std::slice::from_ref(&dir.to_path_buf()))
                .map(|()| None),
            other => Err(PlatformError::Failed(format!(
                "not a background item: {other}"
            ))),
        }
    }

    fn invoke(&self, id: &str, paths: &[PathBuf]) -> Result<(), PlatformError> {
        if let Some(app_id) = id.strip_prefix("xdg.app:") {
            let app = self
                .app(app_id)
                .ok_or_else(|| PlatformError::Failed(format!("no such app: {app_id}")))?;
            return spawn_detached(&super::xdgapps::exec_argv(&app.exec, paths));
        }
        match id {
            "xdg.compress" => {
                let (prog, args) = self
                    .state()
                    .archiver
                    .as_ref()
                    .ok_or(PlatformError::Unsupported("archiver"))?;
                let mut argv = vec![prog.to_string_lossy().into_owned()];
                argv.extend(args.iter().map(|a| (*a).to_string()));
                argv.extend(paths.iter().map(|p| p.to_string_lossy().into_owned()));
                spawn_detached(&argv)
            }
            // 파일 관리자의 속성 창(freedesktop FileManager1 D-Bus — 노틸러스 · 돌핀 · 네모 등이 구현).
            "xdg.props" => self.show_properties(paths),
            "xdg.showin" => self.file_manager1("ShowItems", paths),
            "xdg.terminal" => match paths {
                [one] => self.open_terminal(one),
                _ => Err(PlatformError::Failed("terminal needs one folder".into())),
            },
            "xdg.email" => {
                let mut argv = vec!["xdg-email".to_string()];
                for p in paths {
                    argv.push("--attach".into());
                    argv.push(p.to_string_lossy().into_owned());
                }
                spawn_detached(&argv)
            }
            other => Err(PlatformError::Failed(format!("unknown menu item: {other}"))),
        }
    }
}

impl XdgMenu {
    /// `org.freedesktop.FileManager1.<method>(uris, startup_id)` — gdbus로(의존 0).
    fn file_manager1(&self, method: &str, paths: &[PathBuf]) -> Result<(), PlatformError> {
        let uris: Vec<String> = paths
            .iter()
            .map(|p| format!("'file://{}'", percent_encode(&p.to_string_lossy())))
            .collect();
        spawn_detached(&[
            "gdbus".into(),
            "call".into(),
            "--session".into(),
            "--dest".into(),
            "org.freedesktop.FileManager1".into(),
            "--object-path".into(),
            "/org/freedesktop/FileManager1".into(),
            "--method".into(),
            format!("org.freedesktop.FileManager1.{method}"),
            format!("[{}]", uris.join(",")),
            String::new(),
        ])
    }

    fn show_properties(&self, paths: &[PathBuf]) -> Result<(), PlatformError> {
        self.file_manager1("ShowItemProperties", paths)
    }

    /// 터미널 에뮬레이터를 `dir`에서 연다(작업 폴더 옵션은 [`terminal_argv`] · 그 밖은 `current_dir`).
    fn open_terminal(&self, dir: &Path) -> Result<(), PlatformError> {
        let prog = self
            .state()
            .terminal
            .clone()
            .ok_or(PlatformError::Unsupported("terminal"))?;
        let argv = terminal_argv(&prog, dir);
        std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|e| PlatformError::Failed(format!("{}: {e}", argv[0])))
    }
}

pub(super) struct X11Files;

impl FileClipboard for X11Files {
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)> {
        if !crate::settings_clip_native() {
            return None;
        }
        crate::clipboard_x11::read_files()
    }
    fn write_files(&self, paths: &[PathBuf], cut: bool) -> Result<(), PlatformError> {
        if crate::settings_clip_native() && crate::clipboard_x11::write_files(paths, cut) {
            Ok(())
        } else {
            Err(PlatformError::Unsupported("x11 file clipboard"))
        }
    }
}

pub(super) struct NativeDisk;

impl Disk for NativeDisk {
    fn size_on_disk(&self, path: &Path) -> Option<u64> {
        super::unix_size_on_disk(path)
    }

    fn volumes_stamp(&self) -> u64 {
        super::unix_volumes_stamp()
    }

    fn space(&self, root: &Path) -> Option<(u64, u64)> {
        let c = std::ffi::CString::new(root.as_os_str().as_encoded_bytes()).ok()?;
        let mut st = std::mem::MaybeUninit::<StatVfs>::zeroed();
        // SAFETY: NUL 종단 경로 · 출력 구조체는 glibc 레이아웃과 같다(64비트 리눅스) · 실패 = -1.
        let rc = unsafe { statvfs(c.as_ptr(), st.as_mut_ptr()) };
        if rc != 0 {
            return None;
        }
        // SAFETY: 성공했으므로 초기화됨.
        let st = unsafe { st.assume_init() };
        let unit = if st.f_frsize > 0 {
            st.f_frsize
        } else {
            st.f_bsize
        };
        Some((
            st.f_blocks.saturating_mul(unit),
            st.f_bavail.saturating_mul(unit),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Linux 메뉴 공급(이 PC의 앱 연결 — 내용은 환경마다 다르다): 속성은 늘 있고 · 파일의 첫 항목이 앱이면 `xdg.app:` ·
    /// 폴더에는 기본 앱 항목을 내지 않는다(열기 = 앱 안 이동) · 모르는 id = 오류 · 빈 선택 = 빈 목록. 실행은 하지 않는다.
    #[test]
    fn xdg_menu_lists_apps_and_properties() {
        let dir = std::env::temp_dir().join(format!("ndir-xdgmenu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.txt");
        std::fs::write(&file, b"x").unwrap();
        let m = XdgMenu::default();
        assert!(m.items(&[]).unwrap().is_empty());
        let items = m.items(std::slice::from_ref(&file)).unwrap();
        eprintln!(
            "file menu: {:?}",
            items.iter().map(|i| (&i.id, &i.label)).collect::<Vec<_>>()
        );
        assert_eq!(items.last().map(|i| i.id.as_str()), Some("xdg.props"));
        assert!(items.iter().all(|i| i.separator || !i.label.is_empty()));
        // 첫 항목 = 기본 앱(있으면) · 없으면 T-131 2차 항목(전자메일 · 파일 관리자에서 보기) 또는 압축(CI 러너 = 기본 앱 연결 없음).
        if let Some(first) = items.first().filter(|i| i.id != "xdg.props") {
            assert!(
                first.id.starts_with("xdg.app:")
                    || matches!(
                        first.id.as_str(),
                        "xdg.email" | "xdg.showin" | "xdg.compress"
                    ),
                "{}",
                first.id
            );
        }
        // 파일 메뉴: 파일 관리자에서 보기는 늘 · 터미널은 폴더에만.
        assert!(items.iter().any(|i| i.id == "xdg.showin"));
        assert!(items.iter().all(|i| i.id != "xdg.terminal"));
        let ditems = m.items(std::slice::from_ref(&dir)).unwrap();
        eprintln!(
            "dir menu: {:?}",
            ditems.iter().map(|i| (&i.id, &i.label)).collect::<Vec<_>>()
        );
        assert!(
            ditems.iter().all(|i| !i.id.starts_with("xdg.app:")),
            "폴더 = 하위 메뉴에만"
        );
        assert_eq!(m.mime_of(&dir), "inode/directory");
        assert!(m.invoke("xdg.nope", &[file]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn percent_encoding_keeps_slashes() {
        assert_eq!(percent_encode("/home/u/a b.txt"), "/home/u/a%20b.txt");
        assert_eq!(percent_encode("한"), "%ED%95%9C");
    }

    /// 임시 휴지통: files/ + info/ 생성 · 이름 충돌 꼬리 · trashinfo 내용.
    #[test]
    fn trash_moves_and_writes_info() {
        let base = std::env::temp_dir().join(format!("ndir-trash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"1").unwrap();
        let t = FreedesktopTrash::at(base.join("Trash"));
        assert_eq!(t.trash(&[src.join("a.txt")]), Ok(1));
        assert!(base.join("Trash/files/a.txt").is_file());
        let info = std::fs::read_to_string(base.join("Trash/info/a.txt.trashinfo")).unwrap();
        assert!(info.starts_with("[Trash Info]\nPath=") && info.contains("DeletionDate="));
        std::fs::write(src.join("a.txt"), b"2").unwrap();
        assert_eq!(t.trash(&[src.join("a.txt")]), Ok(1));
        assert!(base.join("Trash/files/a.txt.2").is_file());
        assert!(t.trash(&[src.join("nope")]).is_err());
        assert!(NativeDisk
            .space(Path::new("/"))
            .is_some_and(|(t, f)| t >= f && t > 0));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 터미널 작업 폴더 인자(T-131): 알려진 에뮬레이터별 옵션 · 모르는 것은 프로그램만.
    #[test]
    fn terminal_argv_by_program() {
        let d = Path::new("/tmp/x y");
        assert_eq!(
            terminal_argv(Path::new("/usr/bin/gnome-terminal"), d),
            vec!["/usr/bin/gnome-terminal", "--working-directory=/tmp/x y"]
        );
        assert_eq!(
            terminal_argv(Path::new("konsole"), d),
            vec!["konsole", "--workdir", "/tmp/x y"]
        );
        assert_eq!(
            terminal_argv(Path::new("/opt/wezterm"), d),
            vec!["/opt/wezterm", "start", "--cwd", "/tmp/x y"]
        );
        assert_eq!(terminal_argv(Path::new("xterm"), d), vec!["xterm"]);
    }

    /// 배경 메뉴(T-131): 속성은 늘 · 터미널/파일 관리자는 설치됐을 때 — 항목 id가 `invoke_bg`가 아는 것뿐.
    #[test]
    fn xdg_bg_menu_items_are_invokable_ids() {
        let m = XdgMenu::default();
        let items = m.bg_items(Path::new("/")).unwrap();
        assert!(items.iter().any(|i| i.id == "xdg.props"));
        for i in &items {
            assert!(
                matches!(
                    i.id.as_str(),
                    "xdg.terminal" | "xdg.filemanager" | "xdg.props"
                ),
                "{}",
                i.id
            );
        }
        assert!(m.invoke_bg("nope", Path::new("/")).is_err());
    }

    /// 휴지통 자리 판정 MC/DC: 같은 장치 → 홈 · 다른 장치 + 공용 OK → `.Trash/$uid` · 다른 장치 + 공용 없음 → `.Trash-$uid`.
    #[test]
    fn trash_site_decision() {
        assert_eq!(trash_site(true, true), TrashSite::Home);
        assert_eq!(trash_site(true, false), TrashSite::Home);
        assert_eq!(trash_site(false, true), TrashSite::Shared);
        assert_eq!(trash_site(false, false), TrashSite::PerUser);
        // topdir 탐색: `/`의 topdir은 `/` · 실제 폴더의 topdir은 그 조상이다.
        assert_eq!(topdir_of(Path::new("/")), PathBuf::from("/"));
        let tmp = std::env::temp_dir();
        assert!(tmp.starts_with(topdir_of(&tmp)));
    }

    /// 다른 장치(가짜 topdir): `$topdir/.Trash-$uid/files` 로 옮기고 trashinfo `Path=`는 topdir 기준 상대 경로 · restore가 topdir 휴지통도 뒤져
    /// 원위치로 되돌린다(10-08 tmpfs `/tmp` EXDEV 결함 회귀).
    #[test]
    fn trash_topdir_round_trip() {
        let base = std::env::temp_dir().join(format!("ndir-trash-top-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let top = base.join("mnt");
        let src = top.join("docs");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"1").unwrap();
        let mut t = FreedesktopTrash::at(base.join("HomeTrash"));
        t.fake_topdir = Some(top.clone());
        assert_eq!(t.trash(&[src.join("a.txt")]), Ok(1));
        let uid = unsafe { getuid() };
        let tdir = top.join(format!(".Trash-{uid}"));
        assert!(tdir.join("files/a.txt").is_file());
        assert!(!base.join("HomeTrash").exists());
        let info = std::fs::read_to_string(tdir.join("info/a.txt.trashinfo")).unwrap();
        assert!(info.contains("\nPath=docs/a.txt\n"), "{info}");
        assert_eq!(t.restore(&[src.join("a.txt")]), Ok(1));
        assert_eq!(std::fs::read(src.join("a.txt")).unwrap(), b"1");
        assert!(!tdir.join("info/a.txt.trashinfo").exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
