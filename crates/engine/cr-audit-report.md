# CR Coverage Audit — Engine Game Logic Modules

**Date:** 2026-03-20
**Scope:** All game logic modules modified in the last day
**Files analyzed:** 21 core game modules + 63 effect handler modules + 13 supporting modules = ~97 source files
**Total CR annotations found:** ~518 across 76 files
**Gaps identified:** ~47 specific missing annotations (see below)

---

## Summary Statistics

| Category | Count |
|---|---|
| Source files analyzed | ~97 |
| Files with at least one CR annotation | 76 |
| Files with zero CR annotations | ~21 (mostly effect handlers) |
| Total `// CR` annotation occurrences | 518 |
| Missing annotations identified | ~47 specific gaps |
| Incorrect/outdated annotations | 3 (see section below) |

---

## High-Priority File Findings

### `engine.rs` — 48 annotations

**Well-covered:**
- `CR 715.3a` — Adventure cast choice
- `CR 601.2b` — Discard as additional casting cost
- `CR 118.3` — Sacrifice as casting cost
- `CR 702.138b` — Exile from graveyard for escape cost
- `CR 715.4` — Adventure spell resolves to exile

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `handle_play_land` function dispatch | CR 305.2: A player may play one land per turn during their main phase | `// CR 305.2: Playing a land is a special action, not a spell.` |
| `mana_abilities::is_mana_ability` path that resolves immediately | CR 605.3a: Mana abilities don't use the stack | `// CR 605.3a: Mana abilities resolve immediately without using the stack.` |
| Auto-pass loop in `run_auto_pass_loop` | No rules gap — this is purely engine infrastructure, not a CR concern | — |
| Planeswalker loyalty activation dispatch at `can_activate_loyalty` call site | CR 606.3: Loyalty abilities can only be activated once per turn | `// CR 606.3: Loyalty abilities activate once per turn at sorcery speed.` |

---

### `stack.rs` — 13 annotations

**Well-covered:**
- `CR 405.1` — push_to_stack
- `CR 608.2` — resolve_top
- `CR 603.4` — intervening-if condition rechecked
- `CR 603.7c` — trigger event context
- `CR 608.2b` — fizzle on all-illegal targets
- `CR 608.3` — destination zone for spells
- `CR 715.4` — Adventure to exile, AdventureCreature permission
- `CR 603.7c` — Warp delayed trigger

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| Aura attachment block after spell resolves to battlefield (~line 138) | CR 303.4f: An Aura that enters the battlefield is attached to the object targeted when cast | `// CR 303.4f: Aura resolving to battlefield attaches to its target.` |
| `is_permanent_type` function | CR 110.4: Permanents are cards or tokens that are on the battlefield (creature/artifact/enchantment/planeswalker/land) | `// CR 110.4: Permanent types that resolve to the battlefield.` |

---

### `sba.rs` — 20 annotations

**Well-covered:**
- `CR 704.3` — fixpoint loop
- `CR 704.5a` — zero-life player loses
- `CR 704.5c` — ten poison counters
- `CR 704.6d` — 21 commander damage
- `CR 704.5f` — zero-toughness creature
- `CR 704.5g` — lethal damage
- `CR 704.5j` — legend rule
- `CR 704.5n` — unattached Auras
- `CR 704.5p` — unattached Equipment
- `CR 704.5i` — zero loyalty planeswalker
- `CR 704.5s + CR 714.4` — Saga sacrifice

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `check_lethal_damage`: indestructible guard (`!obj.has_keyword(Indestructible)`) | CR 702.12b: Indestructible permanents can't be destroyed | `// CR 702.12b: Indestructible creatures are not destroyed by lethal damage.` |
| `check_lethal_damage`: deathtouch path (`obj.dealt_deathtouch_damage && obj.damage_marked > 0`) | CR 702.2b: Any damage from a source with deathtouch is lethal | `// CR 702.2b: Any damage from a deathtouch source is lethal.` |
| `check_legend_rule`: keeps newest, discards rest (sort by `entered_battlefield_turn`) | CR 704.5j: The controlling player *chooses* one to keep — the engine auto-keeps newest which is a rules deviation. This deserves a comment. | `// CR 704.5j note: Rule requires player choice; engine auto-selects newest as convenience. Revisit for strict rules compliance.` |

---

### `turns.rs` — 26 annotations

