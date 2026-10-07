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
        // 프로그램: UI 글꼴 파일(폴백 체인 포함 · 프로세스 수명 동안 쥔다) — 값 = **상주 페이지**(잴 수 없으면 매핑 크기).
        let ui_slices = self.ui_font.data_slices();
        let ui_mapped: u64 = ui_slices.iter().map(|d| d.len() as u64).sum();
        acc.add(
            Cat::Fonts,
            platform::procmem::resident_bytes(&ui_slices).unwrap_or(ui_mapped),
        );
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
        let mut term_mapped = 0u64;
        if let Some(m) = &self.mono_font {
            let slices = m.data_slices();
            term_mapped = slices.iter().map(|d| d.len() as u64).sum();
            acc.add(
                Cat::TermFont,
                platform::procmem::resident_bytes(&slices).unwrap_or(term_mapped),
            );
        }
        // 로그 창 버퍼(T-92).
        acc.add(Cat::Logs, self.log_win.approx_bytes());
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
        // 전용 워킹 셋(작업 관리자 "메모리" 축)은 페이지 수 비례 비용이라 전체 표본에서만 센다(사용자 10-07).
        let mut sys = platform::procmem::sys();
        sys.private_ws = platform::procmem::private_ws(&sys);
        Sample {
            at: Instant::now(),
            sys,
            data: acc,
            mapped: (ui_mapped, term_mapped),
            machine: platform::sysload::sample().map(|s| (s.mem_used, s.mem_total)),
        }
    }

    /// 메모리 덤프(기동 명령 `mem.dump:<파일>` · T6 측정 · 사용자 10-06 "dir2 대비 메모리 증가 점검"): 운영체제 값 + 영역별 어림 +
    /// 미집계(런타임 · 라이브러리). 바이트 그대로(표는 보는 쪽이 만든다).
    pub(crate) fn mem_dump(&self) -> String {
        let s = self.mem_sample();
        let mut out = String::new();
        out.push_str(&format!(
            "sys footprint {} resident {} anon {} file_backed {} compressed {} heap_used {} heap_held {} private_ws {}\n",
            s.sys.footprint,
            s.sys.resident,
            s.sys.anon,
            s.sys.file_backed,
            s.sys.compressed,
            s.sys.heap_used,
            s.sys.heap_held,
            s.sys.private_ws
        ));
        for c in Cat::ALL {
            out.push_str(&format!("{:?} {}\n", c, s.data.get(c)));
        }
        out.push_str(&format!("other {}\n", s.other()));
        out
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

    /// 유휴 트림 틱(T-179 A · dir2 M2-8 "유휴/최소화 시 작업집합 트림" 계승 · 사용자 10-06 "메모리 점유 개선"): 마지막 입력 뒤
    /// 설정 `mem.idle_trim_s`가 지나고 작업(전송 · 해시 · 터미널 출력 · 애니메이션)이 없으면 한 번 — 글리프 캐시 비움 · 셸 메뉴
    /// COM 객체 해제 · 힙 반납 · 작업 집합 반납. 다음 입력이 오면 다시 무장. 돌려주는 값 = 다음에 깨어날 시각.
    pub(crate) fn idle_trim_tick(&mut self, now: Instant, busy: bool) -> Option<Instant> {
        let now_ms = self.started.elapsed().as_millis() as u64;
        match idle_trim_plan(
            now_ms,
            self.last_input_ms,
            self.settings.int("mem.idle_trim_s"),
            self.idle_trimmed,
            busy,
        ) {
            IdleTrim::Off => None,
            IdleTrim::Wait(ms) => Some(now + Duration::from_millis(ms.max(1))),
            IdleTrim::Trim => {
                self.ui_font.clear_glyph_cache();
                if let Some(m) = &self.mono_font {
                    m.clear_glyph_cache();
                }
                self.platform.ctxmenu.release();
                platform::procmem::trim();
                platform::trim_working_set();
                self.idle_trimmed = true;
                None
            }
        }
    }

    /// [힙 정리] — 할당자가 들고 있는 빈 조각을 운영체제에 돌려주고 곧바로 새 표본(줄어든 값이 바로 보이게).
    pub(crate) fn mem_trim(&mut self) {
        let before = platform::procmem::sys().footprint;
        let us = platform::procmem::trim();
        let after = platform::procmem::sys().footprint;
        self.mem_win.set_trim_result(before, after, us);
        self.mem_next = Instant::now();
    }
}

