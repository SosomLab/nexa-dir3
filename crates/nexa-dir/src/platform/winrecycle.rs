//! Windows 휴지통 **복원**(T-51 B-2c · dir2 `recycle.rs` 이식 — docs/port/19 SHELL-049 · 22 OPS "일반 삭제 undo"): 휴지통 셸 폴더(`CSIDL_BITBUCKET`)를
//! 열거해 "원래 위치 + 이름"이 일치하는 항목을 찾아 셸 `undelete` 동사를 실행한다(탐색기 Ctrl+Z와 같은 메커니즘). 이름 매칭 = 정확 일치 우선 ·
//! 확장자 숨김 표시 폴백(같은 폴더 + 확장자 제외 이름) · 경로당 최초 일치 1건. `StrRetToBufW`(shlwapi) 대신 STRRET 직접 파싱.

use super::*;
use ::windows::core::{Interface, PCSTR};
use ::windows::Win32::Foundation::{HWND, S_OK};
use ::windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use ::windows::Win32::UI::Shell::Common::{ITEMIDLIST, SHELLDETAILS, STRRET};
use ::windows::Win32::UI::Shell::{
    IContextMenu, IEnumIDList, IShellFolder, IShellFolder2, SHGetDesktopFolder,
    SHGetSpecialFolderLocation, CMINVOKECOMMANDINFO, CSIDL_BITBUCKET, SHCONTF_FOLDERS,
    SHCONTF_INCLUDEHIDDEN, SHCONTF_NONFOLDERS,
};

/// 휴지통 상세 컬럼(0 = 이름 · 1 = 원래 위치 — XP 이후 고정 인덱스).
const COL_NAME: u32 = 0;
const COL_ORIGINAL_LOCATION: u32 = 1;

/// 원래 경로 목록에 해당하는 휴지통 항목을 복원 — 반환 = 복원 실행한 항목 수(요청보다 작을 수 있다).
pub(super) fn restore_by_original_paths(
    original_paths: &[PathBuf],
) -> Result<usize, PlatformError> {
    if original_paths.is_empty() {
        return Ok(0);
    }
    // SAFETY: 셸 COM — 이미 초기화된 STA면 S_FALSE(무시) · 모든 PIDL은 CoTaskMemFree.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        restore_inner(original_paths)
            .map_err(|e| PlatformError::Failed(format!("recycle bin: {e}")))
    }
}

unsafe fn restore_inner(original_paths: &[PathBuf]) -> ::windows::core::Result<usize> {
    let mut wanted: Vec<String> = original_paths
        .iter()
        .map(|p| p.to_string_lossy().trim_end_matches(['\\', '/']).to_owned())
        .collect();
    let desktop: IShellFolder = SHGetDesktopFolder()?;
    let bin_pidl = SHGetSpecialFolderLocation(None, CSIDL_BITBUCKET as i32)?;
    let result = (|| -> ::windows::core::Result<usize> {
        let bin2: IShellFolder2 = desktop.BindToObject(bin_pidl, None)?;
        let bin: IShellFolder = bin2.cast()?;
        let mut enum_opt: Option<IEnumIDList> = None;
        bin.EnumObjects(
            HWND::default(),
            (SHCONTF_FOLDERS.0 | SHCONTF_NONFOLDERS.0 | SHCONTF_INCLUDEHIDDEN.0) as u32,
            &mut enum_opt,
        )
        .ok()?;
        let e = enum_opt.ok_or_else(::windows::core::Error::empty)?;
        let mut matched: Vec<*mut ITEMIDLIST> = Vec::new();
        let mut all_pidls: Vec<*mut ITEMIDLIST> = Vec::new();
        let restored = (|| -> ::windows::core::Result<usize> {
            while !wanted.is_empty() {
                let mut child: [*mut ITEMIDLIST; 1] = [std::ptr::null_mut()];
                let mut fetched = 0u32;
                if e.Next(&mut child, Some(&mut fetched)) != S_OK || fetched != 1 {
                    break;
                }
                all_pidls.push(child[0]);
                let name = details_of(&bin2, child[0], COL_NAME);
                let loc = details_of(&bin2, child[0], COL_ORIGINAL_LOCATION);
                if name.is_empty() || loc.is_empty() {
                    continue;
                }
                let loc = loc.trim_end_matches(['\\', '/']);
                let original = format!("{loc}\\{name}");
                let idx = wanted
                    .iter()
                    .position(|w| w.eq_ignore_ascii_case(&original))
                    .or_else(|| {
                        // 확장자 숨김 표시 폴백: 같은 폴더 + 확장자 제외 이름 일치.
                        wanted.iter().position(|w| {
                            let p = Path::new(w);
                            let dir = p.parent().map(|d| d.to_string_lossy()).unwrap_or_default();
                            let stem = p
                                .file_stem()
                                .map(|s| s.to_string_lossy())
                                .unwrap_or_default();
                            dir.trim_end_matches(['\\', '/']).eq_ignore_ascii_case(loc)
                                && stem.eq_ignore_ascii_case(&name)
                        })
                    });
                if let Some(idx) = idx {
                    matched.push(child[0]);
                    wanted.remove(idx);
                }
            }
            if matched.is_empty() {
                return Ok(0);
            }
            let apidl: Vec<*const ITEMIDLIST> =
                matched.iter().map(|p| *p as *const ITEMIDLIST).collect();
            let icm: IContextMenu = bin.GetUIObjectOf(HWND::default(), &apidl, None)?;
            let inv = CMINVOKECOMMANDINFO {
                cbSize: std::mem::size_of::<CMINVOKECOMMANDINFO>() as u32,
                lpVerb: PCSTR(c"undelete".as_ptr() as *const u8),
                nShow: 1,
                ..Default::default()
            };
            icm.InvokeCommand(&inv)?;
            Ok(matched.len())
        })();
        for p in all_pidls {
            CoTaskMemFree(Some(p as *const core::ffi::c_void));
        }
        restored
    })();
    CoTaskMemFree(Some(bin_pidl as *const core::ffi::c_void));
    result
}