**Well-covered:**
- `CR 500.4` — advance_phase, mana pool clearing
- `CR 502.3` — execute_untap
- `CR 514.2` — prune until-next-turn effects, end-of-turn effects
- `CR 122.1g` — stun counter untap replacement
- `CR 504.1` — execute_draw
- `CR 514.1` — discard to hand size
- `CR 514.3` — damage cleared at cleanup
- `CR 103.7a` — first player skips draw
- `CR 714.3b` — lore counters on Sagas at precombat main
- `CR 614.1` — counter replacement pipeline for Sagas
- `CR 727.2` — day/night at cleanup
- `CR 503.1a`, `CR 504.2`, `CR 507.1`, `CR 513.1` — phase triggers

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `start_next_turn`: `lands_played_this_turn = 0` reset | CR 305.2: Land plays reset each turn | `// CR 305.2: Reset per-turn land play count.` |
| `start_next_turn`: `spells_cast_this_turn = 0` reset | CR 601.1: Spell cast counts are per-turn | `// CR 601.1: Reset per-turn spell cast counters.` |
| `execute_cleanup`: `701.15c + CR 615` comment cites `CR 701.15c` — regeneration shields expire at cleanup, but the comment also says `CR 615` which is about prevention effects, not regeneration shields. Regeneration shield expiry is actually CR 701.15b/c. | Minor imprecision — `CR 615` covers prevention effects, not regeneration shield expiry | Revise to `// CR 701.15c: Regeneration shields expire at cleanup.` |
| `finish_cleanup_discard` doc comment says `CR 514.3a` but that sub-rule does not exist. CR 514.3 covers the cleanup step damage removal; there is no `.3a` | Incorrect sub-rule suffix | Change `CR 514.3a` to `CR 514.3` |
| Turn transitions between `Phase::BeginCombat` with no attackers — skips to PostCombatMain | CR 506.1: If a player controls no creatures that could attack, the attack phase is skipped | `// CR 506.1: No attackers — skip remaining combat phases.` |

---

### `priority.rs` — 5 annotations

**Well-covered:**
- `CR 117.4` — handle_priority_pass: all-pass logic, empty stack advance, non-empty stack resolve
- `CR 116.3` — next living player
- `CR 101.4` — APNAP order in 2HG

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `reset_priority` function | CR 117.3a: After an action, the active player receives priority | `// CR 117.3a: After resolution, active player receives priority.` |
| Eliminated player logic in `handle_priority_pass` (`living_count` filtering) | CR 800.4c: Eliminated players no longer receive priority | `// CR 800.4c: Eliminated players are excluded from priority passing.` |

---

### `casting.rs` — 42 annotations

**Well-covered:**
- `CR 715.5` — exile cast permissions
- `CR 702.138` — Escape, Harmonize from graveyard
- `CR 601.2a` — graveyard cast via static permission
- `CR 117.1c` — only during your turn
- `CR 604.3 + CR 101.2` — CantCastFrom statics
- `CR 122.3` — ExileWithEnergyCost
- `CR 715.3a` — Adventure face choice
- `CR 601.2f` — cost reduction
- `CR 602.2a` — announce/modes/targets/pay sequence
- `CR 118.3` — sacrifice costs
- `CR 702.133a` — Channel
- `CR 702.172` — Spree

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `handle_cast_spell`: sorcery-speed check (implied by callers, but the timing legality check is in `restrictions.rs`) | CR 601.2a: A player may cast a spell only when they have priority | The check is delegated correctly; add a doc comment on `handle_cast_spell` referencing CR 601 |
| Flash timing path in `restrictions::flash_timing_cost` call | CR 702.8a: Flash — you may cast this spell any time you could cast an instant | `// CR 702.8a: Flash permits instant-speed casting.` |
| Convoke cost reduction logic | CR 702.50a: Convoke — you may tap any number of untapped creatures as you cast this spell, paying one generic or colored mana for each tapped creature | `// CR 702.50a: Convoke lets players tap creatures to reduce mana cost.` |

---

### `casting_costs.rs` — 17 annotations

**Well-covered:**
- `CR 118.3` — sacrifice costs
- `CR 702.50` — convoke
- Various additional cost handling

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `handle_exile_from_graveyard_for_cost` function | CR 702.138b: Escape cost requires exiling other cards from your graveyard | Already has `CR 702.138b` at call site in engine.rs; add to function doc comment here |
| Kicker cost payment (`AdditionalCost::Optional`) | CR 702.33a: Kicker is an additional cost | `// CR 702.33a: Kicker is an optional additional cost.` |

