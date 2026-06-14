//! Runtime regression for **issue #541** — Endurance's ETB trigger does nothing.
//!
//! Endurance: "When Endurance enters, choose up to one target player. That
//! player puts all the cards from their graveyard on the bottom of their library
//! in a random order."
//!
//! Root cause (pre-fix): `Effect::ChangeZoneAll` always passed `None` for
//! `library_placement` to `execute_zone_move`, which triggered the auto-shuffle
//! convention (CR 401.4). The cards were moved to the library but the entire
//! library was shuffled afterward, making the "bottom in a random order" clause
//! indistinguishable from a shuffle — and in some code paths the move was
//! silently skipped.
//!
//! Fix: add `library_position: Option<LibraryPosition>` to `ChangeZoneAll` and
//! thread it through the zone pipeline. When `Some(Bottom)`, each card is placed
//! at the bottom without triggering the auto-shuffle. The matching list is
//! randomized before iteration to satisfy "in a random order."
//!
//! This test drives the full cast pipeline with a sorcery host that carries
//! Endurance's ETB clause. It seeds a graveyard, casts the sorcery targeting the
//! player, and asserts that all graveyard cards end up at the bottom of the
//! library (not shuffled into the middle/top).
use engine::game::scenario::{GameScenario, P0};
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;

/// Endurance's ETB clause, driven through a sorcery host.
const ENDURANCE_CLAUSE: &str =
    "Choose up to one target player. That player puts all the cards from their \
     graveyard on the bottom of their library in a random order.";

#[test]
fn endurance_moves_all_graveyard_cards_to_library_bottom() {
    let mut scenario = GameScenario::new_n_player(2, 42);
    scenario.at_phase(Phase::PreCombatMain);

    // Seed P0's graveyard with three cards.
    scenario.with_graveyard(P0, &["GY Card A", "GY Card B", "GY Card C"]);

    // Seed P0's library with two cards so we can verify the graveyard cards
    // end up BELOW the existing library contents.
    let _lib_top = scenario.add_card_to_library_top(P0, "Lib Top");
    let _lib_second = scenario.add_card_to_library_top(P0, "Lib Second");
    // After adds: library = [Lib Second (top/index 0), Lib Top (index 1)]

    // A free sorcery carrying Endurance's clause.
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Endurance Probe", false, ENDURANCE_CLAUSE)
        .with_mana_cost(ManaCost::zero())
        .id();

    let mut runner = scenario.build();
    let spell_card = runner.state().objects[&spell].card_id;

    // Cast the sorcery (free).
    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id: spell_card,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("casting the free sorcery must succeed");

    // The spell targets a player — answer the TargetSelection prompt.
    match runner.state().waiting_for.clone() {
        WaitingFor::TargetSelection { target_slots, .. } => {
            assert!(
                target_slots[0]
                    .legal_targets
                    .contains(&TargetRef::Player(P0)),
                "P0 must be a legal target"
            );
            runner
                .act(GameAction::SelectTargets {
                    targets: vec![TargetRef::Player(P0)],
                })
                .expect("targeting P0 must succeed");
        }
        other => panic!("expected TargetSelection, got {other:?}"),
    }

    // Resolve the spell and any triggers.
    runner.advance_until_stack_empty();

    let state = runner.state();

    // All three graveyard cards must now be in the library.
    let library: Vec<ObjectId> = state.players[0].library.iter().copied().collect();
    let graveyard_names: Vec<&str> = vec!["GY Card A", "GY Card B", "GY Card C"];

    // The graveyard should be empty.
    assert!(
        state.players[0].graveyard.is_empty(),
        "graveyard must be empty after Endurance resolves; got {:?}",
        state.players[0]
            .graveyard
            .iter()
            .map(|id| state.objects[id].name.as_str())
            .collect::<Vec<_>>()
    );

    // The library must contain the original two cards PLUS the three from the
    // graveyard (total 5).
    assert_eq!(
        library.len(),
        5,
        "library must have 5 cards (2 original + 3 from graveyard); got {}",
        library.len()
    );

    // The original library cards must remain at the TOP (indices 0 and 1),
    // proving the graveyard cards were placed at the BOTTOM without shuffling.
    let top_two_names: Vec<&str> = library[..2]
        .iter()
        .map(|id| state.objects[id].name.as_str())
        .collect();
    assert!(
        top_two_names.contains(&"Lib Second") && top_two_names.contains(&"Lib Top"),
        "the original library cards must remain on top; top two = {top_two_names:?}"
    );

    // The bottom three cards must be the graveyard cards (in any order, since
    // "random order" is applied).
    let bottom_three_names: Vec<&str> = library[2..]
        .iter()
        .map(|id| state.objects[id].name.as_str())
        .collect();
    for name in &graveyard_names {
        assert!(
            bottom_three_names.contains(name),
            "graveyard card {name:?} must be at the bottom of the library; \
             bottom three = {bottom_three_names:?}"
        );
    }
}
