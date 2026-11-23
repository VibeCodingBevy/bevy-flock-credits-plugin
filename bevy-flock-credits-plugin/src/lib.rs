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

/// Lifecycle request written by the host and consumed by the plugin.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditsRequest {
    /// Start the roll from the top. Restarts from the top if already active.
    Start,
    /// Stop the roll, tear down the UI, and emit [`CreditsFinished`].
    Stop,
}

/// Emitted exactly once on every `Active -> Idle` transition.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreditsFinished {
    pub reason: CreditsFinishedReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditsFinishedReason {
    /// The roll scrolled past the last line.
    Completed,
    /// The viewer held the skip key long enough.
    Skipped,
    /// The host requested [`CreditsRequest::Stop`].
    Cancelled,
}

#[derive(Component)]
struct CreditsText;

#[derive(Component)]
struct ScrollTarget {
    total_height: f32,
}

#[derive(Component)]
struct SkipCountdown;

/// Skip hold state plus the reason for the pending `Active -> Idle` transition.
#[derive(Resource, Default)]
struct RollProgress {
    held_secs: f32,
    pending_finish: Option<CreditsFinishedReason>,
}

impl RollProgress {
    /// First writer wins. `handle_requests` runs first, so `Cancelled` has priority.
    fn set_pending_finish(&mut self, reason: CreditsFinishedReason) {
        if self.pending_finish.is_none() {
            self.pending_finish = Some(reason);
        }
    }
}

