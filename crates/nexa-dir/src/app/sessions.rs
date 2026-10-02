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
            s.panels[i].modes = p.session_modes();
            s.panels[i].col_widths = p.col_widths();
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
