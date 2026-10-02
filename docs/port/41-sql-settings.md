# 41 · 기준 구조 가이드 — nexa-sql 설정 아키텍처(레지스트리 · settings.conf · 설정 창 · 키맵) → nexa-dir3 차용 명세

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 2026-10-03.
> 목적: 사용자가 "훨씬 잘 만들어졌다 — 그대로 차용"이라고 지정한 nexa-sql 설정 구조를 **dir3가 무엇을 파일째 복사하고 무엇을 dir3 도메인으로 바꿔야 하는지** 구분해 적는다.
> 이 문서의 `SET-NNN` ID는 이후 구현·교차 검증의 체크리스트다. dir2 쪽 설정 키·화면 배치 원장은 [15-dir2-prefs-config.md](15-dir2-prefs-config.md)(`PREFS-NNN`)이며 여기서는 참조만 한다.
>
> **경로 표기 약속**(모든 근거는 `저장소/경로:줄`):
> - `set:X` = `nexa-sql/crates/nsql-settings/src/X` (예: `set:lib.rs:718`)
> - `sql:X` = `nexa-sql/crates/nexa-sql/src/X` (예: `sql:prefs_win.rs:483`)
> - `cli:X` = `nexa-sql/crates/nsql-cli/src/X`
> - `i18n:X` = `nexa-sql/crates/nsql-i18n/src/X`
> - `conf:N` = `nexa-ui/crates/nexa-conf/src/lib.rs:N`
> - `ui:X` = `nexa-ui/crates/X`
> - `sql-doc:X` = `nexa-sql/docs/X`
>
> 확인하지 못한 것은 본문에 **"추정"** 으로 표시했다. 빌드·실행은 하지 않았다(코드 판독만).

---

## 0. 범위

### 0-1. 읽은 파일

| 파일 | 줄 수 | 읽은 범위 |
| --- | --- | --- |
| `set:lib.rs` | 7402 | 엔진부 전부: 1~300 · 690~950 · 5770~6060 · 6140~6260 · 6340~7402(테스트 21건 포함). **표 본문(데이터)** 은 표본 + 집계: `*_OPTS`(300~690)는 윤곽 Grep, `REGISTRY`(718~5784)는 표본 구간(718~950 · 2040~2090 · 2740~2760 · 3440~3610 · 4525~4565 · 4740~4780) + 전수 집계(항목 590 · 접두 49 · 카테고리 34 · 종류별 수), `DEPENDS` 6060~6140 · `ADVANCED` 6260~6340은 데이터 행이라 건수·무결성만 스크립트로 대조 |
| `set:json.rs` | 509 | 전체 |
| `set:perf.rs` | 556 | 전체 |
| `set:projfile.rs` | 156 | 전체 |
| `nexa-sql/crates/nsql-settings/Cargo.toml` | 19 | 전체 |
| `conf`(nexa-conf lib.rs) | 599 | 전체(테스트 14건 포함) |
| `sql:prefs_win.rs` | 2321 | 전체(테스트 2건 포함) |
| `sql:app/settings.rs` | 1013 | 전체 |
| `sql:keymap.rs` | 1799 | 1~130 · 940~1355 + 전 구간 윤곽(Grep). `COMMANDS` 본문(130~940)은 데이터(125건 집계) · 테스트(1355~1799)는 이름만 |
| `sql:keys_win.rs` | 567 | 1~130 · 260~459(그리기 457~567은 미독) |
| `sql:app/event_loop.rs` | (부분) | 920~1210 · 1695~1745 — 설정 창/단축키 창/색 창 액션 처리 |
| `sql:app/windows.rs` | (부분) | 40~140 — 창 기하 기억 |
| `sql:main.rs` | (부분) | 1192~1204 · 1270~1340 · 2850~2880 — 기동 로드 · 외부 열기 |
| `cli:config.rs` | (부분) | 1~260 — `nsql config` |
| `sql-doc:24-settings-and-vscode-analysis.md` | 141 | 전체 |
| `sql-doc:78-settings-reorganization.md` | 142 | 전체 |
| `sql-doc:94-settings-key-naming-and-location.md` | 128 | 전체 |
| `sql-doc:31-indentation-settings.md` | 75 | 전체(요지만 반영) |
| 대조용 | | `ui:nexa-sys/src/lib.rs`(1~60) · `i18n:lib.rs`(1~80) · `i18n:syslang.rs`(14~47) · `nexa-sql/Cargo.toml`(의존 선언) · `nexa-sql/CLAUDE.md`(설정 규칙 줄) · `nexa-dir3/docs/port/15-dir2-prefs-config.md`(76~263 · 295~417) |

### 0-2. 한 줄 요약

**레지스트리(`REGISTRY: &[Entry]`) 하나가 키·종류·기본값·라벨·설명·카테고리의 단일 원천**이고, 파일(`settings.conf`)에는 **기본값과 다른 값만** 적히며, 설정 창·CLI·JSON 내보내기·검색이 전부 이 표에서 생성된다(`set:lib.rs:10-17` · `sql-doc:24-settings-and-vscode-analysis.md:28`). 잠금·숨김·고급·OS별 기본값·이름 바꿈은 `Entry`에 필드를 늘리지 않고 **키를 가리키는 별도 표**(side table)로 둔다.

> 과제 설명의 "레지스트리 정의 매크로"는 **존재하지 않는다**. 레지스트리는 `macro_rules!` 없이 평범한 `const` 구조체 배열이다(`set:lib.rs:718`). Grep `macro_rules` 무결과.

### 0-3. 규모(집계)

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 레지스트리 항목 | 590 | `set:lib.rs:718-5784`(`Entry {` 행 수) |
| 종류 분포 | Bool 184 · Int 172 · Text 131 · Choice 95 · Size 5 · Position 2 · Lang 1 | 〃 |
| 그룹 / 카테고리 | 8 / 34 | `set:lib.rs:5794-5871` |
| 종속(`DEPENDS`) | 94행 | `set:lib.rs:6021-6154` |
| 숨김(`HIDDEN`) / 고급(`ADVANCED`) | 67 / 110 | `set:lib.rs:6165-6234` · `:6247-6362` |
| 성능 프리셋(`PERF`) / 강제(`BOOST`) | 27 / 52 | `set:perf.rs:181-229` · `:236-312` |
| 이름 바꿈(`RENAMED`) / 단위 변환(`RESCALED`) / 옛 기본값(`OLD_DEFAULTS`) | 30 / 6 / 3 | `set:lib.rs:41-84` · `:90-101` · `:31-36` |
| 단축키 명령(`COMMANDS`) / 레지스트리의 `key.*` | 125 / 65(= `key.preset` + 64) | `sql:keymap.rs:61-964` · `set:lib.rs:2745~` |

---

## 1. 구조 요소 표

"dir3 차용 방법" 분류: **복사** = 파일/함수를 그대로(이름만 `nsql`→`ndir`) · **표 교체** = 구조는 그대로, 데이터 표를 dir3 도메인으로 새로 작성 · **개작** = 골격을 따르되 본문을 다시 작성 · **제외** = dir3에 불필요 · **의존** = 형제 저장소 크레이트를 그대로 사용.

### 1-1. 저장 계층 — `nexa-conf`(nexa-ui 소속 · 의존 0)

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-001 | 파일 파서 `parse` | 관용 파싱(실패 없음): `#` 주석·빈 줄·`=` 없는 줄 건너뜀 · 첫 `=`에서 1회 분할 · 키만 trim(값은 trim 안 함) · CRLF 허용 · U+2028/2029 → `\n`/`\r` 복원 · 결과 `Doc { schema, pairs }`(파일 순서 보존) | `conf:44-77` | **의존**(경로 의존 `../nexa-ui/crates/nexa-conf` — `nexa-sql/Cargo.toml:83`) |
| SET-002 | 직렬화 `serialize(known, unknown)` | 첫 줄 `_schema=1` → 아는 키(호출자 순서) → 미지 키 재방출. 같은 키가 양쪽에 있으면 known이 이김. 값 속 개행은 U+2028/2029로 치환(Windows 경로 `C:\new` 보호 — `\n` 이스케이프를 쓰지 않는 이유) | `conf:86-114` | **의존** |
| SET-003 | 스키마 키 `_schema` | 주석이 아닌 실제 키 · `SCHEMA = 1` · 손상 = 0. **nsql-settings는 `doc.schema`를 읽지 않는다**(이주는 키 표 방식 — SET-040~043) | `conf:40` · `:67-68` · `set:lib.rs:6539-6571` | **의존**. dir3도 스키마 분기 대신 키 표 이주를 쓴다 |
| SET-004 | 원자적 쓰기 `write_atomic` | `.{이름}.{pid}.{seq}.tmp` → `sync_all` → 덮어쓰기 rename → (Unix) 부모 폴더 fsync · Unix 모드 `0600` · 실패 시 temp 삭제·원본 보존 · 부모 폴더 자동 생성 | `conf:119-164` | **의존**(OS 분기 = `#[cfg(unix)]` 2곳) |
| SET-005 | 저장 스케줄러 `SaveScheduler` | 변경 = `mark(now)` 플래그만 · 발화 = `조용해진 지 quiet` **또는** `첫 미저장 변경 후 max_delay` · `flush_now()` = 종료 시 강제 수거 · 시계는 호스트 주입 | `conf:174-226` | **의존** — nsql-settings는 **쓰지 않는다**(설정은 변경 즉시 저장). dir3는 **세션 파일**(탭·경로·펼침 — PREFS-051 디바운스) 저장에 사용 |
| SET-006 | `Store` | 파일 1개 = 스토어 1개: 경로·미지 키·직전 저장분(같으면 쓰지 않음)·스케줄러 | `conf:235-281` | **의존**(세션 파일용. 설정 파일은 `Settings`가 직접 `serialize`+`write_atomic` 호출) |
| SET-007 | 사용자 설정 폴더 `user_config_dir(app)` | Windows `%APPDATA%\{app}` · macOS `~/Library/Application Support/{app}` · 그 외 `$XDG_CONFIG_HOME/{app}` 또는 `~/.config/{app}` · 앱 이름은 인자 | `conf:314-334` | **의존**(OS 분기 3갈래) |
| SET-008 | 포터블 판정 보조 `dir_writable` · `is_replaced_on_upgrade` | 프로브 파일(PID+시퀀스) 생성/삭제 · `.app` 번들·Homebrew keg·시스템 프리픽스(`/usr`·`/opt`·`/snap`·`/nix`)면 exe 옆 금지(경로 구성요소만 판정) | `conf:294-308` · `:348-372` | **의존** — nexa-sql은 쓰지 않는다(포터블 미결 `sql-doc:33-distribution-and-packaging.md:11`). dir3는 dir2의 "포터블 우선"(PREFS-040) 계승 여부에 따라 사용(§5-4 결정 1) |

