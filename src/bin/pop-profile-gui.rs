use eframe::egui;
use pop_profile::config::{
    self, validate_and_sanitize, FirewallConfig, FrameworkPowerConfig, PortRule, ProfileConfig,
    ProfileMetadata, SecurityLimitsConfig, DesktopConfig,
};
use pop_profile::system;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Tab {
    General,
    Firewall,
    Kernel,
    Power,
}

struct GuiApp {
    profiles: Vec<ProfileConfig>,
    selected_index: usize,
    active_profile_id: String,
    active_tab: Tab,
    status_message: Option<(String, bool)>, // (message, is_error)
    show_reset_confirm: bool,

    // Form inputs for adding entries
    new_port_str: String,
    new_port_comment: String,
    new_interface_str: String,
    new_sysctl_key: String,
    new_sysctl_val: String,
}

impl GuiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let profiles = config::load_all_profiles();
        let active_profile_id = system::get_active_profile();

        Self {
            profiles,
            selected_index: 0,
            active_profile_id,
            active_tab: Tab::General,
            status_message: None,
            show_reset_confirm: false,
            new_port_str: "8080/tcp".to_string(),
            new_port_comment: "Web Development".to_string(),
            new_interface_str: "docker0".to_string(),
            new_sysctl_key: "fs.inotify.max_user_watches".to_string(),
            new_sysctl_val: "524288".to_string(),
        }
    }

    fn refresh_profiles(&mut self) {
        self.profiles = config::load_all_profiles();
        self.active_profile_id = system::get_active_profile();
        if self.selected_index >= self.profiles.len() {
            self.selected_index = 0;
        }
    }

    fn current_profile(&self) -> Option<&ProfileConfig> {
        self.profiles.get(self.selected_index)
    }

    fn do_factory_reset(&mut self) {
        let dbus_attempt = std::process::Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "io.github.mzia.PopProfile",
                "--object-path",
                "/io/github/mzia/PopProfile",
                "--method",
                "io.github.mzia.PopProfile.ResetToDefaults",
            ])
            .output();

        let mut success = false;
        let mut err_msg = String::new();

        if let Ok(out) = dbus_attempt {
            if out.status.success() {
                success = true;
            } else {
                err_msg = String::from_utf8_lossy(&out.stderr).to_string();
            }
        }

        if !success {
            match system::reset_to_defaults() {
                Ok(_) => success = true,
                Err(e) => {
                    if err_msg.is_empty() {
                        err_msg = e;
                    }
                }
            }
        }

        if success {
            self.active_profile_id = "default".to_string();
            self.refresh_profiles();
            self.status_message = Some((
                "Pop!_OS factory defaults restored successfully (UFW reset/disabled, balanced power, battery 100%).".to_string(),
                false,
            ));
        } else {
            self.status_message = Some((format!("Failed to reset factory defaults: {}", err_msg), true));
        }
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Top status / notification banner
        let mut dismiss = false;
        if let Some((ref msg, is_err)) = self.status_message {
            let banner_color = if is_err {
                egui::Color32::from_rgb(180, 40, 40)
            } else {
                egui::Color32::from_rgb(34, 139, 34)
            };
            egui::Panel::top("status_banner")
                .frame(egui::Frame::new().fill(banner_color).inner_margin(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(msg).color(egui::Color32::WHITE).strong());
                        if ui.button("✕").clicked() {
                            dismiss = true;
                        }
                    });
                });
        }
        if dismiss {
            self.status_message = None;
        }

        // Left Profiles Sidebar
        egui::Panel::left("profiles_sidebar")
            .resizable(false)
            .default_size(240.0)
            .show(ui, |ui| {
                ui.heading("Pop! Profiles");
                let active_header = if self.active_profile_id == "default" {
                    "Active: [FACTORY DEFAULT]".to_string()
                } else {
                    format!("Active: [{}]", self.active_profile_id.to_uppercase())
                };
                ui.label(
                    egui::RichText::new(active_header)
                        .color(egui::Color32::from_rgb(246, 166, 35))
                        .strong(),
                );
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut switch_to = None;
                    for (i, prof) in self.profiles.iter().enumerate() {
                        let is_selected = i == self.selected_index;
                        let is_active = prof.profile.id == self.active_profile_id;

                        let label_text = if is_active {
                            format!("● {} [Active]", prof.profile.name)
                        } else {
                            format!("○ {}", prof.profile.name)
                        };

                        if ui.selectable_label(is_selected, label_text).clicked() {
                            switch_to = Some(i);
                        }
                    }
                    if let Some(idx) = switch_to {
                        self.selected_index = idx;
                    }
                });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    // Reset to Factory Defaults Button
                    if ui
                        .button(
                            egui::RichText::new("🔄 Reset to Factory Defaults")
                                .color(egui::Color32::from_rgb(255, 120, 120)),
                        )
                        .clicked()
                    {
                        self.show_reset_confirm = true;
                    }
                    ui.separator();

                    // Export Config Button
                    if ui.button("📤 Export Selected (.toml)").clicked() {
                        if let Some(prof) = self.current_profile() {
                            if let Ok(toml_str) = prof.to_toml() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_file_name(&format!("{}.toml", prof.profile.id))
                                    .save_file()
                                {
                                    if std::fs::write(&path, toml_str).is_ok() {
                                        self.status_message = Some((
                                            format!("Exported to {}", path.display()),
                                            false,
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    // Import Config Button
                    if ui.button("📥 Import Config (.toml)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("TOML Config", &["toml"])
                            .pick_file()
                        {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                match ProfileConfig::from_toml(&content) {
                                    Ok(cfg) => match validate_and_sanitize(cfg) {
                                        Ok(report) => {
                                            let id = report.sanitized_config.profile.id.clone();
                                            match config::save_custom_profile(
                                                report.sanitized_config,
                                                false,
                                            ) {
                                                Ok(saved_path) => {
                                                    self.status_message = Some((
                                                        format!(
                                                            "Successfully imported '{}' into {}",
                                                            id,
                                                            saved_path.display()
                                                        ),
                                                        false,
                                                    ));
                                                    self.refresh_profiles();
                                                }
                                                Err(e) => {
                                                    self.status_message = Some((
                                                        format!("Failed to save profile: {}", e),
                                                        true,
                                                    ));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.status_message =
                                                Some((format!("Safety validation error: {}", e), true));
                                        }
                                    },
                                    Err(e) => {
                                        self.status_message =
                                            Some((format!("Invalid TOML syntax: {}", e), true));
                                    }
                                }
                            }
                        }
                    }

                    // New Profile Button
                    if ui.button("➕ New Custom Profile").clicked() {
                        let new_id = format!("custom-{}", self.profiles.len() + 1);
                        let new_cfg = ProfileConfig {
                            profile: ProfileMetadata {
                                id: new_id.clone(),
                                name: "New Custom Profile".to_string(),
                                description: "User custom profile configuration".to_string(),
                                icon: "preferences-system-symbolic".to_string(),
                                version: "1.0.0".to_string(),
                                author: std::env::var("USER").unwrap_or_else(|_| "user".to_string()),
                                is_builtin: false,
                            },
                            kernel: HashMap::new(),
                            limits: SecurityLimitsConfig::default(),
                            firewall: FirewallConfig::default(),
                            power: FrameworkPowerConfig::default(),
                            desktop: DesktopConfig::default(),
                        };
                        let _ = config::save_custom_profile(new_cfg, false);
                        self.refresh_profiles();
                        if let Some(pos) = self.profiles.iter().position(|p| p.profile.id == new_id) {
                            self.selected_index = pos;
                        }
                    }
                });
            });

        // Reset to Factory Defaults Confirmation Modal
        if self.show_reset_confirm {
            egui::Window::new("Reset to Pop!_OS Factory Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.heading("⚠️ Confirm Factory Reset");
                    ui.add_space(6.0);
                    ui.label("This will restore all system settings back to Pop!_OS factory defaults:");
                    ui.label("• Disable and reset UFW firewall (allow standard traffic)");
                    ui.label("• Remove custom sysctl optimizations and limits drop-ins");
                    ui.label("• Restore platform power profile to 'Balanced'");
                    ui.label("• Restore battery charge threshold to 100%");
                    ui.label("• Restore desktop screen idle timeout to 15 minutes");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                egui::RichText::new("Yes, Reset to Factory Defaults")
                                    .color(egui::Color32::from_rgb(255, 120, 120))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.show_reset_confirm = false;
                            self.do_factory_reset();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_reset_confirm = false;
                        }
                    });
                });
        }

        // Central Editing Area
        egui::CentralPanel::default().show(ui, |ui| {
            if self.profiles.is_empty() {
                ui.label("No profiles available.");
                return;
            }

            let profile_clone = self.profiles[self.selected_index].clone();
            let is_builtin = profile_clone.profile.is_builtin;

            // Profile Header Bar
            ui.horizontal(|ui| {
                ui.heading(&profile_clone.profile.name);
                if is_builtin {
                    ui.label(
                        egui::RichText::new("[System Built-in]")
                            .color(egui::Color32::from_rgb(100, 180, 255))
                            .small(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("[Custom Profile]")
                            .color(egui::Color32::from_rgb(246, 166, 35))
                            .small(),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let activate_btn = ui.button(
                        egui::RichText::new("⚡ Activate Profile")
                            .color(egui::Color32::BLACK)
                            .strong(),
                    );
                    if activate_btn.clicked() {
                        let id = profile_clone.profile.id.clone();
                        match system::apply_profile_by_id(&id) {
                            Ok(_) => {
                                self.active_profile_id = id.clone();
                                self.status_message = Some((
                                    format!("Activated [{}] mode successfully!", id.to_uppercase()),
                                    false,
                                ));
                            }
                            Err(e) => {
                                self.status_message =
                                    Some((format!("Failed to activate profile: {}", e), true));
                            }
                        }
                    }
                });
            });

            ui.label(&profile_clone.profile.description);
            ui.separator();

            // Tabs
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::General, "🏷️ General");
                ui.selectable_value(&mut self.active_tab, Tab::Firewall, "🛡️ Firewall & Ports");
                ui.selectable_value(&mut self.active_tab, Tab::Kernel, "⚙️ Kernel & Sysctl");
                ui.selectable_value(&mut self.active_tab, Tab::Power, "⚡ Power & Framework");
            });
            ui.separator();

            // Tab Content
            egui::ScrollArea::vertical().show(ui, |ui| {
                let current_profile = &mut self.profiles[self.selected_index];

                match self.active_tab {
                    Tab::General => {
                        ui.label(egui::RichText::new("Profile Metadata").strong());
                        ui.horizontal(|ui| {
                            ui.label("Profile Name: ");
                            ui.add_enabled(!is_builtin, egui::TextEdit::singleline(&mut current_profile.profile.name));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Profile ID:   ");
                            ui.add_enabled(false, egui::TextEdit::singleline(&mut current_profile.profile.id));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Icon Name:    ");
                            ui.add_enabled(!is_builtin, egui::TextEdit::singleline(&mut current_profile.profile.icon));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Description:  ");
                            ui.add_enabled(!is_builtin, egui::TextEdit::multiline(&mut current_profile.profile.description));
                        });
                        ui.separator();

                        ui.label(egui::RichText::new("Desktop Posture").strong());
                        let mut idle = current_profile.desktop.idle_delay_seconds.unwrap_or(300);
                        ui.horizontal(|ui| {
                            ui.label("Screen Lock Delay (seconds):");
                            if ui.add_enabled(!is_builtin, egui::Slider::new(&mut idle, 60..=3600)).changed() {
                                current_profile.desktop.idle_delay_seconds = Some(idle);
                            }
                        });
                    }

                    Tab::Firewall => {
                        ui.label(egui::RichText::new("UFW Traffic Policies").strong());
                        ui.horizontal(|ui| {
                            ui.label("Incoming: ");
                            ui.add_enabled(
                                !is_builtin,
                                egui::TextEdit::singleline(&mut current_profile.firewall.default_incoming),
                            );
                            ui.label("Outgoing: ");
                            ui.add_enabled(
                                !is_builtin,
                                egui::TextEdit::singleline(&mut current_profile.firewall.default_outgoing),
                            );
                        });

                        ui.checkbox(&mut current_profile.firewall.allow_loopback, "Allow Loopback (127.0.0.1)");
                        ui.separator();

                        ui.label(egui::RichText::new("Unblocked Interfaces").strong());
                        let mut iface_to_delete = None;
                        for (idx, iface) in current_profile.firewall.allow_interfaces.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("• {}", iface));
                                if !is_builtin && ui.button("✕").clicked() {
                                    iface_to_delete = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = iface_to_delete {
                            current_profile.firewall.allow_interfaces.remove(idx);
                        }

                        if !is_builtin {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.new_interface_str);
                                if ui.button("+ Add Interface").clicked() && !self.new_interface_str.is_empty() {
                                    current_profile
                                        .firewall
                                        .allow_interfaces
                                        .push(self.new_interface_str.clone());
                                    self.new_interface_str.clear();
                                }
                            });
                        }
                        ui.separator();

                        ui.label(egui::RichText::new("Allowed Incoming Ports").strong());
                        let mut port_to_delete = None;
                        for (idx, port) in current_profile.firewall.allow_ports.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("• {:<16} ({})", port.port, port.comment));
                                if !is_builtin && ui.button("✕").clicked() {
                                    port_to_delete = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = port_to_delete {
                            current_profile.firewall.allow_ports.remove(idx);
                        }

                        if !is_builtin {
                            ui.horizontal(|ui| {
                                ui.label("Port:");
                                ui.text_edit_singleline(&mut self.new_port_str);
                                ui.label("Comment:");
                                ui.text_edit_singleline(&mut self.new_port_comment);
                                if ui.button("+ Add Port").clicked() && !self.new_port_str.is_empty() {
                                    current_profile.firewall.allow_ports.push(PortRule {
                                        port: self.new_port_str.clone(),
                                        comment: self.new_port_comment.clone(),
                                    });
                                    self.new_port_str.clear();
                                    self.new_port_comment.clear();
                                }
                            });
                        }
                    }

                    Tab::Kernel => {
                        ui.label(egui::RichText::new("Kernel Sysctl Tunings").strong());
                        let mut key_to_delete = None;
                        for (k, v) in current_profile.kernel.iter() {
                            ui.horizontal(|ui| {
                                ui.label(format!("• {:<36} = {}", k, v));
                                if !is_builtin && ui.button("✕").clicked() {
                                    key_to_delete = Some(k.clone());
                                }
                            });
                        }
                        if let Some(k) = key_to_delete {
                            current_profile.kernel.remove(&k);
                        }

                        if !is_builtin {
                            ui.horizontal(|ui| {
                                ui.label("Key:");
                                ui.text_edit_singleline(&mut self.new_sysctl_key);
                                ui.label("Val:");
                                ui.text_edit_singleline(&mut self.new_sysctl_val);
                                if ui.button("+ Add Key").clicked() && !self.new_sysctl_key.is_empty() {
                                    current_profile
                                        .kernel
                                        .insert(self.new_sysctl_key.clone(), self.new_sysctl_val.clone());
                                    self.new_sysctl_key.clear();
                                    self.new_sysctl_val.clear();
                                }
                            });
                        }
                    }

                    Tab::Power => {
                        ui.label(egui::RichText::new("Framework Battery & Power Tunings").strong());
                        let mut charge = current_profile.power.battery_charge_limit.unwrap_or(80);
                        ui.horizontal(|ui| {
                            ui.label("Battery Maximum Charge Threshold:");
                            if ui.add_enabled(!is_builtin, egui::Slider::new(&mut charge, 50..=100).suffix("%")).changed() {
                                current_profile.power.battery_charge_limit = Some(charge);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("system76-power Profile:");
                            let mut profile_str = current_profile.power.power_profile.clone().unwrap_or_else(|| "balanced".to_string());
                            ui.add_enabled(!is_builtin, egui::TextEdit::singleline(&mut profile_str));
                            current_profile.power.power_profile = Some(profile_str);
                        });
                    }
                }
            });

            // Bottom Action Controls
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if !is_builtin {
                        if ui.button("💾 Save Custom Profile").clicked() {
                            let cfg = self.profiles[self.selected_index].clone();
                            match config::save_custom_profile(cfg, false) {
                                Ok(p) => {
                                    self.status_message = Some((format!("Saved to {}", p.display()), false));
                                    self.refresh_profiles();
                                }
                                Err(e) => {
                                    self.status_message = Some((format!("Failed to save: {}", e), true));
                                }
                            }
                        }

                        if ui.button("🗑️ Delete Profile").clicked() {
                            let id = self.profiles[self.selected_index].profile.id.clone();
                            if config::delete_custom_profile(&id).is_ok() {
                                self.status_message = Some((format!("Deleted profile '{}'", id), false));
                                self.refresh_profiles();
                            }
                        }
                    }

                    if ui.button("🛡️ Test & Verify Safety").clicked() {
                        let cfg = self.profiles[self.selected_index].clone();
                        match validate_and_sanitize(cfg) {
                            Ok(report) => {
                                if report.warnings.is_empty() {
                                    self.status_message = Some((
                                        "Safety Verification Passed: 0 warnings, anti-lockout rules satisfied.".to_string(),
                                        false,
                                    ));
                                } else {
                                    self.status_message = Some((
                                        format!("Passed with adjustments: {}", report.warnings.join("; ")),
                                        false,
                                    ));
                                }
                            }
                            Err(e) => {
                                self.status_message = Some((format!("Safety Check Failed: {}", e), true));
                            }
                        }
                    }
                });
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-h" | "--help" => {
                println!("Pop! Profile Manager Settings GUI (v1.0.0)");
                println!("Usage: pop-profile-gui [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -r, --reset      Reset all system settings back to Pop!_OS factory defaults");
                println!("  -h, --help       Print help information");
                println!("  -V, --version    Print version information");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("pop-profile-gui 1.0.0");
                return Ok(());
            }
            "-r" | "--reset" | "reset" | "--default" | "default" => {
                println!("[*] Resetting to Pop!_OS factory defaults...");
                let dbus_res = std::process::Command::new("gdbus")
                    .args([
                        "call",
                        "--system",
                        "--dest",
                        "io.github.mzia.PopProfile",
                        "--object-path",
                        "/io/github/mzia/PopProfile",
                        "--method",
                        "io.github.mzia.PopProfile.ResetToDefaults",
                    ])
                    .output();
                if let Ok(out) = dbus_res {
                    if out.status.success() {
                        println!("[+] Successfully reset to Pop!_OS factory defaults (via D-Bus daemon).");
                        return Ok(());
                    }
                }
                match system::reset_to_defaults() {
                    Ok(_) => println!("[+] Successfully reset to Pop!_OS factory defaults."),
                    Err(e) => eprintln!("[-] Reset failed: {}", e),
                }
                return Ok(());
            }
            _ => {}
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pop! Profile Manager")
            .with_app_id("io.github.mzia.PopProfile.Settings")
            .with_inner_size([880.0, 620.0])
            .with_min_inner_size([720.0, 480.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Pop! Profile Manager",
        native_options,
        Box::new(|cc| {
            // Apply COSMIC styling
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(28, 30, 36);
            visuals.window_fill = egui::Color32::from_rgb(33, 36, 44);
            visuals.selection.bg_fill = egui::Color32::from_rgb(246, 166, 35);
            visuals.window_corner_radius = egui::CornerRadius::same(8);
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(GuiApp::new(cc)))
        }),
    )
}
