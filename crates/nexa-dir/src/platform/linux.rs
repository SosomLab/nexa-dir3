//! Linux 구현(T-50·T-51 A): 셸 탐지 · 열기/보기 · **freedesktop 휴지통**(`$XDG_DATA_HOME/Trash` · `.trashinfo` · std만) · **드라이브 용량**(`statvfs` 수동 extern).
//! uri-list 클립보드 · XDND · inotify · openpty는 T-53.

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

pub(super) struct NativeDisk;

impl Disk for NativeDisk {
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
