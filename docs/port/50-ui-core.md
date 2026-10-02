# 50 · nexa-ui API 카탈로그 — 기반(그리기·기하·이벤트·위젯 계약·토큰·gfx·sys·conf) (UIC)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 기준일 2026-10-03 · 대상 = nexa-ui `df75f5a`(102차 · 원복 태그 `baseline/pre-nexa-dir3-2026-10-03` 존재).
> 목적: nexa-dir3가 **컨트롤을 전부 nexa-ui 것으로** 쓰기 위해, 컨트롤 아래에 깔린 "기반" 공개 API를 전수 목록화하고 "새 컨트롤을 추가하는 방법"을 고정한다.
> 표기: 근거는 `저장소/경로:줄`. 확인하지 못한 것은 **추정**으로 적는다. ID = `UIC-NNN`(고정 · 재번호 금지 · 결번 허용) — 이후 구현·교차 검증의 체크리스트.
> 개별 컨트롤(Button·TextBox·Tree·TabBar·ContextMenu…)의 API는 이 문서 범위가 아니다(별도 컨트롤 카탈로그 문서). 여기서는 `controls/mod.rs`의 공용부와 `glyphs`·`scroll`·`splitter`·`flash`만 다룬다.

**경로 약어**(전부 `저장소/경로:줄`의 축약)

| 약어 | 실제 경로 |
|---|---|
| `C/` | `nexa-ui/crates/nexa-ctl/src/` |
| `G/` | `nexa-ui/crates/nexa-gfx/src/` |
| `S/` | `nexa-ui/crates/nexa-sys/src/` |
| `F/` | `nexa-ui/crates/nexa-conf/src/` |
| `D/` | `nexa-ui/docs/` |

---

## 0. 범위

### 0-1. 읽은 파일(전부 끝까지 읽음 — 예외는 표에 명시)

| 파일 | 줄 | 내용 | 읽은 정도 |
|---|---:|---|---|
| `C/lib.rs` | 80 | 모듈 선언 · 재수출 색인 · 크레이트 불변식 | 전부 |
| `C/draw.rs` | 416 | `DrawCtx` 트레이트 · `FontSlot` · 툴팁 · 가운데 축약 | 전부 |
| `C/event.rs` | 164 | `InputEvent` · `Key` · `WheelAccum` | 전부 |
| `C/geom.rs` | 306 | `Point`·`Size`·`Rect` · 팝업 배치 4함수 | 전부 |
| `C/widget.rs` | 81 | `Widget` 트레이트 · `Invalidations` | 전부 |
| `C/raster.rs` | 804 | `RasterCtx`(= `DrawCtx`의 CPU 구현) · `FontSet` | 전부 |
| `C/theme.rs` | 401 | `Theme` · 색 유틸 · `SlotFont`·`FontPrefs` | 전부 |
| `C/tokens.rs` | 1200 | 간격·반경·타입 스케일·상태 레이어·엘리베이션·모션·페이드·의도 부품 | 전부 |
| `C/shape.rs` | 174 | 점-도형 판정 11함수 | 전부 |
| `C/typeahead.rs` | 471 | 타입어헤드 버퍼·필터·HUD | 전부 |
| `C/view_mode.rs` | 97 | `ViewMode` | 전부 |
| `C/hangul.rs` | 441 | 두벌식 조합기 | 본문 전부 · 시험(322~441)은 이름만 |
| `C/highlight.rs` | 523 | 구문 강조 포트·규격·HTML | 본문 전부 · `SQL_SPEC` 키워드 목록(441~)·시험은 이름만 |
| `C/avatar.rs` | 97 | 이니셜 아바타 | 전부 |
| `C/controls/mod.rs` | 817 | 공용 헬퍼 · `ControlBase`/`Control` · 라벨 주입 · 크기 배율 · `ProbeCtx` | 전부 |
| `C/controls/glyphs.rs` | 254 | 코드 도형 아이콘 13종 | 전부 |
| `C/controls/scroll.rs` | 1175 | `ScrollBars`·`FastScroll`·`ScrollAccel`·`SpeedHud` | 본문(1~699) 전부 · 시험(701~1175)은 이름만 |
| `C/controls/splitter.rs` | 216 | `Splitter` | 전부 |
| `C/controls/flash.rs` | 213 | `Flash` | 전부 |
| `C/controls/switch.rs` | 284 | (범위 밖 · **새 컨트롤의 본보기**로만 읽음) | 전부 |
| `G/lib.rs` | 19 | 모듈·재수출 | 전부 |
| `G/surface.rs` | 590 | `Color`·`IconImage`·`Surface` | 전부 |
| `G/text.rs` | 1009 | `Font`·`TextStyle`·전역 텍스트 스위치 | 전부 |
| `G/image.rs` | 742 | PNG·BMP·GIF 디코더 · 판별 | 공개부(1~117) 전부 · 디코더 내부(119~606)는 공개 표면만(grep) |
| `G/jpeg.rs` | 588 | 기저 JPEG 디코더 | 모듈 문서 + 공개 시그니처(9·252) · 내부는 안 읽음 |
| `G/inflate.rs` | 350 | DEFLATE/zlib | 공개부(1~71) 전부 · 내부는 안 읽음 |
| `G/gdi.rs` | 446 | Windows GDI ClearType 글리프 원천(`pub(crate)`) | 모듈 문서 + `pub(crate)` 표면(grep) |
| `G/coretext.rs` | 485 | macOS CoreText 글리프 원천(`pub(crate)`) | 모듈 문서 + `pub(crate)` 표면(grep) |
| `G/names.rs` | 95 | 글꼴 `name` 테이블 패밀리 이름(`pub(crate)`) | 모듈 문서 + 표면(grep) |
| `S/lib.rs` | 384 | OS 신호 4종 | 전부 |
| `S/input_source.rs` | 161 | 한글 입력 소스 신호 | 전부 |
| `S/layer_present.rs` | 529 | macOS IOSurface present | 1~180·280~380·520~529 + 공개 표면(grep) |
| `F/lib.rs` | 599 | 설정 포맷·원자적 쓰기·저장 스케줄 | 전부 |
| `D/01-architecture.md` · `D/10-decision-record.md` · `D/STATUS.md` · `D/TODO.md` | 29·36·117·28 | 계층 · 결정 · 현황 · 백로그 | 전부 |
| `nexa-ui/CLAUDE.md` · `nexa-ui/scripts/check-3os.sh` · `nexa-ui/.github/workflows/ci.yml` · 각 `Cargo.toml` · `rust-toolchain.toml` | — | 작업 규약 · 검사 · 의존 | 전부 |

### 0-2. 범위 밖(이 문서가 다루지 않는 것 — 다른 카탈로그 문서 몫)

- `C/edit.rs`·`C/edit/{ops,textbuf}.rs`·`C/merge3.rs`·`C/gridedit/*`(`C/lib.rs:33,36,70`) — 편집기 코어 · 그리드 편집.
- `C/controls/`의 개별 컨트롤 22개 파일(button·carousel·checkbox·colorpanel·colorpick·combo·ctxmenu·editmenu·icondrop·listedit·pairs·posdrop·posgrid·pulldown·radio·switch·tabbar·textbox·timeout_button·toolbar·tooldock·tree — `C/controls/mod.rs:16-40,64`).
- `nexa-ui/crates/nexa-font`·`nexa-fs`·`nexa-dlg`(워크스페이스 구성원 — `nexa-ui/Cargo.toml:9-17`). 단, `RasterCtx`가 요구하는 `Font`의 공급자가 `nexa-font`이므로 §3-5 사용 예에서만 언급한다.

### 0-3. 크레이트 지도(의존 방향)

```
소비 앱(nexa-dir3)  — 창·OS 이벤트·클립보드·폰트 경로·i18n 문자열은 앱이 소유   (D/01-architecture.md:8-9)
   ▼
nexa-ctl   DrawCtx · Widget · Control/ControlBase · controls/* · tokens      의존 = nexa-gfx 1개  (nexa-ui/crates/nexa-ctl/Cargo.toml:16-17)
   ▼          raster 어댑터 안에서만 nexa-gfx 타입을 만진다(C/lib.rs:25-26)
nexa-gfx   Surface · Font/TextStyle · image/jpeg/inflate                       의존 = ab_glyph 0.2  (nexa-ui/crates/nexa-gfx/Cargo.toml:14-15)
nexa-conf  (독립) 설정 포맷·원자적 쓰기·저장 스케줄                             의존 0
nexa-sys   (독립) OS 신호·입력 소스·macOS present                               의존 0 · feature `gui`(기본 on)
```

---

## 1. API 표

> 열 = ID | 타입/함수 | 시그니처 요약 | 용도 | 위치. "기본 구현" = 트레이트가 제공해 백엔드가 구현하지 않아도 되는 메서드.

### 1-1. nexa-ctl 크레이트 수준

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-001 | 모듈 목록 | `avatar controls draw edit event geom gridedit hangul highlight raster shape theme tokens typeahead view_mode widget merge3` | 공개 모듈 17개 | `C/lib.rs:30-45,70` |
| UIC-002 | 루트 재수출 | `pub use` — DrawCtx·FontSlot·InputEvent·Key·WheelAccum·WHEEL_DELTA·Point·Rect·Size·RasterCtx·FontSet·Theme·Color·FontPrefs·SlotFont·IconImage·Widget·Invalidations·토큰(`motion radius space type_scale Elevation LatestIntent State`)·TypeAhead 계열·ViewMode + 컨트롤 타입 다수 | `nexa_ctl::X`로 바로 쓰는 색인. **주의**: `Fade`·`HoverFade`·`IntentFade`·`HoverIntent`·`FadeSpeed`·`Splitter`·`SplitAxis`·`SplitEvent`·`ContextMenu`·`GlyphKind`·`glyph`·`ProbeCtx`는 루트 재수출에 **없다** → `nexa_ctl::tokens::…` / `nexa_ctl::controls::…` 경로로 쓴다 | `C/lib.rs:47-80` |
| UIC-003 | 크레이트 불변식 | — | ① 앱 도메인 의존 0(문자열·크기 배율은 주입) ② 호스트(창·OS·클립보드·i18n)를 모른다 — 컨트롤은 요청만 남기고 실행은 호스트 ③ 공개 시그니처에 외부 크레이트 타입 0(예외: `Color`·`IconImage`는 `nexa_gfx` 것을 재수출 — UIC-090) | `C/lib.rs:3-26` · `D/10-decision-record.md:10-11` |
| UIC-004 | lint 정책 | `missing_debug_implementations`·`unreachable_pub`·`clippy::unwrap_used` 등 = warn(CI는 `-D warnings`) · 시험은 `allow(clippy::unwrap_used)` | 새 공개 타입은 `Debug` 필요(또는 `#[allow]` + 사유 주석) | `nexa-ui/Cargo.toml:29-36` · `C/lib.rs:28` |

### 1-2. geom.rs — 기하·팝업 배치

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-010 | `Point` | `struct { x: i32, y: i32 }` · `Copy Eq Default` | 창 클라이언트 좌표(좌상 원점 · px) | `C/geom.rs:7-12` |
| UIC-011 | `Size` | `struct { w: i32, h: i32 }` | 크기 | `C/geom.rs:16-21` |
| UIC-012 | `Rect` | `struct { x, y, w, h: i32 }` · `const fn new` · `right()` `bottom()`(exclusive) · `size()` · `is_empty()`(w≤0 또는 h≤0) · `contains(Point)`(반열림) · `intersects(&Rect)`(변 접촉 = 비교차) · `intersection(&Rect)` · `union(&Rect)`(빈 rect = 항등원) | 모든 배치·클립·히트 테스트의 단위 | `C/geom.rs:25-117` |
| UIC-013 | `place_popup` | `fn(anchor: Point, size: (i32,i32), host: Rect) -> Point` | 팝업 배치 규칙: 축마다 정방향 → 반대쪽 → 밀어 넣기 | `C/geom.rs:124-139` |
| UIC-014 | `place_popup_beside` | `fn(anchor: Point, avoid: Rect, size, host: Rect) -> Point` | 대상 행을 가리지 않는 배치(행 바로 아래 → 바로 위 → 일반 규칙) | `C/geom.rs:145-157` |
| UIC-015 | `nudge_into` | `fn(r: Rect, host: Rect) -> Rect` | 이미 놓인 팝업을 host 안으로 옮김(크기 유지) | `C/geom.rs:161-169` |
| UIC-016 | `popup_host` | `fn(host: Rect, surface: Option<(i32,i32)>) -> Rect` | 호출자 host와 그리기 표면의 겹침(끝없는 host 방지) | `C/geom.rs:173-185` |

### 1-3. event.rs — 입력 이벤트

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-020 | `WHEEL_DELTA` | `const i32 = 120` | 휠 1노치 단위(플랫폼 어댑터가 정규화) | `C/event.rs:8` |
| UIC-021 | `Key` | enum 16종: `Up Down PageUp PageDown Home End Right Left Space Enter Escape Delete WordLeft WordRight SubwordLeft SubwordRight` | 네비게이션 키. Backspace는 `Char('\u{8}')`로 온다. 단어/서브워드 이동은 호스트가 수식키를 번역해 보낸다(Win/Linux Ctrl+← · mac ⌥←) | `C/event.rs:12-45` |
| UIC-022 | `InputEvent` | enum 11종: `Wheel{delta}` `HWheel{delta}` `Key{key,shift,primary}` `Char{c,now_ms}` `SelectAll` `Undo` `Redo` `MouseDown{x,y,shift,primary}` `RightDown{x,y}` `MouseMove{x,y}` `MouseUp{x,y}` · `Copy Eq` | 위젯이 받는 유일한 입력 타입. `primary` = OS 주 수식키(mac ⌘ / 그 외 Ctrl). `now_ms` = 단조 시각 주입 | `C/event.rs:49-116` |
| UIC-023 | `WheelAccum` | `struct`(Default) · `add(&mut self, delta: i32, lines_per_notch: i32) -> i32` | 분수 노치(트랙패드) 잔여 이월 → 스크롤할 행 수 | `C/event.rs:120-134` |

### 1-4. widget.rs — 위젯 계약

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-030 | `Invalidations` | `struct`(Default) · `push(Rect)`(빈 rect 무시 · 교차분 union 병합) · `is_empty()` · `drain() -> impl Iterator<Item = Rect>` | 프레임당 더러워진 영역 수집기 | `C/widget.rs:14-41` |
| UIC-031 | `Widget` | `trait { fn bounds(&self) -> Rect; fn set_bounds(&mut self, Rect, &mut Invalidations); fn on_event(&mut self, &InputEvent, &mut Invalidations); fn paint(&self, &mut dyn DrawCtx, &Theme); }` | 논리 위젯 계약(OS 자식 창 없음). 4메서드 전부 필수 · 객체 안전(dyn 가능) | `C/widget.rs:44-56` |

