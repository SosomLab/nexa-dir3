//! WASM 플러그인 런타임(dir2 `preview/wasm.rs` 이식 · wasmi 1.1 · **ABI 바이트 호환 — DR-7**).
//! 탐색 폴더의 `*.wasm`(wasm32-unknown-unknown)을 파일명 순으로 로드 — `.wasm` 1개가 전 OS/아키텍처 동일 동작.
//!
//! **ABI v1** — 버퍼 = 선두 4바이트 LE 길이 + UTF-8 본문:
//! - export: `memory` · `nx_meta() -> ptr`(`id\nname\next1,ext2[\ncaps]`) · `nx_preview() -> ptr`(첫 줄 `lines`|`image`, 이후 본문 — `\u{2}종류|`·`\u{1}img|` 태그 계약)
//! - import(`env`): `read_text(ptr, cap) -> len`(대상 파일만 · 256KB) · `render_svg(sptr, slen, optr, ocap) -> len` · `is_dark() -> i32` · `disp_width(ptr, len) -> i32`
//!
//! **ABI v2 — 압축 목록**(`nx_meta` 4번째 줄 `archive`): export `nx_archive() -> ptr`(첫 줄 `archive`|`password`|`error`) ·
//! import `file_size() -> i64` · `read_at(off, ptr, cap) -> n` · `password(ptr, cap) -> n`(활성 암호만 · 없으면 -1).
//!
//! 격리: fuel 상한 · 메모리 상한(limiter) · 벽시계 상한 · 호스트 임포트 연료 과금(`host_guard`) · 오류 = 그 플러그인만 1줄 ·
//! 연속 3회 실패 = 세션 격리(브레이커). 수치는 dir2 그대로(동봉 archive.wasm 2500 멤버 ar 통과가 회귀 기준 — port/20 §6-2).

