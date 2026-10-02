# 43 · nexa-sql 기준 구조 — i18n(`nsql-i18n`) · 확장(Extension) 런타임 ↔ nexa-dir2 미리보기 플러그인

> 성격: **이해(인벤토리) 단계 산출물** — 읽기 전용 조사 결과. 이 문서의 `EXT-NNN` ID가 이후 구현·교차 검증의 체크리스트다.
> 조사 기준일 2026-10-03 · 기준 = nexa-sql(작업 트리) · 원본 = nexa-dir2 `0.22.0`(Windows 전용) · 대상 = nexa-dir3(Windows·macOS·Linux).
>
> **경로 약칭**(모든 근거는 `저장소/경로:줄`):
> `i18n/` = `nexa-sql/crates/nsql-i18n/src/` · `ext/` = `nexa-sql/crates/nexa-sql/src/extensions/` · `sql/` = `nexa-sql/crates/nexa-sql/src/` ·
> `set/` = `nexa-sql/crates/nsql-settings/src/` · `sdk/` = `nexa-sql/extensions/sdk/` · `sqldoc/` = `nexa-sql/docs/` ·
> `app/` = `nexa-dir2/crates/nexa-app/src/` · `lang/` = `nexa-dir2/crates/nexa-app/lang/` · `d2doc/` = `nexa-dir2/docs/` · `ui-ctl/` = `nexa-ui/crates/nexa-ctl/src/`.
>
> **ID 대역**: `EXT-001~140` = §1 요소(인벤토리) · `EXT-2NN` = §2 i18n 구성안 · `EXT-3NN` = §3 OS 분기점 · `EXT-4NN` = §4 플러그인 계획 · `EXT-5NN` = 위험·결정 필요.
> **"추정" 표기**: 저장소 코드로 확인하지 못한 것(특히 macOS·Linux 배포 경로, 저장소 주소)은 "추정"을 붙였다.

---

## 0. 범위

### 0-1. 읽은 파일

| 파일 | 줄 | 읽은 범위 |
| --- | ---: | --- |
| `i18n/lib.rs` | 9,718 | 구조 전부(1–330 · 3140–3220 · 9600–9718 직접 읽음 + Grep으로 fn·섹션 주석 전체). `Msg` 본문(약 3,000행 = SQL 전용 문자열)은 **형식 표본만** — 내용은 dir3에 이관 대상이 아님 |
| `i18n/syslang.rs` | 183 | 전부 |
| `ext/mod.rs` · `ext/wasm.rs` · `ext/manager.rs` · `ext/sha256.rs` · `ext/rainbow_pairs.rs` | 517 · 600 · 1,019 · 96 · 129 | 전부 |
| `sql/ext_panel.rs` · `sql/ext_view.rs` | 710 · 512 | 전부 |
| `sql/app/extensions.rs` | 1,272 | 1–760 전부(확장 관리자) · 751–1272는 **외부 파일 변경 추적**(`ext_` = external · 확장과 무관)이라 fn 목록만 |
| `sdk/nexa-ext-sdk/src/lib.rs` | 582 | 전부 (`json.rs` 340줄은 미독 — 값·파서·직렬화, 추정) |
| `nexa-sql/extensions/` | — | `index.json` · `README.md` · `rainbow-pairs/extension.json` · `hello-ext/extension.json` · `sdk/Cargo.toml` · `sdk/.cargo/config.toml` 전부. 샘플 4종 소스는 줄 수·Cargo만 |
| `nexa-sql/scripts/ext-build.ps1` · `ext-build.sh` · `ext-sync-installed.ps1` | 43 · 35 · 45 | 전부 |
| `sqldoc/50` · `68` · `75` | 309 · 149 · 155 | 전부 |
| `sqldoc/22-driver-extensions.md` | 243 | 1–40(요지 · §0 DR-29) |
| `app/preview/wasm.rs` · `app/preview/mod.rs` · `app/i18n.rs` | 844 · 427 · 289 | 전부 |
| `app/preview/archive.rs` · `sample_tests.rs` | 435 · 222 | fn 목록 + 암호 스코프·이름 디코더 구간 |
| `lang/en.lang` · `ko.lang` · `ja.lang` | 543 · 542 · 543 | 키 전수 통계(Grep/집계) + 구간 열람 |
| `d2doc/24` · `25` · `09` | 261 · 48 · 90 | 24·25 전부 · 09는 1–40 |
| `nexa-dir2/scripts/build-plugins.ps1` · `installer/nexa.iss` | 64 · — | 스크립트 전부 · iss는 플러그인 관련 줄 |

### 0-2. 범위 밖(다른 문서 담당)

설정 레지스트리 전체(`nsql-settings`) · 라이선스(`nsql-license`) · 미리보기 창 렌더(`previewwnd.rs`) · 압축 그리드(`archivewnd.rs`) · 압축 리더(`nexa-vfs`)는 접점만 적는다.

---

## 1. 요소 표

### 1-A. nexa-sql i18n (기준 구조)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-001 | i18n 크레이트 형태 | 의존 0 · 전부 `&'static str` 컴파일 타임 표 · 파일 로드/힙 0 | `nexa-sql/crates/nsql-i18n/Cargo.toml:1-14` · `i18n/lib.rs:1-10` | 같은 형태의 `ndir-i18n` 크레이트 신설(EXT-201) |
| EXT-002 | `Lang` 열거 | `En=0`(기본) · `Ko=1` · `ALL`·`code()`·`from_code()`(지역 접미 허용)·`endonym()`·`column()`·`next()` | `i18n/lib.rs:18-72` | `Ja=2` 추가(3열) · `next()`는 3개 순환 |
| EXT-003 | 전역 현재 언어 | `static CURRENT: AtomicU8` · `set_lang`/`current_lang` (프로세스 전역 — 워커에서도 `t()`) | `i18n/lib.rs:74-88` | 그대로. dir2의 UI 스레드 `thread_local`(EXT-028) 대체 |
| EXT-004 | `Msg` 키 열거 | `#[non_exhaustive] enum Msg` · 접두 규약(`Ph`/`Btn`/`St`/`Err`/`Cat`/`Lbl`/`Desc`/`Val`/`Ctx`…) | `i18n/lib.rs:90-3149` | dir2 `.lang` 키에서 **생성**(EXT-203) |
| EXT-005 | `Msg::row()` | `const fn row(self) -> [&'static str; 2]` — `match` 한 줄 = `[영어, 한국어]`. 한국어 칸 빈 값 = 영어 폴백 | `i18n/lib.rs:3151-3155`(본문은 6639 부근까지) | `[&'static str; 3]`(en, ko, ja) |
| EXT-006 | `Msg::ALL` | 전 키 슬라이스(테스트·도구용) | `i18n/lib.rs:6641-9637` | 생성 |
| EXT-007 | 조회 API | `tr(lang, msg)` · `t(msg)` · `tf(msg, &[&str])`(`{0}`…`{n}` 치환, 없는 자리는 그대로) | `i18n/lib.rs:9640-9666` | 시그니처 유지. dir2 `tr(&str) -> String`/`trf`(EXT-029) 호출부 전환 |
| EXT-008 | 단위 테스트 5 | 기본 영어 · 영어 칸 비지 않음 + `{0}`~`{3}` 자리표시자 집합 일치 · 폴백 · 코드 왕복 · `tf` 순서 | `i18n/lib.rs:9668-9718` | 이식 + dir2 파리티 테스트(EXT-034) 흡수 |
| EXT-009 | `system_lang()` | OS 화면 언어 → `Lang`(모르면 `En`) · `OnceLock` 캐시 | `i18n/syslang.rs:15-20` | 그대로 |
| EXT-010 | `lang_from_locale` | `ko_KR.UTF-8`·`ko-KR`·`C`/`POSIX`/빈 값 처리 | `i18n/syslang.rs:22-32` | 그대로(Ja는 `Lang::from_code`에서 처리) |
| EXT-011 | `lang_from_langid` | Windows LANGID 주 언어(하위 10비트) `0x12`=ko · `0x09`=en | `i18n/syslang.rs:34-42` | `0x11`=ja 추가 |
| EXT-012 | `lang_from_env` | gettext 순서 `LANGUAGE`(콜론 첫째) → `LC_ALL` → `LC_MESSAGES` → `LANG` · 첫 "값 있는" 변수로 확정 | `i18n/syslang.rs:44-63` | 그대로 |
| EXT-013 | OS 감지 — Windows | `GetUserDefaultUILanguage`(kernel32 직접 `extern` · crate 0) | `i18n/syslang.rs:65-74` | 채택. **dir2와 원천이 다름**(EXT-030 · EXT-301) |
| EXT-014 | OS 감지 — macOS | `CFLocaleCopyPreferredLanguages` 첫 항목(CoreFoundation 직접 FFI) | `i18n/syslang.rs:76-108` | 그대로 |
| EXT-015 | OS 감지 — Linux 등 | 환경 변수(`lang_from_env`) | `i18n/syslang.rs:110-113` | 그대로 |
| EXT-016 | 설정 연동 `ui.lang` | 레지스트리 항목(`SettingKind::Lang`) · 기본값 = `system_lang()`(`default_of`) · `follows_os_default` · `Settings::lang()` | `set/lib.rs:239` · `719-726` · `5888-5898` · `6754-6758` | 채택 — 사용자가 고르지 않으면 OS 언어, `reset` = 다시 OS(EXT-206) |
| EXT-017 | 부팅·전환 배선 | 부팅 `set_lang(settings.lang())` · `ui.lang` 변경 → `set_lang` + `relabel()` · 단축키 순환 `toggle_lang` | `sql/main.rs:1291` · `sql/app/settings.rs:267-270` · `972-980` · `989-1007` | 같은 흐름. `relabel()` = 메뉴·도크·컨트롤 라벨 재구성(dir2의 "테이블 스왑+메뉴/컬럼 재구성"과 동치) |
| EXT-018 | nexa-ctl 내장 라벨 이음새 | `set_ctl_labels(fn(CtlMsg) -> &'static str)`(1회 주입) · `CtlMsg` 4종(전체 선택/복사/잘라내기/붙여넣기) | `ui-ctl/controls/mod.rs:110-143` · `sql/main.rs:1293-1301` | 부팅 때 주입 필수 — **`&'static str` 반환이 전제**(EXT-204 제약) |
| EXT-019 | 설정 레지스트리 라벨 = `Msg` | `Entry { key, cat: Msg, label: Msg, desc: Msg, kind, default }` · `Choice(&[(&str, Msg)])` | `set/lib.rs:235-279` | dir2 `pref.*` 키(159개)가 그대로 `Msg`로 연결됨(EXT-205) |
| EXT-020 | 확장 표시 이름 2언어 | `Label::Msg(Msg)`(내장) / `Label::Text(en, Option<ko>)`(WASM 메타) | `ext/mod.rs:19-42` | dir3 플러그인 이름은 `nx_meta` 한 줄(언어 무관) — 다국어 라벨은 매니페스트 선택 필드로(EXT-405) |

