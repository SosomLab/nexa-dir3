# 26. 대량 전송 엔진 설계 — `ndir-xfer` · `nexa-copy` · (후) cdylib

> T-97 · DR-17(대량 전송 엔진 내장) · DR-34 ①⑦(10-31 사용자 결정) · 원장 NEW-006 · NEW-007. 앞 문서 = [24 빠른 복사](24-fast-copy.md)(§6 전략 계층 · §7 `nexa-copy` 1차).
> 상태: **10-31 협업 작성 · §4~§6 = `crates/ndir-xfer` 골격 코드에 맞춤**(spec.rs · report.rs · lib.rs · 시험 8/0). §11 결정 거리는 열려 있다.

## 1. 목적과 범위

- **목적**: 앱 · 명령행 · 다른 프로그램이 **같은 전송 엔진을 같은 규칙으로** 쓰게 한다. 작업을 **직렬화 가능한 한 덩어리(JobSpec)**로 표현해 저장 · 재실행 · 다른 프로세스 전달이 되게 하고, 진행과 결과도 기계가 읽는 모양(Progress · Report)으로 낸다.
- **이번 회차(DR-34 ①⑦)**: ① 이 설계 문서 ② `ndir-xfer` rlib 골격 — JobSpec/Progress/Report 직렬화 · 복사 · 이동 · 검증 · 미러 4엔진 포장 · dry-run ③ `nexa-copy` `--job job.json` · `--progress`(JSON 줄). **cdylib은 다음 회차.**
- **범위 밖(설계만)**: 재개 · 속도 제한 · 작업 큐 · 이동 검증 · 최상위 항목 병렬 · 내용 비교 미러 · 휴지통 미러(§8).

## 2. 지금 있는 것(재사용 · 바꾸지 않는다)

| 층 | 자리 | 하는 일 | 근거 |
| --- | --- | --- | --- |
| 전송 엔진 | `ndir-ops` `transfer_with` · `plan_transfer` | 항목 순회 · 충돌(`Conflict`) · 스테이징 교체 · 교차 볼륨 이동 · 실행 취소 · 취소 · 항목별 실패 격리 · dry-run 계획(`PlanAction`) | DR-11 패리티 시험 · docs/24 §7 |
| 전략 계층 | `ndir_ops::fastcopy` | 파일 1개 OS 복사(`copy_file` · Windows `CopyFileExW` · Linux FICLONE/`copy_file_range` · macOS `copyfile` CLONE) · 폴더 안 병렬(자동 4/8 · 큰 파일 직렬) · `NameFilter` · `Tuning` · 장치 판정 `seek_of` | docs/24 §6 |
| 비교 · 미러 | `ndir_ops::compare` | `compare` + `plan(LeftToRight · mirror)` + `apply_copies` · 크기 + 수정 시각(허용 2 s) | 앱 폴더 동기화와 같은 길 |
| 해시 | `ndir_ops::hash::hash_file` | SHA-256(검증) | `--verify` |
| CLI | `crates/nexa-copy` | `cli.rs`(인자 해석 · 진행 · 요약 · 종료 코드) · `mirror.rs`(안전장치 `dangerous_dest` · `--yes`) · `verify.rs` · `ctrlc.rs` | docs/24 §7 · 프로세스 시험 `tests/cli.rs` |

→ `ndir-xfer`는 **새 엔진이 아니라 포장층**이다. 판정 · 복사 · 비교 로직은 `ndir-ops`에 그대로 두고, `nexa-copy` `cli.rs`/`mirror.rs`/`verify.rs`에 흩어진 "작업 조립 · 실행 순서 · 결과 집계"를 라이브러리로 올린다. 그래서 앱(진행 창 · 작업 큐) · CLI · cdylib이 같은 순서를 공유한다.

## 3. 제공 형태와 계층

```
앱(nexa-dir)  ── 진행 창 · 상태줄 · 작업 큐(후) ─┐
nexa-copy(bin) ── 인자/--job 해석 · 출력 ────────┼──▶ ndir-xfer(rlib · 외부 crate 0)
(후) cdylib(C ABI · `ndir_xfer_run`) ──────────────────┘        JobSpec ⇄ JSON · run(job, sink, cancel) → Report
                                                            └▶ ndir-ops(transfer_with · plan_transfer · compare · hash · fastcopy)
```

