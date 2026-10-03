//! 대화상자 호스트 배선(T-29 A): 요청 큐(`dlg_pending`) → 보조 창 열기 → 결과 분기(`DlgReply`). 창 없이도 `dlg_pick`으로 결정(시험·기동 명령).
//! 소비자: 영구 삭제 확인(`edit.delete_permanent`) · 전송 충돌 4버튼(작업 스레드 요청 → UI 결정 → 채널 회신) · 압축 암호(T-62 B).

use crate::app::ops::ConflictChoice;
use crate::dlg_win::{DlgAction, DlgSpec};
use crate::*;
use std::sync::mpsc;

/// 대화상자 결과를 누가 받는가.
pub(crate) enum DlgReply {
    /// 확인(1)이면 영구 삭제.
    DeletePermanent(Vec<PathBuf>),
    /// 전송 충돌 — 작업 스레드가 기다린다(1 덮어쓰기 · 2 모두 덮어쓰기 · 3 건너뛰기 · 그 외 취소).
    Conflict(mpsc::Sender<ConflictChoice>),
    /// 압축 암호(마스킹 입력) — 확인(1)이면 입력 텍스트로 재조회(T-62 B).
    ArchivePassword(PathBuf),
    /// About(T-80 LIC-158 ⓑ) — 2 = 라이선스 창.
    About,
    /// 일괄 이름 변경 프리셋 저장(이름 입력 · T-71).
    BulkPreset,
}

impl App {
    /// 대화상자 요청(이미 하나가 열려 있거나 대기 중이면 거절 — 동시 1건 · 상태줄 안내).
    pub(crate) fn ask(&mut self, spec: DlgSpec, reply: DlgReply) -> bool {
        if self.dlg.is_open() || self.dlg_pending.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("ops.busy"), &mut inv);
            self.redraw();
            return false;
        }
        self.dlg_pending = Some((spec, reply));
        true
    }

    /// 대기 중 사양(창이 아직 없을 때 — 시험·덤프).
    pub(crate) fn dlg_dump(&self) -> String {
        if let Some(s) = self.dlg.spec() {
            return format!("open {}\n", s.dump());
        }
        if let Some((s, _)) = &self.dlg_pending {
            return format!("pending {}\n", s.dump());
        }
        "none\n".into()
    }

    /// 버튼 id로 결정(기동 명령 `dlg.pick:<id>` · 시험) — 창이 있으면 창의 입력란 텍스트를 동봉, 없으면 대기 사양으로 바로.
    pub(crate) fn dlg_pick(&mut self, id: i32) {
        if self.dlg.is_open() {
            if let Some(DlgAction::Done { id, text }) = self.dlg.pick(id) {
                self.dlg.close();
                self.dlg_done(id, text);
            }
            return;
        }
        if let Some((spec, reply)) = self.dlg_pending.take() {
            // 창 없이: 사양을 창 상태기계에 실어 입력란 텍스트(dlg.type)까지 동봉.
            self.dlg.set_spec(spec);
            let text = self.dlg.input_text();
            self.dlg.close();
            self.dlg_reply = Some(reply);
            self.dlg_done(id, text);
        }
    }

    /// 입력란 텍스트(기동 명령 `dlg.type:<text>`).
    pub(crate) fn dlg_type(&mut self, text: &str) {
        if self.dlg.is_open() {
            self.dlg.set_input_text(text);
        } else if let Some((spec, _)) = &mut self.dlg_pending {
            if let Some(i) = &mut spec.input {
                i.initial = text.to_string();
            }
        }
    }

    /// 결과 분기.
    pub(crate) fn dlg_done(&mut self, id: i32, text: Option<String>) {
        let _ = text; // 암호 입력(T-62 B)이 쓴다.
        let Some(reply) = self.dlg_reply.take() else {
            return;
        };
        match reply {
            DlgReply::DeletePermanent(paths) => {
                if id == 1 {
                    self.delete_permanent(&paths);
                }
            }
            DlgReply::ArchivePassword(path) => self.archive_password_result(path, id, text),
            DlgReply::About => {
                if id == 2 {
                    self.open_license = true;
                }
            }
            DlgReply::BulkPreset => self.bulk_preset_saved(id, text),
            DlgReply::Conflict(tx) => {
                let choice = match id {
                    1 => ConflictChoice::Overwrite,
                    2 => ConflictChoice::OverwriteAll,
                    3 => ConflictChoice::Skip,
                    _ => ConflictChoice::Cancel,
                };
                let _ = tx.send(choice);
            }
        }
        self.redraw();
    }

    /// `edit.delete_permanent` — 확인 뒤 `ndir_ops::delete_permanent`(휴지통 경유 없음 · undo 없음).
    pub(crate) fn delete_permanent_ask(&mut self) {
        let paths = self.panels[self.active].selected_paths();
        if paths.is_empty() {
            return;
        }
        let spec = DlgSpec::confirm(
            tr("del.title"),
            trf("del.confirm", &[&paths.len().to_string()]),
            tr("menu.edit.delete"),
        );
        self.ask(spec, DlgReply::DeletePermanent(paths));
    }

    fn delete_permanent(&mut self, paths: &[PathBuf]) {
        let mut ok = 0usize;
        let mut failed = 0usize;
        for p in paths {
            match ndir_ops::delete_permanent(p) {
                Ok(()) => ok += 1,
                Err(_) => failed += 1,
            }
        }
        let mut text = trf("del.done", &[&tr("del.kindPermanent"), &ok.to_string()]);
        if failed > 0 {
            text.push_str(" · ");
            text.push_str(&trf("del.partialFail", &[&failed.to_string()]));
        }
        self.toasts.push(
            if failed > 0 {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("del.title"),
            text,
        );
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        self.update_status();
    }

    /// 전송 충돌 요청 → 4버튼(dir2 QA 07-14 개정): 덮어쓰기(1) · 모두 덮어쓰기(2) · 건너뛰기(3) · 취소(4).
    pub(crate) fn conflict_ask(
        &mut self,
        path: &std::path::Path,
        tx: mpsc::Sender<ConflictChoice>,
    ) {
        let spec = DlgSpec {
            title: tr("ops.overwriteTitle"),
            text: trf("ops.overwrite", &[&ndir_ops::leaf_name(path)]),
            buttons: vec![
                (1, tr("ops.yes")),
                (2, tr("ops.yesAll")),
                (3, tr("ops.skip")),
                (4, tr("ops.cancel")),
            ],
            default: 1,
            cancel: 4,
            input: None,
        };
        if !self.ask(spec, DlgReply::Conflict(tx.clone())) {
            let _ = tx.send(ConflictChoice::Skip);
        }
    }
}
