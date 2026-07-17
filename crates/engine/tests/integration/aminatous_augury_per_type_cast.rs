//! Integration test for Aminatou's Augury — "for each nonland card type, you
//! may cast a spell of that type from among the exiled cards without paying its
//! mana cost."
//!
//! Verifies:
//! 1. The oracle text parses into `Effect::ForEachCategory { NonlandCardType,
//!    GrantPerTypeCastPermission { without_paying_mana_cost: true } }`.
//! 2. After resolution, exiled cards matching a nonland type carry a
//!    `PlayFromExile { without_paying_mana_cost: true, single_use: true }` permission.
//! 3. The stamped permission makes the exiled card castable without paying mana.

use engine::game::ability_utils::build_resolved_from_def;
use engine::game::casting::spell_objects_available_to_cast;
use engine::game::effects::resolve_ability_chain;
use engine::game::scenario::{GameScenario, P0};
use engine::game::zones::create_object;
use engine::parser::oracle_effect::parse_effect_chain;
use engine::types::ability::{
    AbilityKind, CastingPermission, Effect, ForEachCategoryAction, IterationCategory,
};
use engine::types::card_type::CoreType;
use engine::types::identifiers::{CardId, TrackedSetId};
use engine::types::zones::Zone;

/// Parse the clause and verify it produces the expected effect structure.
#[test]
fn parses_for_each_nonland_card_type_cast() {
    let def = parse_effect_chain(
        "For each nonland card type, you may cast a spell of that type from among the exiled cards without paying its mana cost.",
        AbilityKind::Spell,
    );
    match def.effect.as_ref() {
        Effect::ForEachCategory {
            category, action, ..
        } => {
            assert_eq!(category, &IterationCategory::NonlandCardType);
            match action {
                ForEachCategoryAction::GrantPerTypeCastPermission {
                    without_paying_mana_cost,
                } => {
                    assert!(
                        *without_paying_mana_cost,
                        "expected without_paying_mana_cost: true"
                    );
                }
                other => panic!("expected GrantPerTypeCastPermission, got {other:?}"),
            }
        }
        other => panic!("expected ForEachCategory, got {other:?}"),
    }
}

/// After resolving the effect with a creature card in exile, the creature
/// should carry a PlayFromExile permission with without_paying_mana_cost: true.
#[test]
fn grants_play_from_exile_permission_on_exiled_creature() {
    let scenario = GameScenario::new();
    let mut runner = scenario.build();

    // Create a source object on the battlefield (the spell that exiled cards).
    let source = {
        let state = runner.state_mut();
        let id = create_object(
            state,
            CardId(1),
            P0,
            "Aminatou's Augury".to_string(),
            Zone::Battlefield,
        );
        state
            .objects
            .get_mut(&id)
            .unwrap()
            .card_types
            .core_types
            .push(CoreType::Sorcery);
        id
    };

    // Create a creature card in exile (simulating it was exiled by the spell).
    let exiled_creature = {
        let state = runner.state_mut();
        let id = create_object(state, CardId(2), P0, "Exiled Bear".to_string(), Zone::Exile);
        let obj = state.objects.get_mut(&id).unwrap();
        obj.card_types.core_types = vec![CoreType::Creature];
        id
    };

    // Seed the chain tracked set with the exiled creature — simulates the
    // preceding ChangeZone(Library->Exile) that Aminatou's Augury performs.
    {
        let state = runner.state_mut();
        state
            .tracked_object_sets
            .insert(TrackedSetId(1), vec![exiled_creature]);
        state.next_tracked_set_id = 2;
        state.chain_tracked_set_id = Some(TrackedSetId(1));
    }

    // Parse and resolve the ForEachCategory effect.
    let def = parse_effect_chain(
        "For each nonland card type, you may cast a spell of that type from among the exiled cards without paying its mana cost.",
        AbilityKind::Spell,
    );
    let resolved = build_resolved_from_def(&def, source, P0);
    let mut events = Vec::new();
    // depth=1: simulates being inside the larger Aminatou's Augury chain
    // where the preceding ChangeZone(Library->Exile) already ran at depth=0
    // and established chain_tracked_set_id. The depth-0 prelude clears
    // chain_tracked_set_id, so entering at depth=1 preserves the binding.
    resolve_ability_chain(runner.state_mut(), &resolved, &mut events, 1)
        .expect("ForEachCategory resolution should succeed");

    // Verify the exiled creature now has a PlayFromExile permission.
    let state = runner.state();
    let obj = state.objects.get(&exiled_creature).unwrap();
    let has_free_cast_permission = obj.casting_permissions.iter().any(|p| {
        matches!(
            p,
            CastingPermission::PlayFromExile {
                without_paying_mana_cost: true,
                single_use: true,
                ..
            }
        )
    });
    assert!(
        has_free_cast_permission,
        "Exiled creature should have PlayFromExile {{ without_paying_mana_cost: true, single_use: true }} \
         but permissions are: {:?}",
        obj.casting_permissions
    );

    // Verify the creature is now available to cast.
    let castable = spell_objects_available_to_cast(state, P0);
    assert!(
        castable.contains(&exiled_creature),
        "Exiled creature should appear in spell_objects_available_to_cast"
    );
}
