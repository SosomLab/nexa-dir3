# 26. 대량 전송 엔진 설계 — `ndir-xfer` · `nexa-copy` · (후) cdylib

> T-97 · DR-17(대량 전송 엔진 내장) · DR-34 ①⑦(10-31 사용자 결정) · 원장 NEW-006 · NEW-007. 앞 문서 = [24 빠른 복사](24-fast-copy.md)(§6 전략 계층 · §7 `nexa-copy` 1차).
> 상태: **10-31 협업 작성 · §4~§6 = `crates/ndir-xfer` 코드에 맞춤**(6b1a22d · spec.rs · report.rs · lib.rs · 시험 8/0 · nexa-copy `job_and_progress_paths`). §11 = 사용자 결정 반영(10-31).

## 1. 목적과 범위

- **목적**: 앱 · 명령행 · 다른 프로그램이 **같은 전송 엔진을 같은 규칙으로** 쓰게 한다. 작업을 **직렬화 가능한 한 덩어리(JobSpec)**로 표현해 저장 · 재실행 · 다른 프로세스 전달이 되게 하고, 진행과 결과도 기계가 읽는 모양(Progress · Report)으로 낸다.
- **이번 회차(DR-34 ①⑦)**: ① 이 설계 문서 ② `ndir-xfer` rlib 골격 — JobSpec/Progress/Report 직렬화 · 복사 · 이동 · 검증 · 미러 4엔진 포장 · dry-run ③ `nexa-copy` `--job job.json` · `--progress`(JSON 줄). **cdylib은 다음 회차.**
- **범위 밖(설계만 · §8)**: 작업 큐 · 최상위 항목 병렬 · 휴지통 미러 앱 배선(재개 · 속도 제한 · 이동 검증 · 내용 비교 미러는 T-202 ②③⑤⑦로 구현됨).

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
  "unbuffered_mb": 0,
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
| `sources` | `Vec<PathBuf>` | — | `<source>...` | 비면 `validate` 오류 · 미러면 1개 · 상대 경로 = **잡 파일이 있는 폴더 기준**(§11 ①) |
| `dest` | `PathBuf` | — | `<destination-folder>` · 상대 경로 = 잡 파일 폴더 기준 | 미러 안전장치 `mirror::dangerous_dest`(드라이브 루트 · 홈 · 원본과 겹침 = 거부) |
| `conflict` | `ConflictPolicy` — `skip` \| `overwrite` | `skip` | `--overwrite` | `ask`는 명세에 없다(대화형은 CLI `--ask`만 · `ConflictPolicy::resolve` → 엔진 `Conflict`) |
| `threads` | `usize`(0 = 자동 · 최대 16) | 0 | `--threads` | |
| `native` | `bool` | true | `--no-native` | |
| `unbuffered_mb` | `u64`(Windows만 의미 · 0 = 끔) | **0** | `--unbuffered-mb` | 엔진 · 간단 인자와 같은 기본(docs/24 §6-2 "실측 손해라 기본 끔") — 초안 골격의 64를 협업 지적으로 0으로 맞춤(6b1a22d) |
| `include` · `exclude` | `Vec<String>` 글롭 → `NameFilter` | `[]` | `--include` · `--exclude` | 규칙 = docs/24 §7(exclude 우선 · include는 파일만 · 대소문자 무시) |
| `verify` | `bool` | false | `--verify` | 복사 = 끝난 뒤 대조(필터로 뺀 항목 제외) · **이동(T-202 ⑤) = 교차 볼륨만 원본 정리 전 대조**(`note: verified`) · 같은 볼륨 rename은 검증 안 함(`note: renamed on the same volume (not verified)`) · 이동이면 `rep.verify` 요약 없음 |
| `mirror` | `bool` | false | `--mirror` | `sources` 1개 · 대상을 원본과 같게(새것/바뀐 것 복사 + 대상에만 있는 것 영구 삭제) |
| `dry_run` | `bool` | false | `-n` | 파일 시스템 불변 · Report 항목 `status` = `planned` |
| `yes` | `bool` | false | `--yes` | 미러 삭제 확인 — false면 지울 것이 있을 때 실행 안 하고 목록 + 오류 |
| `journal` | `Option<PathBuf>` | 없음 | `--journal <file>` | 최상위 항목 완료마다(`Event::ItemEnd`) `{"source","dest","status","bytes"}` 한 줄 덧붙임 · 상대 경로 = 잡 파일 폴더 · dry-run이면 쓰지 않음 · `verify`면 검증 통과 뒤에만 `done` · 못 열면 시작 안 함 · 쓰기 실패 = Report errors + `ok: false` · `resume`과 같은 파일 가능(읽기 → 덧붙이기) |
| `resume` | `Option<PathBuf>` | 없음 | `--resume <file>` | 이전 Report JSON 또는 저널 · `(source, dest)`가 `done`인 항목 건너뜀(Report `status: skipped` · `note: resumed` · bytes/dest는 저널 값 · items 앞에 먼저 적힘) · 같은 쌍은 마지막 기록 우선 · 잘린 끝 줄 무시(그 밖의 깨진 줄 = 오류) · 끝난 쌍은 `validate` 존재 검사에서 뺀다(이동 재개) · 복사/이동만(미러 + journal/resume = `validate` 오류) · `--job`과 함께 명령줄로 주면 키를 덮음 · `--ask`와 못 씀 |
| `max_bps` | `u64`(바이트/초 · 0 = 없음) | 0 | `--max-bps <rate>`(K/M/G ×1024 · 소수 · 끝 B 허용) | 실제 전송만 제한(병렬이면 합계) · 복제 · 검증 읽기 제외 · §8 한계 |
| `mirror_by_content` | `bool` | false | `--by-content`(`--mirror` 없이 = 해석 오류 2) | 미러가 아니면 무시 · 필터 뒤 크기 · 시각이 같은 파일 쌍만 양쪽 SHA-256 · 한쪽이라도 못 읽으면 **다름**(복사 시도) · 해시 진행 = `phase: plan`의 `done` · 비용 = 양쪽 전량 읽기(대상이 네트워크면 큼) · `max_bps` 비제한 |

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

