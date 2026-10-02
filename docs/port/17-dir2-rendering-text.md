# 17 · nexa-dir2 인벤토리 — 그리기·텍스트·테마·아이콘·DPI (RENDER)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 기준일 2026-10-03.
> 대상: nexa-dir2(Windows 전용 · DirectWrite GDI interop + GDI+) → nexa-dir3(winit + softbuffer + nexa-ui CPU 래스터).
> 표기: 근거는 `저장소/경로:줄`. 확인하지 못한 것은 **추정**으로 표시. ID = `RENDER-NNN`(이후 구현·교차 검증 체크리스트).
> 이식 분류: **N** = 플랫폼 중립(거의 그대로) / **A** = nexa-ui 컨트롤·그리기로 교체 / **P** = OS별 구현 분기 / **W** = Windows 전용 유지(타 OS 대체·비활성).

---

## 0. 범위 — 읽은 파일과 줄 수

### 0-1. nexa-dir2 (원본) — 전부 끝까지 읽음

| 파일 | 줄 | 내용 |
| --- | ---: | --- |
| `nexa-dir2/crates/nexa-app/src/dw.rs` | 991 | DirectWrite GDI interop 백엔드(`DwBackend`·`DwCtx`) |
| `nexa-dir2/crates/nexa-app/src/fontchain.rs` | 270 | 글꼴 폴백 체인(쉼표 목록)·GDI 글리프 단위 폴백 `GdiChain` |
| `nexa-dir2/crates/nexa-app/src/ctl/gdipctx.rs` | 647 | GDI+ 백엔드(AA 도형·PNG 디코드·SVG 래스터) |
| `nexa-dir2/crates/nexa-app/src/ctl/style.rs` | 119 | ctl 공통 팔레트 `Style`·GDI 유틸 |
| `nexa-dir2/crates/nexa-gui/src/draw.rs` | 127 | `DrawCtx` 트레이트·`FontSlot` |
| `nexa-dir2/crates/nexa-gui/src/theme.rs` | 126 | `Color`·`Theme`(다크/라이트) |
| `nexa-dir2/crates/nexa-app/src/icons.rs` | 597 | 아이콘 키·`IconStore`(LRU+큐)·셸 로더·임베드 SVG 레지스트리 |
| `nexa-dir2/crates/nexa-app/src/icon.rs` | 47 | 앱 아이콘(ICO → HICON) |
| `nexa-dir2/crates/nexa-app/src/svg.rs` | 692 | SVG 서브셋 파서 |
| `nexa-dir2/crates/nexa-app/build.rs` | 162 | exe 리소스(아이콘·VERSIONINFO·매니페스트). 과업 지시의 `src/build.rs`는 없음 — 실제 위치는 크레이트 루트 |

보조로 읽은 호출부(구간): `nexa-app/src/win.rs`(296-318, 1336-1366, 1838-1877, 1984-2025, 2536-2735, 4819-4954, 9631-9661), `nexa-gui/src/widgets/chrome.rs`(240-362), `widgets/dock.rs`(360-412), `widgets/rows.rs`(360-384, 1355-1401, 1570-1594), `nexa-app/src/ctl/iconbutton.rs`(1-290), `preview/mod.rs`(60-140), `previewwnd.rs`(150-199), `ordereditor.rs`(300-325), `panel.rs`(495-518), `config.rs`(Grep), `ctl/fontbox.rs`(16-105), 자산 README 3종, `docs/07-adr-0002-rendering.md`(1-55), `docs/23-cross-platform-feasibility.md`(98-150).

### 0-2. nexa-ui (대응)

| 파일 | 줄 | 읽은 범위 |
| --- | ---: | --- |
| `nexa-ui/crates/nexa-gfx/src/lib.rs` | 19 | 전부 |
| `nexa-ui/crates/nexa-gfx/src/surface.rs` | 590 | 전부 |
| `nexa-ui/crates/nexa-gfx/src/text.rs` | 1009 | 전부 |
| `nexa-ui/crates/nexa-gfx/src/gdi.rs` | 446 | 전부 |
| `nexa-ui/crates/nexa-gfx/src/coretext.rs` | 485 | 전부(1-60, 56-270, 265-485 구간 합) |
| `nexa-ui/crates/nexa-gfx/src/names.rs` | 95 | 전부 |
| `nexa-ui/crates/nexa-gfx/src/image.rs` | 742 | 1-130 정독 + 나머지는 함수·오류 문구 Grep(디코더 본문은 미정독) |
| `nexa-ui/crates/nexa-gfx/src/inflate.rs` | 350 | 머리말·공개 함수·오류 문구 Grep(본문 미정독) |
| `nexa-ui/crates/nexa-gfx/src/jpeg.rs` | 588 | 머리말·공개 함수·오류 문구 Grep(본문 미정독) |
| `nexa-ui/crates/nexa-ctl/src/draw.rs` | 416 | 전부 |
| `nexa-ui/crates/nexa-ctl/src/raster.rs` | 804 | 전부 |
| `nexa-ui/crates/nexa-ctl/src/theme.rs` | 401 | 전부 |
| `nexa-ui/crates/nexa-ctl/src/tokens.rs` | 1200 | 1-880(본문 전부) · 872 이후 테스트 블록은 미정독 |
| `nexa-ui/crates/nexa-ctl/src/shape.rs` | 174 | 전부 |
| `nexa-ui/crates/nexa-font/src/lib.rs` | 764 | 전부 |

보조: `nexa-ctl/src/lib.rs`(전부), `controls/glyphs.rs`(전부), `controls/ctxmenu.rs`(55-97 `MenuIcon`), `controls/toolbar.rs`(24-36 `ToolIcon`), `nexa-fs/src/shell.rs`(1-330, 730-760), nexa-sql `toolicons.rs`(1-60, 760-910, 1100-1218)·`dbms_icons.rs`(1-50)·`icon.rs`(1-80)·`nsql-settings`(키 Grep).

---

## 1. 기능 목록

### 1-1. 백버퍼·DPI·표면

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| RENDER-001 | 깜빡임 없는 화면 갱신(더블 버퍼) | 창 1개당 `IDWriteBitmapRenderTarget`(자체 메모리 DC)에 전부 그린 뒤 `BitBlt` 1회. GPU 스왑체인 금지. 페인트 순서 = 패널0 → 패널1 → (싱글 정보) 좌 도크 재도장 → 터미널 → 스플리터 → 도크 분리선 → 툴바 → 런처 → 상태바 → **메뉴바 마지막**(드롭다운이 위) | `nexa-dir2/crates/nexa-app/src/dw.rs:238-252,498-501` · `win.rs:4820-4933` | `DWriteCreateFactory`·`CreateBitmapRenderTarget`·`BitBlt` | A (softbuffer `Surface` + present) | 없음 |
| RENDER-002 | 창 크기 변경 시 백버퍼 재할당 | `ensure_dw`: 없으면 생성, 크기 다르면 `Resize(max(1))`. 생성 실패는 `eprintln!` 후 그리기 생략 | `dw.rs:484-489` · `win.rs:1984-2009` | `IDWriteBitmapRenderTarget::Resize` | A | 없음 |
| RENDER-003 | 모니터 DPI 변경 즉시 반영 | `WM_DPICHANGED`: `st.dpi` 갱신 → `set_dpi`(레이아웃 캐시·모노 글리프 캐시 비움 + `SetPixelsPerDip(dpi/96)`) → 패널/메뉴/툴바/런처/상태바 `set_metrics` → 권장 rect로 `SetWindowPos` → 재레이아웃 | `dw.rs:491-496` · `win.rs:9631-9661` | `WM_DPICHANGED`·`GetDpiForWindow`(`win.rs:7695`) | P (winit `ScaleFactorChanged`) | 없음 |
| RENDER-004 | DPI 인식 선언(PerMonitorV2) | 매니페스트(`dpiAwareness=PerMonitorV2`, `dpiAware=true/pm`) + 코드 `SetProcessDpiAwarenessContext` 이중 | `build.rs:84-108` · `win.rs:1725` | 매니페스트 RT_MANIFEST·`SetProcessDpiAwarenessContext` | W (Windows는 winit이 처리 — 매니페스트는 유지 권장) | 없음 |
| RENDER-005 | 지표의 DPI 스케일 | 정수식 `v * dpi / 96`. 행 높이 `s(20).max(14)`·`pad_x s(6)`·`indent_w s(16)`·`tab_h s(22)`·`bar_h s(24)`. 컬럼 폭 340/64/96/140/110 | `win.rs:1344-1366` | 없음 | N (배율 = winit scale_factor) | 없음 |
| RENDER-006 | 유휴 시 메모리 반납(상주 트림) | 마지막 활동 후 60,000ms(`IDLE_TRIM_MS`) → DW 백엔드 통째 해제(백버퍼·레이아웃/이미지 캐시) + 아이콘 전 핸들 해제 + 작업집합 반납. 화면 무효화 안 함 — 다음 페인트에서 지연 재적재 | `win.rs:93,2011-2025` · `icons.rs:415-425` | `SetProcessWorkingSetSize` | P (캐시 비움 = 중립 · 작업집합 반납 = W) | `should_trim`은 순수 함수(테스트는 범위 밖 — 추정) |

