//! App — 외부 끌어다 놓기 **1차**(docs/port/19 §4-7 "1차 범위" · SHELL-060/061/062/068 축약 · 3-OS 공통 = winit `HoveredFile`/`DroppedFile`):
//! 대상 = 놓은 시점 커서 아래 **폴더 행** / 그 패널의 현재 폴더(SHELL-061) · 연산 = Ctrl 복사 · Shift 이동 · 기본 = 같은 볼륨 이동 /
//! 다른 볼륨 복사(SHELL-062) · 소스가 대상과 같거나 상위면 거부 · 전송 중이면 거부(SHELL-068) · 전송 엔진(`start_transfer`) 합류.
//! winit은 파일마다 `DroppedFile`을 주고 끝 신호가 없다 → 모았다가 **틱에서 한 번에** 처리(`dnd_flush`). 발신·OLE 완전 이식·자동 스크롤은 2차.

use crate::*;
use ndir_ops::Op;

/// 누른 자리에서 이만큼 넘게 움직이면 끌기다(px · Windows 기본 드래그 임계 `SM_CXDRAG` = 4와 같은 뜻).
const DRAG_SLOP: i32 = 4;

/// 끌어오는 동안 목록 가장자리에서 자동 스크롤을 시작하는 띠의 두께(px · 배율 전 · 위쪽은 열 머리글(약 24)을 포함한다).
const EDGE_BAND: i32 = 40;
/// 자동 스크롤 · 포인터 추적 간격(ms · dir2 TIMER_DND 100).
pub(crate) const DND_TRACK_MS: u64 = 100;

/// 화면 좌표 → 창 안 좌표(순수): 창 내용 영역의 화면 자리를 뺀다.
pub(crate) fn client_point(screen: (i32, i32), inner: (i32, i32)) -> (i32, i32) {
    (screen.0 - inner.0, screen.1 - inner.1)
}

/// 가장자리 자동 스크롤 방향(순수 · dir2 win.rs:3349): 목록 위쪽 띠 = 위로(+1) · 아래쪽 띠 = 아래로(-1) · 그 밖 = 0.
/// 목록 밖(위 · 아래로 벗어남)은 스크롤하지 않는다.
pub(crate) fn edge_scroll(y: i32, top: i32, bottom: i32, band: i32) -> i32 {
    if y < top || y >= bottom || bottom - top < band * 3 {
        0
    } else if y < top + band {
        1
    } else if y >= bottom - band {
        -1
    } else {
        0
    }
}

/// 끌기가 시작됐는가(순수): 가로나 세로로 임계를 넘었다.
pub(crate) fn drag_started(press: (i32, i32), now: (i32, i32)) -> bool {
    (now.0 - press.0).abs() > DRAG_SLOP || (now.1 - press.1).abs() > DRAG_SLOP
}