### 1-2. 레지스트리 — `nsql-settings/src/lib.rs`

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-010 | 항목 타입 `Entry` | `{ key, cat: Msg, label: Msg, desc: Msg, kind: SettingKind, default: &'static str }` — 전부 `'static` · `Copy` · 기본값은 **파일 표현과 같은 문자열** | `set:lib.rs:265-279` | **복사**. 단 `Msg` 타입은 dir3 i18n으로 교체(§5-4 결정 3) |
| SET-011 | 종류 `SettingKind`(7) | `Choice(&[(값, 라벨)])` · `Lang` · `Int{min,max}` · `Size{min,max}`(`13`·`13px`·`10pt`) · `Bool` · `Text` · `Position`(3×3) — 컨트롤 형태 + 검증 규칙을 겸한다 | `set:lib.rs:233-263` | **복사**. `Lang`은 dir3의 동적 언어 발견(PREFS-074)과 안 맞아 `Text`+동적 후보로 대체 권장(§5-4 결정 3) |
| SET-012 | 단일 원천 표 `REGISTRY` + `entry(key)` | `pub const REGISTRY: &[Entry]`(590) · 조회는 선형 탐색 · 새 설정 = 여기 한 줄 + 라벨/설명 2줄 | `set:lib.rs:717-5790` | **표 교체**(dir3 키 — PREFS-101~170 변환) · `entry()`는 복사 |
| SET-013 | 후보 표 `*_OPTS` | `const X_OPTS: &[(&str, Msg)]` — 값(소문자)과 라벨 키 쌍 | `set:lib.rs:282-715` · `:5967-6019` | **표 교체** |
| SET-014 | 카테고리 트리 `CATEGORY_TREE` | `&[(그룹 Msg, &[카테고리 Msg])]` — 설정 창 사이드바 · CLI `config list` 머리글의 단일 원천 · `tree_order` · `group_of` | `set:lib.rs:5792-5871` · `:5923-5939` | **표 교체**(dir2 사이드바 PREFS-307 순서로) · 함수 복사 |
| SET-015 | OS별 기본값 `OS_DEFAULTS` + `default_of` | `(키, macOS 값, Linux 값)` · Windows = 레지스트리 `default` · `cfg!(target_os)` 분기 · `ui.lang`만 특례(기본값 = OS 표시 언어, `follows_os_default`) | `set:lib.rs:5873-5912` | **복사** + **표 교체**. dir3의 OS 분기(터미널 셸·글꼴 이름 등) 기본값을 여기에 모은다(§2-4) |
| SET-016 | 종속 잠금 `DEPENDS` + `Dep` | `(자식, 부모, 조건)` · 조건 = `On` / `NotEmpty` / `Eq(값)` · 부모가 조건을 못 채우면 **설정 창에서만 잠금**(값은 유지 · CLI `set`은 그대로) | `set:lib.rs:5941-5963` · `:6021-6163` | **복사** + **표 교체**(dir2 "부모 = fast_scroll" 등 PREFS-144~152) |
| SET-017 | 비노출 `HIDDEN` + `is_hidden` | 키 목록 — 자동 기억 값(창 위치·최근 목록)·구현 상수. `set/get/reset`은 되지만 설정 창·`config list`에서 숨김(`list all`로만) | `set:lib.rs:6165-6240` | **복사** + **표 교체**(`layout.split` · `dock.ratio` · `window.*_pos` 등 "UI 없음" 키) |
| SET-018 | 고급 `ADVANCED` + `is_advanced` | 판정 = HIDDEN ∪ DBMS 그룹 전체 ∪ 표. 설정 창 Advanced 스위치가 꺼지면 숨김 + "N개 숨김" 안내, 켜면 키 이름 강조색 | `set:lib.rs:6242-6371` | **복사** + **표 교체**. `GrpDbms` 특례(`:6370`)는 제거 |
| SET-019 | 읽기 전용 정보 키 `INFO_KEYS` + `is_info` | 저장하지 않는 계산 값(탐지 결과·파생 경로) — 호스트가 채우고 카드는 늘 잠김 · `set` 거부 | `set:lib.rs:5972-5988` · `:6709-6716` | **복사**(구조) + 표는 비워 시작. dir3 후보: 설정 폴더 경로·탐지된 기본 셸·플러그인 폴더(추정 — 필요 시) |
| SET-020 | 확장 분류 표 `EXTENSION_CATEGORIES` | `(카테고리, 확장 id)` — 끈/미설치 확장의 분류를 설정 창이 숨긴다 | `set:lib.rs:5914-5919` | **복사**(구조) — dir3 **플러그인별 설정 분류**에 대응(플러그인 켜짐/꺼짐에 따라 분류 표시) |
| SET-021 | 표시 순서 `display_order` | `(그룹 순, 카테고리 순, 접두 묶음의 첫 등재 순, 등재 순)` — 같은 카테고리 안에서 키 접두끼리 모인다 | `set:lib.rs:6373-6388` | **복사** |
| SET-022 | 검증·정규화 `normalize` / `allowed` / `parse_size` / `size_px` | 저장 전 검증 → 정규화 문자열. Bool = `on/true/1/yes` ↔ `off/false/0/no` → `on`/`off` · Choice/Position = 소문자 일치 · Int = 범위 · Size = px 환산 범위(pt = ×96/72), 입력 단위 보존 · Text = trim만 · 실패 = `None` | `set:lib.rs:6390-6477` | **복사** |
| SET-023 | 오류 `SetError` | `UnknownKey(키)` · `InvalidValue(키, 입력, 허용 설명)` · `Display`는 i18n 메시지 | `set:lib.rs:6481-6500` | **복사**(메시지 키만 교체) |
| SET-024 | 테마 모드 `ThemeMode` | `System/Light/Dark` · `parse`(`auto` = system) · `next()` 순환 · `is_dark(os 판정)`(무선호 = 다크) | `set:lib.rs:165-229` | **복사** |

### 1-3. 값 저장소 — `Settings`

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-030 | 구조 | `{ path, values: BTreeMap<String,String>(사용자 변경분 · 정규화 완료), unknown: Vec<(String,String)>(레지스트리 밖 키 보존) }` | `set:lib.rs:6502-6510` | **복사** |
| SET-031 | 열기 `open_default` / `open` / `from_text` | 파일 없음·손상 = 실패 아님(전부 기본값) · `from_text(path, text)`는 순수(시험용) · 읽을 때: 아는 키 → `normalize` → **기본값과 같으면 버림**(= 변경분만 보유) · 손상 값 = 기본값(그 줄은 다음 저장 때 사라짐) · 미지 키 = `unknown` · 끝에 이주 함수 호출 | `set:lib.rs:6523-6571` | **복사**(이주 호출부만 dir3 것으로) |
| SET-032 | 읽기 `get` / `get_as` / `is_modified` | `get` = 사용자 값 → 기본값(`default_of`) · 옛 키도 `canonical_key`로 통함 · `get_as` = 단위가 바뀐 옛 키로 물으면 옛 단위로 답 · `is_modified` = `values`에 있는가 | `set:lib.rs:6669-6694` | **복사** |
| SET-033 | 쓰기 `set(key, raw)` | 옛 단위 키 변환 → `canonical_key` → 정보 키 거부 → `normalize` → **기본값과 같으면 줄 삭제**, 아니면 삽입 · 반환 = 정규화 값 | `set:lib.rs:6696-6728` | **복사** |
| SET-034 | 초기화 `reset(key)` | `values`에서 제거 · 반환 = 기본값 | `set:lib.rs:6730-6736` | **복사** |
| SET-035 | 저장 `save()` | 폴더 생성 → `serialize(values(키 사전순), unknown)` → `write_atomic` — **디바운스 없음 · 호출 즉시 동기 쓰기** | `set:lib.rs:6738-6749` | **복사** |
| SET-036 | 타입 접근자 | `lang()` · `theme_mode()` · `flag(key)` · `int(key)` · `font_px(key)` — `flag`/`int`는 **실효 값**(`perf.rs` `effective_*`)을 돈다 | `set:lib.rs:6751-6787` · `set:perf.rs:497-510` | **복사** — 단 `effective`의 OS 기본값 누락 결함을 고쳐서(SET-134) |
| SET-037 | 목록 `list` / `list_visible` | `(Entry, 현재 값, 변경 여부)` 전 항목 — 설정 창 스냅샷·CLI | `set:lib.rs:6789-6812` | **복사** |
| SET-038 | 설정 폴더 `config_dir` + 상수 | `NSQL_HOME` 환경 변수가 있으면 그것(시험·개발 격리 · 포터블 훅) · 아니면 `user_config_dir("nexa-sql")` · 파일명 `settings.conf` | `set:lib.rs:24-27` · `:154-161` | **개작**: `APP_DIR`·환경 변수 이름(`NDIR_HOME` 제안) 교체 + 포터블 판정 추가 여부(§5-4 결정 1) |
| SET-039 | 도메인 전용 접근자 | `grid_col_max_chars` · `scroll_speed_params` · `COL_MAX_AUTO_CHARS` | `set:lib.rs:301` · `:417-434` · `:6513-6521` | **제외**(SQL 그리드 전용) |

### 1-4. 이주(마이그레이션)

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-040 | 옛 기본값 표 `OLD_DEFAULTS` | `(키, 옛 기본값)` — 파일에 옛 기본값이 그대로 저장돼 있으면 "사용자가 고른 값이 아님"으로 보고 새 기본값을 따른다 | `set:lib.rs:29-36` · `:6552-6559` | **복사**(빈 표로 시작) |
| SET-041 | 이름 바꿈 표 `RENAMED` + `canonical_key` | `(옛 키, 새 키)` — 읽을 때 `unknown`의 옛 줄을 새 키로 옮김(새 키가 파일에 이미 있으면 옛 값 버림 · 옛 줄은 다음 저장 때 사라짐) · `get/set/reset 옛키`도 새 키로 통함 | `set:lib.rs:38-84` · `:103-113` · `:6597-6620` | **복사**(빈 표로 시작) |
| SET-042 | 단위 변환 표 `RESCALED` | `(옛 키, 새 키, 배수)` — 새 값 = 옛 값 × 배수(소수 허용) · `set(옛키)` = 옛 단위 해석 · `get_as(옛키)` = 옛 단위로 답 | `set:lib.rs:86-146` · `:6612` · `:6698-6706` | **복사**. dir2 `transfer_close_secs`→`_ms`(PREFS-046/170)는 **dir2 가져오기** 변환에서 처리(§5-5) |
| SET-043 | 일회성 이주 함수 | 표로 못 푸는 뜻 변환 3건(`migrate_explorer_refresh` · `migrate_result_tabbar` · `migrate_indent_from_tab`) — `unknown`에서 옛 키를 집어 새 키 값을 계산 | `set:lib.rs:6573-6661` | **제외**(본문) · 패턴만 차용(같은 자리에 dir3 이주 함수) |
| SET-044 | fail-soft · 미지 키 보존 | 손상 값 → 기본값(파일은 건드리지 않음) · 레지스트리에 없는 키 → `unknown` 보존 재방출(구판이 저장해도 신판 키가 살아남는다) | `set:lib.rs:14-16` · `:6547-6565` · `conf:79-80` | **복사** |

### 1-5. 부가 모듈

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-050 | JSON 코덱 `json.rs` | 외부 crate 0의 최소 JSON(`Json` enum · `parse` · `dump`) · 내보내기 `to_json` = 키 `a.b.c` → 객체 계층, Int/Size(숫자형) = 숫자, Bool = true/false, 그 외 문자열, HIDDEN 포함, `_comment` 안내, 사전순 2칸 들여쓰기 · 가져오기 `import_json` = **있는 키만** 적용, 결과 `Import { changed, unknown, invalid }` · `json_path()` = 설정 파일 옆 `settings.json` · `export_json()` | `set:json.rs:1-458` | **복사**(파일째). 바꿀 곳 = `_comment` 문구 1줄(`set:json.rs:365`) |
| SET-051 | 성능 거버너 `perf.rs` — 모드 프리셋 | `perf.mode`(auto/full/balanced/low/custom) ▸ `PERF: &[(키, PerfBinding{domain, full, balanced, low})]` ▸ 개별 키. `Settings::effective(key)` 우선순위 = **강제(boost) > 사용자 값 > 모드 프리셋 > 기본** · `perf_source` · `perf_rows` · `perf_mode_display`(개별 변경 시 custom) | `set:perf.rs:21-229` · `:413-556` | **복사**(메커니즘) + **표 교체**. dir2에는 대응 기능이 없다 — 1차는 표를 비우거나 최소(애니메이션·프레임 상한·아이콘 캐시)로(§5-4 결정 5) |
| SET-052 | 강제값 `BOOST` | `perf.boost = on`이면 표의 키가 사용자 값과 무관하게 강제 + 설정 창 잠금 · 저장값은 불변(끄면 복귀) | `set:perf.rs:231-318` · `:447-457` | 〃 |
| SET-053 | OS 신호 캐시 | `nexa_sys::Signals`(배터리·원격 세션·동작 줄이기·코어 수) 60초 캐시 · 시험용 `set_signals_override` · `animations_enabled()`(auto = OS "동작 줄이기"의 반대) · `perf_hint()` | `set:perf.rs:334-376` · `:512-555` · `ui:nexa-sys/src/lib.rs:23-48` | **복사**(nexa-sys 의존 = `default-features = false` — `nexa-sql/Cargo.toml:88`) |
| SET-054 | 프로젝트 파일 `projfile.rs` | `.nsql-project` 헤더 + 바이너리 블록 나누기/합치기 — 설정과 무관한 SQL 프로젝트 파일 | `set:projfile.rs:1-156` | **제외** |

