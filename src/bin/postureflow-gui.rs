use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use postureflow::autoflow::{self, ActiveNetworkInfo, AutoFlowConfig, NetworkRule, VpnRule};
use postureflow::config::{
    self, validate_and_sanitize, DesktopConfig, DnsConfig, FirewallConfig, FrameworkPowerConfig,
    HooksConfig, PeripheralsConfig, ProfileConfig, ProfileMetadata, SecurityLimitsConfig,
};
use postureflow::inspector::{self, ListeningPort, PostureScoreReport};
use postureflow::triggers::{self, TriggerRule, TriggersConfig};
use postureflow::schedule::{self, CircadianConfig, ScheduleWindow};
use postureflow::hardware;
use postureflow::system;
use postureflow::theme;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum MainTab {
    Cockpit,
    Ports,
    Hardware,
    AutoFlow,
    Triggers,
    Schedule,
    Profiles,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ProfileSubTab {
    General,
    Firewall,
    Kernel,
    Power,
    Hooks,
    Dns,
}

struct GuiApp {
    main_tab: MainTab,
    profile_tab: ProfileSubTab,

    // Profile state
    profiles: Vec<ProfileConfig>,
    selected_index: usize,
    active_profile_id: String,
    status_message: Option<(String, bool)>, // (message, is_error)
    show_reset_confirm: bool,

    // Security Cockpit & Inspector state
    score_report: PostureScoreReport,
    listening_ports: Vec<ListeningPort>,
    port_search_query: String,

    // Auto-Flow state
    autoflow_config: AutoFlowConfig,
    active_network: ActiveNetworkInfo,
    new_rule_ssid: String,
    new_rule_profile: String,
    new_rule_comment: String,
    new_vpn_match: String,
    new_vpn_profile: String,
    new_vpn_comment: String,

    // App Triggers state
    triggers_config: TriggersConfig,
    new_trigger_name: String,
    new_trigger_procs: String,
    new_trigger_profile: String,
    new_trigger_epp: String,
    new_trigger_comment: String,

    // Circadian Schedule & Battery state
    schedule_config: CircadianConfig,
    new_sched_name: String,
    new_sched_days: String,
    new_sched_start: String,
    new_sched_end: String,
    new_sched_profile: String,
    new_sched_comment: String,

    // Profile form inputs
    new_port_str: String,
    new_port_comment: String,
    new_sysctl_key: String,
    new_sysctl_val: String,
    new_dns_server: String,
    new_dns_fallback: String,
    new_dns_domain: String,

    // Hardware Defenses & Sensor Privacy state
    hw_config: hardware::HardwareDefenseConfig,
    yubikey_devices: Vec<hardware::yubikey::YubikeyDevice>,
    honeypot_incidents: Vec<hardware::honeypot::HoneypotIncident>,
    paired_bt_devices: Vec<(String, String)>,
    new_trap_port_str: String,
    theme_config: theme::DynamicThemeConfig,

    // Single-instance IPC channel
    ipc_rx: std::sync::mpsc::Receiver<String>,
}

impl GuiApp {
    fn new(_cc: &eframe::CreationContext<'_>, ipc_rx: std::sync::mpsc::Receiver<String>) -> Self {
        let profiles = config::load_all_profiles();
        let active_profile_id = system::get_active_profile();
        let score_report = PostureScoreReport::compute();
        let listening_ports = inspector::ports::scan_listening_ports();
        let autoflow_config = autoflow::load_autoflow_config();
        let active_network = autoflow::detect_active_networks();

        let mut initial_tab = MainTab::Cockpit;
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|a| a == "--tab" || a == "-t") {
            if args.iter().any(|a| a == "ports" || a == "inspector") {
                initial_tab = MainTab::Ports;
            } else if args.iter().any(|a| a == "hardware" || a == "tether" || a == "honeypot") {
                initial_tab = MainTab::Hardware;
            } else if args.iter().any(|a| a == "autoflow") {
                initial_tab = MainTab::AutoFlow;
            } else if args.iter().any(|a| a == "triggers") {
                initial_tab = MainTab::Triggers;
            } else if args.iter().any(|a| a == "schedule" || a == "circadian") {
                initial_tab = MainTab::Schedule;
            } else if args.iter().any(|a| a == "profiles") {
                initial_tab = MainTab::Profiles;
            }
        }

        Self {
            main_tab: initial_tab,
            profile_tab: ProfileSubTab::General,
            profiles,
            selected_index: 0,
            active_profile_id,
            status_message: None,
            show_reset_confirm: false,
            score_report,
            listening_ports,
            port_search_query: String::new(),
            autoflow_config,
            active_network,
            new_rule_ssid: String::new(),
            new_rule_profile: "home".to_string(),
            new_rule_comment: String::new(),
            new_vpn_match: String::new(),
            new_vpn_profile: "dev".to_string(),
            new_vpn_comment: String::new(),
            triggers_config: triggers::load_triggers_config(),
            new_trigger_name: String::new(),
            new_trigger_procs: String::new(),
            new_trigger_profile: "home".to_string(),
            new_trigger_epp: "performance".to_string(),
            new_trigger_comment: String::new(),
            schedule_config: schedule::load_schedule_config(),
            new_sched_name: String::new(),
            new_sched_days: "Mon, Tue, Wed, Thu, Fri".to_string(),
            new_sched_start: "09:00".to_string(),
            new_sched_end: "17:00".to_string(),
            new_sched_profile: "work".to_string(),
            new_sched_comment: String::new(),
            new_port_str: "8080/tcp".to_string(),
            new_port_comment: "Web Development".to_string(),
            new_sysctl_key: "fs.inotify.max_user_watches".to_string(),
            new_sysctl_val: "524288".to_string(),
            new_dns_server: String::new(),
            new_dns_fallback: String::new(),
            new_dns_domain: String::new(),
            hw_config: hardware::load_hardware_config(),
            yubikey_devices: hardware::yubikey::detect_yubikeys(),
            honeypot_incidents: hardware::honeypot::load_recent_incidents(),
            paired_bt_devices: hardware::proximity::list_paired_bluetooth_devices(),
            new_trap_port_str: "2222".to_string(),
            theme_config: theme::load_theme_config(),
            ipc_rx,
        }
    }

    fn refresh_all(&mut self) {
        self.profiles = config::load_all_profiles();
        self.active_profile_id = system::get_active_profile();
        self.score_report = PostureScoreReport::compute();
        self.listening_ports = inspector::ports::scan_listening_ports();
        self.autoflow_config = autoflow::load_autoflow_config();
        self.active_network = autoflow::detect_active_networks();
        self.triggers_config = triggers::load_triggers_config();
        self.schedule_config = schedule::load_schedule_config();
        self.hw_config = hardware::load_hardware_config();
        self.yubikey_devices = hardware::yubikey::detect_yubikeys();
        self.honeypot_incidents = hardware::honeypot::load_recent_incidents();
        self.paired_bt_devices = hardware::proximity::list_paired_bluetooth_devices();
        self.theme_config = theme::load_theme_config();
        if self.selected_index >= self.profiles.len() {
            self.selected_index = 0;
        }
    }

    fn current_profile(&self) -> Option<&ProfileConfig> {
        self.profiles.get(self.selected_index)
    }

    fn do_factory_reset(&mut self) {
        let mut success = false;
        let mut err_msg = String::new();

        let dbus_attempt = std::process::Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "io.github.mzia.PostureFlow",
                "--object-path",
                "/io/github/mzia/PostureFlow",
                "--method",
                "io.github.mzia.PostureFlow.ResetToDefaults",
            ])
            .output();

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
            self.refresh_all();
            self.status_message = Some((
                "Pop!_OS factory defaults restored successfully (UFW reset/disabled, balanced power, battery 100%).".to_string(),
                false,
            ));
        } else {
            self.status_message = Some((format!("Failed to reset factory defaults: {}", err_msg), true));
        }
    }

    fn block_port_in_ufw(&mut self, port: u16, proto: &str) {
        let proto_owned = proto.to_string();
        let dbus_res = std::process::Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "io.github.mzia.PostureFlow",
                "--object-path",
                "/io/github/mzia/PostureFlow",
                "--method",
                "io.github.mzia.PostureFlow.BlockPort",
                &port.to_string(),
                &format!("'{}'", proto_owned),
            ])
            .output();

        let mut ok = false;
        if let Ok(out) = dbus_res {
            if out.status.success() {
                ok = true;
            }
        }

        if !ok {
            if let Ok(()) = system::block_port(port, &proto_owned) {
                ok = true;
            }
        }

        if ok {
            self.status_message = Some((
                format!("Port {}/{} successfully blocked in UFW firewall.", port, proto.to_uppercase()),
                false,
            ));
            self.refresh_all();
        } else {
            self.status_message = Some((
                format!("Failed to block port {}/{}. Ensure privileges are granted.", port, proto),
                true,
            ));
        }
    }

    fn whitelist_port_in_active_profile(&mut self, port: u16, proto: &str, comment: &str) {
        let rule_str = format!("{}/{}", port, proto);
        if let Some(prof) = self.profiles.iter_mut().find(|p| p.profile.id == self.active_profile_id) {
            if !prof.firewall.allow_ports.iter().any(|p| p.port == rule_str) {
                prof.firewall.allow_ports.push(config::PortRule {
                    port: rule_str.clone(),
                    comment: comment.to_string(),
                });
                let _ = config::save_custom_profile(prof.clone(), system::is_privileged());
                let _ = system::apply_profile_by_id(&self.active_profile_id);
                self.status_message = Some((
                    format!("Whitelisted {} in active profile [{}].", rule_str, self.active_profile_id.to_uppercase()),
                    false,
                ));
                self.refresh_all();
            }
        }
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Process single-instance IPC messages to switch tabs and focus window
        while let Ok(msg) = self.ipc_rx.try_recv() {
            if msg == "tab:ports" || msg == "ports" {
                self.main_tab = MainTab::Ports;
            } else if msg == "tab:cockpit" || msg == "cockpit" {
                self.main_tab = MainTab::Cockpit;
            } else if msg == "tab:autoflow" {
                self.main_tab = MainTab::AutoFlow;
            } else if msg == "tab:triggers" {
                self.main_tab = MainTab::Triggers;
            } else if msg == "tab:schedule" {
                self.main_tab = MainTab::Schedule;
            } else if msg == "tab:profiles" {
                self.main_tab = MainTab::Profiles;
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
        }

        // Auto-sync active profile with live system state
        let current_sys_profile = system::get_active_profile();
        if current_sys_profile != self.active_profile_id {
            self.active_profile_id = current_sys_profile;
        }

        // Notification Banner
        let mut dismiss = false;
        if let Some((ref msg, is_err)) = self.status_message {
            let banner_color = if is_err {
                Color32::from_rgb(220, 50, 50)
            } else {
                Color32::from_rgb(46, 160, 67)
            };
            egui::Panel::top("status_banner")
                .frame(egui::Frame::new().fill(banner_color).inner_margin(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(msg).color(Color32::WHITE).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                dismiss = true;
                            }
                        });
                    });
                });
        }
        if dismiss {
            self.status_message = None;
        }

        // Top Navigation Header (COSMIC Style, fully responsive for any window size)
        egui::Panel::top("nav_header")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(22, 24, 32))
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                // Tier 1: App Title on left, Active Badge + Refresh on right
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("PostureFlow")
                            .color(Color32::from_rgb(72, 185, 199))
                            .strong(),
                    );
                    ui.label(RichText::new("v1.0.1").color(Color32::from_rgb(140, 150, 175)).small());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            self.refresh_all();
                        }

                        let (badge_text, badge_color) = match self.active_profile_id.as_str() {
                            "home" => ("ACTIVE: [HOME]", Color32::from_rgb(16, 185, 129)),
                            "work" => ("ACTIVE: [WORK]", Color32::from_rgb(59, 130, 246)),
                            "dev" => ("ACTIVE: [DEV]", Color32::from_rgb(245, 158, 11)),
                            "travel" | "secure" => ("ACTIVE: [TRAVEL]", Color32::from_rgb(239, 68, 68)),
                            _ => ("ACTIVE: [DEFAULT]", Color32::from_rgb(246, 166, 35)),
                        };

                        egui::Frame::new()
                            .fill(badge_color.linear_multiply(0.18))
                            .stroke(Stroke::new(1.0, badge_color))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(badge_text)
                                        .color(badge_color)
                                        .strong()
                                        .size(11.0),
                                );
                            });
                    });
                });

                ui.add_space(8.0);

                // Tier 2: Navigation Tabs with responsive wrapping (never overlaps with title or badge!)
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    ui.selectable_value(&mut self.main_tab, MainTab::Cockpit, "🛡️ Security Cockpit");
                    ui.selectable_value(&mut self.main_tab, MainTab::Ports, "🔌 Port Inspector");
                    ui.selectable_value(&mut self.main_tab, MainTab::Hardware, "🔐 Hardware & Traps");
                    ui.selectable_value(&mut self.main_tab, MainTab::AutoFlow, "⚡ Auto-Flow (Network)");
                    ui.selectable_value(&mut self.main_tab, MainTab::Triggers, "🎮 App Triggers");
                    ui.selectable_value(&mut self.main_tab, MainTab::Schedule, "🕒 Schedule & Battery");
                    ui.selectable_value(&mut self.main_tab, MainTab::Profiles, "🏷️ Profiles Manager");
                });
            });


        // Reset to Factory Defaults Modal
        if self.show_reset_confirm {
            egui::Window::new("Reset to Pop!_OS Factory Defaults")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.heading("⚠️ Confirm Factory Reset");
                    ui.add_space(6.0);
                    ui.label("This will revert all system settings to unmanaged Pop!_OS defaults:");
                    ui.label("• Disable and reset UFW firewall (allow standard traffic)");
                    ui.label("• Remove custom sysctl optimizations and limits drop-ins");
                    ui.label("• Restore platform power profile to 'Balanced'");
                    ui.label("• Restore battery charge threshold to 100%");
                    ui.label("• Restore desktop screen idle timeout to 15 minutes");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new("Yes, Reset to Defaults")
                                    .color(Color32::from_rgb(255, 120, 120))
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

        // Main Panel Routing
        egui::CentralPanel::default().show(ui, |ui| match self.main_tab {
            MainTab::Cockpit => self.render_cockpit_view(ui),
            MainTab::Ports => self.render_ports_view(ui),
            MainTab::Hardware => self.render_hardware_view(ui),
            MainTab::AutoFlow => self.render_autoflow_view(ui),
            MainTab::Triggers => self.render_triggers_view(ui),
            MainTab::Schedule => self.render_schedule_view(ui),
            MainTab::Profiles => self.render_profiles_view(ui),
        });
    }
}

