//! nexa-ops — 파일 조작 엔진(M3-1). **원본 이식**:
//! `app/Nexa.ViewModels/FileOps.cs`(순수 I/O — 청크 진행률·동일 볼륨 fast path·순번 명명) +
//! 원본 docs/33 TRANSFER-ENGINE(`TransferPathsInto` 단일 경로 — 같은 폴더 규칙·충돌 항목만
//! 순차 확인·바이트 진행률·취소·개별 격리).
//!
//! 플랫폼 중립(std 전용) — 전 플랫폼 테스트. 워커 스레드·PostMessage UI 배선은 nexa-app 책임.
//! 원본과의 차이: 취소·오류 시 **부분 복사 파일을 정리**한다(원본은 잔존 — 안전 개선, journal 기록).

pub mod batch_rename;
pub mod compare;
pub mod dupes;
pub mod fastcopy;
pub mod hash;
pub mod history;
pub mod namefilter;
pub mod verify;
/// 유니코드 정규화 — 10-31 ndir-core로 옮김(ndir-tree도 씀) · 종전 경로 `ndir_ops::nfc::*` 유지.
pub mod nfc {
    pub use ndir_core::nfc::*;
}

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// 전송 연산(docs/33 — 진입점이 달라도 이 두 연산으로 수렴).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Copy,
    Move,
}

/// 이름 충돌 결정(원본: 충돌 항목만 순차 확인 — 예=덮어씀/아니오=건너뜀).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conflict {
    Overwrite,
    Skip,
}

/// **폴더** 이름 충돌 처리(사용자 결정 11-02 · DR-37 · 3-OS 공통): 대상에 같은 이름의 폴더가 있을 때 덮어쓰기(`Conflict::Overwrite`)의 뜻 ·
/// 또는 묻지 않고 옆에 두기. 파일 충돌은 종전 그대로(덮어쓰기/건너뛰기). 현재 값 = [`fastcopy::dir_conflict`](스레드별 · ndir-xfer가 명세로 건다).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DirConflict {
    /// 교체: 옛 폴더를 통째로 치우고 새 폴더(dir2 OPS-011 · 대상에만 있던 항목은 사라진다).
    Replace,
    /// 병합(기본 · DR-35): 같은 이름 파일만 교체 · 둘 다 폴더면 안으로 · 대상에만 있던 항목은 남긴다.
    #[default]
    Merge,
    /// 보관: 옛 폴더는 그대로 두고 새 폴더를 " (2)"처럼 순번 이름으로 옆에(묻지 않음 · 계획 = `Duplicate`).
    KeepBoth,
}

impl DirConflict {
    /// 설정 · CLI · JSON 값(`replace` · `merge` · `keep`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "replace" => Some(DirConflict::Replace),
            "merge" => Some(DirConflict::Merge),
            "keep" => Some(DirConflict::KeepBoth),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DirConflict::Replace => "replace",
            DirConflict::Merge => "merge",
            DirConflict::KeepBoth => "keep",
        }
    }
}

/// 엔진이 만드는 오류의 **종류와 영어 문구**(라이브러리 규칙 · 프로그램 내부 문구는 영어 · 사용자 10-10 · 협업 점검 10-24 "CLI에 한글
/// 리터럴"). CLI(`nexa-copy`)는 문구를 그대로 쓰고 · 앱은 [`OpsError::of`]로 종류를 되찾아 i18n 키(`ops.err.*`)로 바꾼다.
/// 문구 뒤에 `: <경로>` 같은 꼬리가 붙을 수 있으므로 `of`는 접두 일치.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpsError {
    /// 폴더를 자기 자신/하위로 이동.
    IntoSelf,
    /// 빈 이름 · 구분자가 든 이름.
    BadName,
    /// 루트(부모 없음)의 이름 변경.
    RootRename,
    /// 같은 이름의 항목이 이미 있음.
    NameTaken,
    /// 상위 폴더를 가리키는 링크(순환) — 복사 중단.
    LinkCycle,
    /// 경로에 NUL 문자.
    NulInPath,
    /// 원본이 없음(일괄 이름 바꾸기).
    SourceMissing,
    /// 이동 뒤 원본 폴더를 비울 수 없어 남겨 둠.
    LeftoverSource,
    /// Windows 예약 이름(CON · PRN · AUX · NUL · COM1-9 · LPT1-9) 또는 끝이 빈칸/점(T-162 · 10-25).
    ReservedName,
    /// 이름이 너무 길다(255자 초과).
    NameTooLong,
}

impl OpsError {
    pub const ALL: [OpsError; 10] = [
        OpsError::IntoSelf,
        OpsError::BadName,
        OpsError::RootRename,
        OpsError::NameTaken,
        OpsError::LinkCycle,
        OpsError::NulInPath,
        OpsError::SourceMissing,
        OpsError::LeftoverSource,
        OpsError::ReservedName,
        OpsError::NameTooLong,
    ];

    /// 영어 문구(접두 — 뒤에 `: 세부`가 붙을 수 있다).
    pub const fn message(self) -> &'static str {
        match self {
            OpsError::IntoSelf => "cannot copy or move a folder into itself or one of its subfolders",
            OpsError::BadName => "invalid name",
            OpsError::RootRename => "cannot rename a root",
            OpsError::NameTaken => "an item with that name already exists",
            OpsError::LinkCycle => "link points to a parent folder (cycle), cannot copy",
            OpsError::NulInPath => "path contains a NUL character",
            OpsError::SourceMissing => "source does not exist",
            OpsError::LeftoverSource => {
                "could not empty the source folder, left in place (some items may not have been copied)"
            }
            OpsError::ReservedName => {
                "name is reserved on Windows (CON, PRN, AUX, NUL, COM1-9, LPT1-9) or ends with a space or period"
            }
            OpsError::NameTooLong => "name is too long (255 characters max)",
        }
    }

    /// 앱 i18n 키(`ndir-i18n` `ops.err.*`).
    pub const fn i18n_key(self) -> &'static str {
        match self {
            OpsError::IntoSelf => "ops.err.intoSelf",
            OpsError::BadName => "ops.err.badName",
            OpsError::RootRename => "ops.err.rootRename",
            OpsError::NameTaken => "ops.err.nameTaken",
            OpsError::LinkCycle => "ops.err.linkCycle",
            OpsError::NulInPath => "ops.err.nulInPath",
            OpsError::SourceMissing => "ops.err.sourceMissing",
            OpsError::LeftoverSource => "ops.err.leftoverSource",
            OpsError::ReservedName => "ops.err.reservedName",
            OpsError::NameTooLong => "ops.err.nameTooLong",
        }
    }

    /// 오류 문구(`io::Error::to_string()` 등)에서 종류 되찾기 — 접두 일치 · 모르는 문구 = `None`(OS 오류 등 · 그대로 보여 준다).
    pub fn of(text: &str) -> Option<OpsError> {
        OpsError::ALL
            .into_iter()
            .find(|k| text.starts_with(k.message()))
    }

    /// `io::Error`로(종류별 `ErrorKind`).
    pub fn io(self) -> io::Error {
        let kind = match self {
            OpsError::BadName
            | OpsError::RootRename
            | OpsError::NulInPath
            | OpsError::ReservedName
            | OpsError::NameTooLong => io::ErrorKind::InvalidInput,
            OpsError::NameTaken => io::ErrorKind::AlreadyExists,
            OpsError::SourceMissing => io::ErrorKind::NotFound,
            OpsError::IntoSelf | OpsError::LinkCycle | OpsError::LeftoverSource => {
                io::ErrorKind::Other
            }
        };
        io::Error::new(kind, self.message())
    }

    /// `io::Error`로 — 문구 뒤에 `: detail`.
    pub fn io_with(self, detail: impl std::fmt::Display) -> io::Error {
        let kind = self.io().kind();
        io::Error::new(kind, format!("{}: {detail}", self.message()))
    }
}

/// 전송 결과 — `transferred`의 (원본, 최종 대상) 쌍은 Undo(M3-3) 기록용.
#[derive(Default, Debug)]
pub struct Outcome {
    pub transferred: Vec<(PathBuf, PathBuf)>,
    /// 복사가 실제로 옮긴 원본 경로(파일 · 폴더 · 링크 안쪽 포함 · `record` 무관) — [`fastcopy::with_verify_collect`] 안에서만 채워진다
    /// (GAP-027 · 11-01). 검증 쌍은 [`verify::pairs_for`]가 만든다. 교차 볼륨 이동의 원본 정리 목록(`copied`)과는 다르다(BUG-001).
    pub copied_paths: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    /// 개별 격리된 실패(항목, 사유) — 한 항목의 실패가 배치를 중단하지 않는다.
    pub errors: Vec<(PathBuf, String)>,
    pub canceled: bool,
}

/// 새 이름 검사(이 OS 규칙 · T-162 · 10-25) — 이름 바꾸기 · 일괄 이름 변경 · 새 항목이 같은 함수를 쓴다.
pub fn validate_name(name: &str) -> Result<(), OpsError> {
    validate_name_for(name, cfg!(windows))
}

/// 새 이름 검사(순수 · OS 규칙을 인자로 — MC/DC 시험):
/// - 공통: 빈 이름 · `.`/`..` · 구분자(`/` `\`) · NUL = `BadName` · 255자 초과 = `NameTooLong`.
/// - Windows: `< > : " | ? *`와 제어 문자 = `BadName` · 끝이 빈칸/점 · 예약 이름(CON PRN AUX NUL COM1-9 LPT1-9 · 확장자 무관 · 대소문자 무관) =
///   `ReservedName`(탐색기는 만들 수 없고 만들어지면 지우기도 어렵다).
pub fn validate_name_for(name: &str, windows: bool) -> Result<(), OpsError> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        return Err(OpsError::BadName);
    }
    if name.chars().count() > 255 {
        return Err(OpsError::NameTooLong);
    }
    if !windows {
        return Ok(());
    }
    if name
        .chars()
        .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || (c as u32) < 0x20)
    {
        return Err(OpsError::BadName);
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Err(OpsError::ReservedName);
    }
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    let upper = stem.to_ascii_uppercase();
    // COM/LPT 뒤 한 글자 = 1~9 또는 위 첨자 ¹ ² ³(Windows 문서 · 협업 검토 10-25) — COM0 · COM10은 예약 아님.
    let dev_digit = |rest: &str| {
        let mut it = rest.chars();
        matches!(
            (it.next(), it.next()),
            (Some(c), None) if matches!(c, '1'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}')
        )
    };
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(dev_digit);
    if reserved {
        return Err(OpsError::ReservedName);
    }
    Ok(())
}

/// 경로 끝 구분자를 제거한 잎(파일/폴더) 이름.
pub fn leaf_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// 파일 또는 폴더 존재 여부(충돌 판정 공용 — 원본 Exists).
pub fn exists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

/// 대소문자 무시 경로 동등(Windows 관례 — 원본 PathEquals).
fn path_equals(a: &Path, b: &Path) -> bool {
    a.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(b.to_string_lossy().trim_end_matches(['\\', '/']))
}

/// `child`가 `ancestor` 자신 또는 그 하위인가(폴더 자기/하위 이동 금지 — 원본 IsSameOrSubPath).
pub fn is_same_or_sub(ancestor: &Path, child: &Path) -> bool {
    let a = ancestor.to_string_lossy();
    let a = a.trim_end_matches(['\\', '/']);
    let c = child.to_string_lossy();
    let c = c.trim_end_matches(['\\', '/']);
    if a.eq_ignore_ascii_case(c) {
        return true;
    }
    let c_low = c.to_lowercase();
    let a_low = a.to_lowercase();
    c_low.starts_with(&format!("{a_low}\\")) || c_low.starts_with(&format!("{a_low}/"))
}

/// 두 경로가 같은 볼륨인가(원본 SameVolume — 루트 비교, 판단 실패 시 true 보수적).
/// Windows = 드라이브/UNC 프리픽스, 그 외 플랫폼 = 단일 루트로 간주.
pub fn same_volume(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        same_volume_by_device(a, b)
    }
    #[cfg(not(unix))]
    {
        same_volume_by_prefix(a, b)
    }
}

/// Unix(GAP-031 · 11-01 협업 실측): 루트 `/` 비교는 늘 같다고 봐서 tmpfs → ext4 이동이 `rename` EXDEV로 **항상 실패**했다(dir2 Windows 전제
/// 그대로 이식) → 가장 가까운 있는 조상의 `st_dev`로 가른다. 못 읽으면 보수적으로 같다고 보고 [`move_onto_with_progress`]의 EXDEV 안전망이 받는다.
#[cfg(unix)]
fn same_volume_by_device(a: &Path, b: &Path) -> bool {
    fn dev_nearest(path: &Path) -> Option<u64> {
        use std::os::unix::fs::MetadataExt as _;
        let mut p = path;
        loop {
            if let Ok(m) = fs::symlink_metadata(p) {
                return Some(m.dev());
            }
            p = p.parent()?;
        }
    }
    match (dev_nearest(a), dev_nearest(b)) {
        (Some(da), Some(db)) => da == db,
        _ => true,
    }
}

/// Windows: 드라이브 접두(`C:` · UNC)로 — dir2 원형.
#[cfg(not(unix))]
fn same_volume_by_prefix(a: &Path, b: &Path) -> bool {
    use std::path::Component;
    fn root(p: &Path) -> Option<String> {
        match p.components().next() {
            Some(Component::Prefix(pr)) => Some(pr.as_os_str().to_string_lossy().to_lowercase()),
            Some(Component::RootDir) => Some("/".into()),
            _ => None, // 상대 경로 — 판단 불가
        }
    }
    match (root(a), root(b)) {
        (Some(ra), Some(rb)) => ra == rb,
        _ => true, // 보수적(원본 동일)
    }
}

/// `rename`이 "다른 장치"로 거부됐는가(GAP-031 안전망): Unix `EXDEV`(18) · Windows `ERROR_NOT_SAME_DEVICE`(17). 바인드 마운트처럼 `st_dev`가
/// 같아도 EXDEV가 날 수 있어 [`move_onto_with_progress`]가 이때 복사 이동으로 되돌린다.
fn is_cross_device(e: &io::Error) -> bool {
    #[cfg(unix)]
    let code = 18;
    #[cfg(windows)]
    let code = 17;
    #[cfg(not(any(unix, windows)))]
    let code = -1;
    e.raw_os_error() == Some(code)
}

/// `dest_dir` 안에서 `name`이 충돌하면 " (2)"…를 부여한 경로(원본 UniqueDest).
/// 폴더는 확장자 분리 안 함(예: "v1.2" 폴더), 파일은 확장자 유지·이름부에 순번.
pub fn unique_dest(dest_dir: &Path, name: &str, is_dir: bool) -> PathBuf {
    let natural = dest_dir.join(name);
    if !exists(&natural) {
        return natural;
    }
    let (stem, ext) = if is_dir {
        (name.to_string(), String::new())
    } else {
        let p = Path::new(name);
        match (p.file_stem(), p.extension()) {
            (Some(s), Some(e)) => (
                s.to_string_lossy().into_owned(),
                format!(".{}", e.to_string_lossy()),
            ),
            _ => (name.to_string(), String::new()),
        }
    };
    for i in 2.. {
        let cand = dest_dir.join(format!("{stem} ({i}){ext}"));
        if !exists(&cand) {
            return cand;
        }
    }
    unreachable!()
}

/// 파일/폴더 총 바이트(폴더 재귀 합계) — 접근 실패는 0으로 개별 격리(원본 SizeOf).
/// 이름 필터(`fastcopy::with_filter`)가 걸려 있으면 폴더 안의 거른 항목은 세지 않는다(GAP-044 · 실행과 같은 판정).
pub fn size_of(path: &Path) -> u64 {
    fn dir_sum(dir: &Path) -> u64 {
        let Ok(rd) = fs::read_dir(dir) else { return 0 };
        rd.flatten()
            .map(|e| {
                let p = e.path();
                match e.file_type() {
                    Ok(t) if t.is_dir() => {
                        if fastcopy::filter_allows(&e.file_name(), true) {
                            dir_sum(&p)
                        } else {
                            0
                        }
                    }
                    Ok(t) if t.is_file() && fastcopy::filter_allows(&e.file_name(), false) => {
                        e.metadata().map(|m| m.len()).unwrap_or(0)
                    }
                    _ => 0, // 심링크·거른 항목·실패 격리
                }
            })
            .sum()
    }
    match fs::metadata(path) {
        Ok(m) if m.is_dir() => dir_sum(path),
        Ok(m) if m.is_file() => m.len(),
        _ => 0,
    }
}

/// 링크형 항목인가(심볼릭 링크 · Windows 정션/마운트 포인트) — **대상을 따라가지 않고** 판정한다.
/// 링크는 "가리키는 것"이지 "담고 있는 것"이 아니다: 지우거나 옮길 때 대상의 내용을 건드리면 안 된다.
pub fn is_link(path: &Path) -> bool {
    let Ok(m) = fs::symlink_metadata(path) else {
        return false;
    };
    if m.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        // FILE_ATTRIBUTE_REPARSE_POINT — std가 심링크로 치지 않는 재분석 지점(일부 정션/마운트 포인트)까지.
        if m.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// 링크 **자체만** 제거(대상은 그대로). Windows의 폴더 링크는 `remove_dir`, 그 밖은 `remove_file`.
pub(crate) fn remove_link(path: &Path) -> io::Result<()> {
    fs::remove_file(path).or_else(|e| match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(_) => Err(e),
    })
}

