//! 메뉴 항목 앞 **아이콘**(사용자 10-05 "복사/잘라내기/붙여넣기/전체 선택 등 메뉴 앞에 이미지 · 운영체제의 정보를 최대한 활용").
//!
//! 공급 계층(위가 이긴다):
//! 1. **OS 아이콘 글꼴** — Windows의 `Segoe Fluent Icons`(11) · `Segoe MDL2 Assets`(10): 탐색기 · 설정 앱의 메뉴가 쓰는 바로 그
//!    글리프라 OS와 같은 그림이 된다. UI 글꼴 사슬에 이미 들어 있으므로(네비 버튼 · 쉐브론과 같은 길) 글리프를 알파 마스크로 래스터한다.
//! 2. **내장 도형**(nexa-ctl `glyphs` — 코드로 그리는 마스크 · 3-OS 동일): 아이콘 글꼴이 없는 OS(macOS · Linux)와 글리프가 빠진 글꼴.
//!    macOS SF Symbols(`NSImage(systemSymbolName:)`) · Linux freedesktop 아이콘 테마(`edit-copy` 등)는 후속 계층으로 얹을 자리다.
//! 3. 없음(그 항목은 아이콘 없이 — 메뉴는 아이콘 칸만 맞춘다).
//!
//! 마스크는 색이 없다 — 우클릭/보조 메뉴는 nexa-ctl이 상태색(보통 · hover · 비활성)으로 틴트하고, 메뉴 바(풀다운)는 그림을 RGBA로
//! 받으므로 테마 글자색으로 미리 칠해 넣는다(테마가 바뀌면 다시 만든다).
//!
//! 켬/끔 = 설정 `menu.icons`(성능 향상 모드면 끔) — 우클릭 쪽은 nexa-ctl 전역 스위치, 메뉴 바 쪽은 이 모듈의 [`set_bar`].

use nexa_ctl::controls::{glyph, CtxItem, GlyphKind, MenuIcon};
use nexa_gfx::{Color, Font, IconImage, Surface};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// 마스크 한 변(px) — nexa-ctl 도형 아이콘과 같은 크기(그릴 때 줄인다).
const SIDE: usize = 64;
/// 글리프를 그리는 크기(px) — 마스크 안에 여백을 두고 들어가는 값.
const GLYPH_PX: f32 = 46.0;

/// 명령 id → (OS 아이콘 글꼴 코드포인트, 내장 도형 폴백). 접두 일치도 본다(`aux.launch.edit:3` 같은 동적 id).
/// 코드포인트 = Segoe MDL2 Assets / Fluent Icons 공통 자리.
fn spec_of(id: &str) -> Option<(char, Option<GlyphKind>)> {
    let exact = match id {
        "edit.copy" | "aux.git.copy" => ('\u{E8C8}', Some(GlyphKind::Copy)),
        "edit.cut" => ('\u{E8C6}', None),
        "edit.paste" => ('\u{E77F}', None),
        "edit.delete" | "aux.launch.remove" => ('\u{E74D}', None),
        "edit.select_all" => ('\u{E8B3}', None),
        "edit.undo" => ('\u{E7A7}', None),
        "edit.redo" => ('\u{E7A6}', None),
        "edit.rename" | "edit.bulk_rename" => ('\u{E8AC}', Some(GlyphKind::Text)),
        "file.new_folder" | "new.folder" => ('\u{E8F4}', Some(GlyphKind::FolderNew)),
        "file.new_file" | "new.file" => ('\u{E7C3}', None),
        "ctx.new" => ('\u{E710}', Some(GlyphKind::FolderNew)),
        "file.new_tab" | "aux.launch.add" => ('\u{E710}', None),
        "file.close_tab" | "file.exit" => ('\u{E711}', None),
        "file.prefs" | "aux.prefs" => ('\u{E713}', None),
        "view.refresh" | "aux.refresh" => ('\u{E72C}', Some(GlyphKind::Refresh)),
        "view.preview_window" => ('\u{E8A7}', Some(GlyphKind::Open)),
        "cmd.activate" => ('\u{E8E5}', Some(GlyphKind::Open)),
        "ctx.copy_path" => ('\u{E71B}', Some(GlyphKind::Link)),
        "ctx.copy_name" => ('\u{E8D2}', Some(GlyphKind::Text)),
        "nav.back" => ('\u{E72B}', Some(GlyphKind::ArrowBack)),
        "nav.forward" => ('\u{E72A}', Some(GlyphKind::ArrowForward)),
        "nav.up" => ('\u{E74A}', Some(GlyphKind::ArrowUp)),
        "aux.info" | "help.about" => ('\u{E946}', None),
        "aux.tb.order" | "aux.sb.edit" | "aux.col.order" => ('\u{E70F}', None),
        "aux.launch.hide" => ('\u{ED1A}', None),
        "help.license" => ('\u{E8D7}', None),
        "help.selfcheck" => ('\u{E9D9}', None),
        _ => {
            return if id.starts_with("aux.launch.edit") {
                Some(('\u{E70F}', None))
            } else if id.starts_with("aux.launch.remove") {
                Some(('\u{E74D}', None))
            } else if id.starts_with("new.tpl:") {
                Some(('\u{E7C3}', None))
            } else {
                None
            };
        }
    };
    Some(exact)
}

