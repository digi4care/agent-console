#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="agent-console-tauri:rust-1.81"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
USE_XVFB=0
HOST_DISPLAY=0
BUILD_IMAGE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --xvfb)
      USE_XVFB=1
      shift
      ;;
    --host-display)
      HOST_DISPLAY=1
      shift
      ;;
    --build)
      BUILD_IMAGE=1
      shift
      ;;
    --)
      shift
      break
      ;;
    *)
      break
      ;;
  esac
done

if [[ $BUILD_IMAGE -eq 1 ]]; then
  docker build -f "$PROJECT_ROOT/docker/Dockerfile.tauri" -t "$IMAGE_NAME" "$PROJECT_ROOT/docker"
fi

if ! docker image inspect "$IMAGE_NAME" >/dev/null 2>&1; then
  echo "[tauri.sh] Docker image not found. Building once..." >&2
  docker build -f "$PROJECT_ROOT/docker/Dockerfile.tauri" -t "$IMAGE_NAME" "$PROJECT_ROOT/docker"
fi

if [[ $# -eq 0 ]]; then
  exit 0
fi

docker_args=(
  --rm
  -it
  -v "$PROJECT_ROOT:/workspace"
  -w /workspace
  --env PNPM_HOME=/usr/local/share/pnpm
  --env RUST_MIN_STACK=67108864
)

if [[ $HOST_DISPLAY -eq 1 ]]; then
  if [[ -z "${DISPLAY:-}" ]]; then
    echo "[tauri.sh] --host-display requested but \$DISPLAY is empty." >&2
    exit 1
  fi
  XAUTH_TMP="${PROJECT_ROOT}/docker/.docker.xauth"
  touch "$XAUTH_TMP"
  chmod 600 "$XAUTH_TMP"
  if command -v xauth >/dev/null 2>&1; then
    xauth nlist "$DISPLAY" | sed -e 's/^..../ffff/' | xauth -f "$XAUTH_TMP" nmerge - >/dev/null 2>&1 || true
  else
    echo "[tauri.sh] Warning: xauth not found on host; X11 auth may fail." >&2
  fi
  docker_args+=(
    -e DISPLAY="$DISPLAY"
    -e XAUTHORITY=/tmp/.docker.xauth
    -v /tmp/.X11-unix:/tmp/.X11-unix
    -v "$XAUTH_TMP:/tmp/.docker.xauth"
  )
fi

if [[ $USE_XVFB -eq 1 ]]; then
  if [[ $# -eq 0 ]]; then
    echo "[tauri.sh] --xvfb requires a command to run." >&2
    exit 1
  fi
  CMD_STR=$(printf ' %q' "$@")
  CMD_STR=${CMD_STR:1}
  docker run "${docker_args[@]}" \
    "$IMAGE_NAME" \
    bash -lc "xvfb-run -s '-screen 0 1920x1080x24' $CMD_STR"
else
  docker run "${docker_args[@]}" \
    "$IMAGE_NAME" "$@"
fi
