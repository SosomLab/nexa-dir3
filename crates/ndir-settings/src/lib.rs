//! `ndir-settings` — 앱 설정(출처: nexa-sql/crates/nsql-settings/src/lib.rs 엔진부 · DR-3 · docs/port/41).
//!
//! ```text
//! <설정 폴더>/settings.conf     config_dir() = NDIR_HOME | exe 옆 data/(포터블 · DR-9) | user_config_dir("nexa-dir")
//!   _schema=1
//!   ui.theme=light             ← 레지스트리 기본값과 같으면 줄을 쓰지 않는다(파일 = 사용자 변경분만)
//! ```
//!
//! 설계(nexa-sql 그대로):
//! - **레지스트리가 단일 원천** — 키·종류·기본값·라벨/설명(i18n 키)·카테고리를 [`REGISTRY`] 한 곳에 적는다(`registry.rs`).
//!   설정 창·JSON·검색·CLI가 전부 이 표를 읽는다.
//! - **파일에는 기본값과 다른 값만** · **모르는 키는 보존** · 손상 값은 읽을 때 기본값(fail-soft).
//! - 잠금·숨김·고급·OS별 기본값·이름 바꿈은 `Entry`에 필드를 늘리지 않고 **곁 표**(side table)로.
//! - 이 크레이트는 UI를 모른다 — 테마는 [`ThemeMode`]만, 팔레트는 GUI가 고른다.
//!
//! nexa-sql과 다른 점(docs/port/41 §5-4 결정): 라벨은 `Msg` 열거가 아니라 **i18n 키 문자열**(DR-14) · `SettingKind::Lang` 없음(`ui.lang`은
//! `Text` + 설정 창의 동적 후보) · `perf.rs`(성능 거버너)는 dir2에 대응 기능이 없어 1차에서 제외(열린 결정 Q-7) · `effective`의
//! OS 기본값 누락 결함(SET-134)은 구조상 없음(`flag`/`int`가 `get`을 읽는다).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub mod commands;
pub mod json;
pub mod keymap;
pub mod migrate;
mod registry;

pub use commands::{command, preset_default, repeatable, setting_key, Command, Preset, COMMANDS};
pub use json::{to_json, Import as JsonImport, Json};
pub use keymap::{split_seq, Chord, Keymap};
pub use registry::{
    ADVANCED, CATEGORY_TREE, DEPENDS, EXTENSION_CATEGORIES, HIDDEN, INFO_KEYS, INTERNAL,
    OLD_DEFAULTS, OS_DEFAULTS, REGISTRY, RENAMED, RESCALED, WINDOWS_ONLY,
};

/// 앱 폴더 이름(`%APPDATA%\nexa-dir` · `~/.config/nexa-dir` · `~/Library/Application Support/nexa-dir`).
pub const APP_DIR: &str = "nexa-dir";
/// 설정 파일 이름.
pub const FILE_NAME: &str = "settings.conf";
/// 설정 폴더 재지정 환경 변수(시험·개발 격리 · docs/18 §5).
pub const ENV_HOME: &str = "NDIR_HOME";
/// 포터블 자리 — exe 옆 폴더 이름(dir2 DR-3 계승).
pub const PORTABLE_DIR: &str = "data";

/// 옛 키면 새 키를, 아니면 그대로([`RENAMED`] + [`RESCALED`]).
#[must_use]
pub fn canonical_key(key: &str) -> &str {
    if let Some((_, new)) = RENAMED.iter().find(|(old, _)| *old == key) {
        return new;
    }
    RESCALED
        .iter()
        .find(|(old, _, _)| *old == key)
        .map_or(key, |(_, new, _)| *new)
}

/// 단위가 바뀐 옛 키면 `(새 키, 배수)` — 옛 단위 값 × 배수 = 새 단위 값.
#[must_use]
pub fn alias_scale(key: &str) -> Option<(&'static str, i64)> {
    RESCALED
        .iter()
        .find(|(old, _, _)| *old == key)
        .map(|(_, new, k)| (*new, *k))
}

/// 옛 단위의 문자열(소수 허용 · `"2.5"`)을 새 단위 정수 문자열로(`"2500"`). 숫자가 아니면 그대로(검증은 `normalize`가 한다).
fn scale_raw(raw: &str, k: i64) -> String {
    let t = raw.trim();
    match t.parse::<f64>() {
        Ok(v) if v.is_finite() => format!("{}", (v * k as f64).round() as i64),
        _ => t.to_string(),
    }
}

/// 새 단위 정수 문자열을 옛 단위로(`"2500"` → `"2.5"` · 나누어떨어지면 `"3"`).
fn unscale_value(v: &str, k: i64) -> String {
    match v.trim().parse::<i64>() {
        Ok(n) if k > 0 => {
            if n % k == 0 {
                (n / k).to_string()
            } else {
                let s = format!("{:.3}", n as f64 / k as f64);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
        }
        _ => v.to_string(),
    }
}

/// 포터블 자리(exe 옆 `data/`) — **이미 있고** 쓰기 가능하며 교체형 설치 자리(`.app` 번들 · Homebrew · `/usr` 등)가 아닐 때만(DR-9).
/// 폴더를 만들지는 않는다(설치본은 사용자 폴더를 쓴다 · 포터블 zip은 `data/`를 동봉).
#[must_use]
pub fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    if nexa_conf::is_replaced_on_upgrade(exe_dir) {
        return None;
    }
    let d = exe_dir.join(PORTABLE_DIR);
    (d.is_dir() && nexa_conf::dir_writable(&d)).then_some(d)
}

/// 설정 폴더 — `NDIR_HOME` → 포터블 `data/` → OS 사용자 설정 폴더. 라이선스·플러그인·세션·언어 오버레이도 같은 규칙.
#[must_use]
pub fn config_dir() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os(ENV_HOME) {
        return Some(PathBuf::from(h));
    }
    if let Some(p) = portable_dir() {
        return Some(p);
    }
    nexa_conf::user_config_dir(APP_DIR)
}

// ────────────────────────────────────────────────────────────── 테마 모드