### 1-2. 텍스트(DirectWrite)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| RENDER-007 | 영역별 글꼴(폰트 슬롯) | 슬롯 3종: `Base`(메뉴·탭·경로바·툴/런처 바·도크) / `List`(파일 목록+컬럼 헤더) / `Status`(상태바). 스타일 id = `슬롯×4 + bold + 2×italic`. 등록 포맷: 0(base) · 4~7(list 보통/굵게/이탤릭/굵은이탤릭) · 8(status). **미등록 id는 0으로 폴백**(Base·Status의 bold/italic 요청은 보통체로 그려짐) | `nexa-gui/src/draw.rs:8-27` · `dw.rs:178-186,254-303,418-424,594-597` | `IDWriteTextFormat` | A | 없음 |
| RENDER-008 | 글꼴 크기·굵기 규칙 | 크기 단위 = **DIP(em)**, `clamp(8.0, 32.0)`, 기본 12 DIP(=9pt). 굵게 = `DWRITE_FONT_WEIGHT_SEMI_BOLD`(600 — 볼드 700 아님), 이탤릭 = 실제 italic face. 로캘 `ko-kr`. 줄바꿈 없음. **세로 중앙 정렬**(레이아웃 maxheight = 행 높이) | `dw.rs:256-288` | DirectWrite | A | 없음 |
| RENDER-009 | 글꼴 "쉼표 목록 = 폴백 체인" | 설정 문자열 `"A, B, C"` → 1순위 = **설치된 첫 패밀리**(없으면 기본 `Segoe UI`/터미널 `Consolas`), 2순위 이후 = 설치된 나머지를 전 유니코드 범위(0~10FFFF)에 매핑 후 시스템 폴백 연결. 체인이 1개면 시스템 폴백만. 실패는 조용히 무동작 | `dw.rs:140-176,289-314,356-384,470-482` · `fontchain.rs:28-57` | `IDWriteFontFallbackBuilder`·`IDWriteTextLayout2::SetFontFallback` | A (nexa-font 체인) | `fontchain.rs:259`(`families_trims_and_drops_empty`) |
| RENDER-010 | 행 배경+텍스트 한 번에(text_opaque) | `clip`을 `bg`로 채운 뒤 글리프. `text` 비었거나 `x >= clip.right()`면 배경만. **`y` 인자는 무시**(`_y`) — 레이아웃이 `clip.y`에서 `clip.h` 안 세로 중앙. 폭 초과분 = **문자 단위 말줄임표(…) 트리밍**(max_w = `clip.right() - x`). 왼쪽·세로 방향은 자르지 않음 | `dw.rs:642-662,426-464` | `CreateTextLayout`·`SetTrimming`·`CreateEllipsisTrimmingSign` | A | 없음 |
| RENDER-011 | 배경 없는 텍스트(text) | 선택 하이라이트 위에 1회 겹쳐 그리기(런 분할 금지 — 경계 이음새 방지). 나머지 규칙은 RENDER-010과 동일 | `dw.rs:781-800` · `nexa-gui/src/draw.rs:50-55` | 동상 | A | 없음 |
| RENDER-012 | 텍스트 폭 측정(text_width) | `ceil(widthIncludingTrailingWhitespace × ppd)`. 빈 문자열 0. 첫 글자가 U+E700~U+E8FF면 MDL2 11 DIP 포맷으로 측정(실패 0). 측정용 가상 폭 `MEASURE_W = 1<<20`. 현재 선택 스타일 적용 | `dw.rs:802-838,36-37` | `IDWriteTextLayout::GetMetrics` | A | 없음 |
| RENDER-013 | 텍스트 레이아웃 캐시 | 외측 키 = `max_w_px + (style << 21)`(아이콘은 음수 네임스페이스 `-clip.w`, 대형은 `-clip.w - 1_000_000`), 내측 = 텍스트(`&str` 무할당 조회). 상한 4096 → **전체 비움**. DPI 변경 시 비움 | `dw.rs:32-37,219-228,426-464` | DirectWrite 레이아웃 객체 | A (nexa-gfx 글리프 비트맵 캐시로 대체) | 없음 |
| RENDER-014 | 큰 글리프 중앙 그리기(glyph_opaque) | `clip`을 `bg`로 채우고 가운데 정렬. 기본 포맷 = Segoe UI **15 DIP**. 첫 글자 U+E700~U+F8FF → Segoe MDL2 Assets **11 DIP**, 그중 셰브론 E70D/E70E/E76B/E76C → **9 DIP**. `clip.w <= 0`이면 배경만 | `dw.rs:316-350,840-919` · `nexa-gui/src/draw.rs:83-89` | Segoe MDL2 Assets(인박스 폰트) | A (코드 도형/SVG 마스크로 교체) | 없음 |
| RENDER-015 | 대형 글리프(glyph_opaque_lg) | 패널 네비 바 [홈][←][→][↑] 전용 — MDL2 **13 DIP**(15는 과하고 11은 안 보인다는 실기 판단). 플래그 세우고 같은 경로 | `dw.rs:197-211,349-350,921-926` · 호출 `chrome.rs:347-351` | 동상 | A | 없음 |
| RENDER-016 | 사용 중인 MDL2 글리프 목록 | 디스클로저 `E76C`(접힘 ›)·`E70D`(펼침 ∨): `rows.rs:49-50`, `prefs.rs:2120-2122`, `ctl/ordertree.rs:848` / 네비 `EA8A`(홈)·`E72B`(뒤로)·`E72A`(앞으로)·`E74A`(위로): `panel.rs:1557-1560` / 세그먼트 라벨 `→ `·`← ` 접두 치환 `E72A`/`E72B`: `ctl/segmented.rs:79-80` / 설정 `E713`: `win.rs:740` | 좌동 | Segoe MDL2 Assets | A (GlyphKind/SVG) | 없음 |
| RENDER-017 | 텍스트 세로 위치 규약 | 위젯은 `ty = y + (h - h*4/5)/2`를 계산해 넘기지만 **DW 백엔드는 y를 쓰지 않는다**(RENDER-010). 실제 위치는 "clip 세로 중앙". 기본 `glyph_opaque` 폴백 구현만 이 ty를 사용 | `rows.rs:1370-1371` · `chrome.rs:246` · `dw.rs:642` · `nexa-gui/src/draw.rs:85-89` | 없음 | A (※ nexa-ctl은 y를 실제로 씀 — §3-1) | 없음 |
| RENDER-018 | 터미널 고정폭 글꼴 | 설정 `term_font`(쉼표 체인) 1순위 = 설치된 첫 패밀리(기본 Consolas), 크기 `term_font_size`(8~32, 기본 12 DIP), 생성 실패 시 Consolas 재시도. 랩 없음·세로 중앙 | `dw.rs:212-216,352-384` · `config.rs:80-84,262-263,576-582` | DirectWrite | A (nexa-font `mono_font`) | 없음 |
| RENDER-019 | 터미널 셀 폭(term_cell_w) | 모노 포맷의 `"0"` 폭 `ceil(width × ppd).max(1)`. 레이아웃/메트릭 실패 시 **8** | `dw.rs:664-682` | 동상 | A | 없음 |
| RENDER-020 | 터미널 텍스트(term_text) | `clip`을 `bg`로 채우고 모노 글리프. **단일 문자** = `char → 레이아웃` 캐시(2048 초과 시 전체 비움 · 레이아웃 폭 100 DIP), 여러 글자(안내문) = 즉석 생성. 폴백 체인 적용 | `dw.rs:684-735` | 동상 | A | 없음 |
| RENDER-021 | 터미널 셀 그리드 그리기 규약(소비 측) | 셀 높이 = `(font_px × 4/3 × dpi / 96).max(12)`. 그리드 원점 `rc.x+2`, `rc.y+1`. 동일 (fg,bg) 런 단위로 배경 채움, **문자는 셀 x에 개별 배치**(런 단위 레이아웃은 폴백 글꼴 전진폭이 그리드와 어긋남). `'\0'`·공백 = 배경만, 다음 셀이 `'\0'`이면 전각(2셀 clip). faint = fg·bg 절반 블렌드. 선택 = fg/bg 반전(밝은 팔레트의 기본색 셀만 accent 배경+팔레트 bg 글자). 캐럿 = 세로 바 폭 `(dpi/96).max(1)`, 색 = 팔레트 fg. 종료 시 `term.exited` 문구를 accent로 | `win.rs:2551-2735` | `push_clip` 의존(RENDER-023) | A (렌더 어휘) · 터미널 자체는 별도 문서 | 없음 |

### 1-3. 도형·클립·이미지

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| RENDER-022 | 단색 사각 채움(fill_rect) | GDI `ExtTextOutW(ETO_OPAQUE)` 빈 문자열. **음수 폭/높이 rect를 GDI가 정규화해 엉뚱한 영역을 칠함** → 호출부가 `w > 0`일 때만 호출(RENDER-L03) | `dw.rs:628-640` · `win.rs:4886-4900` | `SetBkColor`·`ExtTextOutW` | A | 없음 |
| RENDER-023 | 클립 영역 push/pop | 교차 클립 스택. `SaveDC`+`IntersectClipRect` / `RestoreDC(-1)`. 반드시 쌍. 사용처 2곳: 도크 가로 스크롤 본문(`dock.rs:854`), 터미널 그리드(`win.rs:2631-2732`). 기본 구현 = no-op | `dw.rs:612-626` · `nexa-gui/src/draw.rs:40-48` | GDI 클립 리전 | A (**nexa-ctl에 없음 — 추가 필요**) | 없음 |
| RENDER-024 | AA 라운드 사각 채움(+알파) | `fill_round_rect` = 알파 255 위임. `fill_round_rect_alpha(rect, radius, color, alpha: u8)` — 0=투명 … 255=불투명, GDI+ 소스오버. 지름 `d = min(radius×2, w, h)`. 사용: 오버레이 스크롤바(트랙 알파 28 · 썸 `BAR_ALPHA 120`/`BAR_ALPHA_HOT 210`), 고속 스크롤 배지, 설정 창 | `dw.rs:599-610` · `ctl/gdipctx.rs:477-502,515-530,589-606` · `widgets/overlaybar.rs:267-275` · `widgets/rows.rs:23-29` | GDI+ `GdipFillPath` | A (알파 단위 u8→f32 변환) | 없음 |
| RENDER-025 | AA 타원 채움 | rect 내접 타원. 사용: 원형 아이콘 버튼 원판, 그리드 | `ctl/gdipctx.rs:569-587` · `ctl/iconbutton.rs:278-281` | GDI+ `GdipFillEllipse` | A | 없음 |
| RENDER-026 | AA 라운드 외곽선 | **펜 중심선을 rect 안쪽으로 `width/2 + 0.5` 인셋**(선이 rect 밖으로 안 나감). 둥근 캡/조인 | `ctl/gdipctx.rs:608-626,504-513` | GDI+ `GdipDrawPath` | A (※ nexa는 경계 중심 — §3-1) | 없음 |
| RENDER-027 | AA 꺾은선(✓·셰브론·+/−) | 2점 미만 무시. 둥근 캡/조인, 폭 px | `ctl/gdipctx.rs:628-646` | GDI+ `GdipDrawLines` | A | 없음 |
| RENDER-028 | 이미지 미리보기 그리기(draw_image) | `draw_image(rect, 파일 경로)`. `rect.w/h <= 2`면 생략. `(rect.w-2, rect.h-2)` 안에 **비율 유지 축소(확대 안 함 `min(1.0)`)**, 가운데 정렬. WIC 첫 프레임 → Fant 스케일 → 32bpp BGRA. 캐시 키 `(경로, 맞춤 폭, 높이)`, **8개 초과 시 전체 비움**. 실패 = 아무것도 안 그림(호출자가 배경 선도장). 알파 블렌드 없음(`SRCCOPY`) | `dw.rs:126-129,229-232,507-583,737-779` · 호출 `widgets/dock.rs:839,875` | WIC(`IWICImagingFactory`)·`StretchDIBits`·STA COM | P→A (nexa_gfx::image 디코더 + 호스트 캐시) | 예제 `nexa-app/examples/preview_image.rs`(내용 미확인 — 추정) |
| RENDER-029 | ctl용 도형 백엔드(GdipCtx) | `BeginPaint` HDC를 감싸는 일회성 `DrawCtx`. 도형만 구현 — `text_opaque` = no-op, `text_width` = 0(ctl 텍스트는 GDI `DrawTextW` 직접). GDI+ 초기화 실패 시 도형 no-op. **코드베이스의 유일한 GDI+ 접점** | `ctl/gdipctx.rs:1-12,418-455,533-567` | `GdiplusStartup`(프로세스 1회 `OnceLock`) | A (RasterCtx 하나로 통합) | 없음 |
| RENDER-030 | PNG 이미지 버튼 | `decode_png`(SHCreateMemStream → GDI+ 이미지, 알파 보존)·`image_size`·`draw_image`(고품질 바이큐빅 스케일)·`dispose_image`. 표시 모드 `ImageFit::{Native(원본 크기 중앙), Stretch(기본)}` | `ctl/gdipctx.rs:50-80,408-416,456-467` · `ctl/iconbutton.rs:42-51,150-176,252-268` | GDI+·shlwapi | A (`nexa_gfx::image::decode` + `image_scaled`) | 없음 |

