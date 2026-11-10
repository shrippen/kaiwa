#!/usr/bin/env bash
# Runs the QML tests of the app without a screen (app/qml/tests). Needs Qt 6 with QtTest and
# Kirigami; no microphone, no speech models, no tutor. CI runs it in the job `qt`.
#
#   tools/qml-tests.sh
set -euo pipefail
cd "$(dirname "$0")/.."
RUNNER=${QMLTESTRUNNER:-/usr/lib/qt6/bin/qmltestrunner}
# Not `qmltestrunner` from the path: where Qt 5 is installed too, that is Qt 5's.
[ -x "$RUNNER" ] || RUNNER=$(command -v qmltestrunner6 || command -v qmltestrunner-qt6 || true)
[ -n "$RUNNER" ] || { echo "no Qt 6 qmltestrunner found; set QMLTESTRUNNER" >&2; exit 1; }
QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 "$RUNNER" -input app/qml/tests -import app/qml
