//! **화면 내보내기 한 곳**(T-147 · D-133 ② · docs/62 §2) — 모든 창이 이 타입으로 픽셀을 낸다.
//!
//! - 기본 = `softbuffer`(3-OS 공통 · 종전 그대로).
//! - macOS이고 설정 `gfx.mac_present = iosurface`(선택 · 기본은 softbuffer)면 nexa-sys `LayerPresenter`(IOSurface 풀 · 할당 0 · 복사 0 ·
//!   색 맞춤은 합성기) — 만들기에 실패하면 조용히 `softbuffer`로 돌아간다.
//!
//! 모양은 `softbuffer::Surface`와 같다(`resize` → `buffer_mut` → 그리기 → `present`) — 창 코드가 뒷단을 모른다.
//! 방식은 **창을 만들 때** 정해진다(설정을 바꾸면 새로 여는 창부터 · 메인 창은 재시작 뒤).
//!
//! ★ 출처: nexa-sql/crates/nexa-sql/src/present.rs(10-03 복사 · docs/port/40 SKEL-403 — `NSQL_*`→`NDIR_*` · 제품명만 개명 · 로직 불변).

use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use winit::window::Window;

/// 설정 `gfx.mac_present`가 `iosurface`인가(기동·설정 변경 때 호스트가 맞춘다).
static MAC_LAYER: AtomicBool = AtomicBool::new(false);

/// 설정값을 알린다(`"iosurface"` = 켬 · 그 밖 = `softbuffer` = 기본).
pub(crate) fn set_mode(setting: &str) {
    MAC_LAYER.store(setting == "iosurface", Ordering::Relaxed);
}

/// IOSurface 경로를 쓸 것인가 — macOS **이고** 설정이 켜져 있을 때만(순수 판정 · MC/DC 테스트).
pub(crate) fn use_layer(macos: bool, setting_on: bool) -> bool {
    macos && setting_on
}

type Soft = softbuffer::Surface<Rc<Window>, Rc<Window>>;

enum Kind {
    Soft {
        surface: Soft,
        /// 컨텍스트는 표면보다 오래 살아야 한다.
        _ctx: softbuffer::Context<Rc<Window>>,
    },
    Layer {
        p: nexa_sys::layer_present::LayerPresenter,
        win: Rc<Window>,
    },
}

/// 창 하나의 내보내기.
pub(crate) struct Presenter {
    kind: Kind,
}

/// 그릴 버퍼(`[u32]` = `0x00RRGGBB` · 폭 × 높이). 다 그린 뒤 [`Buffer::present`].
pub(crate) enum Buffer<'a> {
    Soft(softbuffer::Buffer<'a, Rc<Window>, Rc<Window>>),
    Layer(nexa_sys::layer_present::Frame<'a>),
}

impl Presenter {
    /// 창에 붙인다. 실패 = `Err(사유)`.
    pub(crate) fn new(win: Rc<Window>) -> Result<Presenter, String> {
        if use_layer(cfg!(target_os = "macos"), MAC_LAYER.load(Ordering::Relaxed)) {
            if let Some(p) = layer_for(&win) {
                return Ok(Presenter {
                    kind: Kind::Layer { p, win },
                });
            }
        }
        let ctx = softbuffer::Context::new(win.clone())
            .map_err(|e| format!("softbuffer context failed: {e}"))?;
        let surface = softbuffer::Surface::new(&ctx, win)
            .map_err(|e| format!("softbuffer surface failed: {e}"))?;
        Ok(Presenter {
            kind: Kind::Soft { surface, _ctx: ctx },
        })
    }

    /// 픽셀 크기를 맞춘다.
    pub(crate) fn resize(&mut self, w: NonZeroU32, h: NonZeroU32) -> Result<(), ()> {
        match &mut self.kind {
            Kind::Soft { surface, .. } => surface.resize(w, h).map_err(|_| ()),
            Kind::Layer { p, win } => {
                if p.resize(w.get(), h.get(), win.scale_factor()) {
                    Ok(())
                } else {
                    Err(())
                }
            }
        }
    }

    /// 이번 프레임의 버퍼.
    pub(crate) fn buffer_mut(&mut self) -> Result<Buffer<'_>, ()> {
        match &mut self.kind {
            Kind::Soft { surface, .. } => surface.buffer_mut().map(Buffer::Soft).map_err(|_| ()),
            Kind::Layer { p, .. } => p.frame().map(Buffer::Layer).ok_or(()),
        }
    }

    /// 이번 프레임 버퍼 — 창 크기에 표면을 맞춘 뒤 준다. 크기 0(최소화)·실패 = `None`(docs/93 §4: 보조 창 11곳의
    /// "크기 확인 → resize → buffer_mut" 상용구를 한 곳으로).
    pub(crate) fn frame(&mut self, size: winit::dpi::PhysicalSize<u32>) -> Option<Buffer<'_>> {
        let (w, h) = (NonZeroU32::new(size.width)?, NonZeroU32::new(size.height)?);
        self.resize(w, h).ok()?;
        self.buffer_mut().ok()
    }

    /// 진단용 이름(`NDIR_TRACE_FRAMES`).
    pub(crate) fn backend(&self) -> &'static str {
        match self.kind {
            Kind::Soft { .. } => "softbuffer",
            Kind::Layer { .. } => "iosurface",
        }
    }
}