use std::path::{Path, PathBuf};
use wasmi::{Caller, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

use super::{PreviewDoc, PreviewProvider};
use ndir_i18n::trf;

/// 실행 연료 상한(호출당 — 인터프리터 명령 수. 초과 = 트랩 → 오류 1줄).
const FUEL: u64 = 200_000_000;
/// 플러그인 선형 메모리 상한(64MB).
const MEM_CAP: usize = 64 * 1024 * 1024;
/// read_text 호스트 클램프.
const READ_CAP: usize = 256 * 1024;
/// 반환 본문 상한(1000줄 상당 여유 — 도크/독립 창 보호).
const OUT_CAP: usize = 1 << 20;
/// `read_at` 1회 클램프(임의 위치 읽기).
const READ_AT_CAP: usize = 4 * 1024 * 1024;
/// `read_at` 호출당 **누적 바이트 상한**(dir2 A15 — 멤버 수천 개 ar·cpio를 통과시키되 데이터 폭주는 막는다).
const READ_AT_TOTAL_CAP: u64 = 64 * 1024 * 1024;
/// `read_at` 1회 고정 연료.
const READ_AT_FUEL_FIXED: u64 = 1_000;
/// `read_at` 바이트 비례 연료의 분모(64바이트당 1).
const READ_AT_FUEL_PER_BYTES: u64 = 64;
/// 압축 목록 항목 상한(그리드 보호 — ndir-vfs 상한과 동일).
const ARCHIVE_CAP: usize = 50_000;
/// 호출당 **벽시계 상한**(UI 스레드 실행이라 짧게 — 호스트 임포트 진입 시 검사).
pub(crate) const CALL_TIMEOUT_MS: u64 = 1_500;
/// 연속 실패 격리(서킷 브레이커).
pub(crate) const BREAKER_LIMIT: u32 = 3;
/// 모듈 크기 상한.
const MODULE_CAP: usize = 8 * 1024 * 1024;

/// 호스트 상태 — 미리보기 대상 파일(샌드박스: 이 파일 외 접근 불가) + 메모리 리미터 + 호출 마감 시각.
struct HostCtx {
    path: PathBuf,
    limits: StoreLimits,
    deadline: std::time::Instant,
    /// 이번 호출에서 `read_at`이 실제로 읽은 누적 바이트.
    read_total: u64,
}

/// 호스트 임포트 공통 게이트: ① 벽시계 상한 초과 → 트랩 ② 호스트 작업 비용을 **연료에 과금**(부족 = 트랩).
fn host_guard(caller: &mut Caller<'_, HostCtx>, cost: u64) -> Result<(), wasmi::Error> {
    if std::time::Instant::now() >= caller.data().deadline {
        return Err(wasmi::Error::new(format!(
            "call timeout {CALL_TIMEOUT_MS}ms exceeded"
        )));
    }
    let fuel = caller.get_fuel()?;
    if fuel < cost {
        return Err(wasmi::Error::new("fuel exhausted (host import)"));
    }
    caller.set_fuel(fuel - cost)
}

/// 로드된 플러그인 — 모듈은 검증·컴파일 완료 캐시(호출마다 인스턴스만 생성).
pub(crate) struct WasmPlugin {
    pub id: String,
    pub name: String,
    pub exts: Vec<String>,
    /// 능력 선언(nx_meta 4번째 줄) — `archive` = 압축 목록 공급자.
    pub caps: Vec<String>,
    module: Module,
    engine: Engine,
}

impl WasmPlugin {
    /// 압축 목록 능력을 선언했는가(ABI v2).
    pub(crate) fn is_archive(&self) -> bool {
        self.caps.iter().any(|c| c == "archive")
    }
}

/// 게스트 메모리에서 (4바이트 LE 길이 + 본문) 버퍼를 읽는다.
fn read_buf(mem: &[u8], ptr: u32) -> Option<String> {
    let p = ptr as usize;
    let len = u32::from_le_bytes(mem.get(p..p + 4)?.try_into().ok()?) as usize;
    if len > OUT_CAP {
        return None;
    }
    Some(String::from_utf8_lossy(mem.get(p + 4..p + 4 + len)?).into_owned())
}

fn guest_memory(caller: &mut Caller<'_, HostCtx>) -> Option<wasmi::Memory> {
    caller.get_export("memory").and_then(|e| e.into_memory())
}

/// 링커 구성 — 호스트 import 7종(v1 4 + v2 3).
fn linker(engine: &Engine) -> Result<Linker<HostCtx>, wasmi::Error> {
    let mut l = Linker::new(engine);
    // read_text(ptr, cap) -> len : 대상 파일 앞부분을 게스트 메모리에 기록
    l.func_wrap(
        "env",
        "read_text",
        |mut caller: Caller<'_, HostCtx>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
            host_guard(&mut caller, 200_000)?;
            let path = caller.data().path.clone();
            let cap = (cap.max(0) as usize).min(READ_CAP);
            let Ok((text, _)) = super::read_text(&path, cap.max(1)) else {
                return Ok(0);
            };
            let bytes = text.as_bytes();
            let n = bytes.len().min(cap);
            let Some(mem) = guest_memory(&mut caller) else {
                return Ok(0);
            };
            if mem.write(&mut caller, ptr as usize, &bytes[..n]).is_err() {
                return Ok(0);
            }
            Ok(n as i32)
        },
    )?;
    // render_svg(sptr, slen, optr, ocap) -> len : SVG → 이미지 경로(실패 = 0)
    l.func_wrap(
        "env",
        "render_svg",
        |mut caller: Caller<'_, HostCtx>,
         sptr: i32,
         slen: i32,
         optr: i32,
         ocap: i32|
         -> Result<i32, wasmi::Error> {
            host_guard(&mut caller, 5_000_000)?; // 래스터 + 임시 파일 — 가장 비싼 임포트
            let Some(mem) = guest_memory(&mut caller) else {
                return Ok(0);
            };
            let mut svg = vec![0u8; (slen.max(0) as usize).min(READ_CAP)];
            if mem.read(&caller, sptr as usize, &mut svg).is_err() {
                return Ok(0);
            }
            let Ok(svg) = String::from_utf8(svg) else {
                return Ok(0);
            };
            let Some(out) = super::render_svg_impl(&svg) else {
                return Ok(0);
            };
            let b = out.as_bytes();
            let n = b.len().min(ocap.max(0) as usize);
            if mem.write(&mut caller, optr as usize, &b[..n]).is_err() {
                return Ok(0);
            }
            Ok(n as i32)
        },
    )?;
    // is_dark() -> i32 : 테마 신호
    l.func_wrap(
        "env",
        "is_dark",
        |mut caller: Caller<'_, HostCtx>| -> Result<i32, wasmi::Error> {
            host_guard(&mut caller, 1_000)?;
            Ok(i32::from(super::is_dark_now()))
        },
    )?;
    // ── ABI v2 ──
    // file_size() -> i64 : 대상 파일 크기(꼬리 오프셋 계산용)
    l.func_wrap(
        "env",
        "file_size",
        |mut caller: Caller<'_, HostCtx>| -> Result<i64, wasmi::Error> {
            host_guard(&mut caller, 10_000)?;
            Ok(std::fs::metadata(&caller.data().path)
                .map(|m| m.len() as i64)
                .unwrap_or(-1))
        },
    )?;
    // read_at(off, ptr, cap) -> n : **대상 파일 임의 위치** 읽기
    l.func_wrap(
        "env",
        "read_at",
        |mut caller: Caller<'_, HostCtx>,
         off: i64,
         ptr: i32,
         cap: i32|
         -> Result<i32, wasmi::Error> {
            use std::io::{Read, Seek, SeekFrom};
            host_guard(&mut caller, READ_AT_FUEL_FIXED)?;
            if off < 0 {
                return Ok(0);
            }
            let path = caller.data().path.clone();
            let want = (cap.max(0) as usize).min(READ_AT_CAP);
            let Ok(mut f) = std::fs::File::open(&path) else {
                return Ok(0);
            };
            // 파일 끝 너머는 읽을 게 없다 — 유효 길이로 잘라 버퍼·과금 모두 실제 바이트 기준.
            let len = f.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
            let cap = len.saturating_sub(off as u64).min(want as u64) as usize;
            if cap == 0 {
                return Ok(0);
            }
            host_guard(&mut caller, (cap as u64) / READ_AT_FUEL_PER_BYTES)?;
            let total = caller.data().read_total + cap as u64;
            if total > READ_AT_TOTAL_CAP {
                return Err(wasmi::Error::new(format!(
                    "read_at total cap {}MB exceeded",
                    READ_AT_TOTAL_CAP >> 20
                )));
            }
            caller.data_mut().read_total = total;
            if f.seek(SeekFrom::Start(off as u64)).is_err() {
                return Ok(0);
            }
            let mut buf = vec![0u8; cap];
            let mut got = 0;
            while got < cap {
                match f.read(&mut buf[got..]) {
                    Ok(0) => break,
                    Ok(n) => got += n,
                    Err(_) => break,
                }
            }
            let Some(mem) = guest_memory(&mut caller) else {
                return Ok(0);
            };
            if mem.write(&mut caller, ptr as usize, &buf[..got]).is_err() {
                return Ok(0);
            }
            Ok(got as i32)
        },
    )?;
    // password(ptr, cap) -> n : **활성 암호**만 전달 · 없으면 -1 · 호스트 사본 즉시 소거
    l.func_wrap(
        "env",
        "password",
        |mut caller: Caller<'_, HostCtx>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
            host_guard(&mut caller, 10_000)?;
            let bytes = super::archive::with_active_password(|pw| pw.map(|s| s.expose().to_vec()));
            let Some(bytes) = bytes else { return Ok(-1) };
            let n = bytes.len().min(cap.max(0) as usize);
            let Some(mem) = guest_memory(&mut caller) else {
                return Ok(-1);
            };
            let ok = mem.write(&mut caller, ptr as usize, &bytes[..n]).is_ok();
            let mut bytes = bytes;
            ndir_core::secret::zeroize_bytes(&mut bytes);
            Ok(if ok { n as i32 } else { -1 })
        },
    )?;
    // disp_width(ptr, len) -> i32 : 표시 폭(CJK 2칸)
    l.func_wrap(
        "env",
        "disp_width",
        |mut caller: Caller<'_, HostCtx>, ptr: i32, len: i32| -> Result<i32, wasmi::Error> {
            host_guard(&mut caller, 10_000)?;
            let Some(mem) = guest_memory(&mut caller) else {
                return Ok(0);
            };
            let mut b = vec![0u8; (len.max(0) as usize).min(4096)];
            if mem.read(&caller, ptr as usize, &mut b).is_err() {
                return Ok(0);
            }
            Ok(super::disp_width_impl(&String::from_utf8_lossy(&b)) as i32)
        },
    )?;
    Ok(l)
}

