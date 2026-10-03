//! 순서/표시 모델(dir2 `config.rs:1034-1185` 이식 · T-71 DLG-069~073 · 원장 T-13) — 도구 모음 · 파일 컬럼 · 컨텍스트 메뉴 세 설정이
//! 같은 문법 **`블록:vis[자식:vis,…]|블록:vis|…`**(단일 블록 = 대괄호 생략 · `:0/1` 생략 = 표시)을 쓴다.
//! 파싱은 미지 블록/자식·중복을 버리고 **누락분은 정의 순으로 보충**(전방 호환 — 새 버튼이 저장된 구 순서에도 정의상 앞 형제 뒤에 들어간다).

/// 순서 정의 — `(블록 key, 자식 key 목록)` · 빈 자식 = 단일 블록.
pub(crate) type OrderDefs = &'static [(&'static str, &'static [&'static str])];

/// 파싱된 블록 — `(블록 key, 블록 표시, 자식[(key, 표시)])`. 블록 숨김 = 통째 비표시(자식 상태는 보존).
pub(crate) type OrderBlock = (String, bool, Vec<(String, bool)>);

/// 도구 모음 블록(dir2 SSOT · 기본 순서 = 사용자 확정 07-19 "현재 순서를 기본값으로").
pub(crate) const TOOLBAR_BLOCKS: OrderDefs = &[
    ("refresh", &["refresh", "ontop"]),
    ("panel", &["toggle", "dock", "info", "colsync"]),
    ("view", &["tree", "flat", "tiles"]),
    ("show", &["hidden", "dot", "foldersfirst", "casesensitive"]),
    ("settings", &[]),
];

/// 점 파일 토글이 없는 OS(Linux · macOS)의 도구 모음 블록 — `show`에서 `dot`만 뺀 것([`TOOLBAR_BLOCKS`]와 나머지는 같아야 한다).
pub(crate) const TOOLBAR_BLOCKS_NO_DOT: OrderDefs = &[
    ("refresh", &["refresh", "ontop"]),
    ("panel", &["toggle", "dock", "info", "colsync"]),
    ("view", &["tree", "flat", "tiles"]),
    ("show", &["hidden", "foldersfirst", "casesensitive"]),
    ("settings", &[]),
];

/// 도구 모음 블록 정의(순수): 점 파일 토글이 있는 OS면 전체 · 없으면 `dot`을 뺀 것.
pub(crate) fn toolbar_blocks_for(dotfile_toggle: bool) -> OrderDefs {
    if dotfile_toggle {
        TOOLBAR_BLOCKS
    } else {
        TOOLBAR_BLOCKS_NO_DOT
    }
}

/// 이 OS의 도구 모음 블록 정의.
pub(crate) fn toolbar_blocks() -> OrderDefs {
    toolbar_blocks_for(crate::platform::has_dotfile_toggle())
}

/// 파일 목록 컬럼(key 순서 = 기본 표시 순서 · `name` = 상시 표시).
pub(crate) const COLUMN_BLOCKS: OrderDefs =
    &[("cols", &["name", "ext", "size", "modified", "kind"])];

/// 앱 고유 컨텍스트 메뉴 항목(셸 제공 동사는 대상 아님 · `new` = 하단 고정 섹션 표시 여부만).
pub(crate) const CTXMENU_BLOCKS: OrderDefs = &[
    ("row", &["new", "deletePermanent", "copyName", "pasteInto"]),
    ("bg", &["paste", "undo", "redo"]),
];

/// 기본 순서 문자열(정의 순 · 전부 표시 · vis 포함).
pub(crate) fn default_order(defs: OrderDefs) -> String {
    serialize_order_with(
        &defs
            .iter()
            .map(|(b, items)| {
                (
                    b.to_string(),
                    true,
                    items.iter().map(|i| (i.to_string(), true)).collect(),
                )
            })
            .collect::<Vec<_>>(),
        true,
    )
}

