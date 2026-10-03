#!/usr/bin/env python3
"""matrix-audit.py — 검증 매트릭스 전수 감사(T-90 · CI-116).

원장(docs/port/[0-8]*.md)의 표 행 머리 ID(`| PREFIX-NNN |`)를 전부 모으고, 매트릭스(docs/port/90)의 각 행 첫 칸이 가리키는 ID
(`PREFIX-NNN` · `NNN~MMM` 범위 · `·`/`/` 나열 · 접두 생략 연속 · `15xx` 와일드카드 · 괄호 꼬리 무시)와 상태(✅ 🚧 ⚠ 🖐 ☐)를 대조해
  1) docs/port/90의 `## 집계` 표를 다시 쓰고(접두별 원장 수 · 매트릭스가 덮는 ID 수 · 상태별)
  2) docs/port/91-matrix-coverage.md(생성물)에 **미착수 ID 전수**(원장 파일:줄 + 제목)를 적는다.
사용: python scripts/matrix-audit.py [--check]   (--check = 파일을 쓰지 않고 집계만 출력 · 90이 최신이 아니면 1)
외부 의존 0(표준 라이브러리).
"""
import io
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PORT = os.path.join(ROOT, "docs", "port")
MATRIX = os.path.join(PORT, "90-verification-matrix.md")
OUT = os.path.join(PORT, "91-matrix-coverage.md")

ID_ROW = re.compile(r"^\| ([A-Z]+)-(\d{2,4})([a-z]?) \|(.*)$")
STATUS_MARKS = ["✅", "🚧", "⚠", "🖐", "☐"]


def read(p):
    return io.open(p, encoding="utf-8").read()


def ledger():
    """{ (prefix, num) : (file, line, title) } — 같은 ID가 여러 문서에 있으면 처음 것."""
    ids = {}
    files = sorted(f for f in os.listdir(PORT) if re.match(r"[0-8]\d-.*\.md$", f))
    for f in files:
        for ln, line in enumerate(read(os.path.join(PORT, f)).splitlines(), 1):
            m = ID_ROW.match(line)
            if not m:
                continue
            key = (m.group(1), int(m.group(2)))
            if key in ids:
                continue
            cells = [c.strip() for c in m.group(4).split("|")]
            title = cells[0] if cells else ""
            ids[key] = (f, ln, title[:80])
    return ids


TOKEN = re.compile(r"([A-Z]+)-(\d+)(?:~(\d+))?|(\d+)(?:~(\d+))?|([A-Z]+)-(\d{1,2})xx")


def parse_cell(cell):
    """첫 칸 → 가리키는 (prefix, num) 집합. 괄호 꼬리는 버린다."""
    cell = re.sub(r"\([^)]*\)", " ", cell)
    out = set()
    prefix = None
    for part in re.split(r"[·/,]|\s+", cell):
        part = part.strip()
        if not part:
            continue
        m = re.fullmatch(r"([A-Z]+)-(\d{1,2})xx", part)
        if m:
            base = int(m.group(2)) * 100
            out.update((m.group(1), n) for n in range(base, base + 100))
            prefix = m.group(1)
            continue
        m = re.fullmatch(r"([A-Z]+)-(\d+)(?:~(\d+)?)?", part)
        if m:
            prefix = m.group(1)
            a = int(m.group(2))
            b = int(m.group(3)) if m.group(3) else a
            out.update((prefix, n) for n in range(a, b + 1))
            continue
        m = re.fullmatch(r"(\d+)(?:~(\d+))?", part)
        if m and prefix:
            a = int(m.group(1))
            b = int(m.group(2)) if m.group(2) else a
            out.update((prefix, n) for n in range(a, b + 1))
    return out


def matrix_rows(text):
    """[(ids, status)] — `## 행` 뒤의 표 행만."""
    rows = []
    seen_head = False
    for line in text.splitlines():
        if line.startswith("## 행"):
            seen_head = True
            continue
        if not seen_head or not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 6 or cells[0] in ("ID", "---"):
            continue
        status = next((s for s in STATUS_MARKS if s in cells[5]), "☐")
        rows.append((parse_cell(cells[0]), status))
    return rows


