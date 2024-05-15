use eframe::egui::{self, DragValue, PointerButton, Sense};
use eframe::emath::{lerp, remap};
use eframe::epaint::{vec2, Color32, Rect, Rgba, Stroke};

use mlaa_impl::{mlaa_features, mlaa_painter, MlaaFeature, MlaaOptions};

const IMAGE_WIDTH: usize = 32;
const IMAGE_HEIGHT: usize = 24;

struct MlaaApplication {
    selected_color: Color32,
    image_pixels: [[Color32; IMAGE_WIDTH]; IMAGE_HEIGHT],

    mlaa_options: MlaaOptions,
    mlaa_features: Vec<MlaaFeature<Color32>>,

    show_vertical_outlines: bool,
    show_horizontal_outlines: bool,
    show_corner_outlines: bool,
}

impl Default for MlaaApplication {
    fn default() -> MlaaApplication {
        let mut mlaa_application = MlaaApplication {
            selected_color: Color32::BLACK,
            image_pixels: Default::default(),

            mlaa_options: MlaaOptions::default(),
            mlaa_features: Vec::new(),

            show_vertical_outlines: true,
            show_horizontal_outlines: true,
            show_corner_outlines: true,
        };

        mlaa_application.generate_test_image();
        mlaa_application.recalculate_mlaa_features();
        mlaa_application
    }
}

impl MlaaApplication {
    fn generate_test_image(&mut self) {
        let aspect_ratio = if IMAGE_WIDTH > IMAGE_HEIGHT {
            vec2(IMAGE_WIDTH as f32 / IMAGE_HEIGHT as f32, 1.0)
        } else {
            vec2(1.0, IMAGE_HEIGHT as f32 / IMAGE_WIDTH as f32)
        };

        for y in 0..IMAGE_HEIGHT {
            for x in 0..IMAGE_WIDTH {
                let v = vec2(
                    remap(x as f32, 0.0..=(IMAGE_WIDTH as f32), -1.0..=1.0),
                    remap(y as f32, 0.0..=(IMAGE_HEIGHT as f32), -1.0..=1.0),
                ) * aspect_ratio;

                self.image_pixels[y][x] = if v.length() <= 0.9 && v.length() >= 0.5 {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                };
            }
        }
    }

    fn recalculate_mlaa_features(&mut self) {
        self.mlaa_features.clear();

        mlaa_features(
            IMAGE_WIDTH,
            IMAGE_HEIGHT,
            |x, y| {
                if (x < 0) || (x >= IMAGE_WIDTH as isize) {
                    return Color32::TRANSPARENT;
                }

                if (y < 0) || (y >= IMAGE_HEIGHT as isize) {
                    return Color32::TRANSPARENT;
                }

                self.image_pixels[y as usize][x as usize]
            },
            |c| Rgba::from(c).intensity(),
            |c1, c2| {
                c1.r().abs_diff(c2.r()) as usize
                    + c1.g().abs_diff(c2.g()) as usize
                    + c1.b().abs_diff(c2.b()) as usize
                    + c1.a().abs_diff(c2.a()) as usize
            },
            &self.mlaa_options,
            |mlaa_feature| self.mlaa_features.push(mlaa_feature),
        );
    }
}

impl eframe::App for MlaaApplication {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut needs_feature_recalc = false;

            ui.horizontal(|ui| {
                ui.color_edit_button_srgba(&mut self.selected_color);
                ui.separator();

                ui.vertical(|ui| {
                    ui.label("New image");

                    if ui.button("Test image").clicked() {
                        self.generate_test_image();
                        self.recalculate_mlaa_features();
                    }

                    if ui.button("Blank image").clicked() {
                        self.image_pixels = [[Color32::WHITE; IMAGE_WIDTH]; IMAGE_HEIGHT];
                        self.recalculate_mlaa_features();
                    }
                });
                ui.separator();

                ui.vertical(|ui| {
                    if (ui.checkbox(&mut self.mlaa_options.strict_mode, "Strict mode")
                        | ui.checkbox(&mut self.mlaa_options.seam_brigtness_balance, "Seam brightness balance"))
                    .changed()
                    {
                        needs_feature_recalc = true;
                    };

                    ui.horizontal(|ui| {
                        let drag_value = DragValue::new(&mut self.mlaa_options.seam_split_position)
                            .clamp_range(0.0..=1.0)
                            .speed(0.01);

                        if ui.add(drag_value).changed() {
                            needs_feature_recalc = true;
                        }

                        ui.label("Seam split position");
                    });

                    ui.horizontal(|ui| {
                        let drag_value =
                            DragValue::new(&mut self.mlaa_options.smoothing_tolerance).clamp_range(0..=768);

                        if ui.add(drag_value).changed() {
                            needs_feature_recalc = true;
                        }

                        ui.label("Smoothing tolerance");
                    });
                });
                ui.separator();

                ui.vertical(|ui| {
                    ui.label("Smoothing");
                    if (ui.checkbox(&mut self.mlaa_options.vertical_smoothing, "Vertical")
                        | ui.checkbox(&mut self.mlaa_options.horizontal_smoothing, "Horizontal")
                        | ui.checkbox(&mut self.mlaa_options.corner_smoothing, "Corners"))
                    .changed()
                    {
                        needs_feature_recalc = true;
                    };
                });
                ui.separator();

                ui.vertical(|ui| {
                    ui.label("Outlines");
                    ui.checkbox(&mut self.show_vertical_outlines, "Vertical");
                    ui.checkbox(&mut self.show_horizontal_outlines, "Horizontal");
                    ui.checkbox(&mut self.show_corner_outlines, "Corners");
                });
                ui.separator();
            });

