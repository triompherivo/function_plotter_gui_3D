use eframe::egui;
use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::nalgebra::Point3;
use kiss3d::window::Window;
use meval::Expr;
use std::sync::{Arc, Mutex};

/// App holds the function, domain, and computed grid points.
/// The points are stored as a tuple: (grid_size, points)
/// where grid_size is the number of divisions along one axis,
/// and the total number of points is (grid_size + 1)².
struct App {
    function_input: String,
    x_range: (f64, f64),
    y_range: (f64, f64),
    points: Arc<Mutex<(usize, Vec<Point3<f32>>)>>,
}

impl App {
    fn new() -> Self {
        let app = Self {
            function_input: "sin(x) * cos(y)".to_string(),
            x_range: (-5.0, 5.0),
            y_range: (-5.0, 5.0),
            // initialize with grid_size = 100 (i.e. 101 x 101 points)
            points: Arc::new(Mutex::new((100, Vec::new()))),
        };
        let mut app = app;
        app.generate_points();
        app
    }

    /// Generates grid points by sampling f(x, y) on a uniform grid.
    fn generate_points(&mut self) {
        let grid_size = 100; // number of divisions along each axis
        let n_points = grid_size + 1; // number of points along each axis

        let mut new_points = Vec::with_capacity(n_points * n_points);
        let expr = match self.function_input.parse::<Expr>() {
            Ok(e) => match e.bind2("x", "y") {
                Ok(f) => f,
                Err(err) => {
                    eprintln!("Error binding variables: {}", err);
                    return;
                }
            },
            Err(err) => {
                eprintln!("Error parsing function: {}", err);
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
        let mut points_lock = self.points.lock().unwrap();
        *points_lock = (grid_size, new_points);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Enter function f(x, y):");
            ui.text_edit_singleline(&mut self.function_input);

            ui.horizontal(|ui| {
                ui.label("X Range:");
                ui.add(egui::DragValue::new(&mut self.x_range.0).speed(0.1));
                ui.add(egui::DragValue::new(&mut self.x_range.1).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Y Range:");
                ui.add(egui::DragValue::new(&mut self.y_range.0).speed(0.1));
                ui.add(egui::DragValue::new(&mut self.y_range.1).speed(0.1));
            });
            if ui.button("Plot Function").clicked() {
                self.generate_points();
            }
        });
    }
}

fn main() {
    let mut app = App::new();
    let points_arc = Arc::clone(&app.points);

    // Spawn the kiss3d window in a separate thread for 3D rendering.
    std::thread::spawn(move || {
        let mut window = Window::new("3D Function Plotter");
        let eye = Point3::new(3.0, 3.0, 3.0);
        let at = Point3::origin();
        let mut arc_ball = ArcBall::new(eye, at);
        window.set_light(Light::StickToCamera);
        while window.render_with_camera(&mut arc_ball) {
            // Copy grid data from the shared state.
            let (grid_size, points) = {
                let lock = points_arc.lock().unwrap();
                (*lock).clone()
            };
            let n = grid_size + 1;
            let color = Point3::new(0.0, 1.0, 0.0);

            // Draw horizontal and vertical grid lines.
            for i in 0..n {
                for j in 0..n {
                    let idx = i * n + j;
                    // Draw line to right neighbor if it exists.
                    if j < n - 1 {
                        let right_idx = idx + 1;
                        window.draw_line(&points[idx], &points[right_idx], &color);
                    }
                    // Draw line to the bottom neighbor if it exists.
                    if i < n - 1 {
                        let bottom_idx = idx + n;
                        window.draw_line(&points[idx], &points[bottom_idx], &color);
                    }
                }
            }
        }
    });

    // Run the GUI on the main thread.
    let options = eframe::NativeOptions::default();
    eframe::run_native("Function Plotter", options, Box::new(|_cc| Box::new(app)))
        .unwrap();
}