def main():
    # Windows 콘솔(cp949)에서도 ✅ 같은 기호를 찍는다.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8")
    check = "--check" in sys.argv
    led = ledger()
    text = read(MATRIX)
    rows = matrix_rows(text)
    covered = {}
    rank = {"✅": 4, "⚠": 3, "🖐": 2, "🚧": 1, "☐": 0}
    for ids, st in rows:
        for i in ids:
            # ☐(사유 행)도 "덮음"이다 — 아직 아무 행도 없을 때(None = -1)보다 높다.
            if i in led and rank[st] > rank.get(covered.get(i), -1):
                covered[i] = st
    prefixes = sorted({p for p, _ in led})
    lines = ["| 접두 | 원장 | 덮음 | ✅ | 🚧 | ⚠ | 🖐 | 미착수 |", "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    total = defaultdict(int)
    uncovered = defaultdict(list)
    for p in prefixes:
        ids = [k for k in led if k[0] == p]
        cnt = defaultdict(int)
        for k in ids:
            st = covered.get(k)
            if st:
                cnt[st] += 1
            else:
                uncovered[p].append(k)
        cov = sum(cnt.values())
        lines.append(f"| {p} | {len(ids)} | {cov} | {cnt['✅']} | {cnt['🚧']} | {cnt['⚠']} | {cnt['🖐']} | {len(ids) - cov} |")
        total["all"] += len(ids)
        total["cov"] += cov
        for s in STATUS_MARKS:
            total[s] += cnt[s]
    lines.append(
        f"| **합계** | {total['all']} | {total['cov']} | {total['✅']} | {total['🚧']} | {total['⚠']} | {total['🖐']} | {total['all'] - total['cov']} |"
    )
    table = "\n".join(lines)
    summary = f"원장 {total['all']} · 덮음 {total['cov']}({100 * total['cov'] // max(total['all'], 1)} %) · ✅ {total['✅']} · 미착수 {total['all'] - total['cov']}"
    print(summary)
    if check:
        return 0 if table in text else 1
    # 1) 90의 집계 표 교체(`## 집계` ~ `## 행` 사이).
    head, _, rest = text.partition("## 집계")
    _, _, tail = rest.partition("## 행")
    new = (
        head
        + "## 집계\n\n> `scripts/matrix-audit.py`가 생성(원장 = docs/port/[0-8]*.md 표 행 머리 ID · 덮음 = 매트릭스 행이 가리키는 ID · 미착수 전수 = [91](91-matrix-coverage.md)).\n\n"
        + table
        + "\n\n## 행"
        + tail
    )
    io.open(MATRIX + ".tmp", "w", encoding="utf-8", newline="\n").write(new)
    os.replace(MATRIX + ".tmp", MATRIX)
    # 2) 91 생성.
    out = [
        "# port/91 · 매트릭스 미착수 전수(생성물 — `scripts/matrix-audit.py`)",
        "",
        f"> {summary}. 원장 ID 중 [90](90-verification-matrix.md) 행이 가리키지 않는 것 전부. 착수하면 90에 행을 더하고 이 파일을 다시 생성한다(손으로 고치지 않는다).",
        "",
    ]
    for p in prefixes:
        if not uncovered[p]:
            continue
        out.append(f"## {p} — 미착수 {len(uncovered[p])} / {sum(1 for k in led if k[0] == p)}")
        out.append("")
        out.append("| ID | 원장 | 제목 |")
        out.append("| --- | --- | --- |")
        for k in sorted(uncovered[p], key=lambda x: x[1]):
            f, ln, title = led[k]
            safe = title.replace("|", "\\|")
            out.append(f"| {k[0]}-{k[1]:03d} | `{f}:{ln}` | {safe} |")
        out.append("")
    io.open(OUT + ".tmp", "w", encoding="utf-8", newline="\n").write("\n".join(out))
    os.replace(OUT + ".tmp", OUT)
    print(f"wrote {os.path.relpath(OUT, ROOT)} · updated 90 집계")
    return 0


if __name__ == "__main__":
    sys.exit(main())