/// 테마 모드. dir2 기본 = **Dark**(레지스트리 `ui.theme` 기본값 — PREFS-101).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    /// 설정 값 순서 = 순환 순서(dir2 F6: system → light → dark → system).
    pub const ALL: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<ThemeMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "system" | "auto" => Some(ThemeMode::System),
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            _ => None,
        }
    }

    /// 표시 라벨 i18n 키(dir2 `pref.theme.*`).
    #[must_use]
    pub const fn label_key(self) -> &'static str {
        match self {
            ThemeMode::System => "pref.theme.system",
            ThemeMode::Light => "pref.theme.light",
            ThemeMode::Dark => "pref.theme.dark",
        }
    }

    /// 다음 모드(단축키 순환).
    #[must_use]
    pub const fn next(self) -> ThemeMode {
        match self {
            ThemeMode::System => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
        }
    }

    /// 모드 + OS 판정 → 다크 여부. `System`에서 OS가 무선호/판정 불가면 **다크**(제품 기본 룩).
    #[must_use]
    pub const fn is_dark(self, system_dark: Option<bool>) -> bool {
        match self {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => match system_dark {
                Some(d) => d,
                None => true,
            },
        }
    }
}

// ────────────────────────────────────────────────────────────── 레지스트리

/// 설정 항목 종류(컨트롤 형태 + 검증 규칙).
#[derive(Clone, Copy, Debug)]
pub enum SettingKind {
    /// 후보 중 택일(값, 라벨 i18n 키) — 드롭다운(dir2 배치는 라디오 — 설정 창의 위젯 힌트로 재현).
    Choice(&'static [(&'static str, &'static str)]),
    /// 정수 범위.
    Int { min: i64, max: i64 },
    /// 글꼴 크기 — `13` · `13px` · `10pt`(1pt = 96/72 px). 범위는 px 기준. 저장은 입력한 단위 그대로.
    Size { min: i64, max: i64 },
    /// on/off.
    Bool,
    /// 자유 텍스트(목록·경로·순서 문자열 등 — 앱이 해석).
    Text,
    /// 화면 안 위치(3×3) — 값 = [`POSITIONS`].
    Position,
}

/// [`SettingKind::Position`] 값(행우선 · 왼쪽 위 → 오른쪽 아래). dir2 정수 0..8과 같은 순서(`POSITIONS[n]`).
pub const POSITIONS: [&str; 9] = [
    "top_left",
    "top_center",
    "top_right",
    "mid_left",
    "center",
    "mid_right",
    "bottom_left",
    "bottom_center",
    "bottom_right",
];

/// 설정 항목 — 레지스트리 한 줄.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    /// 값 키(안정 계약 — rename 시 마이그레이션 표). `<접두>.<이름>` 소문자·`_`.
    pub key: &'static str,
    /// 카테고리 i18n 키(사이드바·검색 대상) — [`CATEGORY_TREE`]에 있어야 한다.
    pub cat: &'static str,
    /// 제목 i18n 키(검색 대상).
    pub label: &'static str,
    /// 설명 i18n 키(검색 대상 · `""` = 설명 없음).
    pub desc: &'static str,
    pub kind: SettingKind,
    /// 기본값(문자열 표현 — 파일 표현과 동일).
    pub default: &'static str,
}

/// 키로 항목 찾기.
#[must_use]
pub fn entry(key: &str) -> Option<&'static Entry> {
    REGISTRY.iter().find(|e| e.key == key)
}

/// 이 OS의 기본값 — [`OS_DEFAULTS`]에 있으면 그것 · 아니면 레지스트리 `default`.
#[must_use]
pub fn default_of(key: &str) -> Option<&'static str> {
    let e = entry(key)?;
    let os = OS_DEFAULTS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, mac, linux)| {
            if cfg!(target_os = "macos") {
                *mac
            } else if cfg!(target_os = "windows") {
                e.default
            } else {
                *linux
            }
        });
    Some(os.unwrap_or(e.default))
}

/// 카테고리의 트리 순서(그룹 index, 카테고리 index) — 없으면 맨 뒤.
#[must_use]
pub fn tree_order(cat: &str) -> (usize, usize) {
    for (gi, (_, cats)) in CATEGORY_TREE.iter().enumerate() {
        if let Some(ci) = cats.iter().position(|c| *c == cat) {
            return (gi, ci);
        }
    }
    (usize::MAX, usize::MAX)
}

/// 카테고리가 속한 그룹(없으면 None).
#[must_use]
pub fn group_of(cat: &str) -> Option<&'static str> {
    CATEGORY_TREE
        .iter()
        .find(|(_, cats)| cats.contains(&cat))
        .map(|(g, _)| *g)
}

/// 종속 조건(설정 화면 잠금용).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dep {
    /// 부모가 `on`.
    On,
    /// 부모가 비어 있지 않음.
    NotEmpty,
    /// 부모가 이 값.
    Eq(&'static str),
    /// 부모가 이 값이 **아님**.
    Ne(&'static str),
    /// 부모가 이 값들 중 하나.
    OneOf(&'static [&'static str]),
}

impl Dep {
    /// 부모 값이 조건을 만족하는가.
    #[must_use]
    pub fn satisfied(self, parent_value: &str) -> bool {
        match self {
            Dep::On => parent_value == "on",
            Dep::NotEmpty => !parent_value.trim().is_empty(),
            Dep::Eq(v) => parent_value == v,
            Dep::Ne(v) => parent_value != v,
            Dep::OneOf(vs) => vs.contains(&parent_value),
        }
    }
}