### 1-5. draw.rs — 그리기 어휘

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-040 | `FontSlot` | enum `Base`(기본) `PeerList` `Message` `Status` `Mono` | 글꼴 슬롯. 이름은 메신저(beep) 유래 — 탐색기용 `List` 슬롯은 없다(§5-6 UIC-315) | `C/draw.rs:16-28` |
| UIC-041 | `DrawCtx` | trait — **필수 4개**: `fill_rect` `text_opaque` `text` `text_width` · 나머지 24개는 기본 구현(no-op 또는 폴백) | 위젯의 유일한 그리기 통로. 래스터 호출은 구현체에만 | `C/draw.rs:31-222` |
| UIC-042 | `DrawCtx::surface_size` | `fn(&self) -> Option<(i32,i32)>` · 기본 `None` | 팝업이 표면 밖으로 안 나가게 하는 안전망의 근거 | `C/draw.rs:37-39` |
| UIC-043 | `DrawCtx::caret_on` | `fn(&self) -> bool` · 기본 `true` | 이번 프레임에 캐럿을 그릴지(깜빡임 위상 — 호스트 주입) | `C/draw.rs:41-43` |
| UIC-044 | `DrawCtx::select_font` | `fn(&mut self, slot: FontSlot, bold: bool)` · 기본 no-op | 이후 `text*`/`text_width`에 적용. **italic 인자 없음** | `C/draw.rs:45-47` |
| UIC-045 | `DrawCtx::select_font_sized` | `fn(&mut self, slot, bold, delta_px: f32)` · 기본 = 증분 무시 | 슬롯 크기 + **논리 px 증분**(절대 크기 금지) | `C/draw.rs:52-55` |
| UIC-046 | `DrawCtx::fill_rect` | `fn(&mut self, rect: Rect, color: Color)` · **필수** | 불투명 단색 채움 | `C/draw.rs:58` |
| UIC-047 | `DrawCtx::text_opaque` | `fn(&mut self, x, y, clip: Rect, text: &str, fg, bg)` · **필수** | `clip`을 `bg`로 채우며 `(x,y)`(왼쪽 위)에 글 — 행 배경+글 1회 호출 | `C/draw.rs:62` |
| UIC-048 | `DrawCtx::text` | `fn(&mut self, x, y, clip: Rect, text: &str, fg)` · **필수** | 배경 없이 글만(선택 하이라이트 위) | `C/draw.rs:65` |
| UIC-049 | `DrawCtx::text_width` | `fn(&mut self, text: &str) -> i32` · **필수** | 렌더 폭(px) | `C/draw.rs:68` |
| UIC-050 | `DrawCtx::set_tab_origin` | `fn(&mut self, x: Option<i32>)` · 기본 no-op | 탭 정지점 원점(줄 시작 x) — 여러 색 구간으로 나눠 그릴 때 | `C/draw.rs:73-75` |
| UIC-051 | `DrawCtx::text_prefix_widths` | `fn(&mut self, text: &str, out: &mut Vec<i32>)` · 기본 = O(n²) 접두사 재측정 | 문자 경계 누적 폭. **계약: `out[i]` == `text_width(앞 i글자)` · `out[0]`=0 · 길이=문자수+1** | `C/draw.rs:82-90` |
| UIC-052 | `DrawCtx::text_height` | `fn(&mut self) -> i32` · 기본 16 | 현재 글꼴 상자 높이(어센트+디센트 · **물리 px**) | `C/draw.rs:94-96` |
| UIC-053 | `DrawCtx::text_ascent` | `fn(&mut self) -> i32` · 기본 = 높이×3/4 | 줄 상단→기준선 | `C/draw.rs:100-102` |
| UIC-054 | `DrawCtx::text_center_y` | `fn(&mut self, y: i32, h: i32) -> i32` · 기본 = 상자 가운데 | 높이 `h` 행에서 **잉크 기준** 세로 가운데가 되는 텍스트 top. 한 줄 라벨은 이걸로 놓는다 | `C/draw.rs:109-111` |
| UIC-055 | `DrawCtx::fill_triangle` | `fn(&mut self, a, b, c: (i32,i32), color)` · 기본 no-op | AA 삼각형 | `C/draw.rs:114-116` |
| UIC-056 | `DrawCtx::image` | `fn(&mut self, x, y, img: &IconImage, clip: Rect)` · 기본 no-op | RGBA 이미지 1:1 알파 블렌드 | `C/draw.rs:119-121` |
| UIC-057 | `DrawCtx::image_scaled` | `fn(&mut self, dst: Rect, img: &IconImage, clip: Rect)` · 기본 no-op | 스케일 블렌드(bilinear) | `C/draw.rs:125-127` |
| UIC-058 | `DrawCtx::fill_ellipse` | `fn(&mut self, rect, color)` · 기본 no-op | AA 타원 | `C/draw.rs:130-132` |
| UIC-059 | `DrawCtx::stroke_ellipse` | `fn(&mut self, rect, color, width: f32)` · 기본 no-op | 타원 테두리 링(안쪽 밴드) | `C/draw.rs:136-138` |
| UIC-060 | `DrawCtx::fill_pie` | `fn(&mut self, rect, start_deg: f32, sweep_deg: f32, color)` · 기본 no-op | 부채꼴(12시 = 0° · 시계 방향 · **sweep ≤ 180°만 보증**) | `C/draw.rs:143-145` |
| UIC-061 | `DrawCtx::fill_round_rect` | `fn(&mut self, rect, radius: i32, color)` · 기본 no-op | AA 라운드 사각형 | `C/draw.rs:148-150` |
| UIC-062 | `DrawCtx::state_layer` | `fn(&mut self, rect, color, state: tokens::State)` · 기본 = `fill_rect_alpha(overlay_alpha)` | hover·press·selected를 알파 오버레이로 | `C/draw.rs:155-160` |
| UIC-063 | `DrawCtx::shadow` | `fn(&mut self, rect, color, level: Elevation, radius: i32)` · 기본 = 두 겹 `fill_round_rect_alpha` | 엘리베이션 그림자 — 본체보다 **먼저** 부른다 | `C/draw.rs:165-175` |
| UIC-064 | `DrawCtx::fill_rect_alpha` | `fn(&mut self, rect, color, alpha: f32)` · 기본 = 불투명 폴백 | 반투명 사각형(`alpha` 0..=1) | `C/draw.rs:181-184` |
| UIC-065 | `DrawCtx::fill_round_rect_alpha` | `fn(&mut self, rect, radius, color, alpha: f32)` · 기본 = 알파 무시 | 반투명 라운드(스크롤바 썸 등) | `C/draw.rs:188-191` |
| UIC-066 | `DrawCtx::stroke_round_rect` | `fn(&mut self, rect, radius, color, width: f32)` · 기본 no-op | AA 라운드 외곽선 | `C/draw.rs:194-196` |
| UIC-067 | `DrawCtx::stroke_round_rect_alpha` | `fn(&mut self, rect, radius, color, width: f32, alpha: f32)` · 기본 = 알파 무시 | 포커스 링 | `C/draw.rs:200-210` |
| UIC-068 | `DrawCtx::polyline` | `fn(&mut self, pts: &[(i32,i32)], color, width: f32)` · 기본 no-op | 꺾은선(✓·셰브론 · 둥근 캡) | `C/draw.rs:213-215` |
| UIC-069 | `DrawCtx::polyline_clipped` | `fn(&mut self, pts, color, width, clip: Rect)` · 기본 = 클립 없이 | 부분적으로 잘린 행의 셰브론 | `C/draw.rs:218-221` |
| UIC-070 | `draw_tooltip` | `fn(ctx: &mut dyn DrawCtx, theme: &Theme, anchor: Rect, clamp_w: i32, text: &str, scale: f32)` | 공용 툴팁(anchor 아래 6px · 역상 캡슐). **팝업 층에서** 호출 | `C/draw.rs:227-236` |
| UIC-071 | `draw_tooltip_in` | 같되 `clamp_x: (i32,i32)` | 가로 클램프를 `(x0,x1)`로 · 여러 줄(`\n`) · 아래로 넘치면 anchor 위로 · `FontSlot::Status` | `C/draw.rs:240-290` |
| UIC-072 | `set_show_full` / `show_full` | `fn(on: bool) -> bool`(바뀌었으면 true) / `fn() -> bool` | 전체 경로 보기 전역 스위치(호스트가 Alt 사건에서 켬/끔) | `C/draw.rs:294-305` |
| UIC-073 | `ellipsize_middle` | `fn(ctx: &mut dyn DrawCtx, text: &str, max_w: i32) -> String` | 가운데 `…` 축약(앞 ≈ 뒤 · 지금 글꼴로 실측) · `show_full()`이면 원문 | `C/draw.rs:309-358` |

### 1-6. raster.rs — CPU 백엔드

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-080 | `FontSet<'f>` | `struct { base: &Font, peerlist, message, status, mono: Option<&Font> }` · `single(base)` · `face(slot) -> &Font`(없으면 base) | 슬롯별 글꼴 **얼굴** 묶음 | `C/raster.rs:19-57` |
| UIC-081 | `RasterCtx::new` | `fn(surface: &'s mut Surface<'b>, font: &'f Font, scale: f32) -> Self` | 표면+글꼴+**배율(필수 인자)** 로 컨텍스트. 배율 < 0.5는 0.5로 | `C/raster.rs:90-92,104` |
| UIC-082 | `RasterCtx::with_font_set` | `fn(surface, fonts: FontSet<'f>, scale: f32) -> Self` | 슬롯별 얼굴을 가진 컨텍스트 | `C/raster.rs:95-109` |
| UIC-083 | `RasterCtx::with_caret_on` | `fn(self, on: bool) -> Self` | 캐럿 깜빡임 위상 주입 | `C/raster.rs:113-116` |
| UIC-084 | `RasterCtx::with_fonts` | `fn(self, prefs: FontPrefs) -> Self` | 슬롯별 크기·굵기 설정 | `C/raster.rs:120-124` |
| UIC-085 | `RasterCtx::set_fonts` | `fn(&mut self, prefs: FontPrefs)` | 그리는 중간에 설정 교체(선택은 Base로 복귀) | `C/raster.rs:128-132` |
| UIC-086 | `RasterCtx::scale` | `fn(&self) -> f32` | 지금 배율 | `C/raster.rs:136-138` |
| UIC-087 | `impl DrawCtx for RasterCtx` | 28개 메서드 전부 구현 | 실 동작 규칙은 §3-2 | `C/raster.rs:339-694` |
| UIC-088 | 라운드 사각형 영역 분해 | (비공개) `rr_fill`·`rr_stroke` | 안쪽 = 단색 띠 · SDF는 모서리만 — 전면 SDF와 픽셀 동일(시험 `region_split_matches_full_sdf`) | `C/raster.rs:236-295,769` |

### 1-7. theme.rs — 테마

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-090 | `Color` / `IconImage` 재수출 | `pub use nexa_gfx::{Color, IconImage}` | nexa-ctl 사용자는 nexa-gfx를 직접 의존하지 않아도 된다 | `C/theme.rs:7` |
| UIC-091 | `Theme` | `struct`(Copy Eq) 필드 22: `window_bg chrome_bg panel_bg panel_bg_alt bubble_peer field_bg border accent focus_ring text text_dim sel_bg sel_bg_inactive syn_keyword syn_string syn_comment syn_number rainbow:[Color;6] danger ok warn is_dark` | 시맨틱 색 토큰. 색 하드코딩 금지 — 전 위젯이 테마를 인자로 받는다 | `C/theme.rs:12-57` |
| UIC-092 | `Theme::dark` / `Theme::light` | `const fn() -> Theme` · `Default` = dark | 내장 팔레트 2종(주석상 "임시") | `C/theme.rs:62-137` |
| UIC-093 | `color_from_hex` | `fn(&str) -> Option<Color>` | `#RRGGBB`/`RRGGBB` 파싱 | `C/theme.rs:141-147` |
| UIC-094 | `color_to_hex` | `fn(Color) -> String` | `#RRGGBB`(대문자) | `C/theme.rs:151-153` |
| UIC-095 | `is_warm` | `fn(Color) -> bool` | 따뜻한 색(색상각 <90° 또는 ≥300°) | `C/theme.rs:180-183` |
| UIC-096 | `color_contrast` | `fn(a: Color, b: Color) -> f32`(0~1) | 나란히 놓였을 때 구별 정도 | `C/theme.rs:188-195` |
| UIC-097 | `contrast_order` | `fn(&[Color]) -> Vec<Color>` | 순환 팔레트를 이웃끼리 잘 구별되게 재배열(첫 색 고정) | `C/theme.rs:202-263` |
| UIC-098 | `SlotFont` | `struct { size: f32, bold: bool, italic: bool }` · `const fn plain(size)` | 슬롯 하나의 크기(논리 px)·faux 굵기/기울임 | `C/theme.rs:331-350` |
| UIC-099 | `FontPrefs` | `struct { base, peerlist, message, status: SlotFont }` · `with_base(px)` · `with_base_status(px)` · `Default` = 16/16/18/15 | 영역별 글꼴 설정 | `C/theme.rs:354-401` |