/// 인스턴스 생성 + `fn_name() -> ptr` 호출 후 버퍼 회수(연료·메모리 상한 적용).
fn call_buf(plugin: &WasmPlugin, path: &Path, fn_name: &str) -> Result<String, String> {
    call_buf_timeout(plugin, path, fn_name, CALL_TIMEOUT_MS)
}

/// [`call_buf`]의 벽시계 상한 지정판 — 시험이 연료 과금만 떼어 검증할 때(디버그 wasmi는 수 배 느리다).
fn call_buf_timeout(
    plugin: &WasmPlugin,
    path: &Path,
    fn_name: &str,
    timeout_ms: u64,
) -> Result<String, String> {
    let ctx = HostCtx {
        path: path.to_path_buf(),
        limits: StoreLimitsBuilder::new().memory_size(MEM_CAP).build(),
        deadline: std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms),
        read_total: 0,
    };
    let mut store = Store::new(&plugin.engine, ctx);
    store.limiter(|c| &mut c.limits);
    store.set_fuel(FUEL).map_err(|e| e.to_string())?;
    let l = linker(&plugin.engine).map_err(|e| e.to_string())?;
    let instance = l
        .instantiate_and_start(&mut store, &plugin.module)
        .map_err(|e| e.to_string())?;
    let f = instance
        .get_typed_func::<(), i32>(&store, fn_name)
        .map_err(|_| format!("{fn_name}() export missing"))?;
    let ptr = f.call(&mut store, ()).map_err(|e| e.to_string())?;
    let mem = instance
        .get_memory(&store, "memory")
        .ok_or("memory export missing")?;
    read_buf(mem.data(&store), ptr as u32).ok_or_else(|| "return buffer corrupt".into())
}

