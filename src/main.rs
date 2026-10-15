use eframe::egui;

mod catalog;
mod config;
mod desktop;
mod device;
mod hidpp;
mod profile;
mod ui;
mod worker;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--probe") => std::process::exit(probe()),
        Some("--buttons") => std::process::exit(buttons_cli(None)),
        Some("--set-button") => std::process::exit(buttons_cli(Some(&args[1..]))),
        Some("--help" | "-h") => {
            println!("hidra                         open the app");
            println!("hidra --probe                 show what Hidra can read from your mice");
            println!("hidra --buttons               show the buttons stored in the active onboard profile");
            println!("hidra --set-button ID ACTION [SHORTCUT]");
            println!("                              store an action, e.g. --set-button g6 play,");
            println!("                              --set-button g4 key Ctrl+Shift+T");
            std::process::exit(0);
        }
        _ => {}
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hidra")
            .with_app_id("hidra")
            .with_decorations(false)
            .with_transparent(false)
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("Hidra", options, Box::new(|cc| Ok(Box::new(ui::HidraApp::new(cc))))) {
        eprintln!("hidra: {e}");
        std::process::exit(1);
    }
}

/// Prints what Hidra can read from each Logitech device. Read-only.
fn probe() -> i32 {
    let candidates = device::candidates();
    if candidates.is_empty() {
        eprintln!("No Logitech hidraw devices found.");
        return 1;
    }
    for c in candidates {
        println!("{}  {}  (pid {:04x})", c.path.display(), c.name, c.product_id);
        let mut dev = match device::Device::open(&c.path) {
            Ok(d) => d,
            Err(e) => {
                println!("  not usable: {e}");
                continue;
            }
        };
        println!("  HID++ {}.{}", dev.protocol.0, dev.protocol.1);
        match dev.name() {
            Ok(n) => println!("  name      {n}"),
            Err(e) => println!("  name      ({e})"),
        }
        for f in dev.firmware().unwrap_or_default() {
            println!("  {:<9} {}", f.kind.to_lowercase(), f.version);
        }
        if let Ok(b) = dev.battery() {
            println!("  battery   {}% {:?}", b.percent, b.charging);
        }
        if let Ok(d) = dev.dpi() {
            println!("  dpi       {} (range {}–{} step {}, default {})", d.current, d.min, d.max, d.step, d.default);
        }
        if let Ok(r) = dev.report_rate() {
            let rates: Vec<String> = r.supported_ms.iter().map(|ms| format!("{}Hz", 1000 / *ms as u32)).collect();
            println!("  rate      {}Hz (supports {})", 1000 / r.current_ms.max(1) as u32, rates.join(" "));
        }
        if let Ok(o) = dev.onboard() {
            println!("  onboard   {:?}, profile {} of {}, {} buttons", o.mode, o.current_profile, o.profile_count, o.button_count);
        }
        if let Ok(fs) = dev.features() {
            let list: Vec<String> = fs.iter().filter(|f| !f.hidden).map(|f| format!("{:04x}", f.id)).collect();
            println!("  features  {}", list.join(" "));
        }
    }
    0
}

/// `--buttons` lists the active profile's bindings; `--set-button` changes one.
fn buttons_cli(set: Option<&[String]>) -> i32 {
    let Some(mut dev) = device::candidates().into_iter().find_map(|c| device::Device::open(&c.path).ok()) else {
        eprintln!("No Logitech mouse found (or no permission to open it).");
        return 1;
    };
    let onboard = match dev.onboard() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("This mouse has no onboard profiles: {e}");
            return 1;
        }
    };
    if let Some(args) = set {
        let (Some(id), Some(action)) = (args.first(), args.get(1)) else {
            eprintln!("usage: hidra --set-button ID ACTION [SHORTCUT]   (IDs: {})", ids());
            return 2;
        };
        let Some(control) = catalog::control(id) else {
            eprintln!("Unknown control {id}. Use one of: {}", ids());
            return 2;
        };
        let raw = match profile::binding_for(action, args.get(2).map(String::as_str)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                return 2;
            }
        };
        if let Err(e) = worker::write_buttons(&mut dev, &[(control.slot, raw)]) {
            eprintln!("Couldn't save: {e}");
            return 1;
        }
        println!("Saved {} → {} to the mouse.", control.tag, catalog::action_label(action));
    }
    let b = match worker::read_buttons(&mut dev, &onboard) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Couldn't read the profile: {e}");
            return 1;
        }
    };
    println!("Onboard profile {} ({:?} mode):", b.sector, onboard.mode);
    for c in catalog::CONTROLS {
        let (action, shortcut) = profile::action_for(b.raws[c.slot]);
        println!("  {:<3} {:<24} {}", c.tag, c.name, shortcut.unwrap_or_else(|| format!("{} ({action})", catalog::action_label(action))));
    }
    0
}

fn ids() -> String {
    catalog::CONTROLS.iter().map(|c| c.id).collect::<Vec<_>>().join(", ")
}