/// 취소 신호를 io::Error(Interrupted)로 변환 — 엔진이 취소로 판정.
fn check_cancel(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(io::Error::new(io::ErrorKind::Interrupted, "canceled"))
    } else {
        Ok(())
    }
}

/// 파일을 복사하며 증분 바이트를 보고(원본 CopyFileWithProgress). 취소/실패 시 부분 대상 파일을 제거한다(원본 대비 안전 개선).
/// 복사 방법은 [`fastcopy`] 전략 계층이 고른다(Windows = `CopyFileExW` · 그 밖 = 읽고 쓰는 루프 — NEW-007 1차).
pub fn copy_file_with_progress(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<()> {
    fastcopy::copy_file(
        src,
        dest,
        overwrite,
        on_bytes,
        &|| cancel.load(Ordering::Relaxed),
        &fastcopy::tuning(),
    )
}

/// 폴더 재귀 복사. 열거 중 `Err` 엔트리는 **오류로 전파**한다(점검 2차 A14 — 종전 `flatten()`은
/// SMB 일시 오류·권한 등으로 빠진 항목을 조용히 버리고 Ok를 돌려줘, 교차 볼륨 이동이 원본을
/// 통째로 지우면 그 항목이 사본에도 원본에도 남지 않았다).
/// `copied`가 있으면 복사를 마친 원본 경로를 **후위 순서**(파일들 → 폴더는 자식 → 부모)로 기록한다 — 교차 볼륨
/// 이동이 복사한 항목만 골라 지우는 데 쓴다.
fn copy_dir_with_progress(
    src: &Path,
    dest: &Path,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
    copied: Option<&mut Vec<PathBuf>>,
) -> io::Result<()> {
    // 순회 규칙(링크 · 순환 · 열거 오류 전파)과 작은 파일 병렬은 `fastcopy::copy_tree`에 있다(NEW-007 1차).
    check_cancel(cancel)?;
    fastcopy::copy_tree(
        src,
        dest,
        on_bytes,
        &|| cancel.load(Ordering::Relaxed),
        copied,
        &fastcopy::tuning(),
    )
}

/// 교차 볼륨 이동의 원본 정리 — `copied`(후위 순서)에 적힌 항목만 지우고 마지막에 빈 `src`를
/// `remove_dir`한다(탐색기 방식). 열거에서 빠졌거나 복사 중 새로 생긴 항목은 남고, 그 때문에 폴더가
/// 비지 않으면 오류로 보고한다 — 사본은 완성돼 있으므로 데이터는 양쪽 어디에도 유실되지 않는다.
fn remove_copied(src: &Path, copied: &[PathBuf]) -> io::Result<()> {
    // 이름 필터(GAP-045)가 걸려 있으면 거른 항목이 원본에 남는 것이 **정상** — 그 때문에 비지 않은 폴더는 오류가 아니다.
    let filtered = fastcopy::filter_active();
    let mut first_err: Option<io::Error> = None;
    let mut note = |r: io::Result<()>, dir: Option<&Path>| {
        if let Err(e) = r {
            if filtered && dir.is_some_and(dir_has_entries) {
                return;
            }
            first_err.get_or_insert(e);
        }
    };
    for p in copied {
        // 링크는 **링크만**(대상은 그대로) · 파일 우선, 실패하면 폴더(빈 폴더만 — 남은 항목이 있으면 그대로 둔다)
        if is_link(p) {
            note(remove_link(p), None);
            continue;
        }
        let r = fs::remove_file(p).or_else(|e| {
            if p.is_dir() {
                remove_empty_dir(p)
            } else {
                Err(e)
            }
        });
        note(r, p.is_dir().then_some(p.as_path()));
    }
    note(remove_empty_dir(src), Some(src));
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// 폴더에 항목이 하나라도 남아 있는가(읽기 실패 = 모름 → `false`).
fn dir_has_entries(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|mut rd| rd.next().is_some())
}

/// 빈 폴더 삭제 — 비어 있지 않으면 사용자 메시지에 사유를 담아 반환.
fn remove_empty_dir(dir: &Path) -> io::Result<()> {
    fs::remove_dir(dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "{}: {}: {e}",
                OpsError::LeftoverSource.message(),
                leaf_name(dir)
            ),
        )
    })
}

/// `copy_onto_with_progress`의 본체 — `copied`는 교차 볼륨 이동 전용(복사한 원본 경로 기록).
fn copy_onto_inner(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
    copied: Option<&mut Vec<PathBuf>>,
) -> io::Result<()> {
    // T-208(11-09 · 사용자 결정 ⓒ): **새 대상도** 스테이징에 쓰고 끝에 커밋 — 프로세스가 죽어도(kill -9 · 전원) 최종 이름의 잘린 파일이 남지 않는다.
    // 남는 것은 숨김 `tmp` 잔여뿐이고 재개(`ndir-xfer`) · 버림(앱 큐)이 치운다. 새 대상 커밋 = 덮어쓰지 않는 rename(`fastcopy::rename_noreplace` ·
    // 종전 `create_new`가 주던 "그 사이 생긴 같은 이름은 건드리지 않음" 보장 유지 · 같은 폴더 복제 " (2)" 포함).
    let existing = dest.symlink_metadata().is_ok(); // stat 1회(파일 다수 비용 · 협업 실측 11-09 파일당 +21 µs 중 일부)
    let replacing = overwrite && existing;
    if !overwrite && existing {
        return Err(already_exists(dest)); // 종전 `create_new`와 같은 시점(복사 전)에 거부 — 다 복사한 뒤 커밋에서 알면 낭비
    }
    let staged = staging_path(dest, "tmp");
    if src.is_dir() {
        let run = copy_dir_with_progress(src, &staged, on_bytes, cancel, copied)
            .and_then(|()| commit(&staged, dest, replacing));
        if run.is_err() {
            let _ = fs::remove_dir_all(&staged);
        }
        run
    } else {
        // copy_file_with_progress가 실패 시 staged를 정리한다
        copy_file_with_progress(src, &staged, true, on_bytes, cancel)?;
        commit(&staged, dest, replacing).inspect_err(|_| {
            let _ = fs::remove_file(&staged);
        })
    }
}

/// 스테이징 → 대상 커밋: 교체(`replacing`) = [`commit_replace`](옛 대상은 새 내용이 완성된 뒤에만 사라진다) · 새 대상 = 덮어쓰지 않는
/// rename(T-208 · 그 틈에 같은 이름이 생겼으면 `AlreadyExists` = 그 항목 실패 · 호출자가 스테이징을 치운다).
fn commit(staged: &Path, dest: &Path, replacing: bool) -> io::Result<()> {
    if replacing {
        commit_replace(staged, dest)
    } else {
        fastcopy::rename_noreplace(staged, dest)
    }
}

/// 새 대상 자리에 이미 무엇이 있다(덮어쓰기 아님 · 종전 `create_new`의 `AlreadyExists`와 같은 뜻).
fn already_exists(dest: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("already exists: {}", dest.display()),
    )
}

