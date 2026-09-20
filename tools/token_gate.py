#!/usr/bin/env python3
"""Reject visual literals and trading arithmetic outside qml/theme."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class Violation:
    path: Path
    line: int
    category: str
    source: str


RULES = (
    (
        "color-literal",
        re.compile(r"(?:color\s*:|\bcolor\s*=)[^\n]*(?:#[0-9a-fA-F]{3,8}\b|\"(?:white|black|red|green|blue|yellow|orange|gray|grey)\")"),
    ),
    ("pixel-size", re.compile(r"\bpixelSize\s*:\s*-?\d+(?:\.\d+)?\b")),
    (
        "raw-dimension",
        re.compile(
            r"(?:^\s*|[;{]\s*)(?:width|height|implicitWidth|implicitHeight|spacing|radius|padding|leftPadding|rightPadding|topPadding|bottomPadding|anchors\.margins|anchors\.(?:left|right|top|bottom)Margin|border\.width|Layout\.(?:preferred|minimum|maximum)Width|Layout\.(?:preferred|minimum|maximum)Height)\s*:\s*-?\d+(?:\.\d+)?\b"
        ),
    ),
    (
        "money-arithmetic",
        re.compile(
            r"(?<!\.)\b(?:price|quantity|requested_quantity|unrealized_pnl|session_pnl|cash_balance|initial_balance|amount|fee|slippage)\b\s*[+*/-]|[+*/-]\s*(?<!\.)\b(?:price|quantity|requested_quantity|unrealized_pnl|session_pnl|cash_balance|initial_balance|amount|fee|slippage)\b",
            re.IGNORECASE,
        ),
    ),
)


def _is_theme(path: Path) -> bool:
    parts = path.as_posix().split("/")
    return any(parts[index : index + 2] == ["qml", "theme"] for index in range(len(parts) - 1))


def scan_paths(paths: Iterable[Path]) -> list[Violation]:
    files: list[Path] = []
    for path in paths:
        if path.is_dir():
            files.extend(sorted(path.rglob("*.qml")))
        elif path.suffix == ".qml":
            files.append(path)

    violations: list[Violation] = []
    for path in files:
        if _is_theme(path):
            continue
        for line_number, source in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            code = source.split("//", 1)[0]
            for category, pattern in RULES:
                if pattern.search(code):
                    violations.append(Violation(path, line_number, category, source.strip()))
    return violations


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail when violations exist")
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()
    violations = scan_paths(args.paths)
    counts = {category: 0 for category, _ in RULES}
    for violation in violations:
        counts[violation.category] += 1
        print(
            f"{violation.path}:{violation.line}: {violation.category}: {violation.source}"
        )
    print("token gate: " + ", ".join(f"{key}={value}" for key, value in counts.items()))
    return 1 if args.check and violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
