//! 압축 풀기(T-169 1차 · NEW-040 · dir3 신규 · 사용자 10-05 "대상 목록") — **zip(Store · Deflate) · tar · gzip(단일 파일 ·
//! `.tar.gz`/`.tgz`)**. 외부 crate 없이(DR-8) RFC 1951 inflate를 직접 구현했다(참조 구현 `puff` 방식 — 명료함 우선 · 디스크 쓰기가
//! 보통 더 느리다). 7z(LZMA) · rar · cab · bzip2 · xz는 코덱이 커서 다음 차수(플러그인 또는 2차).
//!
//! 안전: 항목 경로는 [`normalize_path`]로 정규화하고 **탈출 시도(`..` · 절대 경로)는 건너뛴다**(zip slip) · 이미 있는 파일은 덮어쓰지
//! 않고 건너뛴다(1차 — 충돌 질문은 2차) · 심볼릭 링크 항목은 만들지 않는다 · 항목 하나의 압축 해제 결과는 메모리에 올리므로
//! [`MAX_ENTRY`]를 넘는 항목은 건너뛴다(보고에 남는다).

use super::{
    decode_name, normalize_path, read_exact_at, u16le, u32le, u64le, ArchiveError, FileSource,
    ReadAt, SliceSource,
};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// 항목 하나(압축 해제 뒤)의 상한 — 1 GiB.
pub const MAX_ENTRY: u64 = 1 << 30;

/// 풀기 결과 보고.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    /// 이미 있어서 건너뛴 파일.
    pub skipped_existing: u64,
    /// 지원하지 않는 방식(암호화 · Deflate64 · LZMA · 링크 · 너무 큼)이라 건너뛴 항목.
    pub unsupported: u64,
    /// 경로 탈출 시도라 건너뛴 항목.
    pub suspicious: u64,
    /// 항목별 실패(경로 · 사유) — 하나의 실패가 전체를 멈추지 않는다.
    pub errors: Vec<(String, String)>,
    pub canceled: bool,
}

/// 이 파일을 1차 풀기가 다룰 수 있는가(확장자 또는 시그니처).
#[must_use]
pub fn supported(path: &Path) -> bool {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(ext.as_str(), "zip" | "tar" | "gz" | "tgz") {
        return true;
    }
    let Ok(src) = FileSource::open(path) else {
        return false;
    };
    kind_of(&src, &ext).is_some()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Zip,
    Tar,
    Gzip,
}

fn kind_of(src: &dyn ReadAt, ext: &str) -> Option<Kind> {
    let head = super::read_head(src);
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        return Some(Kind::Zip);
    }
    if head.starts_with(&[0x1F, 0x8B, 0x08]) {
        return Some(Kind::Gzip);
    }
    if is_tar(src) || ext == "tar" {
        return Some(Kind::Tar);
    }
    None
}

fn is_tar(src: &dyn ReadAt) -> bool {
    if src.size() < 512 {
        return false;
    }
    let Ok(h) = read_exact_at(src, 0, 512) else {
        return false;
    };
    h.get(257..262) == Some(b"ustar") || super::tar::checksum_ok(&h)
}

/// 항목의 데이터 위치.
#[derive(Debug, Clone)]
enum Data {
    /// zip 로컬 헤더 위치 · 압축 크기 · 방식 · 암호화.
    ZipLocal {
        off: u64,
        packed: u64,
        method: u16,
        encrypted: bool,
    },
    /// 원본(또는 풀어 둔 메모리) 안의 구간.
    Range { off: u64, len: u64 },
    /// 이미 풀린 바이트(gzip 단일 파일).
    Bytes(Vec<u8>),
    /// 만들지 않는 항목(링크 등).
    Unsupported,
}

#[derive(Debug, Clone)]
struct Item {
    path: String,
    is_dir: bool,
    size: u64,
    mtime: Option<i64>,
    data: Data,
}

/// 풀기 계획 — 항목 목록 + 데이터 원본.
struct Plan {
    items: Vec<Item>,
    /// 범위를 읽을 원본(파일 또는 풀어 둔 tar 바이트).
    src: Source,
}

enum Source {
    File(FileSource),
    Mem(Vec<u8>),
}

impl Source {
    fn read(&self, off: u64, len: u64) -> Result<Vec<u8>, ArchiveError> {
        match self {
            Source::File(f) => read_exact_at(f, off, len as usize),
            Source::Mem(m) => SliceSource(m).read_range(off, len),
        }
    }
}

impl SliceSource<'_> {
    fn read_range(&self, off: u64, len: u64) -> Result<Vec<u8>, ArchiveError> {
        read_exact_at(self, off, len as usize)
    }
}

fn plan(path: &Path) -> Result<Plan, ArchiveError> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let src = FileSource::open(path)?;
    let Some(kind) = kind_of(&src, &ext) else {
        return Err(ArchiveError::NotArchive);
    };
    match kind {
        Kind::Zip => Ok(Plan {
            items: zip_items(&src)?,
            src: Source::File(src),
        }),
        Kind::Tar => Ok(Plan {
            items: tar_items(&src)?,
            src: Source::File(src),
        }),
        Kind::Gzip => {
            let size = src.size();
            if size > MAX_ENTRY {
                return Err(ArchiveError::Corrupt("gzip 파일이 너무 큼".into()));
            }
            let whole = read_exact_at(&src, 0, size as usize)?;
            let (name, payload) = gzip_header(&whole)?;
            let bytes = inflate(payload)?;
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "data".into());
            let inner_is_tar = ext == "tgz"
                || stem.to_ascii_lowercase().ends_with(".tar")
                || (bytes.len() >= 512 && bytes.get(257..262) == Some(b"ustar"));
            if inner_is_tar {
                let items = tar_items(&SliceSource(&bytes))?;
                return Ok(Plan {
                    items,
                    src: Source::Mem(bytes),
                });
            }
            let name = name.unwrap_or_else(|| {
                if ext == "gz" {
                    stem
                } else {
                    format!("{stem}.out")
                }
            });
            let mtime = u32le(&whole, 4).filter(|&t| t != 0).map(i64::from);
            Ok(Plan {
                items: vec![Item {
                    path: name,
                    is_dir: false,
                    size: bytes.len() as u64,
                    mtime,
                    data: Data::Bytes(bytes),
                }],
                src: Source::Mem(Vec::new()),
            })
        }
    }
}

