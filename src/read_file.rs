use std::fs;
use std::fs::File;
use std::io::{Write, BufReader, BufRead, Error, Read};
use std::path::Path;

fn main() -> Result<(), Error> {
    let path = "./TestFile";

    // let mut output = File::create(path)?;
    // write!(output, "Rust\n💖\nFun")?;

    let input = File::open(path)?;
    let v = input.bytes();

    // if Path::new(path).exists() {
    //     fs::remove_file(path).expect("can't clear directory");
    // }



    Ok(())
}