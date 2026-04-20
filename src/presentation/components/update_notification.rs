//! Update Notification Component
//!
//! Shows a non-intrusive notification banner when a new version is available.
//! Positioned at the bottom-right, above the status bar. Uses theme colors.

use egui::{Context, RichText, Rounding, Stroke, Vec2};

use crate::infrastructure::updater::updater::{UpdateInfo, UpdateState};
use crate::presentation::theme;

/// Output actions from the update notification
#[derive(Debug, Clone, Default)]
pub struct UpdateNotificationOutput {
    /// User clicked "Update Now" — start download
    pub download_requested: bool,
    /// User clicked "Install" — launch the downloaded installer
    pub install_requested: bool,
    /// User dismissed the notification
    pub dismissed: bool,
    /// User clicked "View Release" — open in browser
    pub view_release: bool,
}

/// Bottom-right margin above the status bar
const MARGIN_RIGHT: f32 = 12.0;
const MARGIN_BOTTOM: f32 = 36.0; // above the 24px status bar
const BANNER_WIDTH: f32 = 310.0;

/// Compute the bottom-right anchor position for the notification.
fn banner_pos(ctx: &Context) -> egui::Pos2 {
    let screen = ctx.screen_rect();
    egui::pos2(
        screen.right() - BANNER_WIDTH - MARGIN_RIGHT,
        screen.bottom() - MARGIN_BOTTOM,
    )
}

/// Render the update notification banner.
///
/// Shows as a floating banner at the bottom-right of the viewport.
pub fn show_update_notification(
    ctx: &Context,
    update_state: &UpdateState,
    dismissed: &mut bool,
) -> UpdateNotificationOutput {
    let mut output = UpdateNotificationOutput::default();

    if *dismissed {
        return output;
    }

    let t = theme::active();

    match update_state {
        UpdateState::Available(info) => {
            show_available_banner(ctx, info, &t, &mut output, dismissed);
        }
        UpdateState::Downloading(progress) => {
            show_downloading_banner(ctx, *progress, &t);
        }
        UpdateState::ReadyToInstall(_) => {
            show_ready_banner(ctx, &t, &mut output);
        }
        UpdateState::Error(msg) => {
            show_error_banner(ctx, msg, &t, dismissed);
        }
        _ => {}
    }

    output
}

fn banner_frame(t: &theme::ActiveTheme, border_color: egui::Color32) -> egui::Frame {
    egui::Frame::none()
        .fill(t.panel_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0, border_color))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: t.panel_shadow,
        })
}

fn show_available_banner(
    ctx: &Context,
    info: &UpdateInfo,
    t: &theme::ActiveTheme,
    output: &mut UpdateNotificationOutput,
    dismissed: &mut bool,
) {
    egui::Area::new(egui::Id::new("update_notification"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-MARGIN_RIGHT, -MARGIN_BOTTOM))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            banner_frame(t, t.accent).show(ui, |ui| {
                ui.set_max_width(BANNER_WIDTH - 24.0);

                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⬆ Update Available")
                            .size(13.0)
                            .strong()
                            .color(t.accent),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            *dismissed = true;
                            output.dismissed = true;
                        }
                    });
                });

                ui.add_space(4.0);

                // Version info
                ui.label(
                    RichText::new(format!("{} is available", info.version))
                        .size(12.0)
                        .color(t.text_primary),
                );

                if !info.title.is_empty() && info.title != info.version {
                    ui.label(
                        RichText::new(&info.title)
                            .size(11.0)
                            .color(t.text_secondary),
                    );
                }

                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    let update_btn = egui::Button::new(
                        RichText::new("Update Now").size(12.0).color(
                            if t.is_dark { egui::Color32::BLACK } else { egui::Color32::WHITE }
                        ),
                    )
                    .fill(t.accent)
                    .rounding(Rounding::same(4.0))
                    .min_size(Vec2::new(90.0, 26.0));

                    if ui.add(update_btn).clicked() {
                        output.download_requested = true;
                    }

                    let release_btn = egui::Button::new(
                        RichText::new("Release Notes").size(11.0).color(t.text_secondary),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(Stroke::new(1.0, t.toolbar_border))
                    .rounding(Rounding::same(4.0))
                    .min_size(Vec2::new(0.0, 26.0));

                    if ui.add(release_btn).clicked() {
                        output.view_release = true;
                    }
                });
            });
        });
}

fn show_downloading_banner(ctx: &Context, progress: f32, t: &theme::ActiveTheme) {
    egui::Area::new(egui::Id::new("update_notification"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-MARGIN_RIGHT, -MARGIN_BOTTOM))
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            banner_frame(t, t.accent).show(ui, |ui| {
                ui.set_max_width(BANNER_WIDTH - 24.0);

                ui.label(
                    RichText::new("⬇ Downloading Update...")
                        .size(13.0)
                        .strong()
                        .color(t.accent),
                );
                ui.add_space(6.0);

                let bar = egui::ProgressBar::new(progress)
                    .text(format!("{:.0}%", progress * 100.0));
                ui.add(bar);
            });
        });

    ctx.request_repaint();
}

fn show_ready_banner(
    ctx: &Context,
    t: &theme::ActiveTheme,
    output: &mut UpdateNotificationOutput,
) {
    let success_color = t.accent;

    egui::Area::new(egui::Id::new("update_notification"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-MARGIN_RIGHT, -MARGIN_BOTTOM))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            banner_frame(t, success_color).show(ui, |ui| {
                ui.set_max_width(BANNER_WIDTH - 24.0);

                ui.label(
                    RichText::new("✓ Download Complete")
                        .size(13.0)
                        .strong()
                        .color(t.accent),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Click Install to update. The app will restart.")
                        .size(11.0)
                        .color(t.text_secondary),
                );
                ui.add_space(8.0);

                let install_btn = egui::Button::new(
                    RichText::new("Install & Restart").size(12.0).color(
                        if t.is_dark { egui::Color32::BLACK } else { egui::Color32::WHITE }
                    ),
                )
                .fill(t.accent)
                .rounding(Rounding::same(4.0))
                .min_size(Vec2::new(130.0, 28.0));

                if ui.add(install_btn).clicked() {
                    output.install_requested = true;
                }
            });
        });
}

fn show_error_banner(
    ctx: &Context,
    msg: &str,
    t: &theme::ActiveTheme,
    dismissed: &mut bool,
) {
    let error_color = if t.is_dark {
        egui::Color32::from_rgb(220, 100, 100)
    } else {
        egui::Color32::from_rgb(180, 60, 60)
    };

    egui::Area::new(egui::Id::new("update_notification"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-MARGIN_RIGHT, -MARGIN_BOTTOM))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            banner_frame(t, error_color).show(ui, |ui| {
                ui.set_max_width(BANNER_WIDTH - 24.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Update check failed")
                            .size(12.0)
                            .color(error_color),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            *dismissed = true;
                        }
                    });
                });
                let display_msg = if msg.len() > 100 { &msg[..100] } else { msg };
                ui.label(
                    RichText::new(display_msg)
                        .size(10.0)
                        .color(t.text_secondary),
                );
            });
        });
}
