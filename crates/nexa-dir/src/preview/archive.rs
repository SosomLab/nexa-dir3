//! 압축 파일 미리보기 공급자(dir2 `preview/archive.rs` 이식 — X-46 · docs/port/20 §1.5).
//!
//! 목록 읽기는 [`ndir_vfs::archive`](압축 해제 없음)에 위임하고, 여기서는 **호스트 역할**만 한다:
//! - 확장자 선언 = 레지스트리 [`ndir_vfs::archive::all_exts`] 파생
//! - 결과를 [`PreviewDoc::Archive`]로 올려 **하단 도크 = 요약 텍스트**(독립 창 그리드 = T-62 B)
//! - 암호는 **세션 메모리에만**([`pw`]) — 설정·로그·디스크 어디에도 기록하지 않는다
//! - 이름 코드페이지 디코더(Windows CP_ACP) 주입 — 구형 zip의 한글 이름 대응

use std::path::{Path, PathBuf};

use ndir_core::secret::Secret;
use ndir_i18n::{tr, trf};
use ndir_vfs::archive::{self, ArchiveError, Listing};

use super::{PreviewDoc, PreviewProvider};

/// 미리보기 상태 — 실패 사유를 사용자 행동(암호 입력·플러그인 설치)으로 번역한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchiveStatus {
    Ok,
    /// 목록 자체가 암호화 — 그리드 창에서 암호를 받는다.
    NeedPassword,
    /// 코덱이 필요해 내장이 못 읽는다(포맷 표시명, 코덱 표시명).
    NeedPlugin(String, String),
    /// 그 밖의 실패(손상·입출력) — 사용자 표시 문구.
    Failed(String),
}

/// 압축 미리보기 문서 — 도크·독립 창·플러그인 결과가 공유하는 형태.
#[derive(Debug, Clone)]
pub(crate) struct ArchiveDoc {
    pub path: PathBuf,
    pub listing: Listing,
    pub status: ArchiveStatus,
    /// 이 결과를 만든 공급자 id(`builtin.archive` 또는 플러그인 id).
    pub provider: String,
}

impl ArchiveDoc {
    pub(crate) fn is_ok(&self) -> bool {
        self.status == ArchiveStatus::Ok
    }
}

/// 세션 한정 암호 보관 — **메모리에만**, 프로세스가 끝나면 사라진다(UI 스레드 전용 · `Secret` Drop 소거).
pub(crate) mod pw {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static CACHE: RefCell<HashMap<PathBuf, Secret>> = RefCell::new(HashMap::new());
    }

    pub(crate) fn get(path: &Path) -> Option<Secret> {
        CACHE.with(|c| c.borrow().get(path).cloned())
    }

    pub(crate) fn remember(path: &Path, secret: Secret) {
        CACHE.with(|c| {
            c.borrow_mut().insert(path.to_path_buf(), secret);
        });
    }

    pub(crate) fn forget(path: &Path) {
        CACHE.with(|c| {
            c.borrow_mut().remove(path);
        });
    }

    pub(crate) fn forget_all() {
        CACHE.with(|c| c.borrow_mut().clear());
    }

    pub(crate) fn len() -> usize {
        CACHE.with(|c| c.borrow().len())
    }
}

/// **활성 암호 주입**(호스트 → 공급자) — 호출 직전 슬롯에 넣고 호출 직후 비운다(스코프 가드 · Drop = 소거).
mod active {
    use super::Secret;
    use std::cell::RefCell;

    thread_local! {
        static ACTIVE: RefCell<Option<Secret>> = const { RefCell::new(None) };
    }

    pub(super) fn scoped<R>(pw: Option<Secret>, f: impl FnOnce() -> R) -> R {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                ACTIVE.with(|a| *a.borrow_mut() = None);
            }
        }
        ACTIVE.with(|a| *a.borrow_mut() = pw);
        let _g = Guard;
        f()
    }

    pub(super) fn with<R>(f: impl FnOnce(Option<&Secret>) -> R) -> R {
        ACTIVE.with(|a| f(a.borrow().as_ref()))
    }
}

