//! Issue #821 — Expropriate money vote: per-ballot interactive permanent
//! choice + gain control.
//!
//! Oracle text (relevant clause):
//!   "For each money vote, choose a permanent owned by the voter and gain
//!    control of it."
//!
//! The spell controller must be presented with a ChooseFromZoneChoice for
//! each money ballot, selecting from permanents OWNED by the voter (not
//! controlled by the voter). After each choice, the controller gains control
//! of the chosen permanent.
//!
//! This test validates:
//! 1. resolve_tally detects the per-ballot voter-identity path and presents
//!    one ChooseFromZoneChoice per money ballot.
//! 2. After each SelectCards action, the controller gains control of the
//!    chosen permanent.
//! 3. Multiple money ballots resolve sequentially (choose → gain → choose → gain).

use engine::game::ability_utils::build_resolved_from_def;
use engine::game::effects::resolve_ability_chain;
use engine::game::engine::apply;
use engine::game::zones::create_object;
use engine::types::ability::{
    AbilityDefinition, AbilityKind, Chooser, ControllerRef, Effect, FilterProp, SubAbilityLink,
    TargetFilter, TypeFilter, TypedFilter, VoterScope, ZoneOwner,
};
use engine::types::actions::GameAction;
use engine::types::card_type::CoreType;
use engine::types::format::FormatConfig;
use engine::types::game_state::{GameState, WaitingFor};
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

/// Build the per-choice AbilityDefinition for the money clause:
/// ChooseFromZone(Battlefield, Permanent + Owned { ScopedPlayer }) → GainControl.
fn make_money_clause_def() -> Box<AbilityDefinition> {
    let mut gain_control = AbilityDefinition::new(
        AbilityKind::Spell,
        Effect::GainControl {
            target: TargetFilter::Any,
        },
    );
    gain_control.sub_link = SubAbilityLink::ContinuationStep;

    let mut choose_def = AbilityDefinition::new(
        AbilityKind::Spell,
        Effect::ChooseFromZone {
            count: 1,
            zone: Zone::Battlefield,
            additional_zones: Vec::new(),
            zone_owner: ZoneOwner::Controller,
            filter: Some(TargetFilter::Typed(TypedFilter {
                type_filters: vec![TypeFilter::Permanent],
                controller: None,
                properties: vec![FilterProp::Owned {
                    controller: ControllerRef::ScopedPlayer,
                }],
            })),
            chooser: Chooser::Controller,
            up_to: false,
            constraint: None,
        },
    );
    choose_def.sub_ability = Some(Box::new(gain_control));
    choose_def.sub_link = SubAbilityLink::ContinuationStep;
    Box::new(choose_def)
}

/// Build the per-choice AbilityDefinition for the time clause (extra turn).
fn make_time_clause_def() -> Box<AbilityDefinition> {
    // Use a simple ExtraTurn effect for the time clause.
    let def = AbilityDefinition::new(
        AbilityKind::Spell,
        Effect::ExtraTurn {
            target: TargetFilter::Controller,
        },
    );
    Box::new(def)
}

/// Construct the Expropriate vote ability.
fn make_expropriate_vote(controller: PlayerId, source_id: ObjectId) -> engine::types::ability::ResolvedAbility {
    let vote_def = AbilityDefinition::new(
        AbilityKind::Spell,
        Effect::Vote {
            choices: vec!["time".to_string(), "money".to_string()],
            per_choice_effect: vec![make_time_clause_def(), make_money_clause_def()],
            starting_with: ControllerRef::You,
            voter_scope: VoterScope::AllPlayers,
        },
    );
    build_resolved_from_def(&vote_def, source_id, controller)
}

/// Seed the battlefield with one permanent per player (owned by that player).
fn seed_battlefield(state: &mut GameState) -> Vec<ObjectId> {
    let player_ids: Vec<PlayerId> = state.players.iter().map(|p| p.id).collect();
    let mut obj_ids = Vec::new();
    for (i, &pid) in player_ids.iter().enumerate() {
        let obj_id = create_object(
            state,
            CardId(100 + i as u64),
            pid,
            format!("Bear_{}", i),
            Zone::Battlefield,
        );
        // Set card type so TypeFilter::Permanent matches.
        state
            .objects
            .get_mut(&obj_id)
            .unwrap()
            .card_types
            .core_types
            .push(CoreType::Creature);
        obj_ids.push(obj_id);
    }
    obj_ids
}

