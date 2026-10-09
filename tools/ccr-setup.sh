#!/usr/bin/env bash
# Setup script for Claude Code cloud sessions (decision G-05). Paste it into the cloud
# environment's settings under "Setup script"; every new session runs it before it starts.
#
#   always              tea (Gitea CLI); the login stays OAuth, done once per session
#   KAIWA_QT=1          also Qt 6.8 (aqtinstall) and ECM + Kirigami 6.10 built from source
#                       (about 10 minutes); needed for Kante's QML tests and Kaiwa's Qt spikes.
#                       Set it as an environment variable of the cloud environment.
#
# Afterwards: `source /etc/profile.d/kaiwa-qt.sh` puts Qt and Kirigami on the paths.
set -euo pipefail

TEA_VERSION=0.10.1
if ! command -v tea >/dev/null; then
    curl -fsSL -o /usr/local/bin/tea "https://dl.gitea.com/tea/${TEA_VERSION}/tea-${TEA_VERSION}-linux-amd64"
    chmod +x /usr/local/bin/tea
fi

if [ "${KAIWA_QT:-0}" = "1" ] && [ ! -d /opt/kf6 ]; then
    QT_VERSION=6.8.3 KF_VERSION=v6.10.0 WORK=/opt/kaiwa-build
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq cmake ninja-build libgl-dev libxkbcommon-dev libvulkan-dev gettext git-lfs >/dev/null
    pip install -q aqtinstall
    aqt install-qt linux desktop "$QT_VERSION" linux_gcc_64 -O /opt/qt -m qtshadertools qt5compat
    QT=/opt/qt/$QT_VERSION/gcc_64
    mkdir -p "$WORK" && cd "$WORK"
    git clone -q --depth 1 --branch "$KF_VERSION" https://invent.kde.org/frameworks/extra-cmake-modules.git ecm
    git clone -q --depth 1 --branch "$KF_VERSION" https://invent.kde.org/frameworks/kirigami.git kirigami
    cmake -S ecm -B ecm/build -G Ninja -DCMAKE_INSTALL_PREFIX=/opt/kf6 -DBUILD_TESTING=OFF \
        -DBUILD_HTML_DOCS=OFF -DBUILD_MAN_DOCS=OFF -DBUILD_QTHELP_DOCS=OFF
    cmake --build ecm/build && cmake --install ecm/build
    cmake -S kirigami -B kirigami/build -G Ninja -DCMAKE_PREFIX_PATH="$QT;/opt/kf6" -DCMAKE_INSTALL_PREFIX=/opt/kf6 \
        -DBUILD_TESTING=OFF -DBUILD_EXAMPLES=OFF -DCMAKE_BUILD_TYPE=Release
    cmake --build kirigami/build && cmake --install kirigami/build
    rm -rf "$WORK"
    cat > /etc/profile.d/kaiwa-qt.sh <<PROFILE
export PATH=$QT/bin:\$PATH
export QML_IMPORT_PATH=/opt/kf6/lib/x86_64-linux-gnu/qml
export LD_LIBRARY_PATH=/opt/kf6/lib/x86_64-linux-gnu:$QT/lib
export QT_PLUGIN_PATH=/opt/kf6/lib/x86_64-linux-gnu/plugins:$QT/plugins
export CMAKE_PREFIX_PATH=$QT:/opt/kf6
export QMAKE=$QT/bin/qmake
export QT_QPA_PLATFORM=offscreen
export LANG=C.UTF-8
PROFILE
fi
