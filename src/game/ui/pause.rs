use super::*;

/// Auto-pause when the browser tab loses focus.
/// Does NOT auto-resume on regain — player must resume manually via pause menu or ESC.
pub(super) fn pause_on_blur(
    mut events: EventReader<bevy::window::WindowFocused>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
) {
    for event in events.read() {
        if !event.focused && *state.get() == GameState::Playing {
            next_state.set(GameState::Paused);
        }
    }
}

pub(super) fn pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    selected_tower: Res<SelectedPlacedTower>,
    mut pause_button: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<HudPauseButton>)>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
    mut synergy_button: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<HudSynergyButton>, Without<HudPauseButton>)>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    // "?" button with hover feedback
    for (interaction, mut color) in &mut synergy_button {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                synergy_open.0 = !synergy_open.0;
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
                *color = GameColors::BUTTON_HOVER.into();
            }
            Interaction::None => {
                *color = GameColors::BUTTON_NORMAL.into();
            }
        }
    }
    // ESC: close synergy panel first, then deselect tower, then pause
    if keyboard.just_pressed(KeyCode::Escape) {
        if synergy_open.0 {
            synergy_open.0 = false;
        } else if selected_tower.0.is_none() {
            next_state.set(GameState::Paused);
        }
        return;
    }
    // Pause via HUD button click with hover feedback
    for (interaction, mut color) in &mut pause_button {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                next_state.set(GameState::Paused);
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
                *color = GameColors::BUTTON_HOVER.into();
            }
            Interaction::None => {
                *color = GameColors::BUTTON_NORMAL.into();
            }
        }
    }
}

pub(super) fn pause_input_resume(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if synergy_open.0 {
            synergy_open.0 = false;
        } else {
            next_state.set(GameState::Playing);
        }
    }
}

pub(super) fn setup_pause_menu(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            NodeBundle {
                node: Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: GameColors::OVERLAY_DARK.into(),
                ..default()
            },
            PauseMenu,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font: assets.font.clone(),
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Resume button
            parent
                .spawn((
                    ButtonBundle {
                        node: Node {
                            width: Val::Px(200.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                        background_color: GameColors::BUTTON_START.into(),
                        ..default()
                    },
                    ResumeButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("RESUME"),
                        TextFont {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Settings button
            parent
                .spawn((
                    ButtonBundle {
                        node: Node {
                            width: Val::Px(200.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                        background_color: GameColors::BUTTON_GHOST.into(),
                        ..default()
                    },
                    PauseSettingsButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("SETTINGS"),
                        TextFont {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                    ));
                });

            // Synergies button
            parent
                .spawn((
                    ButtonBundle {
                        node: Node {
                            width: Val::Px(200.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                        background_color: GameColors::BUTTON_GHOST.into(),
                        ..default()
                    },
                    PauseSynergyButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("SYNERGIES"),
                        TextFont {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(GameColors::SYNERGY),
                    ));
                });

            // Quit button
            parent
                .spawn((
                    ButtonBundle {
                        node: Node {
                            width: Val::Px(200.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::BUTTON_SELL.into(),
                        ..default()
                    },
                    QuitButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("QUIT TO MENU"),
                        TextFont {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Hint text
            parent.spawn((
                Text::new("Press ESC to resume"),
                TextFont {
                    font: assets.font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(GameColors::TEXT_MEDIUM),
                Node {
                    margin: UiRect::top(Val::Px(30.0)),
                    ..default()
                },
            ));
        });
}

pub(super) fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

pub(super) fn pause_menu_buttons(
    mut resume_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ResumeButton>),
    >,
    mut quit_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitButton>, Without<ResumeButton>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color) in &mut resume_query {
        if button_interaction(interaction, &mut color, GameColors::BUTTON_START, GameColors::BUTTON_START_HOVER) {
            next_state.set(GameState::Playing);
        }
    }

    for (interaction, mut color) in &mut quit_query {
        if button_interaction(interaction, &mut color, GameColors::BUTTON_SELL, GameColors::BUTTON_SELL_HOVER) {
            next_state.set(GameState::Menu);
        }
    }
}

pub(super) fn pause_settings_button(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PauseSettingsButton>)>,
    mut settings_open: ResMut<SettingsOpen>,
) {
    for (interaction, mut color) in &mut query {
        if button_interaction(interaction, &mut color, GameColors::BUTTON_GHOST, GameColors::BUTTON_GHOST_HOVER) {
            settings_open.0 = true;
        }
    }
}

pub(super) fn pause_synergy_button(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PauseSynergyButton>)>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
) {
    for (interaction, mut color) in &mut query {
        if button_interaction(interaction, &mut color, GameColors::BUTTON_GHOST, GameColors::BUTTON_GHOST_HOVER) {
            synergy_open.0 = true;
        }
    }
}
