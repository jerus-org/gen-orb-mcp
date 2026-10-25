set -- gen-orb-mcp save
set -- "$@" --paths "${GCO_PATHS}"
[[ ! "${GCO_MESSAGE:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --message "${GCO_MESSAGE}"
[[ ! "${GCO_PUSH:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --push "${GCO_PUSH}"
[[ "${GCO_NO_PUSH:-false}" = "true" ]] && set -- "$@" --no-push
[[ "${GCO_DRY_RUN:-false}" = "true" ]] && set -- "$@" --dry-run
[[ "${GCO_SIGN:-false}" = "true" ]] && set -- "$@" --sign
[[ ! "${GCO_CONFIG:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --config "${GCO_CONFIG}"
[[ ! "${GCO_GPG_KEY_ENV:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --gpg-key-env "${GCO_GPG_KEY_ENV}"
[[ ! "${GCO_TRUST_ENV:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --trust-env "${GCO_TRUST_ENV}"
[[ ! "${GCO_USER_NAME_ENV:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --user-name-env "${GCO_USER_NAME_ENV}"
[[ ! "${GCO_USER_EMAIL_ENV:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --user-email-env "${GCO_USER_EMAIL_ENV}"
[[ ! "${GCO_SIGN_KEY_ENV:-}" =~ ^[[:space:]]*$ ]] && set -- "$@" --sign-key-env "${GCO_SIGN_KEY_ENV}"
"$@"
