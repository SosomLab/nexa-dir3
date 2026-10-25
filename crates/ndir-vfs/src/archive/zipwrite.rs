//! zip **쓰기**(DR-31 ⑳ⓑ · 10-25 · "Linux 압축하기 없음" — 외부 아카이버가 없어도 폴더/파일을 zip으로): **DEFLATE**(자체 구현 [`super::deflate`] · 10-26 DR-32 ⑦ ·
//! 이미 압축된 형식(zip·7z·png·jpg·mp4 …)은 store · 눌러서 store보다 크면 store로 되돌림(≤ 32 MiB · 그보다 크면 조각 블록 스트리밍) · 외부 crate 0 · DR-8) ·
//! CRC-32 · UTF-8 이름 깃발(0x800) · 항목마다 **데이터 서술자**(깃발 비트 3 — 파일을 한 번만 읽는다) · 폴더는 재귀(`dir/` 항목 포함) ·
//! ZIP64 없음(파일 하나 또는 전체가 4 GiB를 넘으면 **쓰기 전에** `fits_zip32`로 판정해 파일을 만들지 않고 `too_big`에 적는다 — 부분 zip을 남기지
//! 않는다 · 협업 검토 10-25 ③) · 취소 = 항목 경계.
//! 읽기 쪽(`zip.rs` · `extract.rs`)이 그대로 열 수 있다(왕복 시험).

use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::deflate;
use super::extract::crc32_update;

/// 쓰기 결과.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub files: u64,
    pub dirs: u64,
    /// 원본 바이트.
    pub bytes: u64,
    /// 쓴 데이터 바이트(압축 뒤 · 머리/서술자 제외).
    pub compressed: u64,
    /// 4 GiB 이상이라 건너뛴 항목.
    pub too_big: Vec<PathBuf>,
    /// 항목별 실패(경로 · 사유).
    pub errors: Vec<(PathBuf, String)>,
    pub canceled: bool,
}

const SIG_LOCAL: u32 = 0x0403_4b50;
const SIG_CENTRAL: u32 = 0x0201_4b50;
const SIG_END: u32 = 0x0605_4b50;
const SIG_DESC: u32 = 0x0807_4b50;
const FLAG_DESC: u16 = 1 << 3;
const FLAG_UTF8: u16 = 1 << 11;
const VERSION: u16 = 20;
const LIMIT: u64 = u32::MAX as u64;
/// 이 크기까지는 통째로 눌러 보고 store보다 크면 store로 되돌린다 · 넘으면 조각 블록 스트리밍(되돌림 없음).
const IN_MEMORY: u64 = 32 * 1024 * 1024;

/// 이미 압축된 형식(확장자 · 소문자) — DEFLATE해도 줄지 않아 store(7-Zip·탐색기 관례 · DR-32 ⑦).
const STORE_EXTS: &[&str] = &[
    "zip", "7z", "rar", "gz", "tgz", "bz2", "xz", "zst", "lz4", "lzma", "cab", "jar", "apk",
    "docx", "xlsx", "pptx", "odt", "ods", "odp", "png", "jpg", "jpeg", "gif", "webp", "avif",
    "heic", "heif", "mp3", "aac", "ogg", "opus", "flac", "m4a", "mp4", "m4v", "mkv", "mov", "avi",
    "webm", "wmv", "pdf", "woff", "woff2",
];

/// 이 이름의 항목을 DEFLATE할까(순수): 이미 압축된 형식은 store.
#[must_use]
pub fn deflate_wanted(name: &str) -> bool {
    let ext = name
        .rsplit('/')
        .next()
        .and_then(|n| n.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase());
    !ext.is_some_and(|e| STORE_EXTS.contains(&e.as_str()))
}

struct Central {
    name: Vec<u8>,
    crc: u32,
    size: u32,
    csize: u32,
    method: u16,
    is_dir: bool,
    local_off: u32,
    dos_time: u16,
    dos_date: u16,
}

