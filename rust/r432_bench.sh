#!/usr/bin/env bash
set -euo pipefail

luajit=luajit

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

cd "${root}"
source "${root}/build.sh"

exec "${luajit}" "${root}/r432_bench.lua" --binary "${BIN_ABS}" --config "${root}/config.lua" "$@"
