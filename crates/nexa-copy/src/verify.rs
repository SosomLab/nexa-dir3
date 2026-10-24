//! 복사 검증(`--verify` · T-97 "검증" · 10-25): 전송이 끝난 뒤 (원본, 사본) 쌍마다 **크기 + SHA-256**을 대조한다 — 파일은 그대로 ·
//! 폴더는 재귀(원본 쪽 항목 기준 · 사본에만 있는 것은 무시 — 건너뜀 정책으로 남아 있던 파일) · 이동은 원본이 없어 검증하지 않는다.
//! 엔진(`ndir_ops::hash::hash_file`)과 같은 해시 · 취소 깃발을 본다.

use ndir_ops::hash::{hash_file, Algo};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// 검증 결과 — 대조한 파일 수 · 읽은 바이트 · 불일치/오류 목록(사본 경로 · 사유).
#[derive(Debug, Default)]
pub(crate) struct VerifyReport {
    pub files: u64,
    pub bytes: u64,
    pub failures: Vec<(PathBuf, String)>,
}

impl VerifyReport {
    pub(crate) fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// (원본, 사본) 쌍들을 검증한다 — `on_bytes` = 읽은 증분(진행 표시) · 취소되면 `Err(Interrupted)`.
pub(crate) fn verify_pairs(
    pairs: &[(PathBuf, PathBuf)],
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> std::io::Result<VerifyReport> {
    let mut rep = VerifyReport::default();
    for (src, dst) in pairs {
        verify_one(src, dst, &mut rep, on_bytes, cancel)?;
    }
    Ok(rep)
}

fn verify_one(
    src: &Path,
    dst: &Path,
    rep: &mut VerifyReport,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> std::io::Result<()> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "canceled",
        ));
    }
    let (Ok(sm), Ok(dm)) = (std::fs::metadata(src), std::fs::metadata(dst)) else {
        rep.failures
            .push((dst.to_path_buf(), "missing".to_string()));
        return Ok(());
    };
    if sm.is_dir() != dm.is_dir() {
        rep.failures.push((
            dst.to_path_buf(),
            "type mismatch (file vs folder)".to_string(),
        ));
        return Ok(());
    }
    if sm.is_dir() {
        let mut names: Vec<std::ffi::OsString> = std::fs::read_dir(src)?
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        names.sort();
        for n in names {
            verify_one(&src.join(&n), &dst.join(&n), rep, on_bytes, cancel)?;
        }
        return Ok(());
    }
    rep.files += 1;
    if sm.len() != dm.len() {
        rep.failures.push((
            dst.to_path_buf(),
            format!("size differs ({} vs {})", sm.len(), dm.len()),
        ));
        return Ok(());
    }
    let a = hash_file(src, &[Algo::Sha256], on_bytes, cancel)?;
    let b = hash_file(dst, &[Algo::Sha256], on_bytes, cancel)?;
    rep.bytes += sm.len() * 2;
    if a != b {
        rep.failures
            .push((dst.to_path_buf(), "content differs (SHA-256)".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 같은 트리 = 통과(파일 수 · 바이트) · 사본 바꿈 = 내용 불일치 · 크기 다름 · 사본 없음 · 파일↔폴더 · 사본에만 있는 것은 무시 · 취소.
    #[test]
    fn verify_detects_mismatch_missing_and_type() {
        let d = std::env::temp_dir().join(format!("nexa-copy-verify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (s, t) = (d.join("s"), d.join("t"));
        std::fs::create_dir_all(s.join("sub")).unwrap();
        std::fs::create_dir_all(t.join("sub")).unwrap();
        for base in [&s, &t] {
            std::fs::write(base.join("a.bin"), vec![1u8; 3000]).unwrap();
            std::fs::write(base.join("sub").join("b.txt"), b"hello").unwrap();
        }
        std::fs::write(t.join("extra.txt"), b"only in copy").unwrap();
        let cancel = AtomicBool::new(false);
        let mut read = 0u64;
        let rep = verify_pairs(&[(s.clone(), t.clone())], &mut |n| read += n, &cancel).unwrap();
        assert!(rep.ok(), "{:?}", rep.failures);
        assert_eq!((rep.files, rep.bytes), (2, (3000 + 5) * 2));
        assert_eq!(read, rep.bytes, "진행 증분 = 읽은 바이트");
        // 내용 바꿈(크기 같음) · 크기 다름 · 사본 없음 · 종류 불일치.
        std::fs::write(t.join("a.bin"), vec![2u8; 3000]).unwrap();
        std::fs::write(t.join("sub").join("b.txt"), b"hell").unwrap();
        std::fs::write(s.join("c.txt"), b"c").unwrap();
        std::fs::write(s.join("d"), b"file").unwrap();
        std::fs::create_dir_all(t.join("d")).unwrap();
        let rep = verify_pairs(&[(s.clone(), t.clone())], &mut |_| {}, &cancel).unwrap();
        let mut why: Vec<String> = rep.failures.iter().map(|(_, w)| w.clone()).collect();
        why.sort();
        assert_eq!(
            why,
            vec![
                "content differs (SHA-256)".to_string(),
                "missing".to_string(),
                "size differs (5 vs 4)".to_string(),
                "type mismatch (file vs folder)".to_string(),
            ]
        );
        // 파일 쌍 직접 · 취소.
        let rep = verify_pairs(
            &[(s.join("sub/b.txt"), t.join("sub/b.txt"))],
            &mut |_| {},
            &cancel,
        )
        .unwrap();
        assert_eq!(rep.files, 1);
        assert!(!rep.ok());
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(verify_pairs(&[(s, t)], &mut |_| {}, &cancel).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
