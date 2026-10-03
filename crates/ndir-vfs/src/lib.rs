//! nexa-vfs — 가상 파일시스템 추상화. 모든 저장소를 통일 인터페이스로 다룬다.
//!
//! 로컬 **스트리밍 열거**(FR-A1) 초안 + 저장소 공급자 추상화(스텁).
//!
//! [`archive`] = 압축 파일 **목록 계층**(X-46 — 압축 해제 없이 항목을 읽는다.
//! 미리보기 그리드·플러그인 ABI가 이 모델을 공유. 설계 SSOT = docs/28).

pub mod archive;

use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use ndir_core::FileKind;

/// 디렉터리 항목. 이름·종류 + 기본 메타데이터(크기·수정시각·속성).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: FileKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    /// Windows 파일 속성 비트(FILE_ATTRIBUTE_*). Windows 외에는 0.
    /// 열거 시 이미 조회한 메타데이터에서 꺼내므로 추가 syscall이 없다(숨김 필터의 무료 원천).
    pub attrs: u32,
    /// 링크형 항목의 실제 대상 경로(X-36 — 클라우드 연결 등 **표시명 ≠ 경로**).
    /// `Some`이면 트리 노드 경로는 `parent.join(name)` 대신 이 값을 쓴다.
    /// 일반 열거(`read_dir_entries`)·드라이브 항목은 `None`.
    pub target: Option<String>,
}

/// `FILE_ATTRIBUTE_DIRECTORY` — 열거 속성 비트의 폴더 표식.
pub const ATTR_DIRECTORY: u32 = 0x10;
/// 숨김(Windows `FILE_ATTRIBUTE_HIDDEN` · macOS `UF_HIDDEN` 플래그를 이 비트로 옮긴다 · Linux에는 없다 — 점 파일 규칙만).
pub const ATTR_HIDDEN: u32 = 0x2;
/// 이 OS에서 **이름이 점(`.`)으로 시작하면 숨김 파일**인가(Unix 관례 — Linux · macOS). Windows에서는 점 파일이 숨김과 별개의
/// 개념이다(숨김 = 속성 · 점 파일 = 따로 켜고 끄는 보기 옵션 — 사용자 10-03 "리눅스에는 dot file이라는 개념이 없고
/// . 으로 시작하면 숨김파일").
pub const DOT_IS_HIDDEN: bool = cfg!(unix);
/// 시스템(Windows `FILE_ATTRIBUTE_SYSTEM` · macOS `SF_RESTRICTED`(SIP 보호)를 이 비트로 옮긴다 · Linux에는 없다).
pub const ATTR_SYSTEM: u32 = 0x4;

/// **보호된 운영 체제 항목**인가 — 숨김 + 시스템이 **둘 다** 켜진 것(Windows 탐색기 "보호된 운영 체제 파일 숨기기"와 같은 규칙:
/// `$RECYCLE.BIN` · `pagefile.sys` · `System Volume Information` · 호환용 정션 …). macOS는 숨김 + SIP 보호 항목.
#[must_use]
pub fn is_protected_os_item(attrs: u32) -> bool {
    attrs & (ATTR_HIDDEN | ATTR_SYSTEM) == (ATTR_HIDDEN | ATTR_SYSTEM)
}
/// `FILE_ATTRIBUTE_REPARSE_POINT` — 심볼릭 링크·정션·마운트 포인트 등 **링크 표식**.
/// 종류([`FileKind`])와 별개 비트 — 폴더 링크는 `Dir` + 이 비트, 파일 링크는 `Symlink` + 이 비트.
pub const ATTR_REPARSE_POINT: u32 = 0x400;

impl Entry {
    /// 링크형 항목(심볼릭 링크·정션)인가 — [`ATTR_REPARSE_POINT`] 단독 판정.
    /// 종류와 독립: 폴더 정션은 `kind == Dir && is_link()`. 비Windows·메타데이터 실패 시 `false`.
    pub fn is_link(&self) -> bool {
        is_link_attrs(self.attrs)
    }
}

/// 링크형 항목인가(속성만으로): REPARSE 표식이 있고 **클라우드 플레이스홀더가 아닌** 것. OneDrive 같은 클라우드 파일도
/// REPARSE 비트를 달고 있어(실측 = ARCHIVE|SPARSE|REPARSE|OFFLINE|RECALL_ON_DATA_ACCESS) 종전에는 링크로 잘못 취급됐다
/// (링크 화살표 · 경로별 아이콘 조회 · 링크 이동 안전 경로 — GAP-018).
#[must_use]
pub fn is_link_attrs(attrs: u32) -> bool {
    attrs & ATTR_REPARSE_POINT != 0 && attrs & ATTR_CLOUD_MASK == 0
}

/// 클라우드 파일 속성(Windows `FILE_ATTRIBUTE_*` 원값 · macOS는 `SF_DATALESS`를 [`ATTR_RECALL_ON_DATA_ACCESS`]로 옮긴다).
pub const ATTR_OFFLINE: u32 = 0x1000;
pub const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
pub const ATTR_PINNED: u32 = 0x8_0000;
pub const ATTR_UNPINNED: u32 = 0x10_0000;
pub const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
/// 클라우드 동기화 폴더의 항목임을 알려 주는 비트 묶음.
pub const ATTR_CLOUD_MASK: u32 =
    ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_PINNED | ATTR_UNPINNED | ATTR_RECALL_ON_DATA_ACCESS;

