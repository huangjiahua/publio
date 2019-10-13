use std::error::Error;

pub fn bytes_to_usize(b: &[u8]) -> Result<usize, Box<dyn Error>> {
    let s = std::str::from_utf8(b)?;
    let i = s.parse::<usize>()?;
    Ok(i)
}