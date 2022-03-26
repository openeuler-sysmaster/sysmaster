use nix::sys::signal::Signal;
use crate::manager::data::*;





#[derive(PartialEq, Debug, Eq)]
pub enum UnitLoadState {
    UnitStub = 0,
    UnitLoaded,
    UnitNotFound,
    UnitError,
    UnitMerged,
    UnitMasked,
    UnitLoadStateMax,
    UnitLoadStateInvalid = -1,
}

pub enum UnitState {
    UnitActive = 0,
    UnitReloading = 1,
    UnitInActive = 2,
    UnitFailed = 3,
    UnitActiving = 4,
    UnitDeactiving = 5,
    UnitStateMax = 6,
    UnitStateInvalid = -1,
}

enum UnitNameFlags {
    UnitNamePlain =1,
    UnitNameInstance = 2,
    UnitNameTemplate = 4,
    UnitNameAny = 1|2|4,
}

enum UnitFileState {
    UnitFileEnabled,
    UnitFileEnabledRuntime,
    UnitFileLinked,
    UnitFileLinkedRuntime,
    UnitFileAlias,
    UnitFileMasked,
    UnitFileMaskedRuntime,
    UnitFileStatic,
    UnitFileDisabled,
    UnitFileIndirect,
    UnitFileGenerated,
    UnitFileTransient,
    UnitFileBad,
    UnitFileStateMax,
    UnitFileStateInvalid,
}

#[derive(Eq, PartialEq, Debug)]
pub enum UnitActiveState {
    UnitActive,
    UnitReloading,
    UnitInactive,
    UnitFailed,
    UnitActivating,
    UnitDeactiviting,
    UnitMaintenance,
}

pub enum KillOperation {
    KillTerminate,
    KillTerminateAndLog,
    KillRestart,
    KillKill,
    KillWatchdog,
    KillInvalid,
}

impl KillOperation {
    pub fn to_signal(&self) -> Signal {
        match *self {
            KillOperation::KillTerminate | KillOperation::KillTerminateAndLog |
                KillOperation::KillRestart => Signal::SIGTERM,
            KillOperation::KillKill => Signal::SIGKILL,
            KillOperation::KillWatchdog => Signal::SIGABRT,
            _ => Signal::SIGTERM,
        }
    }
}


// #[macro_export]
// macro_rules! unit_name_to_type{
//     ($name:expr) => {
//         match $name{
//             "*.service" => UnitType::UnitService,
//             "*.target" => UnitType::UnitTarget,
//             _ => UnitType::UnitTypeInvalid,
//         }
//     };
// }

pub fn unit_name_to_type(unit_name: &str) -> UnitType {
    let words: Vec<&str> = unit_name.split(".").collect();
    match words[words.len()-1] {
        "service" => UnitType::UnitService,
        "target" => UnitType::UnitTarget,
        _ => UnitType::UnitTypeInvalid,
    }
}

#[macro_export]
macro_rules! null_str {
    ($name:expr) => {
        String::from($name)
    }
}