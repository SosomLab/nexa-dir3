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

/// freedesktop Trash 규격 — `files/<이름>` + `info/<이름>.trashinfo`(원래 경로 · 삭제 시각). 다른 장치(rename 실패)는 `Failed`(복사 삭제는 M6 ops).
pub(super) struct FreedesktopTrash {
    base: PathBuf,
}

impl FreedesktopTrash {
    pub(super) fn new() -> Self {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("Trash");
        FreedesktopTrash { base }
    }

    /// 시험용 — 임시 폴더를 휴지통으로.
    #[cfg(test)]
    fn at(base: PathBuf) -> Self {
        FreedesktopTrash { base }
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
    fn restore(&self, original: &[PathBuf]) -> Result<usize, PlatformError> {
        let files = self.base.join("files");
        let info = self.base.join("info");
        let Ok(rd) = std::fs::read_dir(&info) else {
            return Ok(0);
        };
        let mut wanted: Vec<PathBuf> = original.to_vec();
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
        Ok(n)
    }

    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
        let files = self.base.join("files");
        let info = self.base.join("info");
        std::fs::create_dir_all(&files).map_err(|e| PlatformError::Failed(e.to_string()))?;
        std::fs::create_dir_all(&info).map_err(|e| PlatformError::Failed(e.to_string()))?;
        let mut n = 0;
        for p in paths {
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
            let abs = if p.is_absolute() {
                p.clone()
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            };
            let when = crate::filelist::format_iso_utc(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64),
            );
            let body = format!(
                "[Trash Info]\nPath={}\nDeletionDate={when}\n",
                percent_encode(&abs.to_string_lossy())
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
            XdgState {
                mime,
                apps: super::xdgapps::MimeApps::build(&lists, &caches),
                app_dirs,
                archiver,
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
        if self.state().archiver.is_some() {
            out.push(item("xdg.compress".into(), ndir_i18n::tr("ctx.compress")));
        }
        out.push(item("xdg.props".into(), ndir_i18n::tr("ctx.properties")));
        Ok(out)
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
            "xdg.props" => {
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
                    "org.freedesktop.FileManager1.ShowItemProperties".into(),
                    format!("[{}]", uris.join(",")),
                    String::new(),
                ])
            }
            other => Err(PlatformError::Failed(format!("unknown menu item: {other}"))),
        }
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
        if let Some(first) = items.first().filter(|i| i.id != "xdg.props") {
            assert!(
                first.id.starts_with("xdg.app:") || first.id == "xdg.compress",
                "{}",
                first.id
            );
        }
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
}