### 1-B. nexa-dir2 i18n (이관 원본)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-021 | 내장 언어 3종 | `include_str!` — en · ko · ja(전 키 번역) | `app/i18n.rs:13-26` · `lang/en.lang`(543줄) · `ko.lang`(542) · `ja.lang`(543) | **자원 그대로** — `.lang` 3개를 dir3의 원천(SSOT)으로 복사(EXT-202) |
| EXT-022 | `.lang` 파서 | BOM 스킵 · `#` 주석 · `@key = value` 메타(후행 `#` 주석 제거) · 첫 `=` 분리 · 키/값 trim · 중복 = 마지막 승리 · `=` 없는 줄 스킵 | `app/i18n.rs:58-79` | 파서는 `std`만 사용 — 빌드 생성기와 런타임 오버라이드 양쪽에서 재사용 |
| EXT-023 | 값 이스케이프 | `\n` · `\t` · `\\` (그 외는 리터럴 유지) | `app/i18n.rs:35-56` | 그대로(en.lang 기준 `\n` 4줄 · `\t` 1줄) |
| EXT-024 | 사용자 오버라이드 | `data\lang\{code}.lang`을 내장 위에 **키 단위** 덮어쓰기 | `app/i18n.rs:96-106` | 경로만 OS별 설정 폴더로(EXT-207 · EXT-302) |
| EXT-025 | 폴백 체인 | 현재 언어 → en → 키 문자열 그대로 | `app/i18n.rs:81-118` · `180-188` | 유지(오버레이 → 내장 열 → en) · 키 미존재는 컴파일 오류로 바뀜(열거) |
| EXT-026 | 언어 발견 | 내장 3 + `data\lang\*.lang`(파일명 = 코드 · `@name` = 자기 언어 표기) · 코드순 | `app/i18n.rs:120-151` | 유지 여부 = 결정 필요(EXT-501) |
| EXT-027 | 설정값 해석 | `"system"` = OS 언어 1차 서브태그 · 미보유 = `en` | `app/i18n.rs:153-165` | `ui.lang` 미설정(=OS 추종)으로 대체(EXT-016) |
| EXT-028 | 활성 테이블 | `thread_local! ACTIVE: RefCell<Lang>`(UI 스레드 전용) · `activate()` | `app/i18n.rs:167-178` | 전역 원자값(EXT-003)으로 대체 |
| EXT-029 | 조회 API | `tr(key: &str) -> String` · `trf(key, args)` | `app/i18n.rs:180-197` | `t(Msg)` / `tf(Msg, args)`로 치환(EXT-208) |
| EXT-030 | OS 언어 감지 | `GetUserDefaultLocaleName`(사용자 **지역** 로캘) | `app/win.rs:321-330` | `system_lang()`으로 대체 — Windows는 **표시 언어** 기준으로 바뀜(EXT-301) |
| EXT-031 | 언어 메뉴 | 보기 ▸ "언어: 시스템" + 발견된 언어 목록(체크) · 전환 즉시 메뉴/컬럼 재구성 | `app/win.rs:429-434` · `1427-1429` · `5419-5424` · `5575-5577` | 메뉴 배치 유지 — "시스템" = `settings.reset("ui.lang")` |
| EXT-032 | 호출 지점 분포 | `tr`/`trf` 약 390곳·14파일(`win.rs` 232 · `prefs.rs` 60 · `bulkrename.rs` 32 · `preview/archive.rs` 12 · `archivewnd.rs` 10 · `about.rs` 9 · `pwprompt.rs` 8 · `source.rs` 7 · `panel.rs` 6 …) | Grep `\btrf?\(` (nexa-dir2/crates) | 전수 치환 대상. 타 크레이트(`nexa-ops`·`nexa-vfs` 등)는 i18n 미사용 — 앱이 변환(`nexa-dir2/crates/nexa-ops/src/history.rs:6-31`) |
| EXT-033 | 동적 키 조회 | 키를 변수·표로 넘기는 곳 36건(`prefs.rs` 14 · `win.rs` 14 · `bulkrename.rs` 3 · `archivewnd.rs` 2 · 기타 3 — `e.label_key`/`e.desc_key` 설정 표 · `COLS`/`KINDS` 표 · `if a {"x"} else {"y"}` · `res.err_key`) | `app/prefs.rs:208-210` · `339` · `1751` · `app/archivewnd.rs:147,237` · `app/bulkrename.rs:105-110,836,909,949` · `app/win.rs:669,1950,3528,3534,5661,5749` | 표의 필드 형을 `&'static str` → `Msg`로 바꿔야 함(EXT-208) |
| EXT-034 | 단위 테스트 4 | 파서 규칙 · **내장 3언어 키 파리티** · 병합/폴백/발견/해석 · `trf` 자리표 | `app/i18n.rs:199-289` | 파리티 테스트는 생성기 단계의 빌드 실패로 승격 |
| EXT-035 | 문자열 자원 수치 | 고유 키 **498** × 3언어(en·ko·ja 키 집합 동일 · 중복 키 0) · 자리표시자 줄 79(`{0}` 77 · `{1}` 19 · `{2}` 3 · `{3}` 1) · 접두 28종(`pref` 159 · `bulk` 98 · `cloud` 41 · `menu` 38 · `archive` 35 · `ops` 19 · `del` 17 · `info` 11 · `status` 9 · `ctx` 8 · `about` 8 · `tab` 7 · `preview` 7 · `col` 7 …) | `lang/en.lang` 집계 | 498키 = dir3 `Msg` 초기 집합 |

### 1-C. nexa-sql 확장 호스트 API

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-036 | `Command` | `{ id: String, label: Label }` — 팔레트·키맵·메뉴가 같은 id 문자열 | `ext/mod.rs:44-58` | 참고(dir3 플러그인은 명령 기여 없음 — 후속 확장점) |
| EXT-037 | `MenuContribution` | 우클릭 편집 메뉴 서브메뉴 기여(id·라벨·항목) | `ext/mod.rs:60-66` | 참고(dir2 ADR-0004가 "컨텍스트 메뉴 항목으로 일반화 여지"를 남김 — `d2doc/09:5`) |
| EXT-038 | `Extension` 트레이트 | `id`·`settings_prefix`·`commands`·`menus`·`on_settings`·`disabled_effect`·`name`·`run`·`run_with`·`is_wasm`·`as_wasm`·`formatter`·`formatter_marks`·`format` | `ext/mod.rs:68-111` | dir3의 대응물 = `PreviewProvider`(EXT-122). 트레이트 통합은 하지 않음 |
| EXT-039 | `ExtensionEffect` | 설정 반영 결과(`bracket_opts` 한 종류) | `ext/mod.rs:113-117` | 해당 없음(SQL 편집기 전용) |
| EXT-040 | `EditorOps` | 편집기 조작 표면 5메서드 + `TextBox` 구현 | `ext/mod.rs:119-144` | 해당 없음 |
| EXT-041 | `Registry` 골격 | `extensions: Vec<Box<dyn Extension>>` · `loaded_wasm` · `builtin()` · `owner_of` · `active_indices`(같은 id면 WASM 우선) · `ids` | `ext/mod.rs:146-207` | **패턴 채택**: "같은 id = 사용자 것이 동봉/내장을 가린다"(dir2 EXT-126과 같은 규칙) |
| EXT-042 | `Registry::sync_wasm` | 설치 목록 `(id, 경로)`와 비교해 **바뀐 것만** 내림/올림 · 모듈 id ≠ 설치 id면 거부 · 실패 = 안내 줄 + 내장 폴백 · 재시작 불필요 | `ext/mod.rs:209-268` | **채택** — dir2의 "복사 후 재시작"(EXT-126)을 무재시작 재적재로 개선(EXT-409) |
| EXT-043 | 메뉴/설정/명령 디스패치 | `menu_extras` · `on_settings`(바뀐 접두만) · `run_with` | `ext/mod.rs:270-335` | 해당 없음 |
| EXT-044 | 포맷터 API(ABI v1.1) | `formatters` · `formatter_marks` · `format_with`(JSON 요청/응답) | `ext/mod.rs:337-402` | 해당 없음 |
| EXT-045 | WASM 로그 수거 | `take_wasm_notes()` → 호스트 로그 창 | `ext/mod.rs:404-413` | 채택(플러그인 오류·로그를 dir3 로그에) |
| EXT-046 | `wasm_module_path` | 보관 사본 폴더의 첫 `.wasm` | `ext/mod.rs:416-421` | 채택 |
| EXT-047 | 내장 확장 `RainbowPairs` | in-process 구현(정책 층) · WASM 로드 실패 때 폴백 | `ext/rainbow_pairs.rs:11-109` | 해당 없음(구조 선례만 — dir3 내장 공급자가 같은 역할) |
| EXT-048 | 레지스트리 테스트 2 | 소스 트리 `extensions/`에서 설치→로드→대체→삭제→복귀 · 포맷터 확장 E2E | `ext/mod.rs:423-517` | 같은 형태로 dir3 E2E(EXT-420) |