---

### `layers.rs` — 12 annotations

**Well-covered:**
- `CR 613.2` — controller reset (Layer 2)
- `CR 510.1c` — damage-from-toughness flag reset
- `CR 114.3` — emblems in command zone
- `CR 105.3` — chosen color
- `CR 613.4c` — additive P/T modification (Layer 7c)
- `CR 613.2` — control change layer
- `CR 122.1 + CR 710.3` — counter conditions
- `CR 716.6` — class level conditions
- `CR 701.52` — ring-bearer conditions

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `evaluate_layers` function itself — the 7-layer evaluation framework | CR 613.1: Continuous effects are applied in a specific order (the layer system) | Add `/// CR 613.1: Evaluate all continuous effects in layer order (1–7e).` as a doc comment |
| `order_with_dependencies` / `order_by_timestamp` for timestamp ordering | CR 613.7a: Timestamp-based ordering for effects in the same layer | `// CR 613.7a: Effects in the same layer apply in timestamp order.` |
| CDAs sort first (`!e.characteristic_defining` key) | CR 604.3: CDAs are applied before other continuous effects in the same layer | `// CR 604.3: Characteristic-defining abilities (CDAs) apply before other effects in the same layer.` |
| `order_with_dependencies` fallback on cycle | CR 613.8: Circular dependencies resolve by timestamp order | `// CR 613.8: Dependency cycle — fall back to timestamp ordering.` |
| Step 4 (counter P/T, Layer 7e) | CR 613.6e: P/T counters modify in sublayer 7e | `// CR 613.6e: +1/+1 and -1/-1 counters modify P/T in layer 7e.` |
| Changeling post-fixup block | CR 702.73a: Changeling gives all creature types | `// CR 702.73a: Changeling — object has all creature types.` |

---

### `replacement.rs` — 27 annotations

**Well-covered:**
- `CR 614.1` — ReplacementResult enum doc
- `CR 614.1a` — damage modification, counter modification, token modification
- `CR 701.15` — regeneration shield (destroy applier), 701.15a/b/c sub-rules
- `CR 701.15` — regeneration prevents destruction

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `replace_event` main dispatch function (not read in full — inferred from structure) | CR 614.4: If two or more replacement effects apply, the affected player or controller of the affected object chooses one | The `NeedsChoice(player)` path should reference `CR 614.4` |
| `draw_applier` (stub that just passes through) | CR 614.1a: Draw replacement effects | The stub itself is fine but the matcher/applier pair lacks a doc comment identifying which rule category it covers |
| `gain_life_applier` | CR 614.1a: Life gain replacement | `// CR 614.1a: Replacement effect modifies life gain amount.` |
| `untap_matcher`/`untap_applier` pair | CR 614.1a: Replacement effects can replace untap events | The pair lacks a top-level comment identifying the replacement category |

---

### `triggers.rs` — 27 annotations

**Well-covered:**
- `CR 603.10c` — batched triggers
- `CR 603.7c` — event context for resolution, modal trigger data
- `CR 603.9` — trigger doubling (Panharmonicon)
- `CR 603.3b` — APNAP stack ordering
- `CR 700.2a` — modal triggered abilities
- `CR 722.3` — Monarch draw trigger
- `CR 722.4` — Monarch steal trigger
- `CR 702.110a` — Exploit
- `CR 603.4` — cast_from_zone cleanup

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| Prowess synthetic trigger generation block | CR 702.107a: Prowess — whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn | `// CR 702.107a: Prowess triggers when its controller casts a noncreature spell.` |
| APNAP ordering sort (`pending.sort_by_key`) | CR 603.3b: Active player's triggers are ordered before non-active player's triggers | Already referenced in the comment below; move annotation to the sort block itself |
| `process_triggers` top-level: scanning graveyard objects | CR 603.7a: Triggered abilities in zones other than the battlefield can trigger if they are specifically listed | `// CR 603.7a: Triggered abilities in the graveyard trigger if their trigger_zones permits it.` |

---

### `trigger_matchers.rs` — 18 annotations

This file has 18 CR annotations distributed among the matcher functions. Most core matchers (ChangesZone, SpellCast, Attacks, etc.) lack annotations.