### 1-8. tokens.rs — 디자인 토큰·모션 부품

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-100 | `space` | `XS=4 S=8 M=12 L=16 XL=24 XXL=32` · `const fn snap(i32) -> i32` | 간격 4px 그리드 | `C/tokens.rs:14-33` |
| UIC-101 | `radius` | `WINDOW=12 PANEL=10 CONTROL=6 PILL=999` | 코너 반경 | `C/tokens.rs:36-45` |
| UIC-102 | `type_scale` | `TITLE=(15,true) BODY=(13,false) LABEL=(12,false) CAPTION=(11,false) MONO=(12.5,false)` · `line_height(f32) -> i32` | 타입 스케일(1.0배 기준) | `C/tokens.rs:48-65` |
| UIC-103 | `State` | enum `Rest Hover Pressed Selected SelectedHover Disabled` · `overlay_alpha()` = 0/0.08/0.12/0.16/0.20/0 · `content_alpha()`(Disabled 0.38) · `of(selected, hover, pressed, enabled)` | 상태 레이어 | `C/tokens.rs:73-130` |
| UIC-104 | `Elevation` / `ShadowLayer` | enum `Flat Low Mid High` · `layers() -> &'static [(dy, spread, alpha)]` | 두 겹 그림자 | `C/tokens.rs:136-162` |
| UIC-105 | `motion` | `STATE_MS=90 POPUP_MS=120 PANEL_MS=160 HOVER_IN_MS=1000 HOVER_OUT_MS=220` · `const fn effective(ms, reduce_motion) -> u32` | 모션 지속 상수 | `C/tokens.rs:165-196` |
| UIC-106 | `set_hover_in_ms` / `hover_in_ms` | `fn(u32)` / `fn() -> u32` | 행 hover 진입 시간(전역 · 기본 1000) | `C/tokens.rs:204-212` |
| UIC-107 | `set_button_hover_in_ms` / `button_hover_in_ms` | 같음 | 버튼 hover 진입(전역 · 기본 500) | `C/tokens.rs:215-226` |
| UIC-108 | `set_hover_color` / `set_pressed_color` / `hover_color` / `pressed_color` | `fn(Option<u32>)`(0xRRGGBBAA) / `fn(&Theme) -> (Color, f32)` | hover·눌림 색 오버라이드(미설정 = `(theme.sel_bg, 1.0)`) | `C/tokens.rs:230-263` |
| UIC-109 | `set_fade_out_ms` / `fade_out_ms` | `fn(u32)` / `fn() -> u32` | hover 나감 시간(전역 · 기본 220) | `C/tokens.rs:266-276` |
| UIC-110 | `set_intent_ms` / `intent_ms` | `fn(u64)` / `fn() -> u64` | hover 의도 판정 시간(전역 · 기본 70) | `C/tokens.rs:279-288` |
| UIC-111 | `FadeSpeed` · `set_fade_ms` · `fade_ms` | enum `Fast`(기본) `Slow` · `in_ms()` · `fn(FadeSpeed, u32)` · `fn(FadeSpeed) -> u32` | 컨트롤은 속도 이름만 알고 실제 ms는 전역 설정 | `C/tokens.rs:294-323` |
| UIC-112 | `hover_alpha` | `fn(selected: bool, progress: f32) -> f32` | hover 오버레이 알파의 단일 원천(선택 행은 차이분만) | `C/tokens.rs:332-339` |
| UIC-113 | `Fade` | `const fn new(in_ms, out_ms)` · `hover()` · `button_hover()` · `at(FadeSpeed)` · `set(bool)` · `jump(bool)` · `value() -> f32` · `is_animating()` · `tick(now_ms: u64) -> bool` | 비대칭(느리게 켜짐·빨리 꺼짐) 0..1 값. 첫 틱은 기준 시각만 잡는다 | `C/tokens.rs:357-451` |
| UIC-114 | `HoverFade` | `with_speed(FadeSpeed)` · `button()` · `set_speed` · `speed` · `jump(Option<usize>)` · `set(Option<usize>)` · `current()` · `value(idx)` · `is_animating()` · `tick(now_ms)` · Default = Slow | "한 번에 하나만 hover" — 항목 수와 무관하게 Fade 2개 | `C/tokens.rs:459-568` |
| UIC-115 | `IntentFade` | `new(HoverFade)` · `with_speed` · `rows()` · `buttons()` · `set_speed`/`speed` · `set(Option<usize>)` · `jump` · `tick(now_ms) -> bool` · `value(idx)` · `current()` · `is_animating()` · `INTENT_MS=70` | 의도 코얼레싱 + 페이드(지나간 목표는 페이드에 닿지 않음) — 목록·트리 행 hover의 표준 부품 | `C/tokens.rs:581-695` |
| UIC-116 | `overlay_color` | `fn(&Theme, on_accent: bool) -> Color` | 오버레이 색 = accent 계열이면 accent, 아니면 text | `C/tokens.rs:701-707` |
| UIC-117 | `HoverIntent<T: Copy + PartialEq>` | `set(target, now_ms)` · `settle(now_ms)` · `take_due(now_ms) -> Option<T>` · `clear()` · `is_waiting(now_ms)` · `INTENT_MS=70 SETTLE_MS=120` | 큐 없는 "마지막 의도 1개"(hover 미리보기 등 · dir2 20 승계라고 주석) | `C/tokens.rs:718-775` |
| UIC-118 | `LatestIntent<T: PartialEq + Clone>` | `new(delay_ms)` · `set_delay_ms` · `set(Option<T>)` · `clear()` · `restart()` · `tick(now_ms) -> bool` · `settled() -> Option<&T>` · `target()` | 최신 의도 슬롯(툴팁·검색어 확정 등 지연 동작) | `C/tokens.rs:785-870` |

### 1-9. shape.rs — 점-도형 판정(아이콘 래스터용)

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-120 | `seg_dist` | `fn(x, y, ax, ay, bx, by: f32) -> f32` | 점-선분 거리 | `C/shape.rs:9-15` |
| UIC-121 | `stroke` | `fn(x, y, a: (f32,f32), b: (f32,f32), w: f32) -> bool` | 폭 `w` 획 안인가 | `C/shape.rs:20-22` |
| UIC-122 | `rect` | `fn(x, y, x0, x1, y0, y1) -> bool` | 축 정렬 사각형(**인자 순서 x0,x1,y0,y1**) | `C/shape.rs:27-29` |
| UIC-123 | `rrect` | `fn(x, y, x0, y0, w, h, r) -> bool` | 둥근 사각형 | `C/shape.rs:34-41` |
| UIC-124 | `disc` | `fn(x, y, cx, cy, r) -> bool` | 원판 | `C/shape.rs:46-48` |
| UIC-125 | `ring` | `fn(x, y, cx, cy, r_in, r_out) -> bool` | 고리 | `C/shape.rs:53-56` |
| UIC-126 | `ellipse` | `fn(x, y, cx, cy, rx, ry) -> bool` | 타원 | `C/shape.rs:61-64` |
| UIC-127 | `tri` | `fn(x, y, a, b, c: (f32,f32)) -> bool` | 삼각형(방향 무관) | `C/shape.rs:69-75` |
| UIC-128 | `poly` | `fn(x, y, pts: &[(f32,f32)]) -> bool` | 다각형(짝홀) | `C/shape.rs:80-93` |
| UIC-129 | `polys_evenodd` | `fn(x, y, polys: &[Vec<(f32,f32)>]) -> bool` | 여러 다각형 짝홀(구멍) | `C/shape.rs:98-114` |
| UIC-130 | `polys_nonzero` | 같음 | SVG `nonzero` 규칙 — **SVG 경로를 다각형으로 편 뒤 판정할 때의 바탕** | `C/shape.rs:119-139` |

### 1-10. typeahead.rs · hangul.rs · view_mode.rs · highlight.rs · avatar.rs

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-135 | `TYPEAHEAD_TIMEOUT_MS` | `const u64 = 2000` | 타입어헤드 기본 타임아웃 | `C/typeahead.rs:20` |
| UIC-136 | `Query` | `struct { prefix: String, include_caret: bool }` | 입력 결과(`include_caret` = 접두사 확장이면 지금 행 포함 재평가 · 새 입력이면 다음 행부터) | `C/typeahead.rs:24-29` |
| UIC-137 | `TypeAhead` | `new(timeout_ms)` · `text()` · `composing() -> String` · `is_active()` · `set_timeout(ms)` · `touch(now_ms)` · `clear()` · `push(c, now_ms) -> Query` · `backspace(now_ms) -> Option<Query>` · `tick(now_ms) -> bool` | 버퍼 + 한글 조합기 + 타임아웃(매칭은 안 함) | `C/typeahead.rs:33-143` |
| UIC-138 | `TypeAheadFilter` | `struct { space: bool, special: bool }`(기본 둘 다 true) · `accepts(char) -> bool` | 어떤 글자를 버퍼에 넣는가 | `C/typeahead.rs:147-176` |
| UIC-139 | `HudPos` | enum 9자리(기본 `BottomLeft`) · `parse(&str)`(`top_left`…) · `from_code(&str)`(`tl`…`br`) · `code()` · `as_str()` | HUD 위치(3×3) — `SpeedHud`·위치 컨트롤과 공유 | `C/typeahead.rs:180-257` |
| UIC-140 | `find_prefix` | `fn(n: usize, from: usize, prefix: &str, label: impl Fn(usize) -> String) -> Option<usize>` | 접두 매치 앞으로 순환(대소문자 무시) | `C/typeahead.rs:260-273` |
| UIC-141 | `find_prefix_rev` | 같음 | 뒤로 순환(↑) | `C/typeahead.rs:276-289` |
| UIC-142 | `paint_hud` | `fn(ctx, bounds: Rect, scale: f32, pos: HudPos, text: &str, theme: &Theme)` | 입력 중 접두 카드 | `C/typeahead.rs:292-329` |
| UIC-145 | `hangul::is_jamo` | `fn(char) -> bool` | 조합기가 다루는 자모인가 | `C/hangul.rs:69-71` |
| UIC-146 | `hangul::Composer` | `new()` · `is_composing()` · `preview() -> Option<char>` · `reset()` · `flush() -> Option<char>` · `feed(char) -> String`(확정 0~2자) · `backspace() -> bool` | 두벌식 직접 조합기(겹모음·겹받침·도깨비불) | `C/hangul.rs:117-257` |
| UIC-147 | `hangul::jamo_from_qwerty` | `fn(c: char, shift: bool) -> Option<char>` | QWERTY → 자모(**Windows**: IME를 끊으면 라틴만 오므로 호스트가 한/영 상태를 들고 번역) | `C/hangul.rs:269-307` |
| UIC-150 | `ViewMode` | enum `Rich Compact`(기본) `Plain` · `ALL` · `code()` · `from_code()` · `is_variable_height()` · `fixed_row_height() -> Option<i32>`(Compact 34 · Plain 24) | 목록 보기 모드(메신저/클립 유래 — 탐색기의 상세/아이콘 보기와는 다른 개념) | `C/view_mode.rs:9-59` |
| UIC-152 | `TokenKind` | enum `Plain Keyword Str Comment Number` · `color(&Theme) -> Color` | 구문 토큰 종류 | `C/highlight.rs:28-48` |
| UIC-153 | `Highlighter` | `trait: Debug { fn line_spans(&self, line: &str, state: &mut u32, out: &mut Vec<(usize, TokenKind)>); fn name(&self) -> &str; fn doubled_quote_escapes(&self) -> bool; fn strings_span_lines(&self) -> bool; }` | 줄 단위 강조 포트 | `C/highlight.rs:52-65` |
| UIC-154 | `SyntaxSpec` | `struct { name, extensions, keywords, case_insensitive, line_comments, block_comments, strings, escape_backslash, ident_extra, numbers }` · `plain()` · `sql()` · `parse(&str) -> Result<Self, String>` | `.nexa-syntax` 규격(데이터 주도 토크나이저) — 텍스트 미리보기 구문 강조에 재사용 가능 | `C/highlight.rs:69-202` |
| UIC-155 | `to_html` | `fn(text, hl: &dyn Highlighter, th: &Theme, font_family: &str, font_px: i32) -> String` | 서식 있는 복사용 HTML | `C/highlight.rs:381-427` |
| UIC-156 | `SQL_SPEC` | `pub const &str` | 내장 SQL 규격 원문 | `C/highlight.rs:430` |
| UIC-158 | `avatar::initials` | `fn(&str) -> String` | 앞 2글자(공백 제외) | `C/avatar.rs:25-30` |
| UIC-159 | `avatar::avatar_color` | `fn(seed: &[u8]) -> Color` | 시드 → 8색 팔레트 | `C/avatar.rs:34-39` |
| UIC-160 | `avatar::draw_avatar` | `fn(ctx, rect, name: &str, seed: &[u8], font_delta_logical: f32)` | 원형 이니셜 아바타 | `C/avatar.rs:46-68` |
| UIC-161 | `avatar::draw_builtin` | `fn(ctx, rect, img: &IconImage, seed: &[u8])` | 원 배경 + 투명 일러스트 | `C/avatar.rs:74-77` |

### 1-11. controls/mod.rs — 공용부

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-165 | 컨트롤 모듈·재수출 | `pub mod` 23개(+ 비공개 `editmenu`) · `pub use` 색인 | `nexa_ctl::controls::X` 경로 | `C/controls/mod.rs:16-74` |
| UIC-166 | `set_control_size_mult` / `control_size_mult` | `fn(f32)` / `fn() -> f32` · 전역(AtomicU32 · 기본 1.0) | 체크·스위치·라디오 **글리프** 크기 배율 | `C/controls/mod.rs:82-93` |
| UIC-167 | `control_size_mult_from_code` | `fn(&str) -> f32` | `s/m/l/xl` → 0.8/1.0/1.3/1.6(미지 = 1.0) | `C/controls/mod.rs:97-104` |
| UIC-168 | `ctl_size` | `fn(logical: i32) -> i32`(최소 1) | 논리 px에 컨트롤 크기 배율 적용(글리프 치수 전용) | `C/controls/mod.rs:147-151` |
| UIC-169 | `CtlMsg` | enum `CtxSelectAll CtxCopy CtxCut CtxPaste` | 컨트롤 내장 문자열 키(편집 우클릭 메뉴) | `C/controls/mod.rs:110-119` |
| UIC-170 | `set_ctl_labels` / `ctl_label` | `fn(f: fn(CtlMsg) -> &'static str)`(OnceLock — **1회만**, 이후 무시) / `fn(CtlMsg) -> &'static str` | i18n 라벨 공급자 주입(미주입 = 영어) | `C/controls/mod.rs:132-143` |
| UIC-171 | `LEADING_ICON` | `const i32 = 13` | 선행 아이콘 변 크기(콤보/Choose/트리/버튼 공용) | `C/controls/mod.rs:155` |
| UIC-172 | `BorderSpec` | `struct { width: f32, color: Color, alpha: f32 }` · `new(width, color, alpha)` · Default = 폭 0(없음) | 외곽 테두리 설정 | `C/controls/mod.rs:159-188` |
| UIC-173 | `HAlign` | enum `Left Center`(기본) `Right` | 가로 정렬(전 컨트롤 공통) | `C/controls/mod.rs:192-200` |
| UIC-174 | `VAlign` | enum `Top Center`(기본) `Bottom` | 세로 정렬 | `C/controls/mod.rs:204-212` |
| UIC-175 | `LabelSide` | enum `None Left Right`(기본) | 라벨 위치 | `C/controls/mod.rs:216-224` |
| UIC-176 | `ControlBase` | `struct { bounds: Rect, scale: f32, focused, active, enabled: bool, help: Option<String>, show_help, help_open: bool, halign, valign, last_value: Option<String> }` · Default = scale 1.0 · active true · enabled true | 모든 컨트롤이 컴포지션하는 공통 상태 | `C/controls/mod.rs:228-273` |
| UIC-177 | `Control` | `trait Control: Widget { fn base(&self) -> &ControlBase; fn base_mut(&mut self) -> &mut ControlBase; … 기본 메서드 25개 }` | 공통 기능 "상속"(§2-3에 메서드 표) | `C/controls/mod.rs:278-487` |
| UIC-178 | `wrap_text` | `fn(ctx, text: &str, max_w: i32) -> Vec<String>` | 공백 단위 그리디 줄바꿈 | `C/controls/mod.rs:490-513` |
| UIC-179 | `draw_checkbox_glyph` | `fn(ctx, theme, box_r: Rect, checked: bool, active: bool)` | 체크박스 글리프(행 안 체크 열 등에 재사용) | `C/controls/mod.rs:516-548` |
| UIC-180 | `draw_radio_glyph` | `fn(ctx, theme, r: Rect, selected: bool, active: bool)` | 라디오 글리프 | `C/controls/mod.rs:551-577` |
| UIC-181 | `draw_updown_chevrons` | `fn(ctx, theme, area: Rect, color)` | 콤보 ⇕ 표식 | `C/controls/mod.rs:580-611` |
| UIC-182 | `image_fit_contain` | `fn(area: Rect, iw: i32, ih: i32) -> Rect` | 비율 유지 맞춤(여백) — 이미지 미리보기 배치에 재사용 | `C/controls/mod.rs:616-628` |
| UIC-183 | `image_fit_cover` | 같음 | 비율 유지 가득 채움(호출자가 클립) | `C/controls/mod.rs:633-645` |
| UIC-184 | `draw_check_mark` | `fn(ctx, area: Rect, color)` | ✓ | `C/controls/mod.rs:648-658` |
| UIC-185 | `draw_chevron_down` | `fn(ctx, area, color)` | ∨ | `C/controls/mod.rs:661-675` |
| UIC-186 | `fallback_file_icon` | `fn(is_dir: bool) -> IconImage`(16×16) | 셸 아이콘이 없거나 아직 안 온 파일/폴더의 자체 그림 | `C/controls/mod.rs:679-724` |
| UIC-187 | `draw_chevron_90` | `fn(ctx, area, color, expanded: bool)` | 펼침/접힘 셰브론 — **주석: "nexa-dir2 파일 그리드와 같은 모양 · 꺾임 90°"** | `C/controls/mod.rs:729-731` |
| UIC-188 | `draw_chevron_90_in` | `+ clip: Option<Rect>` | 반쯤 잘린 행에서도 보이는 부분만 | `C/controls/mod.rs:734-765` |
| UIC-189 | `draw_chevron_right` | `fn(ctx, area, color)` | › | `C/controls/mod.rs:767-781` |
| UIC-190 | `ProbeCtx` | `struct ProbeCtx;` · `impl DrawCtx`(그리기 no-op · `text_width` = 문자수×7) | **공개된 테스트 백엔드** — 다운스트림(dir3) 위젯 시험에서도 쓴다 | `C/controls/mod.rs:787-796` |