fn w16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn w32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// 수정 시각 → DOS (time, date). 모르면 1980-01-01 00:00.
fn dos_datetime(meta: &std::fs::Metadata) -> (u16, u16) {
    let secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    // 현지 시간대 없이 UTC(zip은 현지 시각이지만 시간대 정보가 없다 — 읽는 쪽도 그대로 보여 준다).
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (
        (rem / 3600) as u16,
        ((rem % 3600) / 60) as u16,
        (rem % 60) as u16,
    );
    // 1970-01-01 기준 일수 → 연월일(그레고리력).
    let (mut y, mut mo, mut d) = (1970i64, 1i64, 1i64);
    {
        let mut n = days;
        loop {
            let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
            let yl = if leap { 366 } else { 365 };
            if n < yl {
                break;
            }
            n -= yl;
            y += 1;
        }
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let ml = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        for len in ml {
            if n < len {
                break;
            }
            n -= len;
            mo += 1;
        }
        d += n;
    }
    if y < 1980 {
        return (0, (1 << 5) | 1);
    }
    let time = (h << 11) | (m << 5) | (s / 2);
    let date = (((y - 1980) as u16) << 9) | ((mo as u16) << 5) | d as u16;
    (time, date)
}

/// `name` = zip 안 경로(`/` 구분 · 폴더는 끝에 `/`).
fn local_header(out: &mut Vec<u8>, name: &[u8], is_dir: bool, dt: (u16, u16), method: u16) {
    w32(out, SIG_LOCAL);
    w16(out, VERSION);
    w16(out, FLAG_UTF8 | if is_dir { 0 } else { FLAG_DESC });
    w16(out, method); // 0 = store · 8 = deflate
    w16(out, dt.0);
    w16(out, dt.1);
    w32(out, 0); // crc(서술자)
    w32(out, 0); // csize
    w32(out, 0); // usize
    w16(out, name.len() as u16);
    w16(out, 0); // extra
    out.extend_from_slice(name);
}

/// 항목 목록을 모은다(파일 · 폴더 재귀 · 이름은 `base` 기준 상대 경로 · `/`).
fn collect(src: &Path, base: &Path, out: &mut Vec<(PathBuf, String, bool)>, rep: &mut WriteReport) {
    let rel = src
        .strip_prefix(base)
        .unwrap_or_else(|_| Path::new(src.file_name().unwrap_or_default()));
    let mut name = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    let meta = match std::fs::symlink_metadata(src) {
        Ok(m) => m,
        Err(e) => {
            rep.errors.push((src.to_path_buf(), e.to_string()));
            return;
        }
    };
    if meta.file_type().is_symlink() {
        return; // 링크는 건너뜀(풀기 쪽도 지원하지 않는다)
    }
    if meta.is_dir() {
        if !name.is_empty() {
            name.push('/');
            out.push((src.to_path_buf(), name, true));
        }
        let mut kids: Vec<PathBuf> = match std::fs::read_dir(src) {
            Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
            Err(e) => {
                rep.errors.push((src.to_path_buf(), e.to_string()));
                return;
            }
        };
        kids.sort();
        for k in kids {
            collect(&k, base, out, rep);
        }
    } else {
        out.push((src.to_path_buf(), name, false));
    }
}

/// 항목 크기 묶음이 ZIP32(4 GiB · ZIP64 없음)에 들어가는가 — 항목 하나와 전체(항목별 머리·서술자·중앙 항목·이름 여유 포함) 둘 다(순수 판정).
#[must_use]
pub fn fits_zip32(sizes: &[u64]) -> bool {
    const OVERHEAD: u64 = 30 + 16 + 46 + 2 * 1024; // 로컬 머리 + 서술자 + 중앙 항목 + 이름 2회(1 KiB씩 여유)
    let mut total: u64 = 0;
    for &s in sizes {
        if s > LIMIT {
            return false;
        }
        total = total.saturating_add(s).saturating_add(OVERHEAD);
    }
    total <= LIMIT
}

