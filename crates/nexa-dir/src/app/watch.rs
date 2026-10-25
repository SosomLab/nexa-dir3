//! App — 폴더 변경 감시 → 무간섭 재열람(dir2 PANEL-036·042 · `Watcher` 포트 · 간격 = 포트가 정한다(폴링 1 s · Windows 통지 250 ms) · 호스트 틱이 부른다).

use crate::*;
use std::path::Path;

/// 감시 다시 읽기를 미룰 것인가(순수 · dir2 win.rs:9463-9467): 인라인 이름 편집 · 경로 바 편집 · 전송 중 하나라도면 미룬다.
/// 경로가 `.git` 디렉터리(또는 그 안)인가(순수) — 폴더 감시가 git 내부 변경을 목록 재열람과 섞지 않게.
pub(crate) fn is_git_internal(p: &Path) -> bool {
    p.components().any(|c| c.as_os_str() == ".git")
}

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
        for p in &mut self.panels {
            if ndir_vfs::is_virtual_root(p.root_path()) {
                continue;
            }
            for d in p.watch_dirs() {
                if !dirs.contains(&d) {
                    dirs.push(d);
                }
            }
            // 보이는 저장소의 `.git`(T-95 HEAD/index 감시 · 10-25): 다른 앱의 checkout · commit · add가 `.git/HEAD` · `index`를 바꾸면 그 폴더
            // 자체는 바뀌지 않아 종전엔 감시가 못 봤다 → `.git`을 함께 본다(변경은 `is_git_internal`로 걸러 요약 무효화만 · 재열람 아님).
            // worktree · 서브모듈은 `.git`이 **파일**(`gitdir: …`)이고 HEAD/index는 그 gitdir 안에 있다 → `dirinfo::git_dir`(협업 검토 10-25).
            if self.git_enabled {
                if let Some((repo, _)) = p.git_info_fresh() {
                    if let Some(g) = crate::dirinfo::git_dir(&repo) {
                        if g.is_dir() && !dirs.contains(&g) {
                            dirs.push(g);
                        }
                    }
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
        // 포트가 말하는 간격은 **종류**만 본다(OS 통지 = 1000 미만 · 폴링 폴백 = 1000) — 실제 값은 설정(T-100 값 추출 1차 ·
        // `watch.notify_ms` / `watch.poll_ms` · docs/23).
        let port_ms = self.platform.watcher.poll_interval_ms();
        let key = if port_ms < 1000 {
            "watch.notify_ms"
        } else {
            "watch.poll_ms"
        };
        let ms = self.settings.int(key).clamp(50, 60_000) as u64;
        self.watch_next = now + Duration::from_millis(ms);
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
            // git 요약: 닿는 저장소를 버린다(`.git` 안 변경 포함 · 다음 상태 동기가 다시 조회). `.git` 안 변경은 목록 재열람 대상이 아니다.
            self.git_invalidate(&changed);
            changed.retain(|c| !is_git_internal(c));
            if changed.is_empty() {
                let mut inv = Invalidations::default();
                self.git_sync(&mut inv);
                self.redraw();
                return self.watch_next;
            }
            self.dirsizes.invalidate(&changed); // 폴더 크기 캐시(T-166) — 닿는 항목만.
            let mut inv = Invalidations::default();
            let transfer = self.transfer.is_some();
            let self_window =
                Duration::from_millis(self.settings.int("watch.self_window_ms").max(0) as u64);
            let mut reloaded = false;
            for p in &mut self.panels {
                let root = p.root_path();
                // 현재 폴더나 펼쳐진 하위 폴더가 바뀌었으면 이 패널을 다시 읽는다(펼침 · 선택 · 캐럿 · 스크롤 유지).
                let dirs = p.watch_dirs();
                let hits: Vec<&PathBuf> = dirs.iter().filter(|d| changed.contains(d)).collect();
                if hits.is_empty() {
                    continue;
                }
                // 자기 변경 억제(10-18 결정 ③ · `watch.self_window_ms`): 바뀐 것이 현재 폴더뿐이고 이 탭이 방금 스스로 읽었으며
                // 폴더 수정 시각이 그때와 같다 = 우리 작업(전송 · 삭제 · 이름 바꾸기 · 새로 고침 · 진입)의 메아리 — 같은 목록을 두 번
                // 읽지 않는다. 바깥 변경은 수정 시각이 달라져 그대로 읽는다(내용만 바뀐 파일은 다음 통지에).
                if hits.iter().all(|d| **d == root) && p.self_fresh(&root, self_window) {
                    continue;
                }
                // 인라인 이름 편집 · 경로 편집 · 전송 중에는 미룬다(dir2 win.rs:9457-9485 · X-35 D4: 다시 읽기가 편집 행을 흔들거나
                // 진행 중인 전송의 중간 상태를 보여 주지 않게) — 다음 틱에 다시 본다.
                if reload_deferred(p.rows().is_renaming(), p.pathbar.is_editing(), transfer) {
                    if !self.watch_deferred.contains(&root) {
                        self.watch_deferred.push(root);
                    }
                } else {
                    p.reopen_from_watch(&mut inv);
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

#[cfg(test)]
mod git_watch_tests {
    use super::*;

    /// `.git` 안 변경 = git 요약 무효화만(목록 재열람 아님).
    #[test]
    fn git_internal_paths() {
        assert!(is_git_internal(Path::new("D:/repo/.git")));
        assert!(is_git_internal(Path::new("D:/repo/.git/refs/heads")));
        assert!(!is_git_internal(Path::new("D:/repo/src/.gitignore")));
        assert!(!is_git_internal(Path::new("D:/repo/src")));
    }
}
