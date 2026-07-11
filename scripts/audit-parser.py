#!/usr/bin/env python3
"""
Parser Correctness Audit for card-data.json

Checks all "supported" cards (no Unimplemented/Unknown markers) for semantic
parser bugs: missing keywords, wrong effect types, missing triggers, missing
targets, empty ability chains, etc.

Usage: python3 scripts/audit-parser.py [--json] [--limit N]
"""

import json
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field

# ── Known evergreen/common keywords (lowercase) ──
KNOWN_KEYWORDS = {
    'flying', 'first strike', 'double strike', 'deathtouch', 'haste',
    'hexproof', 'indestructible', 'lifelink', 'menace', 'reach',
    'trample', 'vigilance', 'defender', 'flash', 'shroud', 'fear',
    'intimidate', 'skulk', 'shadow', 'horsemanship', 'flanking',
    'banding', 'wither', 'infect', 'prowess', 'devoid', 'changeling',
    'phasing', 'persist', 'undying', 'riot', 'decayed', 'exploit',
    'exalted', 'extort', 'evolve', 'dethrone', 'melee', 'mentor',
    'myriad', 'provoke', 'soulbond', 'unleash', 'convoke', 'delve',
    'improvise', 'cascade', 'rebound', 'storm', 'epic', 'retrace',
    'haunt', 'living weapon', 'living metal', 'split second', 'fuse',
    'ascend', 'explore', 'ravenous', 'enlist', 'read ahead',
    'demonstrate', 'gravestorm', 'conspire', 'ingest', 'ripple',
    'nightbound', 'daybound', 'spree', 'suspend',
}

# Keywords that appear as plain text lines in Oracle text
KEYWORD_LINE_PATTERNS = {
    'flying', 'first strike', 'double strike', 'deathtouch', 'haste',
    'hexproof', 'indestructible', 'lifelink', 'menace', 'reach',
    'trample', 'vigilance', 'defender', 'flash', 'shroud', 'fear',
    'intimidate', 'skulk', 'shadow', 'horsemanship', 'flanking',
    'banding', 'wither', 'infect', 'prowess', 'devoid', 'changeling',
    'phasing', 'persist', 'undying', 'riot', 'decayed', 'exploit',
    'exalted', 'extort', 'evolve', 'dethrone', 'melee', 'mentor',
    'myriad', 'provoke', 'soulbond', 'unleash', 'ascend',
}

# Effect type to Oracle text verb mapping for cross-check
EFFECT_ORACLE_MAPPING = {
    'Destroy': ['destroy'],
    'DealDamage': ['deals', 'damage'],
    'Draw': ['draw'],
    'Discard': ['discard'],
    'GainLife': ['gain', 'life'],
    'LoseLife': ['lose', 'life'],
    'Bounce': ['return', "owner's hand", "to its owner's hand", "to their owner's hand"],
    'Sacrifice': ['sacrifice'],
    'Exile': ['exile'],
    'Shuffle': ['shuffle'],
    'Mill': ['mill'],
    'Scry': ['scry'],
    'Surveil': ['surveil'],
    'Proliferate': ['proliferate'],
}

# Trigger mode to Oracle text patterns
TRIGGER_ORACLE_MAPPING = {
    'ChangesZone': [r'\b(enters|leaves|dies|put into .* graveyard)\b'],
    'Phase': [r'\b(at the beginning of|at end of)\b'],
    'Attacks': [r'\b(attacks|attacked)\b'],
    'SpellCast': [r'\b(cast|casts)\b'],
    'DamageDone': [r'\b(deals? .*damage)\b'],
    'Blocks': [r'\b(blocks|blocked)\b'],
}


@dataclass
class Finding:
    card_name: str
    check: str
    severity: str  # 'error', 'warning', 'info'
    detail: str
    oracle_excerpt: str = ""


@dataclass
class AuditReport:
    total_supported: int = 0
    total_checked: int = 0
    findings: list = field(default_factory=list)
    check_counts: Counter = field(default_factory=Counter)

    def add(self, finding: Finding):
        self.findings.append(finding)
        self.check_counts[finding.check] += 1