/// 활성 암호 열람(런타임 호스트 API `password` 구현 — wasm.rs).
pub(crate) fn with_active_password<R>(f: impl FnOnce(Option<&Secret>) -> R) -> R {
    active::with(f)
}

/// 활성 암호를 건 채 `f` 실행(호출이 끝나면 슬롯은 비워지고 값은 소거된다 — 패닉 경로 포함).
pub(crate) fn with_password_scope<R>(password: Option<Secret>, f: impl FnOnce() -> R) -> R {
    active::scoped(password, f)
}

/// 공급자 경유 목록 읽기 — **플러그인 우선**(설정 `preview.map`·사용 여부 반영), 없으면 내장.
pub(crate) fn read_via(
    path: &Path,
    preview_map: &str,
    disabled: &str,
    password: Option<Secret>,
) -> ArchiveDoc {
    with_password_scope(password, || {
        match super::preview_for(path, preview_map, disabled).0 {
            PreviewDoc::Archive(doc) => *doc,
            _ => ArchiveDoc {
                path: path.to_path_buf(),
                listing: Listing::default(),
                status: ArchiveStatus::Failed(tr("archive.notArchive")),
                provider: String::new(),
            },
        }
    })
}

/// 목록 읽기 — 암호 우선순위 = 명시 인자 > 활성 슬롯(호스트 주입) > 세션 캐시.
pub(crate) fn read(path: &Path, password: Option<&Secret>) -> ArchiveDoc {
    let injected = password
        .is_none()
        .then(|| active::with(|p| p.cloned()))
        .flatten();
    let session = password.is_none() && injected.is_none();
    let session = session.then(|| pw::get(path)).flatten();
    let pass = password.or(injected.as_ref()).or(session.as_ref());
    let opts = archive::ListOpts {
        password: pass,
        ..Default::default()
    };
    let (listing, status) = match archive::list_path(path, &opts) {
        Ok(l) => (l, ArchiveStatus::Ok),
        Err(ArchiveError::PasswordRequired) | Err(ArchiveError::WrongPassword) => {
            (Listing::default(), ArchiveStatus::NeedPassword)
        }
        Err(ArchiveError::NeedsCodec(fmt, codec)) => {
            (Listing::default(), ArchiveStatus::NeedPlugin(fmt, codec))
        }
        Err(ArchiveError::NotArchive) => (
            Listing::default(),
            ArchiveStatus::Failed(tr("archive.notArchive")),
        ),
        Err(ArchiveError::Corrupt(why)) | Err(ArchiveError::Io(why)) => {
            (Listing::default(), ArchiveStatus::Failed(why))
        }
    };
    ArchiveDoc {
        path: path.to_path_buf(),
        listing,
        status,
        provider: "builtin.archive".into(),
    }
}