/// 풀기 전 전체 바이트(진행 막대의 분모) — 모르는 항목은 0.
pub fn total_bytes(path: &Path) -> Result<u64, ArchiveError> {
    Ok(plan(path)?.items.iter().map(|i| i.size).sum())
}

/// `path`를 `dest` 폴더 안에 푼다 — `on_bytes` = 쓴 증분 · `cancel` = 항목 사이와 큰 항목 안에서 확인.
pub fn extract(
    path: &Path,
    dest: &Path,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> Result<Report, ArchiveError> {
    let plan = plan(path)?;
    std::fs::create_dir_all(dest).map_err(|e| ArchiveError::Io(e.to_string()))?;
    let mut rep = Report::default();
    for item in plan.items {
        if cancel.load(Ordering::Relaxed) {
            rep.canceled = true;
            break;
        }
        let (norm, suspicious) = normalize_path(&item.path);
        if suspicious || norm.is_empty() {
            rep.suspicious += 1;
            continue;
        }
        let target: PathBuf = dest.join(norm.replace('/', std::path::MAIN_SEPARATOR_STR));
        if item.is_dir {
            match std::fs::create_dir_all(&target) {
                Ok(()) => rep.dirs += 1,
                Err(e) => rep.errors.push((item.path.clone(), e.to_string())),
            }
            continue;
        }
        if matches!(item.data, Data::Unsupported) || item.size > MAX_ENTRY {
            rep.unsupported += 1;
            continue;
        }
        if target.symlink_metadata().is_ok() {
            rep.skipped_existing += 1;
            continue;
        }
        if let Some(parent) = target.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                rep.errors.push((item.path.clone(), e.to_string()));
                continue;
            }
        }
        match write_item(&plan.src, &item, &target, on_bytes, cancel) {
            Ok(n) => {
                rep.files += 1;
                rep.bytes += n;
            }
            Err(ArchiveError::NeedsCodec(..)) => rep.unsupported += 1,
            Err(ArchiveError::Io(e)) if e == "canceled" => {
                let _ = std::fs::remove_file(&target);
                rep.canceled = true;
                break;
            }
            Err(e) => {
                let _ = std::fs::remove_file(&target);
                rep.errors.push((item.path.clone(), format!("{e:?}")));
            }
        }
    }
    Ok(rep)
}

fn canceled() -> ArchiveError {
    ArchiveError::Io("canceled".into())
}

/// 항목 하나를 쓴다 — 쓴 바이트.
fn write_item(
    src: &Source,
    item: &Item,
    target: &Path,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> Result<u64, ArchiveError> {
    let io = |e: std::io::Error| ArchiveError::Io(e.to_string());
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(io)?;
    let mut written = 0u64;
    let mut put = |out: &mut std::fs::File, data: &[u8]| -> Result<(), ArchiveError> {
        for chunk in data.chunks(1 << 20) {
            if cancel.load(Ordering::Relaxed) {
                return Err(canceled());
            }
            out.write_all(chunk).map_err(io)?;
            written += chunk.len() as u64;
            on_bytes(chunk.len() as u64);
        }
        Ok(())
    };
    match &item.data {
        Data::Bytes(b) => put(&mut out, b)?,
        Data::Range { off, len } => {
            let mut at = *off;
            let end = off + len;
            while at < end {
                let n = (end - at).min(1 << 20);
                let buf = src.read(at, n)?;
                put(&mut out, &buf)?;
                at += n;
            }
        }
        Data::ZipLocal {
            off,
            packed,
            method,
            encrypted,
        } => {
            if *encrypted {
                return Err(ArchiveError::NeedsCodec("ZIP".into(), "암호".into()));
            }
            let lh = src.read(*off, 30)?;
            if u32le(&lh, 0) != Some(0x0403_4B50) {
                return Err(ArchiveError::Corrupt("로컬 헤더 없음".into()));
            }
            let nlen = u64::from(u16le(&lh, 26).unwrap_or(0));
            let xlen = u64::from(u16le(&lh, 28).unwrap_or(0));
            let data_off = off + 30 + nlen + xlen;
            match method {
                0 => {
                    let mut at = data_off;
                    let end = data_off + packed;
                    while at < end {
                        let n = (end - at).min(1 << 20);
                        let buf = src.read(at, n)?;
                        put(&mut out, &buf)?;
                        at += n;
                    }
                }
                8 => {
                    if *packed > MAX_ENTRY {
                        return Err(ArchiveError::NeedsCodec("ZIP".into(), "크기".into()));
                    }
                    // ★ 스트리밍(T-179 D): 압축 입력만 메모리에 · 출력은 32 KiB 창을 넘는 대로 파일로(취소 · 진행은 `put`이 본다).
                    let comp = src.read(data_off, *packed)?;
                    inflate_to(&comp, &mut |chunk| put(&mut out, chunk))?;
                }
                _ => {
                    return Err(ArchiveError::NeedsCodec(
                        "ZIP".into(),
                        format!("method {method}"),
                    ))
                }
            }
        }
        Data::Unsupported => return Err(ArchiveError::NeedsCodec("".into(), "".into())),
    }
    drop(out);
    if let Some(t) = item.mtime {
        if let Ok(f) = std::fs::OpenOptions::new().write(true).open(target) {
            let _ = f.set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(t.max(0) as u64),
            );
        }
    }
    Ok(written)
}

