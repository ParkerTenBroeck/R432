#!/usr/bin/env bash
set -euo pipefail

elf=$1
shift

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
binary="${elf}.bin"
tpt_dir="${root}/target/tpt"

rust-objcopy -O binary "${elf}" "${binary}"
mkdir -p "${tpt_dir}"

lua_root=${root//\\/\\\\}
lua_root=${lua_root//\"/\\\"}
printf 'R4_ROOT = "%s"\ndofile(R4_ROOT .. "/config.lua")\n' "${lua_root}" > "${tpt_dir}/autorun.lua"

exec powder ddir "${tpt_dir}" "$@"
