#!/bin/sh
# A package never contains the demo (rules for all projects; Q-15): no demo profile, no recorded
# tutor answers, no screenshot mode. Those come only with the build feature `demo`. Run on the
# binary a package ships; fails if any trace of the demo is in it.
#
#   packaging/check-release.sh target/release/kaiwa
set -eu
bin=${1:?usage: check-release.sh <binary>}
test -s "$bin"
for mark in 'kaiwa:demo-build' 'KAIWA_DEMO' '今日は寒くないですね。'; do
    if grep -aqF "$mark" "$bin"; then
        echo "check-release: $bin contains the demo ($mark); build without --features demo" >&2
        exit 1
    fi
done
echo "check-release: no demo in $bin"