/// 스테이징 경로(교체 · 새 대상 공용 · T-208) — 대상과 **같은 부모**(같은 볼륨이라 rename이 원자적). 숨김 규약 `.<name>.nexa-<tag>-<pid>-<seq>`.
fn staging_path(dest: &Path, tag: &str) -> PathBuf {
    use std::sync::atomic::AtomicU64;
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    dest.with_file_name(format!(
        ".{}.nexa-{tag}-{}-{}",
        staging_stem(&name),
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

/// 스테이징 이름의 "이름" 부분 상한(바이트) — 파일 시스템 NAME_MAX 255(Linux · APFS = 바이트 · NTFS = UTF-16 255 단위 ≥ 글자 수)에 장식
/// `.` + `.nexa-<tag>-<pid>-<seq>`(최대 ≈ 42 B)를 더해도 넘지 않게. 협업 QA 11-09 #1: 새 대상까지 스테이징하자(T-208) 232 B 넘는 이름(한글 80자 등)이
/// 전부 `File name too long`으로 실패했다(덮어쓰기 경로는 0.30.2부터 같은 한계 — 함께 해소).
const STAGING_STEM_MAX: usize = 200;

/// 긴 이름은 UTF-8 글자 경계에서 자른다. [`staging_path`]와 이름 지정 쓸기([`remove_staging_leftovers`])가 **같은 함수**로 접두를 만든다 — 그래야
/// 재개·버림이 긴 이름의 잔여를 놓치지 않는다. 앞 200 B가 같은 다른 이름의 잔여도 접두에 걸리지만 죽은 pid 것만 지우므로 무해.
fn staging_stem(name: &str) -> &str {
    if name.len() <= STAGING_STEM_MAX {
        return name;
    }
    let mut i = STAGING_STEM_MAX;
    while !name.is_char_boundary(i) {
        i -= 1;
    }
    &name[..i]
}

/// 스테이징 → 대상 **커밋 교체**(점검 1차 #1 — 옛 대상은 새 내용이 완성된 뒤에만 사라진다).
/// 파일: `rename`(std = MOVEFILE_REPLACE_EXISTING, 원자) · 폴더: 옛 폴더를 옆으로 치우고 rename,
/// 실패 시 원복 · 대상이 파일인데 폴더로 교체(형 불일치)면 파일 제거 후 rename.
fn commit_replace(staged: &Path, dest: &Path) -> io::Result<()> {
    if dest.is_dir() {
        if !staged.is_dir() {
            return Err(type_mismatch(dest)); // 파일로 폴더를 덮어쓰기 = 그 항목 실패(11-01 협업 m4 · 사용자 판단 거리)
        }
        // 덮어쓰기 = 병합(11-01 사용자 결정 · 협업 d4 재현 차단) — **새 내용을 옛 대상 안으로** 옮긴다(협업 m6 재현 11-01: 옛 대상을 스테이징으로
        // 옮기던 첫 판은 병합 중 죽으면 대상이 사라지고 재개의 잔여 정리가 사용자 파일을 지웠다 — 6만 개 소실). 불변식: `tmp` = 새 내용만(지워도
        // 안전) · `old` = 사용자가 덮어쓰기로 동의한 교체분만 · 대상 폴더는 어느 순간에도 존재 · 같은 작업을 다시 돌리면 수렴.
        // 교체(DR-37 `DirConflict::Replace` · 사용자 선택): 옛 폴더를 옆으로 치우고 새 폴더를 그 자리에 — 실패하면 옛 폴더 원복.
        if fastcopy::dir_conflict() == DirConflict::Replace {
            return replace_dir(staged, dest);
        }
        let old = staging_path(dest, "old");
        merge_into(staged, dest, &old)?; // 실패 시 안에서 되돌림(옛 대상 그대로)
        let _ = fs::remove_dir_all(&old); // 교체된 옛 항목(동의분) — 새 대상은 이미 자리를 잡았다
        let _ = fs::remove_dir_all(staged); // 빈 뼈대만 남는다
                                            // 지난 실행(죽은 pid)이 남긴 같은 대상의 잔여도 성공 뒤에만 치운다(협업 m7: 같은 볼륨 이동 병합 중 사망 → 재실행 뒤 old 10,082개 잔여).
        remove_staging_leftovers(dest, &["tmp", "old"]);
        return Ok(());
    }
    if staged.is_dir() && dest.is_file() {
        return Err(type_mismatch(dest)); // 폴더로 파일을 덮어쓰기 = 그 항목 실패
    }
    fs::rename(staged, dest)
}

/// 폴더 **교체**(DR-37 `Replace`): `dest`(옛 폴더)를 `old` 스테이징으로 치우고 `new_root`(스테이징 사본 또는 같은 볼륨 원본)를 `dest`로 rename ·
/// 실패하면 옛 폴더를 되돌린다 · 성공 뒤에만 옛 폴더를 지운다(옛 내용은 **전부 사라진다** — 사용자가 설정으로 고른 동작). 같은 부모라 rename 둘 다 즉시.
fn replace_dir(new_root: &Path, dest: &Path) -> io::Result<()> {
    let old = staging_path(dest, "old");
    fs::rename(dest, &old)?;
    if let Err(e) = fs::rename(new_root, dest) {
        let _ = fs::rename(&old, dest);
        return Err(e);
    }
    let _ = fs::remove_dir_all(&old);
    remove_staging_leftovers(dest, &["tmp", "old"]);
    Ok(())
}

/// 같은 이름이 파일 ↔ 폴더로 다르다(덮어쓰기 거부 · 탐색기도 묻거나 실패하는 경계).
fn type_mismatch(dest: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "cannot overwrite: a file and a folder share the name {}",
            dest.display()
        ),
    )
}

/// 폴더 덮어쓰기의 **병합**(DR-35 · 11-01): `new_root`(스테이징 또는 같은 볼륨 원본)의 항목을 `dest` 안으로 옮긴다 — 없던 이름 = 그대로 들어감 ·
/// 둘 다 폴더 = 안으로 되풀이 · 파일/링크 같은 이름 = 옛 것을 `old_root`(같은 상대 경로)로 치운 뒤 새 것 · 파일 ↔ 폴더 = 오류. `new_root`와 `dest`는
/// 같은 볼륨이라 `rename`이 즉시다. 실패하면 옮긴 것을 역순으로 되돌린다(옛 대상 그대로 · 새 항목은 `new_root`로).
fn merge_into(new_root: &Path, dest: &Path, old_root: &Path) -> io::Result<()> {
    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    if let Err(e) = merge_walk(new_root, dest, old_root, &mut moves) {
        revert_moves(&moves);
        let _ = fs::remove_dir_all(old_root); // 되돌린 뒤엔 빈 뼈대
        return Err(e);
    }
    Ok(())
}

fn merge_walk(
    new_dir: &Path,
    dest_dir: &Path,
    old_dir: &Path,
    moves: &mut Vec<(PathBuf, PathBuf)>,
) -> io::Result<()> {
    for e in fs::read_dir(new_dir)? {
        let e = e?;
        let n = e.path();
        let d = dest_dir.join(e.file_name());
        let o = old_dir.join(e.file_name());
        let n_dir = n.is_dir() && !is_link(&n);
        match fs::symlink_metadata(&d) {
            Err(_) => {
                fs::rename(&n, &d)?;
                moves.push((n, d));
            }
            Ok(m) => {
                let d_dir = m.is_dir() && !is_link(&d);
                if n_dir && d_dir {
                    merge_walk(&n, &d, &o, moves)?;
                } else if n_dir != d_dir {
                    return Err(type_mismatch(&d));
                } else {
                    fs::create_dir_all(old_dir)?;
                    fs::rename(&d, &o)?; // 옛 항목(동의한 교체분) → old
                    moves.push((d.clone(), o));
                    fs::rename(&n, &d)?; // 새 항목 → 자리
                    moves.push((n, d));
                }
            }
        }
    }
    Ok(())
}

/// [`merge_into`]가 옮긴 것을 역순으로 되돌린다(실패 경로 · 최선 노력).
fn revert_moves(moved: &[(PathBuf, PathBuf)]) {
    for (orig, now) in moved.iter().rev() {
        let _ = fs::rename(now, orig);
    }
}

/// 원본을 정확히 `dest`로 복사(순번 부여 없음). 늘 **스테이징에 쓰고 끝에 커밋**(T-208 · 11-09 — 새 대상 = 덮어쓰지 않는 rename · 덮어쓰기 = 교체) —
/// 취소/실패/강제 종료 어느 때도 최종 이름에 잘린 파일이 생기지 않고, 덮어쓰기는 옛 대상이 그대로 남고 스테이징만 제거된다(점검 1차 #1: 종전은 폴더를 먼저
/// 지워 취소 시 소실). 폴더 교체는 **병합**(11-01 · 새 내용을 옛 대상
/// 안으로 · 옛 대상에만 있던 항목은 남고 같은 이름만 새 것으로 — 탐색기 "폴더 바꾸기"와 같음 · 대상 폴더는 어느 순간에도 존재 · 파일 ↔ 폴더 이름
/// 충돌은 그 항목 실패).
pub fn copy_onto_with_progress(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<()> {
    copy_onto_inner(src, dest, overwrite, on_bytes, cancel, None)
}

/// 교차 볼륨 이동 = 복사 후 **복사한 항목만** 원본에서 제거(폴더는 `remove_copied`, 파일은 `remove_file`).
/// 종전은 `remove_dir_all(src)`로 통째로 지워 열거에서 빠진 항목·복사 중 생긴 항목이 유실됐다.
fn move_by_copy(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<()> {
    // 이동 검증(T-202 ⑤)의 대조 목록 = 복사가 실제로 옮긴 경로(링크 안쪽 · 빈 폴더 포함 · GAP-027 싱크) — `copied`(원본 정리용 · 링크 안쪽 제외)와 다르다.
    let collect = fastcopy::move_verify();
    if is_link(src) {
        // ⚠ 링크(심볼릭 링크·정션)를 다른 볼륨으로 옮길 때: 대상 내용을 새 자리에 복사하고 원본에서는 **링크만** 없앤다.
        // 종전에는 링크를 폴더처럼 보고 `링크\자식` 경로를 하나씩 지워 **링크가 가리키는 실제 파일이 삭제**됐다(dir2와 공통 결함).
        // 이동 검증(T-202 ⑤)도 건다 — `fs::metadata`가 링크를 따라가 "가리키는 내용 vs 사본"이 바로 맞는 대조(협업 판단 ②).
        let (r, list) = collect_if(collect, || {
            copy_onto_inner(src, dest, overwrite, on_bytes, cancel, None)
        });
        r?;
        verify_moved(src, dest, &list, cancel)?;
        remove_link(src)
    } else if src.is_dir() {
        let mut copied = Vec::new();
        let (r, list) = collect_if(collect, || {
            copy_onto_inner(src, dest, overwrite, on_bytes, cancel, Some(&mut copied))
            // 스테이징 교체
        });
        r?;
        verify_moved(src, dest, &list, cancel)?; // 원본 정리 **전**(T-202 ⑤)
        remove_copied(src, &copied)
    } else {
        copy_onto_inner(src, dest, overwrite, on_bytes, cancel, None)?;
        verify_moved(src, dest, &[], cancel)?;
        fs::remove_file(src)
    }
}

/// `on`이면 [`fastcopy::collect_verify`]로 목록을 모으며 돌리고, 아니면 그냥 돌린다(빈 목록).
fn collect_if<R>(on: bool, f: impl FnOnce() -> R) -> (R, Vec<PathBuf>) {
    if on {
        fastcopy::collect_verify(f)
    } else {
        (f(), Vec::new())
    }
}

/// 이동 검증(T-202 ⑤ · [`fastcopy::with_move_verify`] 범위에서만): 교차 볼륨 이동이 **원본을 지우기 전에** 사본을 원본과 대조한다
/// (목록 검증 `verify_listed` · `inner` = 복사가 실제로 옮긴 경로(GAP-027 싱크 · 링크 안쪽 · 빈 폴더 포함) — 필터로 뺀 항목은 대상에 없는 것이
/// 정상). 불일치 = 오류(원본 **유지** · 사본도 남김 · 사용자가 판단) · 검증 읽기는 진행 바이트에 더하지 않는다(전송 총량 기준 유지).
fn verify_moved(src: &Path, dest: &Path, inner: &[PathBuf], cancel: &AtomicBool) -> io::Result<()> {
    if !fastcopy::move_verify() {
        return Ok(());
    }
    // 항목 자체(파일 = 크기+해시 · 폴더/링크 = 존재·종류) + 안쪽 목록(링크 안쪽 · 빈 폴더 포함 · 대상은 상대 경로로 재계산 — 스테이징 무관).
    let mut pairs: Vec<(PathBuf, PathBuf)> = vec![(src.to_path_buf(), dest.to_path_buf())];
    pairs.extend(inner.iter().filter_map(|p| {
        let rel = p.strip_prefix(src).ok()?;
        (!rel.as_os_str().is_empty()).then(|| (p.clone(), dest.join(rel)))
    }));
    let rep = verify::verify_listed(&pairs, &mut |_| {}, cancel)?;
    match rep.failures.first() {
        None => Ok(()),
        Some((p, why)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "move verify failed: {}: {why} ({} mismatch(es); source kept, copy left in place)",
                p.display(),
                rep.failures.len()
            ),
        )),
    }
}

/// 원본을 정확히 `dest`로 이동 — 같은 볼륨=rename(전체 크기 1회 보고), 다른 볼륨=복사 후 **복사한 항목만** 원본에서 삭제.
/// 자기 자신/하위로의 폴더 이동은 오류(원본 cycleMove).
pub fn move_onto_with_progress(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<()> {
    let is_dir = src.is_dir();
    if is_dir && is_same_or_sub(src, dest) {
        return Err(OpsError::IntoSelf.io());
    }
    // 이름 필터가 걸린 폴더는 통째 rename 금지(GAP-045) — 복사 이동이 고른 항목만 옮기고 거른 항목은 원본에 남긴다.
    if same_volume(src, dest) && !(is_dir && fastcopy::filter_active()) {
        match rename_onto(src, dest, overwrite, is_dir) {
            Ok(()) => {
                on_bytes(size_of(dest)); // 메타데이터 이동(즉시) — 전체 크기 1회 보고
                return Ok(());
            }
            // 안전망(GAP-031): 장치 판정이 같다고 했어도 rename이 EXDEV면(바인드 마운트 · 판정 불가 경로) 복사 이동으로.
            Err(e) if is_cross_device(&e) => {}
            Err(e) => return Err(e),
        }
    }
    move_by_copy(src, dest, overwrite, on_bytes, cancel)
}

/// 같은 볼륨 이동 = `rename`(옛 대상은 새 대상이 자리를 잡은 뒤에만 사라진다 · 점검 1차 #1 — 종전은 선삭제 후 rename).
fn rename_onto(src: &Path, dest: &Path, overwrite: bool, is_dir: bool) -> io::Result<()> {
    if overwrite && dest.is_dir() {
        if !is_dir {
            return Err(type_mismatch(dest)); // 파일로 폴더를 덮어쓰기 = 그 항목 실패
        }
        // 교체(DR-37 `Replace`): 옛 폴더를 치우고 원본 폴더를 rename — 실패 시 원복.
        if fastcopy::dir_conflict() == DirConflict::Replace {
            return replace_dir(src, dest);
        }
        // 병합(DR-35): 같은 볼륨이라 원본 항목을 바로 대상 안으로 옮긴다(옛 대상은 제자리 · 교체분만 old · 실패 시 되돌림) → 원본은 빈 뼈대만.
        let old = staging_path(dest, "old");
        merge_into(src, dest, &old)?;
        let _ = fs::remove_dir_all(&old);
        let _ = fs::remove_dir_all(src);
        remove_staging_leftovers(dest, &["tmp", "old"]); // 지난 실행의 죽은 pid 잔여(m7)
    } else if overwrite && is_dir && dest.is_file() {
        return Err(type_mismatch(dest)); // 폴더로 파일을 덮어쓰기 = 그 항목 실패
    } else {
        fs::rename(src, dest)?; // 파일 위 파일 = std rename(REPLACE_EXISTING) 원자 교체
    }
    Ok(())
}

/// 스테이징 잔여 정리(GAP-030 · 11-01 협업 kill -9 실측): 프로세스가 죽으면 `dest` 옆에 `.<이름>.nexa-<tag>-<pid>-<n>`이 남는다(오류·취소 경로만
/// 치우므로). `tags`에 든 종류(`tmp` = 교체용 사본 · `old` = 치워 둔 옛 대상)만 지운다(파일·폴더 · 링크는 링크만) · 돌려주는 값 = 지운 수.
/// 재개(`ndir-xfer`)가 중단된 항목을 다시 돌리기 **전**에 `tmp`를, 끝난 **뒤**에 `old`를 치운다.
pub fn remove_staging_leftovers(dest: &Path, tags: &[&str]) -> usize {
    let (Some(parent), Some(name)) = (dest.parent(), dest.file_name()) else {
        return 0;
    };
    sweep_staging(parent, Some(&name.to_string_lossy()), tags)
}

/// 대상 **폴더**의 스테이징 잔여를 이름과 무관하게 치운다(T-208 후속 · 협업 실측 11-09: 저널 없는 `nexa-copy` 재실행은 끊긴 항목을 모르니 그 대상의
/// 잔여를 `remove_staging_leftovers`로 짚어 치울 수 없다 → 새 대상도 스테이징에 쓰는 지금은 숨은 잔여가 디스크를 계속 차지한다). 전송 시작 때
/// 대상 폴더에 1회(항목마다 `read_dir`하면 파일 다수에 O(n²)). 규칙 = 엔진 이름 꼴 `.<x>.nexa-<tag>-<pid>-<seq>` · 죽은 pid · **그리고**
/// [`SWEEP_AGE_SECS`](1시간) 넘게 변경이 없는 것만(협업 지적 11-09: `process_alive`는 이 기계의 프로세스 표라 공유 폴더(NAS · SMB)에서 **다른 기계**가
/// 쓰는 중인 스테이징이 "죽은 pid"로 보인다 — 쓰는 중이면 수정 시각이 계속 바뀌므로 나이로 가른다 · 폴더는 안쪽까지 가장 최근 변경). 그래서 방금 죽은
/// 잔여는 이 쓸기가 아니라 재개(저널 · 이름 지정)가 치우고, 저널 없는 재실행의 잔여는 한 시간 뒤 다음 전송이 치운다.
pub fn sweep_staging_leftovers(dir: &Path, tags: &[&str]) -> usize {
    sweep_staging(dir, None, tags)
}

/// 폴더 쓸기의 나이 기준(초) — 시험이 바꾼다.
static SWEEP_AGE_SECS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(3600);

#[cfg(test)]
fn set_sweep_age_for_test(secs: u64) {
    SWEEP_AGE_SECS.store(secs, Ordering::Relaxed);
}

/// `p`(파일 · 폴더는 안쪽 전부) 어디에도 `cutoff`보다 늦은 변경이 없는가 — 폴더는 하나라도 새것이면 바로 `false`(살아 있는 전송은 거의 즉시 끝난다 ·
/// 전부 훑는 것은 정말 오래된 잔여일 때뿐 = 곧 지울 것). 시각을 못 읽으면 보수적으로 `false`(안 지움).
fn untouched_since(p: &Path, cutoff: std::time::SystemTime) -> bool {
    let Ok(meta) = fs::symlink_metadata(p) else {
        return false;
    };
    let Ok(m) = meta.modified() else {
        return false;
    };
    if m > cutoff {
        return false;
    }
    if meta.is_dir() {
        let Ok(rd) = fs::read_dir(p) else {
            return false;
        };
        for e in rd.flatten() {
            if !untouched_since(&e.path(), cutoff) {
                return false;
            }
        }
    }
    true
}

/// 공용 본체: `name`이 있으면 그 대상의 잔여만(`.<name>.nexa-<tag>-` · 재개 경로 · 나이 안 봄 = 44ac475 그대로) · 없으면 폴더 안 엔진 이름 꼴 전부
/// (나이 가드). `<pid>-<seq>` 둘 다 숫자 · pid가 죽었을 때만.
fn sweep_staging(dir: &Path, name: Option<&str>, tags: &[&str]) -> usize {
    let cutoff = name.is_none().then(|| {
        std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(
                SWEEP_AGE_SECS.load(Ordering::Relaxed),
            ))
            .unwrap_or(std::time::UNIX_EPOCH)
    });
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    let mut n = 0;
    for e in rd.flatten() {
        let fname = e.file_name().to_string_lossy().into_owned();
        let rest = match name {
            Some(name) => {
                let stem = staging_stem(name);
                tags.iter()
                    .find_map(|t| fname.strip_prefix(&format!(".{stem}.nexa-{t}-")))
            }
            None => {
                if !fname.starts_with('.') {
                    continue;
                }
                // 마지막 ".nexa-<tag>-" 뒤 = <pid>-<seq> · 앞에 이름이 한 글자 이상.
                tags.iter().find_map(|t| {
                    let marker = format!(".nexa-{t}-");
                    fname
                        .rfind(&marker)
                        .filter(|&i| i > 0)
                        .map(|i| &fname[i + marker.len()..])
                })
            }
        };
        let Some(rest) = rest else {
            continue;
        };
        // 엔진이 만든 이름만(`<pid>-<seq>` 둘 다 숫자) — 해석이 안 되면 사용자 것일 수 있어 건너뛴다(협업 실측 11-01: `.t.nexa-old-notes`가 지워졌다).
        let Some((pid, seq)) = rest.split_once('-') else {
            continue;
        };
        let (Ok(pid), Ok(_)) = (pid.parse::<u32>(), seq.parse::<u64>()) else {
            continue;
        };
        // 이름의 pid가 **살아 있으면** 다른 프로세스가 같은 대상을 쓰는 중 — 건드리지 않는다(협업 지적 11-01 · 단일 인스턴스 장치 없음).
        if process_alive(pid) {
            continue;
        }
        let p = e.path();
        if let Some(cutoff) = cutoff {
            if !untouched_since(&p, cutoff) {
                continue; // 최근에 변한 것 = 다른 기계(또는 모르는 프로세스)가 쓰는 중일 수 있다
            }
        }
        let r = if is_link(&p) {
            remove_link(&p)
        } else if p.is_dir() {
            fs::remove_dir_all(&p)
        } else {
            fs::remove_file(&p)
        };
        if r.is_ok() {
            n += 1;
        }
    }
    n
}

/// 프로세스 생존 확인(GAP-030 스테이징 잔여 정리 · T-202 ④ 큐 소유 표시 · 11-01): unix = `kill(pid, 0)`(0 또는 EPERM = 살아 있음) ·
/// Windows = `OpenProcess` + `GetExitCodeProcess == STILL_ACTIVE`(열기 거부 ERROR_ACCESS_DENIED = 살아 있음 · GAP-035). 외부 crate 0(DR-8 · FFI 직접 선언 · `fastcopy` os 모듈 선례). pid 재사용은
/// 못 가른다(호출자가 기동 시각 등을 함께 대조).
#[must_use]
pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        let Ok(p) = i32::try_from(pid) else {
            return false;
        };
        if p <= 0 {
            return false;
        }
        // SAFETY: 시그널 0 = 존재·권한 확인만(아무 일도 보내지 않는다).
        let r = unsafe { kill(p, 0) };
        r == 0 || io::Error::last_os_error().raw_os_error() == Some(1) // EPERM = 있지만 남의 것
    }
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut core::ffi::c_void;
            fn GetExitCodeProcess(h: *mut core::ffi::c_void, code: *mut u32) -> i32;
            fn CloseHandle(h: *mut core::ffi::c_void) -> i32;
        }
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const STILL_ACTIVE: u32 = 259;
        const ERROR_ACCESS_DENIED: i32 = 5;
        // SAFETY: 핸들을 열어 종료 코드만 읽고 바로 닫는다.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                // GAP-035: 접근 거부(다른 계정 · 보호 프로세스)는 "있지만 남의 것" = 살아 있음(unix EPERM과 같은 판정) ·
                // 그 밖(ERROR_INVALID_PARAMETER 87 = 없는 pid)만 죽음.
                return io::Error::last_os_error().raw_os_error() == Some(ERROR_ACCESS_DENIED);
            }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(h, &raw mut code) != 0;
            CloseHandle(h);
            ok && code == STILL_ACTIVE
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

/// 완전 삭제 — 트리를 **끝까지** 돈다(GAP-046 · 사용자 11-04 ⓐ · 탐색기 방식): 지울 수 있는 것은 전부 지우고 지우지 못한 항목의 경로와
/// 사유를 돌려준다(빈 목록 = 전부 삭제 · 없는 경로 = 무동작). 폴더는 `remove_dir_all`(한 번에 · Windows 읽기 전용 속성 무시)을 먼저 시도하고,
/// 실패하면 항목마다 내려가며 계속한다. 안쪽 항목이 남아 비지 않은 폴더는 그 자체를 실패로 적지 않는다(안쪽 실패만 보인다) · 열거할 수 없는
/// 폴더 = 그 폴더가 실패 · 링크는 링크만. 종전 [`delete_permanent`]는 첫 실패에서 멈춰 폴더가 반쯤 지워진 채 "지우지 못함"만 알렸다.
pub fn delete_permanent_tree(path: &Path) -> Vec<(PathBuf, io::Error)> {
    fn go(path: &Path, failed: &mut Vec<(PathBuf, io::Error)>) {
        if is_link(path) {
            if let Err(e) = remove_link(path) {
                failed.push((path.to_path_buf(), e));
            }
            return;
        }
        let Ok(meta) = fs::symlink_metadata(path) else {
            return; // 없음 = 무동작
        };
        if !meta.is_dir() {
            if let Err(e) = fs::remove_file(path) {
                failed.push((path.to_path_buf(), e));
            }
            return;
        }
        if fs::remove_dir_all(path).is_ok() {
            return;
        }
        let rd = match fs::read_dir(path) {
            Ok(rd) => rd,
            Err(e) => {
                failed.push((path.to_path_buf(), e));
                return;
            }
        };
        let before = failed.len();
        for e in rd {
            match e {
                Ok(e) => go(&e.path(), failed),
                Err(e) => failed.push((path.to_path_buf(), e)),
            }
        }
        if failed.len() > before {
            return; // 안쪽이 남았다 — 폴더는 그대로 두고 실패로 적지 않는다
        }
        if let Err(e) = fs::remove_dir(path) {
            failed.push((path.to_path_buf(), e));
        }
    }
    let mut failed = Vec::new();
    go(path, &mut failed);
    failed
}