def has_unimplemented(obj):
    """Check if JSON tree contains Unimplemented or Unknown markers."""
    if isinstance(obj, dict):
        if obj.get('type') == 'Unimplemented':
            return True
        if obj.get('mode') == 'Unknown':
            return True
        return any(has_unimplemented(v) for v in obj.values())
    elif isinstance(obj, list):
        return any(has_unimplemented(item) for item in obj)
    return False


def collect_effect_types(obj, types=None):
    """Recursively collect all effect type strings from a JSON tree."""
    if types is None:
        types = set()
    if isinstance(obj, dict):
        t = obj.get('type')
        if isinstance(t, str) and t[0].isupper():
            types.add(t)
        for v in obj.values():
            collect_effect_types(v, types)
    elif isinstance(obj, list):
        for item in obj:
            collect_effect_types(item, types)
    return types


def get_keyword_names(keywords):
    """Extract keyword names from the keywords array (handles str and dict)."""
    names = set()
    for kw in keywords:
        if isinstance(kw, str):
            names.add(kw.lower().replace('_', ' '))
        elif isinstance(kw, dict):
            for k in kw:
                names.add(k.lower().replace('_', ' '))
    return names


def strip_reminder_text(text):
    """Remove reminder text (parenthesized) from Oracle text."""
    return re.sub(r'\([^)]*\)', '', text).strip()


def get_oracle_lines(oracle_text):
    """Split Oracle text into non-empty lines with reminder text stripped."""
    if not oracle_text:
        return []
    lines = []
    for line in oracle_text.split('\n'):
        cleaned = strip_reminder_text(line).strip()
        if cleaned:
            lines.append(cleaned)
    return lines


# ── Audit Checks ──

def check_missing_keywords(card, report):
    """Check if Oracle text contains keyword lines that aren't in the keywords array."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    parsed_kws = get_keyword_names(card.get('keywords', []))
    lines = get_oracle_lines(oracle)

    for line in lines:
        lower = line.lower().strip()
        # Strip trailing comma (keyword lists like "Flying, haste")
        for part in re.split(r',\s*', lower):
            part = part.strip()
            # Also handle "keyword N" parameterized keywords
            base = re.sub(r'\s+\d+$', '', part)
            if base in KEYWORD_LINE_PATTERNS and base not in parsed_kws:
                # Check if it might be part of a sentence rather than a keyword line
                if len(line.split()) <= 4 or line.lower() == part:
                    report.add(Finding(
                        card_name=card['name'],
                        check='missing_keyword',
                        severity='error',
                        detail=f"Oracle has keyword '{part}' but not in keywords array. Parsed: {sorted(parsed_kws)}",
                        oracle_excerpt=line,
                    ))


def check_trigger_presence(card, report):
    """Check if Oracle text has trigger words but no triggers were parsed."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    triggers = card.get('triggers', [])
    lower = oracle.lower()

    # Trigger indicators
    has_when = bool(re.search(r'\bwhen(ever)?\b', lower))
    has_at_beginning = bool(re.search(r'\bat the beginning of\b', lower))

    if (has_when or has_at_beginning) and len(triggers) == 0:
        # Exceptions: "when" in reminder text, "when you do", or in choice text
        stripped = strip_reminder_text(oracle).lower()
        if re.search(r'\bwhen(ever)?\b', stripped) or re.search(r'\bat the beginning of\b', stripped):
            # Also check static abilities and replacements - some "when" patterns are replacements
            replacements = card.get('replacements', [])
            statics = card.get('static_abilities', [])
            if not replacements and not statics:
                report.add(Finding(
                    card_name=card['name'],
                    check='missing_trigger',
                    severity='warning',
                    detail=f"Oracle has trigger words but no triggers parsed. Has {len(card.get('abilities', []))} abilities, {len(statics)} statics, {len(replacements)} replacements.",
                    oracle_excerpt=oracle[:200],
                ))


