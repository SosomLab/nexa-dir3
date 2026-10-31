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

/// 지난 프레임 damage 이력의 길이 상한(IOSurface 삼중 회전 = age 3 · 건너뛴 프레임이 있으면 조금 더) — 넘는 age는 전체.
pub(crate) const DAMAGE_HISTORY: usize = 8;

/// 프레임 계획(순수 · MC/DC): 부분은 설정 on · 전체 요청 없음 · 버퍼 내용을 아는 때(`age` ≥ 1) · 오버레이 없음일 때만 —
/// `age` n이면 **이번 사각형 ∪ 지난 n−1 프레임에 그린 사각형**(`prev` · 최신 먼저 · 이력이 모자라면 전체 · 10-31 macOS IOSurface
/// 삼중 버퍼 age 3)을 창 안으로 자르고(빈 것 제거 · 다른 것에 포함된 것 제거) [`MAX_DAMAGE_RECTS`]를 넘으면 합집합 하나로.
pub(crate) fn frame_plan(
    enabled: bool,
    full: bool,
    age: u8,
    overlay: bool,
    rects: &[Rect],
    win: Rect,
    prev: &[Vec<Rect>],
) -> FramePlan {
    if !enabled || full || age == 0 || overlay {
        return FramePlan::Full;
    }
    let back = usize::from(age) - 1;
    if back > prev.len() {
        return FramePlan::Full;
    }
    let mut out: Vec<Rect> = Vec::new();
    for r in rects
        .iter()
        .chain(prev[..back].iter().flatten())
        .map(|r| r.intersection(&win))
        .filter(|r| !r.is_empty())
    {
        // 이미 든 것에 포함되면 버리고 · 새것이 기존 것을 품으면 그것들을 뺀다(같은 상태줄 사각형이 프레임마다 반복되는 꼴).
        if out.iter().any(|k| k.intersection(&r) == r) {
            continue;
        }
        out.retain(|k| r.intersection(k) != *k);
        out.push(r);
    }
    if out.len() > MAX_DAMAGE_RECTS {
        let u = out.iter().skip(1).fold(out[0], |a, r| a.union(r));
        out = vec![u];
    }
    FramePlan::Partial(out)
}

/// 이력에 넣을 "이 프레임에서 **바뀐** 사각형"(순수 · 10-31): 전체 프레임 = 창 전체 · 부분 = 이번 프레임의 **자기 damage**(그린 사각형이
/// 아니다 — 그린 것(= 지난 이력과의 합집합)을 넣으면 전체 프레임 하나가 이력을 타고 영원히 창 전체로 번진다 · 실측 `partial1[0,0,2400x1600]`) ·
/// 내지 않은 프레임(그릴 것 없음) = None. `content_full` = 내용이 실제로 전부 바뀐 전체(전체 요청 · 오버레이) — **age 부족만으로**
/// 전체를 그린 프레임은 바뀐 것이 자기 damage뿐이라 그것만 넣는다(협업 지적 · 기동/표면 회전 뒤 수렴 full×3 → full×1).
pub(crate) fn history_entry(
    plan: &FramePlan,
    own: &[Rect],
    win: Rect,
    content_full: bool,
) -> Option<Vec<Rect>> {
    let own_clipped = || {
        own.iter()
            .map(|r| r.intersection(&win))
            .filter(|r| !r.is_empty())
            .collect()
    };
    match plan {
        FramePlan::Full if content_full => Some(vec![win]),
        FramePlan::Full => Some(own_clipped()),
        FramePlan::Partial(r) if r.is_empty() => None,
        FramePlan::Partial(_) => Some(own_clipped()),
    }
}

