# 42 · 기준 구조 가이드 — 라이선스(nexa-license · nsql-license · dir2 정책) → nexa-dir3 통합 명세

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 2026-10-03.
> 목적: nexa-dir3에 **nexa-license 기반 라이선스 발급/검증**을 얹기 위한 명세. 기준 구현 = nexa-sql(`nsql-license` + 라이선스 창), 정책 원천 = nexa-dir2.
> 이 문서의 `LIC-NNN` ID는 이후 구현·교차 검증의 체크리스트다.
>
> **경로 표기 약속**(모든 근거는 `저장소/경로:줄`):
> - `lic:X` = `nexa-license/crates/nexa-license/src/X` · `tool:X` = `nexa-license/crates/nexa-license-tool/src/X` · `lic-root:X` = `nexa-license/X`
> - `nsql-lic:X` = `nexa-sql/crates/nsql-license/X` · `sql-app:X` = `nexa-sql/crates/nexa-sql/src/X` · `sql:X` = `nexa-sql/crates/X` · `sql-doc:X` = `nexa-sql/docs/X`
> - `dir2:X` = `nexa-dir2/X` · `ui:X` = `nexa-ui/crates/X`
>
> **ID 대역**: 001~ nexa-license 라이브러리 · 041~ 발급기 · 061~ nsql-license 앱 층 · 091~ nexa-sql GUI/배선 · 131~ dir2 정책 · 151~ dir3 통합 계획 · 181~ OS 분기 · 191~ 위험/불일치.
> **표기**: "추정" = 코드로 확인 못 함 · "결정 필요" = dir2에서 근거를 찾지 못해 사용자 결정이 필요한 항목.

## 0. 범위

### 0-1. 읽은 파일(전부 끝까지 읽음 — 표기 없는 것은 전체)

| 저장소 | 파일 | 줄 수 | 비고 |
| --- | --- | --- | --- |
| nexa-license | `README.md` · `CLAUDE.md` · `Cargo.toml` · `.github/workflows/ci.yml` | 76 · 10 · 25 · 29 | 전체 |
| nexa-license | `crates/nexa-license/Cargo.toml` · `src/lib.rs` · `types.rs` · `format.rs` · `verify.rs` · `date.rs` · `version.rs` · `ed25519.rs` · `machine.rs` · `request.rs` · `fs.rs` · `keys.rs` · `sign.rs` · `envelope.rs` | 36 · 50 · 194 · 194 · 445 · 114 · 44 · 25 · 92 · 69 · 317 · 28 · 200 · 273 | 전체(테스트 포함) |
| nexa-license | `src/base32.rs` | 142 | 1~90(구현부 전부 · 뒤는 테스트) |
| nexa-license | `crates/nexa-license-tool/Cargo.toml` · `src/main.rs` · `presets.rs` · `ledger.rs` · `tests/e2e.rs` | 20 · 1191 · 51 · 161 · 478 | 전체 |
| nexa-license | `LICENSE.md` · `LICENSE.ko.md` | 139 · 119 | dir2 판과 diff로 대조(고지 문구만 다름) |
| nexa-sql | `crates/nsql-license/Cargo.toml` · `build.rs` · `src/lib.rs` | 22 · 51 · 824 | 전체 |
| nexa-sql | `crates/nexa-sql/src/license_win.rs` · `app/license.rs` | 565 · 283 | 전체 |
| nexa-sql | `crates/nsql-cli/src/license.rs` | 216 | 전체(CLI 입구) |
| nexa-sql | `docs/13-licensing.md` · `23-license-activation.md` · `25-license-tiers-and-server.md` · `91-license-root-key-operations.md` · `92-digital-asset-protection.md` | 21 · 255 · 553 · 98 · 95 | 전체 |
| nexa-sql | 배선 Grep: `app/event_loop.rs` · `app/menus.rs` · `app/paint.rs` · `app/input.rs` · `app/files.rs` · `app/startup_cmd.rs` · `file_win.rs` · `about_win.rs` · `main.rs` · `nsql-settings/src/lib.rs` · `nsql-i18n/src/lib.rs` · `docs/10-decision-record.md` · `.github/workflows/*.yml` | — | 라이선스 관련 줄만(Grep + 구간 Read) |
| nexa-dir2 | `LICENSE.md` · `LICENSE.ko.md`(1~30 + diff) · `README.md` · `crates/nexa-app/src/about.rs` | 139 · 119 · 59 · 394 | 전체 |
| nexa-dir2 | Grep `licen\|라이선스\|commercial\|PolyForm\|X-15\|nexa-lic\|정품` in `docs/`(23개 파일 57건) · `crates/`(14건) · `Cargo.toml` · `CLAUDE.md` · `installer/` · `packaging/` | — | 일치 줄 전부 확인 |

### 0-2. 한 줄 결론

- nexa-license **라이브러리(검증 쪽)는 dir3를 위해 바꿀 것이 없다** — 제품은 `Product` 상수 주입이고(`lic:types.rs:5-14`) 루트 키·서명 도메인은 계열 공통이다(`lic:lib.rs:11`). 바꿔야 하는 것은 **발급기의 nexa-sql 고정 문구/기본값**뿐이다(§5-3).
- nexa-sql 앱 층(`nsql-license` 824줄 + 창 565줄 + 배선 283줄)은 **거의 그대로 복제**할 수 있다(제품 id · 빌드일 환경 변수 · `Feature` 목록 · i18n 체계만 다름).
- nexa-dir2에는 **라이선스 집행 코드가 0줄**이다(About 창의 문구 1줄뿐 · `dir2:crates/nexa-app/src/about.rs:255`). 확인된 정책 = "PolyForm NC · 개인/비상업 무료 · 상업 사용은 별도 유료 라이선스" + **설계 등록만 된 X-15 오프라인 라이선스**(`dir2:docs/TODO.md:96`). 기능 게이트·등급 정의는 dir2에 **없다** → dir3의 `Feature`/등급은 "결정 필요".

## 1. 요소 표