// VIEW IMPLEMENTATIONS
impl GuiApp {
    // 1. SECURITY COCKPIT VIEW
    fn render_cockpit_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading("🛡️ Real-Time Security Posture Cockpit");
            ui.label(
                RichText::new(
                    "Comprehensive hardware and network audit scoring your machine's resistance to local and remote attack vectors.",
                )
                .color(Color32::from_rgb(160, 170, 195)),
            );
            ui.add_space(12.0);

            // Large Score Hero Card
            let grade_color = match self.score_report.letter_grade.as_str() {
                "A+" | "A" => Color32::from_rgb(80, 250, 123),
                "B" => Color32::from_rgb(72, 185, 199),
                "C" => Color32::from_rgb(246, 166, 35),
                _ => Color32::from_rgb(255, 85, 85),
            };

            egui::Frame::new()
                .fill(Color32::from_rgb(30, 33, 44))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Overall Posture Score").small().color(Color32::from_rgb(160, 170, 195)));
                            ui.heading(
                                RichText::new(format!("{}/100", self.score_report.total_score))
                                    .size(36.0)
                                    .color(grade_color)
                                    .strong(),
                            );
                        });

                        ui.add_space(20.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new("Security Grade").small().color(Color32::from_rgb(160, 170, 195)));
                            ui.heading(
                                RichText::new(format!("[Grade {}]", self.score_report.letter_grade))
                                    .size(28.0)
                                    .color(grade_color),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.active_profile_id != "travel" {
                                if ui
                                    .button(
                                        RichText::new("⚡ One-Click Lockdown (Engage Travel)")
                                            .color(Color32::BLACK)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    let _ = system::apply_profile_by_id("travel");
                                    self.status_message = Some((
                                        "Travel Lockdown posture engaged! Inbound firewall sealed.".to_string(),
                                        false,
                                    ));
                                    self.refresh_all();
                                }
                            } else {
                                ui.label(
                                    RichText::new("✓ Stealth Travel Mode Engaged")
                                        .color(Color32::from_rgb(80, 250, 123))
                                        .strong(),
                                );
                            }
                        });
                    });

                    ui.add_space(10.0);
                    let progress = self.score_report.total_score as f32 / 100.0;
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .text(format!("{}% Protected", self.score_report.total_score))
                            .animate(false),
                    );
                });

            ui.add_space(16.0);

            // 4 Breakdown Cards Grid
            ui.columns(4, |cols| {
                // Card 1: Firewall
                cols[0].group(|ui| {
                    ui.label(RichText::new("Firewall Protection").strong());
                    ui.heading(format!("{}/35 pts", self.score_report.firewall_score));
                    ui.separator();
                    let ufw_str = if self.score_report.ufw_active { "Active (Enforced)" } else { "Inactive (Disabled)" };
                    let ufw_clr = if self.score_report.ufw_active { Color32::from_rgb(80, 250, 123) } else { Color32::from_rgb(255, 85, 85) };
                    ui.label(RichText::new(ufw_str).color(ufw_clr).small());
                    ui.label(RichText::new("Default Inbound: Deny").small());
                    ui.label(RichText::new("Loopback: Isolated").small());
                });

                // Card 2: Attack Surface
                cols[1].group(|ui| {
                    ui.label(RichText::new("Attack Surface").strong());
                    ui.heading(format!("{}/25 pts", self.score_report.attack_surface_score));
                    ui.separator();
                    ui.label(format!("Public Ports: {}", self.score_report.public_ports_count));
                    ui.label(format!("Local Ports:  {}", self.score_report.local_ports_count));
                    if ui.button("Inspect Sockets ➔").clicked() {
                        self.main_tab = MainTab::Ports;
                    }
                });

                // Card 3: Kernel Hardening
                cols[2].group(|ui| {
                    ui.label(RichText::new("Kernel Hardening").strong());
                    ui.heading(format!("{}/25 pts", self.score_report.kernel_score));
                    ui.separator();
                    ui.label(format!("ptrace_scope: {}", self.score_report.ptrace_scope));
                    ui.label(format!("bpf_disabled: {}", self.score_report.bpf_disabled));
                    ui.label(format!("syncookies:   {}", if self.score_report.syncookies { "1" } else { "0" }));
                });

                // Card 4: Inactivity & Memory
                cols[3].group(|ui| {
                    ui.label(RichText::new("Desktop & Limits").strong());
                    ui.heading(format!("{}/15 pts", self.score_report.limits_score));
                    ui.separator();
                    ui.label(format!("Lock delay: {}m", self.score_report.idle_timeout_seconds / 60));
                    ui.label("suid_dumpable: 2 (gdb)");
                    ui.label("file watchers: 524k");
                });
            });

            ui.add_space(12.0);

            // Active DNS Encryption & Privacy Card
            let dns_status = system::get_dns_status();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🌐 Active DNS Privacy & Encryption:").strong());
                    if dns_status.is_encrypted {
                        ui.label(
                            RichText::new(format!("🔒 ENCRYPTED (DoT: {}, DNSSEC: {})", dns_status.dns_over_tls.to_uppercase(), dns_status.dnssec.to_uppercase()))
                                .color(Color32::from_rgb(80, 250, 123))
                                .strong(),
                        );
                    } else {
                        ui.label(
                            RichText::new("⚠️ PLAINTEXT (Unencrypted UDP 53)")
                                .color(Color32::from_rgb(246, 166, 35))
                                .strong(),
                        );
                    }

                    if dns_status.managed_by_postureflow {
                        ui.label(RichText::new("[Managed by PostureFlow]").small().color(Color32::from_rgb(72, 185, 199)));
                    }
                });

                if !dns_status.active_servers.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Active Resolvers:").small().color(Color32::from_rgb(160, 170, 195)));
                        for srv in &dns_status.active_servers {
                            egui::Frame::new()
                                .fill(Color32::from_rgb(38, 42, 56))
                                .inner_margin(egui::Margin::symmetric(6, 2))
                                .corner_radius(CornerRadius::same(4))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(srv).small().color(Color32::from_rgb(180, 190, 215)));
                                });
                        }
                    });
                }
            });

            ui.add_space(8.0);

            // Hardware & Sensor Privacy Card
            let sensor_rep = hardware::sensors::get_sensor_privacy_report();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎙️ Hardware & Sensor Privacy:").strong());

                    // Camera Toggle
                    if sensor_rep.camera_blocked {
                        if ui.button(RichText::new("📷 Camera: BLOCKED").color(Color32::from_rgb(80, 250, 123)).strong()).clicked() {
                            let _ = system::apply_camera_blocked(false);
                            self.refresh_all();
                        }
                    } else {
                        if ui.button(RichText::new("📷 Camera: ACTIVE").color(Color32::from_rgb(255, 85, 85))).clicked() {
                            let _ = system::apply_camera_blocked(true);
                            self.refresh_all();
                        }
                    }

                    // Mic Toggle
                    if sensor_rep.microphone_muted {
                        if ui.button(RichText::new("🎙️ Mic: MUTED").color(Color32::from_rgb(80, 250, 123)).strong()).clicked() {
                            let _ = system::apply_microphone_muted(false);
                            self.refresh_all();
                        }
                    } else {
                        if ui.button(RichText::new("🎙️ Mic: ACTIVE").color(Color32::from_rgb(255, 85, 85))).clicked() {
                            let _ = system::apply_microphone_muted(true);
                            self.refresh_all();
                        }
                    }

                    // Location Toggle
                    if sensor_rep.location_blocked {
                        if ui.button(RichText::new("📍 Location: BLOCKED").color(Color32::from_rgb(80, 250, 123)).strong()).clicked() {
                            let _ = system::apply_location_blocked(false);
                            self.refresh_all();
                        }
                    } else {
                        if ui.button(RichText::new("📍 Location: ACTIVE").color(Color32::from_rgb(246, 166, 35))).clicked() {
                            let _ = system::apply_location_blocked(true);
                            self.refresh_all();
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("🚨 Emergency Kill Switch").color(Color32::from_rgb(255, 85, 85)).strong()).clicked() {
                            let _ = hardware::sensors::emergency_kill_all_sensors();
                            self.status_message = Some(("Emergency Sensor Kill Switch engaged!".to_string(), false));
                            self.refresh_all();
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Row for YubiKey Tether & Honeypot Status Cards
            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.label(RichText::new("🔑 YubiKey Physical Tether").strong());
                    if self.hw_config.yubikey.enabled {
                        ui.label(RichText::new("Status: ARMED & ENFORCED").color(Color32::from_rgb(80, 250, 123)).small());
                    } else {
                        ui.label(RichText::new("Status: Standby (Unarmed)").color(Color32::from_rgb(160, 170, 195)).small());
                    }
                    if !self.yubikey_devices.is_empty() {
                        let k = &self.yubikey_devices[0];
                        ui.label(RichText::new(format!("Token: {} (SN: {})", k.product_name, k.serial.as_deref().unwrap_or("Attached"))).small());
                    } else {
                        ui.label(RichText::new("No YubiKey detected on USB bus").small().color(Color32::from_rgb(246, 166, 35)));
                    }
                    if ui.button("Configure Hardware Tether ➔").clicked() {
                        self.main_tab = MainTab::Hardware;
                    }
                });

                cols[1].group(|ui| {
                    ui.label(RichText::new("🪤 Decoy Honeypot Traps").strong());
                    if self.hw_config.honeypot.enabled {
                        ui.label(RichText::new(format!("Status: ACTIVE ({} trap ports)", self.hw_config.honeypot.trap_ports.len())).color(Color32::from_rgb(80, 250, 123)).small());
                    } else {
                        ui.label(RichText::new("Status: Inactive").color(Color32::from_rgb(160, 170, 195)).small());
                    }
                    ui.label(RichText::new(format!("Intrusion Incidents Logged: {}", self.honeypot_incidents.len())).small());
                    if ui.button("View Honeypot & Incidents ➔").clicked() {
                        self.main_tab = MainTab::Hardware;
                    }
                });
            });

            ui.add_space(8.0);

            // Row 2 for Motion Sentry & Ambient Theme Cards
            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.label(RichText::new("📳 Framework Motion Sentry").strong());
                    if self.hw_config.motion.armed {
                        ui.label(RichText::new("Status: ARMED & SENSING").color(Color32::from_rgb(80, 250, 123)).small());
                    } else {
                        ui.label(RichText::new("Status: Standby (Disarmed)").color(Color32::from_rgb(160, 170, 195)).small());
                    }
                    if let Some(v) = hardware::motion::read_accelerometer_vector() {
                        ui.label(RichText::new(format!("EC Sensor: [X:{}, Y:{}, Z:{}]", v.x, v.y, v.z)).small().color(Color32::from_rgb(180, 190, 215)));
                    } else {
                        ui.label(RichText::new("Accelerometer not available").small().color(Color32::from_rgb(246, 166, 35)));
                    }
                    ui.horizontal(|ui| {
                        if self.hw_config.motion.armed {
                            if ui.button("🛑 Disarm Sentry").clicked() {
                                self.hw_config.motion.armed = false;
                                let _ = hardware::save_hardware_config(&self.hw_config, system::is_privileged());
                                self.status_message = Some(("Motion Sentry disarmed.".to_string(), false));
                            }
                        } else {
                            if ui.button(RichText::new("🛡️ Arm Sentry Now").color(Color32::BLACK).strong()).clicked() {
                                self.hw_config.motion.armed = true;
                                let _ = hardware::save_hardware_config(&self.hw_config, system::is_privileged());
                                self.status_message = Some(("Motion Sentry armed! Movement will trigger alarm.".to_string(), false));
                            }
                        }
                    });
                });

                cols[1].group(|ui| {
                    ui.label(RichText::new("🎨 Dynamic Ambient Theme").strong());
                    if self.theme_config.enabled {
                        ui.label(RichText::new("Status: ACTIVE (Posture-Sync)").color(Color32::from_rgb(80, 250, 123)).small());
                    } else {
                        ui.label(RichText::new("Status: Manual (Disabled)").color(Color32::from_rgb(160, 170, 195)).small());
                    }
                    if let Some(thm) = self.theme_config.themes.get(&self.active_profile_id) {
                        ui.label(RichText::new(format!("Active Theme: {}", thm.display_name)).small());
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Apply Posture Theme ➔").clicked() {
                            let _ = theme::apply_posture_theme(&self.active_profile_id);
                            self.status_message = Some(("Applied dynamic desktop theme and wallpaper!".to_string(), false));
                        }
                    });
                });
            });

            ui.add_space(16.0);

            // Actionable Recommendations Card
            ui.group(|ui| {
                ui.heading("📋 Security Findings & Recommendations");
                if self.score_report.recommendations.is_empty() {
                    ui.label(
                        RichText::new("✓ No vulnerabilities detected. Your laptop posture is fully hardened!")
                            .color(Color32::from_rgb(80, 250, 123)),
                    );
                } else {
                    for rec in &self.score_report.recommendations {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⚠️").color(Color32::from_rgb(246, 166, 35)));
                            ui.label(rec);
                        });
                    }
                }
            });
        });
    }

    // 2. LIVE PORT INSPECTOR VIEW
    fn render_ports_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("🔌 Live Listening Sockets Inspector");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh Sockets").clicked() {
                    self.listening_ports = inspector::ports::scan_listening_ports();
                }
            });
        });
        ui.label(
            RichText::new(
                "Inspect every socket listening on this machine, verify process ownership, and instantly block exposed ports.",
            )
            .color(Color32::from_rgb(160, 170, 195)),
        );

        ui.add_space(10.0);

        // Filter Bar & Counters
        let public_count = self.listening_ports.iter().filter(|p| p.is_public).count();
        let local_count = self.listening_ports.iter().filter(|p| p.is_local_only).count();

        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(egui::TextEdit::singleline(&mut self.port_search_query).hint_text("Filter by process, port, proto..."));

            ui.add_space(16.0);
            ui.label(RichText::new(format!("Total: {}", self.listening_ports.len())).strong());
            ui.label(RichText::new(format!("Exposed to LAN/WAN: {}", public_count)).color(if public_count > 0 { Color32::from_rgb(255, 100, 100) } else { Color32::from_rgb(80, 250, 123) }));
            ui.label(RichText::new(format!("Local Loopback: {}", local_count)).color(Color32::from_rgb(80, 250, 123)));
        });

        ui.add_space(10.0);
        ui.separator();

        let mut block_target: Option<(u16, String)> = None;
        let mut whitelist_target: Option<(u16, String, String)> = None;

        let query = self.port_search_query.to_lowercase();
        let filtered: Vec<&ListeningPort> = self
            .listening_ports
            .iter()
            .filter(|p| {
                if query.is_empty() {
                    return true;
                }
                p.port.to_string().contains(&query)
                    || p.process_name.to_lowercase().contains(&query)
                    || p.protocol.to_lowercase().contains(&query)
                    || p.service_hint.to_lowercase().contains(&query)
            })
            .collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for port in filtered {
                egui::Frame::new()
                    .fill(Color32::from_rgb(30, 33, 44))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Protocol badge
                            let proto_color = if port.protocol == "tcp" {
                                Color32::from_rgb(72, 185, 199)
                            } else {
                                Color32::from_rgb(180, 120, 255)
                            };
                            ui.label(RichText::new(port.protocol.to_uppercase()).color(proto_color).strong());

                            // Port & Address
                            ui.label(
                                RichText::new(format!("{}:{}", port.local_ip, port.port))
                                    .size(16.0)
                                    .strong(),
                            );

                            // Exposure badge
                            if port.is_public {
                                ui.label(
                                    RichText::new("⚠️ EXPOSED TO LAN/WAN")
                                        .color(Color32::from_rgb(255, 85, 85))
                                        .strong(),
                                );
                            } else {
                                ui.label(
                                    RichText::new("✓ LOCAL ONLY")
                                        .color(Color32::from_rgb(80, 250, 123))
                                        .small(),
                                );
                            }

                            // Process information
                            let pid_text = port.pid.map(|p| format!("PID {}", p)).unwrap_or_else(|| "System".to_string());
                            ui.label(RichText::new(format!("• {} ({})", port.process_name, pid_text)).color(Color32::from_rgb(190, 200, 220)));

                            // Service description hint
                            ui.label(RichText::new(format!("[{}]", port.service_hint)).color(Color32::from_rgb(140, 150, 175)).small());

                            // Quick actions
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if port.is_public {
                                    if ui
                                        .button(
                                            RichText::new("🚫 Block in UFW")
                                                .color(Color32::from_rgb(255, 120, 120)),
                                        )
                                        .clicked()
                                    {
                                        block_target = Some((port.port, port.protocol.clone()));
                                    }
                                }

                                if ui.button("➕ Whitelist in Profile").clicked() {
                                    whitelist_target = Some((port.port, port.protocol.clone(), port.service_hint.clone()));
                                }
                            });
                        });
                    });
                ui.add_space(6.0);
            }
        });

        if let Some((p, proto)) = block_target {
            self.block_port_in_ufw(p, &proto);
        }
        if let Some((p, proto, comment)) = whitelist_target {
            self.whitelist_port_in_active_profile(p, &proto, &comment);
        }
    }

    // 2.5 HARDWARE DEFENSES & TOKEN TETHERING VIEW
    fn render_hardware_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("🔐 Hardware Defenses, Physical Tokens & Traps");
                ui.label(
                    RichText::new(
                        "Zero-Trust YubiKey physical presence tethering, decoy honeypot trap ports, and Bluetooth RSSI walk-away distance security.",
                    )
                    .color(Color32::from_rgb(160, 170, 195)),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("💾 Save Hardware Config").color(Color32::BLACK).strong()).clicked() {
                    match hardware::save_hardware_config(&self.hw_config, system::is_privileged()) {
                        Ok(()) => self.status_message = Some(("Hardware defense configuration saved successfully.".to_string(), false)),
                        Err(e) => self.status_message = Some((format!("Failed to save hardware config: {}", e), true)),
                    }
                }
            });
        });

        ui.add_space(12.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // SECTION 1: YubiKey Physical Token Tethering
            egui::Frame::new()
                .fill(Color32::from_rgb(30, 33, 44))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🔑 YubiKey Hardware Presence Tethering");
                        if self.hw_config.yubikey.enabled {
                            ui.label(RichText::new("[ARMED & ENFORCED]").color(Color32::from_rgb(80, 250, 123)).strong());
                        } else {
                            ui.label(RichText::new("[DISABLED]").color(Color32::from_rgb(160, 170, 195)).small());
                        }
                    });

                    ui.label(
                        RichText::new(
                            "Continuously watches for your physical security token. Unplugging the token instantly locks your screen and demotes PostureFlow to lockdown posture.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.hw_config.yubikey.enabled, "Enable Hardware Token Presence Tether");
                    ui.checkbox(&mut self.hw_config.yubikey.lock_session_on_removal, "Lock Desktop Session Immediately Upon Unplugging");

                    ui.horizontal(|ui| {
                        ui.label("Emergency Fallback Posture:");
                        egui::ComboBox::from_id_salt("yubikey_fallback_combo")
                            .selected_text(self.hw_config.yubikey.fallback_profile.to_uppercase())
                            .show_ui(ui, |ui| {
                                for p in &self.profiles {
                                    ui.selectable_value(&mut self.hw_config.yubikey.fallback_profile, p.profile.id.clone(), p.profile.name.clone());
                                }
                            });
                    });

                    ui.checkbox(&mut self.hw_config.yubikey.restore_profile_on_insert, "Restore Profile Automatically When Plugged Back In");
                    ui.horizontal(|ui| {
                        ui.label("Restore Profile Target:");
                        egui::ComboBox::from_id_salt("yubikey_restore_combo")
                            .selected_text(self.hw_config.yubikey.preferred_profile.to_uppercase())
                            .show_ui(ui, |ui| {
                                for p in &self.profiles {
                                    ui.selectable_value(&mut self.hw_config.yubikey.preferred_profile, p.profile.id.clone(), p.profile.name.clone());
                                }
                            });
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("Discovered YubiKey Tokens on USB Bus:").strong());
                    if self.yubikey_devices.is_empty() {
                        ui.label(RichText::new("No YubiKey detected. Plug in your YubiKey C Bio to bind.").color(Color32::from_rgb(246, 166, 35)).small());
                    } else {
                        for (i, dev) in self.yubikey_devices.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("• [{}] {}", i + 1, dev.product_name));
                                ui.label(RichText::new(format!("(USB {}:{})", dev.vendor_id, dev.product_id)).small());
                                if let Some(ref s) = dev.serial {
                                    ui.label(RichText::new(format!("Serial: {}", s)).small().color(Color32::from_rgb(72, 185, 199)));
                                }
                            });
                        }
                    }
                });

            ui.add_space(16.0);

            // SECTION 2: Decoy Honeypot Port Traps
            egui::Frame::new()
                .fill(Color32::from_rgb(30, 33, 44))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🪤 Decoy Honeypot Port Traps & LAN Defense");
                        if self.hw_config.honeypot.enabled {
                            ui.label(RichText::new("[ACTIVE DECOYS]").color(Color32::from_rgb(80, 250, 123)).strong());
                        } else {
                            ui.label(RichText::new("[INACTIVE]").color(Color32::from_rgb(160, 170, 195)).small());
                        }
                    });

                    ui.label(
                        RichText::new(
                            "Opens silent decoy TCP listeners. If another host on public Wi-Fi or LAN probes your laptop, PostureFlow instantly bans their IP.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.hw_config.honeypot.enabled, "Enable Decoy Port Traps");
                    ui.checkbox(&mut self.hw_config.honeypot.auto_block_offender, "Automatically Ban Attacker IP via UFW Firewall");
                    ui.checkbox(&mut self.hw_config.honeypot.notify_on_intrusion, "Send Desktop Notifications Upon Intrusion");

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("Configured Decoy Trap Ports:").strong());

                    let mut remove_port = None;
                    for (idx, port) in self.hw_config.honeypot.trap_ports.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let hint = match *port {
                                2222 => "(Decoy SSH)",
                                8080 => "(Decoy HTTP Admin)",
                                4450 => "(Decoy SMB Share)",
                                _ => "(Decoy Trap)",
                            };
                            ui.label(format!("• TCP Port {} {}", port, hint));
                            if ui.button("🗑").clicked() {
                                remove_port = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = remove_port {
                        self.hw_config.honeypot.trap_ports.remove(idx);
                    }

                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.new_trap_port_str).hint_text("Port e.g. 3389").desired_width(100.0));
                        if ui.button("➕ Add Decoy Port").clicked() {
                            if let Ok(p) = self.new_trap_port_str.trim().parse::<u16>() {
                                if !self.hw_config.honeypot.trap_ports.contains(&p) {
                                    self.hw_config.honeypot.trap_ports.push(p);
                                }
                            }
                        }
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.label(RichText::new("Recent Intrusion Incidents:").strong());
                    if self.honeypot_incidents.is_empty() {
                        ui.label(RichText::new("✓ No unauthorized network probes detected.").small().color(Color32::from_rgb(80, 250, 123)));
                    } else {
                        for inc in self.honeypot_incidents.iter().rev().take(10) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠️").color(Color32::from_rgb(255, 85, 85)));
                                ui.label(RichText::new(&inc.source_ip).strong());
                                ui.label(format!("targeted TCP {}", inc.trap_port));
                                if inc.blocked {
                                    ui.label(RichText::new("[UFW BANNED]").color(Color32::from_rgb(80, 250, 123)).small());
                                } else {
                                    ui.label(RichText::new("[LOGGED]").small().color(Color32::from_rgb(246, 166, 35)));
                                }
                            });
                        }
                    }
                });

            ui.add_space(16.0);

            // SECTION 3: Bluetooth RSSI Walk-Away Proximity Auto-Lock
            egui::Frame::new()
                .fill(Color32::from_rgb(30, 33, 44))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🛰️ Bluetooth RSSI Proximity Auto-Lock");
                        if self.hw_config.proximity.enabled {
                            ui.label(RichText::new("[ARMED]").color(Color32::from_rgb(80, 250, 123)).strong());
                        } else {
                            ui.label(RichText::new("[DISABLED]").color(Color32::from_rgb(160, 170, 195)).small());
                        }
                    });

                    ui.label(
                        RichText::new(
                            "Walk-Away Security: pairs with your smartphone or smartwatch via BLE. When you walk away and signal drops, your laptop automatically locks.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.hw_config.proximity.enabled, "Enable Bluetooth Walk-Away Auto-Lock");

                    ui.horizontal(|ui| {
                        ui.label("Paired Device Target:");
                        let target_display = match (&self.hw_config.proximity.target_device_mac, &self.hw_config.proximity.target_device_name) {
                            (Some(mac), Some(name)) => format!("{} ({})", name, mac),
                            (Some(mac), None) => mac.clone(),
                            _ => "Select Paired Device...".to_string(),
                        };

                        egui::ComboBox::from_id_salt("bt_device_select")
                            .selected_text(target_display)
                            .show_ui(ui, |ui| {
                                for (mac, name) in &self.paired_bt_devices {
                                    if ui.selectable_label(self.hw_config.proximity.target_device_mac.as_deref() == Some(mac), format!("{} ({})", name, mac)).clicked() {
                                        self.hw_config.proximity.target_device_mac = Some(mac.clone());
                                        self.hw_config.proximity.target_device_name = Some(name.clone());
                                    }
                                }
                            });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Lock Distance RSSI Threshold:");
                        ui.add(egui::Slider::new(&mut self.hw_config.proximity.rssi_lock_threshold, -100..=-50).suffix(" dBm"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Walk-Away Grace Period:");
                        ui.add(egui::Slider::new(&mut self.hw_config.proximity.grace_period_seconds, 2..=30).suffix(" seconds"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Lock Posture Target:");
                        egui::ComboBox::from_id_salt("bt_lock_profile_combo")
                            .selected_text(self.hw_config.proximity.lock_profile.to_uppercase())
                            .show_ui(ui, |ui| {
                                for p in &self.profiles {
                                    ui.selectable_value(&mut self.hw_config.proximity.lock_profile, p.profile.id.clone(), p.profile.name.clone());
                                }
                            });
                    });
                });

            ui.add_space(16.0);

            // SECTION 4: Framework Laptop Accelerometer Anti-Theft Motion Sentry
            egui::Frame::new()
                .fill(Color32::from_rgb(30, 33, 44))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("📳 Framework Accelerometer Anti-Theft Motion Sentry");
                        if self.hw_config.motion.armed {
                            ui.label(RichText::new("[ARMED]").color(Color32::from_rgb(80, 250, 123)).strong());
                        } else {
                            ui.label(RichText::new("[DISARMED]").color(Color32::from_rgb(160, 170, 195)).small());
                        }
                    });

                    ui.label(
                        RichText::new(
                            "Monitors the Framework Laptop's embedded ChromeEC 3-axis accelerometer. If someone moves, lifts, or tilts your laptop while stepped away, PostureFlow triggers an audible alarm, locks the screen, and engages Travel lockdown.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.hw_config.motion.enabled, "Enable Accelerometer Sentry Subsystem");
                    ui.checkbox(&mut self.hw_config.motion.armed, "Arm Motion Sentry (Active Vigilance)");
                    ui.checkbox(&mut self.hw_config.motion.auto_arm_on_travel, "Automatically Arm Sentry When Entering Travel Mode");
                    ui.checkbox(&mut self.hw_config.motion.trigger_alarm_sound, "Sound Audio Alarm Siren on Displacement");
                    ui.checkbox(&mut self.hw_config.motion.auto_lock_screen, "Lock Desktop Screen Immediately on Displacement");
                    ui.checkbox(&mut self.hw_config.motion.auto_engage_travel, "Engage 100% Inbound Travel Stealth Lockdown on Displacement");

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Sensitivity Threshold (Displacement Δ):");
                        ui.add(egui::Slider::new(&mut self.hw_config.motion.sensitivity, 500..=5000).suffix(" units"));
                    });

                    if let Some(v) = hardware::motion::read_accelerometer_vector() {
                        ui.label(RichText::new(format!("Live ChromeEC Vector: X={}, Y={}, Z={}", v.x, v.y, v.z)).small().color(Color32::from_rgb(80, 250, 123)));
                    } else {
                        ui.label(RichText::new("Accelerometer device not detected in /sys/bus/iio/devices").small().color(Color32::from_rgb(246, 166, 35)));
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("🔊 Test Alarm Sound").clicked() {
                            hardware::motion::play_alarm_tone();
                        }
                    });
                });

            ui.add_space(16.0);

            // SECTION 5: Posture-Aware Dynamic Ambient Themes & Wallpapers
            egui::Frame::new()
                .fill(Color32::from_rgb(30, 33, 44))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🎨 Posture-Aware Dynamic COSMIC Wallpapers & Themes");
                        if self.theme_config.enabled {
                            ui.label(RichText::new("[ACTIVE]").color(Color32::from_rgb(80, 250, 123)).strong());
                        } else {
                            ui.label(RichText::new("[DISABLED]").color(Color32::from_rgb(160, 170, 195)).small());
                        }
                    });

                    ui.label(
                        RichText::new(
                            "Dynamically coordinates COSMIC theme accent colors and desktop wallpapers with your active posture (Amber for Home, Cyan for Work, Emerald for Dev, Crimson for Travel).",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.theme_config.enabled, "Enable Ambient Theme & Wallpaper Sync");
                    ui.checkbox(&mut self.theme_config.apply_accent, "Apply COSMIC Theme Accent Color");
                    ui.checkbox(&mut self.theme_config.apply_wallpaper, "Apply Desktop Wallpaper");

                    ui.add_space(8.0);
                    ui.label(RichText::new("Posture Palettes & Live Switcher:").strong());
                    ui.columns(4, |cols| {
                        let entries = [
                            ("home", "🏠 Home", Color32::from_rgb(255, 140, 0)),
                            ("work", "💼 Work", Color32::from_rgb(72, 185, 199)),
                            ("dev", "💻 Dev", Color32::from_rgb(38, 172, 114)),
                            ("travel", "✈️ Travel", Color32::from_rgb(255, 85, 85)),
                        ];

                        for (i, (id, label, color)) in entries.iter().enumerate() {
                            cols[i].group(|ui| {
                                ui.label(RichText::new(*label).strong().color(*color));
                                if let Some(t) = self.theme_config.themes.get(*id) {
                                    ui.label(RichText::new(&t.display_name).small());
                                }
                                if ui.button("Apply Theme").clicked() {
                                    let _ = theme::apply_posture_theme(id);
                                }
                            });
                        }
                    });

                    ui.add_space(8.0);
                    if ui.button(RichText::new("💾 Save Theme Settings").color(Color32::BLACK).strong()).clicked() {
                        let _ = theme::save_theme_config(&self.theme_config, system::is_privileged());
                        self.status_message = Some(("Dynamic theme configuration saved.".to_string(), false));
                    }
                });
        });
    }

    // 3. AUTO-FLOW (NETWORK DETECTION) VIEW
    fn render_autoflow_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("⚡ Auto-Flow: Autonomous Context & Network Detection");
        ui.label(
            RichText::new(
                "Configure PostureFlow to dynamically switch postures when connecting to known Wi-Fi SSIDs, VPN tunnels, or public hotspots.",
            )
            .color(Color32::from_rgb(160, 170, 195)),
        );

        ui.add_space(12.0);

        // Master Switch Card
        egui::Frame::new()
            .fill(Color32::from_rgb(30, 33, 44))
            .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.autoflow_config.enabled, "");
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new("Enable Auto-Flow Background Daemon Engine")
                                .color(if self.autoflow_config.enabled { Color32::from_rgb(80, 250, 123) } else { Color32::from_rgb(160, 170, 195) }),
                        );
                        ui.label(
                            RichText::new(
                                "Zero-overhead reactive monitor watches NetworkManager D-Bus and switches profiles automatically without typing passwords.",
                            )
                            .small(),
                        );
                    });
                });
            });

        ui.add_space(16.0);

        // Current Active Network Card
        ui.group(|ui| {
            ui.heading("🌐 Currently Connected Network");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Connection Type:").strong());
                ui.label(self.active_network.primary_type.to_uppercase());

                ui.add_space(16.0);
                ui.label(RichText::new("Active Wi-Fi SSID:").strong());
                let ssid_str = self.active_network.current_ssid.as_deref().unwrap_or("None (Not Connected to Wi-Fi)");
                ui.label(RichText::new(ssid_str).color(Color32::from_rgb(72, 185, 199)).strong());

                ui.add_space(16.0);
                ui.label(RichText::new("Active VPN:").strong());
                let vpn_str = self.active_network.active_vpn.as_deref().unwrap_or("None");
                ui.label(RichText::new(vpn_str).color(if self.active_network.active_vpn.is_some() { Color32::from_rgb(80, 250, 123) } else { Color32::from_rgb(160, 170, 195) }));
            });

            if !self.active_network.vpn_tunnels.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Detected Mesh / VPN Tunnels:").strong());
                    for tun in &self.active_network.vpn_tunnels {
                        let color = if tun.starts_with("tailscale") {
                            Color32::from_rgb(72, 185, 199)
                        } else if tun.starts_with("wg") {
                            Color32::from_rgb(180, 120, 255)
                        } else if tun.starts_with("zt") {
                            Color32::from_rgb(255, 184, 108)
                        } else {
                            Color32::from_rgb(80, 250, 123)
                        };
                        egui::Frame::new()
                            .fill(Color32::from_rgb(38, 42, 56))
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .corner_radius(CornerRadius::same(4))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("🔒 {}", tun)).color(color).strong().small());
                            });
                    }
                });
            }

            let matched = autoflow::evaluate_posture(&self.autoflow_config, &self.active_network);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Auto-Flow Target Posture:").strong());
                ui.label(
                    RichText::new(matched.as_deref().unwrap_or("No Rule Match").to_uppercase())
                        .color(Color32::from_rgb(246, 166, 35))
                        .strong(),
                );
            });
        });

        ui.add_space(16.0);

        // Fallback policy
        ui.horizontal(|ui| {
            ui.label(RichText::new("Unknown / Untrusted Wi-Fi Hotspot Posture:").strong());
            ui.add(egui::TextEdit::singleline(&mut self.autoflow_config.default_unknown_wifi).desired_width(100.0));
            ui.label(RichText::new("(Default: 'travel' for coffee shops, hotels, airports)").small().color(Color32::from_rgb(140, 150, 175)));
        });

        ui.add_space(12.0);

        // Wi-Fi Rules List
        ui.heading("📋 Network Assignment Rules");
        let mut delete_index = None;

        egui::ScrollArea::vertical().id_salt("wifi_rules_scroll").max_height(140.0).show(ui, |ui| {
            for (idx, rule) in self.autoflow_config.rules.iter().enumerate() {
                egui::Frame::new()
                    .fill(Color32::from_rgb(28, 31, 42))
                    .inner_margin(8)
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let match_str = if let Some(ref s) = rule.ssid {
                                format!("Wi-Fi SSID: \"{}\"", s)
                            } else if let Some(ref iface) = rule.interface {
                                format!("Interface: \"{}\"", iface)
                            } else {
                                "Any".to_string()
                            };

                            ui.label(RichText::new(match_str).color(Color32::from_rgb(72, 185, 199)).strong());
                            ui.label("➔");
                            ui.label(RichText::new(format!("[{}]", rule.profile.to_uppercase())).color(Color32::from_rgb(246, 166, 35)).strong());

                            if let Some(ref comm) = rule.comment {
                                ui.label(RichText::new(format!("({})", comm)).color(Color32::from_rgb(140, 150, 175)).small());
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new("🗑").color(Color32::from_rgb(255, 100, 100))).clicked() {
                                    delete_index = Some(idx);
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });

        if let Some(idx) = delete_index {
            self.autoflow_config.rules.remove(idx);
        }

        ui.add_space(8.0);

        // Add New Network Rule Form
        ui.group(|ui| {
            ui.label(RichText::new("Add Network Assignment Rule").strong());
            ui.horizontal(|ui| {
                ui.label("SSID:");
                ui.add(egui::TextEdit::singleline(&mut self.new_rule_ssid).hint_text("e.g. Starbucks WiFi"));
                ui.label("Target Profile:");
                ui.add(egui::TextEdit::singleline(&mut self.new_rule_profile).desired_width(80.0));
                ui.label("Note:");
                ui.add(egui::TextEdit::singleline(&mut self.new_rule_comment).hint_text("e.g. Local Cafe"));

                if ui.button("➕ Add Rule").clicked() && !self.new_rule_ssid.is_empty() {
                    self.autoflow_config.rules.push(NetworkRule {
                        ssid: Some(self.new_rule_ssid.trim().to_string()),
                        interface: None,
                        profile: self.new_rule_profile.trim().to_lowercase(),
                        comment: Some(self.new_rule_comment.trim().to_string()),
                    });
                    self.new_rule_ssid.clear();
                    self.new_rule_comment.clear();
                }
            });
        });

        ui.add_space(14.0);

        // VPN & Mesh Tunnel Routing Rules List
        ui.heading("🔒 VPN & Mesh Tunnel Routing Rules");
        ui.label(
            RichText::new(
                "Map specific VPN connections or mesh interfaces (e.g. Tailscale, WireGuard, ZeroTier) to target postures.",
            )
            .color(Color32::from_rgb(160, 170, 195))
            .small(),
        );
        ui.add_space(6.0);

        let mut delete_vpn_index = None;
        egui::ScrollArea::vertical().id_salt("vpn_rules_scroll").max_height(140.0).show(ui, |ui| {
            if self.autoflow_config.vpn_rules.is_empty() {
                ui.label(RichText::new("No VPN routing rules defined. Default VPN fallback profile will be used.").italics().color(Color32::from_rgb(140, 150, 175)));
            } else {
                for (idx, rule) in self.autoflow_config.vpn_rules.iter().enumerate() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(28, 31, 42))
                        .inner_margin(8)
                        .corner_radius(CornerRadius::same(6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Match: \"{}\"", rule.match_name)).color(Color32::from_rgb(180, 120, 255)).strong());
                                ui.label("➔");
                                ui.label(RichText::new(format!("[{}]", rule.profile.to_uppercase())).color(Color32::from_rgb(246, 166, 35)).strong());

                                if let Some(ref comm) = rule.comment {
                                    ui.label(RichText::new(format!("({})", comm)).color(Color32::from_rgb(140, 150, 175)).small());
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(RichText::new("🗑").color(Color32::from_rgb(255, 100, 100))).clicked() {
                                        delete_vpn_index = Some(idx);
                                    }
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            }
        });

        if let Some(idx) = delete_vpn_index {
            self.autoflow_config.vpn_rules.remove(idx);
        }

        ui.add_space(8.0);

        // Add New VPN Rule Form
        ui.group(|ui| {
            ui.label(RichText::new("Add VPN / Mesh Routing Rule").strong());
            ui.horizontal(|ui| {
                ui.label("Pattern:");
                ui.add(egui::TextEdit::singleline(&mut self.new_vpn_match).hint_text("e.g. tailscale*, wg0, zt*"));
                ui.label("Target Profile:");
                ui.add(egui::TextEdit::singleline(&mut self.new_vpn_profile).desired_width(80.0));
                ui.label("Note:");
                ui.add(egui::TextEdit::singleline(&mut self.new_vpn_comment).hint_text("e.g. Work WireGuard"));

                if ui.button("➕ Add VPN Rule").clicked() && !self.new_vpn_match.trim().is_empty() {
                    self.autoflow_config.vpn_rules.push(VpnRule {
                        match_name: self.new_vpn_match.trim().to_string(),
                        profile: self.new_vpn_profile.trim().to_lowercase(),
                        comment: if self.new_vpn_comment.trim().is_empty() {
                            None
                        } else {
                            Some(self.new_vpn_comment.trim().to_string())
                        },
                    });
                    self.new_vpn_match.clear();
                    self.new_vpn_comment.clear();
                }
            });
        });

        ui.add_space(12.0);

        if ui
            .button(
                RichText::new("💾 Save Auto-Flow Configuration")
                    .color(Color32::BLACK)
                    .strong(),
            )
            .clicked()
        {
            match autoflow::save_autoflow_config(&self.autoflow_config) {
                Ok(path) => {
                    self.status_message = Some((
                        format!("Auto-Flow configuration saved successfully to {}.", path.display()),
                        false,
                    ));
                }
                Err(e) => {
                    self.status_message = Some((format!("Failed to save configuration: {}", e), true));
                }
            }
        }
    }

    // 4. APP-AWARE DYNAMIC TRIGGERS VIEW
    fn render_triggers_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("🎮 App-Aware Dynamic Triggers");
        ui.label(
            RichText::new(
                "Temporarily adapt system posture (governor, CPU EPP, and kernel parameters) when specific applications launch.",
            )
            .color(Color32::from_rgb(160, 170, 195)),
        );
        ui.add_space(12.0);

        // Status & Active Detection Card
        let running_procs = triggers::scan_running_processes();
        let matched = triggers::evaluate_triggers(&self.triggers_config, &running_procs);

        egui::Frame::new()
            .fill(Color32::from_rgb(30, 33, 44))
            .stroke(Stroke::new(1.0, Color32::from_rgb(48, 54, 72)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.triggers_config.enabled, RichText::new("Enable App-Aware Dynamic Triggers").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status_text = if !self.triggers_config.enabled {
                            "Status: DISABLED"
                        } else if matched.is_some() {
                            "Status: ACTIVE OVERRIDE ENGAGED"
                        } else {
                            "Status: WATCHING (Idle)"
                        };
                        let status_clr = if !self.triggers_config.enabled {
                            Color32::from_rgb(160, 170, 195)
                        } else if matched.is_some() {
                            Color32::from_rgb(80, 250, 123)
                        } else {
                            Color32::from_rgb(72, 185, 199)
                        };
                        ui.label(RichText::new(status_text).color(status_clr).strong());
                    });
                });

                ui.add_space(8.0);
                if let Some(ref m) = matched {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Active Match:").strong());
                        ui.label(RichText::new(&m.rule_name).color(Color32::from_rgb(80, 250, 123)).strong());
                        ui.label(format!("(Detected: {})", m.matched_processes.join(", ")));
                    });
                    if let Some(ref target) = m.target_profile {
                        ui.label(format!("➔ Target Posture Override: [{}]", target.to_uppercase()));
                    }
                    if let Some(ref epp) = m.boost_cpu_epp {
                        ui.label(format!("➔ Dynamic CPU EPP: {}", epp));
                    }
                    for (k, v) in &m.boost_sysctl {
                        ui.label(format!("➔ Dynamic Sysctl: {} = {}", k, v));
                    }
                } else {
                    ui.label(
                        RichText::new("✓ No trigger processes currently active. Baseline system posture is engaged.")
                            .color(Color32::from_rgb(140, 150, 175)),
                    );
                }
            });

        ui.add_space(16.0);

        // Rules List
        ui.heading("📋 Configured Trigger Rules");
        let mut delete_index = None;

        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for (idx, rule) in self.triggers_config.rules.iter().enumerate() {
                egui::Frame::new()
                    .fill(Color32::from_rgb(28, 31, 42))
                    .inner_margin(10)
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&rule.name).strong().color(Color32::from_rgb(72, 185, 199)));
                            if let Some(ref target) = rule.target_profile {
                                ui.label(format!("➔ [{}]", target.to_uppercase()));
                            }
                            if let Some(ref epp) = rule.boost_cpu_epp {
                                ui.label(RichText::new(format!("EPP: {}", epp)).color(Color32::from_rgb(246, 166, 35)).small());
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new("🗑").color(Color32::from_rgb(255, 100, 100))).clicked() {
                                    delete_index = Some(idx);
                                }
                            });
                        });
                        ui.label(
                            RichText::new(format!("Binaries: {}", rule.process_names.join(", ")))
                                .small()
                                .color(Color32::from_rgb(140, 150, 175)),
                        );
                        if let Some(ref c) = rule.comment {
                            ui.label(RichText::new(c).small().color(Color32::from_rgb(160, 170, 195)));
                        }
                    });
                ui.add_space(4.0);
            }
        });

        if let Some(idx) = delete_index {
            self.triggers_config.rules.remove(idx);
            let _ = triggers::save_triggers_config(&self.triggers_config);
        }

        ui.add_space(16.0);

        // Add Rule Form
        ui.group(|ui| {
            ui.label(RichText::new("➕ Add New Trigger Rule").strong());
            ui.horizontal(|ui| {
                ui.label("Rule Name:");
                ui.add(egui::TextEdit::singleline(&mut self.new_trigger_name).desired_width(180.0));
                ui.label("Target Profile:");
                ui.add(egui::TextEdit::singleline(&mut self.new_trigger_profile).desired_width(70.0));
                ui.label("CPU EPP:");
                ui.add(egui::TextEdit::singleline(&mut self.new_trigger_epp).desired_width(90.0));
            });
            ui.horizontal(|ui| {
                ui.label("Watched Binaries (comma-separated):");
                ui.add(egui::TextEdit::singleline(&mut self.new_trigger_procs).desired_width(320.0));
            });
            ui.horizontal(|ui| {
                ui.label("Description:");
                ui.add(egui::TextEdit::singleline(&mut self.new_trigger_comment).desired_width(300.0));

                if ui.button(RichText::new("Add Rule").strong()).clicked() {
                    let name = self.new_trigger_name.trim().to_string();
                    let procs: Vec<String> = self.new_trigger_procs
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !name.is_empty() && !procs.is_empty() {
                        let target_prof = if self.new_trigger_profile.trim().is_empty() {
                            None
                        } else {
                            Some(self.new_trigger_profile.trim().to_lowercase())
                        };
                        let boost_epp = if self.new_trigger_epp.trim().is_empty() {
                            None
                        } else {
                            Some(self.new_trigger_epp.trim().to_lowercase())
                        };
                        let comment = if self.new_trigger_comment.trim().is_empty() {
                            None
                        } else {
                            Some(self.new_trigger_comment.trim().to_string())
                        };

                        self.triggers_config.rules.push(TriggerRule {
                            name,
                            process_names: procs,
                            target_profile: target_prof,
                            boost_cpu_epp: boost_epp,
                            boost_sysctl: HashMap::new(),
                            comment,
                        });

                        let _ = triggers::save_triggers_config(&self.triggers_config);
                        self.new_trigger_name.clear();
                        self.new_trigger_procs.clear();
                        self.new_trigger_comment.clear();
                        self.status_message = Some(("New App Trigger rule added.".to_string(), false));
                    }
                }
            });
        });

        ui.add_space(16.0);
        if ui
            .button(
                RichText::new("💾 Save Triggers Configuration")
                    .color(Color32::BLACK)
                    .strong(),
            )
            .clicked()
        {
            match triggers::save_triggers_config(&self.triggers_config) {
                Ok(path) => {
                    self.status_message = Some((
                        format!("Triggers configuration saved successfully to {}.", path.display()),
                        false,
                    ));
                }
                Err(e) => {
                    self.status_message = Some((format!("Failed to save triggers configuration: {}", e), true));
                }
            }
        }
    }

    // 5. CIRCADIAN SCHEDULE & BATTERY VIEW
    fn render_schedule_view(&mut self, ui: &mut egui::Ui) {
        let now = schedule::get_current_local_time();
        let battery = schedule::get_battery_status();
        let decision = schedule::evaluate_schedule(&self.schedule_config, &now, &battery);

        ui.horizontal(|ui| {
            ui.heading("🕒 Circadian Schedule & Battery Fallback");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (btn_text, btn_color) = if self.schedule_config.enabled {
                    ("SCHEDULE ENGINE: ACTIVE", Color32::from_rgb(80, 250, 123))
                } else {
                    ("SCHEDULE ENGINE: DISABLED", Color32::from_rgb(255, 100, 100))
                };
                if ui.button(RichText::new(btn_text).color(btn_color).strong()).clicked() {
                    self.schedule_config.enabled = !self.schedule_config.enabled;
                    let _ = schedule::save_schedule_config(&self.schedule_config);
                }
            });
        });

        ui.label(
            RichText::new("Automate lifestyle posture transitions based on time-of-day and protect against sudden battery drainage.")
                .color(Color32::from_rgb(160, 170, 195)),
        );
        ui.add_space(8.0);

        // Live Telemetry Banner
        egui::Frame::new()
            .fill(Color32::from_rgb(20, 22, 30))
            .inner_margin(12)
            .corner_radius(CornerRadius::same(8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🕒 System Clock:").strong());
                    ui.label(
                        RichText::new(format!("{:02}:{:02} ({})", now.hour, now.minute, now.weekday_name()))
                            .color(Color32::from_rgb(72, 185, 199))
                            .strong(),
                    );

                    ui.add_space(20.0);

                    ui.label(RichText::new("🔋 Battery:").strong());
                    let bat_pct = battery.capacity_percent.unwrap_or(0);
                    let bat_color = if bat_pct <= self.schedule_config.battery_emergency.threshold_percent && battery.is_discharging {
                        Color32::from_rgb(255, 100, 100)
                    } else if bat_pct <= 30 {
                        Color32::from_rgb(246, 166, 35)
                    } else {
                        Color32::from_rgb(80, 250, 123)
                    };
                    ui.label(
                        RichText::new(format!("{}% [{}]", bat_pct, battery.status))
                            .color(bat_color)
                            .strong(),
                    );
                    if battery.on_ac_power {
                        ui.label(RichText::new("⚡ AC Plugged").color(Color32::from_rgb(246, 166, 35)).small());
                    } else {
                        ui.label(RichText::new("🔋 On Battery").color(Color32::from_rgb(140, 150, 175)).small());
                    }
                });

                ui.add_space(8.0);
                match decision {
                    Some(schedule::ScheduleDecision::BatteryEmergency { ref rule_name, ref target_profile, battery_percent, .. }) => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🚨 CURRENT STATUS:").strong().color(Color32::from_rgb(255, 85, 85)));
                            ui.label(
                                RichText::new(format!("{} ({}% <= {}%) ➔ Fallback: [{}]",
                                    rule_name, battery_percent, self.schedule_config.battery_emergency.threshold_percent, target_profile.to_uppercase()))
                                    .color(Color32::from_rgb(255, 120, 120))
                                    .strong(),
                            );
                        });
                    }
                    Some(schedule::ScheduleDecision::ScheduledShift { ref rule_name, ref target_profile, ref window }) => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⏰ ACTIVE WINDOW:").strong().color(Color32::from_rgb(72, 185, 199)));
                            ui.label(
                                RichText::new(format!("{} ({}) ➔ Target Posture: [{}]", rule_name, window, target_profile.to_uppercase()))
                                    .color(Color32::from_rgb(80, 250, 123))
                                    .strong(),
                            );
                        });
                    }
                    None => {
                        ui.label(
                            RichText::new("✓ No schedule window matches current time. Baseline manual posture is active.")
                                .color(Color32::from_rgb(140, 150, 175)),
                        );
                    }
                }
            });

        ui.add_space(14.0);

        // Battery Emergency Settings Group
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔋 Battery-Critical Emergency Fallback").strong().color(Color32::from_rgb(246, 166, 35)));
                ui.checkbox(&mut self.schedule_config.battery_emergency.enabled, "Enable Emergency Mode");
            });

            if self.schedule_config.battery_emergency.enabled {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Trigger Threshold (%):");
                    ui.add(egui::Slider::new(&mut self.schedule_config.battery_emergency.threshold_percent, 5..=30).text("%"));

                    ui.add_space(16.0);
                    ui.label("Emergency Posture:");
                    egui::ComboBox::from_id_salt("emergency_profile_select")
                        .selected_text(&self.schedule_config.battery_emergency.target_profile)
                        .show_ui(ui, |ui| {
                            for prof in ["travel", "home", "work", "dev"] {
                                ui.selectable_value(&mut self.schedule_config.battery_emergency.target_profile, prof.to_string(), prof);
                            }
                        });
                });

                ui.horizontal(|ui| {
                    let mut force_epp = self.schedule_config.battery_emergency.force_cpu_epp.is_some();
                    if ui.checkbox(&mut force_epp, "Force CPU EPP to 'power' (max preservation)").changed() {
                        self.schedule_config.battery_emergency.force_cpu_epp = if force_epp { Some("power".to_string()) } else { None };
                    }
                    ui.checkbox(&mut self.schedule_config.battery_emergency.disable_bluetooth, "Shut down Bluetooth radio");
                    ui.checkbox(&mut self.schedule_config.battery_emergency.auto_recover_on_ac, "Auto-recover on AC power plug");
                });
            }
        });

        ui.add_space(14.0);

        // Scheduled Windows List
        ui.heading("📋 Scheduled Time Windows");
        let mut delete_index = None;

        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
            for (idx, win) in self.schedule_config.schedules.iter().enumerate() {
                egui::Frame::new()
                    .fill(Color32::from_rgb(28, 31, 42))
                    .inner_margin(10)
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&win.name).strong().color(Color32::from_rgb(72, 185, 199)));
                            ui.label(format!("➔ [{}]", win.target_profile.to_uppercase()));
                            ui.label(RichText::new(format!("⏰ {} - {}", win.start_time, win.end_time)).color(Color32::from_rgb(246, 166, 35)).small());
                            let days_display = if win.days.is_empty() { "* (Daily)".to_string() } else { win.days.join(", ") };
                            ui.label(RichText::new(format!("Days: {}", days_display)).color(Color32::from_rgb(140, 150, 175)).small());

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new("🗑").color(Color32::from_rgb(255, 100, 100))).clicked() {
                                    delete_index = Some(idx);
                                }
                            });
                        });
                        if let Some(ref c) = win.comment {
                            ui.label(RichText::new(c).small().color(Color32::from_rgb(160, 170, 195)));
                        }
                    });
                ui.add_space(4.0);
            }
        });

        if let Some(idx) = delete_index {
            self.schedule_config.schedules.remove(idx);
            let _ = schedule::save_schedule_config(&self.schedule_config);
        }

        ui.add_space(14.0);

        // Add Window Form
        ui.group(|ui| {
            ui.label(RichText::new("➕ Add New Schedule Window").strong());
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut self.new_sched_name).desired_width(160.0));
                ui.label("Start (HH:MM):");
                ui.add(egui::TextEdit::singleline(&mut self.new_sched_start).desired_width(50.0));
                ui.label("End (HH:MM):");
                ui.add(egui::TextEdit::singleline(&mut self.new_sched_end).desired_width(50.0));
                ui.label("Target:");
                egui::ComboBox::from_id_salt("new_sched_target_select")
                    .selected_text(&self.new_sched_profile)
                    .show_ui(ui, |ui| {
                        for p in ["work", "home", "dev", "travel"] {
                            ui.selectable_value(&mut self.new_sched_profile, p.to_string(), p);
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Days:");
                ui.add(egui::TextEdit::singleline(&mut self.new_sched_days).desired_width(180.0));
                if ui.button("Workdays").clicked() {
                    self.new_sched_days = "Mon, Tue, Wed, Thu, Fri".to_string();
                }
                if ui.button("Weekends").clicked() {
                    self.new_sched_days = "Sat, Sun".to_string();
                }
                if ui.button("Daily").clicked() {
                    self.new_sched_days = "*".to_string();
                }
                ui.label("Description:");
                ui.add(egui::TextEdit::singleline(&mut self.new_sched_comment).desired_width(160.0));
            });

            ui.add_space(4.0);
            if ui.button(RichText::new("➕ Add Window").color(Color32::from_rgb(80, 250, 123)).strong()).clicked() {
                if !self.new_sched_name.trim().is_empty() && schedule::TimeOfDay::parse(&self.new_sched_start).is_some() && schedule::TimeOfDay::parse(&self.new_sched_end).is_some() {
                    let days: Vec<String> = self.new_sched_days
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let comment = if self.new_sched_comment.trim().is_empty() {
                        None
                    } else {
                        Some(self.new_sched_comment.trim().to_string())
                    };

                    self.schedule_config.schedules.push(ScheduleWindow {
                        name: self.new_sched_name.trim().to_string(),
                        days,
                        start_time: self.new_sched_start.trim().to_string(),
                        end_time: self.new_sched_end.trim().to_string(),
                        target_profile: self.new_sched_profile.clone(),
                        comment,
                    });

                    let _ = schedule::save_schedule_config(&self.schedule_config);
                    self.new_sched_name.clear();
                    self.new_sched_comment.clear();
                    self.status_message = Some(("New schedule window added successfully.".to_string(), false));
                } else {
                    self.status_message = Some(("Please specify a valid window name and HH:MM times (e.g. 09:00 to 17:00).".to_string(), true));
                }
            }
        });

        ui.add_space(14.0);
        if ui
            .button(
                RichText::new("💾 Save Schedule & Battery Configuration")
                    .color(Color32::BLACK)
                    .strong(),
            )
            .clicked()
        {
            match schedule::save_schedule_config(&self.schedule_config) {
                Ok(()) => {
                    self.status_message = Some((
                        format!("Schedule & Battery configuration saved successfully to {}.", schedule::SYSTEM_SCHEDULE_FILE),
                        false,
                    ));
                }
                Err(e) => {
                    self.status_message = Some((format!("Failed to save schedule configuration: {}", e), true));
                }
            }
        }
    }

    // 6. PROFILES & SYSTEM SETTINGS VIEW (Standard Editor)
    fn render_profiles_view(&mut self, ui: &mut egui::Ui) {
        // Left Profiles Sidebar
        egui::Panel::left("profiles_sidebar")
            .resizable(false)
            .default_size(240.0)
            .show(ui, |ui| {
                ui.heading("PostureFlow Profiles");
                let active_header = if self.active_profile_id == "default" {
                    "Active: [FACTORY DEFAULT]".to_string()
                } else {
                    format!("Active: [{}]", self.active_profile_id.to_uppercase())
                };
                ui.label(
                    RichText::new(active_header)
                        .color(Color32::from_rgb(246, 166, 35))
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
                            RichText::new("🔄 Reset to Factory Defaults")
                                .color(Color32::from_rgb(255, 120, 120)),
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
                                    .set_file_name(&format!("{}.postureflow.toml", prof.profile.id))
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
                                                system::is_privileged(),
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
                                                    self.refresh_all();
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
                                description: "User custom posture profile configuration".to_string(),
                                icon: "preferences-system-symbolic".to_string(),
                                version: "1.0.0".to_string(),
                                author: std::env::var("USER").unwrap_or_else(|_| "user".to_string()),
                                is_builtin: false,
                            },
                            kernel: HashMap::new(),
                            limits: SecurityLimitsConfig::default(),
                            firewall: FirewallConfig::default(),
                            power: FrameworkPowerConfig::default(),
                            peripherals: PeripheralsConfig::default(),
                            desktop: DesktopConfig::default(),
                            hooks: HooksConfig::default(),
                            dns: DnsConfig::default(),
                        };
                        let _ = config::save_custom_profile(new_cfg, system::is_privileged());
                        self.refresh_all();
                        if let Some(pos) = self.profiles.iter().position(|p| p.profile.id == new_id) {
                            self.selected_index = pos;
                        }
                    }
                });
            });

        // Central Profile Editor Area
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
                    RichText::new("[System Built-in]")
                        .color(Color32::from_rgb(100, 180, 255))
                        .small(),
                );
            } else {
                ui.label(
                    RichText::new("[Custom Profile]")
                        .color(Color32::from_rgb(246, 166, 35))
                        .small(),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let activate_btn = ui.button(
                    RichText::new("⚡ Activate Profile")
                        .color(Color32::BLACK)
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
                            self.refresh_all();
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

        // Subtabs
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.profile_tab, ProfileSubTab::General, "🏷️ General");
            ui.selectable_value(&mut self.profile_tab, ProfileSubTab::Firewall, "🛡️ Firewall & Ports");
            ui.selectable_value(&mut self.profile_tab, ProfileSubTab::Kernel, "⚙️ Kernel & Sysctl");
            ui.selectable_value(&mut self.profile_tab, ProfileSubTab::Power, "⚡ Power & Framework");
            ui.selectable_value(&mut self.profile_tab, ProfileSubTab::Hooks, "🪝 Hooks & Services");
            ui.selectable_value(&mut self.profile_tab, ProfileSubTab::Dns, "🌐 DNS & Privacy");
        });
        ui.separator();

        // Tab Content
        egui::ScrollArea::vertical().show(ui, |ui| {
            let current_profile = &mut self.profiles[self.selected_index];

            match self.profile_tab {
                ProfileSubTab::General => {
                    ui.label(RichText::new("Profile Metadata").strong());
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

                    ui.label(RichText::new("Desktop Posture").strong());
                    let mut idle = current_profile.desktop.idle_delay_seconds.unwrap_or(300);
                    ui.horizontal(|ui| {
                        ui.label("Screen Lock Delay (seconds):");
                        if ui.add_enabled(!is_builtin, egui::Slider::new(&mut idle, 60..=3600)).changed() {
                            current_profile.desktop.idle_delay_seconds = Some(idle);
                        }
                    });
                }

                ProfileSubTab::Firewall => {
                    ui.label(RichText::new("UFW Traffic Policies").strong());
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

                    ui.add_enabled(
                        !is_builtin,
                        egui::Checkbox::new(&mut current_profile.firewall.allow_loopback, "Allow Loopback (lo) IPC"),
                    );
                    ui.separator();

                    ui.label(RichText::new("Allowed Ports & Rules").strong());
                    let mut to_remove_port = None;
                    for (idx, port) in current_profile.firewall.allow_ports.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", port.port));
                            if !port.comment.is_empty() {
                                ui.label(RichText::new(format!("({})", port.comment)).small());
                            }
                            if !is_builtin && ui.button("🗑").clicked() {
                                to_remove_port = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = to_remove_port {
                        current_profile.firewall.allow_ports.remove(idx);
                    }

                    if !is_builtin {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.new_port_str).desired_width(90.0));
                            ui.add(egui::TextEdit::singleline(&mut self.new_port_comment).hint_text("Comment").desired_width(140.0));
                            if ui.button("➕ Add Port").clicked() {
                                current_profile.firewall.allow_ports.push(config::PortRule {
                                    port: self.new_port_str.clone(),
                                    comment: self.new_port_comment.clone(),
                                });
                            }
                        });
                    }
                }

                ProfileSubTab::Kernel => {
                    ui.label(RichText::new("Kernel Sysctl Parameters").strong());
                    let mut remove_key = None;
                    for (k, v) in &current_profile.kernel {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(k).strong());
                            ui.label("=");
                            ui.label(v);
                            if !is_builtin && ui.button("🗑").clicked() {
                                remove_key = Some(k.clone());
                            }
                        });
                    }
                    if let Some(k) = remove_key {
                        current_profile.kernel.remove(&k);
                    }

                    if !is_builtin {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.new_sysctl_key).desired_width(180.0));
                            ui.label("=");
                            ui.add(egui::TextEdit::singleline(&mut self.new_sysctl_val).desired_width(80.0));
                            if ui.button("➕ Set").clicked() {
                                current_profile.kernel.insert(self.new_sysctl_key.clone(), self.new_sysctl_val.clone());
                            }
                        });
                    }
                }

                ProfileSubTab::Power => {
                    ui.label(RichText::new("Framework Battery & Power Policies").strong());
                    let mut limit = current_profile.power.battery_charge_limit.unwrap_or(100);
                    ui.horizontal(|ui| {
                        ui.label("Battery Charge Threshold (%):");
                        if ui.add_enabled(!is_builtin, egui::Slider::new(&mut limit, 50..=100)).changed() {
                            current_profile.power.battery_charge_limit = Some(limit);
                        }
                    });

                    let mut pprof = current_profile.power.power_profile.clone().unwrap_or_else(|| "balanced".to_string());
                    ui.horizontal(|ui| {
                        ui.label("Platform Power Profile:");
                        if ui.add_enabled(!is_builtin, egui::TextEdit::singleline(&mut pprof)).changed() {
                            current_profile.power.power_profile = Some(pprof);
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("CPU Energy Performance Preference (EPP)").strong());
                    let mut epp = current_profile.power.cpu_epp.clone().unwrap_or_else(|| "balance_performance".to_string());
                    ui.horizontal(|ui| {
                        ui.label("EPP Scaling Mode:");
                        egui::ComboBox::from_id_salt("epp_mode_combo")
                            .selected_text(&epp)
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut epp, "performance".to_string(), "🚀 performance (Max Throughput)").clicked() {
                                    current_profile.power.cpu_epp = Some(epp.clone());
                                }
                                if ui.selectable_value(&mut epp, "balance_performance".to_string(), "⚡ balance_performance (Responsive)").clicked() {
                                    current_profile.power.cpu_epp = Some(epp.clone());
                                }
                                if ui.selectable_value(&mut epp, "balance_power".to_string(), "🌱 balance_power (Cool & Quiet)").clicked() {
                                    current_profile.power.cpu_epp = Some(epp.clone());
                                }
                                if ui.selectable_value(&mut epp, "power".to_string(), "🔋 power (Max Battery Life)").clicked() {
                                    current_profile.power.cpu_epp = Some(epp.clone());
                                }
                            });
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("Hardware & Peripherals Security").strong());
                    let mut block_usb = current_profile.peripherals.block_new_usb.unwrap_or(false);
                    if ui.add_enabled(!is_builtin, egui::Checkbox::new(&mut block_usb, "🛡️ BadUSB Defense (Block unauthorized new USB devices at kernel level)")).changed() {
                        current_profile.peripherals.block_new_usb = Some(block_usb);
                    }

                    let mut bt = current_profile.peripherals.bluetooth.unwrap_or(true);
                    if ui.add_enabled(!is_builtin, egui::Checkbox::new(&mut bt, "📶 Bluetooth Radio Enabled")).changed() {
                        current_profile.peripherals.bluetooth = Some(bt);
                    }

                    let mut cam_b = current_profile.peripherals.camera_blocked.unwrap_or(false);
                    if ui.add_enabled(!is_builtin, egui::Checkbox::new(&mut cam_b, "📷 Block Camera Hardware (Webcam driver lockout)")).changed() {
                        current_profile.peripherals.camera_blocked = Some(cam_b);
                    }

                    let mut mic_m = current_profile.peripherals.microphone_muted.unwrap_or(false);
                    if ui.add_enabled(!is_builtin, egui::Checkbox::new(&mut mic_m, "🎙️ Mute Microphone Hardware (PipeWire/ALSA lock)")).changed() {
                        current_profile.peripherals.microphone_muted = Some(mic_m);
                    }

                    let mut loc_b = current_profile.peripherals.location_blocked.unwrap_or(false);
                    if ui.add_enabled(!is_builtin, egui::Checkbox::new(&mut loc_b, "📍 Isolate Location / Geolocation (Stop geoclue)")).changed() {
                        current_profile.peripherals.location_blocked = Some(loc_b);
                    }
                }

                ProfileSubTab::Hooks => {
                    ui.label(RichText::new("Developer Service Lifecycle & Command Hooks").strong());
                    ui.label(
                        RichText::new(
                            "Automatically suspend/resume Docker containers and start/stop background developer services when entering or leaving this profile.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    let mut manage_doc = current_profile.hooks.manage_docker.unwrap_or(false);
                    if ui.add_enabled(!is_builtin, egui::Checkbox::new(&mut manage_doc, "🐳 Manage Docker Containers (Auto-pause when leaving, unpause on enter)")).changed() {
                        current_profile.hooks.manage_docker = Some(manage_doc);
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("Lifecycle Shell Commands").strong());

                    let mut on_enter = current_profile.hooks.on_enter.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("On Enter Command:");
                        if ui.add_enabled(!is_builtin, egui::TextEdit::singleline(&mut on_enter).hint_text("e.g. systemctl --user start postgresql")).changed() {
                            current_profile.hooks.on_enter = if on_enter.trim().is_empty() { None } else { Some(on_enter) };
                        }
                    });

                    let mut on_exit = current_profile.hooks.on_exit.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("On Exit Command:");
                        if ui.add_enabled(!is_builtin, egui::TextEdit::singleline(&mut on_exit).hint_text("e.g. systemctl --user stop postgresql")).changed() {
                            current_profile.hooks.on_exit = if on_exit.trim().is_empty() { None } else { Some(on_exit) };
                        }
                    });
                }

                ProfileSubTab::Dns => {
                    ui.label(RichText::new("Profile-Aware Encrypted DNS & DNS-over-TLS (systemd-resolved)").strong());
                    ui.label(
                        RichText::new(
                            "Route DNS queries securely via systemd-resolved to prevent eavesdropping, ISP tracking, and DNS tampering. Strict DoT encrypts port 853 traffic with TLS.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    ui.add_space(8.0);
                    ui.label(RichText::new("⚡ Quick Privacy Presets").strong());
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(!is_builtin, egui::Button::new("🔒 Quad9 Strict (DoT + DNSSEC)")).clicked() {
                            current_profile.dns.servers = vec![
                                "9.9.9.9#dns.quad9.net".to_string(),
                                "149.112.112.112#dns.quad9.net".to_string(),
                            ];
                            current_profile.dns.fallback_servers = vec![
                                "1.1.1.1#cloudflare-dns.com".to_string(),
                            ];
                            current_profile.dns.dns_over_tls = Some("yes".to_string());
                            current_profile.dns.dnssec = Some("yes".to_string());
                            current_profile.dns.domains = vec!["~.".to_string()];
                        }

                        if ui.add_enabled(!is_builtin, egui::Button::new("⚡ Cloudflare 1.1.1.1 (Strict DoT)")).clicked() {
                            current_profile.dns.servers = vec![
                                "1.1.1.1#cloudflare-dns.com".to_string(),
                                "1.0.0.1#cloudflare-dns.com".to_string(),
                            ];
                            current_profile.dns.fallback_servers = vec![
                                "9.9.9.9#dns.quad9.net".to_string(),
                            ];
                            current_profile.dns.dns_over_tls = Some("yes".to_string());
                            current_profile.dns.dnssec = Some("yes".to_string());
                            current_profile.dns.domains = vec!["~.".to_string()];
                        }

                        if ui.add_enabled(!is_builtin, egui::Button::new("🛡️ AdGuard (Ad-Blocking DoT)")).clicked() {
                            current_profile.dns.servers = vec![
                                "94.140.14.14#dns.adguard-dns.com".to_string(),
                                "94.140.15.15#dns.adguard-dns.com".to_string(),
                            ];
                            current_profile.dns.fallback_servers = vec![
                                "9.9.9.9#dns.quad9.net".to_string(),
                            ];
                            current_profile.dns.dns_over_tls = Some("yes".to_string());
                            current_profile.dns.dnssec = Some("yes".to_string());
                            current_profile.dns.domains = vec!["~.".to_string()];
                        }

                        if ui.add_enabled(!is_builtin, egui::Button::new("🔄 System Default (DHCP)")).clicked() {
                            current_profile.dns.servers.clear();
                            current_profile.dns.fallback_servers.clear();
                            current_profile.dns.dns_over_tls = None;
                            current_profile.dns.dnssec = None;
                            current_profile.dns.domains.clear();
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();

                    ui.label(RichText::new("Encryption & Validation Policies").strong());
                    ui.horizontal(|ui| {
                        ui.label("DNS-over-TLS (DoT):");
                        let dot_display = match current_profile.dns.dns_over_tls.as_deref() {
                            Some("yes") => "Strict (yes) - Enforce TLS",
                            Some("opportunistic") => "Opportunistic - Encrypt if supported",
                            Some("no") => "Disabled (no) - Plaintext DNS",
                            _ => "Unmanaged / System Default",
                        };

                        egui::ComboBox::from_id_salt("profile_dot_combo")
                            .selected_text(dot_display)
                            .show_ui(ui, |ui| {
                                if !is_builtin {
                                    if ui.selectable_label(current_profile.dns.dns_over_tls.is_none(), "Unmanaged / System Default").clicked() {
                                        current_profile.dns.dns_over_tls = None;
                                    }
                                    if ui.selectable_label(current_profile.dns.dns_over_tls.as_deref() == Some("yes"), "Strict (yes) - Enforce TLS").clicked() {
                                        current_profile.dns.dns_over_tls = Some("yes".to_string());
                                    }
                                    if ui.selectable_label(current_profile.dns.dns_over_tls.as_deref() == Some("opportunistic"), "Opportunistic - Encrypt if supported").clicked() {
                                        current_profile.dns.dns_over_tls = Some("opportunistic".to_string());
                                    }
                                    if ui.selectable_label(current_profile.dns.dns_over_tls.as_deref() == Some("no"), "Disabled (no) - Plaintext DNS").clicked() {
                                        current_profile.dns.dns_over_tls = Some("no".to_string());
                                    }
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("DNSSEC Validation:");
                        let dnssec_display = match current_profile.dns.dnssec.as_deref() {
                            Some("yes") => "Strict (yes) - Cryptographic verification",
                            Some("allow-downgrade") => "Allow Downgrade - Verify if signed",
                            Some("no") => "Disabled (no)",
                            _ => "Unmanaged / System Default",
                        };

                        egui::ComboBox::from_id_salt("profile_dnssec_combo")
                            .selected_text(dnssec_display)
                            .show_ui(ui, |ui| {
                                if !is_builtin {
                                    if ui.selectable_label(current_profile.dns.dnssec.is_none(), "Unmanaged / System Default").clicked() {
                                        current_profile.dns.dnssec = None;
                                    }
                                    if ui.selectable_label(current_profile.dns.dnssec.as_deref() == Some("yes"), "Strict (yes) - Cryptographic verification").clicked() {
                                        current_profile.dns.dnssec = Some("yes".to_string());
                                    }
                                    if ui.selectable_label(current_profile.dns.dnssec.as_deref() == Some("allow-downgrade"), "Allow Downgrade - Verify if signed").clicked() {
                                        current_profile.dns.dnssec = Some("allow-downgrade".to_string());
                                    }
                                    if ui.selectable_label(current_profile.dns.dnssec.as_deref() == Some("no"), "Disabled (no)").clicked() {
                                        current_profile.dns.dnssec = Some("no".to_string());
                                    }
                                }
                            });
                    });

                    ui.add_space(8.0);
                    ui.separator();

                    ui.label(RichText::new("Primary Upstream DNS Resolvers").strong());
                    let mut remove_server = None;
                    for (idx, s) in current_profile.dns.servers.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {s}"));
                            if !is_builtin && ui.button("🗑").clicked() {
                                remove_server = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = remove_server {
                        current_profile.dns.servers.remove(idx);
                    }

                    if !is_builtin {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.new_dns_server).hint_text("e.g. 9.9.9.9#dns.quad9.net").desired_width(220.0));
                            if ui.button("➕ Add Server").clicked() {
                                let trimmed = self.new_dns_server.trim().to_string();
                                if !trimmed.is_empty() && !current_profile.dns.servers.contains(&trimmed) {
                                    current_profile.dns.servers.push(trimmed);
                                    self.new_dns_server.clear();
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.separator();

                    ui.label(RichText::new("Fallback DNS Resolvers").strong());
                    let mut remove_fallback = None;
                    for (idx, s) in current_profile.dns.fallback_servers.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {s}"));
                            if !is_builtin && ui.button("🗑").clicked() {
                                remove_fallback = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = remove_fallback {
                        current_profile.dns.fallback_servers.remove(idx);
                    }

                    if !is_builtin {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.new_dns_fallback).hint_text("e.g. 1.1.1.1#cloudflare-dns.com").desired_width(220.0));
                            if ui.button("➕ Add Fallback").clicked() {
                                let trimmed = self.new_dns_fallback.trim().to_string();
                                if !trimmed.is_empty() && !current_profile.dns.fallback_servers.contains(&trimmed) {
                                    current_profile.dns.fallback_servers.push(trimmed);
                                    self.new_dns_fallback.clear();
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.separator();

                    ui.label(RichText::new("Routing & Search Domains").strong());
                    ui.label(
                        RichText::new(
                            "Routing domain '~.' forces all DNS traffic through the configured servers. Internal domains (e.g. '~corp') route only specific queries.",
                        )
                        .small()
                        .color(Color32::from_rgb(160, 170, 195)),
                    );

                    let mut remove_domain = None;
                    for (idx, d) in current_profile.dns.domains.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {d}"));
                            if !is_builtin && ui.button("🗑").clicked() {
                                remove_domain = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = remove_domain {
                        current_profile.dns.domains.remove(idx);
                    }

                    if !is_builtin {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.new_dns_domain).hint_text("e.g. ~. or corp.internal").desired_width(220.0));
                            if ui.button("➕ Add Domain").clicked() {
                                let trimmed = self.new_dns_domain.trim().to_string();
                                if !trimmed.is_empty() && !current_profile.dns.domains.contains(&trimmed) {
                                    current_profile.dns.domains.push(trimmed);
                                    self.new_dns_domain.clear();
                                }
                            }
                        });
                    }
                }
            }

            if !is_builtin {
                ui.add_space(16.0);
                if ui.button(RichText::new("💾 Save Profile Changes").color(Color32::BLACK).strong()).clicked() {
                    let _ = config::save_custom_profile(current_profile.clone(), system::is_privileged());
                    self.status_message = Some(("Profile saved successfully.".to_string(), false));
                }
            }
        });
    }
}

fn get_single_instance_socket_path() -> std::path::PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("postureflow-gui.sock")
    } else {
        std::env::temp_dir().join(format!("postureflow-gui-{}.sock", unsafe { libc::getuid() }))
    }
}

