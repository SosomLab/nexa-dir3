//! 느린 재클릭 = 이름 바꾸기(탐색기 관례 · dir2 win.rs:8132-8149 예약 · 8572-8581 지연 · 9583-9601 발화 · G3-05 대조).
//!
//! ① 이미 선택된 행을 **1초 이상 지나 다시 누르면** 예약(짧은 간격 = 더블클릭 시도라 무시 · 수식키 · 편집을 끝낸 클릭은 제외)
//! ② 끌지 않고 **뗐을 때** 더블클릭 시간만큼 기다린다(그 안에 두 번째 클릭 = 열기 → 취소)
//! ③ 만료 때 예약한 (패널, 경로)가 **지금의 활성 패널 · 캐럿 행**과 같을 때만 이름 바꾸기에 들어간다 — 그 사이 방향키 · 삭제 ·
//!    우클릭 메뉴 · 다시 읽기로 다른 행이 캐럿이 됐으면 버린다.
//!
//! 새 클릭 · 우클릭 · 가운데 클릭 · 더블클릭 · 키 입력은 모두 예약을 버린다("이름 바꾸기 의도 아님").

use crate::*;
use std::path::Path;

/// 재클릭이 "느린" 것으로 치는 최소 간격(ms · dir2 `SLOW_CLICK_RENAME_MS`).
const SLOW_CLICK_RENAME_MS: u64 = 1_000;
/// 누른 자리에서 이만큼 넘게 움직여 떼면 클릭이 아니라 끌기다(px · 더블클릭 합성의 허용 오차와 같다).
const CLICK_SLOP: i32 = 4;

/// 직전 행 클릭(패널, 경로, 시각).
pub(crate) type SlowClick = (usize, PathBuf, Instant);

/// 이번 누름이 이름 바꾸기를 예약하는가(순수 · dir2 win.rs:8140-8146): 같은 패널 · 같은 경로를 1초 이상 지나 다시 눌렀고,
/// 그 행이 누르기 전에 이미 선택돼 있었으며, 수식키(Shift · Ctrl)가 없고, 이 클릭이 진행 중이던 편집을 끝낸 것이 아닐 때.
pub(crate) fn slow_click_arms(
    prev: Option<&SlowClick>,
    panel: usize,
    path: &Path,
    now: Instant,
    was_selected: bool,
    modifiers: bool,
    was_renaming: bool,
) -> bool {
    let Some((p, prev_path, at)) = prev else {
        return false;
    };
    was_selected
        && !modifiers
        && !was_renaming
        && *p == panel
        && prev_path == path
        && now.saturating_duration_since(*at) >= Duration::from_millis(SLOW_CLICK_RENAME_MS)
}

/// 만료 때 이름 바꾸기에 들어갈 것인가(순수 · dir2 `rename_timer_should_fire`): 예약한 (패널, 경로) = 지금의 활성 패널 · 캐럿 행.
/// 경로 비교는 끝 구분자 · ASCII 대소문자를 무시한다(OneDrive가 대소문자를 바꿔 돌려주는 경우).
pub(crate) fn rename_should_fire(
    pending: Option<&(usize, PathBuf)>,
    active: usize,
    caret: Option<&Path>,
) -> bool {
    let (Some((panel, path)), Some(cur)) = (pending, caret) else {
        return false;
    };
    let norm = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    };
    *panel == active && norm(path) == norm(cur)
}

impl App {
    /// 사건이 컨트롤로 가기 **전** 길목(`route`): 누름 = 예약 판정(선택 상태는 이 클릭이 바꾸기 전 값) · 다른 입력 = 예약 폐기.
    pub(crate) fn slow_click_before(&mut self, ev: &InputEvent) {
        match *ev {
            InputEvent::MouseDown {
                x,
                y,
                shift,
                primary,
            } => {
                self.rename_due = None;
                self.rename_on_up = None;
                self.drag_press = None;
                // 모달(대화상자 · 열린 메뉴) 위 클릭은 목록 클릭이 아니다.
                let hit = if self.dlg.is_open() || self.tab_menu.is_open() {
                    None
                } else {
                    (0..2).find_map(|i| {
                        let p = &self.panels[i];
                        if (i == 1 && !self.dual) || !p.bounds().contains(Point { x, y }) {
                            return None;
                        }
                        let rows = p.rows();
                        let row = rows.row_at(x, y)?;
                        let path = rows.source().row_path(row)?;
                        let selected = p.selected_paths().contains(&path);
                        Some((i, path, selected, rows.is_renaming()))
                    })
                };
                let Some((i, path, selected, renaming)) = hit else {
                    self.slow_click = None;
                    return;
                };
                // 이미 선택된 행을 수식키 없이 눌렀다 = 드래그 발신 후보(`app/dnd.rs::drag_out_after` · 내 PC의 드라이브는 제외).
                if selected
                    && !renaming
                    && !shift
                    && !primary
                    && !self.panels[i].rows().source().is_virtual_root()
                {
                    self.drag_press = Some((i, x, y));
                }
                let now = Instant::now();
                if slow_click_arms(
                    self.slow_click.as_ref(),
                    i,
                    &path,
                    now,
                    selected,
                    shift || primary,
                    renaming,
                ) {
                    self.rename_on_up = Some((x, y));
                }
                self.slow_click = Some((i, path, now));
            }
            InputEvent::MouseUp { x, y } => {
                // 끌지 않고 뗐다 = 클릭 확정 → 더블클릭 시간만큼 기다렸다가 들어간다(그 안의 두 번째 클릭 = 열기가 취소한다).
                if let Some((dx, dy)) = self.rename_on_up.take() {
                    if (x - dx).abs() <= CLICK_SLOP && (y - dy).abs() <= CLICK_SLOP {
                        let wait = self.settings.int("ui.dblclick_ms").clamp(100, 2000) as u64;
                        self.rename_due = self.slow_click.as_ref().map(|(p, path, _)| {
                            (
                                *p,
                                path.clone(),
                                Instant::now() + Duration::from_millis(wait),
                            )
                        });
                    }
                }
            }
            InputEvent::DoubleClick { .. }
            | InputEvent::RightDown { .. }
            | InputEvent::MiddleDown { .. }
            | InputEvent::Key { .. } => {
                self.rename_due = None;
                self.rename_on_up = None;
            }
            _ => {}
        }
    }

    /// 지연이 끝났으면 대조 뒤 이름 바꾸기 — 다음 깨울 시각(예약이 없으면 `None`).
    pub(crate) fn slow_click_tick(&mut self, now: Instant) -> Option<Instant> {
        let at = self.rename_due.as_ref()?.2;
        if now < at {
            return Some(at);
        }
        let (panel, path, _) = self.rename_due.take()?;
        let caret = {
            let rows = self.panels[self.active].rows();
            rows.caret().and_then(|c| rows.source().row_path(c))
        };
        let modal = self.dlg.is_open() || self.tab_menu.is_open();
        if !modal && rename_should_fire(Some(&(panel, path)), self.active, caret.as_deref()) {
            self.command("edit.rename");
            self.redraw();
        }
        None
    }
}
