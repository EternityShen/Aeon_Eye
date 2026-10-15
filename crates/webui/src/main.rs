use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "我的计数器",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    count: i32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self { count: 0 }
    }
}

impl eframe::App for MyApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 留空：计数器逻辑足够简单，不需要额外的帧逻辑
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.label(format!("点击次数: {}", self.count));

        if ui.button("+1").clicked() {
            self.count += 1;
        }
    }
}