### 1-4. 아이콘·SVG·자원

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| RENDER-031 | 아이콘 그리기(draw_icon) | `draw_icon(x, y, size, key, hint) -> bool`(그렸는가 — false면 호출자가 폴백). 미로드 키는 큐잉 후 false. 캐시 히트는 `DrawIconEx(size×size)` | `dw.rs:928-990` · `nexa-gui/src/draw.rs:75-81` | `DrawIconEx`·HICON | A (**nexa-ctl에 없음** — `image_scaled` + 호스트 캐시) | 없음 |
| RENDER-032 | 임베드 아이콘 키 규약(`emb:`) | 위젯이 만드는 키 = `emb:<이름>[#dark][#dis]#RRGGBB`(잉크 = 테마 본문색). 백엔드가 해석: 크기 버킷 `size>=28→32`, `>=22→24`, `>=18→20`, 그 외 16 / `#RRGGBB` 6자리 = 잉크(없으면 검정) / `#dis` = 알파 0x61(38%) / `#dark` → `@dark`. 최종 캐시 키 = `emb:<이름>[@dark]:<버킷>:<ARGB8 hex>`(테마·활성별 캐시 분리) | `dw.rs:928-971` · 생성 `widgets/chrome.rs:302-317`, `widgets/dock.rs:386-398` | 없음(문자열 규약) | N (규약 유지 권장) | 없음 |
| RENDER-033 | 임베드 SVG 아이콘 레지스트리 | `EMBEDDED_SVG` **25항목**(기본 14 + `-dark` 변형 11): panel-toggle(+dark) · info-toggle(+dark) · dock(+dark) · popout · view-tree(+dark) · view-flat(+dark) · view-tiles(+dark) · colsync(+dark) · refresh(+dark) · settings(+dark) · hidden(+dark) · dotfiles(+dark) · folders-first · always-on-top. `@dark`면 `<이름>-dark` 에셋이 있을 때 **원색 그대로** 렌더, 없으면 기본 에셋을 잉크 재색. `emb:` 키는 **동기 생성**(1회 → 캐시, LRU 축출 후 재생성). 미지 키/파싱 실패 = None(글리프 폴백). 구형 키 `emb:<이름>:<버킷>`(잉크 없음 = 검정 `SVG_INK`) 호환 | `icons.rs:171-258,279-336` | `svg_to_hicon`(GDI+) | A | 없음 |
| RENDER-034 | SVG 서브셋 파서 | 루트 `<svg>`: `viewBox` 필수(없거나 w/h ≤ 0 → None) · `stroke-width`(기본 1) · `fill`(none/부재 = 스트로크 모드, 색 = 채움 모드) · 루트 `stroke`/`fill` `#RRGGBB` 상속. 요소: `rect`(x/y/width/height/rx) · `circle` · `line` · `polyline`(홀수 좌표 버림, 2점 미만 무시) · `path`(M/m L/l H/h V/v C/c A/a[**원형 한정** rx=ry·회전 무시] Z/z, 암묵 반복) · `text`(x/y=베이스라인/font-size 기본 10/font-weight bold 또는 ≥600/text-anchor=middle). 요소별 `stroke`·`fill` 색·`fill` 모드·`stroke-width` 오버라이드. 미지 요소 건너뜀, **미지 path 명령(Q/S/T) 또는 타원 호 = 문서 전체 None**. `currentColor` = 잉크. 속성 추출은 키 경계 검사(`x`가 `rx`에 매칭 안 됨) | `svg.rs:19-102(자료형),105-240(parse),243-312(속성),315-487(path)` | 없음(순수 Rust) | N (그대로 이식) | `svg.rs:493-691` 13건 |
| RENDER-035 | SVG 래스터(아이콘) | 정사각 `px` ARGB 비트맵. 균등 스케일 `min(px/vw, px/vh)`. **4배 슈퍼샘플(최대 256) 후 고품질 바이큐빅 축소**. 스트로크: 폭 `max(1.0, width × scale)`, 둥근 캡/조인. 채움: **FillModeAlternate(짝홀)**. 요소 색 오버라이드는 RGB만 — **알파는 잉크 것**(비활성 흐림이 오버라이드 색에도 적용). 텍스트 = Arial 아웃라인 패스(항상 채움, `y - em` 상단 근사, 높이 `em×1.6`). 출력: `svg_to_hicon`(HICON) / `svg_to_image`(GpImage) | `ctl/gdipctx.rs:82-106,152-406` | GDI+ 패스·`GdipCreateHICONFromBitmap` | A (**nexa-ui에 없음 — 스트로크 래스터 추가 필요**) | 없음 |
| RENDER-036 | SVG → 픽셀(미리보기 다이어그램) | `svg_to_pixels`: viewBox 크기(ceil) 그대로 BGRA top-down, **2000×2000 초과 None**, 잉크 = 불투명 검정. 호스트 API `render_svg_impl`: SVG 256KB 초과 None → 알파를 0xFF로 덮어 `temp_dir()/nexa-preview/d<해시16>.bmp`(32bpp top-down) 캐시 후 경로 반환. 비Windows = None | `ctl/gdipctx.rs:108-150` · `preview/mod.rs:97-140` | GDI+ `GdipBitmapLockBits` | A/P (플러그인 문서와 교차) | 없음 |
| RENDER-037 | 툴바 아이콘 상태 표현 | 아이콘 크기 = `(바 높이 - 8).max(8)` — 도구 모음 28px 셀/아이콘 20, 퀵 런처 24px 셀/아이콘 16(버킷과 정확히 일치). 버튼 폭 = 바 높이(정사각). 배경: 켜짐(checked) = `chrome_bg`와 `accent` **38% 블렌드** / hover&&enabled = `sel_bg` / 그 외 `chrome_bg`. 글리프색: enabled = `text`, 비활성 = `text_dim`. 아이콘 미로드 폴백 = 라벨 **앞 2자** 중앙 텍스트. 구분선 = 세로 1px(`y+3`, 높이 `h-6`, 폭 `pad_x.max(4)`) | `widgets/chrome.rs:243-361` · `win.rs:1851-1858` | 없음 | A (nexa-ctl `Toolbar`/`ToolIcon`) | 없음 |
| RENDER-038 | 파일 종류 아이콘 키 | 폴더 = `"dir"` · 확장자 없음/도트파일/끝 점 = `"file"` · 일반 = 소문자 확장자(`".txt"`) · 파일별 고유(`.exe .lnk .ico .cur .msi .scr .appref-ms`) = **소문자 전체 경로**. 구분자 `\`·`/` 모두 처리. 라지(타일) = `L|` 접두 — 캐시 키가 달라 소/라지 공존. `is_per_file`은 접두를 벗기고 판정 | `icons.rs:7-53` | 없음 | N | `icons.rs:485-540` 5건 |
| RENDER-039 | 아이콘 캐시(LRU + 로딩 큐) | `IconStore<T>`: 상한 초과 시 최소 사용 축출(축출·대체 값은 호출자에게 반환 → 핸들 해제), 큐는 키 단위 중복 제거, `take_batch(n)` 후에도 `finish`까지 in-flight 유지(재페인트 중복 로드 방지), 실패 후 재요청 가능 | `icons.rs:55-143` | 없음(제네릭) | N (또는 nexa-fs `IconService`로 대체) | `icons.rs:544-596` 4건 |
| RENDER-040 | 셸 아이콘 비동기 로드 | 상한 `CAPACITY 256` · 틱당 `BATCH 4` · 주기 `TICK_MS 80`. 타입 아이콘(dir/file/확장자) = `SHGFI_USEFILEATTRIBUTES`로 **UI 스레드 동기**(파일 접근 없음, 더미 이름 `x<ext>`), 파일별(exe·lnk…) = **워커 스레드 1개**(지연 생성·STA COM) → `PostMessage(WM_APP_ICON, Box<(키, HICON)>)` → `on_result`. PostMessage 실패 시 워커가 핸들 정리. 실패(raw 0) = 다음 미스에 재시도. 소 = `SHGFI_SMALLICON`, `L|` = `SHGFI_LARGEICON` | `icons.rs:145-170,260-477` · `win.rs:68-69,4949-4950,9072,9537` | `SHGetFileInfoW`·`DestroyIcon`·`PostMessageW`·`CoInitializeEx` | P | 없음(로더) |
| RENDER-041 | 목록·타일 아이콘 크기 | 목록(트리/플랫) 아이콘 = `indent_w`(16px@96dpi), 세로 중앙, 뒤 여백 `pad_x/2`. 타일 아이콘 = `row_h×2 - 4`(행 20 → 36, 라지 32를 근사 확대), 타일 셀 = `row_h×12` × `row_h×3`, 셀 안쪽 2px 여백 | `widgets/rows.rs:366-368,1381-1387,1563,1580-1589` | 없음 | N | 없음 |
| RENDER-042 | 앱 아이콘(창·대화상자) | `assets/nexa-dir.ico`(다크판) `include_bytes!` → ICONDIR 파싱 → 요청 크기에 가장 근접한 엔트리 → `CreateIconFromResourceEx`. 메인 창 32/16, About·대화상자·설정·일괄이름변경·미리보기 창 32. 실패 = 아이콘 없이 진행 | `icon.rs:1-47` · `win.rs:1736,1760` · `about.rs:118` · `dialog.rs:133,638` · `prefs.rs:2870` · `bulkrename.rs:2049` · `previewwnd.rs:149` | `CreateIconFromResourceEx` | P (winit `Icon` / Dock / .desktop) | 없음 |
| RENDER-043 | exe 리소스(아이콘·버전·매니페스트) | Windows 타깃에서만. `rc.exe`(PATH → Windows Kits 10 최신 x64)로 `.res` 생성 후 링크. **못 찾으면 경고 후 스킵**(빌드 실패 없음). 리소스: ICON id 1 · VERSIONINFO(CompanyName `SosomLab`, FileDescription/ProductName `Nexa Dir`, InternalName `nexa-dir`, OriginalFilename `NexaDir.exe`, 버전 = Cargo 버전 x,y,z,0) · RT_MANIFEST(asInvoker · PerMonitorV2 · longPathAware · supportedOS Win10/11) | `build.rs:12-162` | `rc.exe`·링커 인자 | W (타 OS: Info.plist / .desktop) | 없음 |
| RENDER-044 | 이미지 버튼 자원 | (a) 일괄 이름변경 ＋/− = PNG `rename-{add,remove}[-disabled]-32.png`(16/20 버킷 파일도 보관): `bulkrename.rs:55-58` (b) 순서 편집 ▲▼ = SVG `assets/ui/arrow-{up,down}.svg` 16px, 컨트롤 크기 **2배**로 래스터 후 Stretch, 비활성 = 잉크 알파 0x61 재렌더, 파싱 실패 = 벡터 `Plus` 폴백: `ordereditor.rs:304-325`, `ctl/iconbutton.rs:178-207` (c) 벡터 글리프 Plus/Minus/Help/Up/Down(원판 + 2px 폴리라인, Help는 GDI 텍스트 "?") | 좌동 | GDI+ | A | 없음 |

### 1-5. 테마·팔레트·폰트 체인(GDI 경로)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| RENDER-045 | 시맨틱 테마 토큰(다크/라이트) | `Theme` 16필드(§3-2 표). 기본 = **다크**. `Color{r,g,b}`(불투명 sRGB) + `from_hex(0xRRGGBB)`. 토큰 키는 안정 계약(rename 시 마이그레이션 표) | `nexa-gui/src/theme.rs:5-110` | 없음 | A (nexa-ctl `Theme` — 토큰 4개 부족) | `theme.rs:116-125` 2건 |
| RENDER-046 | 테마 모드(system/light/dark) | 설정 `theme` ∈ {system, light, dark}(기본 `dark`). system = OS `AppsUseLightTheme` 레지스트리(조회 실패 = 라이트). `WM_SETTINGCHANGE`에 재해석. 타이틀바 다크 동기(`apply_titlebar_theme`) | `win.rs:296-318,5566,7732,9662-` · `config.rs:67,254,570` | 레지스트리·DWM(추정 — 본문 미확인) | P (winit 테마) | `config.rs` 왕복 테스트 |
| RENDER-047 | ctl 팔레트(Style) | COLORREF 8필드 기본값(라이트): bg `#FFFFFF` · border `#A4A8AC` · text `#202020` · text_dim `#686E78` · accent `#0078D4` · sel_bg `#E4E7EC` · behind `#FFFFFF` · danger `#FF3B30`. **`behind` = 도형 밖 배후색**(1비트 리전 클립 대신 부모 배경으로 모서리를 칠하고 AA 도형을 얹음) | `ctl/style.rs:10-46` | COLORREF | A (nexa `Theme` 매핑 · `behind` 불필요) | 없음 |
| RENDER-048 | ctl 공통 치수 | `PAD_Y = 4` · 자동 높이 = 글꼴 높이(`tmHeight.max(12)`) + `PAD_Y×2` · 라벨 열 폭 = 텍스트 실측(`GetTextExtentPoint32W`) · `fill`(단색)·`frame`(1px 테두리 4변) | `ctl/style.rs:48-119` | GDI | A (`ControlBase`·`text_height`) | 없음 |
| RENDER-049 | 미리보기 창 글리프 단위 폴백 | `GdiChain`: `[0]` = 1순위, 이후 = 설치된 폴백. BMP 문자만 `GetGlyphIndicesW`로 글꼴 배정(비BMP는 1순위에 두고 OS FontLink), 어느 글꼴에도 없으면 1순위. 런 = 연속 동일 글꼴. `offsets`(문자 경계 누적 x — 선택 경계가 그리기와 일치)·`width`·`draw`(런별 `ExtTextOutW`). 콘솔 체인 = `term_font`(FIXED_PITCH, 기본 Consolas) / 본문 = `"Segoe UI, <term_font>"`(bold 700) · 높이 `-(pt.clamp(8,32) × dpi / 72)` | `fontchain.rs:79-253` · `previewwnd.rs:156-196` | `CreateFontW`·`GetGlyphIndicesW`·`GetTextExtentPoint32W`·`ExtTextOutW` | A (nexa `Font` face 체인 + `text_prefix_widths`) | 없음 |
| RENDER-050 | 대화상자·About·설정·툴팁 GDI 글꼴 | 체인 문자열 전체를 얼굴 이름으로 넘기면 매칭 실패 → `first_installed(체인, "Segoe UI")`로 1순위만 선택. 네이티브 컨트롤은 글리프 단위 개입 불가 | `fontchain.rs:1-14,44-49` · `dialog.rs:64` · `about.rs:71` · `prefs.rs:2831` | `CreateFontW` | A | 없음 |
| RENDER-051 | 설치 글꼴 열거 | `EnumFontFamiliesExW`(DEFAULT_CHARSET), `@`(세로쓰기) 제외, 소문자 정렬·dedup, 프로세스 1회 캐시. `is_installed`는 ASCII 대소문자 무시 비교 | `ctl/fontbox.rs:53-91` · `fontchain.rs:37-41` | `EnumFontFamiliesExW` | P (**nexa-font에 열거 API 없음**) | 없음 |
| RENDER-052 | 글꼴 관련 설정 키 | `base_font`/`_size`(Segoe UI/12) · `ctx_font`/`_size`(Segoe UI/12) · `status_font`/`_size`(Segoe UI/12) · `list_font`/`_size`(Segoe UI/12) · `term_font`/`_size`(Consolas/12) · `dlg_font`/`_size`(Segoe UI/9pt) · `list_folder_bold`·`header_bold`·`header_italic`. 크기 클램프: UI 슬롯·터미널 8~32, 대화상자 7~24. 길이 상한 term_font 128·dlg_font 64 | `config.rs:80-84,107-119,262-281,513-521,576-632` | 없음 | N (키는 nexa-sql 레지스트리 방식으로 재배치 — 설정 문서와 교차) | `config.rs:1230-1525` 왕복·퍼즈 |
| RENDER-053 | 색 변환 유틸 | `colorref(Color) → 0x00BBGGRR` · `color(COLORREF) → Color` · `rect(RECT) → Rect` · `c_argb` | `dw.rs:27-30` · `ctl/gdipctx.rs:32-48` | COLORREF | A (불필요 — `Color(u32)` 단일) | 없음 |
| RENDER-054 | 테스트 백엔드 허용 계약 | `DrawCtx` 필수 메서드 = `fill_rect`·`text_opaque`·`text_width` 3개뿐, 나머지는 기본 구현(no-op 또는 위임) — 텍스트/테스트 백엔드가 최소 구현으로 위젯을 돌릴 수 있음 | `nexa-gui/src/draw.rs:21-127` | 없음 | N (nexa-ctl도 같은 사상 — 필수 4개) | nexa-gui 위젯 테스트가 이용(범위 밖 — 추정) |

