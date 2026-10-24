//! App — 그리기(프레임 합성 · nexa-sql `app/paint.rs` 축약). 층 순서 = 본문(패널 · 스플리터) → 크롬(툴바·메뉴바 배경) → 상태줄 →
//! 토스트 → 툴팁·경로 제안 → **메뉴바 드롭다운**(최상위 · nexa-sql 09-15 교훈: 풀다운이 뒤로 가림).
//!
//! `paint`(Shell · 표면 빌림 + 래스터 컨텍스트) / [`App::paint_into`](AppCore · 어떤 `DrawCtx`든 — 시험은 `RecordCtx` · CI-105).

use crate::*;

/// 다음 프레임의 더러워진 영역(10-18 결정 ① 부분 다시 그리기): `full` = 창 전체 · 아니면 `rects`(입력 · 틱이 모은 무효화 사각형 ·
/// 겹치면 합친다). `paint`가 꺼내 가며 비운다.
#[derive(Debug, Default)]
pub(crate) struct Damage {
    pub full: bool,
    pub rects: Vec<Rect>,
}

impl Damage {
    pub(crate) fn push(&mut self, r: Rect) {
        if r.is_empty() {
            return;
        }
        if let Some(hit) = self.rects.iter_mut().find(|x| x.intersects(&r)) {
            *hit = hit.union(&r);
            return;
        }
        self.rects.push(r);
    }
}

/// 한 프레임에 부분 그리기로 받는 사각형 상한 — 넘으면 합집합 하나(클립 패스 수 제한).
const MAX_DAMAGE_RECTS: usize = 8;

/// 이번 프레임을 어떻게 그릴지.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FramePlan {
    /// 창 전체.
    Full,
    /// 이 사각형들만(클립 그리기 + `present_with_damage`) · 빈 목록 = 그릴 것 없음(내지 않는다).
    Partial(Vec<Rect>),
}

/// 프레임 계획(순수 · MC/DC): 부분은 설정 on · 전체 요청 없음 · 버퍼가 지난 프레임 그대로(`age` 1) · 오버레이 없음일 때만 —
/// 사각형을 창 안으로 자르고(빈 것 제거) [`MAX_DAMAGE_RECTS`]를 넘으면 합집합 하나로.
pub(crate) fn frame_plan(
    enabled: bool,
    full: bool,
    age: u8,
    overlay: bool,
    rects: &[Rect],
    win: Rect,
) -> FramePlan {
    if !enabled || full || age != 1 || overlay {
        return FramePlan::Full;
    }
    let mut out: Vec<Rect> = rects
        .iter()
        .map(|r| r.intersection(&win))
        .filter(|r| !r.is_empty())
        .collect();
    if out.len() > MAX_DAMAGE_RECTS {
        let u = out.iter().skip(1).fold(out[0], |a, r| a.union(r));
        out = vec![u];
    }
    FramePlan::Partial(out)
}

/// 오버레이 프레임 판정(순수): 지금 오버레이가 있거나 **지난 프레임에 있었으면** 전체를 그린다 — 자기 사각형 밖에 그리는 층
/// (툴팁 · 드롭다운 · 토스트 …)이 사라진 자리를 damage가 모르기 때문(사용자 10-24 툴팁 잔상).
pub(crate) fn overlay_frame(now: bool, prev: bool) -> bool {
    now || prev
}

/// 메뉴 damage(T-189 ⑥ · 순수): 메뉴는 오버레이로 치지 않고 **이번 프레임 메뉴 사각형 ∪ 지난 프레임 메뉴 사각형**을 더 그린다 —
/// 열림 · 하위 메뉴 펼침 · hover · 닫힘(지난 자리 지우기)이 모두 덮인다. 빈 사각형은 뺀다.
pub(crate) fn menu_damage(cur: Rect, prev: Rect) -> Vec<Rect> {
    [cur, prev].into_iter().filter(|r| !r.is_empty()).collect()
}

impl App {
    /// 열린 메뉴가 차지하는 사각형(메뉴 바 띠 ∪ 풀다운(하위 포함) ∪ 팝업 메뉴(하위 포함)) · 없으면 빈 사각형.
    pub(crate) fn menu_rect(&self) -> Rect {
        let mut parts = Vec::new();
        if self.menubar.is_open() {
            parts.push(self.menubar.bounds());
            parts.push(self.menubar.popup_bounds());
        }
        if self.tab_menu.is_open() {
            parts.push(self.tab_menu.bounds());
        }
        parts
            .into_iter()
            .filter(|r| !r.is_empty())
            .reduce(|a, r| a.union(&r))
            .unwrap_or(Rect::new(0, 0, 0, 0))
    }