/// 완전 삭제(휴지통 아님, 폴더 재귀) — 없으면 무동작(원본 DeletePermanent). 첫 실패에서 멈춘다(정리용 · 사용자 명령은
/// [`delete_permanent_tree`]). 휴지통 삭제는 셸 API가 필요해 앱 계층(win.rs) 담당.
pub fn delete_permanent(path: &Path) -> io::Result<()> {
    if is_link(path) {
        // 링크 자체만(깨진 링크 포함) — 대상 폴더의 내용은 건드리지 않는다.
        remove_link(path)
    } else if path.is_dir() {
        fs::remove_dir_all(path)
    } else if exists(path) {
        fs::remove_file(path)
    } else {
        Ok(())
    }
}

/// 제자리 이름변경(원본 B-6 인라인 리네임 — 같은 폴더 내 rename). 반환: 새 경로.
/// 규칙: 공백 트림·빈 이름/구분자 포함 = 오류·동일 이름 = 무동작·기존 이름과 충돌 = 오류.
pub fn rename(path: &Path, new_name: &str) -> io::Result<PathBuf> {
    let new_name = new_name.trim();
    validate_name(new_name).map_err(OpsError::io)?;
    let parent = path.parent().ok_or_else(|| OpsError::RootRename.io())?;
    let dest = parent.join(new_name);
    if path_equals(path, &dest) {
        return Ok(path.to_path_buf()); // 동일 이름 = 무동작
    }
    if exists(&dest) {
        return Err(OpsError::NameTaken.io());
    }
    fs::rename(path, &dest)?;
    Ok(dest)
}

/// 새 폴더 생성 — 충돌 없는 이름("base"·"base (2)"…, 원본 UniqueChildPath). 반환: 생성 경로.
pub fn create_new_dir(dir: &Path, base: &str) -> io::Result<PathBuf> {
    let dest = unique_dest(dir, base, true);
    fs::create_dir(&dest)?;
    Ok(dest)
}

/// 새 빈 파일 생성 — `base_with_ext`(예: "새 파일.txt")로 충돌 없는 이름. 반환: 생성 경로.
pub fn create_new_file(dir: &Path, base_with_ext: &str) -> io::Result<PathBuf> {
    let dest = unique_dest(dir, base_with_ext, false);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest)?;
    Ok(dest)
}

/// 전송 진행 스냅샷 — UI 표시용.
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub item_index: usize,
    pub item_count: usize,
}

/// 항목 종결 상태(진행 창 세그먼트 바 — 07-21).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemStatus {
    Done,
    Skipped,
    Failed,
}

/// 전송 이벤트(07-21 — 세그먼트 진행 바·전송 중 대상 잠금을 위해 바이트 진행에서 확장).
#[derive(Debug)]
pub enum Event<'a> {
    /// 전송 시작 전 계획 — 항목별 크기(세그먼트 비율)·총 바이트. 정확히 1회.
    Plan { sizes: &'a [u64], total_bytes: u64 },
    /// 항목 쓰기 시작 — 실제 대상 경로(완료 전 열기/이동 차단용) · `overwrite` = 기존 대상을 교체하는 전송(저널 시작 줄 note · GAP-030).
    ItemStart {
        index: usize,
        dest: &'a Path,
        overwrite: bool,
    },
    /// 바이트 진행(4MB 청크 단위 — `done_bytes`는 전체 누적).
    Bytes(Progress),
    /// 항목 종결(성공/건너뜀/실패·취소).
    ItemEnd { index: usize, status: ItemStatus },
}

/// 항목 하나의 **자리 판정**(docs/33 같은 폴더 규칙 · 자기/하위 이동 금지 · 자연 경로 충돌) — [`transfer`]와 [`plan_transfer`]가
/// 이 한 함수로 판정해 미리보기(dry-run)와 실행이 어긋나지 않는다. 파일 시스템은 존재 여부만 읽는다.
enum Placement {
    /// 같은 폴더 이동 = 무동작.
    NoOp,
    /// 같은 폴더 복사 = 순번 복제 자리(" (2)"…).
    Duplicate(PathBuf),
    /// 자기 자신/하위 폴더로의 폴더 복사/이동 — 실행하면 오류(복사도 막는다 · GAP-038: 안 막으면 자기 안에 자기를 끝없이 복사).
    IntoSelf,
    /// 다른 폴더 → 자연 경로(`exists`면 충돌 → `resolve`).
    Into { dest: PathBuf, exists: bool },
}

fn placement(src: &Path, dest_dir: &Path, op: Op) -> Placement {
    let is_dir = src.is_dir();
    let same_folder = src
        .parent()
        .is_some_and(|parent| path_equals(parent, dest_dir));
    if same_folder {
        return match op {
            Op::Move => Placement::NoOp,
            Op::Copy => Placement::Duplicate(unique_dest(dest_dir, &leaf_name(src), is_dir)),
        };
    }
    if is_dir && is_same_or_sub(src, dest_dir) {
        return Placement::IntoSelf;
    }
    let dest = dest_dir.join(leaf_name(src));
    let exists = exists(&dest);
    // 폴더 충돌 = 보관(DR-37 `DirConflict::KeepBoth`): 묻지 않고 옆에 순번 이름(" (2)"…) — 옛 폴더 불변.
    if exists && is_dir && fastcopy::dir_conflict() == DirConflict::KeepBoth {
        return Placement::Duplicate(unique_dest(dest_dir, &leaf_name(src), true));
    }
    Placement::Into { dest, exists }
}

/// 계획 항목의 결정(dry-run 보고서 · T-97 · 10-24).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlanAction {
    /// 새 자리로 복사/이동(충돌 없음).
    Transfer,
    /// 같은 폴더 복사 = 순번 복제.
    Duplicate,
    /// 대상에 이미 있음 → `resolve` = 덮어쓰기.
    Overwrite,
    /// 대상에 이미 있음 → `resolve` = 건너뜀.
    Skip,
    /// 같은 폴더 이동 = 무동작.
    NoOp,
    /// 실행하면 실패할 항목(자기/하위 폴더 복사/이동) — 사유.
    Error(String),
}

impl PlanAction {
    /// 실제로 바이트를 옮기는 결정인가(계획의 `active_bytes`).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            PlanAction::Transfer | PlanAction::Duplicate | PlanAction::Overwrite
        )
    }
}

/// 계획 항목 — 원본 · 최종 자리 · 크기(폴더 = 재귀 합) · 결정.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlanItem {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub bytes: u64,
    pub action: PlanAction,
}

/// 전송 계획(dry-run) — `total_bytes` = 전 항목 합 · `active_bytes` = 실제로 옮길 양.
#[derive(Default, Debug)]
pub struct TransferPlan {
    pub items: Vec<PlanItem>,
    pub total_bytes: u64,
    pub active_bytes: u64,
}

/// **전송 미리보기**(dry-run · T-97 "dry-run 보고서" · 10-24): [`transfer`]와 같은 판정([`placement`])으로 항목마다 자리 · 결정 ·
/// 크기를 돌려주고 **파일 시스템은 바꾸지 않는다**. 충돌은 똑같이 `resolve`로 묻는다(정책 호출자 = 상수 · 대화형 호출자는 미리보기에서도
/// 물을 수 있다). 한계: 같은 이름의 원본 여럿이 같은 폴더로 순번 복제되는 경우 둘째 이후의 번호는 실행 때 정해진다.
pub fn plan_transfer(
    sources: &[PathBuf],
    dest_dir: &Path,
    op: Op,
    resolve: &mut dyn FnMut(&Path) -> Conflict,
) -> TransferPlan {
    let mut plan = TransferPlan::default();
    for src in sources {
        let bytes = size_of(src);
        let (dest, action) = match placement(src, dest_dir, op) {
            Placement::NoOp => (src.clone(), PlanAction::NoOp),
            Placement::Duplicate(dest) => (dest, PlanAction::Duplicate),
            Placement::IntoSelf => (
                dest_dir.join(leaf_name(src)),
                PlanAction::Error(OpsError::IntoSelf.message().to_string()),
            ),
            Placement::Into {
                dest,
                exists: false,
            } => (dest, PlanAction::Transfer),
            Placement::Into { dest, exists: true } => {
                let action = match resolve(&dest) {
                    Conflict::Overwrite => PlanAction::Overwrite,
                    Conflict::Skip => PlanAction::Skip,
                };
                (dest, action)
            }
        };
        plan.total_bytes += bytes;
        if action.is_active() {
            plan.active_bytes += bytes;
        }
        plan.items.push(PlanItem {
            src: src.clone(),
            dest,
            bytes,
            action,
        });
    }
    plan
}

/// **전송 단일 경로**(원본 TransferPathsInto 이식) — 모든 복사/이동 진입점이 이 함수로 수렴.
///
/// 보장(docs/33): 같은 폴더 규칙(이동=무동작·복사=순번 복제) · 다른 폴더 충돌은 `resolve`로
/// **충돌 항목만 순차** 확인(Overwrite/Skip) · 이벤트 통지(`on_event` — 계획/항목/바이트) ·
/// 취소(`cancel`) · 항목 실패 개별 격리. 호출 측(워커 스레드)이 완료 후 재로드를 수행한다.
pub fn transfer(
    sources: &[PathBuf],
    dest_dir: &Path,
    op: Op,
    resolve: &mut dyn FnMut(&Path) -> Conflict,
    on_event: &mut dyn FnMut(Event),
    cancel: &AtomicBool,
) -> Outcome {
    transfer_with(
        sources,
        dest_dir,
        op,
        resolve,
        on_event,
        cancel,
        &fastcopy::tuning(),
    )
}

/// [`transfer`]와 같되 복사 전략 조절값을 **인자로**(전역 `set_tuning`을 건드리지 않는다 — 시험 · CLI · 병렬 호출). 이 스레드에서
/// 도는 동안만 `tuning()`이 `t`를 돌려준다([`fastcopy::with_tuning`]).
pub fn transfer_with(
    sources: &[PathBuf],
    dest_dir: &Path,
    op: Op,
    resolve: &mut dyn FnMut(&Path) -> Conflict,
    on_event: &mut dyn FnMut(Event),
    cancel: &AtomicBool,
    t: &fastcopy::Tuning,
) -> Outcome {
    fastcopy::with_tuning(*t, || {
        transfer_inner(sources, dest_dir, op, resolve, on_event, cancel)
    })
}

