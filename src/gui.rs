use anyhow::Context;
use gtk::gdk;
use gtk::prelude::*;
use rand::prelude::IndexedRandom;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};
use water_reminder::scheduler::{self, ReminderState, Schedule};
use water_reminder::settings::{self, Settings};
use water_reminder::{sound, stats};

const SNOOZE_OPTIONS: [u32; 3] = [5, 10, 15];
const REMINDER_MESSAGES: [&str; 8] = [
    "Bebe ya, tonto, que te vas a quedar seco.",
    "Agua, cabeza de melon. Tu cuerpo no funciona con cafe y fe.",
    "Otro vaso, campeon del desierto domestico.",
    "Bebe agua, que pareces una planta de oficina abandonada.",
    "Hidratate, desastre. Son 200 ml, no una tesis.",
    "Levanta el vaso, genio. La sequia no es una estrategia.",
    "Bebe, que tu cerebro esta haciendo ruido de modem viejo.",
    "Agua ahora. Luego sigues fingiendo que tienes todo controlado.",
];

pub fn run() -> anyhow::Result<()> {
    gtk::init().context("inicializar GTK; comprueba DISPLAY/WAYLAND_DISPLAY")?;

    let app = gtk::Application::builder()
        .application_id("dev.jorge.WaterReminder")
        .build();

    app.connect_activate(|app| {
        install_css();
        if let Err(err) = Service::start(app.clone()) {
            eprintln!("No se pudo iniciar Water Reminder: {err:#}");
            app.quit();
        }
    });

    app.run();
    Ok(())
}

struct TrayActions {
    count: MenuItem,
    next: MenuItem,
    pause_1h: MenuItem,
    pause_tomorrow: MenuItem,
    resume: MenuItem,
    reset: MenuItem,
    settings: MenuItem,
    quit: MenuItem,
}

struct Service {
    app: gtk::Application,
    settings: Settings,
    schedule: Schedule,
    window: Option<gtk::Window>,
    resolving_reminder: bool,
    settings_dialog: Option<gtk::Window>,
    tray: TrayIcon,
    actions: TrayActions,
    tray_icon_state: (u32, u32),
    _hold: gio::ApplicationHoldGuard,
}

impl Service {
    fn start(app: gtk::Application) -> anyhow::Result<()> {
        let settings = settings::load_settings();
        let count = stats::get_today_count();
        let (tray, actions) = build_tray(count, settings.daily_goal)?;
        let service = Rc::new(RefCell::new(Self {
            _hold: app.hold(),
            app,
            settings: settings.clone(),
            schedule: Schedule::default(),
            window: None,
            resolving_reminder: false,
            settings_dialog: None,
            tray,
            actions,
            tray_icon_state: (count, settings.daily_goal),
        }));

        service.borrow_mut().schedule.schedule_next(&settings, None);
        install_event_pump(service.clone());
        install_tick(service);
        Ok(())
    }

    fn tick(service: &Rc<RefCell<Self>>) {
        {
            let mut this = service.borrow_mut();
            this.refresh_tray();

            if let Some(until) = this.schedule.paused_until {
                if chrono::Local::now() >= until {
                    this.schedule.paused_until = None;
                    this.actions.resume.set_enabled(false);
                    let settings = this.settings.clone();
                    this.schedule.schedule_next(&settings, None);
                } else {
                    let label = format!("Pausado hasta {}", until.format("%H:%M"));
                    this.actions.next.set_text(&label);
                    let _ = this
                        .tray
                        .set_tooltip(Some(&format!("Water Reminder - {label}")));
                    return;
                }
            }

            if this.schedule.due_after_suspend()
                && this.window.is_none()
                && !this.resolving_reminder
            {
                drop(this);
                Self::show_reminder(service);
                return;
            }
        }

        service.borrow_mut().refresh_countdown();
    }

