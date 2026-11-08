# 26. 대량 전송 엔진 설계 — `ndir-xfer` · `nexa-copy` · cdylib

> T-97 · DR-17(대량 전송 엔진 내장) · DR-34 ①⑦(10-31 사용자 결정) · 원장 NEW-006 · NEW-007. 앞 문서 = [24 빠른 복사](24-fast-copy.md)(§6 전략 계층 · §7 `nexa-copy` 1차).
> **역할**: 이 문서 = 명세 · API · 구현 현황 · 남은 설계. 복사 전략 계층 · OS별 구현 · 실측 = [24](24-fast-copy.md).
> 상태: §4~§7 = `crates/ndir-xfer` 코드 기준 · §8 = 구현 현황 · §11 = 사용자 결정(10-31) · §12 = 작업 큐 설계(11-01 · 구현 전).

## 1. 목적과 범위

- **목적**: 앱 · 명령행 · 다른 프로그램이 **같은 전송 엔진을 같은 규칙으로** 쓰게 한다. 작업을 **직렬화 가능한 한 덩어리(JobSpec)**로 표현해 저장 · 재실행 · 다른 프로세스 전달이 되게 하고, 진행과 결과도 기계가 읽는 모양(Progress · Report)으로 낸다.
- **구현됨**: `ndir-xfer` rlib — JobSpec/Progress/Report 직렬화 · 복사 · 이동 · 검증 · 미러 4엔진 포장 · dry-run(DR-34 ①⑦) · `nexa-copy` `--job` · `--progress` · `--report` · cdylib(§7).
- **아직 없음**: 작업 큐 · 앱 전송의 `ndir-xfer` 통합(§12) · 최상위 항목 병렬(§8).

## 2. 지금 있는 것(재사용 · 바꾸지 않는다)

| 층 | 자리 | 하는 일 | 근거 |
| --- | --- | --- | --- |
| 전송 엔진 | `ndir-ops` `transfer_with` · `plan_transfer` | 항목 순회 · 충돌(`Conflict` · 폴더 덮어쓰기 = 병합 DR-35) · 스테이징 · 교차 볼륨 이동 · 실행 취소 · 취소 · 항목별 실패 격리 · dry-run 계획(`PlanAction`) | DR-11 패리티 시험 · docs/24 §7 |
| 전략 계층 | `ndir_ops::fastcopy` | 파일 1개 OS 복사(`copy_file` · Windows `CopyFileExW` · Linux FICLONE/`copy_file_range` · macOS `copyfile` CLONE) · 폴더 안 병렬(자동 4/8 · 큰 파일 직렬) · `NameFilter` · `Tuning` · 장치 판정 `seek_of` | docs/24 §6 |
| 비교 · 미러 | `ndir_ops::compare` | `compare` + `plan(LeftToRight · mirror)` + `apply_copies` · 크기 + 수정 시각(허용 2 s) | 앱 폴더 동기화와 같은 길 |
| 해시 | `ndir_ops::hash::hash_file` | SHA-256(검증) | `--verify` |
| CLI | `crates/nexa-copy` | `cli.rs`(인자 해석 · 진행 · 요약 · 종료 코드) · `mirror.rs`(안전장치 `dangerous_dest` · `--yes`) · `verify.rs` · `ctrlc.rs` | docs/24 §7 · 프로세스 시험 `tests/cli.rs` |

→ `ndir-xfer`는 **새 엔진이 아니라 포장층**이다. 판정 · 복사 · 비교 로직은 `ndir-ops`에 그대로 두고, `nexa-copy` `cli.rs`/`mirror.rs`/`verify.rs`에 흩어진 "작업 조립 · 실행 순서 · 결과 집계"를 라이브러리로 올린다. 그래서 앱(진행 창 · 작업 큐) · CLI · cdylib이 같은 순서를 공유한다.

## 3. 제공 형태와 계층

