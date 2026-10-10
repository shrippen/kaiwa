#!/usr/bin/env bash
# kaiwad on a private session bus (run inside `dbus-run-session`): today, reviews, a short talk
# turn and a review answer must come back as JSON. With PLASMOID=1 the plasmoid is loaded too
# (plasmoidviewer) and its QML log must be free of errors.
#
#   dbus-run-session -- tools/kaiwad-smoke.sh [target/debug/kaiwad]
set -euo pipefail
KAIWAD=${1:-target/debug/kaiwad}
[ -x "$KAIWAD" ] || { echo "no $KAIWAD: cargo build -p kaiwad first" >&2; exit 1; }
export KAIWA_DEMO=1 KAIWA_NOW=1791100000:120 KAIWAD_IDLE_SECS=60 KAIWAD_TRACE=1
"$KAIWAD" 2> /tmp/kaiwad.log & pid=$!
trap 'kill $pid 2>/dev/null || true' EXIT
for _ in $(seq 50); do
  gdbus introspect --session -d org.shrippen.Kaiwa1 -o /org/shrippen/Kaiwa1 >/dev/null 2>&1 && break
  sleep 0.2
done
call() { gdbus call --session -d org.shrippen.Kaiwa1 -o /org/shrippen/Kaiwa1 -m "org.shrippen.Kaiwa1.$@"; }
call Today | grep -q '"goalText"'
call Reviews 5 | grep -q '"fromMistake"'
call StartShort | grep -q '"goals"'
call Say "はい、おねがいします。" true | grep -q '"reply"'
call AnswerReview "天気" "天気" false | grep -q '"right":true'
call Reminder | grep -q "null"   # the demo profile has no reminder (off by default)
call Snooze >/dev/null
echo "kaiwad: all calls answered"
if [ "${PLASMOID:-}" = 1 ]; then
  before=$(grep -c "call Today" /tmp/kaiwad.log || true)
  timeout 20 plasmoidviewer -a plasmoid/package > /tmp/plasmoid.log 2>&1 || true
  cat /tmp/plasmoid.log
  # The plasmoid ran and reached kaiwad: its own Today call arrived.
  after=$(grep -c "call Today" /tmp/kaiwad.log || true)
  [ "$after" -gt "$before" ] || { echo "plasmoid: no call reached kaiwad" >&2; exit 1; }
  ! grep -E "qml.*(Error|Unable|is not|TypeError|ReferenceError|not installed|not a type)" /tmp/plasmoid.log
  echo "plasmoid: loaded without QML errors"
fi
