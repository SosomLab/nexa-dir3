//! 앱 아이콘 — **dir2 자원 그대로**(사용자 요구 "자원 유지" · docs/port/40 SKEL-424): `packaging/branding/nexa-dir-256.png`
//! (dir2 `packaging/branding` SSOT · `.ico`는 Windows `.rc` 몫)를 실행 파일에 넣고 첫 사용 때 nexa-gfx로 디코드한다.
//!
//! 구조는 nexa-sql `icon.rs`(코드로 그린 아이콘)에서 **그림 원천만** 바꿨다: `with_icon`(창 속성 · 모든 창이 지난다) ·
//! `set_dock_icon`(macOS Dock · 번들 없는 `cargo run`도 같은 그림) · `icon_rgba(side)`(RGBA 직선 알파).
//! 256 → 32·64는 **정수 배율 상자 축소**(8×8 · 4×4 알파 가중 평균 — 재샘플 오차 0) · 그 밖 크기는 최근접.
//!
//! | OS | 창 아이콘의 효과 |
//! |---|---|
//! | Windows | 타이틀바(小 32) + 작업표시줄(大 64 — `with_taskbar_icon`) · exe 아이콘은 `.rc`(`build.rs`) |
//! | Linux/X11 | 태스크바·창 전환기 · Wayland는 `.desktop`(`app_id` = `nexa-dir`) + hicolor PNG 몫 |
//! | macOS | 창 아이콘은 무시 · **Dock = [`set_dock_icon`]** |

use std::sync::OnceLock;

/// dir2 256px PNG(RGBA · 투명 모서리).
static PNG_256: &[u8] = include_bytes!("../../../packaging/branding/nexa-dir-256.png");
const SIDE_SRC: u32 = 256;

/// 디코드 결과(한 번만 · 실패 = `None` → 아이콘 없이 진행).
fn source() -> Option<&'static [u8]> {
    static SRC: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    SRC.get_or_init(|| {
        let img = nexa_gfx::image::decode(PNG_256, (SIDE_SRC * SIDE_SRC) as usize).ok()?;
        (img.w == SIDE_SRC
            && img.h == SIDE_SRC
            && img.rgba.len() == (SIDE_SRC * SIDE_SRC * 4) as usize)
            .then_some(img.rgba)
    })
    .as_deref()
}

/// `src`(`from`×`from` RGBA)를 `side`×`side`로 — 정수 배율이면 알파 가중 상자 평균 · 아니면 최근접.
pub(crate) fn resample(src: &[u8], from: u32, side: u32) -> Vec<u8> {
    let side = side.max(1);
    let mut out = Vec::with_capacity((side * side * 4) as usize);
    if from.is_multiple_of(side) && from >= side {
        let f = from / side;
        let n = u64::from(f * f);
        for py in 0..side {
            for px in 0..side {
                let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
                for sy in 0..f {
                    for sx in 0..f {
                        let i = (((py * f + sy) * from + (px * f + sx)) * 4) as usize;
                        let al = u64::from(src[i + 3]);
                        r += u64::from(src[i]) * al;
                        g += u64::from(src[i + 1]) * al;
                        b += u64::from(src[i + 2]) * al;
                        a += al;
                    }
                }
                // 알파 가중 평균(덮인 샘플만 · 전부 투명이면 투명).
                match (r.checked_div(a), g.checked_div(a), b.checked_div(a)) {
                    (Some(r), Some(g), Some(b)) => {
                        out.extend_from_slice(&[r as u8, g as u8, b as u8, (a / n) as u8]);
                    }
                    _ => out.extend_from_slice(&[0, 0, 0, 0]),
                }
            }
        }
    } else {
        for py in 0..side {
            for px in 0..side {
                let sx = (u64::from(px) * u64::from(from) / u64::from(side)) as u32;
                let sy = (u64::from(py) * u64::from(from) / u64::from(side)) as u32;
                let i = ((sy * from + sx) * 4) as usize;
                out.extend_from_slice(&src[i..i + 4]);
            }
        }
    }
    out
}

/// `side`×`side` RGBA(직선 알파). 디코드 실패 = 전부 투명(fail-soft · 창은 뜬다).
pub(crate) fn icon_rgba(side: u32) -> Vec<u8> {
    match source() {
        Some(src) => resample(src, SIDE_SRC, side),
        None => vec![0; (side.max(1) * side.max(1) * 4) as usize],
    }
}

/// 창 속성에 아이콘을 붙인다 — 모든 창(메인·설정·대화상자)이 이 한 곳을 지난다. 변환 실패 = 아이콘 없이(fail-soft).
pub(crate) fn with_icon(attrs: winit::window::WindowAttributes) -> winit::window::WindowAttributes {
    let small = winit::window::Icon::from_rgba(icon_rgba(32), 32, 32).ok();
    #[cfg(windows)]
    let attrs = {
        use winit::platform::windows::WindowAttributesExtWindows as _;
        let large = winit::window::Icon::from_rgba(icon_rgba(64), 64, 64).ok();
        attrs.with_taskbar_icon(large)
    };
    // Linux: Wayland는 창 아이콘을 앱이 직접 줄 수 없다 — 합성기가 **`app_id`와 같은 이름의 `.desktop` 파일의 `Icon=`** 을 쓴다.
    // `app_id` = `nexa-dir` · X11은 같은 값이 WM_CLASS.
    #[cfg(all(unix, not(target_os = "macos")))]
    let attrs = {
        use winit::platform::wayland::WindowAttributesExtWayland;
        use winit::platform::x11::WindowAttributesExtX11;
        let attrs = WindowAttributesExtX11::with_name(attrs, "nexa-dir", "nexa-dir");
        WindowAttributesExtWayland::with_name(attrs, "nexa-dir", "nexa-dir")
    };
    // 자체 시험(`NDIR_NO_ACTIVATE=1` · 하네스 T5): 창을 **활성화하지 않고** 띄운다 — 캡처용 격리 인스턴스가 사용자의 전경 포커스·
    // 키 입력을 빼앗지 않게(nexa-sql 09-21 사고). 평소엔 변수 없음 = 그대로.
    let attrs = if std::env::var_os("NDIR_NO_ACTIVATE").is_some() {
        attrs.with_active(false)
    } else {
        attrs
    };
    attrs.with_window_icon(small)
}