### 1-12. controls/glyphs.rs · scroll.rs · splitter.rs · flash.rs

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-192 | `GlyphKind` | enum 13종: `Folder FolderNew Refresh ArrowBack ArrowForward ArrowUp Home ToggleOn ToggleOff Copy Link Open Text` | 코드 도형 아이콘 종류 | `C/controls/glyphs.rs:13-40` |
| UIC-193 | `glyph` | `fn(kind: GlyphKind) -> MenuIcon` | 64px 알파 마스크(4×4 슈퍼샘플 · 256 단위 좌표) · **스레드 로컬 `Rc` 캐시** → `MenuIcon`은 `Send`가 아니다 | `C/controls/glyphs.rs:42-43,197-212` |
| UIC-194 | `MenuIcon`(참조) | `struct { w, h: u32, alpha: Rc<[u8]>, rgba: Option<Rc<[u8]>> }` · `from_alpha` · `from_rgba` | 글리프의 반환 타입(상세는 컨트롤 카탈로그) | `C/controls/ctxmenu.rs:55-87` |
| UIC-196 | `DEFAULT_HIDE_MS` · `set_hide_delay_ms` · `hide_delay_ms` | `const u64 = 2000` · `fn(u64)`(0 = 숨기지 않음) · `fn() -> u64` | 스크롤바 자동 숨김 지연(전역). **주의**: `controls` 재수출에 없다 → `nexa_ctl::controls::scroll::set_hide_delay_ms` | `C/controls/scroll.rs:29-45` · `C/controls/mod.rs:60` |
| UIC-197 | `FastScroll` | `struct { enabled, step: u32, max: i32, window_ms: u64, hud: bool, hud_pos: HudPos, hud_hold_ms, hud_fade_ms: u64 }` · Default = **끔** · step 5 · max 8 · window 160 · HUD 우상단 · hold 250 · fade 600 | 고속 스크롤 설정 값 | `C/controls/scroll.rs:56-89` |
| UIC-198 | `set_fast_scroll` / `fast_scroll` | `fn(FastScroll)` / `fn() -> FastScroll` · 전역(RwLock) | 설정 즉시 적용(핫스왑) | `C/controls/scroll.rs:91-113` |
| UIC-199 | `SpeedHud` | `note(k, &cfg)` · `note_at(k, &cfg, Instant)` · `clear()` · `visible()` · `alpha_at(Instant, &cfg) -> Option<f32>` · `tick(Instant, &cfg) -> bool` · `paint(&self, ctx, theme, area: Rect, scale: f32, &cfg)` | `×k` 속도 캡슐(가속이 끊기면 hold 뒤 제곱 감속 페이드) | `C/controls/scroll.rs:119-250` |
| UIC-200 | `ScrollAccel` | `new()` · `factor(dir: i32) -> i32` · `factor_cfg(dir, &cfg)` · `factor_at(dir, Instant, &cfg)` · `reset()` | 같은 방향 사건이 `window_ms` 안에 이어지면 배수 = `1 + 연속/step`(최대 `max`). 휠·키 자동 반복 공통 | `C/controls/scroll.rs:258-316` |
| UIC-201 | `ScrollBars` | `new()` · `on_event(&mut self, ev, vp: Rect, content_w, content_h, off_x, off_y, scale) -> (i32, i32, bool)` · `tick(now_ms: u64) -> bool` · `paint(&self, ctx, theme, vp, content_w, content_h, off_x, off_y, scale)` · `show()` · `is_visible()` · `set_fast_override(Option<FastScroll>)` · `fast_cfg()` · `note_fast(k)` · `hud() -> &SpeedHud` · `h_thumb_for_test(vp, content_w, off_x, scale) -> Option<Rect>` | macOS식 반투명 오버레이 스크롤바(세로+가로 · 축별 독립). **오프셋은 호스트 소유** — `Widget` 구현체가 아니다 | `C/controls/scroll.rs:320-699` |
| UIC-202 | 스크롤바 치수 | (비공개 상수) `THIN=6 THICK=11 MARGIN=2 MIN_THUMB=28` 논리 px · 알파 `0.35`/`0.6` · 휠 이동량 = `delta / 3 * k` px(1노치 = 40px) | 시각·거동 수치 | `C/controls/scroll.rs:20-26,476,482` |
| UIC-205 | `SplitAxis` | enum `Vertical`(세로선 · x 이동) `Horizontal`(가로선 · y 이동) | 경계선 방향 | `C/controls/splitter.rs:18-21` |
| UIC-206 | `SplitEvent` | enum `None Hover Start Drag(i32) End` | `Drag(v)` = 띠 시작의 새 축 좌표(물리 px · 창 기준) | `C/controls/splitter.rs:25-36` |
| UIC-207 | `Splitter` | `new(SplitAxis)` · `axis()` · `set_rect(Rect)`(폭 0 = 비활성) · `rect()` · `is_dragging()` · `is_hover()` · `on_event(&InputEvent) -> SplitEvent` · `tick(now_ms) -> bool` · `paint(&self, ctx, theme)` | 경계선 + 드래그. 기하·클램프·커서 모양(↔/↕)·저장은 호스트 몫 — `Widget` 구현체가 아니다 | `C/controls/splitter.rs:40-155` |
| UIC-210 | `FlashTone` | enum `Ok`(기본) `Warn Info` | 순간 메시지 색조 | `C/controls/flash.rs:18-23` |
| UIC-211 | `Flash` | `new()`(배경 켬) · `with_background(bool)` · `set_background` · `show(text, tone, hold_ms, fade_ms)`(fade 최소 200) · `clear()` · `active()` · `text()` · `strength_at(Instant) -> Option<f32>` · `place(anchor, w, h, host, gap) -> Rect`(연관 함수) · `paint(&mut self, dc, th, anchor: Rect, host: Rect) -> bool` | "복사됨" 류 순간 메시지. `paint`가 true인 동안 호스트가 다시 그린다 · 창의 **맨 마지막**에 그린다. **`std::time::Instant::now()`를 내부에서 읽는다**(시각 주입 아님) | `C/controls/flash.rs:27-155` |

### 1-13. nexa-gfx

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-220 | `Color` | `struct Color(pub u32)`(`0x00RRGGBB`) · `rgb() -> (u8,u8,u8)` · `from_rgb(r,g,b)` · `lerp(other, t)` | 색. 알파 채널 없음(알파는 호출 인자) | `G/surface.rs:9-37` |
| UIC-221 | `IconImage` | `struct { w, h: u32, rgba: Vec<u8> }`(straight alpha · 행 우선) · `from_rgba(w, h, Vec<u8>)`(길이 불일치 = **패닉**) · `from_alpha_tinted(w, h, &[u8], (r,g,b))` · `resized(w, h)`(bilinear 사본) · `swatch(size, rgb)` | RGBA 이미지. `Clone`은 깊은 복사(Vec) | `G/surface.rs:45-168` |
| UIC-222 | `Surface<'a>` | `new(buf: &mut [u32], width: usize, height: usize)`(버퍼 부족 = 패닉) · `width()` `height()` · `fill(Color)` · `fill_rect(x, y, w: u32, h: u32, Color)` · `fill_rect_alpha(..., alpha: f32)` · `blend_px(x, y, Color, coverage)` · `blend_mask(x, y, &GlyphBitmap, Color, clip, slant, baseline_y)` · `blend_image(x, y, &IconImage, clip)` · `blend_image_scaled(dx, dy, dw, dh, &IconImage, clip)` | 빌린 `u32` 픽셀 버퍼 위의 캔버스. **형식 = softbuffer와 동일**(변환 없이 present). 모든 그리기는 표면 경계로 클립 · 행 간격 = 폭(stride 없음) | `G/surface.rs:173-457` |
| UIC-225 | `set_tab_cols` / `tab_cols` | `fn(u32)`(1~16 · 그 밖 4) / `fn() -> u32` | 탭 폭(칸) 전역 | `G/text.rs:11-17,188-190` |
| UIC-226 | `set_text_contrast` / `text_contrast` | `fn(f32)`(0.5~3.0) / `fn() -> f32` | 커버리지 감마(기본 1.0) | `G/text.rs:21-40` |
| UIC-227 | `set_text_snap` / `text_snap` | `fn(bool)` / `fn() -> bool` | 글리프 원점 정수 스냅(기본 끔) | `G/text.rs:24,176-184` |
| UIC-228 | `set_text_hint` / `text_hint` | 같음 | 오토힌트 근사(x-높이 정수화 + 세로 줄기 모으기 · 기본 끔) | `G/text.rs:45,98-106` |
| UIC-229 | `set_text_gdi` / `text_gdi` | `fn(bool)` / `fn() -> bool`(Windows·macOS에서만 true 가능) | **OS 래스터라이저 글리프 사용**(Windows GDI ClearType · macOS CoreText · 기본 끔 · Linux는 늘 ab_glyph) | `G/text.rs:48-62` |
| UIC-230 | `set_text_weight` / `text_weight` | `fn(f32)`(0.0~0.6) / `fn() -> f32` | 획 두께 보강(stem darkening 근사) | `G/text.rs:66-78` |
| UIC-231 | `set_tab_stops` / `tab_stops` | `fn(bool)` / `fn() -> bool` | 탭 = 정지점(기본) / 절대 폭 | `G/text.rs:194-205` |
| UIC-232 | `GLYPH_CACHE_MAX` · `set_glyph_cache_max` · `glyph_cache_max` | `const usize = 8192` · `fn(usize)`(0 → 1) · `fn() -> usize` | 글리프 비트맵 캐시 상한(넘으면 비우고 다시) | `G/text.rs:249-263` |
| UIC-233 | `GlyphBitmap` | `struct { w, h: u16, ox, oy: i16, cov: Vec<u8>, rgb: bool }` | 래스터된 글리프(`rgb` = ClearType 채널별 커버리지 · 픽셀당 3바이트) | `G/text.rs:233-246` |
| UIC-234 | `Font` 로드 | `from_static(data: &'static [u8], index: u32) -> Result<Self, FontError>` · `from_bytes(Vec<u8>, index)`(의도적 누수로 `'static`화) · `Clone`(캐시 Arc 공유) | 글꼴은 프로세스 수명 자원. **바이트는 밖에서 온다**(nexa-gfx는 파일을 읽지 않음) | `G/text.rs:276-352,774-776` |
| UIC-235 | `Font` 폴백 | `push_fallback(&mut self, data, index) -> Result<(), FontError>` · `push_fallback_font(&mut self, other: &Font)` · `set_face_family(i, family: &str)` · `face_family_names(i) -> Vec<String>` · `covers(char)` / `has_glyph(char)` | 글자마다 글리프가 있는 첫 face(두부 예방). 기준선은 주 face가 정한다 | `G/text.rs:359-381,429-444,478-491,780-782,847-849` |
| UIC-236 | `Font` 측정 | `measure(text, size) -> f32` · `measure_from(text, size, start_rel)` · `measure_from_styled(text, size, start_rel, bold)` · `tab_advance(size, rel) -> f32` | 폭(px · f32). 탭은 원점 기준 정지점 · 제어 문자 폭 0 | `G/text.rs:793-831` |
| UIC-237 | `Font` 메트릭 | `line_height(size)` · `ascent(size)` · `text_box_height(size)`(어센트+디센트) · `digit_height(size)`('0' 실측 높이) | 세로 배치 실측 | `G/text.rs:786-789,841-870` |
| UIC-238 | `Font` 그리기 | `draw_text(surface, x, y, size, color, text) -> f32` · `draw_text_clipped(..., clip)` · `draw_styled(surface, x, y, size, color, text, clip: (i32,i32,i32,i32), style: TextStyle, tab_origin: f32) -> f32` | `(x, y)` = **베이스라인 왼쪽 끝**(DrawCtx의 "왼쪽 위"와 다름). faux 볼드 = x축 2회 · faux 이탤릭 = 전단 0.22 | `G/text.rs:875-963` |
| UIC-239 | `Font` 진단 | `clear_glyph_cache()` · `glyph_cache_len()` · `glyph_stem_visibility(ch, size, bold)` · `glyph_is_subpixel(ch, size)` · `glyph_ink(ch, size, bold)` · `glyph_ascii(ch, size) -> String` | 캡처 없이 래스터 결과를 검사하는 시험 도구 | `G/text.rs:494-504,686-768` |
| UIC-240 | `FontError` | `struct FontError;`(Copy Eq) | 파싱 불가·인덱스 범위 밖 | `G/text.rs:316` |
| UIC-241 | `TextStyle` | `struct { bold: bool, italic: bool }` · `PLAIN` | faux 스타일 | `G/text.rs:320-333` |
| UIC-245 | `image::ImageKind` | enum `Png Jpeg Gif Bmp Webp Unknown` · `name()` · `decodable()`(Png·Gif·Bmp·Jpeg) · `ext()` | 시그니처 판별 결과 | `G/image.rs:13-53` |
| UIC-246 | `image::sniff` | `fn(&[u8]) -> ImageKind` | 앞 바이트로 판별 | `G/image.rs:56-70` |
| UIC-247 | `image::decode` | `fn(b: &[u8], max_pixels: usize) -> Result<IconImage, String>` | PNG(Adam7 포함)·BMP·GIF 첫 프레임·기저 JPEG → RGBA. WebP = 오류 문구. 손상 입력 = `Err`(패닉 없음) | `G/image.rs:73-82` |
| UIC-248 | `image::dimensions` | `fn(&[u8]) -> Option<(u32,u32)>` | (폭, 높이)만 빨리 | `G/image.rs:85-100` |
| UIC-249 | `jpeg::dimensions` / `jpeg::decode` | `fn(&[u8]) -> Option<(u32,u32)>` / `fn(b, check: &dyn Fn(u32,u32) -> Result<(), String>) -> Result<IconImage, String>` | 기저(SOF0/SOF1 · 8비트 · 허프만) JPEG. **미지원**: 프로그레시브(SOF2)·산술 부호·12비트·CMYK | `G/jpeg.rs:1-9,252-255` |
| UIC-250 | `inflate::inflate_zlib` / `inflate_raw` / `adler32` | `fn(&[u8]) -> Result<Vec<u8>, &'static str>` / `-> Result<(Vec<u8>, usize), &'static str>` / `fn(&[u8]) -> u32` | DEFLATE·zlib 풀기(외부 crate 0) — ZIP 항목 풀기에 재사용 가능(**추정**: 원시 DEFLATE이므로 가능하나 CRC-32 함수는 공개 표면에 없다) | `G/inflate.rs:7-71` |
| UIC-252 | `gdi`(Windows · `pub(crate)`) | `face_ok(names: &[String], em_px: i32) -> bool` · `glyph(names, em_px, bold, ch) -> Option<Glyph>` · `#[link(name = "gdi32")]` · 스레드 로컬 face 캐시 | GDI ClearType 32bpp DIB → 채널별 커버리지 + 정수 전진 폭 · 진짜 볼드 face | `G/lib.rs:9-10` · `G/gdi.rs:1-11,107,198,323-328` |
| UIC-253 | `coretext`(macOS · `pub(crate)`) | `face_ok` · `advance(names, em_px, bold, ch) -> Option<f32>` · `glyph(names, em_px, bold, ch, sub: u8) -> Option<Glyph>` · CoreFoundation·CoreGraphics·CoreText 프레임워크 링크 | CoreText 회색 커버리지 + 소수 전진 폭 | `G/lib.rs:7-8` · `G/coretext.rs:1-7,55,68,92,274-313` |
| UIC-254 | `names`(`pub(crate)`) | `family_names(data: &[u8], index: u32) -> Vec<String>` | 글꼴 `name` 테이블의 패밀리 이름 전부(영문·현지어) | `G/names.rs:1-3,48` |
| UIC-255 | 크레이트 속성·재수출 | `#![forbid(unsafe_op_in_unsafe_fn)]` · `pub use surface::{Color, IconImage, Surface}` · `pub use text::{Font, FontError, TextStyle}` | — | `G/lib.rs:5,19-20` |

