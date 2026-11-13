use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use agent_client_protocol::v1::schema::{
    AvailableCommand, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionConfigKind,
    SessionConfigSelectOptions, ToolKind,
};
use egui::{Align, Color32, Key, Layout, RichText};
use egui_commonmark::CommonMarkCache;
use egui_file_dialog::FileDialog;
use tokio::sync::oneshot;

use crate::acp::{self, AcpCommand, AcpEvent, AcpHandle, AgentConfig};
use crate::catalog::{self, SessionSummary};
use crate::model::SessionView;
use crate::settings::{PermissionPolicy, Settings, ThemePreference};
use crate::theme;
use crate::ui;

pub enum ConnectionState {
    Connecting,
    Connected {
        name: String,
        version: String,
        protocol: String,
    },
    Disconnected {
        reason: String,
    },
}

struct PermissionPrompt {
    request: RequestPermissionRequest,
    reply: oneshot::Sender<RequestPermissionResponse>,
}

pub struct App {
    pub acp: AcpHandle,
    pub settings: Settings,
    args_text: String,
    default_cwd_text: String,
    connection: ConnectionState,
    sessions: Vec<SessionSummary>,
    session_filter: String,
    active: Option<SessionView>,
    composer: String,
    cwd: PathBuf,
    permissions: VecDeque<PermissionPrompt>,
    markdown_cache: CommonMarkCache,
    show_settings: bool,
    show_sessions_modal: bool,
    show_command_palette: bool,
    command_filter: String,
    error_banner: Option<String>,
    file_dialog: FileDialog,
    applied_theme: ThemePreference,
    applied_font_scale: f32,
    focus_composer: bool,
    pending_prompt: Option<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        theme::apply(&cc.egui_ctx);
        cc.egui_ctx.set_theme(egui_theme(settings.theme));
        cc.egui_ctx
            .set_zoom_factor(settings.font_scale.clamp(0.75, 1.5));
        install_cjk_font(&cc.egui_ctx);

        let cwd = settings
            .default_cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let acp = acp::spawn(
            AgentConfig {
                binary: settings.agent_binary.clone(),
                args: settings.agent_args.clone(),
            },
            cc.egui_ctx.clone(),
        );
        let file_dialog = FileDialog::new()
            .title("Select project directory")
            .initial_directory(cwd.clone());

