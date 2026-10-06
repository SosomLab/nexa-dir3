//! `ndir-log` — 앱 로그(T-92 · docs/22 NEW-001 · DR-15 · nexa-sql `nsql-log` 이식 · 사용자 10-04 "로그 확인 방법"):
//! 탐색기 단계(목록 열기 · 열기/실행 · 셸 메뉴 · 전송 작업 · 플러그인 · 폴더 감시 · 기동)를 시각 첫 컬럼으로 · 개발자 모드 상세 로그.
//!
//! 구조(어댑터):
//! ```text
//! 생산자(열람 · 작업 · 셸 · 플러그인 · 감시 · 기동) ──▶ LogEntry{ts, kind, items, elapsed, message, layer, level} ──▶ LogBuffer(링)
//!                                                              ▼  trait LogFormat (교체 지점)
//!                                            RawFormat · Markdown · Grid · Compact · Jsonl · Csv/Tsv · Template
//!                                                              ▼
//!                                            소비자(로그 창 · `log.dump` · 클립보드 · "파일로 저장"은 창의 대화상자)
//! ```
//! - 입력 구조 하나([`LogEntry`]) · 출력 구조 하나(`String` 줄 + 선택적 헤더) — 포맷은 [`LogFormat`] 구현을 바꿔 끼운다.
//! - 타임스탬프 = 엔트리 생성 시각(로컬 · [`now_local`] · 외부 crate 0 · Windows `GetLocalTime` · Unix `localtime_r`).
//! - 의존 0 · UI 0 · **파일 I/O 0**(DR-15 — nsql-log의 파일 싱크 · 허브 · stderr 싱크는 가져오지 않는다 · 저장은 창이 한 번에).
//! - 상세 로그 게이트(`wants`)는 꺼져 있으면 원자 load 1 + 분기 1 — `dlog!`(호스트)가 인자를 평가하지 않는다.

#![cfg_attr(test, allow(clippy::unwrap_used))]

use std::collections::VecDeque;
use std::time::Duration;

/// 로그 구분 — 사용자 관점 탐색기 단계 + 정보·오류.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogKind {
    /// 목록 열기(폴더 열람 · 항목 수 · 소요).
    List,
    /// 열기/실행(파일 · 런처 · 터미널).
    Open,
    /// 셸 메뉴(구축 · 실행).
    Shell,
    /// 전송 작업(복사 · 이동 · 삭제 · 이름 바꾸기 · 압축).
    Ops,
    /// 플러그인(적재 · 오류 · 노트).
    Plugin,
    /// 폴더 감시(변경 통지 · 재열람).
    Watch,
    /// 기동 단계(첫 그리기 · 세션 복원).
    Startup,
    /// 완료(총 소요 · 항목 수).
    Done,
    Error,
    Info,
}

impl LogKind {
    /// 전체(표시 필터 메뉴 순서).
    pub const ALL: [LogKind; 10] = [
        LogKind::List,
        LogKind::Open,
        LogKind::Shell,
        LogKind::Ops,
        LogKind::Plugin,
        LogKind::Watch,
        LogKind::Startup,
        LogKind::Done,
        LogKind::Error,
        LogKind::Info,
    ];

    /// 라벨 → 종류(설정 `log.kinds` 파싱 · 대소문자 무관).
    pub fn parse(label: &str) -> Option<LogKind> {
        let l = label.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|k| k.label().eq_ignore_ascii_case(&l))
    }

    /// 고정 폭 라벨(Grid/Raw 정렬용 · 번역하지 않는다 · ≤ 7자).
    pub fn label(self) -> &'static str {
        match self {
            LogKind::List => "list",
            LogKind::Open => "open",
            LogKind::Shell => "shell",
            LogKind::Ops => "ops",
            LogKind::Plugin => "plugin",
            LogKind::Watch => "watch",
            LogKind::Startup => "startup",
            LogKind::Done => "done",
            LogKind::Error => "ERROR",
            LogKind::Info => "info",
        }
    }
}

/// 로컬 시각(밀리초).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LocalTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub ms: u32,
}

impl LocalTime {
    /// `HH:MM:SS.mmm`(12자 · compact/템플릿 `{time}`).
    pub fn time_only(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            self.hour, self.min, self.sec, self.ms
        )
    }

    /// `YYYY-MM-DD`.
    pub fn date_only(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// `YYYY-MM-DD HH:MM:SS.mmm`(23자 고정 — 첫 컬럼 정렬).
    pub fn stamp(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            self.year, self.month, self.day, self.hour, self.min, self.sec, self.ms
        )
    }
}