**기능 수: 54건(RENDER-001 ~ RENDER-054).**

---

## 2. 화면·컨트롤 배치 — 렌더 계층이 규정하는 치수·상수

> 창/대화상자 전체 배치(탭 순서·단축키)는 각 화면 인벤토리 문서 소관이다. 여기에는 **그리기 계층이 고정하는 수치**만 모은다(전부 96dpi 기준 · `s(v) = v × dpi / 96`).

### 2-1. 메인 창 세로 구성과 바 높이

| 영역 | 높이 | 근거 |
| --- | --- | --- |
| 메뉴 바 | `s(22)` | `nexa-dir2/crates/nexa-app/src/win.rs:1858` |
| 도구 모음 | `s(28)` — 셀 28 / 아이콘 20 | `win.rs:1851-1858` |
| 퀵 런처 바 | `s(24)` — 셀 24 / 아이콘 16. 숨김 또는 실행 항목 0이면 높이 0 | `win.rs:1858-1869` |
| 패널 영역 | 나머지 | `win.rs:1872-1877` |
| 상태바 | `s(22)` | `win.rs:1858,1873-1875` |
| 패널 스플리터 두께 | `SPLIT_TH = 3` → `max(2, s(3))` | `win.rs:53,4905` |
| 스플리터 히트 반폭 | `SPLIT_HALF = 3` | `win.rs:206,7970` |
| 스냅 거리 | `SNAP_PX = 20` | `win.rs:55,6155` |

### 2-2. 패널 지표(PanelMetrics)

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 행 높이 `row_h` | `s(20).max(14)` | `win.rs:1345-1348` |
| 좌우 패딩 `pad_x` | `s(6)` | `win.rs:1349` |
| 들여쓰기·소 아이콘 `indent_w` | `s(16)` | `win.rs:1350` |
| 탭 높이 `tab_h` | `s(22)` | `win.rs:1351` |
| 경로/네비 바 `bar_h` | `s(24)` | `win.rs:1352` |
| 기본 컬럼 폭 | 이름 340 · 확장자 64 · 크기 96(우측 정렬) · 수정 140 · 종류 110 | `win.rs:1357-1365` |
| 타일 셀 | 폭 `row_h×12` · 높이 `row_h×3` · 아이콘 `row_h×2-4` · 안쪽 여백 2px | `nexa-gui/src/widgets/rows.rs:366-368,1563,1580` |

### 2-3. 트리 셀(이름 컬럼) 가로 배치

`[pad_x][depth × indent_w][마커 셀 indent_w][아이콘 indent_w][pad_x/2][이름 …]` — 마커 = MDL2 셰브론(9 DIP, `text_dim`), 아이콘 세로 중앙, 폴더 이름 굵게 옵션(트리 보기 한정). 근거 `rows.rs:1370-1400`.

### 2-4. 글리프·글꼴 크기 표

| 용도 | 글꼴 | 크기 | 근거 |
| --- | --- | --- | --- |
| UI 본문(Base/List/Status) | 설정 체인(기본 Segoe UI) | 12 DIP(설정 8~32) | `dw.rs:292-303` · `config.rs:274-281` |
| 큰 글리프(비PUA) | Segoe UI | 15 DIP · 가로/세로 중앙 | `dw.rs:316-328` |
| MDL2 일반 | Segoe MDL2 Assets | 11 DIP | `dw.rs:346` |
| MDL2 셰브론(E70D/E70E/E76B/E76C) | 동상 | 9 DIP | `dw.rs:347-348,881-883` |
| MDL2 대형(네비 바) | 동상 | 13 DIP | `dw.rs:349-350` |
| 터미널 | 설정 체인(기본 Consolas) | 12 DIP(설정 8~32) | `dw.rs:356-381` |
| 대화상자(GDI) | 설정 체인(기본 Segoe UI) | 9pt(설정 7~24) | `config.rs:272-273,584-588` |
| SVG 내 텍스트 | Arial(고정) | `font-size × scale` | `ctl/gdipctx.rs:324-369` |

### 2-5. 보조 요소

| 요소 | 치수 | 근거 |
| --- | --- | --- |
| 도크 "크게(↗)" 버튼 | 변 = `min(row_h+4, 남은 높이-4)` · 우측 `pad_x` 안쪽 · 본문 상단 +2 · 아이콘 = 변−8(최소 8) · 누름 시 1px 우하 이동 · 배경: 누름 = panel_bg+accent 38%, hover = sel_bg, 기본 = header_bg | `widgets/dock.rs:361-412` |
| 진행 배지(패널 우하단) | padx 10 · pady 5 · 높이 22 · 우하 여백 12 · 배경 sel_bg · 1px accent 테두리 · 패널이 80×24 미만이면 생략 | `nexa-app/src/panel.rs:496-518` |
| 터미널 그리드 | 원점 x+2, y+1 · 셀 높이 `(font_px×4/3×dpi/96).max(12)` · 캐럿 폭 `(dpi/96).max(1)`, y+1, 높이 `cell_h-2` | `win.rs:2552,2674,2698,2722-2728` |
| 오버레이 스크롤바 | 얇음 6 / 넓음 10 · 알파 120(hot 210) · 트랙 알파 28 · 유지 22틱 · 페이드 24/틱 · 반경 = 두께/2 | `widgets/rows.rs:23-29` · `widgets/overlaybar.rs:264-275` |
| ctl 컨트롤 | `PAD_Y 4` · 자동 높이 = 글꼴 높이+8 · 아이콘 버튼 지름 = 글꼴 높이(최소 10) · 글리프 팔 길이 `max(3, d/4)` · 폴리라인 2px | `ctl/style.rs:74-84` · `ctl/iconbutton.rs:113-119,283-289` |

---

## 3. nexa-ui 매핑

### 3-1. DrawCtx 메서드 대조표

dir2 = `nexa-dir2/crates/nexa-gui/src/draw.rs` · nexa = `nexa-ui/crates/nexa-ctl/src/draw.rs`(구현 `raster.rs`).

| dir2 메서드(줄) | nexa-ctl 대응(줄) | 상태 | 차이·이식 메모 |
| --- | --- | --- | --- |
| `select_font(slot, bold, italic)` (25) | `select_font(slot, bold)` (45) · `select_font_sized(slot, bold, delta_px)` (52) | **부분** | italic 인자 없음(이탤릭은 `SlotFont.italic` 설정값으로만 — `raster.rs:145-150`). 슬롯 집합이 다름(§3-3). bold = 슬롯 설정 위 OR(`raster.rs:369-370`) |
| `fill_rect(rect, color)` (30) | `fill_rect` (58) | 있음 | nexa는 빈/음수 rect를 무시하고 표면 경계로 클립(`raster.rs:379-390`, `nexa-gfx/src/surface.rs:212-228`) → dir2의 음수 폭 함정 해소 |
| `text_opaque(x, y, clip, text, fg, bg)` (35) | `text_opaque` (62) | 있음(의미 차) | ① dir2는 **y 무시·clip 세로 중앙**, nexa는 **y = 텍스트 top**(`raster.rs:430-452`) → 호출부를 `ctx.text_center_y(clip.y, clip.h)`로 바꿔야 함 ② dir2는 오른쪽 **말줄임(…)**, nexa는 **하드 클립**(말줄임 없음) ③ nexa는 clip 사각 전체로 자름(좌·상·하 포함) |
| `text_width(text)` (38) | `text_width` (68) · `text_prefix_widths` (82) · `text_height` (94) · `text_ascent` (100) · `text_center_y` (109) | 있음(+확장) | nexa는 탭 정지점·제어문자 폭 0 규칙 포함(`nexa-gfx/src/text.rs:807-837`). PUA 분기 없음 |
| `push_clip(rect)` / `pop_clip()` (43/48) | — | **없음** | 추가 필요(§3-5 N-01). 대안: 텍스트는 clip 인자로 이미 사방 클립되므로 `fill_rect`·도형만 호출부에서 `Rect::intersection`(`nexa-ctl/src/geom.rs:90`) |
| `text(x, y, clip, text, fg)` (53) | `text` (65) | 있음(의미 차) | y 의미·말줄임 차이는 text_opaque와 동일 |
| `term_cell_w()` (58) | — | **없음** | 모노 `Font`를 base로 한 `RasterCtx`에서 `text_width("0")`로 구성 가능(nexa-sql 편집기 방식 `nexa-sql/crates/nexa-sql/src/app/paint.rs:408`). 헬퍼 추가 권장(N-07) |
| `term_text(x, y, clip, text, fg, bg)` (64) | — | **없음** | `fill_rect` + `text`로 구성. `FontSlot::Mono`는 크기가 Status를 따르므로(`raster.rs:354-357`) 터미널 전용 컨텍스트/슬롯 필요 |
| `draw_image(rect, 경로)` (71) | `image(x, y, &IconImage, clip)` (119) · `image_scaled(dst, &IconImage, clip)` (125) | 있음(계약 차) | nexa는 **디코드된 RGBA**를 받음. 경로→디코드→비율 축소→캐시는 호스트 몫(N-09) |
| `draw_icon(x, y, size, key, hint) -> bool` (78) | — | **없음** | 호스트 아이콘 캐시(키→`IconImage`) + `image`/`image_scaled`. "그렸는가" 반환은 호스트 조회 결과로 대체(N-05) |
| `glyph_opaque(clip, text, fg, bg)` (85) | — | **없음** | `fill_rect` + `select_font_sized` + 중앙 정렬 `text`, 또는 코드 도형 `controls::glyphs::glyph(GlyphKind)`(`nexa-ctl/src/controls/glyphs.rs:13-40,203`) |
| `glyph_opaque_lg` (93) | — | **없음** | 동상(크기만 다름) |
| `fill_ellipse(rect, color)` (102) | `fill_ellipse` (130) | 있음 | nexa는 근사 SDF(`raster.rs:592-607`) |
| `fill_round_rect(rect, radius, color)` (107) | `fill_round_rect` (148) | 있음 | 반경 클램프 `min(w/2, h/2)`(`raster.rs:219-221`) — dir2는 지름 `min(2r, w, h)`로 동치 |
| `fill_round_rect_alpha(rect, radius, color, alpha: u8)` (113) | `fill_round_rect_alpha(…, alpha: f32)` (188) | 있음(단위 차) | **u8 0~255 → f32 0.0~1.0**(`alpha as f32 / 255.0`) |
| `stroke_round_rect(rect, radius, color, width)` (119) | `stroke_round_rect` (194) · `stroke_round_rect_alpha` (200) | 있음(기하 차) | dir2 = 선이 **rect 안쪽**(인셋 `width/2+0.5`), nexa = **rect 경계 중심**(`sdf.abs() - half_w`, `raster.rs:266-295`). 코드 분석상 폭 1 선이 경계 양쪽 픽셀에 50%씩 걸칠 수 있음 — 호출부에서 rect를 안으로 줄여 넘기는 보정 필요 여부를 스냅샷으로 확인(추정) |
| `polyline(pts, color, width)` (124) | `polyline` (213) · `polyline_clipped` (218) | 있음 | 둥근 캡(선분 SDF) |
| — | `surface_size` (37) · `caret_on` (41) · `set_tab_origin` (73) · `fill_triangle` (114) · `stroke_ellipse` (136) · `fill_pie` (143) · `state_layer` (155) · `shadow` (165) · `fill_rect_alpha` (181) | nexa 전용 | hover/선택 = `state_layer` 알파 오버레이, 팝업 그림자 = `shadow` — dir2의 "사전 블렌드 색" 관행을 대체 가능 |
| — | `draw_tooltip`/`draw_tooltip_in` (227/240) · `ellipsize_middle` (309) · `set_show_full`/`show_full` (297/303) | nexa 전용 | dir2 `tip.rs`(별도 팝업 창) 대체 후보. 경로 가운데 축약 + Alt 전체 보기 |