def check_activated_ability_presence(card, report):
    """Check if Oracle text has {cost}: {effect} patterns but no abilities parsed."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return

    # Look for activated ability patterns: {T}, {mana}: effect
    # Pattern: line starts with cost indicators followed by colon
    lines = oracle.split('\n')
    activated_lines = 0
    for line in lines:
        stripped = strip_reminder_text(line).strip()
        if not stripped:
            continue
        # Activated ability pattern: contains {X}: or "Pay N life:" etc.
        if re.search(r'(\{[WUBRGCTXSE\d/]+\}.*:|[Tt]ap.*:)', stripped):
            # But not if it's a keyword definition line like "Channel — {2}{U}, Discard ..."
            if not re.match(r'^[A-Z][a-z]+ —', stripped):
                activated_lines += 1

    abilities = card.get('abilities', [])
    # Count actual activated abilities (those with costs)
    activated_parsed = sum(1 for ab in abilities if ab.get('cost') is not None)

    if activated_lines > 0 and activated_parsed == 0 and len(abilities) == 0:
        report.add(Finding(
            card_name=card['name'],
            check='missing_activated_ability',
            severity='warning',
            detail=f"Oracle has {activated_lines} activated ability line(s) but no abilities parsed.",
            oracle_excerpt=oracle[:200],
        ))


def check_effect_verb_mismatch(card, report):
    """Check if parsed effects make sense given Oracle text verbs."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    lower = oracle.lower()
    stripped = strip_reminder_text(lower)

    # Collect all effect types from all parts of the card
    all_effects = set()
    for ab in card.get('abilities', []):
        collect_effect_types(ab, all_effects)
    for tr in card.get('triggers', []):
        collect_effect_types(tr, all_effects)
    for sa in card.get('static_abilities', []):
        collect_effect_types(sa, all_effects)
    for rp in card.get('replacements', []):
        collect_effect_types(rp, all_effects)

    # Check: Oracle says "destroy" but no Destroy effect
    if re.search(r'\bdestroy\b', stripped) and 'Destroy' not in all_effects and 'DestroyAll' not in all_effects:
        # Exclude "can't be destroyed", "destroy target" in reminder text, etc.
        if re.search(r'\bdestroy (target|all|each|a )', stripped):
            report.add(Finding(
                card_name=card['name'],
                check='effect_verb_mismatch',
                severity='warning',
                detail=f"Oracle says 'destroy' but no Destroy/DestroyAll effect found. Effects: {sorted(all_effects)[:10]}",
                oracle_excerpt=oracle[:200],
            ))

    # Check: Oracle says "exile" as a verb but no Exile/ChangeZone effect
    if re.search(r'\bexile (target|all|each|a |it\b|them\b)', stripped):
        if 'ChangeZone' not in all_effects and 'Exile' not in all_effects:
            report.add(Finding(
                card_name=card['name'],
                check='effect_verb_mismatch',
                severity='warning',
                detail=f"Oracle says 'exile' but no ChangeZone/Exile effect found. Effects: {sorted(all_effects)[:10]}",
                oracle_excerpt=oracle[:200],
            ))


def check_empty_ability_chains(card, report):
    """Check for abilities/triggers with null or missing effects."""
    for i, ab in enumerate(card.get('abilities', [])):
        if ab.get('effect') is None:
            report.add(Finding(
                card_name=card['name'],
                check='empty_ability',
                severity='error',
                detail=f"Ability {i} has null effect.",
            ))
    for i, tr in enumerate(card.get('triggers', [])):
        execute = tr.get('execute')
        if execute is None:
            report.add(Finding(
                card_name=card['name'],
                check='empty_trigger',
                severity='error',
                detail=f"Trigger {i} (mode={tr.get('mode')}) has null execute.",
            ))
        elif execute.get('effect') is None:
            report.add(Finding(
                card_name=card['name'],
                check='empty_trigger_effect',
                severity='error',
                detail=f"Trigger {i} (mode={tr.get('mode')}) execute has null effect.",
            ))