/// 선택한 경로들(`sources` · 같은 부모 폴더 아래라고 가정 — 이름은 **그 부모 기준**)을 `dest`(zip 파일)로. `on_bytes` = 진행(누적 원본 바이트) ·
/// `cancel` = 항목 경계에서 멈추고 부분 파일을 지운다.
pub fn write_zip(
    sources: &[PathBuf],
    dest: &Path,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<WriteReport> {
    let mut rep = WriteReport::default();
    let base = sources
        .first()
        .and_then(|p| p.parent())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut entries = Vec::new();
    for s in sources {
        collect(s, &base, &mut entries, &mut rep);
    }
    // 크기 한도(ZIP64 없음): 쓰기 전에 판정 — 넘으면 파일을 만들지 않고 넘는 항목(전체 초과면 원본 전부)을 `too_big`에 적는다.
    let sizes: Vec<u64> = entries
        .iter()
        .map(|(p, _, d)| {
            if *d {
                0
            } else {
                std::fs::metadata(p).map_or(0, |m| m.len())
            }
        })
        .collect();
    if !fits_zip32(&sizes) {
        let over: Vec<PathBuf> = entries
            .iter()
            .zip(&sizes)
            .filter(|(_, s)| **s > LIMIT)
            .map(|((p, _, _), _)| p.clone())
            .collect();
        rep.too_big = if over.is_empty() {
            sources.to_vec()
        } else {
            over
        };
        return Ok(rep);
    }
    let mut file = std::fs::File::create(dest)?;
    let mut central: Vec<Central> = Vec::new();
    let mut offset: u64 = 0;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    for (path, name, is_dir) in entries {
        if cancel.load(Ordering::Relaxed) {
            rep.canceled = true;
            break;
        }
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                rep.errors.push((path.clone(), e.to_string()));
                continue;
            }
        };
        let dt = dos_datetime(&meta);
        let nb = name.as_bytes().to_vec();
        let local_off = offset as u32;
        if is_dir {
            let mut head = Vec::with_capacity(30 + nb.len());
            local_header(&mut head, &nb, true, dt, 0);
            file.write_all(&head)?;
            offset += head.len() as u64;
            rep.dirs += 1;
            central.push(Central {
                name: nb,
                crc: 0,
                size: 0,
                csize: 0,
                method: 0,
                is_dir: true,
                local_off,
                dos_time: dt.0,
                dos_date: dt.1,
            });
            continue;
        }
        let mut f = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                rep.errors.push((path.clone(), e.to_string()));
                continue; // 머리를 쓰기 전이라 항목 자체를 뺀다
            }
        };
        let size = meta.len();
        // 방식(DR-32 ⑦): 이미 압축된 형식은 store · 나머지 DEFLATE — ≤ IN_MEMORY는 통째로 눌러 보고 store보다 크면 store로 되돌림 ·
        // 큰 파일은 조각마다 블록으로 흘려 쓴다(되돌림 없음 · 비압축 자료가 커지는 최악 +13 %를 감안해 LIMIT·8/9까지만).
        let want_deflate = deflate_wanted(&name);
        let mut head = Vec::with_capacity(30 + nb.len());
        let (crc, csize, method) = if want_deflate && size <= IN_MEMORY {
            let mut data = Vec::with_capacity(size as usize);
            f.read_to_end(&mut data)?;
            let crc = !crc32_update(0xFFFF_FFFF, &data);
            let comp = deflate::deflate(&data);
            let (body, method): (&[u8], u16) = if comp.len() < data.len() {
                (&comp, 8)
            } else {
                (&data, 0)
            };
            local_header(&mut head, &nb, false, dt, method);
            file.write_all(&head)?;
            offset += head.len() as u64;
            file.write_all(body)?;
            offset += body.len() as u64;
            done += data.len() as u64;
            on_bytes(done);
            (crc, body.len() as u64, method)
        } else {
            let method: u16 = if want_deflate && size <= LIMIT / 9 * 8 {
                8
            } else {
                0
            };
            local_header(&mut head, &nb, false, dt, method);
            file.write_all(&head)?;
            offset += head.len() as u64;
            let mut crc = 0xFFFF_FFFFu32;
            let mut csize = 0u64;
            let mut d = deflate::Deflater::new();
            loop {
                if cancel.load(Ordering::Relaxed) {
                    rep.canceled = true;
                    break;
                }
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                crc = crc32_update(crc, &buf[..n]);
                done += n as u64;
                on_bytes(done);
                if method == 8 {
                    d.block(&buf[..n], false);
                    let out = d.take();
                    file.write_all(&out)?;
                    csize += out.len() as u64;
                } else {
                    file.write_all(&buf[..n])?;
                    csize += n as u64;
                }
            }
            if rep.canceled {
                break;
            }
            if method == 8 {
                d.block(&[], true);
                let out = d.finish();
                file.write_all(&out)?;
                csize += out.len() as u64;
            }
            offset += csize;
            (!crc, csize, method)
        };
        let mut desc = Vec::with_capacity(16);
        w32(&mut desc, SIG_DESC);
        w32(&mut desc, crc);
        w32(&mut desc, csize as u32);
        w32(&mut desc, size as u32);
        file.write_all(&desc)?;
        offset += desc.len() as u64;
        rep.files += 1;
        rep.bytes += size;
        rep.compressed += csize;
        central.push(Central {
            name: nb,
            crc,
            size: size as u32,
            csize: csize as u32,
            method,
            is_dir: false,
            local_off,
            dos_time: dt.0,
            dos_date: dt.1,
        });
    }
    if rep.canceled {
        drop(file);
        let _ = std::fs::remove_file(dest);
        return Ok(rep);
    }
    // 중앙 디렉터리.
    let cd_off = offset;
    let mut cd = Vec::new();
    for c in &central {
        w32(&mut cd, SIG_CENTRAL);
        w16(&mut cd, VERSION); // made by
        w16(&mut cd, VERSION); // needed
        w16(&mut cd, FLAG_UTF8 | if c.is_dir { 0 } else { FLAG_DESC });
        w16(&mut cd, c.method);
        w16(&mut cd, c.dos_time);
        w16(&mut cd, c.dos_date);
        w32(&mut cd, c.crc);
        w32(&mut cd, c.csize);
        w32(&mut cd, c.size);
        w16(&mut cd, c.name.len() as u16);
        w16(&mut cd, 0);
        w16(&mut cd, 0);
        w16(&mut cd, 0);
        w16(&mut cd, 0);
        w32(&mut cd, if c.is_dir { 0x10 } else { 0 }); // external attrs(DOS 폴더 비트)
        w32(&mut cd, c.local_off);
        cd.extend_from_slice(&c.name);
    }
    file.write_all(&cd)?;
    let mut end = Vec::with_capacity(22);
    w32(&mut end, SIG_END);
    w16(&mut end, 0);
    w16(&mut end, 0);
    w16(&mut end, central.len() as u16);
    w16(&mut end, central.len() as u16);
    w32(&mut end, cd.len() as u32);
    w32(&mut end, cd_off as u32);
    w16(&mut end, 0);
    file.write_all(&end)?;
    file.flush()?;
    let _ = file.seek(io::SeekFrom::End(0));
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 왕복: 폴더(하위 폴더 · 파일 2 · 빈 폴더) + 파일 1 → zip → 기존 풀기(`extract`)로 풀어 내용 · 수가 같다 · UTF-8 이름 · 취소 = 파일 없음.
    /// ZIP64 없음(협업 검토 ③): 한도 판정은 순수 함수 — 항목 하나 초과 · 전체 초과 · 머리 여유까지 센다.
    #[test]
    fn zip32_limit_is_checked_before_writing() {
        assert!(fits_zip32(&[]));
        assert!(fits_zip32(&[0, 1 << 20, 3 << 30]));
        assert!(!fits_zip32(&[LIMIT + 1]), "항목 하나 초과");
        assert!(!fits_zip32(&[3 << 30, 3 << 30]), "전체 초과");
        assert!(!fits_zip32(&[LIMIT - 10]), "머리 여유까지 세면 초과");
    }

    #[test]
    fn write_then_extract_round_trip() {
        let base = std::env::temp_dir().join(format!("ndir-zipw-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        std::fs::create_dir_all(src.join("보고서/깊은")).unwrap();
        std::fs::create_dir_all(src.join("empty")).unwrap();
        std::fs::write(src.join("보고서/a.txt"), b"hello zip").unwrap();
        std::fs::write(src.join("보고서/깊은/b.bin"), vec![7u8; 70_000]).unwrap();
        std::fs::write(src.join("top.md"), b"# top").unwrap();
        std::fs::write(src.join("보고서/pic.png"), vec![7u8; 20_000]).unwrap(); // 이미 압축된 형식 = store(줄여도)
        let dest = base.join("out.zip");
        let mut last = 0;
        let rep = write_zip(
            &[src.join("보고서"), src.join("empty"), src.join("top.md")],
            &dest,
            &mut |b| last = b,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!((rep.files, rep.dirs), (4, 3), "{rep:?}");
        assert_eq!(rep.bytes, 9 + 70_000 + 5 + 20_000);
        assert!(
            rep.compressed < 20_000 + 1_000,
            "b.bin 70,000(7 반복)은 수백 바이트로 · png 20,000은 store 그대로: {rep:?}"
        );
        // 중앙 디렉터리의 방식: b.bin = 8(deflate) · pic.png = 0(store) · a.txt(9바이트) = store(눌러도 안 줆).
        let bytes = std::fs::read(&dest).unwrap();
        let mut methods = std::collections::HashMap::new();
        let mut i = 0;
        while i + 46 <= bytes.len() {
            if u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
                == SIG_CENTRAL
            {
                let method = u16::from_le_bytes([bytes[i + 10], bytes[i + 11]]);
                let nlen = u16::from_le_bytes([bytes[i + 28], bytes[i + 29]]) as usize;
                let name = String::from_utf8_lossy(&bytes[i + 46..i + 46 + nlen]).into_owned();
                methods.insert(name, method);
                i += 46 + nlen;
            } else {
                i += 1;
            }
        }
        assert_eq!(methods.get("보고서/깊은/b.bin"), Some(&8), "{methods:?}");
        assert_eq!(methods.get("보고서/pic.png"), Some(&0), "{methods:?}");
        assert_eq!(methods.get("top.md"), Some(&0), "작아서 안 줆 → store");
        assert!(
            deflate_wanted("a/b.TXT")
                && !deflate_wanted("x.ZIP")
                && !deflate_wanted("v.mp4")
                && deflate_wanted("noext")
        );
        assert_eq!(last, rep.bytes, "진행 = 누적 바이트");
        assert!(rep.errors.is_empty() && rep.too_big.is_empty() && !rep.canceled);
        let out = base.join("unz");
        let r = super::super::extract::extract(&dest, &out, &mut |_| {}, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(r.files, 4, "{r:?}");
        assert_eq!(
            std::fs::read(out.join("보고서/a.txt")).unwrap(),
            b"hello zip"
        );
        assert_eq!(
            std::fs::read(out.join("보고서/pic.png")).unwrap(),
            vec![7u8; 20_000]
        );
        assert_eq!(
            std::fs::read(out.join("보고서/깊은/b.bin")).unwrap().len(),
            70_000
        );
        assert_eq!(std::fs::read(out.join("top.md")).unwrap(), b"# top");
        assert!(out.join("empty").is_dir(), "빈 폴더도 항목으로");
        // 헤더 일반 비트 11(UTF-8 이름) = 로컬·중앙 모두(협업 검토 ① — 없으면 Windows 탐색기가 한글 이름을 CP437로 읽어 깨진다).
        let bytes = std::fs::read(&dest).unwrap();
        let (mut n_local, mut n_central) = (0, 0);
        for i in 0..bytes.len() - 10 {
            let sig = u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
            let (flag_at, which) = match sig {
                SIG_LOCAL => (i + 6, &mut n_local),
                SIG_CENTRAL => (i + 8, &mut n_central),
                _ => continue,
            };
            let flags = u16::from_le_bytes([bytes[flag_at], bytes[flag_at + 1]]);
            assert!(flags & FLAG_UTF8 != 0, "UTF-8 비트 없음 @{i}");
            *which += 1;
        }
        assert_eq!(
            (n_local, n_central),
            (7, 7),
            "항목 7개(파일 4 · 폴더 3) 머리 각각"
        );
        // 취소 = 부분 파일 제거.
        let dest2 = base.join("canceled.zip");
        let rep2 = write_zip(
            &[src.join("보고서")],
            &dest2,
            &mut |_| {},
            &AtomicBool::new(true),
        )
        .unwrap();
        assert!(rep2.canceled && !dest2.exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