/// CR 701.38 + CR 608.2c: End-to-end two-player walkthrough. The opponent
/// votes "money"; the controller is presented with a ChooseFromZoneChoice
/// containing the opponent's permanents, selects one, and gains control.
#[test]
fn expropriate_money_vote_two_player_end_to_end() {
    let mut state = GameState::new(FormatConfig::standard(), 2, 42);
    let controller = state.players[0].id;
    let opponent = state.players[1].id;
    let source_id = ObjectId(9000);

    // Seed battlefield: each player has one permanent.
    let obj_ids = seed_battlefield(&mut state);
    let controller_permanent = obj_ids[0];
    let opponent_permanent = obj_ids[1];

    // Build and resolve the vote ability.
    let ability = make_expropriate_vote(controller, source_id);
    let mut events = Vec::new();
    resolve_ability_chain(&mut state, &ability, &mut events, 0).unwrap();

    // The controller votes first (starting_with: You).
    match &state.waiting_for {
        WaitingFor::VoteChoice { player, .. } => {
            assert_eq!(*player, controller, "controller votes first");
        }
        other => panic!("expected VoteChoice, got {:?}", other),
    }

    // Controller votes "time".
    apply(
        &mut state,
        controller,
        GameAction::ChooseOption {
            choice: "time".to_string(),
        },
    )
    .unwrap();

    // Opponent votes "money".
    match &state.waiting_for {
        WaitingFor::VoteChoice { player, .. } => {
            assert_eq!(*player, opponent, "opponent votes second");
        }
        other => panic!("expected VoteChoice for opponent, got {:?}", other),
    }
    apply(
        &mut state,
        opponent,
        GameAction::ChooseOption {
            choice: "money".to_string(),
        },
    )
    .unwrap();

    // After tally: the money clause fires per-ballot. The controller must
    // choose a permanent owned by the opponent (the voter).
    match &state.waiting_for {
        WaitingFor::ChooseFromZoneChoice {
            player,
            ref cards,
            count,
            ..
        } => {
            assert_eq!(*player, controller, "spell controller chooses");
            assert_eq!(*count, 1, "choose exactly 1");
            // The candidate list must contain the opponent's permanent.
            assert!(
                cards.contains(&opponent_permanent),
                "candidates must include opponent's permanent; got {:?}",
                cards
            );
            // The candidate list must NOT contain the controller's permanent
            // (it's owned by the controller, not the voter=opponent).
            assert!(
                !cards.contains(&controller_permanent),
                "candidates must NOT include controller's permanent"
            );
        }
        other => panic!(
            "expected ChooseFromZoneChoice for money ballot, got {:?}",
            other
        ),
    }

    // Controller selects the opponent's permanent.
    apply(
        &mut state,
        controller,
        GameAction::SelectCards {
            cards: vec![opponent_permanent],
        },
    )
    .unwrap();

    // After the choice, GainControl fires. The opponent's permanent should
    // now be controlled by the controller.
    let obj = state
        .objects
        .get(&opponent_permanent)
        .expect("permanent still exists");
    assert_eq!(
        obj.controller, controller,
        "controller must now control the opponent's permanent"
    );
}

/// CR 701.38 + CR 608.2c: Three-player game where two opponents both vote
/// "money". The controller gets two sequential ChooseFromZoneChoice prompts,
/// one per ballot.
#[test]
fn expropriate_multiple_money_ballots_resolve_sequentially() {
    let mut state = GameState::new(FormatConfig::standard(), 3, 42);
    let controller = state.players[0].id;
    let opp1 = state.players[1].id;
    let opp2 = state.players[2].id;
    let source_id = ObjectId(9000);

    // Seed battlefield: each player has one permanent.
    let obj_ids = seed_battlefield(&mut state);
    let _controller_permanent = obj_ids[0];
    let opp1_permanent = obj_ids[1];
    let opp2_permanent = obj_ids[2];

    let ability = make_expropriate_vote(controller, source_id);
    let mut events = Vec::new();
    resolve_ability_chain(&mut state, &ability, &mut events, 0).unwrap();

    // Controller votes "time".
    apply(
        &mut state,
        controller,
        GameAction::ChooseOption {
            choice: "time".to_string(),
        },
    )
    .unwrap();

    // Opp1 votes "money".
    apply(
        &mut state,
        opp1,
        GameAction::ChooseOption {
            choice: "money".to_string(),
        },
    )
    .unwrap();

    // Opp2 votes "money".
    apply(
        &mut state,
        opp2,
        GameAction::ChooseOption {
            choice: "money".to_string(),
        },
    )
    .unwrap();

    // First money ballot: controller chooses from opp1's permanents.
    match &state.waiting_for {
        WaitingFor::ChooseFromZoneChoice {
            player,
            ref cards,
            ..
        } => {
            assert_eq!(*player, controller);
            assert!(
                cards.contains(&opp1_permanent),
                "first ballot candidates must include opp1's permanent; got {:?}",
                cards
            );
        }
        other => panic!(
            "expected ChooseFromZoneChoice for first money ballot, got {:?}",
            other
        ),
    }

    // Controller selects opp1's permanent.
    apply(
        &mut state,
        controller,
        GameAction::SelectCards {
            cards: vec![opp1_permanent],
        },
    )
    .unwrap();

    // After first gain control, second money ballot fires.
    match &state.waiting_for {
        WaitingFor::ChooseFromZoneChoice {
            player,
            ref cards,
            ..
        } => {
            assert_eq!(*player, controller);
            assert!(
                cards.contains(&opp2_permanent),
                "second ballot candidates must include opp2's permanent; got {:?}",
                cards
            );
        }
        other => panic!(
            "expected ChooseFromZoneChoice for second money ballot, got {:?}",
            other
        ),
    }

    // Controller selects opp2's permanent.
    apply(
        &mut state,
        controller,
        GameAction::SelectCards {
            cards: vec![opp2_permanent],
        },
    )
    .unwrap();

    // Both permanents are now controlled by the controller.
    assert_eq!(
        state.objects.get(&opp1_permanent).unwrap().controller,
        controller,
        "opp1's permanent now controlled by controller"
    );
    assert_eq!(
        state.objects.get(&opp2_permanent).unwrap().controller,
        controller,
        "opp2's permanent now controlled by controller"
    );
}
