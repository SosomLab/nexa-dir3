#!/usr/bin/env python3
"""coverage-files.py — dir2 소스 파일 ↔ 이식 원장 참조 전수(T-91 · docs/port/99 생성물).

`../nexa-dir2/crates/**/*.rs`(examples·build.rs 제외) 각 파일이 원장(docs/port/[0-8]*.md)에서 몇 번 언급되는지 센다.
  - 경로 일치: `crates/<crate>/src/<path>.rs` 또는 `<crate>/src/<path>.rs`(원장은 `nexa-dir2/crates/…` 또는 `…/draw.rs:40` 꼴로 쓴다)
  - 이름 일치: 저장소 안에서 **유일한** 파일명(`win.rs` · `panel.rs` …)은 `win.rs:123`·`` `win.rs` `` 같은 짧은 참조와
    모듈 이름(`menuthread` — src/ 바로 아래는 7자 이상 줄기만 · 하위 모듈은 `archive::zip`/`archive/zip`)도 센다.
    `lib.rs`·`mod.rs`처럼 겹치는 이름은 경로로만 센다.
결과 = docs/port/99-coverage-gaps.md(표: 파일 · 줄 수 · 원장 참조 수 · 판정 ☐ 미참조 / ✅) + 미참조 파일 목록(GAP 후보).
사용: python scripts/coverage-files.py [--check]   (--check = 쓰지 않고 집계만 · 미참조가 늘었으면 1 — 99의 '미참조 N' 수와 비교)
외부 의존 0(표준 라이브러리). dir2 저장소가 없으면 0으로 종료(건너뜀 안내).
"""
import io
import os
import re
import sys
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR2 = os.path.normpath(os.path.join(ROOT, "..", "nexa-dir2"))
PORT = os.path.join(ROOT, "docs", "port")
OUT = os.path.join(PORT, "99-coverage-gaps.md")


def read(p):
    return io.open(p, encoding="utf-8").read()


def dir2_files():
    out = []
    base = os.path.join(DIR2, "crates")
    for dp, dns, fns in os.walk(base):
        dns[:] = [d for d in dns if d not in ("target", "examples")]
        for f in fns:
            if not f.endswith(".rs") or f == "build.rs":
                continue
            full = os.path.join(dp, f)
            rel = os.path.relpath(full, DIR2).replace(os.sep, "/")
            lines = sum(1 for _ in io.open(full, encoding="utf-8", errors="replace"))
            out.append((rel, lines))
    return sorted(out)


def ledger_text():
    files = sorted(f for f in os.listdir(PORT) if re.match(r"[0-8]\d-.*\.md$", f))
    return "\n".join(read(os.path.join(PORT, f)) for f in files)


def count_refs(text, rel, unique_names):
    # rel = crates/nexa-app/src/win.rs → 경로 꼬리 "nexa-app/src/win.rs"
    tail = rel.split("/", 1)[1]
    n = len(re.findall(re.escape(tail), text))
    name = rel.rsplit("/", 1)[1]
    stem = name[:-3]
    parts = rel.split("/")
    if name in unique_names:
        # 짧은 참조: `win.rs:123` · `win.rs` · win.rs:8 — 경로 참조도 포함되므로 큰 쪽만.
        short = len(re.findall(r"(?<![A-Za-z0-9_/.-])" + re.escape(name) + r"(?![A-Za-z0-9_-])", text))
        n = max(n, short)
        # 모듈 이름 참조(`menuthread` · `shellnotify` · `archive::zip`/`archive/zip`): src/ 바로 아래 파일은 7자 이상 줄기만
        # (짧은 일반어 — nav·tip·dw·svg·icon — 오탐 방지) · 하위 모듈은 부모 디렉터리와 함께.
        if parts[-2] != "src":
            parent = parts[-2]
            mod = len(re.findall(re.escape(parent) + r"(?:/|::)" + re.escape(stem) + r"", text))
            n = max(n, mod)
        elif len(stem) >= 7:
            mod = len(re.findall(r"(?<![A-Za-z0-9_])" + re.escape(stem) + r"(?![A-Za-z0-9_])", text))
            n = max(n, mod)
    return n


def main():
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8")
    check = "--check" in sys.argv
    if not os.path.isdir(os.path.join(DIR2, "crates")):
        print(f"nexa-dir2 없음({DIR2}) — 건너뜀")
        return 0
    files = dir2_files()
    names = Counter(r.rsplit("/", 1)[1] for r, _ in files)
    unique = {n for n, c in names.items() if c == 1}
    text = ledger_text()
    rows = []
    for rel, lines in files:
        n = count_refs(text, rel, unique)
        rows.append((rel, lines, n))
    missing = [r for r in rows if r[2] == 0]
    total_lines = sum(l for _, l, _ in rows)
    covered_lines = sum(l for _, l, n in rows if n > 0)
    summary = (
        f"dir2 파일 {len(rows)} · 원장 참조 {len(rows) - len(missing)} · 미참조 {len(missing)} · "
        f"줄 기준 {covered_lines}/{total_lines}({100 * covered_lines // max(total_lines, 1)} %)"
    )
    print(summary)
    if check:
        if not os.path.exists(OUT):
            return 1
        m = re.search(r"미참조 (\d+)", read(OUT))
        return 0 if m and int(m.group(1)) == len(missing) else 1
    out = [
        "# port/99 · dir2 파일 커버리지 · GAP(생성물 — `scripts/coverage-files.py`)",
        "",
        f"> {summary}. 원장 = docs/port/[0-8]*.md. 참조 수 = 경로(`<crate>/src/…rs`) 또는 유일 파일명(`win.rs:123`) 언급 횟수. "
        "표는 손으로 고치지 않는다(재생성). **GAP 등재**는 아래 `## GAP` 절에만 손으로 쓴다(스크립트가 보존).",
        "",
        "## 파일별 참조",
        "",
        "| dir2 파일 | 줄 | 원장 참조 | 판정 |",
        "| --- | --- | --- | --- |",
    ]
    for rel, lines, n in rows:
        out.append(f"| `{rel}` | {lines} | {n} | {'✅' if n else '☐ 미참조'} |")
    out += ["", f"## 미참조 파일({len(missing)}) — 원장에 없는 dir2 코드(GAP 후보)", ""]
    if missing:
        out += ["| dir2 파일 | 줄 | 비고 |", "| --- | --- | --- |"]
        for rel, lines, _ in missing:
            out.append(f"| `{rel}` | {lines} | |")
    else:
        out.append("없음.")
    gap = "\n## GAP\n\n| ID | dir2 기능(파일:줄) | 원장 어디에 보충했나 | 상태 |\n| --- | --- | --- | --- |\n"
    if os.path.exists(OUT):
        old = read(OUT)
        if "\n## GAP" in old:
            gap = old[old.index("\n## GAP"):]
    body = "\n".join(out) + "\n" + gap
    io.open(OUT + ".tmp", "w", encoding="utf-8", newline="\n").write(body)
    os.replace(OUT + ".tmp", OUT)
    print(f"wrote {os.path.relpath(OUT, ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