### 1-6. 설정 창 — `prefs_win.rs`(앱 코드 · nexa-ui 컨트롤 조립)

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-060 | 창 계약 `PrefsWin` ↔ `PrefsAction` | 창은 **레지스트리 스냅샷만** 갖고 파일·적용은 호스트 몫. 반환 액션 = `None` · `Paint` · `Changed{key,value}` · `Reset(key)` · `OpenColors(key)` · `OpenKeys` · `OpenExtSettings(id)` · `Category` · `RefreshInfo` · `BrowseFolder{key,current}` · `EditJson` | `sql:prefs_win.rs:1-5` · `:29-58` · `:112-181` | **복사**(골격). `OpenExtSettings`·`Category`의 미리보기 용도는 제거/재해석 |
| SET-061 | 스냅샷 `refresh(&Settings)` | `s.list()` → `Snap{entry,value,modified}` 전 항목. 값 대체 규칙: 정보 키 = 호스트 `info` 값 · 종속 잠금 중 + 호스트 값 있음 = 그 값 · boost 강제 대상 = 강제값. 이미 카드가 있으면 값만 갱신(포커스 중인 입력란은 건드리지 않음, 읽기 전용 칸은 예외) 후 `apply_deps` | `sql:prefs_win.rs:399-474` | **복사** |
| SET-062 | 카드 생성 `rebuild_cards` (**레지스트리 → 컨트롤**) | 선택(분류 또는 검색) → `display_order` 정렬 → `kind`별 컨트롤: `Bool`→`Switch` · `Choice`/`Lang`→`Combo` · `Position`→`PositionDropdown` · `Int`/`Size`/`Text`→`TextBox` · **호스트가 후보를 준 `Text`→`Combo`**(`dyn_choices`) | `sql:prefs_win.rs:482-621` | **복사**. dir2 배치 재현을 위한 위젯 힌트 표 추가는 §4-6 |
| SET-063 | 검색 | 건초 = `키 + 라벨 + 설명 + 카테고리`(소문자) 부분 일치 · 한글은 자모열 비교(`nsql_core::hangul`) · IME 조합 중 글자까지 즉시 반영 · 숨긴 분류 제외 · 검색 이력(↑/↓ · `prefs.search`) · 트리를 누르면 검색어를 비운다 | `sql:prefs_win.rs:485-516` · `:1114-1141` · `:1373-1418` · `:1435-1443` · `:1470-1473` | **복사**. 의존 모듈 이식 필요: `nsql-core/src/hangul.rs`(231줄) · `sql:search_history.rs`(799줄) |
| SET-064 | 사이드바 트리 | `CATEGORY_TREE`에서 숨긴 분류를 뺀 `vtree` → `TreeModel`(그룹 = branch · 분류 = leaf · 전부 펼침) · 그룹 선택 = 그룹의 전 분류 표시(카드 제목에 `분류 › 라벨`) · `select_category(cat)` = 외부에서 분류 지정 열기 | `sql:prefs_win.rs:202-229` · `:518-540` · `:741-767` · `:1460-1480` | **복사** |
| SET-065 | Advanced 토글 | 하단 `Switch` · 상태는 설정 `ui.prefs_advanced`(HIDDEN)로 영속(`Changed`로 보고) · 꺼짐 = 고급 카드 숨김 + "고급 설정 N개 숨김" 한 줄 · 켜짐 = 키 이름 강조색 | `sql:prefs_win.rs:484` · `:509-533` · `:788-796` · `:1444-1452` · `:1961-1971` · `:2003-2007` · `set:lib.rs:2063-2071` | **복사** |
| SET-066 | 종속 잠금 `apply_deps` | `dependency(key)` 불충족 ∨ boost 강제 대상 ∨ 정보 키 → `locked`. 글자 칸 = 읽기 전용(선택·복사 가능) · 콤보/위치 = 포커스 제거 · 입력 무시 · 밀린 변경 신호 폐기 · 흐리게 덮기(알파 0.6) · 값이 바뀔 때마다 재계산 | `sql:prefs_win.rs:623-650` · `:1276-1284` · `:1494-1505` · `:1664-1682` · `:2052-2061` · `:2105-2107` | **복사** |
| SET-067 | 변경 수거 `collect_changes` | 한 이벤트에 첫 변경 하나만 보고 · 글자 칸은 **즉시 검증**(`normalize` 실패 → 카드에 허용 범위 오류 표시, 저장 안 함) · 기본값이 빈 Text에 빈 값 = 허용(호스트가 `reset`으로 처리) | `sql:prefs_win.rs:1637-1720` · `sql:app/event_loop.rs:1052-1064` | **복사** |
| SET-068 | 보조 버튼(aux) — 키 이름 술어 | `*_color` → [선택…]+색 스와치 · `key.*` → [캡처…] · 폴더 키(하드코딩 목록) → [찾아보기…] · `format.default` → [확장 설정] | `sql:prefs_win.rs:183-199` · `:590-601` · `:1644-1662` · `:2062-2089` | **개작**: 술어 목록을 dir3 키로(폴더 키 · `[편집…]` 순서 편집 3종 PREFS-312) |
| SET-069 | 레이아웃 | 상수 `PAD 12 · TREE_W 230 · SPLIT_W 6 · LEFT_MIN 160 · RIGHT_MIN 320 · SEARCH_H 30 · CTL_H 28 · CARD_GAP 10`(논리 px × 배율) · 왼쪽 열 = 검색(위)+트리 · 드래그 스플리터(hover `IntentFade`) · 오른쪽 = 카드 스크롤 목록 · 하단 줄 = Advanced 스위치 · [JSON 편집] · [닫기] · 기본 창 920×640 | `sql:prefs_win.rs:60-68` · `:811` · `:870-955` · `:1190-1222` | **복사**. dir2 수치(PREFS-301: `CAT_W 180` · 760×560)와 다름 → §5-4 결정 4 |
| SET-070 | 카드 그리기 | 카드 = 둥근 사각형(`panel_bg`) · 1행 라벨(굵게) + 오른쪽 키 이름 + 복사 버튼 · 설명(단어 줄바꿈) · 덧말(`th.ok` 색) · 오류(`th.danger`)/boost 안내 · 컨트롤 줄 `[컨트롤][보조][초기화]` + 오른쪽 끝 "Default: …"(안 들어가면 설명 아래 줄) · 컨트롤 폭 고정(Bool 56 · Choice 260 · Pos 64 · Text 320 · 넓은 Text = 카드 폭) · [초기화]는 변경된 카드에만 · 컨트롤은 목록 안에 온전히 들어올 때만 배치 · 열린 콤보는 맨 마지막 층 | `sql:prefs_win.rs:1722-2224` | **복사** |
| SET-071 | 호스트 주입 훅 | `set_dyn_choices(key, [(값,라벨)])` · `set_hidden_categories` · `set_info`(정보 키 값 + `키#note`) · `set_note(key, 한 줄)` · `set_advanced` · `set_history` · `preset_query` · `set_memo`/`take_last` | `sql:prefs_win.rs:231-248` · `:301-332` · `:389-397` · `:731-739` · `:788-796` · `:856-864` | **복사** — dir3 용도: 언어 목록·터미널 스킴 목록·글꼴 목록 = `dyn_choices` · 플러그인 분류 숨김 = `hidden_categories` |
| SET-072 | 포맷 미리보기 | Format 분류에서 카드 아래 55 %/45 % 분할 읽기 전용 SQL 상자 + B/K 토글 + 복사 버튼 | `sql:prefs_win.rs:145-162` · `:250-299` · `:769-786` · `:894-934` · `:1291-1343` · `:2123-2195` | **제외**(SQL 전용). 터미널 색 스킴 미리보기로 재활용 가능(추정 — 선택) |
| SET-073 | 키 복사 버튼 | 카드 오른쪽 위 글꼴 높이 정사각형(`CopyBtn`) · 클릭 = 키 복사 → ✓ → `ui.copy_feedback_ms` 뒤 원복 · Shift/Ctrl(⌘)+클릭 = 보이는 설정 전부를 `# 분류 › 라벨` + `키=값`(settings.conf 형식)으로 | `sql:prefs_win.rs:1512-1575` · `sql:copybtn.rs`(185줄) | **복사**(`copybtn.rs` 이식) |
| SET-074 | 입력 공통 | 주 조합키 = macOS ⌘ / 그 외 Ctrl · Ctrl/⌘+C/X/V/A/Z/Y → 포커스 텍스트박스 · IME Preedit/Commit · 우클릭 편집 메뉴(모달) · Esc = 콤보 닫기 우선, 아니면 창 닫기 · `Focused(true)` → `RefreshInfo` | `sql:prefs_win.rs:1049-1155` · `:1163-1189` · `:1581-1635` | **복사**(OS 분기 = 주 조합키 1곳 `:1069-1073`) |
| SET-075 | 창 기하 기억 | 닫힐 때 (위치, 크기) → 호스트가 `window.<name>_pos` / `window.<name>_size`(Text · HIDDEN)에 저장 → 열 때 같은 모니터면 복원, 아니면 메인 창 가운데 | `sql:prefs_win.rs:798-864` · `sql:app/windows.rs:70-135` · `set:lib.rs:3479-3575` · `sql:wingeom.rs`(206줄) | **복사**(dir2는 창 위치 미영속 PREFS-057 — nexa-sql 방식 채택) |
| SET-076 | 사건 라우팅 규칙 | 마우스 = 커서 아래 컨트롤에만 · 키 = 포커스 컨트롤에만 · MouseDown마다 포커스는 누른 곳 하나 · 열린 콤보 = 모달(머리 재클릭 = 접기만) · 포커스 텍스트박스는 목록 밖 드래그도 추적 | `sql:prefs_win.rs:1223-1290` · `:1364-1372` · `:1481-1511` | **복사** |
| SET-077 | 애니메이션 틱 | `tick(now_ms)` / `animating()` — 스크롤바·스플리터 페이드·버튼·콤보 hover·텍스트박스·복사 버튼 | `sql:prefs_win.rs:671-729` | **복사** |

