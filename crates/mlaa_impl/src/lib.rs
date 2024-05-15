#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MlaaOptions {
    pub vertical_smoothing: bool,
    pub horizontal_smoothing: bool,
    pub corner_smoothing: bool,

    pub strict_mode: bool,
    pub seam_split_position: f32,
    pub seam_brigtness_balance: bool,

    pub smoothing_tolerance: usize,
}

impl Default for MlaaOptions {
    fn default() -> Self {
        MlaaOptions {
            vertical_smoothing: true,
            horizontal_smoothing: true,
            corner_smoothing: true,

            strict_mode: true,
            seam_split_position: 0.0,
            seam_brigtness_balance: false,

            smoothing_tolerance: 0,
        }
    }
}

pub enum MlaaFeature<C> {
    VerticalGradient {
        x: f32,
        y: f32,
        height: f32,
        colors: (C, C),
    },
    HorizontalGradient {
        x: f32,
        y: f32,
        width: f32,
        colors: (C, C),
    },
    Corner {
        x: isize,
        y: isize,
        colors: (C, C),
    },
}

pub fn mlaa_features<B, C>(
    image_width: usize,
    image_height: usize,
    image_colors: impl Fn(isize, isize) -> C,
    color_brightness: impl Fn(C) -> B,
    color_distance: impl Fn(C, C) -> usize + Copy,
    mlaa_options: &MlaaOptions,
    mut emit_mlaa_feature: impl FnMut(MlaaFeature<C>),
) where
    B: PartialOrd,
    C: PartialEq + Copy + Clone,
{
    let vertical_run = |x: isize, y: isize, pred: Box<dyn Fn((C, C)) -> bool>| -> isize {
        let mut run_length = 0;

        while (y + run_length < image_height as isize)
            && pred((image_colors(x, y + run_length), image_colors(x + 1, y + run_length)))
        {
            run_length += 1;
        }

        run_length
    };

    let horizontal_run = |x: isize, y: isize, pred: Box<dyn Fn((C, C)) -> bool>| -> isize {
        let mut run_length = 0;

        while (x + run_length < image_width as isize)
            && pred((image_colors(x + run_length, y), image_colors(x + run_length, y + 1)))
        {
            run_length += 1;
        }

        run_length
    };

    let seams_within_tolerance = |(a1, a2), (b1, b2)| -> bool {
        color_distance(a1, b1) <= mlaa_options.smoothing_tolerance
            && color_distance(a2, b2) <= mlaa_options.smoothing_tolerance
    };

    if mlaa_options.vertical_smoothing {
        for x in -1..image_width as isize {
            let mut y = 0;
            y += vertical_run(x, y, Box::new(|(c1, c2)| c1 == c2));

            while y < image_height as isize {
                let seam_colors = (image_colors(x, y), image_colors(x + 1, y));
                let seam_length = vertical_run(x, y, Box::new(move |c| seams_within_tolerance(c, seam_colors)));

                'neighbor_loop: for neighbor_delta in [-1, 1] {
                    #[allow(clippy::identity_op)]
                    let neighbor_colors = (
                        image_colors(x + neighbor_delta + 0, y + seam_length),
                        image_colors(x + neighbor_delta + 1, y + seam_length),
                    );

                    #[allow(clippy::collapsible_if)]
                    if mlaa_options.seam_brigtness_balance {
                        if (color_brightness(seam_colors.0) < color_brightness(seam_colors.1))
                            != (color_brightness(neighbor_colors.0) < color_brightness(neighbor_colors.1))
                        {
                            continue;
                        }
                    }

                    let neighbor_length = if mlaa_options.strict_mode {
                        vertical_run(x + neighbor_delta, y + seam_length, Box::new(move |c| seams_within_tolerance(c, seam_colors)))
                    } else {
                        let neighbor_length_1 = vertical_run(
                            x + neighbor_delta,
                            y + seam_length,
                            Box::new(move |(c1, c2)| (c1 == neighbor_colors.0) && (c2 == seam_colors.1) && (c1 != c2)),
                        );
                        let neighbor_length_2 = vertical_run(
                            x + neighbor_delta,
                            y + seam_length,
                            Box::new(move |(c1, c2)| (c1 == seam_colors.0) && (c2 == neighbor_colors.1) && (c1 != c2)),
                        );

                        neighbor_length_1.max(neighbor_length_2)
                    };

                    if neighbor_length > 0 {
                        let gradient_x = x.max(x + neighbor_delta) as f32;

                        let gradient_y = (y as f32)
                            + (seam_length as f32 / 2.0)
                            + (seam_length as f32 / 2.0 * mlaa_options.seam_split_position);

                        let gradient_length = (seam_length as f32 / 2.0) + (neighbor_length as f32 / 2.0)
                            - (seam_length as f32 / 2.0 * mlaa_options.seam_split_position)
                            - (neighbor_length as f32 / 2.0 * mlaa_options.seam_split_position);

                        let gradient_colors = if neighbor_delta < 0 {
                            (seam_colors.0, neighbor_colors.1)
                        } else {
                            (seam_colors.1, neighbor_colors.0)
                        };

                        emit_mlaa_feature(MlaaFeature::VerticalGradient {
                            x: gradient_x,
                            y: gradient_y,
                            height: gradient_length,
                            colors: gradient_colors,
                        });

                        break 'neighbor_loop;
                    }
                }

                y += seam_length;
                y += vertical_run(x, y, Box::new(|(c1, c2)| c1 == c2));
            }
        }
    }

    if mlaa_options.horizontal_smoothing {
        for y in -1..image_height as isize {
            let mut x = 0;
            x += horizontal_run(x, y, Box::new(|(c1, c2)| c1 == c2));

            while x < image_width as isize {
                let seam_colors = (image_colors(x, y), image_colors(x, y + 1));
                let seam_length = horizontal_run(x, y, Box::new(move |c| seams_within_tolerance(c, seam_colors)));

                'neighbor_loop: for neighbor_delta in [-1, 1] {
                    #[allow(clippy::identity_op)]
                    let neighbor_colors = (
                        image_colors(x + seam_length, y + neighbor_delta + 0),
                        image_colors(x + seam_length, y + neighbor_delta + 1),
                    );

                    #[allow(clippy::collapsible_if)]
                    if mlaa_options.seam_brigtness_balance {
                        if (color_brightness(seam_colors.0) < color_brightness(seam_colors.1))
                            != (color_brightness(neighbor_colors.0) < color_brightness(neighbor_colors.1))
                        {
                            continue;
                        }
                    }

                    let neighbor_length = if mlaa_options.strict_mode {
                        horizontal_run(x + seam_length, y + neighbor_delta, Box::new(move |c| seams_within_tolerance(c, seam_colors)))
                    } else {
                        let neighbor_length_1 = horizontal_run(
                            x + seam_length,
                            y + neighbor_delta,
                            Box::new(move |(c1, c2)| (c1 == neighbor_colors.0) && (c2 == seam_colors.1) && (c1 != c2)),
                        );
                        let neighbor_length_2 = horizontal_run(
                            x + seam_length,
                            y + neighbor_delta,
                            Box::new(move |(c1, c2)| (c1 == seam_colors.0) && (c2 == neighbor_colors.1) && (c1 != c2)),
                        );

                        neighbor_length_1.max(neighbor_length_2)
                    };

                    if neighbor_length > 0 {
                        let gradient_y = y.max(y + neighbor_delta) as f32;

                        let gradient_x = (x as f32)
                            + (seam_length as f32 / 2.0)
                            + (seam_length as f32 / 2.0 * mlaa_options.seam_split_position);

                        let gradient_length = (seam_length as f32 / 2.0) + (neighbor_length as f32 / 2.0)
                            - (seam_length as f32 / 2.0 * mlaa_options.seam_split_position)
                            - (neighbor_length as f32 / 2.0 * mlaa_options.seam_split_position);

                        let gradient_colors = if neighbor_delta < 0 {
                            (seam_colors.0, neighbor_colors.1)
                        } else {
                            (seam_colors.1, neighbor_colors.0)
                        };

                        emit_mlaa_feature(MlaaFeature::HorizontalGradient {
                            x: gradient_x,
                            y: gradient_y,
                            width: gradient_length,
                            colors: gradient_colors,
                        });

                        break 'neighbor_loop;
                    }
                }

                x += seam_length;
                x += horizontal_run(x, y, Box::new(|(c1, c2)| c1 == c2));
            }
        }
    }

    #[allow(clippy::identity_op)]
    if mlaa_options.corner_smoothing {
        fn all_equals<T: PartialEq>(items: &[T]) -> bool {
            items.iter().all(|item| item == &items[0])
        }

        for y in 1..image_height as isize - 1 {
            for x in 1..image_width as isize - 1 {
                let p = &image_colors;
                let (c1, c2, c3) = (p(x - 1, y - 1), p(x + 0, y - 1), p(x + 1, y - 1));
                let (c4, c5, c6) = (p(x - 1, y + 0), p(x + 0, y + 0), p(x + 1, y + 0));
                let (c7, c8, c9) = (p(x - 1, y + 1), p(x + 0, y + 1), p(x + 1, y + 1));

                // The light and dark corner placements have been separated
                // to handle the following commonly occurring pixel pattern in
                // binarized line arts (with all of its possible reflections):
                //
                // ....##   #: Color #1
                // ...###   .: Color #2
                // ...##,   ,: Color #3
                // .##,,,
                // ###,,,
                // ##,,,,
                //
                // When Color #1 is darker than Color #2 and #3, the algorithm
                // assumes it's an outline trace and tries to place corners to
                // ensure the result looks more continous:
                //
                // ....##   #: Color #1
                // ...###   .: Color #2
                // ..C##,   ,: Color #3
                // .##C,,   C: Corner pixel
                // ###,,,
                // ##,,,,
                //
                // When Color #1 is lighter than Color #2 and #3, the algorithm
                // tries to separate the trace into two separate lines. This is
                // a known quirk of my algorithm. If this behavior is
                // undesirable, fix your line art.
                //
                // ....##   #: Color #1
                // ...###   .: Color #2
                // ...C#,   ,: Color #3
                // .#C,,,   C: Corner pixel
                // ###,,,
                // ##,,,,
                //
                // This algorithm also handles blended corners on sharp boxes,
                // not just outline traces:
                //
                // ......    ......   #: Color #1
                // .####.    .C##C.   .: Color #2
                // .####. => .####.   C: Corner pixel
                // .####.    .####.
                // .####.    .C##C.
                // ......    ......
                //
                // ........    ........   #: Color #1
                // .###....    .C#C....   .: Color #2
                // .###....    .###....   C: Corner pixel
                // .###....    .###C...
                // .######. => .#####C.
                // .######.    .######.
                // .######.    .C####C.
                // ........    ........
                //
                // Corner placement rules:
                //
                // Lighter corner on dark base color (top-left):
                // +-+-+-+    +-+-+-+   D: Dark pixel
                // |L|L|L|    |L|L|L|   L: Light pixel
                // +-+-+-+    +-+-+-+   C: Corner pixel
                // |L|D|D| => |L|C|D|
                // +-+-+-+    +-+-+-+
                // |L|D| |    |L|D| |
                // +-+-+-+    +-+-+-+
                //
                // Darker corner on light base color (top-left):
                // +-+-+-+    +-+-+-+   D: Dark pixel
                // | |D|D|    | |D|D|   L: Light pixel
                // +-+-+-+    +-+-+-+   C: Corner pixel
                // |D|L|L| => |D|C|L|
                // +-+-+-+    +-+-+-+
                // |D|L| |    |D|L| |
                // +-+-+-+    +-+-+-+

                // Lighter corner on dark base color
                {
                    // Top-left corner
                    if all_equals(&[c5, c6, c8])
                        && all_equals(&[c1, c2, c3, c4, c7])
                        && (c1 != c5)
                        && (color_brightness(c1) >= color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c1, c5) })
                    }

                    // Top-right corner
                    if all_equals(&[c4, c5, c8])
                        && all_equals(&[c1, c2, c3, c6, c9])
                        && (c3 != c5)
                        && (color_brightness(c3) >= color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c3, c5) })
                    }

                    // Bottom-left corner
                    if all_equals(&[c2, c5, c6])
                        && all_equals(&[c1, c4, c7, c8, c9])
                        && (c7 != c5)
                        && (color_brightness(c7) >= color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c7, c5) })
                    }

                    // Bottom-right corner
                    if all_equals(&[c2, c5, c4])
                        && all_equals(&[c3, c6, c7, c8, c9])
                        && (c9 != c5)
                        && (color_brightness(c9) >= color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c9, c5) })
                    }
                }

                // Darker corner on light base color
                {
                    // Top-left corner
                    if all_equals(&[c5, c6, c8])
                        && all_equals(&[c2, c3, c4, c7])
                        && (c2 != c5)
                        && (color_brightness(c2) < color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c2, c5) })
                    }

                    // Top-right corner
                    if all_equals(&[c4, c5, c8])
                        && all_equals(&[c1, c2, c6, c9])
                        && (c2 != c5)
                        && (color_brightness(c2) < color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c2, c5) })
                    }

                    // Bottom-left corner
                    if all_equals(&[c2, c5, c6])
                        && all_equals(&[c1, c4, c8, c9])
                        && (c8 != c5)
                        && (color_brightness(c8) < color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c8, c5) })
                    }

                    // Bottom-right corner
                    if all_equals(&[c2, c5, c4])
                        && all_equals(&[c3, c6, c7, c8])
                        && (c8 != c5)
                        && (color_brightness(c8) < color_brightness(c5))
                    {
                        emit_mlaa_feature(MlaaFeature::Corner { x, y, colors: (c8, c5) })
                    }
                }
            }
        }
    }
}

