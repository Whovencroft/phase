#!/usr/bin/env bash
#
# Suno API music generation pipeline for phase.rs
#
# Usage:
#   ./scripts/suno-generate/generate.sh [OPTIONS]
#
# Options:
#   --api-key KEY        Suno API key (or set SUNO_API_KEY env var)
#   --submit             Submit generation requests to Suno API
#   --status             Check status of pending generations
#   --download           Download completed tracks
#   --manifest           Generate the AudioThemeManifest JSON
#   --all                Run full pipeline: submit → poll → download → manifest
#   --track ID           Only process a specific track (by id from tracks.json)
#   --dry-run            Print API requests without sending
#   --pick               Interactive picker: listen & choose best variant per track
#
# Requirements:
#   - jq (JSON processing)
#   - curl (API calls)
#   - ffmpeg (optional, for normalization/conversion)
#
# The script reads tracks.json for prompt definitions and writes state to
# .state.json (gitignored) to track generation progress across runs.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TRACKS_FILE="$SCRIPT_DIR/tracks.json"
STATE_FILE="$SCRIPT_DIR/.state.json"
OUTPUT_DIR="$SCRIPT_DIR/output"
FINAL_DIR="$(cd "$SCRIPT_DIR/../../client/public/audio/music" && pwd)"
MANIFEST_OUT="$SCRIPT_DIR/planeswalker-theme.json"

API_BASE="https://api.sunoapi.org/api/v1"
POLL_INTERVAL=30  # seconds between status checks
MAX_POLLS=40      # ~20 minutes max wait

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

log()  { echo -e "${BLUE}[suno]${NC} $*"; }
ok()   { echo -e "${GREEN}[suno]${NC} $*"; }
warn() { echo -e "${YELLOW}[suno]${NC} $*"; }
err()  { echo -e "${RED}[suno]${NC} $*" >&2; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { err "Required command not found: $1"; exit 1; }
}

require_api_key() {
  if [[ -z "${SUNO_API_KEY:-}" ]]; then
    err "SUNO_API_KEY not set. Pass --api-key KEY or export SUNO_API_KEY"
    exit 1
  fi
}

init_state() {
  if [[ ! -f "$STATE_FILE" ]]; then
    echo '{"tasks":{}}' > "$STATE_FILE"
  fi
  mkdir -p "$OUTPUT_DIR"
}

get_state() {
  jq -r "$1" "$STATE_FILE"
}

set_state() {
  local tmp
  tmp=$(mktemp)
  jq "$1" "$STATE_FILE" > "$tmp" && mv "$tmp" "$STATE_FILE"
}

# ---------------------------------------------------------------------------
# API functions
# ---------------------------------------------------------------------------

api_generate() {
  local title="$1" style="$2" negative_tags="$3" model="$4"

  local body
  body=$(jq -n \
    --arg title "$title" \
    --arg style "$style" \
    --arg neg "$negative_tags" \
    --arg model "$model" \
    '{
      customMode: true,
      instrumental: true,
      model: $model,
      title: $title,
      style: $style,
      negativeTags: $neg
    }')

  if [[ "${DRY_RUN:-false}" == "true" ]]; then
    log "DRY RUN: POST $API_BASE/generate"
    echo "$body" | jq .
    echo '{"code":200,"data":{"taskId":"dry-run-'$(date +%s)'"}}'
    return
  fi

  curl -s -X POST "$API_BASE/generate" \
    -H "Authorization: Bearer $SUNO_API_KEY" \
    -H "Content-Type: application/json" \
    -d "$body"
}

api_status() {
  local task_id="$1"

  curl -s -X GET "$API_BASE/query?taskId=$task_id" \
    -H "Authorization: Bearer $SUNO_API_KEY"
}

# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