            ui.separator();

            ui.scope(|ui| {
                let cell_size = vec2(24.0, 24.0);

                // Draw widget base
                let widget_size = cell_size * vec2(IMAGE_WIDTH as f32, IMAGE_HEIGHT as f32);
                let (rect, _response) = ui.allocate_exact_size(widget_size, Sense::hover());
                ui.painter().rect_filled(rect, 0.0, Color32::GRAY);

                // Draw pixels
                for y in 0..IMAGE_HEIGHT {
                    for x in 0..IMAGE_WIDTH {
                        let pixel_rect =
                            Rect::from_min_size(rect.left_top() + cell_size * vec2(x as f32, y as f32), cell_size);

                        let pixel_response = ui.allocate_rect(pixel_rect, Sense::click_and_drag());

                        if pixel_response.clicked_by(PointerButton::Primary) {
                            self.image_pixels[y][x] = self.selected_color;
                            needs_feature_recalc = true;
                        }

                        if pixel_response.clicked_by(PointerButton::Secondary) {
                            self.selected_color = self.image_pixels[y][x];
                        }

                        ui.painter()
                            .rect_filled(pixel_rect.shrink(1.0), 0.0, self.image_pixels[y][x]);
                    }
                }

                // Recalculate features if necessary
                if needs_feature_recalc {
                    self.recalculate_mlaa_features();
                }

                // Draw features
                for mlaa_feature in &self.mlaa_features {
                    mlaa_painter(
                        |color_a, color_b, t| lerp(Rgba::from(color_a)..=Rgba::from(color_b), t).into(),
                        |x, y, color| {
                            let pixel_rect =
                                Rect::from_min_size(rect.left_top() + cell_size * vec2(x as f32, y as f32), cell_size);

                            ui.painter().rect_filled(pixel_rect.shrink(1.0), 0.0, color);
                        },
                        mlaa_feature,
                    );
                }

                // Draw feature outlines
                for mlaa_feature in &self.mlaa_features {
                    match mlaa_feature {
                        MlaaFeature::VerticalGradient { x, y, height, colors } => {
                            if self.show_vertical_outlines {
                                let gradient_rect = Rect::from_min_size(
                                    rect.left_top() + cell_size * vec2(*x, *y),
                                    cell_size * vec2(1.0, *height),
                                );

                                let color = Color32::GREEN;
                                let stroke_thin = Stroke { width: 2.0, color };
                                let stroke_bold = Stroke { width: 3.0, color };

                                ui.painter().rect_stroke(gradient_rect, 0.0, stroke_thin);

                                ui.painter().line_segment(
                                    [gradient_rect.center_top(), gradient_rect.center_bottom()],
                                    stroke_bold,
                                );

                                ui.painter()
                                    .circle(gradient_rect.center_top(), 4.0, colors.0, stroke_thin);
                                ui.painter()
                                    .circle(gradient_rect.center_bottom(), 4.0, colors.1, stroke_thin);
                            }
                        }
                        MlaaFeature::HorizontalGradient { x, y, width, colors } => {
                            if self.show_horizontal_outlines {
                                let gradient_rect = Rect::from_min_size(
                                    rect.left_top() + cell_size * vec2(*x, *y),
                                    cell_size * vec2(*width, 1.0),
                                );

                                let color = Color32::YELLOW;
                                let stroke_thin = Stroke { width: 2.0, color };
                                let stroke_bold = Stroke { width: 3.0, color };

                                ui.painter().rect_stroke(gradient_rect, 0.0, stroke_thin);

                                ui.painter().line_segment(
                                    [gradient_rect.left_center(), gradient_rect.right_center()],
                                    stroke_bold,
                                );

                                ui.painter()
                                    .circle(gradient_rect.left_center(), 4.0, colors.0, stroke_thin);
                                ui.painter()
                                    .circle(gradient_rect.right_center(), 4.0, colors.1, stroke_thin);
                            }
                        }
                        MlaaFeature::Corner { x, y, colors } => {
                            if self.show_corner_outlines {
                                let corner_rect = Rect::from_min_size(
                                    rect.left_top() + cell_size * vec2(*x as f32, *y as f32),
                                    cell_size,
                                );

                                let color = Color32::RED;
                                let stroke_thin = Stroke { width: 2.0, color };

                                ui.painter().rect_stroke(corner_rect, 0.0, stroke_thin);
                                ui.painter().circle(corner_rect.center(), 8.0, colors.0, stroke_thin);
                                ui.painter().circle(corner_rect.center(), 4.0, colors.1, stroke_thin);
                            }
                        }
                    }
                }
            })
        });
    }
}

// cargo run --release --bin mlaa_egui

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(vec2(800.0, 640.0)),
        ..Default::default()
    };

    eframe::run_native("MLAA", options, Box::new(|_| Box::<MlaaApplication>::default()))
}