        Self {
            acp,
            args_text: settings.agent_args.join(" "),
            default_cwd_text: settings
                .default_cwd
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            applied_theme: settings.theme,
            applied_font_scale: settings.font_scale,
            settings,
            connection: ConnectionState::Connecting,
            sessions: catalog::scan_sessions(),
            session_filter: String::new(),
            active: None,
            composer: String::new(),
            cwd,
            permissions: VecDeque::new(),
            markdown_cache: CommonMarkCache::default(),
            show_settings: false,
            show_sessions_modal: false,
            show_command_palette: false,
            command_filter: String::new(),
            error_banner: None,
            file_dialog,
            focus_composer: false,
            pending_prompt: None,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.acp.rx.try_recv() {
            match event {
                AcpEvent::Connected {
                    agent_name,
                    agent_version,
                    protocol_version,
                    auth_methods,
                } => {
                    tracing::info!(
                        "connected to {agent_name} {agent_version} ({protocol_version:?}), auth: {auth_methods:?}"
                    );
                    self.connection = ConnectionState::Connected {
                        name: agent_name,
                        version: agent_version,
                        protocol: protocol_version,
                    };
                    self.error_banner = None;
                }
                AcpEvent::SessionLoading { session_id, cwd } => {
                    let title = self
                        .active
                        .as_ref()
                        .filter(|session| session.id == session_id)
                        .and_then(|session| session.title.clone());
                    let mut session = SessionView::new(session_id, cwd.clone());
                    session.title = title;
                    session.loading_history = true;
                    self.active = Some(session);
                    self.cwd = cwd;
                }
                AcpEvent::SessionReady {
                    session_id,
                    cwd,
                    modes,
                    config_options,
                    loaded,
                } => {
                    let existing_title = self
                        .active
                        .as_ref()
                        .filter(|session| session.id == session_id)
                        .and_then(|session| session.title.clone());
                    let need_new = !matches!(
                        &self.active,
                        Some(session) if session.id == session_id
                    );
                    if need_new {
                        let mut session = SessionView::new(session_id.clone(), cwd.clone());
                        session.title = existing_title.clone();
                        self.active = Some(session);
                    }
                    if let Some(session) = &mut self.active {
                        if session.title.is_none() {
                            session.title = existing_title;
                        }
                        session.cwd = cwd.clone();
                        session.modes = modes;
                        session.config_options = config_options;
                        session.loading_history = false;
                        if !loaded {
                            session.is_prompting = false;
                        }
                    }
                    self.cwd = cwd;
                    self.sessions = catalog::scan_sessions();
                    self.focus_composer = true;
                    tracing::info!("session ready: {session_id} (loaded={loaded})");
                    if let Some(prompt) = self.pending_prompt.take() {
                        self.send_prompt(prompt);
                    }
                }
                AcpEvent::Update { session_id, update } => {
                    let title_update = matches!(
                        &update,
                        agent_client_protocol::v1::schema::SessionUpdate::SessionInfoUpdate(_)
                    );
                    if let Some(session) = &mut self.active
                        && session.id == session_id
                    {
                        session.apply_update(update);
                        if title_update
                            && let Some(title) = session.title.clone()
                            && let Some(summary) = self
                                .sessions
                                .iter_mut()
                                .find(|summary| summary.id == session_id)
                        {
                            summary.title = title;
                        }
                    }
                }
                AcpEvent::PromptFinished {
                    session_id,
                    stop_reason,
                } => {
                    if let Some(session) = &mut self.active
                        && session.id == session_id
                    {
                        session.is_prompting = false;
                        session.stop_reason = Some(stop_reason);
                    }
                    self.sessions = catalog::scan_sessions();
                    tracing::info!("turn finished: {stop_reason:?}");
                }
                AcpEvent::PermissionRequest { request, reply } => {
                    if let Some(option_id) = self.auto_permission(&request) {
                        let _ = reply.send(RequestPermissionResponse::new(
                            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                                option_id,
                            )),
                        ));
                    } else {
                        self.permissions
                            .push_back(PermissionPrompt { request, reply });
                    }
                }
                AcpEvent::ConfigOptionsChanged {
                    session_id,
                    config_options,
                } => {
                    if let Some(session) = &mut self.active
                        && session.id == session_id
                    {
                        session.config_options = config_options;
                    }
                }
                AcpEvent::Error { message } => {
                    tracing::error!("{message}");
                    if let Some(session) = &mut self.active {
                        session.is_prompting = false;
                    }
                    self.error_banner = Some(message);
                }
                AcpEvent::Closed { reason } => {
                    self.connection = ConnectionState::Disconnected {
                        reason: reason.clone(),
                    };
                    if let Some(session) = &mut self.active {
                        session.is_prompting = false;
                    }
                }
            }
        }
    }

    fn auto_permission(&self, request: &RequestPermissionRequest) -> Option<String> {
        let allow = |kind: PermissionOptionKind| {
            request
                .options
                .iter()
                .find(|option| option.kind == kind)
                .map(|option| option.option_id.to_string())
        };

        match self.settings.permission_policy {
            PermissionPolicy::Ask => None,
            PermissionPolicy::Yolo => allow(PermissionOptionKind::AllowOnce)
                .or_else(|| allow(PermissionOptionKind::AllowAlways)),
            PermissionPolicy::AutoAllowReads => {
                let kind = request.tool_call.fields.kind;
                if matches!(
                    kind,
                    Some(ToolKind::Read | ToolKind::Search | ToolKind::Fetch)
                ) {
                    allow(PermissionOptionKind::AllowOnce)
                        .or_else(|| allow(PermissionOptionKind::AllowAlways))
                } else {
                    None
                }
            }
        }
    }

    fn respond_permission(&mut self, option_id: Option<String>) {
        let Some(prompt) = self.permissions.pop_front() else {
            return;
        };
        let response = match option_id {
            Some(option_id) => RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
                SelectedPermissionOutcome::new(option_id),
            )),
            None => RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled),
        };
        let _ = prompt.reply.send(response);
    }

    fn start_new_session(&mut self) {
        if !matches!(self.connection, ConnectionState::Connected { .. }) {
            return;
        }
        let _ = self.acp.tx.send(AcpCommand::NewSession {
            cwd: self.cwd.clone(),
        });
        self.active = None;
        self.composer.clear();
        self.error_banner = None;
    }

    fn new_session_explicit(&mut self) {
        self.pending_prompt = None;
        self.start_new_session();
    }

    fn send_prompt(&mut self, text: String) {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(session) = &mut self.active else {
            return;
        };
        if session.loading_history || session.is_prompting {
            return;
        }
        session.push_user(text.clone());
        session.is_prompting = true;
        session.stop_reason = None;
        let _ = self.acp.tx.send(AcpCommand::Prompt {
            session_id: session.id.clone(),
            text,
        });
    }

    fn start_with_prompt(&mut self, prompt: String) {
        let has_ready_session = self
            .active
            .as_ref()
            .is_some_and(|session| !session.loading_history && !session.is_prompting);
        if has_ready_session {
            self.send_prompt(prompt);
        } else {
            self.pending_prompt = Some(prompt);
            self.start_new_session();
        }
    }

    fn load_session(&mut self, summary: SessionSummary) {
        let mut session = SessionView::new(summary.id.clone(), summary.cwd.clone());
        session.title = Some(summary.title.clone());
        session.loading_history = true;
        self.active = Some(session);
        self.cwd = summary.cwd.clone();
        let _ = self.acp.tx.send(AcpCommand::LoadSession {
            session_id: summary.id,
            cwd: summary.cwd,
        });
    }

    fn cancel_active(&mut self) {
        if let Some(session) = &self.active {
            let _ = self.acp.tx.send(AcpCommand::Cancel {
                session_id: session.id.clone(),
            });
        }
    }

    fn reconnect(&mut self, ctx: egui::Context) {
        self.acp.shutdown();
        self.acp = acp::spawn(
            AgentConfig {
                binary: self.settings.agent_binary.clone(),
                args: self.settings.agent_args.clone(),
            },
            ctx,
        );
        self.connection = ConnectionState::Connecting;
        self.active = None;
        self.permissions.clear();
        self.error_banner = None;
    }

    fn apply_appearance(&mut self, ctx: &egui::Context) {
        if self.applied_theme != self.settings.theme {
            theme::apply(ctx);
            ctx.set_theme(egui_theme(self.settings.theme));
            self.applied_theme = self.settings.theme;
        }
        if (self.applied_font_scale - self.settings.font_scale).abs() > f32::EPSILON {
            let scale = self.settings.font_scale.clamp(0.75, 1.5);
            ctx.set_zoom_factor(scale);
            self.applied_font_scale = scale;
        }
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui, compact: bool) {
        let mut commands = Vec::new();
        let mut new_session = false;
        let mut reconnect = false;
        let mut open_settings = false;
        let mut refresh = false;
        let connected = matches!(self.connection, ConnectionState::Connected { .. });

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            theme::brand(ui);
            if !compact {
                ui.add_space(2.0);
                ui.label(RichText::new("Command Code").strong().size(15.0));
                ui.add_space(4.0);
            }
            self.draw_connection_pill(ui);
            if !compact {
                ui.add_space(2.0);

                if ui
                    .add(
                        egui::Button::new(RichText::new(project_name(&self.cwd)).small())
                            .frame(false),
                    )
                    .on_hover_text(self.cwd.display().to_string())
                    .clicked()
                {
                    self.file_dialog.pick_directory();
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button(RichText::new("⋯").size(18.0), |ui| {
                    if ui.button("Open project…").clicked() {
                        self.file_dialog.pick_directory();
                        ui.close();
                    }
                    if ui.button("Settings").clicked() {
                        open_settings = true;
                        ui.close();
                    }
                    if ui.button("Reconnect agent").clicked() {
                        reconnect = true;
                        ui.close();
                    }
                    if ui.button("Refresh sessions").clicked() {
                        refresh = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(format!("agent: {}", self.settings.agent_binary))
                            .small()
                            .weak(),
                    );
                });

                if compact && ui.button("Sessions").clicked() {
                    self.show_sessions_modal = true;
                }

                if ui
                    .add_enabled(
                        connected,
                        theme::accent_button(
                            RichText::new("+ New")
                                .strong()
                                .color(Color32::from_rgb(24, 20, 12)),
                        ),
                    )
                    .clicked()
                {
                    new_session = true;
                }
            });
        });

        if let Some(session) = &self.active {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                let title = session
                    .title
                    .clone()
                    .unwrap_or_else(|| "New session".to_string());
                ui.label(RichText::new(title).strong().size(16.0));
                ui.label(
                    RichText::new(short_id(&session.id))
                        .small()
                        .color(theme::muted(ui)),
                );
                ui.add_space(6.0);
                Self::draw_session_controls(ui, session, &mut commands);
            });
        }

        if let Some(error) = self.error_banner.clone() {
            ui.add_space(4.0);
            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(230, 119, 119, 24))
                .stroke(egui::Stroke::new(1.0, theme::DANGER.gamma_multiply(0.55)))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠").color(theme::DANGER));
                        ui.add(
                            egui::Label::new(RichText::new(error.as_str()).color(theme::DANGER))
                                .wrap(),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                self.error_banner = None;
                            }
                        });
                    });
                });
        }

        for command in commands {
            let _ = self.acp.tx.send(command);
        }
        if new_session {
            self.new_session_explicit();
        }
        if refresh {
            self.sessions = catalog::scan_sessions();
        }
        if open_settings {
            self.open_settings();
        }
        if reconnect {
            self.reconnect(ui.ctx().clone());
        }
    }

    fn open_settings(&mut self) {
        self.args_text = self.settings.agent_args.join(" ");
        self.default_cwd_text = self
            .settings
            .default_cwd
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        self.show_settings = true;
    }

    fn draw_connection_pill(&self, ui: &mut egui::Ui) {
        let (label, color, tooltip) = match &self.connection {
            ConnectionState::Connected {
                name,
                version,
                protocol,
            } => (
                "online".to_string(),
                theme::SUCCESS,
                format!("{name} {version} · {protocol}"),
            ),
            ConnectionState::Connecting => (
                "connecting".to_string(),
                theme::WARNING,
                "Connecting to commandcode acp".to_string(),
            ),
            ConnectionState::Disconnected { reason } => {
                ("offline".to_string(), theme::DANGER, reason.clone())
            }
        };

        egui::Frame::NONE
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(20)
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").size(9.0).color(color));
                    ui.label(RichText::new(label).small().color(color));
                });
            })
            .response
            .on_hover_text(tooltip);
    }

    fn draw_session_controls(
        ui: &mut egui::Ui,
        session: &SessionView,
        commands: &mut Vec<AcpCommand>,
    ) {
        ui.spacing_mut().item_spacing.x = 6.0;
        let session_id = session.id.clone();
        let mut config_changes: Vec<(String, String)> = Vec::new();
        let mut mode_change: Option<String> = None;

        for option in &session.config_options {
            match &option.kind {
                SessionConfigKind::Select(select) => {
                    let values = select_values(&select.options);
                    let current = select.current_value.to_string();
                    let current_label = values
                        .iter()
                        .find(|(value, _)| value == &current)
                        .map(|(_, label)| label.clone())
                        .unwrap_or_else(|| current.clone());
                    let width = if option.name.to_lowercase().contains("model") {
                        190.0
                    } else {
                        120.0
                    };
                    egui::ComboBox::from_id_salt(("config", option.id.to_string()))
                        .selected_text(format!("{}: {}", option.name, current_label))
                        .width(width)
                        .show_ui(ui, |ui| {
                            for (value, label) in &values {
                                if ui
                                    .selectable_label(&current == value, label.as_str())
                                    .clicked()
                                    && &current != value
                                {
                                    config_changes.push((option.id.to_string(), value.clone()));
                                }
                            }
                        });
                }
                SessionConfigKind::Boolean(boolean) => {
                    let mut value = boolean.current_value;
                    if ui.checkbox(&mut value, &option.name).changed() {
                        config_changes.push((
                            option.id.to_string(),
                            if value {
                                "true".to_string()
                            } else {
                                "false".to_string()
                            },
                        ));
                    }
                }
                _ => {}
            }
        }

        if let Some(modes) = &session.modes {
            let current = modes.current_mode_id.to_string();
            let current_name = modes
                .available_modes
                .iter()
                .find(|mode| mode.id.to_string() == current)
                .map(|mode| mode.name.clone())
                .unwrap_or_else(|| current.clone());
            egui::ComboBox::from_id_salt("session_mode")
                .selected_text(format!("Mode: {current_name}"))
                .width(130.0)
                .show_ui(ui, |ui| {
                    for mode in &modes.available_modes {
                        let id = mode.id.to_string();
                        if ui
                            .selectable_label(id == current, mode.name.as_str())
                            .clicked()
                            && id != current
                        {
                            mode_change = Some(id);
                        }
                    }
                });
        }

        for (config_id, value) in config_changes {
            commands.push(AcpCommand::SetConfigOption {
                session_id: session_id.clone(),
                config_id,
                value,
            });
        }
        if let Some(mode_id) = mode_change {
            commands.push(AcpCommand::SetMode {
                session_id,
                mode_id,
            });
        }
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        let action = ui::sidebar::draw(
            ui,
            &self.sessions,
            &mut self.session_filter,
            self.active.as_ref().map(|session| session.id.as_str()),
            &self.cwd,
        );
        match action {
            ui::sidebar::SidebarAction::None => {}
            ui::sidebar::SidebarAction::NewSession => self.new_session_explicit(),
            ui::sidebar::SidebarAction::Refresh => {
                self.sessions = catalog::scan_sessions();
            }
            ui::sidebar::SidebarAction::Load(summary) => self.load_session(summary),
        }
    }

    fn draw_chat(&mut self, ui: &mut egui::Ui) {
        let show_thoughts = self.settings.show_thoughts;
        let cache = &mut self.markdown_cache;
        let suggestion = if let Some(session) = &mut self.active {
            ui::chat::draw(ui, session, cache, show_thoughts)
        } else {
            ui::chat::welcome(ui, &self.cwd)
        };

        if let Some(prompt) = suggestion {
            self.start_with_prompt(prompt);
        }
    }

    fn draw_composer(&mut self, ui: &mut egui::Ui) {
        let has_session = self
            .active
            .as_ref()
            .is_some_and(|session| !session.loading_history);
        let is_prompting = self
            .active
            .as_ref()
            .is_some_and(|session| session.is_prompting);
        let stop_reason = self
            .active
            .as_ref()
            .and_then(|session| session.stop_reason)
            .map(|reason| format!("{reason:?}"));

        let action = ui::composer::draw(
            ui,
            &mut self.composer,
            is_prompting,
            has_session,
            stop_reason.as_deref(),
            self.focus_composer,
        );
        self.focus_composer = false;

        match action {
            ui::composer::ComposerAction::None => {}
            ui::composer::ComposerAction::Cancel => self.cancel_active(),
            ui::composer::ComposerAction::Send => {
                let text = std::mem::take(&mut self.composer);
                self.send_prompt(text);
            }
        }
    }

    fn draw_sessions_modal(&mut self, ctx: &egui::Context) {
        if !self.show_sessions_modal {
            return;
        }

        let mut action = ui::sidebar::SidebarAction::None;
        let response = egui::Modal::new(egui::Id::unique("sessions_modal")).show(ctx, |ui| {
            ui.set_min_width((ctx.content_rect().width() - 72.0).clamp(280.0, 340.0));
            ui.set_max_height(560.0);
            action = ui::sidebar::draw(
                ui,
                &self.sessions,
                &mut self.session_filter,
                self.active.as_ref().map(|session| session.id.as_str()),
                &self.cwd,
            );
        });

        if response.should_close() {
            self.show_sessions_modal = false;
        }

        match action {
            ui::sidebar::SidebarAction::None => {}
            ui::sidebar::SidebarAction::NewSession => {
                self.show_sessions_modal = false;
                self.new_session_explicit();
            }
            ui::sidebar::SidebarAction::Refresh => {
                self.sessions = catalog::scan_sessions();
            }
            ui::sidebar::SidebarAction::Load(summary) => {
                self.show_sessions_modal = false;
                self.load_session(summary);
            }
        }
    }

    fn draw_details_modal(&mut self, ctx: &egui::Context) {
        let Some(session) = &mut self.active else {
            return;
        };
        if session.selected_tool.is_none() {
            return;
        }

        let response = egui::Modal::new(egui::Id::unique("details_modal")).show(ctx, |ui| {
            ui.set_min_width((ctx.content_rect().width() - 72.0).clamp(300.0, 520.0));
            ui.set_max_height(620.0);
            ui::details::draw(ui, session);
        });

        if response.should_close()
            && let Some(session) = &mut self.active
        {
            session.selected_tool = None;
        }
    }

    fn draw_permission_modal(&mut self, ctx: &egui::Context) {
        let action = {
            let Some(prompt) = self.permissions.front() else {
                return;
            };
            ui::permission::draw(ctx, &prompt.request)
        };
        match action {
            Some(ui::permission::PermissionAction::Select(option_id)) => {
                self.respond_permission(Some(option_id));
            }
            Some(ui::permission::PermissionAction::Cancel) => {
                self.respond_permission(None);
            }
            None => {}
        }
    }

    fn draw_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let action = ui::settings::draw(
            ctx,
            &mut self.settings,
            &mut self.args_text,
            &mut self.default_cwd_text,
        );
        match action {
            ui::settings::SettingsAction::None => {}
            ui::settings::SettingsAction::Cancel => {
                self.args_text = self.settings.agent_args.join(" ");
                self.default_cwd_text = self
                    .settings
                    .default_cwd
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
                self.show_settings = false;
            }
            ui::settings::SettingsAction::Save => {
                self.settings.agent_args = self
                    .args_text
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                let cwd = self.default_cwd_text.trim();
                self.settings.default_cwd = if cwd.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(cwd))
                };
                self.settings.save();
                self.apply_appearance(ctx);
                self.show_settings = false;
            }
            ui::settings::SettingsAction::Reconnect => {
                self.show_settings = false;
                self.reconnect(ctx.clone());
            }
        }
    }

    fn draw_palette_modal(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }
        let commands: Vec<AvailableCommand> = self
            .active
            .as_ref()
            .map(|session| session.available_commands.clone())
            .unwrap_or_default();
        let action = ui::palette::draw(ctx, &mut self.command_filter, &commands);
        match action {
            Some(ui::palette::PaletteAction::Insert(text)) => {
                self.composer = text;
                self.show_command_palette = false;
            }
            Some(ui::palette::PaletteAction::Close) => {
                self.show_command_palette = false;
            }
            None => {}
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_appearance(ctx);
        self.drain_events();

        if let Some(rect) = ctx.input(|input| input.viewport().inner_rect) {
            self.settings.window_size = [rect.width(), rect.height()];
        }

        let (new_session, palette, settings, escape) = ctx.input(|input| {
            (
                input.modifiers.ctrl && input.key_pressed(Key::N),
                input.modifiers.ctrl && input.key_pressed(Key::K),
                input.modifiers.ctrl && input.key_pressed(Key::Comma),
                input.key_pressed(Key::Escape),
            )
        });

        if new_session && !self.show_settings && !self.show_command_palette {
            self.new_session_explicit();
        }
        if palette && self.active.is_some() {
            self.show_command_palette = true;
        }
        if settings {
            self.open_settings();
        }
        if escape {
            if self.show_settings || self.show_command_palette || self.show_sessions_modal {
                self.show_settings = false;
                self.show_command_palette = false;
                self.show_sessions_modal = false;
            } else if self.permissions.is_empty()
                && self
                    .active
                    .as_ref()
                    .is_some_and(|session| session.is_prompting)
            {
                self.cancel_active();
            }
        }

        if self
            .active
            .as_ref()
            .is_some_and(|session| session.is_prompting)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let width = ui.available_width();
        let compact = width < 620.0;
        let wide = width >= 1080.0;
        let has_details = self
            .active
            .as_ref()
            .and_then(|session| session.selected_tool)
            .is_some();
        let side_margin: i8 = if compact { 12 } else { 20 };

        egui::Panel::top("top_bar")
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 14,
                        top: 8,
                        bottom: 8,
                    }),
            )
            .show(ui, |ui| self.draw_top_bar(ui, compact));

        if !compact {
            egui::Panel::left("sidebar")
                .frame(
                    egui::Frame::NONE
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin {
                            left: 12,
                            right: 12,
                            top: 4,
                            bottom: 10,
                        }),
                )
                .default_size(240.0)
                .min_size(190.0)
                .max_size(320.0)
                .resizable(true)
                .show(ui, |ui| self.draw_sidebar(ui));
        }

        if wide && has_details {
            egui::Panel::right("details")
                .frame(
                    egui::Frame::NONE
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin::same(12)),
                )
                .default_size(360.0)
                .min_size(280.0)
                .max_size(520.0)
                .resizable(true)
                .show(ui, |ui| {
                    if let Some(session) = &mut self.active {
                        ui::details::draw(ui, session);
                    }
                });
        }

        egui::Panel::bottom("composer")
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: 6,
                        bottom: 12,
                    }),
            )
            .default_size(160.0)
            .min_size(120.0)
            .show(ui, |ui| self.draw_composer(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: side_margin,
                right: side_margin,
                top: 4,
                bottom: 4,
            }))
            .show(ui, |ui| self.draw_chat(ui));

        self.draw_sessions_modal(&ctx);
        if !wide {
            self.draw_details_modal(&ctx);
        }
        self.draw_permission_modal(&ctx);
        self.draw_settings_modal(&ctx);
        self.draw_palette_modal(&ctx);

        self.file_dialog.update(&ctx);
        if let Some(path) = self.file_dialog.take_picked() {
            self.cwd = path;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.settings.save();
    }
}

