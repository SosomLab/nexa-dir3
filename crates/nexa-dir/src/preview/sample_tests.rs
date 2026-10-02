//! 동봉 플러그인(`plugins/markdown.wasm` · `plugins/archive.wasm` — dir2 dist 바이너리 **무수정**, DR-7 회귀 기준)을
//! 실제 wasmi 런타임으로 로드·실행해 계약(ABI·태그·Mermaid 폴백·ISO/ar/cpio 목록)을 검증한다.
//! 산출물 갱신: `plugins/sdk/*`에서 `cargo build --release --target wasm32-unknown-unknown`(T-63 `scripts/plugin-build.*`).

use super::wasm::{load_dir, run_archive, run_preview};
use super::PreviewDoc;
use std::path::{Path, PathBuf};

/// 저장소 `plugins/`(동봉 .wasm) — 시험 전용 경로.
pub(crate) fn bundled_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins")
}

fn fresh(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ndir_wasm_sample_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn bundled_markdown_plugin_end_to_end() {
    let d = fresh("md");
    std::fs::copy(bundled_dir().join("markdown.wasm"), d.join("markdown.wasm")).unwrap();
    let (plugins, errors) = load_dir(&d);
    assert!(errors.is_empty(), "{errors:?}");
    let p = &plugins[0];
    assert_eq!(p.id, "markdown");
    assert_eq!(
        p.exts,
        ["md", "markdown", "mdown", "mkd"],
        "적용 대상 = 플러그인 내부 선언(외부 재정의는 preview.map)"
    );
    let fixture = bundled_dir().join("sdk/markdown-viewer/fixtures/sample.md");
    let lines = match run_preview(p, &fixture).unwrap() {
        PreviewDoc::Lines(l) => l,
        _ => panic!("lines 반환"),
    };
    let joined = lines.join("\n");
    assert!(joined.contains("\u{2}h1|"), "h1 태그: {joined}");
    assert!(joined.contains("• 항목 하나"), "불릿");
    assert!(joined.contains('☑'), "체크 목록");
    assert!(joined.contains("\u{2}mono|┌"), "표 박스(모노 태그)");
    assert!(
        joined.contains("굵게") && !joined.contains("**굵게**"),
        "인라인 마커 정리"
    );
    // Mermaid flowchart 3단 폴백(이미지 마커 → 아트 → 원문) — render_svg는 T-62 B라 아트/원문.
    assert!(
        joined.contains("\u{1}img|") || joined.contains('▼') || joined.contains("graph TD"),
        "flowchart 이미지/아트/원문 폴백 중 하나: {joined}"
    );
    assert!(joined.contains("Client"), "sequence participant 별칭");
    assert!(
        joined.contains('▶') || joined.contains('◀'),
        "sequence 화살표"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// newc cpio 1건 조립(110B 16진 헤더 + 이름/데이터 4바이트 정렬).
fn cpio_member(name: &str, data: &[u8], mode: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"070701");
    let f = |x: u64| format!("{x:08X}");
    for x in [
        1u64,
        u64::from(mode),
        0,
        0,
        1,
        1_700_000_000,
        data.len() as u64,
        0,
        0,
        0,
        0,
        name.len() as u64 + 1,
        0,
    ] {
        v.extend_from_slice(f(x).as_bytes());
    }
    v.extend_from_slice(name.as_bytes());
    v.push(0);
    while v.len() % 4 != 0 {
        v.push(0);
    }
    v.extend_from_slice(data);
    while v.len() % 4 != 0 {
        v.push(0);
    }
    v
}

/// ISO 9660 디렉터리 레코드 1건.
fn iso_record(name: &[u8], extent: u32, size: u32, is_dir: bool) -> Vec<u8> {
    let mut r = vec![0u8; 33 + name.len() + usize::from(name.len().is_multiple_of(2))];
    r[0] = r.len() as u8;
    r[2..6].copy_from_slice(&extent.to_le_bytes());
    r[6..10].copy_from_slice(&extent.to_be_bytes());
    r[10..14].copy_from_slice(&size.to_le_bytes());
    r[14..18].copy_from_slice(&size.to_be_bytes());
    r[18..25].copy_from_slice(&[126, 8, 24, 9, 30, 0, 0]); // 2026-08-24 09:30 UTC
    r[25] = u8::from(is_dir) << 1;
    r[32] = name.len() as u8;
    r[33..33 + name.len()].copy_from_slice(name);
    r
}

/// 최소 ISO 9660 이미지(PVD → 루트 디렉터리 → 파일 1개).
fn iso_image() -> Vec<u8> {
    const S: usize = 2048;
    let mut img = vec![0u8; S * 21];
    let pvd = &mut img[S * 16..S * 17];
    pvd[0] = 1;
    pvd[1..6].copy_from_slice(b"CD001");
    pvd[6] = 1;
    let root = iso_record(&[0], 18, S as u32, true);
    pvd[156..156 + root.len()].copy_from_slice(&root);
    img[S * 17] = 255;
    img[S * 17 + 1..S * 17 + 6].copy_from_slice(b"CD001");
    let mut dir = Vec::new();
    dir.extend(iso_record(&[0], 18, S as u32, true));
    dir.extend(iso_record(&[1], 18, S as u32, true));
    dir.extend(iso_record(b"HELLO.TXT;1", 19, 5, false));
    img[S * 18..S * 18 + dir.len()].copy_from_slice(&dir);
    img[S * 19..S * 19 + 5].copy_from_slice(b"hello");
    img
}

#[test]
fn bundled_archive_plugin_lists_iso_ar_and_cpio() {
    let d = fresh("arc");
    std::fs::copy(bundled_dir().join("archive.wasm"), d.join("archive.wasm")).unwrap();
    let (plugins, errors) = load_dir(&d);
    assert!(errors.is_empty(), "{errors:?}");
    let p = &plugins[0];
    assert_eq!(p.id, "archive-sample");
    assert!(p.is_archive(), "nx_meta 4번째 줄 = archive 능력 선언");
    assert!(p.exts.contains(&"iso".to_string()) && p.exts.contains(&"cpio".to_string()));

    let mut cpio = cpio_member("dir", &[], 0o040755);
    cpio.extend(cpio_member("dir/a.txt", b"hello", 0o100644));
    cpio.extend(cpio_member("TRAILER!!!", &[], 0));
    let f = d.join("t.cpio");
    std::fs::write(&f, &cpio).unwrap();
    let doc = run_archive(p, &f).unwrap();
    assert!(doc.is_ok(), "{:?}", doc.status);
    assert_eq!(doc.listing.label, "cpio (newc)");
    let a = doc
        .listing
        .entries
        .iter()
        .find(|e| e.path == "dir/a.txt")
        .unwrap();
    assert_eq!(
        (a.size, a.modified, a.time_is_local),
        (Some(5), Some(1_700_000_000), false)
    );
    assert!(doc
        .listing
        .entries
        .iter()
        .any(|e| e.path == "dir" && e.is_dir));

    let mut ar = b"!<arch>\n".to_vec();
    ar.extend(super::wasm::tests::ar_member("hello.o/", b"OBJ"));
    let f = d.join("t.a");
    std::fs::write(&f, &ar).unwrap();
    let doc = run_archive(p, &f).unwrap();
    assert_eq!(doc.listing.label, "ar");
    assert_eq!(doc.listing.entries[0].path, "hello.o");
    assert_eq!(doc.listing.entries[0].size, Some(3));

    let f = d.join("t.iso");
    std::fs::write(&f, iso_image()).unwrap();
    let doc = run_archive(p, &f).unwrap();
    assert_eq!(doc.listing.label, "ISO 9660");
    let h = doc
        .listing
        .entries
        .iter()
        .find(|e| e.path == "HELLO.TXT")
        .expect("버전 접미(;1) 제거 후 파일 1건");
    assert_eq!(h.size, Some(5));
    assert_eq!(
        h.modified,
        Some(ndir_vfs::archive::ymd_hms_to_unix(2026, 8, 24, 9, 30, 0))
    );

    let f = d.join("t.iso.bad");
    std::fs::write(&f, b"garbage").unwrap();
    let doc = run_archive(p, &f).unwrap();
    assert!(!doc.is_ok(), "미지원 형식은 실패 상태");
    let _ = std::fs::remove_dir_all(&d);
}

/// 시임 전체(E2E): 동봉 폴더를 `NDIR_PLUGINS_DIR`로 지정 → `preview_for`가 `.md`를 markdown 플러그인으로,
/// `.a`를 archive-sample로 라우팅하고 공급자 id를 돌려준다(스레드 로컬 캐시 = 이 시험 스레드 전용).
#[test]
fn seam_routes_bundled_plugins_by_declared_ext() {
    ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
    std::env::set_var("NDIR_PLUGINS_DIR", bundled_dir());
    let d = fresh("seam");
    let md = d.join("note.md");
    std::fs::write(&md, "# Hello\n\n- item\n").unwrap();
    let (doc, id) = super::preview_for(&md, "", "");
    assert_eq!(id, "markdown");
    match doc {
        PreviewDoc::Lines(l) => assert!(l.iter().any(|x| x == "\u{2}h1|Hello"), "{l:?}"),
        other => panic!("{other:?}"),
    }
    // 꺼진 플러그인 = 내장 텍스트 폴백(원문 그대로).
    let (doc, id) = super::preview_for(&md, "", "markdown");
    assert_eq!(id, "builtin.text");
    assert!(matches!(doc, PreviewDoc::Lines(l) if l[0] == "# Hello"));
    let infos = super::plugin_infos();
    assert!(infos.iter().any(|i| i.id == "archive-sample"), "{infos:?}");
    assert!(super::load_notes().is_empty(), "{:?}", super::load_notes());
    let _ = std::fs::remove_dir_all(&d);
}