/// 지금(로컬). 실패하면 UTC.
pub fn now_local() -> LocalTime {
    imp::now_local().unwrap_or_else(now_utc)
}

/// UTC(폴백 · 시간대 조회 실패 시). Howard Hinnant civil_from_days.
pub fn now_utc() -> LocalTime {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    LocalTime {
        year: (if m <= 2 { y + 1 } else { y }) as i32,
        month: m as u32,
        day: day as u32,
        hour: rem / 3600,
        min: (rem % 3600) / 60,
        sec: rem % 60,
        ms: d.subsec_millis(),
    }
}

#[cfg(windows)]
mod imp {
    use super::LocalTime;
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(out: *mut SystemTime);
    }
    pub(super) fn now_local() -> Option<LocalTime> {
        let mut st = SystemTime::default();
        // SAFETY: 출력 구조체 포인터만 넘긴다.
        unsafe { GetLocalTime(&mut st) };
        Some(LocalTime {
            year: i32::from(st.year),
            month: u32::from(st.month),
            day: u32::from(st.day),
            hour: u32::from(st.hour),
            min: u32::from(st.minute),
            sec: u32::from(st.second),
            ms: u32::from(st.millis),
        })
    }
}

#[cfg(unix)]
mod imp {
    use super::LocalTime;
    /// `struct tm`의 앞 9개 int는 glibc·musl·macOS 공통 배치(뒤의 gmtoff/zone은 읽지 않는다).
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        _gmtoff: i64,
        _zone: *const u8,
    }
    extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
    pub(super) fn now_local() -> Option<LocalTime> {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let t = d.as_secs() as i64;
        let mut tm = Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            _gmtoff: 0,
            _zone: std::ptr::null(),
        };
        // SAFETY: time_t 포인터와 충분히 큰 출력 구조체(선두 9 int + gmtoff + zone 포인터).
        let r = unsafe { localtime_r(&t, &mut tm) };
        if r.is_null() {
            return None;
        }
        Some(LocalTime {
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            hour: tm.tm_hour as u32,
            min: tm.tm_min as u32,
            sec: tm.tm_sec as u32,
            ms: d.subsec_millis(),
        })
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    pub(super) fn now_local() -> Option<super::LocalTime> {
        None
    }
}

/// 로그 한 줄 — **입력 구조의 단일 원천**(모든 포맷·싱크가 이것만 받는다).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub ts: LocalTime,
    pub kind: LogKind,
    /// 항목 수(목록 열기 · 작업 완료) · 줄 수(플러그인 노트).
    pub items: Option<u64>,
    /// 구간 소요.
    pub elapsed: Option<Duration>,
    pub message: String,
    /// 처리 층(개발자 모드 상세 로그 · docs/48) — 기본 메시지는 `App`.
    pub layer: LogLayer,
    /// 상세 수준 — `Basic`은 늘 보이는 기본 메시지 · 나머지는 개발자 모드 + 마스크가 켜져 있을 때만 **생성**된다.
    pub level: LogLevel,
}

/// 처리 층(DR-15) — 상세 로그 마스크의 축. 순서 = 마스크 비트 그룹(층마다 4비트 · 최대 16층).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LogLayer {
    App = 0,
    /// 파일 시스템(열거 · 메타 조회 · 가상 루트).
    Vfs = 1,
    /// 목록/트리 모델(정렬 · 필터 · 선택).
    Tree = 2,
    /// 화면 그리기(프레임 · 소요).
    Render = 3,
    /// 셸 연동(컨텍스트 메뉴 · 아이콘 · 열기).
    Shell = 4,
    /// 전송 작업 엔진(복사 · 이동 · 삭제 · 해시).
    Ops = 5,
    /// 플러그인 런타임(WASM 적재 · 호출).
    Plugin = 6,
    /// 폴더 감시.
    Watch = 7,
    /// 터미널(PTY · 출력).
    Term = 8,
    /// 기동(단계별 시각).
    Startup = 9,
}

impl LogLayer {
    pub const ALL: [LogLayer; 10] = [
        LogLayer::App,
        LogLayer::Vfs,
        LogLayer::Tree,
        LogLayer::Render,
        LogLayer::Shell,
        LogLayer::Ops,
        LogLayer::Plugin,
        LogLayer::Watch,
        LogLayer::Term,
        LogLayer::Startup,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LogLayer::App => "app",
            LogLayer::Vfs => "vfs",
            LogLayer::Tree => "tree",
            LogLayer::Render => "render",
            LogLayer::Shell => "shell",
            LogLayer::Ops => "ops",
            LogLayer::Plugin => "plugin",
            LogLayer::Watch => "watch",
            LogLayer::Term => "term",
            LogLayer::Startup => "startup",
        }
    }
    pub fn parse(s: &str) -> Option<LogLayer> {
        let l = s.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|k| k.label() == l)
    }
}

