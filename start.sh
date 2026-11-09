#!/usr/bin/env bash
# Kaiwa from the repository: one entry point for the app, the demo and the QA runner.
#
#   ./start.sh                 the app with your profile (~/.local/share/kaiwa or $KAIWA_DATA)
#   ./start.sh demo [de|en]    the app built with the demo: a fresh demo profile (temporary, two
#                              weeks of history), recorded tutor answers, the screenshot switches
#   ./start.sh sim [args…]     kaiwa-sim, default `run` (the persona journeys in text mode)
#   ./start.sh check           what CI runs on the Rust side: fmt, clippy, tests, personas
#
# The tutor is set up in the settings (Tutor); KAIWA_PROVIDER (a preset id: ollama,
# openrouter, …, see app/README.md) with KAIWA_MODEL and the key variable overrides it. Recorded answers (qa/fixtures) only in the demo build.
# Further arguments go to the program, e.g. `./start.sh demo en --view path`.
set -euo pipefail
cd "$(dirname "$0")"

mode="${1:-app}"
[ $# -gt 0 ] && shift

case "$mode" in
  app)
    exec cargo run -q -p kaiwa-app -- "$@"
    ;;
  demo)
    lang=de
    case "${1:-}" in de|en) lang="$1"; shift ;; esac
    # The demo looks words up in the excerpt of the dictionary (data/dict), never in a download.
    export KAIWA_DEMO=1 KAIWA_LANG="$lang" KAIWA_DICT="$PWD/data/dict/kaiwa-dict-excerpt.sqlite"
    exec cargo run -q -p kaiwa-app --features demo -- "$@"
    ;;
  sim)
    [ $# -eq 0 ] && set -- run
    exec cargo run -q -p kaiwa-sim -- "$@"
    ;;
  check)
    cargo fmt --all --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo run -q -p kaiwa-sim -- validate
    cargo run -q -p kaiwa-sim -- run
    ;;
  -h|--help|help)
    sed -n '2,12s/^# \{0,1\}//p' "$0"
    ;;
  *)
    echo "Unknown mode: $mode (app, demo, sim, check, help)" >&2
    exit 2
    ;;
esac
