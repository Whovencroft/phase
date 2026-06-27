//! Aven Mindcensor — SearchLibraryTopN static ability integration test.
//!
//! Aven Mindcensor: "Flash, Flying. If an opponent would search a library, that
//! player searches the top four cards of that library instead."
//!
//! The `SearchLibraryTopN { depth: 4, scope: Opponents }` static limits the
//! candidate pool surfaced in `WaitingFor::SearchChoice` to the top N cards of
//! the library (library[0..N]) when the searcher is an opponent of the static's
//! controller.
//!
//! These tests exercise the production search pipeline (`SearchLibrary` effect →
//! `search_depth_limit()` → truncated `candidate_ids` → `SearchChoice`) through
//! a synthetic sorcery with "Search your library for a card, put it into your
//! hand, then shuffle." cast by the opponent.
//!
//! DISCRIMINATING ASSERTIONS:
//! - Test 1: 10-card library → SearchChoice.cards.len() == 4 (top 4 only).
//!   Reverting the depth-limit truncation exposes all 10 cards.
//! - Test 2: 2-card library → SearchChoice.cards.len() == 2 (depth limit does
//!   not inflate beyond actual library size).
//! - Test 3 (control): No Aven Mindcensor → SearchChoice.cards.len() == 10
//!   (full library exposed). Proves the static is the sole gate.

use engine::game::scenario::{GameScenario, P0, P1};
use engine::types::ability::StaticDefinition;
use engine::types::game_state::WaitingFor;
use engine::types::keywords::Keyword;
use engine::types::phase::Phase;
use engine::types::statics::{ProhibitionScope, StaticMode};

/// Oracle text for a generic unconditional tutor sorcery.
const TUTOR_ORACLE: &str = "Search your library for a card, put it into your hand, then shuffle.";

/// CR 701.23f + CR 614.1a: With Aven Mindcensor on the battlefield controlled
/// by P0, an opponent (P1) searching their library sees only the top 4 cards.
///
/// DISCRIMINATING: `cards.len() == 4` fails (becomes 10) if the
/// `search_depth_limit` truncation is reverted.
#[test]
fn opponent_search_limited_to_top_four_cards() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // P0 controls Aven Mindcensor on the battlefield.
    scenario
        .add_creature(P0, "Aven Mindcensor", 2, 3)
        .with_keyword(Keyword::Flash)
        .with_keyword(Keyword::Flying)
        .with_static_definition(StaticDefinition::new(StaticMode::SearchLibraryTopN {
            depth: 4,
            scope: ProhibitionScope::Opponents,
        }));

    // P1's library has 10 cards. add_card_to_library_top inserts at index 0
    // (top), so the last call produces the actual top card. We add them in
    // reverse order so "Card 1" ends up at position 0 (top) and "Card 10" at
    // position 9 (bottom).
    for i in (1..=10).rev() {
        scenario.add_card_to_library_top(P1, &format!("Library Card {i}"));
    }

    // P1 has a tutor sorcery in hand (zero mana cost by default).
    let tutor = scenario
        .add_spell_to_hand_from_oracle(P1, "Demonic Tutor", false, TUTOR_ORACLE)
        .id();

    let mut runner = scenario.build();

    // Make it P1's turn so P1 has priority to cast the sorcery.
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    // Cast the tutor — default SearchPolicy::Stop leaves the SearchChoice
    // prompt for us to inspect.
    let outcome = runner.cast(tutor).resolve();

    match outcome.final_waiting_for() {
        WaitingFor::SearchChoice { player, cards, .. } => {
            assert_eq!(
                *player, P1,
                "the searching player must be P1 (the opponent)"
            );
            assert_eq!(
                cards.len(),
                4,
                "Aven Mindcensor must limit the search to the top 4 cards, \
                 but got {} candidates: {:?}",
                cards.len(),
                cards,
            );
        }
        other => panic!("expected SearchChoice after tutor resolves, got {other:?}"),
    }
}

