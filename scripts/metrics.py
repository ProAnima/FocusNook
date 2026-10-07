"""Architecture metrics for FocusNook — the baseline and progress gauge for docs/agent-roadmap.md.

Usage:  python scripts/metrics.py            # human-readable table
        python scripts/metrics.py --json     # machine-readable (for agents / CI artifacts)

Text heuristics only (no compiler), so it runs in seconds anywhere. Metric ids are stable:
roadmap cards reference them in acceptance criteria. Rust metrics look at production code
only (everything before the first `#[cfg(test)]`, and never `tests.rs`/`test_support.rs`).
Line counts skip blank lines and `//` comment lines.
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FRONTEND = os.path.join(ROOT, "apps", "desktop", "src")
DESKTOP_RS = os.path.join(ROOT, "apps", "desktop", "src-tauri", "src")
SERVER_RS = os.path.join(ROOT, "apps", "server", "src")
TRANSLATIONS = os.path.join(FRONTEND, "shared", "translations")
FILE_BUDGET = 300
EXPECTED_MARKER = "// expected:"


def files(base, exts):
    for dirpath, _, names in os.walk(base):
        for name in sorted(names):
            if name.endswith(exts):
                yield os.path.join(dirpath, name)


def read(path):
    with open(path, encoding="utf8") as handle:
        return handle.read()


def rel(path):
    return os.path.relpath(path, ROOT).replace("\\", "/")


def is_test(path):
    name = os.path.basename(path)
    return ".test." in name or name in ("tests.rs", "test_support.rs") or "/test/" in rel(path)


def strip_comments(text):
    return "\n".join(line for line in text.split("\n") if not line.strip().startswith("//"))


def code_lines(text):
    return [line for line in strip_comments(text).split("\n") if line.strip()]


def rust_production(text):
    cut = text.find("#[cfg(test)]")
    return text if cut < 0 else text[:cut]


def count(pattern, texts):
    regex = re.compile(pattern, re.MULTILINE)
    return sum(len(regex.findall(text)) for text in texts)


def swallowed_catches(texts):
    """Catch sites that drop the error value and carry no `// expected:` justification."""
    regex = re.compile(r"\.catch\(\(\) =>|catch \{")
    total = 0
    for text in texts:
        lines = text.split("\n")
        for i, line in enumerate(lines):
            if regex.search(line):
                context = line + (lines[i - 1] if i > 0 else "") + (lines[i + 1] if i + 1 < len(lines) else "")
                if EXPECTED_MARKER not in context:
                    total += 1
    return total


def translation_gaps():
    key = re.compile(r'^\s*"?([A-Za-z][\w.]*)"?\s*:', re.MULTILINE)
    keys = {}
    for path in files(TRANSLATIONS, (".ts",)):
        name = os.path.splitext(os.path.basename(path))[0]
        if name in ("index", "types", "makeDictionary"):
            continue
        keys[name] = {k for k in key.findall(read(path)) if "." in k}
    reference = keys.get("ru", set())
    return {locale: len(reference - found) for locale, found in sorted(keys.items()) if locale != "ru"}


def registered_commands():
    lib = read(os.path.join(DESKTOP_RS, "lib.rs"))
    match = re.search(r"generate_handler!\[(.*?)\]", lib, re.S)
    if not match:
        return 0
    return len([item for item in match.group(1).split(",") if item.strip() and not item.strip().startswith("//")])


def collect():
    fe_paths = [p for p in files(FRONTEND, (".ts", ".tsx")) if not is_test(p)]
    fe = {p: read(p) for p in fe_paths}
    css = {p: read(p) for p in files(FRONTEND, (".css",))}
    rs = {p: rust_production(read(p)) for p in files(DESKTOP_RS, (".rs",)) if not is_test(p)}
    srv = {p: rust_production(read(p)) for p in files(SERVER_RS, (".rs",)) if not is_test(p)}

    oversized = sorted(
        ((len(code_lines(text)), rel(path)) for path, text in {**fe, **css, **rs, **srv}.items()
         if len(code_lines(text)) > FILE_BUDGET),
        reverse=True,
    )
    rs_code = [strip_comments(t) for t in rs.values()]
    commands = [strip_comments(t) for p, t in rs.items() if "/commands/" in rel(p)]
    many_state = sorted(
        ((len(re.findall(r"\buseState[<(]", t)), rel(p)) for p, t in fe.items() if p.endswith(".tsx")
         and len(re.findall(r"\buseState[<(]", t)) >= 6),
        reverse=True,
    )
    domain_to_commands = [
        rel(p) for p, t in rs.items()
        if "/commands/" not in rel(p) and re.search(r"crate::commands\b|use crate::\{[^}]*\bcommands\b", t)
    ]
    dto_types = count(r"\bexport (?:interface|type) \w+", [read(os.path.join(FRONTEND, "shared", "commands", "types.ts"))])
    gaps = translation_gaps()

    metrics = {
        "M1_files_over_budget": len(oversized),
        "M2_rust_string_error_signatures": count(r"\bfn\b[^{;]*?->\s*Result<[^{;]*?,\s*String>", rs_code),
        "M3_rust_errors_flattened_to_string": count(
            r"map_err\(\|\w+\| \w+\.to_string\(\)\)|map_err\(\|\w+\| format!|map_err\(\|_\| \"", rs_code),
        "M4_catches_without_expected_marker": swallowed_catches(fe.values()),
        "M5_handwritten_types_in_commands_types_ts": dto_types,
        "M6_tauri_commands_registered": registered_commands(),
        "M6b_tauri_command_fns_defined": count(r"#\[tauri::command\]", rs_code),
        "M7_commands_locking_state_directly": count(r"\.0\.lock\(\)", commands),
        "M8_components_with_6plus_useState": len(many_state),
        "M9_domain_imports_commands": len(domain_to_commands),
        "M10_print_macros_in_production": count(r"\b(?:e?println)!", rs_code + [strip_comments(t) for t in srv.values()]),
        "M11_lint_suppressions": count(r"eslint-disable", fe.values())
        + count(r"#!?\[(?:cfg_attr\([^\]]*)?allow\((?!clippy::(?:unwrap_used|expect_used))", rs_code + [strip_comments(t) for t in srv.values()]),
        "M12_untranslated_keys_total": sum(gaps.values()),
    }
    details = {
        "oversized_files": oversized,
        "components_with_6plus_useState": many_state,
        "domain_imports_commands": domain_to_commands,
        "untranslated_keys_per_locale": gaps,
    }
    return metrics, details


def main():
    metrics, details = collect()
    if "--json" in sys.argv:
        print(json.dumps({"metrics": metrics, "details": details}, ensure_ascii=False, indent=2))
        return
    width = max(len(k) for k in metrics)
    for key, value in metrics.items():
        print(f"{key.ljust(width)}  {value}")
    print(f"\nOversized (production lines > {FILE_BUDGET}):")
    for n, path in details["oversized_files"]:
        print(f"  {n:5}  {path}")
    if details["components_with_6plus_useState"]:
        print("\nComponents with >= 6 useState calls:")
        for n, path in details["components_with_6plus_useState"]:
            print(f"  {n:5}  {path}")
    print("\nUntranslated keys vs ru (English fallback):")
    print("  " + ", ".join(f"{k}={v}" for k, v in details["untranslated_keys_per_locale"].items()))


if __name__ == "__main__":
    main()
