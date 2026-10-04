//! 파일 목록 행 공급자 — `ndir-tree::Tree`(dir2 인라인 트리 평면 스트림)를 nexa-grid [`RowSource`]로 잇는다.
//!
//! dir2에서는 `nexa-app/src/panel.rs`가 같은 일을 했다(docs/port/13 · 21 G-1): 위젯은 가시 행만 묻고, 펼침/선택/정렬/타입어헤드는
//! 코어(`ndir-tree`)가 결정한다. 이 파일은 **번역만**(셀 텍스트 서식 포함) — 창·OS API를 모른다(docs/15 §2-1).
//!
//! 컬럼 키(dir2 docs/port/13 §2-5 순서): 0 이름(트리) · 1 확장자 · 2 크기 · 3 수정 시각 · 4 종류(`col.*` i18n). 정렬 키 대응 = [`sort_key_of`].

use ndir_core::FileKind;
use ndir_tree::{FindScope, SelectMode, SortKey, SortSpec, Tree};
use nexa_grid::{Marker, RowItem, RowSource, SelectOp};
use std::path::{Path, PathBuf};

/// 컬럼 키(그리드 `Column.key`).
pub(crate) const COL_NAME: u32 = 0;
pub(crate) const COL_EXT: u32 = 1;
pub(crate) const COL_SIZE: u32 = 2;
pub(crate) const COL_MODIFIED: u32 = 3;
pub(crate) const COL_KIND: u32 = 4;
/// 내 PC 전용 열(dir2 X-17 · PANEL-044): 전체 크기 · 여유 공간 — 값은 `Disk` 포트가 채운다(`set_drive_space`).
pub(crate) const COL_TOTAL: u32 = 5;
pub(crate) const COL_FREE: u32 = 6;
/// 상태 열(dir3 신규 10-03 — 클라우드/네트워크 파일 상태 · 아이콘 셀 · dir2 CLOUD-096 β 제안).
pub(crate) const COL_STATUS: u32 = 7;

/// 상태 → 아이콘 키(`status:<이름>` — 리졸버 `app/row_icons.rs`가 색 입힌 그림을 준다) · 표시할 것이 없으면 `None`.
pub(crate) fn status_icon_key(st: ndir_vfs::FileStatus) -> Option<&'static str> {
    use ndir_vfs::FileStatus as S;
    Some(match st {
        S::None => return None,
        S::CloudOnly => "status:cloud",
        S::AvailableLocally => "status:local",
        S::AlwaysKeep => "status:pinned",
        S::Network => "status:network",
    })
}

/// 상태 → 글(아이콘을 못 그릴 때의 폴백 · 정보 표시).
pub(crate) fn status_label(st: ndir_vfs::FileStatus) -> String {
    use ndir_vfs::FileStatus as S;
    match st {
        S::None => String::new(),
        S::CloudOnly => ndir_i18n::tr("status.cloudOnly"),
        S::AvailableLocally => ndir_i18n::tr("status.availableLocally"),
        S::AlwaysKeep => ndir_i18n::tr("status.alwaysKeep"),
        S::Network => ndir_i18n::tr("status.network"),
    }
}

/// 탭 제목(dir2 PANEL-013): 가상 최상위 = `nav.mypc` · 일반 = 마지막 경로 요소 · 드라이브 루트 = `D:`(후행 구분자 제거) ·
/// Unix 루트 = `/`(구분자를 떼면 빈 글자가 되던 것 — 사용자 10-03 "/ 위치의 탭 이름이 공백").
pub(crate) fn title_of(p: &Path) -> String {
    if ndir_vfs::is_virtual_root(p) {
        return ndir_i18n::tr("nav.mypc");
    }
    match p.file_name() {
        Some(n) if !n.is_empty() => n.to_string_lossy().into_owned(),
        _ => {
            let full = p.to_string_lossy();
            let cut = full.trim_end_matches(['\\', '/']);
            if cut.is_empty() && !full.is_empty() {
                "/".to_string()
            } else {
                cut.to_string()
            }
        }
    }
}

/// 경로 바 문자열: 가상 최상위 = 사람이 읽는 라벨 · 그 밖 = 경로 그대로.
pub(crate) fn display_path(p: &Path) -> String {
    if ndir_vfs::is_virtual_root(p) {
        ndir_i18n::tr("nav.mypc")
    } else {
        p.to_string_lossy().into_owned()
    }
}