### 1-D. nexa-sql WASM 런타임

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-049 | 런타임 의존 | `wasmi = "1.1"`(워크스페이스) | `nexa-sql/Cargo.toml:79` · `nexa-sql/crates/nexa-sql/Cargo.toml:42` | dir2도 `wasmi = "1.1.0"`(EXT-105) — 버전 동일 계열 |
| EXT-050 | 격리 상수 | `FUEL` 5천만 · `MEM_CAP` 16 MB · `CALL_TIMEOUT_MS` 200 · `FORMAT_FUEL` 2e9 · `FORMAT_TIMEOUT_MS` 5000 · `OUT_CAP` 1 MB · `MODULE_CAP` 8 MB · `BREAKER_LIMIT` 3 | `ext/wasm.rs:20-34` | dir2 수치(EXT-106)를 유지 — nexa-sql 수치로 바꾸지 않는다 |
| EXT-051 | `HostCtx` · `host_guard` | 리미터 · 마감 시각 · op 큐 · 로그 · `doc_path` / 임포트 진입 시 벽시계 검사 + 연료 과금 | `ext/wasm.rs:36-58` | dir2에 같은 구조 존재(EXT-107) |
| EXT-052 | 버퍼 규약 | 4바이트 LE 길이 + UTF-8 · `read_buf`(상한 검사) | `ext/wasm.rs:60-67` | dir2와 동일 규약(EXT-109) |
| EXT-053 | import `nx_editor_op` | 편집기 조작 **요청을 큐에** 모아 호출 뒤 적용(64개 상한) | `ext/wasm.rs:71-83` | 해당 없음 |
| EXT-054 | import `nx_host_get` | 호스트 상황 값(종류 1 = 활성 문서 경로)을 게스트 버퍼에 기록 | `ext/wasm.rs:84-114` | 참고 — dir3에 새 import를 더할 때의 선례(종류 번호 + 길이 접두 버퍼) |
| EXT-055 | import `nx_log` | 로그 한 줄(호출당 32줄 · 512자) | `ext/wasm.rs:115-133` | **신규 import로 채택 검토**(선택적 — 기존 플러그인은 안 쓰므로 호환 · EXT-407) |
| EXT-056 | `WasmExtension::load` | 모듈 크기 상한 → 검증·컴파일 → `nx_ext_meta` 1회 → JSON 파싱 · **`abi == 1` 아니면 거부** · id 필수 | `ext/wasm.rs:251-350` | dir2는 ABI 번호가 없음(4번째 줄 능력 선언) — EXT-108 |
| EXT-057 | 호출 경로 | 호출마다 **새 인스턴스** · 입력은 `nx_alloc(len)` 버퍼에 써서 포인터 전달 · 반환 버퍼 회수 · 로그 수거 | `ext/wasm.rs:356-463` | dir2는 입력 인자 없음(게스트가 import로 당김 — EXT-117) |
| EXT-058 | 서킷 브레이커 | 연속 실패 3회 = 세션 동안 정지 · 성공 시 0 · 실패 사유 `notes`에 | `ext/wasm.rs:352-394` | dir2에 같은 장치(EXT-121) |
| EXT-059 | JSON 도우미 | `jget`/`jstr`/`label_of`/`effect_from_json`/`settings_json` — 의존 = `nsql_settings::json` | `ext/wasm.rs:164-249` | 매니저 이식 때 JSON 파서 출처 필요(EXT-403) |
| EXT-060 | `impl Extension for WasmExtension` | `on_settings`/`disabled_effect`/`run`/`run_with`/`format` | `ext/wasm.rs:466-544` | 해당 없음 |
| EXT-061 | 런타임 테스트 2 | 저장소의 실제 `.wasm` 로드·왕복 · 효과/설정 JSON | `ext/wasm.rs:546-600` | 방식 채택(동봉 산출물 E2E — dir2 `sample_tests.rs`와 같은 패턴) |

### 1-E. nexa-sql 확장 매니저

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-062 | `Trace` | 추적 줄 수집(호스트가 개발자 로그 층으로 내보냄) | `ext/manager.rs:14-23` | 채택 |
| EXT-063 | `NetStat` | curl `-w`(`%{stderr}` 표식 `NSQLW`)로 HTTP 코드·바이트·속도·dns/connect/ttfb/total·IP·최종 URL | `ext/manager.rs:36-97` | 채택(표식 이름만 변경) |
| EXT-064 | 상수 | `FORMAT = 1` · `DEFAULT_REMOTE`(GitHub raw) · `SOURCE_TREE_DIR`(`CARGO_MANIFEST_DIR/../../extensions`) | `ext/manager.rs:99-105` | 원격 주소는 dir3 저장소로(주소 추정 — EXT-503) |
| EXT-065 | `Kind` | `builtin` · `data` · `wasm` · `process` | `ext/manager.rs:107-134` | dir3는 `wasm`만 우선(종류 구분 필요 시 `preview` 추가 — EXT-404) |
| EXT-066 | `index.json` 파싱 | `Summary{id,dir,name,version,kind,summary}` · `format` 검사 · `dir`에 `..`·경로 구분자 금지 | `ext/manager.rs:136-152` · `387-433` | 채택(형식 v1 그대로) |
| EXT-067 | `extension.json` 파싱 | `Meta`(id·name·version·kind·summary·description·author·license·homepage·platforms·requires·settings_prefix·files·messages.install) · `FileSpec{path,sha256,dest,url}` · 경로 안전(`..`·절대·`:` 거부) · `url`은 https만 | `ext/manager.rs:154-181` · `435-489` | 채택. `min_app`은 README에 있으나 파서가 읽지 않음(`nexa-sql/extensions/README.md` 표 vs `parse_meta`) |
| EXT-068 | `Source::{Dir, Url}` | 문자열 → 원천 · `github.com/O/R/tree/B/P` → raw 주소 변환 · `read_traced` | `ext/manager.rs:183-271` | 채택 |
| EXT-069 | 원격 읽기 = `curl` 서브프로세스 | `curl -fsSL --max-time 30 -w …` (외부 crate 0) | `ext/manager.rs:273-309` | 채택하되 **OS 분기점**(EXT-303) |
| EXT-070 | 저장소 목록 | `default_source`(공식 URL 그대로면 소스 트리 폴더 대체) · `user_sources`(쉼표) · `sources`(중복 제거) | `ext/manager.rs:317-352` | 채택 |
| EXT-071 | `platform()` | `windows`/`macos`/`linux` — `platforms[]`에 이 OS가 없으면 설치 거부 | `ext/manager.rs:509-518` · `691-693` | 채택(`.wasm`은 OS 무관이지만 `render_svg` 의존 플러그인 표기에 유용) |
| EXT-072 | 설치 루트 | `root_dir()` = `<설정 폴더>/extensions` · 설정 폴더 = `NSQL_HOME` 또는 `nexa_conf::user_config_dir` | `ext/manager.rs:520-523` · `set/lib.rs:154-161` · `nexa-ui/crates/nexa-conf/src/lib.rs:310-334` | `<설정 폴더>/plugins`로(EXT-401) |
| EXT-073 | 설치 기록 | `installed.json`(`format`·`id`·`name`·`version`·`kind`·`placed[]`) · `installed_in`/`installed_meta` | `ext/manager.rs:525-609` | 채택 |
| EXT-074 | `version_newer` | 점 조각 숫자 비교(업데이트 대상 판정 · 순수 함수) | `ext/manager.rs:611-647` | 채택 |
| EXT-075 | `install_traced` | 메타 1회 GET → id 일치 · platform · `process` 거부 · wasm은 `.wasm` 필수 → 파일마다 GET → **sha256 불일치 = 거부** → `<root>/<id>/<version>/` 보관(SxS) → `dest` 배치 → 메타 사본 → 옛 버전 정리 → `installed.json` | `ext/manager.rs:649-814` | 채택(`dest` 배치는 dir3에 쓸 일이 없으면 비활성) |
| EXT-076 | `remove_traced` | 배치 파일 되감기 + `<root>/<id>` 삭제 | `ext/manager.rs:816-851` | 채택 |
| EXT-077 | `list_toggle` | 쉼표 목록 설정에 항목 넣기/빼기 | `ext/manager.rs:853-869` | 채택(구분자 정책 — EXT-411) |
| EXT-078 | 매니저 테스트 5 | index/meta 파싱·경로 거부 · 설치/삭제 왕복 + 변조 거부 · `NetStat` · `list_toggle`/주소 변환 · `version_newer` | `ext/manager.rs:871-1019` | 이식 |
| EXT-079 | SHA-256 | FIPS 180-4 자체 구현(crate 0) · `digest`/`hex` · 표준 벡터 3 | `ext/sha256.rs:1-96` | **그대로 복사**(순수 · OS 무관) |

### 1-F. nexa-sql 관리 UI · 호스트 배선

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-080 | `ExtRow` / `ExtPanelAction` | 목록 한 줄 모델 · 동작(`Refresh`/`Settings`/`Open`/`Install`/`Update`/`Remove`/`Enable`/`Disable`) | `sql/ext_panel.rs:15-50` | 모델 채택 |
| EXT-081 | `ExtPanel` 동작 | 검색(`FilterBar` — 이름·id·설명) · "설치됨 (n)"/"설치 가능 (n)" · 행 버튼(누름→같은 버튼에서 뗌) · hover · 고속 스크롤(`ScrollAccel`·`SpeedHud`) | `sql/ext_panel.rs:66-386` | 2단계 UI 후보(EXT-415). 의존: `sql/filterbar.rs`·`search_history`(앱 쪽 부품) |
| EXT-082 | `ExtPanel` 그리기 | `DrawCtx` 직접 그림(행 2줄 · 버전/업데이트 강조 · 종류 · 상태 레이어 · 버튼 폭 = 글 폭) | `sql/ext_panel.rs:388-618` | nexa-ui 컨트롤 원칙 부합(OS 컨트롤 0) |
| EXT-083 | 확장 상세 뷰 | `ExtDetail`(필드 표 + 설명) · `ExtView`(머리 + 버튼 + **읽기 전용 `TextBox`** 본문 — 선택·복사·스크롤·우클릭) | `sql/ext_view.rs:17-427` | 2단계 UI 후보 |
| EXT-084 | `apply_extensions` | 설치 목록 → `sync_wasm` → 끈 것/미설치 내장 = 효과 없음 → 효과 적용 → 설정 창 분류 숨김 | `sql/app/extensions.rs:35-96` | 흐름 채택(설치/삭제/켬끔 직후 공급자 재구성) |
| EXT-085 | 관리자 마스터 스위치 | `extensions.enabled = off`면 **모든 확장 정지** | `sql/app/extensions.rs:9-21` · `sqldoc/50:281-288` | dir3는 **네트워크 기능만** 게이트(로컬 플러그인은 기본 동작 — EXT-412) |
| EXT-086 | `run_extension_cmd` | 소유 확장에 위임 + 로그 수거 | `sql/app/extensions.rs:98-117` | 해당 없음 |
| EXT-087 | 팔레트 명령 10종 | `ext.enable_mgr`/`disable_mgr`/`install`/`remove`/`list`/`enable`/`disable`/`repo_add`/`repo_list`/`repo_remove` | `sql/app/extensions.rs:119-287` · `sql/app/menus.rs:524-527,1496-1517` · `sql/keymap.rs:347-438` | dir3에 팔레트가 없으면 설정 창 버튼으로(추정 — dir2에 명령 팔레트 없음은 타 문서 확인) |
| EXT-088 | `ext_pick` | `ext.<verb>:<key>` 실행 · **설치는 라이선스 게이트**(`Feature::DriverExtensions`) | `sql/app/extensions.rs:289-434`(게이트 296) · `sql/app/license.rs:209-219` | 게이트 여부 = dir2 라이선스 정책 계승 판단(EXT-504) |
| EXT-089 | 목록 동기화 | `ext_panel_sync`(설치 기록 + 마지막 카탈로그 · 네트워크 0) · `ext_details_sync` | `sql/app/extensions.rs:436-527` | 채택 |
| EXT-090 | 비동기 인덱스 읽기 | `std::thread` + `mpsc` · 틱에서 수거 · 패널 열 때/⟳에서만 | `sql/app/extensions.rs:529-583` · `sql/main.rs:188-191` | 채택(UI 스레드 차단 금지) |
| EXT-091 | 설정 분류 연결 | `EXTENSION_CATEGORIES`(분류 `Msg` ↔ 확장 id) · `ext_open_settings` | `set/lib.rs:5914-5919` · `sql/app/extensions.rs:602-622` | dir3 플러그인은 자체 설정이 없음 — 후속 |
| EXT-092 | 상세 뷰 탭 열기/복원 | `ext_open_detail` · `ext_reopen_view` | `sql/app/extensions.rs:624-716` | 후속(dir3 탭 = 폴더 탭이라 뷰 탭 개념이 다름) |
| EXT-093 | 추적·저장소 추가 | `ext_trace`(로그 `ext` 층) · `ext_repo_add`(index.json이 읽혀야 등록) | `sql/app/extensions.rs:718-748` | 채택 |
| EXT-094 | 설정 키 4종 | `extensions.enabled`(off) · `extensions.default_repository` · `extensions.repositories` · `extensions.disabled` | `set/lib.rs:1044-1075` | `plugins.*`로 대응(EXT-411) |
| EXT-095 | 확장 문구 `Msg` | 확장 관련 `row` 약 99줄(`Ext*`·`MnExt*`·`StExt*`·`LblExt*`·`DescExt*`·`CatExt*`·`PhExt*`) | `i18n/lib.rs:2649,2876-2914,6131,6355-6395` 등 | 관리자 UI를 들이면 en·ko 문구 재사용 + **ja 번역 신규 필요** |