    /// 부분 다시 그리기를 막는 오버레이(10-18 결정 ①): 자기 사각형 밖에 그리거나(툴팁 · 드롭다운 · 경로 제안 · 끌기 고스트)
    /// 창을 덮는 층(플래시 · 토스트 · 팔레트 · 놓을 자리 표식 · 탭 끌기)이 있으면 전체를 그린다.
    /// 메뉴(풀다운 · 팝업)는 여기 들지 않는다 — [`menu_damage`]로 자기 사각형만(T-189 ⑥).
    pub(crate) fn overlay_active(&self) -> bool {
        self.palette.is_open()
            || self.palette.is_animating()
            || self.toasts_animating()
            || !self.toasts.is_empty()
            || self.panel_toasts.iter().any(|t| !t.is_empty())
            || self.flash.active()
            || self.dnd_mark.is_some()
            || self.tab_drop_hint.is_some()
            || self.dnd_hovering()
            || self.toolbar.is_dragging()
            || self.toolbar.tooltip_visible()
            || self.launcherbar.tooltip_visible()
            || self.panels.iter().any(panel::Panel::popups_open)
    }

    pub(crate) fn paint(&mut self) {
        // 메뉴가 **새로 열려 그려질 때** OS 테마·언어를 다시 읽어 팔레트·라벨을 최신으로(사용자 10-07 · 모든 OS).
        self.refresh_menu_state();
        // 표면을 잠시 꺼내 둔다 — 버퍼가 표면을 빌리는 동안 `paint_into(&mut self)`를 부르기 위해(그리기 중 `self.surface`는 안 쓴다).
        let (Some(win), Some(mut surface)) = (self.window.clone(), self.surface.take()) else {
            return;
        };
        let size = win.inner_size();
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        // 프레임 추적(`NDIR_TRACE_FRAMES=1` · T-176 ④): 요청 → 시작 대기 · 그리기 · present µs를 stderr `[frame]` 줄로(집계 =
        // scripts/frame-stats.py). 꺼져 있으면 비용 = 분기 1.
        let tracing = crate::input::trace_frames();
        let t0 = tracing.then(Instant::now);
        let wait_us = t0
            .zip(self.frame_req_at.take())
            .map(|(t, r)| t.saturating_duration_since(r).as_micros())
            .unwrap_or(0);
        let backend = surface.backend();
        let mut painted = None;
        // 더러워진 영역(10-18 결정 ①) — 꺼내며 비운다 · 계획은 버퍼의 `age`를 본 뒤.
        let mut dmg = std::mem::take(&mut *self.damage.borrow_mut());
        let partial_on = self.settings.flag("gfx.partial_redraw");
        // 메뉴 자리(이번 ∪ 지난 프레임)는 늘 더 그린다(T-189 ⑥ — 종전 = 메뉴가 열려 있으면 전체).
        let menu_now = self.menu_rect();
        for r in menu_damage(menu_now, self.menu_rect_prev) {
            dmg.push(r);
        }
        self.menu_rect_prev = menu_now;
        // 오버레이가 **사라진 첫 프레임**도 전체(사용자 10-24: 툴팁이 지나간 자리의 테두리 잔상 — 툴팁은 자기 사각형 밖에 그려
        // damage에 없다 · 메뉴는 `menu_damage`가 지난 자리를 안다).
        let overlay_now = self.overlay_active();
        let overlay = partial_on && overlay_frame(overlay_now, self.overlay_prev);
        self.overlay_prev = overlay_now;
        let mut mode = String::from("full");
        if let Some(mut buf) = surface.frame(size) {
            let plan = frame_plan(
                partial_on,
                dmg.full,
                buf.age(),
                overlay,
                &dmg.rects,
                Rect::new(0, 0, wi, hi),
            );
            let rects = match plan {
                FramePlan::Full => None,
                FramePlan::Partial(r) => Some(r),
            };
            {
                let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
                let prefs = self.font_prefs();
                let font = Rc::clone(&self.ui_font);
                let mono = self.mono_font.clone();
                let mut dc =
                    RasterCtx::with_font_set(&mut gfx, self.font_set(&font, mono.as_deref()), s)
                        .with_fonts(prefs);
                match &rects {
                    None => {
                        self.paint_into(&mut dc, wi, hi, s);
                        // 플래시 = 맨 마지막 · 자체 글꼴 dc(nexa-sql 10-07 꼴 · 10-18).
                        drop(dc);
                        self.paint_flash(&mut gfx, wi, hi, s);
                    }
                    Some(rs) => {
                        // 사각형마다 클립을 걸고 같은 합성을 돌린다 — 클립 밖 픽셀은 쓰지 않는다(래스터 어휘 전부 클립 준수).
                        for r in rs {
                            dc.push_clip(*r);
                            self.paint_into(&mut dc, wi, hi, s);
                            dc.pop_clip();
                        }
                        mode = format!("partial{}", rs.len());
                    }
                }
            }
            let t1 = tracing.then(Instant::now);
            match &rects {
                None => {
                    let _ = buf.present();
                }
                Some(rs) if rs.is_empty() => {} // 그릴 것 없음 — 내지 않는다(버퍼는 그대로 다음 프레임의 바탕)
                Some(rs) => {
                    let _ = buf.present_damage(rs);
                }
            }
            painted = t1;
        }
        self.surface = Some(surface);
        if let (Some(t0), Some(t1)) = (t0, painted) {
            self.frame_no += 1;
            let t2 = Instant::now();
            eprintln!(
                "[frame] n={} t={}ms wait={}us paint={}us present={}us size={}x{} backend={} mode={}",
                self.frame_no,
                self.started.elapsed().as_millis(),
                wait_us,
                (t1 - t0).as_micros(),
                (t2 - t1).as_micros(),
                wi,
                hi,
                backend,
                mode
            );
        }
    }

