use bevy::prelude::*;

mod config;

pub use config::{CreditsConfig, CreditsSection};

const HEADER_FONT_SCALE: f32 = 1.5;
const SECTION_SPACING: f32 = 40.0;
const LINE_HEIGHT_FACTOR: f32 = 1.3;
const START_OFFSET: f32 = 50.0;

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

pub struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<CreditsState>()
            .add_systems(OnEnter(CreditsState::Active), show_credits)
            .add_systems(OnExit(CreditsState::Active), hide_credits)
            .add_systems(
                Update,
                scroll_credits.run_if(in_state(CreditsState::Active)),
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
                                font_size: header_font_size,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        ));
                        scroll_parent.spawn((
                            Text::new(&section.text),
                            TextFont {
                                font_size,
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
        });
}

fn hide_credits(mut commands: Commands, query: Query<Entity, With<CreditsText>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
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
