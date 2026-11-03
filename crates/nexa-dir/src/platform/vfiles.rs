//! 가상 파일 · 휘발 스테이징(T-147 ② · 11-03 · dir2 `nexa-app/src/clipboard.rs` X-42 + `dnd.rs` 이식 · SHELL-046~049 · 064~065 · 069) —
//! **플랫폼 중립 부분**(3-OS 시험). OLE 호출(설명서/스트림 읽기)은 `winvirtual`, 수신부 연결은 `windrop`.
//!
//! - **가상 파일**: 디스크에 없는 파일을 "설명서(`FILEGROUPDESCRIPTORW`) + 내용 공급자(`FILECONTENTS` · IStream/HGLOBAL)"로 실어 보내는 OLE 규약
//!   (Outlook 첨부 · 탐색기 zip 안 · MTP 기기 · 브라우저 이미지). 받는 쪽이 항목마다 내용을 요청해 **직접 파일을 만든다**.
//! - **휘발 스테이징**: 7-Zip 류 지연 렌더링 소스는 `DoDragDrop` 반환 직후 임시 폴더를 지운다 → `Drop` **반환 전** 같은 볼륨 rename으로 우리
//!   스테이징(`<Temp>/ndir-dnd-<pid>-<n>`)에 옮겨 둔다(크기 무관 즉시 · 무경쟁) · 드롭이 거부되면 원위치. **임시 폴더 밖 항목은 손대지 않는다**
//!   (dir2 G8-03 — 어떤 판정 오류에서도 사용자 실 파일을 rename하지 않는 안전 방향). 접두사 `ndir-*` = disk-check `--trim` ⑦ 대상.

#![cfg_attr(not(windows), allow(dead_code))]

use std::path::{Path, PathBuf};

/// `FILEDESCRIPTORW`.dwFlags — 어느 칸이 유효한가(shlobj_core.h FD_*).
pub(crate) const FD_ATTRIBUTES: u32 = 0x0000_0004;
pub(crate) const FD_FILESIZE: u32 = 0x0000_0040;
pub(crate) const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
/// `FILEDESCRIPTORW` 바이트 배치(패딩 없음 · 592): dwFlags 0 · clsid 4 · sizel 20 · pointl 28 · dwFileAttributes 36 · ftCreation 40 ·
/// ftLastAccess 48 · ftLastWrite 56 · nFileSizeHigh 64 · nFileSizeLow 68 · cFileName[260] 72. windows 크레이트 타입에 기대지 않고 직접
/// 읽어 3-OS에서 시험한다.
pub(crate) const FD_SIZE: usize = 592;
const OFF_FLAGS: usize = 0;
const OFF_ATTRS: usize = 36;
const OFF_SIZE_HI: usize = 64;
const OFF_SIZE_LO: usize = 68;
const OFF_NAME: usize = 72;
const NAME_LEN: usize = 260;

/// 가상 파일 항목 1개 — 설명서의 상대 경로(하위 폴더 포함 가능) · 종류 · 크기 힌트.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VirtualItem {
    pub rel: PathBuf,
    pub is_dir: bool,
    /// FD_FILESIZE 유효 시의 크기 — 미상(None)이면 EOF까지 읽는다.
    pub size: Option<u64>,
}

/// 설명서 이름 → 안전한 상대 경로. 구분자는 `\`(규약)·`/` 모두 · 말미 구분자 무시. 소스는 외부 앱이라 신뢰 불가 — 절대 경로 · 드라이브(`:`) ·
/// `.`/`..` 조각은 항목째 기각(탈출 방지).
pub(crate) fn sanitize_rel(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains(':') {
        return None;
    }
    let parts: Vec<&str> = name.split(['\\', '/']).collect();
    let mut rel = PathBuf::new();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() && i + 1 == parts.len() {
            break; // 말미 구분자("폴더\") 허용
        }
        if part.is_empty() || *part == "." || *part == ".." {
            return None;
        }
        rel.push(part);
    }
    (!rel.as_os_str().is_empty()).then_some(rel)
}

fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// `FILEGROUPDESCRIPTORW` 바이트(cItems + `FILEDESCRIPTORW` 배열) → 항목 목록. cItems가 실제 크기보다 커도 담긴 만큼만(손상 방어) ·
/// 부적합 이름은 항목별 격리(인덱스는 **설명서 순서**를 지켜야 `FILECONTENTS`의 lindex와 맞는다 → 기각 항목은 `None`).
pub(crate) fn parse_group_descriptor(bytes: &[u8]) -> Vec<Option<VirtualItem>> {
    if bytes.len() < 4 {
        return Vec::new();
    }
    let count = u32_at(bytes, 0) as usize;
    let avail = (bytes.len() - 4) / FD_SIZE;
    let mut out = Vec::with_capacity(count.min(avail));
    for i in 0..count.min(avail) {
        let fd = &bytes[4 + i * FD_SIZE..4 + (i + 1) * FD_SIZE];
        let name: Vec<u16> = (0..NAME_LEN)
            .map(|k| u16::from_le_bytes([fd[OFF_NAME + k * 2], fd[OFF_NAME + k * 2 + 1]]))
            .take_while(|&u| u != 0)
            .collect();
        let Some(rel) = sanitize_rel(&String::from_utf16_lossy(&name)) else {
            out.push(None);
            continue;
        };
        let flags = u32_at(fd, OFF_FLAGS);
        let is_dir =
            flags & FD_ATTRIBUTES != 0 && u32_at(fd, OFF_ATTRS) & FILE_ATTRIBUTE_DIRECTORY != 0;
        let size = (flags & FD_FILESIZE != 0 && !is_dir).then(|| {
            (u64::from(u32_at(fd, OFF_SIZE_HI)) << 32) | u64::from(u32_at(fd, OFF_SIZE_LO))
        });
        out.push(Some(VirtualItem { rel, is_dir, size }));
    }
    out
}

/// 항목별 대상 경로 계획 — 최상위 이름 충돌은 " (2)"(`ndir_ops::unique_dest` · 전송 엔진 규약) · 하위 항목은 같은 매핑을 따른다(자식이
/// 부모보다 먼저 와도 성립). 반환: (항목별 대상 · 생성될 최상위 목록).
pub(crate) fn plan_dests(
    items: &[Option<VirtualItem>],
    dest_dir: &Path,
) -> (Vec<Option<PathBuf>>, Vec<PathBuf>) {
    let mut roots: Vec<(std::ffi::OsString, PathBuf)> = Vec::new();
    let mut dests = Vec::with_capacity(items.len());
    for item in items {
        let Some(item) = item else {
            dests.push(None);
            continue;
        };
        let mut comps = item.rel.iter();
        let Some(first) = comps.next() else {
            dests.push(None);
            continue;
        };
        let rest: PathBuf = comps.collect();
        let nested = !rest.as_os_str().is_empty();
        let root = match roots.iter().find(|(k, _)| k == first) {
            Some((_, p)) => p.clone(),
            None => {
                let uniq = ndir_ops::unique_dest(
                    dest_dir,
                    &first.to_string_lossy(),
                    item.is_dir || nested,
                );
                roots.push((first.to_os_string(), uniq.clone()));
                uniq
            }
        };
        dests.push(Some(if nested { root.join(&rest) } else { root }));
    }
    (dests, roots.into_iter().map(|(_, p)| p).collect())
}

// ── 휘발 스테이징(지연 렌더링 소스 · dir2 dnd.rs) ───────────────────────────

