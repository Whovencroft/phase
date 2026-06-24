//! CR 700.3 + CR 608: Pile-separation primitive — partition objects into two
//! piles, another player chooses one, sub-effect applies.
//!
//! Per CR 700.3b, a pile is not a `GameObject`; it is a transient
//! `im::Vector<ObjectId>` ledger that lives on the [`WaitingFor`] until the
//! chooser picks a side. Per CR 700.3c, partitioned objects do not leave
//! their zone during the partition/choice steps — only the final sub-effect
//! acts on them. Per CR 700.3a the partition is exhaustive and disjoint
//! (pile B is derived as `eligible \ pile_a`) and per CR 700.3d either pile
//! may be empty.
//!
//! This module follows the Vote interactive-queue pattern: build an
//! APNAP-ordered subject queue, park on a dedicated `WaitingFor` for the
//! first subject, and process advance/transition in
//! `engine_resolution_choices.rs`. The chosen-pile sub-effect is fanned out
//! from the choice handler.

use crate::game::players::apnap_order_from;
use crate::types::ability::{
    Effect, EffectError, EffectKind, PlayerScope, ResolvedAbility, VoterScope,
};
use crate::types::events::GameEvent;
use crate::types::game_state::{GameState, PileResult, WaitingFor};
use crate::types::identifiers::ObjectId;
use crate::types::player::PlayerId;

/// CR 700.3 + CR 101.4: Initiate a pile-separation effect. Builds the APNAP
/// subject queue scoped by `partition_subject`, computes each subject's
/// eligible set against `object_filter` (restricted to the subject's own
/// permanents per CR 700.3c — objects stay in their controller's zone), then
/// parks on [`WaitingFor::SeparatePilesPartition`] for the first non-empty
/// subject. Subjects with zero eligible objects are recorded as empty
/// `PileResult`s and skipped (CR 700.3d).
///
/// If every subject is empty, no choice is needed; we emit `EffectResolved`
/// and let the chain continue. (For Make an Example specifically there is no
/// stack-level continuation — the sub-effect is the only work — so this
/// degenerates to a no-op resolution.)
pub fn resolve(
    state: &mut GameState,
    ability: &ResolvedAbility,
    events: &mut Vec<GameEvent>,
) -> Result<(), EffectError> {
    let Effect::SeparateIntoPiles {
        partition_subject,
        object_filter,
        chooser,
        chosen_pile_effect,
    } = &ability.effect
    else {
        return Err(EffectError::InvalidParam(
            "separate_piles::resolve called with non-SeparateIntoPiles effect".into(),
        ));
    };

    let controller = ability.controller;

    // CR 101.4: APNAP order starting at the active player; CR 800.4f drops
    // eliminated players. The public 3-arg `apnap_order_from` (game/players.rs)
    // is the authority — `vote.rs`'s private 2-arg helper is not importable.
    let subjects: Vec<PlayerId> = apnap_order_from(state, None, controller)
        .into_iter()
        .filter(|pid| match partition_subject {
            // CR 800.4g: `EachOpponent` excludes the controller.
            VoterScope::EachOpponent => *pid != controller,
            VoterScope::AllPlayers => true,
            // `ControllerLabels` is a vote-shape concept; pile-separation
            // does not produce labels and the parser does not emit it for
            // `SeparateIntoPiles`. Treat as a degenerate empty queue rather
            // than silently coercing to a different scope.
            VoterScope::ControllerLabels => false,
        })
        .collect();

    let chooser_id = resolve_chooser(state, ability, chooser.clone()).unwrap_or(controller);

    // CR 700.3 + CR 700.3c: Compute each subject's eligible objects. Only
    // objects on the battlefield controlled by the subject and matching
    // `object_filter` are partitioned (CR 700.3c — partition does not move
    // them between zones; eligibility is computed once at resolution start).
    let ctx = crate::game::filter::FilterContext::from_ability(ability);
    let mut subject_pools: Vec<(PlayerId, crate::im::Vector<ObjectId>)> = subjects
        .into_iter()
        .map(|pid| {
            let pool: crate::im::Vector<ObjectId> = state
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    state.objects.get(id).is_some_and(|obj| {
                        obj.controller == pid
                            && !obj.is_emblem
                            && crate::game::filter::matches_target_filter(
                                state,
                                *id,
                                object_filter,
                                &ctx,
                            )
                    })
                })
                .collect();
            (pid, pool)
        })
        .collect();

    // CR 700.3d: Subjects with zero eligible objects are recorded as empty
    // partitions and skipped — they do not need to be prompted.
    let mut completed: crate::im::Vector<PileResult> = crate::im::Vector::new();
    while let Some((pid, pool)) = subject_pools.first() {
        if pool.is_empty() {
            completed.push_back(PileResult {
                subject: *pid,
                pile_a: crate::im::Vector::new(),
                pile_b: crate::im::Vector::new(),
            });
            subject_pools.remove(0);
        } else {
            break;
        }
    }

    if subject_pools.is_empty() {
        // No subject has any eligible objects — `apply_pile_effect` would
        // do nothing for any of them. Emit and continue without parking.
        events.push(GameEvent::EffectResolved {
            kind: EffectKind::SeparateIntoPiles,
            source_id: ability.source_id,
        });
        return Ok(());
    }

    let (first_subject, first_pool) = subject_pools.remove(0);
    let remaining_subjects: crate::im::Vector<(PlayerId, crate::im::Vector<ObjectId>)> =
        subject_pools.into_iter().collect();

    state.waiting_for = WaitingFor::SeparatePilesPartition {
        player: first_subject,
        eligible: first_pool,
        remaining_subjects,
        completed,
        chooser: chooser_id,
        chosen_pile_effect: chosen_pile_effect.clone(),
        source_id: ability.source_id,
    };

    Ok(())
}

