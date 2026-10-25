# 24 — 고속 복사: 운영체제별 분석과 전략 계층 (NEW-007 1차)

> 사용자 10-05: "고속 복사에 대한 부분은 운영체제별 기능 분석하고, 윈도우 용은 개발이 의미가 있을 것 같으니 먼저 진행."
> 근거 결정 = [DR-17](10-decision-record.md) · 기능 정의 = [22 NEW-007](22-dir3-features.md) · 구현 = `crates/ndir-ops/src/fastcopy.rs`.
> 이 문서는 T-97(대량 전송 엔진 설계)의 **복사 전략 부분**을 먼저 확정한 것이다 — 미러/차등 · 검증 · 재개 · 필터 · 큐는 T-97 본문에서 다룬다.

## 1. 무엇이 복사를 느리게 하는가

| 구간 | 지배 비용 | 처방 |
| --- | --- | --- |
| 작은 파일 다수 | 파일당 고정 비용(열기 · 만들기 · 닫기 · 메타데이터 기록 · 실시간 악성코드 검사) — 데이터 양과 무관 | **파일 단위 병렬** · 파일당 시스템 호출 줄이기 |
| 큰 파일 | 장치 순차 처리량 · 사용자 공간 왕복 복사 · 파일 캐시 오염 | **커널 안 복사**(사용자 공간을 거치지 않음) · 같은 볼륨이면 **블록 복제**(데이터를 옮기지 않음) · 네트워크면 **서버 쪽 복사** |
| 회전 디스크 | 헤드 탐색 | 큰 파일은 한 번에 하나 · 병렬 수 제한 |

dir2 엔진(= 지금까지의 dir3)은 어디서나 "4 MiB 버퍼로 읽고 쓰는 루프 · 한 번에 파일 하나"였다. 그래서 ① 작은 파일이 많으면 느리고 ② 블록 복제 · 서버 쪽 복사를 못 쓰고 ③ **사본의 수정 시각이 복사한 시각으로 바뀌고**(탐색기는 보존) ④ Unix에서 **실행 권한이 사라졌다**.

## 2. 운영체제별 수단

### Windows

| 수단 | 효과 | 조건 · 주의 | 채택 |
| --- | --- | --- | --- |
| `CopyFileExW` / `CopyFile2` | 커널 복사 경로 · 진행 콜백 · 취소 · 수정 시각 · 속성 · 대체 데이터 스트림 · 암호화/희소 속성 보존 | 실패/취소 시 대상 파일을 스스로 지움 | **1차 채택**(`CopyFileExW`) |
| ReFS / Dev Drive **블록 복제** | 같은 볼륨 안 복사가 데이터 이동 없이 끝남(거의 즉시) | Windows 11 24H2 · Server 2025부터 `CopyFileEx`가 자동 사용 · 그 전 버전은 `FSCTL_DUPLICATE_EXTENTS_TO_FILE` 직접 호출 | 1차 = 자동분만(OS가 해 주는 만큼) · 직접 호출은 2차 후보 |
| SMB **서버 쪽 복사**(`FSCTL_SRV_COPYCHUNK`) · ODX | 같은 서버 안 복사가 클라이언트를 거치지 않음 | `CopyFileEx`가 자동 사용 | 1차(자동) |
| `COPY_FILE_NO_BUFFERING` | 파일 캐시를 거치지 않음 — 메모리보다 훨씬 큰 파일에서 다른 프로그램의 캐시를 밀어내지 않음(robocopy `/J`) | **이 PC 실측에서는 2배 이상 느렸다**(§4) | 설정으로만(`transfer.unbuffered_mb` · 기본 0 = 끔) |
| 파일 단위 **병렬**(robocopy `/MT`) | 작은 파일 다수에서 3 ~ 5배 | 큰 파일이 겹치면 손해 → 큰 파일(32 MiB 이상)은 한 번에 하나 | **1차 채택** |
| 직접 겹침 I/O + 읽기/쓰기 스레드 분리(FastCopy 방식) · 사전 할당 | 서로 다른 물리 디스크 사이 큰 파일에서 이득 | 섹터 정렬 버퍼 · 구현 큼 | T-98(측정 뒤 결정) |
| `IOCTL_STORAGE_QUERY_PROPERTY`(탐색 지연) | SSD/HDD 판정 → 병렬 수 자동 | 접근 권한 0으로 열어 관리자 불필요 · 이동식은 실패할 수 있음 → 보수적으로 "있음" | **1차 채택** |
| ACL · 소유자 보존(`COPY_FILE_COPY_ACL` 계열 · `SetSecurityInfo`) | 권한까지 복제 | 탐색기 기본 동작 아님(대상 폴더 상속) | T-97 "속성 보존 포트" |