- **`nexa-copy --progress`**: `Progress::to_json_line()` = stdout에 한 줄 하나(`{"phase":"transfer","done":…,"total":…,"item":…,"items":…,"current":"…"}` · 단계가 바뀔 때 또는 100 ms마다) · **마지막 줄 = Report JSON**(§6) · `--ask`와 함께 못 씀 · 사람용 진행은 종전대로 stderr.
- **흐름 규칙**: 스냅샷이라 중간 값을 버려도 된다(마지막 값만 의미) · 끝남 = 마지막 Report JSON 줄(CLI) 또는 `run` 반환(라이브러리) — 진행 줄에 따로 "끝" 사건은 없다.
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
- `nexa-copy --report <file>` = 끝에 Report JSON을 그 파일에 쓴다(§11 ③ · 사용자 결정 10-31 · ed45954 · `--ask` 불가 · 쓰기 실패 = 안내만).

## 7. cdylib(T-202 ① · d130d9f · 사용자 10-31 "패키지 동봉" §11-4)

- **산출물**: `crate-type = ["rlib", "cdylib"]`(같은 크레이트 · nexa-sql D-350 꼴) — `libndir_xfer.so` · `libndir_xfer.dylib` · `ndir_xfer.dll`. 문자열은 UTF-8 JSON만 오간다(§4 `JobSpec` JSON · §5 진행 줄 · §6 Report JSON — 구조체 ABI를 노출하지 않아 판 호환이 쉽다). C 헤더 = `crates/ndir-xfer/include/ndir_xfer.h`(수작업 · 동봉).
- **API 5개**:

  | 함수 | 뜻 |
  |---|---|
  | `ndir_xfer_abi_version() -> u32` | ABI 판(지금 1) |
  | `ndir_xfer_version() -> const char*` | 제품 판(정적 문자열 · 해제 금지) |
  | `ndir_xfer_run(job, cb, user, cancel) -> char*` | JobSpec JSON 실행 · Report JSON 반환(오류도 `ok:false` Report · NULL 없음) · `cancel` = C `const bool*`(Rust `AtomicBool::from_ptr`) |
  | `ndir_xfer_validate(job) -> char*` | JobSpec 검사만(실행 없음) |
  | `ndir_xfer_free(char*)` | 위 두 함수가 돌려준 문자열 해제(호출자 free 금지) |