def check_target_presence(card, report):
    """Check if Oracle text says 'target' but no targeting found in parsed data."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    stripped = strip_reminder_text(oracle).lower()

    # Count "target" mentions outside reminder text
    target_matches = re.findall(r'\btarget\b', stripped)
    if not target_matches:
        return

    # Check if any parsed ability/trigger has targeting
    has_target = False

    def find_target(obj):
        nonlocal has_target
        if has_target:
            return
        if isinstance(obj, dict):
            # Check for target fields
            if 'target' in obj and obj['target'] is not None:
                t = obj['target']
                if isinstance(t, dict) and t.get('type') not in (None, 'None'):
                    has_target = True
                    return
            if 'target_prompt' in obj and obj['target_prompt']:
                has_target = True
                return
            for v in obj.values():
                find_target(v)
        elif isinstance(obj, list):
            for item in obj:
                find_target(item)

    for ab in card.get('abilities', []):
        find_target(ab)
    for tr in card.get('triggers', []):
        find_target(tr)
    for sa in card.get('static_abilities', []):
        find_target(sa)

    if not has_target and len(target_matches) >= 1:
        # Exclude cards where "target" is only in "becomes the target" or similar
        if re.search(r'\btarget (creature|player|opponent|artifact|enchantment|permanent|spell|land|planeswalker|card|nonland|nonblack|nonwhite|nonblue|nonred|nongreen)\b', stripped):
            report.add(Finding(
                card_name=card['name'],
                check='missing_target',
                severity='warning',
                detail=f"Oracle has {len(target_matches)} 'target' mention(s) but no targeting found in parsed data.",
                oracle_excerpt=oracle[:200],
            ))


def check_oracle_line_count(card, report):
    """Check for significant mismatch between Oracle lines and parsed items."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return

    lines = get_oracle_lines(oracle)
    # Filter out pure keyword lines and flavor lines
    ability_lines = []
    for line in lines:
        lower = line.lower().strip()
        # Skip pure keyword lines
        parts = [p.strip() for p in lower.split(',')]
        if all(p in KNOWN_KEYWORDS or re.match(r'^[a-z]+ \d+$', p) for p in parts if p):
            continue
        ability_lines.append(line)

    parsed_count = (
        len(card.get('abilities', []))
        + len(card.get('triggers', []))
        + len(card.get('static_abilities', []))
        + len(card.get('replacements', []))
    )

    # Flag if we have significantly more Oracle lines than parsed items
    if len(ability_lines) > 0 and parsed_count == 0:
        report.add(Finding(
            card_name=card['name'],
            check='zero_parse_items',
            severity='error',
            detail=f"Oracle has {len(ability_lines)} ability line(s) but 0 parsed items (abilities/triggers/statics/replacements).",
            oracle_excerpt=oracle[:200],
        ))
    elif len(ability_lines) >= 3 and parsed_count == 1:
        report.add(Finding(
            card_name=card['name'],
            check='severe_line_mismatch',
            severity='warning',
            detail=f"Oracle has {len(ability_lines)} ability lines but only {parsed_count} parsed item(s). Possible silent drop.",
            oracle_excerpt=oracle[:200],
        ))


DAMAGE_EFFECT_TYPES = {'DealDamage', 'DamageAll', 'DamageEachPlayer'}


def find_damage_info(obj, amounts=None, has_damage=None):
    """Recursively find damage effects and their Fixed amounts."""
    if amounts is None:
        amounts = []
    if has_damage is None:
        has_damage = [False]
    if isinstance(obj, dict):
        t = obj.get('type')
        if t in DAMAGE_EFFECT_TYPES:
            has_damage[0] = True
            amt = obj.get('amount', {})
            if isinstance(amt, dict) and amt.get('type') == 'Fixed':
                amounts.append(amt.get('value'))
        for v in obj.values():
            find_damage_info(v, amounts, has_damage)
    elif isinstance(obj, list):
        for item in obj:
            find_damage_info(item, amounts, has_damage)
    return amounts, has_damage[0]