pub fn mlaa_painter<C>(
    blend_colors: impl Fn(C, C, f32) -> C,
    mut draw_pixel: impl FnMut(isize, isize, C),
    mlaa_feature: &MlaaFeature<C>,
) where
    C: PartialEq + Copy + Clone,
{
    match mlaa_feature {
        MlaaFeature::VerticalGradient { x, y, height, colors } => {
            let y1 = y.floor() as isize;
            let y2 = (y + height).ceil() as isize;
            let x = *x as isize;

            for y in y1..y2 {
                let t = (0.5 + (y as f32) - (y1 as f32)) / ((y2 as f32) - (y1 as f32));
                draw_pixel(x, y, blend_colors(colors.0, colors.1, t));
            }
        }
        MlaaFeature::HorizontalGradient { x, y, width, colors } => {
            let x1 = x.floor() as isize;
            let x2 = (x + width).ceil() as isize;
            let y = *y as isize;

            for x in x1..x2 {
                let t = (0.5 + (x as f32) - (x1 as f32)) / ((x2 as f32) - (x1 as f32));
                draw_pixel(x, y, blend_colors(colors.0, colors.1, t));
            }
        }
        MlaaFeature::Corner { x, y, colors } => {
            draw_pixel(*x, *y, blend_colors(colors.0, colors.1, 0.5));
        }
    }
}
