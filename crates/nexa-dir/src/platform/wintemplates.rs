//! Windows "새로 만들기" 템플릿(SHELL-008 · dir2 `CLSID_NewMenu` 호스팅 → **자체 그림**이라 레지스트리를 직접 읽는다 · docs/port/19 §4 "새로 만들기 ▸"):
//! `HKCR\.ext\ShellNew` 또는 `HKCR\.ext\<ProgId>\ShellNew`의 `NullFile`(빈 파일) · `FileName`(템플릿 복사 — 절대 경로 아니면 사용자
//! Templates 폴더 → `%WINDIR%\ShellNew`) · `Data`(바이트). `Command`/`Handler`만 있는 것(바로가기 · 압축 폴더 등 셸 핸들러)은 뺀다.
//! 라벨 = OS 종류 이름(`nexa_fs::shell::kind_name` · 없으면 확장자 대문자). 첫 조회 때 한 번 열거해 캐시(수십 ms) + 사용자 템플릿 폴더 합류.

use super::*;
use ::windows::core::PCWSTR;
use ::windows::Win32::Foundation::ERROR_SUCCESS;
use ::windows::Win32::System::Com::CoTaskMemFree;
use ::windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_READ,
    REG_VALUE_TYPE,
};
use ::windows::Win32::UI::Shell::{FOLDERID_Templates, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn open(parent: HKEY, sub: &str) -> Option<Key> {
    let w = wide(sub);
    let mut h = HKEY::default();
    let rc = unsafe { RegOpenKeyExW(parent, PCWSTR(w.as_ptr()), None, KEY_READ, &mut h) };
    (rc == ERROR_SUCCESS).then_some(Key(h))
}

fn subkeys(k: HKEY) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0u32;
    loop {
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        let rc = unsafe {
            RegEnumKeyExW(
                k,
                i,
                Some(::windows::core::PWSTR(buf.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
        };
        if rc != ERROR_SUCCESS {
            break;
        }
        out.push(String::from_utf16_lossy(&buf[..len as usize]));
        i += 1;
    }
    out
}

/// 값(종류 · 바이트) — 없으면 None.
fn value(k: HKEY, name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
    let w = wide(name);
    let mut ty = REG_VALUE_TYPE(0);
    let mut len = 0u32;
    let rc = unsafe {
        RegQueryValueExW(
            k,
            PCWSTR(w.as_ptr()),
            None,
            Some(&mut ty),
            None,
            Some(&mut len),
        )
    };
    if rc != ERROR_SUCCESS {
        return None;
    }
    let mut data = vec![0u8; len as usize];
    let rc = unsafe {
        RegQueryValueExW(
            k,
            PCWSTR(w.as_ptr()),
            None,
            Some(&mut ty),
            Some(data.as_mut_ptr()),
            Some(&mut len),
        )
    };
    (rc == ERROR_SUCCESS).then(|| {
        data.truncate(len as usize);
        (ty, data)
    })
}

fn reg_string(bytes: &[u8]) -> String {
    let u: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf16_lossy(&u)
}

/// 사용자 Templates 폴더(`FOLDERID_Templates` · 보통 `%APPDATA%\Microsoft\Windows\Templates`).
fn templates_folder() -> Option<PathBuf> {
    let p = unsafe { SHGetKnownFolderPath(&FOLDERID_Templates, KF_FLAG_DEFAULT, None) }.ok()?;
    let s = unsafe { p.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(p.0 as *const std::ffi::c_void)) };
    s.map(PathBuf::from)
}

/// `FileName` 값 → 실제 경로(절대 · 사용자 Templates · `%WINDIR%\ShellNew` 순).
fn resolve_file_name(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.extend(templates_folder());
    if let Some(w) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(w).join("ShellNew"));
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

/// `ShellNew` 키 하나 → 원천(핸들러만 있으면 None).
fn source_of(shellnew: HKEY) -> Option<TemplateSource> {
    // 셸 핸들러가 만드는 종류(바로가기 마법사 · 라이브러리 …)는 빈 파일로는 쓸모가 없다 — NullFile이 같이 있어도 뺀다.
    if value(shellnew, "Command").is_some() || value(shellnew, "Handler").is_some() {
        return None;
    }
    if value(shellnew, "NullFile").is_some() {
        return Some(TemplateSource::Empty);
    }
    if let Some((_, b)) = value(shellnew, "FileName") {
        return resolve_file_name(&reg_string(&b)).map(TemplateSource::Copy);
    }
    if let Some((_, b)) = value(shellnew, "Data") {
        return Some(TemplateSource::Data(b));
    }
    None
}

/// `.ext` 키에서 템플릿 원천 찾기 — 직접 `ShellNew` → 하위 ProgId의 `ShellNew`.
fn source_for_ext(ext_key: &Key) -> Option<TemplateSource> {
    if let Some(sn) = open(ext_key.0, "ShellNew") {
        if let Some(src) = source_of(sn.0) {
            return Some(src);
        }
    }
    for sub in subkeys(ext_key.0) {
        if sub.eq_ignore_ascii_case("ShellNew") {
            continue;
        }
        if let Some(k) = open(ext_key.0, &format!("{sub}\\ShellNew")) {
            if let Some(src) = source_of(k.0) {
                return Some(src);
            }
        }
    }
    None
}

/// HKCR 전체 열거(한 번) — 확장자 키만 본다.
fn enumerate() -> Vec<NewTemplate> {
    let mut out: Vec<NewTemplate> = Vec::new();
    for name in subkeys(HKEY_CLASSES_ROOT) {
        let Some(ext) = name.strip_prefix('.') else {
            continue;
        };
        if ext.is_empty() || ext.contains('\\') {
            continue;
        }
        let Some(k) = open(HKEY_CLASSES_ROOT, &name) else {
            continue;
        };
        let Some(source) = source_for_ext(&k) else {
            continue;
        };
        let ext_l = ext.to_ascii_lowercase();
        if out.iter().any(|t| t.ext == ext_l) {
            continue;
        }
        let label = nexa_fs::shell::kind_name(&ext_l, false)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| ext_l.to_uppercase());
        out.push(NewTemplate {
            label,
            ext: ext_l,
            source,
        });
    }
    out.sort_by_key(|t| t.label.to_lowercase());
    out
}

/// 레지스트리 ShellNew + 사용자 템플릿 폴더.
pub(super) struct ShellNewTemplates {
    cache: std::cell::OnceCell<Vec<NewTemplate>>,
    user: UserTemplates,
}

impl ShellNewTemplates {
    pub(super) fn new() -> Self {
        ShellNewTemplates {
            cache: std::cell::OnceCell::new(),
            user: UserTemplates::new(os_template_dirs()),
        }
    }
}

impl Templates for ShellNewTemplates {
    fn list(&self) -> Vec<NewTemplate> {
        let mut v = self.cache.get_or_init(enumerate).clone();
        for t in self.user.list() {
            if !v.iter().any(|x| x.ext == t.ext && x.label == t.label) {
                v.push(t);
            }
        }
        v
    }
}

/// OS 문자열 → PathBuf(wide → OsString) — 시험·보조.
#[allow(dead_code)]
fn from_wide(w: &[u16]) -> PathBuf {
    PathBuf::from(std::ffi::OsString::from_wide(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 열거 = 비어 있지 않음(Windows는 bmp/zip 등 기본 ShellNew가 있다) · 라벨 비어 있지 않음 · 확장자 중복 없음 · 핸들러형(lnk) 제외 ·
    /// `.txt`가 있으면(표준 설치 — 일부 PC는 편집기가 지운다) 빈 파일 템플릿.
    #[test]
    fn enumerates_shellnew_templates() {
        let v = enumerate();
        assert!(!v.is_empty(), "ShellNew 템플릿 0개");
        assert!(v
            .iter()
            .all(|t| !t.label.trim().is_empty() && !t.ext.is_empty()));
        assert!(
            !v.iter().any(|t| t.ext == "lnk"),
            "바로가기 = 핸들러형 제외: {v:?}"
        );
        if let Some(txt) = v.iter().find(|t| t.ext == "txt") {
            assert_eq!(txt.source, TemplateSource::Empty);
        }
        let mut exts: Vec<&str> = v.iter().map(|t| t.ext.as_str()).collect();
        exts.sort_unstable();
        exts.dedup();
        assert_eq!(exts.len(), v.len(), "확장자 중복 없음");
        assert!(reg_string(b"a\0b\0\0\0") == "ab");
    }
}