cmd_submit() {
  require_api_key
  init_state

  local model negative_tags track_filter="${TRACK_FILTER:-}"
  model=$(jq -r '.model' "$TRACKS_FILE")
  negative_tags=$(jq -r '.globalDefaults.negativeTags' "$TRACKS_FILE")

  local count=0
  local total
  total=$(jq '.tracks | length' "$TRACKS_FILE")

  log "Submitting $total tracks to Suno API (model: $model)..."
  echo ""

  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id title style
    id=$(echo "$track" | jq -r '.id')
    title=$(echo "$track" | jq -r '.title')
    style=$(echo "$track" | jq -r '.style')

    # Skip if filtered
    if [[ -n "$track_filter" && "$id" != "$track_filter" ]]; then
      continue
    fi

    # Skip if already submitted
    local existing_task
    existing_task=$(get_state ".tasks[\"$id\"].taskId // empty")
    if [[ -n "$existing_task" ]]; then
      warn "  Skip $id — already submitted (task: $existing_task)"
      continue
    fi

    log "  Submitting: $title"
    log "    Style: ${style:0:80}..."

    local response
    response=$(api_generate "$title" "$style" "$negative_tags" "$model")

    local code task_id
    code=$(echo "$response" | jq -r '.code // 0')
    task_id=$(echo "$response" | jq -r '.data.taskId // empty')

    if [[ "$code" == "200" && -n "$task_id" ]]; then
      ok "    -> Task ID: $task_id"
      set_state ".tasks[\"$id\"] = {taskId: \"$task_id\", status: \"pending\", submitted: \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}"
      count=$((count + 1))
    else
      err "    -> Failed: $response"
      set_state ".tasks[\"$id\"] = {taskId: null, status: \"error\", error: $(echo "$response" | jq -c .)}"
    fi

    # Rate limit: max 20 requests per 10 seconds. We do 1 per second to be safe.
    sleep 1
  done

  echo ""
  ok "Submitted $count generation requests."
  log "Run with --status to check progress, or --download when ready."
}

cmd_status() {
  require_api_key
  init_state

  local pending=0 complete=0 errors=0

  log "Checking generation status..."
  echo ""

  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id title
    id=$(echo "$track" | jq -r '.id')
    title=$(echo "$track" | jq -r '.title')

    local task_id status
    task_id=$(get_state ".tasks[\"$id\"].taskId // empty")
    status=$(get_state ".tasks[\"$id\"].status // \"not_submitted\"")

    if [[ -z "$task_id" || "$status" == "not_submitted" ]]; then
      echo -e "  ${YELLOW}NOT SUBMITTED${NC}  $id ($title)"
      continue
    fi

    if [[ "$status" == "complete" ]]; then
      echo -e "  ${GREEN}COMPLETE${NC}       $id ($title)"
      complete=$((complete + 1))
      continue
    fi

    # Poll API for current status
    local response
    response=$(api_status "$task_id")
    local callback_type
    callback_type=$(echo "$response" | jq -r '.data.callbackType // "pending"')

    if [[ "$callback_type" == "complete" ]]; then
      echo -e "  ${GREEN}COMPLETE${NC}       $id ($title)"

      # Save audio URLs to state
      local audio_data
      audio_data=$(echo "$response" | jq -c '.data.data // []')
      set_state ".tasks[\"$id\"].status = \"complete\" | .tasks[\"$id\"].results = $audio_data"
      complete=$((complete + 1))
    elif [[ "$callback_type" == "first" ]]; then
      echo -e "  ${CYAN}STREAMING${NC}      $id ($title)"
      pending=$((pending + 1))
    else
      echo -e "  ${YELLOW}PENDING${NC}        $id ($title)"
      pending=$((pending + 1))
    fi

    sleep 0.5  # Don't hammer the API
  done

  echo ""
  log "Complete: $complete | Pending: $pending | Errors: $errors"
}