impl Buffer<'_> {
    /// 이 버퍼가 몇 프레임 전 것인가(softbuffer `age` · 1 = 지난 프레임 그대로 → 부분 다시 그리기 가능 · 0 = 새 버퍼(내용 미정 ·
    /// 크기 변경 뒤 · macOS CG) · 2+ = 다중 버퍼(Wayland 2 · IOSurface 삼중 회전 3 — nexa-sys 248 `Frame::age` · 10-31) →
    /// 지난 n−1 프레임의 damage를 합치면 부분 다시 그리기 가능(`paint::frame_plan`).
    pub(crate) fn age(&self) -> u8 {
        match self {
            Buffer::Soft(b) => b.age(),
            Buffer::Layer(f) => f.age(),
        }
    }

    /// 더러워진 사각형만 낸다(softbuffer `present_with_damage` · 10-18 결정 ①) — 레이어는 전체. 빈 목록 = 내지 않는다.
    pub(crate) fn present_damage(self, rects: &[nexa_ctl::Rect]) -> Result<(), ()> {
        match self {
            Buffer::Soft(b) => {
                let dmg: Vec<softbuffer::Rect> = rects
                    .iter()
                    .filter_map(|r| {
                        Some(softbuffer::Rect {
                            x: u32::try_from(r.x.max(0)).ok()?,
                            y: u32::try_from(r.y.max(0)).ok()?,
                            width: NonZeroU32::new(u32::try_from(r.w).ok()?)?,
                            height: NonZeroU32::new(u32::try_from(r.h).ok()?)?,
                        })
                    })
                    .collect();
                if dmg.is_empty() {
                    return Ok(());
                }
                b.present_with_damage(&dmg).map_err(|_| ())
            }
            Buffer::Layer(f) => {
                // IOSurface: 그린 사각형만 알파 채우기(nexa-sys 254 `present_damage` · T-201 ⓐ · 종전 = 표면 전체 ~1.6 ms).
                let dmg: Vec<(i32, i32, i32, i32)> =
                    rects.iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
                f.present_damage(&dmg);
                Ok(())
            }
        }
    }

    /// 화면에 낸다.
    pub(crate) fn present(self) -> Result<(), ()> {
        match self {
            Buffer::Soft(b) => b.present().map_err(|_| ()),
            Buffer::Layer(f) => {
                f.present();
                Ok(())
            }
        }
    }
}

impl Deref for Buffer<'_> {
    type Target = [u32];
    fn deref(&self) -> &[u32] {
        match self {
            Buffer::Soft(b) => b,
            Buffer::Layer(f) => f.pixels(),
        }
    }
}

impl DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut [u32] {
        match self {
            Buffer::Soft(b) => b,
            Buffer::Layer(f) => f.pixels_mut(),
        }
    }
}

/// 창의 `NSView`에 IOSurface 레이어를 단다(macOS만 · 그 밖 = `None`).
fn layer_for(win: &Rc<Window>) -> Option<nexa_sys::layer_present::LayerPresenter> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = win.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => {
            // SAFETY: winit이 준 살아 있는 NSView · `Presenter`가 창(`Rc<Window>`)을 함께 쥐어 뷰가 먼저 죽지 않는다 ·
            //         창 코드는 전부 메인 스레드(이벤트 루프)에서 돈다.
            unsafe { nexa_sys::layer_present::LayerPresenter::new(h.ns_view.as_ptr()) }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    /// MC/DC — 조건 둘이 각각 혼자 결과를 바꾼다.
    #[test]
    fn use_layer_mcdc() {
        use super::use_layer as f;
        assert!(f(true, true));
        assert!(!f(false, true), "macOS가 아니면 끔");
        assert!(!f(true, false), "설정이 softbuffer면 끔");
    }
}
