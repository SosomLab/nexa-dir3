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
| CI-109 | `--smoke` = `--ci` 전체 | `nexa-dir/src/main.rs` | T5 | CI | 🚧 | 그룹 env·resources 실제 |