/// 상세 수준(docs/48 §2) — 층마다 3비트(timing · progress · trace).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LogLevel {
    Basic = 0,
    /// 단계 시각·소요·전송량(실행당 몇 줄).
    Timing = 1,
    /// 진행(페치 배치 · 100ms 간격).
    Progress = 2,
    /// 추적(호출 단위 · 많음).
    Trace = 3,
}

impl LogLevel {
    pub const DETAIL: [LogLevel; 3] = [LogLevel::Timing, LogLevel::Progress, LogLevel::Trace];
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Basic => "basic",
            LogLevel::Timing => "timing",
            LogLevel::Progress => "progress",
            LogLevel::Trace => "trace",
        }
    }
    pub fn parse(s: &str) -> Option<LogLevel> {
        let l = s.trim().to_ascii_lowercase();
        Self::DETAIL.into_iter().find(|k| k.label() == l)
    }
}

/// ★ 상세 로그 게이트(docs/48 §3) — 층×수준 비트 하나의 원자 정수. 꺼져 있으면(0) 상세 로그 코드는 **load 1회 + 예측 가능한
/// 분기 1개**만 남고 문자열·시각·할당은 전혀 일어나지 않는다. 쓰기는 설정 변경 때만(`Relaxed`로 충분 — 순서 보장 불필요).
static DETAIL_MASK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[inline(always)]
const fn detail_bit(layer: LogLayer, level: LogLevel) -> u64 {
    1u64 << ((layer as u64) * 4 + (level as u64))
}

/// 이 층·수준의 상세 로그를 만들어야 하는가 — **항상 인라인**(호출 0 · 분기 1).
#[inline(always)]
pub fn wants(layer: LogLayer, level: LogLevel) -> bool {
    #[cfg(feature = "devlog")]
    {
        DETAIL_MASK.load(std::sync::atomic::Ordering::Relaxed) & detail_bit(layer, level) != 0
    }
    #[cfg(not(feature = "devlog"))]
    {
        let _ = (layer, level);
        false
    }
}

pub fn set_detail_mask(mask: u64) {
    DETAIL_MASK.store(mask, std::sync::atomic::Ordering::Relaxed);
}

pub fn detail_mask() -> u64 {
    DETAIL_MASK.load(std::sync::atomic::Ordering::Relaxed)
}

/// 설정 `log.dev_layers` → 마스크. 문법: `layer[:level+level]`을 쉼표로 · 수준 생략 = timing+progress+trace ·
/// `*` = 전 층 전 수준 · 빈 문자열 = 0. 예 `net,fetch:timing+progress,render:timing`.
pub fn parse_detail_layers(spec: &str) -> u64 {
    let mut m = 0u64;
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (layer_s, levels_s) = match part.split_once(':') {
            Some((a, b)) => (a, Some(b)),
            None => (part, None),
        };
        let layers: Vec<LogLayer> = if layer_s.trim() == "*" {
            LogLayer::ALL.to_vec()
        } else {
            LogLayer::parse(layer_s).into_iter().collect()
        };
        let levels: Vec<LogLevel> = match levels_s {
            Some(ls) => ls.split('+').filter_map(LogLevel::parse).collect(),
            None => LogLevel::DETAIL.to_vec(),
        };
        for l in &layers {
            for v in &levels {
                m |= detail_bit(*l, *v);
            }
        }
    }
    m
}

/// 마스크에 층이 하나라도 켜져 있는가(메뉴 체크 표시).
pub fn layer_in_mask(mask: u64, layer: LogLayer) -> bool {
    LogLevel::DETAIL
        .iter()
        .any(|v| mask & detail_bit(layer, *v) != 0)
}

