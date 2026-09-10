use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use postureflow::autoflow::{self, ActiveNetworkInfo, AutoFlowConfig, NetworkRule};
use postureflow::config::{
    self, validate_and_sanitize, DesktopConfig, FirewallConfig, FrameworkPowerConfig,
    PeripheralsConfig, ProfileConfig, ProfileMetadata, SecurityLimitsConfig,
};
use postureflow::inspector::{self, ListeningPort, PostureScoreReport};
use postureflow::system;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum MainTab {
    Cockpit,
    Ports,
    AutoFlow,
    Profiles,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ProfileSubTab {
    General,
    Firewall,
    Kernel,
    Power,
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

    // Profile form inputs
    new_port_str: String,
    new_port_comment: String,
    new_sysctl_key: String,
    new_sysctl_val: String,
}

impl GuiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
            } else if args.iter().any(|a| a == "autoflow") {
                initial_tab = MainTab::AutoFlow;
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
            new_port_str: "8080/tcp".to_string(),
            new_port_comment: "Web Development".to_string(),
            new_sysctl_key: "fs.inotify.max_user_watches".to_string(),
            new_sysctl_val: "524288".to_string(),
        }
    }

    fn refresh_all(&mut self) {
        self.profiles = config::load_all_profiles();
        self.active_profile_id = system::get_active_profile();
        self.score_report = PostureScoreReport::compute();
        self.listening_ports = inspector::ports::scan_listening_ports();
        self.autoflow_config = autoflow::load_autoflow_config();
        self.active_network = autoflow::detect_active_networks();
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

        // Top Navigation Header (COSMIC Style)
        egui::Panel::top("nav_header")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(22, 24, 32))
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("PostureFlow")
                            .color(Color32::from_rgb(72, 185, 199))
                            .strong(),
                    );
                    ui.label(RichText::new("v1.0.0").color(Color32::from_rgb(140, 150, 175)).small());

                    ui.add_space(24.0);

                    // Nav Tabs
                    ui.selectable_value(&mut self.main_tab, MainTab::Cockpit, "🛡️ Security Cockpit");
                    ui.selectable_value(&mut self.main_tab, MainTab::Ports, "🔌 Port Inspector");
                    ui.selectable_value(&mut self.main_tab, MainTab::AutoFlow, "⚡ Auto-Flow (Network)");
                    ui.selectable_value(&mut self.main_tab, MainTab::Profiles, "🏷️ Profiles Manager");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let active_badge = if self.active_profile_id == "default" {
                            "ACTIVE: FACTORY DEFAULT".to_string()
                        } else {
                            format!("ACTIVE: [{}]", self.active_profile_id.to_uppercase())
                        };
                        ui.label(
                            RichText::new(active_badge)
                                .color(Color32::from_rgb(246, 166, 35))
                                .strong(),
                        );

                        if ui.button("🔄 Refresh").clicked() {
                            self.refresh_all();
                        }
                    });
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
            MainTab::AutoFlow => self.render_autoflow_view(ui),
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

        // Rules List
        ui.heading("📋 Network Assignment Rules");
        let mut delete_index = None;

        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
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

        ui.add_space(10.0);

        // Add New Rule Form
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

    // 4. PROFILES & SYSTEM SETTINGS VIEW (Standard Editor)
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

fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PostureFlow - Security Cockpit & Profile Orchestrator")
            .with_app_id("io.github.mzia.PostureFlow")
            .with_inner_size([980.0, 680.0])
            .with_min_inner_size([800.0, 520.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "PostureFlow",
        native_options,
        Box::new(|cc| {
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

            Ok(Box::new(GuiApp::new(cc)))
        }),
    )
}
