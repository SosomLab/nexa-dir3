//! App — 폴더 변경 감시 → 무간섭 재열람(dir2 PANEL-036·042 · `Watcher` 포트 · 간격 = 포트가 정한다(폴링 1 s · Windows 통지 250 ms) · 호스트 틱이 부른다).

use crate::*;

/// 감시 다시 읽기를 미룰 것인가(순수 · dir2 win.rs:9463-9467): 인라인 이름 편집 · 경로 바 편집 · 전송 중 하나라도면 미룬다.
pub(crate) fn reload_deferred(renaming: bool, path_editing: bool, transfer: bool) -> bool {
    renaming || path_editing || transfer
}

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
        // 현재 폴더 + 화면에 펼쳐진 하위 폴더(dir2 WINB-014 · 패널당 64개 상한). 내 PC(가상 최상위)는 드라이브 감시가 본다.
        let mut dirs: Vec<PathBuf> = Vec::new();
        for p in &self.panels {
            if ndir_vfs::is_virtual_root(p.root_path()) {
                continue;
            }
            for d in p.watch_dirs() {
                if !dirs.contains(&d) {
                    dirs.push(d);
                }
            }
        }
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
        // 미뤄 둔 변경(편집·전송 중이던 것)을 이번 변경과 합친다.
        let mut changed = std::mem::take(&mut self.watch_deferred);
        for c in self.platform.watcher.poll() {
            if !changed.contains(&c) {
                changed.push(c);
            }
        }
        if !changed.is_empty() {
            self.dirsizes.invalidate(&changed); // 폴더 크기 캐시(T-166) — 닿는 항목만.
            let mut inv = Invalidations::default();
            let transfer = self.transfer.is_some();
            let mut reloaded = false;
            for p in &mut self.panels {
                let root = p.root_path();
                // 현재 폴더나 펼쳐진 하위 폴더가 바뀌었으면 이 패널을 다시 읽는다(펼침 · 선택 · 캐럿 · 스크롤 유지).
                if !p.watch_dirs().iter().any(|d| changed.contains(d)) {
                    continue;
                }
                // 인라인 이름 편집 · 경로 편집 · 전송 중에는 미룬다(dir2 win.rs:9457-9485 · X-35 D4: 다시 읽기가 편집 행을 흔들거나
                // 진행 중인 전송의 중간 상태를 보여 주지 않게) — 다음 틱에 다시 본다.
                if reload_deferred(p.rows().is_renaming(), p.pathbar.is_editing(), transfer) {
                    if !self.watch_deferred.contains(&root) {
                        self.watch_deferred.push(root);
                    }
                } else {
                    p.reopen(&mut inv);
                    reloaded = true;
                }
            }
            if !reloaded {
                return self.watch_next;
            }
            self.log_with(
                ndir_log::LogKind::Watch,
                trf("log.msg.watch", &[&changed.len().to_string()]),
                Some(changed.len() as u64),
                None,
            );
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
