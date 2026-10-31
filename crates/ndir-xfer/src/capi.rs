//! C ABI(T-202 ① · docs/26 §7 · §11-4 "패키지 동봉") — `ndir_xfer_run(job_json, cb, user, cancel) -> report_json`.
//!
//! - 문자열은 **UTF-8 JSON만** 오간다(§4 `JobSpec` · §5 진행 줄 · §6 `Report`) — 구조체 ABI를 노출하지 않아 판 호환이 쉽다.
//! - 산출물 = `libndir_xfer.so` · `libndir_xfer.dylib` · `ndir_xfer.dll`(rlib + cdylib 한 크레이트 · nexa-sql D-350 꼴) · 헤더 =
//!   `include/ndir_xfer.h`(수작업 · cbindgen 안 씀 · DR-8).
//! - 규칙: 패닉은 경계를 넘지 않는다(`catch_unwind` → 실패 Report — 단 release 프로필은 `panic = "abort"`라 그때는 **호출 프로세스 종료** ·
//!   그래서 입력 오류는 전부 `Result`로 돌리고 이 모듈은 unwrap을 쓰지 않는다 · 협업 분석 10-31) · 모든 결과 문자열은 [`ndir_xfer_free`]로만 해제 ·
//!   진행 콜백은 이 호출 동안 **호출자 스레드 또는 엔진 워커 스레드**에서 불릴 수 있다(콜백은 짧게 · 동기화는 호출자 몫) ·
//!   상대 경로는 프로세스 작업 디렉터리 기준(잡 파일이 없으므로 — §11-1의 "잡 파일 폴더" 규칙은 `nexa-copy --job` 몫 · 절대 경로 권장).

use crate::report::{Progress, Report};
use crate::spec::JobSpec;
use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::AtomicBool;

/// ABI 판 — 함수 집합·JSON 꼴이 호환되지 않게 바뀌면 올린다(헤더 `NDIR_XFER_ABI_VERSION`과 같아야 한다).
pub const ABI_VERSION: u32 = 1;

/// 진행 콜백 — `line` = Progress JSON 한 줄(NUL 종료 · 콜백이 돌아오면 무효 — 보관하려면 복사) · `user` = 호출자가 준 값 그대로.
pub type ProgressCb = Option<unsafe extern "C" fn(line: *const c_char, user: *mut c_void)>;

/// ABI 판 조회.
#[no_mangle]
pub extern "C" fn ndir_xfer_abi_version() -> u32 {
    ABI_VERSION
}

/// 엔진 판("0.28.0" 꼴) — 정적 문자열 · 해제하지 않는다.
#[no_mangle]
pub extern "C" fn ndir_xfer_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr().cast()
}

/// 작업 실행 — `job` = JobSpec JSON · 돌려줌 = Report JSON(오류도 `{"ok":false,"errors":[…]}` 꼴 · NULL 없음).
///
/// # Safety
/// - `job`은 NUL 종료 UTF-8이거나 NULL(NULL = 실패 Report).
/// - `cancel`은 NULL이거나 이 호출이 돌아올 때까지 유효한 `bool` 1바이트(`false` = 계속 · `true` = 협력적 취소 · 다른 스레드에서 써도 된다).
/// - `cb`는 NULL이거나 이 호출 동안 유효한 함수 · `user`는 그대로 넘겨진다 · 콜백은 호출자 스레드 또는 엔진 워커 스레드에서 불릴 수 있다.
/// - 돌려준 포인터는 [`ndir_xfer_free`]로 해제한다.
#[no_mangle]
pub unsafe extern "C" fn ndir_xfer_run(
    job: *const c_char,
    cb: ProgressCb,
    user: *mut c_void,
    cancel: *const bool,
) -> *mut c_char {
    guarded(|| {
        let spec = match job_spec(job) {
            Ok(s) => s,
            Err(e) => return failed(e),
        };
        let never = AtomicBool::new(false);
        // SAFETY: 호출자 계약 — `cancel`은 이 호출 동안 유효한 bool 1바이트(원자 접근 가능).
        let cancel: &AtomicBool = if cancel.is_null() {
            &never
        } else {
            AtomicBool::from_ptr(cancel.cast_mut())
        };
        let mut on_progress = |p: &Progress| {
            if let Some(f) = cb {
                let line = CString::new(p.to_json_line()).unwrap_or_default();
                // SAFETY: 호출자 계약 — `cb` · `user`는 이 호출 동안 유효.
                unsafe { f(line.as_ptr(), user) };
            }
        };
        into_c(crate::run(&spec, &mut on_progress, cancel).to_json())
    })
}

/// 명세 사전 검증만(JSON 파싱 + [`crate::validate`] — 파일 시스템 존재) · 돌려줌 = Report JSON(`ok` = 통과 · `errors` = 사유).
///
/// # Safety
/// `job`은 NUL 종료 UTF-8이거나 NULL · 돌려준 포인터는 [`ndir_xfer_free`]로 해제한다.
#[no_mangle]
pub unsafe extern "C" fn ndir_xfer_validate(job: *const c_char) -> *mut c_char {
    guarded(|| match job_spec(job) {
        Err(e) => failed(e),
        Ok(spec) => match crate::validate(&spec) {
            Some(e) => failed(e),
            None => into_c(
                Report {
                    ok: true,
                    dry_run: spec.dry_run,
                    ..Report::default()
                }
                .to_json(),
            ),
        },
    })
}

/// 이 라이브러리가 돌려준 문자열 해제(NULL 허용 · 두 번 해제 금지).
///
/// # Safety
/// `s`는 NULL이거나 [`ndir_xfer_run`] · [`ndir_xfer_validate`]가 돌려준 포인터(아직 해제하지 않은 것).
#[no_mangle]
pub unsafe extern "C" fn ndir_xfer_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: 호출자 계약 — `CString::into_raw`로 넘긴 포인터.
        drop(CString::from_raw(s));
    }
}

