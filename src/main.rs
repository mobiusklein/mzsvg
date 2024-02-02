use std::env;
use std::io;

use mzdata;

use mzsvg::SpectrumSVG;

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);

    let path = args.next().expect("Please pass an MS data file path");
    let scan_index: usize = args
        .next()
        .expect("Please pass a scan number")
        .parse::<usize>()
        .unwrap_or_else(|e| {
            panic!("Failed to parse scan number: {e}");
        });

    let mut document = SpectrumSVG::default();

    let mut reader = mzdata::io::open_file(path)?;
    if let Some(spectrum) = reader.get_spectrum_by_index(scan_index) {
        document.axes_from(&spectrum).xlim(..);
        document.draw(&spectrum);
        document.save(&"image.svg")?;
    } else {
        panic!("Failed to find spectrum {scan_index}");
    }

    Ok(())
}
