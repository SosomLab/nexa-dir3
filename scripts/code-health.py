#!/usr/bin/env python3
"""코드 건강 점검(T-176 ③ · nexa-sql scripts/code-health.py · docs/93 차용 → dir3 docs/25 §6) — 컴파일러가 못 보는 미사용·중복·크기를
같은 기준으로 다시 잰다.

사용:
  python scripts/code-health.py                 # nexa-dir3 + ../nexa-ui + ../nexa-license 전부
  python scripts/code-health.py --cov           # + cargo llvm-cov 요약(느림 · 수 분)
  python scripts/code-health.py --baseline target/code-health/baseline.json   # 기준선과 수치 비교
  python scripts/code-health.py --save-baseline # 이번 결과를 기준선으로 저장

산출: target/code-health/report.md · result.json (저장소 `target/` 아래 = 추적 안 됨).
외부 패키지 0(표준 라이브러리만) · Windows/macOS/Linux 동일.

판정은 **후보**다 — 문자열로 조립하는 키(`format!("key.{id}")` · `window.{name}_pos`), 매크로·FFI·직렬화로만 닿는 항목은
사람이 한 번 본다. 확인한 예외는 아래 ALLOW_* 표에 이유와 함께 적는다(다음 실행에서 조용해진다).

dir3와 nexa-sql의 차이(C · D 검사):
  - 번역 = `.lang` 자원(`crates/ndir-i18n/lang/en.lang` 키 = `tr("키")`) — nexa-sql의 `Msg` enum 대신 **키 문자열** 참조로 판정.
  - 설정 = `ndir-settings::REGISTRY`의 `e!("키", …)` 매크로 — `key:` 필드 대신 매크로 첫 인자로 뽑는다.
  - nexa-ui · nexa-license는 nexa-sql도 쓰는 공용 라이브러리 → `../nexa-sql`이 있으면 **참조 말뭉치**로만 더한다(보고 대상 아님):
    dir3가 안 쓰는 nexa-ui API라도 nexa-sql이 쓰면 미사용이 아니다.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

# Windows 파이썬의 stdout 기본(cp949)으로 한글이 깨지지 않게.
try:
    sys.stdout.reconfigure(encoding="utf-8")
except (AttributeError, ValueError):
    pass

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
REPOS = {
    "nexa-dir3": ROOT,
    "nexa-ui": ROOT.parent / "nexa-ui",
    "nexa-license": ROOT.parent / "nexa-license",
}
# 참조 말뭉치(보고하지 않음 · 공용 라이브러리의 소비자).
REF_REPOS = {
    "nexa-sql": ROOT.parent / "nexa-sql",
}
OUT = ROOT / "target" / "code-health"

# ── 확인한 예외(이유 필수) ─────────────────────────────────────────────
# 참조 0이어도 남기는 `pub` 항목: "크레이트/이름": 이유
ALLOW_PUB: dict[str, str] = {
    "nexa-license/LEASE_DOMAIN": "라이선스 프로토콜 상수(서버 요청·리스 — 서버 단계용 · nexa-sql docs/25)",
    "nexa-license/SERVER_REQUEST_PREFIX": "라이선스 프로토콜 상수(서버 요청·리스 — 서버 단계용 · nexa-sql docs/25)",
}
# 코드에서 문자열로 안 보이지만 쓰이는 설정 키 접두(조립 키 · 설정 창 전용 등)
ALLOW_SETTING_PREFIX: dict[str, str] = {
    "key.": "단축키 = 명령 id로 조립(`keymap.rs` `key.<id>` · 프리셋 기본)",
    "window.": "보조 창 위치·크기 = 창 이름으로 조립(`window.{name}_pos` · `_size`)",
    "cloud.": "CLOUD 이식 대기(T-72 · Q-6 · 10-05 §9) — dir2 키·자원을 그대로 보존(설정 창 빈 페이지)",
}
# 설정 크레이트 안의 도움 함수가 읽는 키(앱은 그 함수를 부른다): 키: 이유
ALLOW_SETTING_INTERNAL: dict[str, str] = {}
# 코드에 문자열로 안 보이지만 쓰이는 번역 키 접두(조립 · 설정 창이 레지스트리 라벨로 찾는 것 등)
ALLOW_I18N_PREFIX: dict[str, str] = {
    "pref.cat.": "설정 창 카테고리 라벨 = 레지스트리 `cat` 상수로 찾는다",
    "pref.grp.": "설정 창 묶음 라벨 = 레지스트리 PAGES 표",
    "cloud.": "CLOUD 이식 대기(T-72 · Q-6 · 10-05 §9) — dir2 `.lang` 자원을 그대로 보존",
    "menu.cloud": "CLOUD 이식 대기(위와 같음)",
}
# 의존 이름이 소스에 안 보여도 필요한 것(feature 활성·링크 전용 등): "크레이트/의존": 이유
ALLOW_DEP: dict[str, str] = {}

ITEM_RE = re.compile(
    r"^\s*pub\s+(?:const\s+fn|async\s+fn|unsafe\s+fn|fn|struct|enum|trait|const|static|type|mod)\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def rs_files(repo: Path) -> list[Path]:
    out = []
    for p in repo.rglob("*.rs"):
        parts = set(p.relative_to(repo).parts)  # 저장소 기준(워크트리가 `_cmp/` 아래여도 잰다)
        if "target" in parts or ".git" in parts or "_cmp" in parts:
            continue
        out.append(p)
    return sorted(out)


def read(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return p.read_text(encoding="utf-8", errors="replace")


def crate_of(repo: Path, p: Path) -> str:
    rel = p.relative_to(repo).parts
    # 크레이트 = `src`(또는 tests·examples·benches) 바로 위 폴더 — crates/<c>/src · extensions/sdk/<c>/src 모두.
    for k in range(len(rel) - 1, 0, -1):
        if rel[k] in ("src", "tests", "examples", "benches"):
            return rel[k - 1]
    return rel[0]


def strip_line_comment(line: str) -> str:
    i = line.find("//")
    return line if i < 0 else line[:i]


# ── A. 허용 표시 ───────────────────────────────────────────────────────
def check_allow(files: dict[Path, str]) -> list[dict]:
    # rustc 린트만(`clippy::unused_self` 같은 clippy 린트는 코드 스타일 판단이라 대상 아님).
    rx = re.compile(r"#\[(allow|expect)\(([^)]*(?<!::)\b(dead_code|unused[a-z_]*)\b[^)]*)\)\]")
    out = []
    for p, text in files.items():
        for i, line in enumerate(text.splitlines(), 1):
            if rx.search(line):
                out.append({"file": str(p), "line": i, "text": line.strip()[:140]})
    return out


def is_conditional(line: str) -> bool:
    return "cfg_attr" in line


# ── B. 참조 0인 pub ────────────────────────────────────────────────────
def check_unused_pub(repo_files: dict[str, dict[Path, str]], ref_text: str) -> list[dict]:
    all_text = "\n".join(t for fs in repo_files.values() for t in fs.values()) + "\n" + ref_text
    counts: dict[str, int] = defaultdict(int)
    for w in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", all_text):
        counts[w] += 1
    out = []
    for repo, fs in repo_files.items():
        for p, text in fs.items():
            if "/tests/" in p.as_posix() or p.name in ("main.rs", "build.rs"):
                continue
            if "/examples/" in p.as_posix() or "/benches/" in p.as_posix():
                continue
            for i, line in enumerate(text.splitlines(), 1):
                m = ITEM_RE.match(line)
                if not m:
                    continue
                name = m.group(1)
                key = f"{crate_of(REPOS[repo], p)}/{name}"
                if key in ALLOW_PUB or name in ("new", "default", "main", "tests"):
                    continue
                if counts[name] <= 1:
                    out.append({"repo": repo, "file": str(p), "line": i, "name": name})
    return out


# ── C. 안 쓰는 번역 키(dir3 = `.lang` 자원 · 기준 언어 en) ─────────────────
LANG_KEY_RE = re.compile(r"^([A-Za-z][A-Za-z0-9_.]*)\s*=")


def lang_keys(repo: Path, code: str) -> list[str]:
    p = repo / "crates" / "ndir-i18n" / "lang" / f"{code}.lang"
    if not p.exists():
        return []
    keys = []
    for line in read(p).splitlines():
        m = LANG_KEY_RE.match(line)
        if m:
            keys.append(m.group(1))
    return keys


def check_unused_i18n(repo: Path, fs: dict[Path, str]) -> list[str]:
    keys = lang_keys(repo, "en")
    if not keys:
        return []
    # 사용 = 어떤 .rs에든 `"키"` 문자열(tr("키") · 레지스트리 label/desc · 상수 표) — 시험 모듈 안만 있는 것도 사용으로 친다(보수적).
    corpus = "\n".join(fs.values())
    quoted = set(re.findall(r'"([A-Za-z][A-Za-z0-9_.]*)"', corpus))
    out = []
    for k in keys:
        if any(k.startswith(pre) for pre in ALLOW_I18N_PREFIX):
            continue
        if k not in quoted:
            out.append(k)
    return out


def check_i18n_gaps(repo: Path) -> dict[str, list[str]]:
    """기준 언어(en)에 있는데 다른 언어 파일에 없는 키(번역 누락 · 폴백 en으로 나온다)."""
    en = set(lang_keys(repo, "en"))
    gaps = {}
    for p in sorted((repo / "crates" / "ndir-i18n" / "lang").glob("*.lang")):
        code = p.stem
        if code == "en":
            continue
        missing = sorted(en - set(lang_keys(repo, code)))
        if missing:
            gaps[code] = missing
    return gaps


# ── D. 안 쓰는 설정 키(dir3 = `REGISTRY`의 `e!("키", …)`) ─────────────────
def check_unused_settings(fs: dict[Path, str]) -> list[str]:
    reg = next((p for p in fs if p.as_posix().endswith("ndir-settings/src/registry.rs")), None)
    if not reg:
        return []
    keys = re.findall(r'e!\(\s*"([^"]+)"', fs[reg])
    corpus = "\n".join(t for p, t in fs.items() if p != reg)
    outside = "\n".join(t for p, t in fs.items() if "/ndir-settings/" not in p.as_posix())
    out = []
    for k in keys:
        if any(k.startswith(pre) for pre in ALLOW_SETTING_PREFIX):
            continue
        if f'"{k}"' not in corpus:
            out.append(k)
        elif f'"{k}"' not in outside and k not in ALLOW_SETTING_INTERNAL:
            # 설정 크레이트 안(이관 표·시험)에만 이름이 있다 = 앱이 읽지 않을 수 있다(확인 후보).
            out.append(k + "  (ndir-settings 안에서만)")
    return out


# ── E. 안 쓰는 의존 ────────────────────────────────────────────────────
def check_unused_deps(repo: Path) -> list[dict]:
    out = []
    for toml in sorted(repo.glob("crates/*/Cargo.toml")):
        crate = toml.parent.name
        text = read(toml)
        deps = []
        section = None
        for line in text.splitlines():
            s = line.strip()
            if s.startswith("["):
                section = s
                continue
            if section in ("[dependencies]", "[build-dependencies]") or (
                section and section.startswith("[target.") and section.endswith(".dependencies]")
            ):
                m = re.match(r"^([A-Za-z0-9_-]+)\s*(?:\.workspace)?\s*=", s)
                if m:
                    deps.append(m.group(1))
        src = "\n".join(read(p) for p in toml.parent.rglob("*.rs"))
        for d in deps:
            ident = d.replace("-", "_")
            if re.search(rf"\b{re.escape(ident)}\b", src):
                continue
            if f"{crate}/{d}" in ALLOW_DEP:
                continue
            out.append({"crate": crate, "dep": d})
    return out


# ── F. 크기 ────────────────────────────────────────────────────────────
FN_RE = re.compile(r"^(\s*)(?:pub(?:\([a-z]+\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([A-Za-z0-9_]+)")


def fn_lengths(p: Path, text: str) -> list[dict]:
    lines = text.splitlines()
    out = []
    i = 0
    while i < len(lines):
        m = FN_RE.match(lines[i])
        if not m:
            i += 1
            continue
        depth = 0
        started = False
        j = i
        while j < len(lines):
            code = strip_line_comment(lines[j])
            code = re.sub(r'"(?:\\.|[^"\\])*"', '""', code)
            code = re.sub(r"'(?:\\.|[^'\\])'", "''", code)
            depth += code.count("{") - code.count("}")
            if "{" in code:
                started = True
            if started and depth <= 0:
                break
            if not started and code.rstrip().endswith(";"):
                break
            j += 1
        if started:
            out.append({"file": str(p), "line": i + 1, "name": m.group(2), "len": j - i + 1})
        i += 1
    return out


def check_size(repo_files: dict[str, dict[Path, str]]) -> dict:
    files = []
    fns = []
    for fs in repo_files.values():
        for p, t in fs.items():
            files.append({"file": str(p), "lines": t.count("\n") + 1})
            fns.extend(fn_lengths(p, t))
    files.sort(key=lambda x: -x["lines"])
    fns.sort(key=lambda x: -x["len"])
    return {
        "files_over_3000": [f for f in files if f["lines"] > 3000],
        "top_files": files[:25],
        "fns_over_150": len([f for f in fns if f["len"] > 150]),
        "top_fns": fns[:30],
    }


# ── G. 중복 블록 ───────────────────────────────────────────────────────
def check_duplicates(repo_files: dict[str, dict[Path, str]], window: int = 10) -> list[dict]:
    seen: dict[str, list[tuple[str, int]]] = defaultdict(list)
    for fs in repo_files.values():
        for p, t in fs.items():
            if "/tests/" in p.as_posix():
                continue
            norm = []
            for i, line in enumerate(t.splitlines(), 1):
                s = strip_line_comment(line).strip()
                if not s or s in ("{", "}", "};", "},", ")", ");", "),") or s.startswith("#["):
                    continue
                if s.startswith("use ") or s.startswith("assert"):
                    continue
                norm.append((s, i))
            for k in range(0, len(norm) - window + 1):
                chunk = "\n".join(x[0] for x in norm[k : k + window])
                if len(chunk) < 350:
                    continue
                h = hashlib.sha1(chunk.encode()).hexdigest()
                seen[h].append((str(p), norm[k][1]))
    groups = []
    for locs in seen.values():
        files = {f for f, _ in locs}
        if len(locs) >= 2:
            groups.append({"count": len(locs), "files": len(files), "at": locs[:6]})
    # 겹치는 창 묶기: 같은 첫 위치 파일에서 연속된 것은 대표 하나만
    groups.sort(key=lambda g: (-g["count"], g["at"][0]))
    dedup = []
    taken: set[tuple[str, int]] = set()
    for g in groups:
        f, l = g["at"][0]
        if any((f, x) in taken for x in range(l - window * 2, l + 1)):
            continue
        taken.add((f, l))
        dedup.append(g)
    return dedup[:40]


# ── H. 커버리지(선택) ──────────────────────────────────────────────────
def run_cov(repo: Path) -> dict:
    cmd = ["cargo", "llvm-cov", "--workspace", "--summary-only", "--json"]
    r = subprocess.run(cmd, cwd=repo, capture_output=True, text=True, encoding="utf-8")
    if r.returncode != 0:
        return {"error": r.stderr[-600:]}
    data = json.loads(r.stdout)
    per_crate: dict[str, list[int]] = defaultdict(lambda: [0, 0])
    for f in data["data"][0]["files"]:
        name = f["filename"].replace("\\", "/")
        m = re.search(r"/crates/([^/]+)/", name)
        if not m:
            continue
        lines = f["summary"]["lines"]
        per_crate[m.group(1)][0] += lines["covered"]
        per_crate[m.group(1)][1] += lines["count"]
    tot = data["data"][0]["totals"]["lines"]
    return {
        "total_pct": round(tot["percent"], 1),
        "crates": {k: round(100 * c / n, 1) if n else 0.0 for k, (c, n) in sorted(per_crate.items())},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cov", action="store_true")
    ap.add_argument("--baseline")
    ap.add_argument("--save-baseline", action="store_true")
    a = ap.parse_args()

    repo_files: dict[str, dict[Path, str]] = {}
    for name, repo in REPOS.items():
        if repo.exists():
            repo_files[name] = {p: read(p) for p in rs_files(repo)}
    flat = {p: t for fs in repo_files.values() for p, t in fs.items()}
    ref_text = "\n".join(read(p) for r in REF_REPOS.values() if r.exists() for p in rs_files(r))
    dir3 = repo_files.get("nexa-dir3", {})

    allow = check_allow(flat)
    res = {
        "allow_unconditional": [x for x in allow if not is_conditional(x["text"])],
        "allow_conditional": len([x for x in allow if is_conditional(x["text"])]),
        "unused_pub": check_unused_pub(repo_files, ref_text),
        "unused_i18n": check_unused_i18n(ROOT, dir3),
        "i18n_gaps": check_i18n_gaps(ROOT),
        "unused_settings": check_unused_settings(dir3),
        "unused_deps": [
            dict(d, repo=n) for n, r in REPOS.items() if r.exists() for d in check_unused_deps(r)
        ],
        "size": check_size(repo_files),
        "duplicates": check_duplicates(repo_files),
        "lines": {n: sum(t.count("\n") + 1 for t in fs.values()) for n, fs in repo_files.items()},
    }
    if a.cov:
        res["coverage"] = {n: run_cov(r) for n, r in REPOS.items() if r.exists()}

    summary = {
        "lines": res["lines"],
        "allow_unconditional": len(res["allow_unconditional"]),
        "unused_pub": len(res["unused_pub"]),
        "unused_i18n": len(res["unused_i18n"]),
        "i18n_gaps": {k: len(v) for k, v in res["i18n_gaps"].items()},
        "unused_settings": len(res["unused_settings"]),
        "unused_deps": len(res["unused_deps"]),
        "files_over_3000": len(res["size"]["files_over_3000"]),
        "fns_over_150": res["size"]["fns_over_150"],
        "duplicate_groups": len(res["duplicates"]),
    }
    if "coverage" in res:
        summary["coverage_pct"] = {n: c.get("total_pct") for n, c in res["coverage"].items()}
    res["summary"] = summary

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "result.json").write_text(json.dumps(res, ensure_ascii=False, indent=1), encoding="utf-8")
    if a.save_baseline:
        (OUT / "baseline.json").write_text(json.dumps(res, ensure_ascii=False, indent=1), encoding="utf-8")

    base = None
    if a.baseline and Path(a.baseline).exists():
        base = json.loads(Path(a.baseline).read_text(encoding="utf-8")).get("summary")

    md = ["# 코드 건강 점검 보고서", "", "기준 = docs/25 §6(nexa-sql docs/93 차용). 수치는 **후보**(사람이 확인).", "", "| 항목 | 값 | 기준선 |", "|---|---|---|"]
    for k, v in summary.items():
        b = base.get(k) if base else ""
        md.append(f"| {k} | {v} | {b} |")

    def sect(title, rows, fmt):
        md.extend(["", f"## {title} ({len(rows)})", ""])
        md.extend(fmt(r) for r in rows[:200])

    sect("무조건 allow(dead_code/unused)", res["allow_unconditional"], lambda r: f"- {r['file']}:{r['line']} `{r['text']}`")
    sect("참조 0인 pub", res["unused_pub"], lambda r: f"- {r['file']}:{r['line']} `{r['name']}`")
    sect("안 쓰는 번역 키(en.lang)", res["unused_i18n"], lambda r: f"- `{r}`")
    for code, keys in res["i18n_gaps"].items():
        sect(f"번역 누락({code}.lang · en에만 있음)", keys, lambda r: f"- `{r}`")
    sect("안 쓰는 설정 키", res["unused_settings"], lambda r: f"- `{r}`")
    sect("안 쓰는 의존", res["unused_deps"], lambda r: f"- {r['repo']}/{r['crate']}: `{r['dep']}`")
    sect("3000줄 넘는 파일", res["size"]["files_over_3000"], lambda r: f"- {r['file']} — {r['lines']}")
    sect("긴 함수 상위", res["size"]["top_fns"], lambda r: f"- {r['file']}:{r['line']} `{r['name']}` — {r['len']}줄")
    sect("중복 블록(10줄 창)", res["duplicates"], lambda r: f"- ×{r['count']} " + " · ".join(f"{f}:{l}" for f, l in r["at"]))
    if "coverage" in res:
        md.extend(["", "## 커버리지(단위·통합 시험 · 줄 %)", ""])
        for n, c in res["coverage"].items():
            md.append(f"- **{n}** {c.get('total_pct')} %")
            for k, v in c.get("crates", {}).items():
                md.append(f"  - {k}: {v} %")
    (OUT / "report.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    print(json.dumps(summary, ensure_ascii=False, indent=1))
    print(f"report: {OUT / 'report.md'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