fn transfer_inner(
    sources: &[PathBuf],
    dest_dir: &Path,
    op: Op,
    resolve: &mut dyn FnMut(&Path) -> Conflict,
    on_event: &mut dyn FnMut(Event),
    cancel: &AtomicBool,
) -> Outcome {
    let mut out = Outcome::default();
    let sizes: Vec<u64> = sources.iter().map(|p| size_of(p)).collect();
    let total_bytes: u64 = sizes.iter().sum();
    on_event(Event::Plan {
        sizes: &sizes,
        total_bytes,
    });
    // T-208 후속(11-09): 지난 실행(죽은 pid)이 대상 폴더에 남긴 `tmp` 잔여를 전송 시작 때 1회 치운다 — 저널 없는 재실행(`nexa-copy` 옵션 없음 · 앱의
    // 같은 복사 반복)도 숨은 잔여를 쌓지 않는다. `old`는 두는 것이 안전(병합 중 치워 둔 옛 대상 · 재개가 끝난 뒤 치운다).
    sweep_staging_leftovers(dest_dir, &["tmp"]);
    let item_count = sources.len();
    let mut done = 0u64;
    for (i, src) in sources.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            out.canceled = true;
            break;
        }
        let mut verify_paths: Vec<PathBuf> = Vec::new(); // 검증 목록(GAP-027 · 수집 깃발이 켜진 때만 채워짐)
        let result: io::Result<Option<PathBuf>> = (|| {
            // 자리 판정은 `placement`(dry-run `plan_transfer`와 같은 함수 · 10-24).
            let (dest, overwrite) = match placement(src, dest_dir, op) {
                Placement::NoOp => return Ok(None), // 제자리 이동 = 무동작(원본 규칙)
                Placement::Duplicate(dest) => (dest, false), // 같은 폴더 복사 = 순번 복제(" (2)"…)
                Placement::IntoSelf => return Err(OpsError::IntoSelf.io()),
                Placement::Into { dest, exists } => {
                    let overwrite = exists
                        && match resolve(&dest) {
                            Conflict::Skip => return Ok(None),
                            Conflict::Overwrite => true,
                        };
                    (dest, overwrite)
                }
            };
            on_event(Event::ItemStart {
                index: i,
                dest: &dest,
                overwrite,
            });
            let mut bytes = |d: u64| {
                done += d;
                on_event(Event::Bytes(Progress {
                    done_bytes: done,
                    total_bytes,
                    item_index: i,
                    item_count,
                }));
            };
            match op {
                Op::Copy => {
                    if fastcopy::verify_collect() {
                        // 검증 목록(GAP-027): 이 항목이 실제로 옮긴 원본 경로를 모은다(파일 항목 = 자기 자신 · 폴더 = plan/copy_tree가 적음).
                        let (r, list) = fastcopy::collect_verify(|| {
                            copy_onto_with_progress(src, &dest, overwrite, &mut bytes, cancel)
                        });
                        r?;
                        verify_paths.extend(list);
                    } else {
                        copy_onto_with_progress(src, &dest, overwrite, &mut bytes, cancel)?;
                    }
                }
                Op::Move => move_onto_with_progress(src, &dest, overwrite, &mut bytes, cancel)?,
            }
            Ok(Some(dest))
        })();

        let status = match &result {
            Ok(Some(_)) => ItemStatus::Done,
            Ok(None) => ItemStatus::Skipped,
            Err(_) => ItemStatus::Failed,
        };
        on_event(Event::ItemEnd { index: i, status });
        match result {
            Ok(Some(dest)) => {
                out.transferred.push((src.clone(), dest));
                out.copied_paths.append(&mut verify_paths);
            }
            Ok(None) => out.skipped.push(src.clone()),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                out.canceled = true;
                break;
            }
            Err(e) => out.errors.push((src.clone(), e.to_string())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 취소 계약 시험용 조절값 — 읽고 쓰는 루프(OS 블록 복제는 즉시 끝나 취소할 틈이 없다).
    const LOOP: fastcopy::Tuning = fastcopy::Tuning {
        native: false,
        unbuffered_min: 0,
        threads: 1,
    };
    use std::sync::atomic::AtomicBool;

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nexa_ops_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_conflict(_: &Path) -> Conflict {
        panic!("충돌이 없어야 함")
    }

    fn run(
        sources: &[PathBuf],
        dest: &Path,
        op: Op,
        resolve: &mut dyn FnMut(&Path) -> Conflict,
    ) -> Outcome {
        let cancel = AtomicBool::new(false);
        transfer(sources, dest, op, resolve, &mut |_| {}, &cancel)
    }

    /// 이름 검사(T-162 · MC/DC · Windows 규칙 on/off): 공통 거부(빈 · `.` · 구분자 · NUL · 255자 초과) · Windows만(금지 문자 · 끝 빈칸/점 · 예약 이름
    /// 대소문자/확장자 무관 · COM0는 예약 아님) · Unix는 `CON`도 허용 · 유니코드 255자는 통과.
    #[test]
    fn validate_name_rules_per_os() {
        for win in [true, false] {
            assert_eq!(validate_name_for("", win), Err(OpsError::BadName));
            assert_eq!(validate_name_for(".", win), Err(OpsError::BadName));
            assert_eq!(validate_name_for("..", win), Err(OpsError::BadName));
            assert_eq!(validate_name_for("a/b", win), Err(OpsError::BadName));
            assert_eq!(validate_name_for("a\\b", win), Err(OpsError::BadName));
            assert_eq!(validate_name_for("a\0b", win), Err(OpsError::BadName));
            assert_eq!(
                validate_name_for(&"가".repeat(256), win),
                Err(OpsError::NameTooLong)
            );
            assert_eq!(validate_name_for(&"가".repeat(255), win), Ok(()));
            assert_eq!(validate_name_for("normal.txt", win), Ok(()));
            assert_eq!(validate_name_for(".gitignore", win), Ok(()));
        }
        // Windows 규칙.
        for bad in ["a:b", "a<b", "a>b", "a\"b", "a|b", "a?b", "a*b", "a\u{7}b"] {
            assert_eq!(
                validate_name_for(bad, true),
                Err(OpsError::BadName),
                "{bad}"
            );
            assert_eq!(validate_name_for(bad, false), Ok(()), "{bad}");
        }
        for res in [
            "CON",
            "con.txt",
            "Prn",
            "AUX.log.bak",
            "NUL",
            "COM1",
            "lpt9.txt",
            "trailing ",
            "dot.",
            "CON .txt",
            "nul.tar.gz",
            "COM\u{b9}",
            "lpt\u{b3}.log",
        ] {
            assert_eq!(
                validate_name_for(res, true),
                Err(OpsError::ReservedName),
                "{res}"
            );
            assert_eq!(validate_name_for(res, false), Ok(()), "{res}");
        }
        for ok in ["COM0", "COM10", "CONSOLE", "lpt", "a.b.c", "x ", "x."] {
            let r = validate_name_for(ok, true);
            if ok.ends_with(' ') || ok.ends_with('.') {
                assert_eq!(r, Err(OpsError::ReservedName), "{ok}");
            } else {
                assert_eq!(r, Ok(()), "{ok}");
            }
        }
        // rename이 같은 규칙.
        let d = fixture("valname");
        fs::write(d.join("a.txt"), "x").unwrap();
        let e = rename(&d.join("a.txt"), "..").unwrap_err();
        assert_eq!(OpsError::of(&e.to_string()), Some(OpsError::BadName));
        if cfg!(windows) {
            let e = rename(&d.join("a.txt"), "con").unwrap_err();
            assert_eq!(OpsError::of(&e.to_string()), Some(OpsError::ReservedName));
        }
        fs::remove_dir_all(&d).unwrap();
    }

    /// 엔진 오류 어휘(10-24): 문구는 영어 · `of`는 접두 일치(꼬리 `: 세부` 허용) · 모르는 문구 = None · 종류별 ErrorKind · i18n 키 고유.
    #[test]
    fn ops_error_messages_round_trip() {
        for k in OpsError::ALL {
            assert_eq!(OpsError::of(k.message()), Some(k));
            assert_eq!(OpsError::of(&k.io().to_string()), Some(k));
            assert_eq!(OpsError::of(&k.io_with("x/y").to_string()), Some(k));
            assert!(k.message().is_ascii(), "영어 문구: {k:?}");
            assert!(k.i18n_key().starts_with("ops.err."));
        }
        let keys: std::collections::HashSet<&str> =
            OpsError::ALL.iter().map(|k| k.i18n_key()).collect();
        assert_eq!(keys.len(), OpsError::ALL.len(), "i18n 키 고유");
        assert_eq!(OpsError::of("Access is denied. (os error 5)"), None);
        assert_eq!(
            OpsError::NameTaken.io().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(OpsError::BadName.io().kind(), io::ErrorKind::InvalidInput);
        assert_eq!(OpsError::SourceMissing.io().kind(), io::ErrorKind::NotFound);
        // rename 오류가 어휘를 쓴다.
        let d = fixture("opserr");
        fs::write(d.join("a.txt"), "x").unwrap();
        fs::write(d.join("b.txt"), "y").unwrap();
        let e = rename(&d.join("a.txt"), "b.txt").unwrap_err();
        assert_eq!(OpsError::of(&e.to_string()), Some(OpsError::NameTaken));
        let e = rename(&d.join("a.txt"), "x/y").unwrap_err();
        assert_eq!(OpsError::of(&e.to_string()), Some(OpsError::BadName));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn unique_dest_numbering_file_and_dir() {
        let d = fixture("uniq");
        fs::write(d.join("a.txt"), "x").unwrap();
        assert_eq!(unique_dest(&d, "a.txt", false), d.join("a (2).txt"));
        fs::write(d.join("a (2).txt"), "x").unwrap();
        assert_eq!(unique_dest(&d, "a.txt", false), d.join("a (3).txt"));
        fs::create_dir(d.join("v1.2")).unwrap();
        assert_eq!(
            unique_dest(&d, "v1.2", true),
            d.join("v1.2 (2)"),
            "폴더는 확장자 분리 안 함"
        );
        assert_eq!(
            unique_dest(&d, "new.txt", false),
            d.join("new.txt"),
            "무충돌 = 자연 경로"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn same_folder_rules_move_noop_copy_duplicates() {
        let d = fixture("samefolder");
        fs::write(d.join("f.txt"), "내용").unwrap();
        let srcs = vec![d.join("f.txt")];

        let out = run(&srcs, &d, Op::Move, &mut no_conflict);
        assert!(
            out.transferred.is_empty() && out.errors.is_empty(),
            "제자리 이동 = 무동작"
        );
        assert!(d.join("f.txt").exists());

        let out = run(&srcs, &d, Op::Copy, &mut no_conflict);
        assert_eq!(
            out.transferred[0].1,
            d.join("f (2).txt"),
            "같은 폴더 복사 = 순번 복제"
        );
        assert_eq!(fs::read_to_string(d.join("f (2).txt")).unwrap(), "내용");
        fs::remove_dir_all(&d).unwrap();
    }

    /// dry-run(`plan_transfer` · 10-24): 실행과 같은 판정(충돌 → resolve · 같은 폴더 · 자기/하위 이동) · 크기 합 · 파일 시스템 불변 ·
    /// 실제 `transfer` 결과와 자리가 일치한다.
    #[test]
    fn plan_transfer_mirrors_transfer_decisions_without_touching() {
        let d = fixture("plan");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(a.join("sub")).unwrap();
        fs::create_dir(&b).unwrap();
        fs::write(a.join("f.txt"), "1").unwrap();
        fs::write(a.join("sub/g.txt"), "22").unwrap();
        fs::write(b.join("f.txt"), "old").unwrap();
        let srcs = vec![a.join("f.txt"), a.join("sub")];

        let p = plan_transfer(&srcs, &b, Op::Copy, &mut |_| Conflict::Skip);
        assert_eq!(p.items.len(), 2);
        assert_eq!(
            (&p.items[0].action, &p.items[0].dest, p.items[0].bytes),
            (&PlanAction::Skip, &b.join("f.txt"), 1)
        );
        assert_eq!(
            (&p.items[1].action, &p.items[1].dest, p.items[1].bytes),
            (&PlanAction::Transfer, &b.join("sub"), 2)
        );
        assert_eq!((p.total_bytes, p.active_bytes), (3, 2));
        let p2 = plan_transfer(&srcs, &b, Op::Copy, &mut |_| Conflict::Overwrite);
        assert_eq!(p2.items[0].action, PlanAction::Overwrite);
        assert_eq!(p2.active_bytes, 3);
        // 같은 폴더: 복사 = 순번 복제 · 이동 = 무동작 · 자기/하위 이동 = 오류.
        let same = plan_transfer(&srcs[..1], &a, Op::Copy, &mut no_conflict);
        assert_eq!(
            (&same.items[0].action, &same.items[0].dest),
            (&PlanAction::Duplicate, &a.join("f (2).txt"))
        );
        let noop = plan_transfer(&srcs[..1], &a, Op::Move, &mut no_conflict);
        assert_eq!(noop.items[0].action, PlanAction::NoOp);
        assert_eq!(noop.active_bytes, 0);
        let into_self = plan_transfer(
            std::slice::from_ref(&a),
            &a.join("sub"),
            Op::Move,
            &mut no_conflict,
        );
        assert!(matches!(into_self.items[0].action, PlanAction::Error(_)));
        assert!(!into_self.items[0].action.is_active());
        // 아무것도 바뀌지 않았다.
        assert!(!b.join("sub").exists() && !a.join("f (2).txt").exists());
        assert_eq!(fs::read_to_string(b.join("f.txt")).unwrap(), "old");
        // 실행 결과와 일치.
        let out = run(&srcs, &b, Op::Copy, &mut |_| Conflict::Skip);
        assert_eq!(out.skipped, vec![a.join("f.txt")]);
        assert_eq!(out.transferred, vec![(a.join("sub"), b.join("sub"))]);
        fs::remove_dir_all(&d).unwrap();
    }

    /// GAP-038(11-02): 폴더의 자기/하위 **복사**도 거절(종전 = 이동만 · 복사는 자기 안에 자기를 끝없이 복사) — 계획 = `Error` ·
    /// 실행 = 항목 격리 실패 · 파일 시스템 불변. 같은 폴더(부모) 복사는 종전대로 순번 복제.
    #[test]
    fn copy_into_self_or_sub_is_refused() {
        let d = fixture("intoself");
        let a = d.join("a");
        fs::create_dir_all(a.join("sub")).unwrap();
        fs::write(a.join("f.txt"), "1").unwrap();
        for dest in [a.clone(), a.join("sub")] {
            let plan = plan_transfer(std::slice::from_ref(&a), &dest, Op::Copy, &mut no_conflict);
            assert!(
                matches!(plan.items[0].action, PlanAction::Error(_)),
                "{dest:?}: {:?}",
                plan.items[0].action
            );
            let out = run(std::slice::from_ref(&a), &dest, Op::Copy, &mut no_conflict);
            assert_eq!(out.errors.len(), 1, "{dest:?}");
            assert!(
                out.errors[0].1.contains("into itself"),
                "{}",
                out.errors[0].1
            );
            assert!(out.transferred.is_empty());
        }
        assert!(
            !a.join("a").exists() && !a.join("sub/a").exists(),
            "자기 안에 사본 없음"
        );
        // 부모로의 복사 = 순번 복제(종전 규칙 유지).
        let out = run(std::slice::from_ref(&a), &d, Op::Copy, &mut no_conflict);
        assert_eq!(out.transferred, vec![(a, d.join("a (2)"))]);
        assert_eq!(fs::read_to_string(d.join("a (2)/f.txt")).unwrap(), "1");
        fs::remove_dir_all(&d).unwrap();
    }

    /// 폴더 충돌 처리 3종(DR-37 · 사용자 11-02): 병합(기본) = 같은 이름만 교체 · 대상에만 있던 것 유지 / 교체 = 옛 폴더 통째 소멸 /
    /// 보관 = 묻지 않고 "a (2)" 옆에(옛 폴더 불변 · 계획 `Duplicate`). 복사(스테이징 커밋)와 같은 볼륨 이동(rename) 둘 다.
    #[test]
    fn dir_conflict_modes_replace_merge_keep() {
        for op in [Op::Copy, Op::Move] {
            for mode in [
                DirConflict::Merge,
                DirConflict::Replace,
                DirConflict::KeepBoth,
            ] {
                let d = fixture(&format!("dirconf_{op:?}_{mode:?}"));
                let (src, dst) = (d.join("src"), d.join("dst"));
                fs::create_dir_all(src.join("a/deep")).unwrap();
                fs::write(src.join("a/f.txt"), "new").unwrap();
                fs::write(src.join("a/deep/n.txt"), "n").unwrap();
                fs::create_dir_all(dst.join("a")).unwrap();
                fs::write(dst.join("a/f.txt"), "old").unwrap();
                fs::write(dst.join("a/only.txt"), "keep").unwrap();
                let srcs = vec![src.join("a")];
                let mut asked = 0;
                let (plan, out) = fastcopy::with_dir_conflict(mode, || {
                    let plan = plan_transfer(&srcs, &dst, op, &mut |_| Conflict::Overwrite);
                    let out = run(&srcs, &dst, op, &mut |_| {
                        asked += 1;
                        Conflict::Overwrite
                    });
                    (plan, out)
                });
                assert!(out.errors.is_empty(), "{op:?} {mode:?}: {:?}", out.errors);
                let a = dst.join("a");
                match mode {
                    DirConflict::Merge => {
                        assert_eq!(plan.items[0].action, PlanAction::Overwrite);
                        assert_eq!(asked, 1);
                        assert_eq!(fs::read_to_string(a.join("f.txt")).unwrap(), "new");
                        assert_eq!(fs::read_to_string(a.join("deep/n.txt")).unwrap(), "n");
                        assert_eq!(fs::read_to_string(a.join("only.txt")).unwrap(), "keep");
                    }
                    DirConflict::Replace => {
                        assert_eq!(plan.items[0].action, PlanAction::Overwrite);
                        assert_eq!(asked, 1);
                        assert_eq!(fs::read_to_string(a.join("f.txt")).unwrap(), "new");
                        assert_eq!(fs::read_to_string(a.join("deep/n.txt")).unwrap(), "n");
                        assert!(!a.join("only.txt").exists(), "교체 = 대상에만 있던 것 소멸");
                    }
                    DirConflict::KeepBoth => {
                        assert_eq!(
                            (&plan.items[0].action, &plan.items[0].dest),
                            (&PlanAction::Duplicate, &dst.join("a (2)"))
                        );
                        assert_eq!(asked, 0, "보관 = 묻지 않음");
                        assert_eq!(fs::read_to_string(a.join("f.txt")).unwrap(), "old");
                        assert_eq!(fs::read_to_string(a.join("only.txt")).unwrap(), "keep");
                        let b = dst.join("a (2)");
                        assert_eq!(fs::read_to_string(b.join("f.txt")).unwrap(), "new");
                        assert_eq!(fs::read_to_string(b.join("deep/n.txt")).unwrap(), "n");
                    }
                }
                // 스테이징 잔여 없음 · 이동이면 원본 소멸.
                let strays: Vec<_> = fs::read_dir(&dst)
                    .unwrap()
                    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|n| n.contains(".nexa-"))
                    .collect();
                assert!(strays.is_empty(), "{op:?} {mode:?}: {strays:?}");
                assert_eq!(src.join("a").exists(), op == Op::Copy, "{op:?} {mode:?}");
                fs::remove_dir_all(&d).unwrap();
            }
        }
        assert_eq!(DirConflict::parse("keep"), Some(DirConflict::KeepBoth));
        assert_eq!(DirConflict::parse("x"), None);
        assert_eq!(DirConflict::default().as_str(), "merge");
    }

    #[test]
    fn cross_folder_copy_move_and_dir_recursive() {
        let d = fixture("cross");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(a.join("sub")).unwrap();
        fs::create_dir(&b).unwrap();
        fs::write(a.join("f.txt"), "1").unwrap();
        fs::write(a.join("sub/g.txt"), "22").unwrap();

        // 폴더 복사(재귀)
        let out = run(std::slice::from_ref(&a), &b, Op::Copy, &mut no_conflict);
        assert_eq!(out.transferred.len(), 1);
        assert_eq!(fs::read_to_string(b.join("a/sub/g.txt")).unwrap(), "22");

        // 파일 이동(동일 볼륨 fast path) — 원본 소멸
        let out = run(&[a.join("f.txt")], &b, Op::Move, &mut no_conflict);
        assert_eq!(out.transferred[0].1, b.join("f.txt"));
        assert!(!a.join("f.txt").exists());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn conflict_skip_and_overwrite_sequential() {
        let d = fixture("conflict");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("f.txt"), "새값").unwrap();
        fs::write(a.join("g.txt"), "새값g").unwrap();
        fs::write(b.join("f.txt"), "옛값").unwrap();
        fs::write(b.join("g.txt"), "옛값g").unwrap();

        let mut asked = Vec::new();
        let out = run(
            &[a.join("f.txt"), a.join("g.txt")],
            &b,
            Op::Copy,
            &mut |p| {
                asked.push(leaf_name(p));
                if p.ends_with("f.txt") {
                    Conflict::Overwrite
                } else {
                    Conflict::Skip
                }
            },
        );
        assert_eq!(asked, vec!["f.txt", "g.txt"], "충돌 항목만 순차 확인");
        assert_eq!(
            fs::read_to_string(b.join("f.txt")).unwrap(),
            "새값",
            "덮어씀"
        );
        assert_eq!(
            fs::read_to_string(b.join("g.txt")).unwrap(),
            "옛값g",
            "건너뜀"
        );
        assert_eq!(out.transferred.len(), 1);
        assert_eq!(out.skipped, vec![a.join("g.txt")]);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn cycle_move_is_isolated_error() {
        let d = fixture("cycle");
        let outer = d.join("outer");
        fs::create_dir_all(outer.join("inner")).unwrap();
        fs::write(d.join("ok.txt"), "x").unwrap();
        let out = run(
            &[outer.clone(), d.join("ok.txt")],
            &outer.join("inner"),
            Op::Move,
            &mut no_conflict,
        );
        assert_eq!(out.errors.len(), 1, "순환 이동은 오류");
        assert_eq!(out.errors[0].0, outer);
        assert_eq!(out.transferred.len(), 1, "다른 항목은 개별 격리로 계속");
        assert!(outer.join("inner/ok.txt").exists());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn progress_reports_bytes_up_to_total() {
        let d = fixture("progress");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("f.bin"), vec![7u8; 100_000]).unwrap();
        let cancel = AtomicBool::new(false);
        let mut last = None;
        let mut plan: Option<(Vec<u64>, u64)> = None;
        let mut starts: Vec<(usize, PathBuf)> = Vec::new();
        let mut ends: Vec<(usize, ItemStatus)> = Vec::new();
        let out = transfer(
            &[a.join("f.bin")],
            &b,
            Op::Copy,
            &mut no_conflict,
            &mut |ev| match ev {
                Event::Plan { sizes, total_bytes } => plan = Some((sizes.to_vec(), total_bytes)),
                Event::ItemStart { index, dest, .. } => starts.push((index, dest.to_path_buf())),
                Event::Bytes(p) => last = Some(p),
                Event::ItemEnd { index, status } => ends.push((index, status)),
            },
            &cancel,
        );
        assert_eq!(out.transferred.len(), 1);
        let p = last.unwrap();
        assert_eq!(p.done_bytes, 100_000);
        assert_eq!(p.total_bytes, 100_000);
        assert_eq!((p.item_index, p.item_count), (0, 1));
        // 이벤트 프로토콜(07-21): 계획(항목 크기·총합) → 시작(대상 경로) → 종결(Done)
        assert_eq!(plan, Some((vec![100_000], 100_000)), "Plan 1회·크기 정확");
        assert_eq!(starts, vec![(0, b.join("f.bin"))], "ItemStart = 실제 대상");
        assert_eq!(ends, vec![(0, ItemStatus::Done)]);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn item_events_report_skip_per_item() {
        let d = fixture("itemevents");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("f.txt"), "새값").unwrap();
        fs::write(b.join("f.txt"), "옛값").unwrap();
        fs::write(a.join("g.txt"), "g").unwrap();
        let cancel = AtomicBool::new(false);
        let mut ends = Vec::new();
        let out = transfer(
            &[a.join("f.txt"), a.join("g.txt")],
            &b,
            Op::Copy,
            &mut |_| Conflict::Skip,
            &mut |ev| {
                if let Event::ItemEnd { index, status } = ev {
                    ends.push((index, status));
                }
            },
            &cancel,
        );
        assert_eq!(out.skipped.len(), 1);
        assert_eq!(
            ends,
            vec![(0, ItemStatus::Skipped), (1, ItemStatus::Done)],
            "건너뜀/성공이 항목별로 보고"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    /// 점검 1차 #1: 덮어쓰기 중 취소해도 **옛 대상이 보존**되고 스테이징 잔여가 없어야 한다.
    #[test]
    fn overwrite_keeps_old_dest_when_canceled_and_replaces_on_success() {
        let d = fixture("overwrite_stage");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(a.join("x")).unwrap();
        fs::create_dir_all(b.join("x")).unwrap();
        fs::write(a.join("x").join("big.bin"), vec![1u8; 9 * 1024 * 1024]).unwrap();
        fs::write(b.join("x").join("keep.txt"), "옛 폴더 내용").unwrap();
        fs::write(a.join("f.bin"), vec![2u8; 9 * 1024 * 1024]).unwrap();
        fs::write(b.join("f.bin"), "옛 파일").unwrap();
        let leftovers = |dir: &Path| {
            fs::read_dir(dir)
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().contains(".nexa-"))
                .count()
        };
        // ① 폴더 덮어쓰기 중 취소 → 옛 폴더 그대로. 취소 계약 시험은 OS 블록 복제(APFS clonefile · reflink — 즉시 끝나 취소 틈이
        // 없다 · macOS CI 10-24)를 피해 루프 경로를 **인자로**(전역 조절값은 건드리지 않는다).
        let cancel = AtomicBool::new(false);
        let out = transfer_with(
            &[a.join("x")],
            &b,
            Op::Copy,
            &mut |_| Conflict::Overwrite,
            &mut |ev| {
                if matches!(ev, Event::Bytes(_)) {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
            &LOOP,
        );
        assert!(out.canceled);
        assert_eq!(
            fs::read_to_string(b.join("x").join("keep.txt")).unwrap(),
            "옛 폴더 내용"
        );
        assert!(!b.join("x").join("big.bin").exists());
        assert_eq!(leftovers(&b), 0, "스테이징 잔여 없음");
        // ② 파일 덮어쓰기 중 취소 → 옛 파일 그대로
        let cancel = AtomicBool::new(false);
        let out = transfer_with(
            &[a.join("f.bin")],
            &b,
            Op::Copy,
            &mut |_| Conflict::Overwrite,
            &mut |ev| {
                if matches!(ev, Event::Bytes(_)) {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
            &LOOP,
        );
        assert!(out.canceled);
        assert_eq!(fs::read_to_string(b.join("f.bin")).unwrap(), "옛 파일");
        assert_eq!(leftovers(&b), 0);
        // ③ 성공 시 교체 완료(폴더·파일) — 옛 내용 소거·잔여 없음
        fs::write(a.join("x").join("big.bin"), b"new").unwrap();
        fs::write(a.join("f.bin"), b"newf").unwrap();
        let out = run(&[a.join("x"), a.join("f.bin")], &b, Op::Copy, &mut |_| {
            Conflict::Overwrite
        });
        assert_eq!(out.transferred.len(), 2, "{:?}", out.errors);
        assert!(
            b.join("x").join("keep.txt").exists(),
            "병합(11-01): 옛 폴더에만 있던 것은 남는다"
        );
        assert_eq!(fs::read(b.join("x").join("big.bin")).unwrap(), b"new");
        assert_eq!(fs::read(b.join("f.bin")).unwrap(), b"newf");
        assert_eq!(leftovers(&b), 0);
        // ④ 같은 볼륨 이동 덮어쓰기(폴더) — 옛 폴더 옆으로 치운 뒤 rename·정리
        fs::create_dir_all(a.join("y")).unwrap();
        fs::write(a.join("y").join("n.txt"), "새").unwrap();
        fs::create_dir_all(b.join("y")).unwrap();
        fs::write(b.join("y").join("o.txt"), "옛").unwrap();
        let out = run(&[a.join("y")], &b, Op::Move, &mut |_| Conflict::Overwrite);
        assert_eq!(out.transferred.len(), 1, "{:?}", out.errors);
        assert!(
            b.join("y").join("n.txt").exists() && b.join("y").join("o.txt").exists(),
            "같은 볼륨 rename 교체도 병합"
        );
        assert!(!a.join("y").exists());
        assert_eq!(leftovers(&b), 0);
        fs::remove_dir_all(&d).unwrap();
    }

    /// 덮어쓰기 = 병합(DR-35 후보 · 11-01 · 협업 d4 재현 차단): 부분 원본(파일 1개)을 전체 대상(파일 3개)에 덮어쓰기 이동/복사해도 대상에만 있던
    /// 파일은 남고 같은 이름만 교체 · 하위 폴더도 재귀 병합 · 링크는 링크로 · 잔여 없음.
    #[test]
    fn overwrite_merges_destination_only_items() {
        let d = fixture("merge");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(a.join("t/sub")).unwrap();
        fs::write(a.join("t/same.txt"), "new").unwrap();
        fs::write(a.join("t/sub/n.txt"), "new sub").unwrap();
        fs::create_dir_all(b.join("t/sub")).unwrap();
        fs::write(b.join("t/same.txt"), "old").unwrap();
        fs::write(b.join("t/only1.txt"), "keep 1").unwrap();
        fs::write(b.join("t/only2.txt"), "keep 2").unwrap();
        fs::write(b.join("t/sub/o.txt"), "keep sub").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(b.join("t/only1.txt"), b.join("t/ln")).unwrap();
        let never = AtomicBool::new(false);
        let leftovers = |dir: &Path| -> usize {
            fs::read_dir(dir)
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().contains(".nexa-"))
                .count()
        };
        // 복사(스테이징 교체 경로).
        copy_onto_with_progress(&a.join("t"), &b.join("t"), true, &mut |_| {}, &never).unwrap();
        assert_eq!(fs::read_to_string(b.join("t/same.txt")).unwrap(), "new");
        assert_eq!(fs::read_to_string(b.join("t/only1.txt")).unwrap(), "keep 1");
        assert_eq!(fs::read_to_string(b.join("t/only2.txt")).unwrap(), "keep 2");
        assert_eq!(
            fs::read_to_string(b.join("t/sub/o.txt")).unwrap(),
            "keep sub"
        );
        assert_eq!(
            fs::read_to_string(b.join("t/sub/n.txt")).unwrap(),
            "new sub"
        );
        #[cfg(unix)]
        assert!(is_link(&b.join("t/ln")), "링크는 링크로 남는다");
        assert_eq!(leftovers(&b), 0, "스테이징·old 잔여 없음");
        // 이동(교차 볼륨 = move_by_copy · 같은 볼륨 = rename_onto 둘 다 병합) — d4: 부분 원본 재실행.
        fs::write(a.join("t/same.txt"), "newer").unwrap();
        move_by_copy(&a.join("t"), &b.join("t"), true, &mut |_| {}, &never).unwrap();
        assert!(!a.join("t").exists());
        assert_eq!(fs::read_to_string(b.join("t/same.txt")).unwrap(), "newer");
        assert!(b.join("t/only2.txt").exists() && b.join("t/sub/o.txt").exists());
        fs::create_dir_all(a.join("t")).unwrap();
        fs::write(a.join("t/same.txt"), "newest").unwrap();
        move_onto_with_progress(&a.join("t"), &b.join("t"), true, &mut |_| {}, &never).unwrap();
        assert_eq!(fs::read_to_string(b.join("t/same.txt")).unwrap(), "newest");
        assert!(b.join("t/only1.txt").exists() && b.join("t/sub/n.txt").exists());
        assert_eq!(leftovers(&b), 0);
        // 지난 실행(죽은 pid)의 잔여는 다음 성공 뒤 치운다(협업 m7) — 살아 있는 pid 것은 보호.
        fs::create_dir_all(b.join(".t.nexa-old-4000000001-0")).unwrap();
        fs::write(b.join(".t.nexa-old-4000000001-0/x"), "stale").unwrap();
        fs::write(b.join(".t.nexa-tmp-4000000002-0"), "stale").unwrap();
        fs::create_dir_all(a.join("t")).unwrap();
        fs::write(a.join("t/same.txt"), "again").unwrap();
        copy_onto_with_progress(&a.join("t"), &b.join("t"), true, &mut |_| {}, &never).unwrap();
        assert_eq!(leftovers(&b), 0, "죽은 pid 잔여는 성공 뒤 정리");
        // 파일 ↔ 폴더 이름 충돌(협업 m4): 파일로 폴더 덮어쓰기 · 폴더로 파일 덮어쓰기 = 그 항목 실패 · 대상 그대로 · 잔여 없음.
        fs::remove_dir_all(a.join("t")).unwrap();
        fs::write(a.join("t"), "a file now").unwrap();
        let e = copy_onto_with_progress(&a.join("t"), &b.join("t"), true, &mut |_| {}, &never)
            .unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{e}");
        assert!(b.join("t/only1.txt").exists(), "폴더 그대로");
        fs::create_dir_all(a.join("f")).unwrap();
        fs::write(a.join("f/x"), "x").unwrap();
        fs::write(b.join("f"), "a file").unwrap();
        assert!(
            copy_onto_with_progress(&a.join("f"), &b.join("f"), true, &mut |_| {}, &never).is_err()
        );
        assert_eq!(fs::read_to_string(b.join("f")).unwrap(), "a file");
        // 병합 안쪽의 파일 ↔ 폴더 충돌 = 실패 + 되돌림(대상 전용 · 옛 파일 그대로 · 새 항목 없음).
        fs::remove_file(a.join("t")).unwrap();
        fs::create_dir_all(a.join("t/sub")).unwrap();
        fs::write(a.join("t/sub/n.txt"), "n2").unwrap();
        fs::write(a.join("t/clash"), "file").unwrap();
        fs::create_dir_all(b.join("t/clash")).unwrap();
        fs::write(b.join("t/clash/inner"), "inner").unwrap();
        assert!(
            copy_onto_with_progress(&a.join("t"), &b.join("t"), true, &mut |_| {}, &never).is_err()
        );
        assert_eq!(
            fs::read_to_string(b.join("t/sub/n.txt")).unwrap(),
            "new sub",
            "되돌림"
        );
        assert!(b.join("t/clash/inner").exists() && b.join("t/only1.txt").exists());
        assert_eq!(leftovers(&b), 0);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn cancel_mid_copy_cleans_partial_and_reports() {
        let d = fixture("cancel");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        // 4MB 청크 2개 이상이 되도록 9MB — 첫 청크 보고에서 취소. 엔진 계약(취소 = `canceled` · 부분 파일 정리) 시험이라 OS 블록 복제
        // (APFS clonefile · reflink — 즉시 끝나 중단할 틈이 없다 · macOS CI 10-24)를 피해 **루프 경로**를 인자로(`transfer_with` · 전역 불변).
        fs::write(a.join("big.bin"), vec![1u8; 9 * 1024 * 1024]).unwrap();
        let cancel = AtomicBool::new(false);
        let out = transfer_with(
            &[a.join("big.bin")],
            &b,
            Op::Copy,
            &mut no_conflict,
            &mut |ev| {
                if matches!(ev, Event::Bytes(_)) {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
            &LOOP,
        );
        assert!(out.canceled);
        assert!(out.transferred.is_empty());
        assert!(!b.join("big.bin").exists(), "부분 파일 정리");
        assert_eq!(
            fs::read_dir(&b).unwrap().count(),
            0,
            "스테이징 잔여 없음(T-208)"
        );
        assert!(a.join("big.bin").exists(), "원본 무손상");
        fs::remove_dir_all(&d).unwrap();
    }

    /// T-208(11-09): 새 대상(파일 · 폴더)도 스테이징 `.<이름>.nexa-tmp-*`에 쓰고 끝에 커밋한다 — 바이트가 흐르는 동안 최종 이름은 없고
    /// 스테이징만 있다 · 끝나면 최종 이름만 남는다(잔여 0). 같은 폴더 복제 " (2)"도 같은 경로. 루프 경로 인자(블록 복제는 틈이 없다).
    #[test]
    fn new_dest_is_staged_until_commit() {
        let d = fixture("stagenew");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(a.join("dir")).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("big.bin"), vec![1u8; 9 * 1024 * 1024]).unwrap();
        fs::write(a.join("dir/big.bin"), vec![2u8; 9 * 1024 * 1024]).unwrap();
        let listing = |p: &Path| -> Vec<String> {
            let mut v: Vec<String> = fs::read_dir(p)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            v.sort();
            v
        };
        let seen = std::cell::RefCell::new(Vec::<Vec<String>>::new());
        let cancel = AtomicBool::new(false);
        let out = transfer_with(
            &[a.join("big.bin"), a.join("dir")],
            &b,
            Op::Copy,
            &mut no_conflict,
            &mut |ev| {
                if matches!(ev, Event::Bytes(_)) {
                    seen.borrow_mut().push(listing(&b));
                }
            },
            &cancel,
            &LOOP,
        );
        assert!(!out.canceled && out.errors.is_empty(), "{:?}", out.errors);
        let seen = seen.into_inner();
        assert!(!seen.is_empty(), "바이트 보고가 있어야 중간 상태를 본다");
        for names in &seen {
            for n in names {
                assert!(
                    n.starts_with(".big.bin.nexa-tmp-")
                        || n.starts_with(".dir.nexa-tmp-")
                        || n == "big.bin"
                        || n == "dir",
                    "중간 상태의 이름 = 스테이징 또는 커밋된 것: {names:?}"
                );
            }
        }
        assert!(
            seen.iter()
                .any(|v| v.iter().any(|n| n.starts_with(".big.bin.nexa-tmp-")))
                && seen
                    .iter()
                    .any(|v| v.iter().any(|n| n.starts_with(".dir.nexa-tmp-"))),
            "파일 · 폴더 모두 스테이징을 거친다: {seen:?}"
        );
        assert_eq!(
            listing(&b),
            ["big.bin", "dir"],
            "끝나면 최종 이름만(잔여 0)"
        );
        assert_eq!(
            fs::read(b.join("dir/big.bin")).unwrap().len(),
            9 * 1024 * 1024
        );
        // 같은 폴더 복제 = " (2)" — 역시 최종 이름만 남는다.
        let out = transfer_with(
            &[a.join("big.bin")],
            &a,
            Op::Copy,
            &mut no_conflict,
            &mut |_| {},
            &cancel,
            &LOOP,
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(listing(&a), ["big (2).bin", "big.bin", "dir"]);
        fs::remove_dir_all(&d).unwrap();
    }

    /// T-208 후속: 전송 시작 때 대상 폴더의 죽은 pid `tmp` 잔여(파일 · 폴더 · 이름 무관)를 치운다 — 살아 있는 pid 것 · 숫자가 아닌 꼬리(사용자 파일 꼴) ·
    /// `old`는 그대로. 저널 없는 재실행 사례(협업 실측 11-09).
    #[test]
    fn transfer_sweeps_dead_pid_staging_in_dest_dir() {
        let d = fixture("sweep");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(b.join(".d.nexa-tmp-4000000000-1/inner")).unwrap();
        fs::write(a.join("x.txt"), b"x").unwrap();
        fs::write(b.join(".x.txt.nexa-tmp-4000000000-0"), b"part").unwrap();
        fs::write(b.join(".other.bin.nexa-tmp-4000000001-7"), b"part").unwrap();
        fs::write(b.join(".z.nexa-tmp-notes"), b"mine").unwrap();
        fs::write(b.join(".k.nexa-old-4000000000-2"), b"old").unwrap();
        let live = format!(".y.nexa-tmp-{}-0", std::process::id());
        fs::write(b.join(&live), b"busy").unwrap();
        let run = |name: &str| {
            let out = transfer_with(
                &[a.join("x.txt")],
                &b,
                Op::Copy,
                &mut |_| Conflict::Overwrite,
                &mut |_| {},
                &AtomicBool::new(false),
                &LOOP,
            );
            assert!(out.errors.is_empty(), "{name}: {:?}", out.errors);
        };
        // 나이 가드(공유 폴더 · 다른 기계): 기본 1시간 안의 잔여는 pid가 죽어 보여도 둔다.
        run("기본 나이");
        assert!(
            b.join(".x.txt.nexa-tmp-4000000000-0").is_file()
                && b.join(".d.nexa-tmp-4000000000-1/inner").is_dir(),
            "방금 생긴 잔여는 두어야 한다(다른 기계가 쓰는 중일 수 있다)"
        );
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        for f in [
            ".x.txt.nexa-tmp-4000000000-0",
            ".other.bin.nexa-tmp-4000000001-7",
            ".d.nexa-tmp-4000000000-1/inner",
            ".d.nexa-tmp-4000000000-1",
        ] {
            set_mtime(&b.join(f), old);
        }
        // 폴더는 안쪽 파일 하나만 새것이어도 둔다(쓰는 중).
        fs::write(b.join(".d.nexa-tmp-4000000000-1/inner/fresh"), b"f").unwrap();
        set_mtime(&b.join(".d.nexa-tmp-4000000000-1/inner"), old);
        set_mtime(&b.join(".d.nexa-tmp-4000000000-1"), old);
        set_sweep_age_for_test(5);
        run("나이 5 s");
        assert!(
            !b.join(".x.txt.nexa-tmp-4000000000-0").exists()
                && !b.join(".other.bin.nexa-tmp-4000000001-7").exists(),
            "오래된 파일 잔여 = 제거"
        );
        assert!(
            b.join(".d.nexa-tmp-4000000000-1/inner/fresh").is_file(),
            "폴더는 안쪽에 새것이 있으면 둔다"
        );
        set_mtime(&b.join(".d.nexa-tmp-4000000000-1/inner/fresh"), old);
        set_mtime(&b.join(".d.nexa-tmp-4000000000-1/inner"), old);
        set_mtime(&b.join(".d.nexa-tmp-4000000000-1"), old);
        run("나이 5 s · 폴더 전부 오래됨");
        set_sweep_age_for_test(3600);
        let mut names: Vec<String> = fs::read_dir(&b)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        if cfg!(windows) {
            names.retain(|n| n != ".d.nexa-tmp-4000000000-1"); // 폴더 시각을 못 늙혀 남는다(위 set_mtime 설명)
        }
        assert_eq!(
            names,
            [
                ".k.nexa-old-4000000000-2",
                &live,
                ".z.nexa-tmp-notes",
                "x.txt"
            ],
            "오래된 죽은 pid tmp(파일 2 · 폴더 1) 제거 · 살아 있는 pid · 사용자 꼴 · old 유지"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    /// 시험용 수정 시각 설정(파일 · 폴더). Windows는 폴더 핸들에 `FILE_FLAG_BACKUP_SEMANTICS`가 필요해 폴더는 건너뛴다 — 그러면 폴더 자체 시각이
    /// 새것으로 남아 폴더 잔여가 안 지워지므로, Windows에서는 폴더 쪽 단언을 느슨하게 한다(`sweep_dirs_checked`).
    fn set_mtime(p: &Path, t: std::time::SystemTime) {
        let f = if p.is_dir() {
            if cfg!(windows) {
                return;
            }
            fs::File::open(p)
        } else {
            fs::OpenOptions::new().write(true).open(p)
        };
        f.unwrap().set_modified(t).unwrap();
    }

    /// 협업 QA 11-09 #1(회귀): 이름이 긴 항목(한글 80자 = 240 B · ASCII 250 B)은 스테이징 이름이 NAME_MAX를 넘어 실패했다 → 이름 부분을 200 B로 잘라
    /// 새 복사 · 덮어쓰기 모두 성공 · 스테이징 파일 이름 ≤ 255 B · 재개의 이름 지정 쓸기도 같은 접두로 잔여를 찾는다.
    #[test]
    fn long_names_fit_staging_name_max() {
        let d = fixture("longname");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let hangul = format!("{}.bin", "가".repeat(80)); // 244 B · 84자
        let ascii = format!("{}.bin", "x".repeat(246)); // 250 B
        for n in [&hangul, &ascii] {
            fs::write(a.join(n), b"K").unwrap();
            let staged = staging_path(&b.join(n), "tmp");
            let sn = staged.file_name().unwrap().to_string_lossy().into_owned();
            assert!(sn.len() <= 255, "{}: {sn}", sn.len());
            assert!(sn.starts_with(&format!(".{}.nexa-tmp-", staging_stem(n))));
        }
        let cancel = AtomicBool::new(false);
        // 새 복사.
        let out = transfer_with(
            &[a.join(&hangul), a.join(&ascii)],
            &b,
            Op::Copy,
            &mut no_conflict,
            &mut |_| {},
            &cancel,
            &LOOP,
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(fs::read(b.join(&hangul)).unwrap(), b"K");
        // 덮어쓰기(0.30.2는 여기서도 실패하던 한계).
        fs::write(a.join(&hangul), b"K2").unwrap();
        let out = transfer_with(
            &[a.join(&hangul)],
            &b,
            Op::Copy,
            &mut |_| Conflict::Overwrite,
            &mut |_| {},
            &cancel,
            &LOOP,
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(fs::read(b.join(&hangul)).unwrap(), b"K2");
        assert_eq!(fs::read_dir(&b).unwrap().count(), 2, "잔여 0");
        // 이름 지정 쓸기(재개 · 버림)가 잘린 접두로 잔여를 찾는다.
        let left = b.join(format!(".{}.nexa-tmp-4000000000-0", staging_stem(&hangul)));
        fs::write(&left, b"part").unwrap();
        assert_eq!(remove_staging_leftovers(&b.join(&hangul), &["tmp"]), 1);
        assert!(!left.exists());
        fs::remove_dir_all(&d).unwrap();
    }

    /// T-208: 새 대상 자리에 이미 무엇이 있으면(계획 뒤 생김 · 덮어쓰기 아님) 복사 전에 `AlreadyExists` · 있던 것 무손상 · 잔여 0.
    #[test]
    fn new_dest_refuses_existing_without_overwrite() {
        let d = fixture("stagetaken");
        fs::write(d.join("src.txt"), b"new").unwrap();
        fs::write(d.join("dst.txt"), b"old").unwrap();
        let e = copy_onto_with_progress(
            &d.join("src.txt"),
            &d.join("dst.txt"),
            false,
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{e}");
        assert_eq!(fs::read(d.join("dst.txt")).unwrap(), b"old");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 2, "잔여 0");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn rename_rules() {
        let d = fixture("rename");
        fs::write(d.join("a.txt"), "x").unwrap();
        fs::write(d.join("b.txt"), "y").unwrap();
        assert_eq!(rename(&d.join("a.txt"), "c.txt").unwrap(), d.join("c.txt"));
        assert!(!d.join("a.txt").exists() && d.join("c.txt").exists());
        assert_eq!(
            rename(&d.join("c.txt"), "c.txt").unwrap(),
            d.join("c.txt"),
            "동일 이름 = 무동작"
        );
        assert_eq!(
            rename(&d.join("c.txt"), "b.txt").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(rename(&d.join("c.txt"), "  ").is_err(), "빈 이름");
        assert!(rename(&d.join("c.txt"), "x/y").is_err(), "구분자 금지");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn create_new_numbering_and_delete_permanent() {
        let d = fixture("createnew");
        assert_eq!(create_new_dir(&d, "새 폴더").unwrap(), d.join("새 폴더"));
        assert_eq!(
            create_new_dir(&d, "새 폴더").unwrap(),
            d.join("새 폴더 (2)")
        );
        assert_eq!(
            create_new_file(&d, "새 파일.txt").unwrap(),
            d.join("새 파일.txt")
        );
        assert_eq!(
            create_new_file(&d, "새 파일.txt").unwrap(),
            d.join("새 파일 (2).txt"),
            "확장자 앞 순번(원본 규약)"
        );
        fs::write(d.join("새 폴더/x.txt"), "z").unwrap();
        delete_permanent(&d.join("새 폴더")).unwrap(); // 폴더 재귀
        assert!(!d.join("새 폴더").exists());
        delete_permanent(&d.join("없는 경로")).unwrap(); // 무동작
        fs::remove_dir_all(&d).unwrap();
    }

    /// GAP-046(사용자 11-04 ⓐ): 안쪽 하나가 지워지지 않아도 **끝까지** 지우고 그 항목 경로만 돌려준다 · 비지 않은 조상 폴더는 실패에 안 적음 ·
    /// 막힘이 풀린 뒤 같은 호출 = 전부 삭제 · 없는 경로 = 빈 목록.
    #[test]
    fn delete_permanent_tree_continues_past_failures_and_lists_them() {
        let d = fixture("deltree");
        let top = d.join("big");
        fs::create_dir_all(top.join("a/b")).unwrap();
        fs::create_dir_all(top.join("ro")).unwrap();
        for i in 0..20 {
            fs::write(top.join(format!("f{i}.txt")), b"x").unwrap();
        }
        fs::write(top.join("a/b/deep.txt"), b"y").unwrap();
        fs::write(top.join("ro/k"), b"k").unwrap();
        assert!(
            delete_permanent_tree(&d.join("없음")).is_empty(),
            "없는 경로 = 무동작"
        );
        // 막는 방법은 OS별 — unix = 쓰기 금지 폴더(안의 항목을 못 지움) · Windows = 공유 금지로 열어 둔 파일.
        #[cfg(unix)]
        let blocked = {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(top.join("ro"), fs::Permissions::from_mode(0o555)).unwrap();
            // root(권한 무시)면 막히지 않는다 — 그때는 전부 지워지는 쪽만 확인.
            fs::write(top.join("ro/probe"), b"").is_err()
        };
        #[cfg(windows)]
        let hold = {
            use std::os::windows::fs::OpenOptionsExt as _;
            fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(top.join("ro/k"))
                .unwrap()
        };
        #[cfg(windows)]
        let blocked = true;
        let failed = delete_permanent_tree(&top);
        let names: Vec<String> = failed
            .iter()
            .map(|(p, _)| {
                p.strip_prefix(&top)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        if blocked {
            assert_eq!(
                names,
                ["ro/k"],
                "안쪽 실패 경로만 · 조상 폴더(ro · big)는 안 적음: {failed:?}"
            );
            assert!(top.join("ro/k").is_file(), "막힌 항목은 남는다");
            assert!(
                !top.join("a").exists() && !top.join("f0.txt").exists(),
                "나머지는 전부 지워짐"
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(top.join("ro"), fs::Permissions::from_mode(0o755)).unwrap();
            }
            #[cfg(windows)]
            drop(hold);
            assert!(delete_permanent_tree(&top).is_empty(), "풀린 뒤 = 전부");
        } else {
            assert!(failed.is_empty(), "{failed:?}");
        }
        assert!(!top.exists());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn size_of_recursive_and_same_volume() {
        let d = fixture("size");
        fs::create_dir_all(d.join("s/t")).unwrap();
        fs::write(d.join("s/a.bin"), vec![0u8; 10]).unwrap();
        fs::write(d.join("s/t/b.bin"), vec![0u8; 32]).unwrap();
        assert_eq!(size_of(&d.join("s")), 42);
        assert_eq!(size_of(&d.join("없음")), 0, "실패 격리 = 0");
        assert!(same_volume(&d, &d.join("s")), "같은 루트");
        fs::remove_dir_all(&d).unwrap();
    }
    /// A14(점검 2차 G8 누락1): 교차 볼륨 폴더 이동은 **복사한 항목만** 지운다 — 열거에서 빠진 항목이
    /// 원본에 남고 폴더는 비지 않아 오류로 보고된다(종전 `remove_dir_all`은 통째로 삭제 = 유실).
    #[test]
    fn move_by_copy_removes_only_copied_items_and_keeps_strays() {
        let d = fixture("a14_strays");
        let (src, dest) = (d.join("src"), d.join("dest"));
        fs::create_dir_all(src.join("sub/deep")).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("sub/b.txt"), "b").unwrap();
        fs::write(src.join("sub/deep/c.txt"), "c").unwrap();

        // 복사 기록 = 후위 순서(자식 → 부모)
        let mut copied = Vec::new();
        let cancel = AtomicBool::new(false);
        copy_dir_with_progress(&src, &dest, &mut |_| {}, &cancel, Some(&mut copied)).unwrap();
        assert_eq!(copied.len(), 5, "파일 3 + 폴더 2");
        let pos = |p: &Path| copied.iter().position(|c| c == p).unwrap();
        assert!(pos(&src.join("sub/deep/c.txt")) < pos(&src.join("sub/deep")));
        assert!(pos(&src.join("sub/deep")) < pos(&src.join("sub")));
        assert!(pos(&src.join("sub/b.txt")) < pos(&src.join("sub")));

        // 열거에서 빠진(=기록에 없는) 항목이 있는 상태로 원본 정리
        fs::write(src.join("sub/stray.txt"), "열거 오류로 빠진 항목").unwrap();
        let r = remove_copied(&src, &copied);
        assert!(r.is_err(), "폴더가 비지 않으면 오류 보고: {r:?}");
        assert!(!src.join("a.txt").exists());
        assert!(!src.join("sub/b.txt").exists());
        assert!(!src.join("sub/deep").exists(), "비워진 하위 폴더는 삭제");
        assert_eq!(
            fs::read_to_string(src.join("sub/stray.txt")).unwrap(),
            "열거 오류로 빠진 항목",
            "복사되지 않은 항목은 원본에 잔존"
        );
        assert!(src.is_dir(), "원본 폴더 잔존");
        assert_eq!(
            fs::read_to_string(dest.join("sub/deep/c.txt")).unwrap(),
            "c"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    /// 정상 이동 결과 불변: 전부 복사되면 원본은 폴더까지 사라지고 사본이 온전하다.
    /// 이동 검증(T-202 ⑤): `with_move_verify` 안에서만 — 같은 사본 = 통과(원본 소멸) · 사본이 다르면 오류 + 원본·사본 유지 ·
    /// `copied` 목록에 없는(필터로 뺀) 파일은 대조하지 않음 · 범위 밖(기본)이면 검사 없음(DR-11 패리티).
    #[test]
    fn move_verify_checks_copies_before_cleanup() {
        let d = fixture("a14_verify");
        let (src, dest) = (d.join("src"), d.join("dest"));
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("sub/b.txt"), "bb").unwrap();
        let never = AtomicBool::new(false);
        // 통과: 실제 이동(복사 → 검증 → 원본 정리).
        fastcopy::with_move_verify(true, || {
            move_by_copy(&src, &dest, false, &mut |_| {}, &never)
        })
        .unwrap();
        assert!(!src.exists() && dest.join("sub/b.txt").is_file());
        // 불일치: 사본이 다른 상태에서 검증만 호출 — 오류 · 양쪽 유지.
        let (s2, d2) = (d.join("s2"), d.join("d2"));
        fs::create_dir_all(&s2).unwrap();
        fs::create_dir_all(&d2).unwrap();
        fs::write(s2.join("x.txt"), "same").unwrap();
        fs::write(d2.join("x.txt"), "diff").unwrap();
        fs::write(s2.join("skip.log"), "not copied").unwrap();
        let copied = vec![s2.join("x.txt")];
        let r = fastcopy::with_move_verify(true, || verify_moved(&s2, &d2, &copied, &never));
        let e = r.unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        assert!(
            e.to_string().contains("move verify failed") && e.to_string().contains("source kept"),
            "{e}"
        );
        assert!(
            s2.join("x.txt").is_file() && d2.join("x.txt").is_file(),
            "양쪽 유지"
        );
        // 필터로 뺀 skip.log는 copied에 없어 대조하지 않는다(대상에 없어도 통과).
        fs::write(d2.join("x.txt"), "same").unwrap();
        fastcopy::with_move_verify(true, || verify_moved(&s2, &d2, &copied, &never)).unwrap();
        // 파일 하나(copied 없음) · 범위 밖 = 검사 없음.
        fs::write(d2.join("x.txt"), "diff").unwrap();
        assert!(fastcopy::with_move_verify(true, || verify_moved(
            &s2.join("x.txt"),
            &d2.join("x.txt"),
            &[],
            &never
        ))
        .is_err());
        verify_moved(&s2.join("x.txt"), &d2.join("x.txt"), &[], &never).unwrap();
        assert!(!fastcopy::move_verify(), "끝나면 복원");
        fs::remove_dir_all(&d).unwrap();
    }

    /// GAP-027(11-01): 검증 목록 — `with_verify_collect` 안의 복사는 실제 옮긴 경로(파일 · 빈 폴더 · 링크 안쪽 · 필터로 뺀 것 제외)를
    /// `copied_paths`에 싣고 `verify_listed(pairs_for)`는 원본 트리를 다시 훑지 않는다: 복사 뒤 생긴 원본 파일 = ok(재귀 검증은 missing) ·
    /// 복사한 파일 수정 = 불일치 · 빈 폴더 누락 = 적발 · 링크 안쪽 사본 손상 = 적발 · 깃발 꺼짐 = 빈 목록.
    #[test]
    fn verify_collect_lists_copied_paths_and_listed_verify_ignores_newcomers() {
        let d = fixture("gap027");
        let (a, b) = (d.join("a"), d.join("b"));
        fs::create_dir_all(a.join("tree/empty")).unwrap();
        fs::create_dir_all(a.join("tree/sub")).unwrap();
        fs::write(a.join("tree/sub/f.txt"), "f").unwrap();
        fs::write(a.join("tree/skip.iso"), "iso").unwrap();
        fs::create_dir_all(d.join("linked")).unwrap();
        fs::write(d.join("linked/in.txt"), "inside").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(d.join("linked"), a.join("tree/ln")).unwrap();
        fs::create_dir_all(&b).unwrap();
        let cancel = AtomicBool::new(false);
        let f = fastcopy::NameFilter {
            include: vec![],
            exclude: vec!["*.iso".into()],
        };
        let out = fastcopy::with_verify_collect(true, || {
            fastcopy::with_filter(f, || {
                transfer_with(
                    &[a.join("tree")],
                    &b,
                    Op::Copy,
                    &mut |_| Conflict::Skip,
                    &mut |_| {},
                    &cancel,
                    &LOOP,
                )
            })
        });
        assert_eq!(out.transferred.len(), 1, "{out:?}");
        let rel: Vec<String> = out
            .copied_paths
            .iter()
            .map(|p| {
                p.strip_prefix(&a)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert!(
            rel.contains(&"tree/sub/f.txt".to_string())
                && rel.contains(&"tree/empty".to_string())
                && rel.contains(&"tree/sub".to_string()),
            "{rel:?}"
        );
        assert!(
            !rel.iter().any(|r| r.ends_with("skip.iso")),
            "필터로 뺀 파일은 목록에 없다: {rel:?}"
        );
        #[cfg(unix)]
        assert!(
            rel.contains(&"tree/ln".to_string()) && rel.contains(&"tree/ln/in.txt".to_string()),
            "링크 안쪽도 목록에: {rel:?}"
        );
        let pairs = verify::pairs_for(&out);
        assert!(verify::verify_listed(&pairs, &mut |_| {}, &cancel)
            .unwrap()
            .ok());
        // 복사 뒤 생긴 원본 파일 = 목록 밖 = ok(재귀 검증은 missing으로 본다 — 대조).
        fs::write(a.join("tree/new.txt"), "new").unwrap();
        assert!(verify::verify_listed(&pairs, &mut |_| {}, &cancel)
            .unwrap()
            .ok());
        assert!(
            !verify::verify_pairs(&out.transferred, &mut |_| {}, &cancel)
                .unwrap()
                .ok(),
            "재귀 검증은 신생 파일 · 제외 파일을 missing으로 본다"
        );
        // 복사한 파일 수정 = 불일치.
        fs::write(a.join("tree/sub/f.txt"), "F").unwrap();
        let rep = verify::verify_listed(&pairs, &mut |_| {}, &cancel).unwrap();
        assert_eq!(rep.failures.len(), 1, "{:?}", rep.failures);
        fs::write(a.join("tree/sub/f.txt"), "f").unwrap();
        // 빈 폴더 누락 = 적발.
        fs::remove_dir(b.join("tree/empty")).unwrap();
        let rep = verify::verify_listed(&pairs, &mut |_| {}, &cancel).unwrap();
        assert!(
            rep.failures
                .iter()
                .any(|(p, w)| p.ends_with("empty") && w == "missing"),
            "{:?}",
            rep.failures
        );
        fs::create_dir(b.join("tree/empty")).unwrap();
        // 링크 안쪽 사본 손상 = 적발.
        #[cfg(unix)]
        {
            fs::write(b.join("tree/ln/in.txt"), "broken").unwrap();
            let rep = verify::verify_listed(&pairs, &mut |_| {}, &cancel).unwrap();
            assert!(
                rep.failures.iter().any(|(p, _)| p.ends_with("in.txt")),
                "{:?}",
                rep.failures
            );
        }
        // 깃발 꺼짐 = 빈 목록.
        let out2 = transfer_with(
            &[a.join("tree/sub/f.txt")],
            &b,
            Op::Copy,
            &mut |_| Conflict::Overwrite,
            &mut |_| {},
            &cancel,
            &LOOP,
        );
        assert!(out2.copied_paths.is_empty());
        fs::remove_dir_all(&d).unwrap();
    }

    /// GAP-030: 스테이징 잔여 정리는 이름의 pid가 **죽은** 것만(살아 있는 pid = 다른 프로세스가 쓰는 중 · 자기 pid 포함) · tag 종류별 · 파일·폴더.
    #[test]
    fn staging_leftovers_skip_live_pids() {
        let d = fixture("gap030_leftovers");
        let dest = d.join("a.txt");
        let me = std::process::id();
        assert!(process_alive(me));
        assert!(!process_alive(u32::MAX - 7), "없는 pid");
        // GAP-035(Windows): 시스템 프로세스(pid 4)는 열기가 거부돼도(다른 계정 · 보호) 살아 있음 — 죽음으로 보면 그 pid의 잔여를 가로챈다.
        #[cfg(windows)]
        assert!(
            process_alive(4),
            "System(pid 4) = 살아 있음(접근 거부 포함)"
        );
        fs::write(d.join(format!(".a.txt.nexa-tmp-{me}-0")), "live").unwrap();
        fs::write(d.join(".a.txt.nexa-tmp-4000000000-0"), "dead").unwrap();
        fs::create_dir_all(d.join(".a.txt.nexa-old-4000000001-1")).unwrap();
        fs::write(d.join(".b.txt.nexa-tmp-4000000002-0"), "other name").unwrap();
        // 엔진 이름이 아닌 것(pid·seq가 숫자가 아님)은 건드리지 않는다(협업 실측 11-01).
        fs::create_dir_all(d.join(".a.txt.nexa-tmp-notes")).unwrap();
        fs::write(d.join(".a.txt.nexa-tmp-notes/k"), "user").unwrap();
        fs::write(d.join(".a.txt.nexa-tmp-123-notes"), "user").unwrap();
        fs::write(d.join(".a.txt.nexa-tmp-4000000003-0-extra"), "user").unwrap();
        assert_eq!(remove_staging_leftovers(&dest, &["tmp"]), 1);
        assert!(
            d.join(".a.txt.nexa-tmp-notes/k").exists()
                && d.join(".a.txt.nexa-tmp-123-notes").exists()
                && d.join(".a.txt.nexa-tmp-4000000003-0-extra").exists(),
            "해석 안 되는 이름 = 유지"
        );
        assert!(
            d.join(format!(".a.txt.nexa-tmp-{me}-0")).exists(),
            "살아 있는 pid = 유지"
        );
        assert!(!d.join(".a.txt.nexa-tmp-4000000000-0").exists());
        assert!(
            d.join(".a.txt.nexa-old-4000000001-1").exists(),
            "tag 다름 = 유지"
        );
        assert_eq!(remove_staging_leftovers(&dest, &["old"]), 1);
        assert!(
            d.join(".b.txt.nexa-tmp-4000000002-0").exists(),
            "다른 이름 = 유지"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn move_by_copy_full_success_removes_source_tree() {
        let d = fixture("a14_ok");
        let (src, dest) = (d.join("src"), d.join("dest"));
        fs::create_dir_all(src.join("sub/deep")).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("sub/deep/c.txt"), "c").unwrap();
        let mut bytes = 0u64;
        move_by_copy(
            &src,
            &dest,
            false,
            &mut |n| bytes += n,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(bytes, 2);
        assert!(!src.exists(), "원본 소멸");
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "a");
        assert_eq!(
            fs::read_to_string(dest.join("sub/deep/c.txt")).unwrap(),
            "c"
        );

        // 덮어쓰기(스테이징 교체) 경로도 동일
        let src2 = d.join("src2");
        fs::create_dir_all(&src2).unwrap();
        fs::write(src2.join("new.txt"), "new").unwrap();
        move_by_copy(&src2, &dest, true, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert!(!src2.exists());
        assert_eq!(fs::read_to_string(dest.join("new.txt")).unwrap(), "new");
        assert!(
            dest.join("a.txt").exists(),
            "병합(11-01): 옛 대상에만 있던 것은 남는다"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    /// 열리지 않는 파일(다른 프로세스 독점 열기 = 공유 위반)이 든 폴더 이동 → 오류, 원본 전부 잔존.
    #[cfg(windows)]
    #[test]
    fn move_by_copy_locked_file_keeps_source_intact() {
        use std::os::windows::fs::OpenOptionsExt;
        let d = fixture("a14_locked");
        let (src, dest) = (d.join("src"), d.join("dest"));
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("sub/locked.txt"), "잠김").unwrap();
        let _hold = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(src.join("sub/locked.txt"))
            .unwrap();
        let r = move_by_copy(&src, &dest, false, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.is_err(), "{r:?}");
        assert_eq!(fs::read_to_string(src.join("a.txt")).unwrap(), "a");
        drop(_hold); // 독점 열기 해제 후 내용 확인(share_mode(0)은 자기 자신의 재열기도 거부)
        assert_eq!(
            fs::read_to_string(src.join("sub/locked.txt")).unwrap(),
            "잠김"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    /// GAP-031(11-01): Unix `same_volume` = `st_dev`(가장 가까운 있는 조상) · 다른 장치(`/dev/shm` ≠ 임시 폴더일 때만)로의 이동은
    /// 복사 이동으로 끝난다(종전 = `rename` EXDEV로 항상 실패 · 원본 그대로).
    #[cfg(unix)]
    #[test]
    fn cross_device_move_uses_copy_on_unix() {
        let d = fixture("gap031");
        assert!(
            same_volume(&d, &d.join("missing/deeper")),
            "없는 경로 = 가까운 조상의 장치"
        );
        let shm = Path::new("/dev/shm");
        if !shm.is_dir() || same_volume(&d, shm) {
            fs::remove_dir_all(&d).unwrap();
            return; // 둘째 장치 없음 — 건너뜀(CI ubuntu는 /dev/shm이 tmpfs · /tmp는 디스크)
        }
        let other = shm.join(format!("nexa_ops_gap031_{}", std::process::id()));
        let _ = fs::remove_dir_all(&other);
        fs::create_dir_all(&other).unwrap();
        fs::create_dir_all(d.join("tree/sub")).unwrap();
        fs::write(d.join("tree/sub/f.txt"), "f").unwrap();
        fs::write(d.join("one.txt"), "one").unwrap();
        let never = AtomicBool::new(false);
        move_onto_with_progress(
            &d.join("tree"),
            &other.join("tree"),
            false,
            &mut |_| {},
            &never,
        )
        .unwrap();
        move_onto_with_progress(
            &d.join("one.txt"),
            &other.join("one.txt"),
            false,
            &mut |_| {},
            &never,
        )
        .unwrap();
        assert!(
            !d.join("tree").exists() && !d.join("one.txt").exists(),
            "원본 소멸"
        );
        assert_eq!(
            fs::read_to_string(other.join("tree/sub/f.txt")).unwrap(),
            "f"
        );
        assert_eq!(fs::read_to_string(other.join("one.txt")).unwrap(), "one");
        let _ = fs::remove_dir_all(&other);
        fs::remove_dir_all(&d).unwrap();
    }

    /// 실제 교차 볼륨(임시 폴더 ≠ target 볼륨일 때만 — 로컬 C:/D:·Windows CI) 재현: 복사 중 원본에
    /// 생긴 파일은 열거에서 빠질 수 있다. 종전엔 `remove_dir_all`로 함께 사라졌고(10-02 재현 캡처),
    /// 이제는 원본 또는 사본 어느 한쪽에 반드시 남는다.
    #[test]
    fn cross_volume_move_never_loses_item_missed_by_enumeration() {
        let src_root = fixture("a14_xvol");
        let dst_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/nexa_ops_a14_xvol");
        if same_volume(&src_root, &dst_root) {
            fs::remove_dir_all(&src_root).unwrap();
            return; // 둘째 볼륨 없음 — 건너뜀
        }
        let _ = fs::remove_dir_all(&dst_root);
        fs::create_dir_all(&dst_root).unwrap();
        let src = src_root.join("folder");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("sub/big.bin"), vec![7u8; 9 * 1024 * 1024]).unwrap();
        let late = src.join("sub/0late.txt"); // NTFS 이름순 열거에서 big.bin 뒤에 재개되면 빠진다
        let dest = dst_root.join("folder");
        let mut fired = false;
        let r = move_onto_with_progress(
            &src,
            &dest,
            false,
            &mut |_| {
                if !fired {
                    fired = true;
                    fs::write(&late, "late").unwrap();
                }
            },
            &AtomicBool::new(false),
        );
        assert!(fired);
        assert!(dest.join("sub/big.bin").exists(), "사본 완성");
        let in_src = late.exists();
        let in_dest = dest.join("sub/0late.txt").exists();
        assert!(
            in_src || in_dest,
            "늦게 생긴 항목이 양쪽 모두에서 사라짐(유실) — {r:?}"
        );
        if in_src {
            assert!(r.is_err(), "원본에 남은 항목이 있으면 오류 보고");
            assert!(
                !src.join("sub/big.bin").exists(),
                "복사한 항목은 원본에서 제거"
            );
        } else {
            assert!(r.is_ok(), "{r:?}");
            assert!(!src.exists());
        }
        let _ = fs::remove_dir_all(&src_root);
        let _ = fs::remove_dir_all(&dst_root);
    }

    /// 폴더 링크 만들기(시험용) — Unix 심볼릭 링크 · Windows 심볼릭 링크(권한 필요) → 안 되면 정션(`mklink /J` · 권한 불필요).
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(target, link).is_ok()
                || std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(link)
                    .arg(target)
                    .output()
                    .is_ok_and(|o| o.status.success())
        }
    }

    /// ⚠ 데이터 손실 회귀 시험(10-03): 링크(심볼릭 링크·정션)를 교차 볼륨 이동(복사 후 삭제)하거나 완전 삭제해도 **링크가 가리키는
    /// 실제 폴더의 내용은 그대로**다 · 폴더 안에 든 링크도 마찬가지 · 새 자리에는 내용이 복사된다. 링크를 만들 수 없는 환경은 건너뜀.
    #[test]
    fn link_moves_and_deletes_never_touch_the_target() {
        let d = fixture("linksafe");
        let real = d.join("real");
        fs::create_dir_all(real.join("deep")).unwrap();
        fs::write(real.join("keep.txt"), b"precious").unwrap();
        fs::write(real.join("deep").join("more.txt"), b"data").unwrap();
        let link = d.join("link");
        if !make_dir_link(&real, &link) {
            eprintln!("링크를 만들 수 없는 환경 — 건너뜀");
            let _ = fs::remove_dir_all(&d);
            return;
        }
        assert!(is_link(&link) && !is_link(&real));
        let intact = |real: &Path| {
            fs::read(real.join("keep.txt")).ok().as_deref() == Some(b"precious".as_slice())
                && fs::read(real.join("deep").join("more.txt")).ok().as_deref()
                    == Some(b"data".as_slice())
        };
        let never = AtomicBool::new(false);
        // ① 최상위 링크의 교차 볼륨 이동: 내용은 새 자리로 복사 · 원본에서는 링크만 사라짐 · 대상 그대로.
        let out = d.join("moved");
        move_by_copy(&link, &out, false, &mut |_| {}, &never).unwrap();
        assert!(intact(&real), "링크 대상의 파일이 지워지면 안 된다");
        assert!(!exists(&link), "원본 링크는 제거");
        assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"precious");
        assert_eq!(
            fs::read(out.join("deep").join("more.txt")).unwrap(),
            b"data"
        );
        // ② 링크가 든 폴더의 교차 볼륨 이동: 안쪽 링크도 링크만 제거 · 대상 그대로 · 원본 폴더는 비워져 사라짐.
        let holder = d.join("holder");
        fs::create_dir_all(&holder).unwrap();
        fs::write(holder.join("own.txt"), b"own").unwrap();
        assert!(make_dir_link(&real, &holder.join("inner")));
        let out2 = d.join("moved2");
        move_by_copy(&holder, &out2, false, &mut |_| {}, &never).unwrap();
        assert!(
            intact(&real),
            "폴더 안 링크를 통해서도 대상이 지워지면 안 된다"
        );
        assert!(!exists(&holder));
        assert_eq!(fs::read(out2.join("own.txt")).unwrap(), b"own");
        assert_eq!(
            fs::read(out2.join("inner").join("keep.txt")).unwrap(),
            b"precious"
        );
        // ③ 완전 삭제: 링크만.
        let link2 = d.join("link2");
        assert!(make_dir_link(&real, &link2));
        delete_permanent(&link2).unwrap();
        assert!(!exists(&link2) && intact(&real));
        // ④ 깨진 링크도 지워진다.
        let gone = d.join("gone");
        fs::create_dir_all(&gone).unwrap();
        let link3 = d.join("link3");
        assert!(make_dir_link(&gone, &link3));
        fs::remove_dir_all(&gone).unwrap();
        delete_permanent(&link3).unwrap();
        assert!(!exists(&link3));
        // ⑤ 상위를 가리키는 링크(순환)가 든 폴더의 복사는 오류로 멈춘다(무한 재귀 없음).
        let cyc = d.join("cyc");
        fs::create_dir_all(&cyc).unwrap();
        assert!(make_dir_link(&cyc, &cyc.join("self")));
        assert!(
            copy_onto_with_progress(&cyc, &d.join("cyc-out"), false, &mut |_| {}, &never).is_err()
        );
        let _ = delete_permanent(&cyc.join("self"));
        fs::remove_dir_all(&d).unwrap();
    }
}