- **주의**: ① 릴리스 프로필 = `panic = "abort"`(프로필 전체라 크레이트별 불가) → 라이브러리 패닉 = **호출 프로세스 종료** · 입력 오류는 전부 Result/Report로 돌려준다(모듈 안 unwrap 0 · `catch_unwind` 울타리 `guarded`는 dev/test용) ② 진행 콜백은 다른 스레드에서 불릴 수 있다 ③ JobSpec 상대 경로 기준 = 호출 프로세스의 작업 디렉터리(`nexa-copy --job`은 잡 파일 폴더 — §11-1 · C ABI에는 잡 파일이 없다).
- **동봉 자리(구현 확정 · be60dc2)**(§11-4): deb/rpm = `/usr/lib/nexa-dir/libndir_xfer.so`(전용 폴더 · SONAME/ldconfig/multiarch 무관 · 소비자는 전체 경로 dlopen · rpm `%files`는 `%{_prefix}/lib/nexa-dir`) + `/usr/share/nexa-dir/include/ndir_xfer.h` · pkg/dmg/brew = `Contents/Frameworks/libndir_xfer.dylib`(Universal lipo · `install_name_tool -id @rpath/` · codesign --deep 앞) + `Contents/Resources/include/` · MSI = 설치 폴더 `ndir_xfer.dll`(`XferLib` Component) + `include\`(`XferHeader`) · `.dll.lib` 제외(LoadLibrary) · 임포트 게이트에 dll 포함 · 포터블 zip = 스테이징 그대로 · 빌드 명령 4곳 `-p ndir-xfer` · 릴리스 스모크 3-OS = 설치 뒤 Python ctypes 로드 → `ndir_xfer_version()` == 판 · 릴리스 노트 C 라이브러리 문단(영어).

## 8. 남은 기능 설계(이번 회차 구현 안 함)

- **재개 ✅ T-202 ②(abc69f3 · §4 `journal` · `resume` · `ndir_xfer::resume`)** — 한계: 최상위 항목 단위 · 파일 중간 재개 없음(스테이징 교체가 원자적) · 폴더가 끊기면 그 폴더는 다시 돌고 `conflict: skip`이면 있는 파일을 건너뜀(`overwrite`면 다시 씀) · 그사이 원본이 바뀌어도 `done`이면 건너뜀(감지 안 함). cdylib = JobSpec 키라 자동 적용(ABI 1 불변).
- **속도 제한 ✅ T-202 ③(071fc30 · §4 `max_bps` · `ndir_ops::fastcopy::Pacer` + `with_pacer(bps, f)` 스레드 지역 범위 · Tuning 필드 추가 없음)** — 실제 전송 지점(루프 · `copy_file_range` 청크 · `CopyFileExW`/`copyfile` 진행 콜백)에서 `on_bytes` 뒤 `pace` · 병렬은 작업 스레드 TLS에 같은 Pacer(Arc)를 걸어 **합계 기준** · 청크 = `Pacer::chunk_for`(¼초 분량 · 하한 64 KiB · 상한 기본) · 100 ms 조각으로 자며 `stop` 확인. 한계: ① 같은 파일 시스템 복제(FICLONE · clonefile · 블록 복제)는 순간이라 제한 없음 ② Windows · macOS 콜백 간격은 OS가 정해 덩어리질 수 있음(**실기 필요**) ③ 전역 `set_tuning`에는 제한 없음(`with_pacer` 범위만) · 검증 읽기 비제한.
- **미러 내용 비교 ✅ T-202 ⑦(4f2f3ac · §4 `mirror_by_content` · `mirror_plan_with`)** — 엔진 `CmpOpts::by_content`를 쓰지 않고 필터 뒤 직접 해시(제외 파일 미해시). 판정 공용 `compare::content_verdict`(둘 다 읽혔고 같을 때만 Same · GAP-026).
- **휴지통 미러 = Rust API만(4f2f3ac)**: `apply_mirror_with(plan, on_bytes, cancel, delete)` · `run_with(spec, on_progress, cancel, Option<DeleteFn>)`(`run` = None 위임 · 영구 삭제) — CLI · cdylib은 영구 삭제 그대로(ABI 1). **앱 배선 = T-202 ④ 작업 큐/진행 창 때**(휴지통 포트는 `platform/` · DR-5).
- **이동 검증 ✅ T-202 ⑤(7b28766 · `ndir_ops::verify` · `fastcopy::with_move_verify(on, f)` 스레드 지역 · 기본 false = 경로 불변)** — `move_by_copy`가 원본 정리 **전** 대조: 폴더 = `copied` 목록의 파일만(필터로 뺀 항목은 없는 게 정상 · 링크 → 폴더 항목 제외) · 파일 = (src, dest) · 최상위 링크도 검증(metadata가 따라감) · 실패 = `InvalidData` + **부분 정리 없음**(원본 전부 유지 · 사본은 그 자리) · 검증 중 취소 = Interrupted · 원본 유지 · 검증 바이트는 `on_bytes`에 안 더함 · 저널 `done` = 검증 뒤. **경계**: `overwrite`면 스테이징 교체가 먼저라 검증 실패 시 옛 대상은 이미 없고 깨진 사본이 그 자리에 있다(원본은 유지 → 데이터 유실 아님). **동작 변경**: `--move --verify`는 종전 무시 → 검증(느려질 수 있음 · 실패 가능 · 릴리스 노트 대상).

| 기능 | 설계 | 걸림돌 |
| --- | --- | --- |
| **작업 큐** | 앱 = JobSpec 큐(순차 기본 · 장치가 다르면 병렬 후보) · 큐 상태 저장 = JobSpec + 저널 | 진행 창(T-96 카드형)과 함께 설계 |
| **최상위 항목 병렬** | 질문 없는 항목만 묶어 보내기 | docs/24 §6-3 — 충돌 질문 · 스테이징 · 실행 취소가 항목 순서에 묶임 |

## 9. 시험 계획

- **T1(ndir-xfer · 골격 시험 8/0)**: JobSpec JSON 왕복(빠진 키 = 기본 · 모르는 키 무시) · `validate`(빈 sources · 미러 원본 2개 · 위험 대상) · Report JSON · `exit_code` · Progress 줄 키.
- **T1 동등성**: 같은 작업을 **간단 인자 경로와 `--job` 경로**로 각각 만들었을 때 JobSpec이 같다(DR-34 ⑦ 한 길 보장).
- **프로세스(T4급 · `crates/nexa-copy/tests/cli.rs`)**: `--job` 복사/미러/검증/dry-run · `--progress` 줄이 모두 JSON으로 파싱되고 `phase`가 단계 순서(plan → transfer → verify/mirror/delete) · 종료 코드 = Report `exit_code()` · 취소 = 130.
- **재개(abc69f3)**: `resume::{done_items_from_report_and_journal, journal_appends_and_reads_back}` · spec 왕복/검증/상대 경로 · `journal_then_resume_skips_done_items`(이동 재개 포함) · `journal_waits_for_verify` · 프로세스 `journal_and_resume_skip_finished_items`.
- **속도 제한(071fc30)**: ndir-ops `pacer_limits_loop_and_parallel_copies`(8 × 128 KiB · 4스레드 ≥ 200 ms · 1 B/s stop < 1 s) · `Pacer::delay_for` 순수 판정 · ndir-xfer `max_bps_slows_transfer` · nexa-copy `parse_rate` · 프로세스 `max_bps_limits_copy_speed` — 실측 시험은 하한만 단언(흔들림 방지).
- **미러 내용 비교(4f2f3ac)**: ndir-xfer `mirror_by_content_and_delete_callback`(기본 미러 유지 · by_content 덮어씀 · 해시 진행 · 삭제 콜백 · 제외 파일 미해시) · 프로세스 `mirror_by_content_overwrites_same_size_and_time` · ndir-ops `content_verdict_treats_unreadable_as_differ`(5경우).
- **이동 검증(7b28766)**: ndir-ops `move_verify_checks_copies_before_cleanup`(통과 이동 · 불일치 = 오류 + 양쪽 유지 · copied에 없는 파일 미대조 · 범위 밖 = 검사 없음) · `verify_detects_mismatch_missing_and_type`(필터 안/밖 · GAP-027) · ndir-xfer `move_with_verify_still_moves`.
- **T6(성능)**: `bench_copy`를 `scripts/perf-baseline.sh`에 편입 · `ndir-xfer` 포장 전후 같은 작업의 시간 차 ≤ 2 %(포장층 비용 0 확인) · Linux/macOS 실측은 그 PC 회차.

## 10. 성능 기준선 확장 계획(T-97 "+ 성능 기준선 확장")

| 측정 | 지금 | 확장 |
| --- | --- | --- |
| 큰 파일 1개 | `bench_copy`(Windows 10-05 · docs/24 §4) | 3-OS · 같은 볼륨(복제) / 다른 볼륨 / 네트워크 구분 |
| 작은 파일 다수 | 1차 측정만 | 1만 개 × 4 KiB · 스레드 1/4/8 |
| 검증 | 없음 | `--verify` 오버헤드(읽기 2배 기대) |
| 미러 | 없음 | 변경 1 % · 10 % 트리에서 비교 + 적용 시간 |
| 회귀선 | docs/25 규칙(벤치 +20 %) | 위 항목을 `perf-baseline` 기준선 파일에 추가 |

## 11. 결정(사용자 10-31 · 개발 세션 질문 → 답)

1. JobSpec 상대 경로 기준 = **잡 파일이 있는 폴더**(작업 디렉터리 아님 — 저장한 잡을 어디서 실행해도 같은 자리) · 구현 ✅ ed45954 `JobSpec::resolved_against(base)`(절대 경로는 그대로) · `nexa-copy --job`이 잡 파일의 부모로 푼다.
2. `--progress` 출력 자리 = **stdout**(현행 유지 · 사람용 진행은 stderr).
3. **`--report <file>` 추가** — 끝에 Report JSON(§6)을 그 파일에 쓴다(dry-run 기계용 보고서 포함) · 구현 ✅ ed45954 — `--progress` 유무와 무관 · 간단 인자에서도 ndir-xfer 경로 · `--ask`와 함께 못 씀 · 쓰기 실패는 안내만(종료 코드 불변).
4. cdylib = 진입 **`ndir_xfer_run`** · **패키지에 동봉** — 다음 회차(T-202 ①).
