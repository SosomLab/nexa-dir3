# port/52 · nexa-ui 보조 크레이트 현황 — nexa-ctl `TextBox` · nexa-grid `edit` · nexa-dlg `FilePicker` · nexa-fs OS 분기

> 작성 2026-10-03(T-91 · 원장 색인 "미작성" 해소) · 같은 날 사실 검증(dir3 `crates/` grep + nexa-ui `d61a820` 대조). 50(코어 어휘)·51(컨트롤)이 다루지 않은 세 영역을 **dir3가 무엇을 쓰고 무엇을 안 쓰는지** 기준으로 적는다.
> ID 접두 **UIX**(nexa-ui 확장 크레이트). 상태 = dir3 소비 ✅ / 미사용 — / nexa-sql 전용 ⚠ / dir3 빈칸(해야 하는데 안 함) ☐.
> "dir3 소비"는 **dir3 소스에서 직접 부르는 것**만 적는다. nexa-ui 컨트롤 내부에서 간접으로 쓰는 것은 "(간접)"으로 표시한다.

## §1 nexa-ctl `controls::textbox::TextBox` — 편집기급 텍스트 상자

| ID | 영역 | nexa-ui 현황(`textbox.rs` 공개 메서드 161) | dir3 소비 | 상태 |
| --- | --- | --- | --- | --- |
| UIX-001 | 기본 입력(한 줄) | `new`/`text`/`set_text`/캐럿·선택·클립보드·Undo · `with_clearable`(지우기 버튼) · placeholder | 설정 창 검색(`prefs_win.rs` `search` · `with_clearable`) · 일괄 이름 변경(`bulk_win.rs`) · 확인/입력 창(`dlg_win.rs`) · 파일 선택 창(`file_win.rs`) · 라이선스 창(`license_win.rs`) · 클립보드 잇기(`clipboard.rs`) | ✅ |
| UIX-002 | 편집기 기능(여러 줄) | 줄 번호 · 미니맵(`set_minimap*`) · 구문 강조(`set_highlighter` · `sql`/`brackets`) · 괄호 짝(`goto_bracket_*`) · 자동 들여쓰기 · 탭 정지 · 룰러 · 공백 표시 · 찾기 표시(`set_find_marks`) · 링크(`set_link_*`) · diff 표시(`diff_marks`) · 거터 | dir3 없음(dir2에 편집기 없음 — 도크 텍스트 미리보기는 nexa-explorer 도크 텍스트 보기) | ⚠ nexa-sql 전용 |
| UIX-003 | 편집 명령 | `edit_command`(잘라내기/복사/붙여넣기/전체 선택 …) · `EditMenu`(컨텍스트 팝업 · `set_menu_extras`) | dir3 직접 호출 0 · TextBox 내부 우클릭 팝업으로만(간접). 경로 바는 TextBox가 아니라 nexa-grid `EditState`(아래 §2) · 우클릭 편집 명령 = `CMD-086~096` 행(매트릭스) | — (간접) |
| UIX-004 | 클릭 정책 | `click_policy`/`set_click_policy`(단어 밑줄 `word_underscore`) | 기본값 | — |
| UIX-005 | 기록기 | `RecordCtx.texts`로 그려진 문자열 확인 | T3 `paint_records_inside_surface` 등 | ✅ |

## §2 nexa-grid `edit::EditState` — 행 위 인라인 편집기(UIK-216) · 경로 바 편집

> nexa-ctl `gridedit::LiveEditor`(nexa-sql 결과 격자용)와는 **다른 것**이다. dir3는 `LiveEditor`를 쓰지 않는다.

| ID | 영역 | nexa-ui 현황 | dir3 소비 | 상태 |
| --- | --- | --- | --- | --- |
| UIX-010 | 생성·선택 | `new(text, select_all)`/`with_selection_to`(확장자 제외 선택 — 이름부만 선택) · `set_text_end` | 이름 바꾸기(F2 · `app/ops.rs` `begin_rename` → nexa-grid `VirtualRows::begin_rename`(`rows.rs:453`)이 `with_selection_to` 호출) · 경로 바 편집(nexa-explorer `pathbar.rs:148` `EditState::new` · 간접) | ✅ |
| UIX-011 | 편집 | `insert`/`insert_str`/`backspace`/`delete_selected`/`cut_selection`/`selected_text`/`undo`/`can_undo`/`key` | 그리드 안에서 키 라우팅(`app/ops.rs` `rename_edit`) | ✅ |
| UIX-012 | 마우스 | `hit`/`click`/`drag`/`release` | 그리드 | ✅ |
| UIX-013 | 그리기 | `paint_field`(행 사각형 안 · 캐럿 · 선택) | 그리드 `paint_grid` 안(클립 스택 UIC-310 적용) | ✅ |
| UIX-014 | 시험 | nexa-grid `edit.rs` 단위 + dir3 T3 `new_folder_rename_and_undo` · `bulk_rename_window_apply_undo_and_presets` | | ✅ |

## §3 nexa-dlg `FilePicker` — 자체 그리기 파일 선택 창(OS 대화상자 금지 · DR-1)