// ── zip ──────────────────────────────────────────────────────────────────────

fn zip_items(src: &dyn ReadAt) -> Result<Vec<Item>, ArchiveError> {
    let (tail, pos, eocd_abs) = super::zip::find_eocd(src)?;
    let e = &tail[pos..];
    let mut cd_size = u64::from(u32le(e, 12).unwrap_or(0));
    let mut cd_off = u64::from(u32le(e, 16).unwrap_or(0));
    if cd_size == 0xFFFF_FFFF || cd_off == 0xFFFF_FFFF {
        if let Some(loc_at) = pos.checked_sub(20) {
            if u32le(&tail, loc_at) == Some(0x0706_4B50) {
                let z64 = u64le(&tail, loc_at + 8).unwrap_or(0);
                let hdr = read_exact_at(src, z64, 56)?;
                if u32le(&hdr, 0) == Some(0x0606_4B50) {
                    cd_size = u64le(&hdr, 40).unwrap_or(cd_size);
                    cd_off = u64le(&hdr, 48).unwrap_or(cd_off);
                }
            }
        }
    }
    if cd_size as usize > super::MAX_CHUNK {
        return Err(ArchiveError::Corrupt("중앙 디렉터리 과대".into()));
    }
    // SFX 보정(zip.rs와 같다).
    let mut base = 0u64;
    if cd_off.checked_add(cd_size) != Some(eocd_abs) {
        if let Some(delta) = eocd_abs.checked_sub(cd_size) {
            base = delta.saturating_sub(cd_off);
            cd_off = delta;
        }
    }
    let cd = read_exact_at(src, cd_off, cd_size as usize)?;
    let mut items = Vec::new();
    let mut p = 0usize;
    while p + 46 <= cd.len() {
        if u32le(&cd, p) != Some(0x0201_4B50) {
            break;
        }
        let flags = u16le(&cd, p + 8).unwrap_or(0);
        let method = u16le(&cd, p + 10).unwrap_or(0);
        let dos_time = u16le(&cd, p + 12).unwrap_or(0);
        let dos_date = u16le(&cd, p + 14).unwrap_or(0);
        let mut packed = u64::from(u32le(&cd, p + 20).unwrap_or(0));
        let mut size = u64::from(u32le(&cd, p + 24).unwrap_or(0));
        let nlen = u16le(&cd, p + 28).unwrap_or(0) as usize;
        let xlen = u16le(&cd, p + 30).unwrap_or(0) as usize;
        let clen = u16le(&cd, p + 32).unwrap_or(0) as usize;
        let mut off = u64::from(u32le(&cd, p + 42).unwrap_or(0));
        let name_b = cd.get(p + 46..p + 46 + nlen).unwrap_or(&[]);
        let extra = cd.get(p + 46 + nlen..p + 46 + nlen + xlen).unwrap_or(&[]);
        // Zip64 확장(0x0001): 포화된 필드 순서대로 u64.
        let mut q = 0usize;
        while q + 4 <= extra.len() {
            let id = u16le(extra, q).unwrap_or(0);
            let len = u16le(extra, q + 2).unwrap_or(0) as usize;
            if id == 0x0001 {
                let mut r = q + 4;
                for field in [&mut size, &mut packed, &mut off] {
                    if *field == 0xFFFF_FFFF {
                        if let Some(v) = u64le(extra, r) {
                            *field = v;
                        }
                        r += 8;
                    }
                }
            }
            q += 4 + len;
        }
        let name = decode_name(name_b, flags & 0x0800 != 0);
        let is_dir = name.ends_with('/') || name.ends_with('\\');
        // 시각은 DOS 현지 시각 — 시간대를 모르므로 UTC로 적는다(목록과 같은 한계).
        let mtime = super::dos_to_unix(dos_date, dos_time);
        items.push(Item {
            path: name,
            is_dir,
            size,
            mtime,
            data: Data::ZipLocal {
                off: off + base,
                packed,
                method,
                encrypted: flags & 0x0001 != 0,
            },
        });
        p += 46 + nlen + xlen + clen;
    }
    Ok(items)
}

// ── tar ──────────────────────────────────────────────────────────────────────

fn tar_items(src: &dyn ReadAt) -> Result<Vec<Item>, ArchiveError> {
    let mut items = Vec::new();
    let mut off = 0u64;
    let mut long_name: Option<String> = None;
    let mut pax_path: Option<String> = None;
    let size = src.size();
    while off + 512 <= size {
        let h = read_exact_at(src, off, 512)?;
        if h.iter().all(|&b| b == 0) {
            break;
        }
        let entry_size = super::tar::numeric(&h[124..136]).unwrap_or(0);
        let typeflag = h[156];
        let data_off = off + 512;
        let padded = entry_size.div_ceil(512) * 512;
        match typeflag {
            b'L' => {
                let b = read_exact_at(src, data_off, entry_size.min(1 << 16) as usize)?;
                long_name = Some(decode_name(super::tar::cstr(&b), true));
            }
            b'x' => {
                let b = read_exact_at(src, data_off, entry_size.min(1 << 16) as usize)?;
                for (k, v) in super::tar::parse_pax(&b) {
                    if k == "path" {
                        pax_path = Some(v);
                    }
                }
            }
            _ => {
                let mut name = decode_name(super::tar::cstr(&h[0..100]), true);
                let prefix = decode_name(super::tar::cstr(&h[345..500]), true);
                if h.get(257..262) == Some(b"ustar") && !prefix.is_empty() {
                    name = format!("{prefix}/{name}");
                }
                if let Some(n) = pax_path.take().or_else(|| long_name.take()) {
                    name = n;
                }
                let mtime = super::tar::numeric(&h[136..148]).map(|v| v as i64);
                let is_dir = typeflag == b'5' || name.ends_with('/');
                let data = match typeflag {
                    b'0' | 0 | b'7' => Data::Range {
                        off: data_off,
                        len: entry_size,
                    },
                    b'5' => Data::Range { off: 0, len: 0 },
                    _ => Data::Unsupported, // 링크 · 장치 등
                };
                items.push(Item {
                    path: name,
                    is_dir,
                    size: if is_dir { 0 } else { entry_size },
                    mtime,
                    data,
                });
            }
        }
        off = data_off + padded;
    }
    Ok(items)
}