/// 새 스테이징 폴더 경로(`<Temp>/ndir-<kind>-<pid>-<n>` · 만들지는 않는다) — `kind` = `dnd`(7-Zip 확보) · `vdrop`(가상 파일 추출).
pub(crate) fn staging_dir(kind: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "ndir-{kind}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

/// 두 경로 목록이 같은 집합인가(순서 무시) — Drop 재조회가 DragEnter 광고와 다르면 지연 렌더링 소스.
pub(crate) fn same_path_set(a: &[PathBuf], b: &[PathBuf]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut x: Vec<&PathBuf> = a.iter().collect();
    let mut y: Vec<&PathBuf> = b.iter().collect();
    x.sort();
    y.sort();
    x == y
}

/// 지연 렌더링 판정(Drop) — 광고(`cached`)가 **비어 있지 않고** 재조회(`fresh`)와 다를 때만. 광고가 비었던 경우(GetData 일시 실패 · 가상 소스
/// 겸용)는 "처음 받은 것"이라 지연으로 보지 않는다(G8-03 — 그 조합에서 실경로를 스테이징으로 rename하면 사용자 파일이 Temp를 거쳐 이동된다).
pub(crate) fn is_delayed(cached: &[PathBuf], fresh: &[PathBuf]) -> bool {
    !cached.is_empty() && !same_path_set(cached, fresh)
}

/// `p`가 `base` 아래인가 — 구성요소 단위 · ASCII 대소문자 무시(Windows 경로 표기 차 · 8.3 짧은 이름은 비대상).
pub(crate) fn under_dir(p: &Path, base: &Path) -> bool {
    let mut pc = p.components();
    for b in base.components() {
        let Some(c) = pc.next() else {
            return false;
        };
        if !c.as_os_str().eq_ignore_ascii_case(b.as_os_str()) {
            return false;
        }
    }
    true
}

/// 지연 렌더링 소스의 휘발 사본 확보: 임시 폴더 안 항목만 같은 볼륨 rename으로 `staging_dir("dnd")/<i>/<이름>`에 옮긴다(항목별 소분류 =
/// 서로 다른 부모의 동명 충돌 방지). rename 실패(잠금 · 다른 볼륨)나 임시 폴더 밖 항목은 원경로 유지(그 항목만 종전 경쟁으로 강등).
/// 반환: (확보 후 경로들 · 확보 항목의 (스테이징, 원경로) 목록 — 비어 있지 않으면 확보분은 우리 사본이라 호출자가 **이동**으로 옮겨 스테이징을
/// 자연 소거 · 드롭이 거부되면 [`restore_volatile`]).
pub(crate) fn steal_volatile(paths: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<(PathBuf, PathBuf)>) {
    let tmp = std::env::temp_dir();
    let base = staging_dir("dnd");
    let mut stolen: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut out = Vec::with_capacity(paths.len());
    for (i, p) in paths.into_iter().enumerate() {
        if !under_dir(&p, &tmp) {
            out.push(p);
            continue;
        }
        let Some(name) = p.file_name() else {
            out.push(p);
            continue;
        };
        let slot = base.join(i.to_string());
        if std::fs::create_dir_all(&slot).is_ok() {
            let staged = slot.join(name);
            if std::fs::rename(&p, &staged).is_ok() {
                stolen.push((staged.clone(), p));
                out.push(staged);
                continue;
            }
            let _ = std::fs::remove_dir(&slot);
        }
        out.push(p);
    }
    (out, stolen)
}

/// 확보한 항목을 원위치로 — 드롭이 거부됐을 때. 실패 항목은 스테이징에 남는다(소스 임시 폴더는 이미 사라졌을 수 있어 지우지 않는다).
pub(crate) fn restore_volatile(stolen: &[(PathBuf, PathBuf)]) {
    for (staged, orig) in stolen {
        if std::fs::rename(staged, orig).is_ok() {
            if let Some(slot) = staged.parent() {
                let _ = std::fs::remove_dir(slot);
                if let Some(base) = slot.parent() {
                    let _ = std::fs::remove_dir(base);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// `FILEDESCRIPTORW` 1개 바이트(이름 · 플래그 · 속성 · 크기 하위 32비트).
    pub(crate) fn fd(name: &str, flags: u32, attrs: u32, size_lo: u32) -> Vec<u8> {
        let mut b = vec![0u8; FD_SIZE];
        b[OFF_FLAGS..OFF_FLAGS + 4].copy_from_slice(&flags.to_le_bytes());
        b[OFF_ATTRS..OFF_ATTRS + 4].copy_from_slice(&attrs.to_le_bytes());
        b[OFF_SIZE_LO..OFF_SIZE_LO + 4].copy_from_slice(&size_lo.to_le_bytes());
        for (i, u) in name.encode_utf16().enumerate().take(NAME_LEN - 1) {
            b[OFF_NAME + i * 2..OFF_NAME + i * 2 + 2].copy_from_slice(&u.to_le_bytes());
        }
        b
    }

    /// `FILEGROUPDESCRIPTORW` 바이트 조립(cItems + 배열).
    pub(crate) fn group_bytes(fds: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = (fds.len() as u32).to_le_bytes().to_vec();
        for f in fds {
            bytes.extend_from_slice(f);
        }
        bytes
    }

    /// dir2 `parse_group_descriptor_items_and_traversal_guard`: 항목 · 폴더 플래그 · 크기 미상 · 탈출/절대 경로 기각(자리는 `None`으로 보존 —
    /// lindex 정렬) · cItems 거짓말 · 헤더 미달.
    #[test]
    fn parse_group_descriptor_items_and_traversal_guard() {
        let fds = [
            fd("한글 문서.txt", FD_FILESIZE, 0, 7),
            fd("폴더", FD_ATTRIBUTES, FILE_ATTRIBUTE_DIRECTORY, 0),
            fd("폴더\\안쪽.bin", 0, 0, 0),
            fd("..\\탈출.txt", 0, 0, 0),
            fd("C:\\abs\\경로.txt", 0, 0, 0),
        ];
        let items = parse_group_descriptor(&group_bytes(&fds));
        assert_eq!(items.len(), 5, "자리 보존(lindex)");
        assert_eq!(
            items[0],
            Some(VirtualItem {
                rel: "한글 문서.txt".into(),
                is_dir: false,
                size: Some(7),
            })
        );
        assert!(items[1].as_ref().unwrap().is_dir);
        assert_eq!(
            items[2].as_ref().unwrap().rel,
            Path::new("폴더").join("안쪽.bin")
        );
        assert_eq!(
            items[2].as_ref().unwrap().size,
            None,
            "FD_FILESIZE 없음 = 크기 미상"
        );
        assert!(
            items[3].is_none() && items[4].is_none(),
            "탈출 · 절대 경로 기각"
        );
        let mut lying = group_bytes(&fds[..1]);
        lying[0] = 9;
        assert_eq!(
            parse_group_descriptor(&lying).len(),
            1,
            "cItems > 실제 = 담긴 만큼만"
        );
        assert!(parse_group_descriptor(&[1, 0]).is_empty(), "헤더 미달");
        assert_eq!(sanitize_rel("a/b\\"), Some(PathBuf::from("a").join("b")));
        assert_eq!(sanitize_rel(""), None);
        assert_eq!(sanitize_rel("a//b"), None);
    }

    /// 대상 계획: 최상위 충돌 " (2)" · 하위는 같은 매핑(자식이 부모보다 먼저 와도) · 기각 항목 자리 `None`.
    #[test]
    fn plan_dests_resolves_collisions_and_follows_roots() {
        let d = std::env::temp_dir().join(format!("ndir-vfiles-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("문서.txt"), b"x").unwrap();
        let items = vec![
            Some(VirtualItem {
                rel: "문서.txt".into(),
                is_dir: false,
                size: Some(1),
            }),
            Some(VirtualItem {
                rel: Path::new("폴더").join("안쪽.bin"),
                is_dir: false,
                size: None,
            }),
            Some(VirtualItem {
                rel: "폴더".into(),
                is_dir: true,
                size: None,
            }),
            None,
        ];
        let (dests, roots) = plan_dests(&items, &d);
        assert_eq!(dests[0], Some(d.join("문서 (2).txt")));
        assert_eq!(dests[1], Some(d.join("폴더").join("안쪽.bin")));
        assert_eq!(dests[2], Some(d.join("폴더")));
        assert_eq!(dests[3], None);
        assert_eq!(roots, vec![d.join("문서 (2).txt"), d.join("폴더")]);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// dir2 `same_path_set` · `is_delayed` · `under_dir`.
    #[test]
    fn delayed_render_predicates() {
        let a = vec![PathBuf::from("C:\\a"), PathBuf::from("C:\\b")];
        let b = vec![PathBuf::from("C:\\b"), PathBuf::from("C:\\a")];
        let c = vec![PathBuf::from("C:\\a"), PathBuf::from("C:\\c")];
        assert!(same_path_set(&a, &b) && !same_path_set(&a, &c) && !same_path_set(&a, &a[..1]));
        assert!(
            !is_delayed(&[], &a),
            "빈 광고 + 실경로 = 지연 아님(확보 금지)"
        );
        assert!(is_delayed(&a, &c) && !is_delayed(&a, &a));
        let tmp = Path::new("C:\\Users\\u\\AppData\\Local\\Temp");
        assert!(under_dir(
            Path::new("C:\\Users\\u\\AppData\\Local\\Temp\\7zO1\\a.txt"),
            tmp
        ));
        assert!(under_dir(
            Path::new("c:\\users\\U\\appdata\\local\\temp\\Temp1_x.zip\\b"),
            tmp
        ));
        assert!(!under_dir(Path::new("D:\\Projects\\a.txt"), tmp));
        assert!(
            !under_dir(Path::new("C:\\Users\\u\\AppData\\Local\\Temporary\\a"), tmp),
            "구성요소 불일치"
        );
        assert!(under_dir(tmp, tmp));
    }

    /// dir2 `steal_volatile_*`: 임시 폴더 안 파일·폴더는 스테이징으로 rename(원위치에서 사라짐 · 하위 보존) · 밖 항목은 손대지 않음 ·
    /// 거부 = `restore_volatile`로 원위치 + 스테이징 정리.
    #[test]
    fn steal_and_restore_volatile_round_trip() {
        let src = std::env::temp_dir().join(format!("ndir-vfiles-steal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&src);
        std::fs::create_dir_all(src.join("folder")).unwrap();
        std::fs::write(src.join("a.txt"), b"a").unwrap();
        std::fs::write(src.join("folder").join("in.txt"), b"in").unwrap();
        let outside = std::env::current_dir()
            .unwrap()
            .join(format!(".ndir-vfiles-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("real.txt"), b"r").unwrap();
        let cwd_in_tmp = under_dir(&outside, &std::env::temp_dir());

        let (out, stolen) = steal_volatile(vec![
            src.join("a.txt"),
            src.join("folder"),
            outside.join("real.txt"),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(
            stolen.len(),
            if cwd_in_tmp { 3 } else { 2 },
            "임시 폴더 안만 확보"
        );
        assert!(out[0].ends_with("a.txt") && out[0] != src.join("a.txt") && out[0].is_file());
        assert!(!src.join("a.txt").exists(), "원위치에서 이동됨");
        assert!(out[1].join("in.txt").is_file(), "폴더는 내용째");
        if !cwd_in_tmp {
            assert_eq!(out[2], outside.join("real.txt"));
            assert!(outside.join("real.txt").is_file(), "밖 항목은 rename 안 함");
        }
        assert!(
            out[0].to_string_lossy().contains("ndir-dnd-"),
            "접두사 = disk-check ⑦ 대상: {}",
            out[0].display()
        );
        restore_volatile(&stolen);
        assert!(
            src.join("a.txt").is_file() && src.join("folder").join("in.txt").is_file(),
            "원위치 복귀"
        );
        assert!(
            !out[0].exists() && out[0].parent().is_some_and(|s| !s.exists()),
            "스테이징 정리"
        );
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