impl LogEntry {
    pub fn new(kind: LogKind, message: impl Into<String>) -> Self {
        LogEntry {
            ts: now_local(),
            kind,
            items: None,
            elapsed: None,
            message: message.into(),
            layer: LogLayer::App,
            level: LogLevel::Basic,
        }
    }
    pub fn items(mut self, n: impl Into<Option<u64>>) -> Self {
        self.items = n.into();
        self
    }
    pub fn elapsed(mut self, d: impl Into<Option<Duration>>) -> Self {
        self.elapsed = d.into();
        self
    }
    /// 상세 로그 표식(층 · 수준) — 메시지 앞에 `⟨net⟩`.
    pub fn at(mut self, layer: LogLayer, level: LogLevel) -> Self {
        self.layer = layer;
        self.level = level;
        if layer != LogLayer::App {
            self.message = format!("⟨{}⟩ {}", layer.label(), self.message);
        }
        self
    }
}

/// `1.234s` / `12.3ms` / `456µs`.
pub fn fmt_dur(d: Duration) -> String {
    let us = d.as_micros();
    if us >= 1_000_000 {
        format!("{:.3}s", d.as_secs_f64())
    } else if us >= 1_000 {
        format!("{:.1}ms", us as f64 / 1000.0)
    } else {
        format!("{us}µs")
    }
}

/// 출력 어댑터 — **교체 지점**. 새 포맷 = 이 트레이트 구현 1개 + [`formatter`] 표 1줄.
pub trait LogFormat {
    /// 설정 값·표시 이름(`raw` · `markdown` · `grid` · `compact` · `jsonl` · `csv` · `tsv` · `template`).
    fn name(&self) -> &'static str;
    /// 맨 위 헤더 줄(없으면 `None`).
    fn header(&self) -> Option<String> {
        None
    }
    /// 엔트리 한 줄.
    fn line(&self, e: &LogEntry) -> String;
    /// 표시 컬럼 마스크(텍스트 계열만 반영 · 구조형은 무시). 기본 no-op.
    fn set_columns(&mut self, _c: Columns) {}
}

/// 표시 컬럼 마스크(설정 `log.columns` · 로그 창 메뉴) — `message`는 늘 보인다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub ts: bool,
    pub kind: bool,
    pub items: bool,
    pub elapsed: bool,
}

impl Default for Columns {
    fn default() -> Self {
        Columns {
            ts: true,
            kind: true,
            items: true,
            elapsed: true,
        }
    }
}

impl Columns {
    pub const NAMES: [&'static str; 4] = ["time", "kind", "items", "elapsed"];

    /// `time,kind,items,elapsed` 중 보일 것(빈 문자열 = 전부).
    pub fn parse(s: &str) -> Columns {
        let s = s.trim();
        if s.is_empty() {
            return Columns::default();
        }
        let has = |n: &str| s.split(',').any(|p| p.trim().eq_ignore_ascii_case(n));
        Columns {
            ts: has("time"),
            kind: has("kind"),
            items: has("items"),
            elapsed: has("elapsed"),
        }
    }

    /// 설정 문자열(전부면 빈 문자열).
    pub fn to_setting(self) -> String {
        if self == Columns::default() {
            return String::new();
        }
        let mut v = Vec::new();
        if self.ts {
            v.push("time");
        }
        if self.kind {
            v.push("kind");
        }
        if self.items {
            v.push("items");
        }
        if self.elapsed {
            v.push("elapsed");
        }
        v.join(",")
    }

    fn get(self, i: usize) -> bool {
        match i {
            0 => self.ts,
            1 => self.kind,
            2 => self.items,
            3 => self.elapsed,
            _ => true,
        }
    }

    pub fn toggle(&mut self, name: &str) {
        match name {
            "time" => self.ts = !self.ts,
            "kind" => self.kind = !self.kind,
            "items" => self.items = !self.items,
            "elapsed" => self.elapsed = !self.elapsed,
            _ => {}
        }
    }
}

/// 컬럼 값(모든 포맷이 같은 순서로 쓴다 — ts · kind · items · elapsed · message).
fn cells(e: &LogEntry) -> [String; 5] {
    [
        e.ts.stamp(),
        e.kind.label().to_string(),
        e.items.map(|r| r.to_string()).unwrap_or_default(),
        e.elapsed.map(fmt_dur).unwrap_or_default(),
        e.message.replace('\n', " "),
    ]
}

/// 마스크 적용(숨긴 컬럼은 빈 문자열).
fn masked(e: &LogEntry, c: Columns) -> [String; 5] {
    let mut v = cells(e);
    for (i, cell) in v.iter_mut().enumerate().take(4) {
        if !c.get(i) {
            cell.clear();
        }
    }
    v
}

/// Raw(기본): `2026-10-06 15:41:22.123  list     500 items  34.1ms  D:\\Projects`.
#[derive(Debug, Default, Clone, Copy)]
pub struct RawFormat {
    pub cols: Columns,
}

impl LogFormat for RawFormat {
    fn name(&self) -> &'static str {
        "raw"
    }
    fn set_columns(&mut self, c: Columns) {
        self.cols = c;
    }
    fn line(&self, e: &LogEntry) -> String {
        let [ts, kind, items, el, msg] = masked(e, self.cols);
        let mut s = String::new();
        if self.cols.ts {
            s.push_str(&ts);
        }
        if self.cols.kind {
            if !s.is_empty() {
                s.push_str("  ");
            }
            s.push_str(&format!("{kind:<8}"));
        }
        if !items.is_empty() {
            s.push_str(&format!("  {items} items"));
        }
        if !el.is_empty() {
            s.push_str(&format!("  {el}"));
        }
        if !msg.is_empty() {
            s.push_str("  ");
            s.push_str(&msg);
        }
        s
    }
}