```
앱(nexa-dir)  ── 진행 창 · 상태줄 · (후) 작업 큐 · 지금은 ndir-ops 직접 ─┐
nexa-copy(bin) ── 인자/--job 해석 · 출력 ────────┼──▶ ndir-xfer(rlib · 외부 crate 0)
cdylib(C ABI · `ndir_xfer_run`) ───────────────────────┘        JobSpec ⇄ JSON · run(job, sink, cancel) → Report
                                                            └▶ ndir-ops(transfer_with · plan_transfer · compare · hash · fastcopy)
```

| 형태 | 이름 | 소비자 | 상태 |
| --- | --- | --- | --- |
| rlib | `ndir-xfer` | `nexa-copy` · cdylib(앱 = §12 슬라이스 1) | ✅ |
| bin | `nexa-copy` | 셸 · 스크립트 · CI | ✅ `--job` · `--progress` · `--report` · 재개 · 속도 제한 |
| cdylib | 진입 `ndir_xfer_run` | 다른 언어 프로그램 | ✅ 전 채널 동봉 — §7 |

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
| `verify` | `bool` | false | `--verify` | 복사 = 끝난 뒤 **실제로 옮긴 경로 목록** 대조(f745f3d · 필터로 뺀 항목 · 복사 중 신생 파일 제외) · **이동(T-202 ⑤) = 교차 볼륨만 원본 정리 전 대조**(`note: verified`) · 같은 볼륨 rename은 검증 안 함(`note: renamed on the same volume (not verified)`) · 이동이면 `rep.verify` 요약 없음 |
| `mirror` | `bool` | false | `--mirror` | `sources` 1개 · 대상을 원본과 같게(새것/바뀐 것 복사 + 대상에만 있는 것 영구 삭제) |
| `dry_run` | `bool` | false | `-n` | 파일 시스템 불변 · Report 항목 `status` = `planned` |
| `yes` | `bool` | false | `--yes` | 미러 삭제 확인 — false면 지울 것이 있을 때 실행 안 하고 목록 + 오류 |
| `journal` | `Option<PathBuf>` | 없음 | `--journal <file>` | 항목 시작마다 `started` 줄(`note` = `new` \| `overwrite` · GAP-030) + 완료마다 `done` 줄 덧붙임(Report `items[]` 원소 모양 · 항목당 2줄) · 상대 경로 = 잡 파일 폴더 · dry-run이면 쓰지 않음 · `verify`면 검증 통과 뒤에만 `done` · 못 열면 시작 안 함 · 쓰기 실패 = Report errors + `ok: false` · `resume`과 같은 파일 가능(읽기 → 덧붙이기) |
| `resume` | `Option<PathBuf>` | 없음 | `--resume <file>` | 이전 Report JSON 또는 저널 · `(source, dest)`가 `done`인 항목 건너뜀(Report `status: skipped` · `note: resumed` · bytes/dest는 저널 값 · items 앞에 먼저 적힘) · 같은 쌍은 마지막 기록 우선 · 잘린 끝 줄 무시(그 밖의 깨진 줄 = 오류) · 끝난 쌍은 `validate` 존재 검사에서 뺀다(이동 재개) · **끊긴 항목**(쌍별 마지막 상태 = `started`) 처리 = §8 재개 · 복사/이동만(미러 + journal/resume = `validate` 오류) · `--job`과 함께 명령줄로 주면 키를 덮음 · `--ask`와 못 씀 |
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

## 8. 구현 현황과 남은 설계

