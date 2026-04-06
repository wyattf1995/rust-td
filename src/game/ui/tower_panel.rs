use super::*;

pub(super) fn tower_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &TowerButton),
        Changed<Interaction>,
    >,
    mut selected: ResMut<SelectedTowerType>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    for (interaction, mut color, tower_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                selected.0 = tower_button.0;
                *color = GameColors::BUTTON_SELECTED.into();
                ui_clicked.0 = true;
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_HOVER.into();
                ui_clicked.0 = true;
            }
            Interaction::None => {
                if selected.0 == tower_button.0 {
                    *color = GameColors::BUTTON_SELECTED.into();
                } else {
                    *color = GameColors::BUTTON_NORMAL.into();
                }
            }
        }
    }
}

pub(super) fn update_tower_selection(
    selected: Res<SelectedTowerType>,
    mut borders: Query<(&TowerButtonBorder, &mut BorderColor)>,
) {
    if selected.is_changed() {
        for (border, mut color) in &mut borders {
            if border.0 == selected.0 {
                *color = BorderColor(border.0.color());
            } else {
                *color = BorderColor(Color::NONE);
            }
        }
    }
}

pub(super) fn update_tower_affordability(
    economy: Res<PlayerEconomy>,
    button_query: Query<(&TowerButton, &Interaction, &Children)>,
    mut cost_text_query: Query<&mut Text, With<TowerCostText>>,
) {
    for (tower_button, interaction, children) in &button_query {
        let can_afford = economy.gold >= tower_button.0.cost();

        if *interaction != Interaction::None {
            continue;
        }

        for &child in children.iter() {
            if let Ok(mut text) = cost_text_query.get_mut(child) {
                text.sections[0].style.color = if can_afford {
                    GameColors::GOLD
                } else {
                    GameColors::HEALTH_LOW
                };
            }
        }
    }
}

pub(super) fn update_info_panel(
    selected: Res<SelectedTowerType>,
    mut name_query: Query<&mut Text, (With<InfoPanelName>, Without<InfoPanelDesc>, Without<InfoPanelStats>, Without<InfoPanelCost>)>,
    mut desc_query: Query<&mut Text, (With<InfoPanelDesc>, Without<InfoPanelName>, Without<InfoPanelStats>, Without<InfoPanelCost>)>,
    mut stats_query: Query<&mut Text, (With<InfoPanelStats>, Without<InfoPanelName>, Without<InfoPanelDesc>, Without<InfoPanelCost>)>,
    mut cost_query: Query<&mut Text, (With<InfoPanelCost>, Without<InfoPanelName>, Without<InfoPanelDesc>, Without<InfoPanelStats>)>,
) {
    if selected.is_changed() {
        let tower_type = selected.0;

        for mut text in &mut name_query {
            text.sections[0].value = tower_type.name().to_string();
            text.sections[0].style.color = tower_type.color();
        }

        for mut text in &mut desc_query {
            text.sections[0].value = tower_type.description().to_string();
        }

        for mut text in &mut stats_query {
            if tower_type == TowerType::Buff {
                text.sections[0].value = format!(
                    "BUFF: +25%  RNG: {:.0}",
                    tower_type.range()
                );
            } else {
                text.sections[0].value = format!(
                    "DMG: {:.0}  RNG: {:.0}\nSPD: {:.1}/s",
                    tower_type.damage(),
                    tower_type.range(),
                    tower_type.attack_speed()
                );
            }
        }

        for mut text in &mut cost_query {
            text.sections[0].value = format!("Cost: {}g", tower_type.cost());
        }
    }
}