/// Markdown 표.
#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownFormat {
    pub cols: Columns,
}

impl LogFormat for MarkdownFormat {
    fn name(&self) -> &'static str {
        "markdown"
    }
    fn set_columns(&mut self, c: Columns) {
        self.cols = c;
    }
    fn header(&self) -> Option<String> {
        let mut h = String::from("|");
        let mut r = String::from("|");
        for (i, n) in Columns::NAMES.iter().enumerate() {
            if self.cols.get(i) {
                h.push_str(&format!(" {n} |"));
                r.push_str(if i >= 2 { "--:|" } else { "---|" });
            }
        }
        h.push_str(" message |");
        r.push_str("---|");
        Some(format!("{h}\n{r}"))
    }
    fn line(&self, e: &LogEntry) -> String {
        let v = cells(e);
        let mut s = String::from("|");
        for (i, cell) in v.iter().enumerate().take(4) {
            if self.cols.get(i) {
                s.push_str(&format!(" {cell} |"));
            }
        }
        s.push_str(&format!(" {} |", v[4].replace('|', "\\|")));
        s
    }
}

/// Grid: 고정 폭 컬럼(그리드 컨트롤이 오기 전의 텍스트 정렬판 — 컬럼 경계 `│`).
#[derive(Debug, Default, Clone, Copy)]
pub struct GridFormat {
    pub cols: Columns,
}

impl GridFormat {
    pub const WIDTHS: [usize; 4] = [23, 8, 8, 9];

    fn row(&self, v: [String; 5]) -> String {
        let w = Self::WIDTHS;
        let mut s = String::new();
        for i in 0..4 {
            if !self.cols.get(i) {
                continue;
            }
            let cell = &v[i];
            if i >= 2 {
                s.push_str(&format!("{cell:>w$}│", w = w[i]));
            } else {
                s.push_str(&format!("{cell:<w$}│", w = w[i]));
            }
        }
        s.push_str(&v[4]);
        s
    }
}

impl LogFormat for GridFormat {
    fn name(&self) -> &'static str {
        "grid"
    }
    fn set_columns(&mut self, c: Columns) {
        self.cols = c;
    }
    fn header(&self) -> Option<String> {
        Some(self.row([
            "time".into(),
            "kind".into(),
            "items".into(),
            "elapsed".into(),
            "message".into(),
        ]))
    }
    fn line(&self, e: &LogEntry) -> String {
        self.row(cells(e))
    }
}

/// Compact: `15:41:22.123 list     D:\\Projects`(시각만 · items/elapsed는 뒤에 괄호).
#[derive(Debug, Default, Clone, Copy)]
pub struct CompactFormat {
    pub cols: Columns,
}

impl LogFormat for CompactFormat {
    fn name(&self) -> &'static str {
        "compact"
    }
    fn set_columns(&mut self, c: Columns) {
        self.cols = c;
    }
    fn line(&self, e: &LogEntry) -> String {
        let [_, kind, items, el, msg] = cells(e);
        let mut s = String::new();
        if self.cols.ts {
            s.push_str(&e.ts.time_only());
            s.push(' ');
        }
        if self.cols.kind {
            s.push_str(&format!("{kind:<7} "));
        }
        s.push_str(&msg);
        let mut tail = Vec::new();
        if self.cols.items && !items.is_empty() {
            tail.push(format!("{items} items"));
        }
        if self.cols.elapsed && !el.is_empty() {
            tail.push(el);
        }
        if !tail.is_empty() {
            s.push_str(&format!(" ({})", tail.join(" · ")));
        }
        s
    }
}