### 1-1. nexa-license 라이브러리(001~)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| LIC-001 | 워크스페이스 메타 | version 0.0.1 · edition 2021 · rust-version 1.82 · `LicenseRef-PolyForm-Noncommercial-1.0.0` · lints(`unwrap_used` 등 warn) | `lic-root:Cargo.toml:9-25` | dir3 워크스페이스 MSRV ≥ 1.82 · path 의존 `../nexa-license/crates/nexa-license` |
| LIC-002 | feature 플래그 | `default=[]` · `ed25519`(dalek 2.x) · `machine-id`(sha2) · `fs` · `protocol`(빈 feature) · `issuer`(발급 전용) | `lic-root:crates/nexa-license/Cargo.toml:14-31` | dir3 = `["ed25519","machine-id","fs"]` (nsql-license와 동일 · `issuer` 금지) |
| LIC-003 | 도메인 상수 | `FORMAT="nxl1"` · `REQUEST_PREFIX="NEXAREQ1"` · `SERVER_REQUEST_PREFIX="NEXASRV1"` · `MACHINE_DOMAIN="nexa/machine-v1"` · `LICENSE_DOMAIN="nexa/license/v1"` · `LEASE_DOMAIN="nexa/lease/v1"` | `lic:lib.rs:39-50` | 그대로(계열 공통 — 제품 구분은 `product=` 본문) |
| LIC-004 | `format::Doc` key=value 문서 | `parse`(관용 · `#` 주석/빈 줄/`=` 없는 줄 건너뜀 · CRLF 허용 · trim) · `get` · `get_numbered`(`key`,`key.2`…) · `has_duplicate_keys` · `set` · `serialize`(LF · 값의 개행→공백) | `lic:format.rs:14-125` | 요청 코드 메타 조립에 사용(`Doc::set`) |
| LIC-005 | 정규화 서명 대상 | `Doc::canonical(domain)` = `domain\n` + `sig` 제외 전 쌍을 `(key,value)` 정렬 · `key=value\n` 결합 → 줄 순서·CRLF·주석 변화에 불변 | `lic:format.rs:88-108` · `SIG_KEY` `:19` | 그대로 |
| LIC-006 | Crockford base32 | `encode` · `encode_grouped(bytes, group)` · `decode`(대소문자 무관 · `-`/공백 무시 · `O→0` `I/L→1` · 남는 비트 0 검사) | `lic:base32.rs:10,30,63-82` | 기기 코드 표시(`encode`) |
| LIC-007 | 날짜 | `parse_days` · `format_days` · `add_years`(2/29→2/28) · `today_days`(UTC) · 외부 crate 0 | `lic:date.rs:5,19,26,36` | `today_days()`를 판정에 주입 |
| LIC-008 | 버전 비교 | `version::parse`(`-pre`/`+meta` 무시) · `major` · `at_or_after`(못 읽으면 false = 관대) | `lic:version.rs:6,16,22` | 앱 버전 = `CARGO_PKG_VERSION`(LIC-152) |
| LIC-009 | `Product` 기술자 | `{ id, build_date, version }` + `license_file_name()`=`<id>.license` · `lease_file_name()` · `accepts(field)`(쉼표 목록 · `*` 번들) | `lic:types.rs:7-37` | dir3 상수 1개(LIC-152) — **라이브러리 변경 불필요** |
| LIC-010 | `Kind` | `Device`/`User`(기본)/`TeamSeat`/`Org` · `as_str`(`device`/`user`/`team-seat`/`org`) · `parse`(빈 값=User) · `max_machines`(1/5/5/0) | `lic:types.rs:40-80` | 그대로(계열 공통) |
| LIC-011 | `SeatMode` | `Named`(기본)/`Device`/`Concurrent` | `lic:types.rs:83-110` | 표시만(서버 보류) |
| LIC-012 | `Alg` | `Ed25519`(기본)/`P256` — **P256 어댑터·루트 키는 어디에도 없음** | `lic:types.rs:113-137` | `ed25519`만 사용(LIC-173) |
| LIC-013 | `SigVerifier` 포트 | `trait SigVerifier { fn verify(&self, alg, pubkey, msg, sig) -> bool }` | `lic:verify.rs:17-20` | `Ed25519Verifier` 주입 |
| LIC-014 | Ed25519 어댑터 | `Ed25519Verifier` — dalek `verify_strict` · `alg != Ed25519`면 false | `lic:ed25519.rs:7-25` | 그대로 |
| LIC-015 | `License` 내용 | `id · licensee · kind · tier · features: BTreeSet<String> · machines · issued · updates_until · expires · seats · seat_mode · key_id · max_major: Option<u64> · max_version` + `allows(feature)`(`*`=전부) | `lic:verify.rs:23-52` | 창 표시 행의 원천(LIC-102) |
| LIC-016 | `Invalid` 사유 | `Format · Product · DuplicateKeys · NoRootKey · Signature · Machine · Malformed` | `lic:verify.rs:55-64` | 배지/안내 문구 열쇠 |
| LIC-017 | `Verdict` | `Licensed(License) · Outdated(License) · Expired(License) · Invalid(Invalid)` | `lic:verify.rs:67-75` | `LicenseState`로 사상(LIC-067) |
| LIC-018 | `verify_license` 흐름 | 시그니처 `(product, roots, verifier, text, machine: Option<&[u8]>, today: i64) -> Verdict` · 순서는 §2-3 | `lic:verify.rs:79-184` | 앱 층 `judge()`가 호출(LIC-069) |
| LIC-019 | 기한·버전 조항 | `updates_until` < 빌드일 → Outdated · 앱 Major > `max_major` → Outdated · 앱 버전 ≥ `max_version` → Outdated · 오늘 > `expires` → Expired(만료일 당일은 유효) | `lic:verify.rs:160-183` · 시험 `:337-350,400-444` | 그대로 |
| LIC-020 | 기기 ID | `machine_id() -> Option<Vec<u8>>`(20B) = `SHA-256(MACHINE_DOMAIN ‖ 0x00 ‖ trim(원문))[..20]` · `hash_identifier(raw)` · OS별 원천 = LIC-181 | `lic:machine.rs:10-22` | 그대로(3-OS 지원 내장) |
| LIC-021 | 요청 코드 | `request::encode(machine, meta)` → `NEXAREQ1.<base32 20B>[.<base32(메타 Doc)>]` · `decode` → `Request{machine, meta}`(20B 아니면 None) | `lic:request.rs:8-44` | 메타 `app=nexa-dir/<버전>` 형식 유지(LIC-046의 Major 추출 규칙) |
| LIC-022 | 파일 자리 `fs::Store` | `new(product, dirs)` · `path_in` · `primary_path`(첫 폴더) · `locate`(첫 존재 파일) · `stamp`(mtime+len) · `read`(없으면 `Ok(None)`) · `install_text` · `install_from` · `remove`(첫 폴더만) · `Found` · `Stamp` | `lic:fs.rs:17-139` | 폴더 목록만 dir3 규칙으로 주입(LIC-154) |
| LIC-023 | 원자적 쓰기 | `write_atomic(path, bytes)` — 같은 폴더 `<이름>.tmp-<pid>-<nanos>` 생성 → `sync_all` → `rename` · 유닉스 0600 · 실패 시 임시 파일 삭제 | `lic:fs.rs:141-175` | 그대로 |
| LIC-024 | 루트 공개키 | `RootKey{id, alg, public}` · `ROOT_KEYS = [root-v1 (Ed25519 32B)]` — 생성 파일(손으로 고치지 않음) | `lic:keys.rs:8-28` | 그대로 공유 — **dir3 전용 키 없음**(한 루트 키로 전 제품) |
| LIC-025 | 서명(발급 전용) | `sign::Keypair{secret, public}` · `generate` · `from_secret` · `public_b32` · `sign` · `sign_doc` · `sign_license` · `matches_public` | `lic:sign.rs:9-68` | 앱은 켜지 않음. dir3 **테스트는 dev-dep dalek으로 직접 서명**(nsql-license 방식 · LIC-079) |
| LIC-026 | 비밀키 봉투 `nxk1` | PBKDF2-HMAC-SHA256(기본 600,000회) → 스트림 키 32B + MAC 키 32B · HMAC 스트림 XOR · MAC · `seal`/`peek`/`open` · `EnvelopeError{Format,Mac,KeyMismatch}` | `lic:envelope.rs:16,95-174` | 발급 PC 전용 — dir3 무관 |
| LIC-027 | `.pub`(`nxp1`) · `keys.rs` 생성 | `public_key_file` · `parse_public_key_file` · `keys_rs_source` | `lic:sign.rs:70-122` | 무관(회전 시 nexa-license 쪽 작업) |
| LIC-028 | `protocol`(리스) | **미구현**(빈 feature · README 표 ☐) — 인증 서버 보류 | `lic-root:crates/nexa-license/Cargo.toml:22` · `lic-root:README.md:35` | 범위 밖 |
| LIC-029 | 개발 게이트 | `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features` · 3-OS CI | `lic-root:README.md:68-70` · `lic-root:.github/workflows/ci.yml:16-29` | nexa-license를 고치면 이 게이트 통과 필수 |
| LIC-030 | 라이선스 파일 키 | `format · product · id · licensee · email · kind · tier · features · machine[.N] · seats · seat_mode · issued · updates_until · expires · max_major · max_version · key · reissued · alg · sig` | `tool:main.rs:497-534,952` · `lic:verify.rs:87-159` | 그대로(모르는 키는 무시 = 전방 호환) |

### 1-2. 발급기 `nexa-license-tool`(041~)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| LIC-041 | 명령·종료 코드 | `keygen · rekey · keys-rs · decode-request · issue · reissue · verify · ledger` · 종료 0 / 1(실패) / 2(인자) / 3(요청 코드) / 4(봉투) | `tool:main.rs:43-45,155-165` | 발급 절차 문서에 인용 |
| LIC-042 | `keygen` | `--out <봉투> [--id root-v1] [--pass-env\|--pass-stdin] [--iter ≥1000]` · 기존 파일 덮어쓰기 거부 · `.pub` 동시 생성 | `tool:main.rs:263-325` | 무관(루트 키 재사용) |
| LIC-043 | `rekey` | 같은 키쌍을 새 암호로 재봉인 | `tool:main.rs:328-398` | 무관 |
| LIC-044 | `keys-rs` | `.pub` N개 → `keys.rs` 본문 | `tool:main.rs:419-447` | 무관 |
| LIC-045 | `decode-request` | 기기 코드 + 메타 출력 | `tool:main.rs:461-474` | dir3 요청 코드 검수에 그대로 |
| LIC-046 | `issue` 인자·기본값 | `--key --request…(kind별 ≤ max_machines) --kind --licensee [--email] (--tier\|--features) [--seats] [--seat-mode] [--updates-until] [--expires] [--max-major] [--max-version] [--id] [--id-prefix] [--product] [--out] [--ledger] [--note] [--no-mail]` · 기본: `expires`=오늘+3년(trial=14일) · `updates_until`=`expires` · `max_major`=**첫 요청 코드 메타 `app`의 마지막 `/` 뒤 버전의 Major** | `tool:main.rs:552-720` · `app_major_of :537-542` | `--product nexa-dir` 지정 필수(기본값이 `nexa-sql` · LIC-053) |
| LIC-047 | 자기 검증 + 산출물 | 쓰기 전 `verify_license`로 자기 검증(실패 시 아무것도 안 씀) → `<out>/<id>/<product>.license` + `ledger.tsv` 1행 + `mail.txt` | `tool:main.rs:723-853` | 산출 파일명 = `nexa-dir.license` |
| LIC-048 | `reissue` | `--id` + `--add-request`/`--drop-machine`/`--renew`/개별 조항 · 옛 판 `.v<N>` 보존 · `reissued=` 추가 · 조항이 바뀌면 mail.txt 재작성 | `tool:main.rs:857-1028` | 그대로 |
| LIC-049 | `verify` | `<파일> (--pub…\|--key) [--machine] [--build-date] [--app-version] [--product]` — 앱과 같은 검증 코드 · `--product` 기본 = 파일의 `product` | `tool:main.rs:1032-1161` | dir3 발급분 확인 절차 |
| LIC-050 | 대장 `ledger.tsv` | 14열 `id version issued kind tier licensee email features machines updates_until expires note file req_meta` — **product 열 없음** · 기기 ID는 접두 8자 · `next_id` = `<prefix>-<연도>-<6자리>` | `tool:ledger.rs:6,61-83,116-124` | LIC-168 |
| LIC-051 | tier 프리셋 | `features_for(product, tier)` — `trial\|pro\|org\|team` 전부 `*`(product 인자 미사용) · `TRIAL_DAYS=14` · `TERM_YEARS=3` | `tool:presets.rs:5-15` | 초기에는 그대로 통용(LIC-167) |
| LIC-052 | `mail_text` | 한/영 고객 안내 — **`nsql license install/status/request` 명령이 하드코딩** | `tool:presets.rs:18-51` | **dir3용으로는 틀린 안내** → LIC-165 |
| LIC-053 | 제품 고정값 | `--product` 기본 `"nexa-sql"` · `--id-prefix` 기본 `"NSL"` · reissue/verify 폴백도 `nexa-sql` | `tool:main.rs:553,685,923,1073` | LIC-166 |
| LIC-054 | E2E 시험 | 바이너리 실행 2건(keygen→issue→verify→reissue→ledger · renew) — 전부 `nexa-sql` 제품 | `lic-root:crates/nexa-license-tool/tests/e2e.rs:36-370,375-478` | LIC-170 |

