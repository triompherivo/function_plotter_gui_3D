use eframe::egui;
use nalgebra::{Isometry3, Perspective3, Point3, Vector3};
use meval::Expr;
use std::f32::consts::PI;

struct App {
    // Function and domain settings:
    function_input: String,
    x_range: (f64, f64),
    y_range: (f64, f64),
    grid_size: usize,              // number of divisions per axis
    grid_points: Vec<Point3<f32>>, // computed grid of points
    error_message: Option<String>, // holds error message if function parsing fails

    // Camera parameters:
    azimuth: f32,   // rotation around vertical axis (in radians)
    elevation: f32, // rotation around horizontal axis (in radians)
    zoom: f32,      // distance from the origin
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            function_input: "sin(x) * cos(y)".to_string(),
            x_range: (-5.0, 5.0),
            y_range: (-5.0, 5.0),
            grid_size: 50,
            grid_points: Vec::new(),
            error_message: None,
            azimuth: 0.5,
            elevation: 0.5,
            zoom: 10.0,
        };
        app.generate_points();
        app
    }

    /// Generate the grid of 3D points by evaluating f(x,y) on a uniform grid.
    /// If parsing or binding fails, the previous grid is retained and an error is set.
    fn generate_points(&mut self) {
        let grid_size = self.grid_size;
        let n_points = grid_size + 1;
        let mut new_points = Vec::with_capacity(n_points * n_points);

        // Parse the user-supplied function:
        let expr = match self.function_input.parse::<Expr>() {
            Ok(e) => match e.bind2("x", "y") {
                Ok(f) => f,
                Err(err) => {
                    self.error_message = Some(format!("Error binding variables: {}", err));
                    return;
                }
            },
            Err(err) => {
                self.error_message = Some(format!("Error parsing function: {}", err));
                return;
            }
        };

        let (x_min, x_max) = self.x_range;
        let (y_min, y_max) = self.y_range;
        let dx = (x_max - x_min) / grid_size as f64;
        let dy = (y_max - y_min) / grid_size as f64;

        for i in 0..n_points {
            let x = x_min + i as f64 * dx;
            for j in 0..n_points {
                let y = y_min + j as f64 * dy;
                let z = expr(x, y);
                new_points.push(Point3::new(x as f32, y as f32, z as f32));
            }
        }

        // On success, update the grid and clear any previous error message.
        self.grid_points = new_points;
        self.error_message = None;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Left-side panel for controls:
        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.heading("Function Controls");
            ui.horizontal(|ui| {
                ui.label("f(x, y) = ");
                if ui.text_edit_singleline(&mut self.function_input).lost_focus()
                    && ctx.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    self.generate_points();
                }
            });

            ui.horizontal(|ui| {
                ui.label("X range:");
                ui.add(egui::DragValue::new(&mut self.x_range.0));
                ui.add(egui::DragValue::new(&mut self.x_range.1));
            });
            ui.horizontal(|ui| {
                ui.label("Y range:");
                ui.add(egui::DragValue::new(&mut self.y_range.0));
                ui.add(egui::DragValue::new(&mut self.y_range.1));
            });
            if ui.button("Plot Function").clicked() {
                self.generate_points();
            }

            if let Some(ref msg) = self.error_message {
                ui.colored_label(egui::Color32::RED, msg);
            }

            ui.separator();
            ui.heading("Camera Controls");
            ui.add(egui::Slider::new(&mut self.azimuth, 0.0..=2.0 * PI).text("Azimuth"));
            ui.add(egui::Slider::new(&mut self.elevation, -PI / 2.0..=PI / 2.0).text("Elevation"));
            ui.add(egui::Slider::new(&mut self.zoom, 1.0..=20.0).text("Zoom"));
            ui.label("Or drag on the plot to rotate.");
        });

        // Central panel for drawing the 3D plot:
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_size = ui.available_size();
            let (response, painter) =
                ui.allocate_painter(available_size, egui::Sense::drag());

            // Allow mouse dragging on the plot to adjust the camera:
            if response.drag_delta().length() > 0.0 {
                let delta = response.drag_delta();
                self.azimuth += delta.x * 0.005;
                self.elevation += delta.y * 0.005;
            }

            draw_3d_plot(&painter, response.rect, self);
        });
    }
}

/// Draw the 3D plot by projecting 3D grid points onto the 2D panel.
fn draw_3d_plot(painter: &egui::Painter, rect: egui::Rect, app: &App) {
    // Build the view matrix using camera parameters.
    let azimuth = app.azimuth;
    let elevation = app.elevation;
    let zoom = app.zoom;

    // Compute the camera position in spherical coordinates.
    let cam_x = zoom * elevation.cos() * azimuth.sin();
    let cam_y = zoom * elevation.sin();
    let cam_z = zoom * elevation.cos() * azimuth.cos();
    let camera_pos = Point3::new(cam_x, cam_y, cam_z);

    let target = Point3::origin();
    let up = Vector3::y_axis();
    let view = Isometry3::look_at_rh(&camera_pos, &target, &up);

    // Build a perspective projection matrix.
    let aspect = rect.width() / rect.height();
    let fovy = 45_f32.to_radians();
    let near = 0.1;
    let far = 100.0;
    let proj = Perspective3::new(aspect, fovy, near, far);

    // Compute the full transformation: projection * view.
    let transform = proj.to_homogeneous() * view.to_homogeneous();

    // Helper: transform a 3D point into screen coordinates.
    let to_screen = |p: &Point3<f32>| -> Option<egui::Pos2> {
        let p_hom = transform * p.to_homogeneous();
        if p_hom.w.abs() < 1e-6 {
            return None;
        }
        let ndc = p_hom.xyz() / p_hom.w;
        // Convert normalized device coordinates [-1, 1] to screen space.
        let screen_x = rect.left() + (ndc.x + 1.0) / 2.0 * rect.width();
        let screen_y = rect.top() + (1.0 - (ndc.y + 1.0) / 2.0) * rect.height();
        Some(egui::pos2(screen_x, screen_y))
    };

    let grid_size = app.grid_size;
    let n_points = grid_size + 1;

    // If no points exist, skip drawing.
    if app.grid_points.len() != n_points * n_points {
        return;
    }

    // Draw horizontal grid lines.
    for i in 0..n_points {
        let mut last_pos: Option<egui::Pos2> = None;
        for j in 0..n_points {
            let idx = i * n_points + j;
            if let Some(pos) = to_screen(&app.grid_points[idx]) {
                if let Some(last) = last_pos {
                    painter.line_segment(
                        [last, pos],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 0)),
                    );
                }
                last_pos = Some(pos);
            }
        }
    }

    // Draw vertical grid lines.
    for j in 0..n_points {
        let mut last_pos: Option<egui::Pos2> = None;
        for i in 0..n_points {
            let idx = i * n_points + j;
            if let Some(pos) = to_screen(&app.grid_points[idx]) {
                if let Some(last) = last_pos {
                    painter.line_segment(
                        [last, pos],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 0)),
                    );
                }
                last_pos = Some(pos);
            }
        }
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native("3D Function Plotter", options, Box::new(|_cc| Box::new(App::new())));
}
