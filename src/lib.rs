//! # bevy_expected_components
//!
//! Runtime validation for Bevy component dependencies. Like `#[require]` but validates
//! instead of auto-inserting.
//!
//! ## Why Use This?
//!
//! Bevy's `#[require(T)]` automatically inserts missing components using `Default`.
//! This doesn't work when:
//!
//! - Required components need world access to construct
//! - You want bugs to surface immediately rather than silently using defaults
//! - The required component has no sensible default
//!
//! `#[expect(T)]` solves this by panicking if expected components are missing at insert
//! time, making bugs immediately visible during development.
//!
//! ## Performance Warning
//!
//! This crate adds runtime overhead: every time a component with `#[expect(...)]` is
//! inserted, the plugin checks that all expected components exist on the entity.
//!
//! **Recommended usage:** Enable only in development and test builds.
//!
//! ```rust,ignore
//! // Only add the plugin in debug builds
//! #[cfg(debug_assertions)]
//! app.add_plugins(ExpectedComponentsPlugin);
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_expected_components::prelude::*;
//!
//! #[derive(Component, Default)]
//! struct Transform;
//!
//! #[derive(Component, Default)]
//! struct Velocity;
//!
//! // RoadNode expects Transform to exist when it's inserted
//! #[derive(Component, ExpectComponents)]
//! #[expect(Transform, Velocity)]
//! struct PhysicsBody;
//!
//! fn main() {
//!     let mut app = App::new();
//!
//!     // Enable validation (only in debug builds recommended)
//!     #[cfg(debug_assertions)]
//!     app.add_plugins(ExpectedComponentsPlugin);
//!
//!     // This works - all expected components present
//!     app.world_mut().spawn((PhysicsBody, Transform, Velocity));
//!
//!     // This panics - Velocity is missing!
//!     // app.world_mut().spawn((PhysicsBody, Transform));
//! }
//! ```
//!
//! ## How It Works
//!
//! 1. `#[derive(ExpectComponents)]` generates an [`ExpectComponents`] trait implementation
//! 2. The derive macro registers the type with [`inventory`] at compile time
//! 3. [`ExpectedComponentsPlugin`] iterates all registered types and installs `on_add` hooks
//! 4. When a component is inserted, the hook validates expected components exist
//!
//! ## Comparison with `#[require]`
//!
//! | Feature | `#[require]` | `#[expect]` |
//! |---------|--------------|-------------|
//! | Missing component | Auto-inserted with `Default` | Panics |
//! | Requires `Default` | Yes | No |
//! | Runtime cost | Archetype lookup | Component existence check |
//! | Use case | Convenience bundles | Bug detection |

use std::any::TypeId;

use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

// Re-export for macro use
#[doc(hidden)]
pub use inventory;

// Re-export derive macro
pub use bevy_expected_components_macros::ExpectComponents;

/// Prelude module for convenient imports.
///
/// ```rust,ignore
/// use bevy_expected_components::prelude::*;
/// ```
pub mod prelude {
    pub use crate::ExpectComponents;
    pub use crate::ExpectedComponentsPlugin;
}

/// Trait implemented by components that expect other components to be present.
///
/// This trait is automatically implemented by the `#[derive(ExpectComponents)]` macro.
/// You should not need to implement it manually.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Component, ExpectComponents)]
/// #[expect(Transform, Velocity)]
/// struct PhysicsBody;
/// ```
pub trait ExpectComponents: Component {
    /// Returns the `TypeId`s of expected components.
    fn expected_components() -> &'static [TypeId];

    /// Returns human-readable names of expected components for error messages.
    fn expected_component_names() -> &'static [&'static str];
}

/// Registration entry for a component with expectations.
///
/// Created by the `#[derive(ExpectComponents)]` macro and collected via `inventory`.
/// You should not need to use this directly.
pub struct ExpectRegistration {
    register_hooks: fn(&mut World),
}

