//! App — 메모리 창의 표본 수집 · 주기 갱신 · 힙 정리(T-93 · docs/22 NEW-002 · nexa-sql `app/memory.rs` 구조).
//!
//! 창이 열려 있을 때만 돈다: `mem_tick`이 [`MEM_REFRESH_MS`]마다 [`App::mem_sample`]을 만들어 창에 넣는다(닫혀 있으면 `None` = 깨우지 않음).

use crate::memstat::{Acc, Cat, Sample};
use crate::*;

/// 메모리 창 갱신 주기(ms).
pub(crate) const MEM_REFRESH_MS: u64 = 1000;
/// 실행 취소 이력 한 건의 어림 바이트(경로 목록 · 상자).
const HISTORY_OP_BYTES: u64 = 1024;

impl App {
    /// 전체 표본 — 운영체제 값 + 영역별 어림(창이 열려 있을 때만 불린다).
    pub(crate) fn mem_sample(&self) -> Sample {
        let mut acc = Acc::default();
        // 프로그램: UI 글꼴 파일(폴백 체인 포함 · 프로세스 수명 동안 쥔다).
        acc.add(Cat::Fonts, self.ui_font.data_bytes());
        // 탭 · 파일 목록: 보이는 탭 / 배경 탭 / 실행 취소 이력.
        for p in &self.panels {
            let (active, background) = p.mem_estimate_split();
            acc.add(Cat::ListsActive, active);
            acc.add(Cat::ListsBackground, background);
        }
        acc.add(Cat::History, self.history.depth() as u64 * HISTORY_OP_BYTES);
        // 탭 · 리소스: 행 아이콘 · 빠른 실행 아이콘 · 도크 미리보기 글.
        acc.add(Cat::RowIcons, app::row_icons::cache_bytes());
        acc.add(Cat::LauncherIcons, app::launcher_icons::cache_bytes());
        let preview: usize = self
            .dock_preview
            .iter()
            .map(|(provider, lines)| {
                provider.len() + lines.iter().map(|l| l.len() + 24).sum::<usize>()
            })
            .sum();
        acc.add(Cat::Preview, preview as u64);
        // 터미널: 화면 버퍼(스크롤백 포함) · 고정폭 글꼴 파일.
        acc.add(
            Cat::TermBuffer,
            self.terms.iter().map(TermView::mem_estimate).sum(),
        );
        if let Some(m) = &self.mono_font {
            acc.add(Cat::TermFont, m.data_bytes());
        }
        // 플러그인: 올라와 있는 모듈.
        acc.add(Cat::Plugins, preview::loaded_plugin_bytes());
        // 화면 그리기: 창 표면(가로 × 세로 × 4) · 글리프 캐시.
        let main =
            u64::from(self.viewport.0.max(0) as u32) * u64::from(self.viewport.1.max(0) as u32) * 4;
        acc.add(Cat::SurfaceMain, main);
        acc.add(Cat::SurfaceAux, self.mem_win.surface_bytes());
        acc.add(
            Cat::Glyphs,
            self.ui_font.glyph_cache_bytes()
                + self.mono_font.as_ref().map_or(0, |m| m.glyph_cache_bytes()),
        );
        Sample {
            at: Instant::now(),
            sys: platform::procmem::sys(),
            data: acc,
            machine: platform::sysload::sample().map(|s| (s.mem_used, s.mem_total)),
        }
    }

    /// 유휴 틱 — 창이 열려 있으면 주기마다 표본을 넣고 다음 시각을 돌려준다(닫혀 있으면 `None`).
    pub(crate) fn mem_tick(&mut self, now: Instant) -> Option<Instant> {
        if !self.mem_win.is_open() {
            return None;
        }
        if now >= self.mem_next {
            let s = self.mem_sample();
            self.mem_win.set_sample(s, MEM_REFRESH_MS);
            self.mem_next = now + Duration::from_millis(MEM_REFRESH_MS);
        }
        Some(self.mem_next)
    }

    /// [힙 정리] — 할당자가 들고 있는 빈 조각을 운영체제에 돌려주고 곧바로 새 표본(줄어든 값이 바로 보이게).
    pub(crate) fn mem_trim(&mut self) {
        let _ = platform::procmem::trim();
        self.mem_next = Instant::now();
    }
}
