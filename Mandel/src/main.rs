// Mandelbrot plotting
// Program that plots the mandelbrot set, using multiple processor threads

use core::str::FromStr;
use std::env;
use std::fs::File;

use num::Complex;
use image::ColorType;
use image::png::PNGEncoder;
use crossbeam;

struct Arguments {
  file: String,
  pixels: String,
  upper_left: String,
  lower_right: String,
}

fn main() {
  let args = parse_args();

  let bounds = parse_pair(&args.pixels, 'x').expect("error parsing image dimensions");
  let upper_left = parse_complex(&args.upper_left).expect("error parsing upper left corner point");
  let lower_right = parse_complex(&args.lower_right).expect("error parsing lower right corner point");

  let mut pixels = vec![0; bounds.0 * bounds.1];

  let threads = 8;
  let rows_per_band = bounds.1 / threads + 1;

  {
    let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
    
    crossbeam::scope(|spawner| {
      for (i, band) in bands.into_iter().enumerate() {
        let top = rows_per_band * i;
        let height = band.len() / bounds.0;
        let band_bounds = (bounds.0, height);
        let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
        let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);

        spawner.spawn(move |_| {
          render(band, band_bounds, band_upper_left, band_lower_right);
        });
      }
    }).unwrap();
  }

  write_image(&args.file, &pixels, bounds).expect("error writing PNG file");
}

fn parse_args() -> Arguments {
  let args: Vec<String> = env::args().collect();

  if args.len() != 5 {
    eprintln!("Usage: {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0]);
    eprintln!("Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20", args[0]);
    std::process::exit(1);
  }

  Arguments {
    file: args[1].clone(),
    pixels: args[2].clone(),
    upper_left: args[3].clone(),
    lower_right: args[4].clone(),
  }
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error> {
  let output = File::create(filename)?;

  //
  // "?" is the same as doing the following:
  //
  // let output = match File::create(filename) {
  //   Ok(f) => f,
  //   Err(e) => {
  //     return Err(e);
  //   } 
  // }
  //

  let encoder = PNGEncoder::new(output);
  encoder.encode(pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;

  Ok(())
}

fn render(pixels: &mut [u8], bounds: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) {
  assert!(pixels.len() == bounds.0 * bounds.1);

  for row in 0..bounds.1 {
    for column in 0..bounds.0 {
      let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
      pixels[row * bounds.0 + column] = match escape_time(point, 255) {
        None => 0,
        Some(count) => 255 - count as u8
      };
    }
  }
}

fn pixel_to_point(bounds: (usize, usize), pixel: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) -> Complex<f64> {

  let (width, height) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);

  Complex {
    re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
    im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64
  }
}

fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
  let mut z = Complex{ re: 0.0, im: 0.0 };

  for i in 0..limit {
    if z.norm_sqr() > 4.0 {
      return Some(i);
    }
    z = z * z + c;
  }

  None
}

fn parse_complex(string: &str) -> Option<Complex<f64>> {
  match parse_pair(string, ',') {
    Some((re, im)) => Some(Complex { re, im }),
    None => None
  }
}

fn parse_pair<T: FromStr>(string: &str, separator: char) -> Option<(T, T)> {
  match string.find(separator) {
    None => None,
    Some(index) => {
      match (T::from_str(&string[..index]), T::from_str(&string[index + 1..])) {
        (Ok(l), Ok(r)) => Some((l, r)),
        _ => None
      }
    }
  }
}

#[test]
fn test_parse_pair() {
  assert_eq!(parse_pair::<u32>("", ','), None);
  assert_eq!(parse_pair::<u32>("10,", ','), None);
  assert_eq!(parse_pair::<u32>(",10", ','), None);
  assert_eq!(parse_pair::<u32>("10,20", ','), Some((10, 20)));
  assert_eq!(parse_pair::<u32>("200x400", 'x'), Some((200, 400)));
  assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

#[test]
fn test_parse_complex() {
  assert_eq!(parse_complex("1.25,-0.0625"), Some(Complex { re: 1.25, im: -0.0625 }));
  assert_eq!(parse_complex(",-0.0625"), None);
}