/// `.wasm` 1개 로드 — 검증·컴파일 + 메타(nx_meta) 추출.
fn load_one(path: &Path) -> Result<WasmPlugin, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    if bytes.len() > MODULE_CAP {
        return Err("module exceeds 8MB".into());
    }
    let mut cfg = wasmi::Config::default();
    cfg.consume_fuel(true);
    let engine = Engine::new(&cfg);
    let module = Module::new(&engine, &bytes).map_err(|e| e.to_string())?;
    let plugin = WasmPlugin {
        id: String::new(),
        name: String::new(),
        exts: Vec::new(),
        caps: Vec::new(),
        module,
        engine,
    };
    let meta = call_buf(&plugin, Path::new(""), "nx_meta")?;
    let mut it = meta.lines();
    let id = it.next().unwrap_or_default().trim().to_string();
    if id.is_empty() {
        return Err("nx_meta: id missing".into());
    }
    let name = it.next().unwrap_or(&id).trim().to_string();
    let exts = it
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();
    // 4번째 줄 = 능력 선언(ABI v2 — 없으면 미리보기 전용 v1 플러그인 · 하위 호환)
    let caps = it
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|c| c.trim().to_ascii_lowercase())
        .filter(|c| !c.is_empty())
        .collect();
    Ok(WasmPlugin {
        id,
        name,
        exts,
        caps,
        ..plugin
    })
}

/// 디렉터리의 `*.wasm` 전부 로드(파일명 순 — 결정적). 오류는 해당 파일만 격리(`파일명: 사유`).
pub(crate) fn load_dir(dir: &Path) -> (Vec<WasmPlugin>, Vec<String>) {
    let mut plugins = Vec::new();
    let mut errors = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (plugins, errors);
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("wasm"))
        })
        .collect();
    files.sort();
    for f in files {
        match load_one(&f) {
            Ok(p) => plugins.push(p),
            Err(e) => errors.push(format!(
                "{}: {e}",
                f.file_name().unwrap_or_default().to_string_lossy()
            )),
        }
    }
    (plugins, errors)
}

/// preview 실행 — 반환 첫 줄 = `lines` | `image`, 이후 본문(태그 계약 그대로).
pub(crate) fn run_preview(plugin: &WasmPlugin, path: &Path) -> Result<PreviewDoc, String> {
    let out = call_buf(plugin, path, "nx_preview")?;
    let mut it = out.splitn(2, '\n');
    match it.next().unwrap_or("") {
        "lines" => Ok(PreviewDoc::Lines(
            it.next()
                .unwrap_or("")
                .lines()
                .take(1000)
                .map(|l| l.chars().take(4096).collect())
                .collect(),
        )),
        "image" => Ok(PreviewDoc::Image(
            it.next().unwrap_or("").trim().to_string(),
        )),
        k => Err(format!("unknown return kind: {k}")),
    }
}

