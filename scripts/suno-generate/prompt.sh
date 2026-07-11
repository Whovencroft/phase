#!/usr/bin/env bash
#
# Display Suno creation prompt for a track, ready to paste into suno.com.
# Optionally watches ~/Downloads for the result and auto-processes it.
#
# Usage:
#   ./prompt.sh <track-id>           # Show prompt only
#   ./prompt.sh <track-id> --watch   # Show prompt, then watch for download & auto-convert
#   ./prompt.sh --list               # List all tracks and their status
#   ./prompt.sh --next               # Show prompt for next ungenerated track

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TRACKS_FILE="$SCRIPT_DIR/tracks.json"
FINAL_DIR="$(cd "$SCRIPT_DIR/../../client/public/audio/music" && pwd)"
THEME_ID="planeswalker"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# ---------------------------------------------------------------------------
# --list: show all tracks with completion status
# ---------------------------------------------------------------------------

cmd_list() {
  local done=0 total=0

  echo ""
  echo -e "${BOLD}phase.rs Planeswalker Theme — Track Status${NC}"
  echo -e "${DIM}─────────────────────────────────────────────────${NC}"
  printf "  %-18s %-30s %-12s %s\n" "ID" "TITLE" "CONTEXT" "STATUS"
  echo -e "${DIM}─────────────────────────────────────────────────${NC}"

  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id title context phase
    id=$(echo "$track" | jq -r '.id')
    title=$(echo "$track" | jq -r '.title')
    context=$(echo "$track" | jq -r '.context')
    phase=$(echo "$track" | jq -r '.phase')

    local dest="$FINAL_DIR/${THEME_ID}-${id}.m4a"
    total=$((total + 1))

    if [[ -f "$dest" ]]; then
      local size
      size=$(ls -lh "$dest" | awk '{print $5}')
      printf "  %-18s %-30s %-12s ${GREEN}done${NC} (%s)\n" "$id" "$title" "$context/$phase" "$size"
      done=$((done + 1))
    else
      printf "  %-18s %-30s %-12s ${YELLOW}pending${NC}\n" "$id" "$title" "$context/$phase"
    fi
  done

  echo -e "${DIM}─────────────────────────────────────────────────${NC}"
  echo ""
}

# ---------------------------------------------------------------------------
# --next: find first ungenerated track
# ---------------------------------------------------------------------------

find_next_id() {
  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id
    id=$(echo "$track" | jq -r '.id')
    local dest="$FINAL_DIR/${THEME_ID}-${id}.m4a"
    if [[ ! -f "$dest" ]]; then
      echo "$id"
      return
    fi
  done
}

# ---------------------------------------------------------------------------
# Show prompt for a track
# ---------------------------------------------------------------------------

show_prompt() {
  local track_id="$1"

  local track
  track=$(jq -c --arg id "$track_id" '.tracks[] | select(.id == $id)' "$TRACKS_FILE")

  if [[ -z "$track" ]]; then
    echo -e "${RED}Unknown track ID: $track_id${NC}"
    echo "Run with --list to see all track IDs."
    exit 1
  fi

  local title style notes context phase negative_tags
  title=$(echo "$track" | jq -r '.title')
  style=$(echo "$track" | jq -r '.style')
  notes=$(echo "$track" | jq -r '.notes')
  context=$(echo "$track" | jq -r '.context')
  phase=$(echo "$track" | jq -r '.phase')
  negative_tags=$(jq -r '.globalDefaults.negativeTags // ""' "$TRACKS_FILE")

  # Check if already generated
  local dest="$FINAL_DIR/${THEME_ID}-${track_id}.m4a"
  if [[ -f "$dest" ]]; then
    echo -e "${YELLOW}Warning: $dest already exists. Will be overwritten.${NC}"
    echo ""
  fi

  # Build the full prompt block for Suno's Simple box
  local simple_prompt
  simple_prompt="Title: $title

$style

Notes: $notes"

  echo ""
  echo -e "${BOLD}${CYAN}━━━ Suno Prompt ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${DIM}Track: $track_id | Context: $context | Phase: $phase${NC}"
  echo ""
  echo "$simple_prompt"
  echo ""
  echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # Copy the full block to clipboard on macOS
  if command -v pbcopy >/dev/null 2>&1; then
    echo -n "$simple_prompt" | pbcopy
    echo -e "${GREEN}Copied to clipboard.${NC} Paste into Suno's Simple box."
    echo ""
  fi
}

# ---------------------------------------------------------------------------
# --watch: monitor ~/Downloads for new mp3 and auto-process
# ---------------------------------------------------------------------------

watch_download() {
  local track_id="$1"

  echo -e "${BLUE}Watching ~/Downloads for new audio files...${NC}"
  echo -e "${DIM}Generate the track on suno.com, then click Download.${NC}"
  echo -e "${DIM}Press Ctrl+C to cancel.${NC}"
  echo ""

  # Record existing files
  local before_files
  before_files=$(mktemp)
  ls -1 "$HOME/Downloads"/*.mp3 2>/dev/null | sort > "$before_files"

  # Poll for new file
  while true; do
    sleep 2
    local after_files
    after_files=$(mktemp)
    ls -1 "$HOME/Downloads"/*.mp3 2>/dev/null | sort > "$after_files"

    local new_file
    new_file=$(comm -13 "$before_files" "$after_files" | head -1)
    rm -f "$after_files"

    if [[ -n "$new_file" ]]; then
      echo ""
      echo -e "${GREEN}New file detected: $(basename "$new_file")${NC}"

      # Wait a moment for download to finish writing
      sleep 2

      # Check file is complete (size stable)
      local size1 size2
      size1=$(wc -c < "$new_file" 2>/dev/null || echo "0")
      sleep 1
      size2=$(wc -c < "$new_file" 2>/dev/null || echo "0")

      if [[ "$size1" != "$size2" ]]; then
        echo -e "${DIM}File still downloading, waiting...${NC}"
        sleep 5
      fi

      # Process it
      "$SCRIPT_DIR/process-download.sh" "$track_id" "$new_file"
      rm -f "$before_files"
      return 0
    fi
  done
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

case "${1:-}" in
  --list|-l)
    cmd_list
    exit 0
    ;;
  --next|-n)
    next_id=$(find_next_id)
    if [[ -z "$next_id" ]]; then
      echo -e "${GREEN}All tracks generated!${NC}"
      exit 0
    fi
    show_prompt "$next_id"
    if [[ "${2:-}" == "--watch" || "${2:-}" == "-w" ]]; then
      watch_download "$next_id"
    fi
    ;;
  --help|-h)
    head -12 "$0" | tail -10
    exit 0
    ;;
  "")
    echo "Usage: $0 <track-id> [--watch] | --list | --next [--watch]"
    exit 1
    ;;
  *)
    TRACK_ID="$1"
    show_prompt "$TRACK_ID"
    if [[ "${2:-}" == "--watch" || "${2:-}" == "-w" ]]; then
      watch_download "$TRACK_ID"
    fi
    ;;
esac