/// CR 701.23f: When the library has fewer cards than the depth limit, all
/// cards are offered (the limit does not inflate the pool).
///
/// DISCRIMINATING: `cards.len() == 2` — with only 2 cards in library, the
/// depth-4 cap is a no-op. Reverting the feature doesn't change this test
/// (it passes either way), but it guards against an over-eager truncation
/// that might return 0 or panic on a short library.
#[test]
fn depth_limit_does_not_exceed_library_size() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // P0 controls Aven Mindcensor.
    scenario
        .add_creature(P0, "Aven Mindcensor", 2, 3)
        .with_keyword(Keyword::Flash)
        .with_keyword(Keyword::Flying)
        .with_static_definition(StaticDefinition::new(StaticMode::SearchLibraryTopN {
            depth: 4,
            scope: ProhibitionScope::Opponents,
        }));

    // P1's library has only 2 cards.
    for i in (1..=2).rev() {
        scenario.add_card_to_library_top(P1, &format!("Library Card {i}"));
    }

    let tutor = scenario
        .add_spell_to_hand_from_oracle(P1, "Demonic Tutor", false, TUTOR_ORACLE)
        .id();

    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    let outcome = runner.cast(tutor).resolve();

    match outcome.final_waiting_for() {
        WaitingFor::SearchChoice { player, cards, .. } => {
            assert_eq!(*player, P1);
            assert_eq!(
                cards.len(),
                2,
                "with only 2 cards in library the search must offer all 2, got {}",
                cards.len(),
            );
        }
        other => panic!("expected SearchChoice after tutor resolves, got {other:?}"),
    }
}

/// Control case: without Aven Mindcensor, the full library is searchable.
///
/// DISCRIMINATING: `cards.len() == 10` — proves the depth limit is the sole
/// gate. If the static were accidentally always-on (e.g. a broken scope
/// check), this test would fail with `cards.len() == 4`.
#[test]
fn no_mindcensor_full_library_searchable() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // No Aven Mindcensor — P0 has a vanilla creature instead.
    scenario.add_creature(P0, "Grizzly Bears", 2, 2);

    // P1's library has 10 cards.
    for i in (1..=10).rev() {
        scenario.add_card_to_library_top(P1, &format!("Library Card {i}"));
    }

    let tutor = scenario
        .add_spell_to_hand_from_oracle(P1, "Demonic Tutor", false, TUTOR_ORACLE)
        .id();

    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    let outcome = runner.cast(tutor).resolve();

    match outcome.final_waiting_for() {
        WaitingFor::SearchChoice { player, cards, .. } => {
            assert_eq!(*player, P1);
            assert_eq!(
                cards.len(),
                10,
                "without Aven Mindcensor the full library must be searchable, \
                 but got {} candidates",
                cards.len(),
            );
        }
        other => panic!("expected SearchChoice after tutor resolves, got {other:?}"),
    }
}

/// CR 701.23f: The static scopes to opponents only — the controller's own
/// searches are unrestricted.
///
/// DISCRIMINATING: `cards.len() == 10` — P0 controls Aven Mindcensor AND
/// searches their own library. The `ProhibitionScope::Opponents` check must
/// NOT match the controller. If the scope check were inverted, this would
/// return 4 instead of 10.
#[test]
fn controller_search_not_limited() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // P0 controls Aven Mindcensor.
    scenario
        .add_creature(P0, "Aven Mindcensor", 2, 3)
        .with_keyword(Keyword::Flash)
        .with_keyword(Keyword::Flying)
        .with_static_definition(StaticDefinition::new(StaticMode::SearchLibraryTopN {
            depth: 4,
            scope: ProhibitionScope::Opponents,
        }));

    // P0's own library has 10 cards.
    for i in (1..=10).rev() {
        scenario.add_card_to_library_top(P0, &format!("Library Card {i}"));
    }

    // P0 has the tutor.
    let tutor = scenario
        .add_spell_to_hand_from_oracle(P0, "Demonic Tutor", false, TUTOR_ORACLE)
        .id();

    let mut runner = scenario.build();

    // P0 is already the active player by default — just cast.
    let outcome = runner.cast(tutor).resolve();

    match outcome.final_waiting_for() {
        WaitingFor::SearchChoice { player, cards, .. } => {
            assert_eq!(*player, P0);
            assert_eq!(
                cards.len(),
                10,
                "the controller's own search must not be limited by Aven Mindcensor, \
                 but got {} candidates",
                cards.len(),
            );
        }
        other => panic!("expected SearchChoice after tutor resolves, got {other:?}"),
    }
}