필수 구현 메서드: dir2 3개(`fill_rect`·`text_opaque`·`text_width`) / nexa 4개(+`text`).

### 3-2. 테마 색 토큰 대조

dir2 `nexa-dir2/crates/nexa-gui/src/theme.rs:25-103` · nexa `nexa-ui/crates/nexa-ctl/src/theme.rs:12-131`.

| 토큰 | dir2 다크 / 라이트 | nexa-ctl | 비고 |
| --- | --- | --- | --- |
| `window_bg` | `14161A` / `F6F7F9` | 있음 · **동일 값** | dir2 위젯에서 `theme.window_bg` 참조 0회(Grep — 다른 변수명 경유는 미확인: 추정) |
| `chrome_bg` | `1E2228` / `EEF1F5` | 있음 · 동일 | 13회 |
| `panel_bg` | `191C21` / `FFFFFF` | 있음 · 동일 | 15회 |
| `panel_bg_alt` | `1F242B` / `F5F7FA` | 있음 · 동일 | 2회 |
| `tab_bar_bg` | `232830` / `E6EAF0` | **없음** | 3회 — 추가 필요 |
| `header_bg` | `262B33` / `E9ECF1` | **없음** | 13회 — 추가 필요 |
| `field_bg` | `262B33` / `FFFFFF` | 있음 · 동일 | 5회 |
| `bottom_dock_bg` | `1B1F25` / `F1F3F6` | **없음** | 0회(추정 — 동상) — 계약 유지를 위해 추가 권장 |
| `status_bar_bg` | `1E2228` / `EEF1F5` | **없음** | 3회 — 추가 필요(값은 chrome_bg와 같음) |
| `border` | `363C46` / `D5DAE1` | 있음 · 동일 | 30회 |
| `accent` | `3D8BFF` / `3D8BFF` | 있음 · 동일 | 45회 |
| `text` | `D6DAE0` / `1B1F26` | 있음 · 동일 | — |
| `text_dim` | `8A919C` / `6B7280` | 있음 · 동일 | 22회 |
| `sel_bg` | `24405F` / `D8E8FF` | 있음 · 동일 | 14회 |
| `sel_bg_inactive` | `2C313A` / `E6E9EE` | 있음 · 동일 | 4회 |
| `is_dark` | true / false | 있음 | 7회 |
| — | — | nexa 전용: `bubble_peer` · `focus_ring` · `syn_keyword/string/comment/number` · `rainbow[6]` · `danger` · `ok` · `warn` | `danger`(다크 `E5534B`/라이트 `D32F2F`)는 dir2 `Style.danger #FF3B30`과 값이 다름 |

결론: 공통 12토큰은 **값까지 동일**(nexa-ctl이 dir2에서 이식됨 — `nexa-ctl/src/theme.rs:1`). 파일 탐색기 전용 4토큰(`tab_bar_bg`·`header_bg`·`bottom_dock_bg`·`status_bar_bg`)만 nexa-ctl에 없다 — nexa-ctl `Theme`에 추가하거나 dir3 앱 측 확장 구조체(`DirTheme { base: Theme, tab_bar_bg, … }`)로 둔다(N-06).

자료형: dir2 `Color { r, g, b }`(필드 직접 접근 — 위젯이 `theme.text.r` 식으로 블렌드) ↔ nexa `Color(pub u32)` = `0x00RRGGBB` + `rgb()`·`from_rgb()`·`lerp(other, t)`(`nexa-ui/crates/nexa-gfx/src/surface.rs:8-37`). dir2의 `mix(a, b) = a + (b-a)×0.38` 블렌드는 `chrome_bg.lerp(accent, 0.38)`로 치환(반올림 방식 차이: dir2는 절삭, nexa는 round — 1단계 색 차 가능).

ctl `Style`(§RENDER-047) → nexa `Theme`: `bg → field_bg` · `border → border` · `text → text` · `text_dim → text_dim` · `accent → accent` · `sel_bg → sel_bg` · `danger → danger` · `behind → 불필요`(SDF 커버리지 진짜 블렌드 — `raster.rs:1-6`).

### 3-3. 폰트 슬롯·크기·폴백 체인 대조

| 항목 | dir2 | nexa-ui | 이식 메모 |
| --- | --- | --- | --- |
| 슬롯 | `Base` · `List` · `Status`(`nexa-gui/src/draw.rs:11-19`) | `Base` · `PeerList` · `Message` · `Status` · `Mono`(`nexa-ctl/src/draw.rs:16-28`) | `List → PeerList`로 **의미 재사용**하거나 슬롯 추가(N-03). `ctx_font`(컨텍스트 메뉴)는 dir2에서 GDI 메뉴 경로 — nexa `ctxmenu`가 쓰는 슬롯에 대응시켜야 함(추정) |
| 크기 단위 | **DIP = em 크기**(DirectWrite) | **`size` = ab_glyph 높이 스케일(ascent − descent px)** — em이 아님(`nexa-gfx/src/text.rs:401-412`) | 같은 숫자를 넘기면 **글자가 작아진다**. 변환 `size = em × height_unscaled / units_per_em`(예: SF 1.1777 — `nexa-font/src/lib.rs:612-613`). 공개 변환 API 없음 → 추가 필요(N-03) |
| 기본 크기 | 12 DIP | `FontPrefs::default()` base 16 · peerlist 16 · message 18 · status 15(`nexa-ctl/src/theme.rs:386-400`) | dir2 12 DIP ≈ nexa 16 내외(글꼴별 상이 — 추정) |
| 배율 | `SetPixelsPerDip(dpi/96)` | `RasterCtx::new(surface, font, scale)` — scale 필수 인자(`raster.rs:80-109`) | 좌표는 이미 물리 px, 글자 크기만 scale 곱 |
| 굵게 | SEMI_BOLD(600) 실제 face | ab_glyph 경로 = faux(x축 2회 그리기) · GDI/CoreText 경로 = FW_BOLD(700)/bold trait(`text.rs:922-923`, `gdi.rs:221`, `coretext.rs:233-247`) | 굵기 단계 차 — 폭 측정이 달라져 컬럼 auto-fit 결과가 달라질 수 있음 |
| 이탤릭 | 실제 italic face | faux 전단 0.22(`text.rs:904`) | — |
| 말줄임 | 문자 단위 … 트리밍(백엔드) | 없음 — `ellipsize_middle`만(`nexa-ctl/src/draw.rs:309`) | **끝 말줄임 헬퍼 추가 필요**(N-02) |
| 체인 규약 | 쉼표 목록 = 폴백 체인(설치된 것만) → 시스템 폴백 | `ui_font(family: Option<&str>)`·`mono_font(…)` — **단일 패밀리** + 시스템 UI 본 + 기호 폴백 + 고정폭 폴백(`nexa-font/src/lib.rs:417-511`) | 쉼표 체인 로더 추가 필요(N-08): 패밀리마다 `find_font_by_family` → `Font::push_fallback` |
| 설치 판정 | `EnumFontFamiliesExW` 패밀리명 | 파일명 어간 정규화 비교(`find_font_by_family` — 파일명 ≠ 패밀리명이면 못 찾음: `nexa-font/src/lib.rs:302-328`) | 열거 API 없음(N-08) — `Font::face_family_names`(`text.rs:429-444`)를 이용한 name 테이블 열거 추가 |
| 글리프 폴백 단위 | DW = 레이아웃 폴백 / GDI = `GdiChain` 런 분할 | face 체인에서 **글자마다 글리프가 있는 첫 face**(`text.rs:507-512`) + 슬롯 얼굴에 없으면 base로 런 분할(`raster.rs:306-324,430-474`) | 기준선은 주 폰트가 정함 |
| 터미널 한글 | DW 시스템 폴백 | `mono_font`: D2Coding·D2CodingLigature·SarasaMonoK·NanumGothicCoding 우선 → OS 고정폭 → 한글 UI 본 폴백(`nexa-font/src/lib.rs:51-57,336-349,473-511`) | 한글 고정폭 본이 없으면 한글 폭 = 비례폰트 폭(정직한 한계) → 셀 개별 배치 규약(RENDER-021)으로 흡수 |
| 기호 커버리지 | Segoe MDL2 / 시스템 폴백 | `UI_SYMBOLS` 전수 커버 테스트(`nexa-font/src/lib.rs:380,574-589`) | dir2가 쓰는 `↗`(`dock.rs:401`) 등은 `UI_SYMBOLS`에 없음 → 추가 또는 마스크 아이콘화 |
| 캐시 | 레이아웃 4096 / 모노 글리프 2048 | 글리프 비트맵 8192(설정 가능 `set_glyph_cache_max`) — 넘치면 비움(`text.rs:249-263`) | 설정 키 `ui.glyph_cache`(`nexa-sql/crates/nsql-settings/src/perf.rs:227`) |

OS 텍스트 래스터 품질 스위치(nexa-gfx · 프로세스 전역): `set_text_gdi`(Windows GDI ClearType / macOS CoreText — `text.rs:47-62`), `set_text_contrast`(감마), `set_text_snap`, `set_text_hint`, `set_text_weight`. nexa-sql 설정 키 = `ui.text_gdi`·`ui.text_contrast`·`ui.text_snap`·`ui.text_hint`·`ui.text_weight`(`nexa-sql/crates/nsql-settings/src/lib.rs:752-784`, 적용 `nexa-sql/crates/nexa-sql/src/app/settings.rs:142-146`).

### 3-4. 아이콘 자원 — 형식·위치·로딩 방식

| 자원 | 위치(dir2) | 형식 | 로딩(dir2) | nexa-ui 대응 |
| --- | --- | --- | --- | --- |
| 툴바/도크 아이콘 25종 | `nexa-dir2/crates/nexa-app/assets/toolbar/*.svg`(25개 파일 전부 등록) | SVG · viewBox 32 · stroke 2 · currentColor(다크판은 하드코딩 색·`fill-rule="evenodd"` 사용 예 `dock-dark.svg`) | `include_str!` → `svg::parse` → GDI+ 래스터 → HICON LRU | `ToolIcon::{Glyph, Image(Rc<IconImage>), Mask{w,h,alpha:&'static [u8]}}`(`nexa-ctl/src/controls/toolbar.rs:24-36`) · `MenuIcon::{from_alpha, from_rgba}`(`controls/ctxmenu.rs:55-97`). **SVG 파서·스트로크 래스터는 nexa-ui에 없음**(Grep `svg` 0건). nexa-sql 앱 쪽 `toolicons.rs:764-1218`에 path 채움 전용 래스터(`svg_glyph_vb` — 다각형 근사 + 짝홀/비영)가 있으나 **스트로크·rect·text 요소 미지원** |
| 순서 편집 ▲▼ | `assets/ui/arrow-{up,down}.svg` | SVG stroke 3 | `iconbutton::create_svg` | 동상 |
| 일괄 이름변경 ± | `assets/rename/rename-{add,remove}[-disabled]-{16,20,32}.png` | PNG RGBA | `include_bytes!` → GDI+ 디코드 | `nexa_gfx::image::decode`(PNG 지원 `nexa-gfx/src/image.rs:73-82`) → `IconImage` → `image_scaled` |
| 앱 아이콘 | `assets/nexa-dir.ico`·`nexa-dir-light.ico`(각 7엔트리 16/24/32/48/64/128/256 — **전 엔트리 PNG 압축**: 바이트 실측) · `nexa-dir-1024.png`·`nexa-dir-light-1024.png` | ICO / PNG | ICONDIR 파싱 → `CreateIconFromResourceEx` | ICONDIR 파싱은 중립 로직 그대로 + 엔트리 PNG를 `nexa_gfx::image::decode`로 디코드 → winit 창 아이콘. (nexa-sql은 코드 도형으로 앱 아이콘을 그림 `nexa-sql/crates/nexa-sql/src/icon.rs:1-13` — dir3는 "자원 그대로" 기조이므로 ICO/PNG 디코드 경로 채택) |
| 디스클로저·네비 글리프 | Segoe MDL2 Assets(OS 폰트) | 폰트 PUA | DirectWrite | `controls::glyphs::GlyphKind`: `Folder`·`FolderNew`·`Refresh`·`ArrowBack`·`ArrowForward`·`ArrowUp`·`Home`·`ToggleOn/Off`·`Copy`·`Link`·`Open`·`Text`(`glyphs.rs:13-40`) — 홈/뒤/앞/위/새로고침은 **있음**. 셰브론(›/∨)·설정(톱니)은 GlyphKind에 없음 → `polyline` 직접 또는 GlyphKind 추가 |
| 셸 파일 아이콘 | OS | HICON | `SHGetFileInfoW`(§RENDER-040) | `nexa_fs::shell::IconService`(`nexa-ui/crates/nexa-fs/src/shell.rs:108-327`): `IconKey::{Kind{ext,is_dir}, Path}` · `Lookup::{Ready, Pending}` · `version()` 폴링 · 워커 스레드(유휴 30초 회수) · 상한 512 · 실패 기억. **Windows만 구현, macOS/Linux = `SUPPORTED=false` → 항상 None**(`shell.rs:737-760`) |
| SVG 다이어그램(미리보기) | 플러그인 생성 SVG | SVG 서브셋 | `svg_to_pixels` → BMP 캐시 | 동일 SVG 래스터(추가분)로 `IconImage` 생성 — 임시 BMP 우회 불필요 |

