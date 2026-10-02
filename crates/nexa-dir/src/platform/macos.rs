//! macOS 구현(T-50·T-51 A): 셸 탐지 · 열기/보기 · **휴지통(`~/.Trash`로 이동 · 되돌리기 정보는 T-52 `trashItem`)** · **드라이브 용량**(`statvfs`).
//! NSPasteboard 파일 URL · NSDragging · FSEvents · forkpty는 T-52.

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
}
