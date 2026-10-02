# port/90 · 검증 매트릭스 (이식 원장 ID ↔ 구현 ↔ 시험)

> 규칙: **기능 ID 1개 = 시험 1개 이상 또는 사유**. 상태 = ☐ 미착수 · 🚧 진행 · ✅ 구현+시험 · ⚠ 의도된 차이(DR 번호) · 🖐 실기 필요(사유). 마일스톤 끝마다 빈칸을 센다([15 §1](../15-dev-methodology.md) 교차 검증).
> 행은 접두별로 묶는다. 원장 전체 ID 목록은 각 port 문서가 원천이며, 여기에는 **착수한 ID부터** 추가한다(전수 목록은 M8 T-90에서 생성 스크립트로 채운다).

## 집계

| 접두 | 원장 항목 | 매트릭스 행 | ✅ | 🚧 | ⚠ | 🖐 | 갱신 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SKEL | 291 | 5 | 0 | 5 | 0 | 0 | 10-03 |
| CI | 120 | 6 | 1 | 5 | 0 | 0 | 10-03 |
| OPS | (22 문서) | 5 | 5 | 0 | 0 | 0 | 10-03 |

## 행

| ID | 기능 | 구현 위치 | 층 | 시험 | 상태 | 비고 |
| --- | --- | --- | --- | --- | --- | --- |
| SKEL-401 | GUI 크레이트 3층(main · app/ · 호스트) | `crates/nexa-dir/src/main.rs` | — | — | 🚧 | M0 뼈대만 |
| SKEL-405 | 작업공간 설정(lints · 프로필 · 경로 의존 · 정적 CRT) | `Cargo.toml` `.cargo/config.toml` | T0 | CI | 🚧 | 형제 의존은 M1에서 |
| SKEL-413 | `--smoke`(창 없음 · CI 게이트) | `nexa-dir/src/main.rs` | T5 | `cli::tests` | 🚧 | 항목은 M1~M5에서 |
| SKEL-436 | 환경 변수 한 벌(`NDIR_HOME` …) | [18 §5](../18-build-and-test.md) | — | — | 🚧 | 이름 확정 · 구현 M3 |
| SKEL-440 | 순수 함수 우선(인자 해석) | `nexa-dir/src/cli.rs` | T1 | `cli::tests::*` | 🚧 | |
| CI-109 | `--smoke` | 위 | T5 | | 🚧 | |
| CI-110 | `--selfcheck`(표 · `--json` · `--ci` · `--only`) | `nexa-dir/src/selfcheck.rs` | T5 | `selfcheck::tests` | 🚧 | env 그룹만 실제 |
| CI-113 | `ci.yml` 3-OS | `.github/workflows/ci.yml` | T0 | CI | 🚧 | wasm32·임포트 단계는 T-07 |
| CI-115 | `check-all` | — | — | — | ☐ | T-05 |
| CI-116 | 검증 매트릭스 | 이 문서 | — | — | 🚧 | |
| CI-119 | 규칙 문서 dir3판 | `docs/15·16·18` · `CLAUDE.md` | — | — | ✅ | |
| OPS(core) | `nexa-core` → `ndir-core`(FileKind · Secret 소거) | `crates/ndir-core` | T1 | dir2 테스트 6 | ✅ | DR-11 전수 이식 |
| OPS(vfs) | `nexa-vfs` → `ndir-vfs`(열거 · MY_PC · 압축 5형식) | `crates/ndir-vfs` | T1 | dir2 테스트 40(Windows 전용 2 포함) | ✅ | 비Windows `MY_PC` 동작은 T-43에서 확인 |
| OPS(tree) | `nexa-tree` → `ndir-tree` | `crates/ndir-tree` | T1 | dir2 테스트 20(Windows 전용 1) | ✅ | |
| OPS(ops) | `nexa-ops` → `ndir-ops`(전송·히스토리·일괄 이름 변경) | `crates/ndir-ops` | T1 | dir2 테스트 25 + ignored 1(Windows 전용 1) | ✅ | |
| TERM(vt) | `nexa-term` → `ndir-term`(VT 파서·스킴·복사 서식) | `crates/ndir-term` | T1 | dir2 테스트 56 | ✅ | PTY·뷰는 M4·M5 |
| KEY-15xx/16xx | i18n 메타·키 498(3언어 파리티) | `crates/ndir-i18n/lang` + `build.rs` | T0(빌드) · T1 | `builtin_langs_parse_and_key_parity` · 빌드 검사 | ✅ | |
| EXT-201~203 | i18n 크레이트 · 자원 · 빌드 검사 | `crates/ndir-i18n` | T1 | 9 시험 | ✅ | 표 생성(Msg enum)은 DR-14로 하지 않음 |
| EXT-207 | 사용자 오버레이 층 | `ndir_i18n::load(code, home)` | T1 | `merge_override_fallback_and_resolve` | ✅ | home = 설정 폴더(M1 T-13 연결) |
| EXT-212 | `del.lockedMsg`/`failMsg` `{1}` 결함 | `lang/*.lang` | T1 | `locked_and_fail_messages_carry_list_placeholder` | ✅ | 빌드 검사가 재발 방지 |
| EXT-214 | OS 종속 문구 15건 | — | — | — | ☐ | 기능 이식 때 3언어 동시 수정 |
| CI-109 | `--smoke` = `--ci` 전체 | `nexa-dir/src/main.rs` | T5 | CI | 🚧 | 그룹 env·config·resources 실제 |
| SET-001~008 | nexa-conf 저장 계층(파서·직렬화·원자 쓰기·`user_config_dir`·포터블 판정) | 형제 의존 `nexa-conf` | T1 | nexa-conf 14 시험 | ✅ | `config_dir()`가 DR-9 순서로 조합 |
| SET-010~024 | 레지스트리 타입·종류·곁 표 함수·검증·테마 모드 | `ndir-settings/src/lib.rs` | T1 | `registry_defaults_are_valid_and_keys_unique` · `size_units_and_theme_mode` | ✅ | `Lang` 종류 제외(DR-14) |
| SET-030~038 | `Settings` 열기·읽기·쓰기·초기화·저장·목록·설정 폴더 | 〃 | T1 | `set_validates_and_saves_only_changes` · `corrupt_value_falls_back_and_unknown_keys_survive` · `config_dir_honors_env_home` | ✅ | `effective` 없음 |
| SET-040~044 | 이주(옛 기본값·이름 바꿈·단위 변환·fail-soft) | 〃 | T1 | `renamed_and_rescaled_tables_are_consistent` | ✅ | 표는 비어 시작 |
| SET-050 | JSON 코덱·내보내기/가져오기 | `ndir-settings/src/json.rs` | T1 | json 2 시험 | ✅ | 그대로 복사 |
| SET-051~053 | 성능 거버너 | — | — | — | ⚠ Q-7 | 도입하지 않음 |
| SET-120~124·132·134·137 | 무결성 시험·결함 회피 | `ndir-settings` tests | T1 | `side_tables_reference_existing_keys_and_labels` 외 | ✅ | SET-132 신규 시험 |
| PREFS-101~170 · KEY-501~570 | dir2 설정 키 → dir3 레지스트리 + 가져오기 | `ndir-settings/src/{registry,migrate}.rs` | T1 | `defaults_follow_dir2` · migrate 3 시험 | ✅ | 세션 키(PREFS-201~)는 T-45 |
| KEY-591~598 | 신규 키(term.shell · window.* · license.gates · ui.prefs_advanced · text raster · dblclick · scroll_natural) | `registry.rs` | T1 | 레지스트리 무결성 | ✅ | `gfx.*`/`clipboard.x11_native`는 호스트 이식 때 |
| CI-110(config) | 자가 점검 config 그룹 | `nexa-dir/src/selfcheck.rs` | T5 | 스모크 | ✅ | |
| LIC-151~157 | `ndir-license` 크레이트 · PRODUCT · 빌드일 · 폴더 순서 · Feature/Tier · 상태·판정·설치 API | `crates/ndir-license` | T1 | 8 시험(`other_machine_outdated_expired_and_wrong_product` 등) | ✅ | Feature 변형 0(Q-1 기본값) |
| LIC-164 ①③ | 단위 시험 이식 · 타 제품 파일 거부 · `*` 번들 | 〃 | T1 | 〃 | ✅ | ② 기동 명령 `license.*`는 T-46/T-80 |
| LIC-192 | `Product.version` = 앱 버전 | `Cargo.toml version.workspace` | T5 | selfcheck `license/product` | ✅ | |
| LIC-181~182 | 기기 ID 원천 · 기기 공용 폴더(3-OS) | nexa-license `machine.rs` · `machine_dir()` | T5 | selfcheck `license/machine id`·`installed state` | ✅ | |
| CI-110(license) | 자가 점검 license 그룹 5항목 | `nexa-dir/src/selfcheck.rs` | T5 | 스모크 | ✅ | |
| CMD-120~159 · 287~329 | dir2 명령 상수·단축키 → 명령 표 48(문자열 id · OS별 기본 코드) | `ndir-settings/src/commands.rs` | T1 | `dir2_windows_bindings_resolve`(31건) · `every_default_code_parses` | ✅ | 동적 명령(언어·런처·클라우드 i)은 호스트 조립(M3) |
| CMD-480~497 | macOS 대응안 | 〃 | T1 | `macos_preset_follows_cmd_480_497`(13건) | ✅ | ⌘Y 충돌 → 미리보기 ⇧⌘Y(시험 적발) |
| CMD-487 | `tab.prev`(dir2에 없음) | 〃 | T1 | 〃 | ⚠ DR-3 | 의도된 차이(권장안 채택) |
| CMD-491 | F6 단일 표기(`view.theme_cycle`) | 〃 | T1 | 〃 | ⚠ DR-3 | 의도된 차이 |
| SET-090~095 · 130 | 명령 표·프리셋·Chord·Keymap·`key.<id>` 전수 등재·repeatable | `ndir-settings/src/{commands,keymap}.rs` | T1 | `every_command_has_key_entry_and_label` · `no_default_conflicts_within_a_preset` · `overrides_none_sequences_and_conflicts` | ✅ | `from_winit`은 M3 |
| SET-096~097 | 단축키 창 · 변경 적용 | — | — | — | ☐ | M3 T-44 |
| PREFS-307(단축키 페이지) | 설정 창 "단축키" 분류 신설 | `registry.rs` CAT_KEYS | — | — | ⚠ DR-3 | 의도된 차이(dir2에 없던 페이지) |
| UIK-211 · UIC-316 | InputEvent 더블클릭·가운데·X 버튼 · 휠 줄 수 | nexa-ui `nexa-ctl/src/event.rs` | T2 | `new_mouse_variants_carry_coordinates` · `wheel_lines_clamps_and_restores` | ✅ | 103차 |
| CI-104 | `RecordCtx` 공용 기록기 | nexa-ui `controls/mod.rs` | T2 | `record_ctx_collects_and_judges_bounds` | ✅ | 103차 |
| UIK-205 · GUI-060~066 | MenuBar 단축키 열·체크/라디오·활성·프로그램 열기 | nexa-ui `controls/pulldown.rs` | T2 | `shortcut_column_and_check_marks` · `set_enabled_toggles_and_open_menu_index` | ✅ | 104차 · Alt 니모닉은 호스트(M3) |
| UIK-206 · GUI-074 | Toolbar 토글 켜짐(강조색 38 % 블렌드) | nexa-ui `controls/toolbar.rs` | T2 | `checked_toggle_draws_accent_blend_background` | ✅ | 104차 · 오버플로 보류 |
| UIK-207 · GUI-040·046 | TabBar 아이콘·툴팁·가운데 클릭 | nexa-ui `controls/tabbar.rs` | T2 | `icons_tips_and_middle_click` | ✅ | 104차 · press 전환 옵션 보류 |
| UIK-203 · GUI-080 | StatusBar | nexa-ui `controls/statusbar.rs` | T2 | `set_text_invalidates_only_on_change_and_right_aligns` · `right_text_never_goes_left_of_pad` | ✅ | 104차 |
| UIK-201·216·217 · PANEL-1F(rows·columns·typeahead) · GUI(fastscroll·edit) | 가상 행 그리드 엔진(가상화·컬럼·정렬·선택·계층·인라인 이름 바꾸기·보기 모드·픽셀/고속 스크롤·타입어헤드) | nexa-ui `crates/nexa-grid` | T1·T2 | dir2 테스트 52(rows 34 · edit 8 · fastscroll 5 · typeahead 4 · columns 1) + `adapt_forwards_to_ctl_ctx` | ✅ G-1 | 아이콘(`draw_icon`)·italic은 G-2/U-5 |
| UIC-310·311·313·315 | DrawCtx 클립 스택·터미널 셀·italic·List 슬롯 | nexa-grid `draw::Adapt` 우회(`List`→`PeerList` · italic 버림) | — | — | 🚧 | T-31에서 nexa-ctl 보강 |
| GUI(위젯 틱 요청) | `Invalidations::request_tick` | nexa-ui `widget.rs` | T2 | `tick_request_is_idempotent_and_taken_once` | ✅ | 105차 |
| UIK-202 · GUI-090~101 | PathBar(브레드크럼·편집·자동완성 팝업) | nexa-ui `nexa-explorer/src/pathbar.rs` | T2 | dir2 시험 7 | ✅ | 106차 · 경로 문법·제안은 호스트 |
| UIK-204 · GUI-110~ | InfoDock(스트립·텍스트/이미지·선택·팝아웃·오버레이 바) | nexa-ui `nexa-explorer/src/dock.rs` | T2 | dir2 시험 13 | ✅ | 106차 · 이미지 그리기는 호스트 `IconImage`(G-2) |
| GUI-110 · X-6 | 도크 배치(전폭 밴드 · 높이 % · 좌/우 분할 % · 단일 정보 전폭 · 숨김 = 0) | `main.rs::layout_core` | T2·T4 | `dock_layout_and_contents` · `layout_scales_and_stays_inside` · 골든 · `dock-info.scn` | ✅ | 10-03 §23 |
| GUI-111 | 종류 스트립(정보/미리보기/터미널 · →) 클릭 전환 · 내용 갱신은 호스트 | `app/input.rs`(Area::Dock) · `update_docks` | T2 | `dock_layout_and_contents`(클릭 → "hello") | ✅ | → 버튼·↗ 팝아웃 = 토스트(T-61/T-62) |
| GUI-112 | 정보 줄(이름·종류·경로·크기·만든/수정/액세스 · 다중 선택 수 · 현재 폴더) | `dockinfo.rs::info_lines` | T2 | `info_and_preview_on_temp_tree` · core "Name: a.txt" | ⚠ | 디스크 할당 크기·형식별 상세(Windows 속성 시스템) T-5x |
| GUI-113 | 미리보기 텍스트(64 KiB · NUL = 바이너리 · 빈 파일 · 200줄 · 탭 4칸) · 폴더/없음 = none · 실패 | `dockinfo.rs::preview_content` | T2 | `preview_lines_rules` · `info_and_preview_on_temp_tree` | ✅ | 플러그인·압축 미리보기 T-62 |
| GUI-114 | 미리보기 이미지(확장자 → `set_image(경로)`) | `dockinfo.rs::is_image_ext` | T2 | `info_and_preview_on_temp_tree`(p.png) | ⚠ | 실제 그리기 = T-31 nexa-grid `Adapt::draw_image` 뒤 |
| GUI-115 | 도크 갱신 길목 = `update_status`(선택·폴더 변경마다) · 키 `종류|선택수|대상` 유지 | `main.rs::update_docks` | T2 | `dock_layout_and_contents` | ✅ | dir2 `update_dock_info` 동일 |
| TERM-01 · DR-5 | Pty 포트 구현 3-OS(ConPTY · forkpty) · 비차단 읽기 · 리사이즈 · 종료 감지 | `platform/winpty.rs` · `platform/unixpty.rs` | T2(실기) | `conpty_runs_cmd_echo` · `forkpty_runs_sh_echo` | ✅ | 실기 교훈: STARTF_USESTDHANDLES · 종료 flush(§24) |
| TERM-02 | VT 화면 → 셀 격자 렌더(런 배경 · 셀 글자 · 전각 · faint · 선택 반전 · 캐럿 · 종료 안내) | `termview.rs::paint` | T2 | `lifecycle_with_fake_pty` · core `terminal_dock_with_fake_pty` | ✅ | 고정 열·가로 스크롤 T-61 B |
| TERM-03 | 지연 시작(cwd = 원천 패널) · 폴링 틱 30 ms · 캐럿 530 ms · 살아 있을 때만 깸 | `app/term.rs::paint_terms/term_tick/term_wake` | T2 | `terminal_dock_with_fake_pty`(spawn 로그 cwd) | ✅ | |
| TERM-04 | 키 라우팅(포커스 중 목록 단축키 차단 · 화살표/Home/End/Del/PgUp/Dn/Enter/Esc/BS/Tab · Ctrl+글자 제어 문자 · 복사/붙여넣기 조합) | `app/term.rs::term_key/term_ctrl` · `event_loop.rs` | T2 | `terminal_dock_with_fake_pty` · `key_and_char_sequences` | ✅ | |
| TERM-05 | 마우스(격자 클릭 = 포커스 · 드래그 선택 · 엣지 자동 스크롤 · 휠 = 스크롤백 3줄) · 패널 클릭 = 포커스 해제 | `app/input.rs` · `termview.rs` | T2 | `terminal_dock_with_fake_pty` | ✅ | TUI 마우스 모드(DECSET 1000/1006) T-61 B |
| TERM-06 | → 버튼 = `cd "<폴더>"`(cmd `/d`) · 종료/실패 뒤 아무 키·→ = 재시작 | `app/term.rs::term_goto` | T2 | `terminal_dock_with_fake_pty`(reset) | ✅ | |
| TERM-07 | 팔레트 선택(`term.theme` system/dark/light/스킴 id + `theme_dark/light` 폴백) · 셸 선택(`term.shell`) | `app/term.rs::term_palette/term_shell` | T1 | ndir-term `resolve_scheme` 시험 | ✅ | 터미널 글꼴 크기(`term.font_size`) T-61 B |
| TERM-08 | 실제 셸 왕복(기동 명령 `term.focus/term.send/term.dump` · 3-OS) | `app/startup_cmd.rs` | T4 | `term-echo.scn` | ✅ | |
| TERM-09 | 복사 서식 HTML/RTF(`term.copy_format`) · 고정 열/가로 스크롤(`term.wrap/cols`) · 트랙패드 픽셀 스크롤 · 고속 스크롤 | — | — | — | ☐ | T-61 B |
| PLUG-001~008 | 공급자 시임(계약 · `preview.map` 오버라이드 · `plugins.disabled` 내장 면역 · 지연 캐시 · 내장 archive→image→text) | `preview/mod.rs` | T2 | `declared_ext_routes_and_text_falls_back` · `preview_map_overrides_declared_match` · `disabled_plugin_is_skipped_but_builtin_immune` · `archive_ext_routes_to_builtin_provider` | ✅ | dir2 그대로 + 공급자 id 반환 |
| PLUG-005 · EXT-401 | 탐색 경로(`NDIR_PLUGINS_DIR` → 설정 폴더 `plugins` → 동봉 `<exe>/plugins`·mac Resources·Linux share) · 같은 id 앞선 것 | `preview/mod.rs::plugin_dirs` | T2 | `plugin_dirs_are_plugins_folders_without_duplicates` · `seam_routes_bundled_plugins_by_declared_ext` | ✅ | 관리 설치본 하위 폴더 = T-63 |
| PLUG-020~035 | WASM 런타임(로드 · 버퍼 · 격리 연료/메모리/벽시계 · 임포트 7종 과금 · `read_at` 누적 상한 · nx_preview/nx_archive · 브레이커 · 로드 오류 격리) | `preview/wasm.rs` | T2 | `loads_meta_runs_preview_and_fuel_traps_infinite_loop`(+8MB) · `archive_capability_routes_to_nx_archive_and_password_flow` · `read_at_fuel_allows_thousands_of_members` · `read_at_total_bytes_cap_traps_runaway_reads` · `breaker_…_and_recovers` | ✅ | 수치 dir2 동일(DR-7) |
| PLUG-040~042 · 046 | 라인 태그 벗기기(도크) · 압축 요약 60행 · 상태별 문구 | `preview/mod.rs::dock_lines` · `preview/archive.rs::summary_lines` · `dockinfo.rs` | T2 | `dock_lines_strip_tags` · `summary_lines_lead_with_format_counts_and_truncate` · `failure_status_maps_to_user_action` | ✅ | 인라인 이미지 그리기 T-31 |
| PLUG-079~083 | 세션 암호 캐시(메모리만) · 활성 슬롯 스코프 가드(패닉 포함) · `read_via` · DOS 시각 이중 보정 금지 | `preview/archive.rs` | T2 | `password_cache_is_memory_only_and_forgettable`(+패닉 경로) · `dos_times_are_not_shifted_twice` | ✅ | 암호 창·그리드 창 T-62 B |
| PLUG-084 | 구형 이름 디코더(Windows CP_ACP) | `preview/archive.rs::install_name_decoder` | — | — | ⚠ | 호출 배선·비Windows iconv = T-62 B |
| PLUG-110~120 · 124 | 동봉 markdown.wasm·archive.wasm E2E(h1·불릿·체크·표·인라인 정리·Mermaid 폴백·ISO/ar/cpio) | `preview/sample_tests.rs` · `plugins/*.wasm` | T2 | `bundled_markdown_plugin_end_to_end` · `bundled_archive_plugin_lists_iso_ar_and_cpio` | ✅ | Mermaid 이미지 마커 단언은 SVG 래스터 뒤 |
| PLUG-042 · T-27 | 도크 미리보기 = 시임 소비(공급자 id 덤프) · 자가 점검 `plugin`(경로·동봉 2종·로드 오류) | `main.rs::update_docks` · `selfcheck.rs::check_plugin` · `startup_cmd.rs`(preview.dump · list.select) | T2·T4 | `preview_plugin_renders_markdown_in_dock` · `preview-plugin.scn` | ✅ | |
| PLUG-050~053 · 055~056 · 060~062 | F3/↗ 독립 미리보기 창(단일 선택 파일 · 소유자 3/4 · 스타일드 7종 · 글꼴 Base/Mono/굵게 · 스크롤 휠/키 · Esc · 테마 색) | `preview_win.rs` · `app/previewcmd.rs` | T2·T4 | `kinds_and_dump_without_window` · `preview_window_and_archive_password_flow` · `preview-window.scn` | ✅ | 모달 아님(소유 창) |
| PLUG-054 · 057~059 | 인라인 이미지 · 드래그 문자 선택 · 부분 복사 · 우클릭 메뉴 | — | — | — | ☐ | T-62 C(Ctrl+C = 전체 복사만) |
| PLUG-070 · 078~081 | 암호 입력 창(마스킹 · 재시도 문구 · 취소 = 창 없음) · 세션 기억 · `read_via` 재조회 | `app/previewcmd.rs::ask_archive_password/archive_password_result` · `dlg_win.rs` | T2 | `preview_window_and_archive_password_flow` | ✅ | 암호 표시 체크박스는 생략 |
| PLUG-071~077 | 압축 그리드 창(컬럼 8 · 정렬 · TSV 복사 · 상태 줄) | — | — | — | ☐ | T-62 C(지금 = 요약 텍스트 창 · nexa-ui DataGrid 필요) |
| SHELL §2-1 · §2-2 | 파일 행 메뉴(열기 · 편집 · 삭제/완전 삭제/이름 바꾸기 · 경로/이름 복사 · 폴더에 붙여넣기 · 새로 만들기) · 배경 메뉴(붙여넣기 · undo/redo 설명 · 새로 만들기 · 새로 고침) · 우클릭/Shift+F10 · 비활성 패널 활성화 | `app/ctxmenu.rs` · `panel.rs::pending_ctx` | T2·T4 | `row_and_background_context_menus` · `ctx-menu.scn` | ✅ | 셸 항목(IContextMenu) 합류 = T-51 B · `ctx_menu_order` 설정 = T-51 B |
| SHELL-001 · 002 · 005 · 006 · 012 · 013 | 셸 컨텍스트 메뉴(PIDL · 공통 부모 축소 · IContextMenu · HMENU 열거 · 서브메뉴 2단 · verb 가로채기 · InvokeCommand 뒤 재열람 · id 대역 `shell:1..0x6FFF`) | `platform/winshell.rs` · `app/ctxmenu.rs::shell_to_ctx` | T2(실기)·T5 | `shell_menu_lists_items_for_temp_file` · `row_and_background_context_menus`(가짜 포트) | ✅ | Windows만 · 네이티브 HMENU 대신 nexa-ctl 그림 |
| SHELL-003 · 004 · 007 · 008 · 009 · 011 · 014 · 015 · 019 | 키보드 앵커 위치 · Shift+우클릭 확장 동사 · 고유 항목 앵커 삽입 · 새로 만들기 ▸ ShellNew · 배경 셸 메뉴 · 동적 아이콘 · 메뉴 스레드 · 선행 구축 · `ctx_menu_order` | — | — | — | ☐ | T-51 B-2(일부는 dir3 구조상 불필요 — 011·014·015) |
| PLUG-122 · EXT-417 | 플러그인 빌드 스크립트 3-OS(`plugins.list` 단일 출처 · `--out-dir`/`--skip-dist`) | `scripts/plugin-build.{sh,ps1}` | T4(CI) | CI `plugins` 잡 · 로컬 실기 §26 | ✅ | |
| PLUG-123 · CI-113 · T-07 | CI wasm32 빌드 → 갓 빌드한 .wasm 로드 검증(`--selfcheck --only plugin`) | `.github/workflows/ci.yml` | T4 | `plugins` 잡 | ✅ | 바이트 동일성은 묻지 않음(툴체인) |
| PLUG-125 · EXT-414 | 설정 창 플러그인 페이지 — 로드 목록·오류 설명 줄(`plugins.disabled`) | `app/windows.rs::plugin_note` | — | — | ⚠ | 체크박스 묶음 = T-63 B |
| CI-110 · CI-111 · T-54 | Help ▸ 자가 점검 창(백그라운드 `selfcheck::run` · 판정 색 표 · 요약 · 다시 점검 · 복사 · Esc) · 덤프 `check` | `check_win.rs` · `app/windows.rs` · `app/menus.rs`(help.selfcheck) | T2·T4 | `report_rows_counts_summary_and_table` · `start_runs_in_background_and_reports` · `help_selfcheck_requests_check_window` · `selfcheck-win.scn` | ✅ | 그룹 SKIP 항목은 해당 슬라이스가 채움 |
| OPS-001~003 · 006~008 | 붙여넣기 = `transfer` 단일 경로(작업 스레드 · 진행 이벤트 · 결과 집계 · 같은 폴더 규칙) | `app/ops.rs::start_transfer/ops_tick/finish_transfer` | T2·T4 | `copy_cut_paste_undo_through_ops` · `copy-paste.scn` | ✅ | 엔진 시험은 ndir-ops 자체 |
| OPS-004 | 충돌 4버튼 확인 창(덮어쓰기/모두 덮어쓰기/건너뛰기/취소 · 작업 스레드 채널 질문 · "모두"만 이후 무확인 · 취소 = 중단) | `app/ops.rs`(closure + `conflict_rx`) · `app/dialogs.rs::conflict_ask` | T2 | `dialogs_delete_permanent_and_paste_conflict` | ✅ | |
| DLG-001 · T-29 | 대화상자 창(제목 · 줄 바꿈 본문 · 버튼 N · 기본/취소 · 마스킹 입력란 · 모달 · 창 없이 결정) · 기동 명령 `dlg.pick/dlg.type` · 덤프 `dlg` | `dlg_win.rs` · `app/dialogs.rs` | T2·T4 | `spec_dump_and_headless_pick` · `delete-confirm.scn` | ✅ | 폴더 찾아보기(T-29 B) |
| OPS-016 · 영구 삭제 | `edit.delete_permanent` = 확인 창 → `delete_permanent` → 토스트 `del.done/partialFail` | `app/dialogs.rs::delete_permanent_ask` | T2·T4 | 위 | ✅ | undo 없음(dir2 동일) |
| OPS-030~035 · 039 | undo/redo(이동 되돌림 · 복사 = 휴지통 포트 삭제 주입 · 완료 시 기록 규칙 · 문구 변환) | `app/ops.rs::history_step` · `Platform.trash: Rc` | T2 | `copy_cut_paste_undo_through_ops` | ✅ | 휴지통 복원 undo(OPS-036) T-51 B |
| M3-5 · 파일 클립보드 | `edit.copy/cut` = OS 파일 클립보드 + 앱 내 사본 · `edit.paste` 2단 · 대상 규칙(`paste_dest`) | `app/ops.rs::clip_write/clip_sources/paste_dest` | T2 | 위 | ✅ | mac/Linux OS 클립보드 = T-52/53 |
| OPS-017 · 034 | 인라인 이름 바꾸기(F2 · 이름부 선택 · Enter 확정 · Esc 취소 · 편집 키 · RenameOp · 같은 이름 무동작) | `app/ops.rs::begin_rename/apply_rename/rename_edit` · `panel.rs::key_event` | T2·T4 | `new_folder_rename_and_undo` · `new-rename.scn` | ✅ | `.lnk` 확장자 숨김 T-51 B |
| OPS-018 · 035 | 새 폴더/새 파일(unique 이름 · CreateOp · 생성 즉시 이름 바꾸기) | `app/ops.rs::create_new` | T2·T4 | 위 | ✅ | |
| CI-107 | 기동 명령 `ui.type:<text>` · `ui.press:<key>`(편집 흐름 재현) | `app/startup_cmd.rs` | T4 | `new-rename.scn` | ✅ | |
| GUI(overlaybar) | OverlayBars(두 축 오버레이 스크롤바) | nexa-ui `nexa-explorer/src/overlaybar.rs` | T2 | dir2 시험 5 | ✅ | 106차 |
| SKEL-403 · 001~020 | 호스트 껍질(present·winhost·wingeom·winfocus·theme·input·clipboard·toast) — nexa-sql 복사 · `NDIR_*` | `crates/nexa-dir/src/*.rs` | T0 | 복사 스크립트 치환 점검 · 원본 시험(wingeom·winfocus·theme·input·toast 32) | ✅ | T-40 · 보조 창 호스트(winhost)는 T-44에서 소비 |
| SKEL-424 | 창 아이콘 = dir2 `nexa-dir-256.png`/`.ico`(Windows `.rc` · mac Dock · Linux app_id) | `nexa-dir/src/icon.rs` · `packaging/branding` · `build.rs` | T2 | `dir2_png_decodes_and_downscales` · `resample_box_and_nearest` | ✅ | T-40 |
| SKEL-401·402·406~412 | `App` 골격(창·표면·자원·입력 상태·시간·깃발·진단) + `app/*.rs` 조각 + 재그리기 3원칙 | `nexa-dir/src/main.rs` · `app/{event_loop,input,paint}.rs` | T4 | `NDIR_STARTUP_CMD=layout.dump` 실증(10-03 Windows) | ✅ | T-40 · 깨움 표(워커)는 M4 |
| SKEL-421 · CMD-120~167 | 명령 한 길 `App::command` — 메뉴 5 · 툴바 13 · 단축키 · 기동 명령 | `nexa-dir/src/app/menus.rs` | T2·T4 | `menu_ids_are_commands` · `menus_build_with_labels` · 실증(메뉴 클릭·F6·`view.hidden`) | 🚧 | T-42 · 미구현 id = 상태줄 `cmd.notYet` |
| CMD-287~329 · 480~497 | winit 키 → `Chord`(IME 모드 물리 키 · 숫자 물리 키) · 2단 · 자동 반복 가드 | `nexa-dir/src/app/keywinit.rs` · `event_loop.rs` | T2 | `ascii_letters_and_named_keys` · `ime_and_digit_use_physical_key` | ✅ | T-40 |
| PANEL-1A~1F(목록) | 파일 목록 = `ndir-tree` → nexa-grid `RowSource`(컬럼 4 · 펼침 · 선택 · 정렬 · 타입어헤드 · 크기/시각 서식) | `nexa-dir/src/filelist.rs` | T2 | `opens_folder_dirs_first_and_toggles` · `size_and_time_format` · `missing_folder_is_empty_with_error` | ✅ | T-40 · 아이콘(G-2)·시간대(T-72)는 뒤 |
| CI-106·107 | 기동 명령(`NDIR_STARTUP_CMD`) · `layout.dump` · `@after` · `ui.*` 포인터 합성 · `key:` · `app.exit` | `nexa-dir/src/app/startup_cmd.rs` | T4 | 실증(덤프 2장 · 종료 1.5 s) | 🚧 | T-46(`assert`·패닉 훅·골든 비교 러너 T-06) |
| CI-102 | 창 없는 AppCore — `viewport`/`scale` 주입(`layout_for`) · `paint_into(&mut dyn DrawCtx)` · `layout()` 창 무관 | `nexa-dir/src/main.rs` · `app/paint.rs` | T3 | `app/core_tests.rs` 5(`route_and_commands_without_window` · `startup_cmd_vocabulary` …) | ✅ | T-41 · FakePlatform(포트 trait)은 T-50 |
| CI-105 | 배치 골든(Rect 트리 · 픽셀 아님) + `RecordCtx` 표면 밖 0 · 글자 존재 | `tests/golden/layout-1200x800.txt` · `app/core_tests.rs` | T3 | `layout_golden_1200x800` · `layout_scales_and_stays_inside` · `paint_records_inside_surface` | ✅ | T-41 · 3-OS 동일성은 CI가 판정 |
| UIC-310(클립) · PANEL(열 폭) | 셀이 패널 경계를 넘침 → 호스트가 이름 열로 폭을 맞춤(임시) | `main.rs` `layout_core` `cols_for` | T3 | `paint_records_inside_surface` | ⚠ | 근본 = T-31 클립 스택 · 열 폭 기억/동기는 T-43 |
| PANEL-001·002 · 031·032 | 패널 = 탭 바 + 네비([홈][←][→][↑]) + 경로 바 + 목록 수직 스택 · 네비 활성 동기 | `nexa-dir/src/panel.rs` | T2·T3 | `layout_stacks_tabbar_navbar_rows`(dir2 수치) · `nav_buttons_and_path_edit` · 골든 | ✅ | T-43 · 글리프는 유니코드(MDL2 → SVG T-30) |
| PANEL-012·014~017 | 탭 = 독립 뷰 + 히스토리 · 새 탭 복제 · 전환/순환 · 닫기(≥1) · 드래그 재정렬 | `panel.rs` | T2 | `tabs_open_switch_close_keep_at_least_one` · `route_and_commands_without_window` | 🚧 | 잠금·고정·복제·패널 간 이동·stale 재열람은 T-43 잔여 |
| PANEL-026~029 · 033 · 036 | 탭별 back/forward · 경로 진입(실패 = 위치 유지) · 위로 + 떠난 폴더 자동 선택 · 홈 = 내 PC · 행 활성화 · 무간섭 재열람(캐럿·스크롤) | `nav.rs` · `panel.rs` | T2 | nav 3 · `per_tab_history_and_nav_up_selects_left_folder` · `activate_enters_dir_and_reports_file` | ✅ | 선택 복원·사라진 폴더 폴백(037)은 잔여 |
| WINA(layout) · PANEL §2-5 | 창 배치 = 메뉴/도구 28/[좌 ║ 우]/상태 22 · 스플리터 드래그·50 % 스냅(Alt 해제)·최소 200 · 열 기본 5(340·64·96·140·110) | `main.rs` `layout_core`/`split_drag` | T3 | `layout_golden_1200x800` · `splitter_drag_and_snap` · `px_rounds_and_columns_fit` | ✅ | 열 폭 기억·동기(`list.col_width_sync`)는 잔여 |
| PREFS-050 · PANEL §5-2 | 세션 파일 = dir2 `session.cfg` 형식 그대로(`session.conf` · 레거시 이름 읽기 · 원자적 저장 · 미사용 키 보존) | `nexa-dir/src/session.rs` | T2 | `session_roundtrip_with_pipe_separator`(dir2) · `parse_is_tolerant_and_loads_legacy_name` | ✅ | T-45 |
| PREFS-051·052·054 · 616·617 · PANEL-023 | 복원(창 생성 전 · 실행 인자 우선 · 실패 탭 건너뜀) · 디바운스 저장(quiet 1 s · max 5 s · 두 패널 전부 소진) · 종료 저장 | `app/sessions.rs` · `panel.rs::restore` · `main.rs` | T3·T4 | `session_roundtrip_through_app` · 창 2회 실행 실증 | ✅ | T-45 · 펼침(`exp`)·잠금/고정 복원은 잔여 |
| CI-106 | 기동 명령 확장 — `@ready`/`@idle`/`@after` · `quit[:코드]` · `assert.<대상>:<식>`(종료 코드 3) · `ui.click:@영역` · `ui.key` | `app/startup_cmd.rs` | T3·T4 | `startup_ready_assert_and_dumps` · `classify_prefixes` · 창 실증(exit 5 / exit 3) | ✅ | T-46 · `@idle` 작업 큐는 M6 |
| CI-107 · CI-081 | 덤프 어휘 — `layout` `panel` `list` `tabs` `status` `menu` · `dump:` 전부 | `app/startup_cmd.rs::dump_of` | T3 | `startup_ready_assert_and_dumps` | 🚧 | T-46 · tree/ops/term/preview/prefs/plugin/license/log 덤프는 각 기능과 함께 |
| CI-112 · CI-072 | 패닉 훅 → `<HOME>/crash/crash-<unix>.txt`(버전·OS·마지막 명령) · 다음 기동 안내 1회 | `nexa-dir/src/crash.rs` | T2 | `report_has_version_os_and_last_command` · `unreported_is_taken_once_and_newest_first` | ✅ | T-46 |
| CI-108 | `ndir-check` 시나리오 러너(의존 0 · `.scn` · 격리 홈·샘플 트리 · 종료 코드/패닉/검사식 · summary) + 시나리오 5 | `crates/ndir-check` · `tests/scenarios/*.scn` | T4 | 러너 시험 3 · 시나리오 5 PASS(Windows 로컬·CI) | ✅ | T-06 · Linux xvfb·macOS 러너는 후속 · 캡처 플러그 없음 |
| SET-060~097 · PREFS-301~311 | 설정 창(검색 · 트리 · 카드 · 종속 잠금 · 고급 · 복사 · 기하 기억) — nexa-sql 복사 · dir2 사이드바 순 | `nexa-dir/src/prefs_win.rs` · `copybtn.rs` | T2·T4 | `category_search_and_advanced` · `dependent_cards_lock_and_unlock_with_parent` · 시나리오 `prefs-open` | ✅ | T-44 · JSON 편집·폴더 찾아보기·순서 편집(PREFS-312)은 잔여 |
| CMD-287~329 · SET-130 | 단축키 창(명령 48 · 캡처 · 비우기 · 초기화 · 충돌 배지) | `nexa-dir/src/keys_win.rs` | T2·T4 | `rows_follow_keymap_and_have_no_default_conflicts` · `prefs-open` | ✅ | T-44 |
| SET-090~097 · PREFS-3xx(적용) | `apply_setting` 한 길 + **적용 누락 감시**(레지스트리 전 키 = 적용 ∨ NEEDS_RESTART) · 보조 창 호스트 배선(SKEL-419) | `app/settings.rs` · `app/windows.rs` | T3 | `every_registry_key_is_applied_or_declared_restart` · `prefs_host_wiring_without_window` | ✅ | T-44 |
| PANEL-016~022 · 019·020 | 탭 잠금(닫기·분리 거부) · 고정(핀 그룹) · 복제 · 패널 간 분리/부착 · 세션 잠금/고정 | `nexa-dir/src/panel.rs` · `app/sessions.rs` | T2·T3 | `lock_pin_duplicate_detach_attach` · `tab_menu_and_column_sync` | ✅ | T-43 2차 · 드래그로 패널 간 이동은 잔여(메뉴로 가능) |
| PANEL-021 · TAB-MENU | 탭 우클릭 메뉴(nexa-ctl ContextMenu · 잠금·고정·복제·새 탭·다른 패널로·닫기) | `app/input.rs` `open_tab_menu`/`tab_menu_action` | T3 | `tab_menu_and_column_sync` | ✅ | T-43 2차 |
| PANEL(colw) · 07-18 | 열 폭 동기(`list.col_width_sync` · 켜는 순간 즉시 정렬 · 사용자 변경 전파) | `app/input.rs` `sync_col_widths_from` · `panel.rs` | T3 | `tab_menu_and_column_sync` | ✅ | T-43 2차 |
| PANEL-044 · X-17 | 내 PC 전용 열(이름·종류·전체 크기·여유 공간 · 진입/이탈 시점만 교체 · 타일 용량 바) | `filelist.rs`(COL_TOTAL/FREE · drive_space · tile_info) · `panel.rs`(sync_columns_for_root) · `main.rs`(Disk 포트 주입) | T2·T3 | `drive_columns_use_injected_space` · `my_pc_switches_columns_and_back` · `my_pc_drive_columns_from_disk_port` | ✅ | 비Windows 드라이브/마운트 열거는 T-53(ndir-vfs X-17 β) |
| DR-5 · CI-103 · SKEL-404·425·431·432 | 플랫폼 포트 9종(Shell·Pty·ContextMenuProvider·Trash·FileClipboard·DragSource·Watcher·Opener·Disk) · `Platform::native/fake` · Unsupported 규약 | `nexa-dir/src/platform/{mod,fake,windows,macos,linux}.rs` · `docs/adr/0001` | T1·T2·T3 | `pick_shell_prefers_first_existing` · `poll_watcher_reports_changed_dirs_only` · `native_has_a_shell_and_unsupported_ports_say_so` · `fake_records_and_injects` · `platform_ports_wire_open_and_watch` | ✅ | T-50 · Pty/메뉴/휴지통/클립보드/DnD 운영 구현은 T-51~53 |
| PANEL-036 · 042 · SHELL §1-B | 폴더 자동 갱신(폴링 감시 1 s → 그 폴더 탭 무간섭 재열람) | `app/watch.rs` · `platform::PollWatcher` | T3 | `platform_ports_wire_open_and_watch` | 🚧 | OS 통지(ReadDirectoryChangesW·FSEvents·inotify)는 T-51~53 · 감시 상한(WATCH_CAP)·펼친 폴더는 잔여 |
| CI-110 (shell·open·fs) | `--selfcheck` 실제 항목: 기본 셸·후보 · 열기 명령 · 샌드박스 파일 왕복 · 드라이브 용량 | `nexa-dir/src/selfcheck.rs` | T5 | `--selfcheck --ci` PASS(Windows) | ✅ | T-50 · Unix 용량 SKIP(T-52/53) |
| SHELL §1-E · SKEL-427 | 휴지통 3-OS(Windows SHFileOperationW ALLOWUNDO · Linux freedesktop Trash 규격 · macOS ~/.Trash 이동) + `edit.delete` 배선 + selfcheck `trash` | `platform/{windows,linux,macos}.rs` · `app/menus.rs::delete_to_trash` | T1·T3·T5 | `trash_moves_and_writes_info`(Linux) · `home_trash_moves_with_suffix…`(mac) · `delete_goes_through_trash_port` · selfcheck(비CI) | ✅ | T-51 A · 확인창 T-29 · macOS 되돌리기 T-52 · 복원(undelete) 잔여 |
| SHELL §1-C · SKEL-428 | 파일 클립보드 — Windows CF_HDROP + Preferred DropEffect 읽기/쓰기 | `platform/windows.rs::NativeFileClipboard` | T2(ignored · opt-in) | `double_null_list_layout` · `clipboard_roundtrip_and_trash_real`(ignored) | 🚧 | mac/Linux(T-52/53) · 명령 배선(edit.cut/copy/paste)은 M6 |
| PANEL-044(용량) | 드라이브 용량 3-OS(`GetDiskFreeSpaceExW` · glibc/Darwin `statvfs` 수동 extern) | `platform/*.rs::NativeDisk` | T1 | `native_has_a_shell_and_unsupported_ports_say_so`(3-OS 단언) | ✅ | 내 PC 열 표시는 다음 슬라이스 |