### 3-5. nexa-ui에 없는 것 — 추가 필요 목록

| # | 추가 대상 | 위치 제안 | 필요한 API 요약 | 관련 ID |
| --- | --- | --- | --- | --- |
| N-01 | 클립 스택 | `nexa-ctl` `DrawCtx` + `RasterCtx` | `fn push_clip(&mut self, rect: Rect)` / `fn pop_clip(&mut self)`(기본 no-op). RasterCtx는 클립 스택을 두고 `fill_rect`·도형·이미지·텍스트 clip을 교차 | RENDER-023 |
| N-02 | 끝 말줄임 | `nexa-ctl/src/draw.rs` | `pub fn ellipsize_end(ctx, text, max_w) -> Cow<str>`(접두사 폭 표 `text_prefix_widths` 이용 · 문자 단위 · `…` 폭 포함) + 편의 `text_opaque_ellipsis` | RENDER-010/011 |
| N-03 | 슬롯·장식·크기 변환 | `nexa-ctl` `FontSlot`/`DrawCtx` · `nexa-gfx` `Font` | `select_font`에 italic 지정 경로(예: `select_font_styled(slot, bold, italic)`) · 파일 목록/터미널 슬롯 또는 슬롯 별칭 · `Font::size_for_em(em_px) -> f32`(em ↔ 높이 스케일 변환 공개) | RENDER-007/008 |
| N-04 | SVG 서브셋 + 스트로크 래스터 | `nexa-gfx`(신규 `svg` 모듈) 또는 `nexa-ctl` | dir2 `svg.rs` 파서(중립) 이식 + `fn render(doc: &Doc, px: u32, ink_argb: u32) -> IconImage`: rect(rx)/circle/line/polyline/path(M L C Arc Z)/text, 요소별 색·채움·굵기, 둥근 캡/조인, 짝홀 채움, 4× 슈퍼샘플. 텍스트 요소는 `Font`로 그림 | RENDER-034/035/036 |
| N-05 | 아이콘 캐시 + 그리기 | dir3 앱(또는 `nexa-ctl` 헬퍼) | `IconCache::get(key, size) -> Option<Rc<IconImage>>`(버킷 16/20/24/32 · 잉크/알파/다크 변형 키 · LRU) + `draw_icon` 상당 헬퍼(없으면 false) | RENDER-031~033 |
| N-06 | 파일 탐색기 테마 토큰 | `nexa-ctl/src/theme.rs` 또는 dir3 확장 | `tab_bar_bg` · `header_bg` · `bottom_dock_bg` · `status_bar_bg`(값은 §3-2) | RENDER-045 |
| N-07 | 터미널 셀 그리기 헬퍼 | dir3 터미널 뷰(또는 `nexa-ctl`) | 모노 `Font` 기반 `cell_w()`(= "0" 전진 폭 ceil) · `draw_cell(x, y, clip, ch, fg, bg)` — 문자 개별 배치 | RENDER-018~021 |
| N-08 | 폰트 체인·열거 | `nexa-font` | `fn chain_font(chain: &str, mono: bool) -> Option<Loaded>`(쉼표 목록 → 설치된 것만 순서대로 face 체인) · `fn installed_families() -> &'static [String]`(name 테이블 nameID 1/16 기반 — 현재 `nexa-gfx/src/names.rs`는 `pub(crate)`) | RENDER-009/051 |
| N-09 | 경로 이미지 미리보기 캐시 | dir3 앱 | 경로 → 바이트 → `nexa_gfx::image::decode(bytes, max_pixels)` → 비율 축소(`IconImage::resized`) → `(경로, w, h)` 캐시(상한 8) · 디코드는 워커 스레드 권장 | RENDER-028 |
| N-10 | 셸 아이콘 타 OS 구현 | `nexa-fs/src/shell.rs` `imp`(macOS·Linux) | `icon_for_kind`·`icon_for_path`·`kind_name` 구현(§4) + `SUPPORTED = true` | RENDER-040 |
| N-11 | 글리프 종류 보강 | `nexa-ctl/src/controls/glyphs.rs` | `ChevronRight`·`ChevronDown`·`Settings` 등(또는 `polyline` 직접) | RENDER-014~016 |

---

## 4. OS 분기점

| # | 항목 | Windows(dir2 현 구현) | macOS | Linux | dir3 권장 형태 |
| --- | --- | --- | --- | --- | --- |
| B-01 | 화면 표면 | DW 비트맵 렌더 타깃 메모리 DC → `BitBlt` | softbuffer(winit) | softbuffer(X11 SHM / Wayland wl_shm) | **분기 없음** — `u32 0x00RRGGBB` 버퍼를 `nexa_gfx::Surface`로 감싸 present(`nexa-gfx/src/surface.rs:170-188`). nexa-sql `present.rs`·`nexa-sys/src/layer_present.rs` 방식 차용(상세는 창/플랫폼 문서) |
| B-02 | 글리프 래스터 | DirectWrite(ClearType·힌팅) | CoreText 회색 커버리지 + 소수 전진(`nexa-gfx/src/coretext.rs`) | ab_glyph(힌팅 없음) + 감마/스냅/힌트 근사/굵기 보강 옵션 | `set_text_gdi(true)`면 Windows=GDI ClearType(정수 전진·서브픽셀 RGB), macOS=CoreText. Linux는 항상 ab_glyph(`text.rs:420-424`). OS별 기본값을 설정 레지스트리에 둠(nexa-sql `ui.text_*`) |
| B-03 | 기본 UI 글꼴 | Segoe UI(설정) | `.AppleSystemUIFont`(`/System/Library/Fonts/SFNS.ttf`) + Apple SD Gothic Neo | Noto Sans CJK KR(TTC 인덱스 2) → 나눔고딕 → DejaVu Sans | nexa-font 후보 표(`nexa-font/src/lib.rs:17-49,399-415`). **dir2 기본값 "Segoe UI"/"Consolas"를 그대로 저장하면 타 OS에서 미설치** → 기본값은 빈 문자열(= 시스템 기본)로 두고 표시만 "(시스템 기본)" |
| B-04 | 기본 고정폭 글꼴 | Consolas → Courier New | Menlo → SF Mono → Monaco | DejaVu Sans Mono → Liberation Mono → Noto Sans Mono | 어느 OS든 D2Coding 등 한글 고정폭 우선(`lib.rs:51-87`) |
| B-05 | 기호 폴백 | Segoe UI Symbol/Emoji · Arial Unicode | Apple Symbols · Arial Unicode · STIX Two Math | DejaVu Sans · Noto Sans Symbols2 · Symbola 등 | `symbol_fallback_fonts()`(`lib.rs:90-183,351-376`) |
| B-06 | 폰트 폴더 | `C:\Windows\Fonts` · `~/AppData/Local/Microsoft/Windows/Fonts` | `~/Library/Fonts` · `/Library/Fonts` · `/System/Library/Fonts` | `~/.local/share/fonts` · `~/.fonts` · `/usr/share/fonts` · `/usr/local/share/fonts`(깊이 3) | `FONT_DIRS`(`lib.rs:185-198,299-300`) |
| B-07 | 아이콘 폰트(MDL2) | Segoe MDL2 Assets | 없음 | 없음 | **전 OS 동일하게 코드 도형/SVG 마스크**로 교체(분기 제거). 근거: `nexa-dir2/docs/23-cross-platform-feasibility.md:144-147` |
| B-08 | 벡터/도형 래스터 | GDI+ | — | — | 분기 없음 — `RasterCtx` SDF |
| B-09 | 이미지 디코드 | WIC(OS 코덱 전부: JPEG 프로그레시브·TIFF·ICO·WebP[코덱 설치 시] 등) | (선택) ImageIO `CGImageSourceCreateWithData` | — | 기본 = `nexa_gfx::image`(PNG[Adam7 포함]·BMP[비압축]·GIF 첫 프레임·JPEG 기저). **미지원: 프로그레시브 JPEG·WebP·TIFF·ICO·HEIC**(`image.rs:73-82`, `jpeg.rs:315-318`). dir2 대비 미리보기 가능 포맷이 줄어듦 — 미지원은 안내 문구. OS 코덱 사용은 "직접 개발" 기조와 충돌하므로 기본 비채택 |
| B-10 | 파일 종류 아이콘 | `SHGetFileInfoW` + `SHGFI_USEFILEATTRIBUTES`(nexa-fs 구현 있음 — HICON → 32bpp DIB → RGBA `shell.rs:386-735`) | `NSWorkspace iconForFile:` / `iconForContentType:`(UTType) → `NSImage` → `CGImage` → `CGBitmapContext` RGBA(16/32px, Retina는 ×scale) | freedesktop Icon Theme Spec: 테마 탐색(`$XDG_DATA_HOME/icons`, `~/.icons`, `/usr/share/icons`, `/usr/share/pixmaps`) → `index.theme`(Inherits → `hicolor` 폴백) → 크기 디렉터리의 PNG 우선. MIME 판정 = shared-mime-info `globs2`(확장자 → MIME) → 아이콘 이름 `text-plain` / `generic-icons` / 폴더 `folder`. 현재 테마 이름 = GSettings `org.gnome.desktop.interface icon-theme` 또는 `~/.config/gtk-3.0/settings.ini`(추정 — DE별 상이) | `nexa-fs` `imp` 모듈에 OS별 구현(N-10). 전 OS 공통 폴백 = 코드 도형(`GlyphKind::Folder` 등). 키 규약(RENDER-038)은 중립 유지 |
| B-11 | 파일별 고유 아이콘 | `.exe .lnk .ico .cur .msi .scr .appref-ms` → 실제 경로 조회(워커) | `.app` 번들·`.icns`·실행 파일 → `iconForFile:` | `.desktop`(Icon= 키)·AppImage·실행 파일(대개 일반 아이콘) | `PER_FILE_EXTS`를 OS별 상수로 분기(`icons.rs:8`) |
| B-12 | 아이콘 워커 COM | STA `CoInitializeEx` | 불필요(단, AppKit 호출은 메인 스레드 제약 확인 필요 — 추정) | 불필요 | `nexa-fs` `ComApartment`(타 OS no-op `shell.rs:742-747`) |
| B-13 | 결과 통지 | `PostMessageW(WM_APP_ICON)` | — | — | 분기 없음 — `IconService::version()` 폴링 또는 winit `EventLoopProxy` 사용자 이벤트 |
| B-14 | 앱 아이콘(창) | 창 클래스 `hIcon`(32)/소(16) | 창 아이콘 무시 → Dock 아이콘(`NSApplication setApplicationIconImage:`) · 번들 `.icns` | X11 `_NET_WM_ICON`(winit) · Wayland는 `.desktop` + hicolor PNG | winit `Icon::from_rgba` + macOS Dock 설정(nexa-sql `icon.rs:8-13` 표 참고). 원본 = ICO 엔트리 PNG / 1024 PNG |
| B-15 | 실행 파일 리소스 | `rc.exe` — ICON·VERSIONINFO·매니페스트 | `.app` 번들 `Info.plist`(CFBundleShortVersionString·NSHighResolutionCapable)·`.icns` | `.desktop`·AppStream·hicolor 아이콘 | `build.rs`는 Windows 분기 유지(이미 `CARGO_CFG_TARGET_OS` 가드 `build.rs:16-19`), 타 OS는 패키징 단계 |
| B-16 | DPI | PerMonitorV2 + `WM_DPICHANGED` + `GetDpiForWindow` · 분수 배율(125/150%) | Retina 정수 배율(2.0) · 모니터 간 이동 시 변경 | Wayland fractional-scale / X11 `Xft.dpi`(winit 추상) | winit `scale_factor()`·`ScaleFactorChanged`(nexa-sql `app/event_loop.rs:86,654,668-669`) → `RasterCtx::new(.., scale)` + 지표 `s(v)` 재계산 + 글리프 캐시는 크기 키라 자동 분리 |
| B-17 | OS 테마 추종 | 레지스트리 `AppsUseLightTheme` + `WM_SETTINGCHANGE` | `AppleInterfaceStyle`(NSApp effectiveAppearance) | xdg-desktop-portal `org.freedesktop.appearance color-scheme` | winit `Window::theme()`/`ThemeChanged`(추정 — nexa-sql 구현 방식은 설정/창 문서에서 확인). 모드 열거는 `nsql_settings::ThemeMode`(`nexa-sql/crates/nsql-settings/src/lib.rs:167-229`) |
| B-18 | 타이틀바 색 | `apply_titlebar_theme`(DWM — 본문 미확인: 추정) | 시스템 자동 | CSD/SSD에 따라 상이 | winit `set_theme` (추정) |
| B-19 | 상주 트림 | `SetProcessWorkingSetSize(-1,-1)` | 없음 | 없음(`malloc_trim` 선택) | 캐시 해제만 공통, 작업집합 반납은 Windows 한정 |
| B-20 | 설치 글꼴 열거 | `EnumFontFamiliesExW` | 폰트 폴더 스캔 + name 테이블(또는 CoreText `CTFontManagerCopyAvailableFontFamilyNames`) | 폰트 폴더 스캔 + name 테이블(또는 `fc-list`) | 공통 = 폴더 스캔 + name 테이블(N-08) — OS API 미사용으로 분기 최소화 |
| B-21 | 임시 SVG→BMP 캐시 | `temp_dir()/nexa-preview/*.bmp` | 동일 경로 규칙 | 동일 | 분기 없음(또는 메모리 `IconImage`로 대체) |