### 1-14. nexa-sys

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-260 | `Signals` | `struct { on_battery, remote_session, reduce_motion: Option<bool>, cpu_count: usize }` · `Signals::read()` | 네 신호 스냅샷(호출자가 캐시 — 기동 1회 + 60초마다 정도) | `S/lib.rs:25-47` |
| UIC-261 | `on_battery` | `fn() -> Option<bool>` | Win `GetSystemPowerStatus` · mac IOKit · Linux `/sys/class/power_supply` | `S/lib.rs:51-53,109-123,231-244,288-316` |
| UIC-262 | `remote_session` | `fn() -> Option<bool>` | Win `SM_REMOTESESSION` · mac = `None` · Linux `SSH_CONNECTION`/`DISPLAY` | `S/lib.rs:57-59,125-128,246-248,318-327` |
| UIC-263 | `reduce_motion` | `fn() -> Option<bool>` | Win `SPI_GETCLIENTAREAANIMATION` · mac `CFPreferences reduceMotion` · Linux GTK `settings.ini` | `S/lib.rs:63-65,130-142,250-273,329-350` |
| UIC-264 | `cpu_count` | `fn() -> usize`(최소 1) | `available_parallelism` | `S/lib.rs:69-71` |
| UIC-265 | feature `gui` | 기본 on · `input_source`·`layer_present` 게이트 | CLI는 `default-features = false`로 GUI 프레임워크 링크를 피한다 | `nexa-ui/crates/nexa-sys/Cargo.toml:17-21` · `S/lib.rs:18-21` |
| UIC-266 | `input_source::is_korean` | `fn() -> Option<bool>` | mac: Carbon TIS 입력 소스 ID가 `com.apple.inputmethod.Korean…` · **그 밖 = `None`** | `S/input_source.rs:17-19,82-105,137-139` |
| UIC-267 | `input_source::watch` / `take_changed` | `fn() -> bool` / `fn() -> bool` | mac: 입력 소스 바뀜 분산 알림 구독(깃발만) · 그 밖 = no-op/false. **주 스레드 전용** | `S/input_source.rs:22-30,107-132,140-145` |
| UIC-269 | `layer_present::Frame<'a>` | `pixels_mut() -> &mut [u32]` · `pixels()` · `present(self)` | 한 프레임(이전 프레임 내용 보존 안 함) | `S/layer_present.rs:18-41` |
| UIC-270 | `layer_present::LayerPresenter` | `unsafe fn new(ns_view: *mut c_void) -> Option<Self>` · `resize(w: u32, h: u32, scale: f64) -> bool` · `frame() -> Option<Frame<'_>>` · (mac) `pool_len()` | macOS IOSurface 풀 + sRGB 태그(present 36 → 2.9 ms 실측 주석). **macOS 밖에서는 `new` = 늘 `None`**(같은 모양의 빈 구현). 메인 스레드 전용 · `Send` 아님 | `S/layer_present.rs:1-12,60-93,249,290-372,525` · `D/STATUS.md:50` |
| UIC-271 | `layer_present::app_active` | `fn() -> Option<bool>` | mac `NSApplication.isActive`(캐럿 깜빡임 정지 판정) · 그 밖 = `None` | `S/layer_present.rs:46-58` |

### 1-15. nexa-conf

| ID | 타입/함수 | 시그니처 요약 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-280 | `SCHEMA` | `const u32 = 1` | 파일 스키마 버전(`_schema` 키로 기록) | `F/lib.rs:40` |
| UIC-281 | `Doc` | `struct { schema: u32, pairs: Vec<(String, String)> }` | 파싱 결과(파일 순서 보존 · `_schema` 제외) | `F/lib.rs:44-49` |
| UIC-282 | `parse` | `fn(&str) -> Doc`(실패하지 않는다) | UTF-8 · `key=value` · `#` 주석 · `=` 최초 1회 분할 · CRLF 허용 · U+2028/2029 → 개행 복원 | `F/lib.rs:53-77` |
| UIC-283 | `serialize` | `fn(known: &[(&str, &str)], unknown: &[(String, String)]) -> String` | `_schema` 먼저 → 아는 키(호출자 순서) → 미지 키 재방출(known이 이긴다) · 값 속 개행은 U+2028/2029 | `F/lib.rs:86-114` |
| UIC-284 | `write_atomic` | `fn(path: &Path, contents: &str) -> io::Result<()>` | temp → `sync_all` → rename → (Unix) 부모 fsync · Unix 모드 0600 | `F/lib.rs:125-164` |
| UIC-285 | `SaveScheduler` | `new(quiet_ms, max_delay_ms)` · `mark(Instant)` · `dirty()` · `tick(Instant) -> bool` · `flush_now() -> bool` | 변경 N회 → 쓰기 1회(조용해진 지 quiet **또는** 첫 미저장 후 max_delay) | `F/lib.rs:174-226` |
| UIC-286 | `Store` | `open(path: PathBuf, quiet_ms, max_delay_ms) -> (Self, Doc)` · `keep_unknown(key, value)` · `path()` · `save(known) -> io::Result<bool>`(직전과 같으면 안 씀) · `pub sched: SaveScheduler` | 파일 1개 = 스토어 1개. 값은 앱이 소유(레지스트리) | `F/lib.rs:235-281` |
| UIC-287 | `dir_writable` | `fn(&Path) -> bool` | 프로브 파일 생성→삭제(포터블 판정) | `F/lib.rs:294-308` |
| UIC-288 | `user_config_dir` | `fn(app: &str) -> Option<PathBuf>` | Win `%APPDATA%\{app}` · mac `~/Library/Application Support/{app}` · 그 외 `$XDG_CONFIG_HOME/{app}` 또는 `~/.config/{app}` | `F/lib.rs:314-334` |
| UIC-289 | `is_replaced_on_upgrade` | `fn(exe_dir: &Path) -> bool` | macOS `.app` 번들 · Homebrew keg · 시스템 프리픽스(`/usr /opt /snap /nix`) = 실행 파일 옆에 데이터를 두면 안 되는 자리 | `F/lib.rs:348-372` |

### 1-16. 워크스페이스·문서 수준(규약의 근거)

| ID | 항목 | 내용 | 용도 | 위치 |
|---|---|---|---|---|
| UIC-295 | 워크스페이스 | 구성원 7(gfx·ctl·conf·font·fs·dlg·sys) · edition 2021 · rust 1.82 · 라이선스 `LicenseRef-PolyForm-Noncommercial-1.0.0` · 타깃 5(win x64/arm64 · mac x64/arm64 · linux x64) | dir3의 path 의존 대상 | `nexa-ui/Cargo.toml:7-27` · `nexa-ui/rust-toolchain.toml:2-11` |
| UIC-296 | CI | 3-OS(windows-latest·macos-latest·ubuntu-latest) × fmt/clippy `-D warnings`/`cargo test --workspace` · Linux 러너는 `fonts-noto-cjk` 등 설치 | main은 항상 green | `nexa-ui/.github/workflows/ci.yml` |
| UIC-297 | `scripts/check-3os.sh` | fmt + 호스트 clippy + 나머지 두 OS 타깃 clippy(컴파일만) · `--quick` | push 전 검사 | `nexa-ui/scripts/check-3os.sh:1-45` |
| UIC-298 | 확정 결정 DR-1~DR-6 | 공유 라이브러리 분리 · 앱 도메인 의존 0 · 외부 crate 0 지향 · 소비 앱 이관은 각 저장소 결정 · PolyForm NC · 출처 주석 보존 | 새 컨트롤이 지켜야 할 불변식 | `D/10-decision-record.md:7-14` |
| UIC-299 | 열린 결정 D-1~D-10 | D-3 `DrawCtx` 어휘 통일(dir2 세대 `select_font(slot,bold,italic)` vs 현 세대 — 합집합 권장) · D-4 모달 방식 · D-6 파일 아이콘 · D-7 휴지통 범위 · D-10 `nexa-grid` 별도 크레이트 등 | dir3 이식이 건드리게 될 미정 사항 | `D/10-decision-record.md:18-29` |
| UIC-300 | 백로그 | U-2 dir2 `widgets/{dock,tabbar,menubar}` 이식(tabbar·menubar ✅ · **dock ☐**) · U-3/G-1~6 `nexa-grid`(☐) · U-5 `DrawCtx` italic·밑줄·취소선(☐) · F-2 `Overlay` z 스택(☐) · F-5 `nexa-fs::ops`+Progress(☐) · F-8 OS 아이콘(Win ✅ · mac/Linux ☐) | "nexa-ui에 추가해야 할 것"의 기존 목록 | `D/TODO.md:7-27` |
| UIC-301 | 아직 없는 것(아키텍처 문서) | 도킹 패널 · 가상화 데이터 그리드 · 다중행 편집기(문서 시점 — 이후 TextBox가 다중행·미니맵까지 확장됨: `D/STATUS.md:74-80`) · 텍스트 셰이핑 | 문서와 코드의 시차에 주의 | `D/01-architecture.md:22-29` |
| UIC-302 | 원복 태그 | `baseline/pre-nexa-dir3-2026-10-03` · `baseline/pre-refactor-2026-09-27` | dir3 작업 전 상태로 되돌리는 기준점(`git tag` 출력으로 확인) | nexa-ui 저장소 태그 |

---

## 2. 위젯 계약과 이벤트 모델

### 2-1. 세 층의 "인터페이스"(`C/lib.rs:9-19`)

| 층 | 타입 | 누가 구현 | 근거 |
|---|---|---|---|
| Interface | `Widget`(UIC-031) · `DrawCtx`(UIC-041) | 컨트롤 / 렌더 백엔드(`RasterCtx` · 시험은 `ProbeCtx`) | `C/lib.rs:13` |
| Abstract class | `Control` + `ControlBase`(UIC-176·177) | 컨트롤은 `base()`/`base_mut()` 두 개만 구현 | `C/lib.rs:14` |
| 상속 | 조합 + 위임 | 컨트롤이 `ControlBase`를 필드로 품고, 복합 컨트롤은 하위 컨트롤에 위임 | `C/lib.rs:15` |

### 2-2. 전파 규약(반드시 지킬 것)

| # | 규약 | 근거 |
|---|---|---|
| W-1 | **이벤트는 `Widget::on_event` 단일 진입.** 반환값이 없다 — 결과는 ① `Invalidations`에 더러워진 rect push ② 컨트롤의 **1회성 폴링 메서드**(`take_*`)로 호스트가 꺼낸다(본보기 `Switch::take_toggled` — `C/controls/switch.rs:91-93`) | `C/lib.rs:17` · `C/widget.rs:52` |
| W-2 | **무효화는 바뀐 쪽이 신고.** 그리는 쪽이 아니라 상태를 바꾼 위젯이 `inv.push(rect)` | `C/lib.rs:18` · `C/widget.rs:3-5` |
| W-3 | **`paint`는 불변(`&self`)** — 상태 변경 없는 렌더가 계약. 예외: `Flash::paint(&mut self)`(끝나면 스스로 지움 — `C/controls/flash.rs:121`) | `C/lib.rs:19` · `C/widget.rs:55` |
| W-4 | **좌표는 전부 창 클라이언트 좌표(물리 px).** 논리 → 물리는 `Control::s(logical)` = `round(logical × base.scale)`. 호스트가 `set_scale`로 배율을 넣는다(최소 0.5) | `C/widget.rs:43` · `C/controls/mod.rs:285-287,323-325` |
| W-5 | **위젯은 시계가 없다.** 시각은 주입: `InputEvent::Char{now_ms}` · 부품의 `tick(now_ms: u64)` · 캐럿 위상 `DrawCtx::caret_on`. 예외(내부에서 `Instant::now()`): `Flash`·`SpeedHud::note/paint`·`ScrollAccel::factor`·`ScrollBars::tick` 안의 HUD 틱(`C/controls/scroll.rs:128,202,287,633` · `C/controls/flash.rs:65,122`) | `C/event.rs:71` · `C/draw.rs:32-34` |
| W-6 | **호스트를 모른다.** 클립보드·창·i18n은 "요청"만 남긴다(`EditCtxAction` 등) · 문자열은 `set_ctl_labels`로 주입 | `C/lib.rs:23-24` · `C/controls/mod.rs:106-143` |
| W-7 | **`Control`은 `dyn` 불가.** `set_help(impl Into<String>)`·`note_value(impl Into<String>)`가 제네릭 메서드라 트레이트 객체로 만들 수 없다(Rust 객체 안전 규칙 — 이 단계는 빌드 금지라 컴파일 확인은 안 함). 이질 컬렉션이 필요하면 `Box<dyn Widget>` + 구체 타입 보관(`Widget`은 객체 안전) | `C/controls/mod.rs:354,378` · `C/widget.rs:44-56` |
| W-8 | **`Widget`이 아닌 "부품"이 있다**: `ScrollBars`·`Splitter`·`Flash`·`SpeedHud`·`TypeAhead`·`Fade` 계열은 호스트/상위 위젯이 기하·오프셋을 들고 `on_event`/`tick`/`paint`를 직접 부른다(시그니처가 `Widget`과 다르다) | `C/controls/scroll.rs:10-11` · `C/controls/splitter.rs:6-8` |
| W-9 | **빈 bounds = 보이지 않음.** 호스트가 화면 밖 컨트롤을 빈 rect로 두므로 `paint`는 `bounds.is_empty()`면 즉시 반환해야 한다(고정 크기 글리프가 (0,0)에 그려지던 결함) | `C/controls/switch.rs:167-171` |

