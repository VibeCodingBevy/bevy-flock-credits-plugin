use bevy::prelude::*;

mod config;

pub use config::{CreditsConfig, CreditsSection};

use config::parse_skip_key;

const HEADER_FONT_SCALE: f32 = 1.5;
const SECTION_SPACING: f32 = 40.0;
const LINE_HEIGHT_FACTOR: f32 = 1.3;
const START_OFFSET: f32 = 50.0;
const SKIP_COUNTDOWN_MARGIN: f32 = 24.0;
const SKIP_COUNTDOWN_FONT_SCALE: f32 = 1.0;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreditsState {
    #[default]
    Idle,
    Active,
}

#[derive(Component)]
struct CreditsText;

#[derive(Component)]
struct ScrollTarget {
    total_height: f32,
}

#[derive(Component)]
struct SkipCountdown;

#[derive(Resource, Default)]
struct SkipHold {
    held_secs: f32,
}

pub struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<CreditsState>()
            .init_resource::<SkipHold>()
            .add_systems(
                OnEnter(CreditsState::Active),
                (show_credits, reset_skip_hold),
            )
            .add_systems(OnExit(CreditsState::Active), hide_credits)
            .add_systems(
                Update,
                (scroll_credits, hold_to_skip).run_if(in_state(CreditsState::Active)),
            );
    }
}

fn show_credits(mut commands: Commands, credits_config: Res<CreditsConfig>) {
    let start_top = credits_config.screen_height + START_OFFSET;
    let font_size = credits_config.font_size;
    let header_font_size = font_size * HEADER_FONT_SCALE;

    let mut total_height: f32 = 0.0;
    for section in &credits_config.sections {
        total_height += header_font_size * LINE_HEIGHT_FACTOR;
        let line_count = section.text.lines().count().max(1);
        total_height += font_size * LINE_HEIGHT_FACTOR * line_count as f32;
        total_height += SECTION_SPACING;
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                overflow: Overflow::clip(),
                ..default()
            },
            CreditsText,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        top: Val::Px(start_top),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ScrollTarget { total_height },
                ))
                .with_children(|scroll_parent| {
                    for section in &credits_config.sections {
                        scroll_parent.spawn((
                            Text::new(&section.header),
                            TextFont {
                                font_size: FontSize::Px(header_font_size),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        ));
                        scroll_parent.spawn((
                            Text::new(&section.text),
                            TextFont {
                                font_size: FontSize::Px(font_size),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        ));
                        scroll_parent.spawn((Node {
                            height: Val::Px(SECTION_SPACING),
                            ..default()
                        },));
                    }
                });

            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(font_size * SKIP_COUNTDOWN_FONT_SCALE),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(SKIP_COUNTDOWN_MARGIN),
                    bottom: Val::Px(SKIP_COUNTDOWN_MARGIN),
                    ..default()
                },
                Visibility::Hidden,
                SkipCountdown,
            ));
        });
}

fn hide_credits(mut commands: Commands, query: Query<Entity, With<CreditsText>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn reset_skip_hold(mut skip_hold: ResMut<SkipHold>) {
    skip_hold.held_secs = 0.0;
}

fn scroll_credits(
    time: Res<Time>,
    credits_config: Res<CreditsConfig>,
    mut next_state: ResMut<NextState<CreditsState>>,
    mut query: Query<(&mut Node, &ScrollTarget)>,
) {
    for (mut node, scroll_target) in query.iter_mut() {
        if let Val::Px(top) = &mut node.top {
            *top -= credits_config.credits_speed * time.delta_secs();

            if *top < -scroll_target.total_height {
                next_state.set(CreditsState::Idle);
            }
        }
    }
}

fn hold_to_skip(
    time: Res<Time>,
    credits_config: Res<CreditsConfig>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut skip_hold: ResMut<SkipHold>,
    mut next_state: ResMut<NextState<CreditsState>>,
    mut countdown: Query<(&mut Text, &mut Visibility), With<SkipCountdown>>,
    mut cached_key: Local<Option<KeyCode>>,
) {
    let skip_key = *cached_key.get_or_insert_with(|| {
        parse_skip_key(&credits_config.skip_key).unwrap_or_else(|| {
            warn!(
                "unknown skip_key {:?}, falling back to Space",
                credits_config.skip_key
            );
            KeyCode::Space
        })
    });

    let holding = keys.is_some_and(|keys| keys.pressed(skip_key));
    let hold_secs = credits_config.skip_hold_seconds.max(0.0);

    if holding {
        skip_hold.held_secs += time.delta_secs();
    } else {
        skip_hold.held_secs = 0.0;
    }

    if holding && skip_hold.held_secs >= hold_secs {
        next_state.set(CreditsState::Idle);
    }

    let remaining = (hold_secs - skip_hold.held_secs).max(0.0).ceil() as u32;
    for (mut text, mut visibility) in &mut countdown {
        if holding && remaining > 0 {
            *visibility = Visibility::Visible;
            let label = remaining.to_string();
            if text.0 != label {
                text.0 = label;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::input::ButtonState;
    use bevy::input::InputPlugin;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use super::*;

    const FRAME_SECS: f32 = 1.0 / 60.0;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                FRAME_SECS,
            )))
            .insert_resource(CreditsConfig {
                font_size: 24.0,
                credits_speed: 10.0,
                screen_height: 100.0,
                skip_key: "space".into(),
                skip_hold_seconds: 3.0,
                sections: vec![CreditsSection {
                    header: "Header".into(),
                    text: "body".into(),
                }],
            })
            .add_plugins(CreditsPlugin);

        app.world_mut()
            .resource_mut::<NextState<CreditsState>>()
            .set(CreditsState::Active);
        app
    }

    fn send_key(app: &mut App, key_state: ButtonState) {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Space,
            state: key_state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    fn countdown(app: &mut App) -> (String, bool) {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Text, &Visibility), With<SkipCountdown>>();
        let (text, visibility) = query.single(app.world()).expect("one countdown entity");
        (text.0.clone(), *visibility == Visibility::Visible)
    }

    #[test]
    fn countdown_counts_down_while_held_and_hides_on_release() {
        let mut app = app();
        app.update();

        assert_eq!(countdown(&mut app), (String::new(), false));

        send_key(&mut app, ButtonState::Pressed);
        app.update();
        assert_eq!(countdown(&mut app), ("3".to_owned(), true));

        for _ in 0..60 {
            app.update();
        }
        assert_eq!(countdown(&mut app), ("2".to_owned(), true));

        for _ in 0..60 {
            app.update();
        }
        assert_eq!(countdown(&mut app), ("1".to_owned(), true));

        send_key(&mut app, ButtonState::Released);
        app.update();
        assert!(!countdown(&mut app).1);

        send_key(&mut app, ButtonState::Pressed);
        app.update();
        assert_eq!(countdown(&mut app), ("3".to_owned(), true));
    }

    #[test]
    fn zero_hold_skips_on_first_press() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CreditsConfig>()
            .skip_hold_seconds = 0.0;
        app.update();

        send_key(&mut app, ButtonState::Pressed);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<CreditsState>>().get(),
            CreditsState::Idle
        );
    }
}
