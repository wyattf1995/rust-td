use super::*;

pub(super) fn update_gold_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<GoldText>>,
) {
    if economy.is_changed() {
        for mut text in &mut query {
            text.sections[0].value = format!("{}", economy.gold);
        }
    }
}

pub(super) fn update_gold_reject_flash(
    mut flash: ResMut<GoldRejectFlash>,
    mut query: Query<&mut Text, With<GoldText>>,
    time: Res<Time>,
) {
    if let Some(ref mut timer) = flash.timer {
        timer.tick(time.delta());
        let t = timer.fraction();
        let flash_color = GameColors::HEALTH_LOW.mix(&GameColors::GOLD, t);
        for mut text in &mut query {
            text.sections[0].style.color = flash_color;
        }
        if timer.finished() {
            flash.timer = None;
        }
    }
}

pub(super) fn update_gold_rush_indicator(
    abilities: Res<PlayerAbilities>,
    mut indicator_query: Query<(&mut Text, &mut Style), With<GoldRushIndicator>>,
    mut gold_text_query: Query<&mut Text, (With<GoldText>, Without<GoldRushIndicator>)>,
    time: Res<Time>,
    settings: Res<crate::persistence::GameSettings>,
) {
    let is_active = abilities.gold_rush_active.is_some();
    let remaining = abilities.gold_rush_active.as_ref().map(|t| t.remaining_secs()).unwrap_or(0.0);

    for (mut text, mut style) in &mut indicator_query {
        if is_active {
            style.display = Display::Flex;
            text.sections[0].value = format!("2x {:.0}s", remaining);

            let alpha = if settings.reduce_motion {
                1.0
            } else {
                let pulse = (time.elapsed_seconds() * 4.0).sin() * 0.5 + 0.5;
                0.7 + pulse * 0.3
            };
            text.sections[0].style.color = GameColors::ABILITY_GOLD_RUSH.with_alpha(alpha);
        } else {
            style.display = Display::None;
        }
    }

    // Pulse the gold amount text while Gold Rush is active
    for mut text in &mut gold_text_query {
        if is_active {
            if settings.reduce_motion {
                text.sections[0].style.color = GameColors::GOLD;
            } else {
                let pulse = (time.elapsed_seconds() * 3.0).sin() * 0.5 + 0.5;
                let r = GameColors::GOLD.to_srgba().red * (0.85 + pulse * 0.15);
                let g = GameColors::GOLD.to_srgba().green * (0.85 + pulse * 0.15);
                let b = GameColors::GOLD.to_srgba().blue * (0.85 + pulse * 0.15);
                text.sections[0].style.color = Color::srgb(r.min(1.0), g.min(1.0), b.min(1.0));
            }
        } else {
            text.sections[0].style.color = GameColors::GOLD;
        }
    }
}

pub(super) fn update_lives_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<LivesText>>,
    mut flash: ResMut<LifeLostFlash>,
) {
    if economy.is_changed() {
        // Detect life loss and trigger flash
        if flash.previous_lives > 0 && economy.lives < flash.previous_lives {
            flash.timer = Some(Timer::from_seconds(0.4, TimerMode::Once));
        }
        flash.previous_lives = economy.lives;

        for mut text in &mut query {
            text.sections[0].value = format!("{}", economy.lives);
        }
    }
}

pub(super) fn update_life_flash(
    mut flash: ResMut<LifeLostFlash>,
    mut query: Query<&mut Text, With<LivesText>>,
    time: Res<Time>,
) {
    if let Some(ref mut timer) = flash.timer {
        timer.tick(time.delta());
        let progress = timer.fraction();

        for mut text in &mut query {
            // Lerp from bright red back to primary color
            let flash_color = GameColors::HEALTH_LOW;
            let normal_color = GameColors::PRIMARY;
            let r = flash_color.to_srgba().red + (normal_color.to_srgba().red - flash_color.to_srgba().red) * progress;
            let g = flash_color.to_srgba().green + (normal_color.to_srgba().green - flash_color.to_srgba().green) * progress;
            let b = flash_color.to_srgba().blue + (normal_color.to_srgba().blue - flash_color.to_srgba().blue) * progress;
            text.sections[0].style.color = Color::srgb(r, g, b);
        }

        if timer.finished() {
            flash.timer = None;
        }
    }
}

pub(super) fn update_wave_display(
    wave_manager: Res<WaveManager>,
    mut query: Query<&mut Text, With<WaveText>>,
) {
    if !wave_manager.is_changed() { return; }
    for mut text in &mut query {
        let current = wave_manager.current_wave + 1;
        let status = if wave_manager.wave_active { " \u{2694}" } else { "" };
        // Show health multiplier as difficulty indicator
        let difficulty = format!(" ({:.1}x)", wave_manager.health_multiplier);

        // Show wave modifier if active
        let modifier: &str = match wave_manager.current_modifier {
            WaveModifier::None => "",
            WaveModifier::SpeedBoost => " [SPEED]",
            WaveModifier::ArmoredWave => " [ARMOR]",
            WaveModifier::Swarm => " [SWARM]",
            WaveModifier::Regen => " [REGEN]",
            WaveModifier::GoldRush => " [GOLD!]",
        };

        text.sections[0].value = format!("Wave {}{}{}{}", current, difficulty, modifier, status);
    }
}