### 1-G. nexa-sql SDK · 패키지 저장소 · 스크립트

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-096 | 저장소 루트 구조 | `extensions/index.json` + `<패키지>/extension.json` + 파일 · 설치본 `<설정>/extensions/<id>/<version>/` | `nexa-sql/extensions/README.md` · `nexa-sql/extensions/index.json` · `sqldoc/50:156-170` | dir3 `plugins/` 루트에 같은 구조(EXT-402) |
| EXT-097 | SDK 작업 공간 | 앱 작업 공간과 **분리**(`exclude = ["extensions/sdk"]`) · 기본 타깃 `wasm32-unknown-unknown` · release = `opt-level "z"` + `lto` + `codegen-units 1` + `panic abort` + `strip` | `nexa-sql/Cargo.toml:30` · `sdk/Cargo.toml:3-30` · `sdk/.cargo/config.toml:2-3` | 채택 — dir2 샘플은 크레이트마다 독립 `[workspace]`(EXT-137)라 하나로 묶으면 빌드 1회 |
| EXT-098 | `nexa-ext-sdk` | 의존 0 · `Meta`/`Command`/`Menu`/`Label`/`Effect`/`Settings`/`Editor`/`FormatRequest` · `host`(wasm32 import ↔ 호스트 목) · `buf`(make/alloc/read/free) · `export_extension!` | `sdk/nexa-ext-sdk/src/lib.rs:26-489` | **구조 채택**: dir3 `nexa-dir-plugin-sdk`(게스트용 `ret()`·import 선언·호스트 목)를 신설 — 현재 dir2는 SDK 없이 샘플에 복붙(EXT-137) |
| EXT-099 | SDK 테스트 4 | 메타/효과 JSON · 포맷 요청/응답 · 표식 · 버퍼 왕복(호스트 타깃에서 실행) | `sdk/nexa-ext-sdk/src/lib.rs:491-582` | 방식 채택 |
| EXT-100 | 공식 패키지 4종 | `rainbow-pairs` 1.1.1(78,700 B) · `hello-ext` 0.1.1(66,001 B) · `sql-formatter-kiros33` 1.3.1(181,543 B) · `project-explorer-menus` 0.1.0(66,427 B) | `nexa-sql/extensions/index.json` · 각 `extension.json` | 해당 없음(SQL 전용) |
| EXT-101 | `ext-build.ps1` | `cargo build --release`(SDK 폴더) → `.wasm` 복사 → `Get-FileHash` → `extension.json`의 `sha256` 정규식 치환 · `-Only` | `nexa-sql/scripts/ext-build.ps1:1-43` | 채택(EXT-417) |
| EXT-102 | `ext-build.sh` | 같은 일(macOS/Linux) · `sha256sum` 없으면 `shasum -a 256` · JSON 갱신은 **`python3`** 의존 | `nexa-sql/scripts/ext-build.sh:1-35` | 채택하되 주의 2건: ① 패키지 표가 3개뿐(`project_explorer_menus` 누락 — ps1은 4개, `ext-build.ps1:17-22` vs `ext-build.sh:33-35`) = 두 스크립트 표 드리프트 ② `python3` 필요(EXT-304) |
| EXT-103 | `ext-sync-installed.ps1` | 개발 중 패키지를 설치본에 반영(삭제 후 재설치와 같은 배치) · `NSQL_HOME` 또는 `%APPDATA%\nexa-sql` | `nexa-sql/scripts/ext-sync-installed.ps1:1-45` | Windows 전용 스크립트 — dir3는 3-OS판 필요 시 `.sh` 추가 |
| EXT-104 | 문서 결정 | D-87 WASM(`wasmi`) 확정 · D-197 같은 id = WASM이 내장 대체 · D-198 ABI v1 = 버퍼+JSON · D-199 배포 = 저장소 폴더 + Releases 자산(`files[].url`), 둘 다 sha256 필수 · **서명(D-89)·능력 승인(D-90)은 미구현** · 드라이버 확장은 cdylib(DR-29, 설계만) | `sqldoc/75:148-155` · `sqldoc/75:139-146` · `sqldoc/50:93-100` · `sqldoc/22-driver-extensions.md:30-40` · `sqldoc/68:23-27` | dir3 플러그인에 cdylib는 쓰지 않음 · 서명 부재는 위험으로 기록(EXT-505) |

