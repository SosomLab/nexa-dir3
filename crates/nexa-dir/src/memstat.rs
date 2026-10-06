//! 메모리 계측 원장(T-93 · docs/22 NEW-002 · nexa-sql `memstat.rs` 구조 차용): **기능별 묶음**([`Group`]) 안의 **영역**([`Cat`]) —
//! 라벨(i18n 키) · 색 · 묶음. 수집은 호스트 `App::mem_sample` 한 곳 · 표시는 `mem_win.rs`. 운영체제 값은 `platform::procmem`.
//!
//! 영역 값은 각 부품의 **어림**이고, 운영체제가 보는 총량과의 차이는 [`Sample::other`](런타임 · 라이브러리 · 미집계)로 드러낸다.
//! 창이 닫혀 있으면 이 모듈은 불리지 않는다(비용 0).

use crate::platform::procmem::SysMem;
use std::time::Instant;

/// 기능별 묶음(사용자 10-04 분류) — 표의 구획 머리.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    /// 프로그램 실행에 필요해 올라오는 것(런타임 · 라이브러리 · 글꼴 파일).
    Program,
    /// 탭들이 쥔 파일 목록 · 작업 이력.
    Lists,
    /// 탭들이 쓰는 이미지 등 리소스.
    Resources,
    /// 터미널(화면 버퍼 · 글꼴).
    Terminal,
    /// 플러그인.
    Plugins,
    /// 화면 그리기(창 표면 · 글리프 캐시).
    Render,
}

impl Group {
    pub(crate) const ALL: [Group; 6] = [
        Group::Program,
        Group::Lists,
        Group::Resources,
        Group::Terminal,
        Group::Plugins,
        Group::Render,
    ];

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            Group::Program => "mem.grp.program",
            Group::Lists => "mem.grp.lists",
            Group::Resources => "mem.grp.resources",
            Group::Terminal => "mem.grp.terminal",
            Group::Plugins => "mem.grp.plugins",
            Group::Render => "mem.grp.render",
        }
    }
}

/// 영역 원장 — 새 캐시를 만들면 여기 한 줄 + `App::mem_sample` 한 줄.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cat {
    Fonts,
    ListsActive,
    ListsBackground,
    History,
    RowIcons,
    LauncherIcons,
    Preview,
    TermBuffer,
    TermFont,
    Plugins,
    /// 로그 창 버퍼(링 · 배치 · 필터 목록 어림 · T-92).
    Logs,
    SurfaceMain,
    SurfaceAux,
    Glyphs,
}

impl Cat {
    pub(crate) const ALL: [Cat; 14] = [
        Cat::Fonts,
        Cat::ListsActive,
        Cat::ListsBackground,
        Cat::History,
        Cat::RowIcons,
        Cat::LauncherIcons,
        Cat::Preview,
        Cat::TermBuffer,
        Cat::TermFont,
        Cat::Plugins,
        Cat::Logs,
        Cat::SurfaceMain,
        Cat::SurfaceAux,
        Cat::Glyphs,
    ];
    pub(crate) const N: usize = Self::ALL.len();

    pub(crate) fn idx(self) -> usize {
        Self::ALL.iter().position(|c| *c == self).unwrap_or(0)
    }

    /// 파일 매핑(글꼴) — 건드린 페이지만 상주하고 **Private(총량)에는 들지 않는다**(사용자 10-06). 총량 막대 · "기타" 계산에서
    /// 빼고, 비율은 상주 기준으로 보인다.
    pub(crate) fn file_backed(self) -> bool {
        matches!(self, Cat::Fonts | Cat::TermFont)
    }

