//! 대화상자 호스트 배선(T-29 A): 요청 큐(`dlg_pending`) → 보조 창 열기 → 결과 분기(`DlgReply`). 창 없이도 `dlg_pick`으로 결정(시험·기동 명령).
//! 소비자: 영구 삭제 확인(`edit.delete_permanent`) · 전송 충돌 4버튼(작업 스레드 요청 → UI 결정 → 채널 회신) · 압축 암호(T-62 B).

use crate::app::ops::ConflictChoice;
use crate::dlg_win::{DlgAction, DlgInput, DlgSpec};
use crate::*;
use std::sync::mpsc;

/// 완전 삭제 워커(GAP-040): 워커가 항목마다 `(경로, 성공)`을 보내고 끝나면 채널을 닫는다 — UI는 `App::delete_tick`이 수거.
pub(crate) struct DeleteJob {
    /// 항목마다 (경로 · 지우지 못한 안쪽 경로 목록 — 빈 목록 = 전부 삭제 · GAP-046).
    rx: mpsc::Receiver<(PathBuf, Vec<PathBuf>)>,
    total: usize,
    done: usize,
    /// 지우지 못한 경로(안쪽 항목 · 전체 경로) — [다시 시도]는 이 목록만.
    failed: Vec<PathBuf>,
    /// 실패가 하나라도 있던 최상위 항목 수(토스트 "n deleted").
    failed_items: usize,
    /// 지우려던 최상위 경로(폴더 크기 캐시 무효화 — 자신 + 상위 · 11-09).
    paths: Vec<PathBuf>,
}

/// 대화상자 결과를 누가 받는가.
pub(crate) enum DlgReply {
    /// 확인(1)이면 영구 삭제.
    DeletePermanent(Vec<PathBuf>),
    /// 삭제 전 잠금 안내(WINB-024) — 1 = 잠긴 것만 빼고 삭제 · 2 = 다시 검사 · 그 외 = 취소.
    DeleteLocked {
        paths: Vec<PathBuf>,
        locked: Vec<PathBuf>,
        /// 출처(GAP-029) — 취소(0)면 미러는 문구 + 재비교.
        origin: crate::app::menus::TrashOrigin,
    },
    /// 완전 삭제 실패 안내(GAP-040) — 1 = 실패분만 다시 시도(확인 없이 워커로) · 그 외 = 닫기.
    DeleteFailedPermanent(Vec<PathBuf>),
    /// 휴지통 삭제 실패 안내(dir2 X-35) — 1 = 실패분만 다시 시도(잠금 검사부터) · 출처 유지.
    DeleteFailed {
        failed: Vec<PathBuf>,
        origin: crate::app::menus::TrashOrigin,
    },
    /// 전송 충돌 — 작업 스레드가 기다린다(1 덮어쓰기 · 2 모두 덮어쓰기 · 3 건너뛰기 · 그 외 취소).
    Conflict(mpsc::Sender<ConflictChoice>),
    /// 압축 암호(마스킹 입력) — 확인(1)이면 입력 텍스트로 재조회(T-62 B).
    ArchivePassword(PathBuf),
    /// About(T-80 LIC-158 ⓑ) — 2 = 라이선스 창.
    About,
    /// 일괄 이름 변경 프리셋 저장(이름 입력 · T-71).
    BulkPreset,
    /// 빠른 실행 항목 추가(`None`) · 편집(`Some(자리)`) — 확인(1)이면 입력(`라벨|실행 파일|인자`)을 넣는다(T-133).
    LauncherItem(Option<usize>),
    /// 계산 컬럼 추가(`None`) · 편집(`Some(자리)`) — 확인(1)이면 입력(`이름 = 식`)을 활성 탭에 넣는다(NEW-060).
    CalcColumn(Option<usize>),
    /// 이름을 NFC로 변환(10-18): 1 = 폴더 안까지 · 2 = 선택한 것만 · 그 외 취소.
    NfcConvert(Vec<PathBuf>),
    /// 패턴으로 선택(NEW-054 · 10-25): 1 = 선택에 더함 · 2 = 선택에서 뺌 · 그 외 취소 — 입력 = `;` 구분 글롭.
    SelectPattern,
    /// 체크섬 검증 파일 저장 — 이미 있을 때 덮어쓰기 확인(1 = 쓴다 · T-167 · 10-26).
    HashSave { dest: PathBuf, text: String },
    /// 기동 때 남은 큐 작업(T-202 ④ 슬라이스 3 · docs/26 §12.4): 1 = 재개 · 2 = 버림 · 그 외 = 그대로(다음 기동에 다시 묻는다).
    QueueRestore(Vec<(u64, ndir_xfer::JobSpec)>),
}

