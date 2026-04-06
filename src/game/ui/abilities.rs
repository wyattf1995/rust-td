use super::*;

pub(super) fn ability_button_activate(
    button_query: Query<(&Interaction, &AbilityButton), Changed<Interaction>>,
    mut abilities: ResMut<PlayerAbilities>,
    mut freeze_events: EventWriter<FreezeAbilityEvent>,
    mut gold_rush_events: EventWriter<GoldRushAbilityEvent>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    for (interaction, button) in &button_query {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                match button.0 {
                    AbilityType::Freeze => {
                        if abilities.freeze_ready {
                            freeze_events.send(FreezeAbilityEvent);
                            abilities.freeze_ready = false;
                        }
                    }
                    AbilityType::GoldRush => {
                        if abilities.gold_rush_ready {
                            gold_rush_events.send(GoldRushAbilityEvent);
                            abilities.gold_rush_ready = false;
                        }
                    }
                    AbilityType::Artillery => {
                        if abilities.artillery_ready {
                            abilities.artillery_targeting = !abilities.artillery_targeting;
                        }
                    }
                }
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
            }
            // BackgroundColor managed by update_ability_display based on cooldown state
            Interaction::None => {}
        }
    }
}

pub(super) fn update_ability_display(
    abilities: Res<PlayerAbilities>,
    mut button_query: Query<(&AbilityButton, &Interaction, &mut BackgroundColor, &mut BorderColor)>,
    mut cooldown_text_query: Query<(&AbilityCooldownText, &mut Text)>,
) {
    for (button, interaction, mut bg_color, mut border_color) in &mut button_query {
        let (ready, active, _remaining) = ability_state(&abilities, button.0);
        let hovered = *interaction == Interaction::Hovered || *interaction == Interaction::Pressed;

        // Update button opacity based on ready state + hover
        let base_color = match button.0 {
            AbilityType::Freeze => GameColors::ABILITY_FREEZE,
            AbilityType::GoldRush => GameColors::ABILITY_GOLD_RUSH,
            AbilityType::Artillery => GameColors::ABILITY_NUKE,
        };

        if ready {
            let alpha = if hovered { 0.65 } else { 0.5 };
            *bg_color = base_color.with_alpha(alpha).into();
            *border_color = BorderColor(base_color.with_alpha(0.6));
        } else if active {
            *bg_color = base_color.with_alpha(0.8).into();
            *border_color = BorderColor(base_color.with_alpha(0.8));
        } else {
            let alpha = if hovered { 0.3 } else { 0.2 };
            *bg_color = base_color.with_alpha(alpha).into();
            *border_color = BorderColor(Color::NONE);
        }
    }

    for (cooldown_text, mut text) in &mut cooldown_text_query {
        let (ready, active, remaining) = ability_state(&abilities, cooldown_text.0);

        if ready {
            text.sections[0].value = "Ready".to_string();
            text.sections[0].style.color = GameColors::SUCCESS;
        } else if active {
            text.sections[0].value = format!("{:.0}s", remaining);
            text.sections[0].style.color = GameColors::GOLD;
        } else {
            text.sections[0].value = format!("{:.0}s", remaining);
            text.sections[0].style.color = GameColors::TEXT_MEDIUM;
        }
    }
}

pub(super) fn update_ability_tooltips(
    mut commands: Commands,
    assets: Res<GameAssets>,
    button_query: Query<(&Interaction, &AbilityButton)>,
    tooltip_query: Query<Entity, With<AbilityTooltip>>,
) {
    // Find which ability (if any) is hovered
    let mut hovered: Option<AbilityType> = None;
    for (interaction, button) in &button_query {
        if *interaction == Interaction::Hovered {
            hovered = Some(button.0);
        }
    }

    // Despawn existing tooltip
    for entity in &tooltip_query {
        commands.entity(entity).despawn_recursive();
    }

    // Spawn new tooltip if hovering
    if let Some(ability) = hovered {
        let (title, desc) = ability.tooltip();

        commands.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(100.0),
                    top: Val::Px(60.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    max_width: Val::Px(200.0),
                    ..default()
                },
                background_color: GameColors::OVERLAY_TOOLTIP.into(),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            AbilityTooltip,
            GameEntity,
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                title,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color: Color::WHITE,
                },
            ));
            parent.spawn(TextBundle::from_section(
                desc,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 11.0,
                    color: GameColors::TEXT_BRIGHT,
                },
            ));
        });
    }
}

pub(super) fn adapt_ability_bar_layout(
    screen_info: Res<ScreenInfo>,
    mut ability_bar: Query<&mut Style, With<AbilityBar>>,
    mut ability_buttons: Query<&mut Style, (With<AbilityButton>, Without<AbilityBar>)>,
) {
    if !screen_info.is_changed() { return; }

    let Ok(mut bar_style) = ability_bar.get_single_mut() else { return };

    if screen_info.is_landscape {
        // Landscape mobile: horizontal bar at top-right to avoid blocking grid
        bar_style.flex_direction = FlexDirection::Row;
        bar_style.column_gap = Val::Px(6.0);
        bar_style.row_gap = Val::Px(0.0);
        bar_style.left = Val::Auto;
        bar_style.right = Val::Px(10.0);
        bar_style.top = Val::Px(60.0);

        // Compact ability buttons for landscape
        for mut btn_style in &mut ability_buttons {
            btn_style.width = Val::Px(90.0);
        }
    } else {
        // Default: vertical column on the left
        bar_style.flex_direction = FlexDirection::Column;
        bar_style.column_gap = Val::Px(0.0);
        bar_style.row_gap = Val::Px(6.0);
        bar_style.left = Val::Px(10.0);
        bar_style.right = Val::Auto;
        bar_style.top = Val::Px(60.0);

        for mut btn_style in &mut ability_buttons {
            btn_style.width = Val::Px(110.0);
        }
    }
}
