set -- gen-orb-mcp build
set -- "$@" --input "${INPUT}"
[[ -n "${BUILD_NAME:-}" ]] && set -- "$@" --name "${BUILD_NAME}"
"$@"
