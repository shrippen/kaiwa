#!/usr/bin/env bash
# Runs the QML tests of the app without a screen (app/qml/tests). Needs Qt 6 with QtTest and
# Kirigami; no microphone, no speech models, no tutor. Not part of CI yet.
#
#   tools/qml-tests.sh
set -euo pipefail
cd "$(dirname "$0")/.."
RUNNER=${QMLTESTRUNNER:-/usr/lib/qt6/bin/qmltestrunner}
[ -x "$RUNNER" ] || RUNNER=$(command -v qmltestrunner6 || command -v qmltestrunner)
QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 "$RUNNER" -input app/qml/tests -import app/qml
