//! macOS 구현(T-50·T-51 A·T-52): 셸 탐지 · 열기/보기 · **휴지통(`NSFileManager trashItemAtURL` = Finder 되돌리기와 같은 길 · 복원 = 이번 세션 기록으로 `moveItem` · 실패 = `~/.Trash` 이동 폴백)** · **드라이브 용량**(`statvfs`).
//! 파일 클립보드 = `macclip.rs`(NSPasteboard) · 폴더 감시 = `macwatch.rs`(kqueue) · PTY = `unixpty.rs` · NSDragging은 T-52 잔여.

use super::*;

pub(super) struct NativeShell;

impl Shell for NativeShell {
    /// `$SHELL` → `/bin/zsh` → `/bin/bash` → `/bin/sh`(SKEL-425).
    fn candidates(&self) -> Vec<ShellSpec> {
        let mut names: Vec<(String, &[&str], &str)> = Vec::new();
        if let Some(sh) = std::env::var_os("SHELL") {
            names.push((sh.to_string_lossy().into_owned(), &["-l"], "$SHELL"));
        }
        names.push(("/bin/zsh".into(), &["-l"], "zsh"));
        names.push(("/bin/bash".into(), &["-l"], "bash"));
        names.push(("/bin/sh".into(), &[], "sh"));
        let owned: Vec<(&str, &[&str], &str)> =
            names.iter().map(|(n, a, l)| (n.as_str(), *a, *l)).collect();
        let mut v = shell_from_names(&owned);
        v.dedup_by(|a, b| a.program == b.program);
        v
    }
}

/// `open` · `open -R`(Finder에서 보기).
pub(super) fn opener() -> CommandOpener {
    CommandOpener {
        open: ["open", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        reveal: ["open", "-R", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

/// `~/.Trash`로 이동(같은 볼륨만 · 이름 충돌 = 숫자 꼬리). Finder "되돌리기" 정보는 T-52(`NSFileManager trashItemAtURL`).
pub(super) struct HomeTrash {
    base: PathBuf,
}

impl HomeTrash {
    pub(super) fn new() -> Self {
        let base = std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".Trash"))
            .unwrap_or_else(|| PathBuf::from("/tmp/.Trash"));
        HomeTrash { base }
    }

    #[cfg(test)]
    fn at(base: PathBuf) -> Self {
        HomeTrash { base }
    }
}

impl Trash for HomeTrash {
    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
        std::fs::create_dir_all(&self.base).map_err(|e| PlatformError::Failed(e.to_string()))?;
        let mut n = 0;
        for p in paths {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| PlatformError::Failed(format!("no file name: {}", p.display())))?;
            let mut target = name.clone();
            let mut k = 1;
            while self.base.join(&target).exists() {
                k += 1;
                target = format!("{name} {k}");
            }
            std::fs::rename(p, self.base.join(&target))
                .map_err(|e| PlatformError::Failed(format!("{}: {e}", p.display())))?;
            n += 1;
        }
        Ok(n)
    }
}

/// 시스템 휴지통(T-52 · dir2 SHELL-049 복원 대응): `trashItemAtURL:resultingItemURL:`가 돌려주는 휴지통 안 경로를 **원래 경로 → 휴지통 경로**로 기억해
/// `restore`(삭제 undo)가 `moveItemAtURL`로 되돌린다(이번 프로세스가 버린 것만 · Finder가 비우면 실패 = `Failed`). AppKit 아님(Foundation) — 어느 스레드든.
pub(super) struct SystemTrash {
    home: HomeTrash,
    trashed: RefCell<HashMap<PathBuf, PathBuf>>,
}

impl SystemTrash {
    pub(super) fn new() -> Self {
        SystemTrash {
            home: HomeTrash::new(),
            trashed: RefCell::new(HashMap::new()),
        }
    }

    /// 항목 하나를 시스템 휴지통으로 — `Ok(휴지통 안 경로)`(경로를 못 받았으면 `None`) · `Err` = 오류를 돌려줌.
    fn trash_one(p: &Path) -> Result<Option<PathBuf>, ()> {
        use objc2_foundation::{NSFileManager, NSString, NSURL};
        // SAFETY: Foundation 호출 — 살아 있는 NSURL · 출력 슬롯은 Option.
        unsafe {
            let url = NSURL::fileURLWithPath(&NSString::from_str(&p.to_string_lossy()));
            let mut out: Option<objc2::rc::Retained<NSURL>> = None;
            NSFileManager::defaultManager()
                .trashItemAtURL_resultingItemURL_error(&url, Some(&mut out))
                .map_err(|_| ())?;
            Ok(out.and_then(|u| u.path().map(|s| PathBuf::from(s.to_string()))))
        }
    }
}