- **재개 ✅ T-202 ②(abc69f3 · §4 `journal` · `resume` · `ndir_xfer::resume`)** — **GAP-030 44ac475(11-01): 저널 started 줄 · 끊긴 복사 = 부분 대상 덮어쓰기 + 스테이징 잔여 정리(`Op::Copy` + 원본 존재일 때만 · 살아 있는 pid 제외) · 끊긴 이동 = 정책 무관 Skip + 확인 note(4ac8bd9) · §12.7.** **폴더 덮어쓰기 = 병합(DR-35 · b12a137)**: 저널 없는 재실행도 소실 0(실측). 한계: 파일 ↔ 폴더 같은 이름 = 그 항목 실패 · 중단 뒤 재실행 전까지 대상에 옛 + 일부 새 내용 혼재 · 잔여 `.nexa-old-*`(동의 교체분)는 다음 병합 성공 뒤 치움(죽은 pid · 엔진 이름 `<pid>-<seq>`만 · bf9b607) · 최상위 항목 단위 · 파일 중간 재개 없음 · 중단 뒤 사용자가 고친 복사 대상도 덮어씀(OPS-004 잔여 위험 · 사용자 결정) · 이동은 자동 재개 안 함 · 검증 중 사망 = 전 항목 재복사 · pid 재사용 시 잔여가 남을 수 있음 · 폴더가 끊기면 그 폴더는 다시 돌고 `conflict: skip`이면 있는 파일을 건너뜀(`overwrite`면 다시 씀) · 그사이 원본이 바뀌어도 `done`이면 건너뜀(감지 안 함). cdylib = JobSpec 키라 자동 적용(ABI 1 불변).
- **속도 제한 ✅ T-202 ③(071fc30 · §4 `max_bps` · `ndir_ops::fastcopy::Pacer` + `with_pacer(bps, f)` 스레드 지역 범위 · Tuning 필드 추가 없음)** — 실제 전송 지점(루프 · `copy_file_range` 청크 · `CopyFileExW`/`copyfile` 진행 콜백)에서 `on_bytes` 뒤 `pace` · 병렬은 작업 스레드 TLS에 같은 Pacer(Arc)를 걸어 **합계 기준** · 청크 = `Pacer::chunk_for`(¼초 분량 · 하한 64 KiB · 상한 기본) · 100 ms 조각으로 자며 `stop` 확인. 한계: ① 같은 파일 시스템 복제(FICLONE · clonefile · 블록 복제)는 순간이라 제한 없음 ② Windows · macOS 콜백 간격은 OS가 정해 덩어리질 수 있음(**실기 필요**) ③ 전역 `set_tuning`에는 제한 없음(`with_pacer` 범위만) · 검증 읽기 비제한.
- **미러 내용 비교 ✅ T-202 ⑦(4f2f3ac · §4 `mirror_by_content` · `mirror_plan_with`)** — 엔진 `CmpOpts::by_content`를 쓰지 않고 필터 뒤 직접 해시(제외 파일 미해시). 판정 공용 `compare::content_verdict`(둘 다 읽혔고 같을 때만 Same · GAP-026).
- **휴지통 미러 = Rust API만(4f2f3ac)**: `apply_mirror_with(plan, on_bytes, cancel, delete)` · `run_with(spec, on_progress, cancel, Option<DeleteFn>)`(`run` = None 위임 · 영구 삭제) — CLI · cdylib은 영구 삭제 그대로(ABI 1). **앱은 T-172로 이미 휴지통**(`compare.mirror_delete` · `trash_checked` · `ndir-xfer` 안 거침 · 11-01 §3) · `run_with(delete)` 경유 통합 = T-202 ④에서 앱 전송이 `ndir-xfer`로 옮겨 갈 때 함께(휴지통 포트는 `platform/` · DR-5).
- **이동 검증 ✅ T-202 ⑤(7b28766 · `ndir_ops::verify` · `fastcopy::with_move_verify(on, f)` 스레드 지역 · 기본 false = 경로 불변)** — `move_by_copy`가 원본 정리 **전** 대조: 폴더 = 실제로 옮긴 경로 목록(f745f3d · 파일 = 크기 + SHA-256 · 폴더 = 존재 · 종류 · 링크 안쪽 · 빈 폴더 포함 · 필터로 뺀 항목 없음 · 원본 정리용 `copied`와 별개 — BUG-001 불변) · 파일 = (src, dest) · 최상위 링크도 검증(metadata가 따라감) · 실패 = `InvalidData` + **부분 정리 없음**(원본 전부 유지 · 사본은 그 자리) · 검증 중 취소 = Interrupted · 원본 유지 · 검증 바이트는 `on_bytes`에 안 더함 · 저널 `done` = 검증 뒤. **경계**: `overwrite`면 병합이 먼저라 검증 실패 시 대상의 같은 이름 항목은 이미 새 사본으로 바뀌어 있다(원본은 유지 → 데이터 유실 아님 · 대상 전용 파일 유지). **동작 변경**: `--move --verify`는 종전 무시 → 검증(느려질 수 있음 · 실패 가능 · 릴리스 노트 대상).

