# 18. nexa-dir2 인벤토리 — 터미널(VT 파서·PTY 세션·도크 터미널·퀵 런처)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 2026-10-03.
> 대상: nexa-dir2 `0.22.0` 작업 트리(D:/Projects/kiros33/nexa-dir2).
> 표기: 근거는 `저장소/경로:줄`. 확인하지 못한 것은 **추정**으로 표시. ID 접두사 `TERM-`.
> 이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 / **W**=Windows 전용 유지.

---

## 0. 범위 — 읽은 파일과 줄 수

| 파일 | 줄 수 | 읽은 범위 |
| --- | ---: | --- |
| `nexa-dir2/crates/nexa-term/src/lib.rs` | 1,854 | 전부(1~1854) |
| `nexa-dir2/crates/nexa-term/examples/audit_vt.rs` | 47 | 전부 |
| `nexa-dir2/crates/nexa-term/Cargo.toml` | 15 | 전부(의존 0 확인) |
| `nexa-dir2/crates/nexa-app/src/conpty.rs` | 484 | 전부 |
| `nexa-dir2/crates/nexa-app/src/launcher.rs` | 169 | 전부 |
| `nexa-dir2/crates/nexa-app/src/win.rs` | 9,951 | 터미널·런처 관련 구간 전부(Grep `term\|conpty\|pwsh\|scheme\|launcher` 히트 전 줄 + 주변): 60~139 · 790~813 · 960~1195 · 1855~1934 · 1984~2008 · 2474~2736 · 4781~4939 · 5325~5356 · 5994~6260 · 6480~6641 · 6776~6782 · 7290~7633 · 7755~7768 · 7855~8000 · 8030~8350 · 8481~8591 · 8606~8790 · 8905~8983 · 9290~9399 · 9480~9589 · 9674~9689 · 9895~9950 |
| `nexa-dir2/crates/nexa-app/src/panel.rs` | 2,215 | 도크 표시 판정 관련(Grep 히트: 113·158·373·419~436·482·1464~1499·1745~1760) |
| `nexa-dir2/crates/nexa-gui/src/widgets/dock.rs` | (1,330+) | 1~135 · 430~670 · 812~856 + Grep(`goto\|kind\|strip\|content_rect`) 히트 전부. *작업 지시의 `dock.rs`는 nexa-app이 아니라 nexa-gui 위젯에 있다(nexa-app/src에 dock.rs 없음).* |
| `nexa-dir2/crates/nexa-app/src/dw.rs` | — | 터미널 글꼴/글리프 구간 340~416 · 466~496 · 655~735 |
| `nexa-dir2/crates/nexa-app/src/fontchain.rs` | 271 | 전부 |
| `nexa-dir2/crates/nexa-app/src/clipboard.rs` | 1,014 | 텍스트/서식 구간 820~1013 |
| `nexa-dir2/crates/nexa-app/src/config.rs` | — | Grep(`term\|dock\|launcher`) 히트 전부(필드·기본값·직렬화·파싱·테스트) |
| `nexa-dir2/crates/nexa-app/src/prefs.rs` | — | 터미널 항목 구간 150~203 · 455~481 · 715~770 + Grep 히트 |
| `nexa-dir2/crates/nexa-gui/src/{draw,event,fastscroll}.rs` | — | `term_cell_w`/`term_text` 기본 구현(draw.rs:57~66) · `wheel_lines`(event.rs:6~20) · `FastScroller`(fastscroll.rs:296~372) |
| `nexa-dir2/crates/nexa-app/lang/{ko,en}.lang` | — | Grep(`term\|terminal\|launcher`) 히트 전부 |
| `nexa-dir2/docs/*` | — | Grep(`터미널\|conpty\|pwsh\|VtScreen`): 23-cross-platform-feasibility.md · 29-audit-checklist.md · journal/2026-07-14.md · journal/2026-09-04.md · audit/20261002-ultracode/plan.md · wiki/기능-하단-도크.md · BRANCHES/DEVLOG/MILESTONES/TODO |
| 대조: `nexa-ui/crates/nexa-ctl/src/**` | 37,821 | lib.rs·draw.rs·event.rs 전부 · raster.rs 1~140 + Mono 구간 · controls/{mod,editmenu,toolbar,scroll,tooldock,splitter,tabbar}.rs 공개 API Grep |
| 대조: `nexa-ui/crates/{nexa-font,nexa-gfx,nexa-fs}` | — | 공개 API Grep(mono_font·push_fallback·icon_for_path) |
| 대조: `nexa-sql/crates/nexa-sql/src/{clipboard,clipboard_x11}.rs` · `nsql-settings/src/lib.rs` | — | 머리말·공개 API |

읽지 않은 것(범위 밖): win.rs의 비터미널 구간, prefs.rs 창 구현 본체, dock.rs의 Info/Preview 텍스트 선택·스크롤바 본체(다른 인벤토리 소관).

---

## 1. 기능 목록