| 형태 | 이름 | 소비자 | 이번 회차 |
| --- | --- | --- | --- |
| rlib | `ndir-xfer` | 앱 · `nexa-copy` · cdylib | 골격 ✅ 목표 |
| bin | `nexa-copy` | 셸 · 스크립트 · CI | `--job` · `--progress` 추가 |
| cdylib | (후보) 진입 `ndir_xfer_run` | 다른 언어 프로그램 | 다음 회차 — §7 |

- **의존 규칙**: `ndir-xfer` → `ndir-ops`(+ `ndir-core`)만. 창 · 설정 파일 · 라이선스 · `platform/` 포트를 모른다(휴지통 미러가 포트를 필요로 하는 이유 = §8).
- **외부 crate 0**(DR-8): JSON은 손수 파서(ndir-settings `json.rs` 꼴 복사 · 개발 결정).

## 4. JobSpec — 작업 한 덩어리(`ndir_xfer::spec`)

```json
{
  "op": "copy",
  "sources": ["/Users/me/Photos/2026", "/Users/me/notes.txt"],
  "dest": "/Volumes/Backup/inbox",
  "conflict": "skip",
  "threads": 0,
  "native": true,
  "unbuffered_mb": 64,
  "include": ["*.jpg", "*.heic"],
  "exclude": [".DS_Store"],
  "verify": true,
  "mirror": false,
  "dry_run": false,
  "yes": false
}
```

| 필드(JSON 키 = Rust 필드 · snake_case) | 타입 · 값 | 기본 | CLI 대응 | 규칙 |
| --- | --- | --- | --- | --- |
| `op` | `Op` — `copy` \| `move` | `copy` | `--move` | 미러는 `op`가 아니라 `mirror` 불 |
| `sources` | `Vec<PathBuf>` | — | `<source>...` | 비면 `validate` 오류 · 미러면 1개 |
| `dest` | `PathBuf` | — | `<destination-folder>` | 미러 안전장치 `mirror::dangerous_dest`(드라이브 루트 · 홈 · 원본과 겹침 = 거부) |
| `conflict` | `ConflictPolicy` — `skip` \| `overwrite` | `skip` | `--overwrite` | `ask`는 명세에 없다(대화형은 CLI `--ask`만 · `ConflictPolicy::resolve` → 엔진 `Conflict`) |
| `threads` | `usize`(0 = 자동 · 최대 16) | 0 | `--threads` | |
| `native` | `bool` | true | `--no-native` | |
| `unbuffered_mb` | `u64`(Windows만 의미 · 0 = 끔) | **64** | `--unbuffered-mb` | ⚠ CLI 간단 인자 · 엔진 기본은 0(docs/24 §6-2 "실측 손해라 기본 끔") — 잡 경로만 64라 두 경로 차이(§9 동등성 시험으로 확인할 것 · 개발 확인 중) |
| `include` · `exclude` | `Vec<String>` 글롭 → `NameFilter` | `[]` | `--include` · `--exclude` | 규칙 = docs/24 §7(exclude 우선 · include는 파일만 · 대소문자 무시) |
| `verify` | `bool` | false | `--verify` | `move`면 무시 |
| `mirror` | `bool` | false | `--mirror` | `sources` 1개 · 대상을 원본과 같게(새것/바뀐 것 복사 + 대상에만 있는 것 영구 삭제) |
| `dry_run` | `bool` | false | `-n` | 파일 시스템 불변 · Report 항목 `status` = `planned` |
| `yes` | `bool` | false | `--yes` | 미러 삭제 확인 — false면 지울 것이 있을 때 실행 안 하고 목록 + 오류 |

- **JSON 규칙**: 빠진 키 = 기본값 · 모르는 키 = 무시 · 판 키(`version`) 없음(넣어도 무시) · `JobSpec::from_json(&str) -> Result<JobSpec, String>` · `to_json(&self) -> String`. 파서는 `ndir_xfer::json::{Json, parse, dump}`(외부 crate 0 · ndir-settings `json.rs` 꼴).
- **검증**: `validate(&JobSpec) -> Option<String>`(오류 문구 · 없으면 None) — `run`도 먼저 부른다.
- **CLI와 JobSpec은 한 길**: `nexa-copy` 간단 인자는 내부에서 JobSpec을 만들어 `--job`과 같은 `run`으로 보낸다(DR-34 ⑦ · §9 동등성 시험).

## 5. Progress — 진행 스냅샷(`ndir_xfer::report::Progress`)

- **모양**: 사건 열거가 아니라 **스냅샷 콜백** — `run`의 `on_progress: &mut dyn FnMut(&Progress)`가 진행이 바뀔 때마다 지금 상태를 받는다.

