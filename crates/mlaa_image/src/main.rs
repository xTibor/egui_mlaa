#![feature(error_iter)]

use std::error::Error;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use image::{ImageFormat, Pixel, Rgba};

use mlaa_impl::{mlaa_features, mlaa_painter, MlaaOptions};

#[derive(Parser)]
#[command(version)]
struct MlaaArgs {
    #[clap(short = 'i', long = "input")]
    input_path: Option<PathBuf>,

    #[clap(short = 'o', long = "output")]
    output_path: Option<PathBuf>,

    #[clap(short = 'c', long = "config")]
    config_path: Option<PathBuf>,
}

// cargo run --release --bin mlaa_image -- -i test/input.png -o test/output.png

fn main() -> ExitCode {
    match main_inner() {
        Ok(exit_code) => exit_code,
        Err(err) => {
            for source in err.sources() {
                eprintln!("mlaa_image: {}", source);
            }
            ExitCode::FAILURE
        }
    }
}

fn main_inner() -> Result<ExitCode, Box<dyn Error>> {
    let args = MlaaArgs::try_parse()?;

    let mlaa_options = {
        let config_path = {
            if let Some(config_path) = args.config_path {
                Some(config_path)
            } else if let Some(search_root) = args.input_path.as_ref().and_then(|path| path.parent()) {
                search_root
                    .ancestors()
                    .map(|search_directory| search_directory.to_owned().join(".mlaa"))
                    .find(|config_path| config_path.is_file())
            } else {
                None
            }
        };

        if let Some(config_path) = config_path {
            eprintln!("mlaa_image: Using config file \"{}\"", config_path.display());
            toml::from_str(&fs::read_to_string(config_path)?)?
        } else {
            eprintln!("mlaa_image: Using default MLAA options");
            MlaaOptions::default()
        }
    };

    let input_image = {
        let mut reader: Box<dyn Read> = if let Some(input_path) = args.input_path.as_ref() {
            Box::new(File::open(input_path)?)
        } else {
            Box::new(std::io::stdin())
        };

        let image_format = if let Some(input_path) = args.input_path.as_ref() {
            ImageFormat::from_path(input_path)?
        } else {
            ImageFormat::Png
        };

        let mut image_data = Vec::new();
        reader.read_to_end(&mut image_data)?;

        image::load_from_memory_with_format(&image_data, image_format)?
    };

    let input_image = input_image.to_rgba8();
    let mut output_image = input_image.clone();

    mlaa_features(
        input_image.width() as usize,
        input_image.height() as usize,
        |x, y| {
            input_image
                .get_pixel_checked(x as u32, y as u32)
                .unwrap_or(&Rgba([0, 0, 0, 0]))
                .to_owned()
        },
        |c| c.to_luma().0,
        |c1, c2| {
            c1.0[0].abs_diff(c2.0[0]) as usize
                + c1.0[1].abs_diff(c2.0[1]) as usize
                + c1.0[2].abs_diff(c2.0[2]) as usize
                + c1.0[3].abs_diff(c2.0[3]) as usize
        },
        &mlaa_options,
        |mlaa_feature| {
            mlaa_painter(
                |c1, c2, t| {
                    // The `image` crate doesn't give a fuck about gamma correctness.
                    // Have to use `smol-rgb` instead of their blending functions.

                    fn lerp(a: f32, b: f32, t: f32) -> f32 {
                        a * (1.0 - t) + b * t
                    }

                    Rgba([
                        smol_rgb::linear_to_encoded(lerp(
                            smol_rgb::encoded_to_linear(c1.0[0]),
                            smol_rgb::encoded_to_linear(c2.0[0]),
                            t,
                        )),
                        smol_rgb::linear_to_encoded(lerp(
                            smol_rgb::encoded_to_linear(c1.0[1]),
                            smol_rgb::encoded_to_linear(c2.0[1]),
                            t,
                        )),
                        smol_rgb::linear_to_encoded(lerp(
                            smol_rgb::encoded_to_linear(c1.0[2]),
                            smol_rgb::encoded_to_linear(c2.0[2]),
                            t,
                        )),
                        lerp(c1.0[3] as f32, c2.0[3] as f32, t) as u8,
                    ])
                },
                |x, y, c| {
                    if x < 0 || y < 0 {
                        return;
                    }

                    output_image.put_pixel(x as u32, y as u32, c);
                },
                &mlaa_feature,
            );
        },
    );

    {
        let mut writer: Box<dyn Write> = if let Some(output_path) = args.output_path.as_ref() {
            Box::new(File::create(output_path)?)
        } else {
            Box::new(std::io::stdout())
        };

        let image_format = if let Some(output_path) = args.output_path.as_ref() {
            ImageFormat::from_path(output_path)?
        } else {
            ImageFormat::Png
        };

        let mut image_data = Vec::new();
        output_image.write_to(&mut Cursor::new(&mut image_data), image_format)?;

        writer.write_all(&image_data)?;
    }

    Ok(ExitCode::SUCCESS)
}
