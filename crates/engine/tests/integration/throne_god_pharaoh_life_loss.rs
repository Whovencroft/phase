//! Regression test for GitHub issue #1318 — Throne of the God-Pharaoh
//! "each opponent loses life equal to the number of tapped creatures you control"
//! was not applying life loss.
//!
//! The trigger fires at the beginning of the controller's end step. The amount
//! of life lost equals the number of tapped creatures the Throne's controller
//! controls. The effect iterates over each opponent via `player_scope`.

use engine::game::scenario::{GameScenario, P0, P1};
use engine::game::triggers::process_triggers;
use engine::types::ability::{
    ControllerRef, Effect, FilterProp, PlayerFilter, QuantityExpr, QuantityRef, TargetFilter,
    TypeFilter,
};
use engine::types::events::GameEvent;
use engine::types::phase::Phase;

const THRONE_ORACLE: &str =
    "At the beginning of your end step, each opponent loses life equal to the number of tapped creatures you control.";

/// CR 119.2 + CR 603.2b: Throne of the God-Pharaoh's end-step trigger causes
/// each opponent to lose life equal to the number of tapped creatures the
/// controller controls. With 2 tapped creatures, each opponent loses 2 life.
#[test]
fn throne_end_step_each_opponent_loses_life_for_tapped_creatures() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PostCombatMain);
    scenario.with_life(P1, 20);

    // Add Throne of the God-Pharaoh (artifact) controlled by P0
    let mut builder = scenario.add_creature_from_oracle(
        P0,
        "Throne of the God-Pharaoh",
        0,
        0,
        THRONE_ORACLE,
    );
    builder.as_artifact();
    let throne_id = builder.id();

    // Add two creatures controlled by P0 that will be tapped
    let creature1 = scenario.add_creature(P0, "Bear", 2, 2).id();
    let creature2 = scenario.add_creature(P0, "Wolf", 3, 3).id();

    // Add one untapped creature (should not count)
    let _creature3 = scenario.add_creature(P0, "Bird", 1, 1).id();

    let mut runner = scenario.build();

    // Tap the two creatures
    runner.state_mut().objects.get_mut(&creature1).unwrap().tapped = true;
    runner.state_mut().objects.get_mut(&creature2).unwrap().tapped = true;

    // Verify the trigger definition was parsed correctly
    let triggers = runner
        .state()
        .objects
        .get(&throne_id)
        .expect("Throne on battlefield")
        .trigger_definitions
        .len();
    assert!(
        triggers > 0,
        "Throne oracle must install at least one trigger, got {triggers}"
    );

    let end_trigger = runner
        .state()
        .objects
        .get(&throne_id)
        .unwrap()
        .trigger_definitions
        .iter_unchecked()
        .find(|t| t.phase == Some(Phase::End))
        .expect("end step phase trigger");

    let execute = end_trigger.execute.as_ref().expect("execute ability");
    assert_eq!(
        execute.player_scope,
        Some(PlayerFilter::Opponent),
        "each opponent loses … must scope to opponents"
    );

    match &*execute.effect {
        Effect::LoseLife { amount, .. } => match amount {
            QuantityExpr::Ref {
                qty:
                    QuantityRef::ObjectCount {
                        filter: TargetFilter::Typed(tf),
                    },
            } => {
                assert_eq!(
                    tf.controller,
                    Some(ControllerRef::You),
                    "filter must count creatures YOU control"
                );
                assert!(
                    tf.type_filters.contains(&TypeFilter::Creature),
                    "filter must require Creature type"
                );
                assert!(
                    tf.properties.iter().any(|p| matches!(p, FilterProp::Tapped)),
                    "filter must require Tapped property"
                );
            }
            other => panic!("expected ObjectCount for tapped creatures, got {other:?}"),
        },
        other => panic!("expected LoseLife execute, got {other:?}"),
    }

    // Advance to end step and fire the trigger
    runner.state_mut().phase = Phase::End;
    runner.state_mut().active_player = P0;

    process_triggers(
        runner.state_mut(),
        &[GameEvent::PhaseChanged {
            phase: Phase::End,
        }],
    );

    assert!(
        !runner.state().stack.is_empty(),
        "Throne end-step trigger must put a trigger on the stack"
    );

    // Resolve the trigger
    runner.advance_until_stack_empty();

    // P1 should have lost 2 life (2 tapped creatures controlled by P0)
    assert_eq!(
        runner.life(P1),
        18,
        "P1 must lose 2 life (one per tapped creature controlled by P0): 20 - 2 = 18"
    );
}
