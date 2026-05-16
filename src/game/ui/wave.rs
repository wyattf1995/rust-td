use super::*;

pub(super) fn start_wave_button(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<StartWaveButton>),
    >,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut wave_manager: ResMut<WaveManager>,
    mut economy: ResMut<PlayerEconomy>,
    assets: Res<GameAssets>,
    time: Res<Time>,
    existing_announcements: Query<Entity, With<WaveAnnouncement>>,
    mut ui_clicked: ResMut<UiClicked>,
    mut stats: ResMut<super::super::stats::GameStats>,
) {
    let mut should_start = false;

    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = GameColors::BUTTON_START_PRESSED.into();
                ui_clicked.0 = true;
                should_start = true;
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_START_HOVER.into();
                ui_clicked.0 = true;
            }
            Interaction::None => {
                *color = GameColors::BUTTON_START.into();
            }
        }
    }

    // Space to send wave
    if keyboard.just_pressed(KeyCode::Space) {
        should_start = true;
    }

    if should_start && !wave_manager.wave_active {
        // Calculate early-send bonus before starting wave
        let early_bonus = wave_manager.early_send_bonus(time.elapsed_secs_f64());

        wave_manager.start_wave();
        super::super::announce(&format!("Wave {} started", wave_manager.current_wave + 1));

        // Award early-send bonus
        if early_bonus > 0 {
            economy.gold = economy.gold.saturating_add(early_bonus);
            economy.score = economy.score.saturating_add(early_bonus);
            stats.record_early_send(early_bonus);
        }

        // Despawn any existing announcement
        for entity in &existing_announcements {
            commands.entity(entity).despawn_recursive();
        }

        // Spawn wave announcement
        let wave_num = wave_manager.current_wave + 1;
        let modifier_name = wave_manager.current_modifier.name();

        commands.spawn((
            NodeBundle {
                node: Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(35.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-120.0)),
                    width: Val::Px(240.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
                ..default()
            },
            WaveAnnouncement {
                lifetime: Timer::from_seconds(1.5, TimerMode::Once),
            },
            GameEntity,
        )).with_children(|parent| {
            parent.spawn((
                Text::new(format!("WAVE {}", wave_num)),
                TextFont {
                    font: assets.font.clone(),
                    font_size: 40.0,
                    ..default()
                },
                TextColor(GameColors::PRIMARY),
            ));
            if early_bonus > 0 {
                parent.spawn((
                    Text::new(format!("+{}g EARLY BONUS", early_bonus)),
                    TextFont {
                        font: assets.font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(GameColors::GOLD),
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ));
            }
            // Show previous wave's income breakdown (interest + bonus)
            let interest = wave_manager.last_interest;
            let bonus = wave_manager.last_bonus;
            if interest > 0 || bonus > 0 {
                let mut income_parts = Vec::new();
                if bonus > 0 { income_parts.push(format!("+{}g bonus", bonus)); }
                if interest > 0 { income_parts.push(format!("+{}g interest", interest)); }
                let income_text = income_parts.join("  ");

                parent.spawn((
                    Text::new(income_text),
                    TextFont {
                        font: assets.font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(GameColors::GOLD),
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ));
            }
            if !modifier_name.is_empty() {
                parent.spawn((
                    Text::new(modifier_name),
                    TextFont {
                        font: assets.font.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(GameColors::GOLD),
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ));
            }
        });
    }
}

pub(super) fn update_wave_preview(
    wave_manager: Res<WaveManager>,
    mut panel_query: Query<&mut Node, With<WavePreviewPanel>>,
    mut text_query: Query<&mut Text, With<WavePreviewText>>,
    mut cached_wave: Local<usize>,
    mut cached_text: Local<String>,
) {
    for mut style in &mut panel_query {
        if wave_manager.wave_active {
            style.display = Display::None;
            continue;
        }

        style.display = Display::Flex;

        // Only recompute preview when wave changes
        let next_wave = wave_manager.current_wave + 1;
        if *cached_wave != next_wave {
            *cached_wave = next_wave;

            let preview = wave_manager.preview_next_wave();
            let mut text = format!("NEXT: Wave {}", next_wave);

            if preview.modifier != WaveModifier::None {
                text.push_str(&format!("\n{}", preview.modifier.name()));
            }

            // Show fuzzy counts per enemy type with brief descriptions
            for (etype, count) in &preview.counts {
                let fuzzy = fuzzy_count(*count);
                let name = match etype {
                    EnemyType::Basic => "Basic",
                    EnemyType::Fast => "Fast (quick)",
                    EnemyType::Tank => "Tank (tough)",
                    EnemyType::Armored => "Armored (resists)",
                    EnemyType::Flying => "Flying (skips path)",
                    EnemyType::Boss => "BOSS (high HP)",
                    EnemyType::Splitter => "Splitter",
                    EnemyType::MiniSplitter => "Mini",
                };
                text.push_str(&format!("\n  {} x{}", name, fuzzy));
            }

            *cached_text = text;
        }

        for mut text in &mut text_query {
            text.0 = cached_text.clone();
        }
    }
}

/// Display fuzzy counts: 5+, 10+, 15+, 25+, 30+
fn fuzzy_count(count: usize) -> &'static str {
    match count {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4..=7 => "5+",
        8..=12 => "10+",
        13..=18 => "15+",
        19..=27 => "25+",
        _ => "30+",
    }
}

pub(super) fn update_start_button_text(
    wave_manager: Res<WaveManager>,
    time: Res<Time>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<StartWaveButtonText>>,
    mut button_query: Query<&mut BackgroundColor, (With<StartWaveButton>, Without<StartWaveButtonText>)>,
) {
    let active = wave_manager.wave_active;

    for (mut text, mut text_color) in &mut text_query {
        let new_value = if active {
            "IN PROGRESS".to_string()
        } else {
            let bonus = wave_manager.early_send_bonus(time.elapsed_secs_f64());
            if bonus > 0 { format!("START +{}g", bonus) } else { "START".into() }
        };
        if text.0 != new_value {
            text.0 = new_value;
        }
        // Dim text during active wave
        let target_color = if active { GameColors::TEXT_MEDIUM } else { Color::WHITE };
        if text_color.0 != target_color {
            text_color.0 = target_color;
        }
    }

    // Dim the button background during active wave
    for mut color in &mut button_query {
        let target: BackgroundColor = if active {
            Color::srgb(0.1, 0.2, 0.15).into()
        } else {
            GameColors::BUTTON_START.into()
        };
        if *color != target {
            *color = target;
        }
    }
}

pub(super) fn update_wave_announcement(
    mut commands: Commands,
    mut announcements: Query<(Entity, &mut WaveAnnouncement, &Children)>,
    mut text_query: Query<&mut TextColor, Without<StartWaveButtonText>>,
    time: Res<Time>,
) {
    for (entity, mut announcement, children) in &mut announcements {
        announcement.lifetime.tick(time.delta());
        let progress = announcement.lifetime.fraction();

        // Fade out text alpha
        let alpha = if progress < 0.3 {
            // Fade in quickly
            progress / 0.3
        } else if progress > 0.7 {
            // Fade out
            1.0 - (progress - 0.7) / 0.3
        } else {
            1.0
        };

        // Update all child text alphas
        for &child in children.iter() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                let base_color = text_color.0;
                text_color.0 = base_color.with_alpha(alpha);
            }
        }

        if announcement.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
