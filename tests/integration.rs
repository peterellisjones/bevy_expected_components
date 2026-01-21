use bevy_app::App;
use bevy_ecs::component::Component;
use bevy_expected_components::prelude::*;

#[derive(Component, Default)]
struct Position;

#[derive(Component, Default)]
struct Velocity;

#[derive(Component, ExpectComponents)]
#[expects(Position, Velocity)]
struct PhysicsBody;

#[derive(Component, ExpectComponents)]
#[expects(Position)]
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

#[test]
fn multiple_expects_attributes() {
    #[derive(Component, ExpectComponents)]
    #[expects(Position)]
    #[expects(Velocity)]
    struct MultiAttribute;

    let mut app = App::new();
    app.add_plugins(ExpectedComponentsPlugin);

    app.world_mut().spawn((MultiAttribute, Position, Velocity));
}
