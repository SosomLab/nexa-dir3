//! App — 폴더 즐겨찾기(T-168 · NEW-039 · dir3 신규 · 사용자 10-05 "대상 목록"): Total Commander 디렉터리 핫리스트식.
//!
//! - `nav.fav_toggle`(Ctrl+D): 활성 패널의 현재 폴더를 즐겨찾기에 넣거나 뺀다(상태줄 안내).
//! - `nav.favorites`(Ctrl+B): 경로 바 아래에 즐겨찾기 목록 팝업 — 고르면 그 폴더로 이동 · 끝에 "현재 폴더 추가/제거".
//! - 저장 = 설정 `nav.favorites`(`;;` 구분 · 최대 64 · HIDDEN — 메뉴로만 바뀐다). 런처(`launcher.items` = 프로그램 실행)와는
//!   별개다 — 즐겨찾기는 "폴더로 이동"이라 성격이 다르다(통합은 사용자 결정 뒤 · T-168 비고).

use crate::app::ctxmenu::CtxKind;
use crate::*;
use std::path::Path;

const SEP: &str = ";;";
const MAX: usize = 64;

/// 즐겨찾기 내보내기 저장 창의 기본 파일 이름(T-196).
pub(crate) const FAV_EXPORT_NAME: &str = "nexa-dir-favorites.txt";

/// 즐겨찾기 파일(T-196 · 사용자 10-24): UTF-8 글 · **한 줄 = 폴더 1개** · `#` 줄 = 주석 · 빈 줄 무시 — 다른 PC로 옮기거나 손으로 고칠 수
/// 있게 설정 `nav.favorites`(`;;` 연결)와 다른 꼴을 쓴다.
pub(crate) fn encode_fav_file(list: &[PathBuf]) -> String {
    let mut out = String::from(
        "# Nexa Dir favorites - one folder per line (lines starting with # are ignored)\n",
    );
    for p in list {
        out.push_str(&p.display().to_string());
        out.push('\n');
    }
    out
}

/// 즐겨찾기 파일 → 폴더 목록(순수 · 주석/빈 줄 제거 · 중복 제거 · 최대 64).
pub(crate) fn parse_fav_file(text: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for raw in text.lines() {
        let s = raw.trim().trim_start_matches('\u{feff}');
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let p = PathBuf::from(s);
        if !out.iter().any(|q| same_fav(q, &p)) {
            out.push(p);
        }
        if out.len() >= MAX {
            break;
        }
    }
    out
}

/// 가져오기 병합(순수): 지금 목록 뒤에 **없던 폴더만** 덧붙인다(순서 유지 · 상한 64) → (새 목록, 추가 수, 이미 있던 수).
pub(crate) fn merge_favs(cur: &[PathBuf], incoming: &[PathBuf]) -> (Vec<PathBuf>, usize, usize) {
    let mut out: Vec<PathBuf> = cur.to_vec();
    let (mut added, mut dup) = (0usize, 0usize);
    for p in incoming {
        if out.iter().any(|q| same_fav(q, p)) {
            dup += 1;
            continue;
        }
        if out.len() >= MAX {
            break;
        }
        out.push(p.clone());
        added += 1;
    }
    (out, added, dup)
}

/// 설정 값 → 폴더 목록(순수 · 빈 항목 제거 · 중복 제거 · 최대 64).
pub(crate) fn parse_favs(text: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for raw in text.split(SEP) {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        let p = PathBuf::from(s);
        if !out.iter().any(|q| same_fav(q, &p)) {
            out.push(p);
        }
        if out.len() >= MAX {
            break;
        }
    }
    out
}

/// 폴더 목록 → 설정 값.
pub(crate) fn encode_favs(list: &[PathBuf]) -> String {
    list.iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(SEP)
}

/// 같은 폴더인가(끝 구분자 무시 · Windows는 대소문자 무시).
pub(crate) fn same_fav(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy();
        let s = s.trim_end_matches(['\\', '/']);
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s.to_string()
        }
    };
    norm(a) == norm(b)
}

