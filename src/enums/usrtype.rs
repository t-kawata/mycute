#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsrType {
    Other = 0,
    Corp,
    Indi,
}