---

## 5. 상태·영속(설정 키·파일 형식) · 스레딩·메시지 흐름

### 5-1. 영속 상태(렌더 관련)

- 파일: `data\settings.cfg` — `key=value` 줄 형식, 헤더 `# nexa-dir settings v1`(`nexa-dir2/crates/nexa-app/src/config.rs:63,434`).
- 렌더 관련 키와 기본값(RENDER-052): `theme=dark` · `base_font=Segoe UI`/`base_font_size=12` · `ctx_font`/`ctx_font_size` · `status_font`/`status_font_size` · `list_font`/`list_font_size` · `list_folder_bold`·`header_bold`·`header_italic` · `term_font=Consolas`/`term_font_size=12` · `dlg_font=Segoe UI`/`dlg_font_size=9` · (터미널 색) `term_theme=system`/`term_theme_dark`/`term_theme_light`.
- dir3에서는 nexa-sql 설정 레지스트리(nsql-settings) 구조를 차용: 대응 키 후보 `ui.theme` · `ui.font_face` · `ui.font_size` · `ui.menu_font_size` · `ui.text_contrast`/`ui.text_weight`/`ui.text_gdi`/`ui.text_hint`/`ui.text_snap` · `ui.glyph_cache` · `file.os_icons` · `file.icon_cache` · `editor.font_size` · `explorer.font_size`(`nexa-sql/crates/nsql-settings/src/lib.rs:728-796,2184,4699` · `perf.rs:209,227-228`). dir2 키 → 새 키 매핑표는 설정 문서에서 확정(여기서는 **슬롯별 글꼴·크기·장식 9+3개 키가 모두 대응 키를 가져야 한다**는 요구만 기록).

### 5-2. 런타임 상태(비영속)

| 상태 | 소유 | 수명 | 근거 |
| --- | --- | --- | --- |
| `DwBackend`(팩토리·렌더 타깃·포맷 맵·레이아웃 캐시·모노 글리프 캐시·이미지 캐시·WIC 팩토리) | `State.dw: Option<DwBackend>` | 창 1개 · 상주 트림 시 통째 해제 후 지연 재생성 | `dw.rs:188-235` · `win.rs:1990-2008,2020` |
| 현재 폰트 스타일(`cur_style`)·대형 글리프 플래그 | 백엔드 `Cell` | 프레임 내 공유 — 위젯이 페인트 시작에 자기 슬롯을 선택(순서 무관 보장), 장식 변경 후 **복원 필수** | `dw.rs:195-199` · `rows.rs:1393-1399,2250,2309` |
| 글리프 색 | `Rc<Cell<COLORREF>>`(렌더러 콜백과 공유) | 그리기 직전 설정 | `dw.rs:39-46,386-392` |
| 아이콘 캐시 `ShellIcons`(LRU 256 + 큐 + 워커 채널) | `State.icons: RefCell<…>` | 창 수명 · 트림 시 전 핸들 해제 | `icons.rs:265-277,415-435` |
| GDI+ 토큰 | `OnceLock<usize>` | 프로세스(종료 시 OS 회수) | `ctl/gdipctx.rs:418-435` |
| 설치 글꼴 목록 | `OnceLock<Vec<String>>` | 프로세스 1회 | `ctl/fontbox.rs:55-91` |
| 미리보기 다크 플래그 | `thread_local Cell<bool>` | 호스트가 미리보기 전 주입 | `preview/mod.rs:60-74` · `win.rs:2441` |
| nexa 측: 텍스트 품질 전역(`TEXT_GAMMA`·`TEXT_SNAP`·`TEXT_HINT`·`TEXT_GDI`·`TEXT_WEIGHT`·`TAB_COLS`·`TAB_STOPS`·`GLYPH_CACHE_LIMIT`) | 프로세스 전역 atomic | — | `nexa-gfx/src/text.rs:11-66,194-263` |
| nexa 측: GDI/CoreText face 캐시 | **스레드 로컬**(HDC·HFONT·DIB는 스레드 소속) | 스레드 | `nexa-gfx/src/gdi.rs:10-11,198-200` · `coretext.rs:160-162` |
| nexa 측: 폰트 바이트 | mmap `Box::leak`(프로세스 수명) | — | `nexa-font/src/lib.rs:215-224` |

### 5-3. 스레딩·메시지 흐름

- **그리기 = UI 스레드 단일**. `WM_PAINT` → `ensure_dw` → 위젯 `paint(&mut ctx, &theme)` → `BitBlt` → 통계 누적 → (탭 줄 수 변화 시 재레이아웃) → 아이콘 큐가 남았으면 `SetTimer(TIMER_ICONS, 80ms)`(`win.rs:4820-4951`).
- **아이콘**: 페인트 중 미스 → 큐 등록(그리지 않음) → 타이머 틱마다 4개 → 타입 아이콘은 동기 로드·`true` 반환 시 재도장, 파일별은 워커 송신 → 워커가 `PostMessageW(WM_APP_ICON, WPARAM=Box)` → UI `on_result` → 재도장(`icons.rs:342-413` · `win.rs:9072,9537`).
- **이미지 미리보기**: UI 스레드 동기 WIC 디코드(STA 전제 — 기동 시 `OleInitialize`) — 대형 이미지는 UI 블로킹 가능(`dw.rs:525-534`).
- nexa-ui 대응 흐름: `IconService::icon()` → `Lookup::Pending` 동안 자체 도형 → `version()` 변화 시 재도장, `pending() > 0` 동안 폴링 타이머 유지(`nexa-fs/src/shell.rs:165-169,262-275,321-326`). 워커는 유휴 30초면 자동 회수.
- nexa 텍스트 전역 스위치는 **프로세스 전역**이라 테스트 병렬 실행 시 경주가 난다 — 테스트는 직렬화 가드 필요(`nexa-font/src/lib.rs:517-550`).

---

## 6. 이식 시 주의 — 회귀 방지용 실측 교훈