### 1-H. nexa-dir2 미리보기 플러그인 런타임 (이관 원본)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| EXT-105 | 런타임 의존 | `wasmi = "1.1.0"` · 개발 의존 `wat = "1.254.0"`(테스트 모듈 조립) | `nexa-dir2/crates/nexa-app/Cargo.toml:18,28` | 유지 |
| EXT-106 | 격리 상수 | `FUEL` 2억 · `MEM_CAP` 64 MB · `READ_CAP` 256 KB · `OUT_CAP` 1 MB · `READ_AT_CAP` 4 MB · `READ_AT_TOTAL_CAP` 64 MB · `READ_AT_FUEL_FIXED` 1000 · `READ_AT_FUEL_PER_BYTES` 64 · `ARCHIVE_CAP` 50,000 · `CALL_TIMEOUT_MS` 1500 · `BREAKER_LIMIT` 3 · 모듈 8 MB | `app/preview/wasm.rs:34-59` · `327` | **수치 그대로 유지**(A15 회귀 방지 포함) |
| EXT-107 | `HostCtx` · `host_guard` | 대상 파일 경로(이 파일 외 접근 불가) · 리미터 · 마감 · `read_total` / 벽시계 + 연료 과금 | `app/preview/wasm.rs:61-84` | 유지 |
| EXT-108 | 플러그인 메타 | `WasmPlugin{id,name,exts,caps}` · `nx_meta()` 텍스트 **줄 단위**: 1 id · 2 표시명 · 3 확장자 쉼표 목록 · 4 능력(`archive`) | `app/preview/wasm.rs:86-102` · `324-371` | **ABI 유지**(EXT-421) |
| EXT-109 | 버퍼 규약 | 반환 = 선두 4바이트 LE 길이 + UTF-8 본문 · `OUT_CAP` 초과 = 손상 처리 | `app/preview/wasm.rs:104-112` | 유지 |
| EXT-110 | import `env.read_text(ptr, cap) -> len` | 대상 파일 앞부분(256 KB 클램프) · 연료 200,000 | `app/preview/wasm.rs:117-138` | 유지 |
| EXT-111 | import `env.render_svg(sptr, slen, optr, ocap) -> len` | SVG → 래스터 → 임시 BMP **경로**를 게스트 버퍼에 · 실패 0 · 연료 5,000,000 | `app/preview/wasm.rs:139-170` | 시그니처 유지 · **구현은 OS 분기점**(EXT-305) |
| EXT-112 | import `env.is_dark() -> i32` | 테마 신호 | `app/preview/wasm.rs:171-179` | 유지 |
| EXT-113 | import `env.disp_width(ptr, len) -> i32` | 표시 폭(CJK 2칸) | `app/preview/wasm.rs:269-284` · `app/preview/mod.rs:76-95` | 유지(순수 함수) |
| EXT-114 | import `env.file_size() -> i64` | 대상 파일 크기(실패 -1) | `app/preview/wasm.rs:181-191` | 유지 |
| EXT-115 | import `env.read_at(off, ptr, cap) -> n` | 임의 위치 읽기 · 1회 4 MB · 호출당 누적 64 MB 초과 = 트랩 · 연료 = 고정 1000 + 바이트/64 | `app/preview/wasm.rs:192-248` | 유지(`std::fs`만 사용 — OS 무관) |
| EXT-116 | import `env.password(ptr, cap) -> n` | 활성 암호만 1회 복사 · 없으면 -1 · 호스트 임시 사본 즉시 소거 | `app/preview/wasm.rs:249-268` | 유지 |
| EXT-117 | 호출 경로 | 호출마다 새 `Store`/인스턴스 · `fn() -> ptr`(인자 없음) · `nx_alloc` **없음**(ADR 초안에만 있었음 — `d2doc/25:33`) | `app/preview/wasm.rs:288-322` | 유지 |
| EXT-118 | 로더 | `load_one`(8 MB 상한 · 검증·컴파일 · `nx_meta`) · `load_dir`(`*.wasm` **파일명 순** · 오류는 그 파일만 격리) | `app/preview/wasm.rs:324-398` | 유지 + 관리 설치본 경로 추가(EXT-401) |
| EXT-119 | `run_preview` | 첫 줄 `lines`/`image` · 본문 1000줄 × 4096자 클램프 | `app/preview/wasm.rs:400-418` | 유지 |
| EXT-120 | `run_archive`(ABI v2) | 첫 줄 `archive`/`password`/`error` · 머리 `표시명<TAB>플래그`(`solid`·`multivolume`·`truncated`) · 항목 `경로<TAB>원본<TAB>압축<TAB>시각<TAB>속성<TAB>방식`(속성 `dir`·`enc`·`utc`·`unsafe`) · 경로 정규화(`nexa_vfs::archive::normalize_path`) | `app/preview/wasm.rs:420-495` | 유지 |
| EXT-121 | `WasmProvider` + 브레이커 | 능력 선언이면 `run_archive`로 라우팅 · 연속 3회 실패 = 세션 격리 · 오류 = 미리보기 1줄 | `app/preview/wasm.rs:497-550` | 유지 |
| EXT-122 | 공급자 시임 | `PreviewDoc::{Lines, Image, Archive}` · `trait PreviewProvider { id, exts, preview }` | `app/preview/mod.rs:19-39` | 유지(런타임 중립 자산 — `d2doc/25:26-28`) |
| EXT-123 | 내장 공급자 3 | `builtin.archive` → `builtin.image`(`IMAGE_EXTS` 9종 — 경로만 위임) → `builtin.text`(16 KB · 200줄 · 탭 4칸 · 이진 판정) | `app/preview/mod.rs:41-58` · `142-201` · `app/preview/archive.rs:290-314` | 유지. 이미지 디코드는 호스트 소관(OS 분기 — EXT-306) |
| EXT-124 | 공급자 결정 규칙 | **`preview_map` 오버라이드 > 선언 확장자 매치(로드 순) > `builtin.text` 폴백** · `builtin.*`는 사용 안 함 면역 | `app/preview/mod.rs:203-247` | 유지 |
| EXT-125 | 탐색 경로 | ① `data\plugins\`(사용자 설치분) ② `<exe 폴더>\plugins\`(동봉분 · 읽기 전용) — 같은 id는 ①이 이김 | `app/preview/mod.rs:263-282` · `app/config.rs:358-402` | **OS 분기점**(EXT-302) |
| EXT-126 | 지연 로드·캐시 | `thread_local OnceCell` — 미리보기 최초 사용 시 1회 · 이후 재시작 전까지 고정 | `app/preview/mod.rs:284-322` | `sync` 방식으로 교체(EXT-409) |
| EXT-127 | 설정 UI용 메타 | `PluginInfo{id,name,exts}` · `plugin_infos()` | `app/preview/mod.rs:249-261` | 유지(+ 버전·출처·상태 필드 확장) |
| EXT-128 | 설정 키 2종 | `preview_map`(`확장자:id` 세로선 구분 · ≤512) · `plugins_disabled`(id 세로선 구분 · ≤512) | `app/config.rs:144-147` · `292-293` · `462-467` · `706-708` | 키 이관(EXT-411) |
| EXT-129 | 설정 창 "플러그인" 페이지 | 설명 1줄 + (없으면 안내 1줄) + 플러그인당 체크박스 `표시명 (id) — 확장자…` · 해제 = `plugins_disabled` · Win32 `STATIC`/`BUTTON` 직접 생성 | `app/prefs.rs:1786-1843` · `2078-2088` | **배치 유지 · 컨트롤만 nexa-ctl `Checkbox`/라벨로**(EXT-414) |
| EXT-130 | lines 표시 태그 계약 | `\u{2}h1\|`·`h2\|`·`h3\|`·`code\|`·`q\|`·`mono\|`·`hr\|` · `\u{1}img\|<경로>` + `\u{1}pad` × n | `d2doc/24:122-132` · `app/preview/mod.rs:7-9` | 유지(렌더는 미리보기 창 문서 담당) |
| EXT-131 | 호스트 연결 | 도크/독립 창이 `set_dark` 후 `preview_for(path, preview_map, disabled)` 호출 · 설정 변경 즉시 반영 | `app/win.rs:2367-2413` · `2441-2494` · `6685-6686` | 유지 |
| EXT-132 | 암호 스코프 | `pw` 세션 캐시(`thread_local`) · `with_password_scope`/`with_active_password` · `Secret` Drop 소거 | `app/preview/archive.rs:58-145` | 유지(순수) |
| EXT-133 | 구형 zip 이름 디코더 | Windows `MultiByteToWideChar(CP_ACP)` · 비Windows는 빈 함수 | `app/preview/archive.rs:317-341` | **OS 분기점**(EXT-307) |
| EXT-134 | `render_svg_impl` | Windows: `svg::parse` → GDI+ 래스터 → `temp_dir()/nexa-preview/d<해시>.bmp`(32bpp top-down) · 비Windows: `None` | `app/preview/mod.rs:97-140` | **OS 분기점**(EXT-305) |
| EXT-135 | 빌드 스크립트 | `build-plugins.ps1` — 샘플별 `cargo build --release --target wasm32-unknown-unknown` → `dist\` + `-OutDir`(배포 스테이징) · **PowerShell 전용** | `nexa-dir2/scripts/build-plugins.ps1:1-64` | 3-OS 스크립트로(EXT-417) |
| EXT-136 | 설치본 동봉 | Inno Setup이 `{app}\plugins\*.wasm` 배치 | `nexa-dir2/installer/nexa.iss:19-22,73-76` | OS별 패키징에 대응 경로 필요(EXT-302) |
| EXT-137 | 참조 구현 2종 | `markdown-viewer-wasm`(import `read_text`·`render_svg`·`is_dark` · `dist/markdown.wasm` 81,635 B) · `archive-viewer-wasm`(import `file_size`·`read_at` · `dist/archive.wasm` 31,218 B) · 각자 독립 `[workspace]` | `nexa-dir2/samples/markdown-viewer-wasm/src/lib.rs:12-16,52-58` · `nexa-dir2/samples/archive-viewer-wasm/src/lib.rs:29-32,69-76` · 각 `Cargo.toml` | **자원 그대로** — 두 `.wasm`은 재빌드 없이 dir3에서 그대로 로드돼야 한다(EXT-421 수용 기준) |
| EXT-138 | 테스트 11 | `wasm.rs` 5(능력 라우팅+암호 흐름 · 메타/미리보기/연료/호스트 루프 · 2500멤버 ar · 누적 바이트 상한 · 브레이커) · `mod.rs` 4(라우팅/폴백 · 오버라이드 · 탐색 경로 · 사용 안 함) · `sample_tests.rs` 2(동봉 `dist` E2E) | `app/preview/wasm.rs:552-844` · `app/preview/mod.rs:324-427` · `app/preview/sample_tests.rs:17,154` | **전부 이식** = 회귀 하네스(EXT-420) |
| EXT-139 | 결정 문서 | ADR-0005(Starlark → wasmi · 시임/매핑/격리 유지) · 개발자 가이드 24(ABI·태그·설치 위치·트러블슈팅) | `d2doc/25:18-29` · `d2doc/24` | 가이드를 dir3용으로 개정(경로·빌드 3-OS) |
| EXT-140 | 플러그인 문구 | `preview.plugin.error` · `preview.plugin.disabled` · `pref.cat.plugins` · `pref.plugins.desc` · `pref.plugins.empty` + 런타임 오류는 **한국어 하드코딩**(`"호출 시간 상한…"`·`"연료 소진…"`·`"read_at 누적…"`·`"… export 없음"`·`"반환 버퍼 손상"`·`"모듈 8MB 상한 초과"`·`"nx_meta: id 없음"`·`"알 수 없는 반환 종류"`) | `lang/en.lang:174-179` · `app/preview/wasm.rs:75-77,81,222-225,316,320-321,326-328,346,416,448` | 문구는 `Msg`로 · 하드코딩 오류는 i18n 또는 영어 고정(EXT-209) |

---

## 2. i18n

### 2-1. 두 방식 비교

| 항목 | nexa-sql `nsql-i18n` | nexa-dir2 `app/i18n.rs` | dir3 방향 |
| --- | --- | --- | --- |
| 키 | `enum Msg`(컴파일 타임 검사) | 문자열 키(`"menu.file"` — 오타는 런타임에 키가 그대로 표시) | `enum Msg` |
| 저장 | Rust 소스 `match` 표 | 외부 `.lang`(properties) `include_str!` | `.lang` = 원천, 표 = 생성물 |
| 언어 | en · ko | en · ko · ja + 사용자 추가 | en · ko · ja (+ 추가 언어는 결정 필요) |
| 반환 | `&'static str` | `String`(매 호출 할당) | `&'static str` |
| 현재 언어 | 프로세스 전역 `AtomicU8` | UI 스레드 `thread_local` 테이블 | 전역 |
| 사용자 오버라이드 | 없음 | `data\lang\{code}.lang` 키 단위 | 유지(오버레이 층) |
| OS 감지 | 3-OS(`syslang.rs`) | Windows만 | `syslang.rs` 이식 |
| 설정 | `ui.lang`(미설정 = OS) | `lang=system\|코드` | `ui.lang` |
| 폴백 | 빈 칸 → en | 현재 → en → 키 | 오버레이 → 내장 → en |

### 2-2. dir3 i18n 구성안

| ID | 항목 | 내용 |
| --- | --- | --- |
| EXT-201 | 크레이트 | `crates/ndir-i18n`(의존 0) — `lib.rs`(`Lang`·`Msg`·`t`/`tr`/`tf`/`set_lang`/`current_lang`) + `syslang.rs`(EXT-009~015 이식 · `Ja` 추가) + `langfile.rs`(EXT-022·023 파서 이식). nexa-sql과 **같은 공개 API**라 nexa-sql에서 가져오는 코드(설정 레지스트리·매니저·패널)가 `use` 한 줄만 바꿔 붙는다 |
| EXT-202 | 자원 | `crates/ndir-i18n/lang/en.lang`·`ko.lang`·`ja.lang` = dir2 파일 **그대로 복사**(`@app` 메타만 갱신). 번역 수정은 계속 이 파일에서 한다(Rust 표를 손으로 고치지 않는다) |
| EXT-203 | 표 생성 | `build.rs`(std만 · crate 0)가 3개 `.lang`을 읽어 `OUT_DIR/msg_table.rs`에 `enum Msg` · `row() -> [&'static str; 3]` · `ALL` · `key() -> &'static str`(원 키 — 오버라이드 조회용) 생성. **생성기가 빌드를 실패시키는 조건**: 언어 간 키 집합 불일치(EXT-034 승격) · en 값 비어 있음 · 자리표시자 집합 불일치(EXT-008) · 식별자 충돌. 대안 = 1회 변환 후 손 관리(nexa-sql 방식 그대로) — `.lang` 자원 유지 요구와 어긋나 비권장 |
| EXT-204 | 식별자 규칙 | 키의 `.` 경계를 보존해야 한다: 단순 PascalCase는 **2쌍 충돌**(`pref.termTheme.dark` ↔ `pref.termThemeDark`, `pref.termTheme.light` ↔ `pref.termThemeLight` — `lang/en.lang:344-348`). 제안 = 마디마다 PascalCase 후 `_`로 연결하지 않고 **마디 첫 글자만 대문자화 + 마디 경계가 소문자→대문자 연속이 되는 충돌 쌍만 예외 표**로 처리하거나, 일괄 `Menu_File_NewTab` 형식(`#[allow(non_camel_case_types)]`). 어느 쪽이든 생성기의 충돌 검사가 최종 방어선 |
| EXT-205 | 설정 레지스트리 연결 | dir2 `pref.<x>` → `label`, `pref.<x>.desc` → `desc`, `pref.cat.<x>` → `cat`, 값 라벨(`pref.taPos.tl` 등) → `Choice`의 `Msg`. 현재 dir2 설정 표의 `label_key`/`desc_key: &'static str`(`app/prefs.rs:208-210`)를 `Msg`로 바꾸면 nexa-sql `Entry`(EXT-019)와 형이 맞는다 |
| EXT-206 | 언어 설정 | `ui.lang`(`SettingKind::Lang` · 기본 = `system_lang()`) — dir2의 `lang=system`은 "키 미설정"으로 마이그레이션. 보기 메뉴의 "언어: 시스템"(`menu.view.lang.system`)은 `reset("ui.lang")`, 언어 항목은 `Lang::ALL`의 `endonym()`(dir2 `discover`의 내장 표기 `English`/`한국어`/`日本語`와 동일 — `app/i18n.rs:123-127`) |
| EXT-207 | 사용자 오버라이드 층 | `<설정 폴더>/lang/<code>.lang`을 부팅·언어 전환 때 읽어 `Msg` 인덱스 표(`Vec<Option<&'static str>>`)에 얹는다. `&'static str` 유지를 위해 오버라이드 문자열은 적재 시 1회 `Box::leak`(수십 KB · 전환 빈도 낮음). `t()` = 오버레이 → 내장 열 → en. 모르는 키는 무시 + 로그 1줄 |
| EXT-208 | 호출부 전환 | `tr("a.b")` → `t(Msg::…)`(대부분 `&str`로 바로 쓰이므로 `.to_string()` 제거 가능) · `trf("a.b", &[…])` → `tf(Msg::…, &[…])` · 동적 키 36건(EXT-033)은 표 필드/분기 값을 `Msg`로. 문자열 키가 남는 곳은 0이 목표(남기면 `Msg::from_key(&str) -> Option<Msg>`가 필요해짐) |
| EXT-209 | 런타임 오류 문구 | 플러그인 런타임의 한국어 하드코딩(EXT-140)은 `Msg`(3언어)로 올리거나 영어 고정 + `preview.plugin.error`의 `{1}` 자리에 삽입. 테스트가 문구 일부(`"read_at 누적"`·`"nx_preview"`)를 검사하므로(`app/preview/wasm.rs:809,830`) 테스트는 **오류 종류 열거**로 바꾼다 |
| EXT-210 | nexa-ctl 라벨 주입 | 부팅 직후 `set_ctl_labels`에 `CtlMsg` → `t(Msg::Menu…)` 매핑(dir2 키 `menu.edit.selectAll`/`copy`/`cut`/`paste` 재사용 — `lang/en.lang:20-24`). nexa-ui에 새 컨트롤을 추가하며 내장 문구가 늘면 `CtlMsg`에 항목을 더하는 방식을 따른다(`ui-ctl/controls/mod.rs:110-129`) |
| EXT-211 | 테스트 | `ndir-i18n`: EXT-008 5종(3열로) + 파서(EXT-034 첫째) + 오버레이/폴백 + `syslang` 4종(`i18n/syslang.rs:115-183` · ja 사례 추가). 전역 언어를 바꾸는 테스트는 직렬화 필요(전역 원자값 — nexa-sql `tf_substitutes_in_order`가 `set_lang(En)`을 직접 호출, `i18n/lib.rs:9714-9716`) |

### 2-3. 이관 절차(순서)

1. `.lang` 3개 복사 + 생성기 작성 → 498키 `Msg` 생성 · 충돌/파리티 0 확인(EXT-202·203·204).
2. `syslang.rs` 이식 + `Lang::Ja`(EXT-002·011) → `ui.lang` 레지스트리 항목(EXT-206).
3. 호출부 전환은 파일 단위(dir3는 UI를 nexa-ui로 다시 쓰므로 **새 코드가 처음부터 `t(Msg)`** — dir2 호출 지점 목록 EXT-032는 누락 점검표로 쓴다).
4. 오버라이드 층(EXT-207) + 언어 메뉴(EXT-206) + `relabel` 배선(EXT-017).
5. OS 종속 문구 손질(§2-4).

### 2-4. 자원 결함·주의(이관 시 고칠 것)

| ID | 항목 | 근거 | 조치 |
| --- | --- | --- | --- |
| EXT-212 | `del.lockedMsg`·`del.failMsg`의 `{1}`이 **별도 줄**에 있어 파서가 버린다(`=` 없는 줄 스킵) → 코드는 이름 목록을 `{1}`로 넘기지만 치환될 자리가 없음(목록 미표시 **추정** — 화면 확인 필요) | `lang/en.lang:128-134` · `ko.lang` 같은 구간 · `app/i18n.rs:67-69` · `app/win.rs:3798-3801` | 값 끝에 `\n\n{1}`을 붙여 한 줄로. 생성기의 "en에 `=` 없는 비주석 줄 = 오류" 검사로 재발 방지 |
| EXT-213 | 리터럴로 참조되지 않는 키 61개(`bulk.*` 45 · `pref.taPos.*` 9 · `pref.dlgFontSize*` 2 · `ops.applyAll` · `archive.pw.remember` · `pref.termFont.desc` · `pref.termFontSize` 등) — 동적 조립 또는 미사용 **추정** | Grep: `"키"` 전체 일치가 `app/` 안에 없음 | 열거로 바꾸면 미사용은 `dead_code` 경고로 드러남 — 이관하면서 전수 확인 |
| EXT-214 | OS 종속 문구 | `about.desc = … for Windows`(`lang/en.lang:89`) · `pref.plugins.desc`/`empty`의 `data\plugins\`·"next to the exe"(`178-179`) · `term.fail = … (ConPTY)` · `cloud.err.noClientIdMsg`의 `data\settings.cfg` | 문구를 OS 중립으로 고치거나 경로를 `{0}` 자리표시자로(3언어 동시 수정) |
| EXT-215 | `@app = 0.22.0` 메타 | `lang/*.lang:6-7` | dir3 버전으로 갱신(생성기는 메타 무시) |

---

## 3. 확장 런타임 비교

### 3-1. 구조 비교

| 항목 | nexa-sql 확장(ABI v1/v1.1) | nexa-dir2 미리보기 플러그인(ABI v1/v2) |
| --- | --- | --- |
| 런타임 | `wasmi 1.1` 인터프리터 · 연료 계측 | `wasmi 1.1.0` 인터프리터 · 연료 계측 |
| 확장점 | 편집기 명령·메뉴·설정→효과·SQL 포맷터 | 파일 미리보기(`lines`/`image`) · 압축 목록(`archive`) |
| 발견 | **설치 기록**(`<설정>/extensions/<id>/installed.json`) → `wasm_module_path` | **폴더 스캔**(`data\plugins\*.wasm` → `<exe>\plugins\*.wasm` · 파일명 순) |
| 매니페스트 | `index.json` + `extension.json`(format 1) + 모듈 메타 JSON(`nx_ext_meta`) | 없음 — `nx_meta()` 텍스트 4줄뿐 |
| 무결성 | 파일마다 **sha256 필수**(불일치 = 설치 거부) | 없음(폴더에 넣으면 신뢰) |
| ABI 판별 | 메타 JSON `"abi": 1` | 번호 없음 · 4번째 줄 능력 선언(없으면 v1) |
| 게스트 export | `memory` · `nx_alloc` · `nx_ext_meta` · `nx_ext_settings` · `nx_ext_disabled` · `nx_ext_run` · `nx_ext_format` | `memory` · `nx_meta` · `nx_preview` · `nx_archive` |
| 호스트 import(`env`) | `nx_editor_op` · `nx_host_get` · `nx_log` | `read_text` · `render_svg` · `is_dark` · `disp_width` · `file_size` · `read_at` · `password` |
| 데이터 방향 | 호스트 → 게스트 **입력 버퍼**(`nx_alloc`) + 반환 버퍼 | 게스트가 import로 **당겨 읽음** + 반환 버퍼 |
| 직렬화 | JSON | 줄/TAB 구분 텍스트 + 제어문자 태그 |
| 인스턴스 | 호출마다 새로(상태 없음) | 호출마다 새로(상태 없음) |
| 상한 | 연료 5천만 · 16 MB · 200 ms(포맷 2e9 · 5 s) · 반환 1 MB · 모듈 8 MB | 연료 2억 · 64 MB · 1500 ms · 반환 1 MB · 모듈 8 MB · 읽기 256 KB/4 MB/누적 64 MB |
| 실패 격리 | 브레이커 3회 · `notes` → 로그 창 · 내장 폴백 | 브레이커 3회 · 미리보기 1줄 · 내장 폴백 |
| 로드 시점 | `apply_extensions`(시작·설치·삭제·설정 변경) — 무재시작 | 미리보기 최초 사용 시 1회 — **재시작해야 반영** |
| 같은 id | WASM이 내장을 가림 | 사용자 폴더가 동봉 폴더를 가림 |
| 켬/끔 | `extensions.enabled`(마스터) + `extensions.disabled`(쉼표) | `plugins_disabled`(세로선) · `preview_map`(확장자 강제 지정) |
| 관리 UI | 사이드 패널 + 상세 뷰 탭 + 팔레트 명령 + 설정 분류 | 설정 창 "플러그인" 페이지(체크박스 목록) |
| 배포 | 저장소 폴더(raw URL) 또는 Releases 자산(`files[].url`) | `.wasm` 파일 복사 · 설치기 동봉 |
| 빌드 | SDK 작업 공간 1개 + `ext-build.ps1`/`.sh`(sha256 갱신 포함) | 샘플별 독립 작업 공간 + `build-plugins.ps1`(Windows만) |
| SDK | `nexa-ext-sdk`(트레이트 + `export_extension!` + 호스트 목) | 없음(가이드의 `ret()` 복붙 — `d2doc/24:69-101`) |
| 오류 문구 | 영어 | 한국어 하드코딩 |
| 라이선스 게이트 | 설치 = `Feature::DriverExtensions` | 런타임에 게이트 없음(코드상 미확인 — 라이선스 문서 담당) |

두 런타임은 **같은 선례에서 갈라진 형제**다 — nexa-sql `wasm.rs` 머리말이 "격리(nexa-dir2 ADR-0005 선례)"를 명시하고(`ext/wasm.rs:6`), `host_guard`·`read_buf`·브레이커·새 인스턴스 규칙이 문장 단위로 같다. 차이는 **ABI 표면(export/import 이름과 직렬화)** 과 **그 앞단(발견·검증·관리)** 뿐이다.

### 3-2. OS 분기점

| ID | 분기점 | 현재(dir2/nexa-sql) | dir3에서 정할 것 |
| --- | --- | --- | --- |
| EXT-301 | OS 표시 언어 감지 | dir2 = `GetUserDefaultLocaleName`(지역 로캘) · nexa-sql = Windows `GetUserDefaultUILanguage` / macOS `CFLocaleCopyPreferredLanguages` / Linux 환경 변수 | nexa-sql 방식 채택. Windows에서 "지역 = 한국, 표시 언어 = 영어" 사용자는 **영어**로 바뀐다(동작 변경 — 릴리스 노트 대상) |
| EXT-302 | 플러그인·언어 파일 탐색 경로 | dir2 = `<exe>\data\plugins`(포터블) / `%LOCALAPPDATA%\NexaDir\data\plugins` + `<exe>\plugins`(동봉) | 사용자분 = `<설정 폴더>/plugins`(Windows `%APPDATA%\<앱>` · macOS `~/Library/Application Support/<앱>` · Linux `$XDG_CONFIG_HOME/<앱>` — `nexa-ui/crates/nexa-conf/src/lib.rs:310-334`) · 포터블 판정은 `nexa-conf`의 `dir_writable`/`is_replaced_on_upgrade` 규칙을 따름(`같은 파일:294,348`). 동봉분 = Windows `<exe>\plugins` · macOS `<앱>.app/Contents/Resources/plugins` · Linux `<exe>/../share/<앱>/plugins`(macOS·Linux 경로는 **추정** — 패키징 문서와 합의 필요) |
| EXT-303 | 원격 저장소 읽기 `curl` | nexa-sql은 "3-OS 기본 탑재"를 전제(`ext/manager.rs:6`) | Windows 10+/macOS는 기본 탑재 · Linux 최소 설치에는 없을 수 있음(**추정**) → 없으면 "curl이 필요합니다" 오류 1줄(지금도 `curl: <io 오류>`로 실패는 격리됨 — `ext/manager.rs:275-278`). `%{stderr}` 표식은 curl ≥ 7.63 필요(`같은 파일:54`) |
| EXT-304 | 빌드 스크립트 도구 | sha256: `Get-FileHash`(ps1) / `sha256sum`→`shasum -a 256`(sh) · JSON 갱신: .NET 정규식(ps1) / `python3`(sh) | `.ps1` + `.sh` 한 쌍 유지 · 패키지 표는 **한 파일에서 읽게**(EXT-102의 드리프트 방지) · `python3` 의존을 `sed`/Rust 도우미로 대체 검토 |
| EXT-305 | `render_svg` 구현 | Windows GDI+ 래스터 → BMP · 그 외 `None`(= 다이어그램이 텍스트로 폴백 — `d2doc/24:250`) | nexa-ui(`nexa-gfx`) 소프트웨어 래스터로 단일 구현(3-OS 공통)하면 분기 자체가 사라짐. 임시 파일 위치 `std::env::temp_dir()/nexa-preview`는 OS 무관(`app/preview/mod.rs:113-115`). 미구현 OS에서는 0 반환 = 기존 폴백 계약 그대로 |
| EXT-306 | `PreviewDoc::Image` 디코드 | 호스트 WIC(`IMAGE_EXTS` 9종 — `app/preview/mod.rs:41-44`) | 3-OS 이미지 디코더 필요(미리보기 렌더 문서 담당). 플러그인 ABI는 **경로 문자열**만 주고받으므로 영향 없음. Linux 비UTF-8 경로는 `to_string_lossy`로 손실(`같은 파일:186`) — 위험 표기 |
| EXT-307 | 구형 zip 이름 디코더 | Windows `CP_ACP` · 그 외 미설치 | macOS·Linux에서 CP949/CP932 이름이 깨짐 → 자체 변환표 또는 "UTF-8 아님 = 손실 표시" 중 결정(압축 문서 담당 · 플러그인 ABI 무관) |
| EXT-308 | 설치본 동봉 | Inno Setup `{app}\plugins` | macOS 번들/pkg · Linux deb/rpm/tar 각각의 동봉 규칙(패키징 문서 담당) |
| EXT-309 | 경로 구분자·대소문자 | `load_dir`는 확장자 `wasm` 대소문자 무시(`app/preview/wasm.rs:382-385`) · 정렬은 `PathBuf` 순 | 파일명 순 로드가 OS마다 달라질 수 있음(대소문자 구분 파일 시스템) → 정렬 키를 소문자 파일명으로 고정 |

---

## 4. dir3 플러그인 계획

### 4-1. 원칙

1. **dir2 ABI는 바이트 단위로 유지** — 동봉 `markdown.wasm`·`archive.wasm`이 재빌드 없이 그대로 돈다(EXT-137).
2. **ABI 앞단(탐색·매니페스트·해시·관리·빌드)만 nexa-sql에서 가져온다** — 런타임 코어(`wasm.rs`)는 dir2 것을 옮긴다(이미 3-OS 중립: `std::fs`·`wasmi`만 사용, OS 의존은 `render_svg` 한 곳).
3. 느슨한 `.wasm` 드롭인(폴더에 복사)은 계속 지원한다(dir2 사용자 습관 · 개발 편의). 관리 설치본은 그 위에 얹는다.
4. 네트워크는 사용자 동작으로만(nexa-sql 26 §8 규칙 — `ext/manager.rs:6` · `sqldoc/50:172-188` P-7).

### 4-2. nexa-sql에서 가져올 것

| ID | 대상 | 가져오는 것 | 바꿀 것 |
| --- | --- | --- | --- |
| EXT-401 | 탐색 경로 | 설치 루트 `<설정 폴더>/plugins/<id>/<version>/` + `installed.json`(EXT-072·073) | 로드 순서 = ① 관리 설치본 ② 사용자 드롭인 `<설정 폴더>/plugins/*.wasm` ③ 동봉 `plugins/*.wasm` — **같은 id는 앞이 이김**(dir2 규칙 EXT-125 확장). 드롭인과 설치본이 같은 폴더를 쓰므로 "하위 폴더 = 설치본 · 루트의 `.wasm` = 드롭인"으로 구분 |
| EXT-402 | 매니페스트 | `index.json`/`extension.json` format 1(EXT-066·067·096) | 저장소 루트 = dir3 저장소 `plugins/`(이름은 `extension.json` 유지 또는 `plugin.json` — 결정 EXT-502) |
| EXT-403 | 매니저 코드 | `manager.rs` 전체(EXT-062~078) | 의존 교체 4곳: `nsql_settings::json`(JSON 파서) · `nsql_settings::{config_dir, Settings}` · `nsql_core::fmt_bytes` · `nexa_fs::path::display` → dir3 설정 크레이트/공용 도우미. 상수 `DEFAULT_REMOTE`·`SOURCE_TREE_DIR`·curl 표식 이름 |
| EXT-404 | 종류 | `Kind::Wasm`만 설치 허용 · `process` 거부 | `builtin`/`data` 종류는 dir3에서 쓸 일이 생길 때까지 파서만 유지(거부 메시지) |
| EXT-405 | 매니페스트 선택 필드 | `platforms`(EXT-071) · `messages.install` · `requires`(표시만) | dir3 전용 선택 필드 제안: `exts`(미리보기 확장자 — 설치 **전** 목록에 보여 주기용, 정본은 여전히 `nx_meta`) · `name_i18n{ko,ja}` |
| EXT-406 | 해시 검증 | `sha256.rs`(EXT-079) + 설치 때 불일치 거부(EXT-075) | 추가 제안: 동봉분은 빌드 때 해시 목록을 함께 실어 **로드 시 검증**(손상·바꿔치기 탐지) — 드롭인은 검증 없음을 UI에 "관리되지 않음"으로 표시 |
| EXT-407 | 로그 import | `nx_log`(EXT-055) 패턴 | dir3 import에 `env.log(ptr)`를 **추가**해도 기존 모듈은 영향 없음(호스트가 더 많이 제공하는 것은 호환). 이름은 dir2 관례(접두 없음)에 맞춤 |
| EXT-408 | 로드 로그·추적 | `sync_wasm` 안내 줄 · `Trace` · `take_wasm_notes`(EXT-042·045·062) | dir3 로그 창/상태줄에 연결 |
| EXT-409 | 무재시작 재적재 | `Registry::sync_wasm`(EXT-042) | dir2의 `thread_local OnceCell`(EXT-126)을 "공급자 목록 + 로드된 `(id, 경로)`" 상태로 바꾸고 설치/삭제/폴더 새로고침 때 `sync` |
| EXT-410 | 업데이트 판정 | `version_newer`(EXT-074) + 패널의 `latest` 표시(EXT-080) | 드롭인은 버전이 없음 → 업데이트 대상 아님 |
| EXT-411 | 설정 키 | `extensions.*` 4종(EXT-094) | `plugins.disabled`(← `plugins_disabled`) · `preview.map`(← `preview_map`) · `plugins.default_repository` · `plugins.repositories` · `plugins.manager_enabled`(네트워크 게이트 · 기본 off). 구 키는 nexa-sql의 키 개명 표 방식(`set/lib.rs:44-59`)으로 마이그레이션. 구분자: 기존 세로선 값을 읽을 수 있어야 함(id에 쉼표·세로선 금지 규칙 명문화) |
| EXT-412 | 마스터 스위치 범위 | `extensions.enabled`(EXT-085) | dir3는 **저장소 조회·설치만** 게이트 — 설치된/동봉/드롭인 플러그인은 스위치와 무관하게 동작(dir2 동작 보존) |
| EXT-413 | 비동기 인덱스 | 스레드 + `mpsc` + 틱 수거(EXT-090) | 그대로 |
| EXT-414 | 관리 UI 1단계 | — | **dir2 배치 유지**: 설정 창 "플러그인" 페이지 = 설명 1줄 + 체크박스 목록(EXT-129)을 nexa-ctl `Checkbox`로. 표시 문자열 `표시명 (id) — 확장자…` 유지 |
| EXT-415 | 관리 UI 2단계 | `ExtPanel` 행 모델·그리기(EXT-080~082) · 상세(EXT-083) | 같은 설정 페이지 하단 또는 별도 창에 "설치됨/설치 가능" 목록 + 설치·업데이트·삭제 버튼. `FilterBar`·`search_history`는 nexa-sql 앱 부품이므로 nexa-ui로 승격하거나 단순 `TextBox` 검색으로 대체(nexa-ui 추가 개발 후보) |
| EXT-416 | SDK | `nexa-ext-sdk` 구조(EXT-098) | `plugins/sdk/`에 게스트 SDK(import 7종 선언 · `ret()` 버퍼 · 태그 상수 · 호스트 목) + 샘플 2종을 한 작업 공간으로 |
| EXT-417 | 3-OS 빌드 스크립트 | `ext-build.ps1`/`.sh`(EXT-101·102) | `scripts/plugin-build.ps1` + `.sh`: 빌드 → `dist` 복사(dir2 `-OutDir`/`-SkipDist` 옵션 유지 — EXT-135) → sha256 → 매니페스트 갱신. 패키지 표 단일화(EXT-304) |
| EXT-418 | 개발 반영 스크립트 | `ext-sync-installed.ps1`(EXT-103) | 선택 — 드롭인 폴더 복사로 충분하면 생략 |

### 4-3. dir2 ABI에서 유지해야 할 것(체크리스트)

| ID | 유지 항목 | 근거 |
| --- | --- | --- |
| EXT-421 | export 이름·시그니처: `memory` · `nx_meta() -> i32` · `nx_preview() -> i32` · `nx_archive() -> i32`(인자 없음 · 반환 = 버퍼 포인터) | `app/preview/wasm.rs:5-19,311-321` |
| EXT-422 | import 모듈명 `env` + 7종 이름·시그니처(EXT-110~116). **하나라도 빠지면 그 import를 쓰는 모듈은 인스턴스화 실패** | `app/preview/wasm.rs:114-286` |
| EXT-423 | 버퍼 규약(4바이트 LE 길이 + UTF-8) · `nx_meta` 줄 형식(4번째 줄 = 능력, 없으면 v1) | `app/preview/wasm.rs:104-112,342-363` |
| EXT-424 | `nx_preview` 반환(`lines`/`image`) · 줄/글자 클램프 · 표시 태그(EXT-130) | `app/preview/wasm.rs:400-418` · `d2doc/24:122-132` |
| EXT-425 | `nx_archive` 반환 형식·속성 어휘·`password` 재호출 흐름 | `app/preview/wasm.rs:420-495` · `d2doc/24:134-179` |
| EXT-426 | 격리 수치 전부(EXT-106)와 과금 규칙(임포트별 연료) | `app/preview/wasm.rs:34-59,122,149,176,186,203,219,255,274` |
| EXT-427 | `id` 영구성(설정 키) · `builtin.*` 면역 · 결정 우선순위(EXT-124) | `app/preview/mod.rs:203-238` · `d2doc/24:112` |
| EXT-428 | 샌드박스 범위: 플러그인은 **미리보기 대상 파일 1개**만 읽는다(경로는 호스트 컨텍스트에만) | `app/preview/wasm.rs:61-68` |
| EXT-429 | 암호 규약: 게스트 메모리에 1회 복사 · 호스트 사본 소거 · 디스크/로그 기록 금지 | `app/preview/wasm.rs:21-23,249-268` |
| EXT-430 | 실패 표시: 해당 플러그인만 미리보기 1줄 · 브레이커 3회 후 격리 안내 | `app/preview/wasm.rs:531-549` |

### 4-4. 모듈 배치안(dir3)

| 위치 | 내용 | 출처 |
| --- | --- | --- |
| `crates/ndir-i18n/` | i18n(§2) | nexa-sql 형태 + dir2 자원 |
| `crates/<앱>/src/preview/{mod,wasm,archive}.rs` | 공급자 시임 · 런타임 · 압축 연결 | dir2 그대로(OS 의존 2곳만 분리: `render_svg_impl`, 이름 디코더) |
| `crates/<앱>/src/plugins/{manager,sha256}.rs` | 저장소·설치·검증 | nexa-sql `ext/manager.rs`·`sha256.rs` |
| 설정 창 플러그인 페이지 | 1단계 체크박스 목록 → 2단계 관리 목록 | dir2 배치 + nexa-sql 패널 모델 |
| `plugins/`(저장소 루트) | `index.json` · `<id>/extension.json` · `<id>/*.wasm` · `sdk/`(SDK + 샘플 소스) | nexa-sql `extensions/` 구조 + dir2 `samples/` |
| `scripts/plugin-build.{ps1,sh}` | 3-OS 빌드 | nexa-sql `ext-build.*` + dir2 `build-plugins.ps1` |

### 4-5. 단계

| ID | 단계 | 내용 | 완료 기준 |
| --- | --- | --- | --- |
| EXT-441 | P1 런타임 이식 | `preview/` 3파일 이식 · 경로 = 3-OS 탐색(EXT-302·401의 ②③) · `render_svg` 미구현 OS는 0 반환 | dir2 테스트 11종(EXT-138)이 3-OS에서 통과 · 동봉 `.wasm` 2개 무수정 로드 |
| EXT-442 | P2 설정·UI 1단계 | `preview.map`·`plugins.disabled` 키 + 설정 창 페이지(EXT-414) + 무재시작 `sync`(EXT-409) | 체크 해제 즉시 내장 폴백 · 폴더에 `.wasm` 추가 후 새로고침으로 반영 |
| EXT-443 | P3 빌드·SDK | `plugins/sdk` 작업 공간 · `plugin-build.ps1`/`.sh` · 매니페스트·`index.json` 작성 | 3-OS에서 같은 sha256의 `.wasm` 산출(재현 빌드 여부는 실측 — 추정) |
| EXT-444 | P4 매니저 | `manager.rs`·`sha256.rs` 이식 · 설치/삭제/업데이트 · UI 2단계(EXT-415) | nexa-sql 매니저 테스트 5종(EXT-078) + 설치→로드→삭제→동봉 복귀 E2E(EXT-048 형태) |
| EXT-445 | P5 `render_svg` 3-OS | nexa-gfx 래스터로 통일(EXT-305) | Mermaid 샘플이 3-OS에서 이미지로 표시 |

### 4-6. 회귀 테스트 하네스(이 범위)

| ID | 점검 | 방법 |
| --- | --- | --- |
| EXT-451 | ABI 고정 | dir2 `dist/markdown.wasm`·`dist/archive.wasm`을 **바이너리 그대로** 저장소에 두고 E2E(`sample_tests.rs` 2종). 호스트가 import를 하나라도 빠뜨리면 즉시 실패 |
| EXT-452 | 격리 | 연료 무한 루프 · 호스트 임포트 루프 · 누적 바이트 상한 · 브레이커(`wasm.rs` 테스트 4종 — WAT 조립, `wat` 개발 의존) |
| EXT-453 | 라우팅 | `preview_map` 오버라이드 · 사용 안 함 · 내장 면역 · 탐색 경로 중복 없음(`mod.rs` 테스트 4종) |
| EXT-454 | 매니저 | 경로 탈출 거부 · sha256 변조 거부 · 설치/삭제 되감기 · 주소 변환 · 버전 비교 |
| EXT-455 | i18n | 생성기 검사(키 파리티·자리표시자·충돌) = 빌드 실패 · `syslang` 순수 함수 테스트 |
| EXT-456 | 기동 점검 | 부팅 로그 1줄: 로드된 플러그인 수 · 실패 수 · 각 실패 사유(dir2는 `load_dir`의 오류 목록을 버린다 — `app/preview/mod.rs:297`의 `_errors`) |

### 4-7. 위험·결정 필요

| ID | 항목 | 내용 |
| --- | --- | --- |
| EXT-501 | 결정: 추가 언어 발견 유지 여부 | dir2는 `data\lang\fr.lang`만 두면 메뉴에 언어가 생긴다(EXT-026). 열거형 `Lang`과 상충 — (가) 오버레이로 "사용자 언어 1개" 슬롯 지원 (나) 내장 3언어 + 키 단위 오버라이드만. 기능 유지 원칙상 (가)가 맞으나 `SettingKind::Lang`·`from_code`가 동적 코드를 받아야 함 |
| EXT-502 | 결정: 매니페스트 파일명·저장소 폴더명 | `extension.json`/`extensions/`(nexa-sql과 코드 공유 극대화) vs `plugin.json`/`plugins/`(dir2 용어 유지). 코드·설정·문구의 용어를 한쪽으로 통일해야 함(nexa-sql은 09-17에 `plugins` → `extensions`로 개명 — `sqldoc/50:158`) |
| EXT-503 | 추정: 기본 저장소 주소 | dir3 원격 저장소의 조직·이름·브랜치 미확인 → `DEFAULT_REMOTE` 확정 필요 |
| EXT-504 | 결정: 플러그인 설치의 라이선스 게이트 | nexa-sql은 설치를 유료 기능으로 막는다(EXT-088). dir2 정책 계승이 원칙이므로 dir2에 해당 게이트가 없으면 넣지 않는다(라이선스 문서와 교차 확인) |
| EXT-505 | 위험: 서명 없음 | 저장소 인덱스·패키지 서명(D-89)은 nexa-sql에서도 미구현. sha256은 **같은 저장소에 있는 값**이라 저장소가 뚫리면 무력 — 샌드박스(대상 파일 1개 읽기 · 네트워크/쓰기 없음)가 실질 방어선. 암호 import가 있는 압축 플러그인은 특히 출처 표시 필요 |
| EXT-506 | 위험: UI 스레드 실행 | dir2 런타임은 UI 스레드에서 최대 1.5 s(`CALL_TIMEOUT_MS`) 동기 실행 · 공급자 캐시가 `thread_local`. dir3에서 미리보기를 워커로 옮기면 `Cell`/`RefCell`/`thread_local`(브레이커 · 암호 스코프 · 다크 신호) 전부 재설계 대상 |
| EXT-507 | 위험: 디버그 빌드 속도 | wasmi가 디버그에서 수 배 느려 벽시계 상한에 먼저 걸림(`app/preview/wasm.rs:293-294`) — nexa-sql은 워크스페이스에서 의존 크레이트만 최적화(`nexa-sql/Cargo.toml:98` 주석). dir3도 `wasmi`를 dev 프로필에서 최적화 대상에 포함 |
| EXT-508 | 위험: 두 ABI 혼동 | 사용자가 nexa-sql 확장 `.wasm`을 dir3에 넣으면 `nx_meta` export 없음으로 로드 실패 — 오류 문구에 "미리보기 플러그인이 아님"을 구분해 표시. 매니저는 `kind`/`exts` 필드로 사전 차단 |
| EXT-509 | 위험: 앱 크기 | wasmi 도입 증가분 nexa-sql 실측 +1.67 MiB(`sqldoc/75:136`). dir2는 이미 포함 — 추가 증가는 매니저(+curl 호출 코드)뿐 |
| EXT-510 | 위험: ja 번역 공백 | nexa-sql에서 가져오는 UI 문구(확장 관리자 약 99줄 — EXT-095)는 en·ko뿐 → ja 신규 번역 없이는 영어 폴백 |