/// 직렬화 — `with_vis` = 표시 여부 포함.
pub(crate) fn serialize_order_with(order: &[OrderBlock], with_vis: bool) -> String {
    order
        .iter()
        .map(|(b, bv, items)| {
            let head = if with_vis {
                format!("{b}:{}", u8::from(*bv))
            } else {
                b.clone()
            };
            if items.is_empty() {
                head
            } else {
                let inner: Vec<String> = items
                    .iter()
                    .map(|(k, v)| {
                        if with_vis {
                            format!("{k}:{}", u8::from(*v))
                        } else {
                            k.clone()
                        }
                    })
                    .collect();
                format!("{head}[{}]", inner.join(","))
            }
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// 파싱 + 검증(누락 보충 · 미지/중복 제거). 빈 문자열 = 기본.
pub(crate) fn parse_order_with(defs: OrderDefs, s: &str) -> Vec<OrderBlock> {
    let mut out: Vec<OrderBlock> = Vec::new();
    for tok in s.split('|') {
        let tok = tok.trim();
        let (head, inner) = match tok.split_once('[') {
            Some((n, rest)) => (n.trim(), Some(rest.trim_end_matches(']'))),
            None => (tok, None),
        };
        let (name, bvis) = match head.split_once(':') {
            Some((n, v)) => (n.trim(), v.trim() != "0"),
            None => (head, true),
        };
        let Some((_, def_items)) = defs.iter().find(|(b, _)| *b == name) else {
            continue;
        };
        if out.iter().any(|(b, _, _)| b == name) {
            continue;
        }
        let mut items: Vec<(String, bool)> = Vec::new();
        if let Some(inner) = inner {
            for it in inner.split(',') {
                let it = it.trim();
                let (k, vis) = match it.split_once(':') {
                    Some((k, v)) => (k.trim(), v.trim() != "0"),
                    None => (it, true),
                };
                if def_items.contains(&k) && !items.iter().any(|(x, _)| x == k) {
                    items.push((k.to_string(), vis));
                }
            }
        }
        for (di, d) in def_items.iter().enumerate() {
            if !items.iter().any(|(x, _)| x == d) {
                let pos = def_items[..di]
                    .iter()
                    .rev()
                    .find_map(|prev| items.iter().position(|(x, _)| x == prev).map(|p| p + 1))
                    .unwrap_or(0);
                items.insert(pos, (d.to_string(), true));
            }
        }
        out.push((name.to_string(), bvis, items));
    }
    for (b, def_items) in defs {
        if !out.iter().any(|(n, _, _)| n == b) {
            out.push((
                b.to_string(),
                true,
                def_items.iter().map(|i| (i.to_string(), true)).collect(),
            ));
        }
    }
    out
}

/// 정규화(파싱 → 직렬화) — 설정에 쓰는 값은 항상 이 모양.
pub(crate) fn normalize(defs: OrderDefs, s: &str) -> String {
    serialize_order_with(&parse_order_with(defs, s), true)
}

/// 연속 선택 집합 한 칸 이동(DLG-070 `shift_range` — 경계면 무동작 · false).
pub(crate) fn shift_range<T>(v: &mut [T], sel: &[usize], up: bool) -> bool {
    let (Some(&lo), Some(&hi)) = (sel.first(), sel.last()) else {
        return false;
    };
    if hi >= v.len() {
        return false;
    }
    if up {
        if lo == 0 {
            return false;
        }
        v[lo - 1..=hi].rotate_left(1);
    } else {
        if hi + 1 >= v.len() {
            return false;
        }
        v[lo..=hi + 1].rotate_right(1);
    }
    true
}

/// 컬럼 key ↔ 그리드 열 id(`filelist::COL_*`).
pub(crate) fn col_key_id(key: &str) -> Option<u32> {
    Some(match key {
        "name" => crate::filelist::COL_NAME,
        "ext" => crate::filelist::COL_EXT,
        "size" => crate::filelist::COL_SIZE,
        "modified" => crate::filelist::COL_MODIFIED,
        "kind" => crate::filelist::COL_KIND,
        _ => return None,
    })
}

pub(crate) fn col_id_key(id: u32) -> &'static str {
    match id {
        crate::filelist::COL_EXT => "ext",
        crate::filelist::COL_SIZE => "size",
        crate::filelist::COL_MODIFIED => "modified",
        crate::filelist::COL_KIND => "kind",
        _ => "name",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 점 파일 토글이 없는 OS의 도구 모음 정의 = 전체에서 `show/dot`만 뺀 것(나머지 블록·순서 동일).
    #[test]
    fn toolbar_blocks_without_dot_match_full_set() {
        assert_eq!(toolbar_blocks_for(true), TOOLBAR_BLOCKS);
        let no_dot = toolbar_blocks_for(false);
        assert_eq!(no_dot.len(), TOOLBAR_BLOCKS.len());
        for ((b1, c1), (b2, c2)) in TOOLBAR_BLOCKS.iter().zip(no_dot) {
            assert_eq!(b1, b2);
            let want: Vec<&str> = c1.iter().copied().filter(|c| *c != "dot").collect();
            assert_eq!(&want[..], *c2, "{b1}");
        }
        // 저장된 레이아웃의 dot 토큰은 그 OS에서 버려진다.
        let n = normalize(no_dot, "show:1[dot:1,hidden:0]");
        assert!(!n.contains("dot") && n.contains("hidden:0"), "{n}");
    }

    #[test]
    fn roundtrip_and_merge() {
        let d = default_order(TOOLBAR_BLOCKS);
        assert_eq!(normalize(TOOLBAR_BLOCKS, &d), d, "기본 왕복");
        assert_eq!(normalize(TOOLBAR_BLOCKS, ""), d, "빈 값 = 기본");
        let s = "view:0[tiles:1,tree:0,flat:1]|refresh:1[ontop:1,refresh:0]|panel:1[colsync:1,toggle:1,dock:1,info:1]|show:1[dot:1,hidden:1,casesensitive:0,foldersfirst:1]|settings:1";
        assert_eq!(normalize(TOOLBAR_BLOCKS, s), s, "재배열/표시 보존");
        // 구형(vis 없음 · 누락 자식 · 미지 토큰 · 중복) → 보충 · 정의상 앞 형제 뒤에 삽입.
        let old = "panel[dock,toggle]|bogus|panel|view[flat]";
        let p = parse_order_with(TOOLBAR_BLOCKS, old);
        assert_eq!(p[0].0, "panel");
        let keys: Vec<&str> = p[0].2.iter().map(|(k, _)| k.as_str()).collect();
        // 보충 규칙 = 정의상 **가장 가까운 앞 형제**(있는 것) 뒤: info는 dock(정의 idx 1) 뒤 → toggle이 말미로 밀린다(dir2 동일).
        assert_eq!(keys, ["dock", "info", "colsync", "toggle"]);
        // "항상 위"는 새로 고침 그룹으로 옮겼다(사용자 10-03): 옛 저장값의 panel/ontop은 버려지고 refresh 그룹에 자식이 보충된다.
        let moved = normalize(TOOLBAR_BLOCKS, "refresh:1|panel:1[toggle:1,ontop:0]");
        assert!(
            moved.starts_with(
                "refresh:1[refresh:1,ontop:1]|panel:1[toggle:1,dock:1,info:1,colsync:1]|"
            ),
            "{moved}"
        );
        assert_eq!(TOOLBAR_BLOCKS[0], ("refresh", &["refresh", "ontop"][..]));
        assert_eq!(p[1].0, "view");
        assert_eq!(p.len(), TOOLBAR_BLOCKS.len());
        assert!(p
            .iter()
            .all(|(_, v, items)| *v && items.iter().all(|(_, v)| *v)));
    }

    #[test]
    fn shift_range_boundaries() {
        let mut v = vec![1, 2, 3, 4];
        assert!(!shift_range(&mut v, &[0], true));
        assert!(!shift_range(&mut v, &[3], false));
        assert!(!shift_range(&mut v, &[], true));
        assert!(shift_range(&mut v, &[1, 2], true));
        assert_eq!(v, [2, 3, 1, 4]);
        assert!(shift_range(&mut v, &[2], false));
        assert_eq!(v, [2, 3, 4, 1]);
    }

    #[test]
    fn column_keys_map_both_ways() {
        for (_, items) in COLUMN_BLOCKS {
            for k in *items {
                assert_eq!(col_id_key(col_key_id(k).expect("id")), *k);
            }
        }
        assert_eq!(col_key_id("total"), None);
    }
}
