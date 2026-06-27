use loadsmith_core::LockedPackage;

#[derive(Debug)]
pub struct Lockfile {
    pub version: u32,
    pub packages: Vec<LockedPackage>,
}