def check_damage_amount(card, report):
    """Check that damage effects have sensible amounts matching Oracle text."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    stripped = strip_reminder_text(oracle).lower()

    # Find "deals N damage" in Oracle
    damage_matches = re.findall(r'deals? (\d+) damage', stripped)
    if not damage_matches:
        return

    # Collect all damage info from all parsed structures
    all_amounts = []
    any_damage_effect = False
    for section in ('abilities', 'triggers', 'static_abilities', 'replacements'):
        for item in card.get(section, []):
            amounts, has = find_damage_info(item)
            all_amounts.extend(amounts)
            any_damage_effect = any_damage_effect or has

    oracle_amounts = [int(x) for x in damage_matches]

    if not any_damage_effect:
        report.add(Finding(
            card_name=card['name'],
            check='damage_no_effect',
            severity='warning',
            detail=f"Oracle says deals {oracle_amounts} damage but no DealDamage/DamageAll/DamageEachPlayer effect found.",
            oracle_excerpt=oracle[:200],
        ))
    else:
        for oa in oracle_amounts:
            if oa not in all_amounts and all_amounts:
                report.add(Finding(
                    card_name=card['name'],
                    check='damage_amount_mismatch',
                    severity='warning',
                    detail=f"Oracle says deals {oa} damage but parsed amounts are {all_amounts}.",
                    oracle_excerpt=oracle[:200],
                ))


def check_draw_amount(card, report):
    """Check that Draw effects have correct amounts."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    stripped = strip_reminder_text(oracle).lower()

    draw_matches = re.findall(r'draw (\w+) cards?', stripped)
    if not draw_matches:
        return

    word_to_num = {'a': 1, 'one': 1, 'two': 2, 'three': 3, 'four': 4, 'five': 5,
                   'six': 6, 'seven': 7, 'eight': 8, 'nine': 9, 'ten': 10}

    def find_draw_amounts(obj, amounts=None):
        if amounts is None:
            amounts = []
        if isinstance(obj, dict):
            if obj.get('type') == 'Draw':
                amt = obj.get('amount') or obj.get('count')
                if isinstance(amt, dict) and amt.get('type') == 'Fixed':
                    amounts.append(amt.get('value'))
                elif isinstance(amt, int):
                    amounts.append(amt)
            for v in obj.values():
                find_draw_amounts(v, amounts)
        elif isinstance(obj, list):
            for item in obj:
                find_draw_amounts(item, amounts)
        return amounts

    parsed_amounts = []
    for ab in card.get('abilities', []):
        find_draw_amounts(ab, parsed_amounts)
    for tr in card.get('triggers', []):
        find_draw_amounts(tr, parsed_amounts)

    for dm in draw_matches:
        expected = word_to_num.get(dm)
        if expected is None:
            try:
                expected = int(dm)
            except ValueError:
                continue  # "draw X cards" — dynamic, can't check
        if expected not in parsed_amounts and parsed_amounts:
            report.add(Finding(
                card_name=card['name'],
                check='draw_amount_mismatch',
                severity='warning',
                detail=f"Oracle says draw {dm} card(s) (={expected}) but parsed amounts are {parsed_amounts}.",
                oracle_excerpt=oracle[:200],
            ))