/// JSON Lines: 줄마다 객체(`ts` · `kind` · `items` · `elapsed_ms` · `message`) — 도구 연계용.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonlFormat;

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

impl LogFormat for JsonlFormat {
    fn name(&self) -> &'static str {
        "jsonl"
    }
    fn line(&self, e: &LogEntry) -> String {
        format!(
            "{{\"ts\":{},\"kind\":{},\"items\":{},\"elapsed_ms\":{},\"message\":{}}}",
            json_str(&e.ts.stamp()),
            json_str(e.kind.label()),
            e.items.map_or("null".to_string(), |r| r.to_string()),
            e.elapsed.map_or("null".to_string(), |d| format!(
                "{:.1}",
                d.as_secs_f64() * 1000.0
            )),
            json_str(&e.message)
        )
    }
}

/// CSV/TSV(RFC 4180 따옴표 · 헤더 1줄).
#[derive(Debug, Clone, Copy)]
pub struct DelimitedFormat {
    pub delim: char,
}

impl DelimitedFormat {
    fn q(&self, s: &str) -> String {
        if s.contains(self.delim) || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
}

impl LogFormat for DelimitedFormat {
    fn name(&self) -> &'static str {
        if self.delim == '\t' {
            "tsv"
        } else {
            "csv"
        }
    }
    fn header(&self) -> Option<String> {
        Some(["time", "kind", "items", "elapsed", "message"].join(&self.delim.to_string()))
    }
    fn line(&self, e: &LogEntry) -> String {
        cells(e)
            .iter()
            .map(|c| self.q(c))
            .collect::<Vec<_>>()
            .join(&self.delim.to_string())
    }
}

/// 사용자 템플릿: `{ts}` · `{time}` · `{date}` · `{kind}` · `{items}` · `{elapsed}` · `{msg}` + 폭 `{kind:<8}`/`{items:>6}`.
#[derive(Debug, Clone)]
pub struct TemplateFormat {
    pub template: String,
}

/// 기본 템플릿(설정 `log.template`).
pub const DEFAULT_TEMPLATE: &str = "{time} {kind:<8} {msg}";

impl TemplateFormat {
    fn value(e: &LogEntry, key: &str) -> Option<String> {
        Some(match key {
            "ts" => e.ts.stamp(),
            "time" => e.ts.time_only(),
            "date" => e.ts.date_only(),
            "kind" => e.kind.label().to_string(),
            "items" => e.items.map(|r| r.to_string()).unwrap_or_default(),
            "elapsed" => e.elapsed.map(fmt_dur).unwrap_or_default(),
            "msg" | "message" => e.message.replace('\n', " "),
            _ => return None,
        })
    }
}

impl LogFormat for TemplateFormat {
    fn name(&self) -> &'static str {
        "template"
    }
    fn line(&self, e: &LogEntry) -> String {
        let t = &self.template;
        let mut out = String::with_capacity(t.len() + 64);
        let mut rest = t.as_str();
        while let Some(i) = rest.find('{') {
            out.push_str(&rest[..i]);
            let Some(j) = rest[i..].find('}') else {
                out.push_str(&rest[i..]);
                rest = "";
                break;
            };
            let spec = &rest[i + 1..i + j];
            let (key, pad) = spec.split_once(':').unwrap_or((spec, ""));
            match Self::value(e, key.trim()) {
                Some(v) => {
                    let (align, n) = if let Some(n) = pad.strip_prefix('<') {
                        ('<', n.parse::<usize>().unwrap_or(0))
                    } else if let Some(n) = pad.strip_prefix('>') {
                        ('>', n.parse::<usize>().unwrap_or(0))
                    } else {
                        ('<', 0)
                    };
                    let w = v.chars().count();
                    if align == '>' && w < n {
                        out.push_str(&" ".repeat(n - w));
                    }
                    out.push_str(&v);
                    if align == '<' && w < n {
                        out.push_str(&" ".repeat(n - w));
                    }
                }
                None => {
                    out.push('{');
                    out.push_str(spec);
                    out.push('}');
                }
            }
            rest = &rest[i + j + 1..];
        }
        out.push_str(rest);
        out
    }
}

/// 이름 → 포맷(설정 `log.format`). 모르면 Raw.
pub fn formatter(name: &str) -> Box<dyn LogFormat> {
    formatter_with(name, DEFAULT_TEMPLATE, Columns::default())
}

