#!/usr/bin/env bash
# Copies Kante's QML module unchanged into the app (GUI rule: generated from Kante, not
# inspired by it) and writes the resource file that compiles it into the binary, so
# `import Kante` works without an import path.
#
#   tools/sync-kante.sh [path to the Kante checkout, default ../Kante]
#
# CI runs it against Kante's main and fails when app/qml/Kante or the plasmoid's copy differs
# (job `qt`).
set -euo pipefail
cd "$(dirname "$0")/.."
KANTE=${1:-../Kante}
[ -f "$KANTE/qml/Kante/qmldir" ] || { echo "no Kante checkout at $KANTE" >&2; exit 1; }
rm -rf app/qml/Kante
cp -r "$KANTE/qml/Kante" app/qml/Kante
{
  echo '<!DOCTYPE RCC><RCC version="1.0">'
  echo '<qresource prefix="/qt/qml">'
  (cd app/qml && find Kante -type f | LC_ALL=C sort | sed 's|.*|  <file>&</file>|')
  echo '</qresource>'
  echo '</RCC>'
} > app/qml/kante.qrc
echo "synced app/qml/Kante from $KANTE ($(git -C "$KANTE" describe --always 2>/dev/null || echo unknown))"

# The plasmoid (ADR-008, /LF504/): Kante and KantePlasma side by side, as directory imports
# (a Plasma package has no import path).
UI=plasmoid/package/contents/ui
rm -rf "$UI/Kante" "$UI/KantePlasma"
cp -r "$KANTE/qml/Kante" "$UI/Kante"
cp -r "$KANTE/qml/KantePlasma" "$UI/KantePlasma"
echo "synced $UI/Kante and $UI/KantePlasma"
