use zerocopy::*;

#[repr(C)]
#[derive(Copy, Clone, AsBytes, FromBytes, Debug)]
pub struct StackSym {
    pub sym: u32,
    pub prec: u32,
}