/// 파일의 저장 상태("상태" 열 · 사용자 10-03 — 탐색기 OneDrive 폴더의 상태 열에 해당).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// 표시할 것 없음(평범한 로컬 항목).
    None,
    /// 온라인 전용 — 열면 내려받는다(파란 구름).
    CloudOnly,
    /// 이 장치에 내려받아져 있음(공간이 필요하면 다시 온라인 전용이 될 수 있다 — 초록 체크 테두리).
    AvailableLocally,
    /// 항상 이 장치에 유지(채운 초록 체크).
    AlwaysKeep,
    /// 네트워크 위치(NFS · SMB 등 마운트 아래 · UNC 경로).
    Network,
}

/// 속성 비트 → 클라우드 상태(순수 · 위가 우선): 항상 유지 → 온라인 전용 → 로컬에 있음 → 없음.
/// "동기화 중"은 속성만으로 알 수 없어 다루지 않는다(Windows cldapi 필요 — 후속).
#[must_use]
pub fn cloud_status(attrs: u32) -> FileStatus {
    if attrs & ATTR_PINNED != 0 {
        FileStatus::AlwaysKeep
    } else if attrs & (ATTR_RECALL_ON_DATA_ACCESS | ATTR_RECALL_ON_OPEN | ATTR_OFFLINE) != 0 {
        FileStatus::CloudOnly
    } else if attrs & ATTR_UNPINNED != 0 {
        FileStatus::AvailableLocally
    } else {
        FileStatus::None
    }
}