    fn show_reminder(service: &Rc<RefCell<Self>>) {
        {
            let mut this = service.borrow_mut();
            if this.window.is_some() || this.resolving_reminder {
                return;
            }
            this.schedule.next_deadline = None;
            this.schedule.next_at_wall = None;

            if let Some(until) = this.schedule.paused_until {
                if chrono::Local::now() < until {
                    let secs = (until - chrono::Local::now()).num_seconds().max(0) as u64;
                    this.schedule.next_deadline =
                        Some(std::time::Instant::now() + Duration::from_secs(secs));
                    this.schedule.next_at_wall = Some(until);
                    return;
                }
            }

            if !scheduler::within_active_hours(&this.settings, chrono::Local::now()) {
                let secs =
                    scheduler::seconds_until_active_window(&this.settings, chrono::Local::now());
                this.schedule.next_deadline =
                    Some(std::time::Instant::now() + Duration::from_secs(secs));
                this.schedule.next_at_wall =
                    Some(chrono::Local::now() + chrono::Duration::seconds(secs as i64));
                return;
            }

            this.schedule.state = ReminderState::ActiveReminder;
            if this.settings.sound_enabled {
                sound::play_sound();
            }
        }

        let window = build_reminder_window(service.clone());
        service.borrow_mut().window = Some(window);
        service.borrow_mut().refresh_tray();
    }

    fn confirm_done(service: &Rc<RefCell<Self>>) {
        let mut this = service.borrow_mut();
        if this.resolving_reminder {
            return;
        }
        this.resolving_reminder = true;
        if let Err(err) = stats::increment_today() {
            eprintln!("No se pudieron guardar stats: {err}");
        }
        let settings = this.settings.clone();
        this.schedule.schedule_next(&settings, None);
        this.window = None;
        this.resolving_reminder = false;
        this.refresh_tray();
    }

    fn snooze(service: &Rc<RefCell<Self>>, minutes: u32) {
        let mut this = service.borrow_mut();
        if this.resolving_reminder {
            return;
        }
        this.resolving_reminder = true;
        let settings = this.settings.clone();
        this.schedule.schedule_next(&settings, Some(minutes));
        this.window = None;
        this.resolving_reminder = false;
        this.refresh_tray();
    }

    fn open_settings(service: &Rc<RefCell<Self>>) {
        if let Some(dialog) = service.borrow().settings_dialog.as_ref() {
            dialog.present();
            return;
        }

        let dialog = build_settings_dialog(service.clone());
        service.borrow_mut().settings_dialog = Some(dialog.clone());
        dialog.show_all();
        dialog.present();
    }

    fn reset_today(service: &Rc<RefCell<Self>>) {
        let dialog = gtk::MessageDialog::new(
            None::<&gtk::Window>,
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Question,
            gtk::ButtonsType::YesNo,
            "Resetear vasos",
        );
        dialog.set_secondary_text(Some("Resetear los vasos de hoy a 0?"));
        let service_ref = service.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Yes {
                if let Err(err) = stats::reset_today() {
                    eprintln!("No se pudieron resetear stats: {err}");
                }
                service_ref.borrow_mut().refresh_tray();
            }
            dialog.close();
        });
        dialog.show_all();
    }

    fn refresh_tray(&mut self) {
        let count = stats::get_today_count();
        let icon_state = (count, self.settings.daily_goal);
        if icon_state != self.tray_icon_state {
            if let Ok(icon) = build_icon(count, self.settings.daily_goal) {
                let _ = self.tray.set_icon(Some(icon));
            }
            self.tray_icon_state = icon_state;
        }
        self.actions
            .count
            .set_text(format!("Vasos hoy: {count}/{}", self.settings.daily_goal));
    }

    fn refresh_countdown(&mut self) {
        if self.window.is_some() {
            self.actions.next.set_text("Recordatorio activo");
            return;
        }
        let Some(remaining) = self.schedule.remaining() else {
            return;
        };
        let secs = remaining.as_secs();
        let label = format!("Proximo: {:02}:{:02}", secs / 60, secs % 60);
        self.actions.next.set_text(&label);
        let count = stats::get_today_count();
        let _ = self.tray.set_tooltip(Some(&format!(
            "Water Reminder - {}/{} vasos | proximo {:02}:{:02}",
            count,
            self.settings.daily_goal,
            secs / 60,
            secs % 60
        )));
    }
}