fn egui_theme(theme: ThemePreference) -> egui::ThemePreference {
    match theme {
        ThemePreference::System => egui::ThemePreference::System,
        ThemePreference::Dark => egui::ThemePreference::Dark,
        ThemePreference::Light => egui::ThemePreference::Light,
    }
}

fn project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_string()
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn install_cjk_font(ctx: &egui::Context) {
    let Some(path) = cjk_font_path() else {
        tracing::warn!("no CJK font found; set COMMANDCODE_GUI_CJK_FONT to a .ttf/.otf/.ttc path");
        return;
    };
    match std::fs::read(&path) {
        Ok(bytes) => {
            ctx.add_font(egui::FontInsert::new(
                "cjk",
                egui::FontData::from_owned(bytes),
                vec![
                    egui::InsertFontFamily {
                        family: egui::FontFamily::Proportional,
                        priority: egui::FontPriority::Lowest,
                    },
                    egui::InsertFontFamily {
                        family: egui::FontFamily::Monospace,
                        priority: egui::FontPriority::Lowest,
                    },
                ],
            ));
            tracing::info!("loaded CJK font: {}", path.display());
        }
        Err(error) => {
            tracing::warn!("failed to read CJK font {}: {error}", path.display());
        }
    }
}

fn cjk_font_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("COMMANDCODE_GUI_CJK_FONT") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let output = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "Noto Sans CJK JP"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    path.is_file().then_some(path)
}

fn select_values(options: &SessionConfigSelectOptions) -> Vec<(String, String)> {
    match options {
        SessionConfigSelectOptions::Ungrouped(options) => options
            .iter()
            .map(|option| (option.value.to_string(), option.name.clone()))
            .collect(),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| {
                group
                    .options
                    .iter()
                    .map(|option| (option.value.to_string(), option.name.clone()))
            })
            .collect(),
        _ => Vec::new(),
    }
}
