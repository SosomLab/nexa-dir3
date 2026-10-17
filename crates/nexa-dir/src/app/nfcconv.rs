//! 이름을 NFC로 변환(행 메뉴 `ctx.nfc` · 10-18 · macOS HFS+/SMB에서 온 NFD 이름): 선택에 폴더가 있으면 "하위 포함?"을 먼저 묻고(예 = 재귀 ·
//! 아니오 = 선택한 것만 · 취소) · 변환 = 일괄 이름 변경 엔진(`bulk_apply` · undo 1건 · 실패 개별 격리 · 재열람 · 토스트).

use crate::app::dialogs::DlgReply;
use crate::dlg_win::DlgSpec;
use crate::*;

impl App {
    /// `ctx.nfc` — 폴더가 섞여 있으면 확인 창 · 파일만이면 바로.
    pub(crate) fn nfc_convert_ask(&mut self, panel: usize) {
        let sel = self.panels[panel].selected_paths_in_view_order();
        if sel.is_empty() {
            return;
        }
        if sel.iter().any(|p| p.is_dir()) {
            let spec = DlgSpec {
                title: tr("nfc.title"),
                text: trf("nfc.askSub", &[&sel.len().to_string()]),
                buttons: vec![
                    (1, tr("nfc.withSub")),
                    (2, tr("nfc.onlySelf")),
                    (0, tr("dlg.cancel")),
                ],
                default: 1,
                cancel: 0,
                input: None,
            };
            self.ask(spec, DlgReply::NfcConvert(sel));
        } else {
            self.nfc_convert(sel, false);
        }
    }

    /// 변환 실행(`recursive` = 폴더 안까지 · 후위 순회라 부모 이름이 바뀌어도 자식 경로가 유효).
    pub(crate) fn nfc_convert(&mut self, paths: Vec<PathBuf>, recursive: bool) {
        let list = ndir_ops::nfc::rename_plan(&paths, recursive);
        if list.is_empty() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("nfc.none"), &mut inv);
            self.redraw();
            return;
        }
        self.bulk_apply(list);
    }
}