/// 바로 가기 확장자(탐색기가 이름에서 늘 숨기는 것 · 소문자).
const SHORTCUT_EXTS: [&str; 3] = [".lnk", ".url", ".appref-ms"];

/// `name`이 바로 가기 파일이면 `(확장자 뺀 이름, 확장자(점 포함 · 원래 대소문자))`(순수 · 대소문자 무시 · 이름이 확장자뿐이면 `None`).
pub(crate) fn split_shortcut_ext(name: &str) -> Option<(&str, &str)> {
    SHORTCUT_EXTS.iter().find_map(|ext| {
        let cut = name.len().checked_sub(ext.len())?;
        (cut > 0 && name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(ext))
            .then(|| name.split_at(cut))
    })
}

/// Unix "내 PC"의 홈 폴더 행인가(`$HOME`과 같은 경로) — 용량은 보여도 종류는 드라이브가 아니라 폴더.
fn is_home_dir(name: &str) -> bool {
    cfg!(unix)
        && std::env::var_os("HOME")
            .is_some_and(|h| Path::new(&h) == Path::new(name.trim_end_matches('/')) && name != "/")
}

/// Windows 바로 가기 파일(`.lnk` · 대소문자 무시)인가 — 대상 해석(GAP-007) 대상.
pub(crate) fn is_lnk(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
}

/// 목록·이름 바꾸기에 보이는 이름(GAP-006 · dir2 `source.rs:456-465`): `hide`(= 이 OS가 바로 가기 확장자를 숨기는가)이고
/// 폴더가 아닌 바로 가기 파일이면 확장자를 뺀다(확장자 열은 `lnk` 그대로).
pub(crate) fn display_name(name: &str, is_dir: bool, hide: bool) -> &str {
    if hide && !is_dir {
        if let Some((stem, _)) = split_shortcut_ext(name) {
            return stem;
        }
    }
    name
}

/// 이름 바꾸기 확정 이름(dir2 `win.rs:4036-4045`): 원래 이름이 숨겨진 바로 가기 확장자를 가졌고 새 이름에 그 확장자가 없으면
/// 다시 붙인다(사용자가 본 것은 확장자 없는 이름 — 붙이지 않으면 바로 가기가 깨진다).
pub(crate) fn restore_shortcut_ext(old: &str, new: &str, is_dir: bool, hide: bool) -> String {
    if hide && !is_dir {
        if let Some((_, ext)) = split_shortcut_ext(old) {
            let has = new.len() > ext.len()
                && new.is_char_boundary(new.len() - ext.len())
                && new[new.len() - ext.len()..].eq_ignore_ascii_case(ext);
            if !has && !new.is_empty() {
                return format!("{new}{ext}");
            }
        }
    }
    new.to_string()
}

/// 열람 옵션(설정 `list.*`에서) — 다시 열 때 그대로 쓴다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ListOpts {
    pub show_hidden: bool,
    pub show_dotfiles: bool,
    /// 보호된 운영 체제 항목 표시(`list.show_protected`).
    pub show_protected: bool,
    pub folders_first: bool,
    pub case_sensitive: bool,
}

/// 탭 보기 옵션 4종 `(숨김, Dot, 폴더 우선, 대소문자 구분 정렬)` — 탭이 소유하고 보기 범위(`list.view_scope`)만큼 함께 바뀐다.
pub(crate) type ViewOpts = (bool, bool, bool, bool);

impl ListOpts {
    /// 탭 보기 옵션 4종만 바꾼 사본 — 나머지(보호 항목 표시 = 전역)는 그대로.
    pub(crate) fn with_view(self, (hidden, dot, folders_first, case): ViewOpts) -> ListOpts {
        ListOpts {
            show_hidden: hidden,
            show_dotfiles: dot,
            folders_first,
            case_sensitive: case,
            ..self
        }
    }

    /// 탭 보기 옵션 4종.
    pub(crate) fn view(self) -> ViewOpts {
        (
            self.show_hidden,
            self.show_dotfiles,
            self.folders_first,
            self.case_sensitive,
        )
    }

    /// 세션 플래그(dir2 `panel{i}.views` — bit0 숨김 · bit1 Dot · bit2 폴더 우선 · **bit3 대소문자 구분**(dir3 추가 —
    /// dir2는 하위 3비트만 읽는다)).
    pub(crate) fn view_flags(self) -> u8 {
        u8::from(self.show_hidden)
            | u8::from(self.show_dotfiles) << 1
            | u8::from(self.folders_first) << 2
            | u8::from(self.case_sensitive) << 3
    }