/// `nx_archive()` 실행 — 압축 목록 공급자(ABI v2).
/// 항목 줄 = `경로<TAB>원본<TAB>압축<TAB>시각(Unix 초)<TAB>속성<TAB>방식`(속성 = `dir`·`enc`·`utc`·`unsafe` 쉼표 목록).
pub(crate) fn run_archive(
    plugin: &WasmPlugin,
    path: &Path,
) -> Result<super::archive::ArchiveDoc, String> {
    use super::archive::{ArchiveDoc, ArchiveStatus};
    use ndir_vfs::archive::{ArchiveEntry, Listing};

    let out = call_buf(plugin, path, "nx_archive")?;
    let mut it = out.split('\n');
    let kind = it.next().unwrap_or("").trim();
    let doc = |status, listing| ArchiveDoc {
        path: path.to_path_buf(),
        listing,
        status,
        provider: plugin.id.clone(),
    };
    match kind {
        "password" => return Ok(doc(ArchiveStatus::NeedPassword, Listing::default())),
        "error" => {
            let why = it.next().unwrap_or("").trim().to_string();
            return Ok(doc(ArchiveStatus::Failed(why), Listing::default()));
        }
        "archive" => {}
        k => return Err(format!("unknown return kind: {k}")),
    }
    let head = it.next().unwrap_or("");
    let mut hp = head.split('\t');
    let label = hp.next().unwrap_or(&plugin.name).trim().to_string();
    let flags: Vec<&str> = hp.next().unwrap_or("").split(',').map(str::trim).collect();
    let mut listing = Listing {
        format: plugin.id.clone(),
        label,
        solid: flags.contains(&"solid"),
        multivolume: flags.contains(&"multivolume"),
        truncated: flags.contains(&"truncated"),
        ..Default::default()
    };
    for line in it.take(ARCHIVE_CAP) {
        if line.trim().is_empty() {
            continue;
        }
        let mut f = line.split('\t');
        let raw = f.next().unwrap_or("");
        let num = |v: Option<&str>| v.and_then(|v| v.trim().parse::<u64>().ok());
        let size = num(f.next());
        let packed = num(f.next());
        let mtime = f.next().and_then(|v| v.trim().parse::<i64>().ok());
        let attr: Vec<&str> = f.next().unwrap_or("").split(',').map(str::trim).collect();
        let method = f.next().unwrap_or("").trim().to_string();
        let (p, suspicious) = ndir_vfs::archive::normalize_path(raw);
        if p.is_empty() {
            continue;
        }
        let is_dir = attr.contains(&"dir");
        listing.entries.push(ArchiveEntry {
            path: p,
            is_dir,
            size: (!is_dir).then_some(()).and(size),
            packed: (!is_dir).then_some(()).and(packed),
            modified: mtime.filter(|&t| t > 0),
            // 기본은 현지 벽시계(DOS 계열이 다수) — `utc` 속성이 있으면 epoch로 본다
            time_is_local: !attr.contains(&"utc"),
            encrypted: attr.contains(&"enc"),
            method,
            crc32: None,
            suspicious: suspicious || attr.contains(&"unsafe"),
        });
    }
    listing.has_encrypted = listing.entries.iter().any(|e| e.encrypted);
    Ok(doc(ArchiveStatus::Ok, listing))
}

/// WASM 플러그인 → 공급자 어댑터 — 실행 오류는 해당 플러그인만 1줄 격리 · 연속 [`BREAKER_LIMIT`]회면 세션 동안 실행 안 함.
pub(super) struct WasmProvider {
    pub plugin: WasmPlugin,
    failures: std::cell::Cell<u32>,
}

impl WasmProvider {
    pub(super) fn new(plugin: WasmPlugin) -> Self {
        WasmProvider {
            plugin,
            failures: std::cell::Cell::new(0),
        }
    }

    fn tripped(&self) -> bool {
        self.failures.get() >= BREAKER_LIMIT
    }