/// 글리프 하나를 `SIDE`×`SIDE` 알파 마스크로(가운데 정렬) — 글꼴에 그 글자가 없으면 `None`.
fn glyph_mask(font: &Font, ch: char) -> Option<Vec<u8>> {
    if !font.covers(ch) {
        return None;
    }
    // 넉넉한 캔버스에 흰 글자를 그린 뒤 잉크 범위를 찾아 가운데로 옮긴다(글꼴마다 여백 · 기준선이 다르다).
    let big = SIDE * 2;
    let mut buf = vec![0u32; big * big];
    {
        let mut surface = Surface::new(&mut buf, big, big);
        let s = ch.to_string();
        let w = font.measure(&s, GLYPH_PX);
        let x = (big as f32 - w) / 2.0;
        let y = big as f32 / 2.0 + font.ascent(GLYPH_PX) / 2.0;
        font.draw_text(&mut surface, x, y, GLYPH_PX, Color(0x00FF_FFFF), &s);
    }
    let cov = |i: usize| (buf[i] & 0xFF) as u8;
    let (mut x0, mut y0, mut x1, mut y1) = (big, big, 0usize, 0usize);
    for y in 0..big {
        for x in 0..big {
            if cov(y * big + x) > 8 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < x0 || y1 < y0 {
        return None; // 빈 글리프
    }
    let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
    if w > SIDE || h > SIDE {
        return None;
    }
    let (ox, oy) = ((SIDE - w) / 2, (SIDE - h) / 2);
    let mut out = vec![0u8; SIDE * SIDE];
    for y in 0..h {
        for x in 0..w {
            out[(oy + y) * SIDE + ox + x] = cov((y0 + y) * big + x0 + x);
        }
    }
    Some(out)
}

struct State {
    font: Option<Rc<Font>>,
    /// 메뉴 바 그림을 켤 것인가 + 칠할 글자색.
    bar: Option<Color>,
    masks: HashMap<String, Option<MenuIcon>>,
    images: HashMap<String, Option<Rc<IconImage>>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        font: None,
        bar: None,
        masks: HashMap::new(),
        images: HashMap::new(),
    });
}

/// 글꼴을 준다(기동 때 1회 · UI 글꼴 사슬) — 주기 전에는 아이콘이 없다(창 없는 시험은 부르지 않는다 → 덤프가 흔들리지 않는다).
pub(crate) fn init(font: Rc<Font>) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.font = Some(font);
        s.masks.clear();
        s.images.clear();
    });
}

/// 메뉴 바(풀다운) 그림 켬/끔 + 칠할 색(테마 글자색) — 바뀌었으면 `true`(호스트가 메뉴를 다시 만든다).
pub(crate) fn set_bar(color: Option<Color>) -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.bar == color {
            return false;
        }
        s.bar = color;
        s.images.clear();
        true
    })
}

/// 명령 id의 아이콘 마스크(우클릭 · 보조 메뉴용 — 색은 메뉴가 칠한다). 글꼴을 아직 안 줬거나 대응이 없으면 `None`.
pub(crate) fn mask(id: &str) -> Option<MenuIcon> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let font = s.font.clone()?;
        if let Some(hit) = s.masks.get(id) {
            return hit.clone();
        }
        let icon = spec_of(id).and_then(|(ch, fallback)| {
            glyph_mask(&font, ch)
                .map(|m| MenuIcon::from_alpha(SIDE as u32, SIDE as u32, &m))
                .or_else(|| fallback.map(glyph))
        });
        s.masks.insert(id.to_string(), icon.clone());
        icon
    })
}

/// 메뉴 바 항목의 그림(테마 글자색으로 칠한 RGBA) — 꺼져 있거나 대응이 없으면 `None`.
pub(crate) fn bar_image(id: &str) -> Option<Rc<IconImage>> {
    let color = STATE.with(|s| s.borrow().bar)?;
    if let Some(hit) = STATE.with(|s| s.borrow().images.get(id).cloned()) {
        return hit;
    }
    let img = mask(id).map(|m| {
        let (r, g, b) = color.rgb();
        let mut rgba = Vec::with_capacity(m.alpha.len() * 4);
        for &a in m.alpha.iter() {
            rgba.extend_from_slice(&[r, g, b, a]);
        }
        Rc::new(IconImage::from_rgba(m.w, m.h, rgba))
    });
    STATE.with(|s| s.borrow_mut().images.insert(id.to_string(), img.clone()));
    img
}