    /// 세션 플래그 → 보기 옵션 4종.
    pub(crate) fn view_of_flags(f: u8) -> ViewOpts {
        (f & 1 != 0, f & 2 != 0, f & 4 != 0, f & 8 != 0)
    }
}

/// 한 패널의 행 공급자.
#[derive(Debug)]
pub(crate) struct TreeSource {
    tree: Option<Tree>,
    path: PathBuf,
    opts: ListOpts,
    /// 이 폴더가 네트워크 위치(NFS · SMB 마운트 · UNC) 아래인가 — 열 때 한 번 판정(상태 열).
    on_network: bool,
    /// 열기 실패 사유(상태줄에 · 빈 목록).
    error: Option<String>,
    /// 드라이브 이름(`C:\`) → (전체, 여유) — 가상 최상위에서만 · 호스트가 Disk 포트로 채운다.
    drive_space: std::collections::HashMap<String, (u64, u64)>,
    /// 잘라내기 대기 경로(dir2 SHELL-044/X-32 — 그 행은 흐리게 · 호스트가 클립보드와 동기).
    cut_marks: std::collections::HashSet<PathBuf>,
}

impl TreeSource {
    /// 폴더를 연다 — 실패해도 공급자는 만들어진다(빈 목록 + `error`).
    pub(crate) fn open(path: &Path, opts: ListOpts) -> TreeSource {
        let mut s = TreeSource {
            tree: None,
            path: path.to_path_buf(),
            opts,
            on_network: !ndir_vfs::is_virtual_root(path) && ndir_vfs::is_network_path(path),
            error: None,
            drive_space: std::collections::HashMap::new(),
            cut_marks: std::collections::HashSet::new(),
        };
        s.reload();
        s
    }

    /// 잘라내기 표식 집합 교체 — 바뀌었으면 true(호스트가 목록 무효화).
    pub(crate) fn set_cut_marks(&mut self, marks: std::collections::HashSet<PathBuf>) -> bool {
        if self.cut_marks == marks {
            return false;
        }
        self.cut_marks = marks;
        true
    }

    /// 같은 경로를 다시 읽는다(F5 · 옵션 변경 · 폴더 감시) — **무간섭 재열람**(dir2 PANEL-036 `reopen_filtered`):
    /// 정렬 키 · 펼친 폴더 · 선택을 스냅샷해 새 트리에 경로로 되돌린다(사라진 항목은 조용히 빠진다). 종전에는 새 트리를 기본 정렬로
    /// 열기만 해 폴더가 밖에서 바뀔 때마다 선택·펼침·정렬이 풀렸다(GAP-005 · 10-03 T4 `ctx.wait` 시나리오가 적발).
    pub(crate) fn reload(&mut self) {
        let (keys, expanded, selected) = match &self.tree {
            Some(t) => {
                let expanded: Vec<String> = (0..t.visible_len())
                    .filter_map(|i| t.visible_id(i))
                    .filter(|&id| t.is_expanded(id) == Some(true))
                    .filter_map(|id| t.node_path(id))
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                let selected: Vec<String> = t
                    .selected_paths()
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                (t.sort_spec().keys.clone(), expanded, selected)
            }
            None => (vec![(SortKey::Name, false)], Vec::new(), Vec::new()),
        };
        match Tree::open_visible(
            &self.path,
            self.opts.show_hidden,
            self.opts.show_dotfiles,
            self.opts.show_protected,
        ) {
            Ok(mut t) => {
                t.set_sort(SortSpec {
                    keys,
                    folders_first: self.opts.folders_first,
                    case_sensitive: self.opts.case_sensitive,
                });
                // 펼침은 가시 순서(부모 먼저)로 모았으므로 그대로 다시 펼치면 된다 · 없어진 폴더는 건너뛴다.
                for dir in &expanded {
                    let _ = t.expand_path(dir);
                }
                for path in &selected {
                    if let Some(id) = t.index_of_path(path).and_then(|i| t.visible_id(i)) {
                        t.select(id, ndir_tree::SelectMode::Toggle);
                    }
                }
                self.tree = Some(t);
                self.error = None;
            }
            Err(e) => {
                self.tree = None;
                self.error = Some(e.to_string());
            }
        }
    }