### 1-3. nexa-sql 앱 층 `nsql-license`(061~)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| LIC-061 | 크레이트 의존 | `nexa-license{ed25519,machine-id,fs}` + `nsql-settings` · dev-dep `ed25519-dalek(rand_core)` · `rand_core(getrandom)` | `nsql-lic:Cargo.toml:15-22` | `ndir-license` 동일 구성(설정 크레이트만 dir3 것) |
| LIC-062 | 빌드일 `build.rs` | `NSQL_BUILD_DATE`: 환경 변수(형식 검사) → `SOURCE_DATE_EPOCH` → 지금 · `cargo:rustc-env` · 외부 crate 0 | `nsql-lic:build.rs:6-51` | `NDIR_BUILD_DATE`로 복제(LIC-153) |
| LIC-063 | `PRODUCT` 상수 | `Product{ id:"nexa-sql", build_date: env!("NSQL_BUILD_DATE"), version: env!("CARGO_PKG_VERSION") }` | `nsql-lic:src/lib.rs:35-40` | LIC-152 |
| LIC-064 | 폴더명·연락처 | `LICENSE_SUBDIR="license"` · `LICENSE_CONTACT="kiros33@sosomlab.com"` | `nsql-lic:src/lib.rs:43,46` | 동일(연락처는 dir2 LICENSE와 일치 — LIC-133) |
| LIC-065 | `Feature` 열거 | 21개(Pro 편의 11 · 상한 해제 5 · Org 5) · `ALL` · `as_str`(파일 이름 매핑 한 곳) · `parse` · `is_org_only` · `Display` | `nsql-lic:src/lib.rs:52-158` | **목록은 SQL 도메인 — 복제 불가** → LIC-155 |
| LIC-066 | `Tier`(표시용) | `Free/Trial/Pro/Org` · `parse`(`team`→Pro · `organization`/`enterprise`→Org · 그 외 Free) · `as_str` | `nsql-lic:src/lib.rs:161-189` | 그대로 복제 가능(판정에는 안 씀) |
| LIC-067 | `LicenseState` | `Free · Licensed(License) · Invalid(Invalid) · Outdated(License) · Expired(License)` + `is_licensed` · `license()` · `tier()` · `name()` | `nsql-lic:src/lib.rs:194-242` | 그대로 |
| LIC-068 | 판정 타입 | `DenyReason{Free, Invalid(_), Outdated, Expired, NotIncluded}` · `Denial{feature, reason}` · `Entitlement{Allowed, Denied}` · `entitlement(state, feature)`(순수) | `nsql-lic:src/lib.rs:245-293` | 그대로 |
| LIC-069 | `judge()` | `(roots, text, machine, today) -> LicenseState` — `verify_license(&PRODUCT, roots, &Ed25519Verifier, …)` 결과 사상 | `nsql-lic:src/lib.rs:297-304` | 그대로 |
| LIC-070 | 폴더 규칙 | `user_dir()` = `<설정 폴더>/license` · `machine_dir()` = OS별 공용 폴더(LIC-182) · `default_dirs()` = 사용자 → 기기 공용 | `nsql-lic:src/lib.rs:310-343` | LIC-154 |
| LIC-071 | `Licensing` 문맥 | 필드 `store · roots · state · path · stamp` · `open_default()` · `open(dirs)` · `with_roots(dirs, roots)`(시험 픽스처) · `state()` · `path()` · `primary_path()` · `dirs()` | `nsql-lic:src/lib.rs:380-438` | 그대로(프로세스에 하나) |
| LIC-072 | ★ `check(Feature)` | 유일한 판정 함수 · 순수(I/O 없음) — `entitlement(&self.state, feature)` | `nsql-lic:src/lib.rs:441-444` | 그대로 |
| LIC-073 | `refresh()` | 경로·mtime·길이가 바뀌었을 때만 재판정 · 바뀌면 `true` | `nsql-lic:src/lib.rs:447-459` | 주기 틱에서 호출(LIC-109) |
| LIC-074 | 설치 | `install(file)` · `install_text(text)` — **Licensed 판정일 때만 쓴다**(원본 보존) · `InstallError{Io, Rejected(Box<LicenseState>)}` · `judge_text` | `nsql-lic:src/lib.rs:348-370,462-484` | 그대로 |
| LIC-075 | 제거 | `remove()` — 사용자 폴더 파일만 삭제 → 재판정(기기 공용 파일로 복귀 가능) | `nsql-lic:src/lib.rs:487-491` | 그대로 |
| LIC-076 | 기기 코드·요청 코드 | `machine_code()`(base32) · `request_code(&RequestMeta{name,email})` — 메타 `os`(`std::env::consts::OS`) · `app`(`"nexa-sql/<버전>"`) · `d`(오늘) · `n` · `e` | `nsql-lic:src/lib.rs:373-377,494-514` | `app="nexa-dir/<버전>"`로(LIC-152) |
| LIC-077 | `features_of(l)` | `*`면 `Feature::ALL` · 모르는 이름은 버림(표시용) | `nsql-lic:src/lib.rs:540-548` | 그대로 |
| LIC-078 | 로그 규칙 | 이 크레이트는 `licensee`·이메일·기기 ID 원문을 어디에도 쓰지 않는다 | `nsql-lic:src/lib.rs:11` · `sql-doc:92-digital-asset-protection.md:81` | 동일 규칙 계승 |
| LIC-079 | 단위 시험 7건 | 이름 왕복 · 파일 없음=Free · 설치→허용/거부 · `*`=전부 · 변조 거부/순서·CRLF 통과 · 다른 기기/Outdated/Expired/다른 제품/NoRootKey · refresh+폴더 우선순위 · 요청 코드 | `nsql-lic:src/lib.rs:552-824` | 회귀 테스트로 그대로 이식(LIC-164) |

### 1-4. nexa-sql GUI·배선(091~)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| LIC-091 | App 상태 | `license_win: LicenseWin` · `licensing: nsql_license::Licensing`(= `open_default()`) · `open_license: bool`(지연 열기 플래그) · `status_lic_rect` | `sql-app:main.rs:228-230,239,1553,1560-1562` | 같은 4필드 |
| LIC-092 | 라이선스 창 골격 | 모덜리스 winit 창 + `Presenter` · 제목 `"Nexa SQL — License"` · 640×540(논리) · 최소 520×380 · 리사이즈 가능 · 소유 창 가운데(+4) · 아이콘 · IME 허용 · 이미 열려 있으면 포커스만 | `sql-app:license_win.rs:96-129` | 제목 `"Nexa Dir — …"` · 동일 수치 |
| LIC-093 | 보기/동작 타입 | `LicView{state, warn, rows, request, contact}` · `LicAction{None, Paint, Close, OpenFile, Remove, CopyRequest(name,email), CopyText(text)}` | `sql-app:license_win.rs:20-45` | 그대로 |
| LIC-094 | 배치 ① 상태 띠 | 패딩 12 · 상태 한 줄(배지와 같은 글) · 배경 = 색(ok/warn) 알파 0.12 · 글자 = 그 색 | `sql-app:license_win.rs:416-428` | 동일 |
| LIC-095 | 배치 ② 상태 표 | (라벨, 값) 행 · 라벨 폭 = 최대 라벨 + 16 · 행 높이 = 글자 + 6 · 값은 `ellipsize_middle` · 라벨 `text_dim` | `sql-app:license_win.rs:429-444` | 동일(컨트롤이 아니라 직접 그림 → LIC-174) |
| LIC-096 | 배치 ③ 요청 구역 | 구분선 1px → 제목 줄(이메일 = **링크**: 강조색 + 밑줄 · hover 손 모양 · 클릭 = 주소 복사) → `이름` TextBox(150) · `이메일` TextBox(200) · [요청 코드 복사] Button(기기 ID 없으면 비활성) | `sql-app:license_win.rs:445-485` | 동일 |
| LIC-097 | 배치 ④ 코드·힌트 | 요청 코드 한 줄(가운데 생략 · dim · 기기 ID 없으면 안내문) → 힌트 한 줄 · **버튼 행 위로 온전한 줄만** 그림 | `sql-app:license_win.rs:486-502` | 동일 |
| LIC-098 | 배치 ⑤ 버튼 행 | 창 맨 아래 고정: [라이선스 파일 열기…] [제거] [닫기](폭 = 글자 + 28, 최소 84 · 간격 8) + 결과 안내(note)는 버튼 **오른쪽 같은 줄**(경고 = danger색) | `sql-app:license_win.rs:491-494,514-538` | 동일 |
| LIC-099 | 창 높이 맞춤 | 열 때·상태/안내가 바뀔 때 한 번 `request_inner_size`(내용 끝 + 버튼 행 · 최소 200) — 사용자가 늘린 뒤엔 건드리지 않음 | `sql-app:license_win.rs:64-65,503-513` | 동일 |
| LIC-100 | 순간 메시지 | nexa-ctl `Flash`(링크 옆 · hold/fade ms · 창의 **맨 마지막**에 그림 · 진행 중이면 재요청) · 수치 = 설정 `ui.flash_hold_ms`/`ui.flash_ms` | `sql-app:license_win.rs:131-140,548-560` · `sql-app:app/event_loop.rs:1524-1534` · `ui:nexa-ctl/src/controls/flash.rs:27,60,121` | nexa-ui에 이미 있음 |
| LIC-101 | 입력 라우팅 | 포커스 링 ≤ 1(`own_focus`) · 누름은 커서 아래 컨트롤만/뗌은 전부 · 우클릭 = TextBox 편집 메뉴 · IME Commit/Preedit · Esc = 닫기 · Ctrl/Cmd+A · `text_key_event(TextKeys::Line)` · 포커스 잃으면 버튼 transient 해제 | `sql-app:license_win.rs:205-393` | 동일(앱 입력 헬퍼 필요 → LIC-175) |
| LIC-102 | 표 행 구성 | 상태 · 파일 · [ID · 사용자 · 종류/등급 · 기능 · 발급일 · 업데이트 기한 · 만료 · 유효 메이저(`N.x`) · (무효 시작 버전)] · 앱 버전 · 빌드일 · 기기 코드 · 설치 자리 — 대괄호는 라이선스가 있을 때만 | `sql-app:app/license.rs:81-155` | 동일(행 라벨은 dir3 i18n) |
| LIC-103 | 배지 글 | Free → `Free · non-commercial use only` · Licensed → `{등급} · {licensee}` · Invalid → `⚠ License invalid ({사유})` · Outdated · Expired(`{expires}`) | `sql-app:app/license.rs:55-78` · `sql:nsql-i18n/src/lib.rs:3401-3405` | 문구 계승(dir2 정책과 일치 — LIC-133) |
| LIC-104 | 상태줄 배지 | 상태줄 **맨 오른쪽** 세그먼트 · 클릭 = 라이선스 창 | `sql-app:app/paint.rs:261-262,289-290` · `sql-app:app/input.rs:646-647` | dir2 상태줄 배치에 없던 요소 → **결정 필요**(LIC-158) |
| LIC-105 | 진입점 | 메뉴 `help.license`(Help ▸ License… · 열려 있으면 닫기 토글) · 명령 팔레트 등록 · About 창 [License…] 버튼 | `sql-app:app/menus.rs:885-890,1274,1698` · `sql-app:about_win.rs:24,33,209-210` · `sql-app:app/event_loop.rs:1496-1497` | LIC-158 |
| LIC-106 | 파일 열기 → 설치 | `FilePurpose::License` + 자체 파일 창(`nexa_dlg::FilePicker`) · 필터 `*.license` 기본 + 전체 · 라이선스 창 위에 뜸 → `license_install(path)`(안내 = 상태줄 + 창 note) | `sql-app:app/event_loop.rs:970,1517-1521` · `sql-app:app/files.rs:387,393,427` · `sql-app:file_win.rs:95-98` · `sql-app:app/license.rs:157-168` | nexa-dlg 사용(LIC-160) |
| LIC-107 | 제거 | `license_remove()` — 제거됨/없음/IO 오류 3분기 안내 | `sql-app:app/license.rs:170-179` | 동일 |
| LIC-108 | 복사 | 요청 코드(이름·이메일 메타 포함) 클립보드 · 이메일 주소 클립보드(Flash) | `sql-app:app/license.rs:181-194` · `sql-app:app/event_loop.rs:1523-1534` | 클립보드 = OS 분기(LIC-185) |
| LIC-109 | 주기 재판정 | 이벤트 루프 틱에서 `licensing.refresh()` — 밖에서(CLI) 파일이 바뀌면 배지·창 갱신 | `sql-app:app/event_loop.rs:566-568` | 동일 |
| LIC-110 | 게이트 API | `gates_on()`(설정 `license.gates`) · `entitled(f)` · `lic_gate(f)`(거부 = 상태줄 + 창 안내 + 창 열기) · `cap(f, n, cap)`(상한식) · `license_limit_note` · `license_denied_text` · `feature_texts`(이름 · 무료 대체 경로) · `tab_room` | `sql-app:app/license.rs:196-283` | API 골격 복제 · 대상 기능은 LIC-155 |
| LIC-111 | 게이트 배선 12곳 | bookmarks:99 · completion:379 · event_loop:1341,1634 · extensions:296 · grid_results:219,496,522 · meta:313 · menus:1381 · files:467 · session:34,281,367 · project:1124,1407 · paint:218 — "기능당 정확히 한 곳 · UI 행위 진입점" | `sql-app:main.rs:1183-1184` + 각 파일 | 패턴만 참고(SQL 기능) |
| LIC-112 | 설정 키 | `license.gates` Bool 기본 `off` · HIDDEN · 옛 이름 `license.gates_dev`에서 RENAMED(D-145: 개인 사용 = 전 기능) | `sql:nsql-settings/src/lib.rs:60,4528-4538,6168` · `sql-doc:10-decision-record.md:80` | LIC-156 |
| LIC-113 | i18n 키(영/한) | `WinLicense · MnLicense · StLic{Free,Licensed,Invalid,Outdated,Expired} · LicTier{Trial,Pro,Org} · LicLbl{State,File,Id,Licensee,Kind,Features,Issued,Until,Expires,MaxMajor,MaxVersion,Version,Build,Machine,InstallTo} · LicReq{Title,Name,Email} · LicBtn{CopyReq,Open,Remove,Close} · LicNoMachine · LicHint · LicNote{Copied,CopyFailed,Installed,Rejected,Removed,Nothing,Io} · LicDenied · LicLimit · LicFeat*/LicAlt* · FilterLicense · About{BtnLicense,LicenseState,Terms,Copyright}` | `sql:nsql-i18n/src/lib.rs:3349-3352,3363-3436,3887` | dir3 i18n으로 옮김(LIC-159) |
| LIC-114 | CLI | `nsql license status\|request [name [email]]\|install <f>\|remove\|path\|export <f>\|import <f>` · 종료 0/1/2 | `sql:nsql-cli/src/license.rs:24-216` | dir2에 CLI 없음 → **결정 필요**(LIC-162) |
| LIC-115 | 자체 시험 기동 명령 | `license.dump:<파일>`(배지 + 표 + 창 상태 덤프) · `license.install:<파일>` · 창의 `dump()` | `sql-app:app/startup_cmd.rs:332-345` · `sql-app:license_win.rs:149-165` | 테스트 하네스에 동일 훅(LIC-164) |
| LIC-116 | About 줄 | 제품+버전 · 버전 · 빌드일(`PRODUCT.build_date`) · 시스템(OS/ARCH) · 라이선스 상태(배지 글) · 저장소 · 저작권 · 조건(`PolyForm Noncommercial 1.0.0 — commercial use requires a paid license`) | `sql-app:app/license.rs:25-52` · `sql:nsql-i18n/src/lib.rs:3369-3370` | dir2 About 배치 유지 + 추가분은 LIC-158 |
| LIC-117 | 워크스페이스·CI | `nexa-license = { path = "../nexa-license/crates/nexa-license" }` · CI 3개 워크플로가 `SosomLab/nexa-license`를 형제로 체크아웃 | `nexa-sql/Cargo.toml:69-70` · `nexa-sql/.github/workflows/ci.yml:31-34` · `release.yml:96-99` · `integration.yml:77-80` | LIC-171 |
| LIC-118 | 창 관리 | 소유 창 연결(`winfocus::attach_child`) · 모달 동기화(`sync_modal`) · 창 닫기 일괄(`toolbar.rs:181`) · 포커스 대상 목록 | `sql-app:app/license.rs:8-23` · `sql-app:app/windows.rs:19,30,155` | 앱 셸 헬퍼(LIC-175) |