struct SocketCleaner(std::path::PathBuf);
impl Drop for SocketCleaner {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn main() -> Result<(), eframe::Error> {
    let socket_path = get_single_instance_socket_path();
    let args: Vec<String> = std::env::args().collect();
    let tab_arg = if args.iter().any(|a| a == "ports" || a == "inspector") {
        "tab:ports"
    } else if args.iter().any(|a| a == "autoflow") {
        "tab:autoflow"
    } else if args.iter().any(|a| a == "triggers") {
        "tab:triggers"
    } else if args.iter().any(|a| a == "schedule") {
        "tab:schedule"
    } else if args.iter().any(|a| a == "profiles") {
        "tab:profiles"
    } else {
        "focus"
    };

    // 1. If another instance is already running, send focus command and exit immediately
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&socket_path) {
        use std::io::Write;
        let _ = writeln!(stream, "{}", tab_arg);
        let _ = stream.flush();
        println!("[*] PostureFlow GUI is already running. Brought existing window to focus.");
        return Ok(());
    }

    // 2. Clean up any stale socket from a previous unexpected crash
    let _ = std::fs::remove_file(&socket_path);

    // 3. Bind the single-instance listener and register automatic cleanup on exit
    let listener = std::os::unix::net::UnixListener::bind(&socket_path).ok();
    let _cleaner = SocketCleaner(socket_path.clone());