/// CR 700.3 + CR 109.4: Apply the chosen-pile sub-effect across every
/// completed subject. For each `PileResult` and each `ObjectId` in the
/// chosen pile, resolve `chosen_pile_effect` with the subject rebound as
/// controller (e.g., Make an Example's per-opponent `Sacrifice`).
///
/// CR 608 + CR 704: All per-object resolutions accumulate within a single
/// spell resolution; state-based actions and resulting death triggers are
/// checked once after `apply` returns (the engine's standard SBA pass —
/// `run_post_action_pipeline` — runs after the choice handler finishes).
pub fn apply_pile_effect(
    state: &mut GameState,
    source_id: ObjectId,
    chosen_pile_effect: &crate::types::ability::AbilityDefinition,
    results: &[(PileResult, crate::types::game_state::PileSide)],
    events: &mut Vec<GameEvent>,
) -> Result<(), EffectError> {
    for (result, side) in results {
        let chosen: &crate::im::Vector<ObjectId> = match side {
            crate::types::game_state::PileSide::A => &result.pile_a,
            crate::types::game_state::PileSide::B => &result.pile_b,
        };
        if chosen.is_empty() {
            continue;
        }
        // CR 109.4 + CR 608.2c: Rebind the sub-effect controller to the
        // subject so per-pile effects (sacrifice, etc.) target the subject's
        // own permanents, not the spell controller's.
        for &object_id in chosen.iter() {
            let mut chain = sub_effect_as_resolved(chosen_pile_effect, source_id, result.subject);
            chain.targets = vec![crate::types::ability::TargetRef::Object(object_id)];
            super::resolve_ability_chain(state, &chain, events, 1)?;
        }
    }
    events.push(GameEvent::EffectResolved {
        kind: EffectKind::SeparateIntoPiles,
        source_id,
    });
    Ok(())
}

/// Convert a parsed `AbilityDefinition` into a `ResolvedAbility` carrying the
/// requested source/controller. Mirrors `vote::resolved_from_def`.
fn sub_effect_as_resolved(
    def: &crate::types::ability::AbilityDefinition,
    source_id: ObjectId,
    controller: PlayerId,
) -> ResolvedAbility {
    let mut resolved =
        ResolvedAbility::new((*def.effect).clone(), Vec::new(), source_id, controller);
    resolved.kind = def.kind;
    resolved.sub_ability = def
        .sub_ability
        .as_ref()
        .map(|sub| Box::new(sub_effect_as_resolved(sub, source_id, controller)));
    resolved.duration = def.duration.clone();
    resolved.condition = def.condition.clone();
    resolved.optional_targeting = def.optional_targeting;
    resolved.optional = def.optional;
    resolved.target_choice_timing = def.target_choice_timing;
    resolved.description = def.description.clone();
    resolved.min_x_value = def.min_x_value;
    resolved.cant_be_copied = def.cant_be_copied;
    resolved.forward_result = def.forward_result;
    resolved.player_scope = def.player_scope.clone();
    resolved.starting_with = def.starting_with.clone();
    resolved.target_selection_mode = def.target_selection_mode;
    resolved.sub_link = def.sub_link;
    resolved
}