pub struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CreditsRequest>()
            .add_message::<CreditsFinished>()
            .init_state::<CreditsState>()
            .init_resource::<RollProgress>()
            .add_systems(
                OnEnter(CreditsState::Active),
                (show_credits, reset_skip_hold),
            )
            .add_systems(OnExit(CreditsState::Active), hide_credits)
            .add_systems(
                OnTransition {
                    exited: CreditsState::Active,
                    entered: CreditsState::Idle,
                },
                emit_finished,
            )
            .add_systems(
                Update,
                handle_requests.before(scroll_credits).before(hold_to_skip),
            )
            .add_systems(
                Update,
                (scroll_credits, hold_to_skip)
                    .chain()
                    .run_if(in_state(CreditsState::Active)),
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

    let mut root = commands.spawn((
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
    ));
    if let Some([r, g, b, a]) = credits_config.background {
        root.insert(BackgroundColor(Color::srgba(r, g, b, a)));
    }
    root.with_children(|parent| {
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

fn reset_skip_hold(mut progress: ResMut<RollProgress>) {
    progress.held_secs = 0.0;
}

/// Translates host intent into state writes. Only the last request in a buffer wins.
fn handle_requests(
    mut requests: MessageReader<CreditsRequest>,
    state: Res<State<CreditsState>>,
    mut next_state: ResMut<NextState<CreditsState>>,
    mut progress: ResMut<RollProgress>,
) {
    let Some(request) = requests.read().last().copied() else {
        return;
    };

    let pending = match &*next_state {
        NextState::Pending(pending) | NextState::PendingIfDifferent(pending) => Some(*pending),
        NextState::Unchanged => None,
    };
    let current = *state.get();

    match request {
        CreditsRequest::Start => {
            progress.pending_finish = None;
            // `set` always runs the exit/enter schedules, even for an identity
            // transition, so this also restarts an already active roll.
            next_state.set(CreditsState::Active);
        }
        CreditsRequest::Stop => match (current, pending) {
            // A finish is already queued; keep its reason.
            (_, Some(CreditsState::Idle)) => {}
            // The start has not been applied yet; drop it without emitting a finish.
            (CreditsState::Idle, Some(CreditsState::Active)) => next_state.reset(),
            (CreditsState::Active, _) => {
                progress.set_pending_finish(CreditsFinishedReason::Cancelled);
                next_state.set(CreditsState::Idle);
            }
            (CreditsState::Idle, None) => {}
        },
    }
}

fn emit_finished(mut finished: MessageWriter<CreditsFinished>, mut progress: ResMut<RollProgress>) {
    let reason = progress
        .pending_finish
        .take()
        .unwrap_or(CreditsFinishedReason::Completed);
    finished.write(CreditsFinished { reason });
}

fn scroll_credits(
    time: Res<Time>,
    credits_config: Res<CreditsConfig>,
    mut progress: ResMut<RollProgress>,
    mut next_state: ResMut<NextState<CreditsState>>,
    mut query: Query<(&mut Node, &ScrollTarget)>,
) {
    for (mut node, scroll_target) in query.iter_mut() {
        if let Val::Px(top) = &mut node.top {
            *top -= credits_config.credits_speed * time.delta_secs();

            if *top < -scroll_target.total_height {
                progress.set_pending_finish(CreditsFinishedReason::Completed);
                next_state.set(CreditsState::Idle);
            }
        }
    }
}

fn hold_to_skip(
    time: Res<Time>,
    credits_config: Res<CreditsConfig>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut progress: ResMut<RollProgress>,
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
        progress.held_secs += time.delta_secs();
    } else {
        progress.held_secs = 0.0;
    }

    if holding && progress.held_secs >= hold_secs {
        progress.set_pending_finish(CreditsFinishedReason::Skipped);
        next_state.set(CreditsState::Idle);
    }

    let remaining = (hold_secs - progress.held_secs).max(0.0).ceil() as u32;
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

    #[derive(Resource, Default)]
    struct RecordedFinishes(Vec<CreditsFinishedReason>);

    fn record_finishes(
        mut reader: MessageReader<CreditsFinished>,
        mut recorded: ResMut<RecordedFinishes>,
    ) {
        for finished in reader.read() {
            recorded.0.push(finished.reason);
        }
    }

    fn app_without_start() -> App {
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
                background: None,
                sections: vec![CreditsSection {
                    header: "Header".into(),
                    text: "body".into(),
                }],
            })
            .init_resource::<RecordedFinishes>()
            .add_plugins(CreditsPlugin)
            .add_systems(Update, record_finishes);

        app
    }

    fn app() -> App {
        let mut app = app_without_start();
        app.world_mut()
            .resource_mut::<NextState<CreditsState>>()
            .set(CreditsState::Active);
        app
    }

    fn request(app: &mut App, request: CreditsRequest) {
        app.world_mut().write_message(request);
    }

    /// One frame for the handling system, one for the state transition and finish emission.
    fn settle(app: &mut App) {
        app.update();
        app.update();
    }

    fn is_active(app: &App) -> bool {
        *app.world().resource::<State<CreditsState>>().get() == CreditsState::Active
    }

    fn root_count(app: &mut App) -> usize {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<CreditsText>>();
        query.iter(app.world()).count()
    }

    fn finishes(app: &App) -> Vec<CreditsFinishedReason> {
        app.world().resource::<RecordedFinishes>().0.clone()
    }

    fn held_secs(app: &App) -> f32 {
        app.world().resource::<RollProgress>().held_secs
    }

    fn background(app: &mut App) -> Option<BackgroundColor> {
        let mut query = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<CreditsText>>();
        query.single(app.world()).ok().cloned()
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
        assert_eq!(finishes(&app), vec![CreditsFinishedReason::Skipped]);
    }

    #[test]
    fn start_from_idle_spawns_one_root_without_finishing() {
        let mut app = app_without_start();
        app.update();
        assert!(!is_active(&app));

        request(&mut app, CreditsRequest::Start);
        settle(&mut app);

        assert!(is_active(&app));
        assert_eq!(root_count(&mut app), 1);
        assert!(finishes(&app).is_empty());
    }

    #[test]
    fn start_while_active_restarts_without_finishing() {
        let mut app = app();
        settle(&mut app);
        assert_eq!(root_count(&mut app), 1);

        send_key(&mut app, ButtonState::Pressed);
        app.update();
        assert!(held_secs(&app) > 0.0);

        send_key(&mut app, ButtonState::Released);
        request(&mut app, CreditsRequest::Start);
        settle(&mut app);

        assert!(is_active(&app));
        assert_eq!(root_count(&mut app), 1);
        assert_eq!(held_secs(&app), 0.0);
        assert!(finishes(&app).is_empty());
    }

    #[test]
    fn stop_while_active_cancels_and_despawns() {
        let mut app = app();
        settle(&mut app);
        assert_eq!(root_count(&mut app), 1);

        request(&mut app, CreditsRequest::Stop);
        settle(&mut app);

        assert!(!is_active(&app));
        assert_eq!(root_count(&mut app), 0);
        assert_eq!(finishes(&app), vec![CreditsFinishedReason::Cancelled]);
    }

    #[test]
    fn stop_while_idle_is_a_noop() {
        let mut app = app_without_start();
        app.update();

        request(&mut app, CreditsRequest::Stop);
        settle(&mut app);

        assert!(!is_active(&app));
        assert_eq!(root_count(&mut app), 0);
        assert!(finishes(&app).is_empty());
    }

    #[test]
    fn last_request_in_a_buffer_wins() {
        let mut app = app_without_start();
        app.update();

        request(&mut app, CreditsRequest::Start);
        request(&mut app, CreditsRequest::Stop);
        settle(&mut app);

        assert!(!is_active(&app));
        assert_eq!(root_count(&mut app), 0);
        assert!(finishes(&app).is_empty());
    }

    #[test]
    fn natural_completion_emits_completed() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CreditsConfig>()
            .credits_speed = 1000.0;

        for _ in 0..60 {
            app.update();
        }

        assert!(!is_active(&app));
        assert_eq!(finishes(&app), vec![CreditsFinishedReason::Completed]);
    }

    #[test]
    fn hold_to_skip_emits_skipped() {
        let mut app = app();
        app.update();

        send_key(&mut app, ButtonState::Pressed);
        for _ in 0..200 {
            app.update();
        }

        assert!(!is_active(&app));
        assert_eq!(finishes(&app), vec![CreditsFinishedReason::Skipped]);
    }

    #[test]
    fn no_finish_is_emitted_at_startup() {
        let mut app = app_without_start();
        for _ in 0..5 {
            app.update();
        }

        assert_eq!(root_count(&mut app), 0);
        assert!(finishes(&app).is_empty());
    }

    #[test]
    fn legacy_state_writes_keep_working() {
        let mut app = app();
        settle(&mut app);

        // Legacy restart: `set` forces an identity transition, but must not emit a finish.
        app.world_mut()
            .resource_mut::<NextState<CreditsState>>()
            .set(CreditsState::Active);
        settle(&mut app);
        assert!(is_active(&app));
        assert_eq!(root_count(&mut app), 1);
        assert!(finishes(&app).is_empty());

        // Legacy completion emits `Completed`.
        app.world_mut()
            .resource_mut::<NextState<CreditsState>>()
            .set(CreditsState::Idle);
        settle(&mut app);
        assert!(!is_active(&app));
        assert_eq!(root_count(&mut app), 0);
        assert_eq!(finishes(&app), vec![CreditsFinishedReason::Completed]);
    }

    #[test]
    fn background_is_applied_only_when_configured() {
        let mut app = app_without_start();
        request(&mut app, CreditsRequest::Start);
        settle(&mut app);
        assert_eq!(background(&mut app), Some(BackgroundColor(Color::NONE)));

        let mut app = app_without_start();
        app.world_mut().resource_mut::<CreditsConfig>().background = Some([0.0, 0.0, 0.0, 1.0]);
        request(&mut app, CreditsRequest::Start);
        settle(&mut app);
        assert_eq!(
            background(&mut app),
            Some(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)))
        );
    }
}