impl Trash for SystemTrash {
    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
        let mut n = 0;
        let mut fallback: Vec<PathBuf> = Vec::new();
        for p in paths {
            // 원본이 사라졌으면 옮겨진 것이다(결과 경로가 없거나 오류를 돌려줘도) — 폴백은 원본이 남아 있을 때만.
            let api = Self::trash_one(p);
            match super::trash_outcome(api, p.symlink_metadata().is_ok()) {
                super::TrashOutcome::Trashed(t) => {
                    if let Some(t) = t {
                        self.trashed.borrow_mut().insert(p.clone(), t);
                    }
                    n += 1;
                }
                super::TrashOutcome::Fallback => fallback.push(p.clone()),
            }
        }
        if !fallback.is_empty() {
            n += self.home.trash(&fallback)?;
        }
        Ok(n)
    }

    fn restore(&self, original: &[PathBuf]) -> Result<usize, PlatformError> {
        use objc2_foundation::{NSFileManager, NSString, NSURL};
        let mut n = 0;
        for o in original {
            let Some(t) = self.trashed.borrow().get(o).cloned() else {
                continue;
            };
            if !t.exists() {
                return Err(PlatformError::Failed(format!(
                    "gone from trash: {}",
                    o.display()
                )));
            }
            if let Some(parent) = o.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            // SAFETY: Foundation 호출 — 두 URL 모두 살아 있다.
            let moved = unsafe {
                NSFileManager::defaultManager().moveItemAtURL_toURL_error(
                    &NSURL::fileURLWithPath(&NSString::from_str(&t.to_string_lossy())),
                    &NSURL::fileURLWithPath(&NSString::from_str(&o.to_string_lossy())),
                )
            };
            match moved {
                Ok(()) => {
                    self.trashed.borrow_mut().remove(o);
                    n += 1;
                }
                Err(e) => {
                    return Err(PlatformError::Failed(format!("{}: {e:?}", o.display())));
                }
            }
        }
        if n == 0 && !original.is_empty() {
            return Err(PlatformError::Unsupported("trash.restore"));
        }
        Ok(n)
    }
}

/// Darwin `struct statvfs`(`fsblkcnt_t`/`fsfilcnt_t` = u32 · `unsigned long` = u64).
#[repr(C)]
struct StatVfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u32,
    f_bfree: u32,
    f_bavail: u32,
    f_files: u32,
    f_ffree: u32,
    f_favail: u32,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
}

extern "C" {
    fn statvfs(path: *const std::os::raw::c_char, buf: *mut StatVfs) -> i32;
}

pub(super) struct NativeDisk;

impl Disk for NativeDisk {
    fn space(&self, root: &Path) -> Option<(u64, u64)> {
        let c = std::ffi::CString::new(root.as_os_str().as_encoded_bytes()).ok()?;
        let mut st = std::mem::MaybeUninit::<StatVfs>::zeroed();
        // SAFETY: NUL 종단 경로 · 출력 구조체는 Darwin 레이아웃과 같다 · 실패 = -1.
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
            u64::from(st.f_blocks).saturating_mul(unit),
            u64::from(st.f_bavail).saturating_mul(unit),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_trash_moves_with_suffix_and_disk_space_works() {
        let base = std::env::temp_dir().join(format!("ndir-mactrash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"1").unwrap();
        let t = HomeTrash::at(base.join(".Trash"));
        assert_eq!(t.trash(&[src.join("a.txt")]), Ok(1));
        std::fs::write(src.join("a.txt"), b"2").unwrap();
        assert_eq!(t.trash(&[src.join("a.txt")]), Ok(1));
        assert!(base.join(".Trash/a.txt 2").is_file());
        assert!(NativeDisk
            .space(Path::new("/"))
            .is_some_and(|(t, f)| t >= f && t > 0));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 시스템 휴지통 왕복(macOS 러너): 임시 파일 → trashItemAtURL(원본 사라짐 · 기록) → restore(원래 경로로 복원) · 모르는 경로 복원 = Unsupported.
    /// 실제 시스템 휴지통을 쓰므로 **CI에서만** 돈다(규약: 시험은 사용자 휴지통을 건드리지 않는다 — 개발 PC는 건너뜀).
    #[test]
    fn system_trash_round_trip() {
        if std::env::var_os("CI").is_none() {
            return;
        }
        let _g = crate::platform::os_test_guard();
        let dir = std::env::temp_dir().join(format!("ndir-mactrash-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("restore-me.txt");
        std::fs::write(&f, b"restore").unwrap();
        let t = SystemTrash::new();
        assert_eq!(t.trash(std::slice::from_ref(&f)), Ok(1));
        assert!(!f.exists(), "원본은 휴지통으로");
        // 휴지통 안 경로를 받았을 때만 복원할 수 있다(못 받으면 기록이 없어 Unsupported — 러너에 따라 다르다).
        match t.restore(std::slice::from_ref(&f)) {
            Ok(1) => assert_eq!(std::fs::read(&f).unwrap(), b"restore"),
            other => assert_eq!(other, Err(PlatformError::Unsupported("trash.restore"))),
        }
        assert_eq!(
            t.restore(&[dir.join("never.txt")]),
            Err(PlatformError::Unsupported("trash.restore"))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
