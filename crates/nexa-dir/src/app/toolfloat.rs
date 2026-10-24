//! App — 툴바 그룹 **띄우기/붙이기**(T-108 2/2 · NEW-010 · nexa-sql `app/toolbar.rs` 차용): 도크(`ToolDock`)의 그룹 하나를 작은 OS 창
//! ([`crate::toolfloat::ToolFloatWin`])으로 떼어 내고, × · 우클릭 · 메뉴로 도크에 되돌린다. 배치(플로팅 여부 · 창 좌표)는 설정
//! `toolbar.dock_layout`에 함께 저장돼 다음 기동 때 창이 복원된다. 창 생성은 이벤트 루프 핸들이 있는 `open_requested_windows`에서 —
//! 메뉴 · 드래그는 `pending_float`에 요청만 적는다.

use crate::toolfloat::{FloatAction, ToolFloatWin};
use crate::*;

impl App {
    /// 커서 아래의 도크 그룹 id(도크에 붙은 것만 · 플로팅 그룹은 다른 창에 있다).
    pub(crate) fn toolbar_group_at(&self, x: i32, y: i32) -> Option<String> {
        self.toolbar
            .groups()
            .into_iter()
            .filter(|(_, _, floating)| !floating)
            .map(|(id, _, _)| id)
            .find(|id| {
                self.toolbar
                    .bar(id)
                    .is_some_and(|b| b.bounds().contains(Point { x, y }))
            })
    }

    /// 그룹 띄우기 요청 — `at` = 도크 클라이언트 좌표(커서) · 창은 다음 `open_requested_windows`에서.
    pub(crate) fn float_group(&mut self, gid: &str, at: Option<(i32, i32)>) {
        if self.tool_floats.iter().any(|f| f.group == gid)
            || self.pending_float.iter().any(|(g, _)| g == gid)
        {
            return;
        }
        self.pending_float.push((gid.to_string(), at));
    }

    /// 그룹을 도크로 되돌린다(플로팅 창 닫기 포함 · 배치 저장 · 창 재배치).
    pub(crate) fn dock_group(&mut self, gid: &str) {
        if let Some(i) = self.tool_floats.iter().position(|f| f.group == gid) {
            self.tool_floats[i].close();
            self.tool_floats.remove(i);
        }
        self.pending_float.retain(|(g, _)| g != gid);
        self.toolbar.dock(gid);
        let _ = self.toolbar.take_actions();
        self.save_dock_layout();
        self.layout();
        self.redraw();
    }

    /// 도크 배치(순서 · 행 · 플로팅 여부/좌표)를 설정 `toolbar.dock_layout`에(바뀌었을 때만).
    pub(crate) fn save_dock_layout(&mut self) {
        let v = self.toolbar.layout().serialize();
        if self.settings.get("toolbar.dock_layout").unwrap_or("") != v {
            let _ = self.settings.set("toolbar.dock_layout", &v);
            let _ = self.settings.save();
        }
    }

