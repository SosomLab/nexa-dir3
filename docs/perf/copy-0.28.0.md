# 복사 성능 기록 — v0.28.0

> T-203 ⑥ · 규약 = [docs/24 §4-1](../24-fast-copy.md) · 판정 없음(D5 · CI = 기록만 · 하한은 분포가 쌓인 뒤 다시 정함).
> **대체 사유**: v0.28.0 선검증 run(41278141759 · 8e548c2)은 `perf-copy` 잡이 생기기 전이라 측정이 없음 → 태그 직후 main 67fb104(0.28.0 코드 + T-203 하네스)의 수동 run **41291607844**(`perf-copy.yml` · runs 1 · sets small) 아티팩트로 대신한다. 다음 버전부터는 선검증 run의 `perf-copy` 잡 아티팩트를 쓴다.
> 실기기 수치 = journal: Windows 5회 중앙값 [11-01 §18](../journal/2026-11-01.md)(C: SSD · D: HDD) · Linux 5회 = T-203 ⑤(예정).

## GitHub 러너(1회 · small 16 KiB × 4,000 · 대상 fsync 포함)

### Copy bench — linux · cores 4 · seek Free · runs 1 · release · 67fb104

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 1241 | 1241 | 1241 | 50.4 | 1.0x | 1 |
| small | os1 | 1107 | 1107 | 1107 | 56.5 | 1.12x | 1 |
| small | loop4 | 3064 | 3064 | 3064 | 20.4 | 0.41x | 4 |
| small | auto | 1051 | 1051 | 1051 | 59.5 | 1.18x | 4 |

### Copy bench — windows · cores 4 · seek Costly · runs 1 · release · 67fb104

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 4520 | 4520 | 4520 | 13.8 | 1.0x | 1 |
| small | os1 | 4558 | 4558 | 4558 | 13.7 | 0.99x | 1 |
| small | loop4 | 3805 | 3805 | 3805 | 16.4 | 1.19x | 4 |
| small | auto | 5665 | 5665 | 5665 | 11.0 | 0.8x | 4 |

### Copy bench — macos · cores 3 · seek Free · runs 1 · release · 67fb104

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 2400 | 2400 | 2400 | 26.0 | 1.0x | 1 |
| small | os1 | 463 | 463 | 463 | 135.0 | 5.18x | 1 |
| small | loop4 | 2196 | 2196 | 2196 | 28.5 | 1.09x | 4 |
| small | auto | 335 | 335 | 335 | 186.6 | 7.16x | 3 |

## 관찰

- **windows-latest 러너 = seek Costly 판정** → `auto` 4스레드가 plain보다 느림(0.8× · 1회라 편차 포함). 공유 VM 디스크 특성이며 실기기 C: SSD 5회 중앙값(`auto` 3.0×)과 다르다. 하한 설정은 분포 누적 뒤(D5).
- **Linux 러너**: `loop4` 0.41× = 4코어 러너에서 루프 4스레드는 손해 · `auto`(OS 경로) 1.18×는 이득.
- **macOS 러너**: OS 경로(clonefile/copyfile)가 압도적 — `os1` 5.18× · `auto` 7.16×(3스레드).