/// CR 109.4 + CR 608.2c: Resolve a `PlayerScope` to the concrete chooser
/// PlayerId for the pile-separation effect. Currently supports `Controller`
/// (Make an Example's "you choose"); other scopes degrade to `None` so the
/// caller falls back to the ability controller — matching the conservative
/// "you chose by default" semantics of similar resolvers.
fn resolve_chooser(
    _state: &GameState,
    ability: &ResolvedAbility,
    chooser: PlayerScope,
) -> Option<PlayerId> {
    match chooser {
        PlayerScope::Controller => Some(ability.controller),
        // Future shapes (Liliana −6 "target player chooses", etc.) will
        // extend this match. Until they ship, fall back so a stray scope
        // doesn't crash the resolver — the caller defaults to controller.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ability::{AbilityDefinition, AbilityKind, QuantityExpr, TargetFilter};
    use crate::types::identifiers::CardId;
    use crate::types::zones::Zone;

    fn sacrifice_sub() -> Box<AbilityDefinition> {
        Box::new(AbilityDefinition::new(
            AbilityKind::Spell,
            Effect::Sacrifice {
                target: TargetFilter::ParentTarget,
                count: QuantityExpr::Fixed { value: 1 },
                min_count: 0,
            },
        ))
    }

    fn make_an_example_ability(source_id: ObjectId, controller: PlayerId) -> ResolvedAbility {
        ResolvedAbility::new(
            Effect::SeparateIntoPiles {
                partition_subject: VoterScope::EachOpponent,
                object_filter: TargetFilter::Typed(crate::types::ability::TypedFilter::creature()),
                chooser: PlayerScope::Controller,
                chosen_pile_effect: sacrifice_sub(),
            },
            Vec::new(),
            source_id,
            controller,
        )
    }

    fn place_creature(state: &mut GameState, owner: PlayerId, card_id: u64) -> ObjectId {
        let id = crate::game::zones::create_object(
            state,
            CardId(card_id),
            owner,
            format!("C{card_id}"),
            Zone::Battlefield,
        );
        state
            .objects
            .get_mut(&id)
            .unwrap()
            .card_types
            .core_types
            .push(crate::types::card_type::CoreType::Creature);
        id
    }

    /// CR 700.3 + CR 800.4g: Initiating Make an Example with an opponent who
    /// controls creatures parks on `SeparatePilesPartition` for that opponent
    /// (not the controller).
    #[test]
    fn make_an_example_parks_on_opponent_partition() {
        let mut state = GameState::new_two_player(42);
        let caster = state.players[0].id;
        let opp = state.players[1].id;
        let c1 = place_creature(&mut state, opp, 1);
        let c2 = place_creature(&mut state, opp, 2);
        let c3 = place_creature(&mut state, opp, 3);

        let ability = make_an_example_ability(ObjectId(100), caster);
        let mut events = Vec::new();
        resolve(&mut state, &ability, &mut events).expect("resolves");
        match &state.waiting_for {
            WaitingFor::SeparatePilesPartition {
                player,
                eligible,
                chooser,
                ..
            } => {
                assert_eq!(*player, opp);
                assert_eq!(*chooser, caster);
                assert!(eligible.contains(&c1));
                assert!(eligible.contains(&c2));
                assert!(eligible.contains(&c3));
                assert_eq!(eligible.len(), 3);
            }
            other => panic!("expected SeparatePilesPartition, got {other:?}"),
        }
    }

    /// CR 700.3d: An opponent with no creatures is recorded as an empty
    /// `PileResult` and skipped — no partition prompt is raised for them.
    /// When every opponent is empty, the resolver emits `EffectResolved`
    /// without parking.
    #[test]
    fn empty_opponent_pools_skip_to_completion() {
        let mut state = GameState::new_two_player(42);
        let caster = state.players[0].id;
        let ability = make_an_example_ability(ObjectId(100), caster);
        let mut events = Vec::new();
        let initial = state.waiting_for.clone();
        resolve(&mut state, &ability, &mut events).expect("resolves");
        assert!(matches!(state.waiting_for, ref w if *w == initial));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::EffectResolved {
                kind: EffectKind::SeparateIntoPiles,
                ..
            }
        )));
    }

    /// CR 700.3 / CR 701.21: End-to-end runtime test driving Make an Example
    /// through the engine pipeline. Opponent splits 3 creatures into 2/1;
    /// caster picks the 2-pile; those exact 2 are sacrificed and the 1 in
    /// the unchosen pile remains on the battlefield. This is the
    /// discriminating runtime test required by the issue.
    #[test]
    fn discriminator_make_an_example_sacrifices_chosen_pile() {
        use crate::game::engine::apply;
        use crate::types::actions::GameAction;
        use crate::types::game_state::PileSide;

        let mut state = GameState::new_two_player(42);
        // Run during a regular main phase so SBAs/priority behave normally.
        state.turn_number = 2;
        state.phase = crate::types::phase::Phase::PreCombatMain;
        state.active_player = state.players[0].id;
        state.priority_player = state.players[0].id;
        state.waiting_for = WaitingFor::Priority {
            player: state.players[0].id,
        };

        let caster = state.players[0].id;
        let opp = state.players[1].id;
        let c1 = place_creature(&mut state, opp, 10);
        let c2 = place_creature(&mut state, opp, 11);
        let c3 = place_creature(&mut state, opp, 12);

        // Drive resolution by hand: build the ability and resolve through
        // the chain, then submit the interactive actions through `apply`.
        let ability = make_an_example_ability(ObjectId(500), caster);
        let mut events = Vec::new();
        super::super::resolve_ability_chain(&mut state, &ability, &mut events, 0)
            .expect("resolves the chain");
        // Park should be SeparatePilesPartition for opp.
        assert!(matches!(
            state.waiting_for,
            WaitingFor::SeparatePilesPartition { player, .. } if player == opp
        ));

        // Opponent partitions: pile A = [c1, c2], pile B (derived) = [c3].
        apply(
            &mut state,
            opp,
            GameAction::SubmitPilePartition {
                pile_a: vec![c1, c2],
            },
        )
        .expect("partition accepted");

        // Now caster picks pile A (the 2-pile).
        assert!(matches!(
            state.waiting_for,
            WaitingFor::SeparatePilesChoice { player, .. } if player == caster
        ));
        apply(
            &mut state,
            caster,
            GameAction::ChoosePile { pile: PileSide::A },
        )
        .expect("pile choice accepted");

        // c1 and c2 sacrificed; c3 remains.
        assert!(!state.battlefield.contains(&c1), "c1 must be sacrificed");
        assert!(!state.battlefield.contains(&c2), "c2 must be sacrificed");
        assert!(
            state.battlefield.contains(&c3),
            "c3 (in unchosen pile) must remain on battlefield"
        );
        assert!(state.players[1].graveyard.contains(&c1));
        assert!(state.players[1].graveyard.contains(&c2));
    }

    /// CR 700.3d: When the chooser picks the empty pile, zero creatures are
    /// sacrificed and no panic occurs.
    #[test]
    fn empty_pile_choice_sacrifices_nothing() {
        use crate::game::engine::apply;
        use crate::types::actions::GameAction;
        use crate::types::game_state::PileSide;

        let mut state = GameState::new_two_player(42);
        state.turn_number = 2;
        state.phase = crate::types::phase::Phase::PreCombatMain;
        state.active_player = state.players[0].id;
        state.priority_player = state.players[0].id;
        state.waiting_for = WaitingFor::Priority {
            player: state.players[0].id,
        };
        let caster = state.players[0].id;
        let opp = state.players[1].id;
        let c1 = place_creature(&mut state, opp, 20);
        let c2 = place_creature(&mut state, opp, 21);

        let ability = make_an_example_ability(ObjectId(600), caster);
        let mut events = Vec::new();
        super::super::resolve_ability_chain(&mut state, &ability, &mut events, 0)
            .expect("resolves the chain");
        // Opponent puts both creatures in pile A.
        apply(
            &mut state,
            opp,
            GameAction::SubmitPilePartition {
                pile_a: vec![c1, c2],
            },
        )
        .expect("partition accepted");
        // CR 700.3d: caster chooses the empty pile B → zero sacrifices.
        apply(
            &mut state,
            caster,
            GameAction::ChoosePile { pile: PileSide::B },
        )
        .expect("empty-pile choice accepted");
        assert!(state.battlefield.contains(&c1));
        assert!(state.battlefield.contains(&c2));
        assert!(state.players[1].graveyard.is_empty());
    }
}