| ID | 교훈 | 근거 |
| --- | --- | --- |
| RENDER-L01 | **텍스트 y 의미가 다르다.** dir2 백엔드는 `y`를 무시하고 clip 세로 중앙에 그린다. nexa-ctl은 `y`를 top으로 쓴다 → 위젯 이식 시 `ty = b.y + (b.h - b.h*4/5)/2` 식을 그대로 넘기면 세로 위치가 달라진다. `ctx.text_center_y(clip.y, clip.h)`(잉크 기준 중앙 — 맥/윈도 글꼴 차 보정 포함)로 통일 | `dw.rs:642,781` · `nexa-ctl/src/draw.rs:104-111` · `raster.rs:571-581` |
| RENDER-L02 | **글꼴 크기 단위가 다르다.** dir2 = em(DIP), nexa = 높이 스케일(ascent−descent). 숫자를 그대로 옮기면 약 25% 작게 보인다(글꼴별 상이). em→size 변환을 한 곳에 둔다 | `nexa-gfx/src/text.rs:401-412` · `nexa-font/src/lib.rs:612-613` |
| RENDER-L03 | **음수 폭 rect 채움 = 화면 전체 덮어칠.** 싱글 패널에서 우 패널이 0-rect라 스플리터 폭이 음수 → GDI가 정규화해 패널 전체를 border 색으로 칠함(QA 07-17 공백 화면). nexa는 빈 rect를 무시하지만 `w > 0` 가드는 유지 | `win.rs:4886-4900,4907-4916` |
| RENDER-L04 | **런 단위 터미널 그리기 금지.** 폴백 글꼴(한글·아이콘)의 전진폭이 셀 그리드와 어긋나 열이 밀린다 → 문자는 셀 x에 개별 배치(QA 07-14) | `win.rs:2656-2658,2692-2702` |
| RENDER-L05 | **셀 단위 렌더는 캐시 필수.** 프레임당 레이아웃 생성 비용 → 단일 문자 캐시(QA 07-14). nexa는 글리프 비트맵 캐시가 대체하나 캐시 키에 size 비트가 들어가므로 크기 변경 연타 시 상한 비움 동작 확인 | `dw.rs:220-221,684-713` · `text.rs:213-229,248-263` |
| RENDER-L06 | **캐시 조회는 무할당으로.** 튜플 키 `(String, i32)`는 조회마다 할당 → 중첩 맵 + `&str` 조회(X-16) | `dw.rs:222-226,432-435` |
| RENDER-L07 | **아이콘 폰트 대역 판정은 E700~F8FF 전체.** E8FF 초과 글리프(EA8A HomeSolid)가 본문 폰트로 새면 두부(□)(08-01). 측정 쪽(`text_width`)은 아직 E8FF까지만 본다(불일치 — 이식 시 글리프를 마스크로 바꾸면 소멸) | `dw.rs:810-813,876-878` · `chrome.rs:339-343` |
| RENDER-L08 | **PUA 글리프 폭이 0으로 측정되던 결함**(본문 폰트에 없음) → 그리기와 같은 폰트로 측정(07-18). 일반 원칙: 측정과 그리기는 같은 규칙 | `dw.rs:808-828` |
| RENDER-L09 | **체인 문자열 전체를 얼굴 이름으로 넘기면 1순위도 못 맞춘다**("A, B" → 기본 글꼴 대체) → 반드시 파싱 후 설치된 첫 패밀리(09-04) | `fontchain.rs:1-7` · `dw.rs:289-291` · `dialog.rs:63-64` |
| RENDER-L10 | **아이콘 래스터 버킷 = 실제 그리는 크기.** 24px 아이콘을 20 버킷으로 보내 늘리면 흐림(08-11) → 16/20/24/32 버킷, 바 높이와 버킷을 정합(도구 28→20, 런처 24→16) | `dw.rs:933-947` · `win.rs:1851-1857` |
| RENDER-L11 | **얇은 SVG 스트로크는 직접 px 렌더 시 반투명 회색으로 퍼져 다크 배경에 묻힌다** → 4× 슈퍼샘플 후 고품질 축소(07-19) | `ctl/gdipctx.rs:152-181` |
| RENDER-L12 | **다크 전용 에셋의 루트 `stroke`/`fill` 색 상속 누락**으로 흰 외곽선이 잉크색으로 그려지던 결함(07-19) → 요소 미지정 시 루트 상속(채움 요소 = 루트 fill, 스트로크 요소 = 루트 stroke) | `svg.rs:96-100,291-308` |
| RENDER-L13 | **요소 색 오버라이드의 알파는 잉크를 따른다** — 비활성 흐림(38%)이 accent 선에도 적용되어야 함 | `ctl/gdipctx.rs:214-219` · `svg.rs:73-76` |
| RENDER-L14 | **SVG 채움은 짝홀(Alternate).** `dock-dark.svg`가 `fill-rule="evenodd"`로 구멍을 표현 — 파서는 fill-rule을 읽지 않고 렌더러가 항상 짝홀. nexa-sql 래스터의 기본은 비영(nonzero)이므로 그대로 쓰면 구멍이 메워진다 | `ctl/gdipctx.rs:221` · `assets/toolbar/dock-dark.svg` · `nexa-sql/crates/nexa-sql/src/toolicons.rs:764-768` |
| RENDER-L15 | **파일별 아이콘은 반드시 비동기.** Downloads의 대형 exe가 Defender 실시간 검사로 수십 초 블로킹 → UI "응답 없음"(QA 07-14). nexa-fs 실측: `icon_for_kind` ≈ 12ms/확장자 · `icon_for_path` ≈ 17ms — 타입 아이콘도 워커로 | `icons.rs:145-149` · `nexa-fs/src/shell.rs:59-62` |
| RENDER-L16 | **빠른 스크롤 시 셸 호출 폭주 → 크래시**(원본 교훈) → 큐 + 틱당 상한. in-flight 키는 완료까지 재요청 무시 | `icons.rs:1-3,102-118` |
| RENDER-L17 | **1비트 리전 클립은 계단 가장자리** → 부모 배경색(`behind`)으로 모서리를 칠하고 AA 도형을 얹는 우회(07-17). nexa는 SDF 커버리지 진짜 블렌드라 우회 불필요 — `behind` 개념을 옮기지 말 것 | `ctl/style.rs:25-28` · `ctl/iconbutton.rs:12-13,145-146` · `nexa-ctl/src/raster.rs:3-6` |
| RENDER-L18 | **GDI+ 호출은 DrawCtx 구현체에만**(위젯/컨트롤 직접 호출 금지). nexa도 "래스터 호출은 구현체에만" 규약 — 위젯이 `Surface`를 직접 만지지 않게 | `ctl/gdipctx.rs:1-8` · `nexa-ctl/src/draw.rs:8` |
| RENDER-L19 | **폰트 슬롯은 상태 공유** — 위젯은 페인트 시작에 자기 슬롯을 선택하고, 중간에 굵게 등을 켰으면 끝나기 전에 복원(안 하면 다음 텍스트가 굵게) | `nexa-gui/src/draw.rs:22-24` · `rows.rs:1393-1399,2250,2309` |
| RENDER-L20 | **RasterCtx의 scale은 필수 인자** — 빠뜨리면 레이아웃만 2배·글자는 1배(Retina에서 "깨알 글씨"), Windows 100%에서는 우연히 맞아 한쪽 OS에서만 조용히 틀림(08-27) | `nexa-ctl/src/raster.rs:83-92` |
| RENDER-L21 | **Surface 밖에서 시작하는 사각형이 역전 범위로 패닉**했던 회귀(08-10) — 수정됨. dir3의 가로 스크롤/부분 행 그리기가 이 경로를 자주 탐 | `nexa-gfx/src/surface.rs:207-228,523-538` |
| RENDER-L22 | **영문 Windows에서 현지어 글꼴 이름("맑은 고딕")은 GDI가 못 찾는다** → name 테이블의 영문·현지어 후보 전부 시도(CI 실패 교훈) | `nexa-gfx/src/names.rs:1-3` · `gdi.rs:298-301` |
| RENDER-L23 | **Linux 폰트 폴더 걷기 비용** — 항목마다 `is_dir()`(statx)를 부르면 기동 100~700ms. `file_type()` + 프로세스 1회 캐시 | `nexa-font/src/lib.rs:253-297` |
| RENDER-L24 | **고정폭 숫자가 본문보다 커 보임** → 광학 크기 보정(`mono_mult`, 숫자 '0' 높이 비 0.75~1.15) | `nexa-ctl/src/raster.rs:71-73,360-368` |
| RENDER-L25 | **DW 글리프와 GDI 클립의 관계에 대한 주석이 서로 다르다** — `dw.rs:613-614`는 "클립 영역을 따른다", `win.rs:2626-2627`은 "DW 글리프는 GDI 클립 무시(위로 번진 부분은 스트립 재도장으로 덮음)". dir3에서는 클립 스택(N-01)으로 명확히 하고 스트립 재도장 우회(`win.rs:4880-4883`)가 여전히 필요한지 검증 | 좌동 |
| RENDER-L26 | **미로드 아이콘 자리는 비워 두고 폴백을 그린다** — `draw_icon`이 false면 툴바는 라벨 앞 2자, 도크 팝아웃은 `↗`. 목록 행은 폴백 없음(공백 유지) | `chrome.rs:325-337` · `dock.rs:399-410` · `rows.rs:1385` |
| RENDER-L27 | **rc.exe 미발견 = 경고 후 스킵**(빌드 실패 금지). 작업표시줄 고정 시 빈 아이콘 가능성만 안내 | `build.rs:29-35,125-130` |

---

## 7. 회귀 테스트 후보

자동화: **자** = 헤드리스 단위/스냅샷 테스트 가능(3-OS CI) · **반** = OS 자원 의존(해당 OS 러너에서만) · **수** = 육안 확인 필요.

| # | 시나리오 | 기대 | 자동화 | 관련 ID |
| --- | --- | --- | --- | --- |
| T-01 | dir2 `svg.rs` 테스트 13건 이식(실자산 `view-flat.svg` 왕복 포함) | 전부 통과 | 자 | RENDER-034 |
| T-02 | 임베드 SVG 25종 + `assets/ui` 2종 전부 `parse` 성공 | None 0건 | 자 | RENDER-033/034 |
| T-03 | SVG 래스터 스냅샷: 16/20/24/32 버킷 × (기본·`#dis`·`@dark`) | 잉크 픽셀 수 > 임계 · 비활성 알파 ≈ 38% · `dock-dark` 구멍(짝홀) 유지 · 다크 에셋 흰 외곽선 유지 | 자 | RENDER-035 · L12~L14 |
| T-04 | 아이콘 키 해석: `emb:refresh#dark#dis#D6DAE0` × size 20 | 최종 키 `emb:refresh@dark:20:61D6DAE0` · size 22→24 · 28→32 · 17→16 버킷 | 자 | RENDER-032 |
| T-05 | dir2 `icons.rs` 테스트 9건 이식(icon_key 5 · IconStore 4) | 전부 통과 | 자 | RENDER-038/039 |
| T-06 | 테마 값 고정: 다크/라이트 16토큰이 dir2 값과 동일 | 표 §3-2와 일치 | 자 | RENDER-045 |
| T-07 | 끝 말줄임: 폭 초과 텍스트 | 결과 폭 ≤ max_w · `…`로 끝남 · 맞으면 원문 그대로 · 극단 좁음 = `…` | 자(Quirky 목업 `nexa-ctl/src/draw.rs:366-376`) | RENDER-010 · N-02 |
| T-08 | 클립 스택: push 후 `fill_rect`·`fill_round_rect`·`text`가 clip 밖 픽셀 미변경, pop 후 복원, 중첩 교차 | 픽셀 비교 | 자 | RENDER-023 · N-01 |
| T-09 | 음수/빈 rect `fill_rect`·표면 밖 rect | 패닉 없음·픽셀 무변 | 자(기존 `surface.rs:523-552` 확장) | L03 · L21 |
| T-10 | 폰트 체인 파싱 `" D2Coding , JetBrainsMono Nerd Font,, "` | 2항목 · 빈 문자열 = 빈 목록 | 자 | RENDER-009 |
| T-11 | 체인 로더: 미설치 1순위 + 설치된 2순위 | 2순위가 주 face · 전부 미설치 = 시스템 기본 | 반(OS 폰트 의존) | RENDER-009 · N-08 |
| T-12 | UI에 쓰는 기호(`↗ ✕ ▲ ▼ › ∨` 등 dir3 리터럴 전수) 커버 | UI·고정폭 체인 모두 글리프 보유(`UI_SYMBOLS` 갱신) | 반(3-OS) | §3-3 · B-05 |
| T-13 | em→size 변환: 12 DIP 요청 | 래스터된 대문자 높이가 기대 범위(글꼴별 기준값) | 반 | L02 · N-03 |
| T-14 | `text_center_y`: 행 높이 20/22/24/28에서 텍스트 잉크 중심이 행 중심 ±1px | 3-OS 동일 판정 | 반 | L01 |
| T-15 | `text_prefix_widths[i] == text_width(prefix_i)`(한글·탭·폴백 런 포함) | 비트 동일 | 자(기존 `draw.rs:403-415`) + 반(실폰트) | RENDER-012 |
| T-16 | 터미널 셀 그리드: `cell_w` = "0" 전진 폭 ceil · 전각 문자 2셀 · 한글 폴백 문자도 셀 x 고정 | 열 정렬 픽셀 검사(각 셀 잉크가 자기 clip 안) | 반 | RENDER-019~021 · L04 |
| T-17 | DPI 배율 1.0/1.25/1.5/2.0에서 지표 `s(v)`와 글자 크기 동시 스케일 | `RasterCtx.scale()` = 호스트 배율 · 행 높이 20→25→30→40 | 자 | RENDER-005 · L20 |
| T-18 | 배율 변경 이벤트 후 재도장 | 글리프 캐시가 새 크기로 채워지고 레이아웃 재계산 | 자(이벤트 주입) | RENDER-003 |
| T-19 | 이미지 미리보기: PNG/BMP/GIF/JPEG(기저) 디코드 → 비율 유지·확대 없음·가운데 | 대상 rect 안 · 원본보다 크지 않음 · 손상 입력 = 오류(패닉 없음) | 자 | RENDER-028 |
| T-20 | 미지원 포맷(프로그레시브 JPEG·WebP) | 오류 문구 반환·UI 안내 | 자 | B-09 |
| T-21 | 앱 아이콘: ICO 7엔트리 파싱 + 요청 크기 최근접 선택 + PNG 디코드 | 16/32/256 요청에 해당 엔트리 · RGBA 크기 일치 | 자 | RENDER-042 |
| T-22 | PNG 아이콘 버튼 자원 4종 디코드 | 32×32 RGBA · 알파 채널 보존 | 자 | RENDER-044 |
| T-23 | 셸 아이콘 서비스: Pending → Ready 전이·`version()` 증가·상한 축출·실패 기억 | OS 구현 유무와 무관하게 계약 동일(미지원 OS = 즉시 `Ready(None)`) | 자 + 반(Windows `resource_tests` `nexa-fs/src/shell.rs:762-`) | RENDER-040 |
| T-24 | 툴바 상태 색: checked 배경 = chrome_bg와 accent 38% 블렌드(라이트 ≈ `#ABCAF9` · 다크 ≈ `#2A4A7A`) · hover = sel_bg · 비활성 글리프 = text_dim | 픽셀 색 검사 | 자 | RENDER-037 |
| T-25 | 아이콘 미로드 폴백: 툴바 = 라벨 앞 2자, 팝아웃 = `↗` | 스냅샷 | 자 | L26 |
| T-26 | 폰트 장식 누수: 폴더 굵게 행 다음 행이 보통체 | 폭 측정값 비교 | 자 | L19 |
| T-27 | 라운드 외곽선 1px가 rect 밖으로 번지지 않음(dir2 인셋 규약 재현) | rect 밖 픽셀 무변 | 자 | RENDER-026 |
| T-28 | Windows GDI/맥 CoreText 경로: 세로 줄기 가시성·볼드 잉크·전진 폭 | 기존 nexa-font 테스트 유지 | 반 | B-02 |
| T-29 | 실기 육안: 다크/라이트 × 100/150/200% × 3-OS에서 목록·툴바·탭·상태바 스크린샷을 dir2와 나란히 비교 | 배치·색·세로 정렬 일치 | 수 | 전체 |
| T-30 | 유휴 트림 후 첫 페인트 | 캐시 재적재로 화면 동일 · 패닉 없음 | 자(캐시 clear 후 스냅샷 동일) | RENDER-006 |

**핵심 점검 로직(기동 자가 진단) 후보**: ① UI·고정폭 체인 로드 성공 + `UI_SYMBOLS` 커버 ② 임베드 SVG 전수 파싱 ③ 앱 아이콘 디코드 ④ `RasterCtx.scale()` == 창 배율 ⑤ 테마 토큰 전부 설정됨 — 실패 시 로그 + 상태바 경고.
