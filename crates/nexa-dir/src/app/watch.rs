//! App — 폴더 변경 감시 → 무간섭 재열람(dir2 PANEL-036·042 · `Watcher` 포트 · 간격 = 포트가 정한다(폴링 1 s · Windows 통지 250 ms) · 호스트 틱이 부른다).

use crate::*;

/// "내 PC" 볼륨 구성 확인 간격(ms).
const DRIVES_POLL_MS: u64 = 1000;

/// 볼륨 지문이 바뀌었는가(순수): 처음 본 값 · 모름(0)은 "바뀜"이 아니다 — 기준만 잡는다.
pub(crate) fn volumes_changed(seen: &mut Option<u64>, now: u64) -> bool {
    if now == 0 {
        return false;
    }
    match seen.replace(now) {
        Some(prev) => prev != now,
        None => false,
    }
}

impl App {
    /// 드라이브/볼륨이 붙거나 떨어지면 "내 PC"를 보는 탭을 다시 읽는다(dir2 WINC-061 · SHELL-038 `WM_DEVICECHANGE` 대응 ·
    /// 종전 dir3 = 처리 없음 → F5 전까지 낡은 목록). 내 PC를 보는 패널이 없으면 조회하지 않고 기준도 버린다(다시 들어오면 새로 읽으므로).
    pub(crate) fn drives_tick(&mut self, now: Instant) {
        if now < self.drives_next {
            return;
        }
        self.drives_next = now + Duration::from_millis(DRIVES_POLL_MS);
        if !self
            .panels
            .iter()
            .any(|p| ndir_vfs::is_virtual_root(p.root_path()))
        {
            self.drives_seen = None;
            return;
        }
        let stamp = self.platform.disk.volumes_stamp();
        if volumes_changed(&mut self.drives_seen, stamp) {
            let mut inv = Invalidations::default();
            for p in &mut self.panels {
                if ndir_vfs::is_virtual_root(p.root_path()) {
                    p.reopen(&mut inv);
                }
            }
            self.update_status(); // 새 목록의 용량 열을 채운다.
            self.redraw();
        }
    }

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
        self.drives_tick(now);
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