/// 메뉴 항목들에 아이콘을 붙인다(이미 아이콘이 있는 항목 — 셸 확장 등 — 과 체크 표시 항목은 그대로 · 하위 메뉴까지).
pub(crate) fn decorate(items: Vec<CtxItem>) -> Vec<CtxItem> {
    items
        .into_iter()
        .map(|it| match it {
            CtxItem::Item {
                id,
                label,
                enabled,
                icon,
                shortcut,
                sub,
                emph,
                marks,
                children,
                checked,
                active,
                mark,
            } => {
                let icon = icon.or_else(|| {
                    (checked.is_none() && mark.is_none())
                        .then(|| mask(&id))
                        .flatten()
                });
                CtxItem::Item {
                    id,
                    label,
                    enabled,
                    icon,
                    shortcut,
                    sub,
                    emph,
                    marks,
                    children: decorate(children),
                    checked,
                    active,
                    mark,
                }
            }
            CtxItem::Separator => CtxItem::Separator,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 대응 표: 편집 4종 + 자주 쓰는 명령이 있고 · 동적 id는 접두로 · 모르는 id = 없음. 글꼴을 주기 전에는 아이콘이 없다
    /// (시험 덤프 안정) · 준 뒤에는 글꼴에 글리프가 없어도 내장 도형이 있는 항목은 아이콘이 나온다.
    #[test]
    fn spec_table_and_fallback() {
        for id in [
            "edit.copy",
            "edit.cut",
            "edit.paste",
            "edit.select_all",
            "edit.delete",
            "edit.undo",
            "file.new_folder",
            "view.refresh",
            "ctx.copy_path",
            "aux.prefs",
        ] {
            assert!(spec_of(id).is_some(), "{id}");
        }
        assert!(spec_of("aux.launch.edit:3").is_some());
        assert!(spec_of("new.tpl:0").is_some());
        assert!(
            spec_of("view.mode_tree").is_none(),
            "체크/라디오 항목은 표에 없다"
        );
        assert!(spec_of("fake.open").is_none());
        // 글꼴을 주기 전 = 없음.
        assert!(mask("edit.copy").is_none());
        assert!(bar_image("edit.copy").is_none());
        let font = Rc::new(nexa_font::ui_font(None).expect("ui font").font);
        init(Rc::clone(&font));
        // 복사 = 아이콘 글꼴이 있든 없든 나온다(내장 도형 폴백).
        let copy = mask("edit.copy").expect("copy icon");
        assert_eq!((copy.w, copy.h), (SIDE as u32, SIDE as u32));
        assert!(copy.alpha.iter().any(|&a| a > 0), "빈 마스크가 아니다");
        assert!(mask("fake.open").is_none());
        // 글리프 래스터: 흔한 글자는 마스크 안에 들어오고 가운데에 놓인다.
        let m = glyph_mask(&font, 'A').expect("A");
        let ink: Vec<usize> = (0..SIDE * SIDE).filter(|&i| m[i] > 8).collect();
        let (minx, maxx) = (
            ink.iter().map(|i| i % SIDE).min().unwrap(),
            ink.iter().map(|i| i % SIDE).max().unwrap(),
        );
        assert!(
            (minx as i32 - (SIDE - 1 - maxx) as i32).abs() <= 1,
            "가로 가운데"
        );
        // 메뉴 바 그림: 색을 줘야 나오고 · 그 색으로 칠해진다.
        assert!(set_bar(Some(Color(0x0011_2233))));
        assert!(!set_bar(Some(Color(0x0011_2233))), "같은 값 = 변화 없음");
        let img = bar_image("edit.copy").expect("bar image");
        assert_eq!(&img.rgba[..3], &[0x11, 0x22, 0x33]);
        assert!(set_bar(None));
        assert!(bar_image("edit.copy").is_none());
        // 붙이기: 아이콘 없는 항목만 · 체크 항목은 그대로 · 하위 메뉴까지.
        let items = decorate(vec![
            CtxItem::item("edit.copy", "Copy"),
            CtxItem::Separator,
            CtxItem::item("fake.open", "Other"),
            CtxItem::submenu(
                "ctx.new",
                "New",
                vec![CtxItem::item("new.folder", "Folder")],
            ),
        ]);
        let has = |it: &CtxItem| matches!(it, CtxItem::Item { icon: Some(_), .. });
        assert!(has(&items[0]) && !has(&items[2]) && has(&items[3]));
        if let CtxItem::Item { children, .. } = &items[3] {
            assert!(has(&children[0]), "하위 메뉴 항목도");
        }
    }
}