/// 도크(축약 뷰)용 텍스트 — 요약 + 항목 미리보기(`max_rows`) + `… 외 N개`.
pub(crate) fn summary_lines(doc: &ArchiveDoc, tz_offset_min: i32, max_rows: usize) -> Vec<String> {
    let mut out = Vec::new();
    match &doc.status {
        ArchiveStatus::NeedPassword => {
            out.push(tr("archive.needPassword"));
            out.push(tr("archive.openHint"));
            return out;
        }
        ArchiveStatus::NeedPlugin(fmt, codec) => {
            out.push(trf("archive.needPlugin", &[fmt, codec]));
            return out;
        }
        ArchiveStatus::Failed(why) => {
            out.push(trf("archive.failed", &[why]));
            return out;
        }
        ArchiveStatus::Ok => {}
    }
    let l = &doc.listing;
    let (files, dirs) = l.counts();
    let (size, packed) = l.totals();
    out.push(trf(
        "archive.summary",
        &[&l.label, &files.to_string(), &dirs.to_string()],
    ));
    let saved = if size > 0 {
        format!("{}%", 100 - (packed.min(size) * 100 / size.max(1)))
    } else {
        "-".into()
    };
    out.push(trf(
        "archive.sizes",
        &[
            &crate::filelist::format_size(size),
            &crate::filelist::format_size(packed),
            &saved,
        ],
    ));
    for (flag, key) in [
        (l.has_encrypted, "archive.encrypted"),
        (l.solid, "archive.solid"),
        (l.multivolume, "archive.multivolume"),
        (l.truncated, "archive.truncated"),
    ] {
        if flag {
            out.push(tr(key));
        }
    }
    if let Some(c) = &l.comment {
        out.push(trf("archive.comment", &[c]));
    }
    out.push(tr("archive.openHint"));
    out.push(String::new());
    for e in l.entries.iter().take(max_rows) {
        let size = match (e.is_dir, e.size) {
            (true, _) => String::new(),
            (_, Some(s)) => crate::filelist::format_size(s),
            _ => "-".into(),
        };
        let when = e
            .modified
            .map(|t| fmt_entry_time(t, e.time_is_local, tz_offset_min))
            .unwrap_or_default();
        let lock = if e.encrypted { "🔒 " } else { "" };
        out.push(format!(
            "{lock}{}{}  {size}  {when}",
            e.path,
            if e.is_dir { "/" } else { "" }
        ));
    }
    if l.entries.len() > max_rows {
        out.push(trf(
            "archive.more",
            &[&(l.entries.len() - max_rows).to_string()],
        ));
    }
    out
}

/// 항목 시각 표시 — DOS 계열(현지 벽시계 그대로)은 보정하지 않는다(이중 보정 방지).
pub(crate) fn fmt_entry_time(unix_secs: i64, time_is_local: bool, tz_offset_min: i32) -> String {
    let off = if time_is_local {
        0
    } else {
        i64::from(tz_offset_min)
    };
    crate::filelist::format_time(unix_secs.saturating_add(off * 60).saturating_mul(1000))
}

/// 내장 압축 공급자 — 확장자는 레지스트리에서 파생(포맷 추가 시 자동 확장).
pub(crate) struct BuiltinArchive {
    pub exts: Vec<String>,
}

impl BuiltinArchive {
    pub(crate) fn new() -> Self {
        BuiltinArchive {
            exts: archive::all_exts()
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

impl PreviewProvider for BuiltinArchive {
    fn id(&self) -> &str {
        "builtin.archive"
    }
    fn exts(&self) -> &[String] {
        &self.exts
    }
    fn preview(&self, path: &Path) -> PreviewDoc {
        PreviewDoc::Archive(Box::new(read(path, None)))
    }
}

/// OS 코드페이지 이름 디코더 주입(구형 zip의 CP949/CP932 이름) — 시작 시 1회. Windows = `MultiByteToWideChar(CP_ACP)` ·
/// 다른 OS = 없음(UTF-8 → CP437 폴백 · iconv 주입은 T-62 B).
#[cfg(windows)]
pub(crate) fn install_name_decoder() {
    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            cp: u32,
            flags: u32,
            src: *const u8,
            src_len: i32,
            dst: *mut u16,
            dst_len: i32,
        ) -> i32;
    }
    fn decode(bytes: &[u8]) -> Option<String> {
        if bytes.is_empty() {
            return Some(String::new());
        }
        let len = i32::try_from(bytes.len()).ok()?;
        // SAFETY: 유효 입력 버퍼·길이 · 출력 버퍼는 반환 길이만큼 확보 · CP_ACP = 0.
        unsafe {
            let n = MultiByteToWideChar(0, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0);
            if n <= 0 {
                return None;
            }
            let mut buf = vec![0u16; n as usize];
            let n = MultiByteToWideChar(0, 0, bytes.as_ptr(), len, buf.as_mut_ptr(), n);
            if n <= 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..n as usize]))
        }
    }
    archive::set_name_decoder(decode);
}

#[cfg(not(windows))]
pub(crate) fn install_name_decoder() {}