    pub(crate) fn group(self) -> Group {
        match self {
            Cat::Fonts => Group::Program,
            Cat::ListsActive | Cat::ListsBackground | Cat::History => Group::Lists,
            Cat::RowIcons | Cat::LauncherIcons | Cat::Preview => Group::Resources,
            Cat::TermBuffer | Cat::TermFont => Group::Terminal,
            Cat::Plugins => Group::Plugins,
            Cat::Logs => Group::Program,
            Cat::SurfaceMain | Cat::SurfaceAux | Cat::Glyphs => Group::Render,
        }
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            Cat::Fonts => "mem.cat.fonts",
            Cat::ListsActive => "mem.cat.listsActive",
            Cat::ListsBackground => "mem.cat.listsBackground",
            Cat::History => "mem.cat.history",
            Cat::RowIcons => "mem.cat.rowIcons",
            Cat::LauncherIcons => "mem.cat.launcherIcons",
            Cat::Preview => "mem.cat.preview",
            Cat::TermBuffer => "mem.cat.termBuffer",
            Cat::TermFont => "mem.cat.termFont",
            Cat::Plugins => "mem.cat.plugins",
            Cat::Logs => "mem.cat.logs",
            Cat::SurfaceMain => "mem.cat.surfaceMain",
            Cat::SurfaceAux => "mem.cat.surfaceAux",
            Cat::Glyphs => "mem.cat.glyphs",
        }
    }

    /// 고정 팔레트(테마 무관 · 밝은/어두운 배경 모두 읽히는 중간 채도) — 같은 묶음은 같은 색조의 명도 차.
    pub(crate) fn color(self) -> (u8, u8, u8) {
        match self {
            Cat::Fonts => (0x6B, 0x7A, 0x8F),
            Cat::ListsActive => (0x3A, 0x7B, 0xD5),
            Cat::ListsBackground => (0x7F, 0xB0, 0xE6),
            Cat::History => (0xB5, 0xD2, 0xF0),
            Cat::RowIcons => (0xE0, 0x8E, 0x2B),
            Cat::LauncherIcons => (0xE8, 0xA8, 0x55),
            Cat::Preview => (0xF2, 0xC6, 0x8C),
            Cat::TermBuffer => (0x2E, 0xA0, 0x6E),
            Cat::TermFont => (0x7C, 0xC4, 0x9A),
            Cat::Plugins => (0x9B, 0x6F, 0xC9),
            Cat::Logs => (0x8A, 0x8A, 0x5C),
            Cat::SurfaceMain => (0xD9, 0x53, 0x53),
            Cat::SurfaceAux => (0xE8, 0x9A, 0x9A),
            Cat::Glyphs => (0xF2, 0xC2, 0xC2),
        }
    }

    /// "기타"(런타임 · 라이브러리 · 미집계) 색 — 영역이 아니라 총량과의 차이라 원장 밖에 둔다(표에서는 [`Group::Program`] 끝).
    pub(crate) const OTHER_COLOR: (u8, u8, u8) = (0x9A, 0xA0, 0xA6);
}

/// 영역 누적기.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Acc {
    bytes: [u64; Cat::N],
}

impl Acc {
    pub(crate) fn add(&mut self, cat: Cat, n: u64) {
        self.bytes[cat.idx()] = self.bytes[cat.idx()].saturating_add(n);
    }
    pub(crate) fn get(&self, cat: Cat) -> u64 {
        self.bytes[cat.idx()]
    }
    #[cfg(test)]
    pub(crate) fn sum(&self) -> u64 {
        self.bytes.iter().fold(0u64, |a, b| a.saturating_add(*b))
    }
    /// Private에 드는 영역만의 합(파일 매핑 제외).
    pub(crate) fn private_sum(&self) -> u64 {
        Cat::ALL
            .iter()
            .filter(|c| !c.file_backed())
            .fold(0u64, |a, c| a.saturating_add(self.get(*c)))
    }
    /// 묶음 소계.
    pub(crate) fn group_sum(&self, g: Group) -> u64 {
        Cat::ALL
            .iter()
            .filter(|c| c.group() == g)
            .fold(0u64, |a, c| a.saturating_add(self.get(*c)))
    }
}

