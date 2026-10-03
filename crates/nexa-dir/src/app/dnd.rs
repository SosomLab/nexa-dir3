//! App — 외부 끌어다 놓기 **1차**(docs/port/19 §4-7 "1차 범위" · SHELL-060/061/062/068 축약 · 3-OS 공통 = winit `HoveredFile`/`DroppedFile`):
//! 대상 = 놓은 시점 커서 아래 **폴더 행** / 그 패널의 현재 폴더(SHELL-061) · 연산 = Ctrl 복사 · Shift 이동 · 기본 = 같은 볼륨 이동 /
//! 다른 볼륨 복사(SHELL-062) · 소스가 대상과 같거나 상위면 거부 · 전송 중이면 거부(SHELL-068) · 전송 엔진(`start_transfer`) 합류.
//! winit은 파일마다 `DroppedFile`을 주고 끝 신호가 없다 → 모았다가 **틱에서 한 번에** 처리(`dnd_flush`). 발신·OLE 완전 이식·자동 스크롤은 2차.

use crate::*;
use ndir_ops::Op;

impl App {
    pub(crate) fn dnd_hover(&mut self, path: PathBuf) {
        if !self.dnd_hover.contains(&path) {
            self.dnd_hover.push(path);
        }
    }

    pub(crate) fn dnd_cancel(&mut self) {
        self.dnd_hover.clear();
    }

    pub(crate) fn dnd_dropped(&mut self, path: PathBuf) {
        self.dnd_hover.clear();
        if !self.dnd_drop.contains(&path) {
            self.dnd_drop.push(path);
        }
    }

    /// 틱 — 모인 드롭을 처리했으면 true.
    pub(crate) fn dnd_flush(&mut self) -> bool {
        if self.dnd_drop.is_empty() {
            return false;
        }
        let sources = std::mem::take(&mut self.dnd_drop);
        let (x, y) = self.cursor;
        self.external_drop(sources, Point { x, y });
        true
    }

    /// 놓는 자리 → (패널, 대상 폴더): 폴더 행 위 = 그 폴더 · 파일 행/빈 본문 = 그 패널의 현재 폴더 · 패널 밖/가상 최상위 = None(SHELL-061).
    pub(crate) fn drop_dest_at(&self, p: Point) -> Option<(usize, PathBuf)> {
        let i = if self.panels[0].bounds().contains(p) {
            0
        } else if self.dual && self.panels[1].bounds().contains(p) {
            1
        } else {
            return None;
        };
        let panel = &self.panels[i];
        let root = panel.root_path();
        if ndir_vfs::is_virtual_root(&root) {
            return None;
        }
        let dest = panel
            .rows()
            .row_at(p.x, p.y)
            .and_then(|r| panel.rows().source().row_path(r))
            .filter(|path| path.is_dir())
            .unwrap_or(root);
        Some((i, dest))
    }

    /// 외부 드롭 실행 — 시작했으면 (대상, 연산). 거부 사유는 상태줄.
    pub(crate) fn external_drop(
        &mut self,
        sources: Vec<PathBuf>,
        at: Point,
    ) -> Option<(PathBuf, Op)> {
        if sources.is_empty() {
            return None;
        }
        if self.transfer.is_some() {
            self.note_status(&tr("ops.busy"));
            return None;
        }
        let (panel, dest) = match self.drop_dest_at(at) {
            Some(d) => d,
            None => {
                let root = self.panels[self.active].root_path();
                if ndir_vfs::is_virtual_root(&root) {
                    return None;
                }
                (self.active, root)
            }
        };
        // 자기 자신/상위 폴더 안으로는 🚫(SHELL-062).
        if sources
            .iter()
            .any(|s| s == &dest || ndir_ops::is_same_or_sub(s, &dest))
        {
            self.note_status(&tr("dnd.rejected"));
            return None;
        }
        let op = if self.primary {
            Op::Copy
        } else if self.shift || ndir_ops::same_volume(&sources[0], &dest) {
            Op::Move
        } else {
            Op::Copy
        };
        if panel != self.active {
            self.set_active(panel);
        }
        let n = sources.len().to_string();
        self.note_status(&trf("dnd.dropped", &[&n, &ndir_ops::leaf_name(&dest)]));
        self.start_transfer(sources, dest.clone(), op, false);
        Some((dest, op))
    }

    fn note_status(&mut self, text: &str) {
        let mut inv = Invalidations::default();
        self.statusbar.set_left(text, &mut inv);
        self.redraw();
    }
}