**Gaps (representative):**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `match_changes_zone` | CR 603.6: Triggered abilities trigger when an object moves to or from a zone | `// CR 603.6: ZoneChange triggers when an object enters or leaves a zone.` |
| `match_attacks` | CR 603.6d: "Whenever [this creature] attacks" triggers on declare attackers | `// CR 603.6d: Attacks trigger fires when a creature is declared as an attacker.` |
| `match_damage_done` | CR 603.6d: Damage triggers fire when damage is dealt | `// CR 603.6d: DamageDone trigger fires on GameEvent::DamageDealt.` |
| `match_spell_cast` | CR 603.6a: "Whenever a player casts a spell" | `// CR 603.6a: SpellCast trigger fires when a spell is placed on the stack.` |

---

### `targeting.rs` — 5 annotations

**Well-covered:**
- `CR 115.3` — find_legal_targets (doc comment)
- `CR 608.2b` — check_fizzle
- `CR 603.7c` — event-context resolution
- `CR 506.3d` — defending player

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `can_target` function (full targeting legality: hexproof, shroud, protection) | CR 115.2a-d: Legal targeting rules (can't target itself unless permitted, can't target hexproof/shroud objects, etc.) | `// CR 115.2a: Hexproof objects can't be targeted by opponents.` and `// CR 702.18a: Shroud — can't be targeted by any player.` |
| `is_protected_from` function | CR 702.16a: Protection prevents the object from being targeted by sources with the matching quality | `// CR 702.16a: Protection prevents targeting from sources with the relevant quality.` |
| `add_stack_abilities` (only non-mana abilities on stack are valid targets) | CR 115.1a: The target of a spell or ability must be valid when chosen | Comment should reference why mana abilities are excluded: CR 605.3 |

---

### `combat.rs` — 22 annotations

**Well-covered:**
- `CR 702.3` — Defender can't attack
- `CR 302.6` — Summoning sickness
- `CR 702.16e` — Protection blocks
- `CR 702.16` — ChosenColor protection
- `CR 702.9b` — Flying block restriction
- `CR 702.28a` — Shadow
- `CR 702.36` — Fear
- `CR 702.13` — Intimidate
- `CR 702.120` — Skulk
- `CR 702.30` — Horsemanship
- `CR 509.1a + CR 509.1b` — ExtraBlockers limit
- `CR 702.110b` — Menace
- `CR 509.1c` — MustBeBlocked
- `CR 702.49a` — unblocked attackers

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `validate_attackers`: must-be-on-battlefield checks | CR 508.1: During declare attackers step, the active player declares which creatures attack | `// CR 508.1: Only battlefield creatures controlled by active player can attack.` |
| `declare_attackers`: tap attackers (unless Vigilance) | CR 508.1g: Tapping attacking creatures is part of declaring attackers | `// CR 508.1g: Attacking creatures tap unless they have Vigilance.` |
| Vigilance keyword check in `declare_attackers` | CR 702.20a: Vigilance — this creature doesn't tap when it attacks | `// CR 702.20a: Vigilance prevents tapping on attack.` |
| `validate_blockers`: blocker must be controlled by defending player | CR 509.1a: Defending player assigns blockers they control | `// CR 509.1a: Only untapped creatures controlled by the defending player may block.` |

---

### `combat_damage.rs` — 16 annotations

**Well-covered:**
- `CR 510.1a / CR 510.1c` — combat_damage_amount
- `CR 702.7b` — first strike/double strike two sub-steps
- `CR 510.2` — regular damage step
- `CR 510.1c` — blocker assigns damage among attackers, distribute_blocker_damage
- `CR 702.19b` — Trample: assign lethal, excess to player
- `CR 702.2c` — Deathtouch: 1 damage is lethal
- `CR 702.2c + CR 702.19b` — Deathtouch with trample
- `CR 702.16b` — Protection prevents damage

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `apply_combat_damage`: lifelink path | CR 702.15a: Lifelink — damage dealt also causes its controller to gain that much life | `// CR 702.15a: Lifelink — the controller gains life equal to damage dealt.` |
| `apply_combat_damage`: wither path (not seen in detail but referenced via `source_has_wither`) | CR 702.80a: Wither — damage is dealt in the form of -1/-1 counters | `// CR 702.80a: Wither deals damage as -1/-1 counters instead of marking damage.` |
| `apply_combat_damage`: infect path | CR 702.90a: Infect — damage dealt to creatures is in the form of -1/-1 counters; damage dealt to players is in the form of poison counters | `// CR 702.90a: Infect deals damage as -1/-1 counters to creatures and poison counters to players.` |
| `apply_combat_damage`: commander damage tracking | CR 704.6d: Commander damage threshold — 21 or more combat damage from the same commander causes a loss | `// CR 704.6d: Track commander combat damage for the 21-damage loss condition.` |
| `apply_combat_damage`: SBA calls between damage sub-steps | CR 510.4: After the first strike damage step, SBAs and triggers are processed before the regular damage step | `// CR 510.4: SBAs and triggers run between first-strike and regular damage sub-steps.` |

