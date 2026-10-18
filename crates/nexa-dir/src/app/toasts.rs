//! 토스트 범위(사용자 10-18 "파일 삭제 등 패널 작업의 토스트는 그 패널 안에"): **전역**(앱 전체에 걸친 일 · 창 영역 기준) · **패널**(좌/우 패널 각각의
//! 파일 작업 · 그 패널 사각형 기준) — 모서리는 둘 다 설정 `ui.toast_position`(기본 우하단). nexa-ui 214차 `Toasts::paint_in`.

use crate::*;
use nexa_ctl::controls::toast::ToastCorner;

impl App {
    /// 설정 `ui.toast_position` → 모서리.
    pub(crate) fn toast_corner(&self) -> ToastCorner {
        ToastCorner::parse(
            self.settings
                .get("ui.toast_position")
                .unwrap_or("bottom-right"),
        )
    }

    /// 패널 범위 토스트(삭제 · 복사/잘라내기 · 이름 바꾸기 · 풀기 · 드롭 … 그 패널에서 일어난 일) — 패널 번호는 0/1로 접는다.
    pub(crate) fn toast_panel(
        &mut self,
        panel: usize,
        kind: toast::ToastKind,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        self.panel_toasts[panel.min(1)].push(kind, title, body);
        self.redraw();
    }

    pub(crate) fn toasts_tick(&mut self, now: Instant) -> bool {
        let g = self.toasts.tick(now);
        let a = self.panel_toasts[0].tick(now);
        let b = self.panel_toasts[1].tick(now);
        g || a || b
    }

    pub(crate) fn toasts_animating(&self) -> bool {
        self.toasts.animating() || self.panel_toasts.iter().any(|t| t.animating())
    }

    /// 클릭으로 닫기(전역 → 패널 순 · 하나라도 맞으면 그 클릭은 아래로 흘리지 않는다).
    pub(crate) fn toasts_click(&mut self, p: Point) -> bool {
        self.toasts.click(p) || self.panel_toasts[0].click(p) || self.panel_toasts[1].click(p)
    }

    /// 그리기: 패널 토스트는 각 패널 사각형 안(단일 패널이면 왼쪽만) · 전역은 `area`(창 · 상태줄 위) — 둘 다 설정 모서리.
    pub(crate) fn paint_toasts(&mut self, dc: &mut dyn DrawCtx, th: &Theme, area: Rect, s: f32) {
        let corner = self.toast_corner();
        for i in 0..2 {
            if i == 1 && !self.dual {
                continue;
            }
            let b = self.panels[i].bounds();
            self.panel_toasts[i].paint_in(dc, th, b, corner, s);
        }
        self.toasts.paint_in(dc, th, area, corner, s);
    }
}
