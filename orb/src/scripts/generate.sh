set -- gen-orb-mcp generate
set -- "$@" --orb-path "${ORB_PATH}"
[ -n "${OUTPUT:-}" ] && set -- "$@" --output "${OUTPUT}"
[ -n "${FORMAT:-}" ] && set -- "$@" --format "${FORMAT}"
[ -n "${GENERATE_NAME:-}" ] && set -- "$@" --name "${GENERATE_NAME}"
[ -n "${VERSION:-}" ] && set -- "$@" --version "${VERSION}"
[ "${FORCE:-false}" = "true" ] && set -- "$@" --force
[ -n "${MIGRATIONS:-}" ] && set -- "$@" --migrations "${MIGRATIONS}"
[ -n "${PRIOR_VERSIONS:-}" ] && set -- "$@" --prior-versions "${PRIOR_VERSIONS}"
[ -n "${TAG_PREFIX:-}" ] && set -- "$@" --tag-prefix "${TAG_PREFIX}"
"$@"