### 1-7. 호스트 적용·변경 통지 — `app/settings.rs` · `event_loop.rs`

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-080 | 기동 로드 | `Settings::open_default()` 실패 시 임시 폴더 경로로 폴백(앱은 뜬다) → `set_lang` → 글꼴 → 이후 각 부품에 설정을 직접 주입(`apply_fast_scroll` · `apply_minimap` · `apply_text_render` …) | `sql:main.rs:1283-1291` · `:1784` · `:1808` · `:1867` | **개작**(순서는 dir2 PREFS-054 계승 + 이 패턴) |
| SET-081 | 변경 처리 흐름(통지의 실체) | **옵저버·이벤트 버스 없음.** `PrefsAction::Changed` → `settings.set`(검증) → `persist_settings()`(즉시 저장) → `apply_setting(key)`(false면 상태줄 "재시작 필요") → `prefs_win.refresh` → 다시 그리기. 실패 = `prefs_win.set_error` | `sql:app/event_loop.rs:1038-1131` | **복사**(패턴) |
| SET-082 | 적용 분배 `apply_setting(key) -> bool` | 키 → 동작 `match`를 도메인 조각 6개(`_ui` · `_conn` · `_explorer` · `_grid` · `_editor` · `_misc`)로 나눠 순서대로 물음 · 어느 조각도 모르면 `false` · 맞으면 `layout()`+`redraw()` · 접두 매칭(`k.starts_with("intel.")`)도 사용 | `sql:app/settings.rs:230-246` · `:249-863` | **개작**(본문은 전부 dir3 도메인) — 조각 분할·`bool` 반환 규약만 계승 |
| SET-083 | 저장 `persist_settings` | `settings.save()` 실패 시 상태줄 메시지(앱은 계속) | `sql:app/settings.rs:982-986` | **복사** |
| SET-084 | JSON 편집·감시 | [JSON 편집] → `export_json` → 외부 프로그램(또는 내장 탭)으로 열기 → **연 뒤부터 1초 mtime 폴링** → `import_json` → 바뀐 키만 `apply_setting` + 저장 + 설정 창 갱신 + 상태줄/로그 | `sql:app/settings.rs:8-93` · `sql:main.rs:1192-1204` | **복사**(외부 열기 OS 분기: `cmd /C start` / `open` / `xdg-open`) |
| SET-085 | 테마·언어 즉시 전환 | `apply_theme`(모드 + OS 판정 → 팔레트 · 창 제목줄 테마) · `cycle_theme` · `toggle_lang` · `relabel`(메뉴·툴바·창 문자열 재생성) | `sql:app/settings.rs:950-1007` · `sql:theme.rs`(180줄) | **개작**(dir2 동적 전환 PREFS-078과 합침) |
| SET-086 | 묶음 재적용 | `perf.mode` 변경 → `PERF` 전 키 `apply_setting` · `perf.boost` → `BOOST` 전 키 · `ui.animations` → 페이드 4키 | `sql:app/settings.rs:289-299` · `:733-760` | **복사**(perf 채택 시) |
| SET-087 | 코드발 변경 동기 `prefs_sync` | 코드가 설정을 바꾼 뒤(메뉴 토글 등) 열려 있는 설정 창 스냅샷 갱신 | `sql:app/settings.rs:95-100` | **복사** — dir2의 메뉴/툴바 토글(PREFS-053)이 전부 이 경로를 타야 한다 |
| SET-088 | 폴더 고르기 → 설정 | `BrowseFolder` → 폴더 전용 대화상자 → `settings.set` → 저장 → 적용 → 갱신 | `sql:app/event_loop.rs:941-955` · `:1118-1126` | **복사** |
| SET-089 | 색 창 연동 | `OpenColors(key)` → 색 창(키 모드) → `Changed{target, hex}` → `settings.set` → `apply_setting` → 최근 색 `ui.color_recent` 저장 | `sql:app/event_loop.rs:1086-1096` · `:1158-1200` · `sql:colors_win.rs`(533줄) | **복사**(색 설정 키를 둘 경우) |

### 1-8. 키맵 — `keymap.rs` · `keys_win.rs`

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-090 | 명령 표 `COMMANDS` | `Command { id, label: Msg, win, mac, linux }` — id는 메뉴·툴바·팔레트와 같은 어휘 · OS별 기본 코드 3열 · 표 순서 = 캡처 창 순서 | `sql:keymap.rs:14-23` · `:60-964` | **표 교체**(dir2 명령·단축키) · 구조 복사 |
| SET-091 | 프리셋 `Preset` | 설정 `key.preset` = `auto`/`windows`/`macos`/`linux` · `auto` = 실행 OS | `sql:keymap.rs:25-58` · `set:lib.rs:2744-2751` | **복사** |
| SET-092 | 조합 `Chord` | 코드 문법 `ctrl+shift+p` · `cmd+…` · `f5` · 여러 개는 `\|`로 · 2단은 `,`(`ctrl+k,ctrl+u`) · `none` = 없음. `ctrl`/`cmd`/`primary` = 주 조합키로 정규화 · macOS Control은 `control+…`(단독) 또는 `ctrl+cmd+…` · `display()`(mac = `⌃⌘⌥⇧` 글리프) · `code()`(저장용 · mac이면 `cmd+`) · `from_winit`(논리 키가 비ASCII면 물리 키 이름 폴백 — IME 한글 모드 대응 · 숫자는 물리 이름) | `sql:keymap.rs:980-1229` · `:1243-1250` | **복사**(파일째) |
| SET-093 | 조회 표 `Keymap` | `from_settings`: `key.<id>`가 비면 프리셋 기본 · `lookup` · `is_prefix`/`lookup_seq`(2단) · `code_of`/`display_of` · `conflict` | `sql:keymap.rs:1231-1325` | **복사** |
| SET-094 | 설정 키 `key.<명령 id>` | 레지스트리 `Text` · 기본 `""`(= 플랫폼 기본) · 분류 `CatKeys` · 라벨 = 명령 라벨 재사용 · 설명 = 공통 `DescKey` | `sql:keymap.rs:966-969` · `set:lib.rs:2752-2759` | **복사** — 단 **전 명령 등재 강제**(SET-130) |
| SET-095 | 자동 반복 허용 `repeatable(id)` | 편집·이동·찾기·탭 넘기기만 키 반복 허용, 한 번짜리 동작은 첫 사건만 | `sql:keymap.rs:1336-1347` | **복사**(허용 목록만 dir3로) |
| SET-096 | 단축키 창 `KeysWin` | 명령 목록(라벨 · 현재 조합 · 충돌 배지) + [지정](캡처 모드 · Esc 취소) · [비우기](`none`) · [초기화](전부 기본) · [닫기] → `KeysAction::Changed{id, code}` / `ResetAll` | `sql:keys_win.rs:1-456` | **복사** |
| SET-097 | 키 변경 적용 | `settings.set("key.<id>", code)` → `save` → `Keymap::from_settings` 재조립 → 메뉴 단축키 표기 갱신 → 창 갱신. 설정 창에서 `key.*`가 바뀌어도 같은 재조립 | `sql:app/event_loop.rs:1132-1157` · `sql:app/settings.rs:828-832` | **복사** |

### 1-9. CLI · 규칙 · 미구현 안

| ID | 요소 | 구현 방식 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-100 | CLI `nsql config` | `list [all\|perf]`(트리 순 · 머리글 `[그룹 ▸ 분류]` · 기본값 표식 · 허용 범위 · 설명) · `get` · `set`(검증 후 저장) · `reset` · `export-json [file\|-]` · `import-json` · `path` | `cli:config.rs:1-260` | **개작**(선택). dir3에 CLI가 없어도 **점검 하네스**의 `--config list/get/set` 진입점으로 가치가 있다(§5-6) |
| SET-110 | 키 명명 규칙 | 2레벨 `<접두>.<이름>` 소문자·`_` · 접두 ≤ 9자 · 이름 ≤ 2낱말 · **접두 ↔ 설정 창 카테고리 1:1** 목표 · 의미 직관 > 위치 연계 | `sql-doc:94-settings-key-naming-and-location.md:6-13` · `:57-70` · `set:lib.rs:268` | **규칙 계승**(§2-5) |
| SET-111 | 단위·접미 규칙 | 시간: 기본값 ≤ 10초 = `_ms` · 초과 = `_secs` · 분 = `_min` · 크기 `_mb`/`_kb` · 비율 `_pct` · 색 `_color` · 상한은 `max_<대상>` 또는 `<대상>_max` | `sql-doc:94-settings-key-naming-and-location.md:76-87` · `:113-118` | **규칙 계승** |
| SET-112 | 확장 소유 키 규칙 | `ext.<확장>.<키>` · 매니페스트 `settings_prefix` · 호스트는 그 접두의 설정만 확장에 넘긴다 | `sql-doc:94-settings-key-naming-and-location.md:107-111` | **규칙 계승** — dir3 플러그인(WASM 미리보기) 설정에 적용 |
| SET-113 | "구현 상수는 레지스트리로" 규칙 | 하드코딩 상수 대신 설정 키(자주 안 바꾸면 `HIDDEN`) · 사용자 문자열은 전부 i18n 키 · 설정 키는 `REGISTRY`에만 추가 | `nexa-sql/CLAUDE.md:79` · `:91` | **규칙 계승**(dir3 개발 기준 문서에 반영) |
| SET-114 | 시험 격리 규칙 | 앱 실행 시험 = `NSQL_HOME` 격리 · 단위 테스트는 실제 설정 폴더에 쓰지 않는다(`temp_dir()` 아래 · 폴더를 인자로 받는 패턴) | `nexa-sql/CLAUDE.md:67` · `set:lib.rs:7078-7082` | **규칙 계승** |
| SET-115 | 미구현 안(문서만) | `LAYOUT`(분류 안 순서·절·링크 표) · `FORCES`(일반화된 강제값) · `Dep::Gt`/`Dep::Not` · 프로젝트/문법 계층 설정 — 코드에 없음(Grep `LAYOUT`·`FORCES`·`Dep::Gt` 무결과) | `sql-doc:78-settings-reorganization.md:132-142` · `sql-doc:31-indentation-settings.md:25-32` · `sql-doc:24-settings-and-vscode-analysis.md:89` | **차용하지 않음**(1차). `Dep::Gt(0)`만 dir3에서 필요하면 추가(예: `transfer.close_ms = 0`이면 진행 창 관련 키 잠금) |

### 1-10. 기존 테스트(차용 대상)

| ID | 테스트 | 검증 내용 | 위치 | dir3 차용 방법 |
| --- | --- | --- | --- | --- |
| SET-120 | `registry_defaults_are_valid_and_keys_unique` | 모든 기본값이 자기 `normalize`를 통과 · 키 중복 없음 | `set:lib.rs:7188-7199` | **복사**(핵심 무결성) |
| SET-121 | `os_defaults_are_valid_and_applied` | `OS_DEFAULTS` 키가 레지스트리에 있고 값이 검증 통과 · 이 OS에서 `get`이 그 값 | `set:lib.rs:7171-7186` | **복사** |
| SET-122 | `set_validates_and_saves_only_changes` · `corrupt_value_falls_back_and_unknown_keys_survive` · `defaults_english_and_system` · `lang_follows_os_until_user_picks` | 변경분만 저장 · 기본값 복귀 시 줄 삭제 · 손상 값 폴백 · 미지 키 왕복 보존 | `set:lib.rs:7084-7158` | **복사**(키 이름만 교체) |
| SET-123 | `renamed_keys_migrate_from_old_lines` · `rescaled_keys_migrate_with_unit` | 옛 키 이주 · 옛 키가 레지스트리에 남지 않음 · 배수 검산 | `set:lib.rs:6839-6956` | **복사**(표가 비면 루프만 남김) |
| SET-124 | `advanced_keys_exist` · `display_order_groups_by_category_then_prefix` · `size_units` · `theme_mode_resolution` | 표 무결성 · 접두 묶음 연속성 · 단위 파싱 | `set:lib.rs:6958-7013` · `:7065-7076` · `:7160-7169` | **복사** |
| SET-125 | perf 5건 | 강제값·우선순위·custom 표시·auto 신호·원장 무결성(`full` 열 = 기본값) | `set:lib.rs:7247-7401` | **복사**(perf 채택 시) |
| SET-126 | JSON 2건 | 파서 왕복·유니코드 · 내보내기 계층 · 가져오기 `changed/unknown/invalid` | `set:json.rs:469-508` | **복사** |
| SET-127 | nexa-conf 14건 | 스케줄러(가짜 시계) · 동일 스냅샷 건너뜀 · 미지 키 왕복 · 쓰레기 파싱 · 실패 시 원본 보존 · 개행 왕복 · 역슬래시 경로 · 0600 · 교체형 설치 자리 판정 · 병렬 프로브 | `conf:374-599` | 의존 크레이트 소유(dir3는 재작성 불필요) |
| SET-128 | 설정 창 2건 | 창 없이 `PrefsWin::new()` + `refresh` + 상태 단언: 분류 이동 · 잠금이 컨트롤(읽기 전용)까지 걸림 | `sql:prefs_win.rs:2227-2321` | **복사**(패턴 — 창 로직을 winit 창 없이 시험) |
| SET-129 | 키맵 11건 | IME 한글 물리 키 · 2단 코드 · 프리셋 · 파싱/표시 왕복 · 자동 반복 표 | `sql:keymap.rs:1349-1799`(이름만 확인) | **복사**(엔진 시험) + 명령 표 시험은 dir3 것으로 |