### 1-5. nexa-dir2 라이선스 정책(131~)

| ID | 요소 | 구현(확인된 사실) | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- |
| LIC-131 | 본체 라이선스 | **PolyForm Noncommercial 1.0.0**(영문 정본 139줄 — 본문은 nexa-sql·nexa-license 판과 동일, 고지 문구만 다름) | `dir2:LICENSE.md:1-140` · DR-6 `dir2:docs/10-decision-record.md:15` · C6 `dir2:docs/05-requirements.md:44` | 계승 — dir3 루트에 `LICENSE.md` 복사 |
| LIC-132 | 필수 고지 | `Required Notice: Copyright (c) 2026 SosomLab — Nexa Dir (<https://github.com/SosomLab/nexa-dir>)` | `dir2:LICENSE.md:3-4,46` | 문구 계승(저장소 URL을 dir3로 바꿀지 **결정 필요**) |
| LIC-133 | 상업 사용 조건 | "상업적 사용은 별도 유료 라이선스 필요 · 개인·비상업 무료 · 문의 **kiros33@sosomlab.com** · https://sosomlab.com" | `dir2:LICENSE.md:6-8` · `dir2:README.md:59` | `LICENSE_CONTACT` = 같은 주소(nsql-license와 일치) |
| LIC-134 | 한글본 | 비공식 번역 · 영문 정본 우선 명시 | `dir2:LICENSE.ko.md:1-12` | 계승 |
| LIC-135 | Cargo 메타 | `license = "LicenseRef-PolyForm-Noncommercial-1.0.0"` · version `0.22.0` · authors · repository `…/nexa-dir2` · homepage `https://sosomlab.com` | `dir2:Cargo.toml:17-22` | 동일 `license` 값 |
| LIC-136 | 의존성 정책 | 퍼미시브 온리(MIT/Apache/BSD/ISC/MPL-2.0) · GPL/AGPL 금지 | `dir2:docs/05-requirements.md:40` · `dir2:CLAUDE.md:24` | 계승 — `ed25519-dalek`(BSD-3) · `sha2`(MIT/Apache) 통과 |
| LIC-137 | About 창 | 행: 제품명(Title) · `about.desc` · 버전 · 링크 3(저장소 · Releases · SosomLab — 클릭 = 브라우저) · `about.license`(Dim) · `about.copyright`(Dim) · [확인] 우하단 · 모달 | `dir2:crates/nexa-app/src/about.rs:241-257,355-390` | 배치 유지 · 링크 열기 = OS 분기(LIC-186) |
| LIC-138 | About 문자열 | `about.license = License: PolyForm Noncommercial 1.0.0`(en) / `라이선스: …`(ko) / `ライセンス: …`(ja) · `about.copyright = © 2026 SosomLab · Sangyong Bae` | `dir2:crates/nexa-app/lang/en.lang:93-94` · `ko.lang:92-93` · `ja.lang:93-94` | 자원 그대로 사용 |
| LIC-139 | 도움말 메뉴 | 항목은 **About 하나뿐**(`CMD_ABOUT=66` → `WM_APP_ABOUT` 지연 실행) | `dir2:crates/nexa-app/src/win.rs:255,478-482,5369-5371,9050-9054` | 라이선스 항목 추가 여부 = LIC-158 |
| LIC-140 | 배포 메타 | 설치본 LICENSE 동의 페이지(`LicenseFile=..\LICENSE.md`) · winget `License`/`LicenseUrl` + 설명문 · chocolatey `requireLicenseAcceptance=true` + 설명문(이메일 금지 → `https://sosomlab.com/apps/nexa-dir/`로 안내) | `dir2:installer/nexa.iss:46` · `dir2:packaging/winget/0.22.0/SosomLab.NexaDir.locale.en-US.yaml:12-13,36-37` · `dir2:packaging/chocolatey/nexa-dir/nexa-dir.nuspec:20-21,49-51` · `dir2:docs/journal/2026-07-19.md:59-64` | 3-OS 패키징 메타에 같은 문구(LIC-176) |
| LIC-141 | X-15 오프라인 라이선스(계획) | **☐ 설계 등록만 — 코드 없음.** ① 서명된 텍스트 파일(exe 옆 `license.key`): 페이로드 = 이름·이메일·에디션·발급일·만료·기기 바인딩 **옵션**(HWID 해시) + ECDSA P-256 ② 검증 = Windows CNG(bcrypt.dll) · 공개키만 임베드 ③ 발급 = 오프라인 CLI `nexa-lic`(배포 제외 · 개인키는 발급 PC) ④ 서명 검증 실패 = **미등록 동작** · "착수 시 ADR" | `dir2:docs/TODO.md:96` · `dir2:docs/journal/2026-07-16.md:370-372` · `dir2:docs/10-decision-record.md:54` · `dir2:docs/19-parity-gap.md:39` | nexa-license 형식으로 **대체**(§4-2) |
| LIC-142 | 집행·게이트 부재 | `crates/` 전체에서 라이선스 관련 코드는 About 문구 1줄과 `Cargo.toml` 상속뿐 — 판정·등급·기능 제한·배지 **없음** | Grep 결과: `dir2:crates/nexa-app/src/about.rs:255` · `dir2:crates/*/Cargo.toml:5` | "전 기능 무료 사용" 상태가 dir2의 실제 동작 |
| LIC-143 | 영속 폴더 전제 | 포터블: exe 옆 `data\` 기본 · 쓰기 불가면 `%LOCALAPPDATA%\NexaDir\data` 폴백(DR-3) — X-15의 "exe 옆 `license.key`"가 이 규칙에 기대고 있었음 | `dir2:crates/nexa-app/src/config.rs:358-392` · `dir2:docs/10-decision-record.md:12` | dir3 설정 구조는 nexa-sql 차용(사용자 지시) → LIC-154 |
| LIC-144 | 끊긴 참조 | `LICENSE.md`가 가리키는 `docs/13-licensing.md`는 nexa-dir2에 **없다**(원본 nexa-dir 저장소 문서 · `docs/12-packaging-single-exe.md:43`도 같은 링크) | `dir2:LICENSE.md:9` · `ls dir2:docs`(12 다음이 15) | dir3 LICENSE에서는 이 문서(42) 또는 dir3 정책 문서로 교체 |

### 1-6. dir3 통합 계획(151~) · OS 분기(181~) · 위험(191~)

§5(계획) · §5-5(OS 분기) · §5-6(위험)에 표로 싣는다.

## 2. nexa-license API

### 2-1. 공개 모듈과 켜야 하는 feature

| 모듈 | feature | 공개 항목 |
| --- | --- | --- |
| crate 루트 | 항상 | `FORMAT` `REQUEST_PREFIX` `SERVER_REQUEST_PREFIX` `MACHINE_DOMAIN` `LICENSE_DOMAIN` `LEASE_DOMAIN`(`lic:lib.rs:39-50`) |
| `format` | 항상 | `Doc{pairs}` · `SIG_KEY` · `Doc::{parse,get,get_numbered,has_duplicate_keys,set,canonical,serialize}` |
| `base32` | 항상 | `encode` `encode_grouped` `decode` |
| `types` | 항상 | `Product` `Kind` `SeatMode` `Alg` |
| `date` | 항상 | `parse_days` `format_days` `add_years` `today_days` |
| `version` | 항상 | `parse` `major` `at_or_after` |
| `verify` | 항상 | `SigVerifier` `License` `Invalid` `Verdict` `verify_license` |
| `request` | 항상 | `Request` `encode` `decode` |
| `keys` | 항상 | `RootKey` `ROOT_KEYS` |
| `ed25519` | `ed25519` | `Ed25519Verifier` |
| `machine` | `machine-id` | `machine_id` `hash_identifier` |
| `fs` | `fs` | `Store` `Found` `Stamp` `write_atomic` |
| `sign` · `envelope` | `issuer`(앱 금지) | `Keypair` `public_key_file` `parse_public_key_file` `keys_rs_source` · `seal` `peek` `open` `hmac_sha256` `pbkdf2_sha256` `DEFAULT_ITER` `EnvelopeError` |

### 2-2. 키 체계

- **루트 키 1쌍(Ed25519)** 이 계열 전 제품의 라이선스에 서명한다. 앱에는 공개키만(`ROOT_KEYS` · `lic:keys.rs:17-28`), 비밀키는 발급 PC의 암호 봉투 `nxk1`에만 있다(`sql-doc:91-license-root-key-operations.md:8-12`). 현재 등재 = `root-v1`(`sql-doc:91…:23`).
- 파일의 `key=`가 있으면 그 id의 키만, 없으면 같은 `alg`의 모든 키를 시도한다(`lic:verify.rs:104-118`). 회전 = `ROOT_KEYS`에 키를 추가(`sql-doc:91…:78-83`).
- 서명 대상 = `Doc::canonical("nexa/license/v1")` — 제품 이름은 도메인이 아니라 본문 `product=`에 있다(`lic:lib.rs:11` · `sql-doc:25-license-tiers-and-server.md:308`). 따라서 **dir3 전용 키·도메인·형식 변경이 필요 없다.**
- `Alg::P256`은 열거만 있고 검증 어댑터·루트 키·발급 코드가 없다(`lic:types.rs:113-137` · `lic:ed25519.rs:11-13` · `lic:sign.rs:111`의 `Alg::Ed25519` 고정). dir2용 "CNG P-256 어댑터" 계획(`sql-doc:25…:311,339`)은 **구현된 적이 없다.**

### 2-3. 검증 흐름(`verify_license` · `lic:verify.rs:79-184`)

```
1. format == "nxl1"                     아니면 Invalid(Format)          :88-90
2. product 필드가 Product.accepts()      아니면 Invalid(Product)         :91-93
3. 중복 키 없음                          있으면 Invalid(DuplicateKeys)   :94-96
4. sig base32 디코드 · alg 파싱          실패 Invalid(Malformed)         :97-103
5. 루트 키 후보(alg 일치 · key id 일치)   없으면 Invalid(NoRootKey)       :104-112
6. 후보 중 하나로 서명 검증               실패 Invalid(Signature)         :113-118
7. machine[.N] 줄이 있으면 이 PC 기기 ID가 그 안에(대소문자 무관)
                                         없음/기기 ID 못 얻음 Invalid(Machine)   :119-132
   machine 줄이 없으면 기기 검사 생략(kind=org 파일)                      :124