### 2-3. `Control` 기본 메서드 표(`C/controls/mod.rs:278-487`)

| 메서드 | 시그니처 | 뜻 | 줄 |
|---|---|---|---:|
| `base` / `base_mut` | `fn(&self) -> &ControlBase` / `fn(&mut self) -> &mut ControlBase` | **구현 필수 2개** | 280·282 |
| `s` | `fn(&self, logical: i32) -> i32` | 논리 → 물리 px | 285 |
| `set_enabled` / `is_enabled` | `fn(&mut self, bool)` / `fn(&self) -> bool` | 사용 가능(끄면 포커스도 내려감 · 구현체가 흐리게+입력 무시를 존중) | 295·303 |
| `set_focused` / `is_focused` | — | 키보드 포커스(→ 포커스 링). **포커스 이동은 호스트가 관리**(컨트롤끼리 Tab 순서 없음 — 추정: 이 크레이트에 포커스 관리자 타입이 없다) | 307·311 |
| `set_active` / `is_active` | — | 창 활성(비활성 = 강조 무채화 · macOS 관례) | 315·319 |
| `set_scale` | `fn(&mut self, f32)` | 배율(≥0.5) | 323 |
| `set_halign`/`halign` · `set_valign`/`valign` | — | 정렬 | 328~342 |
| `align_y` | `fn(&self, area: Rect, item_h: i32, pad: i32) -> i32` | VAlign대로 y 계산 | 345 |
| `set_help` · `set_show_help` · `has_help_badge` · `toggle_help` | — | "?" 배지·툴팁 | 354~372 |
| `note_value` / `last_value` | `fn(&mut self, impl Into<String>)` / `fn(&self) -> Option<&str>` | 직전 확정값 기록·원복(검증 실패 시) | 378·382 |
| `accent_now` | `fn(&self, &Theme) -> Color` | 활성 = accent · 비활성 = text_dim | 387 |
| `draw_focus_ring` | `fn(&self, ctx, theme, around: Rect)` | 2px · 50% 반투명 링(`theme.focus_ring`) | 396 |
| `help_badge_rect` · `draw_help_badge` · `handle_help_click` · `draw_help_tip` | — | 배지 위치·그리기·클릭·말풍선(**paint 맨 끝에** 호출) | 413~486 |

### 2-4. 이벤트 번역(호스트 책임 — nexa-dir3 플랫폼 계층이 구현할 것)

| 항목 | 규칙 | 근거 |
|---|---|---|
| 휠 | 1노치 = `WHEEL_DELTA`(120)로 정규화 · 양수 = 위/오른쪽 · 트랙패드 분수 delta 그대로 전달(잔여는 `WheelAccum` 또는 픽셀 스크롤이 처리) · Shift+휠 = `HWheel` | `C/event.rs:8,50-59,118-134` |
| 주 수식키 | `primary` = mac ⌘ / Win·Linux Ctrl. 호스트가 번역 | `C/event.rs:60-61` |
| 단어 이동 | Win/Linux Ctrl+←/→ · mac ⌥←/→ → `Key::WordLeft/WordRight` · 서브워드 = Win/Linux Alt+←/→ · mac ⌃←/→ | `C/event.rs:37-44` |
| Backspace | `InputEvent::Char{c: '\u{8}'}` | `C/event.rs:35,70` |
| 전체 선택·실행 취소·다시 실행 | 전용 변형 `SelectAll`·`Undo`·`Redo`(호스트가 단축키를 해석) | `C/event.rs:78-83` |
| 마우스 | `MouseDown` = **좌클릭만** · `RightDown` · `MouseMove`(버튼 상태 무관 — 위젯이 드래그 상태 보유) · `MouseUp` | `C/event.rs:84-115` |
| 캐럿 깜빡임 | 호스트가 프레임마다 위상 계산 → `RasterCtx::with_caret_on` (nexa-sql 예: 500ms 주기 · 창/앱 비활성이면 고정 — `nexa-sql/crates/nexa-sql/src/app/paint.rs:52-55`) | `C/raster.rs:113-116` |
| 한글 타입어헤드 | 목록 타입어헤드는 IME를 끄고 raw 자모를 `Composer`로 직접 조합. Windows는 호스트가 한/영 상태를 들고 `jamo_from_qwerty`로 번역 · macOS는 `nexa_sys::input_source::is_korean()`으로 한글 입력 소스일 때만 | `C/hangul.rs:3-6,259-267` · `S/input_source.rs:3-6` |
| **이벤트 어휘에 없는 것** | 가운데 버튼 · 더블클릭 · X1/X2(뒤로/앞으로) 버튼 · Tab/F키/임의 단축키 · 드래그 앤 드롭 · IME 조합 사건 · 수식키 Alt 단독 — `InputEvent`에 변형이 없다. nexa-sql은 호스트(winit 사건)에서 직접 처리하고 컨트롤의 전용 메서드를 부른다(예: Alt → `draw::set_show_full` — `nexa-sql/crates/nexa-sql/src/app/event_loop.rs:695`) | `C/event.rs:49-116` |

### 2-5. 호스트 틱(프레임 예약) 규약

| 부품 | 틱 | "다시 그려야 함" 판정 | 근거 |
|---|---|---|---|
| `Fade`/`HoverFade`/`IntentFade` | `tick(now_ms) -> bool` | true = 값이 변함 · `is_animating()` = 다음 프레임 예약 여부 | `C/tokens.rs:421-450,553-567,660-694` |
| `HoverIntent` | `take_due(now_ms)` | `is_waiting(now_ms)` 동안 박동을 촘촘히 | `C/tokens.rs:752-774` |
| `LatestIntent` | `tick(now_ms) -> bool` | 확정 상태가 바뀌면 true · 표시 전 목표가 아직 유효한지 호출자가 재확인 | `C/tokens.rs:782-783,834-853` |
| `ScrollBars` | `tick(now_ms) -> bool` | 표시 전환·hover 두께·HUD 페이드 | `C/controls/scroll.rs:628-651` |
| `Splitter` | `tick(now_ms) -> bool` | hover 페이드 | `C/controls/splitter.rs:131-133` |
| `TypeAhead` | `tick(now_ms) -> bool` | 타임아웃으로 초기화되면 true(HUD 소거) | `C/typeahead.rs:135-142` |
| `Flash` | `paint(...) -> bool` | true인 동안 호스트가 계속 다시 그림 | `C/controls/flash.rs:121-154` |
| `nexa_conf::SaveScheduler` | `tick(Instant) -> bool` | true = 지금 저장(dirty 소비) | `F/lib.rs:206-217` |

### 2-6. 전역(프로세스) 설정 스위치 — 호스트가 설정 적용 시 넣는 것

> 값을 컨트롤마다 들고 다니지 않고 한 곳에서 읽는 "핫스왑 원칙"(`C/controls/scroll.rs:31-33`). **전역을 만지는 시험은 가드로 직렬화**해야 한다(§5-4).

| 스위치 | 기본 | 근거 | nexa-sql 설정 키(사용 예) |
|---|---|---|---|
| `controls::set_control_size_mult` | 1.0 | UIC-166 | `ui.control_size`(주석 `C/controls/mod.rs:81`) |
| `controls::set_ctl_labels` | 영어 | UIC-170 | `nexa-sql/crates/nexa-sql/src/main.rs:1293` |
| `controls::scroll::set_hide_delay_ms` | 2000 | UIC-196 | — |
| `set_fast_scroll` | 끔 | UIC-198 | `scroll.*`(`nexa-sql/crates/nexa-sql/src/app/settings.rs:613-639`) |
| `tokens::set_fade_ms(Fast/Slow)` | 500 / 1000 | UIC-111 | `ui.fade_fast_ms`·`ui.fade_slow_ms`(`…/settings.rs:281-285`) |
| `tokens::set_fade_out_ms` | 220 | UIC-109 | `ui.fade_out_ms` |
| `tokens::set_intent_ms` | 70 | UIC-110 | `ui.hover_intent_ms` |
| `tokens::set_hover_color`/`set_pressed_color` | 테마 `sel_bg` | UIC-108 | `nexa-sql/crates/nexa-sql/src/main.rs:1254-1255` |
| `draw::set_show_full` | false | UIC-072 | Alt 누름(여러 창) |
| `nexa_gfx::text::set_tab_cols`/`set_tab_stops` | 4 / 정지점 | UIC-225·231 | `editor.tab_size`·`editor.tab_stops` |
| `nexa_gfx::text::set_text_contrast`/`set_text_snap`/`set_text_hint`/`set_text_gdi`/`set_text_weight` | 1.0 / 끔 / 끔 / 끔 / 0 | UIC-226~230 | `ui.text_contrast`·`ui.text_snap`·`ui.text_hint`·`ui.text_gdi`·`ui.text_weight`(`…/settings.rs:142-146`) |
| `nexa_gfx::text::set_glyph_cache_max` | 8192 | UIC-232 | `ui.glyph_cache` |
| (범위 밖 · 참고) `set_default_click_guard_ms`·`set_click_policy`·`set_menu_icons`·`set_edit_menu_decor`·`set_hangul_app_compose` | — | `C/lib.rs:47-48` · `C/controls/mod.rs:50-51,67` | 컨트롤 카탈로그 문서 |

---

## 3. 그리기·텍스트·이미지

### 3-1. 픽셀 파이프라인

```
winit 창 → softbuffer 버퍼(&mut [u32] · 0x00RRGGBB)        ← (macOS 선택) nexa_sys::layer_present::LayerPresenter(IOSurface · 같은 픽셀 형식)
        → nexa_gfx::Surface::new(buf, w, h)                  G/surface.rs:185
        → nexa_ctl::RasterCtx::new(&mut surface, &font, scale).with_fonts(prefs).with_caret_on(on)   C/raster.rs:90-124
        → widget.paint(&mut dc, &theme) …                    C/widget.rs:55
        → present
```

- 픽셀 형식은 softbuffer와 같아 변환 없이 present(`G/surface.rs:3` · `S/layer_present.rs:10`).
- nexa-sql은 한 프레임 안에서 층(UI 글꼴 층 · 고정폭 층 · 팝업 층)마다 `RasterCtx`를 **새로 만들어** 쓴다(`nexa-sql/crates/nexa-sql/src/app/paint.rs:65,399,408,423,436,502,517`). `RasterCtx`는 `Surface`를 가변 대여하므로 동시에 둘을 둘 수 없다.
- **클립 스택이 없다.** `DrawCtx`에는 `push_clip`/`pop_clip`이 없고, 클립은 ① 텍스트·이미지 호출의 `clip: Rect` 인자 ② `polyline_clipped` ③ 표면 경계뿐이다. `fill_rect`·`fill_round_rect`·`fill_ellipse` 등 도형은 **표면 경계로만** 잘린다 → 스크롤 영역 밖으로 번지지 않게 하려면 호출자가 `Rect::intersection`으로 미리 자르거나 그리는 순서(뒤에 크롬을 덮기)로 처리한다(`C/draw.rs:31-222` · `C/geom.rs:89-99`).

### 3-2. `RasterCtx`의 실제 동작(위젯 작성 시 알아야 할 것)

| 주제 | 동작 | 근거 |
|---|---|---|
| 배율 | 글자 크기 = `SlotFont.size × scale`. **좌표는 이미 물리 px**(호출자 몫). `scale`을 빠뜨리면 레이아웃만 커지고 글자는 1배로 남는 회귀가 있었다 → 생성자 필수 인자 | `C/raster.rs:69-70,83-92,141-143` |
| 슬롯 크기 | `Base/PeerList/Message/Status` = `FontPrefs`의 해당 칸 · **`Mono`는 `status` 크기를 따른다** | `C/raster.rs:349-358` |
| 고정폭 광학 보정 | `Mono` 슬롯 얼굴이 base와 다르면 숫자 '0' 높이 비로 0.75~1.15배 보정(`mono_mult`) | `C/raster.rs:362-368` |
| 글리프 폴백 | 슬롯 얼굴에 없는 글자는 **base 얼굴로 폴백**(런 분할 · 베이스라인 공유) — 측정(`text_width`·`text_prefix_widths`)도 같은 접기 순서로 **비트 동일** | `C/raster.rs:306-324,430-553` |
| 강제 볼드 | `select_font(slot, true)` = 슬롯 설정 위에 볼드 OR | `C/raster.rs:369-370` |
| `select_font_sized` | 증분은 **논리 px**(뒤에 배율이 곱해진다). `text_height()`는 **물리 px** → 측정값으로 증분을 만들 때는 배율로 나눈다(102차 규칙) | `C/raster.rs:373-377` · `C/controls/scroll.rs:212-215` · `D/STATUS.md:5` |
| `text_opaque` | `fill_rect(clip, bg)` 뒤 `text` | `C/raster.rs:392-395` |
| `text`의 y | `(x, y)` = 텍스트 상자 **왼쪽 위** · 베이스라인 = `y + ascent` | `C/raster.rs:438` |
| `text_center_y` | 잉크 가운데: `y + h/2 + digit_height/2 − ascent`(글꼴·OS가 달라도 같은 자리) | `C/raster.rs:571-581` |
| AA 도형 | 픽셀별 SDF 커버리지(0.5px AA) · 1비트 리전 클립 없음 · 라운드 채움/외곽선은 영역 분해로 가속 | `C/raster.rs:156-180,236-295` |
| `fill_ellipse` | 근사 SDF(원에 가까울수록 정확) | `C/raster.rs:592-607` |
| `fill_pie` | sweep은 180°로 잘린다(`sweep_deg.min(180.0)`) | `C/raster.rs:642` |
| `surface_size` | `Some((w, h))` | `C/raster.rs:340-342` |

### 3-3. 텍스트 스택(`nexa-gfx/text.rs`)

| 주제 | 내용 | 근거 |
|---|---|---|
| 엔진 | `ab_glyph` 글리프 래스터. **셰이핑 엔진 없음** — 한/영 전제(한글 완성형은 cmap 직결). 합자·아랍어·인도계 등 복합 문자, 결합 문자, 이모지 컬러 글리프는 지원 대상이 아니다(추정: 컬러 테이블 처리 코드가 없다) | `G/text.rs:1-8` · `D/01-architecture.md:29` |
| 글리프 캐시 | 키 = (face · 글리프 id · 크기 비트 · 가로 서브픽셀 1/3 · 감마 · 힌트 · 굵기 · gdi · bold) · 복제본끼리 `Arc` 공유 · 상한 넘으면 통째로 비움 | `G/text.rs:212-229,283,597-601` |
| 서브픽셀 배치 | 기본 = 가로 1/3px · 베이스라인 y는 정수 스냅 · `text_snap` 또는 Windows GDI 경로면 펜 x 반올림 | `G/text.rs:265,906-933` |
| **OS 분기 ①** Windows | `set_text_gdi(true)`면 GDI ClearType(채널별 커버리지 · 정수 전진 폭 · 진짜 볼드 face). face 이름은 `name` 테이블 + `set_face_family` | `G/text.rs:50-62,583-604` · `G/gdi.rs:1-11` |
| **OS 분기 ②** macOS | 같은 스위치로 CoreText(회색 커버리지 · 소수 전진 폭 · 서브픽셀 위치) | `G/text.rs:605-627` · `G/coretext.rs:1-7` |
| **OS 분기 ③** Linux·기타 | 늘 ab_glyph 경로(`gdi_face_uncached` = `None`). 선명도 보정은 `text_hint`·`text_snap`·`text_contrast`·`text_weight` 조합 | `G/text.rs:59-62,420-424` |
| 탭 | 정지점 방식(기본) = 줄 시작(탭 원점)부터 `공백 폭 × tab_cols`의 배수 · 글리프는 그리지 않음 | `G/text.rs:192-205,821-831,909-914` |
| 제어 문자 | 폭 0 · 글리프 없음(탭 두부 차단) | `G/text.rs:833-837,915-919` |
| faux 스타일 | 볼드 = x축 2회 그리기(GDI/CoreText face는 진짜 볼드) · 이탤릭 = 전단 0.22 | `G/text.rs:888-890,904,922-923` |
| 글꼴 공급 | nexa-gfx는 파일을 읽지 않는다. `nexa-font`가 시스템 글꼴을 mmap해 `Loaded{font, chain}`을 준다: `ui_font(Option<&str>)` · `mono_font(Option<&str>)`(범위 밖 크레이트 — 시그니처만 확인) | `G/text.rs:7-8` · `nexa-ui/crates/nexa-font/src/lib.rs:384-388,417,473` |