// ── gzip ─────────────────────────────────────────────────────────────────────

/// gzip 헤더를 벗긴다 — (FNAME · DEFLATE 본문[CRC32 · ISIZE 꼬리 제외]).
fn gzip_header(whole: &[u8]) -> Result<(Option<String>, &[u8]), ArchiveError> {
    if whole.len() < 18 || !whole.starts_with(&[0x1F, 0x8B, 0x08]) {
        return Err(ArchiveError::NotArchive);
    }
    let flg = whole[3];
    let mut p = 10usize;
    if flg & 0x04 != 0 {
        let xlen = u16le(whole, p).unwrap_or(0) as usize;
        p += 2 + xlen;
    }
    let mut name = None;
    if flg & 0x08 != 0 {
        let end = whole[p.min(whole.len())..]
            .iter()
            .position(|&b| b == 0)
            .map(|i| p + i)
            .unwrap_or(whole.len());
        let s = decode_name(whole.get(p..end).unwrap_or(&[]), false);
        if !s.is_empty() {
            name = Some(s);
        }
        p = end + 1;
    }
    if flg & 0x10 != 0 {
        let end = whole[p.min(whole.len())..]
            .iter()
            .position(|&b| b == 0)
            .map(|i| p + i)
            .unwrap_or(whole.len());
        p = end + 1;
    }
    if flg & 0x02 != 0 {
        p += 2;
    }
    if p + 8 > whole.len() {
        return Err(ArchiveError::Corrupt("gzip 헤더 손상".into()));
    }
    Ok((name, &whole[p..whole.len() - 8]))
}

// ── inflate(RFC 1951) ────────────────────────────────────────────────────────

const MAXBITS: usize = 15;
const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn corrupt(what: &str) -> ArchiveError {
    ArchiveError::Corrupt(format!("deflate: {what}"))
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    buf: u32,
    cnt: u32,
}

impl Bits<'_> {
    fn bits(&mut self, n: u32) -> Result<u32, ArchiveError> {
        while self.cnt < n {
            let b = *self.d.get(self.pos).ok_or_else(|| corrupt("입력 끝"))?;
            self.pos += 1;
            self.buf |= u32::from(b) << self.cnt;
            self.cnt += 8;
        }
        let v = self.buf & ((1u32 << n) - 1);
        self.buf >>= n;
        self.cnt -= n;
        Ok(v)
    }
}

/// 정준 허프먼 표(길이별 개수 + 길이순 기호).
struct Huff {
    count: [u16; MAXBITS + 1],
    symbol: Vec<u16>,
}

impl Huff {
    fn new(lengths: &[u8]) -> Result<Huff, ArchiveError> {
        let mut count = [0u16; MAXBITS + 1];
        for &l in lengths {
            count[l as usize] += 1;
        }
        count[0] = 0;
        // 과다 할당 검사(불완전 코드는 허용 — 기호 1개 코드).
        let mut left: i32 = 1;
        for &c in &count[1..] {
            left <<= 1;
            left -= i32::from(c);
            if left < 0 {
                return Err(corrupt("허프먼 코드 과다"));
            }
        }
        let mut offs = [0u16; MAXBITS + 1];
        for l in 1..MAXBITS {
            offs[l + 1] = offs[l] + count[l];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huff { count, symbol })
    }