    /// 행의 저장 상태(상태 열): 클라우드 속성이 있으면 그것 · 없고 네트워크 위치면 네트워크 · 그 밖 = 없음.
    pub(crate) fn status_of(&self, index: usize) -> ndir_vfs::FileStatus {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row(index)) else {
            return ndir_vfs::FileStatus::None;
        };
        match ndir_vfs::cloud_status(r.attrs) {
            ndir_vfs::FileStatus::None if self.on_network => ndir_vfs::FileStatus::Network,
            st => st,
        }
    }

    /// 이 소스의 열람 옵션(탭별 보기 옵션의 원천 — 숨김 · Dot · 폴더 우선은 탭이 소유한다).
    pub(crate) fn opts(&self) -> ListOpts {
        self.opts
    }

    pub(crate) fn set_opts(&mut self, opts: ListOpts) {
        if self.opts != opts {
            self.opts = opts;
            self.reload();
        }
    }

    /// 가상 최상위(내 PC)를 보고 있는가.
    pub(crate) fn is_virtual_root(&self) -> bool {
        ndir_vfs::is_virtual_root(&self.path)
    }

    /// 드라이브 행 이름들(가상 최상위) — 호스트가 용량을 조회해 [`Self::set_drive_space`]로 넣는다.
    pub(crate) fn drive_names(&self) -> Vec<String> {
        if !self.is_virtual_root() {
            return Vec::new();
        }
        (0..self.len()).map(|i| self.row(i).text).collect()
    }

    pub(crate) fn drive_space_known(&self) -> bool {
        !self.drive_space.is_empty()
    }

    pub(crate) fn set_drive_space(&mut self, space: Vec<(String, (u64, u64))>) {
        self.drive_space = space.into_iter().collect();
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// 가시 행의 전체 경로.
    pub(crate) fn row_path(&self, index: usize) -> Option<PathBuf> {
        let t = self.tree.as_ref()?;
        let id = t.visible_id(index)?;
        t.node_path(id).map(Path::to_path_buf)
    }

    pub(crate) fn row_is_dir(&self, index: usize) -> bool {
        self.tree
            .as_ref()
            .and_then(|t| t.row(index))
            .is_some_and(|r| r.kind == FileKind::Dir)
    }

    /// 선택된 항목의 전체 경로(선택 순).
    pub(crate) fn selected_paths(&self) -> Vec<PathBuf> {
        self.tree
            .as_ref()
            .map(|t| {
                t.selected_paths()
                    .into_iter()
                    .map(Path::to_path_buf)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 선택된 항목의 전체 경로를 **화면에 보이는 순서**(위 → 아래)로 — 경로/이름 복사용(dir2 `display_order_targets` · win.rs:2759:
    /// 선택 순은 화면 순서와 다르다). 보이는 행에 선택이 없으면 선택 순 그대로.
    pub(crate) fn selected_paths_in_view_order(&self) -> Vec<PathBuf> {
        let Some(t) = self.tree.as_ref() else {
            return Vec::new();
        };
        let out: Vec<PathBuf> = (0..t.visible_len())
            .filter_map(|i| {
                let id = t.visible_id(i)?;
                if !t.is_selected(id) {
                    return None;
                }
                t.node_path(id).map(Path::to_path_buf)
            })
            .collect();
        if out.is_empty() {
            return self.selected_paths();
        }
        out
    }

    pub(crate) fn selection_count(&self) -> usize {
        self.tree.as_ref().map_or(0, Tree::selection_count)
    }

    /// 상태줄 요약 — `status.itemCount` / `status.selectedCount`.
    pub(crate) fn status_text(&self) -> String {
        if let Some(e) = &self.error {
            return e.clone();
        }
        let n = self.len();
        let sel = self.selection_count();
        if sel > 0 {
            ndir_i18n::trf("status.selectedCount", &[&sel.to_string()])
        } else {
            ndir_i18n::trf("status.itemCount", &[&n.to_string()])
        }
    }
}

/// 그리드 컬럼 키 → 코어 정렬 키.
pub(crate) fn sort_key_of(col: u32) -> Option<SortKey> {
    Some(match col {
        COL_NAME => SortKey::Name,
        // 확장자 열 = 확장자순(dir2 source.rs:434 PANEL-065 — 종전 dir3는 종류 키로 묶어 확장자 + 종류 다중 정렬에 같은 키가
        // 두 번 들어갔다 · 10-03 적발).
        COL_EXT => SortKey::Ext,
        COL_KIND => SortKey::Kind,
        COL_SIZE => SortKey::Size,
        COL_MODIFIED => SortKey::Modified,
        COL_STATUS => SortKey::Status,
        _ => return None,
    })
}

/// 크기 서식(dir2 규약 · 탐색기식): 1024 단위 · 소수 1자리(KB 이상) · 폴더는 빈 값(호출자가 결정).
pub(crate) fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    format!("{v:.1} {}", UNITS[u])
}

/// Unix 초(UTC) → `YYYY-MM-DDThh:mm:ss`(freedesktop `.trashinfo` DeletionDate).
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))] // freedesktop Trash(Linux)만 쓴다.
pub(crate) fn format_iso_utc(unix_secs: i64) -> String {
    let days = unix_secs.div_euclid(86_400);
    let rem = unix_secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Unix ms(UTC) → `YYYY-MM-DD HH:MM`(로컬 시간대 변환 없음 — 시간대는 platform 층 과제 · T-72). 0 이하 = 빈 값.
pub(crate) fn format_time(unix_ms: i64) -> String {
    if unix_ms <= 0 {
        return String::new();
    }
    let secs = unix_ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// 1970-01-01 기준 일수 → (년, 월, 일) — Howard Hinnant의 `civil_from_days`(윤년 포함 · 외부 crate 0).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 종류 라벨(i18n `kind.*` — 없으면 영문).
fn kind_label(kind: FileKind, name: &str) -> String {
    match kind {
        FileKind::Dir => ndir_i18n::tr("kind.folder"),
        FileKind::Symlink => ndir_i18n::tr("kind.link"),
        // 바로 가기 파일(.lnk · .url · .appref-ms) = "바로 가기"(탐색기 종류 · 확장자를 이름에서 숨기는 OS에서만 — GAP-008).
        _ if crate::platform::hides_shortcut_ext() && split_shortcut_ext(name).is_some() => {
            ndir_i18n::tr("kind.link")
        }
        _ => match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_ascii_uppercase(),
            _ => ndir_i18n::tr("kind.file"),
        },
    }
}

/// 파일마다 아이콘이 다른 확장자(dir2 `icons.rs:8-36` · PANEL-064) — 키 = 소문자 전체 경로.
const PER_FILE_ICON_EXTS: [&str; 7] = ["exe", "lnk", "ico", "cur", "msi", "scr", "appref-ms"];

/// 행 아이콘 키(dir2 `icons::icon_key` · PANEL-064): 폴더 = `dir` · 확장자 없음 = `file` · 파일별 확장자 = 소문자 전체 경로 · 그 외 = 확장자.
/// 드라이브 루트 같은 **가상 최상위의 항목**은 경로마다 고유 아이콘(셸이 경로를 본다) → 경로 키.
pub(crate) fn icon_key(is_dir: bool, path: &Path, per_path: bool) -> String {
    let lower = || path.to_string_lossy().to_lowercase();
    if per_path {
        return lower();
    }
    if is_dir {
        return "dir".into();
    }
    match path.extension().and_then(|e| e.to_str()) {
        None => "file".into(),
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            if PER_FILE_ICON_EXTS.contains(&ext.as_str()) {
                lower()
            } else {
                ext
            }
        }
    }
}

