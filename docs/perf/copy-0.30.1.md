# 복사 성능 기록 — v0.30.1

> T-203 ⑥ · 규약 = [docs/24 §4-1](../24-fast-copy.md) · 판정 없음(D5 · CI = 기록만).
> **출처**: v0.30.1 선검증 run **41829718499**(3a6ef10 = 코드 62c25d3 · `release.yml` 선검증의 `perf-copy` 잡 · runs 3 · 세트 3종) 아티팩트 `perf-copy-Windows/Linux/macOS`. 태그 release run은 이 잡을 건너뛴다(설계).
> 실기기 수치 = journal: Windows 5회 중앙값 [11-01 §18](../journal/2026-11-01.md) · Linux 5회 = [docs/24 §4](../24-fast-copy.md)(11-04 · VMware 가상 디스크 · 참고용).

## GitHub 러너(3회 중앙값 · 대상 fsync 포함)

### Copy bench — windows · cores 4 · seek Costly · runs 3 · release · 3a6ef10

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 4333 | 4177 | 4430 | 14.4 | 1.0x | 1 |
| small | os1 | 4745 | 4682 | 5103 | 13.2 | 0.91x | 1 |
| small | loop4 | 4224 | 4168 | 4346 | 14.8 | 1.03x | 4 |
| small | auto | 4253 | 4190 | 4413 | 14.7 | 1.02x | 4 |
| big | plain | 3912 | 3752 | 5484 | 130.9 | 1.0x | 1 |
| big | os | 3157 | 3131 | 3158 | 162.2 | 1.24x | 1 |
| big | os_unbuffered | 3268 | 3225 | 3470 | 156.7 | 1.2x | 1 |
| mixed | plain | 9400 | 7130 | 13640 | 97.4 | 1.0x | 1 |
| mixed | os1 | 7236 | 7046 | 8577 | 126.5 | 1.3x | 1 |
| mixed | loop4 | 6615 | 6517 | 6626 | 138.4 | 1.42x | 4 |
| mixed | auto | 6596 | 6511 | 6661 | 138.8 | 1.43x | 4 |

### Copy bench — linux · cores 4 · seek Free · runs 3 · release · 3a6ef10

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 1499 | 1443 | 1925 | 41.7 | 1.0x | 1 |
| small | os1 | 1467 | 1454 | 1558 | 42.6 | 1.02x | 1 |
| small | loop4 | 1409 | 1402 | 1465 | 44.4 | 1.06x | 4 |
| small | auto | 1431 | 1409 | 1461 | 43.7 | 1.05x | 4 |
| big | plain | 2640 | 2624 | 2808 | 193.9 | 1.0x | 1 |
| big | os | 2589 | 2589 | 2789 | 197.8 | 1.02x | 1 |
| big | os_unbuffered | 2592 | 2588 | 2605 | 197.5 | 1.02x | 1 |
| mixed | plain | 5346 | 5310 | 6714 | 171.3 | 1.0x | 1 |
| mixed | os1 | 5228 | 5219 | 7458 | 175.1 | 1.02x | 1 |
| mixed | loop4 | 5097 | 5073 | 5154 | 179.6 | 1.05x | 4 |
| mixed | auto | 5019 | 5007 | 5031 | 182.4 | 1.07x | 4 |

### Copy bench — macos · cores 3 · seek Free · runs 3 · release · 3a6ef10

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 3122 | 1869 | 4046 | 20.0 | 1.0x | 1 |
| small | os1 | 633 | 503 | 800 | 98.7 | 4.93x | 1 |
| small | loop4 | 2968 | 1807 | 3563 | 21.1 | 1.05x | 4 |
| small | auto | 369 | 334 | 478 | 169.4 | 8.46x | 3 |
| big | plain | 524 | 368 | 1036 | 977.1 | 1.0x | 1 |
| big | os | 8 | 2 | 13 | 64000.0 | 65.5x | 1 |
| big | os_unbuffered | 1 | 0 | 6 | 512000.0 | 524.0x | 1 |
| mixed | plain | 4461 | 3597 | 10047 | 205.3 | 1.0x | 1 |
| mixed | os1 | 292 | 269 | 327 | 3135.7 | 15.28x | 1 |
| mixed | loop4 | 2980 | 2787 | 3503 | 307.3 | 1.5x | 4 |
| mixed | auto | 210 | 190 | 260 | 4360.1 | 21.24x | 3 |

## 읽는 법

- 러너는 매번 다른 공유 VM이라 같은 실행 안의 비율만 본다.
- **macOS big = clone(복사 아님)** — `os` 8 ms · 65.5× · 같은 볼륨 APFS `clonefile`이라 배수의 의미가 작다. small `auto` 8.46× · mixed `auto` 21.24×도 같은 볼륨 조건이다.
- **Windows 러너(seek Costly)**: small `auto` 1.02× · big `os` 1.24× · mixed `auto` 1.43×.
- **Linux 러너(seek Free)**: 전략 차이 작음(small `auto` 1.05× · big `os` 1.02× · mixed `auto` 1.07×).
