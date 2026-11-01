# 복사 성능 기록 — v0.28.1

> T-203 ⑥ · 규약 = [docs/24 §4-1](../24-fast-copy.md) · 판정 없음(D5 · CI = 기록만 · 하한은 분포가 쌓인 뒤 다시 정함).
> **출처**: v0.28.1 선검증 run **41320202068**(68d926b · `release.yml` 선검증의 `perf-copy` 잡 · runs 3 · 세트 3종) 아티팩트 `perf-copy-Windows/Linux/macOS` — 선검증에서 `perf-copy` 잡이 돈 첫 사례(0.28.0은 수동 run 대체 · [copy-0.28.0.md](copy-0.28.0.md)).
> 실기기 수치 = journal: Windows 5회 중앙값 [11-01 §18](../journal/2026-11-01.md)(C: SSD · D: HDD) · Linux 5회 = T-203 ⑤(예정).

## GitHub 러너(3회 중앙값 · 대상 fsync 포함)

### Copy bench — linux · cores 4 · seek Costly · runs 3 · release · 68d926b

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 1157 | 1145 | 1519 | 54.0 | 1.0x | 1 |
| small | os1 | 1169 | 1152 | 1191 | 53.5 | 0.99x | 1 |
| small | loop4 | 1079 | 1068 | 1103 | 57.9 | 1.07x | 4 |
| small | auto | 1076 | 1073 | 1137 | 58.1 | 1.08x | 4 |
| big | plain | 1333 | 1271 | 1361 | 384.1 | 1.0x | 1 |
| big | os | 1274 | 1271 | 1275 | 401.9 | 1.05x | 1 |
| big | os_unbuffered | 1273 | 1273 | 1524 | 402.2 | 1.05x | 1 |
| mixed | plain | 2441 | 2419 | 3397 | 375.1 | 1.0x | 1 |
| mixed | os1 | 2419 | 2416 | 2430 | 378.5 | 1.01x | 1 |
| mixed | loop4 | 2253 | 2248 | 3569 | 406.4 | 1.08x | 4 |
| mixed | auto | 2274 | 2263 | 2276 | 402.6 | 1.07x | 4 |

### Copy bench — windows · cores 4 · seek Costly · runs 3 · release · 68d926b

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 4020 | 3951 | 4207 | 15.5 | 1.0x | 1 |
| small | os1 | 4622 | 4581 | 4686 | 13.5 | 0.87x | 1 |
| small | loop4 | 4247 | 4214 | 4274 | 14.7 | 0.95x | 4 |
| small | auto | 4079 | 4065 | 4253 | 15.3 | 0.99x | 4 |
| big | plain | 4095 | 4044 | 5394 | 125.0 | 1.0x | 1 |
| big | os | 3165 | 3071 | 3177 | 161.8 | 1.29x | 1 |
| big | os_unbuffered | 3262 | 3232 | 3479 | 157.0 | 1.26x | 1 |
| mixed | plain | 10055 | 7033 | 14746 | 91.1 | 1.0x | 1 |
| mixed | os1 | 7684 | 6901 | 16741 | 119.2 | 1.31x | 1 |
| mixed | loop4 | 9655 | 6654 | 10274 | 94.8 | 1.04x | 4 |
| mixed | auto | 6534 | 6471 | 6600 | 140.1 | 1.54x | 4 |

### Copy bench — macos · cores 5 · seek Free · runs 3 · release · 68d926b

| set | case | median ms | min | max | MB/s | vs plain | threads |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | plain | 2249 | 1867 | 3088 | 27.8 | 1.0x | 1 |
| small | os1 | 410 | 383 | 411 | 152.4 | 5.49x | 1 |
| small | loop4 | 1696 | 1628 | 1726 | 36.9 | 1.33x | 4 |
| small | auto | 311 | 287 | 372 | 201.0 | 7.23x | 5 |
| big | plain | 201 | 161 | 447 | 2547.3 | 1.0x | 1 |
| big | os | 1 | 1 | 5 | 512000.0 | 201.0x | 1 |
| big | os_unbuffered | 0 | 0 | 0 | — | — | 1 |
| mixed | plain | 1188 | 1057 | 1347 | 770.7 | 1.0x | 1 |
| mixed | os1 | 142 | 133 | 242 | 6448.1 | 8.37x | 1 |
| mixed | loop4 | 1714 | 1043 | 1875 | 534.2 | 0.69x | 4 |
| mixed | auto | 145 | 109 | 146 | 6314.7 | 8.19x | 5 |

## 관찰

- **macOS big = clone(복사 아님) · 배수 의미 없음** — APFS `clonefile`로 `os` 1 ms(표의 512000 MB/s · 201× 포함) · `os_unbuffered` 0 ms(같은 볼륨 512 MiB). 0 ms 행은 아티팩트 표에 `None` · `Nonex`로 찍혀 이 문서에서는 "—"로 옮김(스크립트 표시 처방 = 04a318d `scripts/bench-copy.sh` · 0 ms면 MB/s · 배수 = "—" · JSON은 null 그대로 · 태그 뒤 커밋이라 0.28.1 공개본과 무관). macOS mixed도 `os1` 8.37× · `auto` 8.19×로 clone 효과가 섞여 있다.
- **macOS 그 밖**: small `auto` 7.23×(5스레드) · `os1` 5.49× — 0.28.0 수동 run(small `auto` 7.16×)과 같은 경향. mixed `loop4` 0.69× = 루프 4스레드 손해.
- **Windows 러너(seek Costly)**: small은 전략 무관(`auto` 0.99× · `os1` 0.87×) · big `os` 1.29× · `os_unbuffered` 1.26× · mixed `auto` 1.54× · `os1` 1.31×. mixed plain은 min 7,033 / 중앙값 10,055 / max 14,746으로 편차가 큼(공유 VM). 0.28.0 수동 run(small 1회 · `auto` 0.8×)과는 회수가 달라(1회 대 3회) 비교는 참고만.
- **Linux 러너**: 이번 run은 seek **Costly**로 판정(0.28.0 수동 run = Free) → 같은 러너 이미지라도 판정이 흔들림. 전략 차이 작음(전부 0.99~1.08×).