/// 시험용 최소 ZIP(다른 모듈의 시험 — 그리드 창·core).
#[cfg(test)]
pub(crate) fn zip_bytes_for_tests(name: &str) -> Vec<u8> {
    tests::zip_bytes(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험용 최소 ZIP(항목 1개 — 중앙 디렉터리 규약만 충족).
    pub(super) fn zip_bytes(name: &str) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&[0u8; 26]);
        let cd_off = out.len() as u32;
        let mut cd: Vec<u8> = Vec::new();
        cd.extend_from_slice(b"PK\x01\x02");
        cd.extend_from_slice(&[0u8; 4]);
        cd.extend_from_slice(&0x800u16.to_le_bytes()); // UTF-8 이름
        cd.extend_from_slice(&[0u8; 10]);
        cd.extend_from_slice(&7u32.to_le_bytes()); // 압축 크기
        cd.extend_from_slice(&10u32.to_le_bytes()); // 원본 크기
        cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
        cd.extend_from_slice(&[0u8; 12]);
        cd.extend_from_slice(&0u32.to_le_bytes()); // 로컬 오프셋
        cd.extend_from_slice(name.as_bytes());
        let cd_size = cd.len() as u32;
        out.extend_from_slice(&cd);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&[0u8; 4]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_off.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ndir_arc_{}_{name}", std::process::id()));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn archive_ext_routes_to_builtin_provider() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let p = tmp("a.zip", &zip_bytes("hello.txt"));
        match super::super::preview_for(&p, "", "") {
            (PreviewDoc::Archive(doc), id) => {
                assert!(doc.is_ok());
                assert_eq!(doc.listing.entries[0].path, "hello.txt");
                assert_eq!(
                    (doc.provider.as_str(), id.as_str()),
                    ("builtin.archive", "builtin.archive")
                );
            }
            other => panic!("압축 확장자는 압축 공급자여야 함: {other:?}"),
        }
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn summary_lines_lead_with_format_counts_and_truncate() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let p = tmp("b.zip", &zip_bytes("docs/readme.md"));
        let doc = read(&p, None);
        let lines = summary_lines(&doc, 540, 10);
        assert!(lines[0].contains("ZIP"), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("docs/readme.md")));
        // 절단: 항목 상한 0 = `… 외 N개` 안내.
        let cut = summary_lines(&doc, 0, 0);
        assert!(cut.last().is_some_and(|l| l.contains('1')), "{cut:?}");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn failure_status_maps_to_user_action() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let p = tmp("c.zip", b"not a zip at all");
        let doc = read(&p, None);
        assert!(matches!(doc.status, ArchiveStatus::Failed(_)));
        assert!(!summary_lines(&doc, 0, 5).is_empty());
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn password_cache_is_memory_only_and_forgettable() {
        let p = PathBuf::from("X:/only-a-key.zip");
        assert!(pw::get(&p).is_none());
        pw::remember(&p, Secret::new(b"pw".to_vec()));
        assert_eq!(
            pw::get(&p).map(|s| s.expose().to_vec()),
            Some(b"pw".to_vec())
        );
        pw::forget(&p);
        assert!(pw::get(&p).is_none());
        pw::remember(&p, Secret::new(b"pw".to_vec()));
        pw::forget_all();
        assert_eq!(pw::len(), 0);
        // 활성 슬롯은 패닉 경로에서도 비워진다.
        let r = std::panic::catch_unwind(|| {
            with_password_scope(Some(Secret::new(b"x".to_vec())), || {
                panic!("boom");
            })
        });
        assert!(r.is_err());
        assert!(
            with_active_password(|p| p.is_none()),
            "스코프 가드가 슬롯을 비운다"
        );
    }

    #[test]
    fn dos_times_are_not_shifted_twice() {
        let t = ndir_vfs::archive::ymd_hms_to_unix(2026, 8, 24, 13, 45, 0);
        assert_eq!(fmt_entry_time(t, true, 540), "2026-08-24 13:45");
        assert_eq!(fmt_entry_time(t, false, 540), "2026-08-24 22:45");
    }
}