| 필드 | 타입 | JSON 키 | 뜻 |
| --- | --- | --- | --- |
| `phase` | `Phase` — `plan` \| `transfer` \| `verify` \| `mirror` \| `delete` | `phase` | 지금 단계 |
| `done_bytes` | `u64` | `done` | 이 단계에서 끝낸 바이트 |
| `total_bytes` | `u64` | `total` | 이 단계 총 바이트 |
| `item_index` | `usize` | `item` | 지금 항목 순번 |
| `item_count` | `usize` | `items` | 항목 수 |
| `current` | `Option<PathBuf>` | `current`(있을 때만) | 지금 다루는 경로 |

- **`nexa-copy --progress`**: `Progress::to_json_line()` = stdout에 한 줄 하나(`{"phase":"transfer","done":…,"total":…,"item":…,"items":…,"current":"…"}`) · 사람용 진행은 종전대로 stderr.
- **흐름 규칙**: 스냅샷이라 중간 값을 버려도 된다(마지막 값만 의미) · 끝남은 `Report`로 안다(진행 줄에 "끝" 사건이 없다 — 소비자는 프로세스 종료 · `run` 반환으로 판단).
- **앱 연결**: 앱은 콜백에서 공유 상태(원자값 · `Mutex`)에 적고 유휴 틱이 거둔다(CLAUDE.md §3 스레딩 규약) — 진행 창 · 상태줄 진행 칸이 같은 스냅샷을 본다.

## 6. Report — 결과(`ndir_xfer::report::Report`)

| 필드 | 타입 | 뜻 |
| --- | --- | --- |
| `ok` | `bool` | 실패 · 불일치 · 작업 오류가 없음 |
| `canceled` | `bool` | 취소됨 |
| `dry_run` | `bool` | 계획만 |
| `items` | `Vec<ItemReport>` | 항목별 — `action`(copy · move · duplicate · overwrite · skip · noop · error · delete) · `status`(planned · done · skipped · failed · canceled) · `bytes` · `source` · `dest` · `note` |
| `transferred_bytes` | `u64` | 옮긴 바이트 |
| `failed` · `skipped` · `deleted` | `usize` | 개수 |
| `verify` | `Option<VerifySummary>` | `files` · `bytes` · `mismatches: Vec<(PathBuf, String)>` |
| `elapsed_ms` | `u64` | 걸린 시간 |
| `errors` | `Vec<String>` | 작업 전체 수준 오류(명세 검증 · 대상 없음 · 미러 거부 …) — `Report::failed_with(msg)` |

- `to_json()` = Report JSON · `exit_code() -> u8` = **취소 130 · 실패/불일치/오류 1 · 그 밖 0**(nexa-copy와 같다). 인자 오류 2는 CLI 해석 단계 몫(라이브러리는 0/1/130).
- 함수 전체: `run(&JobSpec, &mut dyn FnMut(&Progress), &AtomicBool) -> Report` · `validate` · 모듈 `verify::verify_pairs`(nexa-copy에서 이동 · `VerifyReport{files, bytes, failures}`) · `mirror::{mirror_plan, dangerous_dest, apply_mirror(→ MirrorOutcome{failed, deleted, delete_failed, canceled}), home_dir, normalize_sep}`.
- `nexa-copy --report <file>`(결과 JSON을 파일로)은 결정 거리(§11).

## 7. cdylib(후속 · 이번 회차 아님)

- **제안 이름 `ndir_xfer_run`** — C ABI 최소면: `ndir_xfer_run(job_json: *const c_char, cb: extern "C" fn(*const c_char, *mut c_void), user: *mut c_void, cancel: *const AtomicBool) -> *mut c_char`(Report JSON · 호출자가 해제 함수로 돌려줌) + 판 조회 함수. 문자열은 UTF-8 JSON만 오간다(§4 `JobSpec` JSON · §5 진행 줄 · §6 Report JSON — 구조체 ABI를 노출하지 않아 판 호환이 쉽다).
- 진행 콜백은 다른 스레드에서 불릴 수 있음(문서화) · 배포 채널 동봉 여부는 그때 결정.

## 8. 남은 기능 설계(이번 회차 구현 안 함)

