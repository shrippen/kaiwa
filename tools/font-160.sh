#!/usr/bin/env bash
# Large text (hinnerk_font, /LL202/, T-03): every view of the demo at 160 %, without a screen.
# Writes one picture per view and lists every text that is elided, wider than its place or
# beyond the window's edge. Local only, not part of CI: the pictures still want a look
# (overlaps, furigana), step 4.4 of spec/LOKALER-TEST.md.
#
#   tools/font-160.sh [folder for the pictures, default out/font-160]
#   SCALE=2 SCREEN=1920 tools/font-160.sh     # another text size, another screen width
#   SCALE=1 SCREEN=800 tools/font-160.sh      # the narrowest window the app allows (/LL303/)
#
# The window is as wide as the app opens it (900), or as the screen leaves room for at this
# text size: a small laptop (1366 px) at 160 % has 853. The scale is the profile's text size,
# which the app applies the same way (QT_SCALE_FACTOR).
#
# Needs the app's build dependencies (Qt 6, Kirigami). No microphone, no speech models, no
# tutor: the demo profile and its recorded answers.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${1:-out/font-160}
SCALE=${SCALE:-1.6}
SCREEN=${SCREEN:-1366}
WIDTH=$(awk -v s="$SCALE" -v w="$SCREEN" 'BEGIN { v = int(w / s); print (v < 900 ? v : 900) }')
mkdir -p "$OUT"
EMPTY=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}" kaiwa-font-XXXXXX)
trap 'rm -rf "$EMPTY"' EXIT
cargo build -q -p kaiwa-app --features demo
BIN=${CARGO_TARGET_DIR:-target}/debug/kaiwa
# The findings come as log lines: on stderr also where Qt would log to the journal.
export KAIWA_DEMO=1 KAIWA_MODELS="$EMPTY" QT_QPA_PLATFORM=offscreen QT_SCALE_FACTOR=$SCALE QT_FORCE_STDERR_LOGGING=1
bad=0
shot() { # name, then the app's switches
    local name=$1; shift
    local log="$OUT/$name.log"
    if ! timeout 120 "$BIN" --shot "$OUT/$name.png" --check-text --kind 1 --width "$WIDTH" "$@" > "$log" 2>&1; then
        echo "$name: the app did not finish (see $log)"; bad=1; return
    fi
    if ! grep -q "kaiwa-text-checked:" "$log" || [ ! -s "$OUT/$name.png" ]; then
        echo "$name: no picture or no check (see $log)"; bad=1; return
    fi
    if grep -q "kaiwa-text: " "$log"; then
        echo "$name:"; grep "kaiwa-text: " "$log" | sed 's/^.*kaiwa-text: /  /' | sort -u; bad=1
    else
        echo "$name: every text fits"
    fi
}
for lang in de en; do
    export KAIWA_LANG=$lang
    for view in start path review profile settings; do
        shot "$view-$lang" --view "$view" --height 1500
    done
    shot "talk-$lang" --view talk --scenario n5-02-jikoshoukai --height 1500 \
        --say '?|マラです。よろしくおねがいします。|ビール二ください。|コーヒーを好きです。'
done
echo "text size $SCALE, window $WIDTH wide; pictures in $OUT"
exit $bad