    /// 플로팅 창 만들기 — `at`: 도크 클라이언트 좌표(커서) · 저장된 화면 좌표가 있으면(복원) 그것.
    pub(crate) fn open_float(&mut self, el: &ActiveEventLoop, gid: &str, at: Option<(i32, i32)>) {
        if self.tool_floats.iter().any(|f| f.group == gid) {
            return;
        }
        let Some(title) = self.toolbar.title(gid).map(str::to_string) else {
            return;
        };
        let scale = self.scale.max(0.5);
        let (bw, bh) = {
            let h = self.toolbar.preferred_height() as f32 / scale;
            let w = self
                .toolbar
                .bar(gid)
                .map_or(120.0, |b| b.preferred_width() as f32 / scale);
            (w, h)
        };
        // 저장된 좌표(이미 플로팅으로 표시)면 그대로, 아니면 커서(클라이언트) → 화면 좌표(창 가운데가 커서에 오게).
        let pos = if let Some(p) = self.toolbar.floating_pos(gid) {
            Some(p)
        } else {
            at.and_then(|(cx, cy)| {
                let w = self.window.as_ref()?;
                let o = w.inner_position().ok()?;
                Some((
                    o.x + cx - (bw * scale / 2.0) as i32,
                    o.y + cy - (bh * scale / 2.0) as i32,
                ))
            })
        };
        let owner = self.window.clone();
        let win = ToolFloatWin::open(
            el,
            gid,
            &format!("{} — {title}", crate::APP_TITLE),
            theme::window_theme(self.settings.theme_mode()),
            pos,
            (bw, bh),
            owner.as_deref(),
        );
        let Some(win) = win else { return };
        let (x, y) = win.position().or(pos).unwrap_or((0, 0));
        self.toolbar.float(gid, x, y);
        let _ = self.toolbar.take_actions();
        self.save_dock_layout();
        self.tool_floats.push(win);
        self.layout();
        self.redraw();
    }

    /// 플로팅 창 동기(이벤트 루프 유휴 · 메인 창이 있을 때): 요청분 열기 · 배치가 플로팅인데 창이 없는 그룹 열기(기동 복원 · 툴바
    /// 재생성 뒤) · 배치가 플로팅이 아닌데 창이 남은 그룹 닫기(툴바 배치 초기화 · 설정 변경).
    pub(crate) fn sync_float_windows(&mut self, el: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }
        let reqs = std::mem::take(&mut self.pending_float);
        for (gid, at) in reqs {
            self.open_float(el, &gid, at);
        }
        let floating: Vec<String> = self
            .toolbar
            .groups()
            .into_iter()
            .filter(|(_, _, f)| *f)
            .map(|(id, _, _)| id)
            .collect();
        for gid in &floating {
            if !self.tool_floats.iter().any(|f| &f.group == gid) {
                self.open_float(el, gid, None);
            }
        }
        let mut i = 0;
        while i < self.tool_floats.len() {
            if floating.contains(&self.tool_floats[i].group) {
                i += 1;
            } else {
                self.tool_floats[i].close();
                self.tool_floats.remove(i);
            }
        }
    }

    /// 플로팅 창의 사건 — 처리했으면 `true`.
    pub(crate) fn float_window_event(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        let Some(fi) = self.tool_floats.iter().position(|f| f.is(id)) else {
            return false;
        };
        let gid = self.tool_floats[fi].group.clone();
        match self.tool_floats[fi].handle(event) {
            FloatAction::Paint => {
                if let Some(bar) = self.toolbar.bar_mut(&gid) {
                    self.tool_floats[fi].paint(bar, &self.ui_font, &self.theme);
                }
            }
            FloatAction::Input(ev) => {
                let client = self.tool_floats[fi].client();
                let sc = self.tool_floats[fi].scale();
                let mut inv = Invalidations::default();
                let mut clicked = None;
                if let Some(bar) = self.toolbar.bar_mut(&gid) {
                    bar.set_scale(sc);
                    bar.set_bounds(client, &mut inv);
                    bar.on_event(&ev, &mut inv);
                    clicked = bar.take_clicked();
                }
                if matches!(ev, InputEvent::RightDown { .. }) {
                    // 플로팅 창 우클릭 = 붙이기(한 번의 입력으로 · nexa-sql 규약).
                    self.dock_group(&gid);
                    return true;
                }
                if !inv.is_empty() {
                    self.tool_floats[fi].redraw();
                }
                if let Some(cmd) = clicked {
                    self.command(&cmd);
                    self.tool_floats[fi].redraw();
                    self.redraw();
                }
            }
            FloatAction::Moved(x, y) => {
                self.toolbar.set_floating_pos(&gid, x, y);
                let _ = self.toolbar.take_actions();
                self.save_dock_layout();
            }
            FloatAction::Close => self.dock_group(&gid),
            FloatAction::None => {}
        }
        true
    }
}