### 1-11. 판독 중 발견한 결함·함정(dir3가 **상속하면 안 되는 것**)

| ID | 현상 | 근거 | dir3 대책 |
| --- | --- | --- | --- |
| SET-130 | **단축키 재지정이 조용히 버려진다**: `COMMANDS` 125개 중 레지스트리에 `key.<id>`가 있는 것은 64개뿐. 나머지 61개(예 `edit.format` · `view.search` · `find.regex`)는 단축키 창에서 지정해도 `settings.set`이 `UnknownKey`를 내고 호출부가 `let _ =`로 무시 | `sql:keymap.rs:61-964` ↔ `set:lib.rs`(Grep 대조) · `sql:app/event_loop.rs:1137` | 시험 `every_command_has_key_entry`(전 명령 id ↔ `key.<id>` 1:1) + `set` 실패를 무시하지 않기 |
| SET-131 | **동적으로 만든 키가 미등재면 조용히 유실**: `window.mem_pos`/`window.mem_size`는 레지스트리에 없는데 호스트가 `format!("window.{name}_pos")`로 저장 시도 → 무시됨 | `sql:app/windows.rs:73-97` ↔ `set:lib.rs:3480-3575` | 창 이름 목록 ↔ 레지스트리 대조 시험 · `debug_assert!(entry(key).is_some())` |
| SET-132 | 표 무결성 시험 공백: `HIDDEN` · `DEPENDS`(자식·부모) · `CATEGORY_TREE`(모든 항목의 분류가 트리에 있는가) · `EXTENSION_CATEGORIES`는 시험이 없다(현재 값은 스크립트 대조로 정상 확인) | `set:lib.rs:6815-7402`(해당 시험 없음) | 표마다 "존재하는 키만" 시험 추가(§5-6) |
| SET-133 | `settings.conf` 외부 변경을 GUI가 감시하지 않는다(`settings.watch_ms` 미구현) — CLI로 바꾼 값은 실행 중 GUI에 반영되지 않는 것으로 **추정**(감시 코드 Grep 무결과). 다중 인스턴스는 각자 메모리의 변경분 전체를 덮어써 **마지막 저장이 이긴다** | `set:perf.rs:235` · `set:lib.rs:6739-6749` | dir3 다중 창/다중 인스턴스 정책을 정한다(§5-4 결정 6) |
| SET-134 | **`effective()`가 OS별 기본값을 건너뛴다**: 변경 안 한 비부하원 키는 `Some(e.default)`(레지스트리 기본)를 돌려주므로, `flag()`/`int()`로 읽으면 `OS_DEFAULTS`가 무시된다(`get()`은 `default_of`를 씀). 예: macOS에서 `flag("ui.text_hint")`는 `on`, `get`은 `off`. 코드 판독 결과이며 실기 확인은 하지 않았다 | `set:perf.rs:461-477` ↔ `set:lib.rs:6671-6676` · `:6770-6787` · `sql:app/settings.rs:140-149` | dir3는 마지막 줄을 `default_of(key)`로 · 시험 `effective_equals_get_for_non_perf_keys`를 OS_DEFAULTS 키로 |
| SET-135 | 저장 시 **주석이 사라진다**(파서는 `#` 줄을 버리고 직렬화는 재방출하지 않음) · 아는 키는 사전순으로 재배열 | `conf:57-59` · `conf:86-114` · `set:lib.rs:6743-6748` | 사용자 문서에 "손으로 단 주석은 보존되지 않음" 명시 |
| SET-136 | 기동 적용과 `apply_setting`이 **두 경로**(기동 = 부품별 직접 주입 · 변경 = `match`) — 한쪽만 고치면 기동값과 변경값의 동작이 갈린다 | `sql:main.rs:1784-1867` ↔ `sql:app/settings.rs:232-863` | dir3는 기동도 "전 키 `apply_setting` 순회 + 재시작 필요 키 목록"으로 통일 검토 |
| SET-137 | BOM 미제거: 첫 줄이 BOM으로 시작하면 `_schema` 키가 미지 키로 남는 것으로 **추정**(파서에 BOM 처리 없음 · 시험 없음) | `conf:53-77` | dir2 `.cfg` 가져오기와 사용자 수기 편집 대비 — 읽기 전 BOM 제거(dir3 쪽 또는 nexa-conf 보강) |

---

## 2. 레지스트리 정의 문법(예시)

### 2-1. 항목 한 줄

매크로는 없다. `const` 배열에 구조체 리터럴을 더한다(`set:lib.rs:717-750`). 아래는 dir3 도메인으로 옮긴 예시다(키 이름은 PREFS-101~170의 "dir3 제안 키" — 확정 아님).

```rust
// ndir-settings/src/registry.rs
use crate::{Entry, SettingKind};
use ndir_i18n::Msg; // 라벨 타입 — §5-4 결정 3

const THEME_OPTS: &[(&str, Msg)] = &[
    ("system", Msg::ValSystem),
    ("light", Msg::ValLight),
    ("dark", Msg::ValDark),
];
const TAB_DBLCLICK_OPTS: &[(&str, Msg)] = &[
    ("close", Msg::ValTabClose),
    ("pin", Msg::ValTabPin),
    ("lock", Msg::ValTabLock),
];

/// ★ 설정 레지스트리 — 단일 원천. 새 설정 = 여기 한 줄 + 라벨/설명 2줄.
pub const REGISTRY: &[Entry] = &[
    // Choice — 드롭다운(또는 위젯 힌트로 라디오 · §4-6)
    Entry {
        key: "ui.theme",
        cat: Msg::CatAppearance,
        label: Msg::LblTheme,
        desc: Msg::DescTheme,
        kind: SettingKind::Choice(THEME_OPTS),
        default: "dark", // dir2 기본 계승(PREFS-101)
    },
    // Bool — 저장 표현은 "on"/"off"
    Entry {
        key: "list.show_hidden",
        cat: Msg::CatListView,
        label: Msg::LblShowHidden,
        desc: Msg::DescShowHidden,
        kind: SettingKind::Bool,
        default: "on",
    },
    // Int — 범위는 검증·오류 문구("80..1000")·CLI 표시의 원천
    Entry {
        key: "term.cols",
        cat: Msg::CatTerminal,
        label: Msg::LblTermCols,
        desc: Msg::DescTermCols,
        kind: SettingKind::Int { min: 80, max: 1000 },
        default: "240",
    },
    // Size — "12" · "12px" · "9pt" 허용(범위는 px 기준)
    Entry {
        key: "term.font_size",
        cat: Msg::CatFonts,
        label: Msg::LblTermFontSize,
        desc: Msg::DescTermFontSize,
        kind: SettingKind::Size { min: 8, max: 32 },
        default: "12",
    },
    // Position — 3×3(값 = POSITIONS 이름 · dir2 정수 6 = "bottom_left")
    Entry {
        key: "typeahead.hud_pos",
        cat: Msg::CatTypeahead,
        label: Msg::LblTypeaheadPos,
        desc: Msg::DescTypeaheadPos,
        kind: SettingKind::Position,
        default: "bottom_left",
    },
    // Text — 자유 글(목록·경로·순서 문자열 등 앱이 해석). OS별 기본값은 OS_DEFAULTS(§2-4)
    Entry {
        key: "term.shell",
        cat: Msg::CatTerminal,
        label: Msg::LblTermShell,
        desc: Msg::DescTermShell,
        kind: SettingKind::Text,
        default: "pwsh", // Windows 기본 = 레지스트리 default
    },
    // Choice 종속 예: 부모 scroll.fast가 꺼지면 잠김(DEPENDS)
    Entry {
        key: "scroll.fast_hud",
        cat: Msg::CatFastScroll,
        label: Msg::LblFastScrollHud,
        desc: Msg::DescFastScrollHud,
        kind: SettingKind::Bool,
        default: "on",
    },
];
```

### 2-2. 종류별 계약

| 종류 | 파일 표현 | 검증(`normalize`) | 설정 창 컨트롤 | JSON 표현 | 근거 |
| --- | --- | --- | --- | --- | --- |
| `Bool` | `on` / `off` | `on true 1 yes` → `on` · `off false 0 no` → `off`(대소문자 무관) | `Switch` | `true`/`false` | `set:lib.rs:6441-6445` · `sql:prefs_win.rs:557-559` · `set:json.rs:330` |
| `Choice(opts)` | 후보 값(소문자) | 소문자 변환 후 후보 일치 | `Combo`(값, 번역 라벨) | 문자열 | `set:lib.rs:6413-6418` · `sql:prefs_win.rs:560-567` |
| `Lang` | 언어 코드 | `Lang::from_code` | `Combo`(endonym) | 문자열 | `set:lib.rs:6419` · `sql:prefs_win.rs:568-578` |
| `Int{min,max}` | 10진 정수 | `i64` 파싱 + 범위 | `TextBox`(즉시 검증·오류 줄) | 숫자 | `set:lib.rs:6420-6423` · `sql:prefs_win.rs:1705-1716` |
| `Size{min,max}` | `13` / `10pt`(입력 단위 보존 · `px` 접미는 떼어 저장) | px 환산(pt×96/72) 범위 | `TextBox` | `13` = 숫자 · `10pt` = 문자열 | `set:lib.rs:6424-6440` · `set:json.rs:319-329` |
| `Text` | 자유 글(trim) | 항상 통과 | `TextBox` · 호스트 후보가 있으면 `Combo` · 키 술어에 따라 보조 버튼 | 문자열 | `set:lib.rs:6446` · `sql:prefs_win.rs:547-556` · `:582-588` |
| `Position` | `top_left` … `bottom_right`(9종) | 소문자 일치 | `PositionDropdown`(컨트롤 코드 `tl…br` ↔ 긴 이름 변환 = `HudPos`) | 문자열 | `set:lib.rs:252-263` · `:6447-6453` · `sql:prefs_win.rs:579-581` · `:1697-1703` |

### 2-3. 곁 표(side table) — `Entry`에 필드를 늘리지 않는 이유와 문법

"레지스트리에 항목을 덧붙이는 다른 작업과 컴파일 충돌이 없도록 별도 표로 둔다"가 명시된 설계 근거다(`set:perf.rs:14`).