impl ExpectRegistration {
    /// Creates a registration for a component type.
    ///
    /// Called by the derive macro. You should not need to use this directly.
    #[must_use]
    pub fn of<T: ExpectComponents>() -> Self {
        Self {
            register_hooks: |world| {
                world
                    .register_component_hooks::<T>()
                    .on_add(validate_expected::<T>);
            },
        }
    }
}

inventory::collect!(ExpectRegistration);

/// Plugin that enables runtime validation of component expectations.
///
/// When added to your app, this plugin registers `on_add` hooks for all components
/// that use `#[derive(ExpectComponents)]`. When those components are inserted,
/// the hooks validate that all expected components exist on the entity.
///
/// # Performance Warning
///
/// This plugin adds runtime overhead. It is recommended to only enable it in
/// development and test builds:
///
/// ```rust,ignore
/// #[cfg(debug_assertions)]
/// app.add_plugins(ExpectedComponentsPlugin);
/// ```
///
/// # Panics
///
/// When a component is inserted and its expected components are missing, the
/// plugin will panic with a message like:
///
/// ```text
/// my_crate::RoadNode expects bevy::transform::components::Transform
/// but it was not found on entity 42v3
/// ```
pub struct ExpectedComponentsPlugin;

impl Plugin for ExpectedComponentsPlugin {
    fn build(&self, app: &mut App) {
        for registration in inventory::iter::<ExpectRegistration> {
            (registration.register_hooks)(app.world_mut());
        }
    }
}

/// Validation hook called when a component with expectations is inserted.
fn validate_expected<T: ExpectComponents>(
    world: DeferredWorld,
    entity: Entity,
    _component_id: ComponentId,
) {
    let expected = T::expected_components();
    let names = T::expected_component_names();

    for (type_id, name) in expected.iter().zip(names.iter()) {
        let component_id = world.components().get_id(*type_id);
        let has_component = component_id
            .is_some_and(|id| world.entity(entity).contains_id(id));

        if !has_component {
            panic!(
                "{} expects {} but it was not found on entity {:?}",
                std::any::type_name::<T>(),
                name,
                entity
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component, Default)]
    struct Position;

    #[derive(Component, Default)]
    struct Velocity;

    #[derive(Component, ExpectComponents)]
    #[expect(Position, Velocity)]
    struct PhysicsBody;

    #[derive(Component, ExpectComponents)]
    #[expect(Position)]
    struct SingleExpectation;

    #[test]
    fn succeeds_when_all_expected_components_present() {
        let mut app = App::new();
        app.add_plugins(ExpectedComponentsPlugin);

        app.world_mut().spawn((PhysicsBody, Position, Velocity));
        // No panic = success
    }

    #[test]
    fn succeeds_with_single_expectation() {
        let mut app = App::new();
        app.add_plugins(ExpectedComponentsPlugin);

        app.world_mut().spawn((SingleExpectation, Position));
    }

    #[test]
    #[should_panic(expected = "expects")]
    fn panics_when_expected_component_missing() {
        let mut app = App::new();
        app.add_plugins(ExpectedComponentsPlugin);

        app.world_mut().spawn((PhysicsBody, Velocity)); // Missing Position
    }

    #[test]
    #[should_panic(expected = "Position")]
    fn panic_message_includes_missing_component_name() {
        let mut app = App::new();
        app.add_plugins(ExpectedComponentsPlugin);

        app.world_mut().spawn((PhysicsBody, Velocity));
    }

    #[test]
    fn no_validation_without_plugin() {
        let mut app = App::new();
        // Plugin intentionally not added

        app.world_mut().spawn((PhysicsBody,)); // Would panic if plugin was added
        // No panic = validation disabled
    }

    #[test]
    fn order_independent_insertion() {
        let mut app = App::new();
        app.add_plugins(ExpectedComponentsPlugin);

        // Expected components inserted before the expecting component
        app.world_mut().spawn((Position, Velocity, PhysicsBody));
    }
}