    fn decode(&self, b: &mut Bits<'_>) -> Result<u16, ArchiveError> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..=MAXBITS {
            code |= b.bits(1)? as i32;
            let count = i32::from(self.count[len]);
            if code - count < first {
                return Ok(self.symbol[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(corrupt("허프먼 코드 불일치"))
    }
}

/// DEFLATE 최대 뒤참조 거리(RFC 1951 §3.2.5) — 스트리밍 출력이 남겨 둬야 하는 창.
const WINDOW: usize = 32 * 1024;
/// 스트리밍 출력이 창을 넘어 쌓이면 흘려보내는 문턱(1 MiB · 쓰기 호출 수와 메모리의 절충).
const FLUSH_AT: usize = 1 << 20;

/// inflate 출력(★ T-179 D 스트리밍): `sink`가 있으면 **마지막 [`WINDOW`]만 남기고** 넘치는 앞부분을 바로 흘려보낸다(압축 해제 결과를
/// 통째로 메모리에 쌓지 않는다 — 종전 = 항목 하나당 최대 1 GiB `Vec`) · 없으면 전부 모은다([`inflate`] · 짧은 입력 · 시험).
struct Out<'s> {
    hist: Vec<u8>,
    sink: Option<Sink<'s>>,
    /// 지금까지 낸 바이트(상한 판정 · 진행).
    total: u64,
}

/// 스트리밍 출력 받는 쪽(조각을 쓰고 · 취소/쓰기 오류는 `Err`).
pub type Sink<'s> = &'s mut dyn FnMut(&[u8]) -> Result<(), ArchiveError>;

impl<'s> Out<'s> {
    fn new(sink: Option<Sink<'s>>) -> Self {
        Out {
            hist: Vec::with_capacity(if sink.is_some() { FLUSH_AT + WINDOW } else { 0 }),
            sink,
            total: 0,
        }
    }

    fn push(&mut self, b: u8) -> Result<(), ArchiveError> {
        self.hist.push(b);
        self.total += 1;
        self.maybe_flush()
    }

    fn extend(&mut self, data: &[u8]) -> Result<(), ArchiveError> {
        self.hist.extend_from_slice(data);
        self.total += data.len() as u64;
        self.maybe_flush()
    }

    /// 뒤참조 복사(`d` 바이트 앞에서 `len`개 · 겹침 허용) — 거리가 지금까지 낸 출력보다 멀면 손상.
    fn copy_back(&mut self, d: usize, len: usize) -> Result<(), ArchiveError> {
        if d == 0 || d > self.hist.len() {
            return Err(corrupt("거리가 출력보다 멀다"));
        }
        let start = self.hist.len() - d;
        for k in 0..len {
            let c = self.hist[start + k];
            self.hist.push(c);
        }
        self.total += len as u64;
        self.maybe_flush()
    }

    fn maybe_flush(&mut self) -> Result<(), ArchiveError> {
        if let Some(sink) = self.sink.as_mut() {
            if self.hist.len() > FLUSH_AT {
                let keep = self.hist.len() - WINDOW;
                sink(&self.hist[..keep])?;
                self.hist.drain(..keep);
            }
        }
        Ok(())
    }

    /// 끝 — 스트리밍이면 남은 창을 흘려보내고 빈 벡터 · 아니면 모은 전부.
    fn finish(mut self) -> Result<Vec<u8>, ArchiveError> {
        if let Some(sink) = self.sink.as_mut() {
            sink(&self.hist)?;
            return Ok(Vec::new());
        }
        Ok(std::mem::take(&mut self.hist))
    }
}

/// RFC 1951 원시 DEFLATE 스트림을 푼다(전부 메모리에 — 짧은 입력 · gzip 머리 판별 · 시험).
pub fn inflate(data: &[u8]) -> Result<Vec<u8>, ArchiveError> {
    let mut out = Out::new(None);
    inflate_into(data, &mut out)?;
    out.finish()
}

/// RFC 1951 원시 DEFLATE 스트림을 풀어 `sink`로 흘려보낸다(★ T-179 D — 메모리 = 압축 입력 + 32 KiB 창 + 1 MiB 버퍼) · 돌려주는 값 =
/// 낸 바이트. 취소·쓰기 오류는 `sink`가 `Err`로 돌려 전파한다.
pub fn inflate_to(data: &[u8], sink: Sink<'_>) -> Result<u64, ArchiveError> {
    let mut out = Out::new(Some(sink));
    inflate_into(data, &mut out)?;
    let total = out.total;
    out.finish()?;
    Ok(total)
}

fn inflate_into(data: &[u8], out: &mut Out<'_>) -> Result<(), ArchiveError> {
    let mut b = Bits {
        d: data,
        pos: 0,
        buf: 0,
        cnt: 0,
    };
    loop {
        let last = b.bits(1)? == 1;
        match b.bits(2)? {
            0 => {
                // 저장 블록: 바이트 경계 · LEN · NLEN.
                b.buf = 0;
                b.cnt = 0;
                let len = u16le(data, b.pos).ok_or_else(|| corrupt("입력 끝"))? as usize;
                let nlen = u16le(data, b.pos + 2).ok_or_else(|| corrupt("입력 끝"))? as usize;
                if len != (!nlen & 0xFFFF) {
                    return Err(corrupt("저장 블록 길이 불일치"));
                }
                b.pos += 4;
                let chunk = data
                    .get(b.pos..b.pos + len)
                    .ok_or_else(|| corrupt("입력 끝"))?;
                out.extend(chunk)?;
                b.pos += len;
            }
            1 => {
                let mut lengths = [0u8; 288];
                lengths[..144].fill(8);
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                lengths[280..].fill(8);
                let lit = Huff::new(&lengths)?;
                let dist = Huff::new(&[5u8; 30])?;
                codes(&mut b, out, &lit, &dist)?;
            }
            2 => {
                let nlen = b.bits(5)? as usize + 257;
                let ndist = b.bits(5)? as usize + 1;
                let ncode = b.bits(4)? as usize + 4;
                if nlen > 286 || ndist > 30 {
                    return Err(corrupt("코드 수 초과"));
                }
                let mut cl = [0u8; 19];
                for &i in CL_ORDER.iter().take(ncode) {
                    cl[i] = b.bits(3)? as u8;
                }
                let clh = Huff::new(&cl)?;
                let mut lengths = vec![0u8; nlen + ndist];
                let mut i = 0usize;
                while i < nlen + ndist {
                    let sym = clh.decode(&mut b)?;
                    match sym {
                        0..=15 => {
                            lengths[i] = sym as u8;
                            i += 1;
                        }
                        16 => {
                            if i == 0 {
                                return Err(corrupt("반복할 길이 없음"));
                            }
                            let prev = lengths[i - 1];
                            let n = 3 + b.bits(2)? as usize;
                            if i + n > lengths.len() {
                                return Err(corrupt("반복 초과"));
                            }
                            lengths[i..i + n].fill(prev);
                            i += n;
                        }
                        17 | 18 => {
                            let n = if sym == 17 {
                                3 + b.bits(3)? as usize
                            } else {
                                11 + b.bits(7)? as usize
                            };
                            if i + n > lengths.len() {
                                return Err(corrupt("반복 초과"));
                            }
                            i += n;
                        }
                        _ => return Err(corrupt("길이 코드")),
                    }
                }
                if lengths[256] == 0 {
                    return Err(corrupt("블록 끝 코드 없음"));
                }
                let lit = Huff::new(&lengths[..nlen])?;
                let dist = Huff::new(&lengths[nlen..])?;
                codes(&mut b, out, &lit, &dist)?;
            }
            _ => return Err(corrupt("블록 종류 3")),
        }
        if out.total > MAX_ENTRY {
            return Err(corrupt("결과가 너무 큼"));
        }
        if last {
            return Ok(());
        }
    }
}

fn codes(b: &mut Bits<'_>, out: &mut Out<'_>, lit: &Huff, dist: &Huff) -> Result<(), ArchiveError> {
    loop {
        let sym = lit.decode(b)? as usize;
        if sym < 256 {
            out.push(sym as u8)?;
        } else if sym == 256 {
            return Ok(());
        } else {
            let si = sym - 257;
            if si >= 29 {
                return Err(corrupt("길이 기호"));
            }
            let len = LEN_BASE[si] as usize + b.bits(u32::from(LEN_EXTRA[si]))? as usize;
            let ds = dist.decode(b)? as usize;
            if ds >= 30 {
                return Err(corrupt("거리 기호"));
            }
            let d = DIST_BASE[ds] as usize + b.bits(u32::from(DIST_EXTRA[ds]))? as usize;
            out.copy_back(d, len)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXED_ABC: &[u8] = &[0x4b, 0x4c, 0x4a, 0x06, 0x00];
    const DYN_FOX: &[u8] = &[
        0x0b, 0xc9, 0x48, 0x55, 0x28, 0x2c, 0xcd, 0x4c, 0xce, 0x56, 0x48, 0x2a, 0xca, 0x2f, 0xcf,
        0x53, 0x48, 0xcb, 0xaf, 0x50, 0xc8, 0x2a, 0xcd, 0x2d, 0x28, 0x56, 0xc8, 0x2f, 0x4b, 0x2d,
        0x52, 0x28, 0x01, 0x4a, 0xe7, 0x24, 0x56, 0x55, 0x2a, 0xa4, 0xe4, 0xa7, 0xeb, 0x29, 0x84,
        0x8c, 0x2a, 0x1e, 0x55, 0x3c, 0xaa, 0x78, 0x54, 0xf1, 0xa8, 0xe2, 0x51, 0xc5, 0xc3, 0x4b,
        0x31, 0x00,
    ];
    const MIXED_256: &[u8] = &[
        0x63, 0x60, 0x64, 0x62, 0x66, 0x61, 0x65, 0x63, 0xe7, 0xe0, 0xe4, 0xe2, 0xe6, 0xe1, 0xe5,
        0xe3, 0x17, 0x10, 0x14, 0x12, 0x16, 0x11, 0x15, 0x13, 0x97, 0x90, 0x94, 0x92, 0x96, 0x91,
        0x95, 0x93, 0x57, 0x50, 0x54, 0x52, 0x56, 0x51, 0x55, 0x53, 0xd7, 0xd0, 0xd4, 0xd2, 0xd6,
        0xd1, 0xd5, 0xd3, 0x37, 0x30, 0x34, 0x32, 0x36, 0x31, 0x35, 0x33, 0xb7, 0xb0, 0xb4, 0xb2,
        0xb6, 0xb1, 0xb5, 0xb3, 0x77, 0x70, 0x74, 0x72, 0x76, 0x71, 0x75, 0x73, 0xf7, 0xf0, 0xf4,
        0xf2, 0xf6, 0xf1, 0xf5, 0xf3, 0x0f, 0x08, 0x0c, 0x0a, 0x0e, 0x09, 0x0d, 0x0b, 0x8f, 0x88,
        0x8c, 0x8a, 0x8e, 0x89, 0x8d, 0x8b, 0x4f, 0x48, 0x4c, 0x4a, 0x4e, 0x49, 0x4d, 0x4b, 0xcf,
        0xc8, 0xcc, 0xca, 0xce, 0xc9, 0xcd, 0xcb, 0x2f, 0x28, 0x2c, 0x2a, 0x2e, 0x29, 0x2d, 0x2b,
        0xaf, 0xa8, 0xac, 0xaa, 0xae, 0xa9, 0xad, 0xab, 0x6f, 0x68, 0x6c, 0x6a, 0x6e, 0x69, 0x6d,
        0x6b, 0xef, 0xe8, 0xec, 0xea, 0xee, 0xe9, 0xed, 0xeb, 0x9f, 0x30, 0x71, 0xd2, 0xe4, 0x29,
        0x53, 0xa7, 0x4d, 0x9f, 0x31, 0x73, 0xd6, 0xec, 0x39, 0x73, 0xe7, 0xcd, 0x5f, 0xb0, 0x70,
        0xd1, 0xe2, 0x25, 0x4b, 0x97, 0x2d, 0x5f, 0xb1, 0x72, 0xd5, 0xea, 0x35, 0x6b, 0xd7, 0xad,
        0xdf, 0xb0, 0x71, 0xd3, 0xe6, 0x2d, 0x5b, 0xb7, 0x6d, 0xdf, 0xb1, 0x73, 0xd7, 0xee, 0x3d,
        0x7b, 0xf7, 0xed, 0x3f, 0x70, 0xf0, 0xd0, 0xe1, 0x23, 0x47, 0x8f, 0x1d, 0x3f, 0x71, 0xf2,
        0xd4, 0xe9, 0x33, 0x67, 0xcf, 0x9d, 0xbf, 0x70, 0xf1, 0xd2, 0xe5, 0x2b, 0x57, 0xaf, 0x5d,
        0xbf, 0x71, 0xf3, 0xd6, 0xed, 0x3b, 0x77, 0xef, 0xdd, 0x7f, 0xf0, 0xf0, 0xd1, 0xe3, 0x27,
        0x4f, 0x9f, 0x3d, 0x7f, 0xf1, 0xf2, 0xd5, 0xeb, 0x37, 0x6f, 0xdf, 0xbd, 0xff, 0xf0, 0xf1,
        0xd3, 0xe7, 0x2f, 0x5f, 0xbf, 0x7d, 0xff, 0xf1, 0xf3, 0xd7, 0xef, 0x3f, 0x7f, 0xff, 0xfd,
        0x67, 0x18, 0xf5, 0xff, 0x88, 0xf6, 0x3f, 0x00,
    ];
    const STORED: &[u8] = &[
        0x01, 0x07, 0x00, 0xf8, 0xff, 0x73, 0x74, 0x6f, 0x72, 0x65, 0x64, 0x21,
    ];

    fn fox() -> Vec<u8> {
        b"The quick brown fox jumps over the lazy dog. ".repeat(40)
    }

    /// zlib(Python)이 만든 벡터: 고정 허프먼 · 동적 허프먼(반복 많음) · 혼합 · 저장 블록 · 손상 입력 = 오류(패닉 없음).
    #[test]
    fn inflate_vectors() {
        assert_eq!(inflate(FIXED_ABC).unwrap(), b"abc");
        assert_eq!(inflate(DYN_FOX).unwrap(), fox());
        let mixed: Vec<u8> = (0..=255u8).collect::<Vec<_>>().repeat(3);
        assert_eq!(inflate(MIXED_256).unwrap(), mixed);
        assert_eq!(inflate(STORED).unwrap(), b"stored!");
        assert!(inflate(&[]).is_err());
        assert!(inflate(&[0x07]).is_err(), "블록 종류 3");
        assert!(inflate(&DYN_FOX[..20]).is_err(), "잘린 입력");
    }

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir-extract-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// zip 바이트 조립 — (이름, 데이터, 압축 데이터(None = 저장), 방식).
    /// (이름, 데이터, 압축 데이터(None = 저장), 방식).
    type ZipSpec<'a> = (&'a str, &'a [u8], Option<&'a [u8]>, u16);

    fn build_zip(items: &[ZipSpec<'_>]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd = Vec::new();
        for (name, data, comp, method) in items {
            let off = out.len() as u32;
            let body: &[u8] = comp.unwrap_or(data);
            out.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
            out.extend_from_slice(&[20, 0, 0x00, 0x08]); // 버전 · 플래그(UTF-8)
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&[0, 0, 0x21, 0x58]); // 시각 · 날짜(2024-01-01)
            out.extend_from_slice(&0u32.to_le_bytes()); // crc
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(body);
            cd.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
            cd.extend_from_slice(&[20, 0, 20, 0, 0x00, 0x08]);
            cd.extend_from_slice(&method.to_le_bytes());
            cd.extend_from_slice(&[0, 0, 0x21, 0x58]);
            cd.extend_from_slice(&0u32.to_le_bytes());
            cd.extend_from_slice(&(body.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
            cd.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // extra · comment · disk · int attr
            cd.extend_from_slice(&0u32.to_le_bytes()); // ext attr
            cd.extend_from_slice(&off.to_le_bytes());
            cd.extend_from_slice(name.as_bytes());
        }
        let cd_off = out.len() as u32;
        out.extend_from_slice(&cd);
        out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(items.len() as u16).to_le_bytes());
        out.extend_from_slice(&(items.len() as u16).to_le_bytes());
        out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_off.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn tar_header(name: &str, size: u64, typeflag: u8) -> Vec<u8> {
        let mut h = vec![0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[100..107].copy_from_slice(b"0000644");
        h[124..135].copy_from_slice(format!("{size:011o}").as_bytes());
        h[136..147].copy_from_slice(format!("{:011o}", 1_700_000_000u64).as_bytes());
        h[156] = typeflag;
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        let sum: u64 = h
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    b as u64
                }
            })
            .sum();
        h[148..154].copy_from_slice(format!("{sum:06o}").as_bytes());
        h[155] = b' ';
        h
    }

    fn build_tar(parts: &[(&str, &[u8], u8)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, data, tf) in parts {
            out.extend(tar_header(name, data.len() as u64, *tf));
            out.extend_from_slice(data);
            let pad = (512 - data.len() % 512) % 512;
            out.extend(std::iter::repeat_n(0u8, pad));
        }
        out.extend(std::iter::repeat_n(0u8, 1024));
        out
    }

    /// 저장 블록만으로 원시 DEFLATE를 만든다(압축기 없이 gzip 시험용).
    fn stored_deflate(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let chunks: Vec<&[u8]> = data.chunks(65535).collect();
        for (i, c) in chunks.iter().enumerate() {
            out.push(u8::from(i + 1 == chunks.len()));
            out.extend_from_slice(&(c.len() as u16).to_le_bytes());
            out.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
            out.extend_from_slice(c);
        }
        out
    }

    fn gzip_wrap(name: Option<&str>, raw: &[u8], isize_: u32) -> Vec<u8> {
        let mut g = vec![
            0x1F,
            0x8B,
            0x08,
            if name.is_some() { 0x08 } else { 0 },
            0,
            0,
            0,
            0,
            0,
            0xFF,
        ];
        if let Some(n) = name {
            g.extend_from_slice(n.as_bytes());
            g.push(0);
        }
        g.extend_from_slice(raw);
        g.extend_from_slice(&0u32.to_le_bytes());
        g.extend_from_slice(&isize_.to_le_bytes());
        g
    }

    /// zip: 저장 + Deflate 항목 · 폴더 항목 · 중첩 경로 · zip slip(`../`)은 건너뜀 · 이미 있는 파일 건너뜀 · 지원 안 하는 방식 집계 ·
    /// 진행 합계 = 쓴 바이트 · 수정 시각 적용.
    #[test]
    fn zip_store_and_deflate_extract_safely() {
        let d = fixture("zip");
        let fox = fox();
        let zip = build_zip(&[
            ("dir/", b"", None, 0),
            ("dir/a.txt", b"hello", None, 0),
            ("fox.txt", &fox, Some(DYN_FOX), 8),
            ("../evil.txt", b"x", None, 0),
            ("lzma.bin", b"zz", None, 14),
        ]);
        let zp = d.join("t.zip");
        std::fs::write(&zp, &zip).unwrap();
        assert!(supported(&zp));
        assert_eq!(total_bytes(&zp).unwrap(), 5 + fox.len() as u64 + 1 + 2);
        let dest = d.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("fox.txt"), b"keep").unwrap();
        let mut sum = 0u64;
        let rep = extract(&zp, &dest, &mut |n| sum += n, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            (
                rep.files,
                rep.dirs,
                rep.skipped_existing,
                rep.suspicious,
                rep.unsupported
            ),
            (1, 1, 1, 1, 1),
            "{rep:?}"
        );
        assert_eq!(sum, 5);
        assert_eq!(
            std::fs::read(dest.join("dir").join("a.txt")).unwrap(),
            b"hello"
        );
        assert_eq!(std::fs::read(dest.join("fox.txt")).unwrap(), b"keep");
        assert!(!d.join("evil.txt").exists());
        // 대상 폴더를 비우고 다시 → Deflate 항목이 풀린다 · 시각 = 2024-01-01 00:00 UTC.
        let dest2 = d.join("out2");
        let rep = extract(&zp, &dest2, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(rep.files, 2, "{rep:?}");
        assert_eq!(std::fs::read(dest2.join("fox.txt")).unwrap(), fox);
        let m = std::fs::metadata(dest2.join("fox.txt"))
            .unwrap()
            .modified()
            .unwrap();
        let secs = m.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(secs, 1_704_067_200);
        // 취소 = 첫 확인에서 멈춤.
        let rep = extract(&zp, &d.join("out3"), &mut |_| {}, &AtomicBool::new(true)).unwrap();
        assert!(rep.canceled && rep.files == 0);
        // 압축 파일이 아니면 NotArchive.
        std::fs::write(d.join("plain.bin"), b"nothing here").unwrap();
        assert_eq!(
            extract(
                &d.join("plain.bin"),
                &d.join("x"),
                &mut |_| {},
                &AtomicBool::new(false)
            )
            .err(),
            Some(ArchiveError::NotArchive)
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// tar: 파일 · 폴더 · GNU 긴 이름 · 심볼릭 링크(만들지 않음) · gzip 단일 파일(FNAME) · tgz(안쪽 tar).
    #[test]
    fn tar_gzip_and_tgz_extract() {
        let d = fixture("tar");
        let long = "x".repeat(120) + "/deep.txt";
        let tar = build_tar(&[
            ("sub/", b"", b'5'),
            ("sub/b.txt", b"bee", b'0'),
            ("././@LongLink", long.as_bytes(), b'L'),
            ("placeholder", b"deep!", b'0'),
            ("link", b"", b'2'),
        ]);
        let tp = d.join("t.tar");
        std::fs::write(&tp, &tar).unwrap();
        let dest = d.join("out");
        let rep = extract(&tp, &dest, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!((rep.files, rep.dirs, rep.unsupported), (2, 1, 1), "{rep:?}");
        assert_eq!(
            std::fs::read(dest.join("sub").join("b.txt")).unwrap(),
            b"bee"
        );
        assert_eq!(
            std::fs::read(dest.join("x".repeat(120)).join("deep.txt")).unwrap(),
            b"deep!"
        );
        // gzip 단일 파일 — 이름은 FNAME.
        let gz = gzip_wrap(Some("note.txt"), &stored_deflate(b"gzip body"), 9);
        let gp = d.join("note.txt.gz");
        std::fs::write(&gp, &gz).unwrap();
        let rep = extract(&gp, &d.join("g"), &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(rep.files, 1);
        assert_eq!(
            std::fs::read(d.join("g").join("note.txt")).unwrap(),
            b"gzip body"
        );
        // FNAME 없으면 파일 이름에서 .gz를 벗긴다.
        let gp2 = d.join("data.bin.gz");
        std::fs::write(&gp2, gzip_wrap(None, &stored_deflate(b"raw"), 3)).unwrap();
        extract(&gp2, &d.join("g2"), &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            std::fs::read(d.join("g2").join("data.bin")).unwrap(),
            b"raw"
        );
        // tgz.
        let tgz = gzip_wrap(None, &stored_deflate(&tar), tar.len() as u32);
        let zp = d.join("t.tgz");
        std::fs::write(&zp, &tgz).unwrap();
        let rep = extract(&zp, &d.join("tg"), &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!((rep.files, rep.dirs), (2, 1), "{rep:?}");
        assert_eq!(
            std::fs::read(d.join("tg").join("sub").join("b.txt")).unwrap(),
            b"bee"
        );
        assert!(supported(&zp) && supported(&tp) && !supported(&d.join("nope.xyz")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