fn build_tray(count: u32, goal: u32) -> anyhow::Result<(TrayIcon, TrayActions)> {
    let menu = Menu::new();
    let count_item = MenuItem::new(&format!("Vasos hoy: {count}/{goal}"), false, None);
    let next_item = MenuItem::new("Proximo: calculando...", false, None);
    let pause_1h = MenuItem::new("Pausar 1 hora", true, None);
    let pause_tomorrow = MenuItem::new("Pausar hasta manana", true, None);
    let resume = MenuItem::new("Reanudar", false, None);
    let reset = MenuItem::new("Resetear vasos de hoy", true, None);
    let settings = MenuItem::new("Ajustes...", true, None);
    let quit = MenuItem::new("Salir", true, None);

    menu.append_items(&[
        &count_item,
        &next_item,
        &PredefinedMenuItem::separator(),
        &pause_1h,
        &pause_tomorrow,
        &resume,
        &PredefinedMenuItem::separator(),
        &reset,
        &settings,
        &quit,
    ])?;

    let tray = TrayIconBuilder::new()
        .with_tooltip("Water Reminder")
        .with_icon(build_icon(count, goal)?)
        .with_menu(Box::new(menu))
        .build()
        .context("crear icono de bandeja")?;

    Ok((
        tray,
        TrayActions {
            count: count_item,
            next: next_item,
            pause_1h,
            pause_tomorrow,
            resume,
            reset,
            settings,
            quit,
        },
    ))
}

fn install_event_pump(service: Rc<RefCell<Service>>) {
    glib::timeout_add_local(Duration::from_millis(200), move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let id = event.id;
            let mut this = service.borrow_mut();
            if id == this.actions.pause_1h.id() {
                this.schedule.pause_for_hours(1);
                this.actions.resume.set_enabled(true);
            } else if id == this.actions.pause_tomorrow.id() {
                let settings = this.settings.clone();
                this.schedule.pause_until_tomorrow(&settings);
                this.actions.resume.set_enabled(true);
            } else if id == this.actions.resume.id() {
                let settings = this.settings.clone();
                this.schedule.resume(&settings);
                this.actions.resume.set_enabled(false);
            } else if id == this.actions.reset.id() {
                drop(this);
                Service::reset_today(&service);
            } else if id == this.actions.settings.id() {
                drop(this);
                Service::open_settings(&service);
            } else if id == this.actions.quit.id() {
                this.app.quit();
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(event, TrayIconEvent::Click { .. }) {
                Service::open_settings(&service);
            }
        }
        glib::ControlFlow::Continue
    });
}

fn install_tick(service: Rc<RefCell<Service>>) {
    glib::timeout_add_local(Duration::from_secs(1), move || {
        Service::tick(&service);
        glib::ControlFlow::Continue
    });
}