def check_static_ability_presence(card, report):
    """Check if Oracle text has static ability patterns but none parsed."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    stripped = strip_reminder_text(oracle).lower()
    statics = card.get('static_abilities', [])

    # Static patterns: "creatures you control get/have", "spells cost X less"
    static_patterns = [
        (r'creatures? you control (get|have|gain)\b', 'creatures you control get/have'),
        (r'spells? (you cast )?cost \{?\d\}? (less|more)', 'cost modification'),
        (r'(can\'t|cannot) (attack|block|be blocked)\b', "can't attack/block"),
    ]

    for pattern, desc in static_patterns:
        if re.search(pattern, stripped) and len(statics) == 0:
            # Check if it's inside an ability or trigger (not top-level static)
            # Some of these may be in triggered/activated abilities
            has_in_other = False
            all_effects = set()
            for ab in card.get('abilities', []):
                collect_effect_types(ab, all_effects)
            for tr in card.get('triggers', []):
                collect_effect_types(tr, all_effects)
            if 'AddKeyword' in all_effects or 'Pump' in all_effects or 'AddPower' in all_effects:
                has_in_other = True
            if not has_in_other:
                report.add(Finding(
                    card_name=card['name'],
                    check='missing_static_ability',
                    severity='warning',
                    detail=f"Oracle has static pattern '{desc}' but no static abilities parsed.",
                    oracle_excerpt=oracle[:200],
                ))


def check_replacement_presence(card, report):
    """Check if Oracle text has replacement patterns but none parsed."""
    oracle = card.get('oracle_text', '')
    if not oracle:
        return
    stripped = strip_reminder_text(oracle).lower()
    replacements = card.get('replacements', [])

    # "instead" is the classic replacement indicator
    if re.search(r'\binstead\b', stripped) and len(replacements) == 0:
        # Check it's not inside a trigger or ability
        if not re.search(r'\bwhenever?\b.*\binstead\b', stripped):
            report.add(Finding(
                card_name=card['name'],
                check='missing_replacement',
                severity='info',
                detail=f"Oracle contains 'instead' but no replacement effects parsed.",
                oracle_excerpt=oracle[:200],
            ))


# ── Main ──

def run_audit(data, limit=None):
    report = AuditReport()

    cards = list(data.items())
    if limit:
        cards = cards[:limit]

    for name, card in cards:
        oracle = card.get('oracle_text', '')

        # Skip unsupported cards
        if has_unimplemented(card):
            continue

        report.total_supported += 1

        # Run all checks
        check_missing_keywords(card, report)
        check_trigger_presence(card, report)
        check_activated_ability_presence(card, report)
        check_effect_verb_mismatch(card, report)
        check_empty_ability_chains(card, report)
        check_target_presence(card, report)
        check_oracle_line_count(card, report)
        check_damage_amount(card, report)
        check_draw_amount(card, report)
        check_static_ability_presence(card, report)
        check_replacement_presence(card, report)

        report.total_checked += 1

    return report


def print_report(report, as_json=False):
    if as_json:
        output = {
            'total_supported': report.total_supported,
            'total_checked': report.total_checked,
            'total_findings': len(report.findings),
            'by_check': dict(report.check_counts),
            'by_severity': {},
            'findings': [],
        }
        severity_counts = Counter(f.severity for f in report.findings)
        output['by_severity'] = dict(severity_counts)
        for f in report.findings:
            output['findings'].append({
                'card': f.card_name,
                'check': f.check,
                'severity': f.severity,
                'detail': f.detail,
                'oracle': f.oracle_excerpt,
            })
        print(json.dumps(output, indent=2))
        return

    print(f"# Parser Correctness Audit Report")
    print(f"\nCards checked: {report.total_checked} / {report.total_supported} supported")
    print(f"Total findings: {len(report.findings)}")
    print()

    severity_counts = Counter(f.severity for f in report.findings)
    print(f"## Summary by Severity")
    for sev in ['error', 'warning', 'info']:
        print(f"  {sev}: {severity_counts.get(sev, 0)}")
    print()

    print(f"## Summary by Check")
    for check, count in report.check_counts.most_common():
        print(f"  {check}: {count}")
    print()

    # Group findings by check
    by_check = defaultdict(list)
    for f in report.findings:
        by_check[f.check].append(f)

    for check in sorted(by_check.keys()):
        findings = by_check[check]
        print(f"## {check} ({len(findings)} findings)")
        print()
        # Show first 10 examples
        for f in findings[:10]:
            print(f"  **{f.card_name}** [{f.severity}]")
            print(f"    {f.detail}")
            if f.oracle_excerpt:
                excerpt = f.oracle_excerpt.replace('\n', ' | ')[:120]
                print(f"    Oracle: {excerpt}")
            print()
        if len(findings) > 10:
            print(f"  ... and {len(findings) - 10} more")
        print()


if __name__ == '__main__':
    as_json = '--json' in sys.argv
    limit = None
    for i, arg in enumerate(sys.argv):
        if arg == '--limit' and i + 1 < len(sys.argv):
            limit = int(sys.argv[i + 1])

    with open('client/public/card-data.json') as f:
        data = json.load(f)

    report = run_audit(data, limit=limit)
    print_report(report, as_json=as_json)
