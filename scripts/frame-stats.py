#!/usr/bin/env python3
"""frame-stats.py — `NDIR_TRACE_FRAMES=1`로 받은 stderr의 `[frame]` 줄을 집계한다(T-176 ④ 입력 지연 계측 · docs/25 §5-4 · 외부 패키지 0).

사용:
  NDIR_HOME=C:/tmp/ndir-frames NDIR_TRACE_FRAMES=1 NDIR_STARTUP_CMD="@ready:nav:C:\\Windows\\System32,@after:1500:ui.press:down,…,@after:6000:quit" \\
      target/release/nexa-dir.exe 2> C:/tmp/frames.log
  python scripts/frame-stats.py C:/tmp/frames.log [--warm 2] [--json]

줄 형식: `[frame] n=<k> t=<ms> wait=<us> paint=<us> present=<us> size=<w>x<h> backend=<name>`  (t = 기동 뒤 경과 ms)
  wait    = 다시 그리기 요청(입력 처리 끝) → 그리기 시작(이벤트 루프 대기 · OS vsync 조율)  ≈ 입력 → 화면 지연의 앞부분
  paint   = CPU 래스터(paint_into)                                                  예산 ≤ 8 ms(docs/25 §3)
  present = 표면 → 창(softbuffer/iosurface)
  total   = wait + paint + present                                                   예산 ≤ 16 ms(입력 한 번 → 화면)
출력: 프레임 수 · 각 항목의 중앙값 / p95 / 최대(µs → ms · 소수 1) · 예산 넘은 프레임 수. 앞 `--warm`개(기본 2 · 첫 프레임 글리프 적재)는 버린다.
0 프레임 = 측정 실패(환경 변수 · stderr 리다이렉션 확인).
"""
import json
import re
import sys

LINE = re.compile(r"\[frame\] n=(\d+) t=(\d+)ms wait=(\d+)us paint=(\d+)us present=(\d+)us size=(\d+)x(\d+) backend=(\S+)")
BUDGET_US = {"paint": 8000, "total": 16000}
# Windows 파이썬의 stdout 기본(cp949)으로 한글 · '—'가 깨지지 않게.
try:
    sys.stdout.reconfigure(encoding="utf-8")
except (AttributeError, ValueError):
    pass


def pct(xs, p):
    if not xs:
        return 0
    s = sorted(xs)
    k = min(len(s) - 1, int(round((len(s) - 1) * p)))
    return s[k]


def main() -> int:
    args = sys.argv[1:]
    warm, as_json, path = 2, False, None
    while args:
        a = args.pop(0)
        if a == "--warm":
            warm = int(args.pop(0))
        elif a == "--json":
            as_json = True
        else:
            path = a
    if not path:
        print(__doc__, file=sys.stderr)
        return 2
    frames = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = LINE.search(line)
            if m:
                n, t, w, p, pr, sw, sh, be = m.groups()
                frames.append({"n": int(n), "t": int(t), "wait": int(w), "paint": int(p), "present": int(pr), "size": f"{sw}x{sh}", "backend": be})
    if not frames:
        print("frames=0  (측정 실패 — NDIR_TRACE_FRAMES=1 · stderr 리다이렉션 확인)")
        return 1
    used = frames[warm:] if len(frames) > warm else frames
    for fr in used:
        fr["total"] = fr["wait"] + fr["paint"] + fr["present"]
    span_ms = max(1, frames[-1]["t"] - frames[0]["t"])
    out = {
        "frames": len(frames), "used": len(used), "backend": used[-1]["backend"], "size": used[-1]["size"],
        "span_ms": span_ms, "fps_avg": round(len(frames) * 1000 / span_ms, 1),
        # 1초 구간별 프레임 수(어느 때 많이 그렸는지 — 목록 적재 · 아이콘 도착 · 깜빡임 구분).
        "per_sec": [sum(1 for fr in frames if s * 1000 <= fr["t"] - frames[0]["t"] < (s + 1) * 1000) for s in range(span_ms // 1000 + 1)],
    }
    for k in ("wait", "paint", "present", "total"):
        xs = [fr[k] for fr in used]
        out[k] = {"med_ms": round(pct(xs, 0.5) / 1000, 2), "p95_ms": round(pct(xs, 0.95) / 1000, 2), "max_ms": round(max(xs) / 1000, 2)}
        if k in BUDGET_US:
            out[k]["over_budget"] = sum(1 for x in xs if x > BUDGET_US[k])
    if as_json:
        print(json.dumps(out, ensure_ascii=False, indent=1))
        return 0
    print(f"frames={out['frames']} (집계 {out['used']} · 예열 {len(frames) - len(used)} 제외) · {out['size']} · {out['backend']} · "
          f"{span_ms} ms 동안 평균 {out['fps_avg']} fps · 초별 {out['per_sec']}")
    print("| 항목 | 중앙값 | p95 | 최대 | 예산 초과 |")
    print("| --- | --- | --- | --- | --- |")
    for k, label in (("wait", "요청 → 그리기 시작"), ("paint", "그리기(CPU 래스터)"), ("present", "present"), ("total", "합(입력 → 화면)")):
        v = out[k]
        ob = f"{v['over_budget']} / {BUDGET_US[k] // 1000} ms" if "over_budget" in v else "—"
        print(f"| {label} | {v['med_ms']} ms | {v['p95_ms']} ms | {v['max_ms']} ms | {ob} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())
