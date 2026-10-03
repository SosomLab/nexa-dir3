#!/usr/bin/env python3
"""qa-checklist.py — dir2 대조 실기 QA 표 생성(T-91 · docs/port/92 생성물).

검증 매트릭스(docs/port/90)에서 **실기(사람 손) 확인이 남은 행**을 모은다 — 상태 🖐, 또는 비고/시험 칸에 "실기"가 있는 행.
각 행을 QA 항목으로 바꿔 docs/port/92-qa-checklist.md에 쓴다: ID · 기능 · 어디서(구현) · 확인 방법(비고에서) · OS 칸(win/mac/linux ☐) ·
결과 칸. 결과 칸은 손으로 채우되 **파일은 재생성되므로** 결과는 `## 결과 기록` 절(보존)에 날짜별로 적는다.
사용: python scripts/qa-checklist.py [--check]   (--check = 쓰지 않고 항목 수만 · 92의 '항목 N'과 다르면 1)
외부 의존 0.
"""
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PORT = os.path.join(ROOT, "docs", "port")
MATRIX = os.path.join(PORT, "90-verification-matrix.md")
OUT = os.path.join(PORT, "92-qa-checklist.md")


def read(p):
    return io.open(p, encoding="utf-8").read()


def rows():
    out = []
    seen = False
    for line in read(MATRIX).splitlines():
        if line.startswith("## 행"):
            seen = True
            continue
        if not seen or not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 7 or cells[0] in ("ID", "---"):
            continue
        ids, feat, impl_, layer, test, status, note = cells[:7]
        if "🖐" in status or "실기" in note or "실기" in test or "실기" in layer:
            out.append((ids, feat, impl_, layer, test, status, note))
    return out


# OS 단서 — 영문자 경계로만 맞춘다(한글이 붙어도 됨). 부분 문자열 "win"은 winit · WINA/WINB(원장 접두) · window를,
# "mac"은 machine · macro를, "셸"은 3-OS 공통 셸 기능(SHELL §4)을 잘못 잡았다(10-03 검토).
_OS_WORDS = {
    "win": ("windows", "win32", "winshell", "winwatch", "winpty", "wintemplates", "conpty", "shcne", "icontextmenu",
            "readdirectorychangesw", "shgetfileinfo", "cf_hdrop", "ole", "uia", "msi", "pwsh", "powershell"),
    "mac": ("macos", "mac", "macwatch", "nspasteboard", "nsdragging", "fsevents", "kqueue", "trashitem", "objc2",
            "dmg", "pkg"),
    "linux": ("linux", "linuxwatch", "x11", "x11rb", "wayland", "inotify", "xdnd", "xdg", "xdg-open", "freedesktop",
              "deb", "rpm"),
}
_OS_RE = {k: re.compile(r"(?<![a-z0-9_])(?:" + "|".join(re.escape(w) for w in ws) + r")(?![a-z0-9_])")
          for k, ws in _OS_WORDS.items()}


def os_hint(text):
    t = text.lower()
    tags = [k for k in ("win", "mac", "linux") if _OS_RE[k].search(t)]
    return "·".join(tags) or "3-OS"


def main():
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8")
    check = "--check" in sys.argv
    items = rows()
    print(f"실기 QA 항목 {len(items)}")
    if check:
        if not os.path.exists(OUT):
            return 1
        m = re.search(r"항목 (\d+)", read(OUT))
        return 0 if m and int(m.group(1)) == len(items) else 1
    out = [
        "# port/92 · dir2 대조 실기 QA 표(생성물 — `scripts/qa-checklist.py`)",
        "",
        f"> 항목 {len(items)}. 원천 = [90](90-verification-matrix.md)에서 🖐 또는 \"실기\"가 적힌 행. 표는 재생성되므로 손으로 고치지 않는다 —",
        "> 실기 결과는 아래 `## 결과 기록` 절(보존)에 날짜 · OS · 항목 ID · 결과(✓/✗ + 증상)로 적고, 실기가 끝나 자동 시험으로 바뀐 행은 90에서 \"실기\"를 지운다.",
        "> 실기 절차 공통: `NDIR_HOME` 격리 폴더 + dir2(`../nexa-dir2` `0.22.0`)를 나란히 띄워 **같은 폴더 · 같은 조작**으로 비교(docs/18 §4). 입력 주입·포커스 탈취 금지.",
        "",
        "| # | ID | 기능 | 구현 | 자동 시험(있으면) | 확인 방법 / 왜 실기인가 | OS |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for i, (ids, feat, impl_, layer, test, status, note) in enumerate(items, 1):
        out.append(f"| {i} | {ids} | {feat} | {impl_} | {test} | {note} | {os_hint(' '.join((ids, feat, impl_, note)))} |")
    out += ["", "## 실기 순서(권장)", "",
            "1. 셸·OS 자원(컨텍스트 메뉴 · 휴지통 · 클립보드 · DnD · 감시) — OS마다 1회.",
            "2. 렌더(글꼴 크기 · 전각 · 테마 · 고DPI) — dir2 스크린샷과 나란히.",
            "3. 라이선스 발급→설치→제거 E2E · 플러그인 설치/삭제.",
            "4. 패키지 설치/제거(MSI · zip · pkg/dmg · deb/rpm) — release.yml 산출물.", ""]
    keep = "\n## 결과 기록\n\n| 날짜 | OS | # / ID | 결과 | 비고 |\n| --- | --- | --- | --- | --- |\n"
    if os.path.exists(OUT):
        old = read(OUT)
        if "\n## 결과 기록" in old:
            keep = old[old.index("\n## 결과 기록"):]
    io.open(OUT + ".tmp", "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n" + keep)
    os.replace(OUT + ".tmp", OUT)
    print(f"wrote {os.path.relpath(OUT, ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