fn build_reminder_window(service: Rc<RefCell<Service>>) -> gtk::Window {
    let settings = service.borrow().settings.clone();
    let window = gtk::ApplicationWindow::new(&service.borrow().app);
    window.set_decorated(false);
    window.set_keep_above(true);
    window.style_context().add_class("reminder-window");

    if settings.fullscreen_mode {
        window.fullscreen();
    } else {
        window.set_default_size(460, 280);
        position_toast(&window);
    }

    let root = gtk::Box::new(gtk::Orientation::Vertical, 14);
    root.set_halign(gtk::Align::Center);
    root.set_valign(gtk::Align::Center);
    root.set_margin_top(30);
    root.set_margin_bottom(30);
    root.set_margin_start(40);
    root.set_margin_end(40);

    let emoji = gtk::Label::new(Some("💧"));
    add_class(
        &emoji,
        if settings.fullscreen_mode {
            "emoji-full"
        } else {
            "emoji-toast"
        },
    );
    root.pack_start(&emoji, false, false, 0);

    let title = gtk::Label::new(Some(if settings.fullscreen_mode {
        "Protocolo de Hidratacion"
    } else {
        "Bebe agua, desastre"
    }));
    add_class(
        &title,
        if settings.fullscreen_mode {
            "title-full"
        } else {
            "title-toast"
        },
    );
    root.pack_start(&title, false, false, 0);

    let mut rng = rand::rng();
    let message = REMINDER_MESSAGES
        .choose(&mut rng)
        .copied()
        .unwrap_or(REMINDER_MESSAGES[0]);
    let subtitle = gtk::Label::new(Some(message));
    subtitle.set_line_wrap(true);
    subtitle.set_justify(gtk::Justification::Center);
    add_class(
        &subtitle,
        if settings.fullscreen_mode {
            "subtitle-full"
        } else {
            "subtitle-toast"
        },
    );
    root.pack_start(&subtitle, false, false, 0);

    if settings.fullscreen_mode {
        let count = stats::get_today_count();
        let progress = gtk::Label::new(Some(&make_progress(count, settings.daily_goal)));
        add_class(&progress, "progress");
        root.pack_start(&progress, false, false, 0);
    }

    let done = gtk::Button::with_label("Ya he bebido, pesado");
    add_class(&done, "primary-button");
    root.pack_start(&done, false, false, 0);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_halign(gtk::Align::Center);
    for minutes in SNOOZE_OPTIONS {
        let button = gtk::Button::with_label(&format!("+{minutes} min"));
        add_class(&button, "snooze-button");
        let service_ref = service.clone();
        let window_ref = window.clone();
        button.connect_clicked(move |_| {
            Service::snooze(&service_ref, minutes);
            window_ref.close();
        });
        row.pack_start(&button, false, false, 0);
    }
    root.pack_start(&row, false, false, 0);
    window.add(&root);

    let resolved = Rc::new(Cell::new(false));
    let service_ref = service.clone();
    let resolved_ref = resolved.clone();
    let window_ref = window.clone();
    done.connect_clicked(move |_| {
        if resolved_ref.replace(true) {
            return;
        }
        Service::confirm_done(&service_ref);
        window_ref.close();
    });

    let service_ref = service.clone();
    let resolved_ref = resolved.clone();
    let window_ref = window.clone();
    window.connect_key_press_event(move |_, event| {
        if event.keyval() == gdk::keys::constants::Escape {
            if !resolved_ref.replace(true) {
                Service::confirm_done(&service_ref);
                window_ref.close();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });

    let service_ref = service.clone();
    window.connect_delete_event(move |_, _| {
        if !resolved.replace(true) {
            Service::confirm_done(&service_ref);
        }
        glib::Propagation::Proceed
    });

    window.show_all();
    window.upcast()
}

fn build_settings_dialog(service: Rc<RefCell<Service>>) -> gtk::Window {
    let current = service.borrow().settings.clone();
    let dialog = gtk::ApplicationWindow::new(&service.borrow().app);
    dialog.set_title("Water Reminder - Ajustes");
    dialog.set_default_width(420);
    dialog.set_modal(true);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(16);
    root.set_margin_end(16);

    let interval = spin(1.0, 240.0, current.interval_minutes as f64);
    let goal = spin(1.0, 20.0, current.daily_goal as f64);
    let start_hour = spin(0.0, 23.0, current.active_start_hour as f64);
    let end_hour = spin(1.0, 24.0, current.active_end_hour as f64);
    let sound = gtk::CheckButton::with_label("Reproducir sonido");
    sound.set_active(current.sound_enabled);
    let fullscreen =
        gtk::CheckButton::with_label("Pantalla completa (si no, notificacion discreta)");
    fullscreen.set_active(current.fullscreen_mode);

    root.pack_start(&section("Recordatorios"), false, false, 0);
    root.pack_start(&row("Intervalo:", &interval), false, false, 0);
    root.pack_start(&row("Meta diaria:", &goal), false, false, 0);
    let hours = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    hours.pack_start(&gtk::Label::new(Some("De")), false, false, 0);
    hours.pack_start(&start_hour, false, false, 0);
    hours.pack_start(&gtk::Label::new(Some("a")), false, false, 0);
    hours.pack_start(&end_hour, false, false, 0);
    root.pack_start(&row("Horario activo:", &hours), false, false, 0);
    root.pack_start(&section("Notificaciones"), false, false, 0);
    root.pack_start(&sound, false, false, 0);
    root.pack_start(&fullscreen, false, false, 0);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let reset = gtk::Button::with_label("Restablecer");
    let cancel = gtk::Button::with_label("Cancel");
    let ok = gtk::Button::with_label("OK");
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    buttons.pack_start(&reset, false, false, 0);
    buttons.pack_start(&spacer, true, true, 0);
    buttons.pack_start(&cancel, false, false, 0);
    buttons.pack_start(&ok, false, false, 0);
    root.pack_start(&buttons, false, false, 0);
    dialog.add(&root);

    let dialog_ref = dialog.clone();
    cancel.connect_clicked(move |_| dialog_ref.close());

    let interval_ref = interval.clone();
    let goal_ref = goal.clone();
    let start_ref = start_hour.clone();
    let end_ref = end_hour.clone();
    let sound_ref = sound.clone();
    let fullscreen_ref = fullscreen.clone();
    let service_ref = service.clone();
    let dialog_ref = dialog.clone();
    ok.connect_clicked(move |_| {
        let new_settings = Settings {
            interval_minutes: interval_ref.value_as_int() as u32,
            daily_goal: goal_ref.value_as_int() as u32,
            active_start_hour: start_ref.value_as_int() as u32,
            active_end_hour: end_ref.value_as_int() as u32,
            sound_enabled: sound_ref.is_active(),
            fullscreen_mode: fullscreen_ref.is_active(),
        };
        if new_settings.active_end_hour <= new_settings.active_start_hour {
            let warning = gtk::MessageDialog::new(
                Some(&dialog_ref),
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Warning,
                gtk::ButtonsType::Ok,
                "Horario invalido",
            );
            warning.set_secondary_text(Some("La hora de fin debe ser posterior a la de inicio."));
            warning.connect_response(|dialog, _| dialog.close());
            warning.show_all();
            return;
        }
        if let Err(err) = settings::save_settings(&new_settings) {
            eprintln!("No se pudieron guardar ajustes: {err}");
            return;
        }
        let mut this = service_ref.borrow_mut();
        let old_interval = this.settings.interval_minutes;
        this.settings = new_settings;
        if this.settings.interval_minutes != old_interval && this.schedule.paused_until.is_none() {
            let settings = this.settings.clone();
            this.schedule.schedule_next(&settings, None);
        }
        this.refresh_tray();
        dialog_ref.close();
    });

    let interval_ref = interval.clone();
    let goal_ref = goal.clone();
    let start_ref = start_hour.clone();
    let end_ref = end_hour.clone();
    let sound_ref = sound.clone();
    let fullscreen_ref = fullscreen.clone();
    reset.connect_clicked(move |_| {
        let defaults = Settings::default();
        interval_ref.set_value(defaults.interval_minutes as f64);
        goal_ref.set_value(defaults.daily_goal as f64);
        start_ref.set_value(defaults.active_start_hour as f64);
        end_ref.set_value(defaults.active_end_hour as f64);
        sound_ref.set_active(defaults.sound_enabled);
        fullscreen_ref.set_active(defaults.fullscreen_mode);
    });

    let service_ref = service.clone();
    dialog.connect_delete_event(move |_, _| {
        service_ref.borrow_mut().settings_dialog = None;
        glib::Propagation::Proceed
    });

    dialog.upcast()
}

fn spin(min: f64, max: f64, value: f64) -> gtk::SpinButton {
    let adjustment = gtk::Adjustment::new(value, min, max, 1.0, 5.0, 0.0);
    gtk::SpinButton::new(Some(&adjustment), 1.0, 0)
}

fn section(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::Start);
    add_class(&label, "section");
    label
}

fn row(label: &str, child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let label = gtk::Label::new(Some(label));
    label.set_width_chars(16);
    label.set_xalign(1.0);
    row.pack_start(&label, false, false, 0);
    row.pack_start(child, false, false, 0);
    row
}

fn add_class(widget: &impl IsA<gtk::Widget>, class_name: &str) {
    widget.style_context().add_class(class_name);
}

fn position_toast(window: &gtk::ApplicationWindow) {
    let Some(screen) = gdk::Screen::default() else {
        return;
    };
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let Some(monitor) = display.primary_monitor() else {
        return;
    };
    let geometry = monitor.geometry();
    let scale = screen.resolution().max(96.0) / 96.0;
    let margin = (24.0 * scale) as i32;
    window.move_(
        geometry.x() + geometry.width() - 460 - margin,
        geometry.y() + geometry.height() - 280 - margin,
    );
}

fn make_progress(count: u32, goal: u32) -> String {
    let filled = "💧".repeat(count.min(goal) as usize);
    let empty = "○".repeat(goal.saturating_sub(count) as usize);
    format!("{filled}{empty}   {count}/{goal} vasos hoy")
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(
            b"
        .reminder-window { background: #0f172a; }
        .emoji-full { font-size: 80px; color: #38bdf8; }
        .emoji-toast { font-size: 36px; color: #38bdf8; }
        .title-full { font-size: 40px; font-weight: 800; color: white; }
        .title-toast { font-size: 18px; font-weight: 800; color: white; }
        .subtitle-full { font-size: 16px; color: #94a3b8; }
        .subtitle-toast { font-size: 12px; color: #94a3b8; }
        .progress { font-family: monospace; font-size: 15px; color: #38bdf8; }
        .primary-button { background: #0284c7; color: white; font-weight: 800; min-width: 280px; min-height: 50px; border-radius: 10px; }
        .primary-button:hover { background: #0369a1; }
        .snooze-button { color: #94a3b8; border: 1px solid #334155; border-radius: 8px; min-width: 80px; min-height: 36px; }
        .section { font-weight: 800; margin-top: 6px; }
        ",
        )
        .expect("CSS embebido valido");
    if let Some(screen) = gdk::Screen::default() {
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_icon(count: u32, goal: u32) -> anyhow::Result<Icon> {
    let color = if count >= goal {
        [22, 163, 74, 255]
    } else {
        [2, 132, 199, 255]
    };
    let mut rgba = vec![0_u8; 64 * 64 * 4];
    for y in 0..64_i32 {
        for x in 0..64_i32 {
            let dx = x - 32;
            let dy = y - 32;
            if dx * dx + dy * dy <= 28 * 28 {
                let idx = (y as usize * 64 + x as usize) * 4;
                rgba[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
    draw_number(&mut rgba, count.min(99));
    Icon::from_rgba(rgba, 64, 64).context("crear icono RGBA")
}

fn draw_number(rgba: &mut [u8], number: u32) {
    let text = number.to_string();
    let scale = if text.len() == 1 { 7 } else { 5 };
    let digit_w = 3 * scale;
    let gap = scale;
    let total_w = digit_w * text.len() as i32 + gap * (text.len().saturating_sub(1) as i32);
    let start_x = (64 - total_w) / 2;
    let start_y = (64 - 5 * scale) / 2;
    for (i, ch) in text.chars().enumerate() {
        if let Some(pattern) = digit_pattern(ch) {
            let x = start_x + i as i32 * (digit_w + gap);
            draw_digit(rgba, pattern, x, start_y, scale);
        }
    }
}

fn draw_digit(rgba: &mut [u8], pattern: [&str; 5], start_x: i32, start_y: i32, scale: i32) {
    for (row, line) in pattern.iter().enumerate() {
        for (col, bit) in line.as_bytes().iter().enumerate() {
            if *bit != b'1' {
                continue;
            }
            for yy in 0..scale {
                for xx in 0..scale {
                    let x = start_x + col as i32 * scale + xx;
                    let y = start_y + row as i32 * scale + yy;
                    if (0..64).contains(&x) && (0..64).contains(&y) {
                        let idx = (y as usize * 64 + x as usize) * 4;
                        rgba[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
            }
        }
    }
}

fn digit_pattern(ch: char) -> Option<[&'static str; 5]> {
    Some(match ch {
        '0' => ["111", "101", "101", "101", "111"],
        '1' => ["010", "110", "010", "010", "111"],
        '2' => ["111", "001", "111", "100", "111"],
        '3' => ["111", "001", "111", "001", "111"],
        '4' => ["101", "101", "111", "001", "001"],
        '5' => ["111", "100", "111", "001", "111"],
        '6' => ["111", "100", "111", "101", "111"],
        '7' => ["111", "001", "010", "010", "010"],
        '8' => ["111", "101", "111", "101", "111"],
        '9' => ["111", "101", "111", "001", "111"],
        _ => return None,
    })
}