/// Handle keyboard shortcuts for tower context menu (separate system to stay within param limit)
pub(super) fn tower_context_shortcuts(
    towers: Query<&Tower>,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    keyboard: Res<ButtonInput<KeyCode>>,
    economy: Res<PlayerEconomy>,
    mut upgrade_events: EventWriter<UpgradeTowerEvent>,
    mut sell_events: EventWriter<SellTowerEvent>,
    mut sell_pending: ResMut<SellPending>,
    time: Res<Time>,
) {
    // Tick sell confirmation timer
    if let Some(ref mut timer) = sell_pending.timer {
        timer.tick(time.delta());
        if timer.finished() {
            sell_pending.timer = None;
            sell_pending.tower = None;
        }
    }

    // Escape to deselect (also cancels pending sell)
    if keyboard.just_pressed(KeyCode::Escape) && selected_tower.0.is_some() {
        selected_tower.0 = None;
        sell_pending.timer = None;
        sell_pending.tower = None;
        return;
    }

    // Keyboard shortcuts when tower is selected
    if let Some(tower_entity) = selected_tower.0 {
        if let Ok(tower) = towers.get(tower_entity) {
            // U to upgrade
            if keyboard.just_pressed(KeyCode::KeyU)
                && tower.can_upgrade() && economy.gold >= tower.upgrade_cost() {
                    upgrade_events.send(UpgradeTowerEvent { tower: tower_entity });
                }
            // S to sell — requires double-tap within 1.5 seconds
            if keyboard.just_pressed(KeyCode::KeyS) {
                if sell_pending.tower == Some(tower_entity) && sell_pending.timer.is_some() {
                    // Second press — confirm sell
                    sell_events.send(SellTowerEvent { tower: tower_entity });
                    selected_tower.0 = None;
                    sell_pending.timer = None;
                    sell_pending.tower = None;
                } else {
                    // First press — enter pending state
                    sell_pending.tower = Some(tower_entity);
                    sell_pending.timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
                }
            }
        }
    }
}

