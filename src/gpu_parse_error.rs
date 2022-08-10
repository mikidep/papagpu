use zerocopy::*;

#[repr(C)]
#[derive(Copy, Clone, AsBytes, FromBytes, Debug)]
pub struct GPUParseError {
    error: u32, // 0 if no error, > 0 otherwise.
    location: u32,
}

#[cfg(not(target_arch = "spirv"))]
impl std::fmt::Display for GPUParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.error == 0 {
            write!(f, "No parse error.")
        } else {
            write!(f, "Parse error at location {}.", self.location)
        }
    }
}

impl GPUParseError {
    pub fn no_error() -> GPUParseError {
        return GPUParseError {
            error: 0,
            location: 0,
        };
    }
}
