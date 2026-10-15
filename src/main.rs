use eframe::egui;

mod catalog;
mod config;
mod desktop;
mod device;
mod hidpp;
mod ui;
mod worker;

fn main() {
    if std::env::args().any(|a| a == "--probe") {
        std::process::exit(probe());
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
