use eframe::{egui, App};
use egui_plot::{Legend, Line};
use log::{info, error};

mod motor;
use motor::{MotorCmd, MotorDriver};
use mover::MoverConnection;

mod mover;

fn main() {
    env_logger::init();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "simpos",
        options,
        Box::new(|cc| Ok(Box::new(Application::new(cc)))),
    );
}

struct Application {
    com_port: String,
    motor: Option<motor::MotorDriver>,
    mover: Option<MoverConnection>,
}

impl Application {
    fn new(_cc: &eframe::CreationContext) -> Self {
        info!("running");
        Application {
            com_port: "COM20".into(),
            motor: None,
            mover: MoverConnection::new("127.0.0.1:10000").ok()
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::left("config").show(ctx, |ui| {
            ui.label("COM Port:");
            ui.text_edit_singleline(&mut self.com_port);
            if self.motor.is_some() {
                if ui.button("Disconnect").clicked() {
                    self.motor = None;
                }
                ui.label("Connected...");
                if ui.button("Enable").clicked() {
                    self.motor.as_mut().unwrap().send_command(MotorCmd::Enable(true));
                    self.mover.as_mut().unwrap().connect_to_motor(&self.motor.as_ref().unwrap().command);
                }
                if ui.button("Home").clicked() {
                    self.motor.as_mut().unwrap().send_command(MotorCmd::Home);
                }
                if ui.button("Disable").clicked() {
                    self.motor.as_mut().unwrap().send_command(MotorCmd::Enable(false))
                }
            } else {
                if ui.button("Connect").clicked() {
                    self.motor = MotorDriver::connect(&self.com_port).ok();
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(motor) = self.motor.as_ref() {
                let telem = motor.get_telemetry();
                let link_id = ui.id();
                egui_plot::Plot::new("plot-torque")
                    .legend(Legend::default())
                    .width(1000.0)
                    .height(200.0)
                    .link_axis(link_id, true, false)
                    .link_cursor(link_id, true, false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(telem.torque_sns.clone()).name("isns"));
                        plot_ui.line(Line::new(telem.torque_cmd.clone()).name("icmd"));
                    });
                egui_plot::Plot::new("plot-speed")
                    .legend(Legend::default())
                    .width(1000.0)
                    .height(200.0)
                    .link_axis(link_id, true, false)
                    .link_cursor(link_id, true, false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(telem.speed_sns.clone()).name("cmd"));
                        plot_ui.line(Line::new(telem.speed_cmd.clone()).name("rpm"));
                        plot_ui.line(Line::new(telem.speed_p.clone()).name("p"));
                        plot_ui.line(Line::new(telem.speed_i.clone()).name("i"));
                        plot_ui.line(Line::new(telem.speed_d.clone()).name("d"));
                    });
                egui_plot::Plot::new("plot-pos")
                    .legend(Legend::default())
                    .width(1000.0)
                    .height(200.0)
                    .link_axis(link_id, true, false)
                    .link_cursor(link_id, true, false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(telem.position_cmd.clone()).name("cmd"));
                        plot_ui.line(Line::new(telem.position_sns.clone()).name("pos"));
                    });
            }
        });
    }
}