impl App {
    pub(crate) fn favorites(&self) -> Vec<PathBuf> {
        parse_favs(self.settings.get("nav.favorites").unwrap_or(""))
    }

    fn save_favorites(&mut self, list: &[PathBuf]) {
        let _ = self.settings.set("nav.favorites", &encode_favs(list));
        let _ = self.settings.save();
    }

    /// 활성 패널의 현재 폴더가 즐겨찾기에 있는가(가상 최상위는 대상이 아니다).
    pub(crate) fn fav_has_current(&self) -> Option<bool> {
        let root = self.panels[self.active].root_path();
        if ndir_vfs::is_virtual_root(&root) {
            return None;
        }
        Some(self.favorites().iter().any(|p| same_fav(p, &root)))
    }

    /// `nav.fav_toggle` — 현재 폴더를 넣거나 뺀다 · 상태줄 안내.
    pub(crate) fn fav_toggle(&mut self) {
        let root = self.panels[self.active].root_path();
        let Some(has) = self.fav_has_current() else {
            return;
        };
        let mut list = self.favorites();
        let text = if has {
            list.retain(|p| !same_fav(p, &root));
            trf("fav.removed", &[&filelist::display_path(&root)])
        } else {
            list.push(root.clone());
            list.truncate(MAX);
            trf("fav.added", &[&filelist::display_path(&root)])
        };
        self.save_favorites(&list);
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&text, &mut inv);
        self.redraw();
    }

    /// `nav.favorites` — 경로 바 아래에 목록 팝업(보조 메뉴 · 항목 id `aux.fav:<번호>` · 끝 = 현재 폴더 추가/제거).
    pub(crate) fn open_favorites_menu(&mut self) {
        let list = self.favorites();
        let mut items: Vec<CtxItem> = Vec::new();
        if list.is_empty() {
            items.push(CtxItem::maybe("aux.fav.none", tr("fav.empty"), false));
        }
        for (i, p) in list.iter().enumerate() {
            // 지금 없는 폴더(분리된 드라이브 · 지워진 폴더)는 회색(고를 수 없음 · 목록에는 남겨 둔다 — 다시 붙으면 살아난다).
            items.push(CtxItem::maybe(
                format!("aux.fav:{i}"),
                filelist::display_path(p),
                p.is_dir(),
            ));
        }
        if let Some(has) = self.fav_has_current() {
            items.push(CtxItem::Separator);
            items.push(CtxItem::item(
                "aux.fav.toggle",
                tr(if has { "fav.remove" } else { "fav.add" }),
            ));
        }
        // 내보내기/가져오기(T-196) — 팝업 끝(이동 메뉴 · 팔레트에도 같은 명령).
        items.push(CtxItem::Separator);
        items.push(CtxItem::item("aux.fav.export", tr("cmd.favExport")));
        items.push(CtxItem::item("aux.fav.import", tr("cmd.favImport")));
        let r = self.panels[self.active].pathbar.bounds();
        self.ctx_anchor_next = Some((r.x, r.bottom()));
        self.open_ctx(CtxKind::Aux(self.active), items);
    }

    /// 보조 메뉴의 즐겨찾기 항목 처리 — 처리했으면 `true`.
    pub(crate) fn fav_menu_action(&mut self, panel: usize, id: &str) -> bool {
        if let Some(i) = id
            .strip_prefix("aux.fav:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            if let Some(p) = self.favorites().get(i).cloned() {
                let mut inv = Invalidations::default();
                if let Some(why) = self.panels[panel].navigate_to(p, &mut inv) {
                    self.statusbar.set_left(&why, &mut inv);
                }
                self.update_status();
                self.redraw();
            }
            return true;
        }
        if id == "aux.fav.toggle" {
            self.fav_toggle();
            return true;
        }
        if id == "aux.fav.export" {
            self.open_file_window(crate::app::license::FilePurpose::FavExport);
            return true;
        }
        if id == "aux.fav.import" {
            self.open_file_window(crate::app::license::FilePurpose::FavImport);
            return true;
        }
        false
    }

    /// ★ 즐겨찾기를 파일로(T-196 · 한 줄 = 폴더 1개) — 상태줄 · 로그.
    pub(crate) fn fav_export_to(&mut self, path: &Path) {
        let list = self.favorites();
        let shown = path.display().to_string();
        let (kind, text) = match std::fs::write(path, encode_fav_file(&list)) {
            Ok(()) => (
                ndir_log::LogKind::Info,
                trf("fav.exported", &[&shown, &list.len().to_string()]),
            ),
            Err(e) => (
                ndir_log::LogKind::Error,
                trf("fav.fileError", &[&shown, &e.to_string()]),
            ),
        };
        self.log(kind, text.clone());
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&text, &mut inv);
        self.redraw();
    }

    /// ★ 파일에서 즐겨찾기 가져오기 — 지금 목록 뒤에 없던 폴더만 덧붙인다(지우지 않는다 · 상한 64) — 상태줄 · 로그.
    pub(crate) fn fav_import_from(&mut self, path: &Path) {
        let shown = path.display().to_string();
        let (kind, text) = match std::fs::read_to_string(path) {
            Ok(text) => {
                let incoming = parse_fav_file(&text);
                let (merged, added, dup) = merge_favs(&self.favorites(), &incoming);
                if added > 0 {
                    self.save_favorites(&merged);
                }
                (
                    ndir_log::LogKind::Info,
                    trf(
                        "fav.imported",
                        &[&shown, &added.to_string(), &dup.to_string()],
                    ),
                )
            }
            Err(e) => (
                ndir_log::LogKind::Error,
                trf("fav.fileError", &[&shown, &e.to_string()]),
            ),
        };
        self.log(kind, text.clone());
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&text, &mut inv);
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorites_parse_encode_round_trip() {
        let list = parse_favs(" D:/a ;; ;;D:/b;;d:/A/;;D:/c ");
        assert_eq!(list.len(), if cfg!(windows) { 3 } else { 4 });
        assert_eq!(list[0], PathBuf::from("D:/a"));
        assert_eq!(parse_favs(&encode_favs(&list)), list);
        assert!(parse_favs("").is_empty());
        assert!(same_fav(Path::new("D:/x/"), Path::new("D:/x")));
        let many = (0..100)
            .map(|i| format!("D:/f{i}"))
            .collect::<Vec<_>>()
            .join(SEP);
        assert_eq!(parse_favs(&many).len(), MAX);
    }

    /// 즐겨찾기 파일(T-196): 한 줄 = 폴더 · 주석/빈 줄/BOM 무시 · 중복 제거 · 왕복 · 병합은 없던 것만 덧붙이고 상한을 지킨다.
    #[test]
    fn fav_file_round_trip_and_merge() {
        let list = vec![PathBuf::from("D:/a"), PathBuf::from("D:/b c")];
        let text = encode_fav_file(&list);
        assert!(text.starts_with('#') && text.contains("D:/b c\n"));
        assert_eq!(parse_fav_file(&text), list);
        // 끝 구분자 차이는 모든 OS에서 같은 폴더(`same_fav` · 대소문자만 Windows 한정) → "D:/a/"는 unix에서도 중복.
        let parsed = parse_fav_file("\u{feff}# x\n\n  D:/a  \nD:/a/\nD:/c\n");
        assert_eq!(parsed, vec![PathBuf::from("D:/a"), PathBuf::from("D:/c")]);
        let (merged, added, dup) =
            merge_favs(&list, &[PathBuf::from("D:/a"), PathBuf::from("D:/z")]);
        assert_eq!((added, dup), (1, 1));
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[2], PathBuf::from("D:/z"));
        let full: Vec<PathBuf> = (0..MAX)
            .map(|i| PathBuf::from(format!("D:/f{i}")))
            .collect();
        let (m2, added, _) = merge_favs(&full, &[PathBuf::from("D:/more")]);
        assert_eq!((m2.len(), added), (MAX, 0), "상한 64");
    }
}