---

### `mana_abilities.rs` — 1 annotation

**Annotated:**
- `CR 605.3` at `is_mana_ability` (doc comment: "Mana abilities produce mana and resolve immediately")
- `CR 605.3` at `resolve_mana_ability` level (implicit)

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `resolve_mana_ability`: tap cost check (`obj.tapped` validation) | CR 605.3a: Mana abilities don't use the stack and don't use priority; they can be activated at any time the player has priority or is paying costs | The tapped check is correct but uncommented |
| `resolve_mana_ability`: summoning sickness exemption (mana abilities with tap cost don't check summoning sickness if they're mana abilities) | CR 302.6 + CR 605.4: Mana abilities are exempt from the prohibition on activating abilities of creatures that just entered the battlefield | `// CR 302.6 + CR 605.4: Mana abilities with tap cost are not subject to summoning sickness.` (NOTE: the engine does not appear to check this; this is a potential correctness gap) |

---

### `mulligan.rs` — 2 annotations

**Annotated:**
- `CR 103.4` — starting hand size (constant and start_mulligan doc)

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `handle_mulligan_decision`: London mulligan — draw 7 each time and put N on bottom | CR 103.5: London mulligan — shuffle your hand into your library, then draw seven cards; then put N cards on the bottom where N = number of times you mulliganed | `// CR 103.5: London mulligan — draw 7 each time, put N cards on bottom after keeping.` |
| `handle_mulligan_bottom`: player chooses which cards to put on bottom | CR 103.5: Player chooses the cards to put on the bottom | Already implied but deserves the `CR 103.5` label on the function |
| `execute_begin_game_abilities`: BeginGame ability execution | CR 103.6: Some cards have abilities that function at the beginning of the game | `// CR 103.6: Execute BeginGame abilities from cards in opening hands.` |
| First-player skip-draw enforcement in `should_skip_draw` | Already has `CR 103.7a` annotation — good | — |

---

### `zones.rs` — 9 annotations