cmd_poll() {
  require_api_key
  init_state

  log "Polling until all tracks complete (max ${MAX_POLLS} checks, ${POLL_INTERVAL}s interval)..."

  for ((i=1; i<=MAX_POLLS; i++)); do
    local all_done=true

    jq -r '.tasks | to_entries[] | select(.value.status != "complete" and .value.status != "error" and .value.taskId != null) | .key' "$STATE_FILE" | while read -r id; do
      local task_id
      task_id=$(get_state ".tasks[\"$id\"].taskId")

      local response callback_type
      response=$(api_status "$task_id")
      callback_type=$(echo "$response" | jq -r '.data.callbackType // "pending"')

      if [[ "$callback_type" == "complete" ]]; then
        local audio_data
        audio_data=$(echo "$response" | jq -c '.data.data // []')
        set_state ".tasks[\"$id\"].status = \"complete\" | .tasks[\"$id\"].results = $audio_data"
        ok "  Complete: $id"
      else
        all_done=false
      fi

      sleep 0.5
    done

    local remaining
    remaining=$(jq '[.tasks | to_entries[] | select(.value.status != "complete" and .value.status != "error" and .value.taskId != null)] | length' "$STATE_FILE")

    if [[ "$remaining" == "0" ]]; then
      ok "All tracks complete!"
      return 0
    fi

    log "Poll $i/$MAX_POLLS — $remaining tracks remaining. Waiting ${POLL_INTERVAL}s..."
    sleep "$POLL_INTERVAL"
  done

  warn "Timed out with tracks still pending. Run --status to check."
}

cmd_download() {
  init_state
  mkdir -p "$OUTPUT_DIR"

  log "Downloading completed tracks..."
  echo ""

  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id
    id=$(echo "$track" | jq -r '.id')

    local status
    status=$(get_state ".tasks[\"$id\"].status // \"not_submitted\"")

    if [[ "$status" != "complete" ]]; then
      warn "  Skip $id — status: $status"
      continue
    fi

    # Each Suno request returns 2 variants
    local num_results
    num_results=$(get_state ".tasks[\"$id\"].results | length")

    for ((v=0; v<num_results; v++)); do
      local audio_url audio_id
      audio_url=$(get_state ".tasks[\"$id\"].results[$v].audio_url")
      audio_id=$(get_state ".tasks[\"$id\"].results[$v].id")

      local filename="${id}_v${v}.mp3"
      local filepath="$OUTPUT_DIR/$filename"

      if [[ -f "$filepath" ]]; then
        log "  Already downloaded: $filename"
        continue
      fi

      log "  Downloading: $filename"
      curl -sL "$audio_url" -o "$filepath"

      if [[ -f "$filepath" ]]; then
        local size
        size=$(wc -c < "$filepath" | tr -d ' ')
        ok "    -> $filename (${size} bytes)"
      else
        err "    -> Failed to download $filename"
      fi
    done
  done

  echo ""
  ok "Downloads complete. Files in: $OUTPUT_DIR"
  log "Use --pick to interactively select best variants, or --manifest to auto-pick v0."
}

cmd_pick() {
  init_state

  log "Interactive variant picker"
  log "For each track, you'll choose which variant (v0 or v1) to use."
  echo ""

  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id title context phase
    id=$(echo "$track" | jq -r '.id')
    title=$(echo "$track" | jq -r '.title')
    context=$(echo "$track" | jq -r '.context')
    phase=$(echo "$track" | jq -r '.phase')

    local v0="$OUTPUT_DIR/${id}_v0.mp3"
    local v1="$OUTPUT_DIR/${id}_v1.mp3"

    if [[ ! -f "$v0" ]]; then
      warn "  Skip $id — not downloaded yet"
      continue
    fi

    echo ""
    echo -e "${CYAN}--- $title ---${NC}"
    echo -e "  Context: $context | Phase: $phase"
    echo -e "  [0] ${id}_v0.mp3"
    [[ -f "$v1" ]] && echo -e "  [1] ${id}_v1.mp3"
    echo -e "  [s] Skip (keep current selection)"

    # If on macOS, offer playback
    if command -v afplay >/dev/null 2>&1; then
      echo -e "  [p0] Play v0  |  [p1] Play v1"
    fi

    while true; do
      read -rp "  Choice: " choice
      case "$choice" in
        0)  set_state ".tasks[\"$id\"].picked = 0"; ok "  -> Selected v0"; break ;;
        1)  set_state ".tasks[\"$id\"].picked = 1"; ok "  -> Selected v1"; break ;;
        s)  log "  -> Skipped"; break ;;
        p0) afplay "$v0" &
            local pid=$!
            read -rp "  (Press Enter to stop)" _
            kill "$pid" 2>/dev/null || true
            ;;
        p1) [[ -f "$v1" ]] && { afplay "$v1" & local pid=$!; read -rp "  (Press Enter to stop)" _; kill "$pid" 2>/dev/null || true; } || warn "  v1 not available" ;;
        *)  warn "  Invalid choice. Enter 0, 1, s, p0, or p1" ;;
      esac
    done
  done

  echo ""
  ok "Picks saved to .state.json"
  log "Run --manifest to generate theme with your selections."
}