### Linux

| 수단 | 효과 | 조건 | 계획 |
| --- | --- | --- | --- |
| `ioctl(FICLONE)` · `copy_file_range(2)` | Btrfs · XFS(reflink) · bcachefs에서 블록 복제 · 그 밖에서도 커널 안 복사 · NFS 4.2/SMB 서버 쪽 복사 | 커널 4.5+(교차 파일 시스템은 5.3+) · 실패 시 루프로 | 2차(다음 차수 — libc 바인딩 없이 `std::fs::copy`가 이미 `copy_file_range`를 쓴다 → 진행 통지 단위 조정만 하면 된다) |
| `sendfile(2)` · `splice(2)` | 커널 안 복사 | `copy_file_range`가 안 되는 조합의 대안 | 필요 시 |
| `posix_fadvise(SEQUENTIAL)` · `fallocate` | 미리 읽기 확대 · 조각화 감소 | — | 2차 |
| `/sys/block/<dev>/queue/rotational` | SSD/HDD 판정 | 장치 매핑 경유 시 추적 필요 | 2차(지금 = 보수적 4스레드) |
| `io_uring` | 작은 파일 다수의 시스템 호출 묶기 | 구현 큼 · 커널 편차 | 보류 |

### macOS

| 수단 | 효과 | 조건 | 계획 |
| --- | --- | --- | --- |
| `clonefile(2)` · `copyfile(3)`(`COPYFILE_CLONE`) | APFS 같은 볼륨 안 복사가 즉시(쓰기 시 복사) · 확장 속성 · ACL · 리소스 포크 보존 | APFS · 같은 볼륨 | 2차(`std::fs::copy`가 `fclonefileat` → `fcopyfile` 순으로 이미 쓴다 → 큰 파일 진행 통지만 보완) |
| `fcopyfile` + `COPYFILE_STATE_STATUS_CB` | 커널 안 복사 + 진행 콜백 | — | 2차 |
| `F_NOCACHE` · `F_RDAHEAD` | 캐시 우회 · 미리 읽기 | — | 필요 시 |

**정리**: 세 OS 모두 "OS가 주는 파일 복사 호출을 쓴다 → 블록 복제 · 서버 쪽 복사 · 속성 보존이 따라온다 → 그 위에서 파일 단위 병렬"이 같은 모양이다. Windows가 먼저인 이유는 ① 지금 쓰는 루프와의 격차가 가장 크고(메타데이터 미보존 포함) ② 진행 콜백 · 취소가 호출 하나에 들어 있어 엔진 계약(진행 · 취소 · 부분 파일 정리)을 그대로 지키기 쉽기 때문이다.

## 3. 구현(1차 · Windows 우선)

