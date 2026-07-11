#!/usr/bin/env bash
# Downloads the current Magic: The Gathering Comprehensive Rules text from
# Wizards of the Coast's official rules page into docs/MagicCompRules.txt.
#
# The CR is © Wizards of the Coast LLC. This file is git-ignored — it is
# downloaded locally for development reference and is not redistributed by
# this repository.
#
# Usage:
#   ./scripts/fetch-comp-rules.sh
#
# The script scrapes magic.wizards.com/en/rules for the latest .txt link
# rather than hardcoding a date-stamped URL, since Wizards publishes a new
# CR file at a new URL with each rules update.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEST="$REPO_ROOT/docs/MagicCompRules.txt"
RULES_PAGE="https://magic.wizards.com/en/rules"

mkdir -p "$(dirname "$DEST")"

echo "Looking up current CR URL from $RULES_PAGE..."
TXT_URL="$(curl -fsSL "$RULES_PAGE" \
  | grep -oE 'https://media\.wizards\.com/[^"]+MagicCompRules[^"]+\.txt' \
  | head -n 1)"

if [ -z "${TXT_URL:-}" ]; then
  echo "ERROR: Could not find Comprehensive Rules .txt link on $RULES_PAGE" >&2
  echo "       The page structure may have changed. Download manually from:" >&2
  echo "       $RULES_PAGE" >&2
  exit 1
fi

echo "Downloading $TXT_URL"
# WotC's published URL may contain unescaped spaces ("MagicCompRules 20260227.txt").
# Encode spaces so curl can handle it.
TXT_URL_ENCODED="${TXT_URL// /%20}"
curl -fsSL "$TXT_URL_ENCODED" -o "$DEST"

# Wizards' CR file is UTF-16 with BOM; convert to UTF-8 if needed for grep.
if file "$DEST" | grep -qi 'utf-16'; then
  echo "Converting UTF-16 → UTF-8"
  iconv -f UTF-16 -t UTF-8 "$DEST" > "$DEST.utf8"
  mv "$DEST.utf8" "$DEST"
fi

LINES="$(wc -l < "$DEST" | tr -d ' ')"
echo "Wrote $DEST ($LINES lines)"