/// 패닉 울타리 — 경계를 넘는 패닉은 UB/abort라 실패 Report로 바꾼다.
fn guarded(f: impl FnOnce() -> *mut c_char) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(p) => p,
        Err(_) => failed("internal error: engine panicked"),
    }
}

/// `job` 포인터 → JobSpec(NULL · UTF-8 · JSON 오류는 글로).
///
/// # Safety
/// `job`은 NULL이거나 NUL 종료 문자열.
unsafe fn job_spec(job: *const c_char) -> Result<JobSpec, String> {
    if job.is_null() {
        return Err("job json is NULL".into());
    }
    // SAFETY: 위에서 NULL을 걸렀고 호출자 계약으로 NUL 종료.
    let text = CStr::from_ptr(job)
        .to_str()
        .map_err(|_| "job json is not valid UTF-8".to_string())?;
    JobSpec::from_json(text)
}

fn failed(msg: impl Into<String>) -> *mut c_char {
    into_c(Report::failed_with(msg).to_json())
}

/// Rust 문자열 → 소유권 넘기는 C 문자열. JSON dump는 제어 문자를 `\uXXXX`로 쓰므로 내부 NUL이 없지만 방어적으로 걸러 둔다.
fn into_c(s: String) -> *mut c_char {
    let c = CString::new(s).unwrap_or_else(|e| {
        let bytes: Vec<u8> = e.into_vec().into_iter().filter(|b| *b != 0).collect();
        CString::new(bytes).unwrap_or_default()
    });
    c.into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 결과 포인터 → String(해제까지).
    unsafe fn take(p: *mut c_char) -> String {
        assert!(!p.is_null());
        let s = CStr::from_ptr(p).to_string_lossy().into_owned();
        ndir_xfer_free(p);
        s
    }

    fn sandbox(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = std::env::temp_dir().join(format!(
            "ndir-xfer-capi-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    unsafe extern "C" fn count_cb(line: *const c_char, user: *mut c_void) {
        let text = CStr::from_ptr(line).to_str().unwrap();
        assert!(text.starts_with("{\"phase\":"), "진행 줄 꼴: {text}");
        (*user.cast::<AtomicUsize>()).fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn version_and_abi() {
        assert_eq!(ndir_xfer_abi_version(), ABI_VERSION);
        let v = unsafe { CStr::from_ptr(ndir_xfer_version()) }
            .to_str()
            .unwrap();
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn null_and_bad_input_give_failed_report() {
        let s = unsafe {
            take(ndir_xfer_run(
                std::ptr::null(),
                None,
                std::ptr::null_mut(),
                std::ptr::null(),
            ))
        };
        assert!(s.contains("\"ok\":false") && s.contains("NULL"), "{s}");
        let bad = CString::new("{\"sources\":").unwrap();
        let s = unsafe {
            take(ndir_xfer_run(
                bad.as_ptr(),
                None,
                std::ptr::null_mut(),
                std::ptr::null(),
            ))
        };
        assert!(s.contains("\"ok\":false"), "{s}");
        let utf = CString::new(vec![0xffu8, 0xfe]).unwrap();
        let s = unsafe { take(ndir_xfer_validate(utf.as_ptr())) };
        assert!(s.contains("UTF-8"), "{s}");
        unsafe { ndir_xfer_free(std::ptr::null_mut()) };
    }

    #[test]
    fn validate_reports_missing_dest() {
        let spec = JobSpec {
            sources: vec![std::env::temp_dir()],
            dest: std::env::temp_dir().join("ndir-xfer-capi-no-such-dir-0"),
            ..JobSpec::default()
        };
        let job = CString::new(spec.to_json()).unwrap();
        let s = unsafe { take(ndir_xfer_validate(job.as_ptr())) };
        assert!(
            s.contains("\"ok\":false") && s.contains("destination"),
            "{s}"
        );
    }

    #[test]
    fn run_copies_and_calls_progress() {
        let root = sandbox("copy");
        let src = root.join("a.txt");
        std::fs::write(&src, b"hello").unwrap();
        let dest = root.join("out");
        std::fs::create_dir(&dest).unwrap();
        let spec = JobSpec {
            sources: vec![src],
            dest: dest.clone(),
            ..JobSpec::default()
        };
        let job = CString::new(spec.to_json()).unwrap();
        let ok = unsafe { take(ndir_xfer_validate(job.as_ptr())) };
        assert!(ok.contains("\"ok\":true"), "{ok}");
        let calls = AtomicUsize::new(0);
        let cancel = false;
        let s = unsafe {
            take(ndir_xfer_run(
                job.as_ptr(),
                Some(count_cb),
                (&calls as *const AtomicUsize).cast_mut().cast(),
                &cancel,
            ))
        };
        assert!(s.contains("\"ok\":true"), "{s}");
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"hello");
        assert!(calls.load(Ordering::SeqCst) >= 1, "진행 콜백 0회");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn preset_cancel_flag_is_honored() {
        let root = sandbox("cancel");
        let src = root.join("a.txt");
        std::fs::write(&src, b"hello").unwrap();
        let dest = root.join("out");
        std::fs::create_dir(&dest).unwrap();
        let spec = JobSpec {
            sources: vec![src],
            dest: dest.clone(),
            ..JobSpec::default()
        };
        let job = CString::new(spec.to_json()).unwrap();
        let cancel = true;
        let s = unsafe {
            take(ndir_xfer_run(
                job.as_ptr(),
                None,
                std::ptr::null_mut(),
                &cancel,
            ))
        };
        assert!(s.contains("\"canceled\":true"), "{s}");
        assert!(!dest.join("a.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