/// 유휴 트림의 다음 행동.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdleTrim {
    /// 꺼짐(설정 0) 또는 이미 트림함 — 다음 입력까지 할 일 없음.
    Off,
    /// 아직 — 이 ms 뒤에 다시 본다.
    Wait(u64),
    /// 지금 트림.
    Trim,
}

/// 유휴 트림 판정(순수 · 조건 3개 = MC/DC 시험): 설정 ≤ 0 또는 이미 트림 = `Off` · 작업 중 = 한 주기 뒤 재확인 ·
/// 마지막 입력 뒤 `idle_s`가 안 지났으면 남은 시간만큼 `Wait` · 지났으면 `Trim`.
pub(crate) fn idle_trim_plan(
    now_ms: u64,
    last_input_ms: u64,
    idle_s: i64,
    trimmed: bool,
    busy: bool,
) -> IdleTrim {
    if idle_s <= 0 || trimmed {
        return IdleTrim::Off;
    }
    let idle_ms = (idle_s as u64).saturating_mul(1000);
    if busy {
        return IdleTrim::Wait(idle_ms);
    }
    let due = last_input_ms.saturating_add(idle_ms);
    if now_ms < due {
        IdleTrim::Wait(due - now_ms)
    } else {
        IdleTrim::Trim
    }
}

/// 사용자 입력으로 치는 창 사건(유휴 시계를 되감는다) — 키 · 마우스 · 휠 · 터치 · IME · 포커스 얻음.
pub(crate) fn is_user_input(ev: &winit::event::WindowEvent) -> bool {
    use winit::event::WindowEvent as W;
    matches!(
        ev,
        W::KeyboardInput { .. }
            | W::MouseInput { .. }
            | W::MouseWheel { .. }
            | W::CursorMoved { .. }
            | W::Touch(_)
            | W::Ime(_)
            | W::Focused(true)
    )
}

#[cfg(test)]
mod idle_tests {
    use super::*;

    /// MC/DC: 설정 0 · 이미 트림 · 작업 중 · 시간 미달 · 시간 충족 — 조건 하나씩만 바꿔 결과가 바뀜을 본다.
    #[test]
    fn idle_trim_plan_mcdc() {
        // 기준: 60 s 지남 · 트림 안 함 · 작업 없음 → Trim.
        assert_eq!(
            idle_trim_plan(70_000, 10_000, 60, false, false),
            IdleTrim::Trim
        );
        // 설정 0 → Off.
        assert_eq!(
            idle_trim_plan(70_000, 10_000, 0, false, false),
            IdleTrim::Off
        );
        // 이미 트림 → Off.
        assert_eq!(
            idle_trim_plan(70_000, 10_000, 60, true, false),
            IdleTrim::Off
        );
        // 작업 중 → 한 주기 뒤 재확인.
        assert_eq!(
            idle_trim_plan(70_000, 10_000, 60, false, true),
            IdleTrim::Wait(60_000)
        );
        // 시간 미달 → 남은 시간.
        assert_eq!(
            idle_trim_plan(50_000, 10_000, 60, false, false),
            IdleTrim::Wait(20_000)
        );
        // 경계: 정확히 지남 = Trim.
        assert_eq!(
            idle_trim_plan(70_000, 10_000, 60, false, false),
            IdleTrim::Trim
        );
        assert_eq!(
            idle_trim_plan(69_999, 10_000, 60, false, false),
            IdleTrim::Wait(1)
        );
    }
}