/// 프레임이 필요한가(순수 · T-201 ⓒ · 10-31): 전체 요청이 있거나 더러워진 사각형이 있을 때만 — 틱 요청만(사각형 0)은 프레임이 아니라
/// 깨움(`about_to_wait`의 `live` → `ui.frame_ms`)으로 충분하다(위젯 틱은 유휴 틱에서 돈다 · 그릴 것이 없는데 버퍼만 잡던 빈 프레임 제거).
pub(crate) fn frame_needed(full: bool, has_rects: bool) -> bool {
    full || has_rects
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
            // 크기가 바뀌면 지난 사각형 이력은 다른 좌표계 — 비운다(표면도 새것이라 age 0 = 전체).
            if self.damage_hist_size != (wi, hi) {
                self.damage_hist.clear();
                self.damage_hist_size = (wi, hi);
            }
            let plan = frame_plan(
                partial_on,
                dmg.full,
                buf.age(),
                overlay,
                &dmg.rects,
                Rect::new(0, 0, wi, hi),
                &self.damage_hist,
            );
            let hist_entry = history_entry(
                &plan,
                &dmg.rects,
                Rect::new(0, 0, wi, hi),
                dmg.full || overlay,
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
                // 영역별 글꼴(GAP-015 · 10-25): 목록·상태줄·대화 본문은 FontSet 슬롯 · 메뉴·대화상자는 RasterCtx 확장(nexa-ui 232).
                let (list_f, status_f, menu_f, dialog_f) = (
                    self.list_font.clone(),
                    self.status_font.clone(),
                    self.menu_font.clone(),
                    self.dialog_font.clone(),
                );
                let (menu_p, dialog_p) = self.area_prefs();
                let fonts = nexa_ctl::raster::FontSet {
                    base: &font,
                    peerlist: list_f.as_deref(),
                    message: dialog_f.as_deref(),
                    status: status_f.as_deref(),
                    mono: mono.as_deref(),
                };
                let mut dc = RasterCtx::with_font_set(&mut gfx, fonts, s)
                    .with_fonts(prefs)
                    .with_area_fonts(menu_f.as_deref(), dialog_f.as_deref())
                    .with_area_prefs(Some(menu_p), Some(dialog_p));
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
                            self.paint_clip = Some(*r);
                            self.paint_into(&mut dc, wi, hi, s);
                            self.paint_clip = None;
                            dc.pop_clip();
                        }
                        // 진단(`NDIR_TRACE_FRAMES`): 사각형 좌표까지 — 어느 영역이 깨우는지 바로 보이게(10-31).
                        mode = format!(
                            "partial{}[{}]",
                            rs.len(),
                            rs.iter()
                                .map(|r| format!("{},{},{}x{}", r.x, r.y, r.w, r.h))
                                .collect::<Vec<_>>()
                                .join(";")
                        );
                    }
                }
            }
            let t1 = tracing.then(Instant::now);
            if let Some(path) = self.capture_path.take() {
                // 프레임 캡처(`capture:<파일>` · 10-24): 머리 "NDIRCAP <w> <h>" 한 줄 + 픽셀 u32(XRGB · little-endian) — 변환은 밖에서.
                let mut out = format!("NDIRCAP {wi} {hi}\n").into_bytes();
                out.extend(buf.iter().flat_map(|px| px.to_le_bytes()));
                let _ = std::fs::write(&path, out);
            }
            match &rects {
                None => {
                    let _ = buf.present();
                }
                Some(rs) if rs.is_empty() => {} // 그릴 것 없음 — 내지 않는다(버퍼는 그대로 다음 프레임의 바탕)
                Some(rs) => {
                    let _ = buf.present_damage(rs);
                }
            }
            // 이 프레임에서 바뀐 사각형을 이력 맨 앞에(`history_entry` · 내지 않은 프레임은 없다) · age ≥ 2 부분 그리기의 재료.
            if let Some(d) = hist_entry {
                self.damage_hist.insert(0, d);
                self.damage_hist.truncate(DAMAGE_HISTORY);
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
        // 부분 프레임 컬링(10-31): 지금 그리는 사각형과 겹치지 않는 최상위 위젯은 건너뛴다 — 클립은 픽셀만 막고 글 배치·글리프
        // 비용은 그대로라(맥 실측 partial1 래스터 7.99 ms = full과 같음) 위젯 단위로 걸러야 상태줄 깜빡임 한 번이 상태줄 값이 된다.
        // 팝업 층(메뉴 · 툴팁 · 토스트 · 팔레트)은 오버레이 규칙이 전체 프레임으로 돌리므로 그대로 그린다.
        let clip = self.paint_clip;
        let vis = move |r: Rect| clip.is_none_or(|c| c.intersects(&r));
        dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
        // 크기 열의 폴더 크기(T-166 3차): 스크롤로 드러난 행을 그리기 직전에 요청/반영(무효화는 지금 그리는 프레임이 덮는다).
        if self.dirsize_col_enabled() {
            let mut col_inv = Invalidations::default();
            self.dirsize_col_sync(&mut col_inv);
        }
        // 본문
        if vis(self.panels[0].bounds()) {
            self.panels[0].paint(dc, &th);
        }
        if self.dual {
            if vis(self.panels[1].bounds()) {
                self.panels[1].paint(dc, &th);
            }
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
            if d.bounds().h > 0 && vis(d.bounds()) {
                d.paint(dc, &th);
            }
        }
        if self
            .docks
            .iter()
            .any(|d| d.bounds().h > 0 && vis(d.bounds()))
        {
            self.paint_terms(dc, &th, panel_metrics(&self.settings, s).row_h);
        }
        // 크롬(창 전폭)
        dc.fill_rect(self.toolbar.bounds(), th.chrome_bg);
        if vis(self.toolbar.bounds()) {
            self.toolbar.paint(dc, &th);
        }
        if self.launcherbar.bounds().h > 0 && vis(self.launcherbar.bounds()) {
            dc.fill_rect(self.launcherbar.bounds(), th.chrome_bg);
            self.launcherbar.paint(dc, &th);
        }
        // 툴바 아래 구분선 — 드래그 중 자리 표시 틀(강조 · 도크 바닥 줄)이 덮이지 않게 그 폭은 비운다(사용자 10-24 · nexa-ui 227).
        let sep_y = self.toolbar.bounds().bottom() - 1;
        match self.toolbar.placeholder_rect() {
            Some(p) if p.w > 0 => {
                dc.fill_rect(Rect::new(0, sep_y, p.x.max(0), 1), th.border);
                dc.fill_rect(
                    Rect::new(p.right(), sep_y, (wi - p.right()).max(0), 1),
                    th.border,
                );
            }
            _ => dc.fill_rect(Rect::new(0, sep_y, wi, 1), th.border),
        }
        dc.fill_rect(self.menubar.bounds(), th.chrome_bg);
        // 상태줄
        let sb = self.statusbar.bounds();
        dc.fill_rect(sb, th.chrome_bg);
        dc.fill_rect(Rect::new(0, sb.y, wi, 1), th.border);
        if vis(sb) {
            self.statusbar.paint(dc, &th);
        }
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
            frame_plan(true, false, 1, false, &rs, win, &[]),
            FramePlan::Partial(vec![Rect::new(10, 10, 20, 20), Rect::new(90, 40, 10, 10)]),
            "창 안으로 자르고 창 밖은 버린다"
        );
        assert_eq!(
            frame_plan(false, false, 1, false, &rs, win, &[]),
            FramePlan::Full,
            "설정 끔"
        );
        assert_eq!(
            frame_plan(true, true, 1, false, &rs, win, &[]),
            FramePlan::Full,
            "전체 요청"
        );
        assert_eq!(
            frame_plan(true, false, 0, false, &rs, win, &[]),
            FramePlan::Full,
            "새 버퍼"
        );
        assert_eq!(
            frame_plan(true, false, 2, false, &rs, win, &[]),
            FramePlan::Full,
            "이중 버퍼"
        );
        assert_eq!(
            frame_plan(true, false, 1, true, &rs, win, &[]),
            FramePlan::Full,
            "오버레이"
        );
        assert_eq!(
            frame_plan(true, false, 1, false, &[], win, &[]),
            FramePlan::Partial(vec![]),
            "사각형 없음 = 그릴 것 없음"
        );
        let many: Vec<Rect> = (0..MAX_DAMAGE_RECTS as i32 + 1)
            .map(|i| Rect::new(i * 10, 0, 5, 5))
            .collect();
        assert_eq!(
            frame_plan(true, false, 1, false, &many, win, &[]),
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

    /// 프레임 필요 판정(T-201 ⓒ · MC/DC): 전체 · 사각형 각각 혼자 참 · 둘 다 없음(틱만) = 거짓.
    #[test]
    fn frame_needed_only_for_full_or_rects() {
        assert!(frame_needed(true, false));
        assert!(frame_needed(false, true));
        assert!(!frame_needed(false, false), "틱만 = 프레임 없음(깨움만)");
    }

    /// 이력 항목(10-31): 전체 = 창 · 부분 = 자기 damage(창 안으로) · 빈 부분 = 없음. 자기 damage를 넣어야 전체 프레임이 3프레임 뒤 사라진다.
    #[test]
    fn history_entry_records_own_damage_not_drawn_union() {
        let win = Rect::new(0, 0, 100, 50);
        let own = [Rect::new(0, 40, 100, 10), Rect::new(90, 45, 30, 30)];
        let clipped = vec![Rect::new(0, 40, 100, 10), Rect::new(90, 45, 10, 5)];
        assert_eq!(
            history_entry(&FramePlan::Full, &own, win, true),
            Some(vec![win]),
            "내용이 전부 바뀐 전체(전체 요청 · 오버레이)"
        );
        assert_eq!(
            history_entry(&FramePlan::Full, &own, win, false),
            Some(clipped.clone()),
            "age 부족만으로 전체 = 바뀐 건 자기 damage뿐"
        );
        assert_eq!(
            history_entry(&FramePlan::Partial(vec![win]), &own, win, false),
            Some(clipped),
            "그린 것(창 전체)이 아니라 자기 damage(창 안으로 자름)"
        );
        assert_eq!(
            history_entry(&FramePlan::Partial(vec![]), &own, win, false),
            None
        );
        // 수렴: 전체 3 → 상태줄만 바뀌는 프레임이 이어지면 3번째부터 상태줄 사각형만 그린다.
        let st = Rect::new(0, 40, 100, 10);
        let mut hist: Vec<Vec<Rect>> = vec![vec![win], vec![win], vec![win]];
        let mut drawn = Vec::new();
        for _ in 0..4 {
            let plan = frame_plan(true, false, 3, false, &[st], win, &hist);
            drawn.push(plan == FramePlan::Partial(vec![st]));
            hist.insert(0, history_entry(&plan, &[st], win, false).unwrap());
        }
        assert_eq!(drawn, vec![false, false, true, true]);
    }

    /// age ≥ 2(10-31 macOS IOSurface 삼중 회전): 이번 ∪ 지난 n−1 프레임의 사각형 · 이력이 모자라면 전체 · 포함된 중복은 하나로.
    #[test]
    fn frame_plan_age_n_unions_recent_history() {
        let win = Rect::new(0, 0, 100, 50);
        let status = Rect::new(0, 40, 100, 10);
        let now = [status];
        let hist = vec![
            vec![status],
            vec![Rect::new(10, 10, 5, 5)],
            vec![Rect::new(50, 0, 10, 10)],
        ];
        assert_eq!(
            frame_plan(true, false, 3, false, &now, win, &hist),
            FramePlan::Partial(vec![status, Rect::new(10, 10, 5, 5)]),
            "age 3 = 이번 + 지난 2프레임 · 같은 상태줄 사각형은 한 번 · 3프레임 전 것은 안 든다"
        );
        assert_eq!(
            frame_plan(true, false, 3, false, &now, win, &hist[..1]),
            FramePlan::Full,
            "이력 1개뿐이면 age 3은 전체"
        );
        assert_eq!(
            frame_plan(true, false, 2, false, &[], win, &[vec![win]]),
            FramePlan::Partial(vec![win]),
            "지난 프레임이 전체였으면 창 전체 사각형 하나(= 전체 다시 그리기와 같은 비용)"
        );
        assert_eq!(
            frame_plan(
                true,
                false,
                2,
                false,
                &[Rect::new(0, 0, 100, 50)],
                win,
                &[vec![status]]
            ),
            FramePlan::Partial(vec![win]),
            "새것이 기존 것을 품으면 기존 것을 뺀다"
        );
    }
}