    fn record(&self, ok: bool) {
        self.failures
            .set(if ok { 0 } else { self.failures.get() + 1 });
    }
}

impl PreviewProvider for WasmProvider {
    fn id(&self) -> &str {
        &self.plugin.id
    }
    fn exts(&self) -> &[String] {
        &self.plugin.exts
    }
    fn preview(&self, path: &Path) -> PreviewDoc {
        if self.tripped() {
            return PreviewDoc::Lines(vec![trf(
                "preview.plugin.disabled",
                &[&self.plugin.id, &BREAKER_LIMIT.to_string()],
            )]);
        }
        let result = if self.plugin.is_archive() {
            run_archive(&self.plugin, path).map(|doc| PreviewDoc::Archive(Box::new(doc)))
        } else {
            run_preview(&self.plugin, path)
        };
        self.record(result.is_ok());
        match result {
            Ok(doc) => doc,
            Err(e) => PreviewDoc::Lines(vec![trf("preview.plugin.error", &[&self.plugin.id, &e])]),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// 시험 모듈(WAT) — 메타 + read_text/is_dark import + 무한루프·호스트 루프.
    const WAT: &str = r#"
(module
  (import "env" "read_text" (func $read (param i32 i32) (result i32)))
  (import "env" "is_dark" (func $dark (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 1024) "up\nUpper\nabc,md")
  (func (export "nx_meta") (result i32)
    (i32.store (i32.const 1020) (i32.const 15))
    (i32.const 1020))
  (func (export "nx_preview") (result i32)
    (i32.store (i32.const 2044) (i32.const 8))
    (i32.store (i32.const 2048) (i32.const 0x656e696c)) ;; "line"
    (i32.store (i32.const 2052) (i32.const 0x6b6f0a73)) ;; "s\nok"
    (i32.const 2044))
  (func (export "nx_loop") (result i32) (loop br 0) (i32.const 0))
  (func (export "nx_hostloop") (result i32) (loop (drop (call $dark)) (br 0)) (i32.const 0)))
"#;

    /// nx_preview export가 없는 모듈 — 매 호출 실패(브레이커 검증용).
    const WAT_BROKEN_PREVIEW: &str = r#"
(module
  (memory (export "memory") 1)
  (data (i32.const 1024) "bad\nBad\nabc")
  (func (export "nx_meta") (result i32)
    (i32.store (i32.const 1020) (i32.const 11))
    (i32.const 1020)))
"#;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir_wasm_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 압축 목록 ABI v2 시험 모듈(WAT) — `password` import 결과에 따라 "암호 필요" 또는 항목 2건.
    fn archive_wat() -> String {
        let meta = "arc\nArchive Sample\nfoo\narchive";
        let body = concat!(
            "archive\n",
            "FOO\tsolid\n",
            "a/b.txt\t100\t40\t1700000000\tutc\tStore\n",
            "d\t0\t0\t0\tdir\t"
        );
        let esc = |s: &str| {
            s.replace('\\', "\\\\")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        };
        format!(
            r#"
(module
  (import "env" "password" (func $pw (param i32 i32) (result i32)))
  (import "env" "read_at" (func $readat (param i64 i32 i32) (result i32)))
  (import "env" "file_size" (func $fsize (result i64)))
  (memory (export "memory") 1)
  (data (i32.const 1024) "{meta}")
  (data (i32.const 2048) "{body}")
  (data (i32.const 3072) "password")
  (func (export "nx_meta") (result i32)
    (i32.store (i32.const 1020) (i32.const {meta_len}))
    (i32.const 1020))
  (func (export "nx_archive") (result i32)
    (if (i32.lt_s (call $pw (i32.const 4096) (i32.const 64)) (i32.const 0))
      (then
        (i32.store (i32.const 3068) (i32.const 8))
        (return (i32.const 3068))))
    (drop (call $fsize))
    (drop (call $readat (i64.const 0) (i32.const 5120) (i32.const 16)))
    (i32.store (i32.const 2044) (i32.const {body_len}))
    (i32.const 2044)))
"#,
            meta = esc(meta),
            body = esc(body),
            meta_len = meta.len(),
            body_len = body.len(),
        )
    }

    #[test]
    fn archive_capability_routes_to_nx_archive_and_password_flow() {
        use super::super::archive::ArchiveStatus;
        let d = tmp_dir("arc");
        std::fs::write(d.join("arc.wasm"), wat::parse_str(archive_wat()).unwrap()).unwrap();
        let (plugins, errors) = load_dir(&d);
        assert_eq!(plugins.len(), 1, "{errors:?}");
        let p = &plugins[0];
        assert!(p.is_archive(), "nx_meta 4번째 줄 = 능력 선언");
        assert_eq!(p.exts, ["foo".to_string()]);

        let target = d.join("t.foo");
        std::fs::write(&target, b"payload").unwrap();
        let doc = run_archive(p, &target).unwrap();
        assert_eq!(doc.status, ArchiveStatus::NeedPassword);
        assert_eq!(doc.provider, "arc");

        let doc = super::super::archive::with_password_scope(
            Some(ndir_core::secret::Secret::new(b"pw".to_vec())),
            || run_archive(p, &target).unwrap(),
        );
        assert_eq!(doc.status, ArchiveStatus::Ok);
        assert_eq!(doc.listing.label, "FOO");
        assert!(doc.listing.solid);
        assert_eq!(doc.listing.entries.len(), 2);
        let f = doc
            .listing
            .entries
            .iter()
            .find(|e| e.path == "a/b.txt")
            .unwrap();
        assert_eq!(
            (f.size, f.packed, f.method.as_str()),
            (Some(100), Some(40), "Store")
        );
        assert_eq!((f.modified, f.time_is_local), (Some(1_700_000_000), false));
        let dir = doc.listing.entries.iter().find(|e| e.path == "d").unwrap();
        assert!(dir.is_dir && dir.size.is_none(), "폴더 행은 크기 없음");
        // 스코프 종료 = 슬롯 비움 → 다시 암호 요청.
        assert_eq!(
            run_archive(p, &target).unwrap().status,
            ArchiveStatus::NeedPassword
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn loads_meta_runs_preview_and_fuel_traps_infinite_loop() {
        let d = tmp_dir("basic");
        std::fs::write(d.join("up.wasm"), wat::parse_str(WAT).unwrap()).unwrap();
        std::fs::write(d.join("broken.wasm"), b"\x00asm junk").unwrap();
        std::fs::write(d.join("huge.wasm"), vec![0u8; MODULE_CAP + 1]).unwrap();
        let (plugins, errors) = load_dir(&d);
        assert_eq!(plugins.len(), 1, "{errors:?}");
        assert_eq!(errors.len(), 2, "깨진 모듈·8MB 초과 격리: {errors:?}");
        assert!(errors.iter().any(|e| e.contains("8MB")));
        let p = &plugins[0];
        assert_eq!(
            (p.id.as_str(), p.name.as_str(), p.exts.as_slice()),
            ("up", "Upper", ["abc".to_string(), "md".into()].as_slice())
        );
        assert!(!p.is_archive(), "caps 줄 없음 = v1 미리보기 전용");
        let t = d.join("t.abc");
        std::fs::write(&t, "hi").unwrap();
        match run_preview(p, &t).unwrap() {
            PreviewDoc::Lines(l) => assert_eq!(l, ["ok"]),
            _ => panic!("lines"),
        }
        let err = call_buf(p, &t, "nx_loop").unwrap_err();
        assert!(!err.is_empty(), "연료 소진 트랩: {err}");
        let t0 = std::time::Instant::now();
        let err = call_buf(p, &t, "nx_hostloop").unwrap_err();
        let dt = t0.elapsed();
        assert!(!err.is_empty(), "호스트 루프 트랩: {err}");
        assert!(
            dt.as_millis() < (CALL_TIMEOUT_MS as u128) * 4,
            "호스트 루프가 상한 안에 끝나야 한다: {dt:?}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// ar 멤버 1건(60B 헤더 + 데이터, 짝수 정렬).
    pub(in crate::preview) fn ar_member(name: &str, data: &[u8]) -> Vec<u8> {
        let mut h = format!(
            "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}",
            name,
            1_700_000_000u64,
            0,
            0,
            100644,
            data.len()
        );
        h.push('`');
        h.push('\n');
        let mut v = h.into_bytes();
        v.extend_from_slice(data);
        if v.len() % 2 != 0 {
            v.push(b'\n');
        }
        v
    }

    /// 동봉 archive.wasm은 ar 멤버마다 `read_at` 1회 — 2500 멤버 합성 ar가 Ok여야 한다(dir2 A15 회귀 기준).
    #[test]
    fn read_at_fuel_allows_thousands_of_members() {
        let dist = super::super::sample_tests::bundled_dir().join("archive.wasm");
        let d = tmp_dir("ar2500");
        std::fs::copy(&dist, d.join("archive.wasm")).unwrap();
        let (plugins, errors) = load_dir(&d);
        assert!(errors.is_empty(), "{errors:?}");
        let p = &plugins[0];
        assert!(p.is_archive());

        const N: usize = 2500;
        let mut ar = b"!<arch>\n".to_vec();
        for i in 0..N {
            ar.extend(ar_member(&format!("m{i}.o/"), b"xx"));
        }
        let f = d.join("big.a");
        std::fs::write(&f, &ar).unwrap();
        let out = call_buf_timeout(p, &f, "nx_archive", 60_000)
            .unwrap_or_else(|e| panic!("{N}멤버 ar 목록 실패: {e}"));
        let mut it = out.lines();
        assert_eq!(it.next(), Some("archive"), "{}", &out[..out.len().min(80)]);
        assert_eq!(it.next().map(|h| h.starts_with("ar\t")), Some(true));
        let rows: Vec<&str> = it.collect();
        assert_eq!(rows.len(), N, "멤버 전부 나열");
        assert!(
            rows[N - 1].starts_with(&format!("m{}.o\t", N - 1)),
            "{}",
            rows[N - 1]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `read_at`을 끝없이 부르는 모듈 — 누적 바이트 상한 검증용(메모리 70페이지 = 4.4MB).
    const WAT_READ_LOOP: &str = r#"
(module
  (import "env" "read_at" (func $readat (param i64 i32 i32) (result i32)))
  (memory (export "memory") 70)
  (data (i32.const 1024) "rl\nReadLoop\nbin")
  (func (export "nx_meta") (result i32)
    (i32.store (i32.const 1020) (i32.const 15))
    (i32.const 1020))
  (func (export "nx_readloop") (result i32)
    (loop (drop (call $readat (i64.const 0) (i32.const 4096) (i32.const 4194304))) (br 0))
    (i32.const 0)))
"#;

    #[test]
    fn read_at_total_bytes_cap_traps_runaway_reads() {
        let d = tmp_dir("rl");
        std::fs::write(d.join("rl.wasm"), wat::parse_str(WAT_READ_LOOP).unwrap()).unwrap();
        let (plugins, errors) = load_dir(&d);
        assert_eq!(plugins.len(), 1, "{errors:?}");
        let t = d.join("t.bin");
        std::fs::write(&t, vec![7u8; READ_AT_CAP]).unwrap();
        let err = call_buf(&plugins[0], &t, "nx_readloop").unwrap_err();
        assert!(err.contains("read_at total cap"), "누적 상한 트랩: {err}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn breaker_disables_plugin_after_consecutive_failures_and_recovers() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let d = tmp_dir("brk");
        std::fs::write(
            d.join("bad.wasm"),
            wat::parse_str(WAT_BROKEN_PREVIEW).unwrap(),
        )
        .unwrap();
        let (mut plugins, _) = load_dir(&d);
        let prov = WasmProvider::new(plugins.remove(0));
        let t = d.join("x.abc");
        std::fs::write(&t, "hi").unwrap();
        for i in 0..BREAKER_LIMIT {
            match prov.preview(&t) {
                PreviewDoc::Lines(l) => {
                    assert!(l[0].contains("nx_preview"), "{i}: 실행 오류 1줄: {l:?}")
                }
                _ => panic!("lines"),
            }
        }
        assert!(prov.tripped(), "연속 {BREAKER_LIMIT}회 실패 → 격리");
        match prov.preview(&t) {
            PreviewDoc::Lines(l) => {
                assert!(!l[0].contains("nx_preview"), "격리 안내로 대체: {l:?}")
            }
            _ => panic!("lines"),
        }
        // 복귀 경로: 성공 1회 = 카운터 0(격리 해제).
        prov.record(true);
        assert!(!prov.tripped());
        let _ = std::fs::remove_dir_all(&d);
    }
}