impl App {
    /// **드래그 발신**(dir2 win.rs `drag_press` · dnd.rs `begin_drag` · SHELL-063~066 · T-147): 이미 선택된 행을 누른 채 임계 넘게
    /// 끌면 선택 전체(여러 폴더에 걸쳐도 · 화면 순서)를 OS 드래그로 내보낸다 — 탐색기 · 가상 머신 창 · 터미널 · 다른 창의 패널.
    /// 선택 안 된 행을 끄는 것은 러버밴드 선택이다(그리드 규약). 후보(`drag_press`)는 누르기 **전** 선택 상태로
    /// `slow_click_before`가 잡고, 여기(사건이 컨트롤을 거친 뒤 길목)는 이동 · 뗌만 본다.
    ///
    /// OS 드래그는 버튼을 뗄 때까지 돌아오지 않고 그 뗌도 OS가 삼킨다 → 돌아온 뒤 누름 상태(그리드의 클릭 확정 보류 ·
    /// 패널/앱의 포인터 캡처 · 느린 재클릭 예약)를 직접 정리하고 목록을 다시 읽는다. 선택은 그대로 둔다.
    pub(crate) fn drag_out_after(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        match *ev {
            InputEvent::MouseMove { x, y } => {
                let Some((i, px, py)) = self.drag_press else {
                    return;
                };
                if !drag_started((px, py), (x, y)) {
                    return;
                }
                self.drag_press = None;
                let paths = self.panels[i].selected_paths_in_view_order();
                if paths.is_empty() || self.transfer.is_some() {
                    return;
                }
                self.rename_on_up = None;
                self.rename_due = None;
                let result = self.platform.drag.begin_drag(&paths);
                // 뗌은 OS가 삼켰다 — 누름 상태 정리(선택은 유지).
                self.panels[i].abort_press();
                self.pressed = None;
                self.last_click = None;
                match result {
                    Ok(platform::DragOutcome::Cancelled)
                    | Err(platform::PlatformError::Unsupported(_)) => {}
                    Ok(_) => {
                        // 놓였다 — 대상이 옮겼을 수 있으니 다시 읽어 맞춘다(dir2: 결과와 무관하게 재로드로 수렴).
                        for p in &mut self.panels {
                            p.reopen(inv);
                        }
                        self.update_status();
                    }
                    Err(e) => {
                        self.toasts
                            .push(toast::ToastKind::Warn, tr("dnd.rejected"), e.to_string());
                    }
                }
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            InputEvent::MouseUp { .. } => self.drag_press = None,
            _ => {}
        }
    }

    /// 다른 프로그램에서 끌어오는 중인가(창 위에 파일이 떠 있다 — 추적 틱이 돌아야 하는 동안).
    pub(crate) fn dnd_hovering(&self) -> bool {
        !self.dnd_hover.is_empty()
    }

    /// 끌어오는 동안의 포인터 반영(T-147 수신 보강 · dir2 `DropHooks::track` · win.rs:3326-3349): 호스트가 OS에서 읽은
    /// 창 안 좌표 · 수식키를 넣는다 → 놓는 자리 · 복사/이동 판정이 **지금 포인터** 기준이 되고(종전 = 드래그가 들어오기 전
    /// 마지막 자리), 목록 가장자리 띠에서는 그쪽으로 한 노치씩 스크롤한다. 스크롤했으면 `true`.
    pub(crate) fn dnd_track(&mut self, at: (i32, i32), ctrl: bool, shift: bool) -> bool {
        self.cursor = at;
        self.primary = ctrl;
        self.shift = shift;
        let p = Point { x: at.0, y: at.1 };
        let band = (EDGE_BAND as f32 * self.scale).round() as i32;
        for i in 0..2 {
            if i == 1 && !self.dual {
                continue;
            }
            let b = self.panels[i].rows().bounds();
            if !b.contains(p) {
                continue;
            }
            let dir = edge_scroll(at.1, b.y, b.bottom(), band);
            if dir != 0 {
                let before = self.panels[i].rows().scroll_row();
                let mut inv = Invalidations::default();
                self.panels[i].on_event(&InputEvent::Wheel { delta: dir * 120 }, &mut inv);
                return self.panels[i].rows().scroll_row() != before;
            }
        }
        false
    }

    pub(crate) fn dnd_hover(&mut self, path: PathBuf) {
        if !self.dnd_hover.contains(&path) {
            self.dnd_hover.push(path);
        }
    }

    pub(crate) fn dnd_cancel(&mut self) {
        self.dnd_hover.clear();
    }

    pub(crate) fn dnd_dropped(&mut self, path: PathBuf) {
        self.dnd_hover.clear();
        if !self.dnd_drop.contains(&path) {
            self.dnd_drop.push(path);
        }
    }

    /// 틱 — 모인 드롭을 처리했으면 true.
    pub(crate) fn dnd_flush(&mut self) -> bool {
        if self.dnd_drop.is_empty() {
            return false;
        }
        let sources = std::mem::take(&mut self.dnd_drop);
        let (x, y) = self.cursor;
        self.external_drop(sources, Point { x, y });
        true
    }

    /// 놓는 자리 → (패널, 대상 폴더): 폴더 행 위 = 그 폴더 · 파일 행/빈 본문 = 그 패널의 현재 폴더 · 패널 밖/가상 최상위 = None(SHELL-061).
    pub(crate) fn drop_dest_at(&self, p: Point) -> Option<(usize, PathBuf)> {
        let i = if self.panels[0].bounds().contains(p) {
            0
        } else if self.dual && self.panels[1].bounds().contains(p) {
            1
        } else {
            return None;
        };
        let panel = &self.panels[i];
        let root = panel.root_path();
        if ndir_vfs::is_virtual_root(&root) {
            return None;
        }
        let dest = panel
            .rows()
            .row_at(p.x, p.y)
            .and_then(|r| panel.rows().source().row_path(r))
            .filter(|path| path.is_dir())
            .unwrap_or(root);
        Some((i, dest))
    }

    /// 외부 드롭 실행 — 시작했으면 (대상, 연산). 거부 사유는 상태줄.
    pub(crate) fn external_drop(
        &mut self,
        sources: Vec<PathBuf>,
        at: Point,
    ) -> Option<(PathBuf, Op)> {
        if sources.is_empty() {
            return None;
        }
        if self.transfer.is_some() {
            self.note_status(&tr("ops.busy"));
            return None;
        }
        let (panel, dest) = match self.drop_dest_at(at) {
            Some(d) => d,
            None => {
                let root = self.panels[self.active].root_path();
                if ndir_vfs::is_virtual_root(&root) {
                    return None;
                }
                (self.active, root)
            }
        };
        // 제자리 놓기(모든 소스가 이미 그 폴더에 있다 — 자기 창 안에서 끌다 같은 폴더에 놓은 경우) = 아무 일도 하지 않는다.
        if sources.iter().all(|s| s.parent() == Some(dest.as_path())) {
            return None;
        }
        // 자기 자신/상위 폴더 안으로는 🚫(SHELL-062).
        if sources
            .iter()
            .any(|s| s == &dest || ndir_ops::is_same_or_sub(s, &dest))
        {
            self.note_status(&tr("dnd.rejected"));
            return None;
        }
        let op = if self.primary {
            Op::Copy
        } else if self.shift || ndir_ops::same_volume(&sources[0], &dest) {
            Op::Move
        } else {
            Op::Copy
        };
        if panel != self.active {
            self.set_active(panel);
        }
        let n = sources.len().to_string();
        self.note_status(&trf("dnd.dropped", &[&n, &ndir_ops::leaf_name(&dest)]));
        self.start_transfer(sources, dest.clone(), op, false);
        Some((dest, op))
    }

    fn note_status(&mut self, text: &str) {
        let mut inv = Invalidations::default();
        self.statusbar.set_left(text, &mut inv);
        self.redraw();
    }
}