/// 휴지통 상세 컬럼 텍스트 — 개별 항목 실패는 빈 문자열(격리).
unsafe fn details_of(folder: &IShellFolder2, pidl: *mut ITEMIDLIST, col: u32) -> String {
    let mut sd = SHELLDETAILS::default();
    if folder
        .GetDetailsOf(pidl as *const ITEMIDLIST, col, &mut sd)
        .is_err()
    {
        return String::new();
    }
    // SHELLDETAILS는 packed — str 필드를 정렬된 지역으로 복사한 뒤 파싱.
    let mut strret = sd.str;
    strret_to_string(&mut strret, pidl)
}

/// STRRET → String(WSTR = CoTaskMem 해제까지 · CSTR/OFFSET = ANSI 최선).
unsafe fn strret_to_string(s: &mut STRRET, pidl: *mut ITEMIDLIST) -> String {
    const WSTR: u32 = 0;
    const OFFSET: u32 = 1;
    const CSTR: u32 = 2;
    match s.uType {
        WSTR => {
            let p = s.Anonymous.pOleStr;
            if p.is_null() {
                return String::new();
            }
            let out = p.to_string().unwrap_or_default();
            CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
            out
        }
        CSTR => {
            let bytes = &s.Anonymous.cStr;
            let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..len]).into_owned()
        }
        OFFSET => {
            let base = pidl as *const u8;
            let p = base.add(s.Anonymous.uOffset as usize);
            let mut len = 0usize;
            while *p.add(len) != 0 {
                len += 1;
            }
            String::from_utf8_lossy(std::slice::from_raw_parts(p, len)).into_owned()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 없는 경로 = 복원 0(오류 아님) · 빈 목록 = 0 — 실제 휴지통을 열거만 한다(변경 없음).
    #[test]
    fn restore_unknown_path_is_zero() {
        assert_eq!(restore_by_original_paths(&[]).unwrap(), 0);
        let bogus = std::env::temp_dir().join("ndir-recycle-never-existed-xyz.txt");
        assert_eq!(
            restore_by_original_paths(std::slice::from_ref(&bogus)).unwrap(),
            0
        );
    }

    /// 실 휴지통 왕복(삭제 → 복원 → 원위치) — 사용자 휴지통을 건드리므로 `--ignored`(실기 QA · selfcheck가 non-CI에서 같은 일을 한다).
    #[test]
    #[ignore = "실 휴지통 부수효과 — cargo test -p nexa-dir -- --ignored"]
    fn recycle_round_trip_restores_original() {
        let dir = std::env::temp_dir().join(format!("ndir-recycle-qa-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("restore-me-{}.txt", std::process::id()));
        std::fs::write(&file, "restore me").unwrap();
        let t = super::super::windows::NativeTrash;
        assert_eq!(t.trash(std::slice::from_ref(&file)).unwrap(), 1);
        assert!(!file.exists());
        assert_eq!(t.restore(std::slice::from_ref(&file)).unwrap(), 1);
        assert!(file.exists(), "원위치로 복원");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "restore me");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