8. kind 파싱                             실패 Invalid(Malformed)         :133-136
9. 빌드일 > updates_until               Outdated                        :160-167
10. 앱 Major > max_major                Outdated                        :169-173
11. 앱 버전 ≥ max_version               Outdated                        :174-177
12. 오늘 > expires                      Expired(당일은 유효)             :178-182
13. 그 외                               Licensed(License)               :183
```

- 판정 기준 시각: 영구 조항(`updates_until`)은 **빌드일**(시계 되돌리기 무효), 만료형(`expires`)만 시스템 시계(`sql-doc:23-license-activation.md:84`).
- `features=*` = 전 기능(이후 추가 기능 포함 · D-48 · `sql-doc:25…:552`), 그 밖은 쉼표 낱개. 앱이 모르는 이름은 무시.

### 2-4. 요청 코드

- 형식: `NEXAREQ1.<base32(기기 ID 20B)>.<base32(메타 Doc 직렬화)>`(`lic:request.rs:15-25`). 메타가 비면 뒤 절 생략.
- 메타 키(앱 층이 채움 · `nsql-lic:src/lib.rs:503-512`): `os`(`windows`/`macos`/`linux`) · `app`(`<제품 id>/<버전>`) · `d`(오늘) · `n`(이름 · 선택) · `e`(이메일 · 선택). 서명 대상은 기기 ID뿐이고 메타는 표시·대장용이다(`lic:request.rs:2`).
- 발급기는 **`app` 값의 마지막 `/` 뒤를 버전으로 읽어 `max_major` 기본값**을 정한다(`tool:main.rs:537-542,666-674`). dir3가 `app` 형식을 바꾸면 발급기가 `--max-major`를 요구하게 된다.

### 2-5. 발급 도구 사용법(코드 기준 — 문서와 다른 곳은 §5-6)

```bash
T=<nexa-license>/target/release/nexa-license-tool        # cargo build --release -p nexa-license-tool
cd ~/nexa-issuer                                          # 기본 --out ./issued · 대장 ./issued/ledger.tsv
read -s NEXA_LICENSE_KEY_PASS && export NEXA_LICENSE_KEY_PASS
$T decode-request 'NEXAREQ1.…'                            # os · app(nexa-dir/<ver>) · n · e 확인(종료 3 = 불량)
$T issue --key root-v1.key --pass-env NEXA_LICENSE_KEY_PASS \
   --product nexa-dir --id-prefix NDL \                   # ★ 지금은 둘 다 직접 줘야 한다(기본 nexa-sql / NSL)
   --request 'NEXAREQ1.…' --kind user --tier pro --licensee "이름" [--email …] [--note 주문번호] [--no-mail]
$T verify issued/<ID>/nexa-dir.license --pub root-v1.key.pub --machine <기기 코드> --app-version <dir3 버전>
```

- 기본 조항: `expires` = 발급일 + 3년(`trial` = 14일 · `--expires none` = 영구) · `updates_until` = `expires` · `max_major` = 요청 앱의 Major(`none` = 제한 없음) · `max_version` 없음(`tool:main.rs:636-681`).
- `--kind device` 1대 · `user`/`team-seat` ≤5대(`--request` 반복) · `org`는 기기 없음 + `--seats N` 필수(`tool:main.rs:583-594,632-635`).
- 기기 추가/갱신 = `reissue --id <ID> --add-request <코드>` / `--renew`(`tool:main.rs:857-962`). 이미 대장에 있는 ID로 `issue` = 거부(`tool:main.rs:688-694`).
- 루트 비밀키 봉투는 **어느 저장소에도 없다** — 실제 발급은 운영자(사용자)의 발급 PC에서만 가능하다(`sql-doc:91…:27-31`). 자동화 구현·테스트는 시험 전용 키(`Licensing::with_roots`)로 한다(`nsql-lic:src/lib.rs:403-414,569-582`).

## 3. nexa-sql 앱 층

### 3-1. 계층

```
nexa-license (형제 저장소 · 형식/서명/기기 ID/파일 자리)
   └─ nsql-license (앱이 아는 것: PRODUCT · Feature · LicenseState · check · 폴더 순서 · 요청 코드)   nsql-lic:src/lib.rs
        ├─ nexa-sql GUI: App.licensing + LicenseWin + app/license.rs(보기·설치·제거·게이트)          sql-app:license_win.rs · app/license.rs
        └─ nsql-cli: nsql license …                                                                  sql:nsql-cli/src/license.rs