| 기능 | 설계 | 걸림돌 |
| --- | --- | --- |
| **작업 큐** | ✅ 슬라이스 1~3(b7f77c2 · 2d86426 · eb8e127 · 47340c7 · [§12](#12-작업-큐t-202---설계-확정-11-01--구현--다음-회차)) · 슬라이스 4 = 종결(사용자 11-01 · 아래) | 남은 = Windows · macOS 실기 · 바이트 전송 중 `kill -9` 뒤 복원 실측 |
| **최상위 항목 병렬** | 질문 없는 항목만 묶어 보내기 | docs/24 §6-3 — 충돌 질문 · 스테이징 · 실행 취소가 항목 순서에 묶임 |

## 9. 시험 계획

- **T1(ndir-xfer · 골격 시험 8/0)**: JobSpec JSON 왕복(빠진 키 = 기본 · 모르는 키 무시) · `validate`(빈 sources · 미러 원본 2개 · 위험 대상) · Report JSON · `exit_code` · Progress 줄 키.
- **T1 동등성**: 같은 작업을 **간단 인자 경로와 `--job` 경로**로 각각 만들었을 때 JobSpec이 같다(DR-34 ⑦ 한 길 보장).
- **프로세스(T4급 · `crates/nexa-copy/tests/cli.rs`)**: `--job` 복사/미러/검증/dry-run · `--progress` 줄이 모두 JSON으로 파싱되고 `phase`가 단계 순서(plan → transfer → verify/mirror/delete) · 종료 코드 = Report `exit_code()` · 취소 = 130.
- **재개(abc69f3)**: `resume::{done_items_from_report_and_journal, journal_appends_and_reads_back}` · spec 왕복/검증/상대 경로 · `journal_then_resume_skips_done_items`(이동 재개 포함) · `journal_waits_for_verify` · 프로세스 `journal_and_resume_skip_finished_items`.
- **속도 제한(071fc30)**: ndir-ops `pacer_limits_loop_and_parallel_copies`(8 × 128 KiB · 4스레드 ≥ 200 ms · 1 B/s stop < 1 s) · `Pacer::delay_for` 순수 판정 · ndir-xfer `max_bps_slows_transfer` · nexa-copy `parse_rate` · 프로세스 `max_bps_limits_copy_speed` — 실측 시험은 하한만 단언(흔들림 방지).
- **미러 내용 비교(4f2f3ac)**: ndir-xfer `mirror_by_content_and_delete_callback`(기본 미러 유지 · by_content 덮어씀 · 해시 진행 · 삭제 콜백 · 제외 파일 미해시) · 프로세스 `mirror_by_content_overwrites_same_size_and_time` · ndir-ops `content_verdict_treats_unreadable_as_differ`(5경우).
- **이동 검증(7b28766)**: ndir-ops `move_verify_checks_copies_before_cleanup`(통과 이동 · 불일치 = 오류 + 양쪽 유지 · copied에 없는 파일 미대조 · 범위 밖 = 검사 없음) · `verify_detects_mismatch_missing_and_type`(필터 안/밖 · GAP-027) · ndir-xfer `move_with_verify_still_moves`.
- **목록 대조(f745f3d · GAP-027 후반)**: ndir-ops `verify_collect_lists_copied_paths_and_listed_verify_ignores_newcomers`(목록 = 파일 · 빈 폴더 · 폴더 · 링크 안쪽 · 제외 파일 없음 · 신생 파일 ok · 수정 파일 불일치 · 빈 폴더 누락 적발 · 링크 안쪽 손상 적발 · 깃발 꺼짐 = 빈 목록).
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
4. cdylib = 진입 **`ndir_xfer_run`** · **패키지에 동봉** — 구현 ✅ T-202 ①(§7).

## 12. 작업 큐(T-202 ④ · 설계 확정 11-01 · 구현 = 다음 회차)

> 설계 = 개발 세션 · 코드 아님. 사용자 결정 4건 + 협업 검토 7건 채택(코드 읽기 근거 · journal 11-01 §4). 구현 때 정할 것 3건 = 사용자 결정 11-01로 본문에 반영 · §12.7 = 구현 전 확인.

### 12.1 사용자 결정(11-01)

1. 전송 중 새 전송 = **큐에 추가**(설정 `transfer.queue` 기본 on · off = 종전 거부).
2. 충돌 = **실행 차례에 인라인 질문**(대기 중엔 묻지 않음 · 질문이 떠 있으면 큐가 멈춤).
3. 재시작 = **물어보고 재개**(저널 기준 · "미완 전송 n건 — 재개 / 버림").
4. 진행 창 = **기존 창 + 대기 목록 줄**(카드형 T-96은 보류 유지).

### 12.2 자료 구조와 저장

- 큐 항목 = `QueuedJob { id, spec: JobSpec, clear_clipboard_after: bool, state: Queued|Running|Done(Report)|Failed|Canceled, added_at }`.
- 저장 = **설정 폴더/queue**(DR-9 · `NDIR_HOME`이면 그 아래) · `<id>.job.json`(JobSpec) + `<id>.journal`(ndir-xfer `journal` 키). **Done · Canceled · Failed는 파일을 지운다**(결과는 완료 토스트 · 로그 창).
- **UTF-8이 아닌 경로가 든 작업은 저장하지 않는다**: 메모리 큐에서만 실행 · 재시작 복원 제외 · 토스트 1회(`spec.rs`가 `to_string_lossy`라 저장본 경로가 달라진다). JobSpec 인코딩은 바꾸지 않는다(cdylib JSON 호환).
- **소유 표시(✅ eb8e127 · `platform/procid.rs` `process_start_time` = Linux `/proc/<pid>/stat` 22번째 · macOS `proc_pid_rusage(RUSAGE_INFO_V2).ri_proc_start_abstime` · Windows `OpenProcess` + `GetProcessTimes` · 새 외부 crate 0 · 잠금 = `pid\n기동시각` · 기동 때 claim · `persist_window`(종료 4경로)에서 해제 · 둘째 인스턴스 = 메모리 큐 + 상태줄 · `ops.dump` "queue n memory")** = `queue/owner.lock`(pid · 기동 시각). 기동 시 `platform::process_alive(pid)` 포트(신설 · DR-5 · Linux `/proc` · macOS `kill(0)` · Windows `OpenProcess`)로 확인하되 **pid와 기동 시각을 둘 다 대조**한다(11-01 기술 결정 · pid가 재사용되면 다른 프로세스로 판정) — 살아 있으면 복원 대신 "다른 인스턴스가 전송 중" 안내 · 죽었으면 잠금을 인수한다(앱에 단일 인스턴스 장치가 없어 둘째 프로세스가 같은 전송을 다시 돌리는 것을 막는다).
- **둘째 인스턴스의 새 전송 = 메모리 큐만**(사용자 11-01): 저장하지 않고 · 종료 때 비우고 · 상태바로 안내한다.

### 12.3 실행

- 워커 = 1개(순차). 머리 작업을 `run_with(spec, progress, cancel, delete, resolve) -> (Report, Outcome)`로 실행한다.
- **ndir-xfer API 확장 3건(✅ 슬라이스 1 · b7f77c2 · `run_full(spec, progress, cancel, Hooks) -> (Report, Outcome)` · `run`/`run_with`는 그 위 포장 · CLI · cdylib 동작 불변)**: ③ `on_event` = 엔진 `Event` 4종 통과(앱 세그먼트 바) · `lenient_sources` = 앱만 원본 존재 검사 생략(그 항목만 `errors`로 격리) · 조기 반환도 `Outcome.errors` 채움 · ① `ResolveFn` — 충돌 콜백 · `conflict` 정책 대신 호출자가 결정(앱 인라인 질문 채널 그대로) ② `Outcome` 반환 — Report `dest`는 사전 계획값이라, 앱 undo는 지금처럼 `Outcome.transferred`(실제 결과)로 `MoveBatchOp`/`CopyBatchOp`를 기록한다.
- 기존 `start_transfer` · `TransferShared` · `ops_tick`은 "큐 머리 1건 실행"으로 재배선한다(진행 창 · 상태바 `xfer` 칸 · 완료 토스트 · undo 유지). DnD 드롭은 종전에 `transfer.is_some()` 문지기 3곳(`dnd.rs` 발신 · 드롭 효과 · 드롭 처리)과 `paste` 선문지기가 먼저 막고 있었다(협업 코드 읽기 11-01) → 슬라이스 2(2d86426)에서 `transfer_blocks_new`(큐 off일 때만 막음)로 바꾸고 `paste` 선문지기를 지웠다.
- **바쁨 판정 분리(✅ 2d86426 · `transfer_running` · `queue_pending` · `transfer_busy` = 둘 중 하나 · undo/redo · 풀기 · 동기화 · 압축 · 상태바 · `ops.wait`/`idle.wait`/`jobs.wait`가 `transfer_busy`)**: `transfer_running()`(실행 중) / `queue_pending()`(대기 있음) · T4 `ops.wait` = 둘 다 비울 때까지 · `extract_job`/`sync_job` 중이면 머리 작업은 거부 대신 **대기**.
- **실행 차례에 재검사**: 원본 · 대상이 없으면 Failed + 안내.
- 미러 삭제 콜백(`platform` 휴지통 포트 주입) = **종결(사용자 11-01 · 슬라이스 4)**: 앱 미러 삭제는 GAP-029 경로(잠금 검사 → 대화상자 → 비동기 휴지통 → 재비교)를 유지하고, `run_with(delete)` 콜백은 CLI · cdylib 내장용으로 남긴다. 워커 스레드의 동기 콜백 하나로는 그 UX를 담지 못하고, 비교 창 동기화는 큐 범위 밖(§12.6)이라 주입하지 않는다.

### 12.4 화면과 재시작

- 진행 창: 아래에 "대기 중 n건" 목록(라벨 · 크기 · [위로] [아래로] [삭제]) · 상태바 `xfer` 칸 = 실행 중 % + "대기 n".
- 재시작: 기동 시 `queue/`에 Running/Queued가 남아 있고 소유자가 죽었으면 토스트/대화상자 "미완 전송 n건 — 재개 / 버림" → 재개 = `resume: <journal>`을 넣어 다시 큐.
- **재개 항목 충돌 질문 = 이전 전송이 만든 대상만 덮어쓰기 기본**(사용자 11-01): 저널에 started 줄이 있고 done 줄이 없는 항목 = "이전 전송에서 중단된 사본 · 덮어쓰기 권장" · 기본 버튼 = 덮어쓰기 · 그 밖은 건너뛰기가 기본이다(OPS-004 "조용히 덮어쓰지 않는다" · 판별 근거 = §12.7). 완료 토스트에 "재개 n건"(저널이 최상위 항목 단위라 중단 항목의 대상은 일부만 있을 수 있다).

### 12.5 설정 키(REGISTRY · `CAT_TRANSFER`)

| 키 | 형식 · 기본 | 뜻 |
| --- | --- | --- |
| `transfer.queue` | Bool · on | 전송 중 새 전송을 큐에 추가 · off = 종전 거부 |
| `transfer.queue_restore` | Enum `ask`\|`auto`\|`discard` · `ask` | 재시작 때 미완 전송 처리 |

`transfer.queue_parallel`은 만들지 않는다(병렬을 만들 때 추가).

### 12.6 시험 · 범위 · 슬라이스

- **시험**: T3 = 큐 추가/순서/삭제 · 실행 중 질문 대기 · 재시작 복원(격리 홈 `queue/` 고정물) · `transfer.queue` off = 종전 거부. T4 시나리오 1건 = 두 번 연속 copy → 둘 다 완료.
- **범위 밖**: 폴더 비교 창 `sync_job`(별도 유지) · 장치별 병렬 · 카드형 진행 창 · 일시 정지.
- **구현 순서**: (1) ✅ b7f77c2 — ndir-xfer 확장 3건 + 앱이 ndir-xfer로 전송(동작 동일 · 큐 없음) · `process_alive` 포트는 (3)으로 미룸(호출자 없는 함수 = dead_code) → (2) ✅ 2d86426 — 큐 자료 구조(`app/queue.rs` · `설정 폴더/queue/<id>.job.json` + `<id>.journal` · 시험 = 메모리) + 설정 키 `transfer.queue` + 진행 창 대기 줄([위로][아래로][삭제]) + 상태바 "대기 n" · 저널 started 줄은 GAP-030에서 이미 · 큐 자료 구조 + 저장 + 설정 키 + 진행 창 대기 줄 + 저널 started 줄(§12.7) → (3) ✅ eb8e127 재시작 복원(`transfer.queue_restore` ask|auto|discard · 대화상자 "끝나지 않은 전송 n건 — 재개 / 버림" · Esc = 그대로 두고 다음 기동에 다시 · **정상 종료도 파일 유지 = 사용자 결정 ③을 크래시 · 종료 같은 경로로(ⓑ · 협업 권고와 같음 · 종료 때 질문 ⓐ는 미구현)** · 종료 = 실행 중 전송을 취소 경로로(부분 파일 · 스테이징 정리 · 최대 3초) · 재개 충돌 질문 = `Hooks.ask_interrupted` + 글에 "중단된 사본 · 크기 · 시각" · 끊긴 이동 Skip 규칙 유지) → (4) 종결(사용자 11-01 · 휴지통 콜백 주입 안 함 · 사유 = §12.3).

### 12.7 재개 충돌의 판별(협업 코드 읽기 · 실측 · 개발 11-01 · CLI 구현 = 44ac475)

**판별 = 저널 started 줄(수정 시각이 아니다).** 사용자 결정 문구 "이전 전송이 만든 대상만 덮어쓰기 기본"은 그대로 두고 판별 방법만 정했다(개발 제안).

- **started 줄**: 항목 시작 때 `{"status":"started","source","dest","note":"new"|"overwrite"}`를 덧붙인다(T-202 ② 저널 모양 그대로). `done_items`는 쌍별 마지막 기록의 `done`만 뽑으므로 호환된다. 재개 때 "started 있고 done 없음" = 이전 전송이 그 대상을 만들었거나(new) 사용자가 덮어쓰기로 결정했던(overwrite) 항목 → 기본 = 덮어쓰기. 그 밖(시작 전 · 건너뛰기로 답한 항목 = `Conflict::Skip`이 `ItemStart` 앞에서 돌아감) = 건너뛰기 기본.
- **수정 시각을 쓰지 않는 이유**(코드 읽기): 저널에 시각이 없고 · 중단된 항목은 마지막 저널 줄보다 뒤에 쓰이고 · 끝난 사본은 수정 시각을 원본 값으로 보존한다(`preserve_after_copy`).
- **프로세스가 죽었을 때 남는 것**(실측 11-01 · Linux 샌드박스 · `max_bps` 3 MB/s · 30 MB · 2초 뒤 `kill -9`): 새 대상 = **부분 파일이 대상 이름 그대로 남음**(6.75 MB) · 덮어쓰기 = 옛 대상 그대로 + 스테이징 `.big.bin.nexa-tmp-<pid>-0` 잔여(6.75 MB). 정리 코드는 오류 · 취소 경로에만 있다(GAP-030). 그래서 파일 항목도 재개 때 충돌 질문이 뜬다. **안전 규칙(44ac475)**: 대상을 지우거나 덮는 것은 복사 + 원본 존재일 때만 — 중단된 이동은 대상이 유일한 사본일 수 있다(rename 직후 · 원본 정리 중). **끊긴 이동은 충돌 정책과 무관하게 Skip**(4ac8bd9 · `conflict: overwrite` 재개가 남은 원본으로 전체 대상을 교체해 5439개 소실한 실측 뒤) · 대상이 없을 때만 평소대로 이동한다. CLI `--resume`은 질문 없이 처리하고, 앱(슬라이스 3)은 이 문서대로 묻는다. 저널 없는 재실행은 끊긴 작업인지 구분할 수 없지만 병합(DR-35)이라 대상 전용 파일을 잃지 않는다. 재시작 복원은 재개 판단 때문에 저널을 쓴다.
- **T-208 뒤 동작(11-09 · 6164ed8 · 위 실측은 그 전 기록)**: 새 대상(파일 · 폴더 · 같은 폴더 복제 " (2)")도 스테이징 `.<이름>.nexa-tmp-<pid>-<seq>`에 쓰고 끝에 커밋한다(교체 = `commit_replace` · 새 대상 = `fastcopy::rename_noreplace` · 그 사이 생긴 같은 이름은 덮지 않음). 바이트가 흐르는 동안 **최종 이름은 없고**, 폴더는 통째로 스테이징 폴더 안에 쌓인다. 죽으면 숨김 `tmp`만 남고 재개 · 앱 복원 "버림"이 치운다(Linux 격리 실측 ✓ [journal 11-09 §2](journal/2026-11-09.md)). 전송 시작 때 대상 폴더의 죽은 pid `tmp`를 이름과 무관하게 한 번 쓴다(`sweep_staging_leftovers` · 2f3a512 · `old`는 유지 · **나이 가드 33eef06** = 1시간 넘게 변경 없는 것만 · 폴더는 안쪽까지 가장 최근 변경을 본다 · 시각을 못 읽으면 지우지 않는다) — 저널 없는 `nexa-copy` 재실행 뒤 `tmp`는 1시간 안에는 남고 그 뒤의 전송이 치운다(실측 ✓). 나이 가드는 생존 판정이 이 기계의 프로세스 표뿐이라 공유 폴더에서 다른 기계가 쓰는 중인 `tmp`를 지우지 않으려는 것이다. 이름을 짚는 재개 · 버림 경로는 나이를 보지 않는다.
- **보조 검사 폐기(개발 동의)**: "대상 수정 시각 > 저널 수정 시각이면 건너뛰기 기본"은 항목이 차례로 도는 동안 started 줄이 저널의 마지막 쓰기라, 중단된 새 대상에서 늘 참이 된다(코드 읽기 · 위 실측의 대상 시각도 같은 방향 · 저널 비교 자체는 실행 안 함). 중단 뒤 사용자가 고친 대상은 가를 수 없는 잔여 위험으로 남기고, **질문 창에 대상 크기 · 시각을 표시**해 사람이 보게 한다.
- **구현 때 함께 볼 것**: ① `Event::ItemStart`에 `overwrite: bool` 추가(ndir-ops 코어 · 추가만 · gate full) ② 저널 줄 수를 세는 시험(`journal_then_resume_skips_done_items` 등) 갱신 ③ copy + verify는 done이 검증 때 기록되므로 검증 중 죽으면 전 항목이 덮어쓰기 기본(결정과 맞음 · 다시 복사량은 큼) ④ `Placement::Duplicate`는 묻지 않아 재개 때 부분 파일 " (2)"가 남고 새 " (3)"가 생긴다(GAP-030) ⑤ ⑥ 최상위 병렬이 들어오면 "started 있고 done 없음"이 여러 건일 수 있다.