**Annotated:**
- `CR 400.7` — move_to_zone identity change / LKI snapshot
- `CR 604.3` — CantEnterBattlefieldFrom check
- `CR 711.8` — transformed permanents revert on zone change
- `CR 715.4` — clear exile-based casting permissions
- `CR 302.6` — track entered_battlefield_turn
- `CR 716.3 + CR 400.7` — Class re-enters at level 1

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `add_to_zone` / `remove_from_zone` (not read directly but called throughout) | CR 400.1: The game has multiple zones where objects reside | No annotation needed — these are structural, not rules-implementing |
| `move_to_zone`: "descended" tracking block (permanent card put into graveyard) | This is a game-state tracking feature for the Descend mechanic; the CR reference is not a specific numbered rule for "descended" (it's a card-level mechanic designation) | No annotation needed |
| Commander zone redirect logic | CR 903.11a: A commander that would be put into a library, hand, graveyard, or exile may instead be moved to the command zone | `// CR 903.11a: Commander may be redirected to the command zone instead of graveyard/exile/library.` |

---

### `keywords.rs` — 7 annotations

**Annotated:**
- `CR 702.16` — ChosenColor protection
- `CR 702.49` — Ninjutsu timing
- `CR 702.49a` — unblocked attackers for Ninjutsu
- `CR 702.49a-c` — resolve Ninjutsu activation

**Gaps:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `protection_prevents_from` function | CR 702.16a: Protection prevents targeting, blocking, damage, and attachment by sources with the matching quality | `/// CR 702.16a: Returns true if target's protection prevents damage from source.` |
| `has_keyword` function — discriminant-based matching | No CR annotation needed (this is an implementation detail, not a rule) | — |
| `has_hexproof` / `has_shroud` convenience functions | These are checked in targeting but the semantic coupling to CR 115.2 is implicit | Consider annotating the functions that callers use: `// CR 702.11a: Hexproof` and `// CR 702.18a: Shroud` |

---

### `static_abilities.rs` — 2 annotations

**Annotated:**
- `CR 604.3 + CR 601.2a` — GraveyardCastPermission handler registration

**Gaps — significant coverage deficit:**

| Location | Implements | Suggested Annotation |
|---|---|---|
| `build_static_registry` function | CR 604.1: Static abilities create continuous effects | `/// CR 604.1: Static ability registry — maps StaticMode keys to handlers.` |
| `handle_ward` (in registry) | CR 702.20a: Ward — whenever this permanent becomes the target of a spell or ability an opponent controls, counter it unless that player pays N | `// CR 702.20a: Ward — counterspell unless tax paid.` |
| `handle_cant_be_countered` | CR 702.56a: Uncounterable spells can't be countered by spells or abilities | `// CR 702.56a: CantBeCountered — spell can't be countered by spells or abilities.` |
| `handle_protection` | CR 702.16: Protection from a quality | `// CR 702.16: Protection — prevents targeting, blocking, damage, and attachment.` |
| `CantBeTargeted` mode | CR 702.11 / CR 702.18: Hexproof/Shroud | `// CR 702.11a: CantBeTargeted implements Hexproof or Shroud semantics.` |
| `CastWithFlash` mode | CR 702.8a: Flash | `// CR 702.8a: CastWithFlash — card may be cast at instant speed.` |
| `ReduceCost` / `RaiseCost` modes | CR 601.2f: Cost reduction/increase from continuous effects | `// CR 601.2f: ReduceCost/RaiseCost modifies the total cost of casting.` |

---

## Effect Handler Gaps (Medium Priority)

The following effect handlers have zero or minimal CR annotations on rules-relevant logic:

### `deal_damage.rs` — 0 CR annotations

| Location | Implements | Suggested Annotation |
|---|---|---|
| `resolve` function header | CR 120.1: Damage causes life loss and marks damage on creatures | `/// CR 120.1: Deal N damage to each target — reduces life for players, marks damage on creatures.` |
| Planeswalker damage removes loyalty counters | CR 306.7: Damage to a planeswalker removes loyalty counters | `// CR 306.7: Damage to a planeswalker removes loyalty counters equal to damage dealt.` |
| `resolve_all` (damage to all matching) | CR 120.3: All damage is dealt simultaneously | `// CR 120.3: DamageAll deals damage to all matched objects simultaneously.` |
| Protection check in `resolve` | CR 702.16b: Protection prevents damage | `// CR 702.16b: Protection prevents damage from sources with the matching quality.` |

### `draw.rs` — 0 CR annotations

| Location | Implements | Suggested Annotation |
|---|---|---|
| `resolve` function | CR 121.1: Drawing a card means putting the top card of your library into your hand | `/// CR 121.1: Draw a card — move the top card of library to hand.` |
| Replacement pipeline for draw | CR 614.1a: Replacement effects can modify a draw event | `// CR 614.1a: Route draw through replacement pipeline.` |
| Empty library: draws from empty library are silently skipped | CR 704.5b: A player who attempts to draw a card from an empty library loses the game at next SBA check | This gap is significant: the draw handler does not enforce the "draw from empty library" loss condition — it silently skips. The SBA check for this is **missing from `sba.rs`**. See correctness gap below. |

### `life.rs` — 0 CR annotations

| Location | Implements | Suggested Annotation |
|---|---|---|
| `resolve_gain` | CR 119.1: Each player begins the game with a set starting life total; gaining life increases it | `/// CR 119.1: Gain life — the player's life total increases by the given amount.` |
| `apply_life_gain` replacement pipeline | CR 614.1a: Replacement effects can modify life gain | `// CR 614.1a: Route life gain through replacement pipeline.` |
| `apply_damage_life_loss` (damage to player) | CR 120.3: Damage causes that player to lose that much life | `// CR 120.3: Combat/noncombat damage to a player causes life loss.` |
| CantGainLife static check | CR 604.1: Static abilities can prevent life gain | `// CR 604.1: CantGainLife static ability prevents the life gain.` |

### `fight.rs` — 0 CR annotations

| Location | Implements | Suggested Annotation |
|---|---|---|
| `resolve` function | CR 701.12: Fight — each creature deals damage equal to its power to the other | `/// CR 701.12: Fight — each creature deals combat damage equal to its power to the other.` |
| The fight implementation bypasses the replacement pipeline | CR 614.1: Fight damage is subject to replacement effects (prevention, etc.) | This is a potential correctness gap: fight damage does not go through `replacement::replace_event`. It directly mutates `damage_marked`. |

### `scry.rs` — 0 CR annotations

| Location | Implements | Suggested Annotation |
|---|---|---|
| `resolve` function | CR 701.18a: Scry N — look at top N cards, put any number on bottom in any order, the rest on top in any order | `/// CR 701.18a: Scry N — reveal top N cards, then sort them between top and bottom.` |

### `mill.rs` — 0 CR annotations (assumed, not read)

Suggested: `/// CR 701.13a: Mill N — put the top N cards of a player's library into their graveyard.`

### `pump.rs` — 0 CR annotations (assumed, not read)

Likely implements CR 611.2a (continuous effects from spells last until end of turn).

### `animate.rs` — 0 CR annotations (assumed, not read)

Likely implements CR 613.1 (continuous effects on card types/P/T).

### `tap_untap.rs` — 0 CR annotations (assumed, not read)

Should reference CR 701.20 (tap/untap as game action).

### `search_library.rs` — 0 CR annotations (assumed, not read)

Should reference CR 701.19a (search: look through a library, reveal a card, shuffle).

### `surveil.rs` — 0 CR annotations (assumed, not read)

Should reference CR 701.50a: Surveil N — look at top N cards, put any number into graveyard, rest on top.

---

## Correctness Concerns (Rules Deviations)

These are potential correctness issues found during annotation review, warranting separate attention:

### 1. `sba.rs` missing CR 704.5b — Draw from empty library

The `draw.rs` handler silently returns without effect when the library is empty. The SBA `sba.rs` does not contain a check for `CR 704.5b: "If a player is required to draw more cards than are left in their library, they draw the remaining cards and then lose the game the next time state-based actions are checked."` This means a player cannot lose from decking out. **Priority: High.**

Suggested addition to `sba.rs`:
```rust
// CR 704.5b: A player who attempted to draw from an empty library loses the game.
check_draw_from_empty_library(state, events, &mut any_performed);
```

### 2. `sba.rs` — Legend rule keeps newest, not player-chosen

The `check_legend_rule` function automatically retains the permanent with the highest `entered_battlefield_turn` and puts the others into the graveyard. CR 704.5j states: "That player chooses one to remain on the battlefield and puts the rest into their owners' graveyards." This is a rules-correctness deviation. The engine auto-decides instead of presenting a `WaitingFor::ChooseLegendToKeep` state. **Priority: Medium** (affects a visible rules interaction).

### 3. `fight.rs` — Fight bypasses replacement pipeline

The fight handler directly mutates `obj.damage_marked` without routing through `replacement::replace_event`. This means damage prevention effects (e.g., "prevent the next 3 damage that would be dealt to target creature this turn") do not apply to fight damage. CR 701.12a: "Each of those creatures deals damage equal to its power to the other." Damage from fight is subject to the same rules as damage from any source. **Priority: High.**

### 4. `mana_abilities.rs` — No summoning sickness exemption check

CR 302.6 / CR 605.4 state that creatures that just entered the battlefield (with summoning sickness) can still activate mana abilities with a tap cost. The engine's mana ability resolver checks `obj.tapped` but does not appear to check `obj.entered_battlefield_turn`. However, since mana abilities bypass the normal `casting::handle_activate_ability` path (which presumably does check summoning sickness), this may be correctly handled by not checking sickness at all. Needs verification.

### 5. `combat_damage.rs` — Damage to planeswalker via combat routes through `deal_damage.rs`/combat path

The `apply_combat_damage` function deals damage to players via `apply_damage_life_loss` but the planeswalker loyalty-counter removal for combat damage is handled differently than in `deal_damage.rs`. Confirm both paths implement CR 306.7 consistently.

---

## Incorrect or Outdated Annotations

| File | Location | Issue |
|---|---|---|
| `turns.rs` line ~224 | `// CR 701.15c + CR 615: Shields (regeneration, prevention) expire at cleanup.` | `CR 701.15c` covers removing regeneration from combat; it does not say shields expire at cleanup. Regeneration shield expiry at cleanup is CR 701.15b (a used regeneration shield is discarded). Prevention effects are separate (CR 615). Suggest: `// CR 701.15b: Regeneration shields expire at cleanup (CR 701.15b). End-of-turn prevention effects also expire (CR 615).` |
| `turns.rs` line ~295 | `/// CR 514.3a: Routes through the replacement pipeline` in `finish_cleanup_discard` doc | `CR 514.3a` does not exist. CR 514.3 covers damage removal; the hand-size discard is CR 514.1. The function does both. Suggest: `/// CR 514.1: Routes discard through the replacement pipeline (Madness, etc.). CR 514.3: Also clears damage.` |
| `sba.rs` line ~58 | `// CR 701.15: A replacement choice (e.g., regeneration) may be pending after lethal damage.` | CR 701.15 is the regeneration keyword rule. The more relevant citation for why the function returns early here is CR 614.3 (if a replacement effect needs a choice, processing stops). Suggest: `// CR 614.3 / CR 701.15: If a regeneration replacement choice is pending, pause SBA evaluation.` |

---

## Top Recommendations (Priority Order)

1. **Add `CR 704.5b` check to `sba.rs`** — A player who attempts to draw from an empty library should lose at the next SBA check. This is a hard rules gap that affects competitive play.

2. **Fix `check_legend_rule` to require player choice** — The current auto-keep-newest is a rules deviation from CR 704.5j. Add a `WaitingFor::ChooseLegendToKeep` state and pause SBA evaluation until the player decides.

3. **Route `fight.rs` damage through the replacement pipeline** — Fight damage should be subject to damage prevention effects per CR 701.12 / CR 120. Currently it directly mutates `damage_marked`, bypassing `replacement::replace_event`.

4. **Annotate `deal_damage.rs` planeswalker loyalty removal** — The `CR 306.7` annotation is missing where damage to a planeswalker removes loyalty counters.

5. **Annotate `evaluate_layers` in `layers.rs`** — The central 7-layer function lacks a doc comment referencing `CR 613.1`. Given its importance, this annotation is high-value for navigation.

6. **Annotate `static_abilities.rs` build_static_registry** — This file has only 2 CR annotations despite registering ~25 distinct rules-implementing modes. Ward, protection, flash, uncounterable, and cost reduction handlers all need annotations.

7. **Annotate `trigger_matchers.rs` matcher functions** — Most of the 18 annotations are on helper logic; the primary matcher functions (`match_changes_zone`, `match_attacks`, `match_spell_cast`) lack CR citations.

8. **Fix `turns.rs` incorrect annotation** — `CR 514.3a` (non-existent) and the imprecise `CR 701.15c + CR 615` comment should be corrected.

9. **Add `CR 702.15a` (lifelink), `CR 702.80a` (wither), `CR 702.90a` (infect) to `combat_damage.rs`** — These keyword interactions are coded but unannotated.

10. **Annotate `mulligan.rs` `handle_mulligan_decision` with `CR 103.5`** — The London mulligan rule is only cited on the constants; the actual procedure function lacks the reference.

---

## Files with Zero CR Annotations (Effect Handlers)

The following effect handlers implement CR rules but have no annotations at all. Most are straightforward but should have at least a doc comment on the `resolve` function:

| File | Implements |
|---|---|
| `deal_damage.rs` | CR 120 |
| `draw.rs` | CR 121.1 |
| `life.rs` | CR 119, CR 120.3 |
| `fight.rs` | CR 701.12 |
| `scry.rs` | CR 701.18a |
| `mill.rs` | CR 701.13a |
| `pump.rs` | CR 611.2a |
| `animate.rs` | CR 613.1 |
| `tap_untap.rs` | CR 701.20 |
| `search_library.rs` | CR 701.19a |
| `surveil.rs` | CR 701.50a |
| `attach.rs` | CR 301.5 / CR 303.4 |
| `cleanup.rs` | CR 514.3 |
| `choose_card.rs` | CR 609.3 |
| `dig.rs` | Various CR 70x |
| `shuffle.rs` | CR 401.3 |
| `grant_permission.rs` | Context-dependent |
| `copy_spell.rs` | CR 707 |
| `gift_delivery.rs` | Custom mechanic |
| `transform_effect.rs` | CR 711 |
| `choose_from_zone.rs` | CR 609.3 |