```

- 판정 상태 넷(Free · Invalid · Outdated · Expired)은 전부 features = ∅이고 차이는 안내 문구뿐(`nsql-lic:src/lib.rs:8`).
- 루트 키가 비어 있으면 모든 파일이 `Invalid(NoRootKey)` = Free 취급(`nsql-lic:src/lib.rs:9`).
- 게이트 위치 규칙: Core 계층은 라이선스를 모른다 · UI 행위 진입점 **기능당 정확히 한 곳** · 메뉴는 숨기지 않고 안내(`sql-doc:23…:182-192`).
- **게이트는 기본 끔**(D-145): 라이선스가 없어도 전 기능 동작, 숨은 설정 `license.gates`를 켤 때만 적용(`sql-app:app/license.rs:196-206` · `sql-doc:23…:6`).

### 3-2. 활성화(라이선스) 창 UI 배치 — `sql-app:license_win.rs:395-564`

```
┌ Nexa SQL — License ──────────────────────────────── 640×540(최소 520×380) ┐
│ [상태 띠]  Free · non-commercial use only            ← ok/warn 색 · 알파 0.12 배경
│ 상태         free                                     ← (라벨 dim, 값) 표 · 값은 가운데 생략
│ 파일         -
│ (라이선스 ID / 사용자 / 종류·등급 / 기능 / 발급일 / 업데이트 기한 / 만료 / 유효 메이저 / 무효 시작 버전)
│ 앱 버전      0.1.3
│ 빌드일       2026-10-02
│ 기기 코드    6QJ3KPN0…
│ 설치 자리    <설정 폴더>/license/nexa-sql.license
│ ───────────────────────────────────────────────────  ← 1px 구분선
│ 요청 코드 — 이 한 줄을 kiros33@sosomlab.com 로 보내면 …   ← 이메일 = 링크(밑줄·hover 손·클릭 복사) + Flash
│ 이름 [__________]  이메일 [______________]  [요청 코드 복사]
│ NEXAREQ1.6QJ3…                                        ← dim · 가운데 생략
│ 라이선스는 로컬에서 검증하는 서명된 파일입니다 · 앱은 서버에 접속하지 않습니다.
│                                                       ← 창 높이는 여기까지 자동 맞춤
│ [라이선스 파일 열기…] [제거] [닫기]   설치됨: NSL-…     ← 맨 아래 고정 · 결과 안내는 같은 줄 오른쪽
└───────────────────────────────────────────────────────────────────────────┘
```

- 쓰는 nexa-ui 항목: `nexa_ctl::{Button, TextBox, Flash, FlashTone, Control, Widget, InputEvent, Invalidations}` · `nexa_ctl::draw::{DrawCtx, FontSlot, ellipsize_middle}` · `nexa_ctl::raster::RasterCtx` · `nexa_ctl::theme::{FontPrefs, Theme}` · `nexa_ctl::geom::{Point, Rect}` · `nexa_gfx::{Font, Surface}`(`sql-app:license_win.rs:7-12`).
- 쓰는 **앱 내부 헬퍼**(nexa-ui가 아님): `crate::present::Presenter` · `crate::winfocus::{focus, owned_by, attach_child}` · `crate::wingeom::centered_over` · `crate::icon::with_icon` · `crate::input::{system_ime, shortcut_letter, text_key_event, TextKeys}` · `clipboard::write_text` · `file_win`(`nexa_dlg::FilePicker`)(`sql-app:license_win.rs:103,115-123,370-383` · `sql-app:file_win.rs:11`).
- 표·링크는 **컨트롤이 아니라 창이 직접 그린다**(`dc.text` + 밑줄 `fill_rect` + `link_rect` 히트 테스트 · `sql-app:license_win.rs:438-466`).

### 3-3. 동작 흐름

| 사용자 행위 | 처리 | 근거 |
| --- | --- | --- |
| Help ▸ License… / 상태줄 배지 클릭 / About [License…] | `open_license = true` → 다음 틱에 `open_license_window`(먼저 `licensing.refresh()`) | `sql-app:app/menus.rs:885-890` · `app/input.rs:646-647` · `app/event_loop.rs:1496-1497,1664-1665` · `app/license.rs:8-23` |
| [라이선스 파일 열기…] | 파일 창(`.license` 필터) → `Licensing::install` → 통과 시 `<사용자 폴더>/license/<id>.license` 원자적 쓰기 · 거부 시 사유 안내(파일 미변경) | `app/event_loop.rs:970,1517-1521` · `app/license.rs:157-168` |
| [제거] | 사용자 폴더 파일 삭제 → 재판정 | `app/license.rs:170-179` |
| [요청 코드 복사] | 이름·이메일을 메타로 넣은 코드 → 클립보드 | `app/license.rs:181-194` |
| 이메일 링크 클릭 | 주소 클립보드 + Flash | `license_win.rs:299-302` · `app/event_loop.rs:1524-1534` |
| 밖에서 파일 변경(CLI 설치 등) | 틱마다 `refresh()` → 배지·창 다시 그림 | `app/event_loop.rs:566-568` |
| 게이트 거부 | 상태줄 안내 + 창 note + 창 열기 | `app/license.rs:208-219` |

## 4. dir2 라이선스 정책(근거 인용)

### 4-1. 확인된 정책(전부 인용 가능)

| # | 정책 | 원문 | 근거 |
| --- | --- | --- | --- |
| P1 | 본체 = PolyForm Noncommercial 1.0.0 | "본체 **PolyForm Noncommercial 1.0.0**(개인무료/상업유료) · 의존성 **퍼미시브 온리**(GPL/AGPL 금지)" | `dir2:docs/10-decision-record.md:15`(DR-6) |
| P2 | 상업 사용 = 별도 유료 | "**상업적 사용(commercial use)은 별도 유료 라이선스가 필요합니다.** 개인·비상업 용도는 본 라이선스로 무료. 상업 라이선스 문의: **kiros33@sosomlab.com**" | `dir2:LICENSE.md:6-8` |
| P3 | 허용 목적 | Noncommercial Purposes · Personal Uses · Noncommercial Organizations(자선·교육·공공 연구·정부 등) | `dir2:LICENSE.md:60-79` |
| P4 | README 고지 | "개인·비상업 무료, 상업 사용은 유료 라이선스(문의 kiros33@sosomlab.com)" | `dir2:README.md:59` |
| P5 | 배포 문구 | "Nexa Dir is free for personal and other noncommercial use … Commercial use requires a separate paid license." | `dir2:packaging/winget/0.22.0/SosomLab.NexaDir.locale.en-US.yaml:36-37` |
| P6 | 앱 안 표시 | About 창 `License: PolyForm Noncommercial 1.0.0` + `© 2026 SosomLab · Sangyong Bae` | `dir2:crates/nexa-app/lang/en.lang:93-94` |
| P7 | 오프라인 인증 계획(X-15) | "유출 방지 + 발급 용이한 오프라인 활성화 … 서명된 텍스트 파일 … 공개키만 exe에 임베드 … 발급 = 오프라인 CLI … 서명 검증 실패 시 미등록 동작" | `dir2:docs/TODO.md:96` |
| P8 | 집행 코드 없음 | 라이선스 판정·등급·기능 제한 코드 0 | LIC-142 |

- nexa-sql의 계열 정책과 같은 뿌리다: "nexa-clip 등과 동일하게 영리 목적인 경우에만 라이선스 구매"(`sql-doc:13-licensing.md:3`), 무료 배지 `Free · non-commercial use only`(`sql:nsql-i18n/src/lib.rs:3401`).
- nexa-sql 쪽 설계 문서는 이미 "dir2 X-15가 잡은 형식은 이 라이브러리 형식으로 **대체**된다"고 적었다(`sql-doc:25-license-tiers-and-server.md:341`).

### 4-2. X-15 계획 → nexa-license 사상(계승/변경/결정 필요)

| X-15 항목(`dir2:docs/TODO.md:96`) | nexa-license 대응 | 판정 |
| --- | --- | --- |
| 서명된 텍스트 파일 | `nxl1` key=value + `sig`(LIC-004·005) | 계승(형식 대체) |
| 이름 · 이메일 | `licensee=` · `email=` | 계승 |
| 발급일 · 만료 | `issued=` · `expires=`(발급기 기본 3년 · `none` = 영구) | 계승 — **기본 기간은 결정 필요**(dir2는 값을 정하지 않음) |
| 에디션 | `tier=`(표시용) + `features=`(판정용) | **결정 필요** — dir2는 에디션 이름·구성을 정의하지 않았다 |
| 기기 바인딩 **옵션**(HWID 해시) | `machine[.N]=`(kind=device/user/team-seat) · 없으면 기기 검사 생략(kind=org) | 계승 — **필수/선택은 결정 필요**(LIC-172와 연결) |
| ECDSA P-256 + Windows CNG(외부 crate 0 · B3) | Ed25519(`ed25519-dalek`) | **변경** — CNG는 Windows 전용이라 크로스플랫폼 불가 · 루트 키도 Ed25519뿐(LIC-173) |
| exe 옆 `license.key`(포터블 DR-3) | `<설정 폴더>/license/nexa-dir.license`(+ 기기 공용 폴더) | **변경** — 설정 구조는 nexa-sql 차용(사용자 지시) · 포터블 자리 추가는 결정 필요(LIC-154) |
| 발급 CLI `nexa-lic`(별도 크레이트) | `nexa-license-tool` | 대체(`sql-doc:25…:339`) |
| 위변조 = 검증 실패 시 미등록 동작 | `Invalid(_)` → Free 취급 + 안내 | 계승(동작 일치) |
| 공개키만 임베드 · 개인키는 발급 PC | `ROOT_KEYS` · 봉투 `nxk1` | 계승 |

### 4-3. Feature/등급 매핑 제안

**dir2에서 근거를 찾을 수 있는 것은 "전 기능 무료 + 상업 사용자는 라이선스 필요"까지다.** 그 너머는 전부 "결정 필요"로 둔다.

| 항목 | 제안 | 근거 | 상태 |
| --- | --- | --- | --- |
| 무료(Free) 범위 | **dir2의 전 기능** — 라이선스 없이 그대로 동작 | P8(dir2에 게이트 0) · nexa-sql D-145(`sql-doc:10-decision-record.md:80`) | 확인(dir2 실제 동작) |
| 무료 상태 표시 | 배지/창에 `Free · non-commercial use only`(무료 · 비상업용) | P2 · `sql:nsql-i18n/src/lib.rs:3401` | 문구는 정책과 일치 · **표시 위치는 결정 필요**(LIC-158) |
| 라이선스의 의미 | "상업 사용 자격의 증명"(기능 잠금 해제가 아님) | P2 · `sql-doc:25…:470`("기업 = 기능이 아니라 사용 조건으로 구매") | 제안 |
| 게이트 스위치 | 설정 `license.gates`(HIDDEN · 기본 off) 자리만 두고 **배선은 0곳** | LIC-112 · P8 | 제안 |
| `Feature` 목록 | 초기 = **게이트 대상 없음**. 열거형은 dir3 기능 인벤토리(문서 10~19)에서 후보를 고른 뒤 확정 | dir2에 유료 기능 정의 없음 | **결정 필요** |
| `Tier` | `Free/Trial/Pro/Org`(nsql-license와 동일 · 발급기 프리셋 `trial\|pro\|org\|team`이 전 제품 공통 · `tool:presets.rs:10-13`) | X-15 "에디션" 필드만 존재 | **결정 필요**(이름·종류) |
| `Kind`(발급 단위) | `device`(1대) · `user`(5대) · `team-seat` · `org` — 계열 공통(DR-26 · `sql-doc:10-decision-record.md:34`) | X-15는 "기기 바인딩 옵션"만 언급 | **결정 필요**(dir 제품에 4종 모두 팔지) |
| 유효기간·버전 조항 | 발급기 기본(3년 · `max_major` = 요청 앱 Major) 수용 | `tool:main.rs:636-675` | **결정 필요**(dir3 버전 체계와 함께 — LIC-177) |
| 가격·재발급 횟수 | — | dir2 문서에 없음 | **결정 필요**(범위 밖) |

## 5. dir3 통합 계획

### 5-1. `ndir-license` 크레이트(151~157 · 164)

| ID | 항목 | 내용 | 근거/주의 |
| --- | --- | --- | --- |
| LIC-151 | 크레이트 신설 | `crates/ndir-license`(lib) — `nsql-license/src/lib.rs`를 1:1 복제 후 제품 상수·`Feature`·설정 크레이트만 교체. `version.workspace = true`(앱 버전 = `Product.version`이 되도록) · `build = "build.rs"` | `nsql-lic:Cargo.toml:1-22` |
| LIC-152 | `PRODUCT` | `Product{ id: "nexa-dir", build_date: env!("NDIR_BUILD_DATE"), version: env!("CARGO_PKG_VERSION") }` · 파일명 `nexa-dir.license` · 요청 메타 `app = "nexa-dir/<버전>"` | 제품 id 근거: `lic:types.rs:8`·`tool:presets.rs:10`이 계열 id로 `nexa-dir`를 이미 열거 · dir2 제품명 정리 "nexa-dir2→nexa-dir"(`dir2:crates/nexa-app/src/config.rs:378`). **`nexa-dir`(dir2와 같은 제품) vs `nexa-dir3`(별도 제품)은 결정 필요** — 추천 `nexa-dir` |
| LIC-153 | 빌드일 | `build.rs` 복제: `NDIR_BUILD_DATE` → `SOURCE_DATE_EPOCH` → 지금 | `nsql-lic:build.rs:6-51` · 릴리스 CI에서 재현 빌드 값 주입 가능 |
| LIC-154 | 폴더 순서 | `user_dir()` = `<dir3 설정 폴더>/license`(설정 폴더 = dir3 설정 크레이트의 `config_dir()` — `NSQL_HOME`에 해당하는 격리 환경 변수 포함) → `machine_dir()`(LIC-182) | `nsql-lic:src/lib.rs:308-343` · `sql:nsql-settings/src/lib.rs:154-160`. **포터블(exe 옆) 자리를 목록에 넣을지는 결정 필요**(dir2 DR-3 · `sql-doc:25…:374` "exe 옆"도 자동 복원 후보로 언급) |
| LIC-155 | `Feature` · `Tier` | §4-3대로: `Tier`는 복제 · `Feature`는 **결정 필요**. 구현 제안 = 열거형 + `ALL`/`as_str`/`parse` 골격과 `check` API를 유지하되 변형은 결정 뒤 채움(결정 전에는 창·배지·설치/제거/요청만 동작해도 정책 P1~P8을 만족) | `nsql-lic:src/lib.rs:52-158` — 변형 0개 열거형은 `check` 시험을 쓸 수 없으므로, 결정 전 임시 변형을 둘지 여부도 함께 결정 |
| LIC-156 | 게이트 설정 | dir3 설정 레지스트리에 `license.gates`(Bool · 기본 off · HIDDEN) | LIC-112 · P8 |
| LIC-157 | 상태·판정·설치 API | `LicenseState` · `DenyReason` · `Denial` · `Entitlement` · `entitlement` · `judge` · `Licensing{open_default, open, with_roots, state, path, primary_path, dirs, check, refresh, judge_text, install, install_text, remove, machine_code, request_code}` · `InstallError` · `RequestMeta` · `features_of` · `LICENSE_SUBDIR` · `LICENSE_CONTACT` — 시그니처 그대로 | LIC-067~077 |
| LIC-164 | 회귀 테스트 | ① `nsql-license` 단위 시험 7건 이식(제품 id만 교체 · 시험 키 = dev-dep dalek) ② 기동 명령 `license.dump:<파일>` · `license.install:<파일>` + 창 `dump()` ③ "다른 제품(`nexa-sql`) 파일 = `Invalid(Product)`" · "`product=*` 번들 = 통과" ④ 격리 홈(환경 변수)에서 실행 — 실제 사용자 폴더를 건드리지 않음 | `nsql-lic:src/lib.rs:552-824` · `sql-app:app/startup_cmd.rs:332-345` · `lic:types.rs:29-36` |

### 5-2. 활성화 창·진입점(158~)

| ID | 항목 | 내용 | 근거/주의 |
| --- | --- | --- | --- |
| LIC-158 | 진입점 | ⓐ 도움말 메뉴에 "라이선스…" 항목 추가(About 아래) ⓑ About 창에 라이선스 상태 줄 + [라이선스…] 버튼 ⓒ 상태줄 배지 — **셋 다 dir2에 없던 UI**(dir2 도움말 = About 하나 · `dir2:crates/nexa-app/src/win.rs:478-482`). "컨트롤 배치 유지" 지시와 충돌하는 유일한 지점 → **결정 필요**. 추천 = ⓐ + ⓑ(최소 변경), ⓒ는 dir2 상태줄 배치를 바꾸므로 보류 | LIC-104·105 · LIC-137·139 |
| LIC-160 | 창 구현 | `license_win.rs`(565줄) 복제 — 배치·수치는 §3-2 그대로. 제목 `Nexa Dir — <라이선스>` · 연락처 = `LICENSE_CONTACT` | `sql-app:license_win.rs` 전체 |
| LIC-161 | 보기 조립 | `license_view()`·`license_badge_of()`·`license_install/remove/copy_request` 복제 | `sql-app:app/license.rs:55-194` |
| LIC-159 | i18n | dir2 자원 유지 = `.lang` 파일(en/ko/ja). nexa-sql 문자열(영/한 2열 · LIC-113)을 `license.*` 키로 옮기고 **일본어 번역을 새로 써야 한다**(dir2는 3개 언어 · nexa-sql은 2개) | `dir2:crates/nexa-app/lang/{en,ko,ja}.lang` · `sql:nsql-i18n/src/lib.rs:3349-3436` |
| LIC-162 | CLI/기동 인자 | nexa-sql은 `nsql license …` CLI가 있고 발급기 안내문도 그것을 가리킨다. dir2에는 CLI 바이너리가 없다 → dir3에서 ⓐ GUI만 ⓑ 실행 인자(`--license-install <파일>` 등) 추가 중 **결정 필요**. 테스트용 기동 명령(LIC-164 ②)은 어느 쪽이든 필요 | `sql:nsql-cli/src/license.rs:24-62` · `tool:presets.rs:41-49` |
| LIC-163 | LICENSE 파일 | dir3 루트에 `LICENSE.md`·`LICENSE.ko.md`(dir2 판 복사 · 고지 문구/URL은 LIC-132 · 끊긴 `docs/13` 참조는 LIC-144) · `Cargo.toml` `license = "LicenseRef-PolyForm-Noncommercial-1.0.0"` · README 라이선스 절 | `dir2:LICENSE.md` · `dir2:Cargo.toml:19` · `dir2:README.md:51-59` |
| LIC-174 | nexa-ui 보강 후보 | 라이선스 창은 **표(라벨/값 행)와 링크를 직접 그린다** — nexa-ui에 독립 컨트롤이 없다(`LinkStyle`/`LinkLine`은 TextBox 내부 링크용 · `ui:nexa-ctl/src/controls/textbox.rs:67-89`). dir2 About도 링크 행을 쓴다(`dir2:crates/nexa-app/src/about.rs:47,210-223`). "컨트롤은 전부 nexa-ui" 기조면 **`LinkLabel`(밑줄·hover 커서·클릭) · `KeyValueList`(라벨/값 표)** 를 nexa-ui에 추가해 두 창이 공유 | `sql-app:license_win.rs:429-466` |
| LIC-175 | 앱 셸 헬퍼 | 창 생성/소유/가운데 배치/Presenter/입력 변환/클립보드/파일 창은 nexa-sql **앱 크레이트 내부 모듈**이다 — dir3 앱 셸(다른 기준 구조 문서 범위)이 같은 헬퍼를 먼저 갖춰야 라이선스 창을 옮길 수 있다 | §3-2 |
| LIC-176 | 패키징 메타 | 3-OS 패키지(설치본·winget·choco·macOS/Linux 번들)에 LIC-140과 같은 라이선스 문구·동의 페이지 | `dir2:installer/nexa.iss:46` 외 |

### 5-3. nexa-license 변경 필요분(165~)

**검증 라이브러리(`crates/nexa-license`)는 변경 없음.** 아래는 전부 발급기·문서다.

| ID | 변경 | 현재 | 제안 | 필수도 |
| --- | --- | --- | --- | --- |
| LIC-165 | `mail_text` 제품별 안내 | `nsql license install/status/request` 하드코딩(`tool:presets.rs:39-49`) | `product`로 분기 — `nexa-dir`는 "도움말 ▸ 라이선스… ▸ 라이선스 파일 열기…" + "요청 코드 복사" 안내(CLI 유무는 LIC-162 결정에 따름) | **필수**(안 고치면 dir3 고객에게 없는 명령을 안내 · 임시 회피 = `--no-mail`) |
| LIC-166 | ID 접두 기본값 | `--id-prefix` 기본 `"NSL"`(`tool:main.rs:685`) | 제품별 기본(예: `nexa-dir` → `NDL`) 또는 운영 절차에 `--id-prefix` 명시 | 권장 — **접두 문자열은 결정 필요** |
| LIC-167 | tier 프리셋 | 전 제품 `*`(`tool:presets.rs:9-15`) | dir3 `Feature`가 확정될 때까지 그대로(`*` = 이후 추가 기능 포함 · D-48) | 지금은 불필요 |
| LIC-168 | 대장의 제품 구분 | `ledger.tsv`에 product 열 없음(`tool:ledger.rs:6`) · 파서가 `f.len() < 14`로 열 수를 고정(`:63`) | ⓐ 제품별 `--out` 폴더로 분리(코드 변경 0) ⓑ 15번째 열 `product` 추가(옛 14열 행 호환 처리 필요) | 권장 — **결정 필요** |
| LIC-169 | 문서·주석 정정 | README "nexa-dir2 → `ed25519` feature를 끄고 `alg=p256` CNG 어댑터"(`lic-root:README.md:64`) · `lic:lib.rs:5,9` · CLAUDE.md "검증 전용 — 서명 생성은 비공개 서버 저장소에만"(`lic-root:CLAUDE.md:7` — 09-27 이후 사실과 다름) | dir3 = `ed25519 + machine-id + fs` 소비자로 고쳐 적기 | 권장 |
| LIC-170 | E2E에 제품 케이스 | 전부 `nexa-sql`(`lic-root:crates/nexa-license-tool/tests/e2e.rs:19-25,131`) | `--product nexa-dir` 발급 → `nexa-dir.license` 생성 · `app=nexa-dir/<ver>` 메타에서 Major 추출 · mail.txt 문구 검사 | 권장(LIC-165와 한 묶음) |
| LIC-171 | CI 형제 체크아웃 | — | dir3 CI에 `SosomLab/nexa-license`(+ nexa-ui) 체크아웃 단계 | 필수(`nexa-sql/.github/workflows/ci.yml:31-34` 방식) |
| LIC-178 | 원복 기준 태그 | nexa-license·nexa-ui 둘 다 `baseline/pre-nexa-dir3-2026-10-03` 태그가 **이미 있다**(nexa-license 태그 = HEAD `4f02524` · 작업 트리 깨끗 — 조사 시점 관측) | LIC-165~170을 커밋하기 전 상태로 원복 가능 · 변경 후 push + 새 태그는 구현 단계에서 | 정보 |

- nexa-license 저장소 규약: Conventional Commits · `git add <파일>`만 · 진행 기록은 nexa-sql journal에 남긴다(`lic-root:CLAUDE.md:9`) — dir3 작업 중 변경분의 기록 위치는 **결정 필요**(dir3 journal 권장).

### 5-4. 구현 순서 제안

1. `ndir-license`(LIC-151~157) + 단위 시험(LIC-164 ①③) — UI 없이 완결.
2. 설정 키(LIC-156) · i18n 키(LIC-159).
3. 앱 셸 헬퍼가 준비되면(LIC-175) 라이선스 창(LIC-160·161) + 진입점(LIC-158) + 기동 명령(LIC-164 ②).
4. nexa-license 발급기 변경(LIC-165·166·170) → fmt/clippy/test 게이트(LIC-029) → push + 태그.
5. LICENSE/README/패키징 메타(LIC-163·176).

### 5-5. OS 분기 지점(181~)

| ID | 분기 | Windows | macOS | Linux | 근거 |
| --- | --- | --- | --- | --- | --- |
| LIC-181 | 기기 ID 원천 | `reg query HKLM\SOFTWARE\Microsoft\Cryptography /v MachineGuid`(`CREATE_NO_WINDOW`) | `ioreg -rd1 -c IOPlatformExpertDevice`의 `IOPlatformUUID` | `/etc/machine-id` → `/var/lib/dbus/machine-id` | `lic:machine.rs:24-71` — 라이브러리 내장 · 그 외 OS = None |
| LIC-182 | 기기 공용 폴더 | `%ProgramData%\nexa\<product>` | `/Library/Application Support/nexa/<product>` | `/etc/nexa/<product>` | `nsql-lic:src/lib.rs:316-337`(읽기만 · 설치는 관리자가 손으로) |
| LIC-183 | 파일 권한 | — | 0600 | 0600 | `lic:fs.rs:160-164` |
| LIC-184 | 주 수정 키 | Ctrl | Cmd(`super_key`) | Ctrl | `sql-app:license_win.rs:245-252` |
| LIC-185 | 클립보드 쓰기 | 앱 모듈 `clipboard` | 같음 | X11 전용 모듈 별도(`clipboard_x11`) | `sql-app:main.rs:17,19` · `sql-app:clipboard.rs:20` |
| LIC-186 | 링크 열기(About) | dir2 = `ShellExecuteW("open")` | 대응 필요 | 대응 필요 | `dir2:crates/nexa-app/src/about.rs:166-178` · nexa-sql은 "OS 연결 프로그램으로 열기" 함수 보유(`sql-app:main.rs:1191`) |
| LIC-187 | 사용자 설정 폴더 | `user_config_dir(<앱>)` 또는 격리 환경 변수 | 같음 | 같음 | `sql:nsql-settings/src/lib.rs:154-160` — dir2의 exe 옆 `data\`(Windows 포터블)와 다름(LIC-154) |
| LIC-188 | 창 소유·포커스 | `winfocus::{owned_by, attach_child, focus}` | 같음(구현은 OS별) | 같음 | `sql-app:license_win.rs:103,115,124` · `sql-app:app/license.rs:17-21`(구현 세부는 앱 셸 문서 범위 — 이 문서에서는 미확인) |
| LIC-189 | 발급기 암호 입력 | 에코 끔 없음(프롬프트 입력이 보임) | `stty -echo` | `stty -echo` | `tool:main.rs:208-220` — 발급 PC 운영 주의(`--pass-env` 권장) |

### 5-6. 위험·불일치(191~)

| ID | 내용 | 근거 | 조치 |
| --- | --- | --- | --- |
| LIC-172 | `kind=org` 파일은 `machine` 줄이 없어 **어느 PC에 설치해도 기기 검사 없이 정식** — nexa-sql에서도 결정 대기(D-231 · 서버 구현 전까지 앱이 거부하는 안 권장) | `lic:verify.rs:124` · `sql-doc:25…:154` | dir3에서도 같은 구멍 → **결정 필요**(앱 층에서 `Kind::Org` 단독 설치 거부 여부) |
| LIC-173 | dir2용 P-256/CNG 계획(D-40)은 미구현이며 dir3(크로스플랫폼)에는 맞지 않는다 → Ed25519로 통일. 단 D-40은 nexa-sql 결정 기록에 "열린 결정"으로 남아 있다 | `sql-doc:10-decision-record.md:134` · LIC-012 | dir3 문서에 "Ed25519 채택"을 결정으로 기록 · nexa-license README 정정(LIC-169) |
| LIC-177 | `max_major` 기본 = 요청 앱의 Major. dir3 버전이 0.x로 시작해 1.0이 되는 순간 **0.x 때 발급한 라이선스가 전부 Outdated**(설계 의도이나 고객 영향 큼). dir2는 0.22.0(`dir2:Cargo.toml:17`) | `tool:main.rs:656-675` · `lic:verify.rs:169-173` | dir3 버전 체계 **결정 필요** |
| LIC-191 | 요청 메타 `app` 형식 의존 — `<id>/<버전>`이 아니면 발급기가 Major를 못 읽어 `--max-major` 필수 오류 | `tool:main.rs:537-542,666-674` | `request_code` 구현 시 형식 고정 + 시험 |
| LIC-192 | `Product.version` = `env!("CARGO_PKG_VERSION")`는 **ndir-license 크레이트의** 버전이다 — 워크스페이스 버전 상속을 빼먹으면 앱 버전과 어긋나 버전 조항 판정이 틀린다 | `nsql-lic:src/lib.rs:39` · `nsql-lic:Cargo.toml:3` | `version.workspace = true` + 시험 |
| LIC-193 | 문서와 코드 불일치(코드가 정본): ① 운영 문서 "`--updates-until` 기본 = 오늘+1년 · `--expires` 기본 없음" ↔ 코드 3년 ② 문서 `keygen --show` ↔ 코드에 없음 ③ 문서 `--key-pass-env` · `--kind team` ↔ 코드 `--pass-env` · `team-seat` ④ 문서 `format=nsl1`·`NSQLREQ1` ↔ 코드 `nxl1`·`NEXAREQ1` | `sql-doc:91…:53,74` ↔ `tool:main.rs:639-650,263-325` · `sql-doc:25…:407,416,427` ↔ `tool:main.rs:59,555` · `sql-doc:23…:39,105` ↔ `lic:lib.rs:40-42` | dir3 발급 절차서는 §2-5(코드 기준)를 따른다 |
| LIC-194 | 실제 라이선스 발급은 자동화 범위 밖 — 루트 비밀키 봉투·암호가 저장소에 없다 | `sql-doc:91…:27-31,66` | 테스트는 시험 키로 · 실 발급 확인은 사용자 수동 단계로 표기 |
| LIC-195 | 컨테이너/일부 Linux는 `machine-id`가 없거나 이미지마다 달라 요청 코드를 만들 수 없다(`None`) | `lic:machine.rs:8,24-33` · `sql-doc:23…:48` | 창은 "기기 ID를 얻을 수 없음" 안내 + 복사 버튼 비활성(LIC-096·097) · CI 시험은 건너뜀 분기(`nsql-lic:src/lib.rs:800-806`) |
| LIC-196 | 한 대장에 두 제품을 섞으면 ID 순번·검색이 섞인다(product 열 없음) | LIC-050 | LIC-166·168 |
| LIC-197 | 창 추가로 dir2 UI에 없던 요소(메뉴 항목·버튼·배지)가 생긴다 — "배치 유지" 원칙의 예외 | LIC-158 | 사용자 결정 뒤 구현 · 교차 검증 때 "의도된 차이"로 등재 |