```rust
// 종속 잠금: (자식, 부모, 조건)
pub const DEPENDS: &[(&str, &str, Dep)] = &[
    ("scroll.fast_step", "scroll.fast", Dep::On),
    ("scroll.fast_hud", "scroll.fast", Dep::On),
    ("scroll.fast_hud_pos", "scroll.fast_hud", Dep::On),      // 2단 종속
    ("term.theme_dark", "term.theme", Dep::Eq("system")),     // 값 일치(추정 — dir2 동작 확인 뒤 확정)
];

// 비노출: 자동 기억 값·구현 상수(설정 창에 안 보임 · get/set/reset은 된다)
pub const HIDDEN: &[&str] = &["ui.prefs_advanced", "layout.split", "dock.ratio", "dock.split",
    "window.main_pos", "window.main_size", "window.prefs_pos", "window.prefs_size"];

// 고급: Advanced 스위치가 켜져야 보임
pub const ADVANCED: &[&str] = &["scroll.fast_window_ms", "scroll.fast_hud_hold_ms"];

// OS별 기본값: (키, macOS, Linux) — Windows = 레지스트리 default
pub const OS_DEFAULTS: &[(&str, &str, &str)] = &[
    ("term.shell", "", ""),                 // "" = 로그인 셸($SHELL) 자동 — 해석은 터미널 층
    ("term.font_face", "Menlo", "DejaVu Sans Mono"),
    ("ui.font_face", "", ""),               // "" = nexa-font OS 사슬
];

// 카테고리 트리: (그룹, [카테고리]) — dir2 사이드바 순서(PREFS-307)
pub const CATEGORY_TREE: &[(Msg, &[Msg])] = &[
    (Msg::GrpGeneral, &[Msg::CatAppearance, Msg::CatFonts, Msg::CatLang, Msg::CatKeys, Msg::CatWindow]),
    (Msg::GrpFileList, &[Msg::CatListView, Msg::CatTypeahead, Msg::CatFastScroll, Msg::CatCtxMenu, Msg::CatTransfer]),
    (Msg::GrpTabs, &[Msg::CatTabs]),
    (Msg::GrpDock, &[Msg::CatDock, Msg::CatTerminal]),
    (Msg::GrpPlugins, &[Msg::CatPlugins /* + 플러그인별 분류 */]),
];

// 이주(처음에는 빈 표 — 키를 바꿀 때 한 줄)
pub const RENAMED: &[(&str, &str)] = &[];
pub const RESCALED: &[(&str, &str, i64)] = &[];
const OLD_DEFAULTS: &[(&str, &str)] = &[];
```

### 2-4. OS 분기를 레지스트리에 담는 방법(현 구조가 제공하는 3가지)

| 방법 | 쓰임 | 근거 |
| --- | --- | --- |
| `OS_DEFAULTS`(키는 공통 · 기본값만 OS별) | 터미널 기본 셸 · 글꼴 이름 · 텍스트 래스터 옵션 | `set:lib.rs:5873-5912` |
| OS 전용 키를 **전 OS에 등재**하고 다른 OS는 무시 | `gfx.mac_present` · `gfx.linux_backend` · `clipboard.x11_native` — 레지스트리에 `cfg` 분기는 없다(파일을 OS 간에 옮겨도 키가 살아남는다) | `set:lib.rs:4744-4773`(`cfg` Grep = `default_of` 1곳뿐) |
| 실행 환경을 따르는 기본값(`follows_os_default`) | `ui.lang` 기본 = OS 표시 언어 · 사용자가 고르면 기본값과 같아도 저장 | `set:lib.rs:5887-5898` · `:6556-6559` · `:6720-6726` |

### 2-5. 키 명명 규칙(dir3 적용판)

1. 형식 `<접두>.<이름>` — 소문자·숫자·`_`만. 2레벨 고정(플러그인만 `ext.<플러그인>.<키>` 3레벨)(SET-110 · SET-112).
2. **접두 = 설정 창 분류 1:1**을 목표로 한다. 접두는 "사용자의 일"을 말한다(`term.` · `list.` · `transfer.`). 분류를 위해 접두를 늘리지 않는다.
3. 접미로 단위·종류를 말한다: `_ms`(기본값 ≤ 10초) · `_secs` · `_min` · `_mb` · `_kb` · `_pct` · `_color` · 글꼴 `_face`/`_size`.
4. 마스터 스위치는 접두만 또는 `.enabled`/`.visible`(기능 켬 / 보임 구분).
5. 창 기하 = `window.<창 이름>_pos` / `window.<창 이름>_size`(HIDDEN) · 단축키 = `key.<명령 id>` · 라이선스 게이트 = `license.*`(nexa-sql은 `license.gates` 1키 — `set:lib.rs:4531-4538`).
6. 키는 **안정 계약**이다. 바꾸면 `RENAMED`(단위까지 바뀌면 `RESCALED`) 한 줄 + 시험.

### 2-6. 새 설정 추가 절차(체크리스트)

1. `REGISTRY`에 `Entry` 한 줄(§2-1) + i18n 라벨·설명 2줄.
2. 필요하면 곁 표: `DEPENDS`(부모가 있나) · `HIDDEN`/`ADVANCED`(노출 수준) · `OS_DEFAULTS`(OS별 기본값) · `PERF`/`BOOST`(부하원인가).
3. 새 분류면 `CATEGORY_TREE`에 한 줄 + 분류 라벨.
4. 호스트 `apply_setting`에 `match` 한 줄(즉시 반영) — 없으면 "재시작 필요"로 표시된다(SET-081).
5. 기동 적용 경로에도 반영(SET-136).
6. `cargo test -p ndir-settings`(기본값 자기 검증·키 유일·표 무결성이 자동으로 잡는다).

설정 창·검색·CLI·JSON은 **손대지 않는다**(레지스트리에서 생성).

---

## 3. 파일·위치·형식

### 3-1. 위치(OS별)

| OS | nexa-sql 경로 | 근거 |
| --- | --- | --- |
| 공통 우선 | 환경 변수 `NSQL_HOME`(있으면 그 폴더 — 시험·개발 격리, 포터블 훅) | `set:lib.rs:154-161` |
| Windows | `%APPDATA%\nexa-sql\settings.conf` | `conf:315-318` · `set:lib.rs:24-27` |
| macOS | `~/Library/Application Support/nexa-sql/settings.conf` | `conf:319-326` |
| Linux 등 | `$XDG_CONFIG_HOME/nexa-sql/settings.conf` 또는 `~/.config/nexa-sql/settings.conf` | `conf:327-333` |
| 폴더를 모를 때 | `temp_dir()/nexa-sql/settings.conf`(저장은 실패해도 앱은 뜬다) | `sql:main.rs:1284-1290` |

같은 폴더에 사는 것(전부 `config_dir()` 기준): `settings.json`(내보내기 뷰 — `set:json.rs:445-447`) · `license/`(`nexa-sql/crates/nsql-license/src/lib.rs:308`) · `search-history.json`(`sql:search_history.rs:5`) · `workspaces/` · `meta/` · `instance.lock`(`sql:main.rs:1335-1339`) · 금고(nsql-vault — `nexa-sql/crates/nsql-vault/src/lib.rs:92`). **exe 옆 파일 금지**가 nexa-sql 규칙이다(`sql-doc:33-distribution-and-packaging.md:11`).

### 3-2. 형식

```text
_schema=1
list.show_hidden=off
term.cols=160
ui.theme=light
window.main_size=1400,800
future.key=42
```

| 규칙 | 내용 | 근거 |
| --- | --- | --- |
| 인코딩·줄 | UTF-8 · 한 줄 = `key=value` · CRLF 허용 · 쓰기는 `\n` | `conf:19-22` · `conf:56` · `conf:101` |
| 분할 | 첫 `=`에서 1회(`k=v=w` → 값 `v=w`) · 키 trim · 값은 파서가 trim하지 않고 `normalize`가 trim | `conf:60-63` · `conf:493` · `set:lib.rs:6411` |
| 주석 | `#`으로 시작하는 줄 무시 — **저장 시 보존되지 않음**(SET-135) | `conf:57-59` |
| 첫 줄 | `_schema=1`(항상 재방출) | `conf:88-90` |
| 내용 | **기본값과 다른 값만**(VS Code "변경분" 모델) → 기본값을 바꿔도 손대지 않은 항목은 따라온다 | `set:lib.rs:14` · `:6557-6559` · `:6722-6726` |
| 순서 | 아는 키 = 사전순(BTreeMap) → 미지 키(파일에 있던 순서) | `set:lib.rs:6507` · `:6743-6748` |
| 여러 줄 값 | `\n`→U+2028 · `\r`→U+2029로 한 물리 줄 유지 · 읽을 때 복원 | `conf:70-72` · `conf:94-99` |
| 권한 | Unix `0600`(설정에 비밀이 실릴 수 있음) | `conf:140-146` |
| 쓰기 | 원자적(temp → fsync → rename) · 실패 시 원본 보존 | `conf:125-164` |

### 3-3. 값의 층(우선순위 · 위가 이김)

| 층 | 내용 | 근거 |
| --- | --- | --- |
| ① 강제 | `perf.boost = on`일 때 `BOOST` 표의 값(저장값 불변 · 설정 창 잠금) | `set:perf.rs:463-467` |
| ② 사용자 값 | `settings.conf`의 줄(= `Settings.values`) | `set:perf.rs:468-470` |
| ③ 모드 프리셋 | `perf.mode`가 풀린 모드의 `PERF` 값(auto = OS 신호) | `set:perf.rs:471-475` |
| ④ OS별 기본값 | `OS_DEFAULTS` · `ui.lang` = OS 언어 | `set:lib.rs:5892-5912` |
| ⑤ 레지스트리 기본값 | `Entry.default` | `set:lib.rs:278` |

- `get()` = ② → ④ → ⑤ · `effective()`/`flag()`/`int()` = ① → ② → ③ → ⑤(**④ 누락 — SET-134**).
- 워크스페이스/프로젝트 층·언어별 오버라이드·프로필·동기화는 **없다**(`sql-doc:24-settings-and-vscode-analysis.md:89-91`).

### 3-4. 이주 방식

스키마 번호 분기가 아니라 **키 표**로 한다: 옛 키는 레지스트리에서 빠지므로 읽을 때 `unknown`으로 떨어지고, 이주 단계가 그 목록에서 옛 키를 집어 새 키로 옮긴다. 옛 줄은 다음 저장 때 자연히 사라진다(파일을 읽기만 하는 이주 — 되돌리기 쉬움).

| 상황 | 수단 | 근거 |
| --- | --- | --- |
| 기본값만 바뀜 | `OLD_DEFAULTS` 한 줄 | `set:lib.rs:29-36` |
| 키 이름만 바뀜 | `RENAMED` 한 줄(+ CLI/코드의 옛 키 호환 `canonical_key`) | `set:lib.rs:38-84` |
| 단위가 바뀜 | `RESCALED` 한 줄(배수) | `set:lib.rs:86-101` |
| 뜻이 바뀜(값 변환) | 전용 `migrate_*` 함수(`from_text` 끝에서 호출) | `set:lib.rs:6566-6569` · `:6573-6661` |

---

## 4. 설정 창 UI 생성

### 4-1. 데이터 흐름

```text
REGISTRY ──Settings::list()──▶ PrefsWin.snap (전 항목 스냅샷)
                                   │  선택(트리) 또는 검색어 + Advanced + 숨긴 분류
                                   ▼
                              rebuild_cards()  : kind → 컨트롤 · display_order 정렬 · apply_deps(잠금)
                                   │
        사용자 조작 ──handle(WindowEvent)──▶ collect_changes() ──▶ PrefsAction
                                                                     │
호스트: settings.set(검증) → save() → apply_setting(key) → prefs_win.refresh(&settings) → redraw
```