pub(super) fn update_combo_display(
    kill_streak: Res<KillStreak>,
    mut display_query: Query<&mut Style, With<ComboDisplay>>,
    mut text_query: Query<&mut Text, With<ComboText>>,
) {
    if !kill_streak.is_changed() { return; }
    for mut style in &mut display_query {
        // Show combo display when streak is 3+
        style.display = if kill_streak.count >= 3 && kill_streak.active {
            Display::Flex
        } else {
            Display::None
        };
    }

    for mut text in &mut text_query {
        if kill_streak.count >= 3 {
            text.sections[0].value = format!("COMBO x{}", kill_streak.count);

            // Color changes based on combo level
            text.sections[0].style.color = if kill_streak.count >= 10 {
                Color::srgb(1.0, 0.3, 0.3) // Red for 10+
            } else if kill_streak.count >= 6 {
                Color::srgb(1.0, 0.6, 0.0) // Orange for 6+
            } else {
                GameColors::GOLD // Yellow for 3+
            };
        }
    }
}

pub(super) fn update_score_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<ScoreText>>,
) {
    if economy.is_changed() {
        for mut text in &mut query {
            text.sections[0].value = format!("Score: {}", economy.score);
        }
    }
}

pub(super) fn speed_button_system(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor, &SpeedButton, &Children),
        With<SpeedButton>,
    >,
    mut game_speed: ResMut<GameSpeed>,
    mut text_query: Query<&mut Text>,
    mut time: ResMut<Time<Virtual>>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    let active_bg: BackgroundColor = GameColors::BUTTON_ACTIVE.into();
    let inactive_bg: BackgroundColor = GameColors::BUTTON_NORMAL.into();
    let hover_bg: BackgroundColor = GameColors::BUTTON_HOVER.into();

    // First pass: detect press and update game speed
    let mut new_speed: Option<f32> = None;
    for (interaction, _, _, speed_button, _) in &button_query {
        if *interaction == Interaction::Pressed {
            new_speed = Some(speed_button.0);
            ui_clicked.0 = true;
        } else if *interaction == Interaction::Hovered {
            ui_clicked.0 = true;
        }
    }

    if let Some(speed) = new_speed {
        game_speed.0 = speed;
        time.set_relative_speed(speed);
    }

    // Second pass: update all button visuals based on current game speed
    for (interaction, mut color, mut border, speed_button, children) in &mut button_query {
        let is_active = speed_button.0 == game_speed.0;

        if is_active {
            *color = active_bg;
            *border = BorderColor(GameColors::PRIMARY);
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color = GameColors::PRIMARY;
                }
            }
        } else if *interaction == Interaction::Hovered {
            *color = hover_bg;
            *border = BorderColor(Color::NONE);
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color = GameColors::TEXT_MEDIUM;
                }
            }
        } else {
            *color = inactive_bg;
            *border = BorderColor(Color::NONE);
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color = GameColors::TEXT_MEDIUM;
                }
            }
        }
    }
}

/// Show/hide the "Z = Undo" hint based on RecentPlacement timer.
pub(super) fn update_undo_hint(
    mut commands: Commands,
    recent: Res<RecentPlacement>,
    mut hint_query: Query<(Entity, &mut Text, &mut Style), With<UndoHintText>>,
    assets: Res<GameAssets>,
) {
    let is_active = recent.timer.as_ref().is_some_and(|t| !t.finished());

    if is_active {
        let remaining = recent.timer.as_ref().map_or(0.0, |t| t.remaining_secs());
        let label = format!("Z = Undo ({:.0}s)", remaining.ceil());

        if let Ok((_entity, mut text, mut style)) = hint_query.get_single_mut() {
            // Update existing hint
            text.sections[0].value = label;
            style.display = Display::Flex;
            // Fade out in last second
            let alpha = if remaining < 1.0 { remaining } else { 0.7 };
            text.sections[0].style.color = Color::srgba(0.4, 1.0, 0.5, alpha);
        } else {
            // Spawn hint text (positioned bottom-center, above ability bar)
            commands.spawn((
                TextBundle::from_section(
                    label,
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 14.0,
                        color: Color::srgba(0.4, 1.0, 0.5, 0.7),
                    },
                ).with_style(Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(120.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-40.0)),
                    display: Display::Flex,
                    ..default()
                }),
                UndoHintText,
                super::GameEntity,
            ));
        }
    } else {
        // Hide hint when not active
        for (_entity, _text, mut style) in &mut hint_query {
            style.display = Display::None;
        }
    }
}