### 3-4. 이미지

| 주제 | 내용 | 근거 |
|---|---|---|
| 표현 | `IconImage` = RGBA straight · `Vec<u8>` 소유. 큰 이미지는 표시 크기로 `resized()`해 두고 `DrawCtx::image`로 찍는 "사전 스케일 캐시"가 권장 경로 | `G/surface.rs:39-52,90-93,387-391` |
| 디코드 가능 | PNG(8/16비트 · 팔레트 · Adam7) · BMP · GIF 첫 프레임 · 기저 JPEG(회색/YCbCr · 서브샘플링 최근접 업샘플 · 재시작 간격) | `G/image.rs:1-6` · `G/jpeg.rs:1-4` |
| 디코드 불가 | WebP(판별만) · 프로그레시브/산술/12비트/CMYK JPEG · **ICO · SVG · TIFF · HEIC · GIF 애니메이션**(코드 없음 — `sniff`가 `Unknown`을 준다) | `G/image.rs:13-20,56-70,79-80` · `G/jpeg.rs:3` |
| 안전 | 손상 입력 = `Err`(패닉 없음) · `max_pixels` 상한(메모리 보호) | `G/image.rs:5,106-117` |
| 틴트 아이콘 | 알파 마스크 + 상태색: `IconImage::from_alpha_tinted` · `MenuIcon::from_alpha` · `controls::glyph(GlyphKind)` · 도형은 `shape::*`로 코드에서 래스터(정적 자원 0바이트) | `G/surface.rs:74-88` · `C/controls/glyphs.rs:1-4,176-195` |
| 맞춤 | `image_fit_contain`/`image_fit_cover`(UIC-182·183) | `C/controls/mod.rs:616-645` |
| 품질 | `blend_image_scaled` = bilinear(확대·소폭 축소용). 큰 축소의 박스 필터/밉맵은 없다(추정: 코드에 4점 보간만 있다) | `G/surface.rs:372-437` |

### 3-5. 사용 예(호스트 페인트 — nexa-sql 실제 코드의 요약)

```rust
// nexa-sql/crates/nexa-sql/src/app/paint.rs:57-67 의 형태
let mut gfx = nexa_gfx::Surface::new(&mut buf, w as usize, h as usize);
let prefs = nexa_ctl::FontPrefs::with_base(ui_px);          // 논리 px
let mut dc = nexa_ctl::RasterCtx::new(&mut gfx, &ui_font, scale)   // scale = 창 배율(필수)
    .with_fonts(prefs)
    .with_caret_on(caret_on);
dc.fill_rect(Rect::new(0, 0, w, h), theme.window_bg);
widget.paint(&mut dc, &theme);                               // 팝업·툴팁·Flash는 맨 뒤(최상위 층)
```

```rust
// 위젯 시험(다운스트림에서도 동일) — C/controls/mod.rs:783-796
let mut ctx = nexa_ctl::controls::ProbeCtx;                  // text_width = 문자수 × 7
widget.paint(&mut ctx, &nexa_ctl::Theme::dark());            // 패닉 없이 그려지는지
```

### 3-6. OS 분기점 요약(이 범위에서 확인된 것 전부)

| # | 분기 | Windows | macOS | Linux | 근거 |
|---|---|---|---|---|---|
| OSB-1 | 글리프 래스터(선택) | GDI ClearType(`gdi32`) | CoreText | ab_glyph만 | UIC-229·252·253 |
| OSB-2 | present 뒷단 | softbuffer | softbuffer 또는 `LayerPresenter`(IOSurface) | softbuffer | UIC-270 |
| OSB-3 | 앱 활성 판정 | 창 포커스 | `app_active()` | 창 포커스 | UIC-271 |
| OSB-4 | 한글 입력 | 호스트가 한/영 상태 + `jamo_from_qwerty` | `is_korean()`/`watch()` + raw 자모 조합 | 신호 없음(`None`) — IME 경로(추정) | UIC-147·266·267 |
| OSB-5 | OS 신호 | kernel32/user32 | IOKit/CoreFoundation | sysfs/환경변수/GTK ini | UIC-261~263 |
| OSB-6 | 설정 폴더 | `%APPDATA%` | `~/Library/Application Support` | XDG | UIC-288 |
| OSB-7 | 관리형 설치 자리 | (해당 없음 — NSIS가 `data/` 보존) | `.app` 번들 · Homebrew | `/usr /opt /snap /nix` · Linuxbrew | UIC-289 |
| OSB-8 | 원자적 쓰기 권한 | — | 0600 + 부모 fsync | 0600 + 부모 fsync | UIC-284 |
| OSB-9 | 주 수식키·단어 이동 키 | Ctrl · Ctrl+←/→ · Alt+←/→ | ⌘ · ⌥←/→ · ⌃←/→ | Ctrl(Windows와 같음) | UIC-021·022 |

---

## 4. 테마·토큰

### 4-1. `Theme` 필드와 값(`C/theme.rs:59-131`)

| 토큰 | 뜻 | dark | light |
|---|---|---|---|
| `window_bg` | 창 루트 배경 | `#14161A` | `#F6F7F9` |
| `chrome_bg` | 메뉴·툴바 크롬 | `#1E2228` | `#EEF1F5` |
| `panel_bg` | 목록·패널 | `#191C21` | `#FFFFFF` |
| `panel_bg_alt` | 행 교대 음영 | `#1F242B` | `#F5F7FA` |
| `bubble_peer` | (메신저) 수신 말풍선 | `#313947` | `#E2E7EE` |
| `field_bg` | 입력 필드 | `#262B33` | `#FFFFFF` |
| `border` | 경계선·스플리터 | `#363C46` | `#D5DAE1` |
| `accent` | 강조 | `#3D8BFF` | `#3D8BFF` |
| `focus_ring` | 포커스 링 | `#7FB4FF` | `#A9CCFF` |
| `text` | 본문 | `#D6DAE0` | `#1B1F26` |
| `text_dim` | 보조 | `#8A919C` | `#6B7280` |
| `sel_bg` | 선택 행(포커스) | `#24405F` | `#D8E8FF` |
| `sel_bg_inactive` | 선택 행(비포커스) | `#2C313A` | `#E6E9EE` |
| `syn_keyword/string/comment/number` | 구문 강조 4색 | `#79B8FF/#E3A26A/#7C8A5A/#B5CEA8` | `#0A4FB5/#A31515/#578A2A/#098658` |
| `rainbow[6]` | 레인보우 괄호(이웃이 따뜻함↔차가움 교대 — 시험으로 고정) | `#F2C94C #4FC1FF #DA70D6 #7EE787 #FF9F43 #B392F0` | `#B58900 #268BD2 #D33682 #2AA198 #CB4B16 #6C71C4` |
| `danger` / `ok` / `warn` | 위험·긍정·주의 | `#E5534B/#2EA043/#B57C1E` | `#D32F2F/#1A7F37/#9A6700` |
| `is_dark` | 다크 여부 | true | false |

- 규칙: **색 하드코딩 금지 — 전 위젯이 테마를 인자로 받는다**(`C/theme.rs:3-4`). 예외로 코드에 박힌 색: 스위치 켜짐 초록 `#34C759`·손잡이 흰색(`C/controls/switch.rs:26-28`) · 체크/배지의 흰색(`C/controls/mod.rs:436,531`) · 아바타 팔레트 8색(`C/avatar.rs:12-21`) · `fallback_file_icon`의 호박색/회색(`C/controls/mod.rs:687-707`).
- `Theme`는 `Copy`인 평범한 구조체 — 앱이 필드를 덮어써 사용자 색 설정을 반영할 수 있다(nexa-sql은 `self.theme`를 복사해 쓴다 — `nexa-sql/crates/nexa-sql/src/app/paint.rs:58`). 토큰 키는 "안정 계약 — rename 시 마이그레이션 표"(`C/theme.rs:4`).
- **nexa-dir2 `Theme`에 있고 nexa-ui `Theme`에 없는 토큰**: `tab_bar_bg`·`header_bg`·`bottom_dock_bg`·`status_bar_bg`(`nexa-dir2/crates/nexa-gui/src/theme.rs:35,37,41,43`) → §5-6 UIC-314.

### 4-2. 디자인 토큰 사용 규칙

| 규칙 | 내용 | 근거 |
|---|---|---|
| 간격 | 4px 그리드 6단(`XS 4`~`XXL 32`) 밖의 값을 쓰지 않는다 · 계산 결과는 `space::snap` | `C/tokens.rs:13-33` |
| 반경 | 창 12 · 패널 10 · 컨트롤 6 · pill 999(= 높이 절반) | `C/tokens.rs:35-45` |
| 상태 표현 | **색을 새로 만들지 않는다** — 기존 색 위에 같은 색을 알파로 덮는다: `ctx.state_layer(rect, overlay_color(theme, on_accent), State::of(...))` 또는 `fill_rect_alpha(rect, color, hover_alpha(selected, fade.value(i)))` | `C/tokens.rs:67-71,325-339,697-707` · `C/draw.rs:152-160` |
| hover | 느리게 들어오고(행 1000ms · 버튼 500ms) 빨리 나간다(220ms). **눌림은 페이드 없이 즉시** | `C/tokens.rs:172-185` |
| hover 의도 | 커서가 거쳐 가는 대상마다 효과를 켜지 않는다 — `IntentFade`(70ms 머문 마지막 목표만) | `C/tokens.rs:570-579` |
| 그림자 | 두 겹(먼 것 → 가까운 것) · 본체보다 먼저 | `C/tokens.rs:132-162` · `C/draw.rs:162-175` |
| 팝업 애니메이션 | 120ms 이하(시험으로 고정) · `reduce_motion`이면 0(`motion::effective` + `nexa_sys::reduce_motion()`) | `C/tokens.rs:164-196,978-990` |
| 글꼴 크기 | 절대 크기를 박지 않는다 — 슬롯 + 증분(`select_font_sized`) | `C/draw.rs:49-55` |
| 컨트롤 글리프 크기 | `self.s(ctl_size(논리))`(DPI 배율 × 사용자 컨트롤 크기 배율) | `C/controls/switch.rs:62-70` · `C/controls/mod.rs:145-151` |
| 선행 아이콘 | `LEADING_ICON = 13` 논리 px 단일 원천 | `C/controls/mod.rs:154-155` |

---

## 5. 새 컨트롤 추가 절차와 테스트 관례

### 5-1. 어디에 무엇을 두는가

| 종류 | 위치 | 등록 |
|---|---|---|
| `Widget`+`Control` 컨트롤(버튼류 · 입력류) | `nexa-ui/crates/nexa-ctl/src/controls/<이름>.rs` | `controls/mod.rs`에 `pub mod <이름>;`(`C/controls/mod.rs:16-40`) + `pub use <이름>::{…};`(`:42-74`) · 필요하면 `lib.rs` 루트 재수출(`C/lib.rs:47-63`) |
| 호스트가 기하를 주는 "부품"(스크롤바·스플리터·플래시 형) | 같은 폴더 | 같음 — `Widget`을 구현하지 않고 `on_event(...) -> 결과` · `tick(now_ms) -> bool` · `paint(&self, ctx, theme, …)` 형태 |
| 순수 로직 부품(타입어헤드·조합기·페이드) | `nexa-ctl/src/<이름>.rs` 또는 `tokens.rs` | `lib.rs`에 `pub mod` |
| 그리기 어휘 확장 | `C/draw.rs`에 **기본 구현이 있는** 메서드로 추가(기존 백엔드·`ProbeCtx`가 깨지지 않게) + `C/raster.rs`에 실제 구현 | — |
| 픽셀 수준 기능(디코더·블렌드) | `nexa-gfx` | `G/lib.rs` |
| OS 신호·OS 전용 FFI | `nexa-sys`(외부 crate 0 · 수동 extern · 실패 = `None` · 다른 OS는 같은 모양의 빈 구현) | `S/lib.rs` |
| 파일시스템·셸(휴지통·아이콘·감시) | `nexa-fs`(OS 분기는 여기만 — `nexa-ui/Cargo.toml:15`) · 대화상자 조립은 `nexa-dlg` | 범위 밖(해당 카탈로그 문서) |

### 5-2. 최소 골격(본보기 = `Switch` — `C/controls/switch.rs` 전체 284줄)

```rust
use super::{Control, ControlBase};
use crate::draw::{DrawCtx, FontSlot};
use crate::event::{InputEvent, Key};
use crate::geom::{Point, Rect};
use crate::theme::Theme;
use crate::widget::{Invalidations, Widget};

#[derive(Debug)]                                  // UIC-004: Debug 필수
pub struct MyCtl { base: ControlBase, value: bool, changed: bool }

impl Control for MyCtl {                           // 구현 2줄로 포커스 링·도움말·배율·정렬·직전값을 물려받는다
    fn base(&self) -> &ControlBase { &self.base }
    fn base_mut(&mut self) -> &mut ControlBase { &mut self.base }
}

impl Widget for MyCtl {
    fn bounds(&self) -> Rect { self.base.bounds }
    fn set_bounds(&mut self, b: Rect, inv: &mut Invalidations) { self.base.bounds = b; inv.push(b); }
    fn on_event(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        match *ev {
            InputEvent::MouseDown { x, y, .. } if self.base.bounds.contains(Point { x, y }) => {
                self.value = !self.value; self.changed = true; inv.push(self.base.bounds);   // W-2
            }
            InputEvent::Key { key: Key::Space | Key::Enter, .. } if self.base.focused => { /* … */ }
            _ => {}
        }
    }
    fn paint(&self, ctx: &mut dyn DrawCtx, theme: &Theme) {
        if self.base.bounds.is_empty() { return; }                 // W-9
        self.draw_focus_ring(ctx, theme, self.base.bounds);
        ctx.select_font(FontSlot::Base, false);                    // 페인트 시작에 자기 슬롯 선택(C/draw.rs:14)
        let ty = ctx.text_center_y(self.base.bounds.y, self.base.bounds.h);
        ctx.text(self.base.bounds.x + self.s(8), ty, self.base.bounds, "…", theme.text);
    }
}

impl MyCtl {
    pub fn take_changed(&mut self) -> Option<bool> {               // W-1: 1회성 폴링
        std::mem::take(&mut self.changed).then_some(self.value)
    }
}
```

