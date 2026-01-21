# bevy_expected_components

Runtime validation for Bevy component dependencies. Like `#[require]` but panics instead of auto-inserting.

## Performance Warning

**This crate adds runtime overhead.** Every time a component with `#[expects(...)]` is inserted, the plugin checks that all expected components exist on the entity.

Enable only in development and test builds:

```rust
#[cfg(debug_assertions)]
app.add_plugins(ExpectedComponentsPlugin);
```

With this pattern, release builds have zero overhead.

## Why Use This?

Bevy's `#[require(T)]` automatically inserts missing components using `Default`. This doesn't work when:

- Required components need world access to construct (e.g., handles, entity references)
- You want bugs to surface immediately rather than silently using defaults
- The required component has no sensible default

`#[expects(T)]` solves this by panicking if expected components are missing, making bugs immediately visible during development.

## Installation

```toml
[dependencies]
bevy_expected_components = "0.1"
```

## Usage

```rust
use bevy::prelude::*;
use bevy_expected_components::prelude::*;

// PhysicsBody expects Transform and Velocity to exist when inserted
#[derive(Component, ExpectComponents)]
#[expects(Transform, Velocity)]
struct PhysicsBody;

#[derive(Component, Default)]
struct Velocity(Vec3);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    // Enable validation in debug builds only
    #[cfg(debug_assertions)]
    app.add_plugins(ExpectedComponentsPlugin);

    app.add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands) {
    // This works - all expected components present
    commands.spawn((
        PhysicsBody,
        Transform::default(),
        Velocity::default(),
    ));

    // This panics in debug builds - Velocity is missing!
    // commands.spawn((PhysicsBody, Transform::default()));
}
```

## Error Messages

When validation fails, you get a clear panic message:

```
my_game::PhysicsBody expects my_game::Velocity but it was not found on entity 42v3
```

The stack trace points to the spawn site, making debugging straightforward.

## Comparison with `#[require]`

| Feature | `#[require]` | `#[expects]` |
|---------|--------------|--------------|
| Missing component | Auto-inserted via `Default` | Panics |
| Requires `Default` | Yes | No |
| Runtime cost | Archetype lookup | Component check |
| Use case | Convenience bundles | Bug detection |
| When to use | Components with sensible defaults | Components that must be explicitly provided |

## Multiple Expectations

You can list multiple components in one attribute or use multiple attributes:

```rust
// Single attribute with multiple components
#[derive(Component, ExpectComponents)]
#[expects(Transform, Velocity, Health)]
struct Enemy;

// Multiple attributes (equivalent)
#[derive(Component, ExpectComponents)]
#[expects(Transform)]
#[expects(Velocity)]
#[expects(Health)]
struct Enemy;
```

## Qualified Paths

Full paths work too:

```rust
#[derive(Component, ExpectComponents)]
#[expects(bevy::transform::components::Transform)]
struct MyComponent;
```

## How It Works

1. `#[derive(ExpectComponents)]` generates an `ExpectComponents` trait implementation
2. The macro registers the type with `inventory` at compile time
3. `ExpectedComponentsPlugin` installs `on_add` hooks for all registered types
4. When a component is inserted, the hook validates expected components exist
5. If any are missing, it panics with a descriptive message

## Limitations

**Validates insertion only, not removal.** If you later remove an expected component from an entity, no error occurs. This keeps the implementation simple and covers the main use case: catching mistakes at spawn time.

If you need removal protection, consider using Bevy's `on_remove` hooks directly or waiting for archetype invariants.

## Future of This Crate

This crate may become unnecessary when Bevy adds native support for non-defaultable required components. Relevant upstream discussions:

- [Issue #16194: Require components that can't be defaulted](https://github.com/bevyengine/bevy/issues/16194) - Proposes `#[require(Component(explicit))]` syntax
- [Issue #18717: Support required components which have no sensible default](https://github.com/bevyengine/bevy/issues/18717) - Proposes `#[must_provide(Component)]`
- Archetype invariants - A future Bevy feature that would enforce component relationships at the type level

Until then, this crate provides a simple, opt-in solution for development-time validation.

## Bevy Version Compatibility

| bevy | bevy_expected_components |
|------|--------------------------|
| 0.18 | 0.1                      |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