    let (ipc_tx, ipc_rx) = std::sync::mpsc::channel::<String>();
    let (ctx_tx, ctx_rx) = std::sync::mpsc::channel::<egui::Context>();

    if let Some(listener) = listener {
        std::thread::spawn(move || {
            let egui_ctx = ctx_rx.recv().ok();
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    use std::io::{BufRead, BufReader};
                    let mut reader = BufReader::new(&mut stream);
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_ok() {
                        let _ = ipc_tx.send(line.trim().to_string());
                        if let Some(ref ctx) = egui_ctx {
                            ctx.request_repaint();
                        }
                    }
                }
            }
        });
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PostureFlow - Security Cockpit & Profile Orchestrator")
            .with_app_id("io.github.mzia.PostureFlow")
            .with_inner_size([980.0, 680.0])
            .with_min_inner_size([640.0, 420.0])
            .with_resizable(true),

        ..Default::default()
    };

    eframe::run_native(
        "PostureFlow",
        native_options,
        Box::new(move |cc| {
            let _ = ctx_tx.send(cc.egui_ctx.clone());

            // Apply refined Pop!_OS / COSMIC dark aesthetics
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(24, 26, 34);
            visuals.window_fill = Color32::from_rgb(30, 33, 44);
            visuals.selection.bg_fill = Color32::from_rgb(72, 185, 199);
            visuals.window_corner_radius = CornerRadius::same(10);
            visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 33, 44);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 42, 56);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 54, 72);
            visuals.widgets.active.bg_fill = Color32::from_rgb(72, 185, 199);
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(GuiApp::new(cc, ipc_rx)))
        }),
    )
}