/// 이름 + 템플릿 + 컬럼 마스크 → 포맷.
pub fn formatter_with(name: &str, template: &str, cols: Columns) -> Box<dyn LogFormat + Send> {
    let mut f: Box<dyn LogFormat + Send> = match name.trim().to_ascii_lowercase().as_str() {
        "markdown" | "md" => Box::new(MarkdownFormat::default()),
        "grid" => Box::new(GridFormat::default()),
        "compact" => Box::new(CompactFormat::default()),
        "jsonl" | "json" => Box::new(JsonlFormat),
        "csv" => Box::new(DelimitedFormat { delim: ',' }),
        "tsv" => Box::new(DelimitedFormat { delim: '\t' }),
        "template" | "custom" => Box::new(TemplateFormat {
            template: if template.trim().is_empty() {
                DEFAULT_TEMPLATE.to_string()
            } else {
                template.to_string()
            },
        }),
        _ => Box::new(RawFormat::default()),
    };
    f.set_columns(cols);
    f
}

/// 포맷 이름 목록(설정 후보).
pub const FORMAT_NAMES: [&str; 8] = [
    "raw", "markdown", "grid", "compact", "jsonl", "csv", "tsv", "template",
];

/// 링 버퍼(상한 초과 시 오래된 것부터 버림 — 메모리 상시 상한 · UI 스레드 전용 · 잠금 없음).
#[derive(Debug)]
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    cap: usize,
}