근거: `sql:prefs_win.rs:399-474`(refresh) · `:482-621`(rebuild) · `:1049-1560`(handle) · `:1637-1720`(collect) · `sql:app/event_loop.rs:1038-1131`(호스트).

### 4-2. 화면 구성

```text
┌──────────────── Nexa SQL — Preferences (기본 920×640 · 크기 조절 가능) ────────────────┐
│ ┌검색(지우기 ✕)──┐ ║ (고급 설정 N개 숨김)                                              │
│ └────────────────┘ ║ ┌─ 카드 ────────────────────────────────────────────────────┐   │
│ ┌트리────────────┐ ║ │ [분류 ›] 라벨(굵게)                           키.이름 [⧉] │   │
│ │ ▾ 그룹          │ ║ │ 설명(단어 줄바꿈) · 덧말 · 오류/강제 안내                 │   │
│ │    분류         │ ║ │ [컨트롤][보조 버튼][색 스와치][초기화]      Default: 값  │   │
│ │    분류 ←선택   │ ║ └───────────────────────────────────────────────────────────┘   │
│ └────────────────┘ ║ ┌─ 카드 … ┐                                          스크롤바 ▐ │
│ (Advanced ⬤)                                              [JSON 편집]      [닫기]     │
└──────────────────────────────────────────────────────────────────────────────────────┘
        ↑ 왼쪽 열 폭 = 드래그 스플리터(160 이상 · 오른쪽 320 이상 남김)
```

근거: `sql:prefs_win.rs:60-68` · `:870-955` · `:1746-2224`.

### 4-3. 사용하는 nexa-ui 컨트롤(전부 실재 확인)

| 용도 | 컨트롤 | 위치 |
| --- | --- | --- |
| 검색·값 입력 | `TextBox`(+`with_clearable` · IME preedit · 읽기 전용 · 우클릭 편집 메뉴) | `ui:nexa-ctl/src/controls/textbox.rs:350` |
| 사이드바 | `TreeView` + `TreeModel`/`TreeNode` | `ui:nexa-ctl/src/controls/tree.rs:493` · `:98` |
| on/off | `Switch`(+`LabelSide`) | `ui:nexa-ctl/src/controls/switch.rs:32` · `ui:nexa-ctl/src/controls/mod.rs:216` |
| 후보 선택 | `Combo` + `ComboItem(값, 라벨)` | `ui:nexa-ctl/src/controls/combo.rs:534` · `:44` |
| 3×3 위치 | `PositionDropdown` + `HudPos` | `ui:nexa-ctl/src/controls/posdrop.rs:24` · `ui:nexa-ctl/src/typeahead.rs:180` |
| 버튼 | `Button` | `ui:nexa-ctl/src/controls/button.rs:90` |
| 카드 스크롤 | `ScrollBars`(오프셋은 호스트 소유) | `ui:nexa-ctl/src/controls/scroll.rs:320` |
| hover 페이드 | `IntentFade` · `hover_alpha` | `ui:nexa-ctl/src/tokens.rs:581` · `:332` |
| 그리기 | `RasterCtx`/`DrawCtx`/`FontSlot`/`FontPrefs`/`Theme` | `ui:nexa-ctl/src/raster.rs:60` |
| 색 값 | `rgba_from_hex` · `ColorPicker`(색 창) | `ui:nexa-ctl/src/controls/colorpanel.rs:103` · `ui:nexa-ctl/src/controls/colorpick.rs:36` |

**nexa-ui에 없고 nexa-sql 앱 코드에 있는 것**(dir3로 옮기거나 nexa-ui로 승격해야 함): 설정 창 조립 `PrefsWin` · 단축키 창 `KeysWin` · 색 창 `ColorsWin`(`sql:colors_win.rs`) · `CopyBtn`(`sql:copybtn.rs`) · 검색 이력 `SharedHistory`/`Recall`(`sql:search_history.rs`) · 한글 자모 검색(`nexa-sql/crates/nsql-core/src/hangul.rs`) · 창 보조(`sql:wingeom.rs` · `sql:winfocus.rs` · `sql:present.rs` · `sql:icon.rs`) · 입력 보조(`sql:input.rs`의 `system_ime` · `wheel_event` · `shortcut_letter`) · 클립보드(`sql:clipboard.rs`).

### 4-4. 변경 통지·핫리로드 요약

| 경로 | 발화 | 처리 | 근거 |
| --- | --- | --- | --- |
| 설정 창 조작 | `PrefsAction::Changed/Reset` | set → save → `apply_setting` → refresh | `sql:app/event_loop.rs:1051-1085` |
| 단축키 창 | `KeysAction::Changed/ResetAll` | set → save → `Keymap::from_settings` → 메뉴 표기 | `sql:app/event_loop.rs:1136-1153` |
| 색 창 | `ColorsAction::Changed/Reset` | set → `apply_setting` → save | `sql:app/event_loop.rs:1162-1197` |
| 메뉴·단축키·코드 | 코드가 `settings.set` | `persist_settings` → 필요한 apply → `prefs_sync` | `sql:app/settings.rs:95-100` · `:963-980` |
| settings.json 저장 | 1초 mtime 폴링(연 뒤에만) | `import_json` → 바뀐 키만 `apply_setting` | `sql:app/settings.rs:38-93` |
| settings.conf 외부 변경 | **감시 없음**(추정 · SET-133) | — | `set:perf.rs:235` |
| OS 테마/신호 | winit `ThemeChanged` · 신호 60초 캐시 | `apply_theme` · `effective` 재계산 | `sql:app/settings.rs:950-960` · `set:perf.rs:346-369` |

즉시 반영이 안 되는 키는 `apply_setting`이 `false`를 돌려 상태줄에 "재시작 필요"가 뜬다(`sql:app/event_loop.rs:1068-1070`).

### 4-5. 창 없이 시험하는 방법

`PrefsWin::new()` → `refresh(&Settings)` → `preset_query`/`select_category`/`advanced.set_on` → `cards`·`sel`·`tree` 상태 단언. winit 창·표면을 만들지 않는다(`window: None`이어도 `rebuild_cards`가 동작)(`sql:prefs_win.rs:2227-2321`).

### 4-6. dir2 화면 배치와의 차이 — 재현 수단

dir2 설정 창은 페이지형(분류당 제목 + 캡션/라디오/체크/글꼴 행/[편집…] — PREFS-309·310)이고 nexa-sql은 카드형(종류 → 컨트롤 고정 매핑)이다. 레지스트리 생성 방식을 유지하면서 dir2 컨트롤을 재현하려면 **위젯 힌트 곁 표**를 추가하는 것이 현 구조와 가장 잘 맞는다(nexa-sql의 키 이름 술어 `is_color_key` 등 `sql:prefs_win.rs:183-199`를 표로 일반화).

```rust
// ndir-settings — 설정 창이 kind 기본 매핑 대신 쓸 표시 형태(없으면 kind 기본)
pub enum Widget {
    Radio,                       // Choice를 라디오 그룹으로(dir2 테마·탭 더블클릭 등)
    Checkbox,                    // Bool을 체크박스로(dir2 배치 유지)
    FontRow { size_key: &'static str }, // 패밀리 + 크기 한 줄(PREFS-412 FontBox 필요)
    OrderEditor(OrderKind),      // [편집…] → 순서 편집 창(PREFS-312 · OrderTree 필요)
    Folder,                      // [찾아보기…]
    Color,                       // [선택…] + 스와치
    KeyCapture,                  // [캡처…]
}
pub const WIDGETS: &[(&str, Widget)] = &[
    ("ui.theme", Widget::Radio),
    ("toolbar.order", Widget::OrderEditor(OrderKind::Toolbar)),
    ("ui.font_face", Widget::FontRow { size_key: "ui.font_size" }),
];
```

이 표는 nexa-sql에 없는 **dir3 신규 제안**이다(확정 아님 — §5-4 결정 4).

---

## 5. dir3 적용 계획

### 5-1. 크레이트 구성 제안

| 크레이트 | 역할 | 원본 | 의존 |
| --- | --- | --- | --- |
| `nexa-conf`(형제 저장소) | 파일 형식·원자적 쓰기·스케줄러·경로 | `ui:nexa-conf` | 없음 |
| `ndir-i18n` | 메시지 카탈로그(dir2 `.lang` 자원 유지 여부 = 결정 3) | dir2 `i18n.rs` + nexa-sql `nsql-i18n` 형태 | 없음 |
| **`ndir-settings`** | 레지스트리 + `Settings` + 이주 + JSON + (perf) | `nsql-settings` | `ndir-i18n` · `nexa-conf` · `nexa-sys`(`default-features = false`) |
| `ndir-session`(또는 `ndir-settings::session` 모듈) | 세션 파일(탭·경로·펼침·컬럼 — PREFS-201~211) — **레지스트리 밖** · `nexa_conf::Store` + `SaveScheduler` | dir2 `config.rs` 세션부 | `nexa-conf` |
| `nexa-dir`(bin) `prefs_win.rs` · `keys_win.rs` · `keymap.rs` · `app/settings.rs` | 설정 창·단축키 창·키맵·적용 | 동명 파일 | `ndir-settings` · `nexa-ctl` · `nexa-gfx` · winit |

`ndir-settings` 내부 모듈 분할 제안(원본은 `lib.rs` 한 파일 7402줄 — 표와 엔진이 섞여 있다):

| 모듈 | 내용 | 원본 줄 |
| --- | --- | --- |
| `kind.rs` | `SettingKind` · `POSITIONS` · `normalize` · `allowed` · `parse_size` · `size_px` | `set:lib.rs:233-263` · `:6390-6477` |
| `registry.rs` | `Entry` · `REGISTRY` · `*_OPTS` · `entry()` | `:265-279` · `:282-5790` |
| `tree.rs` | `CATEGORY_TREE` · `tree_order` · `group_of` · `display_order` · `EXTENSION_CATEGORIES` | `:5792-5871` · `:5914-5939` · `:6373-6388` |
| `tables.rs` | `OS_DEFAULTS`/`default_of` · `DEPENDS`/`Dep` · `HIDDEN` · `ADVANCED` · `INFO_KEYS` · (`WIDGETS`) | `:5873-5912` · `:5941-6371` |
| `migrate.rs` | `OLD_DEFAULTS` · `RENAMED` · `RESCALED` · `canonical_key` · `alias_scale` · `migrate_*` · dir2 가져오기 | `:29-146` · `:6573-6661` |
| `store.rs` | `Settings` · `SetError` · `config_dir` · `ThemeMode` | `:154-229` · `:6481-6813` |
| `json.rs` | 그대로 | `set:json.rs` |
| `perf.rs` | 그대로(표만 교체) | `set:perf.rs` |

### 5-2. 파일 단위 복사/교체 구분