pub(super) fn update_tower_context_menu(
    hovered_tile: Res<HoveredTile>,
    map: Res<GameMap>,
    towers: Query<(Entity, &Tower, Option<&TowerSynergies>)>,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut context_menu: Query<&mut Style, With<TowerDetailPanel>>,
    mut upgrade_text: Query<&mut Text, (With<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut sell_text: Query<&mut Text, (With<SellValueText>, Without<UpgradeCostText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut stats_text: Query<&mut Text, (With<TowerStatsText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut preview_text: Query<&mut Text, (With<TowerUpgradePreview>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TargetingText>, Without<SynergyText>)>,
    mut targeting_text: Query<&mut Text, (With<TargetingText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<SynergyText>)>,
    mut synergy_text: Query<&mut Text, (With<SynergyText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>)>,
    pointer: Res<super::super::PointerState>,
    economy: Res<PlayerEconomy>,
    sell_pending: Res<SellPending>,
) {
    // Left-click or touch to select/deselect towers
    if pointer.just_pressed {
        if let Some((hx, hy)) = hovered_tile.position {
            if map.tiles[hx][hy] == TileType::Tower {
                // Find the tower at this position
                for (entity, tower, _) in &towers {
                    if tower.grid_x == hx && tower.grid_y == hy {
                        // Toggle selection - if already selected, deselect
                        if selected_tower.0 == Some(entity) {
                            selected_tower.0 = None;
                        } else {
                            selected_tower.0 = Some(entity);
                        }
                        break;
                    }
                }
            } else {
                // Clicked on non-tower tile - deselect
                selected_tower.0 = None;
            }
        }
    }

    // Update detail panel visibility
    for mut style in &mut context_menu {
        if let Some(tower_entity) = selected_tower.0 {
            if let Ok((_, tower, synergies)) = towers.get(tower_entity) {
                style.display = Display::Flex;

                // Get current attack speed
                let attack_speed = tower.attack_speed();
                let is_buff_tower = tower.tower_type == TowerType::Buff;

                // Update stats text
                for mut text in &mut stats_text {
                    let spec_label = tower.specialization
                        .map(|s| format!(" [{}]", s.name()))
                        .unwrap_or_default();
                    text.sections[0].value = format!("{}{} Lv{}\n", tower.tower_type.name(), spec_label, tower.level);
                    if is_buff_tower {
                        let buff_pct = tower.buff_percentage() * 100.0;
                        text.sections[1].value = format!(
                            "BUFF: +{:.0}%  RNG: {:.0}",
                            buff_pct,
                            tower.range
                        );
                    } else {
                        text.sections[1].value = format!(
                            "DMG: {:.0}  RNG: {:.0}  SPD: {:.1}/s",
                            tower.damage,
                            tower.range,
                            attack_speed
                        );
                    }
                }

                let needs_spec = tower.needs_specialization();

                if needs_spec {
                    // Clear upgrade button/preview when spec choice is needed
                    for mut text in &mut upgrade_text {
                        text.sections[0].value = String::new();
                    }
                    for mut text in &mut preview_text {
                        text.sections[0].value = "Choose Specialization\n".to_string();
                        text.sections[1].value = String::new();
                    }
                } else {
                    // Update upgrade preview text using the tower's preview method
                    for mut text in &mut preview_text {
                        let (next_damage, next_range, next_speed) = tower.preview_upgrade();
                        text.sections[0].value = format!("Level {} Preview\n", tower.level + 1);
                        if is_buff_tower {
                            let curr_buff = tower.buff_percentage() * 100.0;
                            let next_buff = tower.buff_percentage_next() * 100.0;
                            text.sections[1].value = format!(
                                "BUFF: +{:.0}% (+{:.0}%)\nRNG: {:.0} (+{:.0})",
                                next_buff, next_buff - curr_buff,
                                next_range, next_range - tower.range
                            );
                        } else {
                            let spec_label = tower.specialization
                                .map(|s| format!(" ({})", s.name()))
                                .unwrap_or_default();
                            text.sections[1].value = format!(
                                "DMG: {:.0} (+{:.0})\nRNG: {:.0} (+{:.0})\nSPD: {:.2}/s (+{:.2}){}",
                                next_damage, next_damage - tower.damage,
                                next_range, next_range - tower.range,
                                next_speed, next_speed - attack_speed,
                                spec_label
                            );
                        }
                    }

                    // Update button text
                    let upgrade_cost = tower.upgrade_cost();
                    let can_afford = economy.gold >= upgrade_cost;
                    for mut text in &mut upgrade_text {
                        text.sections[0].value = if can_afford {
                            format!("Upgrade [U] - {}g", upgrade_cost)
                        } else {
                            format!("Need {}g (have {})", upgrade_cost, economy.gold)
                        };
                    }
                }
                for mut text in &mut sell_text {
                    if sell_pending.tower == Some(tower_entity) && sell_pending.timer.is_some() {
                        text.sections[0].value = "Confirm [S]?".into();
                        text.sections[0].style.color = GameColors::WARNING_TEXT;
                    } else {
                        text.sections[0].value = format!("Sell [S] +{}g", tower.sell_value());
                        text.sections[0].style.color = Color::WHITE;
                    }
                }
                // Update targeting text to show current mode
                for mut text in &mut targeting_text {
                    text.sections[0].value = format!("Target: {} [T]", tower.targeting.name());
                }
                // Update synergy text
                for mut text in &mut synergy_text {
                    if let Some(syn) = synergies {
                        if !syn.active.is_empty() {
                            use std::fmt::Write;
                            let mut result = String::new();
                            for (i, s) in syn.active.iter().enumerate() {
                                if i > 0 { result.push('\n'); }
                                let _ = write!(result, "\u{26A1} {}: {}", s.name(), s.description());
                            }
                            text.sections[0].value = result;
                        } else {
                            text.sections[0].value = String::new();
                        }
                    } else {
                        text.sections[0].value = String::new();
                    }
                }
            } else {
                style.display = Display::None;
                selected_tower.0 = None;
            }
        } else {
            style.display = Display::None;
        }
    }
}

pub(super) fn update_spec_panel(
    selected_tower: Res<SelectedPlacedTower>,
    towers: Query<&Tower>,
    mut spec_panel: Query<&mut Style, With<SpecChoicePanel>>,
    mut spec_buttons: Query<(&mut SpecButton, &Children)>,
    mut spec_texts: Query<&mut Text, With<SpecButtonText>>,
) {
    let Some(tower_entity) = selected_tower.0 else {
        for mut style in &mut spec_panel {
            style.display = Display::None;
        }
        return;
    };

    let Ok(tower) = towers.get(tower_entity) else {
        for mut style in &mut spec_panel {
            style.display = Display::None;
        }
        return;
    };

    let needs_spec = tower.needs_specialization();

    for mut style in &mut spec_panel {
        style.display = if needs_spec { Display::Flex } else { Display::None };
    }

    if needs_spec {
        if let Some(choices) = Specialization::choices_for(tower.tower_type) {
            let upgrade_cost = tower.upgrade_cost();
            let mut button_iter = spec_buttons.iter_mut();
            for spec in choices.iter() {
                if let Some((mut spec_btn, children)) = button_iter.next() {
                    spec_btn.0 = *spec;
                    for child in children.iter() {
                        if let Ok(mut text) = spec_texts.get_mut(*child) {
                            text.sections[0].value = format!(
                                "{}\n{}\n{}g",
                                spec.name(),
                                spec.description(),
                                upgrade_cost
                            );
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn tower_context_buttons(
    mut upgrade_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<UpgradeButton>, Without<SellButton>, Without<TargetingButton>),
    >,
    mut sell_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<SellButton>, Without<UpgradeButton>, Without<TargetingButton>),
    >,
    mut targeting_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<TargetingButton>, Without<UpgradeButton>, Without<SellButton>),
    >,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut towers: Query<&mut Tower>,
    economy: Res<PlayerEconomy>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut upgrade_events: EventWriter<UpgradeTowerEvent>,
    mut sell_events: EventWriter<SellTowerEvent>,
    mut targeting_text: Query<&mut Text, With<TargetingText>>,
    mut ui_clicked: ResMut<UiClicked>,
    mut sell_pending: ResMut<SellPending>,
) {
    // Check if we can afford upgrade and it's allowed
    let can_afford = if let Some(tower_entity) = selected_tower.0 {
        if let Ok(tower) = towers.get(tower_entity) {
            tower.can_upgrade() && economy.gold >= tower.upgrade_cost()
        } else {
            false
        }
    } else {
        false
    };

    // Upgrade button - color based on affordability
    for (interaction, mut color) in &mut upgrade_query {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                if let Some(tower_entity) = selected_tower.0 {
                    if let Ok(tower) = towers.get(tower_entity) {
                        if tower.can_upgrade() && economy.gold >= tower.upgrade_cost() {
                            upgrade_events.send(UpgradeTowerEvent { tower: tower_entity });
                        }
                    }
                }
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
                if can_afford {
                    *color = Color::srgb(0.25, 0.65, 0.4).into();
                } else {
                    *color = Color::srgb(0.35, 0.35, 0.4).into();
                }
            }
            Interaction::None => {
                if can_afford {
                    *color = Color::srgb(0.2, 0.5, 0.3).into();
                } else {
                    *color = Color::srgb(0.25, 0.25, 0.3).into();
                }
            }
        }
    }

    // Sell button — requires double-click to confirm
    for (interaction, mut color) in &mut sell_query {
        if *interaction != Interaction::None { ui_clicked.0 = true; }
        if button_interaction(interaction, &mut color, GameColors::BUTTON_SELL, GameColors::BUTTON_SELL_HOVER) {
            if let Some(tower_entity) = selected_tower.0 {
                if sell_pending.tower == Some(tower_entity) && sell_pending.timer.is_some() {
                    sell_events.send(SellTowerEvent { tower: tower_entity });
                    selected_tower.0 = None;
                    sell_pending.timer = None;
                    sell_pending.tower = None;
                } else {
                    sell_pending.tower = Some(tower_entity);
                    sell_pending.timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
                }
            }
        }
    }

    // Handle targeting button click or [T] key
    let mut should_cycle_targeting = false;

    for (interaction, mut color) in &mut targeting_query {
        if *interaction != Interaction::None { ui_clicked.0 = true; }
        if button_interaction(interaction, &mut color, GameColors::BUTTON_TARGETING, GameColors::BUTTON_TARGETING_HOVER) {
            should_cycle_targeting = true;
        }
    }

    // Keyboard shortcut [T]
    if keyboard.just_pressed(KeyCode::KeyT) && selected_tower.0.is_some() {
        should_cycle_targeting = true;
    }

    // Apply targeting cycle
    if should_cycle_targeting {
        if let Some(tower_entity) = selected_tower.0 {
            if let Ok(mut tower) = towers.get_mut(tower_entity) {
                tower.cycle_targeting();

                // Update targeting text
                for mut text in &mut targeting_text {
                    text.sections[0].value = format!("Target: {} [T]", tower.targeting.name());
                }
            }
        }
    }
}

pub(super) fn spec_button_system(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &SpecButton),
        Changed<Interaction>,
    >,
    selected_tower: Res<SelectedPlacedTower>,
    towers: Query<&Tower>,
    economy: Res<PlayerEconomy>,
    mut spec_events: EventWriter<SpecializeTowerEvent>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    for (interaction, mut color, spec_btn) in &mut query {
        if *interaction != Interaction::None { ui_clicked.0 = true; }
        if button_interaction(interaction, &mut color, Color::srgb(0.2, 0.3, 0.5), Color::srgb(0.3, 0.4, 0.65)) {
            if let Some(tower_entity) = selected_tower.0 {
                if let Ok(tower) = towers.get(tower_entity) {
                    if tower.needs_specialization() && economy.gold >= tower.upgrade_cost() {
                        spec_events.send(SpecializeTowerEvent {
                            tower: tower_entity,
                            specialization: spec_btn.0,
                        });
                    }
                }
            }
        }
    }
}