- **자리**: `ndir-ops::fastcopy`(전략 계층) — dir2 엔진(`lib.rs`: 충돌 · 스테이징 교체 · 교차 볼륨 이동의 원본 정리 · 개별 격리 · 실행 취소 기록)은 그대로 두고 **파일 하나를 옮기는 방법**과 **폴더 안 파일을 도는 방법**만 바꿨다. OS 의존은 그 파일의 `os` 모듈에만(NEW-007 "엔진 크레이트의 os 포트" · 외부 crate 0 — `kernel32` 직접 선언).
- **파일 1개**: Windows = `CopyFileExW`(긴 경로는 `\\?\` 꼴로 변환 · 진행 = 기본 스트림 증분 · 취소 = 콜백이 `PROGRESS_CANCEL`) → 커널 경로가 지원되지 않는 파일 시스템이면 루프로 다시. 그 밖 = 루프 + **수정 시각 · 권한 보존**(dir3 보강).
- **폴더**: 한 번 훑어 폴더를 만들고 작업 목록을 모은 뒤 작업 스레드가 나눠 복사. 진행 통지는 부른 스레드에서만. 하나가 실패하면 나머지도 멈추고 **진짜 오류가 취소보다 우선**해 보고된다. 교차 볼륨 이동용 `copied`는 "파일들 → 폴더(자식 → 부모)" 순서를 지킨다. 링크 규칙(폴더 링크 = 대상 내용 복사 · 링크 안쪽은 정리 대상에서 제외 · 순환 = 오류)은 종전 그대로.
- **자동 스레드 수**(순수 함수 `thread_count`): 설정이 1 이상이면 그 값 · 0이면 양쪽 모두 탐색 지연 없음 = `min(코어, 8)`, 아니면 `min(코어, 4)`. 32 MiB 이상 파일은 병렬 중에도 한 번에 하나.
- **설정**(파일 전송 · 고급): `transfer.native`(기본 켬) · `transfer.threads`(0 = 자동 · 1 ~ 16) · `transfer.unbuffered_mb`(기본 0 = 끔).

## 4. 실측 (10-05 · 이 PC · 릴리스 빌드 · `cargo test -p ndir-ops --release bench_copy -- --ignored --nocapture`)

| 작업 | 전략 | C:(탐색 지연 없음) | D:(탐색 지연 있음/미상) |
| --- | --- | --- | --- |
| 16 KiB × 4,000개 | 루프 · 1스레드(**종전**) | 15,037 ms | 18,754 ms |
| | `CopyFileExW` · 1스레드 | 15,022 ms | 15,452 ms |
| | `CopyFileExW` · 4스레드 | 10,406 ms | 5,201 ms |
| | `CopyFileExW` · 8스레드 | **5,501 ms** | **4,090 ms** |
| 512 MiB × 1개 | 루프(**종전**) | 10,807 ms | 5,827 ms |
| | `CopyFileExW` 버퍼드 | **6,107 ms** | **5,306 ms** |
| | `CopyFileExW` 비버퍼드 | 15,288 ms | 12,624 ms |

읽는 법: ① 작은 파일은 호출 종류보다 **병렬**이 결정적(2.7 ~ 4.6배) — "탐색 지연 있음"으로 판정된 D:에서도 이득이 커서 자동 값을 1이 아닌 4로 뒀다 ② 큰 파일은 커널 경로가 같거나 빠르다(같은 볼륨 ReFS · 같은 서버 SMB라면 여기서 자릿수가 달라진다 — 이 PC에는 해당 볼륨이 없어 측정하지 못했다) ③ 비버퍼드는 이 PC에서 손해 → 기본 끔. 1회 측정값이므로 장치 · 백신 설정에 따라 달라진다.

**C: SSD 재측정(release · 10-05 저녁 · T6 1회 · [journal perf-2026-10-05](journal/perf-2026-10-05.md))**

| 작업 | 전략 | C: 재측정 |
| --- | --- | --- |
| 작은 파일 × 4,000개 | 루프 · 1스레드(**종전**) | 13,062 ms |
| | 루프 · 4스레드 | 10,922 ms |
| | `CopyFileExW` · 1스레드 | 10,819 ms |
| | `CopyFileExW` · 4스레드 | 5,357 ms |
| | `CopyFileExW` · 8스레드 | **4,226 ms**(종전 대비 3.1배) |
| 512 MiB × 1개 | 루프(**종전**) | 3,805 ms |
| | `CopyFileExW` 버퍼드 | **1,217 ms**(3.1배 · ≈ 440 MB/s) |
| | `CopyFileExW` 비버퍼드 | 2,481 ms |

재측정 읽는 법: 절대값은 위 첫 측정보다 빠르지만(캐시 · 백신 상태 차이로 보임) 결론은 같다 — 작은 파일은 병렬 + 커널 경로 둘 다 필요(루프 4스레드는 거의 이득 없음) · 큰 파일은 `CopyFileExW` 버퍼드가 최선 · 비버퍼드는 여전히 손해.

## 5. 남은 것

| 항목 | 내용 |
| --- | --- |
| Linux · macOS 2차 | ✅ 10-24(T-164 · §6) — Linux `FICLONE` → `copy_file_range` · macOS `copyfile(ALL \| CLONE)` + 진행 콜백 · Linux `queue/rotational` 장치 판정 · macOS 장치 판정은 후속 |
| 최상위 항목 병렬 | 지금 병렬은 **폴더 안**에만 적용 — 파일 수천 개를 직접 선택해 복사하는 경우는 여전히 차례로(전송 진행 창의 항목 세그먼트 모델과 함께 T-96/T-98에서) |
| ReFS 블록 복제 직접 호출 | Windows 11 24H2 이전에서의 같은 볼륨 즉시 복사 |
| 검증 · 미러/차등 · 재개 · 필터 · 속도 제한 · 큐 · `nexa-copy` CLI | T-97 설계 → T-98 |
| T6 성능 기준선 | `bench_copy`를 기준선 하네스(T-08)에 편입 |

## 6. 2차 — 크로스플랫폼 설계와 운영체제별 구현 방식(T-164 · 10-24 · 사용자 "설계·개발 + OS별 방식 요약")

### 6-1. 한 모양으로 맞춘 계층

```
전송 엔진 lib.rs (OS 무관)        ──  항목 순회 · 충돌 · 스테이징 교체 · 교차 볼륨 이동 · 실행 취소 · 취소
   └ fastcopy.rs 전략 계층 (OS 무관) ──  파일 1개 = copy_file(native?) · 폴더 = plan → 작업 스레드 N개 · 큰 파일 직렬화 · 진행 합산
        └ fastcopy::os (OS별 1모듈 · cfg)  ──  copy_file(src, dest, overwrite, on_bytes, stop) · unsupported(err) · seek_of(path)
             Windows  CopyFileExW            Linux  FICLONE → copy_file_range            macOS  copyfile(ALL | CLONE)
```

- **계약은 셋이 같다**: `copy_file`은 "증분 바이트를 보고하고 · `stop`이 참이면 `Interrupted` · 실패/중단 때 이번에 만든 대상은 남기지
  않는다 · 수정 시각과 권한을 보존한다". 상위 계층은 어느 OS인지 모른다(DR-5와 같은 포트 모양 · 외부 crate 0 — libc/libSystem/kernel32
  심벌만 `extern`으로 선언).
- **폴백 규칙도 같다**: OS 경로가 "지원 안 함"(`unsupported`)으로 끝나면 — 아직 보고한 바이트를 빼고 — 읽고 쓰는 루프(dir2 원형)로
  다시 한다. 그래서 오래된 커널 · 특수 파일 시스템 · 네트워크 드라이브에서도 결과는 같고 속도만 다르다.
- **병렬은 OS 위에서**: 폴더 안 파일 작업을 스레드 N개가 나눈다(자동 = 양쪽 장치가 탐색 지연 없음이면 8 · 아니면 4 · 설정 1 ~ 16) ·
  32 MiB 이상은 한 번에 하나(회전 디스크 보호). 장치 판정 `seek_of`만 OS가 답한다.
- **설정은 공통 3키**: `transfer.native`(켬) · `transfer.threads`(0 = 자동) · `transfer.unbuffered_mb`(Windows만 의미 · 기본 끔).

### 6-2. 운영체제별 구현

| | Windows(1차 · 10-05) | Linux(2차 · 10-24) | macOS(2차 · 10-24) |
| --- | --- | --- | --- |
| 호출 | `CopyFileExW` + 진행 콜백 | ① `ioctl(FICLONE)` ② `copy_file_range` 32 MiB 청크 | `copyfile(COPYFILE_ALL \| COPYFILE_CLONE)` + `COPYFILE_STATE_STATUS_CB` |
| 같은 볼륨 즉시 복사 | ReFS/Dev Drive 블록 복제(24H2+ 자동) | Btrfs · XFS(reflink) · bcachefs = FICLONE(데이터 이동 없음) | APFS = `clonefile`(쓰기 시 복사) — 복제되면 콜백 없이 끝나므로 합계를 채운다 |
| 네트워크 | SMB 서버 쪽 복사(`FSCTL_SRV_COPYCHUNK`) 자동 | NFS 4.2 · SMB 3 서버 쪽 복사(`copy_file_range`) | SMB = 커널 복사 |
| 보존 | 수정 시각 · 속성 · 대체 스트림(호출 안) | 수정 시각 · 권한(복사 뒤 `set_modified`/`set_permissions`) | 수정 시각 · 권한 · 확장 속성 · ACL(`COPYFILE_STAT\|XATTR\|ACL`) |
| 중단 | 콜백 `PROGRESS_CANCEL` → OS가 대상 삭제 | 청크 사이 `stop` → 대상 삭제 | 콜백 `COPYFILE_QUIT`(ECANCELED) → 대상 삭제 |
| 폴백 조건(**한 바이트도 안 옮겼을 때만** · 중간 실패는 원 오류) | `ERROR_INVALID_FUNCTION/NOT_SUPPORTED/INVALID_PARAMETER` | 첫 호출 `EPERM/EBADF/EXDEV/EINVAL/ENOSYS/EOPNOTSUPP` 또는 **첫 호출 0 바이트**(procfs/sysfs · 일부 FUSE — std `kernel_copy` 규칙) | `ENOTSUP/EINVAL/EXDEV` · **심볼릭 링크 원본**(CLONE = NOFOLLOW_SRC라 루프로 내용 복사 → 다른 OS와 같은 결과) |
| 장치 판정 | `IOCTL_STORAGE_QUERY_PROPERTY`(SeekPenalty) · UNC = 없음 | `st_dev` → `/sys/dev/block/<maj>:<min>/queue/rotational`(파티션 = 상위 `../queue`) · major 0(Btrfs/ZFS/overlayfs 익명 장치 · NFS · CIFS · tmpfs · FUSE) = **모름 = 있음**(자동 4 · 네트워크 판정은 `statfs` 후속) | 아직 없음 = 알 수 없음(자동 4) — IOKit 특성 조회는 후속 |
| 캐시 우회 | `COPY_FILE_NO_BUFFERING`(설정 · 실측 손해라 기본 끔) | `posix_fadvise(SEQUENTIAL)`만(O_DIRECT는 정렬 제약 · 보류) | `F_NOCACHE` 보류 |
| 덮어쓰기 | `FAIL_IF_EXISTS` 플래그 | `create_new`/`create` | CLONE = EXCL 함의 → 덮어쓰기는 먼저 지움 · 아니면 `AlreadyExists` |

### 6-3. 아직 안 한 것(결정 거리 · 사용자 지시 대기)

| 항목 | 내용 · 왜 미뤘나 |
| --- | --- |
| 최상위 항목 병렬 | 지금 병렬은 **폴더 안**에만 — 파일 수천 개를 직접 골라 복사하면 차례로. 엔진 항목 루프(`transfer`)가 항목마다 충돌 질문 · 스테이징 · 실행 취소 기록을 순서대로 하므로, 병렬화는 "질문 없는 항목만 묶어 보내기"(T-96 카드형 진행 창의 항목 세그먼트)와 함께 설계해야 한다. |
| Windows ReFS 블록 복제 직접 호출 | 24H2 이전에서 같은 볼륨 즉시 복사(`FSCTL_DUPLICATE_EXTENTS_TO_FILE` · 클러스터 정렬 · 무결성 스트림 끔 조건). 이 PC에 ReFS 볼륨이 없어 실측 불가. |
| macOS 장치 판정 | IOKit `kIOPropertyMediumTypeKey`(Solid State) 조회 → 자동 8. Mac 실기 필요. |
| 큰 파일 읽기/쓰기 분리(FastCopy 방식) | 서로 다른 물리 디스크 사이에서만 이득 · 구현 큼 · 측정 뒤 결정(T-98). |
| 검증 · 미러/차등 · 재개 · 필터 · 속도 제한 · 큐 · `nexa-copy` CLI | T-97 설계 → T-98(NEW-007 ②~⑩). |
| T6 기준선 | `bench_copy`를 perf-baseline에 편입 · Linux/macOS 실측은 그 PC 세션에서. |

## 7. 명령행 실행파일 `nexa-copy`(1차 · 10-24 · 사용자 "별도 실행파일로 제공해서 cli로")

- **자리**: `crates/nexa-copy`(bin · 의존 = ndir-ops뿐 · 창 · 설정 파일 · 라이선스 없음 · 외부 crate 0) — NEW-007 제공 형태 "bin `nexa-copy`".
  엔진은 앱과 **같은 길**(`ndir_ops::transfer` + `fastcopy` 전략 계층)이라 같은 폴더 규칙 · 충돌 정책 · 스테이징 교체 · 교차 볼륨 이동 ·
  항목별 실패 격리 · OS 복사 경로 · 작은 파일 병렬이 그대로다.
- **사용**: `nexa-copy [options] <source>... <destination-folder>` — `--move` · `--overwrite`(기본 = 건너뜀) · `--threads <n>`(0 = 자동) ·
  `--no-native` · `--unbuffered-mb <n>`(Windows) · `-n`/`--dry-run` · `-q` · `-h` · `-V`. 진행 · 요약은 stderr(stdout은 비움 · 파이프 친화) · 영어 고정.
- **dry-run ✅ 10-24**(T-97 "dry-run 보고서" 1차): 엔진에 `plan_transfer`(`ndir-ops`) — `transfer`와 **같은 판정 함수 `placement`**(같은 폴더
  규칙 · 자기/하위 이동 · 자연 경로 충돌 → `resolve`)로 항목마다 자리 · 결정(`PlanAction` = Transfer · Duplicate · Overwrite · Skip · NoOp · Error) ·
  크기(폴더 = 재귀 합) · `total_bytes`/`active_bytes`를 돌려주고 파일 시스템은 바꾸지 않는다(앱의 진행 창 미리보기 T-96에서도 쓸 수 있다).
  CLI는 stdout에 탭 구분 한 줄/항목(`action · bytes · source · destination · note` · note = `exists` 또는 오류 사유) · 요약은 stderr ·
  종료 0(실행하면 실패할 항목 있으면 1). 한계: 같은 이름 원본 여럿의 순번 복제는 둘째 이후 번호가 실행 때 정해진다.
- **종료 코드**: 0 = 전부 완료 · 1 = 실패 항목 있음 · 2 = 인자/대상 오류 · 130 = 취소.
- **동봉(DR-31 ① · 10-25)**: 전 채널 — Windows MSI(파일 + PATH 기능) · 포터블 zip(winget `PortableCommandAlias nexa-copy` · choco 자동 shim) · macOS `.app/Contents/MacOS/nexa-copy` + pkg postinstall `/usr/local/bin/nexa-copy` 링크(uninstall.sh가 지움 · dmg는 INSTALL.txt 안내) · deb/rpm `/usr/bin/nexa-copy`. release.yml 스모크가 `nexa-copy --version`과 제거 뒤 잔여를 본다.
- **시험**: `cli::tests`(인자 해석 순수 · 사람이 읽는 수치 · 샌드박스 실행 = 복사 · 건너뜀 · 덮어쓰기 · 이동 · 오류 코드).
- **프로세스 시험(T4급 · 10-24 사용자 "테스트 자동화")**: `crates/nexa-copy/tests/cli.rs` = 실제 실행 파일을 띄워 도움말/버전/인자 오류 · 복사(내용 대조 · 100 % 줄) · 건너뜀(0 B of) · `--overwrite -q` · `--move` · `--no-native --threads 1` · 없는 원본/대상 2 · **취소**(512 MiB 희소 파일 + 루프 경로 + `NEXA_COPY_CANCEL_AFTER_MS=20` 시험 훅 = Ctrl+C 대역 → 130 · `canceled` · 부분 파일 없음 · 원본 그대로) — `cargo test -p nexa-copy`로 3-OS CI에서 돈다.
- **Ctrl+C 취소 ✅ 10-24**(`ctrlc.rs` · Windows `SetConsoleCtrlHandler` / Unix `signal(SIGINT)` → 프로세스 전역 `cancel` 원자값 → 엔진이 항목/청크(32 MiB) 경계에서 멈추고 부분 파일 정리 → 요약 `canceled` · 130 · `run_with(cancel)` 시험).
- **검증 ✅ 10-25**(`--verify` · T-97 "검증" 1차): 전송이 끝난 뒤 (원본, 사본) 쌍마다 크기 + SHA-256(`ndir_ops::hash::hash_file` · 엔진과 같은
  해시) · 폴더는 재귀(원본 기준 · 사본에만 있는 것은 무시) · 불일치/누락/종류 불일치 = `verify failed: …` + 종료 1 · `--move`는 원본이 없어 무시 ·
  진행 `verifying · n of m`(m = 복사한 크기 × 2 · 250 ms) · 요약 `verified n file(s) · x read · k mismatch(es)[ · s skipped item(s) not verified] in t` ·
  `--move`면 `note: --verify is ignored with --move` 한 줄 · 취소면 `verification skipped` · Ctrl+C = 130(협업 수동 점검 10-25 반영). 시험 `verify_detects_mismatch_missing_and_type` +
  프로세스 시험(`--verify` 41 파일).
- **필터 ✅ 10-25**(`--include <glob>` · `--exclude <glob>` · 반복 가능 · `;`로 여러 개(앱의 패턴 선택/빠른 필터와 같은 습관 — 협업 점검 10-25) · T-97 "필터" 1차): 엔진 `fastcopy::NameFilter` + `with_filter`(thread_local ·
  계획 단계 `plan`에서 거른 항목은 복사도 · 교차 볼륨 이동의 원본 정리도 안 함 = 원본에 남음) · 규칙 = exclude(파일·폴더) 우선 · include는 파일만
  (폴더는 늘 들어감) · 대소문자 무시 · 최상위 원본은 CLI가 걸러 `filtered out:` 안내 · dry-run도 최상위 반영(폴더 안은 실행 때). 한계: 진행
  총량(`size_of`)은 거르기 전 크기라 끝에 100 % 미만일 수 있다 · include가 파일을 하나도 안 고르면 `note: --include matched no files` 경고(10-25).
- **미러 ✅ 10-25**(`--mirror` · T-97 "미러/차등" CLI 1차 · 엔진 = 앱 폴더 동기화와 같은 `compare::compare` + `plan(LeftToRight · mirror)` +
  `apply_copies`): 원본 폴더 1개 → 대상 폴더를 같게(크기 + 수정 시각 · 허용 2초 · 재귀) — 새것/바뀐 것 복사 뒤 대상에만 있는 것 **영구 삭제**
  (휴지통 없음 · `deleted:` 한 줄씩) · **안전장치**(협업 10-25 · robocopy /MIR 사고 유형): ① 대상이 드라이브 루트·사용자 홈·원본과 겹침 = 거부(2)
  ② 지울 것이 있으면 `--yes` 필요 — 없으면 `would delete:` 목록 + 종료 2(비대화형 CLI라 프롬프트 대신) · 지울 것이 없으면 `--yes` 불필요 ·
  ③ 휴지통 `--trash`는 후속(CLI는 platform/ 포트 미사용) · `--dry-run` = copy/overwrite/delete 탭 줄 · `--include/--exclude` = 상대 경로 이름으로 거름(거른 것은 지우지도
  않음) · `--verify` = 복사한 파일 대조 · 원본 2개/파일/겹침 = 2 · 요약 `mirror: a copied · b deleted · c failed · x in t`. 시험 `mirror_plan_and_rows`(T1) ·
  `mirror_makes_destination_identical`(프로세스). 한계: 내용 비교(`by_content`) 옵션은 후속 · 양방향(`Both`)은 CLI에 없음(앱 동기화 창).
- **대화형 충돌 ✅ 10-25**(`--ask`): 충돌마다 stderr 프롬프트 `exists: <대상> — overwrite? [y]es / [n]o / [a]ll / [s]kip all / [c]ancel:` +
  stdin 한 줄 · y/n · a/s = 남은 전부 · c = 취소(130 · 이후 항목 손대지 않음) · 빈 줄/EOF/모르는 답 = 건너뜀 · 정책(`--overwrite`)보다 우선 · dry-run은
  건너뜀으로 봄 · 프롬프트는 진행 출력과 다른 핸들(실제 stderr). 시험 `ask_conflict_maps_answers`(T1) · `ask_prompts_on_conflicts_and_honors_answers`(프로세스 ·
  stdin 파이프).
- **남은 것**: 패키지 동봉(MSI/pkg/deb/rpm ·
  포터블 zip · `release.yml` — 채널 결정) · **이동 검증**(협업 제안 10-25: 교차 볼륨 이동 =
  복사 → 검증 → 원본 삭제 순서로 — 엔진에 "원본 정리 전 훅" 필요 · 같은 볼륨 rename은 데이터가 안 움직여 대상 아님).
