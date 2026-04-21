#!/usr/bin/env bash
# OMNI integration for OpenAI Codex CLI
# Usage: bash integrations/codex-cli/install.sh

set -euo pipefail

CODEX_CONFIG_DIR="${HOME}/.codex"
OMNI_WRAPPER="${CODEX_CONFIG_DIR}/omni-wrapper.sh"

mkdir -p "${CODEX_CONFIG_DIR}"

# Create OMNI wrapper script
cat > "${OMNI_WRAPPER}" << 'WRAPPER'
#!/usr/bin/env bash
# OMNI wrapper for Codex CLI
# Executes command and pipes through OMNI for distillation

COMMAND="$*"
export OMNI_CMD="${COMMAND}"
export OMNI_AGENT="codex_cli"

eval "${COMMAND}" 2>&1 | omni
WRAPPER
chmod +x "${OMNI_WRAPPER}"

# Create/update Codex config
CODEX_CONFIG="${CODEX_CONFIG_DIR}/config.json"
if [ -f "${CODEX_CONFIG}" ]; then
    cp "${CODEX_CONFIG}" "${CODEX_CONFIG}.bak"
    echo "Backed up existing config to ${CODEX_CONFIG}.bak"
fi

cat > "${CODEX_CONFIG}" << CONFIG
{
  "shellWrapper": "${OMNI_WRAPPER}",
  "systemPromptSuffix": "When running shell commands, output is automatically distilled by OMNI Signal Engine. Errors and key signals are preserved; noise is filtered. If you need full output, ask for it explicitly."
}
CONFIG

echo "✅ OMNI + Codex CLI integration installed"
echo "   Config: ${CODEX_CONFIG}"
echo "   Wrapper: ${OMNI_WRAPPER}"
echo ""
echo "   Monitor savings: omni stats"
