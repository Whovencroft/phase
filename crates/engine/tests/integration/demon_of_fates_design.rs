//! Demon of Fate's Design — once-per-turn pay-life alternative cost for
//! enchantment spells (CR 118.9 + CR 601.2b).
//!
//! Verifies:
//! 1. The Oracle text parses to a `CastWithAlternativeCost` static with
//!    `OncePerTurn` frequency.
//! 2. An enchantment spell in hand is offered the pay-life alternative cost.
//! 3. A non-enchantment spell is NOT offered the grant.

use engine::game::scenario::{GameScenario, P0, P1};
use engine::types::ability::{AbilityCost, QuantityExpr, QuantityRef};
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;
use engine::types::statics::{CastFrequency, StaticMode};
use engine::types::zones::Zone;

const DEMON_ORACLE: &str = "Once during each of your turns, you may cast an enchantment spell by paying life equal to its mana value rather than paying its mana cost.";

/// The Oracle text must parse to a `CastWithAlternativeCost` static with
/// `OncePerTurn` frequency and a `PayLife { amount: SelfManaValue }` cost.
#[test]
fn demon_of_fates_design_parses_to_cast_with_alternative_cost() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    let demon_id = scenario
        .add_creature(P0, "Demon of Fate's Design", 6, 6)
        .from_oracle_text(DEMON_ORACLE)
        .id();

    let runner = scenario.build();

    let demon = &runner.state().objects[&demon_id];
    let has_alt_cost_static = demon.static_definitions.iter_unchecked().any(|d| {
        matches!(
            d.mode,
            StaticMode::CastWithAlternativeCost {
                frequency: CastFrequency::OncePerTurn,
                ..
            }
        )
    });
    assert!(
        has_alt_cost_static,
        "Demon of Fate's Design must carry a CastWithAlternativeCost static with OncePerTurn frequency"
    );
}

/// An enchantment spell in hand must be offered the pay-life alternative cost
/// when Demon of Fate's Design is on the battlefield.
#[test]
fn demon_offers_pay_life_alt_cost_for_enchantment() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);

    scenario
        .add_creature(P0, "Demon of Fate's Design", 6, 6)
        .from_oracle_text(DEMON_ORACLE);

    let ench_id = scenario
        .add_creature_to_hand(P0, "Test Enchantment", 0, 0)
        .as_enchantment()
        .with_mana_cost(ManaCost::Cost {
            shards: vec![],
            generic: 3,
        })
        .id();

    let mut runner = scenario.build();

    let ench_card = runner.state().objects[&ench_id].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: ench_id,
            card_id: ench_card,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("casting an enchantment should succeed");

    // The runner should now be waiting for the player to choose the alt cost.
    assert!(
        matches!(
            runner.state().waiting_for,
            WaitingFor::OptionalCostChoice { .. }
        ),
        "enchantment spell must be offered the pay-life alternative cost, got {:?}",
        runner.state().waiting_for,
    );

    // Accept the alternative cost.
    runner
        .act(GameAction::DecideOptionalCost { pay: true })
        .expect("accepting the alternative cost should succeed");

    assert_eq!(
        runner.state().objects[&ench_id].zone,
        Zone::Stack,
        "enchantment should be on the stack after paying the alternative cost"
    );
}

/// A non-enchantment spell must NOT be offered the Demon's alternative cost.
#[test]
fn demon_does_not_offer_alt_cost_for_non_enchantment() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);

    scenario
        .add_creature(P0, "Demon of Fate's Design", 6, 6)
        .from_oracle_text(DEMON_ORACLE);

    let creature_id = scenario
        .add_creature_to_hand(P0, "Test Creature", 2, 2)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![],
            generic: 2,
        })
        .id();

    let mut runner = scenario.build();

    let creature_card = runner.state().objects[&creature_id].card_id;
    let result = runner.act(GameAction::CastSpell {
        object_id: creature_id,
        card_id: creature_card,
        targets: vec![],
        payment_mode: CastPaymentMode::Auto,
    });

    // The creature cast may fail (no mana) but must NEVER enter OptionalCostChoice.
    if result.is_ok() {
        assert!(
            !matches!(
                runner.state().waiting_for,
                WaitingFor::OptionalCostChoice { .. }
            ),
            "non-enchantment spell must not be offered the Demon's grant, got {:?}",
            runner.state().waiting_for,
        );
    }
}
