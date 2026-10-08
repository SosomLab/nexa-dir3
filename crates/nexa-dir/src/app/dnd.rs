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

/// 창 안 드래그 상태(T-147 Linux/macOS 1차 — OS 드래그 없이 포인터만으로): 끌고 있는 경로 · 원래 패널 · 마지막 효과(= 커서).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalDrag {
    pub panel: usize,
    pub paths: Vec<PathBuf>,
    pub choice: platform::DropChoice,
}

/// 창 안 드래그의 커서(순수): 효과 → winit 커서 모양.
pub(crate) fn drag_cursor(choice: platform::DropChoice) -> winit::window::CursorIcon {
    match choice {
        platform::DropChoice::Copy => winit::window::CursorIcon::Copy,
        platform::DropChoice::Move => winit::window::CursorIcon::Move,
        platform::DropChoice::None => winit::window::CursorIcon::NoDrop,
    }
}

/// 지금 놓일 자리의 표시: 강조 사각형(폴더 행 또는 목록 전체) · 효과 · 대상 폴더.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DropMark {
    pub rect: Rect,
    pub choice: platform::DropChoice,
    pub dest: PathBuf,
}

/// 끌어오다 머물면 여는 대상(dir2 X-32 `DndHover`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Dwell {
    /// 활성이 아닌 탭(자리) — 머물면 그 탭으로 바꾼다.
    Tab(usize),
    /// 접힌 폴더 행 — 머물면 펼친다.
    Folder(PathBuf),
}

/// 머물기 판정(순수 · dir2 win.rs:3395 `dnd_hover_fire`): 대상이 바뀌면 시계를 다시 재고, 같은 대상 위에 `wait` 이상 있었으면
/// 발화한다(발화 뒤에는 시계를 다시 재므로 한 번만). 돌려주는 것 = (새 상태, 지금 열 것인가).
pub(crate) fn dwell_step<T: Clone + PartialEq>(
    prev: Option<(T, Instant)>,
    cur: Option<T>,
    now: Instant,
    wait: Duration,
) -> (Option<(T, Instant)>, bool) {
    match (prev, cur) {
        (_, None) => (None, false),
        (Some((p, since)), Some(c)) if p == c => {
            if now.saturating_duration_since(since) >= wait {
                (Some((c, now)), true)
            } else {
                (Some((c, since)), false)
            }
        }
        (_, Some(c)) => (Some((c, now)), false),
    }
}

