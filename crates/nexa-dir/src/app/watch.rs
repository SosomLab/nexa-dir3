//! App — 폴더 변경 감시 → 무간섭 재열람(dir2 PANEL-036·042 · `Watcher` 포트 · 간격 = 포트가 정한다(폴링 1 s · Windows 통지 250 ms) · 호스트 틱이 부른다).

use crate::*;

impl App {
    /// 감시 대상 = 두 패널 활성 탭의 폴더(바뀌었을 때만 재지정 — `Watcher::watch`가 같은 집합이면 무시).
    pub(crate) fn watch_sync(&mut self) {
        let mut dirs: Vec<PathBuf> = self
            .panels
            .iter()
            .map(|p| p.root_path())
            .filter(|p| !ndir_vfs::is_virtual_root(p))
            .collect();
        dirs.dedup();
        self.platform.watcher.watch(&dirs);
    }

    /// 포트 간격마다 변경을 거둬 그 폴더를 보는 탭을 다시 읽는다(캐럿·스크롤 유지). 다음 깨울 시각을 돌려준다.
    pub(crate) fn watch_tick(&mut self, now: Instant) -> Instant {
        if now < self.watch_next {
            return self.watch_next;
        }
        self.watch_next = now + Duration::from_millis(self.platform.watcher.poll_interval_ms());
        self.watch_sync();
        let changed = self.platform.watcher.poll();
        if !changed.is_empty() {
            let mut inv = Invalidations::default();
            for p in &mut self.panels {
                if changed.iter().any(|c| *c == p.root_path()) {
                    p.reopen(&mut inv);
                }
            }
            // 준비해 둔 셸 메뉴는 옛 폴더 상태 기준 — 버리고 머무름부터 다시(선행 구축 무효화).
            self.platform.ctxmenu.invalidate();
            self.ctx_dwell_done = false;
            self.ctx_dwell_since = now;
            self.update_status();
            self.redraw();
        }
        self.watch_next
    }
}
