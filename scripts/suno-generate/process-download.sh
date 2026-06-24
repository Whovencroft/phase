#!/usr/bin/env bash
#
# Process a downloaded Suno track: find in ~/Downloads, convert to M4A, move to project.
#
# Usage:
#   ./process-download.sh <track-id> [suno-filename]
#
# If suno-filename is omitted, finds the most recent .mp3 in ~/Downloads.
#
# Examples:
#   ./process-download.sh menu-main                    # grab latest mp3 from Downloads
#   ./process-download.sh menu-main "Hall of the Planeswalkers.mp3"  # specific file

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FINAL_DIR="$(cd "$SCRIPT_DIR/../../client/public/audio/music" && pwd)"
DOWNLOADS_DIR="$HOME/Downloads"
THEME_ID="planeswalker"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log()  { echo -e "${BLUE}[process]${NC} $*"; }
ok()   { echo -e "${GREEN}[process]${NC} $*"; }
warn() { echo -e "${YELLOW}[process]${NC} $*"; }
err()  { echo -e "${RED}[process]${NC} $*" >&2; }

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <track-id> [suno-filename]"
  echo ""
  echo "Track IDs from tracks.json:"
  jq -r '.tracks[] | "  \(.id)  (\(.title))"' "$SCRIPT_DIR/tracks.json"
  exit 1
fi

TRACK_ID="$1"
SUNO_FILE="${2:-}"

# Validate track ID exists
if ! jq -e --arg id "$TRACK_ID" '.tracks[] | select(.id == $id)' "$SCRIPT_DIR/tracks.json" >/dev/null 2>&1; then
  err "Unknown track ID: $TRACK_ID"
  echo "Valid IDs:"
  jq -r '.tracks[].id' "$SCRIPT_DIR/tracks.json" | sed 's/^/  /'
  exit 1
fi

TRACK_TITLE=$(jq -r --arg id "$TRACK_ID" '.tracks[] | select(.id == $id) | .title' "$SCRIPT_DIR/tracks.json")

# Find source file
if [[ -n "$SUNO_FILE" ]]; then
  SRC="$DOWNLOADS_DIR/$SUNO_FILE"
  if [[ ! -f "$SRC" ]]; then
    # Try without directory prefix in case full path was given
    if [[ -f "$SUNO_FILE" ]]; then
      SRC="$SUNO_FILE"
    else
      err "File not found: $SRC"
      exit 1
    fi
  fi
else
  # Find most recent mp3 in Downloads (Suno downloads as .mp3)
  SRC=$(ls -t "$DOWNLOADS_DIR"/*.mp3 2>/dev/null | head -1)
  if [[ -z "$SRC" ]]; then
    err "No audio files found in $DOWNLOADS_DIR"
    exit 1
  fi
  log "Auto-detected: $(basename "$SRC")"
fi

DEST_NAME="${THEME_ID}-${TRACK_ID}.m4a"
DEST_FILE="$FINAL_DIR/$DEST_NAME"

log "Processing: $TRACK_TITLE"
log "  Source: $(basename "$SRC")"
log "  Output: $DEST_NAME"

# Convert to M4A with loudness normalization
if ! command -v ffmpeg >/dev/null 2>&1; then
  err "ffmpeg not found. Install with: brew install ffmpeg"
  exit 1
fi

log "  Converting to M4A (AAC-LC 96kbps, loudness normalized)..."
ffmpeg -y -i "$SRC" \
  -vn \
  -af "loudnorm=I=-16:TP=-1.5:LRA=11" \
  -c:a aac -b:a 96k -ar 44100 \
  -movflags +faststart \
  "$DEST_FILE" 2>/dev/null

if [[ -f "$DEST_FILE" ]]; then
  local_size=$(wc -c < "$DEST_FILE" | tr -d ' ')
  src_size=$(wc -c < "$SRC" | tr -d ' ')
  savings=$(( (src_size - local_size) * 100 / src_size ))
  ok "  Done: $DEST_NAME ($(numfmt --to=iec "$local_size" 2>/dev/null || echo "${local_size} bytes"), ${savings}% smaller)"
else
  err "  Conversion failed!"
  exit 1
fi

echo ""
ok "Track ready: /audio/music/$DEST_NAME"
