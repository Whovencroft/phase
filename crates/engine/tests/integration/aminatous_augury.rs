//! Integration test for Aminatou's Augury — "For each nonland card type, you
//! may cast a spell of that type from among the exiled cards without paying its
//! mana cost."
//!
//! CR 608.2g: The effect iterates each nonland card type (Artifact, Creature,
//! Enchantment, Instant, Kindred, Planeswalker, Sorcery, Battle) and for each
//! type with eligible cards in the pool, offers a free cast-during-resolution.

use engine::game::effects::resolve_ability_chain;
use engine::game::scenario::{GameScenario, P0, P1};
use engine::parser::oracle_effect::parse_effect_chain;
use engine::types::ability::{
    AbilityKind, Chooser, Effect, ForEachCategoryAction, IterationCategory, ResolvedAbility,
};
use engine::types::card_type::CoreType;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::TrackedSetId;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const AUGURY_EFFECT: &str =
    "for each nonland card type, you may cast a spell of that type from among \
the exiled cards without paying its mana cost";

/// Verify the parser produces the correct effect shape.
#[test]
fn aminatous_augury_parses_to_for_each_nonland_cast_free() {
    let def = parse_effect_chain(AUGURY_EFFECT, AbilityKind::Spell);
    assert!(
        matches!(
            &*def.effect,
            Effect::ForEachCategory {
                category: IterationCategory::NonlandCardType,
                action: ForEachCategoryAction::CastFreeFromPool { zone: Zone::Exile },
                ..
            }
        ),
        "expected ForEachCategory(NonlandCardType, CastFreeFromPool(Exile)), got {:?}",
        def.effect
    );
}

/// Runtime: with eligible exiled cards in the tracked set, the engine parks
/// `ChooseFromZoneChoice` for the first nonland card type with candidates.
#[test]
fn aminatous_augury_offers_choice_for_first_eligible_type() {
    let mut scenario = GameScenario::new_n_player(2, 9999);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 20);

    // Add cards to exile with different nonland types.
    let creature_card = scenario.add_creature_to_exile(P0, "Exiled Bear", 2, 2).id();
    let instant_card = scenario.add_creature_to_exile(P0, "Exiled Bolt", 0, 0).id();
    let sorcery_card = scenario
        .add_creature_to_exile(P0, "Exiled Divination", 0, 0)
        .id();

    let source = scenario.add_creature(P0, "Aminatou Source", 0, 0).id();
    let mut runner = scenario.build();

    // Set correct core types on the exiled cards.
    runner
        .state_mut()
        .objects
        .get_mut(&instant_card)
        .unwrap()
        .card_types
        .core_types = vec![CoreType::Instant];
    runner
        .state_mut()
        .objects
        .get_mut(&instant_card)
        .unwrap()
        .base_card_types
        .core_types = vec![CoreType::Instant];
    runner
        .state_mut()
        .objects
        .get_mut(&sorcery_card)
        .unwrap()
        .card_types
        .core_types = vec![CoreType::Sorcery];
    runner
        .state_mut()
        .objects
        .get_mut(&sorcery_card)
        .unwrap()
        .base_card_types
        .core_types = vec![CoreType::Sorcery];

    // Set up tracked set to simulate "exile the top eight" having put these
    // cards into the tracked set.
    let set_id = TrackedSetId(1);
    runner
        .state_mut()
        .tracked_object_sets
        .insert(set_id, vec![creature_card, instant_card, sorcery_card]);
    runner.state_mut().chain_tracked_set_id = Some(set_id);
    runner.state_mut().next_tracked_set_id = 2;

    // Build the resolved ability for the ForEachCategory effect.
    let ability = ResolvedAbility::new(
        Effect::ForEachCategory {
            category: IterationCategory::NonlandCardType,
            chooser: Chooser::Controller,
            action: ForEachCategoryAction::CastFreeFromPool { zone: Zone::Exile },
        },
        vec![],
        source,
        P0,
    );

    let mut events = Vec::new();
    resolve_ability_chain(runner.state_mut(), &ability, &mut events, 0)
        .expect("ForEachCategory CastFreeFromPool must resolve");

    // The iteration order is NonlandCardType::member_filters() which goes
    // through Artifact, Creature, Enchantment, Instant, Kindred, Planeswalker,
    // Sorcery, Battle. The first type with eligible cards is Creature.
    match &runner.state().waiting_for {
        WaitingFor::ChooseFromZoneChoice {
            cards,
            up_to,
            player,
            ..
        } => {
            assert!(*up_to, "cast is optional (you may)");
            assert_eq!(*player, P0);
            assert!(
                cards.contains(&creature_card),
                "creature card must be offered for the Creature type, got {cards:?}"
            );
        }
        other => panic!("expected ChooseFromZoneChoice for first eligible type, got {other:?}"),
    }
}