    /// 한 프레임을 `dc`에(창 무관). `wi`×`hi` = 표면 크기(장치 px) · `s` = 배율.
    pub(crate) fn paint_into(&mut self, dc: &mut dyn DrawCtx, wi: i32, hi: i32, s: f32) {
        let th = self.theme;
        dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
        // 본문
        self.panels[0].paint(dc, &th);
        if self.dual {
            self.panels[1].paint(dc, &th);
            self.splitter.paint(dc, &th);
        }
        // 탭 바 줄 수가 바뀌었으면(여러 줄 — 그리기가 측정한다) 다시 배치하고 한 번 더 그린다(dir2 win.rs:4941-4947).
        let l0 = self.panels[0].take_tab_lines_changed();
        let l1 = self.panels[1].take_tab_lines_changed();
        if l0 || l1 {
            self.layout_core();
            self.redraw();
        }
        // 탭을 반대 패널로 끄는 중: 대상 탭 바를 강조색으로 옅게 덮고 놓일 자리에 세로선(탭 바 안 이동의 표식과 같은 색).
        if let Some((dst, line)) = self.tab_drop_hint {
            dc.fill_rect_alpha(self.panels[dst].tabbar_bounds(), th.accent, 0.12);
            dc.fill_rect(line, th.accent);
        }
        // 끌어오는 중: **놓일 자리만** 강조한다(폴더 행 = 그 행 · 파일 행/빈 곳 = 목록 전체 = 현재 폴더 — 탐색기와 같다 ·
        // 놓을 수 없는 곳은 표시 없음 + 금지 커서). 옅은 강조색 채움 + 2px 테두리.
        if let Some(m) = &self.dnd_mark {
            let r = m.rect;
            let t = (2.0 * s).round().max(1.0) as i32;
            dc.fill_rect_alpha(r, th.accent, 0.14);
            dc.fill_rect(Rect::new(r.x, r.y, r.w, t), th.accent);
            dc.fill_rect(Rect::new(r.x, r.bottom() - t, r.w, t), th.accent);
            dc.fill_rect(Rect::new(r.x, r.y, t, r.h), th.accent);
            dc.fill_rect(Rect::new(r.right() - t, r.y, t, r.h), th.accent);
        }
        // 도크 경계 2종(비어 있으면 안 그린다 — 도크 숨김 · 단일 정보).
        self.dock_split_h.paint(dc, &th);
        self.dock_split_v.paint(dc, &th);
        for d in &self.docks {
            if d.bounds().h > 0 {
                d.paint(dc, &th);
            }
        }
        self.paint_terms(dc, &th, panel_metrics(&self.settings, s).row_h);
        // 크롬(창 전폭)
        dc.fill_rect(self.toolbar.bounds(), th.chrome_bg);
        self.toolbar.paint(dc, &th);
        if self.launcherbar.bounds().h > 0 {
            dc.fill_rect(self.launcherbar.bounds(), th.chrome_bg);
            self.launcherbar.paint(dc, &th);
        }
        dc.fill_rect(
            Rect::new(0, self.toolbar.bounds().bottom() - 1, wi, 1),
            th.border,
        );
        dc.fill_rect(self.menubar.bounds(), th.chrome_bg);
        // 상태줄
        let sb = self.statusbar.bounds();
        dc.fill_rect(sb, th.chrome_bg);
        dc.fill_rect(Rect::new(0, sb.y, wi, 1), th.border);
        self.statusbar.paint(dc, &th);
        // 팝업 층
        self.paint_toasts(dc, &th, Rect::new(0, 0, wi, sb.y), s);
        self.toolbar.paint_tooltip(dc, &th);
        self.toolbar.paint_drag_overlay(dc, &th); // 끌려가는 그룹(툴바 밖까지 나간다)
        self.panels[0].paint_popups(dc, &th);
        if self.dual {
            self.panels[1].paint_popups(dc, &th);
        }
        self.menubar.paint(dc, &th);
        self.tab_menu.paint(dc, &th);
        self.palette.paint(dc, &th); // 명령 팔레트 = 맨 위 층(T-138)
    }
}

