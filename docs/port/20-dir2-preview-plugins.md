# 20. nexa-dir2 인벤토리 — 미리보기 · 플러그인(WASM) · 압축 목록

> 접두사 **PLUG-NNN**. 이 문서의 ID는 이후 구현·교차 검증의 체크리스트다.
> "추정"이라고 적은 항목은 소스를 열어 확인하지 못한 것이다.
>
> **경로 줄임 규약**(모든 근거는 아래 접두를 붙여 `저장소/경로:줄`로 읽는다)
>
> | 줄임 | 실제 경로 |
> |---|---|
> | `pv/…` | `nexa-dir2/crates/nexa-app/src/preview/…` (`mod.rs`·`wasm.rs`·`archive.rs`·`sample_tests.rs`) |
> | `app/…` | `nexa-dir2/crates/nexa-app/src/…` (`previewwnd.rs`·`archivewnd.rs`·`pwprompt.rs`·`win.rs`·`dw.rs`·`prefs.rs`·`config.rs`·`clipboard.rs`·`fontchain.rs`·`svg.rs`·`ctl/gdipctx.rs`·`ctl/grid.rs`) |
> | `vfs/…` | `nexa-dir2/crates/nexa-vfs/src/archive/…` |
> | `gui/dock.rs` | `nexa-dir2/crates/nexa-gui/src/widgets/dock.rs` |
> | `core/secret.rs` | `nexa-dir2/crates/nexa-core/src/secret.rs` |
> | `md/…` | `nexa-dir2/samples/markdown-viewer-wasm/…` |
> | `arc/…` | `nexa-dir2/samples/archive-viewer-wasm/…` |
> | `d2docs/…` | `nexa-dir2/docs/…` |
> | `ui/…` | `nexa-ui/crates/…` |
> | `sql/…` | `nexa-sql/crates/…` |

## 0. 범위 — 읽은 파일과 줄 수

| 대상 | 줄 수 | 읽은 범위 |
|---|---:|---|
| `pv/mod.rs` | 427 | 전부 |
| `pv/wasm.rs` | 844 | 전부 |
| `pv/archive.rs` | 435 | 전부 |
| `pv/sample_tests.rs` | 222 | 전부 |
| `app/previewwnd.rs` | 980 | 전부 |
| `app/archivewnd.rs` | 430 | 전부 |
| `vfs/mod.rs` | 719 | 전부 |
| `vfs/zip.rs` | 468 | 전부 |
| `vfs/rar.rs` | 535 | 전부 |
| `vfs/tar.rs` | 465 | 전부 |
| `vfs/stream.rs` | 262 | 전부 |
| `vfs/cab.rs` | 226 | 전부 |
| `vfs/sevenz.rs` | 114 | 전부 |
| `md/src/lib.rs` · `md/src/mm.rs` | 461 · 537 | 전부 |
| `md/Cargo.toml` · `md/README.md` · `md/fixtures/sample.md` · `md/.gitignore` | 20 · 42 · 47 · 1 | 전부 (`md/dist/markdown.wasm` = 81,635 B 바이너리 — 크기만) |
| `arc/src/lib.rs` | 384 | 전부 |
| `arc/Cargo.toml` · `arc/README.md` · `arc/.gitignore` | 20 · 86 · 1 | 전부 (`arc/dist/archive.wasm` = 31,218 B · `arc/fixtures/sample.a` 153 B · `sample.cpio` 384 B — 크기·선두 바이트만) |
| `nexa-dir2/scripts/build-plugins.ps1` | 64 | 전부 |
| `d2docs/09-adr-0004-preview-plugins.md` · `24-plugin-dev-guide.md` · `25-adr-0005-wasm-plugins.md` · `28-archive-preview.md` | 90 · 261 · 48 · 142 | 전부 |
| **담당 범위 밖(호출·의존 확인용)** | | |
| `app/pwprompt.rs` | 305 | 전부(압축 그리드 창이 호출 — docs/28 계층표에 포함) |
| `app/win.rs` | — | 2366~2529(도크 미리보기·F3), 6684~6690(설정 반영), 8563~8570(↗ 발화), 8825~8833(F3 키) |
| `app/dw.rs` | — | 507~583(`image_scaled` WIC), 737~779(`draw_image`) |
| `gui/dock.rs` | 1407 | 78~82, 341~412(↗ 버튼), 818~957(paint) |
| `app/prefs.rs` | — | 1786~1845(플러그인 페이지), 2078~2088(수확) |
| `app/config.rs` | — | 144~147, 292~293, 358~401(`data_dir`), 462~466, 706~708 |
| `app/clipboard.rs` | — | 822~896(`write_text`·`write_text_rich`), 961~982(`to_rtf_mono`) |
| `app/fontchain.rs` | 270 | 1~125 + 공개 시그니처 |
| `app/svg.rs` · `app/ctl/gdipctx.rs` | 692 · 647 | 1~135 · 100~250, 312~386 |
| `app/ctl/grid.rs` | 1156 | 1~135 + 공개 시그니처 |
| `core/secret.rs` | 164 | 1~110 |
| `nexa-dir2/crates/nexa-app/lang/ko.lang` | — | `preview.*`·`archive.*`·`pref.plugins.*` 키 Grep(20~23, 168~178, 508~542) |
| `d2docs/18-build-and-test.md` §3-1 · `21-distribution.md` §5-2 · `.github/workflows/{ci,release}.yml` · `installer/nexa.iss` | — | 플러그인 관련 구간만 |
| `sql/nexa-sql/src/extensions/{mod.rs 517, wasm.rs 600}` | — | `mod.rs` 전부 · `wasm.rs` 1~470 · `manager.rs` 1~60 + 시그니처 Grep |
| `sql/nexa-sql/src/{clipboard.rs, grid.rs, input_win.rs, ext_panel.rs}` | — | 머리 주석 + 시그니처 Grep |
| `ui/nexa-ctl/src/**` · `ui/nexa-gfx/src/{image,jpeg,surface}.rs` · `ui/nexa-conf/src/lib.rs` | — | 공개 API Grep · `draw.rs` 1~180 · `tree.rs` 1~140 · `image.rs` 1~130 |

기존 테스트 합계(이 범위): **앱 19개**(`pv/mod.rs` 4 · `pv/wasm.rs` 5 · `pv/archive.rs` 5 · `pv/sample_tests.rs` 2 · `archivewnd.rs` 3 · `previewwnd.rs` 0) + **nexa-vfs 45개**(`mod` 10 · `zip` 8 · `rar` 6 · `tar` 11 · `stream` 4 · `sevenz` 3 · `cab` 3) = 64개.

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 필요 / **W**=Windows 전용 유지(타 OS는 대체·비활성).

