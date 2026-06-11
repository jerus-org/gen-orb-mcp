# Use the freshly-built binary if it was attached from the workspace,
# otherwise fall back to the gen-orb-mcp installed in the image.
if [[ -f "${WORKSPACE_BIN_PATH}/${NAME}" ]]; then
  chmod +x "${WORKSPACE_BIN_PATH}/${NAME}"
  echo "export PATH=${WORKSPACE_BIN_PATH}:\$PATH" >> "$BASH_ENV"
fi
VERSION="${CIRCLE_TAG#${TAG_PREFIX}}"
echo "export VERSION=${VERSION}" >> "$BASH_ENV"
echo "export CIRCLE_BRANCH=main" >> "$BASH_ENV"
git fetch origin main
git checkout -B main origin/main