#[cfg(test)]
mod damage_tests {
    use super::*;

    /// MC/DC — 조건 넷(설정 · 전체 요청 · age · 오버레이)이 각각 혼자 전체로 바꾼다 · 사각형은 창 안으로 · 빈 것은 버림 · 상한 넘으면 합집합.
    #[test]
    fn frame_plan_mcdc_and_clipping() {
        let win = Rect::new(0, 0, 100, 50);
        let rs = [
            Rect::new(10, 10, 20, 20),
            Rect::new(90, 40, 30, 30),
            Rect::new(200, 0, 5, 5),
        ];
        assert_eq!(
            frame_plan(true, false, 1, false, &rs, win),
            FramePlan::Partial(vec![Rect::new(10, 10, 20, 20), Rect::new(90, 40, 10, 10)]),
            "창 안으로 자르고 창 밖은 버린다"
        );
        assert_eq!(
            frame_plan(false, false, 1, false, &rs, win),
            FramePlan::Full,
            "설정 끔"
        );
        assert_eq!(
            frame_plan(true, true, 1, false, &rs, win),
            FramePlan::Full,
            "전체 요청"
        );
        assert_eq!(
            frame_plan(true, false, 0, false, &rs, win),
            FramePlan::Full,
            "새 버퍼"
        );
        assert_eq!(
            frame_plan(true, false, 2, false, &rs, win),
            FramePlan::Full,
            "이중 버퍼"
        );
        assert_eq!(
            frame_plan(true, false, 1, true, &rs, win),
            FramePlan::Full,
            "오버레이"
        );
        assert_eq!(
            frame_plan(true, false, 1, false, &[], win),
            FramePlan::Partial(vec![]),
            "사각형 없음 = 그릴 것 없음"
        );
        let many: Vec<Rect> = (0..MAX_DAMAGE_RECTS as i32 + 1)
            .map(|i| Rect::new(i * 10, 0, 5, 5))
            .collect();
        assert_eq!(
            frame_plan(true, false, 1, false, &many, win),
            FramePlan::Partial(vec![Rect::new(0, 0, 85, 5)]),
            "상한 넘으면 합집합 하나"
        );
    }

    /// 오버레이 프레임: 있음 · 사라진 첫 프레임 = 전체 · 둘 다 없음 = 부분 가능(사용자 10-24 툴팁 잔상).
    #[test]
    fn overlay_frame_includes_the_frame_after_it_disappears() {
        assert!(overlay_frame(true, false), "나타남");
        assert!(overlay_frame(true, true), "유지");
        assert!(
            overlay_frame(false, true),
            "사라진 첫 프레임 = 지난 자리 지우기"
        );
        assert!(!overlay_frame(false, false), "없음 = 부분 다시 그리기");
    }

    /// 메뉴 damage: 이번 ∪ 지난 · 빈 것 제외(열림 = 이번만 · 닫힘 = 지난만 · 둘 다 없음 = 없음).
    #[test]
    fn menu_damage_covers_open_and_close() {
        let z = Rect::new(0, 0, 0, 0);
        let a = Rect::new(10, 10, 50, 80);
        let b = Rect::new(10, 10, 120, 80);
        assert!(menu_damage(z, z).is_empty());
        assert_eq!(menu_damage(a, z), vec![a], "열림");
        assert_eq!(menu_damage(z, a), vec![a], "닫힘 = 지난 자리 지우기");
        assert_eq!(menu_damage(b, a), vec![b, a], "하위 메뉴 펼침");
    }

    /// 겹치는 사각형은 합치고 · 빈 사각형은 무시 · `full`은 그대로.
    #[test]
    fn damage_push_merges() {
        let mut d = Damage::default();
        d.push(Rect::new(0, 0, 0, 10));
        assert!(d.rects.is_empty());
        d.push(Rect::new(0, 0, 10, 10));
        d.push(Rect::new(5, 5, 10, 10));
        d.push(Rect::new(50, 0, 5, 5));
        assert_eq!(
            d.rects,
            vec![Rect::new(0, 0, 15, 15), Rect::new(50, 0, 5, 5)]
        );
        assert!(!d.full);
    }
}