/// 자식 키의 모든 (부모 키, 조건) — 전부 만족해야 쓸 수 있다(AND · [`DEPENDS`]에 여러 줄).
pub fn dependencies(child: &str) -> impl Iterator<Item = (&'static str, Dep)> + '_ {
    DEPENDS
        .iter()
        .filter(move |(c, _, _)| *c == child)
        .map(|(_, p, d)| (*p, *d))
}

/// 지금 이 설정을 **쓸 수 없게 만든 원인** `(부모 키, 조건)`(순수 · `value_of` = 키의 현재 값): 조건이 하나라도 어긋나면 그
/// 부모 · 조건은 맞아도 **부모가 잠겨 있으면 그 부모를 잠근 원인**(전이 — 상위를 끄면 하위의 하위도 잠긴다 · 사용자 10-03).
/// 쓸 수 있으면 `None`.
#[must_use]
pub fn locked_by(child: &str, value_of: &dyn Fn(&str) -> String) -> Option<(&'static str, Dep)> {
    fn walk(
        child: &str,
        value_of: &dyn Fn(&str) -> String,
        depth: u8,
    ) -> Option<(&'static str, Dep)> {
        if depth > 8 {
            return None; // 순환 방어(곁 표 시험이 순환을 막는다)
        }
        for (parent, dep) in dependencies(child) {
            if !dep.satisfied(&value_of(parent)) {
                return Some((parent, dep));
            }
            if let Some(cause) = walk(parent, value_of, depth + 1) {
                return Some(cause);
            }
        }
        None
    }
    walk(child, value_of, 0)
}

/// 자식 키의 (부모 키, 조건) — [`DEPENDS`].
#[must_use]
pub fn dependency(child: &str) -> Option<(&'static str, Dep)> {
    DEPENDS
        .iter()
        .find(|(c, _, _)| *c == child)
        .map(|(_, p, d)| (*p, *d))
}

/// 비노출 설정인가 — 레지스트리에는 있어 `set/get/reset`은 되지만 설정 화면엔 기본 숨김.
#[must_use]
pub fn is_hidden(key: &str) -> bool {
    HIDDEN.contains(&key)
}

/// 내부 전용 설정인가 — 설정 창(고급 포함) · 검색 · JSON 어디에도 나오지 않는다([`INTERNAL`]).
#[must_use]
pub fn is_internal(key: &str) -> bool {
    is_internal_on(key, cfg!(windows))
}

/// [`is_internal`]의 순수 판정(`windows` = 이 OS가 Windows인가): 내부 전용 키 ∪ (Windows가 아니면) [`WINDOWS_ONLY`].
#[must_use]
pub fn is_internal_on(key: &str, windows: bool) -> bool {
    INTERNAL.contains(&key) || (!windows && WINDOWS_ONLY.contains(&key))
}

/// 고급 설정인가(설정 창 Advanced 토글 대상) — 비노출 ∪ [`ADVANCED`].
#[must_use]
pub fn is_advanced(key: &str) -> bool {
    is_hidden(key) || ADVANCED.contains(&key)
}

/// 읽기 전용 정보 키인가(호스트가 채우는 계산 값 · `set` 거부).
#[must_use]
pub fn is_info(key: &str) -> bool {
    INFO_KEYS.contains(&key)
}

/// 카테고리 안 표시 순서: (트리 순서, 키 접두의 첫 등재 순, 등재 순) — 같은 카테고리의 접두가 흩어지지 않는다.
#[must_use]
pub fn display_order(key: &str) -> (usize, usize, usize, usize) {
    let Some(idx) = REGISTRY.iter().position(|e| e.key == key) else {
        return (usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    };
    let e = &REGISTRY[idx];
    let (g, c) = tree_order(e.cat);
    let prefix = key.split('.').next().unwrap_or(key);
    let first = REGISTRY
        .iter()
        .position(|x| x.cat == e.cat && x.key.split('.').next().unwrap_or(x.key) == prefix)
        .unwrap_or(idx);
    (g, c, first, idx)
}

/// 허용 값 설명(오류 메시지·목록용).
#[must_use]
pub fn allowed(kind: SettingKind) -> String {
    match kind {
        SettingKind::Choice(opts) => opts.iter().map(|(v, _)| *v).collect::<Vec<_>>().join(" | "),
        SettingKind::Int { min, max } => format!("{min}..{max}"),
        SettingKind::Size { min, max } => format!("{min}..{max}px | Npt"),
        SettingKind::Bool => "on | off".into(),
        SettingKind::Text => "text".into(),
        SettingKind::Position => POSITIONS.join(" | "),
    }
}

/// 값 검증 → 정규화된 저장 표현. 실패 = `None`.
#[must_use]
pub fn normalize(kind: SettingKind, raw: &str) -> Option<String> {
    let v = raw.trim();
    match kind {
        SettingKind::Choice(opts) => {
            let lower = v.to_ascii_lowercase();
            opts.iter()
                .find(|(o, _)| *o == lower)
                .map(|(o, _)| (*o).to_string())
        }
        SettingKind::Int { min, max } => {
            let n: i64 = v.parse().ok()?;
            (min..=max).contains(&n).then(|| n.to_string())
        }
        SettingKind::Size { min, max } => {
            let (n, unit) = parse_size(v)?;
            let px = if unit == "pt" { n * 96.0 / 72.0 } else { n };
            if !(min as f32..=max as f32).contains(&px) {
                return None;
            }
            let num = if (n.fract()).abs() < 1e-6 {
                format!("{}", n as i64)
            } else {
                format!("{n}")
            };
            Some(if unit == "pt" {
                format!("{num}pt")
            } else {
                num
            })
        }
        SettingKind::Bool => match v.to_ascii_lowercase().as_str() {
            "on" | "true" | "1" | "yes" => Some("on".into()),
            "off" | "false" | "0" | "no" => Some("off".into()),
            _ => None,
        },
        SettingKind::Text => Some(v.to_string()),
        SettingKind::Position => {
            let lower = v.to_ascii_lowercase();
            POSITIONS
                .iter()
                .find(|p| **p == lower)
                .map(|p| (*p).to_string())
        }
    }
}

/// `13` · `13px` · `10pt` → (수치, 단위 `px`|`pt`). 모르는 형식 = None.
#[must_use]
pub fn parse_size(v: &str) -> Option<(f32, &'static str)> {
    let t = v.trim().to_ascii_lowercase();
    let (num, unit) = if let Some(n) = t.strip_suffix("pt") {
        (n.trim(), "pt")
    } else if let Some(n) = t.strip_suffix("px") {
        (n.trim(), "px")
    } else {
        (t.as_str(), "px")
    };
    let n: f32 = num.parse().ok()?;
    (n.is_finite() && n >= 0.0).then_some((n, unit))
}

/// 글꼴 크기 값 → px(`10pt` = 13.33px). 형식이 틀리면 None.
#[must_use]
pub fn size_px(v: &str) -> Option<f32> {
    let (n, unit) = parse_size(v)?;
    Some(if unit == "pt" { n * 96.0 / 72.0 } else { n })
}

// ────────────────────────────────────────────────────────────── 설정 값

/// `set` 실패 사유.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError {
    UnknownKey(String),
    /// (키, 입력값, 허용 값 설명)
    InvalidValue(String, String, String),
}

impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetError::UnknownKey(k) => write!(f, "{}", ndir_i18n::trf("cfg.unknownKey", &[k])),
            SetError::InvalidValue(k, v, a) => {
                write!(f, "{}", ndir_i18n::trf("cfg.invalidValue", &[k, v, a]))
            }
        }
    }
}