/// 한 번의 표본.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    pub at: Instant,
    pub sys: SysMem,
    pub data: Acc,
    /// 시스템 전체 메모리 `(쓰는 양, 전체)`(모르면 `None`).
    pub machine: Option<(u64, u64)>,
    /// 글꼴 파일 매핑 크기 `(UI, 터미널)` — 영역 값은 상주 몫이고 이것은 파일 전체(표시용).
    pub mapped: (u64, u64),
}

impl Sample {
    /// 총량(풋프린트) − Private에 드는 영역 합(포화) = 런타임 · 라이브러리 · 미집계. 파일 매핑(글꼴)은 Private 밖이라 빼지 않는다
    /// (종전 = 글꼴 파일 크기까지 빼 "기타 0 B" · 비율 110 %로 보였다 · 사용자 10-06).
    pub(crate) fn other(&self) -> u64 {
        self.sys.footprint.saturating_sub(self.data.private_sum())
    }
}

/// 변화 표시가 남는 표본 수(바뀐 뒤 이만큼의 표본 동안 ▲/▼를 보여 준다).
pub(crate) const TREND_HOLD: u8 = 6;
/// "기타"는 운영체제 값의 차이라 늘 조금씩 흔들린다 — 이보다 작은 변화는 표시하지 않는다.
const OTHER_NOISE: u64 = 64 * 1024;

/// 영역별 **늘고 주는 과정**(사용자 10-04): 직전 표본과의 차이를 영역마다 기억해 [`TREND_HOLD`] 표본 동안 보여 준다.
/// 칸 = 영역 [`Cat::N`]개 + 마지막 한 칸("기타").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Trend {
    last: Option<[u64; Cat::N + 1]>,
    delta: [i64; Cat::N + 1],
    ttl: [u8; Cat::N + 1],
}

impl Trend {
    /// "기타" 칸 번호.
    pub(crate) const OTHER: usize = Cat::N;

    fn values(s: &Sample) -> [u64; Cat::N + 1] {
        let mut v = [0u64; Cat::N + 1];
        for c in Cat::ALL {
            v[c.idx()] = s.data.get(c);
        }
        v[Self::OTHER] = s.other();
        v
    }

    /// 새 표본을 받는다 — 바뀐 칸은 차이를 새로 적고, 안 바뀐 칸은 남은 표시 시간을 하나 줄인다.
    pub(crate) fn update(&mut self, s: &Sample) {
        let now = Self::values(s);
        if let Some(prev) = self.last {
            for i in 0..=Cat::N {
                let d = now[i] as i64 - prev[i] as i64;
                let noise = i == Self::OTHER && d.unsigned_abs() < OTHER_NOISE;
                if d != 0 && !noise {
                    self.delta[i] = d;
                    self.ttl[i] = TREND_HOLD;
                } else if self.ttl[i] > 0 {
                    self.ttl[i] -= 1;
                }
            }
        }
        self.last = Some(now);
    }

    /// 칸 `i`의 최근 변화(표시 시간이 남아 있을 때만 · 양수 = 늘었다).
    pub(crate) fn shown(&self, i: usize) -> Option<i64> {
        (self.ttl.get(i).copied().unwrap_or(0) > 0).then(|| self.delta[i])
    }
}

/// 바이트 표기 — 1024 단위 · 유효숫자 3(`312 MB` · `1.24 GB` · `640 KB`).
pub(crate) fn fmt(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else if v >= 100.0 {
        format!("{v:.0} {}", U[i])
    } else if v >= 10.0 {
        format!("{v:.1} {}", U[i])
    } else {
        format!("{v:.2} {}", U[i])
    }
}