| 원본 파일 | 처리 | 바꿀 곳 |
| --- | --- | --- |
| `nexa-ui/crates/nexa-conf/src/lib.rs` | **의존(복사 안 함)** | 없음(BOM 보강은 nexa-ui 쪽 변경 — SET-137) |
| `set:json.rs` | **그대로 복사** | `_comment` 문구(`:365`) · `use` 경로 |
| `set:perf.rs` | **구조 복사 + 표 교체** | `Domain`(Db/Net → dir3 도메인: 예 Fs/Io) · `PERF` · `BOOST` · 메시지 키 · `effective` 마지막 줄(SET-134) |
| `set:projfile.rs` | **제외** | — |
| `set:lib.rs` 엔진부(1~280의 타입·함수 · 5786~5940 · 5941~5963 · 6156~6163 · 6236~6240 · 6364~6477 · 6481~6813) | **복사**(모듈 분할) | `APP_DIR` · 환경 변수 · `Lang`/`Msg` 타입 · `GrpDbms` 특례 제거 · `migrate_*` 3개 제거 · `grid_col_max_chars` 등 도메인 접근자 제거 |
| `set:lib.rs` 표(`*_OPTS` · `REGISTRY` · `CATEGORY_TREE` · `OS_DEFAULTS` · `DEPENDS` · `HIDDEN` · `ADVANCED` · `INFO_KEYS` · `EXTENSION_CATEGORIES` · `RENAMED` · `RESCALED` · `OLD_DEFAULTS`) | **전부 새로 작성** | PREFS-101~170 변환 + 공통 키(§5-3) |
| `set:lib.rs` 테스트(6815~7402) | **복사 후 키 교체** | DBMS·perf·이주 전용 시험은 해당 기능 채택 여부에 따름 |
| `sql:prefs_win.rs` | **복사 후 덜어내기** | 제거: 포맷 미리보기(SET-072) · `is_default_formatter_key` · Oracle 폴더 키 · `OpenExtSettings`. 교체: 창 제목 · 메시지 키 · 폴더/보조 버튼 술어 · (선택) `WIDGETS` 분기 |
| `sql:keys_win.rs` | **그대로 복사** | 메시지 키 |
| `sql:keymap.rs` | **엔진 그대로 + `COMMANDS` 교체** | 명령 표 · `repeatable` 목록 · 테스트의 명령 id |
| `sql:app/settings.rs` | **패턴만**(본문 재작성) | `apply_setting` 조각을 dir3 도메인(목록·터미널·도크·전송·툴바…)으로 |
| `sql:copybtn.rs` · `sql:wingeom.rs` · `sql:winfocus.rs` · `sql:present.rs` · `sql:search_history.rs` · `nsql-core/src/hangul.rs` · `sql:colors_win.rs` · `sql:theme.rs` | **복사**(설정 창의 직접 의존) | 다른 인벤토리 문서(nexa-sql 창·입력 기반)와 중복 여부 확인 후 한 곳에서 이식 |
| `cli:config.rs` | **선택 개작** | 점검 하네스 진입점으로(§5-6) |

### 5-3. dir3 레지스트리 초기 구성(출처)

| 묶음 | 키(예) | 출처 |
| --- | --- | --- |
| dir2 설정 70키 | `ui.theme` · `list.*` · `term.*` · `dock.*` · `transfer.*` · `typeahead.*` · `scroll.*` · `tabs.*` · `toolbar.order` · `ctxmenu.order` … | [15 §5-1](15-dir2-prefs-config.md) PREFS-101~170("dir3 제안 키" 열) |
| nexa-sql 공통 키(그대로 가져올 후보) | `ui.lang` · `ui.font_face`/`ui.font_size` · `ui.text_*`(래스터 5키 + `OS_DEFAULTS`) · `ui.fade_*_ms` · `ui.animations` · `ui.max_fps` · `ui.copy_feedback_ms` · `ui.prefs_advanced` · `input.scroll_natural` · `input.ime_hint*` · `input.hangul_compose` · `gfx.mac_present` · `gfx.linux_backend` · `clipboard.x11_native` · `window.always_on_top` · `window.<name>_pos/_size` · `key.preset` · `key.<id>` · `settings.json_editor` · `license.gates` | `set:lib.rs:719-799` · `:2044-2089` · `:3423-3453` · `:3480-3598` · `:4531-4553` · `:4744-4773` |
| OS 분기 신규 키 | `term.shell`(+`OS_DEFAULTS`) · 셸 컨텍스트 메뉴·휴지통·DnD 관련 스위치 | 터미널·셸 통합 인벤토리 문서에서 확정 |
| 플러그인 | `plugins.disabled` · `preview.map` · 플러그인별 `ext.<id>.*` | PREFS-122·123 · SET-112 |

정적 레지스트리에 안 들어가는 것(PREFS-165~169의 동적 키 군 `launcher{N}` · `cloud{N}` · `cloud_client_*_{kind}`)은 레지스트리 밖 전용 파일로 분리하는 것을 권장한다(미지 키 보존에 기대면 `set` 검증·설정 창·JSON이 모두 못 본다). nexa-sql에도 "번호가 붙는 키"는 없다(전부 고정 키 — `set:lib.rs:718-5784` 집계).

### 5-4. 결정이 필요한 것

| # | 쟁점 | nexa-sql | dir2 | 권장 |
| --- | --- | --- | --- | --- |
| 1 | 설정 폴더 정책 | 사용자 폴더 고정 + `NSQL_HOME` 재지정 · exe 옆 금지(`set:lib.rs:154-161`) | exe 옆 `data\` 우선 → `%LOCALAPPDATA%` 폴백(PREFS-040) | `NDIR_HOME` → (exe 옆 `data/`가 **이미 있고** 쓰기 가능하며 `is_replaced_on_upgrade`가 아닐 때) → `user_config_dir("nexa-dir")`. 포터블 계승과 macOS 번들/Homebrew 안전을 함께 만족(`conf:294-372`) |
| 2 | 설정·세션 파일 분리 | `settings.conf` 하나(창 기하·최근 목록도 HIDDEN 키) | `settings.cfg` + `session.cfg` | 분리 유지: `settings.conf`(레지스트리) + `session.conf`(탭 목록 등 가변 구조 · `Store`+`SaveScheduler`) |
| 3 | i18n 라벨 타입 | 컴파일 타임 `Msg` enum 2언어(`i18n:lib.rs:1-25`) · `SettingKind::Lang` = `Lang::ALL` | `.lang` 파일 3언어 + 사용자 오버라이드 + 동적 발견(PREFS-070~079) | "자원 그대로" 원칙이면 `Entry.label/desc/cat`을 **언어 키 문자열**(`&'static str` 예 `"pref.theme"`)로 두고 `ui.lang`은 `Text`+`dyn_choices`(기본 `system`). `Msg` enum을 택하면 `.lang` → enum 생성 단계가 필요. 어느 쪽이든 `SetError`·`allowed` 문구의 번역 호출만 바뀐다 |
| 4 | 설정 창 모양 | 카드형(종류 → 컨트롤 고정) | 페이지형(라디오·체크·글꼴 행·[편집…]) | 사용자 지시가 "설정 구조는 nexa-sql 차용" + "컨트롤 배치 유지"이므로: **nexa-sql 골격(검색+트리+카드+Advanced+JSON)** 위에 `WIDGETS` 표(§4-6)로 dir2 컨트롤 종류를 재현 |
| 5 | 성능 거버너 | `perf.mode` + `perf.boost` | 없음 | `perf.rs` 구조는 가져오되 표를 최소로 시작(`flag`/`int`가 `effective`를 돌므로 파일을 빼면 접근자를 고쳐야 한다 — `set:lib.rs:6770-6787`) |
| 6 | 다중 인스턴스 | 마지막 저장이 이김 · 외부 변경 감시 없음(SET-133) | 단일 인스턴스 기준(추정) | 저장 직전 파일 mtime이 바뀌었으면 다시 읽어 **바꾼 키만 병합** 후 쓰기, 또는 인스턴스 잠금(`instance.lock` 패턴) |
| 7 | 기본값 정책 차이 | `ui.theme = system` · `ui.lang = OS 언어` | `theme = dark` · `lang = system`(PREFS-101·102) | "기능 유지" 원칙으로 dir2 기본값 계승 |

### 5-5. dir2 설정 가져오기(1회 이주)

nexa-sql에는 없는 dir3 고유 작업이다. dir2 `data\settings.cfg`(PREFS-042 · 불리언 `0/1` · 위치 `0..8` · 키 이름 다름)를 처음 실행 때 읽어 dir3 키로 옮긴다.

- 변환표 = PREFS-101~170(옛 키 → "dir3 제안 키").
- 불리언 `0/1`은 `normalize`가 그대로 받는다(`set:lib.rs:6441-6445`). 위치 정수는 `POSITIONS[n]`로(`set:lib.rs:253-263`).
- `Settings::set`을 통해 넣으므로 범위 밖 값은 자동으로 버려진다(기본값 유지 = fail-soft).
- 구현 자리 = `migrate.rs`의 `import_dir2(text) -> Vec<(key, value)>`(순수 함수 → 단위 시험 가능).

### 5-6. 테스트·점검 하네스(회귀 대비)

| 구분 | 내용 | 원본 |
| --- | --- | --- |
| 레지스트리 무결성(자동) | 기본값 자기 검증 · 키 유일 · `OS_DEFAULTS`/`PERF`/`BOOST` 값 검증 | SET-120 · SET-121 · SET-125 |
| **추가할 무결성**(원본에 없음) | `HIDDEN`·`DEPENDS`(자식·부모)·`ADVANCED`·`INFO_KEYS`·`WIDGETS`의 키가 전부 레지스트리에 있다 · 모든 `Entry.cat`이 `CATEGORY_TREE`에 있다 · 종속 키는 부모 뒤에 등재 · **전 명령 id ↔ `key.<id>`** · **전 창 이름 ↔ `window.<name>_pos/_size`** · 모든 라벨/설명 키가 i18n에 있다 | SET-130 · SET-131 · SET-132 |
| 저장 왕복 | 변경분만 저장 · 기본값 복귀 시 줄 삭제 · 손상 값 폴백 · 미지 키 보존 · 개행·역슬래시 값 | SET-122 · SET-127 |
| 이주 | `RENAMED`/`RESCALED` 표 루프 · dir2 가져오기 변환표 전수(PREFS-708) | SET-123 |
| 실효 값 | `effective == get`(비부하원 키 · OS_DEFAULTS 키 포함) | SET-134 |
| 설정 창(창 없음) | 분류 선택 → 카드 = 그 분류만 · 검색(영문·한글 자모) · Advanced 숨김 수 · 종속 잠금 → 컨트롤 읽기 전용 · 잠긴 카드 변경 폐기 | SET-128 |
| 적용 누락 감시 | **모든 노출 키에 대해 `apply_setting(key)`가 `true`이거나 "재시작 필요" 허용 목록에 있다** — 새 키를 추가하고 적용을 빼먹으면 잡힌다(원본에 없는 신규 제안) | SET-081 · SET-136 |
| 실행 점검 | `NDIR_HOME=<격리 폴더>`로 앱 기동 → 설정 변경 → 격리 폴더의 `settings.conf` 내용·수정 시각 확인 · 실제 설정 폴더 불변 확인 | SET-114 |
| CLI/하네스 | `--config list|get|set|reset|path|export-json` — 핵심 설정을 GUI 없이 점검 | SET-100 |

단위 테스트는 실제 설정 폴더에 쓰지 않는다: `Settings::from_text(PathBuf::from("x"), "...")`(순수) 또는 `temp_dir()` 아래 PID 폴더(`set:lib.rs:7078-7082` · `:7205`).

### 5-7. 구현 순서 제안

1. `ndir-i18n`(결정 3) → `ndir-settings` 엔진(kind · store · tables 빈 표 · json) + 무결성 시험.
2. 레지스트리 표 작성(PREFS-101~170 + 공통 키) · `CATEGORY_TREE` · `DEPENDS` · `HIDDEN` · `OS_DEFAULTS`.
3. dir2 가져오기 + 세션 파일(`Store`/`SaveScheduler`).
4. `keymap.rs`(엔진 복사 + dir2 명령 표) · `key.<id>` 전수 등재.
5. `prefs_win.rs`(복사 → 덜어내기 → `WIDGETS`) · `keys_win.rs` · 창 기하 기억.
6. 호스트 `apply_setting`(도메인 조각) + 기동 적용 통일 + "적용 누락 감시" 시험.
7. (선택) `perf.rs` 표 · JSON 편집 감시 · CLI/하네스 `config`.