/// 행 하나가 가리키는 **놓일 폴더**(순수): 폴더 행 = 그 폴더 · 파일 행 = 그 파일이 든 폴더 — 단 탭의 현재 폴더(`root`) 밖을
/// 가리키게 되면(부모를 알 수 없거나 `root` 바깥) `root`로 둔다.
pub(crate) fn drop_folder_of(
    path: &std::path::Path,
    is_dir: bool,
    root: &std::path::Path,
) -> PathBuf {
    if is_dir {
        return path.to_path_buf();
    }
    match path.parent() {
        Some(parent) if parent.starts_with(root) => parent.to_path_buf(),
        _ => root.to_path_buf(),
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
                // OS 드래그가 없는 OS(Linux · macOS) = 창 안 드래그로(포인터 사건이 드롭 수신부로 · T-147 1차).
                if !self.platform.drag.supports_os_drag() {
                    self.dnd_internal_begin(i, paths, (x, y));
                    return;
                }
                // 드래그가 도는 동안(OS 모달) 앱은 여기 멈춰 있다 → 우리 창 위에서의 추적 · 그리기 · 효과 판정은 드롭 수신부가
                // **실시간 수신기**로 바로 불러 준다(사용자 10-05 실기: 머물러도 안 열리고 커서도 안 바뀌던 원인).
                // SAFETY(아래 `&mut *app`): 수신기는 `begin_drag`가 도는 동안에만, 같은 UI 스레드에서, OLE 콜백으로 불린다.
                // 그동안 이 함수는 `self`를 쓰지 않고(`drag` · `paths`는 지역 복제) 돌아온 뒤에야 다시 쓴다 → 동시에 살아 있는
                // 두 접근이 없다.
                let app: *mut App = self;
                let sink: platform::DropSink = Box::new(move |ev| {
                    let app = unsafe { &mut *app };
                    let choice = app.dnd_event(ev.clone(), Instant::now());
                    app.paint(); // 모달 중에는 RedrawRequested가 오지 않는다 — 직접 그린다
                    choice
                });
                let drag = Rc::clone(&self.platform.drag);
                let result = platform::with_live_drop_sink(sink, || drag.begin_drag(&paths));
                self.dnd_end();
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

    /// 창 안 드래그 시작(T-147 Linux/macOS 1차): 누름 상태를 정리하고(뗌은 드롭이 된다) 드롭 수신부에 "들어옴"을 넣는다.
    pub(crate) fn dnd_internal_begin(&mut self, panel: usize, paths: Vec<PathBuf>, at: (i32, i32)) {
        self.panels[panel].abort_press();
        self.pressed = None;
        self.last_click = None;
        let choice = self.dnd_event(
            platform::DropEvent::Enter {
                paths: paths.clone(),
                at,
                ctrl: self.primary,
                shift: self.shift,
            },
            Instant::now(),
        );
        self.dnd_internal = Some(InternalDrag {
            panel,
            paths,
            choice,
        });
        self.set_drag_cursor(Some(choice));
    }

    /// 창 안 드래그 중의 입력(T-147): 이동 = 놓일 자리 · 커서 갱신 · 뗌 = 놓기 · Esc = 취소. 돌려주는 값 = 사건을 삼켰는가
    /// (드래그 중이면 다른 컨트롤로 흘리지 않는다).
    pub(crate) fn dnd_internal_event(&mut self, ev: &InputEvent) -> bool {
        let Some(drag) = self.dnd_internal.clone() else {
            return false;
        };
        match *ev {
            InputEvent::MouseMove { x, y } => {
                let choice = self.dnd_event(
                    platform::DropEvent::Over {
                        at: (x, y),
                        ctrl: self.primary,
                        shift: self.shift,
                    },
                    Instant::now(),
                );
                if choice != drag.choice {
                    if let Some(d) = &mut self.dnd_internal {
                        d.choice = choice;
                    }
                    self.set_drag_cursor(Some(choice));
                }
            }
            InputEvent::MouseUp { x, y } => {
                self.dnd_internal = None;
                let _ = self.dnd_event(
                    platform::DropEvent::Drop {
                        paths: drag.paths,
                        at: (x, y),
                        ctrl: self.primary,
                        shift: self.shift,
                    },
                    Instant::now(),
                );
                self.set_drag_cursor(None);
            }
            InputEvent::Key {
                key: nexa_ctl::Key::Escape,
                ..
            } => self.dnd_internal_cancel(),
            _ => {}
        }
        true
    }

    /// 창 안 드래그 취소(Esc · 포커스 잃음) — 표시를 걷고 커서를 되돌린다.
    pub(crate) fn dnd_internal_cancel(&mut self) {
        if self.dnd_internal.take().is_some() {
            let _ = self.dnd_event(platform::DropEvent::Leave, Instant::now());
            self.set_drag_cursor(None);
        }
    }

    /// 창 안 드래그 틱(머물면 열기 · 가장자리 자동 스크롤은 사건이 없어도 돌아야 한다) — 화면이 바뀌었으면 `true`.
    pub(crate) fn dnd_internal_tick(&mut self, now: Instant) -> bool {
        if self.dnd_internal.is_none() {
            return false;
        }
        let _ = self.dnd_event(
            platform::DropEvent::Over {
                at: self.cursor,
                ctrl: self.primary,
                shift: self.shift,
            },
            now,
        );
        true
    }

    fn set_drag_cursor(&self, choice: Option<platform::DropChoice>) {
        if let Some(w) = &self.window {
            w.set_cursor(choice.map_or(winit::window::CursorIcon::Default, drag_cursor));
        }
    }

    /// 드롭 수신부의 사건 하나를 처리하고 **효과**(= 커서 모양)를 돌려준다 — 자체 수신부(Windows)의 큐 · 실시간 수신기 ·
    /// 포인터 폴링이 모두 이 한 길로 온다.
    /// - 들어옴/위에 있음: 포인터 반영(자동 스크롤 · 머물면 열기) → 놓일 자리 판정 → 강조 사각형 + 상태줄 안내("이동 → 폴더").
    /// - 벗어남: 표시를 걷는다.
    /// - 놓음: 그 자리 · 그 수식키로 전송을 시작한다.
    pub(crate) fn dnd_event(
        &mut self,
        ev: platform::DropEvent,
        now: Instant,
    ) -> platform::DropChoice {
        use platform::{DropChoice, DropEvent};
        match ev {
            DropEvent::Enter {
                paths,
                at,
                ctrl,
                shift,
            } => {
                self.dnd_hover = paths;
                self.dnd_over(at, ctrl, shift, now)
            }
            DropEvent::Over { at, ctrl, shift } => self.dnd_over(at, ctrl, shift, now),
            DropEvent::Leave => {
                self.dnd_end();
                DropChoice::None
            }
            DropEvent::Drop {
                paths,
                at,
                ctrl,
                shift,
            } => {
                let mods = (self.shift, self.primary);
                self.cursor = at;
                (self.shift, self.primary) = (shift, ctrl);
                let (plan, choice) = self.drop_plan(&paths, Point { x: at.0, y: at.1 });
                self.dnd_end();
                if plan.is_some() {
                    self.external_drop(paths, Point { x: at.0, y: at.1 });
                }
                (self.shift, self.primary) = mods;
                choice
            }
        }
    }

    fn dnd_over(
        &mut self,
        at: (i32, i32),
        ctrl: bool,
        shift: bool,
        now: Instant,
    ) -> platform::DropChoice {
        let mods = (self.shift, self.primary);
        self.dnd_track(at, ctrl, shift, now);
        let sources = self.dnd_hover.clone();
        let (plan, choice) = self.drop_plan(&sources, Point { x: at.0, y: at.1 });
        (self.shift, self.primary) = mods;
        let mark = plan.map(|(panel, dest)| {
            // 강조 = **놓일 폴더**: 탭의 현재 폴더면 목록 전체 · 하위 폴더면 그 폴더 행 + 펼쳐진 내용(파일 행 위에 있어도 그 파일이
            // 든 폴더 묶음이 강조된다) · 접힌 폴더 행이면 그 행만.
            let p = &self.panels[panel];
            let rect = if dest == p.root_path() {
                p.rows().bounds()
            } else {
                p.folder_block_rect(&dest)
                    .unwrap_or_else(|| p.rows().bounds())
            };
            DropMark { rect, choice, dest }
        });
        if self.dnd_mark != mark {
            let text = match &mark {
                Some(m) => trf(
                    if m.choice == platform::DropChoice::Move {
                        "dnd.willMove"
                    } else {
                        "dnd.willCopy"
                    },
                    &[&m.dest.display().to_string()],
                ),
                None => tr("dnd.cannot"),
            };
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&text, &mut inv);
            self.dnd_mark = mark;
            self.redraw();
        }
        choice
    }

    /// 드래그 표시 걷기(벗어남 · 놓음 · 취소) — 머묾 · 강조 · 상태줄 안내.
    pub(crate) fn dnd_end(&mut self) {
        let had = self.dnd_mark.take().is_some() || !self.dnd_hover.is_empty();
        self.dnd_dwell = None;
        self.dnd_hover.clear();
        if had {
            self.update_status();
            self.redraw();
        }
    }

    /// 놓일 자리와 효과(순수 판정에 가깝다 — 상태를 바꾸지 않는다): 놓을 수 없으면 `(None, None)`.
    /// 금지 = 패널 밖 · 내 PC(가상 최상위) · 전송 중 · 자기 자신/하위 폴더 안 · **제자리**(모든 소스가 이미 그 폴더에 있다).
    pub(crate) fn drop_plan(
        &self,
        sources: &[PathBuf],
        at: Point,
    ) -> (Option<(usize, PathBuf)>, platform::DropChoice) {
        let none = (None, platform::DropChoice::None);
        if sources.is_empty() || self.transfer.is_some() {
            return none;
        }
        let Some((panel, dest)) = self.drop_dest_at(at) else {
            return none;
        };
        if sources.iter().all(|s| s.parent() == Some(dest.as_path()))
            || sources
                .iter()
                .any(|s| s == &dest || ndir_ops::is_same_or_sub(s, &dest))
        {
            return none;
        }
        let choice = platform::drop_choice(
            self.primary,
            self.shift,
            ndir_ops::same_volume(&sources[0], &dest),
        );
        (Some((panel, dest)), choice)
    }

    /// 수신부에 넘겨 줄 패널 요약(다른 프로그램의 드래그에 즉시 효과를 답하는 데 쓰인다) — 내 PC는 뺀다.
    pub(crate) fn drop_zones(&self) -> Vec<platform::DropZone> {
        (0..2)
            .filter(|&i| i == 0 || self.dual)
            .filter_map(|i| {
                let root = self.panels[i].root_path();
                if ndir_vfs::is_virtual_root(&root) {
                    return None;
                }
                let b = self.panels[i].bounds();
                Some(platform::DropZone {
                    rect: (b.x, b.y, b.w, b.h),
                    root,
                })
            })
            .collect()
    }

    /// 자체 수신부의 큐를 거둬 처리하고 패널 요약을 새로 넣는다(틱) — 처리한 사건이 있으면 `true`.
    pub(crate) fn drop_pump(&mut self, now: Instant) -> bool {
        let Some(shared) = self.drop_shared.clone() else {
            return false;
        };
        let events = std::mem::take(&mut shared.borrow_mut().events);
        let any = !events.is_empty();
        for ev in events {
            let _ = self.dnd_event(ev, now);
        }
        shared.borrow_mut().zones = self.drop_zones();
        any
    }

    /// 다른 프로그램에서 끌어오는 중인가(창 위에 파일이 떠 있다 — 추적 틱이 돌아야 하는 동안).
    pub(crate) fn dnd_hovering(&self) -> bool {
        !self.dnd_hover.is_empty()
    }

    /// 끌어오는 동안의 포인터 반영(T-147 수신 보강 · dir2 `DropHooks::track` · win.rs:3326-3349): 호스트가 OS에서 읽은
    /// 창 안 좌표 · 수식키를 넣는다 → 놓는 자리 · 복사/이동 판정이 **지금 포인터** 기준이 되고(종전 = 드래그가 들어오기 전
    /// 마지막 자리), 목록 가장자리 띠에서는 그쪽으로 한 노치씩 스크롤한다. 활성이 아닌 탭 · 폴더 행 위에 설정 시간
    /// (`transfer.dnd_hover_ms`)만큼 머물면 그 탭으로 바꾸거나 폴더를 펼친다(SHELL-067). 화면이 바뀌었으면 `true`.
    pub(crate) fn dnd_track(
        &mut self,
        at: (i32, i32),
        ctrl: bool,
        shift: bool,
        now: Instant,
    ) -> bool {
        self.cursor = at;
        self.primary = ctrl;
        self.shift = shift;
        let p = Point { x: at.0, y: at.1 };
        // 머물면 열기.
        let cur = (0..2).find_map(|i| {
            if (i == 1 && !self.dual) || !self.panels[i].bounds().contains(p) {
                return None;
            }
            self.panels[i].dnd_dwell_at(at.0, at.1).map(|d| (i, d))
        });
        let wait = Duration::from_millis(
            self.settings
                .int("transfer.dnd_hover_ms")
                .clamp(200, 10_000) as u64,
        );
        let (state, fire) = dwell_step(self.dnd_dwell.take(), cur, now, wait);
        self.dnd_dwell = state;
        if fire {
            if let Some(((i, target), _)) = self.dnd_dwell.clone() {
                let mut inv = Invalidations::default();
                if self.panels[i].dnd_dwell_open(&target, &mut inv) {
                    self.update_status();
                    return true;
                }
            }
        }
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
        self.dnd_end();
    }

    pub(crate) fn dnd_dropped(&mut self, path: PathBuf) {
        self.dnd_dwell = None;
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
        // 폴더 행 위 = 그 폴더 · **파일 행 위 = 그 파일이 든 폴더**(트리에서 펼친 하위 폴더 안의 파일이면 그 하위 폴더 —
        // 사용자 10-05: 종전에는 파일 위면 늘 탭의 최상위 폴더였다) · 빈 곳 = 탭의 현재 폴더.
        let dest = panel
            .rows()
            .row_at(p.x, p.y)
            .and_then(|r| panel.rows().source().row_path(r))
            .map(|path| drop_folder_of(&path, path.is_dir(), &root))
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