impl std::error::Error for SetError {}

/// 설정 값 집합 + 파일. **파일에는 기본값과 다른 값만** 쓴다.
#[derive(Debug)]
pub struct Settings {
    path: PathBuf,
    /// 사용자가 바꾼 값(정규화 완료 · 기본값과 다른 것만).
    values: BTreeMap<String, String>,
    /// 레지스트리에 없는 키(보존 재방출).
    unknown: Vec<(String, String)>,
}

impl Settings {
    /// 기본 폴더의 `settings.conf`. 폴더를 알 수 없으면 오류(파일이 없는 것은 오류가 아니다 — 전부 기본값).
    pub fn open_default() -> io::Result<Settings> {
        let dir = config_dir().ok_or_else(|| io::Error::other(ndir_i18n::tr("cfg.noConfigDir")))?;
        Ok(Self::open(dir.join(FILE_NAME)))
    }

    /// 지정 파일. 없거나 손상이어도 실패하지 않는다(손상 값은 기본값으로 · 파일은 건드리지 않음).
    #[must_use]
    pub fn open(path: PathBuf) -> Settings {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        Self::from_text(path, &text)
    }

    /// 파일 없이 본문으로(시험 · 격리 인스턴스 — 저장은 `path`로).
    #[must_use]
    pub fn from_text(path: PathBuf, text: &str) -> Settings {
        // BOM 보강(SET-137 · 사용자 수기 편집·dir2 가져오기 대비) — nexa-conf 파서는 BOM을 모른다.
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let doc = nexa_conf::parse(text);
        let mut s = Settings {
            path,
            values: BTreeMap::new(),
            unknown: Vec::new(),
        };
        let mut seen: Vec<String> = Vec::new();
        for (k, v) in doc.pairs {
            match entry(&k) {
                Some(e) => {
                    seen.push(k.clone());
                    if let Some(n) = normalize(e.kind, &v) {
                        // 옛 기본값 그대로 저장돼 있던 값은 "기본값 유지"로 본다(기본값이 바뀌면 따라간다).
                        let old_default =
                            OLD_DEFAULTS.iter().any(|(key, old)| *key == k && *old == n);
                        let def = default_of(e.key).unwrap_or(e.default);
                        if n != def && !old_default {
                            s.values.insert(k, n);
                        }
                    }
                    // 손상 값 = 기본값(fail-soft) · 저장 시 그 줄은 사라진다.
                }
                None => s.unknown.push((k, v)),
            }
        }
        s.migrate_renamed(&seen);
        s
    }

    /// [`RENAMED`] 표대로 옛 키의 값을 새 키로 옮긴다(새 키를 이미 정했으면 옛 값은 버림 · 옛 줄은 다음 저장 때 사라진다).
    /// [`RESCALED`] 표의 키는 **배수를 곱해** 옮긴다.
    fn migrate_renamed(&mut self, seen: &[String]) {
        let renamed = RENAMED.iter().map(|(o, n)| (*o, *n, 1));
        let rescaled = RESCALED.iter().copied();
        for (old, new, k) in renamed.chain(rescaled) {
            let Some(pos) = self.unknown.iter().position(|(key, _)| key == old) else {
                continue;
            };
            let (_, v) = self.unknown.remove(pos);
            let Some(e) = entry(new) else { continue };
            if self.values.contains_key(new) || seen.iter().any(|key| key == new) {
                continue;
            }
            let v = if k == 1 { v } else { scale_raw(&v, k) };
            if let Some(n) = normalize(e.kind, &v) {
                let def = default_of(e.key).unwrap_or(e.default);
                if n != def {
                    self.values.insert(new.to_string(), n);
                }
            }
        }
    }

    /// dir2 `settings.cfg` 1회 가져오기 — [`migrate::import_dir2`]가 변환한 쌍을 `set`으로 넣는다(검증 통과분만).
    /// 반환 = 들어간 키 수. 이미 값이 있는 키(사용자가 dir3에서 정한 것)는 덮지 않는다.
    pub fn import_dir2(&mut self, text: &str) -> usize {
        let mut n = 0;
        for (k, v) in migrate::import_dir2(text) {
            if self.values.contains_key(&k) {
                continue;
            }
            if self.set(&k, &v).is_ok() {
                n += 1;
            }
        }
        n
    }