| ID | 영역 | nexa-ui 현황 | dir3 소비 | 상태 |
| --- | --- | --- | --- | --- |
| UIX-020 | 모드 | `PickerMode::{Open, Save, Folder}` · `FileFilter::new(label, exts)` · `PickerLabels`(i18n 주입) | `file_win.rs`(라이선스 파일 열기 `license_filters` · 플러그인 설치 `plugin_filters` · 폴더 선택) — 호출부 `app/license.rs:252~265`는 `Open`·`Folder`만 | ✅ |
| UIX-021 | 상태 | `set_default_name`/`set_recent`/`set_show_hidden`/`set_show_dot`/`set_extra`(부가 콤보)/`current_dir`/`take_action`/`popup_open` | `set_show_dot(true)`만 호출(`file_win.rs:163`) · `set_recent`/`set_show_hidden`/`set_default_name`/`set_extra`는 미호출 — 최근 목록·숨김 파일 설정(`list.show_hidden`)이 창에 주입되지 않는다 | 🚧 |
| UIX-022 | 다중 선택 | `set_multi`/`marked_files` | `set_multi(false)` 명시(`file_win.rs:162`) — 단일 선택만 | — |
| UIX-023 | 덮어쓰기 확인 | `set_overwrite_confirm_ms`(Save 모드 카운트다운) | Save 모드 미사용(dir3는 파일 저장 창 없음 — 세션/설정은 자동 저장) | — |
| UIX-024 | 틱·아이콘 | `tick(now_ms)`/`icons_pending`/`animating`(nexa-fs 아이콘 지연 로드) | 호스트 `tick`에 연결 | ✅ |
| UIX-025 | 시험 보조 | `set_probe_chevrons`/`probe_chevrons_enabled` · `header_edge_hover` | — | — |

## §4 nexa-fs — 파일 시스템 보조 · OS 분기 지점(DR-5: 앱 `platform/`과 역할 구분)

| ID | 영역 | nexa-ui 현황(`cfg`) | dir3 소비 | 상태 |
| --- | --- | --- | --- | --- |
| UIX-030 | 목록·정렬 | `list`/`list_opts`/`has_visible_child`/`has_subfolder`/`natural_cmp`/`sort_by`/`sort` · `Entry` | dir3는 **ndir-vfs/ndir-tree**(dir2 이식)를 쓴다 — nexa-fs 목록은 FilePicker 내부 전용 | — |
| UIX-031 | 비동기 목록 | `lister::ListHandle::{start, probe, try_recv, is_done, cancel}` | FilePicker 내부 | — |
| UIX-032 | 드라이브·장소 | `drives()`(`cfg(windows)` 비트마스크 · 그 외 `/`) · `places()`/`home_dir()` · `is_virtual_root`/`drive_entries` | FilePicker 사이드바 · dir3 패널은 ndir-vfs 루트(PANEL-011) | — |
| UIX-033 | 아이콘 | `shell::IconService::global().icon(&IconKey, large)` → `Lookup<Arc<RgbaIcon>>` · `kind_name` · `set_os_icons` · `icon_cache_max` — **`cfg(windows)`** SHGetFileInfo 워커(`shell.rs:386`) · **`cfg(not(windows))`** 폴백 글리프/확장자 표(`:737`) | 런처 exe 아이콘(T-30 B `launcher_icons.rs` · `IconKey::Path`) · 새 항목 템플릿 라벨 `kind_name`(`platform/wintemplates.rs` · SHELL-008). **패널 행 아이콘**(10-03 §73 · GAP-003 해소): `filelist.rs` `RowSource::icon` = dir2 아이콘 키 → `app/row_icons.rs` 리졸버(nexa-ui 116차 `set_icon_resolver`)가 `IconKey::Path`/`IconKey::Kind`로 조회(아이콘 계층 NEW-009). 검증 당시(T-91)에는 `icon` 미구현으로 행 아이콘이 없었다 | ✅ |
| UIX-034 | 외부 열기 | `reveal_in_file_manager`(`windows` explorer /select · `macos` open -R · 그 외 xdg-open) · `explorer_path` | dir3는 `platform::Opener`(DR-5)로 — nexa-fs 것은 미사용 | — |
| UIX-035 | 변경 감지 | `watch::{FileSig(file_sig — cfg(unix) inode/ctime · 그 외 mtime+len), content_hash, StatWatch::{spawn, check, poll}, WatchReq, WatchEvent}` | dir3는 `platform::Watcher`(Win ReadDirectoryChangesW · Linux inotify · macOS kqueue · 폴백 PollWatcher) — nexa-fs StatWatch는 nexa-sql 스크립트 파일용 | — |
| UIX-036 | 경로·시간·크기 | `paths::{validate, display, resolve, expand_env, parent_chain}` · `local_time`/`civil_from_unix`(`cfg(windows)` TIME_ZONE_INFORMATION · `cfg(unix)` libc localtime_r) · `fmt_size` | dir3 = ndir-core 이식 함수(dir2 규칙) · 도크 크기 표기는 자체 `dockinfo.rs` `fmt_size_long`(dir2 계승) — nexa-fs 것은 하나도 쓰지 않는다 | — |

## §5 결론

- dir3가 실제로 쓰는 것은 **TextBox(한 줄) · nexa-grid EditState(이름 바꾸기 · 경로 바) · FilePicker(Open/Folder) · IconService(런처 아이콘 · `kind_name` · 패널 행 아이콘 §73)**뿐이다.
- 빈칸(T-91 검증 때 2): ① FilePicker에 최근 목록·숨김 파일 설정 미주입(UIX-021 · GAP-004 🚧) ② 패널 행 셸 아이콘 미구현(UIX-033 · GAP-003 — 10-03 §73 해소 ✅). 나머지는 nexa-sql 전용이거나 dir2 이식 크레이트(ndir-*)와 `platform/`이 대신한다 — DR-5(OS 분기는 앱 `platform/` 한 층)와 충돌하지 않는다: nexa-fs의 `cfg` 분기는 **nexa-ui 내부 구현**이고 dir3는 OS를 모르는 API만 부른다.
- 이 문서는 nexa-ui 쪽 API가 바뀔 때(추가만 · DR-2) 행을 더한다.