/// Dock 아이콘(macOS) — 번들 없이 실행하면(`cargo run`) Dock에 셸 실행 파일 아이콘(`exec`)이 뜬다 →
/// `NSApplication.setApplicationIconImage`로 같은 그림(256px). **메인 스레드 · 이벤트 루프 생성 뒤**(NSApp 존재). 다른 OS no-op.
pub(crate) fn set_dock_icon() {
    imp::set_dock_icon();
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2::msg_send_id;
    use objc2::rc::Retained;
    use objc2::ClassType as _; // `alloc`
    use objc2_app_kit::{NSApplication, NSBitmapImageRep, NSImage};
    use objc2_foundation::{MainThreadMarker, NSSize, NSString};

    /// Dock 타일은 128@2x까지 쓴다 — 256이면 어느 배율에서도 재샘플 없음.
    const SIDE: u32 = 256;

    pub(super) fn set_dock_icon() {
        let Some(mtm) = MainThreadMarker::new() else {
            return; // AppKit 규약 — 메인 스레드가 아니면 하지 않는다(fail-soft)
        };
        let Some(img) = image_from_rgba(&super::icon_rgba(SIDE), SIDE) else {
            return;
        };
        // SAFETY: 메인 스레드 · NSApp 생성 뒤 · 인자는 살아 있는 NSImage.
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&img)) };
    }

    /// RGBA(직선 알파) → NSImage(nexa-sql `icon.rs` ← nexa-clip `tray.rs` 이식).
    fn image_from_rgba(rgba: &[u8], side: u32) -> Option<Retained<NSImage>> {
        if side == 0 || rgba.len() != (side as usize) * (side as usize) * 4 {
            return None;
        }
        // SAFETY: AppKit 생성자 호출 · rep가 소유한 버퍼는 side*side*4 바이트(bitmapData ≠ null 확인 뒤 복사).
        unsafe {
            let rep: Option<Retained<NSBitmapImageRep>> = msg_send_id![
                NSBitmapImageRep::alloc(),
                initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>(),
                pixelsWide: side as isize,
                pixelsHigh: side as isize,
                bitsPerSample: 8_isize,
                samplesPerPixel: 4_isize,
                hasAlpha: true,
                isPlanar: false,
                colorSpaceName: &*NSString::from_str("NSDeviceRGBColorSpace"),
                bytesPerRow: (side * 4) as isize,
                bitsPerPixel: 32_isize,
            ];
            let rep = rep?;
            let data = rep.bitmapData();
            if data.is_null() {
                return None;
            }
            std::ptr::copy_nonoverlapping(rgba.as_ptr(), data, rgba.len());
            let img = NSImage::initWithSize(
                NSImage::alloc(),
                NSSize::new(f64::from(side), f64::from(side)),
            );
            img.addRepresentation(&rep);
            Some(img)
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub(super) fn set_dock_icon() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// dir2 PNG가 디코드되고(256²) 32·64 축소본이 "보이는 그림"이다(불투명 픽셀이 과반 · 모서리는 투명 — 둥근 타일).
    #[test]
    fn dir2_png_decodes_and_downscales() {
        let src = source().expect("dir2 nexa-dir-256.png decodes");
        assert_eq!(src.len(), (256 * 256 * 4) as usize);
        for side in [32u32, 64, 256] {
            let px = icon_rgba(side);
            assert_eq!(px.len(), (side * side * 4) as usize, "{side}");
            let opaque = px.chunks(4).filter(|p| p[3] > 128).count();
            assert!(
                opaque * 2 > (side * side) as usize,
                "{side}: opaque {opaque}"
            );
            assert_eq!(px[3], 0, "{side}: 왼쪽 위 모서리 = 투명(둥근 타일)");
        }
    }

    /// 상자 축소: 2×2 → 1×1 알파 가중 평균 · 비정수 배율 = 최근접.
    #[test]
    fn resample_box_and_nearest() {
        // (빨강 α255) (파랑 α0) / (빨강 α255) (빨강 α255)
        let src = [255, 0, 0, 255, 0, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255];
        let out = resample(&src, 2, 1);
        assert_eq!(
            out,
            vec![255, 0, 0, 191],
            "투명 픽셀은 색에 안 섞이고 알파만 1/4 깎인다"
        );
        let near = resample(&src, 2, 3);
        assert_eq!(near.len(), 3 * 3 * 4);
        assert_eq!(&near[..4], &[255, 0, 0, 255]);
    }

    /// `NDIR_ICON_DUMP=<경로.ppm>`이면 256px를 PPM으로 떨어뜨린다(육안 검수용 · 기본 무시).
    #[test]
    fn dump_ppm_when_asked() {
        let Ok(path) = std::env::var("NDIR_ICON_DUMP") else {
            return;
        };
        let s = 256u32;
        let px = icon_rgba(s);
        let mut out = format!("P6\n{s} {s}\n255\n").into_bytes();
        for p in px.chunks(4) {
            let a = u32::from(p[3]);
            for c in &p[..3] {
                out.push(((u32::from(*c) * a + 255 * (255 - a)) / 255) as u8);
            }
        }
        std::fs::write(path, out).expect("write ppm");
    }
}