cmd_manifest() {
  init_state

  log "Generating AudioThemeManifest..."

  # Copy picked files to final directory and build manifest
  local theme_id theme_name theme_version theme_author theme_desc
  theme_id=$(jq -r '.theme.id' "$TRACKS_FILE")
  theme_name=$(jq -r '.theme.name' "$TRACKS_FILE")
  theme_version=$(jq -r '.theme.version' "$TRACKS_FILE")
  theme_author=$(jq -r '.theme.author' "$TRACKS_FILE")
  theme_desc=$(jq -r '.theme.description' "$TRACKS_FILE")

  # Start building manifest
  local manifest
  manifest=$(jq -n \
    --arg id "$theme_id" \
    --arg name "$theme_name" \
    --argjson version "$theme_version" \
    --arg author "$theme_author" \
    --arg desc "$theme_desc" \
    '{
      id: $id,
      name: $name,
      version: $version,
      author: $author,
      description: $desc,
      phaseBreakpoints: { mid: 5, late: 10 },
      sfx: [],
      music: {
        menu: [],
        deck_builder: [],
        lobby: [],
        battlefield: [],
        victory: [],
        defeat: []
      }
    }')

  # Process each track
  jq -c '.tracks[]' "$TRACKS_FILE" | while read -r track; do
    local id context phase label
    id=$(echo "$track" | jq -r '.id')
    context=$(echo "$track" | jq -r '.context')
    phase=$(echo "$track" | jq -r '.phase')
    label=$(echo "$track" | jq -r '.label')

    # Determine which variant to use (picked or default to 0)
    local picked
    picked=$(get_state ".tasks[\"$id\"].picked // 0")

    local src_file="$OUTPUT_DIR/${id}_v${picked}.mp3"
    local dest_name="${theme_id}-${id}.m4a"
    local dest_file="$FINAL_DIR/$dest_name"
    local url="/audio/music/$dest_name"

    if [[ ! -f "$src_file" ]]; then
      warn "  Skip $id — file not found: $src_file"
      continue
    fi

    # Convert to M4A (AAC-LC 96kbps) with loudness normalization
    if command -v ffmpeg >/dev/null 2>&1; then
      log "  Converting to M4A: $dest_name"
      ffmpeg -y -i "$src_file" \
        -vn \
        -af "loudnorm=I=-16:TP=-1.5:LRA=11" \
        -c:a aac -b:a 96k -ar 44100 \
        -movflags +faststart \
        "$dest_file" 2>/dev/null
    else
      # Fallback: copy as MP3 if ffmpeg not available
      dest_name="${theme_id}-${id}.mp3"
      dest_file="$FINAL_DIR/$dest_name"
      url="/audio/music/$dest_name"
      warn "  ffmpeg not found — copying as MP3 (install ffmpeg for M4A conversion)"
      cp "$src_file" "$dest_file"
    fi

    # Build track entry
    local track_entry
    track_entry=$(jq -n \
      --arg id "$id" \
      --arg url "$url" \
      --arg phase "$phase" \
      --arg label "$label" \
      '{id: $id, url: $url, phase: $phase, label: $label}')

    # Add to manifest (write to temp file since we're in a subshell)
    echo "$context|$track_entry" >> "$OUTPUT_DIR/.manifest_tracks"
  done

  # Assemble final manifest from collected tracks
  if [[ -f "$OUTPUT_DIR/.manifest_tracks" ]]; then
    while IFS='|' read -r context entry; do
      manifest=$(echo "$manifest" | jq --arg ctx "$context" --argjson entry "$entry" \
        '.music[$ctx] += [$entry]')
    done < "$OUTPUT_DIR/.manifest_tracks"
    rm "$OUTPUT_DIR/.manifest_tracks"
  fi

  # Copy SFX mapping from classic theme (reuse existing SFX)
  local sfx_entries
  sfx_entries=$(jq -c '[
    {eventType: "DamageDealt", url: "/audio/sfx/destroy.mp3"},
    {eventType: "LifeChanged", url: "/audio/sfx/life_loss.mp3"},
    {eventType: "SpellCast", url: "/audio/sfx/instant.mp3"},
    {eventType: "CreatureDestroyed", url: "/audio/sfx/destroy.mp3"},
    {eventType: "AttackersDeclared", url: "/audio/sfx/creature.mp3"},
    {eventType: "BlockersDeclared", url: "/audio/sfx/block.mp3"},
    {eventType: "LandPlayed", url: "/audio/sfx/green_land.mp3"},
    {eventType: "CardDrawn", url: "/audio/sfx/draw.mp3"},
    {eventType: "SpellCountered", url: "/audio/sfx/sorcery.mp3"},
    {eventType: "TokenCreated", url: "/audio/sfx/token.mp3"},
    {eventType: "GameStarted", url: "/audio/sfx/shuffle.mp3"},
    {eventType: "PermanentSacrificed", url: "/audio/sfx/destroy.mp3"},
    {eventType: "CounterAdded", url: "/audio/sfx/add_counter.mp3"},
    {eventType: "AbilityActivated", url: "/audio/sfx/enchant.mp3"}
  ]' <<< '{}')

  manifest=$(echo "$manifest" | jq --argjson sfx "$sfx_entries" '.sfx = $sfx')

  # Write manifest
  echo "$manifest" | jq '.' > "$MANIFEST_OUT"

  ok "Manifest written to: $MANIFEST_OUT"
  echo ""
  log "Files placed in: $FINAL_DIR"
  log "To register as a built-in theme, run with --register"
  echo ""
  echo "Theme summary:"
  echo "$manifest" | jq '{
    id: .id,
    name: .name,
    tracks: (.music | to_entries | map({context: .key, count: (.value | length)}) | from_entries),
    total_tracks: ([.music[]] | flatten | length)
  }'
}