### 1-A. VT 파서·셀 그리드 (`nexa-term` — 의존 0 · `#![no unsafe]` · 전 플랫폼 테스트)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-001 | 셀 모델 | `TermCell{ch, fg:u32, bg:u32, bold, reverse, faint}` — 16바이트 유지가 설계 제약(벤치가 `size_of` 출력). 빈 셀 = `' '`. 전각 연속 셀 = `'\0'` | `nexa-term/src/lib.rs:10-34` | 없음 | N | `wide_char_takes_two_cells` |
| TERM-002 | 기호 색 인코딩 | 셀은 **해석된 색이 아니라 기호**: `0xFF_RRGGBB`=트루컬러(테마 무관) · `0x00000000`=기본 전경 · `0x00000001`=기본 배경 · `0x01_0000ii`=ANSI 16색 인덱스. `TermPalette::resolve`가 렌더 시 해석(알파 바이트로 분기, ANSI는 `c & 0xF`) | `lib.rs:36-47` · `:117-129` | 없음 | N | `sgr_colors_16_256_true` |
| TERM-003 | 화면 버퍼 조회 API | `VtScreen::new(cols,rows)` · `cols()/rows()` · `cursor_col()/cursor_row()`(0-기준 가시 좌표) · `scrollback_count()` · `line_count()`=스크롤백+rows · `line_at(abs)`(스크롤백→화면, 참조 반환) · `mouse_mode()` | `lib.rs:594-667` | 없음 | N | 전 테스트 공용 |
| TERM-004 | 파서 상태 기계 | 상태 6종 `Ground/Esc/Csi/Osc/Charset/Str`. `feed(&str)`가 **char 단위**로 구동(입력은 이미 UTF-8 디코드된 문자열) | `lib.rs:553-563` · `:731-742` | 없음 | N | 전 테스트 |
| TERM-005 | C0 제어 문자 | `ESC`→Esc · `CR`→cx=0 · `LF`→line_feed · `BS`→cx-1(포화) · `HT`→다음 8배수 열(최대 cols-1) · `BEL` 무시 · 그 외 `< 0x20` 무시 · 나머지 `put`. **C0는 Ground에서만 실행**(CSI 중 C0 즉시 실행 없음 — 감사 A10 미반영) | `lib.rs:744-758` | 없음 | N | `put_wrap_and_scrollback` |
| TERM-006 | ESC 시퀀스 | `[`→CSI(파라미터·private 초기화) · `]`→OSC · `( ) * +`→Charset(다음 1글자 폐기) · `P X ^ _`→Str(ST까지 폐기) · `M`=RI · `D`=IND · `E`=NEL · `7`=DECSC · `8`=DECRC · `= >` 무시 · `c`=RIS(full_reset) · 그 외 무시 | `lib.rs:760-789` | 없음 | N | `save_restore_cursor_and_resize` · `charset_designator_is_discarded` |
| TERM-007 | CSI 파라미터 누적 | 숫자: 포화 누적 + 상한 **65535** · `;` 구분(빈 파라미터=0) · `?`=private 마커만 인식(`> = <` 미처리) · 0x40~0x7E 최종 바이트에서 디스패치 · 그 외(중간 바이트) 무시. `par(i,def)`: 0·누락 = 기본값 | `lib.rs:791-818` · `:843-849` | 없음 | N | `nasty_sequences_…` |
| TERM-008 | 커서 이동 | `H/f`=CUP · `A/B/C/D`=CUU/CUD/CUF/CUB · `G`=CHA · `d`=VPA · `E`=CNL · `F`=CPL · `s/u`=저장/복원. 전부 화면 범위 클램프. **CUU/CUD는 스크롤 마진을 무시**(화면 0~rows-1 클램프) | `lib.rs:869-918` | 없음 | N | `cup_and_erase` |
| TERM-009 | 지우기 | `J`=ED 0/1/2/**3**(3=스크롤백 비움) · `K`=EL 0/1/2 · `X`=ECH(커서 불이동). 지운 셀은 **현재 SGR fg/bg**로 채움(`blank_filled_row`) | `lib.rs:1002-1045` | 없음 | N | `cup_and_erase` · `ech_erases_without_moving_cursor` |
| TERM-010 | 삽입·삭제 | `@`=ICH · `P`=DCH · `L`=IL · `M`=DL. IL/DL 반복 횟수는 `rows-cy`로 클램프(CPU 소진 방지). **IL/DL은 스크롤 마진이 아니라 화면 끝(rows)까지 회전** | `lib.rs:1047-1079` | 없음 | N | `insert_delete_chars_and_lines` |
| TERM-011 | 스크롤·마진 | `r`=DECSTBM(무효 범위→전체, 커서 홈) · `S`=SU · `T`=SD · LF가 마진 하단이면 영역 스크롤 · `ESC M`이 마진 상단이면 SD. SU/SD 횟수는 영역 높이로 클램프 | `lib.rs:891-903` · `:948-996` | 없음 | N | `decstbm_region_scroll_keeps_outside` · `su_sd_clamped_to_region` |
| TERM-012 | 스크롤백 | 상한 **800줄**(`MAX_SCROLLBACK`). **전체 화면 마진일 때만** 밀려난 줄 보존, 부분 마진은 미보존. 초과분은 앞에서 drain. `ED 3`으로 비움. 스크롤백 줄은 저장 당시 폭 그대로(리사이즈 미반영) | `lib.rs:48` · `:959-979` · `:1021` | 없음 | N | `put_wrap_and_scrollback` · `su_sd_clamped_to_region` |
| TERM-013 | SGR 색·속성 | `0` 리셋 · `1` 굵게 · `2` faint · `22` 굵게+faint 해제 · `7/27` 반전 · `30-37/40-47/90-97/100-107` ANSI 16 · `38/48;5;n` 256색 · `38/48;2;r;g;b` 트루컬러 · `39/49` 기본. **미지원(무시)**: 3 이탤릭·4 밑줄·5 깜빡임·8 숨김·9 취소선·콜론(`38:2::`) 문법 | `lib.rs:1102-1155` | 없음 | N | `sgr_colors_16_256_true` · `sgr_bold_faint_reverse` · `pwsh_table_header_and_psreadline_colors_resolve` |
| TERM-014 | 256색 변환 | 0~15=ANSI 기호(테마 추종) · 16~231=6×6×6 큐브(`0 또는 55+40v`) · 232~255=그레이(`8+10k`) → 트루컬러 | `lib.rs:1373-1394` | 없음 | N | `sgr_colors_16_256_true` |
| TERM-015 | 전각 문자·자동 줄바꿈 | `is_wide`: U+1100–115F · 2E80–A4CF · AC00–D7A3 · F900–FAFF · FE30–FE4F · FF00–FF60 · FFE0–FFE6(BMP만 — 이모지는 1칸). `put`: `cx+w > cols`면 **즉시** 줄바꿈(지연 줄바꿈 플래그 없음), 전각은 다음 셀을 `'\0'` 연속 셀로 | `lib.rs:931-946` · `:1361-1371` | 없음 | N | `wide_char_takes_two_cells` · `put_wrap_and_scrollback` |
| TERM-016 | 미지원 시퀀스 폐기 | OSC: BEL 또는 ESC(→Esc 상태)까지 폐기(창 제목 등 **전부 무시** — OSC 7 cwd·OSC 52 클립보드 미구현) · Charset 지정자 1글자 폐기 · DCS/SOS/PM/APC: ST까지 폐기, CAN/SUB로 중단, BEL은 종결자 아님 | `lib.rs:820-841` | 없음 | N | `charset_designator_is_discarded` · `dcs_pm_apc_sos_payload_is_discarded` |
| TERM-017 | 마우스 추적 모드 추적 | `CSI ? Pm h/l` 중 **1000/1002/1003**(모드)·**1006**(SGR 인코딩)만 추적. 그 외 private 모드(1 DECCKM·25 DECTCEM·1049 대체 화면·2004 괄호 붙여넣기 등)는 **무시** | `lib.rs:584-589` · `:851-868` · `:645-653` | 없음 | N | `decset_mouse_modes_tracked` |
| TERM-018 | 리사이즈 | `resize(cols,rows)`(최소 1×1): 좌상단 기준 복사(리플로 없음·줄어든 행은 버림), 커서 클램프, 마진 리셋. 동일 크기면 무동작 | `lib.rs:707-729` | 없음 | N | `save_restore_cursor_and_resize` |
| TERM-019 | 선택 텍스트 추출 | `get_text(sl,sc,el,ec)`(절대 라인·양끝 포함): 연속 셀 스킵, 각 줄 `trim_end()`(유니코드 공백 전부), 줄 구분 **CRLF** | `lib.rs:669-705` | 없음 | N(줄 구분은 P 검토 — §4) | `put_wrap_and_scrollback` · `wide_char_takes_two_cells` |
| TERM-020 | 선택 → 서식 런 | `get_runs` → 줄별 `TextRun{text, fg, bg, bold}`(기호 색). reverse 셀은 fg/bg 교환. 줄 끝은 `' '`만 제거. 역순 범위는 빈 Vec. faint는 런에 없음 | `lib.rs:1158-1227` | 없음 | N | `get_runs_groups_by_style_and_trims` |
| TERM-021 | HTML 서식 내보내기 | `export::to_html(lines, pal, font, font_px)`: `<pre style="font-family:'<font>',Consolas,monospace;font-size:<px>px;color;background-color;margin:0;white-space:pre">` + 기본과 다른 색·굵기만 `<span style>`. `& < >` 이스케이프, 줄 구분 `\n` | `lib.rs:1238-1281` | 없음 | N | `export_html_rtf_and_cf_html_offsets` |
| TERM-022 | CF_HTML 래핑 | `export::cf_html(fragment)`: `Version:0.9` 헤더 + StartHTML/EndHTML/StartFragment/EndFragment(10자리 0채움, **UTF-8 바이트 오프셋**) + `<!--StartFragment-->…<!--EndFragment-->` | `lib.rs:1283-1298` | Windows 클립보드 규격(문자열 생성 자체는 중립) | N(사용처는 P) | `export_html_rtf_and_cf_html_offsets` |
| TERM-023 | RTF 서식 내보내기 | `export::to_rtf`: `{\rtf1\ansi\deff0{\fonttbl{\f0\fmodern <font>;}}{\colortbl;…}\f0\fs<px*3/2>\cf1\chshdng0\chcbpat2 …}`. 런마다 `{\cfN\chshdng0\chcbpatN\highlightN[\b] text}`, 줄 구분 `\par `, 비ASCII = `\uN?`(부호 있는 16비트·서로게이트 쌍), `\ { }` 이스케이프 | `lib.rs:1300-1358` | 없음 | N | `export_html_rtf_and_cf_html_offsets` |
| TERM-024 | 적대적 입력 견고성 | 거대 파라미터·미완 CSI/OSC·NUL·비BMP·범위 밖 이동·1×1 리사이즈에도 무패닉, 커서·스크롤백 상한 유지 | `lib.rs:1400-1445` | 없음 | N | `nasty_sequences_do_not_panic_and_stay_bounded` |

### 1-B. 터미널 테마(스킴 15종)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-030 | 내장 스킴 15종 | 다크 9 + 라이트 6. `TermScheme{id, name, dark, palette{fg,bg,ansi[16]}}`. **id는 설정 값(안정 계약)**, 배열 순서 = 설정 창 표시 순. 표는 §2-5 | `lib.rs:132-517` | 없음 | N | `schemes_are_well_formed` |
| TERM-031 | 테마 선택 규칙 | `resolve_scheme(selector, dark_default, light_default, app_is_dark)`: `system`=앱 테마 추종 · `dark`/`light`=모드 기본 강제 · 스킴 id=고정. 모르는 기본 id → 내장 기본(`campbell`/`github-light`), 모르는 selector(빈 문자열 포함) → system 동작 | `lib.rs:520-550` | 없음 | N | `resolve_scheme_selector_rules` |
| TERM-032 | 라이트 팔레트 가독성 기준 | GitHub Light 16색 전부 배경 대비 ≥3:1, 기본 전경 ≥7:1(AAA). 흰색 계열(7·15)을 회색으로 매핑(PSReadLine이 97/37을 본문에 사용). 전 스킴: id 유일·dark 분류=배경 휘도(<128)·불투명·기본 전경 ≥3:1 | `lib.rs:85-114` | 없음 | N | `light_palette_is_legible_on_its_background` · `schemes_are_well_formed` |
| TERM-033 | 테마 전환 즉시 재도장 | 셀이 기호 색이므로 앱 테마 전환(F6)·설정 변경 시 **스크롤백까지** 새 팔레트로 그려짐(재색칠 루프 없음 — 무효화만) | `win.rs:4851-4858` · `:6621-6630` | 없음 | N | (렌더 — 자동 테스트 없음) |

### 1-C. PTY 세션(ConPTY)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-040 | 기본 셸 선택 | `pwsh.exe` → `powershell.exe` 순으로 **PATH에서 전체 경로** 탐색(빈/상대 PATH 항목 제외 = 바이너리 플랜팅 방지) → 없으면 `%SystemRoot%\System32\cmd.exe`. 사용자 지정 셸 설정은 **없음** | `conpty.rs:281-301` | `PATH`·`SystemRoot` 환경 변수 | **P** | `default_shell_is_absolute_and_exists` |
| TERM-041 | 세션 시작 | `ConPty::start(hwnd, msg, panel, gen, cwd, cols, rows)`: 파이프 2쌍 → `CreatePseudoConsole(COORD{max(cols,2), max(rows,2)})` → ConPTY 소유 끝 닫기 → `STARTUPINFOEXW`+`PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE(0x20016)` → `CreateProcessW(app=전체 경로, cmdline="\"경로\"", cwd, EXTENDED_STARTUPINFO_PRESENT)`. 인자·환경 변수 추가 없음(부모 환경 상속). 실패 시 `None` | `conpty.rs:51-231` | CreatePipe·CreatePseudoConsole·InitializeProcThreadAttributeList·UpdateProcThreadAttribute·CreateProcessW | **P** | 없음(실기) |
| TERM-042 | 출력 읽기 + UTF-8 경계 보존 | 읽기 스레드: `ReadFile` 4096B 루프 → `Utf8Chunker::push`(잘린 멀티바이트 꼬리 ≤3B 보존, 확정 불량 바이트는 U+FFFD 치환) → `Arc<Mutex<String>>`에 누적 → 통지. EOF/오류 시 종료·핸들 닫기 | `conpty.rs:155-184` · `:303-366` | ReadFile·CloseHandle | P(스레드 I/O) · **N**(`Utf8Chunker`) | chunker 7종(`conpty.rs:379-483`) |
| TERM-043 | 출력 통지·세대 가드 | `PostMessageW(hwnd, WM_APP_TERM(0x8003), wparam=panel, lparam=gen)`. UI는 `t.pty.gen == gen`일 때만 버퍼를 `take` → `screen.feed` → 도크 rect 무효화. 낡은 세션 통지는 무시 | `conpty.rs:172-178` · `win.rs:8952-8983` | PostMessageW | **P**(→ winit `EventLoopProxy`) | 없음 |
| TERM-044 | 셸 종료 감지 | 대기 스레드: **복제 핸들**로 `WaitForSingleObject(INFINITE)` → `PostMessage(wparam=panel\|EXIT_FLAG(0x100))`. UI는 `t.exited = true` | `conpty.rs:185-219` · `win.rs:8954-8960` | DuplicateHandle·WaitForSingleObject | **P** | 없음 |
| TERM-045 | 입력 쓰기 | `write(&str)`: UTF-8 바이트를 **동기 `WriteFile`**(UI 스레드). 빈 문자열 무시, 실패 무시 | `conpty.rs:233-247` | WriteFile | **P** | 없음 |
| TERM-046 | 크기 동기 | `resize(cols,rows)`(≤0 무시) → `ResizePseudoConsole`. 페인트에서 그리드 크기가 달라지면 `screen.resize` + `pty.resize` | `conpty.rs:249-257` · `win.rs:2592-2595` | ResizePseudoConsole | **P** | 없음 |
| TERM-047 | 세션 정리 | `Drop`: `TerminateProcess(셸)` → 입력 파이프 닫기 → `ClosePseudoConsole` → 속성 목록 해제 → 스레드/프로세스 핸들 닫기. 셸의 **자식 프로세스 트리는 직접 종료하지 않음**(ConPTY 종료에 위임 — 추정) | `conpty.rs:260-279` | TerminateProcess·ClosePseudoConsole | **P** | 없음 |
| TERM-048 | 지연 시작 | 패널당 1세션(최대 2). **페인트 시점**에 `도크 표시 && 도크 h>0 && 종류=터미널(2)`이고 세션이 없으면 시작. 그리드가 2×2 미만이면 시작 보류. `term_gen` 증가로 세대 부여 | `win.rs:1049-1051` · `:4842-4874` · `:2561-2577` | — | N(로직) | 없음 |
| TERM-049 | 기동 실패 처리 | 내용 영역 첫 줄에 `term.fail` 문구(`text_dim` on `panel_bg`) 표시. `term_paint`가 `false` 반환 → 터미널 키 포커스 해제(키 영구 삼킴 방지). **매 페인트마다 재시도**(실패 메모 없음 — 감사 A70 미반영) | `win.rs:2578-2589` · `:4875-4879` | — | N | 없음 |
| TERM-050 | 종료 안내·재시작 | 셸 종료 시 그리드 최하단 줄에 `term.exited`(accent on panel_bg). 터미널 포커스에서 **아무 키(KEYDOWN) 또는 문자(CHAR)** → `terms[i]=None` → 다음 페인트에서 현재 폴더로 재시작. `[→]` 클릭도 종료 상태면 재시작 | `win.rs:2706-2716` · `:8747-8751` · `:9348-9351` · `:8086-8087` | — | N | 없음 |
| TERM-051 | 세션 수명 | 도크 숨김·종류 전환(정보/미리보기)·패널 폴더 이동에도 세션 **유지**(출력은 계속 feed). 앱 종료(State drop) 때만 정리. 패널 폴더 이동은 셸 cwd를 **바꾸지 않는다** | `win.rs:8957-8970`(가시 여부 무관 feed) | — | N | 없음 |
| TERM-052 | 싱글 정보 모드 = 좌 세션 고정 | 전폭 공유 도크(좌 위젯)의 터미널은 **패널 0 세션**. 지연 시작 cwd = `panels[0].root_path()`, 단 `[→]`의 "현재 폴더"는 **활성 패널**. 우 도크는 0-rect라 숨은 PTY를 기동하지 않음 | `win.rs:4844-4848` · `:8082-8084` · `:2483-2485` | — | N | `dock_shown_requires_visible_and_height`(panel.rs:1749) |

### 1-D. 렌더

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-060 | 셀 크기·그리드 계산 | `cell_w` = 모노 글꼴 `"0"` 폭(px, ceil) · `cell_h` = `max(12, font_px*4/3*dpi/96)` · `vis_cols=(rc.w-4)/cell_w` · `rows=(rc.h-2)/cell_h` · 그리드 원점 = `(rc.x+2, rc.y+1)` | `win.rs:2551-2563` · `dw.rs:664-682` | DirectWrite 측정 | **A** | 없음 |
| TERM-061 | 줄 바꿈/고정 열 | `term_wrap=true`: cols=뷰 폭 열(창 크기 따라 PTY 리사이즈) · `false`: cols=`term_cols`(80~1000) 고정 + 가로 스크롤(`view_x`·`view_fx`) | `win.rs:2553-2559` · `:2597` | — | N | config 왕복 |
| TERM-062 | 셀 그리드 렌더 | 행마다: 동일 유효 (fg,bg) 런 = **배경 채움 1회**, 문자는 **셀 x에 1글자씩 개별 배치**(런 레이아웃 금지 — 폴백 글꼴 전진폭이 그리드와 어긋남). reverse=fg/bg 교환 · faint=전경을 배경 쪽 50% 블렌드 · 공백·연속 셀은 배경만 · 전각은 2셀 클립. **bold는 렌더에 반영하지 않음**(복사 서식에만 반영) | `win.rs:2632-2704` | DWrite `IDWriteTextLayout::Draw` | **A** | 없음 |
| TERM-063 | 선택 하이라이트 | 선택 셀 = fg/bg 반전(사용자 확정 — 재론 금지). 예외: **밝은 팔레트**(배경 휘도 ≥128, `(299R+587G+114B)/1000`)에서 기본색 셀만 `(글자=pal.bg, 배경=theme.accent)` | `win.rs:2608-2625` · `:2647-2653` | — | A | 없음 |
| TERM-064 | 캐럿 | 세로바: 폭 `max(1, dpi/96)`px · 높이 `cell_h-2` · 색 `pal.fg`. 스크롤백 보기·가로 스크롤로 화면 밖이면 생략. 키 포커스일 때만 깜빡임(주기 `GetCaretBlinkTime`, 0/INFINITE → 530ms), 비포커스는 **상시 표시**, 입력 시 위상 리셋(켜짐). DECTCEM(?25) 미추적 → 항상 표시 | `win.rs:2717-2731` · `:4849-4850` · `:6189-6197` · `:9550-9563` | GetCaretBlinkTime·SetTimer | A + **P**(깜빡임 주기) | 없음 |
| TERM-065 | 터미널 글꼴 | `term_font` = **쉼표 폴백 체인**(예 `D2Coding, JetBrainsMono Nerd Font`): 설치된 첫 패밀리가 1순위(없으면 `Consolas`), 나머지 설치 패밀리 = 명시 폴백 → 시스템 폴백. 크기 8~32 DIP(기본 12). 단일 글리프 레이아웃 캐시(상한 2048 초과 시 전체 비움, DPI 변경 시 비움). 글꼴/크기 변경 = 백엔드 재생성 | `dw.rs:352-384` · `:684-735` · `fontchain.rs:28-57` · `win.rs:6496-6500` | DirectWrite(TextFormat·FontFallback) | **A** | `families_trims_and_drops_empty` |
| TERM-066 | 안내 문구 | 종료: `term.exited`를 `y = rc.bottom()-cell_h-1`, `x = rc.x+2`에 accent 색. 실패: `term.fail`을 `(rc.x+2, rc.y)`에 text_dim 색. 둘 다 모노 글꼴·`panel_bg` 배경 | `win.rs:2706-2716` · `:2579-2586` | — | A | 없음 |
| TERM-067 | 부분 행/열 픽셀 스크롤 | `view_frac`(0..cell_h)·`view_fx`(0..cell_w): 뷰를 frac px 밀어 `top-1` 줄 아래쪽이 위에 보임(행 루프가 -1부터). 위로 번진 부분은 **종류 스트립 재도장**으로 덮음 | `win.rs:2626-2638` · `:4880-4883` | — | A(클립으로 단순화 가능) | 없음 |
| TERM-068 | 고속 스크롤 ×N 배지 | 스크롤백 휠 노치 연타 시 배수 적용 + 그리드 위 배지(`t.fast.paint(ctx, theme, rc, cell_h, 6)`), 위젯 틱 타이머(40ms)로 페이드 | `win.rs:2733-2734` · `:7885` · `:7894-7901` · `:9502-9514` | SetTimer | A(nexa-ui `ScrollAccel`+`SpeedHud`) | fastscroll 자체 테스트(범위 밖) |

### 1-E. 키 입력

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-070 | 키 포커스 획득·해제 | 획득: 터미널 종류 도크 영역 좌클릭(스트립 포함 `y >= dock.y`) · `[→]` 클릭 · 터미널 우클릭(메뉴 대상 확정). 해제: 패널 내 다른 영역 클릭(매 클릭 선두에서 `term_focus=None` 후 재판정). 획득 시 경로바 편집·인라인 리네임 취소(`cancel_text_edits`), 캐럿 타이머 무장 | `win.rs:8044` · `:8094-8114` · `:8257-8266` | SetTimer | N(로직) | 없음 |
| TERM-071 | 키 라우팅 판정 | `route_key_with_term(term_focus, dock_shown, kind, term_started)` → `Term`(PTY로) / `TermPending`(PTY 미기동 — **키 삼킴**, 목록 누수 금지) / `ClearFocus`(도크 숨김·종류 전환 — 포커스 해제 후 목록 경로) / `List`. 경로바 편집 중이면 경로바가 우선 | `win.rs:6079-6129` · `:8743-8771` | — | **N**(순수 함수) | `key_route_mcdc_pairs`(win.rs:9916) |
| TERM-072 | 비문자 키 → VT | ↑`ESC[A` ↓`ESC[B` →`ESC[C` ←`ESC[D` Home`ESC[H` End`ESC[F` Delete`ESC[3~` PgUp`ESC[5~` PgDn`ESC[6~`. **미매핑(삼킴)**: F1~F12·Insert·수식키 조합(Ctrl/Shift+방향)·Shift+Tab. 응용 커서 모드(DECCKM `ESC O A`) 없음 | `win.rs:6131-6145` · `:8752-8756` | VK_* 가상 키 | **P**(키 이벤트 원천) + N(표) | 없음 |
| TERM-073 | 문자 입력 | `WM_CHAR`의 모든 문자(제어 문자 포함: Enter=`\r`, Tab, Esc, Ctrl+문자=0x01~0x1A)를 UTF-8로 PTY에. **Backspace(0x08 도착)→`0x7F`**, **Ctrl+Backspace(0x7F 도착)→`0x08`** 교차 매핑(ConPTY/PSReadLine이 0x08을 단어 삭제로 해석) | `win.rs:9331-9381` | WM_CHAR(OS가 Ctrl 조합·IME 결과를 문자로 변환) | **P** | 없음 |
| TERM-074 | Ctrl+C / Ctrl+V | `0x03`: **선택이 있으면 복사**(→TERM-090), 없으면 0x03 그대로(인터럽트). `0x16`: 붙여넣기(→TERM-091). Windows Terminal 규약 | `win.rs:9353-9361` | — | N(정책) + P(단축키 관례 — §4-5) | 없음 |
| TERM-075 | 입력 부수 효과 | 문자 입력 시 `view_off=0`(스크롤백 보기 해제)·선택 해제·캐럿 위상 켜짐. 비문자 키(화살표 등)는 `view_off`/선택을 건드리지 않음 | `win.rs:9369-9377` · `:8752-8756` | — | N | 없음 |
| TERM-076 | IME(한글 조합) | 터미널은 **IME 조합 창 배치 대상이 아님**(경로바·리네임만) — 조합 창은 OS 기본 위치, 확정 문자열이 `WM_IME_CHAR→WM_CHAR`로 도착해 PTY로 전송. 조합 중 문자의 인라인 표시 없음 | `win.rs:4781-4796` · `:9323-9330` | Imm32 | **P** | 없음 |
| TERM-077 | 터미널 포커스 중 앱 단축키 | `WM_KEYDOWN` 경로가 터미널에서 **전부 반환**되므로 Ctrl+`` ` ``(도크 토글)·F6·Ctrl+Tab 등 앱 단축키가 동작하지 않음(메뉴 클릭은 가능). 예외: `WM_SYSKEYDOWN`은 터미널 분기가 없어 **Alt+←/→/↑/↓(패널 탐색)·Shift+F10이 터미널 포커스에서도 활성 패널에 작용** | `win.rs:8740-8762` · `:8905-8939` | — | N(동작 계승 여부는 결정 필요 — §6) | 없음 |

### 1-F. 마우스

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-080 | 드래그 선택 | 그리드 안 좌클릭 = 앵커(=끝) 설정·`term_drag` 시작, 이동 = 끝점 확장, 뗌 = 확정(선택 **유지**). 좌표→셀 `cell_at`: `col = view_x + (x-rc.x-2+view_fx)/cw`(≤cols 클램프), `row = (y-rc.y-1-view_frac).div_euclid(ch)`(≤rows-1), 절대 라인 = `top+row`. 앵커=끝이면 선택 없음(`sel_norm`=None). 선택 범위는 **줄 흐름(스트림)** 방식·양끝 포함 | `win.rs:8119-8129` · `:1140-1164` · `:8315-8337` · `:8504-8507` | SetCapture | A(컨트롤 내부 상태로) | 없음 |
| TERM-081 | 엣지 자동 스크롤 | 드래그 중 포인터가 그리드 위/아래로 벗어나면 1줄 스크롤 후 클램프 좌표로 확장. 밖에 머무는 동안 **60ms** 타이머(`TIMER_TERM_SEL`)로 반복(커서 위치 폴링) | `win.rs:6223-6239` · `:8326-8331` · `:9564-9582` | SetTimer·GetCursorPos | A + P(타이머) | 없음 |
| TERM-082 | 휠 = 스크롤백 보기 | 터미널 위(hover 기준 — 활성 패널 아님) 휠: 노치(≥120) = `wheel_lines()`줄(시스템 값, 기본 3) × 고속 배수, 노치 미만(트랙패드) = 픽셀 스크롤. `view_off` 0..scrollback 클램프. 행 단위 이동은 `view_frac=0` 스냅 | `win.rs:7872-7904` · `:1166-1194` | SPI_GETWHEELSCROLLLINES(기동 시 주입) | A | 없음 |
| TERM-083 | 보던 위치 고정 | 스크롤백 보기 중(`view_off>0`) 새 출력으로 스크롤백이 늘면 그만큼 `view_off` 증가(화면이 따라 내려가지 않음). 문자 입력·붙여넣기·`[→]` 시 하단 스냅 | `win.rs:8962-8969` · `:9369` · `:7441` · `:8090` | — | N | 없음 |
| TERM-084 | 가로 스크롤(고정 열 모드) | `term_wrap=false`일 때만: Shift+휠 또는 가로(틸트) 휠 = 4열/노치(px 환산·트랙패드 비례), `0..(cols-vis_cols)*cw` 클램프. wrap 모드에서 Shift+휠은 터미널에서 소비되지 않고 아래 경로로 내려감(감사 A33 미반영) | `win.rs:7863-7871` · `:7936-7944` · `:1123-1138` | — | A | 없음 |
| TERM-085 | TUI 마우스 전달 | DECSET 마우스 모드 ON **이고 SGR(1006)** 일 때만 `ESC[<b;col;row M/m` 전송(레거시 인코딩 미지원). 좌 press/release(b=0) · 우 press/release(b=2 — 터미널 포커스 필요, 컨텍스트 메뉴 억제) · 드래그 모션(b\|32, 모드 ≥1002, 버튼 유지 중만) · 휠(64 위/65 아래, 1회/노치). **Shift = 로컬 우회**(선택·스크롤백). 좌표는 `(x-rc.x-2)/cw+1`·`(y-rc.y-1)/ch+1` | `win.rs:6199-6221` · `:8117-8123` · `:8184-8199` · `:8228-8246` · `:8301-8313` · `:7874-7881` · `:8494-8503` | SetCapture/ReleaseCapture | A + N | `decset_mouse_modes_tracked`(파서 측) |
| TERM-086 | 캡처 상실 정리 | 캡처를 잃으면(Alt+Tab·팝업) 선택 드래그·TUI 버튼 상태·타이머 일괄 정리. TUI 버튼 유지 판정은 버튼별(`tui_btn_held`: 좌=MK_LBUTTON, 우=MK_RBUTTON). 좌·우 동시 눌림 시 LBUTTONUP이 캡처를 풀지 않음 | `win.rs:5994-6029` · `:8583-8589` | WM_CAPTURECHANGED | **P**(winit: CursorLeft/Focused(false)/버튼 상태) | 없음 |
| TERM-087 | 히트 판정 | `term_hit`: 도크 실제 표시(h>0) && 종류=터미널 && 세션 존재 && 캐시된 그리드 rect 포함. 텍스트 도크 휠 판정(`dock_text_at`)은 터미널 종류를 제외 | `win.rs:6241-6248` · `:6166-6176` | — | N | 없음 |

### 1-G. 클립보드·컨텍스트 메뉴

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-090 | 선택 복사(서식 포함) | 평문(필수) + 설정 `term_copy_format`에 따라 HTML/RTF 동시 게시. 색 = **지금 화면 팔레트**로 해석, 글꼴 = `term_font` 체인 1순위 **원문**(설치 여부 무관, 비면 `Consolas`), 크기 = `term_font_size`. 선택 없으면 `false`. **복사 후 선택 해제**(WT 규약) | `win.rs:7394-7429` | (→TERM-095) | N + P | `export_html_rtf_and_cf_html_offsets` |
| TERM-091 | 붙여넣기 | 클립보드 텍스트의 `CRLF`·`LF` → `CR` 변환 후 PTY에 일괄 쓰기. `view_off=0`. 괄호 붙여넣기(2004) 래핑 없음, 크기 제한 없음(UI 스레드 동기 쓰기) | `win.rs:7431-7443` | 클립보드 읽기(CF_UNICODETEXT) | N + P | 없음 |
| TERM-092 | 전체 선택 | 스크롤백 첫 줄 (0,0) ~ 마지막 줄 (n-1, cols-1) | `win.rs:7445-7456` | — | N | 없음 |
| TERM-093 | 우클릭 편집 메뉴 | 살아 있는 터미널 내용 영역 우클릭(TUI 마우스 모드가 가로채지 않은 경우) → 포커스 이동 + 팝업 `복사`(선택 있을 때) · `붙여넣기`(클립보드에 텍스트 있을 때) · ─ · `전체 선택`(항상). 종료된 터미널은 대상 아님 | `win.rs:7458-7490` · `:7541-7549` · `:7605-7614` · `:8247-8270` | CreatePopupMenu·TrackPopupMenuEx(**OS 네이티브 메뉴**) | **A**(nexa-ui `ContextMenu`) | 없음 |
| TERM-094 | 편집 명령 디스패치 순서 | `do_clip(act)`: ①경로바 편집 → ②인라인 리네임 → **③터미널 포커스**(Copy/Cut=복사, Paste, SelectAll; Undo/Delete=무동작) → ④도크 텍스트 선택 → ⑤파일 목록. Edit 메뉴·컨텍스트 메뉴 공용 | `win.rs:7317-7330` · `:7387-7392` | — | N | 없음 |
| TERM-095 | 다중 형식 클립보드 게시 | `write_text_html_rtf(hwnd, text, cf_html, rtf)`: `CF_UNICODETEXT` + 등록 형식 `"HTML Format"` + `"Rich Text Format"`(각각 NUL 종료 바이트). 서식 게시 실패는 무신호. `has_text()`·`read_text()` | `clipboard.rs:898-958` · `:984-1006` | OpenClipboard·SetClipboardData·RegisterClipboardFormatW | **P** | 없음 |

### 1-H. 도크 통합·cwd 동기

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-100 | 도크 종류 스트립 | `[정보][미리보기][터미널][→]` — 클릭으로 종류 전환(인덱스 0/1/2). 터미널(2)은 위젯이 내용을 그리지 않고 호스트가 `content_rect()`에 직접 렌더. **활성 종류는 영속하지 않음**(기동 시 항상 0=정보) | `nexa-gui/src/widgets/dock.rs:464-487` · `:551-598` · `:632-660` · `win.rs:2486-2498` | — | **A** | dock.rs 테스트(범위 밖) |
| TERM-101 | `[→]` 현재 폴더로 이동 | 터미널 라벨 옆 한 몸 버튼. 클릭 = 종류를 터미널로 전환 + 키 포커스 + ①세션 살아 있음: `cd "<경로>"\r` 전송·하단 스냅 ②종료 상태: 세션 폐기 → 현재 폴더로 재시작 ③미기동: 지연 시작이 현재 폴더로 엶. "현재 폴더" = 그 패널(공유 도크면 활성 패널)의 `root_path()` | `dock.rs:577-594` · `:638-647` · `win.rs:8080-8099` | 셸 문법(`cd "…"`) | **P**(셸별 명령 문자열) | 없음 |
| TERM-102 | 시작 작업 폴더 | 세션 시작 cwd = 패널 `root_path()`. 가상 루트(내 PC)·클라우드 센티널 경로에 대한 가드 **없음** → 그런 경로면 `CreateProcessW` 실패 → `term.fail`(추정 — 코드상 가드 부재만 확인) | `win.rs:4848` · `conpty.rs:129-143` | — | P | 없음 |
| TERM-103 | 포커스 시각 동기 | 터미널이 키 포커스면: 양 패널 목록·탭 바는 비활성 색, 그 도크의 활성 종류 라벨 = `sel_bg`·`[→]` = `accent`. 비포커스 활성 종류 = `sel_bg_inactive` | `win.rs:6042-6051` · `dock.rs:563-588` | — | A | 없음 |

### 1-I. 설정

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-110 | 글꼴·크기 | `term_font`(기본 `Consolas`, 비어 있으면 무시, ≤128자) · `term_font_size`(기본 12, 8~32). 미리보기 창(F3)도 이 글꼴을 사용(`win.rs:2468`) | `config.rs:82-84` · `:262-263` · `:576-582` | 기본 글꼴 이름 | **P**(OS별 기본 모노 글꼴) | config 왕복·퍼징(`config.rs:1231-1260` · `:1396-1399`) |
| TERM-111 | 줄 바꿈·열 수 | `term_wrap`(기본 true) · `term_cols`(기본 240, 80~1000) | `config.rs:87-89` · `:264-265` · `:638-642` | — | N | `config.rs:1400-1401` · `:1483` |
| TERM-112 | 테마 3키 | `term_theme`(기본 `system`) · `term_theme_dark`(기본 `campbell`) · `term_theme_light`(기본 `github-light`). 각 ≤64자·비어 있으면 무시. **모르는 id도 저장은 그대로**(해석 시 폴백) | `config.rs:90-96` · `:266-268` · `:645-653` | — | N | `config.rs:1402-1412` |
| TERM-113 | 복사 형식 | `term_copy_format` ∈ `text`(기본)/`html`/`rtf`/`both`. 모르는 값은 무시(기본 유지) | `config.rs:99` · `:269` · `:654-656` | — | N | `config.rs:1405-1408` |
| TERM-114 | 설정 창 항목 | 글꼴 카테고리: `pref.termFont`(Font = 이름+크기 1줄). 터미널 카테고리(순서 고정): `termWrap`(CheckBox) · `termCols`(Number) · `termTheme`(Select: system/dark/light + 스킴 15, 라벨 `"이름 (다크\|라이트)"`) · `termThemeDark`(Select: 다크 9) · `termThemeLight`(Select: 라이트 6) · `termCopy`(Select 4택) | `prefs.rs:154-203` · `:468-474` · `:721-763` · `:432-437` | HWND 컨트롤 | **A** | 없음 |
| TERM-115 | 설정 즉시 적용 | 글꼴/크기 변경 → 그리기 백엔드 재생성(`st.dw=None`) · wrap/cols → 전체 무효화(다음 페인트에서 cols 재계산·PTY 리사이즈) · 테마 3키 → 무효화만 · 복사 형식 → 값만 교체. 설정 창 정규화: 빈 글꼴→`Consolas`, 크기·열 클램프 | `win.rs:6495-6500` · `:6616-6631` · `prefs.rs:983-990` | — | N | 없음 |

### 1-J. 퀵 런처(외부 프로그램·셸 실행)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| TERM-120 | 런처 바 표시 | 도구 모음 아래 한 줄(높이 24 @96dpi). 보기 메뉴 `퀵 런처 바`(`CMD_TOGGLE_LAUNCHER=9`, 체크 표시) 토글·영속. 숨김이거나 실행 항목 0(구분선만)이면 높이 0 | `win.rs:217-218` · `:415` · `:1858-1872` · `:5331-5339` | — | **A**(nexa-ui `Toolbar`/`MenuBar`) | config 왕복 |
| TERM-121 | 항목 모델·영속 | `LauncherItem{label, exe, args}` · 구분선 = `launcherN=-`. 파일: `launcher=0\|1` · `launcher_seed=N` · `launcher_count=N` · `launcherN=라벨\|exe\|인자`. `launcher_count` 키 존재 = 목록 확정(비움 존중). UI CRUD **없음**(설정 파일 직접 편집) | `config.rs:10-32` · `:191-198` · `:549-560` · `:758-790` | — | N | `config.rs:1541-1546` |
| TERM-122 | 시드·마이그레이션 | 첫 실행(키 부재): `[VS Code("%path%" 인자)] │ [pwsh 또는 PowerShell] [cmd]`(발견분만). `launcher_seed < SEED_VERSION(2)`면 pwsh/powershell·cmd 누락분만 1회 추가(사용자 항목 보존). VS Code 후보 3경로, pwsh는 PATH → 없으면 `System32\WindowsPowerShell\v1.0\powershell.exe`, cmd는 `%ComSpec%` | `launcher.rs:19-143` · `win.rs:1547-1556` | Windows 경로·환경 변수 | **P** | 없음 |
| TERM-123 | 항목 실행 | 버튼 클릭(`CMD_LAUNCHER_BASE=200`+인덱스) → `args`의 `%path%`를 **활성 패널 현재 폴더**로 치환 → `ShellExecuteW("open", exe, args, workdir=폴더, SW_SHOWNORMAL)`. 성공 = 반환값 >32. 실행 전 포그라운드 양도(`allow_foreground_handoff`) | `launcher.rs:145-169` · `win.rs:5340-5356` | ShellExecuteW | **P** | 없음 |
| TERM-124 | 버튼 아이콘 | exe의 **셸 아이콘 스몰 16×16**(비동기 로딩, 로딩 틱마다 런처 바 무효화). 미로드/실패 시 라벨 앞 2자 폴백 | `win.rs:794-813` · `:9543` | SHGetFileInfoW(icons.rs) | **P**(nexa-fs `icon_for_path`는 Windows만 구현 — `nexa-ui/crates/nexa-fs/src/shell.rs:44`·`:754`) | 없음 |
| TERM-125 | 실행 결과 안내 | 타이틀바 꼬리에 ` · {라벨} 실행` / ` · {라벨} 실행 실패`(`launcher.ran`/`launcher.failed`) | `win.rs:5350-5355` | SetWindowTextW | A | 없음 |

### 1-K. 품질 자산

| ID | 기능 | 동작 상세 | 진입점 | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- |
| TERM-130 | VT 처리량·견고성 벤치 | `cargo run --release -p nexa-term --example audit_vt`: SGR 섞인 32MB feed → MB/s·셀 크기·스크롤백 메모리 출력 + 비정상 시퀀스 1만 회. 기준 ≥10MB/s(docs/29 P-4, 예제 주석은 ≥50MB/s) | `nexa-term/examples/audit_vt.rs:1-46` · `docs/29-audit-checklist.md:47` | N | (벤치) |
| TERM-131 | i18n 문자열 | `dock.terminal` · `term.exited` · `term.fail`(문구에 "ConPTY" 포함 — OS 중립 문구로 교체 필요) · `menu.view.launcher` · `launcher.ran/failed` · `pref.cat.terminal` · `pref.termFont(.desc)` · `pref.termFontSize(.desc)` · `pref.termWrap(.desc)` · `pref.termCols(.desc)` · `pref.termTheme(.desc/.system/.dark/.light)` · `pref.termThemeDark(.desc)` · `pref.termThemeLight(.desc)` · `pref.termCopy(.desc/.text/.html/.rtf/.both)` · `pref.consoleFont.desc` | `lang/ko.lang:31,179-181,231-232,239,248-249,302-303,334-352` · `lang/en.lang:32,180-182,…,426` | N(자원 그대로) | i18n 키 테스트(범위 밖) |

**항목 수: 90**

---

## 2. 화면·컨트롤 배치

### 2-1. 도크 밴드(터미널이 들어가는 자리)

- 창 세로 구성(위→아래, @96dpi 논리값 · 전부 `s(v)=v*dpi/96`): 메뉴 22 · 도구 모음 28 · **퀵 런처 24**(표시 시) · 패널 영역 · **가로 분리선 3**(`SPLIT_TH`, 최소 2px) · **도크 밴드** · 상태바 22 — `win.rs:1858-1875` · `:53`.
- 도크 밴드 높이 = `area_h * dock_ratio`를 `[min(row_h*3, area_h/2), area_h/2]`로 클램프. `dock_ratio` 기본 0.3(0.15~0.5 영속) — `win.rs:1885-1890` · `config.rs:260`.
- 도크 좌/우: `dsx = rc.w * dock_split`(기본 0.5, 영속 0.15~0.85, 배치 시 `rc.w/8..rc.w*7/8` 클램프), 좌 도크 `[0, dsx-g2)`, 우 도크 `[dsx-g2+gap, rc.w)` — `win.rs:1903-1929`. 싱글 정보 모드 = 좌 도크 전폭·우 도크 0-rect.
- 가로 분리선 색: 평소 `text_dim`, 높이 드래그 중 `accent`. 도크 좌/우 스플리터: `border`/드래그 중 `accent` — `win.rs:4902-4924`.

### 2-2. 도크 내부(패널별)

```
y = dock.y            : 상단 경계선 1px (theme.border)
y = dock.y+1 .. +row_h: 종류 스트립 (배경 theme.header_bg)
                        [pad_x][ 정보 ][pad_x][ 미리보기 ][pad_x][ 터미널 ][→][pad_x]
y = dock.y+1+row_h .. : 내용 영역(content_rect) — 터미널 종류면 호스트가 셀 그리드 렌더
```

- `row_h = max(14, 20*dpi/96)` · `pad_x = 6*dpi/96` — `win.rs:1345-1349`.
- 스트립 셀 폭 = `text_width(label) + pad_x*2`, 셀 사이 간격 `pad_x`, 첫 셀 x = `strip.x + pad_x`. `[→]` = 마지막 종류 셀에 **간격 없이** 부착, 폭 `text_width("→") + pad_x*2`. 텍스트 y = `cell.y + (cell.h - cell.h*4/5)/2` — `dock.rs:553-598`.
- 스트립 색: 활성+포커스 `(text, sel_bg)` · 활성+비포커스 `(text, sel_bg_inactive)` · 비활성 `(text_dim, header_bg)`. `[→]`: 활성+포커스 `(text, accent)` · 활성+비포커스 `(text, sel_bg_inactive)` · 비활성 `(text_dim, header_bg)`.
- 글꼴: 스트립 = Base 슬롯(`dock.rs:607` · `:819`), 그리드 = 터미널 모노 글꼴.

### 2-3. 터미널 그리드(내용 영역 `rc` 기준)

| 요소 | 위치·크기 |
| --- | --- |
| 그리드 원점 | `(rc.x + 2, rc.y + 1)` |
| 가시 열 수 | `(rc.w - 4) / cell_w` |
| 행 수 | `(rc.h - 2) / cell_h` |
| 셀 폭 | 모노 글꼴 `"0"` 폭(ceil px) |
| 셀 높이 | `max(12, font_px * 4 / 3 * dpi / 96)`(정수 연산 순서 그대로) |
| 최소 그리드 | 열 2 × 행 2 미만이면 아무것도 그리지 않고 세션도 시작하지 않음 |
| 캐럿 | `(grid_x + (cc - view_x)*cell_w - view_fx, grid_y + view_frac + (cr-top)*cell_h + 1)`, 폭 `max(1, dpi/96)`, 높이 `cell_h - 2` |
| 종료 안내 | `(rc.x+2, rc.bottom() - cell_h - 1)`, 폭 `rc.w-4`, 높이 `cell_h` |
| 실패 안내 | `(rc.x+2, rc.y)`, 높이 `cell_h` |
| ×N 배지 | `FastScroller::paint(ctx, theme, rc, cell_h, 6)` — 그리드 위 마지막 |
| 배경 | 셀 배경(팔레트 `bg`)으로 런 단위 채움. 그리드 밖 여백(좌우 2px·상하 1px·남는 픽셀)은 도크 위젯의 `panel_bg` |

- 스크롤바 **없음**(세로·가로 모두 — 휠/드래그 자동 스크롤만).
- 탭 순서: 해당 없음(터미널 포커스는 클릭으로만 획득, Tab 키는 셸로 전달).

### 2-4. 단축키·마우스 요약

| 입력 | 조건 | 동작 |
| --- | --- | --- |
| Ctrl+`` ` `` | 터미널 비포커스 | 하단 도크 토글(`win.rs:8894`) — 터미널 포커스 중엔 삼킴 |
| 좌클릭(도크 터미널 영역) | — | 키 포커스 획득 / 그리드 안이면 선택 시작(또는 TUI 전달) |
| 좌클릭 `[→]` | — | 터미널 전환 + `cd` 현재 폴더 |
| 드래그 | 로컬 모드 또는 Shift | 선택(스트림), 그리드 밖 상/하 = 자동 스크롤 |
| 휠 | 터미널 위 | 스크롤백(로컬) / TUI 모드면 휠 이벤트 전달 |
| Shift+휠 · 가로 휠 | `term_wrap=false` | 가로 스크롤 |
| 우클릭 | 살아 있는 터미널 | 포커스 + 메뉴(복사/붙여넣기/전체 선택) / TUI 모드+포커스면 앱에 전달 |
| Ctrl+C | 터미널 포커스 | 선택 있으면 복사, 없으면 인터럽트(0x03) |
| Ctrl+V | 터미널 포커스 | 붙여넣기 |
| Backspace / Ctrl+Backspace | 터미널 포커스 | `0x7F` / `0x08` |
| ↑↓←→ Home End Del PgUp PgDn | 터미널 포커스 | VT 시퀀스(TERM-072) |
| 아무 키 | 종료 상태 | 재시작 |
| Alt+←/→/↑/↓ | 터미널 포커스여도 | 활성 패널 뒤로/앞으로/위로/행 활성화(TERM-077) |

### 2-5. 스킴 표(설정 창 표시 순 = 배열 순)

| # | id | 이름 | 다크 | 기본 전경 | 배경 | ANSI 16색 근거 |
| ---: | --- | --- | :---: | --- | --- | --- |
| 1 | `campbell` | Campbell | ✔ | `E6E6E6` | `0C0F12` | `lib.rs:60-83` |
| 2 | `one-half-dark` | One Half Dark | ✔ | `DCDFE4` | `282C34` | `lib.rs:160-186` |
| 3 | `solarized-dark` | Solarized Dark | ✔ | `839496` | `002B36` | `lib.rs:187-213` |
| 4 | `tango-dark` | Tango Dark | ✔ | `D3D7CF` | `000000` | `lib.rs:214-240` |
| 5 | `dracula` | Dracula | ✔ | `F8F8F2` | `282A36` | `lib.rs:241-267` |
| 6 | `nord` | Nord | ✔ | `D8DEE9` | `2E3440` | `lib.rs:268-294` |
| 7 | `gruvbox-dark` | Gruvbox Dark | ✔ | `EBDBB2` | `282828` | `lib.rs:295-321` |
| 8 | `catppuccin-mocha` | Catppuccin Mocha | ✔ | `CDD6F4` | `1E1E2E` | `lib.rs:322-348` |
| 9 | `tokyo-night` | Tokyo Night | ✔ | `C0CAF5` | `1A1B26` | `lib.rs:349-375` |
| 10 | `github-light` | GitHub Light | | `1B1F26` | `FFFFFF` | `lib.rs:91-114` |
| 11 | `one-half-light` | One Half Light | | `383A42` | `FAFAFA` | `lib.rs:382-408` |
| 12 | `solarized-light` | Solarized Light | | `657B83` | `FDF6E3` | `lib.rs:409-435` |
| 13 | `tango-light` | Tango Light | | `555753` | `FFFFFF` | `lib.rs:436-462` |
| 14 | `gruvbox-light` | Gruvbox Light | | `3C3836` | `FBF1C7` | `lib.rs:463-489` |
| 15 | `catppuccin-latte` | Catppuccin Latte | | `4C4F69` | `EFF1F5` | `lib.rs:490-516` |

Campbell의 배경(`0C0F12`)·전경(`E6E6E6`)은 Windows Terminal 원전(`0C0C0C`/`CCCCCC`)이 아니라 **앱 다크 토큰에 맞춘 값**이다(`lib.rs:59`). 값은 파일 그대로 이식한다(재입력 금지 — 복사).

### 2-6. 퀵 런처 바

- 위치: `y = 메뉴 22 + 도구 모음 28`, 높이 24(@96dpi), 전폭 — `win.rs:1858-1871`.
- 버튼: 16×16 아이콘 정사각 버튼 + 구분선(`ToolButton::sep()`), 순서 = 설정 파일 항목 순. 툴팁 = 라벨(추정 — `ToolButton::new(id, label)`의 라벨이 툴팁으로 쓰이는지는 chrome.rs 미확인).
- 클릭 라우팅: `y < panels[0].y`이면 `lb.h>0 && y >= lb.y` → 런처 바, 아니면 도구 모음 — `win.rs:7981-7995`.

---

## 3. nexa-ui 매핑

nexa-ui 확인 위치: `D:/Projects/kiros33/nexa-ui/crates/nexa-ctl/src`(Grep `term|pty|vt100|ansi` → **터미널 관련 타입 0건**).

| dir2 요소 | nexa-ui 대응 | 판정 | 필요한 추가/API |
| --- | --- | --- | --- |
| `nexa_term::VtScreen`·`TermPalette`·`SCHEMES`·`export` | 없음 | **추가(이식)** | 크레이트 그대로 이식(의존 0). 위치 제안: nexa-ui에 `nexa-term`(도메인 중립 — nexa-sql 등도 재사용 가능) 또는 nexa-dir3 워크스페이스 `ndir-term`. 아키텍처 문서에서 확정 |
| `term_paint`(호스트 직접 렌더) + `TermState`(선택·스크롤 상태) | 없음 | **추가 필요: `TermView` 컨트롤** | `nexa-ctl/src/controls/termview.rs`(제안). 컨트롤은 PTY를 모른다(nexa-ctl 경계 규칙 `lib.rs:21-26`): `set_bounds`·`paint(&self, ctx, theme, &VtScreen, &TermPalette)`·`on_event` → `TermAction::{Write(String), Copy, Paste, Resize{cols,rows}, RequestTick}` 큐. 내부 상태 = `view_off/view_frac/view_x/view_fx/sel/grid 캐시/WheelAccum×4/ScrollAccel+SpeedHud`. `cell_at`·`sel_norm`·`scroll_view(_px)`·`scroll_view_x_px`·`drag_extend`·`mouse_report()`를 **순수 메서드**로 두어 단위 테스트 |
| `DrawCtx::term_cell_w` / `term_text`(nexa-gui `draw.rs:57-66`) | `DrawCtx`에 없음(`nexa-ctl/src/draw.rs:5` "dir2 전용 어휘(터미널 셀…)는 제외") | **추가 필요** | ① `FontSlot::Term`(신규 슬롯 — 얼굴·크기 독립). 기존 `FontSlot::Mono`는 **크기가 Status 설정을 따르고 광학 보정(`mono_mult`)이 걸려** 셀 그리드에 부적합(`raster.rs:357-370`) ② 셀 폭 = `select_font(Term)` 후 `text_width("0")` ③ 글리프 = `text(x, y, clip, ch, fg)`(clip 인자로 셀 클립 — `draw.rs:65`) ④ 배경 = `fill_rect` |
| `push_clip`/`pop_clip`(dir2 `DrawCtx`) | `DrawCtx`에 없음(Grep: textbox.rs 내부만) | 대체 가능 | `text`/`text_opaque`가 clip 인자를 받으므로 **행·셀 rect를 내용 rect와 교차**시켜 넘기면 됨. `fill_rect`는 호출 측이 교차 계산. → 부분 행 번짐을 덮기 위한 "스트립 재도장"(TERM-067) 불필요해짐 |
| 글꼴 폴백 체인(`fontchain.rs` + `IDWriteFontFallback`) | `nexa_font::mono_font(family)`(`nexa-font/src/lib.rs:473`) · `find_font_by_family`(`:304`) · `nexa_gfx::Font::push_fallback`/`push_fallback_font`(`nexa-gfx/src/text.rs:359`·`:478`) · `has_glyph`(`:847`) | **있음(조립 필요)** | 쉼표 체인 → 설치된 첫 패밀리 = 주 얼굴, 나머지 = `push_fallback`. `find_font_by_family`는 **파일명 어간 비교**라 패밀리명≠파일명 글꼴은 못 찾음(`lib.rs:302` "정직한 한계") — 체인 해석 규칙 테스트 필요 |
| 단일 글리프 레이아웃 캐시(`mono_glyphs`) | `nexa_gfx::Font` 글리프 캐시(`text.rs:255-261`·`:494-502`) | 있음 | 추가 불요 |
| `WheelAccum` | `nexa_ctl::WheelAccum`(`event.rs:120`) | 있음 | — |
| `FastScroller`(배수+배지) | `ScrollAccel`(`controls/scroll.rs:258`) + `SpeedHud`(`:119`) + `FastScroll` 설정(`:56`) | 있음 | dir2 `FastScroller::wheel(delta, units)` 조합 래퍼는 TermView 안에서 재구성 |
| `wheel_lines()`(시스템 줄 수) | nexa-ctl에 전역 없음 | 추가 또는 호스트 주입 | TermView에 `lines_per_notch` 설정 메서드(기본 3) |
| 도크 종류 스트립 `InfoDock`(+`[→]`·`take_goto`·`content_rect`·`paint_strip`·`set_focused`) | 동일 위젯 없음. `TabBar`(`controls/tabbar.rs:111`)·`ToolDock`(`tooldock.rs:171` — 도구 모음 도킹용, 무관) | **추가 필요**(도크 인벤토리 소관) | 터미널 측 요구: 종류 인덱스 조회·`[→]` 1회성 통지·내용 rect·포커스 강조 3단계 색 규칙(§2-2) |
| 우클릭 편집 팝업(`TrackPopupMenuEx`) | `ContextMenu`/`CtxItem`(`controls/ctxmenu.rs`) · `EditMenu`(`controls/editmenu.rs:70`) | **있음(차이 있음)** | `EditMenu`는 **복사·잘라내기·붙여넣기·─·전체 선택** 고정 순서(`editmenu.rs:96-124`). dir2 터미널 메뉴는 **잘라내기 없음**. → `ContextMenu`로 직접 3항목 구성하거나 `EditMenuCaps`에 `hide_cut` 추가 |
| 퀵 런처 바 `Toolbar`+`ToolButton::with_icon` | `Toolbar`(`controls/toolbar.rs:230`) · `ToolItem::new(id, ToolIcon)`/`separator()`/`tip()`(`:125-202`) · `ToolIcon::Image(Rc<IconImage>)`·`Glyph(String)`(`:24-28`) | 있음 | 항목 id가 문자열(`take_clicked() -> Option<String>`) — `launcher:<idx>` 규약. 아이콘 로딩 전/실패 = `ToolIcon::Glyph(라벨 앞 2자)` |
| 메뉴 항목(체크) `MenuItem::checked` | `MenuBar`/`MenuDef`/`MenuEntry`(`controls/pulldown.rs`) | 있음(세부는 메뉴 인벤토리) | — |
| 설정 항목: CheckBox / Number / Select / Font | `Checkbox`·`Switch` / `TextBox`(숫자) / `Combo` / **글꼴 선택 컨트롤 없음**(Grep `fontbox|FontPicker` 0건 — nexa-ui·nexa-sql 모두) | Font만 **추가 필요**(설정 인벤토리 소관) | 터미널 측 요구: 쉼표 체인 직접 입력 + 설치 글꼴 목록 선택 |
| 캐럿 깜빡임 위상 | `DrawCtx::caret_on()`(`draw.rs:41`) · `RasterCtx::with_caret_on`(`raster.rs:113`) | 있음 | 호스트가 프레임마다 주입(nexa-sql `app/paint.rs:52-55` — 500ms 고정). dir2의 "비포커스 = 상시 표시" 규칙은 TermView가 `focused`로 판정 |
| 타이틀바 안내(` · {라벨} 실행`) | 해당 없음(호스트 창 제목) | 호스트 | nexa-sql 기준에 따라 상태바/토스트로 옮길지는 UI 인벤토리에서 결정 |

---

## 4. OS 분기점

### 4-1. PTY 백엔드 — 공통 추상

dir2의 `ConPty` 공개 표면이 그대로 트레이트가 된다(`conpty.rs:33-44`·`:51-59`·`:234`·`:250`):

```rust
pub struct PtyConfig<'a> { cwd: &'a Path, cols: u16, rows: u16, shell: Option<&'a Path> }
pub trait Pty: Send {                       // drop = 셸 종료·자원 정리
    fn write(&self, bytes: &[u8]);
    fn resize(&self, cols: u16, rows: u16);
}
pub fn spawn(cfg: PtyConfig, gen: u64, sink: Arc<Mutex<String>>, notify: impl Fn(PtyEvent) + Send + 'static)
    -> io::Result<Box<dyn Pty>>;           // PtyEvent::{Output, Exit} — notify는 EventLoopProxy::send_event 래핑
```

- `Utf8Chunker`(`conpty.rs:312-366`)는 **순수 타입**이므로 공통 모듈로 옮겨 3개 OS가 공유한다(테스트 7종 동반).
- 통지: `PostMessageW(WM_APP_TERM)` → winit `EventLoopProxy::send_event`(nexa-sql 선례 `nexa-sql/crates/nexa-sql/src/app/connwin.rs:92` `proxy.send_event(Wake)`). 세대(`gen`)·패널 인덱스는 이벤트 페이로드 또는 공유 큐에 싣는다.

### 4-2. Windows — ConPTY 유지

| 항목 | 현 구현 | dir3 방안 |
| --- | --- | --- |
| API | `windows` crate(`conpty.rs:10-25`) | nexa-sql/nexa-ui 기조 = **수동 FFI**(`extern "system"` — `nexa-sql/.../clipboard.rs:57-74` 선례, nexa-sys "수동 FFI"). kernel32: `CreatePipe`·`CreatePseudoConsole`·`ResizePseudoConsole`·`ClosePseudoConsole`·`InitializeProcThreadAttributeList`·`UpdateProcThreadAttribute`·`DeleteProcThreadAttributeList`·`CreateProcessW`·`ReadFile`·`WriteFile`·`WaitForSingleObject`·`TerminateProcess`·`DuplicateHandle`·`CloseHandle` |
| 셸 | pwsh → powershell → cmd(전체 경로) | 동일(TERM-040) |
| 하한 | Windows 10 1809+ | 동일 |

### 4-3. macOS·Linux — POSIX PTY(구체안)

**권장: `posix_openpt` + `std::process::Command` + `pre_exec`** (다중 스레드 프로세스에서 `forkpty(3)` 후 직접 exec하는 방식보다 안전 — fork~exec 사이의 async-signal-safe 제약을 std가 처리).

1. **마스터 열기**: `posix_openpt(O_RDWR | O_NOCTTY)` → `grantpt` → `unlockpt` → 슬레이브 경로(`ptsname_r` Linux / `ptsname` macOS — macOS는 스레드 안전하지 않으므로 UI 스레드에서만 호출). 마스터에 `fcntl(F_SETFD, FD_CLOEXEC)` **필수**(자식이 마스터를 상속하면 셸이 죽어도 EOF가 오지 않음. macOS `posix_openpt`는 `O_CLOEXEC`를 받지 않음).
2. **크기 설정**: `ioctl(master, TIOCSWINSZ, &winsize{ws_row, ws_col, 0, 0})`. 요청 번호(x86_64/aarch64): Linux `0x5414`, macOS `0x80087467`. *구현 시 헤더로 재확인(추정 아님 — 표준 값이나 수동 FFI이므로 테스트로 고정: `TIOCGWINSZ`로 읽어 왕복 검증).*
3. **termios**(Linux): `c_iflag |= IUTF8`(조리 모드에서 멀티바이트 Backspace 정확도). 그 외 커널 기본값 유지(`VERASE=0x7F` — dir2의 Backspace→0x7F 매핑과 일치).
4. **자식 기동**: 슬레이브를 `open(O_RDWR | O_NOCTTY)`로 열어 `Stdio::from(OwnedFd)` 3개(dup)로 stdin/stdout/stderr 연결. `Command::new(shell).current_dir(cwd)` + 환경(아래) + `pre_exec(|| { setsid(); ioctl(0, TIOCSCTTY, 0); Ok(()) })`(`TIOCSCTTY`: Linux `0x540E`, macOS `0x20007461`). spawn 후 **부모는 슬레이브 fd를 닫는다**.
5. **읽기 스레드**: `read(master)` 4096B 루프 → `Utf8Chunker` → 공유 버퍼 → `notify(Output)`. 종료 조건: Linux는 슬레이브가 전부 닫히면 `EIO`, macOS는 `0`(EOF) — **둘 다 EOF로 처리**. 세션 Drop 시 블로킹 read를 깨우기 위해 `poll([master, wake_pipe])` + self-pipe 사용(다른 스레드에서 `close(master)`만으로는 Linux에서 블로킹 read가 깨지지 않는다).
6. **종료 감지**: 별도 스레드 `child.wait()` → `notify(Exit)`(백그라운드 잡이 슬레이브를 잡고 있으면 EOF가 늦으므로 EOF가 아니라 **waitpid 기준** — dir2의 대기 스레드와 동일 구조). 좀비 방지를 위해 반드시 reap.
7. **쓰기**: `write(master)`. PTY 입력 버퍼가 차면 블로킹 → 대용량 붙여넣기는 **쓰기 스레드(채널)** 로 분리 권장(dir2는 UI 스레드 동기 쓰기 — `conpty.rs:234-247`).
8. **리사이즈**: `ioctl(master, TIOCSWINSZ)` → 커널이 포그라운드 프로세스 그룹에 `SIGWINCH`.
9. **Drop**: `kill(-pid, SIGHUP)`(세션 리더 pid = pgid) → wake_pipe로 읽기 스레드 종료 → `close(master)` → 일정 시간 내 미종료면 `SIGKILL` → `wait`.
10. **FFI 조달**: `libc` crate(MIT/Apache — 퍼미시브, 사실상 표준) 채택 또는 수동 `extern "C"` 선언. 기존 기조("외부 crate 0 지향")와의 절충은 개발 기준 문서에서 결정 — 수동 FFI면 OS별 상수(`O_NOCTTY`·ioctl 번호·`winsize`/`termios` 레이아웃)를 `cfg(target_os)`로 분리하고 왕복 테스트로 고정.

대안(비권장): `forkpty(3)`(Linux glibc < 2.34는 `-lutil` 링크 필요, macOS는 libSystem) — 호출은 단순하나 자식 쪽 코드를 직접 써야 하고 다중 스레드 fork 제약을 스스로 지켜야 한다. `docs/23-cross-platform-feasibility.md:126`은 `forkpty(3)`를 언급하나 구현 상세는 없다.

### 4-4. 기본 셸·환경 변수

| | Windows(현행) | macOS | Linux |
| --- | --- | --- | --- |
| 셸 탐색 | pwsh → powershell(PATH, 절대 항목만) → `%SystemRoot%\System32\cmd.exe` | `$SHELL`(절대 경로·파일·실행 가능 검증) → `getpwuid(getuid())->pw_shell` → `/bin/zsh` → `/bin/sh` | `$SHELL` → `pw_shell` → `/bin/bash` → `/bin/sh` |
| 기동 인자 | 없음 | **로그인 셸** 관례(`argv[0] = "-zsh"` 또는 `-l`) — Finder에서 띄운 GUI 앱은 PATH가 최소라 로그인 셸이어야 사용자 PATH가 잡힘 | 인자 없음(대화형 비로그인) |
| 환경 | 부모 상속 그대로 | `TERM=xterm-256color` · `COLORTERM=truecolor` · `LANG` 없으면 `en_US.UTF-8`(또는 시스템 로캘 + `.UTF-8`) · `TERM_PROGRAM=nexa-dir`(제안) | `TERM=xterm-256color` · `COLORTERM=truecolor` · `LANG`/`LC_*` 상속 |
| 주의 | **`NO_COLOR` 상속 시 pwsh 색이 꺼짐**(`docs/journal/2026-09-04.md:165-170`) | 동일 원리 — 개발 도구 셸에서 띄울 때 `NO_COLOR` 제거 | 동일 |
| 사용자 지정 | 없음 | (제안) 설정 `terminal.shell`(빈 값 = 자동) — dir2에 없는 신규 키이므로 계승 범위 밖, 선택 사항 |

`default_shell` 테스트(`conpty.rs:373-377` "절대 경로·존재")는 3개 OS 공통 불변식으로 유지.

### 4-5. 키 입력 인코딩

| 항목 | Windows(현행) | macOS / Linux 방안 |
| --- | --- | --- |
| 문자 원천 | `WM_CHAR`가 Ctrl+문자(0x01~0x1A)·Enter(`\r`)·Tab·Esc·IME 확정 문자열을 모두 문자로 전달 | winit `KeyEvent{logical_key, text, …}` + `Ime::Commit`. **Ctrl 조합은 OS가 문자로 주지 않으므로 직접 인코딩**: Ctrl+A~Z → `c & 0x1F`, Ctrl+`[`→ESC, Ctrl+`\`→0x1C, Ctrl+`]`→0x1D, Ctrl+Space→0x00. Windows도 같은 표를 쓰면 3개 OS 동작이 일치(권장 — 순수 함수 `encode_key(key, mods, modes) -> Option<Vec<u8>>`로 분리·표 테스트) |
| Backspace | `0x7F` | `0x7F`(VERASE 기본과 일치) |
| Ctrl+Backspace | `0x08`(PSReadLine 단어 삭제) | bash/zsh에서 `0x08`은 한 글자 삭제 → **`0x17`(Ctrl+W, 단어 삭제)** 로 분기(제안) |
| 화살표·Home/End | `ESC[A` … `ESC[H`/`ESC[F`(고정) | **DECCKM(`?1`) 추적 필요**: 응용 커서 모드면 `ESC O A` 등. ConPTY는 입력을 conhost가 해석해 주지만 POSIX PTY는 변환 계층이 없다(vim·less·htop이 의존) |
| Alt+문자 | 미전달(SYSKEYDOWN은 앱 처리) | POSIX: `ESC` 접두(메타). macOS Option은 기본적으로 문자 조합이므로 "Option을 Meta로" 여부는 설정 후보 |
| F1~F12·Insert·Shift+Tab | 미매핑(삼킴) | dir2 계승 = 미매핑. POSIX TUI 사용성을 위해 표 확장 권장(`ESC OP`…`ESC[24~`, `ESC[2~`, `ESC[Z`) — 선택 |
| 복사/붙여넣기 단축키 | Ctrl+C(선택 시 복사)·Ctrl+V | 기본은 dir2 규칙 계승 + OS 관례 추가: macOS `⌘C`/`⌘V`(Ctrl+C는 항상 0x03), Linux `Ctrl+Shift+C`/`Ctrl+Shift+V`·`Shift+Insert`. nexa-ctl `InputEvent::Key{primary}`가 OS 주 수식키를 추상화(`event.rs:60-69`) |
| IME | Imm32, 조합 창 OS 기본 위치 | winit `set_ime_allowed(true)` + `set_ime_cursor_area(캐럿 셀 rect)` — 캐럿 위치에 조합 창(개선). 조합 중 문자열 인라인 표시는 dir2에 없음 |
| 캐럿 깜빡임 주기 | `GetCaretBlinkTime`(폴백 530ms) | macOS `NSTextInsertionPointBlinkPeriod`(기본 약 560ms — 추정)·Linux 데스크톱 설정 → nexa-sql은 **500ms 고정**(`app/event_loop.rs:559`). nexa-sql 기준에 맞춰 고정값 사용 권장 |

### 4-6. ConPTY가 대신해 주던 것(★ 가장 큰 이식 위험 — 추정 포함)

ConPTY(conhost)는 자식의 출력을 자기 화면 버퍼에 렌더한 뒤 **정규화한 VT 스트림**을 내보낸다. dir2 `VtScreen`은 그 정규화된 스트림을 전제로 최소 구현만 갖고 있다. POSIX PTY는 프로그램 출력이 **가공 없이** 온다 → 아래가 없으면 zsh/bash 프롬프트·vim·less·htop·mc가 깨진다. (어느 시퀀스가 ConPTY에서 실제로 걸러지는지는 실기 캡처로 확인하지 않았다 — **추정**. 감사 계획도 "대체 화면(1049) 처리 = 인박스 ConPTY 실기 캡처 후"로 보류 중: `docs/audit/20261002-ultracode/plan.md:694`.)

| 누락 기능 | 영향 | 우선순위 |
| --- | --- | --- |
| **응답이 필요한 질의**: DSR `CSI 6n`(커서 위치) · `CSI 5n` · DA `CSI c` · `CSI > c` — 파서가 PTY에 **되써야** 함(현재 `VtScreen`에 출력 채널 없음) | zsh/fish 프롬프트·readline이 응답을 기다리며 지연/깨짐 | 필수 |
| 대체 화면 `?1049`/`?47`/`?1047`(+커서 저장) | vim·less·htop 종료 후 화면 복원 안 됨, 스크롤백 오염 | 필수 |
| DECCKM `?1` · 키패드 모드 `ESC =`/`>` | 화살표 키 오동작(§4-5) | 필수 |
| 괄호 붙여넣기 `?2004` | bash/zsh가 기본으로 켬 — 여러 줄 붙여넣기가 즉시 실행됨(감사 A65b 미반영) | 필수 |
| 지연 줄바꿈(pending wrap) | 마지막 열에 쓴 뒤 `CR`/커서 이동 시 한 줄 밀림(`lib.rs:934-937` 즉시 줄바꿈) — 감사 BL-17 | 필수 |
| DEC 특수 그래픽(`ESC ( 0` 선 그리기) | mc·dialog·일부 ncurses 테두리가 `lqqk`로 표시(현재 지정자 폐기만) | 높음 |
| DECTCEM `?25`(커서 숨김) | TUI에서 캐럿이 항상 보임(감사 A65a) | 높음 |
| SGR 3/4/9(이탤릭·밑줄·취소선)·콜론 문법 | 표시 누락(무해) | 중 |
| 탭 정지(HTS/TBC)·CBT/CHT·REP(`CSI b`)·IRM·DECOM | 일부 TUI 레이아웃 어긋남 | 중 |
| IL/DL·CUU/CUD의 스크롤 마진 준수(TERM-008·010) | vim 스크롤 영역 깨짐 | 높음 |
| OSC 0/2(제목)·OSC 7(cwd)·OSC 8(링크) | 무시해도 무해(파싱은 됨) | 낮음 |
| 이모지·비BMP 폭(`is_wide` BMP만) | 열 어긋남 | 중 |
| CSI 중 C0 즉시 실행·`> = <` 마커(감사 A10) | 드문 깨짐 | 낮음 |

→ dir3에서는 `VtScreen`을 **그대로 이식한 뒤**(기존 테스트 20종 green 유지) 위 항목을 **추가 테스트와 함께** 확장한다. 확장은 Windows에도 무해해야 한다(ConPTY 스트림은 부분집합).

### 4-7. `[→]` cd 명령(셸별 문자열)

| 셸 | dir2 현행 | dir3 방안 |
| --- | --- | --- |
| pwsh/powershell | `cd "<경로>"\r`(`win.rs:8089`) | `Set-Location -LiteralPath '<경로>'\r`(`'`→`''`) — 현행 큰따옴표는 `$`·백틱이 들어간 폴더명에서 보간됨(추정 — 실기 미확인) |
| cmd | 동일 문자열(드라이브가 다르면 `cd`만으로는 이동 안 됨 — 추정) | `cd /d "<경로>"\r` |
| POSIX sh 계열 | — | `cd '<경로>'\r`(`'`→`'\''`). 선행 공백을 붙여 히스토리 제외(HISTCONTROL 의존)는 선택 |

순수 함수 `cd_command(ShellKind, &Path) -> String`로 분리해 표 테스트. 셸 종류는 실행 파일 이름으로 판정.

### 4-8. 클립보드(서식 복사)

| | Windows(현행) | macOS | Linux |
| --- | --- | --- | --- |
| 평문 | `CF_UNICODETEXT` | `public.utf8-plain-text`(NSPasteboard) | X11 `UTF8_STRING`/`text/plain;charset=utf-8` · Wayland `text/plain;charset=utf-8` |
| HTML | 등록 형식 `HTML Format` + CF_HTML 헤더(`export::cf_html`) | `public.html`(**헤더 없이** fragment) | `text/html` |
| RTF | 등록 형식 `Rich Text Format` | `public.rtf` | `text/rtf`(+`application/rtf`) |
| nexa-sql 기반 | `write_rich(text, html)` 있음(`clipboard.rs:27`), **RTF 없음** | osascript `«class HTML»` 방식(`clipboard.rs:252`) — RTF 추가 시 `«class RTF »` | X11 직접 구현은 **텍스트 타깃만**(`clipboard_x11.rs:11`) → `text/html`·`text/rtf` 타깃 추가 필요. Wayland는 `wl-copy` 단일 형식 |
| 줄 구분 | 평문 CRLF(`get_text`) | LF로 변환 권장 | LF로 변환 권장 |

→ 공용 API 제안: `clipboard::write_formats(text, html_fragment: Option<&str>, rtf: Option<&str>)`. `cf_html` 래핑은 Windows 구현 안으로. 클립보드 열기 실패 시에도 선택이 해제되는 현행 동작(감사 #25 — `docs/29-audit-checklist.md:256`)은 **성공 시에만 해제**로 고친다.

### 4-9. 퀵 런처

| | Windows(현행) | macOS | Linux |
| --- | --- | --- | --- |
| 실행 | `ShellExecuteW("open", exe, args, workdir)` | 실행 파일: `Command::new(exe).args(…).current_dir(dir).spawn()` · `.app` 번들: `open -a <App> --args …` 또는 `open -a <App> <dir>` | `Command::new(exe)…spawn()`(자식 분리: `setsid`/이중 fork 또는 `Stdio::null` + reap 스레드) |
| 인자 분해 | 문자열 그대로 OS에 전달 | **인자 문자열을 argv로 분해하는 규칙 필요**(따옴표 처리 — 순수 함수 + 테스트) | 동일 |
| 시드 | VS Code(3경로) │ pwsh/PowerShell · cmd | VS Code(`/Applications/Visual Studio Code.app`) │ Terminal.app(`open -a Terminal "%path%"`) | VS Code(`code` PATH) │ 터미널 에뮬레이터(`x-terminal-emulator` → `gnome-terminal --working-directory=%path%` → `konsole --workdir %path%` → `xterm`) |
| 아이콘 | exe 셸 아이콘 16px | `.app` 아이콘(`NSWorkspace.icon(forFile:)`) — nexa-fs 미구현 → 라벨 2자 폴백 | `.desktop`/아이콘 테마 — 미구현 → 라벨 2자 폴백 |
| `%path%` | 활성 패널 현재 폴더 | 동일 | 동일 |
| 포그라운드 양도 | `allow_foreground_handoff` | 불필요 | 불필요 |

`SEED_VERSION` 마이그레이션 로직(`seed_missing` — "pwsh/powershell/cmd.exe 포함 여부" 판정)은 OS별 needle 표로 일반화.

### 4-10. 기타

| 항목 | Windows | macOS / Linux |
| --- | --- | --- |
| 타이머(캐럿 530ms·선택 자동 스크롤 60ms·위젯 틱 40ms) | `SetTimer` 3종 | winit `ControlFlow::WaitUntil`로 다음 마감 시각 계산(nexa-sql `app/event_loop.rs:499`) — **창 비활성/최소화 시 캐럿 타이머 정지** 포함 |
| 선택 자동 스크롤의 커서 위치 | `GetCursorPos`+`ScreenToClient` 폴링 | 마지막 `CursorMoved` 좌표 캐시(winit은 캡처 중 창 밖 좌표도 전달 — 플랫폼별 확인 필요, 추정) |
| 마우스 캡처 | `SetCapture`/`ReleaseCapture`/`WM_CAPTURECHANGED` | winit 암묵 캡처(버튼 누른 채 창 밖 이동 시 이벤트 지속 여부는 OS별 상이 — 추정) → `Focused(false)`·`CursorLeft`에서 `reset_mouse_transients` 등가 호출 |
| 휠 단위 | `WHEEL_DELTA=120`, 노치 미만 = 정밀 터치패드 | winit `MouseScrollDelta::LineDelta`/`PixelDelta` → 호스트가 120 단위로 정규화(nexa-ctl `event.rs:7-8` 규약). macOS는 대부분 PixelDelta |
| 기본 터미널 글꼴 | `Consolas` | macOS `Menlo`/`SF Mono` · Linux `DejaVu Sans Mono` 등 — `nexa_font::system_mono_font()`가 한글 고정폭(D2Coding 등) 우선 탐색(`nexa-font/src/lib.rs:336-349`). 설정 기본값은 빈 값 = 시스템 모노로 두는 방안 권장 |
| HTML 복사의 폴백 글꼴 | `font-family:'<font>',Consolas,monospace` 하드코딩(`lib.rs:1240`) | `Consolas` 대신 OS 기본 모노 이름 주입 또는 `monospace`만 |

---

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 설정 키(dir2 `settings.cfg` — `key=value` 줄 형식)

| dir2 키 | 타입·범위 | 기본값 | 근거 | dir3 키 제안(nsql-settings 식 `카테고리.이름`) |
| --- | --- | --- | --- | --- |
| `term_font` | 문자열 ≤128(쉼표 체인) | `Consolas` | `config.rs:262`·`:576` | `terminal.font` |
| `term_font_size` | 정수 8~32 | 12 | `config.rs:263`·`:579` | `terminal.font_size` |
| `term_wrap` | 0/1 | 1 | `config.rs:264`·`:638` | `terminal.wrap` |
| `term_cols` | 정수 80~1000 | 240 | `config.rs:265`·`:639` | `terminal.cols` |
| `term_theme` | `system`/`dark`/`light`/스킴 id, ≤64 | `system` | `config.rs:266`·`:645` | `terminal.theme` |
| `term_theme_dark` | 스킴 id ≤64 | `campbell` | `config.rs:267`·`:648` | `terminal.theme_dark` |
| `term_theme_light` | 스킴 id ≤64 | `github-light` | `config.rs:268`·`:651` | `terminal.theme_light` |
| `term_copy_format` | `text`/`html`/`rtf`/`both` | `text` | `config.rs:269`·`:654` | `terminal.copy_format` |
| `launcher` | 0/1 | 1 | `config.rs:316`·`:758` | `launcher.visible` |
| `launcher_seed` | 정수 | 0(저장 시 2) | `config.rs:318`·`:759`·`win.rs:7013` | `launcher.seed` |
| `launcher_count` / `launcherN` | `라벨\|exe\|인자` 또는 `-` | (시드) | `config.rs:549-560`·`:760-790` | `launcher.items`(목록 표현은 설정 구조 문서에서 확정) |
| `dock` · `dock_ratio` · `dock_split` | 0/1 · 0.15~0.5 · 0.15~0.85 | 1 · 0.3 · 0.5 | `config.rs:259-261`·`:575`·`:590-600` | (도크 인벤토리 소관) |

- 영속하지 **않는** 상태: 도크 활성 종류(터미널 여부)·터미널 세션·스크롤백·선택·`view_off`. 재기동하면 도크는 정보 종류로 뜨고 터미널은 지연 시작.
- nsql-settings 규약(레지스트리 단일 원천 · 기본값과 다른 값만 저장 · 모르는 키 보존 · 저장 전 검증 — `nexa-sql/crates/nsql-settings/src/lib.rs:10-17`)에 맞추면 dir2의 "파싱 시 클램프·모르는 값 무시"가 레지스트리 검증으로 옮겨간다. `terminal.theme`의 **모르는 스킴 id 허용(해석 시 폴백)** 은 유지해야 하므로 이 키는 자유 문자열 종류로 등록한다(열거 검증 금지).

### 5-2. 런타임 상태

| 상태 | 위치 | 설명 |
| --- | --- | --- |
| `terms: [Option<TermState>; 2]` · `term_gen: u64` | `win.rs:1049-1051` | 패널별 세션, 세대 카운터(시작마다 +1) |
| `term_focus: Option<usize>` | `win.rs:1053` | 터미널 키 포커스(패널 인덱스) |
| `term_drag: Option<usize>` | `win.rs:1055` | 선택 드래그 중 |
| `term_mouse_btn: Option<(usize, u8)>` | `win.rs:1057` | TUI 전달 중 눌린 버튼(패널, SGR 코드 0/2) |
| `term_caret_on: bool` | `win.rs:1067` | 깜빡임 위상 |
| `TermState{pty, screen, exited, view_off, sel, grid, view_x, wheel, hwheel, tui_wheel, fast, view_frac, view_fx, wheel_px}` | `win.rs:1073-1101` | `grid`=(내용 rect, cell_w, cell_h) **페인트가 캐시** → 히트 테스트가 읽음 |
| `ConPty{hpc, process, thread, writer, attr_list, attr_size, gen, output}` | `conpty.rs:33-44` | `output: Arc<Mutex<String>>` = 읽기 스레드 ↔ UI 공유 버퍼 |

### 5-3. 스레드·메시지 흐름

```
[UI 스레드]                                   [읽기 스레드/세션]            [대기 스레드/세션]
paint → term_paint
  └ 세션 없음 → ConPty::start ─────────────→ ReadFile 루프(4096B)          WaitForSingleObject(셸)
                                              Utf8Chunker.push
                                              output.lock().push_str
                             ←── PostMessage(WM_APP_TERM, panel, gen) ──
WM_APP_TERM: gen 일치 확인                                                  셸 종료
  take(output) → screen.feed                 ←── PostMessage(WM_APP_TERM, panel|EXIT_FLAG, gen) ──
  view_off 보정 → 도크 rect 무효화            (EOF → 스레드 종료)           (복제 핸들 닫고 종료)
WM_CHAR/WM_KEYDOWN → pty.write (동기 WriteFile)
크기 변화(페인트 시 감지) → screen.resize + pty.resize
Drop(State) → TerminateProcess → ClosePseudoConsole → 핸들 정리
```

- 세션당 스레드 2개(읽기·종료 대기). 통지는 청크마다 1회(코얼레싱 없음 — UI는 통지당 버퍼 전체를 `take`하므로 중복 통지는 빈 문자열 feed).
- 출력 버퍼에 **상한 없음**: UI가 막히면 `String`이 계속 자란다(역압 없음).
- 터미널이 보이지 않아도(다른 종류·도크 숨김) feed와 도크 rect 무효화는 수행(감사 B-G3-07 "다른 종류 도크일 때 출력 무효화 생략" 미반영 — `plan.md:414`).
- 타이머: `TIMER_TERM_SEL=5`(60ms) · `TIMER_TERM_CARET=6`(캐럿 주기) · `TIMER_WIDGET_TICK=15`(40ms, 배지 페이드) — `win.rs:79-81`·`:123-127`.
- 락 규약: `crate::win::plock(&mutex)`(poison 무시 잠금 — 구현은 범위 밖, 추정)으로 버퍼 접근(`conpty.rs:173`·`win.rs:8963`).

---

## 6. 이식 시 주의(회귀 방지에 필요한 실측 교훈·결함 이력)

1. **셀 단위 글리프 배치 필수** — 런 단위 텍스트 레이아웃은 폴백 글꼴(한글·아이콘)의 전진폭이 셀 그리드와 어긋나 `ls` 이름 열이 밀린다. 배경은 런 병합, 문자는 셀 x에 1글자씩(`win.rs:2656-2658` · journal 07-14).
2. **Backspace = 0x7F, 0x08 직송 금지** — ConPTY/PSReadLine이 0x08을 Ctrl+Backspace(단어 삭제)로 해석해 입력 전체가 지워졌다(`win.rs:9336-9337` · journal 07-14:263-265). POSIX에서는 Ctrl+Backspace 값을 재매핑(§4-5).
3. **셸은 전체 경로로 실행 · PATH의 상대 항목 제외** — 이름만 넘기면 앱 폴더/CWD 검색 = 바이너리 플랜팅(`conpty.rs:116-118`·`:281-301` · `docs/29-audit-checklist.md:235`). POSIX `$SHELL`도 절대 경로·실행 가능 검증.
4. **UTF-8 디코더는 불량 바이트를 U+FFFD로 치환하고 전진** — 종전 구현은 불량 바이트 1개에 출력이 영구 정지·메모리 무한 증가(`conpty.rs:303-311`). `Utf8Chunker` + 테스트 7종을 그대로 가져간다.
5. **종료 대기 스레드에는 복제 핸들** — 원본 핸들 값 재활용 시 무관 객체를 영원히 대기(`conpty.rs:186-189`). POSIX는 `Child` 소유권을 대기 스레드로 이동해 같은 문제를 구조적으로 제거.
6. **ConPTY가 소유하는 파이프 끝은 부모 사본을 닫아야 EOF가 전파**(`conpty.rs:88-90`). POSIX 등가: 부모의 슬레이브 fd 닫기 + 마스터 `FD_CLOEXEC`.
7. **터미널 포커스 중 키가 파일 목록으로 새면 안 된다** — PTY 미기동/종료 상태에서 Delete=휴지통·Enter=실행·문자=타입어헤드가 보이지 않는 목록 캐럿에 작용했던 결함(HIGH). `KeyRoute::TermPending`은 **삼킴**, 기동 실패 시 포커스 해제(`win.rs:6079-6113`·`:4875-4879`·`:9340-9345`). `route_key_with_term`과 MC/DC 테스트를 그대로 이식.
8. **도크 표시 판정은 `visible && h>0`** — 싱글 정보 모드의 0-rect 우 도크가 터미널 종류면 숨은 터미널이 포커스를 가져가고 PTY가 기동됐다(`panel.rs:432-436` · `win.rs:4844`·`:8101-8104`).
9. **터미널 대상 편집 명령 전에 경로바 편집·리네임 취소** — 안 하면 Ctrl+V가 리네임 필드에 붙는다(`win.rs:8110-8112`·`:7606-7613`).
10. **캡처 상실 시 과도 상태 일괄 정리** — 선택 드래그 타이머·TUI 버튼 상태가 남아 모든 마우스 이동이 죽었다(`win.rs:5994-6029`). winit에서는 포커스 상실·커서 이탈 이벤트에 연결.
11. **스크롤백 보기 중 새 출력 = 보던 위치 고정**, 입력 시 하단 스냅(WT 규약 — `win.rs:8965-8969`).
12. **트랙패드 분수 delta는 누적기로** — `3*delta/120` 정수 나눗셈은 0줄로 버려 천천히 움직이면 무반응. TUI 휠도 누적기로 1회/노치(`win.rs:1085-1091`).
13. **CSI 파라미터는 포화 누적 + 상한 65535, 반복 연산은 영역 크기로 클램프** — 곱셈 오버플로 패닉·`ESC[999999999L` CPU 소진·`ESC[65535S` 수백 MB 할당(`lib.rs:797-804`·`:960-962`·`:1048-1049`).
14. **`get_runs` 역순 범위 언더플로 가드**(`lib.rs:1184-1186`).
15. **선택 = 반전(사용자 확정, 재론 금지)**, 밝은 팔레트의 기본색 셀만 accent 블록(`win.rs:2615-2617` · journal 09-04:154-176).
16. **캐럿 색 = 팔레트 `fg`** — 고정 회색(`CCCCCC`)·`theme.text`는 반대 밝기 조합에서 보이지 않는다(`win.rs:2726-2728`).
17. **라이트 스킴에서 흰색 계열(7·15)은 회색 매핑** — pwsh/PSReadLine이 숫자·멤버에 97, 타입에 37을 쓴다(`lib.rs:85-90`). 값 변경 금지(대비 테스트로 고정).
18. **`NO_COLOR` 환경 변수 상속 주의** — 개발 도구 셸에서 앱을 띄우면 pwsh 색이 꺼져 "색이 안 나온다"는 오탐이 난다(journal 09-04:165-170). 테스트 하네스·실행 스크립트에서 제거.
19. **ECH 미구현 시 잔상**(PSReadLine 백스페이스 재그리기 — `lib.rs:910`), **DECSTBM 미구현 시 영역 스크롤 어긋남**(`lib.rs:894`).
20. **설정 파일 손상 내성** — 모르는 스킴 id는 저장 시 거르지 않고 해석 시 폴백(검은 화면 방지 — `lib.rs:532-534`). 숫자 키는 클램프(`config.rs:1231-1260` 퍼징 테스트).

**미해결로 남아 있는 것(dir3에서 같이 고칠 후보 — 계승 여부 결정 필요)**

| # | 내용 | 근거 |
| --- | --- | --- |
| a | 터미널 포커스를 둔 채 창이 비활성/최소화돼도 캐럿 타이머가 계속 돌며 재도장(유휴 CPU 0% 규율 위반) | `docs/29-audit-checklist.md:241` · `plan.md:57` |
| b | 글리프 x 좌표에 `view_fx`(부분 열 오프셋) 미반영 — 배경은 `-fx` 적용(`win.rs:2674`), 글자는 미적용(`win.rs:2698`) → 가로 픽셀 스크롤 중 글자와 배경이 최대 1셀 미만 어긋남(코드 대조로 확인, 실기 미확인) | `win.rs:2674`·`:2698` |
| c | `term_send_mouse` 좌표가 `view_x`/`view_fx`/`view_frac`/`view_off`를 반영하지 않음(고정 열 모드에서 가로 스크롤 후 TUI 클릭 좌표 어긋남 — 코드 대조, 실기 미확인) | `win.rs:6214-6215` |
| d | 복사 실패(클립보드 열기 실패)에도 선택 해제 · HTML/RTF 부분 실패 무신호 · `TextRun`에 faint 없음 · HTML 글꼴명 = 체인 1순위 원문(미설치여도) | `docs/29-audit-checklist.md:256` · `win.rs:7404-7428` |
| e | 평문은 `trim_end()`(전각 공백 포함), 서식은 `' '`만 제거 — 줄 끝 규칙 불일치 | `lib.rs:699`·`:1203` · `plan.md:684`(U3) |
| f | 기동 실패 시 매 페인트 재시도 | `plan.md:140`(A70) |
| g | wrap 모드에서 Shift+휠이 터미널에서 소비되지 않고 목록으로 내려감 | `win.rs:7863`·`:7872` · `plan.md:106`(A33) |
| h | 리사이즈 시 리플로 없음·행 축소 시 이력 손실(의도적 보류 — ConPTY 재송신과 중복 위험) | `lib.rs:707-729` · `plan.md:655` |
| i | 터미널 포커스 중 Alt+방향키가 패널 탐색으로 작용 | `win.rs:8905-8939` |
| j | 가상 루트·클라우드 경로에서 터미널 시작 시 실패(가드 없음 — 추정) | `win.rs:4848` |
| k | i18n `term.fail` 문구에 "(ConPTY)" 하드코딩 | `lang/ko.lang:181` · `lang/en.lang:182` |
| l | 공유 도크(싱글 정보)의 터미널이 활성 패널을 따르지 않음(좌 고정 — β 잔여) | `docs/TODO.md:101` · `win.rs:2484` |

---

## 7. 회귀 테스트 후보

자동화 표기: **U**=단위 테스트(순수 로직) · **I**=통합(실제 PTY 프로세스, 3 OS CI) · **R**=렌더 골든(오프스크린 래스터 → 픽셀/셀 덤프 비교) · **M**=수동 실기.

| # | 시나리오 | 대상 ID | 자동화 |
| ---: | --- | --- | :---: |
| 1 | 기존 nexa-term 테스트 20종 전부 이식·green(`lib.rs:1396-1853`) | TERM-001~024·030~032 | U |
| 2 | `Utf8Chunker` 7종 이식(`conpty.rs:379-483`) | TERM-042 | U |
| 3 | `route_key_with_term` MC/DC 4쌍(`win.rs:9916-9938`) | TERM-071 | U |
| 4 | 설정 왕복·손상 내성: 8개 키 왕복, `term_cols=20`→80, `term_copy_format=xml`→기본, `term_theme=`(빈 값)→기본, 1MB 글꼴 문자열 | TERM-110~113 | U |
| 5 | `resolve_scheme` 12 조합 + 스킴 15종 무결성(id 유일·순서·휘도 분류) + **팔레트 값 스냅샷**(dir2와 바이트 동일) | TERM-030~032 | U |
| 6 | 키 인코딩 표: 9종 비문자 키, Backspace=0x7F, Ctrl+Backspace(OS별), Ctrl+A~Z, Enter=`\r`, DECCKM on/off | TERM-072·073 | U |
| 7 | `cell_at`/`sel_norm`/`scroll_view`/`scroll_view_px`/`scroll_view_x_px`/`drag_extend` 경계값(view_off·view_frac·view_x 조합, 그리드 밖 클램프) | TERM-080~084 | U |
| 8 | SGR 마우스 보고 문자열: press/release/motion/wheel × Shift 우회 × 모드(1000/1002/1003, 1006 유무) | TERM-085 | U |
| 9 | `cd_command` 표: pwsh/cmd/sh × 공백·`'`·`$`·한글·다른 드라이브 | TERM-101 | U |
| 10 | 붙여넣기 변환: CRLF/LF→CR, 괄호 붙여넣기 래핑(2004 on/off) | TERM-091 | U |
| 11 | 서식 내보내기: HTML 이스케이프·CF_HTML 바이트 오프셋·RTF `\uN?`·`\fs` 환산 + 평문/서식 본문 일치 | TERM-019~023·090 | U |
| 12 | 런처: `%path%` 치환·인자 분해·시드/`seed_missing` 멱등(2회 호출해도 중복 없음)·구분선 처리·`launcher_count=0` 비움 존중 | TERM-121~123 | U |
| 13 | `default_shell`: 절대 경로·존재·실행 가능(3 OS) | TERM-040 | U/I |
| 14 | **PTY 스모크**: 세션 시작 → `echo NEXA_OK`+Enter 쓰기 → 타임아웃 내 화면에 `NEXA_OK` 출현 → `exit` → Exit 통지 수신 | TERM-041~045·050 | I |
| 15 | PTY 리사이즈: `resize(100,30)` 후 셸에서 크기 조회(`stty size` / `$Host.UI.RawUI.WindowSize`) 결과 일치 | TERM-046 | I |
| 16 | 시작 cwd: 임시 폴더로 시작 → `pwd`/`cd` 출력에 경로 포함 | TERM-102 | I |
| 17 | Drop 정리: 세션 100회 생성/파기 후 핸들·fd·좀비 프로세스 수 불변 | TERM-047 | I |
| 18 | 세대 가드: 세션 교체 후 낡은 세대의 Output/Exit 이벤트 무시 | TERM-043 | U |
| 19 | 한글·전각 출력: `echo 한글` → 셀 폭 2·연속 셀·`get_text` 일치, 읽기 경계 분할 무손상 | TERM-015·042 | I/U |
| 20 | 그리드 렌더 골든: 고정 `VtScreen` 상태(색·reverse·faint·전각·선택·캐럿)를 다크/라이트 팔레트로 래스터 → 셀별 배경/전경 색 덤프 비교 | TERM-060~064 | R |
| 21 | 테마 전환: 같은 화면을 스킴 A→B로 재도장 시 스크롤백 줄 색이 B 팔레트 | TERM-033 | R |
| 22 | 스크롤백: 1000줄 출력 후 `scrollback_count()==800`, 보기 고정(`view_off` 보정), 입력 시 0 스냅 | TERM-012·083 | U |
| 23 | 종료→재시작: Exit 후 키 입력 → 새 세대 세션, 문자 누수 없음(목록 캐럿 불변) | TERM-050·071 | U/I |
| 24 | 기동 실패: 존재하지 않는 cwd → 실패 표시 + 포커스 해제 + 재시도 폭주 없음 | TERM-049 | I |
| 25 | VT 처리량 벤치(`audit_vt`) ≥ 10MB/s·비정상 시퀀스 1만 회 무패닉 — CI 게이트 | TERM-130 | U(벤치) |
| 26 | (POSIX 확장) DSR `CSI 6n` 응답·대체 화면 진입/복귀·지연 줄바꿈·DEC 선 그리기 — 신규 테스트 | §4-6 | U |
| 27 | (POSIX) `vim`/`less`/`htop` 실행 후 종료 시 화면 복원·화살표 동작 | §4-6 | M(일부 I) |
| 28 | 서식 복사 붙여넣기: Word/Pages/LibreOffice에 색 유지, 메모장류에 평문 | TERM-090·095 | M |
| 29 | IME: 한글 조합 입력이 PTY에 완성형으로 전달, 조합 창 위치 | TERM-076 | M |
| 30 | 포커스 시각: 터미널 포커스 시 목록 비활성 색·스트립 accent, 창 비활성 시 캐럿 타이머 정지 | TERM-064·103 | R/M |

**핵심 기능 즉시 점검(헬스 체크) 후보**: #14(PTY 스모크) + #5(스킴 스냅샷) + #3(키 라우팅) + #25(벤치)를 묶어 `cargo test -p <term 크레이트>` 1회로 5초 내 판정 가능하게 구성한다(PTY 스모크는 타임아웃 3초·실패 시 셸 경로와 마지막 화면 덤프를 메시지에 포함).