impl RowSource for TreeSource {
    fn len(&self) -> usize {
        self.tree.as_ref().map_or(0, Tree::visible_len)
    }

    /// 상태 열 = 아이콘 셀(nexa-ui 132차 `cell_icon`) — 표시할 상태가 없으면 빈 칸.
    fn cell_icon(&self, index: usize, key: u32) -> Option<(String, String)> {
        if key != COL_STATUS {
            return None;
        }
        status_icon_key(self.status_of(index)).map(|k| (k.to_string(), String::new()))
    }

    /// 행 아이콘 `(키, 경로)`(dir2 `source.rs:426-432` · M1-7 셸 아이콘) — 그리는 쪽(nexa-grid `Adapt::draw_icon`)이 호스트 리졸버에 묻는다.
    fn icon(&self, index: usize) -> Option<(String, String)> {
        let row = self.tree.as_ref()?.row(index)?;
        let is_dir = row.kind == FileKind::Dir;
        let path = self.row_path(index)?;
        // 링크(심볼릭 링크 · 정션)는 경로로 조회해야 셸이 화살표 오버레이를 얹는다(GAP-008 — 종류별 아이콘에는 링크 정보가 없다).
        let per_path = ndir_vfs::is_virtual_root(&self.path) || ndir_vfs::is_link_attrs(row.attrs);
        Some((
            icon_key(is_dir, &path, per_path),
            path.to_string_lossy().into_owned(),
        ))
    }