/// 이 경로가 네트워크 위치인가: Windows = UNC(`\\서버\…`) · Linux = 네트워크 파일 시스템 마운트 아래(`/proc/self/mounts`) ·
/// 그 밖 = 알 수 없음(false).
#[must_use]
pub fn is_network_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with("\\\\") && !s.starts_with("\\\\?\\") {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        let mounts = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
        network_mount_covers(&mounts, &s)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// `/proc/self/mounts` 본문에서 `path`를 덮는 **가장 긴** 마운트가 네트워크 파일 시스템인가(순수).
#[must_use]
pub fn network_mount_covers(mounts: &str, path: &str) -> bool {
    const NET: [&str; 8] = [
        "nfs",
        "nfs4",
        "cifs",
        "smb3",
        "smbfs",
        "fuse.sshfs",
        "davfs",
        "fuse.rclone",
    ];
    let mut best: Option<(usize, bool)> = None;
    for line in mounts.lines() {
        let mut it = line.split_whitespace();
        let (Some(_dev), Some(raw), Some(fstype)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        let mp = unescape_mount_path(raw);
        let covers = path == mp
            || mp == "/"
            || path
                .strip_prefix(mp.as_str())
                .is_some_and(|rest| rest.starts_with('/'));
        if covers && best.is_none_or(|(len, _)| mp.len() >= len) {
            best = Some((mp.len(), NET.contains(&fstype)));
        }
    }
    best.is_some_and(|(_, net)| net)
}

/// 열거 항목의 종류 판정(순수 — A31). **폴더 우선**: `is_dir` 또는 속성에
/// [`ATTR_DIRECTORY`]가 있으면 `Dir`(폴더 심볼릭 링크·정션은 Windows `FileType`이
/// `is_dir()=false`·`is_symlink()=true`라 종전엔 `Symlink`로 떨어져 진입·펼침이 막혔다).
/// 그다음 `is_symlink`면 `Symlink`(파일 링크·대상 소실로 메타데이터가 없는 링크), 나머지 `File`.
/// 링크 표식은 종류에 섞지 않고 [`Entry::is_link`](속성 0x400)로 따로 든다.
pub fn classify_kind(is_dir: bool, is_symlink: bool, attrs: u32) -> FileKind {
    if is_dir || attrs & ATTR_DIRECTORY != 0 {
        FileKind::Dir
    } else if is_symlink {
        FileKind::Symlink
    } else {
        FileKind::File
    }
}

/// Unix 심볼릭 링크 → Windows와 같은 뜻의 비트(GAP-009): 링크 표식([`ATTR_REPARSE_POINT`]) + **대상이 폴더면**
/// [`ATTR_DIRECTORY`](대상 추적 `fs::metadata` — 링크일 때만 한 번). 종전에는 Unix에서 속성이 0이라 폴더 심볼릭 링크가
/// `Symlink`(파일 취급)로 분류돼 들어가거나 펼칠 수 없었다. 깨진 링크는 표식만(종류 `Symlink` 유지).
#[cfg(unix)]
fn unix_link_attrs(dirent: &fs::DirEntry, is_symlink: bool) -> u32 {
    if !is_symlink {
        return 0;
    }
    let dir = fs::metadata(dirent.path()).is_ok_and(|m| m.is_dir());
    ATTR_REPARSE_POINT | if dir { ATTR_DIRECTORY } else { 0 }
}

#[cfg(not(unix))]
fn unix_link_attrs(_dirent: &fs::DirEntry, _is_symlink: bool) -> u32 {
    0
}

/// 열거 메타데이터에서 Windows 파일 속성 비트를 꺼낸다(비Windows=0).
#[cfg(windows)]
fn file_attrs(m: &fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    m.file_attributes()
}

/// macOS: BSD 파일 플래그 → 같은 뜻의 비트(`UF_HIDDEN` 0x8000 → 숨김 · `SF_RESTRICTED` 0x80000 → 시스템).
#[cfg(target_os = "macos")]
fn file_attrs(m: &fs::Metadata) -> u32 {
    use std::os::macos::fs::MetadataExt;
    let f = m.st_flags();
    let mut a = 0;
    if f & 0x8000 != 0 {
        a |= ATTR_HIDDEN;
    }
    if f & 0x0008_0000 != 0 {
        a |= ATTR_SYSTEM;
    }
    // SF_DATALESS(iCloud 등에서 내용이 내려받아지지 않은 파일) → 온라인 전용.
    if f & 0x4000_0000 != 0 {
        a |= ATTR_RECALL_ON_DATA_ACCESS;
    }
    a
}

#[cfg(not(any(windows, target_os = "macos")))]
fn file_attrs(_m: &fs::Metadata) -> u32 {
    0
}

/// 로컬 디렉터리를 **스트리밍 열거**한다 — 엔트리를 도착하는 대로 순차 산출.
///
/// 전체 스캔을 기다리지 않고 점진 처리(가상화 렌더·인라인 트리 펼침의 기반, FR-A1).
/// 반환 이터레이터의 각 항목은 개별 `Result` — 한 엔트리의 실패가 전체 열거를 막지 않는다.
/// 메타데이터 조회 실패(권한 등)는 격리하여 엔트리는 산출하되 크기/시각만 기본값으로 둔다.
pub fn read_dir_entries(
    path: impl AsRef<Path>,
) -> io::Result<impl Iterator<Item = io::Result<Entry>>> {
    let iter = fs::read_dir(path)?.map(|res| {
        let dirent = res?;
        let file_type = dirent.file_type()?;
        // 메타데이터(링크 자체 — 대상 추적 없음)를 먼저 꺼내 속성 비트로 폴더 링크를 판별한다.
        let (size, modified, attrs) = match dirent.metadata() {
            Ok(m) => (m.len(), m.modified().ok(), file_attrs(&m)),
            Err(_) => (0, None, 0),
        };
        let attrs = attrs | unix_link_attrs(&dirent, file_type.is_symlink());
        let kind = classify_kind(file_type.is_dir(), file_type.is_symlink(), attrs);
        Ok(Entry {
            name: dirent.file_name().to_string_lossy().into_owned(),
            kind,
            size,
            modified,
            attrs,
            target: None,
        })
    });
    Ok(iter)
}

/// 가상 최상위 "내 PC"의 **센티널 경로**(X-17). 콜론이 파일명에 불가한 문자라
/// 실제 경로와 충돌하지 않는다. 이 경로를 루트로 열면 드라이브 목록이 열거되고,
/// 항목 이름이 `C:\` 형태(절대 경로)라 `join` 시 부모를 대체 — 진입이 실 경로가 된다.
pub const MY_PC: &str = "::PC::";

/// `path`가 가상 최상위(내 PC)인가.
pub fn is_virtual_root(path: impl AsRef<Path>) -> bool {
    path.as_ref().as_os_str() == MY_PC
}

/// 존재하는 드라이브 루트 열거(X-17 — std만: `A:\`~`Z:\` metadata 프로브,
/// Win32 API 불요 = 크레이트 플랫폼 중립 유지. 비Windows에선 자연히 빈 목록).
/// 이름 = `C:\`(절대 경로 형태 — [`MY_PC`] 문서 참조). 볼륨명·용량 데코는 β(Win32).
pub fn drive_entries() -> Vec<Entry> {
    let mut out = Vec::new();
    // Unix에는 드라이브 문자가 없다 → "내 PC" = 루트(`/`) · 홈 · 마운트된 볼륨(사용자 10-03 Linux 실기 — 종전에는 빈 목록).
    #[cfg(unix)]
    for root in unix_roots() {
        out.push(Entry {
            name: root,
            kind: FileKind::Dir,
            size: 0,
            modified: None,
            attrs: 0,
            target: None,
        });
    }
    #[cfg(not(unix))]
    for c in b'A'..=b'Z' {
        let root = format!("{}:\\", c as char);
        if fs::metadata(&root).is_ok() {
            out.push(Entry {
                name: root,
                kind: FileKind::Dir,
                size: 0,
                modified: None, // 드라이브는 수정일 개념 없음 — 표시층에서 빈 셀
                attrs: 0,
                target: None,
            });
        }
    }
    out
}

/// Unix "내 PC" 항목(절대 경로 · 순서 = `/` → 홈 → 볼륨 이름순 · 중복 없음): Linux = `/proc/self/mounts`의 실제 볼륨 ·
/// macOS = `/Volumes/*`(시스템 볼륨을 가리키는 링크 제외).
#[cfg(unix)]
fn unix_roots() -> Vec<String> {
    let home = std::env::var("HOME").ok().filter(|h| h.starts_with('/'));
    #[cfg(target_os = "macos")]
    let volumes: Vec<String> = fs::read_dir("/Volumes")
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.path().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    #[cfg(not(target_os = "macos"))]
    let volumes = unix_mount_points(&fs::read_to_string("/proc/self/mounts").unwrap_or_default());
    merge_unix_roots(home.as_deref(), volumes)
}

/// 루트 · 홈 · 볼륨을 합친다(순수): `/` 맨 앞 · 홈(있고 `/`가 아니면) · 나머지 볼륨 이름순 · 중복 제거.
pub fn merge_unix_roots(home: Option<&str>, volumes: Vec<String>) -> Vec<String> {
    let mut out = vec!["/".to_string()];
    if let Some(h) = home {
        let h = h.trim_end_matches('/');
        if !h.is_empty() {
            out.push(h.to_string());
        }
    }
    let mut rest: Vec<String> = volumes.into_iter().filter(|v| !out.contains(v)).collect();
    rest.sort();
    rest.dedup();
    out.extend(rest);
    out
}

/// `/proc/self/mounts` 본문 → **사용자가 볼 볼륨**의 마운트 경로(순수 · 순서 = 파일 순서):
/// 블록 장치(`/dev/…`)나 네트워크 파일 시스템(nfs · cifs · smb3 · sshfs)만 · 시스템용 자리(`/boot` · `/efi` · `/snap` · `/var/…` ·
/// `/run/…` 중 `/run/media` 제외 · `/proc` · `/sys` · `/dev`)와 스냅 이미지(squashfs)는 뺀다. 경로의 8진 이스케이프(`\040` = 공백)를 푼다.
pub fn unix_mount_points(mounts: &str) -> Vec<String> {
    const NET: [&str; 5] = ["nfs", "nfs4", "cifs", "smb3", "fuse.sshfs"];
    const SKIP: [&str; 8] = [
        "/boot", "/efi", "/snap", "/var", "/proc", "/sys", "/dev", "/run",
    ];
    let mut out = Vec::new();
    for line in mounts.lines() {
        let mut it = line.split_whitespace();
        let (Some(dev), Some(raw), Some(fstype)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        if fstype == "squashfs" || !(dev.starts_with("/dev/") || NET.contains(&fstype)) {
            continue;
        }
        let path = unescape_mount_path(raw);
        let system = SKIP
            .iter()
            .any(|s| path == *s || path.starts_with(&format!("{s}/")))
            && !path.starts_with("/run/media/");
        if system || !path.starts_with('/') || out.contains(&path) {
            continue;
        }
        out.push(path);
    }
    out
}

/// mounts 경로의 8진 이스케이프(`\040` 공백 · `\011` 탭 · `\134` 역슬래시) 풀기.
fn unescape_mount_path(raw: &str) -> String {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
        {
            let v = (u32::from(b[i + 1] - b'0') << 6)
                | (u32::from(b[i + 2] - b'0') << 3)
                | u32::from(b[i + 3] - b'0');
            out.push(v as u8);
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 가상 최상위에 합류할 **추가 루트**(X-36 — 클라우드 연결 등 앱 정의 항목) 전역 등록부.
/// (표시명, 실경로) 쌍 — 표시명은 사용자 라벨, 진입은 [`Entry::target`] 경유 실경로.
/// 앱(nexa-app)이 설정 로드/변경 시 갱신하고, [`MY_PC`] 열거가 드라이브 뒤에 합류한다.
/// 탐지·직렬화는 앱 소관(이 크레이트는 플랫폼 중립 유지 — 검토서 26 §2-4).
static EXTRA_ROOTS: std::sync::RwLock<Vec<(String, String)>> = std::sync::RwLock::new(Vec::new());

/// 추가 루트 목록 교체(전량) — 실존 프로브는 호출자 몫.
pub fn set_extra_roots(roots: Vec<(String, String)>) {
    *EXTRA_ROOTS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = roots;
}

/// 등록된 추가 루트를 Entry로 열거(가상 최상위 전용 — 드라이브 목록 뒤 합류).
pub fn extra_root_entries() -> Vec<Entry> {
    EXTRA_ROOTS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .map(|(label, path)| Entry {
            name: label.clone(),
            kind: FileKind::Dir,
            size: 0,
            modified: None,
            attrs: 0,
            target: Some(path.clone()),
        })
        .collect()
}

/// 저장소 공급자 추상화. (로컬/SFTP/S3/클라우드)
///
/// 후속 단위에서 `list`/`stat`/`read`/`watch` 등을 추가한다.
pub trait Provider {
    /// 공급자 스킴 식별자 (예: "local", "sftp", "s3").
    fn scheme(&self) -> &str;
}

/// 클라우드 API 연결 경로의 **센티널 접두사**(X-37 2차 — ADR-0006 §3).
/// 형식 = `::CLOUD:<연결 인덱스>::<클라우드 내부 경로>`
/// (예: `::CLOUD:0::` = 그 연결의 루트 · `::CLOUD:0::/Documents`).
/// [`MY_PC`]와 같은 규약 — 콜론은 파일명에 불가해 실경로와 충돌하지 않는다.
pub const CLOUD_PREFIX: &str = "::CLOUD:";

/// `path`가 클라우드 API 경로면 `(연결 인덱스, 내부 경로)`. 아니면 `None`.
/// 내부 경로는 빈 문자열(루트) 또는 `/`로 시작하는 경로.
pub fn cloud_parts(path: impl AsRef<Path>) -> Option<(usize, String)> {
    let s = path.as_ref().to_str()?;
    let rest = s.strip_prefix(CLOUD_PREFIX)?;
    let (idx, tail) = rest.split_once("::")?;
    Some((idx.parse().ok()?, tail.to_string()))
}

/// 클라우드 연결 루트 경로 문자열.
pub fn cloud_root(idx: usize) -> String {
    format!("{CLOUD_PREFIX}{idx}::")
}

/// 클라우드 하위 경로(부모 + 자식 이름) — 트리 `join` 대체용.
pub fn cloud_child(idx: usize, parent_inner: &str, name: &str) -> String {
    format!("{CLOUD_PREFIX}{idx}::{parent_inner}/{name}")
}

/// 클라우드 연결의 **표시 라벨**(등록된 추가 루트에서 조회 — 예 "OneDrive – a@b.com").
/// 미등록이면 `None`.
pub fn cloud_label(idx: usize) -> Option<String> {
    let root = cloud_root(idx);
    EXTRA_ROOTS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find(|(_, p)| *p == root)
        .map(|(label, _)| label.clone())
}

/// 사람이 읽는 경로 표기(X-37 — 센티널 노출 금지).
/// `::CLOUD:0::/Docs/a.txt` → `OneDrive – a@b.com\Docs\a.txt`.
/// 클라우드 경로가 아니면 `None`(호출자가 원래 표기를 쓴다).
pub fn cloud_display(path: impl AsRef<Path>) -> Option<String> {
    let (idx, inner) = cloud_parts(path)?;
    let label = cloud_label(idx).unwrap_or_else(|| format!("Cloud {idx}"));
    if inner.is_empty() {
        return Some(label);
    }
    Some(format!("{label}{}", inner.replace('/', "\\")))
}

/// 클라우드 경로의 **마지막 세그먼트**(탭 제목용). 루트면 연결 라벨.
pub fn cloud_leaf(path: impl AsRef<Path>) -> Option<String> {
    let (idx, inner) = cloud_parts(path)?;
    match inner.rsplit('/').next().filter(|s| !s.is_empty()) {
        Some(name) => Some(name.to_string()),
        None => Some(cloud_label(idx).unwrap_or_else(|| format!("Cloud {idx}"))),
    }
}

/// 클라우드 경로의 **부모**(상위 이동). 루트의 부모 = 내 PC.
pub fn cloud_parent(path: impl AsRef<Path>) -> Option<String> {
    let (idx, inner) = cloud_parts(path)?;
    if inner.is_empty() {
        return Some(MY_PC.to_string());
    }
    let cut = inner.rfind('/').unwrap_or(0);
    Some(format!("{CLOUD_PREFIX}{idx}::{}", &inner[..cut]))
}

/// **표시 경로 → 센티널 경로** 역변환(X-37 — 경로 바 세그먼트 클릭).
///
/// 경로 바는 사람이 읽는 `OneDrive – a@b.com\Docs`를 보여주므로, 세그먼트를 누르면
/// 그 문자열이 그대로 탐색 요청으로 온다. 등록된 연결 라벨과 대조해 원래
/// `::CLOUD:idx::/Docs`로 되돌린다. 대응하는 연결이 없으면 `None`(일반 경로 취급).
pub fn cloud_from_display(display: &str) -> Option<String> {
    let roots = EXTRA_ROOTS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // 라벨이 긴 것부터 대조(짧은 라벨이 접두사인 경우 오매칭 방지)
    let mut cands: Vec<&(String, String)> = roots.iter().collect();
    cands.sort_by_key(|(l, _)| std::cmp::Reverse(l.len()));
    for (label, path) in cands {
        // 동기화 폴더 **링크**(X-36)는 실경로라 클라우드 센티널이 아니다 → 건너뛴다.
        // 여기서 `?`를 쓰면 그런 항목 하나 때문에 함수 전체가 None이 되어
        // 다른 연결의 조각 클릭까지 죽는다(사용자 QA 08-01의 진범).
        let Some((idx, _)) = cloud_parts(path) else {
            continue;
        };
        if display == label {
            return Some(cloud_root(idx));
        }
        if let Some(rest) = display.strip_prefix(label) {
            if let Some(tail) = rest.strip_prefix('\\').or_else(|| rest.strip_prefix('/')) {
                let inner = tail.replace('\\', "/");
                return Some(format!("{CLOUD_PREFIX}{idx}::/{inner}"));
            }
        }
    }
    None
}

/// 클라우드 열거 콜백 — 앱(nexa-app)이 등록한다. 이 크레이트는 네트워크를 모른다.
/// 반환 `None` = 아직 로딩 중(빈 목록으로 표시하고 완료 통지가 재로드).
type CloudLister = Box<dyn Fn(usize, &str) -> Option<Vec<Entry>> + Send + Sync>;
static CLOUD_LISTER: std::sync::RwLock<Option<CloudLister>> = std::sync::RwLock::new(None);

/// 클라우드 열거 콜백 등록(앱 기동 시 1회).
pub fn set_cloud_lister(f: CloudLister) {
    *CLOUD_LISTER
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(f);
}

/// 클라우드 경로 열거 — 등록된 콜백에 위임. 클라우드 경로가 아니면 `None`.
/// 콜백 미등록·로딩 중이면 **빈 목록**(트리는 정상 동작하고 완료 후 재로드된다).
pub fn cloud_entries(path: impl AsRef<Path>) -> Option<Vec<Entry>> {
    let (idx, inner) = cloud_parts(path)?;
    let guard = CLOUD_LISTER
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Some(match guard.as_ref() {
        Some(f) => f(idx, &inner).unwrap_or_default(),
        None => Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `EXTRA_ROOTS`는 **프로세스 전역**이라, 이를 세팅하는 테스트끼리 병렬로 돌면
    /// 서로의 값을 덮어써 간헐 실패한다(08-01 실측). 해당 테스트를 직렬화한다.
    static ROOTS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn roots_guard() -> std::sync::MutexGuard<'static, ()> {
        ROOTS_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// 상태 열 판정(순수): 클라우드 속성 → 상태(위가 우선) · 클라우드 플레이스홀더는 링크가 아니다(GAP-018) ·
    /// 네트워크 마운트 = 그 경로를 덮는 가장 긴 마운트의 파일 시스템 종류.
    #[test]
    fn cloud_status_link_and_network_rules() {
        use FileStatus as S;
        assert_eq!(cloud_status(0), S::None);
        assert_eq!(cloud_status(0x20), S::None, "ARCHIVE만 = 평범한 파일");
        assert_eq!(cloud_status(ATTR_RECALL_ON_DATA_ACCESS), S::CloudOnly);
        assert_eq!(cloud_status(ATTR_OFFLINE), S::CloudOnly);
        assert_eq!(cloud_status(ATTR_RECALL_ON_OPEN), S::CloudOnly);
        assert_eq!(cloud_status(ATTR_UNPINNED), S::AvailableLocally);
        assert_eq!(cloud_status(ATTR_PINNED), S::AlwaysKeep);
        assert_eq!(
            cloud_status(ATTR_PINNED | ATTR_RECALL_ON_DATA_ACCESS),
            S::AlwaysKeep,
            "항상 유지가 먼저"
        );
        assert_eq!(
            cloud_status(ATTR_UNPINNED | ATTR_OFFLINE),
            S::CloudOnly,
            "온라인 전용이 로컬보다 먼저"
        );
        // OneDrive 플레이스홀더 실측 속성(ARCHIVE|SPARSE|REPARSE|OFFLINE|RECALL_ON_DATA_ACCESS).
        let placeholder =
            0x20 | 0x200 | ATTR_REPARSE_POINT | ATTR_OFFLINE | ATTR_RECALL_ON_DATA_ACCESS;
        assert!(!is_link_attrs(placeholder), "클라우드 파일은 링크가 아니다");
        assert!(
            is_link_attrs(ATTR_REPARSE_POINT) && is_link_attrs(ATTR_REPARSE_POINT | ATTR_DIRECTORY)
        );
        assert!(!is_link_attrs(0) && !is_link_attrs(ATTR_REPARSE_POINT | ATTR_PINNED));
        // 네트워크 마운트.
        let mounts = "\
/dev/sda2 / ext4 rw 0 0
nas:/share /mnt/nas nfs4 rw 0 0
//srv/pub /mnt/smb\\040share cifs rw 0 0
/dev/sdb1 /mnt/nas/local ext4 rw 0 0
user@host: /home/u/remote fuse.sshfs rw 0 0
";
        assert!(!network_mount_covers(mounts, "/home/u"));
        assert!(network_mount_covers(mounts, "/mnt/nas"));
        assert!(network_mount_covers(mounts, "/mnt/nas/docs/a"));
        assert!(
            !network_mount_covers(mounts, "/mnt/nas/local/x"),
            "더 긴 로컬 마운트가 이긴다"
        );
        assert!(
            !network_mount_covers(mounts, "/mnt/nasty"),
            "접두사만 같은 다른 폴더"
        );
        assert!(
            network_mount_covers(mounts, "/mnt/smb share/x"),
            "이스케이프 푼 경로"
        );
        assert!(network_mount_covers(mounts, "/home/u/remote/p"));
        assert!(is_network_path(Path::new("\\\\server\\share\\a")), "UNC");
    }

    /// Unix "내 PC": mounts 해석(실제 볼륨만 · 이스케이프) + 루트/홈/볼륨 합치기.
    #[test]
    fn unix_roots_from_mounts() {
        let mounts = "\
sysfs /sys sysfs rw 0 0
proc /proc proc rw 0 0
/dev/nvme0n1p2 / ext4 rw 0 0
/dev/nvme0n1p1 /boot/efi vfat rw 0 0
/dev/loop3 /snap/core22/1 squashfs ro 0 0
tmpfs /run/user/1000 tmpfs rw 0 0
/dev/sdb1 /media/kiros33/USB\\040DISK vfat rw 0 0
/dev/sdc1 /run/media/kiros33/data ext4 rw 0 0
nas:/share /mnt/nas nfs4 rw 0 0
/dev/sda1 /home ext4 rw 0 0
/dev/sdb1 /media/kiros33/USB\\040DISK vfat rw 0 0
";
        let v = unix_mount_points(mounts);
        assert_eq!(
            v,
            [
                "/",
                "/media/kiros33/USB DISK",
                "/run/media/kiros33/data",
                "/mnt/nas",
                "/home"
            ]
        );
        let all = merge_unix_roots(Some("/home/kiros33/"), v);
        assert_eq!(
            all,
            [
                "/",
                "/home/kiros33",
                "/home",
                "/media/kiros33/USB DISK",
                "/mnt/nas",
                "/run/media/kiros33/data"
            ]
        );
        assert_eq!(merge_unix_roots(None, Vec::new()), ["/"]);
        assert_eq!(merge_unix_roots(Some("/"), vec!["/".into()]), ["/"]);
        // 절대 이름이라 센티널과 join하면 부모가 대체된다(진입 = 실제 경로).
        #[cfg(unix)]
        {
            let d = drive_entries();
            assert_eq!(d[0].name, "/");
            assert_eq!(Path::new(MY_PC).join(&d[0].name), Path::new("/"));
        }
    }

    #[test]
    fn virtual_root_and_drive_entries() {
        assert!(is_virtual_root(MY_PC));
        assert!(!is_virtual_root("C:\\"));
        // 드라이브 항목: 이름 = `X:\`(절대) → 센티널과 join하면 부모가 대체된다
        #[cfg(windows)]
        {
            let drives = drive_entries();
            assert!(!drives.is_empty(), "Windows에는 드라이브 1개 이상");
            for d in &drives {
                assert!(d.name.len() == 3 && d.name.ends_with(":\\"), "{}", d.name);
                assert_eq!(d.kind, FileKind::Dir);
                assert_eq!(
                    Path::new(MY_PC).join(&d.name),
                    Path::new(&d.name),
                    "절대 이름 join = 실 드라이브 경로"
                );
            }
        }
    }

    #[test]
    fn entry_holds_kind() {
        let e = Entry {
            name: "a.txt".into(),
            kind: FileKind::File,
            size: 5,
            modified: None,
            attrs: 0,
            target: None,
        };
        assert_eq!(e.kind, FileKind::File);
        assert_eq!(e.name, "a.txt");
        assert_eq!(e.size, 5);
    }

    /// X-37 2차: 클라우드 센티널 경로 분해·조립.
    #[test]
    fn cloud_path_parts_and_build() {
        assert_eq!(cloud_parts("::CLOUD:0::"), Some((0, String::new())));
        assert_eq!(
            cloud_parts("::CLOUD:2::/Docs/a.txt"),
            Some((2, "/Docs/a.txt".into()))
        );
        assert_eq!(cloud_parts("C:\\Users"), None);
        assert_eq!(cloud_parts(MY_PC), None);
        assert_eq!(cloud_root(3), "::CLOUD:3::");
        assert_eq!(cloud_child(1, "", "Docs"), "::CLOUD:1::/Docs");
        assert_eq!(cloud_child(1, "/Docs", "a.txt"), "::CLOUD:1::/Docs/a.txt");
        // 조립 → 분해 왕복
        let p = cloud_child(4, "/x", "y");
        assert_eq!(cloud_parts(&p), Some((4, "/x/y".into())));
    }

    /// X-37 5차: 표시명·leaf·부모 — **센티널이 UI에 노출되면 안 된다**.
    #[test]
    fn cloud_display_leaf_and_parent() {
        let _g = roots_guard();
        set_extra_roots(vec![("OneDrive – a@b.com".into(), cloud_root(0))]);
        assert_eq!(cloud_label(0).as_deref(), Some("OneDrive – a@b.com"));
        assert_eq!(
            cloud_display("::CLOUD:0::").as_deref(),
            Some("OneDrive – a@b.com")
        );
        assert_eq!(
            cloud_display("::CLOUD:0::/Docs/a.txt").as_deref(),
            Some("OneDrive – a@b.com\\Docs\\a.txt")
        );
        assert!(
            cloud_display("C:\\x").is_none(),
            "일반 경로는 원래 표기 유지"
        );
        // 탭 제목 = 마지막 세그먼트, 루트는 연결 라벨
        assert_eq!(cloud_leaf("::CLOUD:0::/Docs").as_deref(), Some("Docs"));
        assert_eq!(
            cloud_leaf("::CLOUD:0::").as_deref(),
            Some("OneDrive – a@b.com")
        );
        // 상위 이동: 하위 → 부모, 루트 → 내 PC
        assert_eq!(
            cloud_parent("::CLOUD:0::/Docs/x").as_deref(),
            Some("::CLOUD:0::/Docs")
        );
        assert_eq!(
            cloud_parent("::CLOUD:0::/Docs").as_deref(),
            Some("::CLOUD:0::")
        );
        assert_eq!(cloud_parent("::CLOUD:0::").as_deref(), Some(MY_PC));
        assert!(cloud_parent("D:\\a").is_none());
        // 미등록 연결도 패닉 없이 폴백 라벨
        assert_eq!(cloud_display("::CLOUD:9::").as_deref(), Some("Cloud 9"));
        set_extra_roots(Vec::new());
    }

    /// X-37: 표시 경로 → 센티널 역변환(경로 바 세그먼트 클릭).
    #[test]
    fn cloud_from_display_roundtrip() {
        let _g = roots_guard();
        set_extra_roots(vec![
            // 동기화 폴더 링크(실경로)가 섞여 있어도 다른 연결이 죽지 않아야 한다
            (
                "OneDrive – 회사".into(),
                "C:\\Users\\me\\OneDrive - Corp".into(),
            ),
            ("OneDrive – a@b.com".into(), cloud_root(0)),
            ("Google Drive – a@b.com".into(), cloud_root(1)),
        ]);
        assert_eq!(
            cloud_from_display("OneDrive – a@b.com").as_deref(),
            Some("::CLOUD:0::")
        );
        assert_eq!(
            cloud_from_display(r"Google Drive – a@b.com\Benthic").as_deref(),
            Some("::CLOUD:1::/Benthic")
        );
        assert_eq!(
            cloud_from_display(r"OneDrive – a@b.com\Docs\Sub").as_deref(),
            Some("::CLOUD:0::/Docs/Sub")
        );
        assert!(
            cloud_from_display(r"C:\Users").is_none(),
            "일반 경로는 통과"
        );
        // display → from_display 왕복
        let disp = cloud_display("::CLOUD:1::/Benthic").unwrap();
        assert_eq!(
            cloud_from_display(&disp).as_deref(),
            Some("::CLOUD:1::/Benthic")
        );
        set_extra_roots(Vec::new());
    }

    /// 콜백 미등록이어도 클라우드 경로는 빈 목록(패닉 없음) · 비클라우드는 None.
    #[test]
    fn cloud_entries_without_lister_is_empty() {
        assert_eq!(cloud_entries("::CLOUD:9::").map(|v| v.len()), Some(0));
        assert!(cloud_entries("D:\\tmp").is_none());
    }

    /// X-36: 추가 루트 등록 → 표시명·target 실경로 Entry로 열거.
    #[test]
    fn extra_roots_roundtrip() {
        let _g = roots_guard();
        set_extra_roots(vec![(
            "OneDrive – Test".into(),
            "C:\\Users\\t\\OneDrive".into(),
        )]);
        let e = extra_root_entries();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].name, "OneDrive – Test");
        assert_eq!(e[0].target.as_deref(), Some("C:\\Users\\t\\OneDrive"));
        assert_eq!(e[0].kind, FileKind::Dir);
        set_extra_roots(Vec::new());
        assert!(extra_root_entries().is_empty());
    }

    #[test]
    fn read_dir_entries_streams_local() {
        // 격리된 임시 디렉터리 생성(파일 1 + 하위 폴더 1)
        let base = std::env::temp_dir().join(format!("nexa_vfs_stream_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("a.txt"), b"hello").unwrap();
        fs::create_dir(base.join("sub")).unwrap();

        let mut entries: Vec<Entry> = read_dir_entries(&base)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        // 정리(assert 전에 수행 → 실패해도 임시폴더 잔류 방지)
        fs::remove_dir_all(&base).unwrap();

        assert_eq!(entries.len(), 2);
        let file = entries.iter().find(|e| e.name == "a.txt").unwrap();
        assert_eq!(file.kind, FileKind::File);
        assert_eq!(file.size, 5);
        let sub = entries.iter().find(|e| e.name == "sub").unwrap();
        assert_eq!(sub.kind, FileKind::Dir);
    }

    /// GAP-009: Unix 폴더 심볼릭 링크 = `Dir` + 링크 표식(들어갈 수 있다) · 파일 링크/깨진 링크 = `Symlink` + 표식.
    #[cfg(unix)]
    #[test]
    fn unix_dir_symlink_is_an_enterable_dir() {
        let base = std::env::temp_dir().join(format!("nexa_vfs_symlink_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("real")).unwrap();
        fs::write(base.join("f.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(base.join("real"), base.join("to-dir")).unwrap();
        std::os::unix::fs::symlink(base.join("f.txt"), base.join("to-file")).unwrap();
        std::os::unix::fs::symlink(base.join("gone"), base.join("broken")).unwrap();
        let entries: Vec<Entry> = read_dir_entries(&base)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        fs::remove_dir_all(&base).unwrap();
        let get = |n: &str| entries.iter().find(|e| e.name == n).unwrap();
        assert!(get("to-dir").kind == FileKind::Dir && get("to-dir").is_link());
        assert!(get("to-file").kind == FileKind::Symlink && get("to-file").is_link());
        assert!(get("broken").kind == FileKind::Symlink && get("broken").is_link());
        assert!(get("real").kind == FileKind::Dir && !get("real").is_link());
    }

    /// A31: 종류 판정 표 — 폴더 우선, 링크 표식은 속성 비트로 분리.
    #[test]
    fn classify_kind_dir_wins_over_link() {
        // (is_dir, is_symlink, attrs) → kind
        let table: [(bool, bool, u32, FileKind); 7] = [
            (true, false, ATTR_DIRECTORY, FileKind::Dir),
            (false, false, 0, FileKind::File),
            (false, false, 0x20, FileKind::File), // ARCHIVE만 — 일반 파일
            // 폴더 정션/폴더 심볼릭 링크: Windows FileType = symlink, 속성 = DIRECTORY|REPARSE
            (
                false,
                true,
                ATTR_DIRECTORY | ATTR_REPARSE_POINT,
                FileKind::Dir,
            ),
            // 파일 심볼릭 링크: 종전과 같이 Symlink
            (false, true, ATTR_REPARSE_POINT, FileKind::Symlink),
            // 대상 소실·메타데이터 실패(attrs=0)인 링크: Symlink 유지
            (false, true, 0, FileKind::Symlink),
            // 비Windows(attrs=0) 폴더
            (true, false, 0, FileKind::Dir),
        ];
        for (is_dir, is_symlink, attrs, want) in table {
            assert_eq!(
                classify_kind(is_dir, is_symlink, attrs),
                want,
                "is_dir={is_dir} is_symlink={is_symlink} attrs={attrs:#x}"
            );
        }
        // 링크 표식은 종류와 독립
        let mk = |kind, attrs| Entry {
            name: "x".into(),
            kind,
            size: 0,
            modified: None,
            attrs,
            target: None,
        };
        assert!(mk(FileKind::Dir, ATTR_DIRECTORY | ATTR_REPARSE_POINT).is_link());
        // 보호된 운영 체제 항목 = 숨김과 시스템 둘 다(MC/DC: 한쪽만이면 아니다).
        assert!(is_protected_os_item(
            ATTR_HIDDEN | ATTR_SYSTEM | ATTR_DIRECTORY
        ));
        assert!(!is_protected_os_item(ATTR_HIDDEN));
        assert!(!is_protected_os_item(ATTR_SYSTEM));
        assert!(!is_protected_os_item(0));
        assert!(mk(FileKind::Symlink, ATTR_REPARSE_POINT).is_link());
        assert!(!mk(FileKind::Dir, ATTR_DIRECTORY).is_link());
        assert!(!mk(FileKind::File, 0x20).is_link());
    }

    /// A31(cfg(windows)): 실제 폴더 정션(권한 불요 `mklink /J`) — 없으면 폴더 심볼릭 링크
    /// (개발자 모드/권한 필요) — 둘 다 못 만들면 생략. 열거 결과 `Dir` + `is_link()`.
    #[cfg(windows)]
    #[test]
    fn read_dir_entries_dir_junction_is_dir() {
        let base = std::env::temp_dir().join(format!("nexa_vfs_junction_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("real")).unwrap();
        fs::write(base.join("real").join("inner.txt"), b"x").unwrap();
        let link = base.join("jlink");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(base.join("real"))
            .output()
            .map(|o| o.status.success() && link.exists())
            .unwrap_or(false)
            || std::os::windows::fs::symlink_dir(base.join("real"), &link).is_ok();
        if !made {
            let _ = fs::remove_dir_all(&base);
            eprintln!("skip: 정션/폴더 링크 생성 불가(권한)");
            return;
        }

        let entries: Vec<Entry> = read_dir_entries(&base)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        // 링크 안쪽도 폴더로 열거되는지(진입 가능) 확인
        let inner: Vec<Entry> = read_dir_entries(&link)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        // 정리(링크를 먼저 지워야 대상 내용이 지워지지 않는다 — remove_dir_all은 링크를 추적하지 않음)
        let _ = fs::remove_dir(&link);
        let _ = fs::remove_dir_all(&base);

        let j = entries
            .iter()
            .find(|e| e.name == "jlink")
            .expect("정션 항목");
        assert_eq!(j.kind, FileKind::Dir, "폴더 정션은 Dir");
        assert!(j.is_link(), "링크 표식 = REPARSE 비트 attrs={:#x}", j.attrs);
        let r = entries.iter().find(|e| e.name == "real").unwrap();
        assert_eq!(r.kind, FileKind::Dir);
        assert!(!r.is_link(), "비링크 폴더는 표식 없음");
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].name, "inner.txt");
        assert_eq!(inner[0].kind, FileKind::File);
    }

    #[test]
    fn read_dir_entries_missing_path_errors() {
        let missing = std::env::temp_dir().join("nexa_vfs_does_not_exist_zzz");
        assert!(read_dir_entries(missing).is_err());
    }
}
