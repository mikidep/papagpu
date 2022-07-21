#[cfg(not(target_arch = "spirv"))]
use zerocopy::*;

#[repr(C)]
#[derive(Copy, Clone)]
#[cfg_attr(not(target_arch = "spirv"), derive(AsBytes, FromBytes, Debug))]
pub struct ParseError {
    error: u32, // 0 if no error, > 0 otherwise.
    location: u32
}

#[cfg(not(target_arch = "spirv"))]
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.error == 0 {
            write!(f, "No parse error.")
        }
        else {
            write!(f, "Parse error at location {}.", self.location)
        }
    }
}

impl ParseError {
    pub fn no_error() -> ParseError {
        return ParseError {
            error: 0,
            location: 0
        }
    }
}