| 기능 | 설계 | 걸림돌 |
| --- | --- | --- |
| **재개** | `Report`(§6 `items[].status`)를 **저널**로도 쓴다 — 항목 단위 완료 기록(원자적 덧붙임) · `--resume <report>`면 `done` 항목은 건너뛰고 나머지만 · 파일 중간 재개는 하지 않는다(스테이징 교체가 원자적이라 부분 파일이 남지 않음) | 폴더 안 병렬은 파일 단위 기록이 필요 — `fastcopy` 작업 스레드에서 완료 통지 |
| **속도 제한** | JobSpec `max_bps`(신설 후보) — 전략 계층 바이트 보고 지점(`on_bytes`)에서 토큰 버킷으로 잠깐 쉼 · OS 복사 경로는 청크 단위 콜백(Windows 진행 콜백 · Linux 32 MiB 청크 · macOS 상태 콜백) 사이에서만 걸 수 있다 | 복제(clone)는 순간이라 제한 무의미(문서화) |
| **작업 큐** | 앱 = JobSpec 큐(순차 기본 · 장치가 다르면 병렬 후보) · 큐 상태 저장 = JobSpec + 저널 | 진행 창(T-96 카드형)과 함께 설계 |
| **이동 검증** | 교차 볼륨 이동 = 복사 → 검증 → 원본 삭제 순서(엔진에 "원본 정리 전 훅") · 같은 볼륨 rename은 대상 아님 | `transfer_with` 이동 경로에 훅 추가(ndir-ops · DR-11 패리티 유지) |
| **최상위 항목 병렬** | 질문 없는 항목만 묶어 보내기 | docs/24 §6-3 — 충돌 질문 · 스테이징 · 실행 취소가 항목 순서에 묶임 |
| **미러 내용 비교** | `compare` `by_content` 옵션을 JobSpec 불 `mirror_by_content`(후보)로 | 비용(전부 해시) — 크기·시각 같을 때만 해시하는 2단계 |
| **휴지통 미러** | JobSpec 불 `mirror_trash`(후보) | 휴지통은 `platform/` 포트(DR-5) — `ndir-xfer`는 포트를 모르므로 앱 · CLI가 삭제 콜백을 주입 |

## 9. 시험 계획

- **T1(ndir-xfer · 골격 시험 8/0)**: JobSpec JSON 왕복(빠진 키 = 기본 · 모르는 키 무시) · `validate`(빈 sources · 미러 원본 2개 · 위험 대상) · Report JSON · `exit_code` · Progress 줄 키.
- **T1 동등성**: 같은 작업을 **간단 인자 경로와 `--job` 경로**로 각각 만들었을 때 JobSpec이 같다(DR-34 ⑦ 한 길 보장 · `unbuffered_mb` 기본 차이(§4 ⚠)가 여기서 드러난다).
- **프로세스(T4급 · `crates/nexa-copy/tests/cli.rs`)**: `--job` 복사/미러/검증/dry-run · `--progress` 줄이 모두 JSON으로 파싱되고 `phase`가 단계 순서(plan → transfer → verify/mirror/delete) · 종료 코드 = Report `exit_code()` · 취소 = 130.
- **T6(성능)**: `bench_copy`를 `scripts/perf-baseline.sh`에 편입 · `ndir-xfer` 포장 전후 같은 작업의 시간 차 ≤ 2 %(포장층 비용 0 확인) · Linux/macOS 실측은 그 PC 회차.

## 10. 성능 기준선 확장 계획(T-97 "+ 성능 기준선 확장")

| 측정 | 지금 | 확장 |
| --- | --- | --- |
| 큰 파일 1개 | `bench_copy`(Windows 10-05 · docs/24 §4) | 3-OS · 같은 볼륨(복제) / 다른 볼륨 / 네트워크 구분 |
| 작은 파일 다수 | 1차 측정만 | 1만 개 × 4 KiB · 스레드 1/4/8 |
| 검증 | 없음 | `--verify` 오버헤드(읽기 2배 기대) |
| 미러 | 없음 | 변경 1 % · 10 % 트리에서 비교 + 적용 시간 |
| 회귀선 | docs/25 규칙(벤치 +20 %) | 위 항목을 `perf-baseline` 기준선 파일에 추가 |

## 11. 결정 거리(사용자)

1. JobSpec 상대 경로 기준 — **잡 파일 위치(추천)** vs 작업 디렉터리.
2. `--progress` 출력 자리 — **stdout(추천 · 사람용 진행은 stderr 유지)** vs 별도 파일 `--progress-file`.
   (덧) `unbuffered_mb` 잡 기본 64 vs CLI · 엔진 기본 0 — 하나로 맞출 것(추천 = 0 · docs/24 실측).
3. `--report <file>` 추가 여부(추천 = 추가 · dry-run 기계용 판).
4. cdylib 진입 함수 이름 — `ndir_xfer_run`(개발 제안) · 동봉 채널.