### 1.1 미리보기 시임(공급자 레지스트리)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-001 | 공급자 계약 | `PreviewProvider { id(), exts(), preview(path) }` · 산출물 `PreviewDoc::{Lines(Vec<String>), Image(String 경로), Archive(Box<ArchiveDoc>)}`. 내장 id = `builtin.*` | `pv/mod.rs:19-39` | 없음(단 `Image(String)`은 `to_string_lossy` — 비UTF-8 경로 손실) | N | 간접(`pv/mod.rs:334-365`) |
| PLUG-002 | 확장자→공급자 결정 | 우선순위 = **`preview_map` 오버라이드 > 선언 매치(로드 순) > `builtin.text` 폴백**. `preview_map` = `ext:id\|ext:id`(확장자 대소문자 무시, 무효 id는 무시하고 선언 매치 유지). 확장자 = 소문자·점 없음. 확장자 없는 파일은 오버라이드 건너뜀 | `pv/mod.rs:208-247` | 없음 | N | `preview_map_overrides_declared_match`(`pv/mod.rs:353`) |
| PLUG-003 | 플러그인 사용 안 함 | `plugins_disabled` = `id\|id`. **`builtin.`으로 시작하는 id는 면역**(끌 수 없음). 꺼진 플러그인은 오버라이드에 지정돼도 건너뜀 → 다음 매치/텍스트 폴백 | `pv/mod.rs:203-206`, `218` | 없음 | N | `disabled_plugin_is_skipped_but_builtin_immune`(`pv/mod.rs:403`) |
| PLUG-004 | 지연 로드·캐시 | `with_providers` = **스레드 로컬 `OnceCell`** — 미리보기 최초 사용(또는 설정 플러그인 페이지 진입) 시 1회 구성. 순서 = 플러그인(경로 순·파일명 순) → 내장. **수정 반영 = 앱 재시작**(핫 리로드 없음) | `pv/mod.rs:284-322` | 없음 | N | 없음 |
| PLUG-005 | 플러그인 탐색 경로 | ① `data_dir()/plugins`(사용자 설치분 — 포터블 = exe 옆 `data\`, 설치형 = `%LOCALAPPDATA%\NexaDir\data`) → ② `<exe 폴더>/plugins`(동봉분·읽기 전용). 중복 경로 제거. **같은 id는 앞선 것만 채택** | `pv/mod.rs:263-282`, `296-304` · `app/config.rs:358-401` | `current_exe()` · `%LOCALAPPDATA%` | **P** | `plugin_dirs_lists_user_first_then_bundled_without_duplicates`(`pv/mod.rs:385`) |
| PLUG-006 | 내장 텍스트 미리보기 | 첫 **16KB**(`TEXT_READ_CAP`) 읽기 → 앞 1024B에 NUL 있으면 이진 판정 → `preview.binary` 1줄. 빈 파일 = `preview.empty`, 열기 실패 = `preview.fail`. 최대 **200줄**, 탭 → 공백 4칸, `from_utf8_lossy` | `pv/mod.rs:46-58`, `142-171` | 없음 | N | `declared_ext_routes_and_text_falls_back`(`pv/mod.rs:335`) |
| PLUG-007 | 내장 이미지 미리보기 | 선언 확장자 9종 `png jpg jpeg bmp gif ico tif tiff webp` → `PreviewDoc::Image(경로)`만 반환(디코드는 표시 백엔드) | `pv/mod.rs:41-44`, `173-188` | WIC 인박스 코덱에 의존(표시측 PLUG-043) | **P** | 위와 같음 |
| PLUG-008 | 내장 공급자 순서 | `builtins()` = **archive → image → text**("구조를 아는 공급자 우선"). text는 선언 확장자 0개 = 폴백 전용 | `pv/mod.rs:190-201` | 없음 | N | `archive_ext_routes_to_builtin_provider`(`pv/archive.rs:383`) |
| PLUG-009 | 테마 신호 주입 | `set_dark(bool)` 스레드 로컬(기본 true) — 미리보기 호출 직전 호스트가 주입, 플러그인 `is_dark()`가 읽음 | `pv/mod.rs:60-74` · 호출 `app/win.rs:2376`, `2441` | 없음 | N | 없음 |
| PLUG-010 | 표시 폭 계산 | `disp_width_impl` — CJK·이모지 범위 2칸, 나머지 1칸(범위표 13구간) | `pv/mod.rs:76-95` | 없음 | N | 없음(샘플 `dw`가 같은 표 — `md/src/lib.rs:74-91`) |
| PLUG-011 | SVG → 이미지 캐시 | `render_svg_impl(svg)`: 256KB 초과 거부 → `svg::parse` → **GDI+ 래스터**(`svg_to_pixels`, viewBox 크기·2000px 상한) → 알파 0xFF 고정 → `temp_dir()/nexa-preview/d{DefaultHasher:016x}.bmp`(32bpp top-down BI_RGB, 없을 때만 기록) → 경로 반환. **비Windows = 항상 `None`** | `pv/mod.rs:97-140` · `app/ctl/gdipctx.rs:114-150` · `app/svg.rs:105` | GDI+ (`GdipCreateBitmapFromScan0`·`GdipAddPath*`·글꼴 `Arial` `gdipctx.rs:329`) | **P** | 없음(E2E가 3단 폴백으로 간접 — `pv/sample_tests.rs:46-54`) |
| PLUG-012 | 상한 읽기 공용 | `read_text(path, cap)` → `(lossy 문자열, 이진 여부)`. 읽기 실패는 0바이트 취급, 열기 실패만 `Err` | `pv/mod.rs:49-58` | 없음 | N | 간접 |
| PLUG-013 | 플러그인 메타 목록 | `plugin_infos()` → `PluginInfo { id, name, exts }` — 설정 "플러그인" 페이지 원천 | `pv/mod.rs:249-261` | 없음 | N | 없음 |

### 1.2 WASM 런타임(wasmi 1.1 · ABI v1/v2)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-020 | 모듈 로드 | 디렉터리의 `*.wasm`(확장자 대소문자 무시)을 **파일명 정렬 순**으로 로드. 모듈 **8MB 초과 거부**. `Config::consume_fuel(true)`. `nx_meta()` 호출(대상 경로 = 빈 경로) → 줄 파싱: 1=id(필수·없으면 거부) 2=name(없으면 id) 3=확장자 쉼표 목록(선두 `.` 제거·소문자) **4=능력(caps) 쉼표 목록** | `pv/wasm.rs:324-398` | 없음 | N | `loads_meta_runs_preview_and_fuel_traps_infinite_loop`(`pv/wasm.rs:686`) |
| PLUG-021 | 버퍼 규약 | 게스트 반환 = 포인터 → **선두 4바이트 LE 길이 + UTF-8 본문**. 길이 > `OUT_CAP`(1MB) = "반환 버퍼 손상". `memory` export 필수. **호스트→게스트 입력 버퍼 없음**(`nx_alloc` 불필요 — 게스트가 import로 당겨 간다) | `pv/wasm.rs:104-112`, `314-321` | 없음 | N | 위와 같음 |
| PLUG-022 | 호출 격리 | **호출마다 새 Store/인스턴스**(상태 없음). `FUEL` 200,000,000 · 선형 메모리 `MEM_CAP` 64MB(StoreLimits) · 벽시계 `CALL_TIMEOUT_MS` 1,500ms(마감 시각). 오류 = 문자열(`Err(String)`) | `pv/wasm.rs:34-57`, `288-322` | 없음 | N | 연료 트랩·시간 상한 검증(`pv/wasm.rs:706-717`) |
| PLUG-023 | 호스트 임포트 게이트 | `host_guard(cost)`: ① 마감 초과 → 트랩 ② **호스트 작업 비용을 연료에서 차감**(부족 = 트랩). 순수 게스트 루프는 연료가, 임포트 루프는 연료+벽시계가 막는다 | `pv/wasm.rs:70-84` | 없음 | N | `nx_hostloop` 검증(`pv/wasm.rs:709-717`) |
| PLUG-024 | import `env.read_text(ptr, cap) -> len` | **대상 파일 앞부분**만. `cap`은 256KB(`READ_CAP`) 클램프. 연료 200,000. 실패 = 0 | `pv/wasm.rs:117-138` | 없음 | N | 간접(샘플 E2E) |
| PLUG-025 | import `env.render_svg(sptr, slen, optr, ocap) -> len` | SVG(최대 256KB) → PLUG-011 → **BMP 경로 문자열**을 게스트 버퍼에 기록. 연료 5,000,000(가장 비쌈). 실패/비Windows = 0 | `pv/wasm.rs:139-170` | PLUG-011 경유 | **P** | 간접 |
| PLUG-026 | import `env.is_dark() -> i32` | 테마 신호. 연료 1,000 | `pv/wasm.rs:171-179` | 없음 | N | WAT 모듈이 import(`pv/wasm.rs:560`) |
| PLUG-027 | import `env.disp_width(ptr, len) -> i32` | 최대 4096B 읽어 PLUG-010. 연료 10,000 | `pv/wasm.rs:269-284` | 없음 | N | 없음 |
| PLUG-028 | import `env.file_size() -> i64` (v2) | 대상 파일 크기, 실패 = -1. 연료 10,000 | `pv/wasm.rs:181-191` | 없음 | N | `archive_capability_routes_to_nx_archive_and_password_flow`(`pv/wasm.rs:641`) |
| PLUG-029 | import `env.read_at(off: i64, ptr, cap) -> n` (v2) | 대상 파일 임의 위치. 1회 `READ_AT_CAP` 4MB 클램프 · 파일 끝 너머는 유효 길이로 절단 · `off<0`=0. 연료 = 고정 1,000 + **바이트/64**. 호출당 **누적 64MB**(`READ_AT_TOTAL_CAP`) 초과 = 트랩("read_at 누적 …MB 상한 초과") | `pv/wasm.rs:42-52`, `192-248` | 없음 | N | `read_at_fuel_allows_thousands_of_members`(`:746`) · `read_at_total_bytes_cap_traps_runaway_reads`(`:799`) |
| PLUG-030 | import `env.password(ptr, cap) -> n` (v2) | **활성 암호 슬롯**(PLUG-080)만 전달. 없으면 **-1**. 호스트 임시 사본은 기록 직후 `zeroize_bytes`. 연료 10,000 | `pv/wasm.rs:249-268` | 없음 | N | 위 `:641` |
| PLUG-031 | export `nx_preview() -> ptr` | 첫 줄 `lines` → 이후 본문 줄(최대 **1000줄**, 줄당 **4096자**) = `PreviewDoc::Lines` / 첫 줄 `image` → 다음 1줄(trim) = `PreviewDoc::Image(경로)` / 그 외 = 오류 "알 수 없는 반환 종류" | `pv/wasm.rs:400-418` | `image` 경로는 표시측 디코더 의존 | N | `:686` · 샘플 E2E |
| PLUG-032 | export `nx_archive() -> ptr` (v2) | 첫 줄 `archive`\|`password`\|`error`. `password` → `ArchiveStatus::NeedPassword`. `error` → 둘째 줄 사유 = `Failed`. `archive` → 둘째 줄 `표시명<TAB>플래그`(플래그 = `solid,multivolume,truncated`), 이후 항목 줄 `경로<TAB>원본<TAB>압축<TAB>시각(Unix초)<TAB>속성<TAB>방식`(속성 = `dir,enc,utc,unsafe`). 최대 **50,000 항목**(`ARCHIVE_CAP`). 경로는 `normalize_path` 통과(빈 경로 버림), 폴더는 크기 `None`, 시각 ≤0 = `None`, **`utc` 없으면 현지 벽시계** 해석, `listing.format = 플러그인 id`, `provider = 플러그인 id` | `pv/wasm.rs:420-495` | 없음 | N | `:641` · `sample_archive_plugin_lists_iso_ar_and_cpio`(`pv/sample_tests.rs:154`) |
| PLUG-033 | 공급자 어댑터 | `WasmProvider`: caps에 `archive`가 있으면 `nx_archive`, 아니면 `nx_preview`. 실행 오류 = **그 플러그인만 1줄** `preview.plugin.error`(`{id}`, `{오류}`) | `pv/wasm.rs:497-550` | 없음 | N | `breaker_disables_plugin_after_consecutive_failures`(`:814`) |
| PLUG-034 | 서킷 브레이커 | **연속 3회**(`BREAKER_LIMIT`) 실패 → 세션 동안 실행 안 함, `preview.plugin.disabled`(`{id}`,`3`) 1줄. 성공 시 0으로 복귀. `nx_archive`가 `error`/`password`를 정상 반환한 것은 **성공**으로 센다 | `pv/wasm.rs:58-59`, `514-537` | 없음 | N | `:814` |
| PLUG-035 | 로드 오류 격리 | 깨진 모듈은 그 파일만 제외하고 `errors: Vec<String>`에 `파일명: 사유`. **현재 호스트는 이 목록을 버린다**(`_errors` — 사용자에게 안 보임) | `pv/wasm.rs:373-398` · `pv/mod.rs:297` | 없음 | N | `:691-694` |

### 1.3 라인 태그 계약 · 하단 도크 표시

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-040 | 라인 종류 태그 | 줄 접두 `\u{2}h1\|` `\u{2}h2\|` `\u{2}h3\|` `\u{2}code\|` `\u{2}q\|` `\u{2}mono\|` `\u{2}hr\|` → 종류 1~7(무접두 = 0 본문) | `app/previewwnd.rs:38-56` · `d2docs/24-plugin-dev-guide.md:122-132` | 없음 | N | 샘플 E2E(`pv/sample_tests.rs:38-41`) |
| PLUG-041 | 인라인 이미지 마커 | `\u{1}img\|<경로>` + 뒤따르는 `\u{1}pad` n줄 = (n+1)행 영역 예약. 선택·복사에서 제외 | `gui/dock.rs:78-82` · `app/previewwnd.rs:475-498` | 없음(이미지 로드는 P) | N/A | 샘플 E2E(`:49-54`) |
| PLUG-042 | 도크 축약 뷰 | 단일 선택 파일(폴더·다중 선택·선택 없음 = `preview.none`)에 대해 `preview_for` → Lines: `\u{2}` 줄은 태그 제거, `hr` → `─`×40, `q` → `│ ` 접두 / Image → 이미지 표시 / Archive → 요약 | `app/win.rs:2366-2417` | 없음 | A | 없음 |
| PLUG-043 | 도크 이미지 표시 | `draw_image(area, 경로)`: WIC `CreateDecoderFromFilename` → 첫 프레임 → `(area.w-2, area.h-2)` 안 **비율 유지 축소(확대 없음)** Fant 스케일 → 32bppBGRA → 가운데 `StretchDIBits`. 캐시 키 `(경로, max_w, max_h)` **8개 상한(초과 시 전체 비움)**. 실패 = 표시 생략 | `app/dw.rs:507-583`, `737-779` · `gui/dock.rs:831-842` | WIC·GDI | **P** | 없음(`examples/preview_image.rs`는 수동 스파이크) |
| PLUG-044 | 도크 인라인 이미지 | 마커 행에서 `row_h × k`(k = 1+pad 수, 아래 경계 클램프) 영역에 `draw_image`. 가로 스크롤 불변 | `gui/dock.rs:859-885` | PLUG-043 경유 | A/P | 없음 |
| PLUG-045 | 도크 ↗ "크게" 버튼 | 미리보기 종류일 때만 표시(`set_popout`). 버튼 안에서 누르고 **안에서 뗄 때** 발화(`take_popout`) → 독립 창. 3상태 배경(pressed = panel_bg↔accent 38% / hover = sel_bg / 기본 header_bg), 아이콘 `emb:popout`(없으면 `↗` 글리프) | `gui/dock.rs:341-412` · `app/win.rs:2524-2526`, `8563-8570` | 없음 | A | `popout_button_hover_press_release_semantics`(`gui/dock.rs:1062`) · `popout_click_wins_over_flashed_bar`(`:1281`) |
| PLUG-046 | 도크 압축 요약 | `summary_lines(doc, tz, 60)`: 상태별 1~2줄(암호 필요 + 열기 안내 / 플러그인 필요 / 실패) 또는 `archive.summary`·`archive.sizes`(절감률 = `100 - packed*100/size`, 크기 0 = `-`)·암호/솔리드/분할/절단 표시·주석·`archive.openHint`·빈 줄·앞 **60개 항목**(`🔒 ` 접두, 폴더 `/` 접미, 크기, 시각)·`archive.more` | `pv/archive.rs:202-281` · `app/win.rs:2411-2420` | 없음 | N | `summary_lines_lead_with_format_and_counts`(`pv/archive.rs:397`) · `failure_status_maps_to_user_action`(`:407`) |
| PLUG-047 | 설정 변경 즉시 반영 | 설정 창에서 `plugins_disabled`가 바뀌면 `update_dock_info`로 도크 재계산 | `app/win.rs:6684-6690` | 없음 | N | 없음 |

### 1.4 독립 미리보기 창(F3 · ↗)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-050 | 열기 | **F3**(활성 패널) 또는 도크 **↗**(싱글 정보 모드면 활성 패널, 아니면 그 도크의 패널). 조건 = 단일 선택 + 폴더 아님. `Shift+F3` = 개발 벤치(별개) | `app/win.rs:2422-2472`, `8825-8833`, `8563-8570` | 없음 | N | 없음 |
| PLUG-051 | 창 생성 | 제목 `"{파일명} — Preview"`. 소유자 창의 **3/4 크기**(폭 480~1400, 높이 360~1000 클램프), 소유자 중앙. 소유자 사각형을 못 얻으면 900×640 @ (120,120). 리사이즈 가능. **모달**(소유자 비활성 + 자체 메시지 펌프, 닫히면 소유자 전경 복귀). 커서 = I-빔, 아이콘 = 앱 아이콘 32 | `app/previewwnd.rs:139-154`, `875-980` | `CreateWindowExW`·`EnableWindow`·`GetMessageW` 중첩 루프 | **P**(창 골격) + A | 없음 |
| PLUG-052 | 스타일드 렌더 | 종류별: 1~3 = 굵은 본문 글꼴(h1·h2는 행 하단 1px 괘선 전폭) · 4 = 모노 + 배경 밴드 · 5 = 흐린 글자 + 좌측 바 · 6 = 모노 · 7 = 행 중앙 1px 수평선 · 0 = 본문. 탭 → 4칸. 태그는 표시/복사 텍스트에서 제거 | `app/previewwnd.rs:457-608`, `909-919` | GDI `ExtTextOutW` | A | 없음 |
| PLUG-053 | 글꼴 체인 | 모노 = 설정 `term_font` 쉼표 체인의 설치된 첫 패밀리(기본 `Consolas`, FIXED_PITCH) + 나머지 폴백 · 본문 = `"Segoe UI, {모노 체인}"`(기본 Segoe UI) · 굵게 = weight 700. 크기 = `term_font_size`pt(8~32 클램프) × DPI/72. **글리프 단위 폴백**(1순위에 없는 문자는 다음 글꼴 런) — 측정·그리기 같은 런 규칙 | `app/previewwnd.rs:156-213` · `app/fontchain.rs:79-125`, `185-222` | GDI `CreateFontW`·`GetGlyphIndicesW`·`GetDpiForWindow` | **P**/A | 없음 |
| PLUG-054 | 인라인 이미지 | 자체 기록 **32bpp BMP만** 로드(`load_bmp`: 헤더 54B·폭/높이 ≤4096·bottom-up이면 뒤집기). 경로별 캐시. 표시 = 좌측 정렬(`PAD_X`), 예약 영역 안 세로 중앙, 스케일 = `min(가용폭/iw, 영역높이/ih, 1.0)`. **행 배경 도장 이후 마지막에 그림**(pad 행이 덮지 않게) | `app/previewwnd.rs:87-117`, `470-498`, `609-653` | `StretchDIBits` | A | 없음 |
| PLUG-055 | 스크롤(바·키) | 세로 = 줄 단위(`top`) + 부분 줄 px(`top_frac`) · 가로 = px(`left`, 상한 = 최장 줄 폭 + 2×PAD_X − 창폭). 키: ↑↓ ±1줄 · ←→ ±24px · PgUp/PgDn ±가시 줄 수 · Home = (0,0) · End = 맨 아래(가로 유지). 스크롤바 메시지: line ±1줄/±24px, page ±가시분, thumb = 위치, top/bottom | `app/previewwnd.rs:215-295`, `685-726`, `844-852` | 네이티브 `WS_VSCROLL\|WS_HSCROLL`·`SetScrollInfo` | A | 없음 |
| PLUG-056 | 휠 | Shift+휠 = 가로(노치당 120px 누적 × 고속 배수) · 노치(\|delta\|≥120) = 시스템 줄 수 × 고속 배수 · 정밀 터치패드(\|delta\|<120) = **픽셀 스크롤**(`wheel_lines × line_h` 환산) · 가로 휠(`WM_MOUSEHWHEEL`) = `left += delta`. 누적기는 스레드 공용 | `app/previewwnd.rs:258-282`, `727-770` | `WM_MOUSEWHEEL`·`WM_MOUSEHWHEEL` · 노치당 줄 수 = 호스트가 주입한 시스템 값(`SPI_GETWHEELSCROLLLINES` 존중·기본 3·1~20 클램프 — `nexa-dir2/crates/nexa-gui/src/event.rs:6-20`) · 누적기 `WheelAccum::add`(`:89-103`) | **P**(입력 원천) + A | 누적기만(`event.rs:109-123`) |
| PLUG-057 | 드래그 문자 선택 | 누름 = 앵커, 이동 = 확장, 뗌 = 확정(이동 없는 클릭 = 선택 없음). 히트 = 최근접 문자 경계. 영역 밖이면 **자동 스크롤**(세로 1줄/가로 24px, 50ms 타이머로 커서 정지 중에도 계속). 마우스 캡처. 이미지/패드 줄은 경계 0 하나(선택 불가) | `app/previewwnd.rs:297-316`, `428-451`, `771-820` | `SetCapture`·`SetTimer`·`GetCursorPos` | A | 없음 |
| PLUG-058 | Ctrl+C rich 복사 | 선택 텍스트(줄 구분 `\r\n`, 이미지/패드 줄은 빈 줄) → **평문 + 모노 RTF 동시 게시**(RTF = `\fmodern Consolas` `\fs18`, 비ASCII = `\uN?` 부호 있는 16비트·서러게이트 쌍) | `app/previewwnd.rs:389-426`, `838-843` · `app/clipboard.rs:849-896`, `961-982` | `CF_UNICODETEXT` + 등록 포맷 "Rich Text Format" | **P** | 없음 |
| PLUG-059 | 우클릭 메뉴 | 항목 = `menu.edit.copy`(선택 있을 때만 활성) · 구분선 · `menu.edit.selectAll`(텍스트 있을 때만). 전체 선택 = 첫 줄 0 ~ 마지막 줄 끝(마지막이 마커면 0) | `app/previewwnd.rs:318-387`, `821-827` | 네이티브 `TrackPopupMenuEx` | A | 없음 |
| PLUG-060 | 닫기 | Esc 또는 닫기 버튼 → 창 파괴, 글꼴 해제 | `app/previewwnd.rs:835-837`, `857-870` | — | A | 없음 |
| PLUG-061 | 이미지 문서 처리 | 공급자 결과가 `Image`면 창에는 `preview.window.image` **안내 1줄**만(이미지는 도크 담당) | `app/win.rs:2444` | — | N(개선 여지) | 없음 |
| PLUG-062 | 테마 색 | 다크: bg `#191C21` · 글자 `#D6DAE0` · 선택 `#24405F` · 흐림 `#8A919C` · 밴드 `#262B33` · 괘선 `#363C46` / 라이트: `#FFFFFF` · `#1B1F26` · `#D8E8FF` · `#57606A` · `#F6F8FA` · `#D8DEE4`(COLORREF는 BGR 표기 — 소스 값 `0x009C918A` 등) | `app/previewwnd.rs:119-135`, `508-512` | — | A(nexa-ui Theme 토큰으로 매핑) | 없음 |

### 1.5 압축 그리드 창 · 암호

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-070 | 열기 흐름 | F3/↗ 결과가 `Archive`면 텍스트 창 대신 그리드 창. 이미 읽은 `doc`을 그대로 전달(재조회 없음). `NeedPassword`인 동안 반복: 암호 창 → 취소면 **아무 창도 안 엶** → `read_via`(같은 공급자 경로)로 재시도 → `Ok`면 세션 기억, 다시 `NeedPassword`면 폐기 후 `retry=true`(틀림 안내), 그 외 상태면 루프 탈출 후 표시 | `app/win.rs:2445-2461` · `app/archivewnd.rs:158-192` | 동기 중첩 모달 | **P**(흐름을 비동기 상태기계로) | 없음 |
| PLUG-071 | 그리드 창 | 제목 `archive.window.title`(`{0} — 압축 미리보기`). 소유자 3/4(폭 560~1500, 높이 360~1000), 기본 960×640. 모달. 초기 포커스 = 그리드 | `app/archivewnd.rs:194-303` | Win32 창·펌프 | **P** + A | 없음 |
| PLUG-072 | 컬럼 8개 | (키, 기본 폭) = name 240 · path 200 · size 90 · packed 90 · ratio 60 · method 90 · modified 130 · flags 90. 지브라 켬 | `app/archivewnd.rs:40-50`, `236-253` | — | A | `cells_follow_column_order_and_blank_dirs`(`archivewnd.rs:396`) |
| PLUG-073 | 행 셀 | 이름(마지막 경로 요소) · 상위 경로 · 크기(폴더 = 빈칸, 미상 = `-`, `human_size`) · 압축 크기(같은 규칙) · 압축률 `N%`(미상 = 빈칸) · 방식 · 시각(PLUG-083) · 표시(`폴더`·`잠김`·`위험 경로`를 ` · `로 연결) | `app/archivewnd.rs:61-93` | — | N | 위와 같음 |
| PLUG-074 | 정렬 | 초기 = 경로순. 헤더 정렬 통지 → `(컬럼, desc)` 목록(다중)으로 호스트가 재정렬: 크기·압축 크기·압축률·시각 = **수치**, 이름·경로·방식 = 문자열, 표시 = `(is_dir, encrypted, suspicious)` 튜플. 동률 = 경로순. **폴더 우선 없음** | `app/archivewnd.rs:95-122`, `364-372` | — | N(비교) + A(헤더) | `sort_uses_numeric_order_for_size_and_reverses`(`:408`) · `empty_sort_spec_falls_back_to_path_order`(`:425`) |
| PLUG-075 | 상태 줄 | `archive.status`(`{포맷} · 항목 N개 · 원본 X · 압축 Y`) + 암호/솔리드/분할/절단 표시를 ` · `로 덧붙임. 실패 상태는 사유 문구 그대로. **주석(comment)은 창에 표시 안 함**(도크 요약만) | `app/archivewnd.rs:124-156` | — | N | 없음 |
| PLUG-076 | Ctrl+C TSV 복사 | 선택 행(없으면 **전체**)을 셀 탭 구분 + `\r\n` 줄로 평문 복사 | `app/archivewnd.rs:336-355`, `290-293` | 클립보드 | **P** | 없음 |
| PLUG-077 | 키·선택 | Esc = 닫기 · Ctrl+C = 복사(메시지 펌프에서 가로챔) · 그 외 = 그리드 규약(클릭 단일/Shift 연속/Ctrl 토글/Ctrl+A/Space/Home·End·PgUp·PgDn, 헤더 경계 드래그 리사이즈 최소 40px, 오버레이 스크롤바) | `app/archivewnd.rs:282-299` · `app/ctl/grid.rs:1-27`, `79-87` | — | A | 그리드 자체 테스트는 문서 16 소관 |
| PLUG-078 | 암호 입력 창 | 제목 `archive.pw.title`. 안내(파일명 포함) · (재시도 시 빨간 "암호가 맞지 않습니다") · 라벨+마스킹 입력(●, 상한 1024자) · "암호 표시" 체크(화면만 토글) · 주의 문구(흐림) · 확인(기본)/취소. Enter = 확인, Esc/닫기 = 취소. 확인 시 값을 `Secret`으로 **1회 이동**, 경유 UTF-16 버퍼·컨트롤 내용·되돌리기 버퍼 소거. 빈 값 = `None` | `app/pwprompt.rs:51-305` | Win32 창·`WM_GETTEXT` | **P** + A | 없음 |
| PLUG-079 | 세션 암호 캐시 | `pw::{get, remember, forget, forget_all, len}` — 스레드 로컬 `HashMap<PathBuf, Secret>`, **메모리 한정**(디스크·설정·로그 기록 경로 없음). 성공한 암호만 기억. `forget_all`은 운영 코드에서 호출처 없음(테스트만) | `pv/archive.rs:53-100` | 없음 | N | `password_cache_is_memory_only_and_forgettable`(`pv/archive.rs:416`) |
| PLUG-080 | 활성 암호 슬롯 | `with_password_scope(pw, f)`: 호출 직전 주입 → `f` → **스코프 가드로 반드시 비움**(패닉 경로 포함). `with_active_password`로 빌려 읽기 | `pv/archive.rs:102-143` | 없음 | N | `pv/wasm.rs:661-664` |
| PLUG-081 | 공급자 경유 재조회 | `read_via(path, preview_map, disabled, password)`: 암호 스코프 안에서 `preview_for` → `Archive`면 그대로, 압축이 아닌 공급자로 매핑돼 있으면 `Failed(archive.notArchive)` | `pv/archive.rs:145-165` | 없음 | N | 없음 |
| PLUG-082 | 내장 목록 읽기 + 상태 번역 | `read(path, password)`: 암호 우선순위 = **명시 인자 > 활성 슬롯 > 세션 캐시**. `PasswordRequired`/`WrongPassword` → `NeedPassword` · `NeedsCodec(fmt, codec)` → `NeedPlugin` · `NotArchive` → `Failed(archive.notArchive)` · `Corrupt`/`Io` → `Failed(사유)`. `provider = "builtin.archive"` | `pv/archive.rs:167-200` | 없음 | N | `pv/archive.rs:383-413` |
| PLUG-083 | 항목 시각 표시 | `fmt_entry_time(unix, time_is_local, tz분)`: DOS 계열(현지 벽시계)은 **오프셋 0**, UTC 계열만 시간대 보정. 형식 `YYYY-MM-DD HH:MM` | `pv/archive.rs:283-287` | `tz` 원천은 호스트(`st.tz`) | N | `dos_times_are_not_shifted_twice`(`pv/archive.rs:429`) |
| PLUG-084 | 구형 이름 코드페이지 | 시작 시 1회 `install_name_decoder()` → `MultiByteToWideChar(CP_ACP)`를 nexa-vfs 훅에 주입. **비Windows = 빈 함수**(UTF-8 → CP437 폴백만) | `pv/archive.rs:317-341` · 호출 `app/win.rs:1432` | `MultiByteToWideChar(CP_ACP)` | **P** | 없음 |
| PLUG-085 | 비밀 타입 | `Secret`(`Vec<u8>`): `Debug` = `Secret(***)`, Display/직렬화 없음, Drop에서 volatile 0 덮기, `take_from_string`/`take_from_u16`(원본 소거), `expose`/`expose_str`, `Clone`(사본도 소거) · `zeroize_bytes/u16/string` | `core/secret.rs:20-108` | 없음 | N | (같은 파일 테스트 — 문서 밖) |

### 1.6 압축 목록 리더(nexa-vfs `archive`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-090 | 포맷 중립 모델 | `ArchiveEntry { path, is_dir, size?, packed?, modified?, time_is_local, encrypted, method, crc32?, suspicious }` + `ratio()`(반올림 %)·`name()`·`parent()` / `Listing { format, label, entries, truncated, comment?, has_encrypted, header_encrypted, multivolume, solid }` + `totals()`·`counts()` / `ArchiveError::{NotArchive, PasswordRequired, WrongPassword, NeedsCodec, Corrupt, Io}`. `header_encrypted`는 어느 리더도 세팅하지 않음 | `vfs/mod.rs:45-150` | 없음 | N | `ratio_and_name_helpers`(`vfs/mod.rs:579`) |
| PLUG-091 | 원본 추상 | `ReadAt { size, read_at }` · `FileSource`(seek+read 루프, Interrupted 재시도) · `SliceSource` · `read_exact_at`(부족 = Corrupt, 64MB 상한) · `read_head`(512B) · 안전 LE 읽기 `u16le/u32le/u64le`(범위 밖 = None) | `vfs/mod.rs:152-248` | 없음 | N | 포맷 테스트 전반 |
| PLUG-092 | 포맷 레지스트리 | `FORMATS` 순서 = zip → 7z → rar → cab → tar → gzip → stream. `detect` = **시그니처 우선**, 실패 시 확장자. `all_exts`(정렬·중복 제거) → `BuiltinArchive` 선언 확장자. `list_from`/`list_path`(파일명을 `name_hint`로). 새 포맷 = 파일 1개 + 배열 1줄 | `vfs/mod.rs:404-491` · `pv/archive.rs:289-315` | 없음 | N | `registry_exts_are_unique_and_routable`(`vfs/mod.rs:614`) |
| PLUG-093 | 상한·마감 | `MAX_ENTRIES` 50,000 · `MAX_NAME` 4096B · `MAX_CHUNK` 64MB. 리더는 상한+1까지 push → `finish`가 절단 + `truncated=true`, 경로 정렬, `(path,is_dir)` 중복 제거, `has_encrypted` 집계 | `vfs/mod.rs:38-43`, `384-402`, `493-504` | 없음 | N | `limit_marks_truncated_like_other_formats`(`vfs/cab.rs:201`) |
| PLUG-094 | 이름 디코드 | UTF-8 플래그면 lossy UTF-8, 아니면 UTF-8 검증 → **호스트 디코더 훅**(`set_name_decoder`, `OnceLock`) → CP437 상위 128자 표 | `vfs/mod.rs:250-298` | 훅 구현만 OS 의존 | N(훅 = P) | `name_decode_falls_back_to_cp437`(`vfs/mod.rs:628`) |
| PLUG-095 | 경로 정규화(zip slip 차단) | `\` → `/`, 선두 `/`·`\`, 드라이브 `X:/`(바이트 비교), `..` 흡수 → `suspicious=true`. `.`·빈 요소 제거. 4096B 초과는 **문자 경계**에서 절단 | `vfs/mod.rs:300-335` | 없음 | N | `normalize_blocks_escape_and_flags_it` · `long_cjk_name_truncates_on_char_boundary` · `drive_check_survives_multibyte_second_char` · `zip_entry_with_multibyte_second_char_lists_without_panic`(`vfs/mod.rs:545-718`) |
| PLUG-096 | 시각 변환 | `ymd_hms_to_unix`(days_from_civil) · `dos_to_unix`(date 0·월/일 범위 밖 = None, 초 = 2초 단위) · `filetime_to_unix`(0 = None) | `vfs/mod.rs:337-382` | 없음 | N | `dos_time_matches_known_values` · `unix_epoch_anchors`(`vfs/mod.rs:558-576`) |
| PLUG-097 | 암시 폴더 채우기 | `with_implied_dirs` — 폴더 항목 없는 zip/tar의 상위 노드 생성. **현재 호출처 없음**(트리 그리드 후속용) | `vfs/mod.rs:506-538` | 없음 | N | `implied_dirs_are_filled_once`(`vfs/mod.rs:592`) |
| PLUG-098 | ZIP | 확장자 26종(`zip zipx jar war ear apk aar docx xlsx pptx odt ods odp epub whl crx xpi nupkg appx msix vsix ipa kmz cbz sar pk3`). 꼬리 65,557B에서 EOCD 역탐색(주석 길이 정합 확인) → Zip64 로케이터 승격 → **SFX 델타 보정**(`checked_add`) → 중앙 디렉터리 순회. 확장 필드 `0x0001` Zip64 · `0x9901` AES(`AES-128/192/256 + 방식`) · `0x000A` NTFS · `0x5455` UT. 플래그 bit0 = 항목 암호화, **bit13 = CD 암호화 → PasswordRequired**, bit11 = UTF-8 이름. 폴더 = 이름 끝 `/`·`\` 또는 (속성 0x10 **且** 크기 0). 확장 시각 = UTC, DOS 시각 = 현지. 주석 표시. 방식명 표(Store/Deflate/…/AES) | `vfs/zip.rs:1-269` | 없음 | N | 8개(`vfs/zip.rs:333-467`) |
| PLUG-099 | TAR | `tar`. 512B 블록, ustar 시그니처 또는 체크섬(부호 있는/없는 합 모두 허용). GNU `L` 긴 이름(K는 건너뜀) · PAX `x`/`g`(`path`·`size`·`mtime`) · base-256 크기 · prefix 필드. 0 블록 2개 = 종료. 첫 항목부터 체크섬 불일치 = Corrupt, 뒤쪽 잡음은 조용히 종료. `checked_next_multiple_of`·`checked_add`로 wrap 방어. 시각 = UTC, 방식 = Store | `vfs/tar.rs:1-226` | 없음 | N | 11개(`vfs/tar.rs:311-464`) |
| PLUG-100 | RAR 5 | `rar r00 rev`. vint 블록 순회: 타입 1 = 메인(볼륨 플래그) · 2 = 파일 · 3 = 서비스(목록 제외) · **4 = 암호화 헤더 → PasswordRequired** · 5 = 끝. 파일 플래그(폴더·mtime·CRC), compression_info(방식 Store~Best, bit6 = 솔리드), 확장 영역 type 1 = 항목 암호화. 시각 = UTC. 표시명 "RAR 5" | `vfs/rar.rs:13-210`, `299-332` | 없음 | N | 4개(`vfs/rar.rs:395-479`) |
| PLUG-101 | RAR 4 | 헤더 0x73 메인(`0x0080` = 헤더 암호화 → PasswordRequired, 볼륨, 솔리드) · 0x74 파일(64비트 확장 `0x0100`, 유니코드 플래그 `0x0200` = NUL 앞 ASCII 부분만, 폴더 = `0x00E0`, 암호 `0x0004`, DOS 시각 = 현지) · 0x7B 끝. 표시명 "RAR 4" | `vfs/rar.rs:212-297` | 없음 | N | 2개(`vfs/rar.rs:481-534`) |
| PLUG-102 | CAB | `cab`. `MSCF` + reserved 0. CFHEADER → 예약 영역·이전/다음 캐비닛 이름 건너뜀 → CFFOLDER(방식 Store/MSZIP/Quantum/LZX, 최대 4096) → CFFILE(이름 NUL 종료, `0x80` = UTF-8 이름, DOS 시각). 폴더 항목 없음, 항목별 압축 크기 없음(`None`), 분할 표시 | `vfs/cab.rs:1-149` | 없음 | N | 3개(`vfs/cab.rs:186-225`) |
| PLUG-103 | 7z(판정만) | `7z`. 시작 헤더 → 다음 헤더 첫 바이트: `0x17` + AES 코더 ID(`06 F1 07 01`) = PasswordRequired · `0x17`/`0x01` = `NeedsCodec("7z","LZMA")`(플러그인 안내) · 크기 0 = 빈 목록(정상) · 그 외 Corrupt | `vfs/sevenz.rs:1-69` | 없음 | N | 3개(`vfs/sevenz.rs:87-113`) |
| PLUG-104 | GZIP | `gz gzip tgz`. FEXTRA 건너뜀, FNAME(없으면 파일명에서 유추 — `.tgz` → `.tar` 등), MTIME(UTC — `time_is_local` 기본 false), 꼬리 ISIZE(2^32 모듈러) = 원본 크기, 압축 크기 = 파일 크기, 방식 Deflate. 항목 1건 | `vfs/stream.rs:16-121` | 없음 | N | `gzip_reads_stored_name_time_and_isize` · `tgz_without_name_derives_inner_tar`(`vfs/stream.rs:200-234`) |
| PLUG-105 | 단일 스트림 | `bz2 tbz2 tbz bz xz txz zst zstd tzst lz4 lz tlz lzma z taz`. 시그니처 표 7종(BZIP2/XZ/Zstandard/LZ4/lzip/LZMA/compress) 우선, 없으면 확장자. 항목 1건(원본 크기 미상, 압축 크기 = 파일 크기). 표시명 = 표의 라벨 | `vfs/stream.rs:123-192` | 없음 | N | `single_stream_formats_are_identified` · `unknown_stream_is_rejected`(`vfs/stream.rs:237-261`) |

### 1.7 샘플(동봉) 플러그인 · 빌드 · 배포 · 설정

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PLUG-110 | markdown.wasm 메타 | `nx_meta` = `markdown\nMarkdown Viewer\nmd,markdown,mdown,mkd`. import = `read_text`·`render_svg`·`is_dark`(`disp_width`는 안 씀). `nx_preview` = 64KB 읽기 → 빈 파일 `lines\n(empty file)` → `render` | `md/src/lib.rs:10-65` | 없음(.wasm 단일 아티팩트) | N(자원 그대로) | `sample_wasm_plugin_end_to_end`(`pv/sample_tests.rs:17`) |
| PLUG-111 | 마크다운 블록 렌더 | 입력 4000줄·출력 400줄 상한(초과 = `\u{2}q\|… (표시 상한 — 이후 생략)`). 펜스(``` / ~~~, 닫힘 = 같은 문자 run ≥ 열림) → `\u{2}code\|` · `mermaid` 펜스 → PLUG-114~116 · 빈 줄 접기 · 제목 `#`×1~6(h3 이상은 h3) · `---`/`***`/`___` → hr · `>` 중첩(`» ` 반복) → q · 목록 `- * +`(`• `), 체크 `[ ]`→`☐ ` `[x]`→`☑ `, 번호 `N. `/`N) `(1~9자리), 들여쓰기 최대 16 · 본문 `<br/>` = 줄바꿈(후속 줄 2칸 들여쓰기) | `md/src/lib.rs:295-461` | 없음 | N | 위 E2E(`:38-45`) |
| PLUG-112 | 인라인 정리 | `\` 이스케이프 · 백틱 코드(백틱 유지) · 링크 `[라벨](url)` → 라벨 · 이미지 `![alt](url)` → `🖼 alt` · `*`/`_` 강조 마커 제거(`_`는 단어 경계 규칙) | `md/src/lib.rs:134-207` | 없음 | N | 위 E2E(`:42-45`) |
| PLUG-113 | 표 렌더 | 구분행 필수. 정렬 `:--`/`--:`/`:-:`. 본문 최대 50행. 셀 폭 = 표시 폭 최대 60칸(초과 `…` 말줄임). 박스 드로잉(`┌┬┐│├┼┤└┴┘─`) → 줄마다 `\u{2}mono\|` | `md/src/lib.rs:209-289`, `385-400` | 없음 | N | 위 E2E(`:41`) |
| PLUG-114 | Mermaid flowchart → SVG | `graph`/`flowchart` + 방향(LR/RL = 가로). 노드 모양 8종, 화살표 6종 + `--라벨-->`, `\|라벨\|`. `subgraph/end/style/classDef/class/click/linkStyle/direction` 줄 무시. 상한 노드 24·간선 60·캔버스 2000px. 레벨 = 최장 경로. 테마색 2벌. 인접 레벨 간선만 그림(건너뛰는 간선은 아래 텍스트 `· A ─▶ B (라벨)`). 호스트 `render_svg` 성공 → `\u{1}img\|경로` + `\u{1}pad`×(`clamp(h/22, 3, 18)`−1) | `md/src/mm.rs:1-357` | `render_svg` 가용성(PLUG-011) | N(게스트) / **P**(호스트) | 위 E2E(`:46-54`) |
| PLUG-115 | Mermaid sequence → 텍스트 아트 | `participant`/`actor`(`as` 별칭) · 화살표 8종(점선 = `--`로 시작) · note/loop/alt/opt/par/critical/break/rect/else/end 표식 · 자기 호출 `⟲`. 상한 참가자 6·행 40·격자 240×160칸 | `md/src/mm.rs:359-537` | 없음 | N | 위 E2E(`:55-56`) |
| PLUG-116 | Mermaid 폴백 | 미지원 종류·상한 초과·`render_svg` 실패 → 원문 상자(`┌── mermaid` / `│ …` / `└──`) | `md/src/mm.rs:110-139` | — | N | 위 E2E(`graph TD` 원문 허용) |
| PLUG-117 | archive.wasm 메타 | `nx_meta` = `archive-sample\nArchive Sample (ISO/ar/cpio)\niso,a,deb,lib,cpio\narchive`. import = `file_size`·`read_at`(password 미사용). 게스트 상한 20,000 항목. 미지원 형식 = `error\n…` | `arc/src/lib.rs:1-88` | 없음 | N | `sample_archive_plugin_lists_iso_ar_and_cpio`(`pv/sample_tests.rs:154`) |
| PLUG-118 | ISO 9660(+Joliet) | 16번 섹터 `CD001` 판정 → 볼륨 기술자 16~39 훑기(Joliet 우선) → 루트 레코드 재귀(깊이 ≤12, 디렉터리당 4MB) · `;1` 버전 접미 제거 · 시각 7바이트 + 15분 단위 tz 보정 → `utc` | `arc/src/lib.rs:120-268` | 없음 | N | 위(`:198-213`) |
| PLUG-119 | ar(.a/.deb/.lib) | `!<arch>\n` · 60B 헤더 · GNU 긴 이름 표(`//`) · BSD `#1/N` · 심볼 인덱스(`/`·`/SYM64/`) 생략 · 짝수 정렬 | `arc/src/lib.rs:270-333` | 없음 | N | 위(`:188-196`) + `read_at_fuel_allows_thousands_of_members`(2500 멤버) |
| PLUG-120 | cpio(newc) | `070701`/`070702` · 110B 16진 헤더 · 4바이트 정렬 · `TRAILER!!!` 종료 · 폴더 = mode `0o040000` | `arc/src/lib.rs:335-384` | 없음 | N | 위(`:170-186`) |
| PLUG-121 | 샘플 크레이트 구성 | `crate-type = ["cdylib"]` · **빈 `[workspace]`로 앱 워크스페이스와 분리** · release: `opt-level="z"`·`lto`·`panic="abort"`·`strip`·`codegen-units=1`. 타깃 `wasm32-unknown-unknown`. `dist/*.wasm` 동봉(테스트가 로드) | `md/Cargo.toml:1-20` · `arc/Cargo.toml:1-20` | 없음 | N | — |
| PLUG-122 | 빌드 스크립트 | `build-plugins.ps1 [-OutDir <dir>] [-SkipDist]`: 타깃 설치 확인 → 샘플별 `cargo build --release --target wasm32-unknown-unknown` → `dist\<이름>.wasm` 복사(SkipDist면 생략) → OutDir 복사 → 크기(KB) 출력. 목록 = `markdown-viewer-wasm → markdown.wasm`, `archive-viewer-wasm → archive.wasm` | `nexa-dir2/scripts/build-plugins.ps1:15-64` | PowerShell 전용 | **P**(sh 병행) | CI가 실행 |
| PLUG-123 | CI·릴리스·설치본 동봉 | CI: `-OutDir target/release/plugins -SkipDist`. 릴리스: 태그 소스로 재빌드 → 포터블 zip에 `plugins\` 폴더째 · Inno `{app}\plugins` · `NexaDir-Plugins-<ver>.zip` · 단일 exe 자산에는 미동봉 | `nexa-dir2/.github/workflows/ci.yml:36` · `release.yml:27-35, 101-110` · `installer/nexa.iss:73-76` · `d2docs/21-distribution.md` §5-2 | Windows 패키징 | **P** | — |
| PLUG-124 | 샘플 E2E | `dist/*.wasm`을 임시 폴더에 복사 → 실제 wasmi로 로드·실행. cpio/ar/ISO 픽스처는 테스트가 **바이트로 조립** | `pv/sample_tests.rs:1-222` | 없음 | N | 2개 |
| PLUG-125 | 설정 > 플러그인 페이지 | 설명 1줄(`pref.plugins.desc`) → 목록이 비면 `pref.plugins.empty` → 플러그인당 체크박스 `"{name} ({id}) — {ext, …}"`(체크 = 사용). 수확 시 체크 해제된 id를 `\|`로 이어 `plugins_disabled` 구성(페이지에 체크박스가 있을 때만) | `app/prefs.rs:1786-1845`, `2078-2088` | 네이티브 STATIC/BUTTON | A | 없음 |
| PLUG-126 | 설정 키 | `preview_map`(기본 빈 값 · ≤512B · 파일로만 편집 — UI 없음) · `plugins_disabled`(기본 빈 값 · ≤512B). **빈 값이면 저장 생략** | `app/config.rs:144-147`, `292-293`, `462-466`, `706-708` | 없음 | N(형식은 nexa-sql 레지스트리로 이관 — 문서 15 소관) | `app/config.rs:1234-1263`, `1327-1328`, `1493-1497` |
| PLUG-127 | 문자열 키 | `preview.{none,empty,binary,fail,plugin.error,plugin.disabled,window.image}` · `dock.preview` · `pref.plugins.{desc,empty}` · `archive.*` 35개(요약·상태·컬럼·표시·암호 창). `archive.pw.remember`는 **정의만 있고 미사용** | `nexa-dir2/crates/nexa-app/lang/ko.lang:168-178, 508-542`(en·ja 동형) | 없음 | N(자원 그대로 — 경로 문구 `data\plugins\`는 OS별 수정 필요) | — |

## 2. 화면·컨트롤 배치

### 2.1 독립 미리보기 창(`app/previewwnd.rs`)

- **창**: 제목 `{파일명} — Preview` · 크기 = 소유자의 3/4, 폭 clamp(480, 1400)·높이 clamp(360, 1000), 소유자 중앙(`:880-889`). 표준 겹침 창(최대화·리사이즈 가능) + 세로·가로 스크롤바 상시. 배경은 WM_PAINT가 전면 도장(`WM_ERASEBKGND` = 1).
- **캔버스**: 자식 컨트롤 없음 — 클라이언트 전체가 문자 캔버스.
  - `PAD_X = 8`(좌측 문자 원점·좌우 여백) · 상단 패드 = `PAD_X/2 = 4`(`:34`, `:468`).
  - 행 높이 `line_h` = max(12, 세 글꼴(모노·본문·굵게)의 `"Ag한"` 높이 최댓값) **+ 3**(`:921-946`). 모든 행 동일 높이.
  - 가시 행 수 = `client_h / line_h`(최소 1). 그리기 범위 = `top .. top+vis+2`.
  - 텍스트 x = `PAD_X − left`, y = `4 − top_frac + i×line_h`.
  - 종류별 장식(§1.4 PLUG-052): 인용 바 = x 2~5, y+2 ~ y+line_h−2 · h1/h2 괘선 = 행 하단 1px 전폭 · hr = x PAD_X ~ client_w−PAD_X, y = 행 중앙 1px · 코드 밴드 = 행 전폭 배경.
  - 선택 하이라이트 = 문자 경계 px 구간(행 높이 전체). 그리기 순서 = 행 배경 → 선택 배경 → 텍스트 1회(투명 모드).
  - 인라인 이미지 = x `PAD_X` 좌측 정렬, 예약 영역 `min(line_h×k, client_h − y)` 안 세로 중앙, 축소만.
  - 잔여 배경 = 상단 패드 + 마지막 줄 아래.
- **탭 순서**: 없음(단일 캔버스).
- **단축키/입력**

| 입력 | 동작 | 근거 |
|---|---|---|
| Esc | 닫기 | `:835` |
| Ctrl+C | 선택 rich 복사 | `:838` |
| ↑ / ↓ | 1줄 | `:844-845` |
| ← / → | 가로 24px | `:846-847` |
| PgUp / PgDn | 가시 줄 수 | `:848-849` |
| Home | 맨 위·맨 왼쪽 | `:850` |
| End | 맨 아래(가로 유지) | `:851` |
| 휠 / Shift+휠 / 가로 휠 / 터치패드 | PLUG-056 | `:727-770` |
| 좌클릭 드래그 | 문자 선택 + 경계 밖 자동 스크롤(50ms) | `:771-820` |
| 우클릭(뗌) | 복사 · ─ · 전체 선택 | `:821-827` |

### 2.2 압축 그리드 창(`app/archivewnd.rs`)

- **창**: 제목 `{파일명} — 압축 미리보기` · 소유자 3/4, 폭 clamp(560, 1500)·높이 clamp(360, 1000), 중앙 · 기본 960×640(`:210-218`).
- **배치**(`layout`, `:318-334`): `PAD = 10`, `lh = max(font_height, 12)`, `status_h = lh + 8`.
  - 그리드: `(0, 0, client_w, gh)`, `gh = max(client_h − status_h − PAD, 10)`.
  - 상태 라벨: `(PAD, gh + 4, max(client_w − 2×PAD, 10), status_h)`, 왼쪽 정렬.
- **그리드**: 헤더 있음 · 지브라 · 마크 없음 · 행 높이 자동(글꼴 + 상하 4px — `ctl/grid.rs:127-128`). 컬럼 폭 = PLUG-072(리사이즈 최소 40px).
- **탭 순서**: 그리드만 포커스 대상.
- **단축키**: Esc 닫기 · Ctrl+C 복사(선택 없으면 전체) · 방향키/Home/End/PgUp/PgDn/Ctrl+A/Space/Shift·Ctrl 조합 = 그리드 선택 규약 · 헤더 클릭 = 정렬.

### 2.3 암호 입력 창(`app/pwprompt.rs`)

- **창**: 제목 `압축 파일 암호` · 캡션+닫기 버튼만(크기 고정) · 모달. 위치 = 소유자 가로 중앙, 세로 **1/3 지점**(`:76-79`).
- **치수**: `PAD = 14`, `FORM_W = 420`, `lh = max(font_height, 12)`, `row = lh + 12`. 클라이언트 폭 = `FORM_W + 2×PAD = 448`. 클라이언트 높이 = `PAD + lh×(1 또는 2) + 10 + row + 8 + row + 6 + lh + PAD + row + PAD`, 창 높이는 +30(캡션 보정).
- **컨트롤(위→아래, y 누적)**

| 순서 | 컨트롤 | 위치·크기 | 비고 |
|---|---|---|---|
| 1 | 라벨(안내 `archive.pw.prompt` — 파일명 포함) | `(PAD, y, FORM_W, lh)` | `y += lh` |
| 2 | 라벨(재시도 시만 `archive.pw.wrong`) | `(PAD, y, FORM_W, lh)` | 글자색 = danger · `y += lh` |
| — | 간격 | `y += 10` | |
| 3 | 라벨 `암호` | `(PAD, y, 72, row)` | `label_w = 72` |
| 4 | **입력란**(마스킹 `●` U+25CF) | `(PAD+72, y, FORM_W−72, row)` | 초기 포커스 · 상한 1024자 · `y += row + 8` |
| 5 | 체크박스 `암호 표시` | `(PAD+72, y, FORM_W−72, row)` | 2상태 · 토글 후 포커스를 입력란으로 · `y += row + 6` |
| 6 | 라벨(주의 `archive.pw.note`) | `(PAD, y, FORM_W, lh)` | 글자색 = text_dim · `y += lh + PAD` |
| 7 | 버튼 `확인`(기본 버튼) | `(PAD + FORM_W − 88×2 − 8, y, 88, row)` | |
| 8 | 버튼 `취소` | `(PAD + FORM_W − 88, y, 88, row)` | |

- **탭 순서**: 입력란 → 표시 체크 → 확인 → 취소(생성 순서 — 추정: `ctl` 컨트롤의 탭 정지 규약은 문서 16 소관).
- **단축키**: Enter = 확인(값 회수·소거 후 닫기) · Esc = 취소 · 닫기 버튼 = 취소(입력 소거).

### 2.4 하단 도크의 미리보기 부분(`gui/dock.rs`)

- 종류 스트립(정보 \| 미리보기 \| 터미널) 아래가 내용 영역. 미리보기 종류(인덱스 1)일 때:
  - **이미지**: 영역 `(b.x + pad_x, strip.bottom + 2, b.w − 2×pad_x, b.bottom − strip.bottom − 4)`에 비율 유지 가운데(`:831-842`).
  - **라인**: 행 높이 `row_h`, 세로·가로 스크롤, 드래그 문자 선택, 인라인 이미지 영역 `(b.x + pad_x, y, b.w − 2×pad_x, min(row_h×k, b.bottom − y))`(`:859-885`).
  - **↗ 버튼**: 한 변 `side = min(row_h + 4, b.bottom − content_top − 4)`, 위치 `(b.right − side − pad_x, content_top + 2)`, 아이콘 크기 `max(side − 8, 8)`, 누름 시 아이콘 1px 우하 이동(`:361-411`).
- 도크 위젯 전체(스트립·스크롤·선택)는 문서 14(`14-dir2-gui-widgets.md`) 소관 — 여기서는 미리보기 고유 부분만.

### 2.5 설정 > 플러그인 페이지(`app/prefs.rs:1786-1845`)

- 설명 정적 텍스트: `(x0, y, pane_w, 36)` → `y += 40`.
- (목록 0건) 안내 정적 텍스트: `(x0, y, pane_w, 36)` → `y += 40`.
- 플러그인당 체크박스: `(x0, y, pane_w, 24)` → `y += ROW_H`. 라벨 `"{name} ({id}) — {exts 쉼표}"`. 탭 정지 있음.
- 검색 토큰이 있을 때는 이 동적 목록을 그리지 않는다(`tokens.is_empty()` 조건).

## 3. nexa-ui 매핑

`ui/nexa-ctl/src`를 Grep해 실재를 확인했다(기준 커밋 `df75f5a`).

| dir2 요소 | nexa-ui 대응 | 상태 |
|---|---|---|
| 미리보기 창 캔버스(줄 단위 스타일·문자 선택·가로/세로 스크롤·인라인 이미지) — `previewwnd.rs` 전체 + 도크 라인 뷰 | **없음**. `TextBox`는 읽기 전용(`ui/nexa-ctl/src/controls/textbox.rs:3211`)·멀티라인을 지원하지만 **줄마다 글꼴(프로포셔널/모노/굵게)·밴드·괘선·인라인 이미지**를 섞지 못한다 | **추가 필요**: `LineView`(가칭). API 요약 — `set_lines(Vec<StyledLine { kind, text }>)` · `set_images(경로→Rc<IconImage>)` · 선택 `(line, char)` 앵커/현재 · `selected_text()` · `select_all()` · `scroll_to(top, left)`/픽셀 스크롤 · `Widget::on_event/paint` · 종류별 글꼴 = `DrawCtx::select_font(FontSlot::Mono \| Base, bold)`(`ui/nexa-ctl/src/draw.rs:16-28, 45`) · 도크 축약 뷰와 독립 창이 같은 컨트롤을 모드만 달리해 공유 |
| 네이티브 스크롤바(`WS_VSCROLL/WS_HSCROLL`) | `ScrollBars`(오버레이·자동 숨김 — `ui/nexa-ctl/src/controls/scroll.rs:389, 458`) · `FastScroll`/`ScrollAccel`/`SpeedHud` · `WheelAccum`(`ui/nexa-ctl/src/lib.rs:66`) | 있음 |
| 네이티브 팝업 메뉴(복사/전체 선택) | `EditMenu` + `EditMenuAction::{Copy, SelectAll}`·`EditMenuCaps`(`ui/nexa-ctl/src/controls/editmenu.rs:55-66`) 또는 `ContextMenu`/`CtxItem` | 있음 |
| `ctl::grid`(NxGrid — 헤더 드래그 리사이즈·**다중 컬럼 3상태 정렬 통지**·**다중 선택**·지브라·오버레이 스크롤) | `TreeGrid`/`GridColumn`(`ui/nexa-ctl/src/controls/tree.rs:645-800`)은 **단일 선택**·`set_column_width`·`fit_columns`만 있고 정렬 헤더·다중 선택·지브라가 없다(Grep `sort\|zebra\|multi` 0건). nexa-sql 결과 그리드는 앱 내부(`sql/nexa-sql/src/grid.rs:1-3` — "nexa-grid 크레이트가 오면 교체")라 nexa-ui에 없음 | **추가 필요**: 평면 `DataGrid`(가칭). API 요약 — `new(columns: Vec<GridColumn>)` · `set_rows(Vec<Vec<String>>)` · `sort_spec() -> Vec<(usize, bool)>` + 정렬 변경 이벤트(비교는 호스트) · `selected_rows() -> Vec<usize>`(클릭/Shift/Ctrl/Ctrl+A/Space) · 옵션 `zebra`·`row_h` · 헤더 경계 드래그(최소 40px) · `ScrollBars` 내장. 파일 목록 그리드(문서 13)와 같은 컨트롤을 공유하는지 결정 필요 |
| `ctl::label`(정적 텍스트) | 전용 Label 컨트롤 **없음**(Grep `pub struct Label` 0건) | `DrawCtx::text`로 직접 그리거나 소형 `Label` 추가(왼쪽 정렬·색 지정·말줄임) |
| `ctl::textbox` + `set_password_char(●)` + `clear_secret` | `TextBox::set_masked(true)`(`textbox.rs:2129`) · `take_secret_text()`/`wipe()`(`:3253-3263` — 본문·되돌리기·조합 글 0 덮기) | 있음(단 반환이 `String` — 받은 즉시 `Secret::take_from_string`으로 옮길 것) |
| `ctl::checkbox`(2상태) | `Checkbox`(`ui/nexa-ctl/src/controls/checkbox.rs`) / `Switch` | 있음 |
| `ctl::button`(Default/Normal) | `Button` + `ButtonMode`/`ButtonTone`(`ui/nexa-ctl/src/controls/mod.rs:42-44`) | 있음 |
| ↗ 팝아웃 이미지 버튼(3상태 배경 + SVG 아이콘) | `Button::glyph(MenuIcon)`(`button.rs:156`) 또는 `ToolIcon::Mask` | 있음(아이콘 마스크 변환은 문서 17 소관) |
| `DrawCtx::draw_image(rect, 경로)`(WIC 디코드 + 가운데 축소) | `DrawCtx::image` / `image_scaled(dst, &IconImage, clip)`(`draw.rs:119-127`) + `image_fit_contain`(`controls/mod.rs:616` — **작은 이미지를 확대**하므로 dir2의 "확대 없음"과 다름) + 디코더 `nexa_gfx::image::decode(bytes, max_pixels)`(`ui/nexa-gfx/src/image.rs:73`) | 부분: 디코드·캐시(8개)·축소 전용 맞춤은 호스트/새 헬퍼 필요 |
| `fontchain::GdiChain`(글리프 단위 폴백·경계 오프셋) | `nexa_gfx` 글꼴 폴백(`ui/nexa-gfx/src/text.rs:359, 478` `push_fallback`) + `DrawCtx::text_prefix_widths`(`draw.rs:82` — 문자 경계 누적 폭) | 있음(체인 구성 = 문서 17 소관) |
| 모달 창(소유자 비활성 + 중첩 메시지 루프) | nexa-ui에 창 계층 없음. 기준 = nexa-sql의 **보조 winit 창 + 액션 열거**(`sql/nexa-sql/src/input_win.rs:1-30` — Run/Skip/Cancel/`Password(Secret)`) | 앱 층에서 구현(§4-6) |
| 클립보드(평문·RTF) | nexa-ui에 없음. 기준 = `sql/nexa-sql/src/clipboard.rs:15-29`(`read_text`·`write_text`·`write_rich(text, html)`) | 앱 층(§4-5) |
| 설정 플러그인 페이지(네이티브 STATIC/체크박스) | `Checkbox` 목록 + 설명 텍스트. 참고 UI = nexa-sql 확장 패널(`sql/nexa-sql/src/ext_panel.rs:1-7` — 설치됨/켜기·끄기) | 컨트롤 있음(페이지 골격 = 문서 15) |
| SVG 파서(`svg.rs` — 플랫폼 중립) + GDI+ 래스터(`svg_to_pixels`) | 파서 = 그대로 이식 가능. 래스터 = **없음**(nexa-ui Grep `svg` 0건). 재료 = `nexa_ctl::shape`(점-도형 판정 `rrect/poly/stroke/…` `ui/nexa-ctl/src/shape.rs:9-119`) · `DrawCtx::polyline/fill_round_rect/fill_triangle/text` · 오프스크린 `nexa_gfx::Surface` | **추가 필요**: `svg::Doc → IconImage` CPU 래스터(rect·rx / line / polyline / path(M·L·H·V·C·A·Z) / circle / text·middle 앵커 · 채움/스트로크 · 4× 슈퍼샘플 불필요 시 생략) |

### 3.1 nexa-sql WASM 확장과 겹치는 재사용 가능 부분

| 요소 | dir2 (`pv/wasm.rs`) | nexa-sql (`sql/nexa-sql/src/extensions/wasm.rs`) | 재사용 판단 |
|---|---|---|---|
| 엔진 | wasmi `1.1.0`(`nexa-dir2/crates/nexa-app/Cargo.toml:18`) | wasmi `1.1`(`nexa-sql/Cargo.toml:79` — "nexa-dir2 미리보기 플러그인과 같은 판") | 같은 판 — 의존 그대로 |
| 게이트 `host_guard` | `:73-84` | `:49-58` | 동일 구조(마감 + 연료 차감) |
| 버퍼 읽기 `read_buf` | `:105-112` | `:60-67` | 동일(4B LE + UTF-8 · 1MB) |
| 호출당 새 인스턴스 + StoreLimits | `:295-322` | `:396-463` | 동일 골격. nexa-sql은 `nx_alloc`으로 **입력 버퍼 전달**·`returns_buf` 분기·로그 수거가 추가 |
| 상한 | 연료 2억 · 64MB · 1,500ms · 모듈 8MB | 연료 5천만 · 16MB · 200ms(포맷 20억/5초) · 모듈 8MB | 값만 다름 — **dir2 값을 유지**(동봉 archive.wasm이 2500 멤버 ar를 통과해야 함) |
| 브레이커 3회 | `:500-522` | `:352-394` | 동일 |
| 메타 | 줄 텍스트(`id\nname\nexts\ncaps`) | JSON + `abi` 버전 검사(`:275-280`) | **dir2 ABI 유지**(자원 = 동봉 .wasm 그대로). JSON 메타는 차용하지 않음 |
| 로드 오류·로그 표면화 | 버림(`pv/mod.rs:297`) | `notes` → 로그 창(`mod.rs:404-413`) | nexa-sql 방식 차용 권장(로드 오류를 로그/설정 페이지에 표시) |
| 같은 id 대체 | 경로 우선순위로 앞선 것만(`pv/mod.rs:299-301`) | WASM이 내장을 대체(`mod.rs:171-186`) | 정책이 다름 — dir2 정책 유지 |
| 설치·저장소·무결성 | 없음(파일 복사) | 확장 매니저(`manager.rs:1-7` — `index.json`·sha256·`<설정 폴더>/extensions/<id>/<ver>/`·켜기/끄기 `list_toggle`) · `sha256.rs` | 후속 후보(플러그인 내려받기). 1차 이식 범위 밖 |
| 켜기/끄기 목록 | `plugins_disabled` `\|` 구분 | `disabled: &[String]`(설정 레지스트리 키) | 설정 레지스트리로 옮기되 의미 동일 |

- nexa-sql의 런타임은 **앱 크레이트 내부**(`pub(crate)`)라 그대로 의존할 수 없다 → dir2 `pv/wasm.rs`를 이식하는 것이 정답이고, 공통부(게이트·버퍼·한도·브레이커)를 공용 크레이트로 뽑는 것은 별도 결정 사항.
- 비밀 타입: nexa-sql `nsql_core::Secret`(String 기반 — `sql/nsql-core/src/secret.rs:35`)과 dir2 `nexa_core::secret::Secret`(Vec<u8> 기반)이 별개다. 플러그인 `password` import가 **바이트**를 넘기므로 dir2 것을 이식한다.

## 4. OS 분기점

| # | 항목 | Windows 현 구현 | macOS 대응 | Linux 대응 |
|---|---|---|---|---|
| 4-1 | 이미지 디코드(PLUG-007·043) | WIC — png/jpg/bmp/gif/ico/tif/webp, Fant 축소 | **전 OS 공통으로 `nexa_gfx::image::decode`**(PNG[Adam7 포함]·BMP·GIF 첫 프레임·기저 JPEG — `ui/nexa-gfx/src/image.rs:1-6, 73-82`). 미지원 = 프로그레시브 JPEG·CMYK·12비트(`ui/nexa-gfx/src/jpeg.rs:1-4`)·**WebP·ICO·TIFF** → ① nexa-gfx에 디코더 추가(직접 개발 기조) 또는 ② 미지원 안내 1줄. (선택) OS 디코더 보조 = ImageIO `CGImageSourceCreateWithURL` | 같음. (선택) OS 디코더 보조 = gdk-pixbuf — 기조상 비권장 |
| 4-2 | 축소·표시 | WIC 스케일러 + `StretchDIBits` | `IconImage::resized`(`ui/nexa-gfx/src/surface.rs:93`) 또는 `DrawCtx::image_scaled`. **확대 금지**(`scale.min(1.0)`) 규칙을 호스트가 보장 | 같음 |
| 4-3 | SVG 래스터(PLUG-011·025) | GDI+ 경로·`Arial` 텍스트 | 공통 CPU 래스터(§3 추가 항목) → BMP 임시 파일 계약 유지(`std::env::temp_dir()/nexa-preview/`), 텍스트 글꼴 = nexa-font UI 글꼴 | 같음. 이것이 없으면 Mermaid flowchart가 **원문 상자 폴백**으로만 보인다 |
| 4-4 | 구형 이름 디코더(PLUG-084) | `MultiByteToWideChar(CP_ACP)` | 시스템 ACP 개념 없음 → UI 언어/`LANG`으로 코드페이지 추정(ko→CP949, ja→CP932, zh→GBK/Big5) 후 `iconv_open("UTF-8", "CP949")`(libiconv 기본 탑재) 또는 자체 CP949/CP932 표 | glibc `iconv` 동일. 미주입 시 CP437 폴백이 그대로 동작(기능 저하만) |
| 4-5 | 클립보드(PLUG-058·076) | `CF_UNICODETEXT` + "Rich Text Format" | nexa-sql 방식: `pbcopy`(평문) · rich는 osascript(`sql/nexa-sql/src/clipboard.rs:5-9, 24-29`). RTF를 유지하려면 `NSPasteboard` `public.rtf` 직접 게시 | `wl-copy` → `xclip` → `xsel`(평문). 다중 표현 불가 → 평문만(또는 `wl-copy --type text/rtf` 단일). 줄 구분 `\r\n` → `\n` |
| 4-6 | 모달 창·동기 암호 루프(PLUG-051·070·071·078) | `EnableWindow(owner,false)` + 중첩 `GetMessageW` | winit는 중첩 루프가 없다 → **보조 창 + 상태기계**: `PreviewWin`/`ArchiveWin`/`PasswordWin`이 열려 있는 동안 주 창 입력 차단(포커스 되돌리기), 암호 창 결과 이벤트 → `read_via` 재시도 → 그리드 창 열기. NSWindow 시트/`runModal`은 쓰지 않음(OS 간 차이 최소화) | 같음(X11/Wayland 모두 보조 창). 창 위치 = 소유자 기준 중앙(Wayland는 위치 지정 불가 — 컴포지터 배치 수용) |
| 4-7 | 플러그인 탐색 경로(PLUG-005) | ① `<exe>\data\plugins` 또는 `%LOCALAPPDATA%\NexaDir\data\plugins` ② `<exe>\plugins` | ① nexa-sql 규칙 `user_config_dir(app)/plugins` = `~/Library/Application Support/<앱>/plugins`(`ui/nexa-conf/src/lib.rs:310-334`) ② 번들 `<앱>.app/Contents/Resources/plugins`(exe 기준 `../Resources/plugins`) | ① `$XDG_CONFIG_HOME/<앱>/plugins`(없으면 `~/.config/<앱>/plugins`) ② `<exe>/plugins`(tarball·AppImage) + `<exe>/../share/<앱>/plugins`(패키지 설치). 관리형 설치 자리 판정은 nexa-conf 것 사용(`ui/nexa-conf/src/lib.rs:336-342`) |
| 4-8 | 글꼴 기본값(PLUG-053) | 모노 `Consolas` · 본문 `Segoe UI` | 모노 `Menlo`(또는 SF Mono) · 본문 시스템 UI 글꼴 — nexa-font 기본 탐색에 위임 | 모노 `DejaVu Sans Mono`/`Noto Sans Mono` · 본문 시스템 UI 글꼴 |
| 4-9 | DPI·커서·아이콘 | `GetDpiForWindow` · `IDC_IBEAM` · 리소스 아이콘 | winit `scale_factor()` · `CursorIcon::Text` · 번들 아이콘 | 같음 |
| 4-10 | 휠 입력(PLUG-056) | `WM_MOUSEWHEEL`(delta/120·MK_SHIFT)·`WM_MOUSEHWHEEL`·시스템 줄 수 | winit `MouseScrollDelta::PixelDelta`(트랙패드 = 픽셀 스크롤 경로) · Shift+휠은 OS가 가로로 바꿔 줌(중복 변환 금지) · 줄 수 기본 3 | `LineDelta`/`PixelDelta` · 줄 수 기본 3 |
| 4-11 | 빌드 스크립트(PLUG-122) | `scripts/build-plugins.ps1` | `scripts/build-plugins.sh` 병행(같은 목록·같은 옵션 `--out-dir`·`--skip-dist`). 목록 단일 출처 유지(예: 두 스크립트가 같은 표 파일을 읽음) | 같음. CI는 3 OS 모두에서 빌드 검증(산출 .wasm은 OS 무관 동일) |
| 4-12 | 배포 동봉(PLUG-123) | 포터블 zip `plugins\` · Inno `{app}\plugins` · 플러그인 zip | `.app/Contents/Resources/plugins/*.wasm`(코드 서명 대상 리소스) | tarball/AppImage `plugins/` · deb/rpm `/usr/share/<앱>/plugins` |
| 4-13 | 임시 파일 | `%TEMP%\nexa-preview\d<hash>.bmp`(정리 없음) | `$TMPDIR/nexa-preview/…` — 그대로 동작. 종료 시 정리 또는 메모리 캐시로 대체 검토 | `/tmp/nexa-preview/…` — 다중 사용자 시 권한(0700 디렉터리) 고려 |
| 4-14 | 경로 표현 | `PreviewDoc::Image(String)` — lossy | 문제 적음(UTF-8) | **비UTF-8 파일명 가능** → `PathBuf`로 변경 필요 |
| 4-15 | 시간대 오프셋(`tz`) | 호스트가 `st.tz`(분) 제공(원천은 문서 10~12) | `localtime_r`의 `tm_gmtoff` | 같음 |
| 4-16 | 사용자 문구의 경로 표기 | `data\plugins\`·`exe 옆 plugins\`(`ko.lang:177-178`) | OS별 실제 경로를 `{0}` 자리표시로 주입 | 같음 |

WASM 런타임·ABI·압축 리더(§1.2·§1.6)는 **OS 분기 없음** — `std::fs`·`std::time::Instant`·wasmi만 쓴다. `.wasm` 산출물 2개도 그대로 쓴다.

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5.1 영속(설정 키·파일)

| 항목 | 형식 | 기본값 | 상한 | 저장 위치 | 근거 |
|---|---|---|---|---|---|
| `preview_map` | `ext:id\|ext:id` (예 `md:markdown\|png:builtin.text`) | 빈 값 | 512B | `data\settings.cfg`의 `key=value` 줄(빈 값이면 줄 생략) | `app/config.rs:144, 462-463, 706` |
| `plugins_disabled` | `id\|id` | 빈 값(전부 사용) | 512B | 같음 | `app/config.rs:147, 465-466, 708` |
| 플러그인 파일 | `*.wasm`(≤8MB) | 동봉 2개 | — | PLUG-005의 두 폴더 | `pv/mod.rs:271-282` |
| SVG 래스터 캐시 | 32bpp BMP | — | 2000×2000px | `temp_dir()/nexa-preview/d{hash:016x}.bmp` | `pv/mod.rs:113-133` |
| 암호 | **영속 없음** — 스레드 로컬 메모리만 | — | 입력 1024자 | — | `pv/archive.rs:53-100` |
| 글꼴(미리보기 창) | 설정 `term_font`·`term_font_size`를 빌려 씀(별도 키 없음) | — | 8~32pt | — | `app/win.rs:2468` |

nexa-dir3에서는 설정 구조를 nexa-sql 레지스트리(`nsql-settings`)로 옮긴다 — 키 이름(예 `preview.map`·`plugins.disabled`)과 이관 규칙은 문서 15 소관이며, 여기서는 **값 형식·상한·기본값을 그대로 유지**해야 한다는 점만 고정한다.

### 5.2 메모리 상태

- 스레드 로컬: 공급자·플러그인 메타 캐시(`pv/mod.rs:288-290`) · 다크 신호(`:60-63`) · 활성 암호 슬롯(`pv/archive.rs:111-113`) · 세션 암호 캐시(`:64-66`) · 미리보기 창 휠 누적기(`app/previewwnd.rs:258-267`).
- 프로세스 전역: 이름 디코더 `OnceLock`(`vfs/mod.rs:253`).
- 플러그인별: 연속 실패 수 `Cell<u32>`(`pv/wasm.rs:503`). 컴파일된 `Module`·`Engine`은 캐시, 인스턴스는 호출마다 폐기.
- 표시 백엔드: 이미지 디코드 캐시 8개(`app/dw.rs:576-580`) · 미리보기 창 BMP 캐시(창 수명 — `app/previewwnd.rs:84`).

### 5.3 스레딩

**전부 UI 스레드 동기 실행**이다. 워커·채널·사용자 메시지(`WM_APP_*`) 없음.

- 선택 변경 → `update_dock_info` → `preview_content` → `preview_for`(파일 읽기·플러그인 실행 포함, 플러그인 1회 최대 1.5s 벽시계) → `dock.set_content/set_image/set_popout`(`app/win.rs:2476-2529`).
- 방향키로 목록을 훑으면 매 선택마다 위 경로가 돈다 → 브레이커(PLUG-034)와 벽시계가 UI 정지를 막는 유일한 장치.
- 압축 목록도 UI 스레드 동기 파싱(UNC·클라우드의 큰 아카이브는 체감 지연 가능 — `d2docs/28-archive-preview.md:129-132`).

### 5.4 메시지 흐름

```
[F3 / ↗]  open_preview_window (win.rs:2425)
   ├─ 단일 선택·파일 아님 → 무시
   ├─ set_dark(theme) → preview_for(path, preview_map, plugins_disabled)
   ├─ Lines  → previewwnd::show(owner, 제목, lines, (term_font, size), dark)   ← 모달 루프
   ├─ Image  → previewwnd::show(… ["이미지 파일 — 하단 도크 …"] …)
   └─ Archive(doc) → archivewnd::open(owner, path, doc, (map, disabled), tz, font)
          └─ while doc.status == NeedPassword:
                pwprompt::ask(owner, 파일명, retry, font)      ← 모달 루프(중첩)
                   None → return (창 열지 않음)
                   Some(secret) → read_via(path, map, disabled, Some(secret.clone()))
                        └ with_password_scope → preview_for → 공급자
                             내장: read() ← 활성 슬롯
                             WASM: nx_archive() → import password() ← 활성 슬롯
                        Ok           → pw::remember(path, secret)
                        NeedPassword → pw::forget(path); retry = true
                        기타          → break
             show(owner, 파일명, doc, tz, font)                 ← 모달 루프
                그리드 정렬 통지(WM_COMMAND id=1, code=NXGR_SORT) → sort_entries → set_rows
```

## 6. 이식 시 주의 — 회귀 방지에 필요한 실측 교훈·결함 이력

1. **임포트 연료 0 결함**(점검 1차 #5): 호스트 임포트가 연료를 안 쓰던 시절 게스트가 임포트 루프(4MB `read_at`·2000² SVG 래스터)로 호스트를 무한정 태울 수 있었다 → `host_guard`로 **모든 임포트에 연료 과금 + 벽시계 검사**(`pv/wasm.rs:70-84`). 새 임포트를 추가할 때 게이트를 빠뜨리면 재발.
2. **`read_at` 고정 과금 과다**(G11 누락1 / A15): 호출당 100,000 연료 → 약 2,000회에서 소진 → 멤버 수천 개의 `.lib`·cpio가 통째로 실패. 현재 = 고정 1,000 + 바이트/64 + **누적 64MB 상한**(`pv/wasm.rs:44-52, 202-227`). 연료 수치를 nexa-sql 값(5천만)으로 낮추면 회귀한다.
3. **실패마다 재인스턴스**: 브레이커 도입 전에는 방향키마다 연료 2억을 소진했다 → 연속 3회 격리(`pv/wasm.rs:498-522`).
4. **디버그 wasmi는 릴리스보다 수 배 느리다**: 1.5s 상한에 먼저 걸리므로 연료만 검증하는 테스트는 `call_buf_timeout(…, 60_000)`을 쓴다(`pv/wasm.rs:293-300, 766`). 테스트 하네스에서 벽시계 의존 단언은 `CALL_TIMEOUT_MS × 4` 여유(`:714-717`).
5. **Mermaid 3단 폴백**: 이미지 마커 → 아트 → 원문. 예전 단언이 앞의 둘만 인정해 비Windows CI에서 실패했다(08-02 — `pv/sample_tests.rs:46-54`). nexa-dir3에서 SVG 래스터를 넣은 뒤에는 **이미지 마커가 나오는지**를 OS 공통으로 단언하도록 강화할 것.
6. **`normalize_path` 패닉 2건**: ① 드라이브 판정을 `str` 슬라이스로 하면 둘째 글자가 멀티바이트일 때 문자 경계 패닉(G7-01 — `vfs/mod.rs:305-314`) ② 4096B 절단이 CJK 글자 중간이면 `String::truncate` 패닉(R-#2 — `:325-333`). 둘 다 **UI 스레드 abort**였다.
7. **TAR**: PAX 길이 필드를 `&str`로 자르면 멀티바이트 중간 패닉(G7-02 — `vfs/tar.rs:74-99`) · base-256 크기가 wrap하면 같은 헤더로 되돌아가 **무한 루프**(G7-11 — `:151-159`).
8. **RAR5/ZIP 오버플로**: 확장 영역 `size` vint wrap = 제자리 무한 루프(`vfs/rar.rs:65-75`) · `data_size` 덧셈(`:197-207`) · Zip64 `cd_off + cd_size`(`vfs/zip.rs:188-194`) — 전부 `checked_*`로 고정. 디버그 빌드에서는 overflow panic으로 나타난다.
9. **상한 +1 규약**: 리더는 상한을 1개 넘겨 push하고 `finish`가 `truncated`를 세운다. CAB이 이 규약을 어겨 절단 표시가 빠졌던 이력(`vfs/cab.rs:86-90`).
10. **시각 이중 보정 금지**: DOS 계열(zip·cab·rar4)은 현지 벽시계 그대로, UTC 계열(tar·rar5·gzip·zip 확장 필드)만 보정(`pv/archive.rs:283-287` · `d2docs/28-archive-preview.md:83-85`). 플러그인 항목은 `utc` 속성이 없으면 현지로 본다(`pv/wasm.rs:485-486`).
11. **인라인 이미지는 마지막에 그린다**: pad 행의 불투명 배경 도장이 이미지를 지운다(`app/previewwnd.rs:470-472, 609`).
12. **마커 줄은 선택·복사에서 제외**: 제어 문자(`\u{1}`) 유출 방지(`app/previewwnd.rs:396-404` · `gui/dock.rs:274-292`). 태그(`\u{2}…|`)도 표시/복사 텍스트에는 없어야 한다(`:909-919`).
13. **선택 줄 그리기**: 3분할 대신 "행 배경 → 선택 배경 → 텍스트 1회(투명)" — 분할하면 경계에 이음새(`app/previewwnd.rs:558-583`).
14. **팝업 메뉴 재진입**: 메뉴 표시 중 창 프로시저가 재진입하므로 상태의 `&mut` 차용을 끊고 표시 후 다시 빌린다(`app/previewwnd.rs:334-336`). nexa-ui `ContextMenu`는 비모달이라 같은 문제는 없지만, 메뉴가 열린 동안 입력 라우팅 순서를 확인할 것.
15. **Secret 소거 테스트**: 폐기된 메모리를 다시 읽는 검사는 UB이며 **macOS·Linux에서 실제로 깨진다**(할당자가 free 리스트 메타데이터를 덮음) → 살아 있는 버퍼로 계약 확인(`core/secret.rs:87-92`).
16. **암호 취급 계약**(사용자 지시 08-24): 저장 경로가 코드에 없어야 한다 · `Debug` = `Secret(***)` · 입력란 마스킹 · 경유 버퍼 즉시 소거 · 성공한 암호만 세션 기억 · 틀리면 즉시 폐기(`d2docs/28-archive-preview.md:90-101`). nexa-ui `take_secret_text()`는 `String`을 돌려주므로 받은 즉시 `Secret::take_from_string`으로 옮기지 않으면 계약 위반.
17. **(관찰 — 결함 추정) 플러그인 경로는 세션 암호 캐시를 보지 않는다**: 내장 `read()`는 "명시 > 활성 슬롯 > 세션 캐시"(`pv/archive.rs:168-173`)인데 WASM `password` import는 **활성 슬롯만** 읽는다(`pv/wasm.rs:256` → `pv/archive.rs:128-137`). 도크·F3 첫 호출은 스코프 없이 `preview_for`를 부르므로, 암호가 필요한 **플러그인 포맷**은 `pw::remember` 이후에도 매번 다시 묻는다(실기 확인은 못 함 — 현재 동봉 플러그인 3종은 암호 개념이 없어 드러나지 않음). 이식 시 호스트가 `preview_for` 전에 세션 암호를 활성 슬롯에 주입하도록 통일할지 결정 필요.
18. **(관찰) 로드 오류가 사용자에게 안 보인다**: `load_dir`의 `errors`를 버린다(`pv/mod.rs:297`). 설정 페이지에 "목록이 비면 위치·재시작 문제"라고만 안내(`d2docs/24-plugin-dev-guide.md:228-230`). nexa-sql처럼 로그 창/설정 페이지에 표시 권장.
19. **(관찰) 픽스처 줄바꿈 변환**: nexa-dir2에는 `.gitattributes`가 없고 `arc/fixtures/sample.a`가 텍스트로 취급돼(`git ls-files --eol` = `i/lf w/crlf`) Windows 작업 트리에서 선두가 `!<arch>\r\n`으로 **깨져 있다**(ar 매직 불일치). `sample.cpio`·`*.wasm`은 `-text`라 안전. 자원을 작업 트리에서 복사하면 깨진 파일을 가져오게 된다 → nexa-dir3에 `.gitattributes`(`*.wasm`·`*.a`·`*.cpio`·`*.iso` = `binary`)를 먼저 두고, 필요하면 git blob에서 꺼낸다. 자동 테스트는 이 픽스처를 쓰지 않아(바이트 조립) 드러나지 않았다.
20. **`dist/*.wasm`은 소스와 같은 커밋**: 테스트가 로드하는 고정 산출물. 릴리스는 태그 소스로 다시 빌드(`-SkipDist`)(`d2docs/24-plugin-dev-guide.md:220-223`).
21. **zip 동봉 형태**: `Compress-Archive -Path plugins\*.wasm`은 파일만 들어가 사용자가 폴더를 만들어야 한다 → `-Path plugins`(폴더째)(`nexa-dir2/.github/workflows/release.yml:108-109`).
22. **샌드박스 경계의 구멍 후보**: `nx_preview`가 `image\n<경로>`를 돌려주면 호스트가 **그 경로를 그대로 디코드**한다(`pv/wasm.rs:413-415`) — 대상 파일 외 임의 이미지 경로 읽기가 가능하다. `\u{1}img|<경로>`도 같다. 이식 시 허용 경로를 "대상 파일" 또는 "`nexa-preview` 캐시 폴더"로 제한할지 결정.
23. **문서와 구현의 차이**: ADR-0005 초안의 export `nx_alloc`(`d2docs/25-adr-0005-wasm-plugins.md:33`)은 **구현에 없다**(호스트→게스트 입력 없음). ADR-0004의 `.star`·`{"kv": …}` 반환은 폐기된 구판(`d2docs/09-adr-0004-preview-plugins.md:4`). 가이드 24가 실제 계약의 SSOT.
24. **하드코딩 문자열(i18n 누락)**: 런타임 오류 문구가 한국어 고정(`pv/wasm.rs:75-81, 222-225, 316, 320-321, 328, 346, 416`)이며 테스트가 `contains("read_at 누적")`·`contains("nx_preview")`로 단언한다 · nexa-vfs의 `Corrupt` 사유·`"단일 스트림"` 라벨·`"(안쪽 내용 — …)"`(`vfs/stream.rs:23, 149`) · 샘플의 `(empty file)`(영어)·`… (표시 상한 — 이후 생략)`(`md/src/lib.rs:61, 458`)·archive 샘플 오류문(`arc/src/lib.rs:85, 253`). 그대로 이식하면 동작은 같다 — 문구를 바꾸면 테스트도 함께.
25. **이미지 문서의 F3**: 안내 1줄만 뜬다(`app/win.rs:2444`) — dir2의 의도된 현 동작(이미지 = 도크 담당). "기능 유지" 기준으로는 그대로, 개선은 별도 결정.
26. **`image_fit_contain`은 확대한다**(`ui/nexa-ctl/src/controls/mod.rs:613-616`) — dir2는 축소만. 도크·창 모두 `min(…, 1.0)` 규칙을 지킬 것.
27. **그리드 빈 문자열 셀 크래시**(Win32 한정 — 빈 Vec 댕글링 포인터 `app/ctl/grid.rs:26-27`): nexa-ui에서는 해당 없음. 다만 폴더 행의 빈 크기 셀이 정상 표시되는지 확인.
28. **메타 4번째 줄 하위 호환**: caps 줄이 없는 v1 플러그인은 미리보기 전용으로 동작해야 한다(`pv/wasm.rs:356-363`).
29. **확장자 충돌 우선순위**: 같은 확장자를 두 플러그인이 선언하면 **파일명 사전순 앞**이 이긴다(가이드 `00-` 접두 — `d2docs/24-plugin-dev-guide.md:191-193`). 플러그인은 내장보다 항상 앞(`pv/mod.rs:313-317`) — archive.wasm이 선언한 `iso a deb lib cpio`는 내장과 겹치지 않는다.
30. **압축 창의 `doc` 소유**: 창이 `entries`를 가져가고 원본 `doc`은 소멸(`app/archivewnd.rs:266`) — 정렬은 `entries`를 제자리 정렬 후 전체 행 재주입(50,000행에서 비용 확인 필요 — 추정).

## 7. 회귀 테스트 후보

자동화: **◎** = 헤드리스 단위/통합 테스트로 가능(OS 공통) · **○** = 오프스크린 렌더/이벤트 주입 하네스 필요 · **△** = 실기 수동(체크리스트).

| # | 시나리오 | 대응 ID | 자동화 | 비고 |
|---|---|---|---|---|
| T-01 | dir2 기존 64개 테스트를 그대로 이식해 3 OS에서 통과 | 전 범위 | ◎ | 임시 경로·`CARGO_MANIFEST_DIR` 상대 경로만 조정 |
| T-02 | `resolve` 우선순위 표(오버라이드/무효 id/꺼진 플러그인/내장 면역/확장자 없음) | PLUG-002·003 | ◎ | 표 주도 테스트로 확장 |
| T-03 | `plugin_dirs()` — OS별 기대 경로(설정 폴더 재지정 환경변수로 격리), 중복 제거, 같은 id는 사용자 설치분 채택 | PLUG-005 | ◎ | 두 폴더에 같은 id 모듈을 두고 채택 확인(신규) |
| T-04 | 동봉 `markdown.wasm` E2E — h1·불릿·체크·표 박스·인라인 정리·sequence | PLUG-110~116 | ◎ | 기존. SVG 래스터 도입 후 `\u{1}img\|` 단언 강화 |
| T-05 | 동봉 `archive.wasm` E2E — cpio/ar/ISO/미지원 형식 | PLUG-117~120 | ◎ | 기존 |
| T-06 | 격리: 무한 루프(연료) · 임포트 루프(연료+시간) · `read_at` 누적 64MB · 2500 멤버 ar 통과 · 깨진 모듈 격리 · 8MB 초과 거부 | PLUG-020~029 | ◎ | 기존 + 8MB 초과(신규) |
| T-07 | 브레이커: 3회 실패 후 격리 문구, 성공 시 카운터 복귀 | PLUG-034 | ◎ | 복귀 경로 신규 |
| T-08 | ABI v2 암호 흐름: 암호 없음 → `NeedPassword` → 스코프 주입 → 목록 · 스코프 종료 후 슬롯 비움(패닉 경로 포함) | PLUG-030·032·080 | ◎ | 패닉 경로 신규 |
| T-09 | 세션 암호: 성공만 기억·실패 시 폐기·`Debug` 마스킹·디스크에 흔적 없음(설정 파일 저장 후 문자열 검색) | PLUG-079·085 | ◎ | 설정 파일 검색 신규 |
| T-10 | 압축 리더 손상 입력 퍼즈성 회귀(§6-6~9 전 항목) — 패닉·무한 루프 없음 | PLUG-095·098~102 | ◎ | 기존 테스트가 대부분 보유. 시간 제한을 건 러너에서 실행 |
| T-11 | 포맷별 정상 목록(zip/tar/rar4/rar5/cab/gzip/단일 스트림/7z 판정) | PLUG-098~105 | ◎ | 기존 |
| T-12 | `fmt_entry_time` 이중 보정 없음 · `summary_lines` 상태별 문구·60행 절단·`… 외 N개` | PLUG-046·083 | ◎ | 절단 신규 |
| T-13 | `row_cells` 컬럼 순서·폴더 빈칸 · `sort_entries` 수치/문자/다중 키/동률 경로순 | PLUG-073·074 | ◎ | 기존 3 + 다중 키 신규 |
| T-14 | 도크 태그 벗기기(hr → `─`×40, q → `│ `, 그 외 제거) | PLUG-042 | ◎ | 순수 함수로 분리해 테스트(신규) |
| T-15 | 미리보기 창 `parse_kind`·`selected_text`(여러 줄·마커 제외·역방향 선택)·`select_all`·`load_bmp`(top-down/bottom-up/손상) | PLUG-040·054·057~059 | ◎ | dir2에는 테스트 없음 — 로직을 OS 중립 모듈로 분리해 신규 작성 |
| T-16 | 미리보기 창 렌더 스냅샷: 종류 7종·선택 하이라이트·인라인 이미지·다크/라이트 | PLUG-052·054·062 | ○ | 오프스크린 `Surface`에 그려 해시/골든 비교 |
| T-17 | 미리보기 창 입력: 키 스크롤 표, 휠 3경로, 드래그 자동 스크롤(타이머 틱 주입), Esc 닫기 | PLUG-055~057·060 | ○ | 이벤트 주입 |
| T-18 | 압축 그리드 창: 컬럼 폭·정렬 헤더 클릭 → 행 순서 · Ctrl+C TSV(선택/무선택) · 상태 줄 문구 | PLUG-071~077 | ○ | 클립보드는 가짜 싱크로 대체 |
| T-19 | 암호 창: 마스킹 표시·표시 토글·Enter/Esc·재시도 문구·취소 시 창 미생성·확인 후 입력란 소거 | PLUG-070·078 | ○ | 상태기계 단위 테스트 + 렌더 스냅샷 |
| T-20 | 이미지 디코드: png/jpg(기저)/bmp/gif 정상 · 프로그레시브 JPEG·webp·ico·tif = 안내 문구(또는 디코더 추가 후 정상) · 축소만(확대 없음) · 캐시 8개 | PLUG-007·043 | ◎/○ | 디코드 = ◎, 표시 = ○ |
| T-21 | SVG 래스터: 샘플 flowchart SVG → 기대 크기 BMP·핵심 픽셀(노드 테두리색) · 256KB 초과/2000px 초과 = 실패 | PLUG-011·025 | ◎ | 신규(공통 래스터 도입 시) |
| T-22 | 이름 디코더: CP949 바이트 zip 이름 → 한글(Windows = ACP, 타 OS = 선택 코드페이지) · 미주입 = CP437 | PLUG-084·094 | ◎ | 훅을 테스트에서 주입 |
| T-23 | 설정: `preview_map`·`plugins_disabled` 왕복·512B 상한·빈 값 생략 · 플러그인 페이지 체크 해제 → 즉시 내장 폴백 | PLUG-047·125·126 | ◎/○ | 왕복 = ◎ |
| T-24 | 빌드 스크립트: ps1/sh 양쪽이 같은 두 산출물을 만들고 `dist`와 바이트 동일(재현 빌드 조건에서) · `--skip-dist` 동작 | PLUG-122 | ◎(CI 잡) | 3 OS CI |
| T-25 | 배포물 구조: 포터블/번들/패키지에서 동봉 플러그인 2개가 설정 > 플러그인에 보임 | PLUG-005·123 | △ | 설치 스모크 스크립트로 일부 자동화 가능 |
| T-26 | 실기: F3·↗ 왕복, 대형 마크다운(400줄 상한 문구), 암호 zip(CD 암호화)·헤더 암호화 rar, UNC/느린 매체의 큰 아카이브 체감 | PLUG-050·070 | △ | 체크리스트 |
| T-27 | **상시 점검 로직**(시작 시 자가 진단 후보): 동봉 플러그인 2개 로드 성공·`nx_meta` id 일치(`markdown`·`archive-sample`)·로드 오류 0건 — 실패 시 로그/상태 표시 | PLUG-020·035 | ◎ | "핵심 기능 오류 즉시 확인" 요구 대응 |