impl LogBuffer {
    pub fn new(cap: usize) -> Self {
        LogBuffer {
            entries: VecDeque::with_capacity(cap.min(1024)),
            cap: cap.max(1),
        }
    }
    pub fn push(&mut self, e: LogEntry) {
        if self.entries.len() >= self.cap {
            self.entries.pop_front();
        }
        self.entries.push_back(e);
    }
    /// 상한(줄 수).
    pub fn cap(&self) -> usize {
        self.cap
    }
    /// 상한을 바꾼다(0은 1로 · 설정 `log.max_lines` · docs/39 T-90d) — 넘치는 만큼 **앞(오래된 것)에서 즉시 버리고** 버린 수를 돌려준다.
    pub fn set_cap(&mut self, cap: usize) -> usize {
        self.cap = cap.max(1);
        let drop_n = self.entries.len().saturating_sub(self.cap);
        for _ in 0..drop_n {
            self.entries.pop_front();
        }
        drop_n
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }
    pub fn get(&self, i: usize) -> Option<&LogEntry> {
        self.entries.get(i)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 마스크 0 = 아무 층도 원하지 않음 · 문법 파싱 · 층 체크.
    #[test]
    fn detail_mask_parse_and_gate() {
        set_detail_mask(0);
        assert!(!wants(LogLayer::Vfs, LogLevel::Timing));
        let m = parse_detail_layers("vfs, ops:timing+progress ,render:timing,bogus");
        set_detail_mask(m);
        assert!(wants(LogLayer::Vfs, LogLevel::Trace));
        assert!(wants(LogLayer::Ops, LogLevel::Progress));
        assert!(!wants(LogLayer::Ops, LogLevel::Trace));
        assert!(wants(LogLayer::Render, LogLevel::Timing));
        assert!(!wants(LogLayer::Shell, LogLevel::Timing));
        assert!(layer_in_mask(m, LogLayer::Render) && !layer_in_mask(m, LogLayer::Shell));
        assert_eq!(
            parse_detail_layers("*"),
            parse_detail_layers("app,vfs,tree,render,shell,ops,plugin,watch,term,startup")
        );
        set_detail_mask(0);
        let e = LogEntry::new(LogKind::Info, "x").at(LogLayer::Vfs, LogLevel::Timing);
        assert_eq!(e.message, "⟨vfs⟩ x");
        assert_eq!(e.level, LogLevel::Timing);
    }

    fn e() -> LogEntry {
        LogEntry {
            ts: LocalTime {
                year: 2026,
                month: 9,
                day: 14,
                hour: 15,
                min: 41,
                sec: 22,
                ms: 123,
            },
            kind: LogKind::List,
            items: Some(500),
            elapsed: Some(Duration::from_micros(340_100)),
            message: "D:\\Projects".into(),
            layer: LogLayer::App,
            level: LogLevel::Basic,
        }
    }

    #[test]
    fn stamp_is_fixed_width() {
        assert_eq!(e().ts.stamp(), "2026-09-14 15:41:22.123");
        assert_eq!(e().ts.stamp().len(), 23);
    }

    #[test]
    fn raw_markdown_grid_lines() {
        assert_eq!(
            RawFormat::default().line(&e()),
            "2026-09-14 15:41:22.123  list      500 items  340.1ms  D:\\Projects"
        );
        assert_eq!(
            MarkdownFormat::default().line(&e()),
            "| 2026-09-14 15:41:22.123 | list | 500 | 340.1ms | D:\\Projects |"
        );
        let g = GridFormat::default().line(&e());
        assert!(
            g.starts_with("2026-09-14 15:41:22.123│list    │     500│  340.1ms│D:"),
            "{g}"
        );
        assert!(GridFormat::default().header().unwrap().starts_with("time"));
        assert!(RawFormat::default().header().is_none());
    }

    #[test]
    fn factory_and_names() {
        for n in FORMAT_NAMES {
            assert_eq!(formatter(n).name(), n);
        }
        assert_eq!(formatter("??").name(), "raw");
        assert_eq!(formatter("MD").name(), "markdown");
    }

    #[test]
    fn ring_buffer_caps() {
        let mut b = LogBuffer::new(3);
        for i in 0..5 {
            b.push(LogEntry::new(LogKind::Info, i.to_string()));
        }
        assert_eq!(b.len(), 3);
        assert_eq!(b.get(0).unwrap().message, "2");
    }

    /// T-90d(docs/39 §3-6 `log.max_lines`): 상한을 줄이면 앞(오래된 것)에서 즉시 버린다 · 늘리면 그대로 · 0은 1로.
    #[test]
    fn ring_buffer_set_cap_trims_front() {
        let mut b = LogBuffer::new(10);
        for i in 0..8 {
            b.push(LogEntry::new(LogKind::Info, i.to_string()));
        }
        assert_eq!(b.set_cap(3), 5, "5줄을 버렸다");
        assert_eq!(b.len(), 3);
        assert_eq!(b.get(0).unwrap().message, "5");
        assert_eq!(b.cap(), 3);
        b.push(LogEntry::new(LogKind::Info, "x"));
        assert_eq!(b.len(), 3);
        assert_eq!(b.get(2).unwrap().message, "x");
        assert_eq!(b.set_cap(100), 0);
        assert_eq!(b.len(), 3);
        assert_eq!(b.set_cap(0), 2);
        assert_eq!(b.cap(), 1);
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn now_local_is_sane() {
        let t = now_local();
        assert!(t.year >= 2026 && (1..=12).contains(&t.month) && (1..=31).contains(&t.day));
        assert!(t.hour < 24 && t.min < 60 && t.sec < 60 && t.ms < 1000);
        let u = now_utc();
        assert!(u.year >= 2026);
    }
}

#[cfg(test)]
mod format_ext_tests {
    use super::*;

    fn entry() -> LogEntry {
        let mut e = LogEntry::new(LogKind::Ops, "copy a\nto b");
        e.ts = LocalTime {
            year: 2026,
            month: 9,
            day: 16,
            hour: 14,
            min: 15,
            sec: 19,
            ms: 907,
        };
        e.items(201u64).elapsed(Duration::from_millis(33))
    }

    #[test]
    fn new_formats_and_template() {
        let e = entry();
        assert_eq!(
            formatter("compact").line(&e),
            "14:15:19.907 ops     copy a to b (201 items · 33.0ms)"
        );
        let j = formatter("jsonl").line(&e);
        assert!(j.starts_with("{\"ts\":\"2026-09-16 14:15:19.907\",\"kind\":\"ops\",\"items\":201,\"elapsed_ms\":33.0,"), "{j}");
        assert!(j.contains("\"message\":\"copy a\\nto b\""), "{j}");
        let c = formatter("csv");
        assert_eq!(
            c.header().as_deref(),
            Some("time,kind,items,elapsed,message")
        );
        assert!(
            c.line(&e).ends_with(",\"copy a to b\"") || c.line(&e).ends_with(",copy a to b"),
            "{}",
            c.line(&e)
        );
        let t = formatter_with(
            "template",
            "[{kind:>7}] {time} {msg} n={items}",
            Columns::default(),
        );
        assert_eq!(t.line(&e), "[    ops] 14:15:19.907 copy a to b n=201");
        // 컬럼 마스크: raw에서 시각·items 숨김
        let m = formatter_with("raw", "", Columns::parse("kind,elapsed"));
        assert_eq!(m.line(&e), "ops       33.0ms  copy a to b");
        assert_eq!(Columns::parse("").to_setting(), "");
        assert_eq!(Columns::parse("time,kind").to_setting(), "time,kind");
        assert_eq!(LogKind::parse("ERROR"), Some(LogKind::Error));
    }
}