/// 변화 표기 — `▲ 1.20 MB` / `▼ 300 KB`.
pub(crate) fn fmt_delta(d: i64) -> String {
    let arrow = if d >= 0 { "▲" } else { "▼" };
    format!("{arrow} {}", fmt(d.unsigned_abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(footprint: u64, parts: &[(Cat, u64)]) -> Sample {
        let mut data = Acc::default();
        for (c, n) in parts {
            data.add(*c, *n);
        }
        Sample {
            at: Instant::now(),
            sys: SysMem {
                footprint,
                ..SysMem::default()
            },
            data,
            machine: None,
            mapped: (0, 0),
        }
    }

    #[test]
    fn fmt_units_and_precision() {
        assert_eq!(fmt(0), "0 B");
        assert_eq!(fmt(999), "999 B");
        assert_eq!(fmt(1536), "1.50 KB");
        assert_eq!(fmt(12 * 1024 * 1024 + 300 * 1024), "12.3 MB");
        assert_eq!(fmt(312 * 1024 * 1024), "312 MB");
        assert_eq!(fmt_delta(2048), "▲ 2.00 KB");
        assert_eq!(fmt_delta(-300 * 1024), "▼ 300 KB");
    }

    /// 원장 무결성: 영역마다 묶음 · 라벨 · 색이 있고 색은 서로 다르며 "기타" 색과도 다르다 · 모든 묶음에 영역이 있거나(프로그램은
    /// 글꼴 + 기타) · 소계 합 = 전체 합.
    #[test]
    fn ledger_is_consistent() {
        let mut colors: Vec<(u8, u8, u8)> = Cat::ALL.iter().map(|c| c.color()).collect();
        colors.push(Cat::OTHER_COLOR);
        let n = colors.len();
        colors.sort_unstable();
        colors.dedup();
        assert_eq!(colors.len(), n, "색 중복");
        for g in Group::ALL {
            assert!(Cat::ALL.iter().any(|c| c.group() == g), "{g:?}");
        }
        let mut acc = Acc::default();
        for (i, c) in Cat::ALL.iter().enumerate() {
            acc.add(*c, (i as u64 + 1) * 10);
        }
        let by_group: u64 = Group::ALL.iter().map(|g| acc.group_sum(*g)).sum();
        assert_eq!(by_group, acc.sum());
    }

    /// 기타 = 총량 − 영역 합(음수 없음) · 변화: 늘면 ▲ · 줄면 ▼ · 그대로면 표시 시간이 줄다가 사라진다 · 기타의 잔물결은 무시.
    #[test]
    fn other_never_underflows_and_trend_tracks_grow_and_shrink() {
        let s = sample(100, &[(Cat::ListsActive, 30), (Cat::SurfaceMain, 50)]);
        assert_eq!(s.other(), 20);
        assert_eq!(sample(60, &[(Cat::ListsActive, 90)]).other(), 0);

        let mb = 1024 * 1024;
        let mut t = Trend::default();
        let i = Cat::ListsActive.idx();
        t.update(&sample(100 * mb, &[(Cat::ListsActive, mb)]));
        assert_eq!(t.shown(i), None, "첫 표본 = 비교할 것 없음");
        t.update(&sample(100 * mb, &[(Cat::ListsActive, 3 * mb)]));
        assert_eq!(t.shown(i), Some(2 * mb as i64), "늘었다");
        t.update(&sample(100 * mb, &[(Cat::ListsActive, mb)]));
        assert_eq!(t.shown(i), Some(-2 * (mb as i64)), "줄었다");
        for _ in 0..TREND_HOLD {
            assert!(t.shown(i).is_some());
            t.update(&sample(100 * mb, &[(Cat::ListsActive, mb)]));
        }
        assert_eq!(t.shown(i), None, "그대로면 사라진다");
        // 기타: 1 KiB 흔들림은 무시 · 1 MiB 변화는 표시.
        t.update(&sample(100 * mb + 1024, &[(Cat::ListsActive, mb)]));
        assert_eq!(t.shown(Trend::OTHER), None);
        t.update(&sample(101 * mb + 1024, &[(Cat::ListsActive, mb)]));
        assert_eq!(t.shown(Trend::OTHER), Some(mb as i64));
    }
}
