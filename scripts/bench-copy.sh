#!/usr/bin/env bash
# bench-copy.sh — 복사 성능 측정(T-203 · docs/24 §4-1 · 사용자 결정 D1~D6 11-01 §15): ndir-ops `bench_copy`(#[ignore] · release)를 돌려
#   `copy set=… case=… median_ms=…` 키=값 줄을 **JSON + 마크다운 표**로 만든다. perf-baseline.sh --copy · perf-copy.yml(수동) ·
#   release.yml 선검증이 전부 이 스크립트를 쓴다(스크립트가 원장 · 로컬과 CI가 같은 명령).
#
# 사용: scripts/bench-copy.sh [--runs N] [--sets small,big,mixed] [--dir <폴더>] [--out <json>] [--md <file>] [--debug]
#   --runs N  = 세트·전략마다 N회 → 중앙값(기본 1 · 실기기 5 · CI 3)
#   --sets    = 세트 목록(기본 전부 · small = 16 KiB × 4,000 · big = 512 MiB × 1 · mixed = 1~8 MiB × 200 + 16 KiB × 1,000)
#   --dir     = 자료를 만들 폴더 = **측정할 볼륨**(기본 시스템 임시 폴더 · 벤치가 하위 폴더를 만들고 끝나면 지운다)
#   --out     = JSON 경로(기본 target/perf/copy-last.json · 원문 줄은 그 옆 copy-last.txt)
#   --md      = 마크다운 표를 이 파일에도(표준 출력에는 늘 찍는다 · GITHUB_STEP_SUMMARY가 있으면 거기에도 덧붙인다)
# 전략 라벨(OS 중립): 트리 = plain(일반 = 루프 1스레드) · os1(OS 경로 1스레드) · loop4(루프 4스레드) · auto(향상 = 기본값) / 큰 파일 =
#   plain · os · os_unbuffered. 회차마다 대상 fsync 포함 · 전략을 회차마다 번갈아 돈다.
# 판정은 하지 않는다(D5): CI = 기록만(공유 VM · 같은 실행 안의 비율 `vs plain`만 의미) · 실기기 = perf-baseline.sh가 기준선과 비교.
# 종료 코드 = cargo test 결과(측정 실패 = 0이 아님 · 원문은 copy-last.txt).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PYTHONIOENCODING=utf-8
RUNS=1; SETS=""; DIR=""; OUT=""; MD=""; PROFILE=release
while [ $# -gt 0 ]; do
    case "$1" in
        --runs) RUNS="$2"; shift ;;
        --sets) SETS="$2"; shift ;;
        --dir) DIR="$2"; shift ;;
        --out) OUT="$2"; shift ;;
        --md) MD="$2"; shift ;;
        --debug) PROFILE=debug ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
    shift
done
PERF_DIR="$ROOT/target/perf"; mkdir -p "$PERF_DIR"
[ -n "$OUT" ] || OUT="$PERF_DIR/copy-last.json"
TXT="${OUT%.json}.txt"
# 파이썬 = `python3` 우선(순정 macOS · CI macos 러너에는 `python`이 없다) · `PYTHON` 환경 변수로 지정 가능.
PY="${PYTHON:-$(command -v python3 || command -v python)}"
[ -n "$PY" ] || { echo "python3(또는 python)이 필요하다" >&2; exit 2; }
HEAD_SHA="$(cd "$ROOT" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"

REL=(); [ "$PROFILE" = release ] && REL=(--release)
[ -n "$DIR" ] && export NDIR_BENCH_DIR="$DIR"
(cd "$ROOT" && NDIR_BENCH_RUNS="$RUNS" NDIR_BENCH_SETS="$SETS" \
    cargo test -q -p ndir-ops "${REL[@]}" bench_copy -- --ignored --nocapture >"$TXT" 2>&1)
RC=$?
if [ "$RC" -ne 0 ]; then
    echo "bench_copy 실패(exit $RC) — 원문 $TXT" >&2
    tail -n 20 "$TXT" >&2
fi

"$PY" - "$TXT" "$OUT" "$MD" "$HEAD_SHA" "$RUNS" "$PROFILE" <<'EOF'
import json, os, sys, time
txt, out, md, head, runs, profile = sys.argv[1:7]
env, rows = {}, []
with open(txt, encoding="utf-8", errors="replace") as f:
    for line in f:
        line = line.strip()
        if line.startswith("copy_env "):
            env = dict(p.split("=", 1) for p in line.split()[1:] if "=" in p)
        elif line.startswith("copy set="):
            kv = dict(p.split("=", 1) for p in line.split()[1:] if "=" in p)
            for k in ("median_ms", "min_ms", "max_ms", "files", "bytes", "threads", "runs"):
                kv[k] = int(kv[k])
            kv["native"] = kv.get("native") == "true"
            rows.append(kv)
# MB/s = bytes / median · vs plain = 같은 세트 plain 중앙값 / 이 전략 중앙값(배 · 1.0 이상 = 빠름)
plain = {r["set"]: r["median_ms"] for r in rows if r["case"] == "plain"}
for r in rows:
    r["mb_s"] = round(r["bytes"] / 1048576 / (r["median_ms"] / 1000), 1) if r["median_ms"] else None
    p = plain.get(r["set"])
    r["vs_plain"] = round(p / r["median_ms"], 2) if p and r["median_ms"] else None
doc = {"schema": 1, "head": head, "profile": profile, "when": time.strftime("%Y-%m-%dT%H:%M:%S"),
       "os": env.get("os"), "cores": env.get("cores"), "seek": env.get("seek"), "runs": int(runs), "cases": rows}
with open(out, "w", encoding="utf-8") as f:
    json.dump(doc, f, ensure_ascii=False, indent=1)
L = [f"### Copy bench — {doc['os']} · cores {doc['cores']} · seek {doc['seek']} · runs {runs} · {profile} · {head}", "",
     "| set | case | median ms | min | max | MB/s | vs plain | threads |",
     "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
for r in rows:
    L.append(f"| {r['set']} | {r['case']} | {r['median_ms']} | {r['min_ms']} | {r['max_ms']} | {r['mb_s']} | {r['vs_plain']}x | {r['threads']} |")
if not rows:
    L.append(f"| (no rows — see {os.path.basename(txt)}) | | | | | | | |")
text = "\n".join(L) + "\n"
print(text, end="")
if md:
    with open(md, "w", encoding="utf-8") as f:
        f.write(text)
gs = os.environ.get("GITHUB_STEP_SUMMARY")
if gs:
    with open(gs, "a", encoding="utf-8") as f:
        f.write(text + "\n")
EOF
echo "json: $OUT"
exit "$RC"