### 5-3. 체크리스트(새 컨트롤이 지켜야 하는 것)

| # | 항목 | 근거 |
|---|---|---|
| N-1 | 앱 도메인 타입·문자열을 넣지 않는다 — 라벨은 생성자/세터 인자 또는 주입(`PickerLabels` 식 구조체, `CtlMsg` 확장) | `C/lib.rs:5-7` · `C/controls/mod.rs:106-108` · `D/TODO.md:22` |
| N-2 | OS API·창·클립보드를 부르지 않는다 — 요청을 enum으로 남기고 호스트가 수행 | `C/lib.rs:23-24` |
| N-3 | 색은 `Theme`에서 · 간격/반경/모션은 `tokens`에서 · 치수는 `self.s(논리)` · 글리프 치수는 `ctl_size` | §4-2 |
| N-4 | hover는 `IntentFade`/`HoverFade` + `hover_alpha`, 눌림은 즉시 · `tick(now_ms)` 공개 | UIC-112~115 |
| N-5 | 한 줄 글은 `text_center_y`로 놓는다(상자 가운데 금지 — mac/Windows 글꼴 차) | `C/draw.rs:104-111` |
| N-6 | 측정값(물리 px)으로 글꼴 증분(논리 px)을 만들 때 배율로 나눈다 | `D/STATUS.md:5` |
| N-7 | **떠 있는 것(메뉴·드롭다운·툴팁)**: 위치는 `geom::place_popup`/`place_popup_beside` · 이미 놓인 것은 `nudge_into` · 영역은 `popup_host(host, ctx.surface_size())` — 컨트롤 안에서 위치를 따로 계산하지 않는다. 패널 폭에 가두지 말고 **창 전체를 host로**, 창의 **팝업 층(맨 뒤에 그림)** 에서 그린다. paint 시점에 `surface_size()`로 한 번 더 안으로(안전망) | `nexa-ui/CLAUDE.md` §3-0 · `C/geom.rs:119-185` · `C/draw.rs:35-39,261-278` |
| N-8 | 팝업이 열려 있는 동안의 입력 독점·z 순서는 **호스트가 관리**한다(`Overlay` z 스택은 아직 없음 — F-2 ☐) | `D/TODO.md:20` |
| N-9 | 빈 bounds = 그리지 않음 · `enabled == false` = 흐리게 + 입력 무시 | `C/controls/switch.rs:167-171` · `C/controls/mod.rs:237,294-300` |
| N-10 | 긴 글·경로는 `ellipsize_middle`(Alt = 전체 보기 스위치 존중) | UIC-073 |
| N-11 | 큰 목록은 보이는 행만 그린다(가상화) · 자주 도는 길(키·그리기·틱)에서 전체 복사 금지 | `nexa-ui/CLAUDE.md` §3-1 · `C/view_mode.rs:4-5` |
| N-12 | 공개 타입에 `Debug`(또는 사유 주석과 `#[allow(missing_debug_implementations)]`) · `unwrap` 금지(시험 제외) · `#[must_use]` 관례 | UIC-004 · `C/raster.rs:18,59` |
| N-13 | 공개 API를 바꾸면 커밋 본문에 `영향: nexa-sql | clip | beep` + 같은 작업 안에서 nexa-sql·`nexa-dlg` 빌드·테스트(지금 nexa-ctl 소비자). **dir3가 소비자가 되면 여기에 추가된다** | `nexa-ui/CLAUDE.md` §3 · §3-1 마지막 줄 |
| N-14 | 외부 crate 추가는 `docs/10 §3` 원장 등재(현재 `ab_glyph`·`memmap2` 둘뿐) | `D/10-decision-record.md:31-36` |
| N-15 | push 전 `scripts/check-3os.sh`(fmt + 3타깃 clippy `-D warnings`) · nexa-ui를 nexa-sql보다 **먼저** push(path 의존) | UIC-297 · `nexa-ui/CLAUDE.md` §3-1 |

### 5-4. 테스트 관례

| 관례 | 내용 | 본보기 |
|---|---|---|
| 위치 | 같은 파일 하단 `#[cfg(test)] mod tests` · 통합 테스트 폴더 없음(nexa-ctl) · 테스트 벡터는 `crates/nexa-gfx/tests/*.hex`를 `include_str!` | `C/controls/switch.rs:206-284` · `G/image.rs:729-730` · `G/jpeg.rs:531` |
| 이름 | 동작을 문장으로: `click_toggles_once` · `drag_reports_new_start_along_axis` · `wheel_wakes_only_its_axis` | `C/controls/switch.rs:226` · `C/controls/splitter.rs:162` · `C/controls/scroll.rs:863` |
| 주석 | 시험 위 `///`에 **왜**(회귀의 날짜·증상)를 한글로 | `G/surface.rs:523-524` · `C/tokens.rs:932-933` |
| 이벤트 주입 | 헬퍼 `click(x, y)`·`wheel(d)`·`down/mv/up()`으로 `InputEvent`를 만들어 `on_event`에 직접 | `C/controls/switch.rs:216-223` · `C/controls/scroll.rs:708-724` |
| 시각 주입 | `tick(가짜 ms)` · `*_at(…, Instant)` 변형으로 벽시계 없이 검증 | `C/tokens.rs:1003-1010` · `C/controls/scroll.rs:131,291` · `C/controls/flash.rs:83` |
| 측정 백엔드 | `ProbeCtx`(폭 = 문자수×7) · 계약 검증용 비선형 목업(`Quirky`: 폭 = n²×3) | `C/controls/mod.rs:783-796` · `C/draw.rs:366-376` |
| 픽셀 검증 | `Vec<u32>` 버퍼 + `Surface::new`로 직접 그려 픽셀 비교(최적화 경로 == 기준 경로) | `C/raster.rs:767-803` · `G/surface.rs:474-487` |
| 팝업 배치 | 새 떠 있는 컨트롤 = `geom.rs popup_tests`에 배치 사례 추가 + 호스트 앱에서 창 모서리 근처 캡처 1장 | `nexa-ui/CLAUDE.md` §3-0 · `C/geom.rs:187-256` |
| 토큰 불변식 | 수치 규칙 자체를 시험으로 고정(간격 4의 배수 · 오버레이 단조 · 그림자 두 겹 · 팝업 ≤120ms는 `const` 단언) | `C/tokens.rs:909-999` |
| 전역 스위치 | 프로세스 전역을 만지는 시험은 **정적 뮤텍스 가드로 직렬화**(시험은 병렬) — `DELAY_LOCK`/`lock_delay()` · nexa-font `tests::GdiOn`. 가드 없이 켜고 끄다 CI windows에서만 4회 실패한 전례 | `C/controls/scroll.rs:726-731` · `nexa-ui/CLAUDE.md` §3-1 · `D/STATUS.md:42` |
| 자료 구조 변경 | 단순 모델과 난수 대조 테스트(자체 xorshift · 한글·이모지·개행 포함) · 수치는 `--release` 벤치(`examples/bench_editor`·`bench_undo`) | `nexa-ui/CLAUDE.md` §3-1 |
| OS 무관 패닉 금지 | OS 신호 시험은 "어느 OS에서든 패닉 없이 값 또는 None" | `S/lib.rs:377-383` · `S/input_source.rs:150-160` |
| CI 실패 | 로그를 본 뒤에 고친다(`gh run view <id> --log-failed`) | `nexa-ui/CLAUDE.md` §3-1 |
| 규모 | 102차 기준 시험 382(STATUS 기재 — 이 조사에서 실행하지 않음) | `D/STATUS.md:5` |

### 5-5. 문서·커밋 규약(nexa-ui 저장소 쪽)

- 문서·커밋/푸시 규약 SSOT = `nexa-ui/docs/16-doc-git-conventions.md`(이 조사에서 본문은 읽지 않음 — 제목·존재만 확인). `git add -A`·`git add .` 금지, `git add <파일>`만(`nexa-ui/CLAUDE.md` §3).
- STATUS는 시간 역순 한 줄 요약 + `N차`·시험 수 · 상세는 `docs/journal/`(`D/STATUS.md:1-5`).
- nexa-ui CLAUDE.md는 "push는 사용자 명시 요청 시에만"이라고 적는다(`nexa-ui/CLAUDE.md` §3) — dir3 작업에서 nexa-ui를 push할 때는 이번 사용자 요청(최신화·push·태그)이 그 명시 요청에 해당하는지 오케스트레이터가 판단할 것.

### 5-6. nexa-dir3 이식 관점의 갭(이 범위에서 확인된 "없는 것")

> dir2 쪽 사실은 `nexa-dir2/crates/nexa-gui/src/draw.rs`·`theme.rs`·`event.rs`를 grep/부분 열람으로 확인했다(전수 조사는 `14-dir2-gui-widgets.md`·`17-dir2-rendering-text.md` 몫).

| ID | 갭 | dir2 쪽 | nexa-ui 쪽 | 대응 방향(제안) |
|---|---|---|---|---|
| UIC-310 | 클립 스택 | `DrawCtx::push_clip`/`pop_clip`(가로 스크롤 콘텐츠의 왼쪽 클립 보장 — `nexa-dir2/crates/nexa-gui/src/draw.rs:40-48`) | 없음(§3-1) | `DrawCtx`에 기본 no-op 메서드로 추가 + `RasterCtx`에 클립 rect 스택 구현, 또는 위젯에서 `intersection`으로 사전 클립 |
| UIC-311 | 터미널 셀 그리기 | `term_cell_w`·`term_text`(모노스페이스 셀 격자 — `…/draw.rs:58-66`) | 없음. 대체 수단 = `FontSlot::Mono` + `text_opaque` + `text_width("0")`(셀 폭) — 단 `Mono` 크기는 `status` 칸을 따르고 광학 보정이 걸린다(§3-2) | 터미널 위젯용 전용 슬롯/메서드 추가 여부 결정 필요(셀 격자 정렬에는 정수 전진 폭이 필요 — `text_snap` 또는 고정폭 실측) |
| UIC-312 | 키 기반 아이콘·경로 기반 이미지 | `draw_icon(x, y, size, key, hint) -> bool`·`draw_image(rect, hint)`(백엔드가 로드·캐시 — `…/draw.rs:71-81`) | 없음. `image(x, y, &IconImage, clip)`·`image_scaled`만 — **로드·캐시는 호출자(앱/`nexa-fs::shell`)** | 아이콘 캐시를 앱/`nexa-fs`에 두고 `IconImage`를 넘기는 구조로 변환 |
| UIC-313 | 이탤릭·밑줄·취소선 | `select_font(slot, bold, italic)`(`…/draw.rs:25`) | `select_font(slot, bold)` — italic 인자 없음(슬롯 설정 `SlotFont.italic`로만 가능). D-3·U-5 미결 | D-3 결정(합집합 권장) 후 `DrawCtx` 확장 |
| UIC-314 | 테마 토큰 | `tab_bar_bg`·`header_bg`·`bottom_dock_bg`·`status_bar_bg`(`nexa-dir2/crates/nexa-gui/src/theme.rs:35-43`) · `Color{r,g,b}` 구조체(`:8-10`) | 해당 토큰 없음 · `Color(u32)` | `Theme`에 토큰 추가(안정 계약 — 추가는 호환) 또는 `chrome_bg`/`panel_bg_alt`로 매핑. nexa-sql의 설정 구조를 따른다면 앱 쪽 색 설정 → `Theme` 덮어쓰기 |
| UIC-315 | 글꼴 슬롯 | `FontSlot::{Base, List, Status}`(`…/draw.rs:11-19`) | `Base/PeerList/Message/Status/Mono` — `List` 없음 | `PeerList`를 파일 목록 슬롯으로 쓰거나(이름 불일치) 슬롯 추가/개명(공개 API 변경 — 영향 표기) |
| UIC-316 | 입력 어휘 | (dir2 `InputEvent`도 같은 9종 — `nexa-dir2/crates/nexa-gui/src/event.rs:40-81`) | 가운데 버튼·더블클릭·X버튼·DnD·IME 사건 없음(§2-4) | 호스트(winit)에서 직접 처리 + 컨트롤 전용 메서드. 파일 드래그 앤 드롭은 OS 분기 필요(범위 밖) |
| UIC-317 | 이미지 형식 | WIC 디코드(미리보기 — `…/draw.rs:68-70` 주석) · SVG/ICO(dir2 앱 쪽 — `17-dir2-rendering-text.md` 참조) | PNG·BMP·GIF 첫 프레임·기저 JPEG만(§3-4) | nexa-gfx에 ICO·SVG 서브셋·프로그레시브 JPEG 추가(외부 crate 0 원칙) 또는 플러그인(WASM) 미리보기로 위임 — 결정 필요 |
| UIC-318 | 표면 stride·부분 무효화 | — | `Surface`는 행 간격 = 폭 고정 · STATUS에 "남은 것 = `Surface` stride"(`D/STATUS.md:50`). `Invalidations`는 있으나 nexa-sql 호스트는 매 프레임 전체를 다시 그리는 형태로 보인다(**추정** — `paint.rs:68`의 전체 배경 채움만 확인) | 큰 창(4K)에서 프레임 비용 측정 후 부분 그리기 도입 여부 결정 |
| UIC-319 | 복합 문자·이모지 | (dir2 = DirectWrite) | 셰이핑 없음(§3-3) — 파일 이름에 결합 문자·아랍어·이모지가 오면 낱자/두부로 그려질 수 있다(**추정**) | 폴백 체인(기호 글꼴)으로 두부만 줄이고 한계로 문서화, 또는 U-4 셰이퍼 |
| UIC-320 | 도킹·그리드·오버레이 | dir2 `widgets/{dock,rows,columns,chrome,overlaybar,pathbar}`(`nexa-dir2/crates/nexa-gui/src/widgets/` 목록) | dock ☐ · `nexa-grid` ☐ · `Overlay` ☐(UIC-300) — tabbar·menubar(pulldown)는 이식됨 | 컨트롤 카탈로그 문서와 교차 확인 후 nexa-ui에 추가 |
| UIC-321 | Linux 입력 소스·present 가속 | — | `is_korean()` = `None` · `LayerPresenter` = 없음(softbuffer만) | Linux는 IME 경로 유지(추정) · present는 softbuffer |
| UIC-322 | `MenuIcon`/글리프의 스레드 제약 | — | `Rc` 기반 · 스레드 로컬 캐시(UIC-193) — 워커 스레드에서 만든 아이콘을 UI로 넘길 수 없다 | 워커는 `IconImage`(Send) 또는 원시 바이트를 넘기고 UI 스레드에서 `MenuIcon`화 |
| UIC-323 | `set_ctl_labels` 1회 주입 | — | `OnceLock` — 두 번째 호출은 무시(UIC-170). 공급자 함수가 앱 i18n의 `t()`를 부르면 언어 전환은 자동 반영 | 부팅 시 1회 주입 + 공급자 안에서 현재 언어 조회 |