    /// 파일 경로.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 현재 값(사용자 값 → 기본값). 모르는 키는 `None`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        let key = canonical_key(key);
        let e = entry(key)?;
        let def = default_of(e.key).unwrap_or(e.default);
        Some(self.values.get(key).map_or(def, String::as_str))
    }

    /// 현재 값을 **물은 키의 단위로**(단위가 바뀐 옛 키면 새 값을 배수로 나눈 문자열).
    #[must_use]
    pub fn get_as(&self, key: &str) -> Option<String> {
        let v = self.get(key)?;
        Some(match alias_scale(key) {
            Some((_, k)) => unscale_value(v, k),
            None => v.to_string(),
        })
    }

    /// 사용자가 바꾼 값인가(기본값과 다른가).
    #[must_use]
    pub fn is_modified(&self, key: &str) -> bool {
        let key = canonical_key(key);
        self.values.contains_key(key)
    }

    /// 검증 후 설정(메모리). 기본값과 같으면 사용자 값을 지운다. 단위가 바뀐 옛 키로 오면 옛 단위로 해석해 배수를 곱한다.
    pub fn set(&mut self, key: &str, raw: &str) -> Result<String, SetError> {
        let scaled;
        let raw = match alias_scale(key) {
            Some((_, k)) => {
                scaled = scale_raw(raw, k);
                scaled.as_str()
            }
            None => raw,
        };
        let key = canonical_key(key);
        let e = entry(key).ok_or_else(|| SetError::UnknownKey(key.to_string()))?;
        if is_info(key) {
            return Err(SetError::InvalidValue(
                key.to_string(),
                raw.to_string(),
                ndir_i18n::tr("cfg.readOnly"),
            ));
        }
        let n = normalize(e.kind, raw).ok_or_else(|| {
            SetError::InvalidValue(key.to_string(), raw.to_string(), allowed(e.kind))
        })?;
        if n == default_of(e.key).unwrap_or(e.default) {
            self.values.remove(key);
        } else {
            self.values.insert(key.to_string(), n.clone());
        }
        Ok(n)
    }

    /// 기본값으로(메모리).
    pub fn reset(&mut self, key: &str) -> Result<&'static str, SetError> {
        let key = canonical_key(key);
        let e = entry(key).ok_or_else(|| SetError::UnknownKey(key.to_string()))?;
        self.values.remove(key);
        Ok(default_of(e.key).unwrap_or(e.default))
    }

    /// 원자적 저장(폴더가 없으면 만든다). 내용 = 변경분 + 모르는 키.
    pub fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let known: Vec<(&str, &str)> = self
            .values
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        nexa_conf::write_atomic(&self.path, &nexa_conf::serialize(&known, &self.unknown))
    }

    // ── 타입 있는 접근자(자주 쓰는 키)

    /// 언어 설정값(`system` 또는 코드) — 실제 코드는 `ndir_i18n::resolve_code`로.
    #[must_use]
    pub fn lang_setting(&self) -> &str {
        self.get("ui.lang").unwrap_or("system")
    }

    #[must_use]
    pub fn theme_mode(&self) -> ThemeMode {
        self.get("ui.theme")
            .and_then(ThemeMode::parse)
            .unwrap_or_default()
    }

    /// on/off 설정(모르는 키·손상 = false).
    #[must_use]
    pub fn flag(&self, key: &str) -> bool {
        self.get(key) == Some("on")
    }

    /// 글꼴 크기(px) — `Size` 항목(`13` · `13px` · `10pt`) · 틀리면 레지스트리 기본.
    #[must_use]
    pub fn font_px(&self, key: &str) -> f32 {
        self.get(key)
            .and_then(size_px)
            .or_else(|| entry(key).and_then(|e| size_px(e.default)))
            .unwrap_or(0.0)
    }

    /// 정수 설정(레지스트리 기본값 보장 → 실패 없음 · 모르는 키 = 0).
    #[must_use]
    pub fn int(&self, key: &str) -> i64 {
        self.get(key)
            .and_then(|v| v.parse().ok())
            .or_else(|| entry(key).and_then(|e| e.default.parse().ok()))
            .unwrap_or(0)
    }

    /// 위치 설정 → 3×3 인덱스(행우선 0..8 · dir2 정수와 같다).
    #[must_use]
    pub fn position_index(&self, key: &str) -> usize {
        let v = self.get(key).unwrap_or("");
        POSITIONS.iter().position(|p| *p == v).unwrap_or(0)
    }

    /// 전 항목 (키, 현재 값, 변경 여부) — 설정 화면·목록.
    #[must_use]
    pub fn list(&self) -> Vec<(&'static Entry, &str, bool)> {
        REGISTRY
            .iter()
            .map(|e| {
                (
                    e,
                    self.values
                        .get(e.key)
                        .map_or(default_of(e.key).unwrap_or(e.default), String::as_str),
                    self.values.contains_key(e.key),
                )
            })
            .collect()
    }

    /// 노출 항목만(설정 화면) — 비노출([`is_hidden`])은 제외.
    #[must_use]
    pub fn list_visible(&self) -> Vec<(&'static Entry, &str, bool)> {
        self.list()
            .into_iter()
            .filter(|(e, _, _)| !is_hidden(e.key))
            .collect()
    }

    /// 레지스트리 밖 키(보존분) — 진단·가져오기 뒤 잔여 확인용.
    #[must_use]
    pub fn unknown_keys(&self) -> &[(String, String)] {
        &self.unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir-settings-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        d.join(FILE_NAME)
    }

    /// ★ 레지스트리 무결성: 모든 기본값이 자기 `normalize`를 통과 · 키 중복 없음 · 키 꼴(`<접두>.<이름>` 소문자).
    #[test]
    fn registry_defaults_are_valid_and_keys_unique() {
        let mut seen = HashSet::new();
        for e in REGISTRY {
            assert!(seen.insert(e.key), "중복 키: {}", e.key);
            assert!(
                normalize(e.kind, e.default).is_some(),
                "기본값이 검증을 통과하지 못함: {} = {:?}",
                e.key,
                e.default
            );
            let (p, n) = e
                .key
                .split_once('.')
                .unwrap_or_else(|| panic!("2레벨 키가 아님: {}", e.key));
            assert!(
                !p.is_empty() && !n.is_empty() && p.len() <= 9,
                "접두 규칙: {}",
                e.key
            );
            assert!(
                e.key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'),
                "키 글자: {}",
                e.key
            );
        }
        assert!(REGISTRY.len() >= 60, "dir2 70키 계승: {}", REGISTRY.len());
    }

    /// ★ 곁 표 무결성(SET-132 — 원본에 없던 시험): HIDDEN·ADVANCED·DEPENDS·INFO_KEYS·OS_DEFAULTS의 키가 전부 레지스트리에 있고 ·
    /// 모든 `cat`이 트리에 있고 · 종속 자식은 부모 뒤에 등재 · 라벨/설명/카테고리 i18n 키가 전부 있다.
    #[test]
    fn side_tables_reference_existing_keys_and_labels() {
        for k in HIDDEN.iter().chain(ADVANCED).chain(INFO_KEYS) {
            assert!(entry(k).is_some(), "곁 표에 없는 키: {k}");
        }
        for (k, mac, linux) in OS_DEFAULTS {
            let e = entry(k).unwrap_or_else(|| panic!("OS_DEFAULTS에 없는 키: {k}"));
            assert!(
                normalize(e.kind, mac).is_some() && normalize(e.kind, linux).is_some(),
                "OS 기본값 검증 실패: {k}"
            );
            assert!(default_of(k).is_some());
        }
        for (child, parent, _) in DEPENDS {
            let ci = REGISTRY
                .iter()
                .position(|e| e.key == *child)
                .unwrap_or_else(|| panic!("DEPENDS 자식 없음: {child}"));
            let pi = REGISTRY
                .iter()
                .position(|e| e.key == *parent)
                .unwrap_or_else(|| panic!("DEPENDS 부모 없음: {parent}"));
            assert!(pi < ci, "종속 자식은 부모 뒤에: {child} ← {parent}");
        }
        let mut hidden_seen = HashSet::new();
        for k in HIDDEN {
            assert!(hidden_seen.insert(*k), "HIDDEN 중복: {k}");
        }
        for e in REGISTRY {
            assert_ne!(
                tree_order(e.cat).0,
                usize::MAX,
                "트리에 없는 카테고리: {} ({})",
                e.cat,
                e.key
            );
            for key in [e.cat, e.label] {
                assert!(ndir_i18n::has(key), "i18n 키 없음: {key} ({})", e.key);
            }
            if !e.desc.is_empty() {
                assert!(
                    ndir_i18n::has(e.desc),
                    "i18n 설명 키 없음: {} ({})",
                    e.desc,
                    e.key
                );
            }
            if let SettingKind::Choice(opts) = e.kind {
                for (_, l) in opts {
                    assert!(ndir_i18n::has(l), "i18n 선택지 키 없음: {l} ({})", e.key);
                }
            }
        }
        for (g, cats) in CATEGORY_TREE {
            assert!(ndir_i18n::has(g), "그룹 i18n 키 없음: {g}");
            for c in *cats {
                assert!(ndir_i18n::has(c), "카테고리 i18n 키 없음: {c}");
            }
        }
        for key in [
            "cfg.unknownKey",
            "cfg.invalidValue",
            "cfg.noConfigDir",
            "cfg.readOnly",
        ] {
            assert!(ndir_i18n::has(key), "{key}");
        }
    }

    /// 종속 판정(T-120): 조건 종류 · 여러 부모(AND) · 전이(부모가 잠기면 자식도) · 원인은 맨 위의 어긋난 조건.
    #[test]
    fn locks_follow_parents_transitively_and_all_conditions() {
        assert!(Dep::Ne("1").satisfied("2") && !Dep::Ne("1").satisfied("1"));
        assert!(Dep::OneOf(&["system", "dark"]).satisfied("dark"));
        assert!(!Dep::OneOf(&["system", "dark"]).satisfied("campbell"));
        let with = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(pk, _)| *pk == k)
                    .map(|(_, v)| (*v).to_string())
                    .or_else(|| default_of(k).map(str::to_string))
                    .unwrap_or_default()
            }
        };
        // 기본값에서는 잠긴 것이 없다(터미널 열 수만 줄 바꿈 조건 — 기본 off라 풀려 있다).
        let d = with(&[]);
        for (child, _, _) in DEPENDS {
            if *child == "tabs.scroll_buttons" {
                continue; // 기본 = 여러 줄 → 버튼 자리는 잠김(의도)
            }
            if child.starts_with("toolbar.on_") {
                continue; // 기본 = 초록(스위치와 통일) → 강조색 농도 설정은 잠김(의도)
            }
            assert_eq!(locked_by(child, &d), None, "{child}");
        }
        // 전이: 고속 스크롤을 끄면 배지(켜져 있어도)의 하위 설정까지 잠기고 원인 = scroll.fast.
        let off = with(&[("scroll.fast", "off"), ("scroll.fast_hud", "on")]);
        assert_eq!(
            locked_by("scroll.fast_hud", &off),
            Some(("scroll.fast", Dep::On))
        );
        assert_eq!(
            locked_by("scroll.fast_hud_pos", &off),
            Some(("scroll.fast", Dep::On))
        );
        let hud_off = with(&[("scroll.fast_hud", "off")]);
        assert_eq!(
            locked_by("scroll.fast_hud_pos", &hud_off),
            Some(("scroll.fast_hud", Dep::On))
        );
        // 여러 부모(AND): 도크 좌우 분할 = 도크 표시 ∧ 듀얼 정보(∧ 전이로 듀얼 패널).
        assert_eq!(dependencies("layout.dock_split_pct").count(), 2);
        let hidden = with(&[("dock.visible", "off")]);
        assert_eq!(
            locked_by("layout.dock_split_pct", &hidden),
            Some(("dock.visible", Dep::On))
        );
        let single_info = with(&[("layout.info_mode", "single")]);
        assert_eq!(
            locked_by("layout.dock_split_pct", &single_info),
            Some(("layout.info_mode", Dep::Eq("dual")))
        );
        let single_panel = with(&[("layout.panel_mode", "single")]);
        assert_eq!(
            locked_by("layout.dock_split_pct", &single_panel),
            Some(("layout.panel_mode", Dep::Eq("dual"))),
            "전이: 단일 패널 → 정보 배치 잠김 → 도크 좌우도"
        );
        assert_eq!(locked_by("layout.dock_height_pct", &single_panel), None);
        // 터미널 테마: 스킴 id를 직접 적으면 다크/라이트 스킴 둘 다 쓰이지 않는다.
        let scheme = with(&[("term.theme", "campbell")]);
        assert!(locked_by("term.theme_dark", &scheme).is_some());
        assert!(locked_by("term.theme_light", &scheme).is_some());
        let dark = with(&[("term.theme", "dark")]);
        assert_eq!(locked_by("term.theme_dark", &dark), None);
        assert!(locked_by("term.theme_light", &dark).is_some());
        // 곁 표에 순환이 없다(자식은 부모 뒤에 등재 — side_tables 시험 · 여기서는 깊이로 확인).
        for (child, _, _) in DEPENDS {
            let _ = locked_by(child, &off);
        }
    }

    /// 시간 단위 규칙(nexa-sql docs/94 §6-5 · 사용자 10-03 "10초 이상은 초 단위 · 10초 미만은 ms"): 기본값이 10초 이하인 시간
    /// 설정 = `_ms` · 넘으면 `_secs` · 분 = `_min`. 단위 없는 시간 키(`_sec` · `_timeout` · `_delay` · `_interval`로 끝남)는 금지 —
    /// 새 키가 규칙을 어기면 여기서 걸린다. 단위를 바꾸면 `RESCALED`에 (옛 키, 새 키, 배수)를 적는다.
    #[test]
    fn time_keys_follow_unit_rule() {
        let mut ms = 0;
        for e in REGISTRY {
            let default: Option<i64> = e.default.parse().ok();
            if e.key.ends_with("_ms") {
                ms += 1;
                let d = default.unwrap_or_else(|| panic!("{}: 숫자 기본값", e.key));
                assert!(d <= 10_000, "{} 기본 {d} ms > 10초 → `_secs`로", e.key);
            } else if e.key.ends_with("_secs") {
                let d = default.unwrap_or_else(|| panic!("{}: 숫자 기본값", e.key));
                assert!(d > 10 || d == 0, "{} 기본 {d}초 ≤ 10초 → `_ms`로", e.key);
            }
            for bad in [
                "_sec",
                "_seconds",
                "_timeout",
                "_delay",
                "_interval",
                "_millis",
            ] {
                assert!(
                    !e.key.ends_with(bad),
                    "{}: 단위 접미(`_ms`/`_secs`/`_min`)를 붙인다",
                    e.key
                );
            }
        }
        assert!(ms >= 7, "시간 설정 {ms}개");
        for (old, new, scale) in RESCALED {
            assert!(
                entry(new).is_some() && entry(old).is_none() && *scale > 0,
                "{old} → {new}"
            );
        }
    }

    /// dir2 기본값 계승(DR-3 · PREFS-101~): 테마 = **system**(dir2 dark에서 바꿈 — 사용자 10-03 "테마는 시스템을 기본값으로") · 언어 system · 숨김 on · 고속 스크롤 3/16 · 타입어헤드 좌하 · 전송 2000 ms.
    #[test]
    fn defaults_follow_dir2() {
        let s = Settings::open(tmp("defaults"));
        assert_eq!(s.theme_mode(), ThemeMode::System);
        assert_eq!(s.lang_setting(), "system");
        assert!(
            s.flag("list.show_hidden")
                && s.flag("list.show_dotfiles")
                && s.flag("list.folders_first")
        );
        assert_eq!(s.int("scroll.fast_step"), 3);
        assert_eq!(s.int("scroll.fast_max"), 16);
        assert_eq!(s.get("typeahead.hud_pos"), Some("bottom_left"));
        assert_eq!(s.position_index("typeahead.hud_pos"), 6);
        assert_eq!(s.position_index("scroll.fast_hud_pos"), 2);
        assert_eq!(s.int("transfer.close_ms"), 2000);
        assert_eq!(s.get("term.theme_dark"), Some("campbell"));
        assert_eq!(s.get("term.theme_light"), Some("github-light"));
        assert_eq!(s.get("tabs.dblclick"), Some("close"));
        assert!(
            (s.font_px("ui.dialog_font_size") - 12.0).abs() < 1e-3,
            "9pt = 12px"
        );
        assert_eq!(s.get("nope"), None);
        assert!(!s.is_modified("ui.theme"));
    }

    #[test]
    fn set_validates_and_saves_only_changes() {
        let p = tmp("set");
        let mut s = Settings::open(p.clone());
        assert_eq!(s.set("ui.theme", "Light").unwrap(), "light");
        assert!(matches!(
            s.set("ui.theme", "blue"),
            Err(SetError::InvalidValue(..))
        ));
        assert!(matches!(s.set("nope.x", "1"), Err(SetError::UnknownKey(_))));
        assert_eq!(
            s.set("list.show_hidden", "0").unwrap(),
            "off",
            "dir2 불리언 0/1 수용"
        );
        assert_eq!(s.set("scroll.fast_step", "7").unwrap(), "7");
        assert!(s.set("scroll.fast_step", "99").is_err(), "범위 밖은 거부");
        assert_eq!(
            s.set("typeahead.hud_pos", "TOP_RIGHT").unwrap(),
            "top_right"
        );
        s.save().unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.starts_with("_schema=1\n"));
        assert!(
            text.contains("ui.theme=light\n")
                && text.contains("list.show_hidden=off\n")
                && text.contains("scroll.fast_step=7\n")
        );
        assert!(!text.contains("ui.lang"), "기본값은 쓰지 않는다");
        // 기본값으로 돌아오면 줄이 사라진다.
        s.set("ui.theme", "system").unwrap();
        assert!(!s.is_modified("ui.theme"));
        s.save().unwrap();
        assert!(!std::fs::read_to_string(&p).unwrap().contains("ui.theme"));
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn corrupt_value_falls_back_and_unknown_keys_survive() {
        let p = tmp("corrupt");
        let s = Settings::from_text(p.clone(), "\u{feff}_schema=1\nui.theme=blue\nscroll.fast_step=abc\nfuture.key=42\nterm.cols=500\n");
        assert_eq!(s.theme_mode(), ThemeMode::System, "손상 값 = 기본값");
        assert_eq!(s.int("scroll.fast_step"), 3);
        assert_eq!(s.int("term.cols"), 500);
        assert_eq!(
            s.unknown_keys(),
            &[("future.key".to_string(), "42".to_string())]
        );
        s.save().unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("future.key=42"), "미지 키 보존: {text}");
        assert!(!text.contains("ui.theme"), "손상 줄은 사라진다");
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// 이주 표 루프(표가 비어 있어도 돈다 — 채워지면 그대로 검사).
    #[test]
    fn renamed_and_rescaled_tables_are_consistent() {
        for (old, new) in RENAMED
            .iter()
            .map(|(o, n)| (*o, *n))
            .chain(RESCALED.iter().map(|(o, n, _)| (*o, *n)))
        {
            assert!(
                entry(old).is_none(),
                "옛 키가 레지스트리에 남아 있다: {old}"
            );
            assert!(entry(new).is_some(), "새 키가 레지스트리에 없다: {new}");
            assert_eq!(canonical_key(old), new);
        }
        for (_, new, k) in RESCALED {
            assert!(*k > 0 && new.ends_with("_ms"), "새 키는 단위 접미: {new}");
        }
        assert_eq!(canonical_key("ui.theme"), "ui.theme");
        assert_eq!(unscale_value("2500", 1000), "2.5");
        assert_eq!(unscale_value("2", 1000), "0.002");
        assert_eq!(scale_raw(" 1.2345 ", 1000), "1235");
        assert_eq!(scale_raw("abc", 1000), "abc");
    }

    #[test]
    fn display_order_groups_by_category_then_prefix() {
        for (_, cats) in CATEGORY_TREE {
            for cat in *cats {
                let mut keys: Vec<&str> = REGISTRY
                    .iter()
                    .filter(|e| e.cat == *cat)
                    .map(|e| e.key)
                    .collect();
                keys.sort_by_key(|k| display_order(k));
                let mut seen: Vec<&str> = Vec::new();
                for k in &keys {
                    let p = k.split('.').next().unwrap_or(k);
                    if seen.last() != Some(&p) {
                        assert!(!seen.contains(&p), "접두 {p}가 흩어졌다({cat}): {keys:?}");
                        seen.push(p);
                    }
                }
            }
        }
        assert!(
            display_order("ui.theme") < display_order("list.show_hidden"),
            "그룹 순(일반 → 파일 목록)"
        );
        assert!(
            display_order("list.show_hidden") < display_order("typeahead.scope"),
            "카테고리 순"
        );
        assert_eq!(display_order("nope").0, usize::MAX);
    }

    #[test]
    fn dependencies_and_hidden_and_advanced() {
        assert_eq!(
            dependency("scroll.fast_step"),
            Some(("scroll.fast", Dep::On))
        );
        assert_eq!(
            dependency("scroll.fast_hud_pos"),
            Some(("scroll.fast_hud", Dep::On))
        );
        assert_eq!(dependency("term.cols"), Some(("term.wrap", Dep::Eq("off"))));
        assert!(Dep::On.satisfied("on") && !Dep::On.satisfied("off"));
        assert!(Dep::NotEmpty.satisfied("x") && !Dep::NotEmpty.satisfied(" "));
        assert!(is_hidden("layout.panel_split_pct") && is_advanced("layout.panel_split_pct"));
        assert!(is_advanced("preview.map") && !is_hidden("preview.map"));
        for k in [
            "ui.theme",
            "ui.lang",
            "list.show_hidden",
            "scroll.fast",
            "term.wrap",
            "dock.visible",
        ] {
            assert!(!is_advanced(k), "기본 표시여야 한다: {k}");
        }
    }

    #[test]
    fn size_units_and_theme_mode() {
        assert_eq!(size_px("13"), Some(13.0));
        assert_eq!(size_px("13px"), Some(13.0));
        assert!((size_px("10pt").unwrap_or(0.0) - 13.333_333).abs() < 1e-3);
        assert_eq!(size_px("abc"), None);
        let k = SettingKind::Size { min: 8, max: 40 };
        assert_eq!(normalize(k, "10pt"), Some("10pt".into()));
        assert_eq!(normalize(k, "13px"), Some("13".into()));
        assert_eq!(normalize(k, "10.5pt"), Some("10.5pt".into()));
        assert_eq!(normalize(k, "99"), None);
        assert_eq!(ThemeMode::parse("AUTO"), Some(ThemeMode::System));
        assert_eq!(ThemeMode::Dark.next(), ThemeMode::System);
        assert!(
            ThemeMode::System.is_dark(None)
                && !ThemeMode::System.is_dark(Some(false))
                && !ThemeMode::Light.is_dark(Some(true))
        );
        assert_eq!(ThemeMode::Light.label_key(), "pref.theme.light");
    }

    #[test]
    fn config_dir_honors_env_home() {
        // 환경 변수는 프로세스 전역 — 다른 시험이 읽지 않으므로 여기서만 잠깐 건드린다.
        let d = std::env::temp_dir().join(format!("ndir-home-{}", std::process::id()));
        std::env::set_var(ENV_HOME, &d);
        assert_eq!(config_dir().as_deref(), Some(d.as_path()));
        std::env::remove_var(ENV_HOME);
        assert!(config_dir().is_some(), "포터블 또는 사용자 폴더 중 하나");
    }

    /// 내부 전용 키(`license.gates`): 레지스트리에는 있고 코드로는 읽지만 JSON 내보내기에 없고 가져오기로 못 바꾼다.
    #[test]
    fn internal_keys_never_surface() {
        assert!(is_internal("license.gates") && !is_internal("ui.theme"));
        // Windows 전용 설정: Windows에서는 보이고 다른 OS에서는 내부 전용처럼 빠진다 · 키는 레지스트리에 있어야 한다.
        for k in WINDOWS_ONLY {
            assert!(entry(k).is_some(), "{k}");
            assert!(!is_internal_on(k, true) && is_internal_on(k, false), "{k}");
        }
        assert!(!is_internal_on("list.show_hidden", false));
        assert!(is_internal_on("license.gates", true));
        for k in INTERNAL {
            assert!(entry(k).is_some(), "레지스트리에 있어야 한다: {k}");
            assert!(
                is_hidden(k),
                "내부 전용은 비노출 목록에도 둔다(설정 화면 기본 목록 제외): {k}"
            );
        }
        let dir =
            std::env::temp_dir().join(format!("ndir-settings-internal-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut s = Settings::from_text(dir.join("settings.conf"), "");
        let before = s.get("license.gates").map(str::to_string);
        let path = s.export_json().expect("export");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("gates"),
            "내보낸 JSON에 내부 키가 없어야 한다"
        );
        let r = s
            .import_json(r#"{"license": {"gates": true}, "ui": {"theme": "light"}}"#)
            .expect("import");
        assert_eq!(
            s.get("license.gates").map(str::to_string),
            before,
            "가져오기로 못 바꾼다"
        );
        assert!(r.unknown.iter().any(|k| k == "license.gates"));
        assert!(r.changed.iter().any(|k| k == "ui.theme"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
