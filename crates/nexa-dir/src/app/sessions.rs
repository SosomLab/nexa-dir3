//! App — 세션 스냅숏·복원·디바운스 저장(T-45 · dir2 PREFS-050~054 · docs/port/13 §5-2).
//!
//! 저장 계기 = 패널 `session_dirty`(탭/경로/보기 변경) → `SaveScheduler`(quiet 1 s · max_delay 5 s — 디바운스 기아 방지 PREFS-617) +
//! 종료(`persist_window`). 복원 = 기동 때(창 생성 전 · PREFS-054) — 실행 인자 경로가 있으면 세션 무시.

use crate::*;

impl App {
    /// 지금 상태의 세션(탭 경로 · 활성 탭 · 보기 모드 · 열 폭 · 활성 패널). dir2가 쓰는 다른 키는 복원 때 받은 값을 보존한다.
    pub(crate) fn session_snapshot(&self) -> Session {
        let mut s = self.session_keep.clone();
        s.active_panel = self.active;
        for (i, p) in self.panels.iter().enumerate() {
            let (tabs, active) = p.session();
            s.panels[i].tabs = tabs;
            s.panels[i].active = active;
            // 전부 tree(기본)면 빈 목록 — 직렬화 생략 규약과 왕복이 같아진다.
            let modes = p.session_modes();
            s.panels[i].modes = if modes.iter().any(|m| m != "tree") {
                modes
            } else {
                Vec::new()
            };
            s.panels[i].col_widths = p.col_widths();
            // 열 순서/표시(T-71) — 기본 그대로면 빈 값(직렬화 생략 왕복).
            let layout = p.col_layout_str();
            s.panels[i].col_layout =
                if layout == crate::order::default_order(crate::order::COLUMN_BLOCKS) {
                    String::new()
                } else {
                    layout
                };
            // 전부 거짓이면 빈 목록(dir2 직렬화는 하나라도 참일 때만 기록 → 파싱 왕복이 같아진다).
            let any = |v: Vec<bool>| if v.iter().any(|x| *x) { v } else { Vec::new() };
            s.panels[i].locked = any(p.session_locked());
            s.panels[i].pinned = any(p.session_pinned());
            // 탭별 보기 옵션(dir2 08-02 `views`) — 항상 기록한다(값의 주인은 탭 · 설정 기본값이 나중에 바뀌어도 탭은 자기 값).
            s.panels[i].views = p.session_view_flags();
        }
        s
    }

    /// 패널 더러움 수거(두 패널 **전부** 소진 — 단락 OR 금지 · PREFS-616) → 디바운스 예약.
    pub(crate) fn session_collect_dirty(&mut self, now: Instant) {
        let mut dirty = false;
        for p in &mut self.panels {
            if p.take_session_dirty() {
                dirty = true;
            }
        }
        if dirty {
            self.session_save.mark(now);
        }
    }

    /// 유휴 틱 — 디바운스 만료면 저장. 다음 깨울 시각(더러운 동안 ≤1 s)을 돌려준다.
    pub(crate) fn session_tick(&mut self, now: Instant) -> Option<Instant> {
        if self.session_save.tick(now) {
            self.session_write();
        }
        self.session_save
            .dirty()
            .then(|| now + Duration::from_millis(250))
    }

    /// 종료 때 — 더러움과 무관하게 한 번 쓴다(창 기하와 함께 · PREFS-052).
    pub(crate) fn session_flush(&mut self) {
        let _ = self.session_save.flush_now();
        self.session_write();
    }

    fn session_write(&mut self) {
        let Some(dir) = self.session_dir.clone() else {
            return;
        };
        if let Err(e) = self.session_snapshot().save(&dir) {
            eprintln!("nexa-dir: session save failed: {e}");
        }
    }
}