cmd_register() {
  # Output instructions for wiring into themeRegistry.ts
  if [[ ! -f "$MANIFEST_OUT" ]]; then
    err "No manifest found. Run --manifest first."
    exit 1
  fi

  log "To register the Planeswalker theme as a built-in preset:"
  echo ""
  echo "The manifest has been generated at:"
  echo "  $MANIFEST_OUT"
  echo ""
  echo "Ask Claude to wire it into the themeRegistry.ts as a built-in theme"
  echo "alongside CLASSIC_THEME, or import it via the Settings > Audio > Import Theme UI."
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

ACTION=""
DRY_RUN=false
TRACK_FILTER=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-key)    SUNO_API_KEY="$2"; shift 2 ;;
    --submit)     ACTION="submit"; shift ;;
    --status)     ACTION="status"; shift ;;
    --poll)       ACTION="poll"; shift ;;
    --download)   ACTION="download"; shift ;;
    --pick)       ACTION="pick"; shift ;;
    --manifest)   ACTION="manifest"; shift ;;
    --register)   ACTION="register"; shift ;;
    --all)        ACTION="all"; shift ;;
    --track)      TRACK_FILTER="$2"; shift 2 ;;
    --dry-run)    DRY_RUN=true; shift ;;
    -h|--help)
      head -20 "$0" | tail -18
      exit 0
      ;;
    *)
      err "Unknown option: $1"
      exit 1
      ;;
  esac
done

require_cmd jq
require_cmd curl

export SUNO_API_KEY="${SUNO_API_KEY:-}"
export DRY_RUN
export TRACK_FILTER

case "$ACTION" in
  submit)   cmd_submit ;;
  status)   cmd_status ;;
  poll)     cmd_poll ;;
  download) cmd_download ;;
  pick)     cmd_pick ;;
  manifest) cmd_manifest ;;
  register) cmd_register ;;
  all)
    cmd_submit
    echo ""
    cmd_poll
    echo ""
    cmd_download
    echo ""
    cmd_manifest
    ;;
  *)
    echo "Usage: $0 [--submit|--status|--poll|--download|--pick|--manifest|--all]"
    echo "       Run with --help for full options."
    exit 1
    ;;
esac
