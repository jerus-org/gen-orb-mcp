set -- gen-orb-mcp build
set -- "$@" --input "${INPUT}"
[[ -n "${BUILD_NAME:-}" ]] && set -- "$@" --name "${BUILD_NAME}"
[[ -n "${TARGET:-}" ]] && set -- "$@" --target "${TARGET}"
[[ "${DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
"$@"