    /// 잘라내기 대기 행 = 흐림(nexa-grid X-32 · 집합이 비어 있으면 경로 계산도 하지 않는다 — dir2 `has_cut_marks` 선판정).
    fn is_ghosted(&self, index: usize) -> bool {
        !self.cut_marks.is_empty()
            && self
                .row_path(index)
                .is_some_and(|p| self.cut_marks.contains(&p))
    }

    fn row(&self, index: usize) -> RowItem {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row(index)) else {
            return RowItem {
                text: String::new(),
                is_dir: false,
                depth: 0,
                marker: Marker::None,
            };
        };
        RowItem {
            is_dir: r.kind == FileKind::Dir,
            depth: r.depth,
            marker: if !r.has_children {
                Marker::None
            } else if r.expanded {
                Marker::Expanded
            } else {
                Marker::Collapsed
            },
            text: display_name(
                &r.name,
                r.kind == FileKind::Dir,
                crate::platform::hides_shortcut_ext(),
            )
            .to_string(),
        }
    }

    fn cell(&self, index: usize, key: u32) -> String {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row(index)) else {
            return String::new();
        };
        match key {
            COL_EXT if r.kind == FileKind::Dir => String::new(),
            COL_EXT => match r.name.rsplit_once('.') {
                Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_string(),
                _ => String::new(),
            },
            COL_STATUS => status_label(self.status_of(index)),
            COL_SIZE if r.kind == FileKind::Dir => String::new(),
            COL_SIZE => format_size(r.size),
            COL_MODIFIED => format_time(r.modified_unix_ms),
            COL_KIND
                if (self.drive_space.contains_key(&r.name) || r.name.ends_with(":\\"))
                    && !is_home_dir(&r.name) =>
            {
                ndir_i18n::tr("kind.drive")
            }
            COL_KIND => kind_label(r.kind, &r.name),
            COL_TOTAL => self
                .drive_space
                .get(&r.name)
                .map(|(t, _)| format_size(*t))
                .unwrap_or_default(),
            COL_FREE => self
                .drive_space
                .get(&r.name)
                .map(|(_, f)| format_size(*f))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// 타일 보조 줄(dir2 07-16): 드라이브 = "X 중 Y 사용 가능" + 사용량 바 · 그 밖 = 종류.
    fn tile_info(&self, index: usize) -> (String, Option<f32>) {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row(index)) else {
            return (String::new(), None);
        };
        if let Some(&(total, free)) = self.drive_space.get(&r.name) {
            let used = total.saturating_sub(free);
            let frac = if total > 0 {
                used as f32 / total as f32
            } else {
                0.0
            };
            return (
                ndir_i18n::trf("drive.freeOf", &[&format_size(total), &format_size(free)]),
                Some(frac),
            );
        }
        (self.cell(index, COL_KIND), None)
    }

    fn toggle(&mut self, index: usize) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let Some(id) = t.visible_id(index) else {
            return false;
        };
        match t.is_expanded(id) {
            Some(true) => {
                t.collapse(id);
                true
            }
            Some(false) => t.expand(id).is_ok(),
            None => false,
        }
    }

    fn set_sort(&mut self, keys: &[(u32, bool)]) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let mapped: Vec<(SortKey, bool)> = keys
            .iter()
            .filter_map(|(k, desc)| sort_key_of(*k).map(|s| (s, *desc)))
            .collect();
        t.set_sort(SortSpec {
            keys: if mapped.is_empty() {
                vec![(SortKey::Name, false)]
            } else {
                mapped
            },
            folders_first: self.opts.folders_first,
            case_sensitive: self.opts.case_sensitive,
        });
        true
    }

    fn is_selected(&self, index: usize) -> bool {
        self.tree
            .as_ref()
            .and_then(|t| t.visible_id(index).map(|id| t.is_selected(id)))
            .unwrap_or(false)
    }

    fn select(&mut self, index: usize, op: SelectOp) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let Some(id) = t.visible_id(index) else {
            return false;
        };
        match op {
            SelectOp::Single => t.select(id, SelectMode::Single),
            SelectOp::Toggle => t.select(id, SelectMode::Toggle),
            SelectOp::RangeTo => t.select_range(id),
        }
        true
    }

    fn select_span(&mut self, lo: usize, hi: usize) -> bool {
        let Some(t) = self.tree.as_mut() else {
            return false;
        };
        let (Some(a), Some(b)) = (t.visible_id(lo), t.visible_id(hi)) else {
            return false;
        };
        t.select(a, SelectMode::Single);
        t.select_range(b);
        true
    }

    fn select_all(&mut self) -> bool {
        if let Some(t) = self.tree.as_mut() {
            t.select_all_visible();
            return true;
        }
        false
    }

    fn clear_selection(&mut self) -> bool {
        match self.tree.as_mut() {
            Some(t) if t.selection_count() > 0 => {
                t.clear_selection();
                true
            }
            _ => false,
        }
    }

    fn find_prefix(&self, caret: Option<usize>, prefix: &str) -> Option<usize> {
        self.tree
            .as_ref()?
            .find_prefix(caret, prefix, FindScope::VisibleStream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ListOpts {
        ListOpts {
            show_hidden: true,
            show_dotfiles: true,
            show_protected: true,
            folders_first: true,
            case_sensitive: false,
        }
    }

    #[test]
    fn size_and_time_format() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1_536_000), "1.5 MB");
        assert_eq!(format_time(0), "");
        // 2026-10-03 12:34:56 UTC
        assert_eq!(format_time(1_791_030_896_000), "2026-10-03 12:34");
        assert_eq!(format_iso_utc(1_791_030_896), "2026-10-03T12:34:56");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29), "윤년");
    }

    #[test]
    fn opens_folder_dirs_first_and_toggles() {
        let dir = std::env::temp_dir().join(format!("ndir-filelist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), b"hello").unwrap();
        std::fs::write(dir.join("sub/inner.rs"), b"x").unwrap();
        let mut src = TreeSource::open(&dir, opts());
        assert!(src.error().is_none());
        assert_eq!(src.len(), 2);
        let r0 = src.row(0);
        assert!(r0.is_dir && r0.text == "sub" && r0.marker == Marker::Collapsed);
        assert_eq!(src.row(1).text, "a.txt");
        assert_eq!(src.cell(1, COL_SIZE), "5 B");
        assert_eq!(src.cell(0, COL_SIZE), "");
        assert_eq!(src.cell(1, COL_KIND), "TXT");
        assert_eq!(src.cell(1, COL_EXT), "txt");
        assert_eq!(src.cell(0, COL_EXT), "");
        assert_eq!(title_of(&dir.join("sub")), "sub");
        // Unix 루트 = `/`(공백 아님) · 드라이브 루트 = `D:`.
        assert_eq!(title_of(Path::new("/")), "/");
        // 열 → 정렬 키: 확장자와 종류는 서로 다른 키 · 내 PC 열(전체 크기 · 여유 공간)은 정렬 대상이 아니다.
        assert_eq!(sort_key_of(COL_EXT), Some(SortKey::Ext));
        assert_eq!(sort_key_of(COL_KIND), Some(SortKey::Kind));
        assert_eq!(sort_key_of(COL_STATUS), Some(SortKey::Status));
        assert_eq!(sort_key_of(COL_TOTAL), None);
        #[cfg(windows)]
        assert_eq!(title_of(Path::new("D:\\")), "D:");
        assert_eq!(
            title_of(Path::new(ndir_vfs::MY_PC)),
            ndir_i18n::tr("nav.mypc")
        );
        assert_eq!(display_path(&dir), dir.to_string_lossy());
        assert!(src.toggle(0), "펼침");
        assert_eq!(src.len(), 3);
        assert_eq!(src.row(1).text, "inner.rs");
        assert_eq!(src.row(1).depth, 1);
        assert!(src.row_path(1).unwrap().ends_with("inner.rs"));
        assert!(src.toggle(0), "접힘");
        assert_eq!(src.len(), 2);
        // 선택
        assert!(src.select(1, SelectOp::Single));
        assert!(src.is_selected(1) && !src.is_selected(0));
        assert_eq!(src.selection_count(), 1);
        assert!(src.select_all());
        assert_eq!(src.selection_count(), 2);
        assert!(src.clear_selection());
        assert!(!src.clear_selection(), "이미 비었으면 false");
        // 타입어헤드
        assert_eq!(src.find_prefix(None, "a"), Some(1));
        // 정렬 키 대응
        assert_eq!(sort_key_of(COL_MODIFIED), Some(SortKey::Modified));
        assert_eq!(sort_key_of(99), None);
        assert!(src.set_sort(&[(COL_SIZE, true)]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 내 PC 열: 용량을 넣으면 전체/여유 셀 + 타일 보조 줄 + 종류 = 드라이브 · 안 넣으면 빈 셀.
    #[test]
    fn drive_columns_use_injected_space() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let mut src = TreeSource::open(Path::new(ndir_vfs::MY_PC), opts());
        assert!(src.is_virtual_root());
        let names = src.drive_names();
        if names.is_empty() {
            return; // 비Windows: 드라이브 열거 없음(X-17) — 열 전환은 panel 시험이 본다.
        }
        assert_eq!(src.cell(0, COL_TOTAL), "");
        src.set_drive_space(vec![(names[0].clone(), (2048, 1024))]);
        assert!(src.drive_space_known());
        assert_eq!(src.cell(0, COL_TOTAL), "2.0 KB");
        assert_eq!(src.cell(0, COL_FREE), "1.0 KB");
        assert_eq!(src.cell(0, COL_KIND), "Drive");
        let (line, frac) = src.tile_info(0);
        assert_eq!(line, "1.0 KB free of 2.0 KB");
        assert_eq!(frac, Some(0.5));
    }

    #[test]
    fn missing_folder_is_empty_with_error() {
        let src = TreeSource::open(Path::new("Z:/definitely/not/here-ndir"), opts());
        assert_eq!(src.len(), 0);
        assert!(src.error().is_some());
        assert!(!src.status_text().is_empty());
        assert_eq!(src.row(0).text, "");
    }

    /// GAP-006: 바로 가기 확장자 숨김/복원(순수 · MC/DC — 숨기는 OS × 폴더 여부 × 확장자 유무).
    #[test]
    fn shortcut_ext_is_hidden_and_restored_on_rename() {
        assert_eq!(split_shortcut_ext("Chrome.lnk"), Some(("Chrome", ".lnk")));
        assert_eq!(split_shortcut_ext("Site.URL"), Some(("Site", ".URL")));
        assert_eq!(
            split_shortcut_ext("App.appref-ms"),
            Some(("App", ".appref-ms"))
        );
        assert_eq!(split_shortcut_ext(".lnk"), None, "이름이 확장자뿐");
        assert_eq!(split_shortcut_ext("메모.txt"), None);
        assert_eq!(display_name("Chrome.lnk", false, true), "Chrome");
        assert_eq!(
            display_name("Chrome.lnk", false, false),
            "Chrome.lnk",
            "숨기지 않는 OS"
        );
        assert_eq!(
            display_name("folder.lnk", true, true),
            "folder.lnk",
            "폴더는 그대로"
        );
        assert_eq!(
            display_name("한글 바로 가기.lnk", false, true),
            "한글 바로 가기"
        );
        assert_eq!(
            restore_shortcut_ext("Chrome.lnk", "Browser", false, true),
            "Browser.lnk"
        );
        assert_eq!(
            restore_shortcut_ext("Chrome.lnk", "Browser.LNK", false, true),
            "Browser.LNK"
        );
        assert_eq!(
            restore_shortcut_ext("Chrome.lnk", "Browser", false, false),
            "Browser"
        );
        assert_eq!(restore_shortcut_ext("a.txt", "b", false, true), "b");
        assert_eq!(restore_shortcut_ext("d.lnk", "e", true, true), "e");
        assert_eq!(restore_shortcut_ext("Chrome.lnk", "", false, true), "");
        // 종류 열(GAP-008): 바로 가기 파일 = "바로 가기"(확장자를 숨기는 OS) · 그 밖 = 확장자 대문자.
        let link = ndir_i18n::tr("kind.link");
        if crate::platform::hides_shortcut_ext() {
            assert_eq!(kind_label(FileKind::File, "Chrome.lnk"), link);
            assert_eq!(kind_label(FileKind::File, "Site.url"), link);
        } else {
            assert_eq!(kind_label(FileKind::File, "Chrome.lnk"), "LNK");
        }
        assert_eq!(kind_label(FileKind::File, "a.txt"), "TXT");
        assert_eq!(kind_label(FileKind::Symlink, "x"), link);
    }
}
