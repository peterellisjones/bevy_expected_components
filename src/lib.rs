//! # `bevy_expected_components`
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
//! `#[expects(T)]` solves this by panicking if expected components are missing at insert
//! time, making bugs immediately visible during development.
//!
//! ## Performance Warning
//!
//! This crate adds runtime overhead: every time a component with `#[expects(...)]` is
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
//! // PhysicsBody expects Transform and Velocity to exist when it's inserted
//! #[derive(Component, ExpectComponents)]
//! #[expects(Transform, Velocity)]
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
//! | Feature | `#[require]` | `#[expects]` |
//! |---------|--------------|--------------|
//! | Missing component | Auto-inserted with `Default` | Panics |
//! | Requires `Default` | Yes | No |
//! | Runtime cost | Archetype lookup | Component existence check |
//! | Use case | Convenience bundles | Bug detection |
//!
//! ## Future of This Crate
//!
//! This crate may become unnecessary when Bevy adds native support for non-defaultable
//! required components. Relevant discussions:
//!
//! - [Issue #16194: Require components that can't be defaulted](https://github.com/bevyengine/bevy/issues/16194)
//! - [Issue #18717: Support required components which have no sensible default](https://github.com/bevyengine/bevy/issues/18717)
//! - Archetype invariants (future Bevy feature)

use std::any::TypeId;

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::{DeferredWorld, World};

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
/// #[expects(Transform, Velocity)]
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
    /// Creates a new registration with a hook registration function.
    ///
    /// Called by the derive macro. You should not need to use this directly.
    #[must_use]
    pub const fn new(register_hooks: fn(&mut World)) -> Self {
        Self { register_hooks }
    }

    /// Registers the component hooks with the world.
    pub fn register(&self, world: &mut World) {
        (self.register_hooks)(world);
    }
}

/// Registers component hooks for type T. Used by the derive macro.
#[doc(hidden)]
pub fn register_hooks_for<T: ExpectComponents>(world: &mut World) {
    world
        .register_component_hooks::<T>()
        .on_add(validate_expected::<T>);
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
            registration.register(app.world_mut());
        }
    }
}

/// Validation hook called when a component with expectations is inserted.
#[allow(clippy::needless_pass_by_value)] // Bevy hook signature requires owned DeferredWorld
fn validate_expected<T: ExpectComponents>(world: DeferredWorld, ctx: HookContext) {
    let expected = T::expected_components();
    let names = T::expected_component_names();
    let entity = ctx.entity;

    for (type_id, name) in expected.iter().zip(names.iter()) {
        let component_id = world.components().get_id(*type_id);
        let has_component = component_id.is_some_and(|id| world.entity(entity).contains_id(id));

        assert!(
            has_component,
            "{} expects {} but it was not found on entity {:?}",
            std::any::type_name::<T>(),
            name,
            entity
        );
    }
}
