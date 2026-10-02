//! 파일 목록 행 공급자 — `ndir-tree::Tree`(dir2 인라인 트리 평면 스트림)를 nexa-grid [`RowSource`]로 잇는다.
//!
//! dir2에서는 `nexa-app/src/panel.rs`가 같은 일을 했다(docs/port/13 · 21 G-1): 위젯은 가시 행만 묻고, 펼침/선택/정렬/타입어헤드는
//! 코어(`ndir-tree`)가 결정한다. 이 파일은 **번역만**(셀 텍스트 서식 포함) — 창·OS API를 모른다(docs/15 §2-1).
//!
//! 컬럼 키: 0 이름(트리) · 1 크기 · 2 수정 시각 · 3 종류(`col.*` i18n). 정렬 키 대응 = [`sort_key_of`].

use ndir_core::FileKind;
use ndir_tree::{FindScope, SelectMode, SortKey, SortSpec, Tree};
use nexa_grid::{Marker, RowItem, RowSource, SelectOp};
use std::path::{Path, PathBuf};

/// 컬럼 키(그리드 `Column.key`).
pub(crate) const COL_NAME: u32 = 0;
pub(crate) const COL_SIZE: u32 = 1;
pub(crate) const COL_MODIFIED: u32 = 2;
pub(crate) const COL_KIND: u32 = 3;

/// 열람 옵션(설정 `list.*`에서) — 다시 열 때 그대로 쓴다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ListOpts {
    pub show_hidden: bool,
    pub show_dotfiles: bool,
    pub folders_first: bool,
    pub case_sensitive: bool,
}

/// 한 패널의 행 공급자.
#[derive(Debug)]
pub(crate) struct TreeSource {
    tree: Option<Tree>,
    path: PathBuf,
    opts: ListOpts,
    /// 열기 실패 사유(상태줄에 · 빈 목록).
    error: Option<String>,
}

impl TreeSource {
    /// 폴더를 연다 — 실패해도 공급자는 만들어진다(빈 목록 + `error`).
    pub(crate) fn open(path: &Path, opts: ListOpts) -> TreeSource {
        let mut s = TreeSource {
            tree: None,
            path: path.to_path_buf(),
            opts,
            error: None,
        };
        s.reload();
        s
    }

    /// 같은 경로를 다시 읽는다(F5 · 옵션 변경).
    pub(crate) fn reload(&mut self) {
        match Tree::open_filtered(&self.path, self.opts.show_hidden, self.opts.show_dotfiles) {
            Ok(mut t) => {
                t.set_sort(SortSpec {
                    keys: vec![(SortKey::Name, false)],
                    folders_first: self.opts.folders_first,
                    case_sensitive: self.opts.case_sensitive,
                });
                self.tree = Some(t);
                self.error = None;
            }
            Err(e) => {
                self.tree = None;
                self.error = Some(e.to_string());
            }
        }
    }

    pub(crate) fn set_opts(&mut self, opts: ListOpts) {
        if self.opts != opts {
            self.opts = opts;
            self.reload();
        }
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
        COL_SIZE => SortKey::Size,
        COL_MODIFIED => SortKey::Modified,
        COL_KIND => SortKey::Kind,
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
        _ => match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_ascii_uppercase(),
            _ => ndir_i18n::tr("kind.file"),
        },
    }
}

impl RowSource for TreeSource {
    fn len(&self) -> usize {
        self.tree.as_ref().map_or(0, Tree::visible_len)
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
            text: r.name,
        }
    }

    fn cell(&self, index: usize, key: u32) -> String {
        let Some(r) = self.tree.as_ref().and_then(|t| t.row(index)) else {
            return String::new();
        };
        match key {
            COL_SIZE if r.kind == FileKind::Dir => String::new(),
            COL_SIZE => format_size(r.size),
            COL_MODIFIED => format_time(r.modified_unix_ms),
            COL_KIND => kind_label(r.kind, &r.name),
            _ => String::new(),
        }
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

    #[test]
    fn missing_folder_is_empty_with_error() {
        let src = TreeSource::open(Path::new("Z:/definitely/not/here-ndir"), opts());
        assert_eq!(src.len(), 0);
        assert!(src.error().is_some());
        assert!(!src.status_text().is_empty());
        assert_eq!(src.row(0).text, "");
    }
}