/// 덮어쓰기 질문 글 — 이름 + **대상 폴더**(사용자 10-05 "대상 폴더 식별이 되지 않아": 종전에는 이름만 있어 어느 폴더에
/// 덮어쓰는지 알 수 없었다). `path` = 이미 있는 대상 파일의 경로.
pub(crate) fn overwrite_question(path: &std::path::Path) -> String {
    let folder = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    trf("ops.overwrite", &[&ndir_ops::leaf_name(path), &folder])
}

/// **폴더** 충돌 질문(DR-37 · 사용자 11-02 "병합 처리 안내"): 설정의 뜻(병합/교체)을 질문 글에 — 보관은 묻지 않으므로 오지 않는다(오면 병합 글).
pub(crate) fn dir_question(path: &std::path::Path, mode: ndir_ops::DirConflict) -> String {
    let folder = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let key = match mode {
        ndir_ops::DirConflict::Replace => "ops.replaceDir",
        _ => "ops.mergeDir",
    };
    trf(key, &[&ndir_ops::leaf_name(path), &folder])
}

impl App {
    /// 대화상자 요청(이미 하나가 열려 있거나 대기 중이면 거절 — 동시 1건 · 상태줄 안내).
    pub(crate) fn ask(&mut self, spec: DlgSpec, reply: DlgReply) -> bool {
        if self.dlg.is_open() || self.dlg.spec().is_some() || self.dlg_pending.is_some() {
            let mut inv = Invalidations::default();
            // 전송 중 문구(`ops.busy`)를 같이 쓰면 오해를 산다(협업 관찰 11-14) — 대화상자 전용 문구.
            self.statusbar.set_left(&tr("dlg.busy"), &mut inv);
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
        if self.dlg.spec().is_none() {
            // 창 없이: 사양을 창 상태기계에 싣는다(입력란 텍스트 · 잠금 · 무장이 창과 같은 규칙으로 돈다 · 10-18).
            let Some((spec, reply)) = self.dlg_pending.take() else {
                return;
            };
            self.dlg.set_spec(spec);
            self.dlg_reply = Some(reply);
        }
        // 잠김·무장(첫 누름)은 `None` = 대화상자가 그대로 남는다.
        if let Some(DlgAction::Done { id, text }) = self.dlg.pick(id) {
            self.dlg.close();
            self.dlg_done(id, text);
        }
    }

    /// 입력란 텍스트(기동 명령 `dlg.type:<text>`).
    pub(crate) fn dlg_type(&mut self, text: &str) {
        if self.dlg.is_open() || self.dlg.spec().is_some() {
            self.dlg.set_input_text(text);
        } else if let Some((spec, _)) = &mut self.dlg_pending {
            // 두 칸 사양(계산 컬럼)은 구분 글에서 한 번 갈라 나눠 넣는다 — 창의 `set_input_text`와 같은 규칙.
            let join = spec
                .more
                .as_ref()
                .map(|m| m.join.trim().to_owned())
                .filter(|j| !j.is_empty());
            let (first, second) = match &join {
                Some(j) => text.split_once(j.as_str()).unwrap_or((text, "")),
                None => (text, ""),
            };
            if let Some(i) = &mut spec.input {
                i.initial = if join.is_some() {
                    first.trim().to_string()
                } else {
                    text.to_string()
                };
            }
            if let Some(m) = &mut spec.more {
                m.input.initial = second.trim().to_string();
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
                    self.delete_permanent_start(paths);
                }
            }
            DlgReply::DeleteFailedPermanent(failed) => {
                if id == 1 {
                    self.delete_permanent_start(failed);
                }
            }
            DlgReply::DeleteLocked {
                paths,
                locked,
                origin,
            } => match id {
                1 => {
                    let rest: Vec<PathBuf> =
                        paths.into_iter().filter(|p| !locked.contains(p)).collect();
                    self.trash_now_from(rest, origin);
                }
                // 다시 시도 = 처음부터 다시 검사(그 사이 풀렸으면 바로 삭제 · 아니면 다시 묻는다).
                2 => {
                    self.trash_checked_from(paths, origin);
                }
                // 취소 = 그대로 — 미러면 "취소" 문구 + 재비교(GAP-029 출구 B·C).
                _ => self.trash_canceled(origin, paths.len()),
            },
            DlgReply::QueueRestore(jobs) => match id {
                1 => self.queue_resume(jobs),
                2 => self.queue_discard(jobs),
                _ => {}
            },
            DlgReply::DeleteFailed { failed, origin } => {
                if id == 1 {
                    self.trash_checked_from(failed, origin);
                }
            }
            DlgReply::ArchivePassword(path) => self.archive_password_result(path, id, text),
            DlgReply::About => {
                if id == 2 {
                    self.open_license = true;
                }
            }
            DlgReply::BulkPreset => self.bulk_preset_saved(id, text),
            DlgReply::NfcConvert(paths) => match id {
                1 => self.nfc_convert(paths, true),
                2 => self.nfc_convert(paths, false),
                _ => {}
            },
            DlgReply::HashSave { dest, text } => {
                if id == 1 {
                    self.checksum_write(&dest, &text);
                }
            }
            DlgReply::SelectPattern => {
                if matches!(id, 1 | 2) {
                    let spec = text.unwrap_or_default();
                    self.sel_pattern_last = spec.clone();
                    let mut inv = Invalidations::default();
                    let a = self.active;
                    let n = self.panels[a].select_pattern(&spec, id == 1, &mut inv);
                    let key = if id == 1 {
                        "sel.patternSelected"
                    } else {
                        "sel.patternDeselected"
                    };
                    // 상태줄 갱신(선택 수 · 탭 상태바) **뒤에** 안내 문구(갱신이 왼쪽 글을 덮어쓴다).
                    self.update_status();
                    self.statusbar
                        .set_left(&trf(key, &[&n.to_string()]), &mut inv);
                }
            }
            DlgReply::LauncherItem(at) => {
                if id == 1 {
                    self.launcher_item_entered(at, text.as_deref().unwrap_or(""));
                }
            }
            DlgReply::CalcColumn(at) => {
                if id == 1 {
                    self.calc_entered(at, text.as_deref().unwrap_or(""));
                }
            }
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

    /// `edit.select_pattern`(NEW-054): 패턴 입력 대화상자 — [선택] = 맞는 행을 선택에 더함 · [선택 해제] = 뺌 · 마지막 패턴을 기억.
    pub(crate) fn select_pattern_ask(&mut self) {
        let spec = DlgSpec {
            title: tr("sel.patternTitle"),
            text: tr("sel.patternText"),
            buttons: vec![
                (1, tr("sel.select")),
                (2, tr("sel.deselect")),
                (0, tr("dlg.cancel")),
            ],
            default: 1,
            cancel: 0,
            input: Some(DlgInput {
                label: tr("sel.pattern"),
                masked: false,
                initial: self.sel_pattern_last.clone(),
                placeholder: "*.txt; *.md".into(),
            }),
            delay: None,
            more: None,
            arm: None,
        };
        self.ask(spec, DlgReply::SelectPattern);
    }

    /// `edit.delete_permanent` — 확인 뒤 `ndir_ops::delete_permanent`(휴지통 경유 없음 · undo 없음).
    pub(crate) fn delete_permanent_ask(&mut self) {
        // 부모와 함께 선택된 자식 · 중복은 뺀다(GAP-053) — 개수 안내도 실제로 지울 최상위 항목 수가 된다.
        let paths = ndir_ops::prune_nested(self.panels[self.active].selected_paths());
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

    /// 완전 삭제 시작(확인 뒤 · GAP-040 · 사용자 11-03): **워커 스레드**에서 `ndir_ops::delete_permanent`를 항목마다 — 큰 트리 · 잠긴 파일
    /// (Windows 공유 위반은 바로 실패하지만 느린 볼륨 · 수천 항목은 수 초)이 UI를 멈추지 않는다. 종전(dir2 win.rs:3754 계승) = UI 스레드 동기
    /// 루프 → "응답 없음". 결과는 유휴 틱 `delete_tick`이 수거(docs/01 §3 · 자체 타이머 없음) · 동시 1잡(dir2 SHELL-070 `pending_delete`).
    pub(crate) fn delete_permanent_start(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        if self.delete_job.is_some() {
            let a = self.active;
            self.toast_panel(
                a,
                toast::ToastKind::Warn,
                tr("del.title"),
                tr("del.progress"),
            );
            return;
        }
        let total = paths.len();
        let targets = paths.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for p in paths {
                // 끝까지 지우고 지우지 못한 안쪽 경로만(GAP-046 · 사용자 11-04 ⓐ) — 종전 `delete_permanent`는 첫 실패에서 멈췄다.
                let bad: Vec<PathBuf> = ndir_ops::delete_permanent_tree(&p)
                    .into_iter()
                    .map(|(bad, _)| bad)
                    .collect();
                if tx.send((p, bad)).is_err() {
                    break; // 앱이 먼저 끝났다 — 남은 항목은 지우지 않는다
                }
            }
        });
        self.delete_job = Some(DeleteJob {
            rx,
            total,
            done: 0,
            failed: Vec::new(),
            failed_items: 0,
            paths: targets,
        });
        self.status_note(&trf("del.progressCount", &["0", &total.to_string()]));
        self.redraw();
    }

    pub(crate) fn delete_busy(&self) -> bool {
        self.delete_job.is_some()
    }

    /// 완전 삭제 워커 수거(틱) — 진행이 바뀌었거나 끝났으면 `true`(다시 그림). 채널이 닫히면 끝.
    pub(crate) fn delete_tick(&mut self) -> bool {
        let (changed, finished, done, total) = {
            let Some(job) = &mut self.delete_job else {
                return false;
            };
            let mut changed = false;
            let mut finished = false;
            loop {
                match job.rx.try_recv() {
                    Ok((_, bad)) => {
                        job.done += 1;
                        if !bad.is_empty() {
                            job.failed_items += 1;
                            job.failed.extend(bad);
                        }
                        changed = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
            (changed, finished, job.done, job.total)
        };
        if finished {
            let job = self.delete_job.take().expect("checked above");
            self.delete_finish(job);
            return true;
        }
        if changed {
            self.status_note(&trf(
                "del.progressCount",
                &[&done.to_string(), &total.to_string()],
            ));
        }
        changed
    }

    /// 완전 삭제 결과 반영: 토스트(지운 수 · 실패 수) · 재열람 · 실패분은 [다시 시도]/[닫기] 안내(휴지통 삭제 실패 안내 dir2 X-35와 같은 모양).
    /// 실패 목록은 지우지 못한 **안쪽 항목의 전체 경로**(GAP-046 · 나머지는 이미 지워졌다고 함께 알림) · [다시 시도] = 그 목록만.
    fn delete_finish(&mut self, job: DeleteJob) {
        let ok = job.done - job.failed_items;
        let failed = job.failed.len();
        let mut text = trf("del.done", &[&tr("del.kindPermanent"), &ok.to_string()]);
        if failed > 0 {
            text.push_str(" · ");
            text.push_str(&trf("del.partialFail", &[&failed.to_string()]));
        }
        let a = self.active;
        self.toast_panel(
            a,
            if failed > 0 {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("del.title"),
            text,
        );
        let mut inv = Invalidations::default();
        self.dirsize_touched(&job.paths); // 폴더 크기 캐시(T-166 · 11-09): 지운 항목 + 상위 폴더 — 종전엔 무효화 없음.
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        self.status_hold = None; // 진행 "n/m"은 끝났다 — 항목 수로
        self.update_status();
        if failed > 0 {
            // 목록 밖은 전부 시도해 지웠다(끝까지 도는 삭제) — 나머지가 없었어도 참이라 늘 붙인다.
            let mut text = app::menus::fail_message_paths(&job.failed);
            text.push_str("\n\n");
            text.push_str(&tr("del.failRest"));
            let spec = DlgSpec {
                title: tr("del.failTitle"),
                text,
                buttons: vec![(1, tr("del.retry")), (0, tr("del.close"))],
                default: 1,
                cancel: 0,
                input: None,
                delay: None,
                more: None,
                arm: None,
            };
            self.ask(spec, DlgReply::DeleteFailedPermanent(job.failed));
        }
    }

    /// 질문 글 — 재개 중 끊긴 덮어쓰기 사본(docs/26 §12.7 · 저널 started 줄 · `TransferJob.interrupted`)이면 "이전 전송에서 중단된 사본 ·
    /// 덮어쓰기 권장" + 지금 대상의 크기 · 시각 줄을 덧붙인다(사람이 보고 가른다 · 기본 버튼 = 1 덮어쓰기는 dir2와 같다 · 협업 확인 ⑶).
    pub(crate) fn conflict_question(&self, path: &std::path::Path) -> String {
        // 폴더면 설정(DR-37)의 뜻을 질문에 — 파일은 종전 글.
        let mut q = if path.is_dir() {
            dir_question(path, Self::dir_conflict_mode(&self.settings))
        } else {
            overwrite_question(path)
        };
        if self
            .transfer
            .as_ref()
            .is_some_and(|j| j.interrupted.contains(path))
        {
            let (size, when) = std::fs::metadata(path)
                .map(|m| {
                    let when = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| crate::filelist::format_time_local(d.as_millis() as i64))
                        .unwrap_or_default();
                    (crate::filelist::format_size(m.len()), when)
                })
                .unwrap_or_default();
            q.push('\n');
            q.push_str(&trf("ops.overwriteResumed", &[&size, &when]));
        }
        q
    }

    /// 전송 충돌 요청 → 4버튼(dir2 QA 07-14 개정): 덮어쓰기(1) · 모두 덮어쓰기(2) · 건너뛰기(3) · 취소(4).
    pub(crate) fn conflict_ask(
        &mut self,
        path: &std::path::Path,
        tx: mpsc::Sender<ConflictChoice>,
    ) {
        // 진행 창이 떠 있으면 **그 창 안에서** 묻는다(사용자 10-04 — 따로 뜬 확인 창이 진행 창을 덮어 목록 · 진행 상태가
        // 안 보였다 · Wayland는 창 자리를 지정할 수 없어 겹침을 피할 길이 없다). 창이 없으면(진행 창 끔 · 시험) 대화상자.
        // ★ "떠 있으면"이 아니라 **이 전송이 진행 창을 쓰면**으로 판정한다 — 첫 질문은 진행 창이 만들어지기 전에(같은 틱에)
        // 도착해서, 창이 떠 있는지로 보면 첫 질문만 따로 뜬 확인 창으로 갔다(사용자 10-04 캡처). 진행 창은 질문을 먼저
        // 받아 두었다가 그 크기로 열린다. 메인 창이 없는 시험은 종전 대화상자 길 그대로.
        let question = self.conflict_question(path);
        if self.progress_win.is_active() && self.window.is_some() {
            self.progress_win.set_conflict(Some(question));
            self.conflict_inline = Some(tx);
            return;
        }
        let spec = DlgSpec {
            title: tr("ops.overwriteTitle"),
            text: question,
            buttons: vec![
                (1, tr("ops.yes")),
                (2, tr("ops.yesAll")),
                (3, tr("ops.skip")),
                (4, tr("ops.cancel")),
            ],
            default: 1,
            cancel: 4,
            input: None,
            delay: None,
            more: None,
            arm: None,
        };
        if !self.ask(spec, DlgReply::Conflict(tx.clone())) {
            let _ = tx.send(ConflictChoice::Skip);
        }
    }
}
