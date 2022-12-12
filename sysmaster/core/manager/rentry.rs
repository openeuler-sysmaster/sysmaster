use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ReliLastQue {
    Load = 0,
    TargetDeps,
    GcJob,
    GcUnit,
    Clean,
    CgRealize,
    StartWhenUpheld,
    StopWhenBound,
    StopWhenUnneeded,
    Dbus,
}

impl TryFrom<u32> for ReliLastQue {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReliLastQue::Load),
            1 => Ok(ReliLastQue::TargetDeps),
            2 => Ok(ReliLastQue::GcJob),
            3 => Ok(ReliLastQue::GcUnit),
            4 => Ok(ReliLastQue::Clean),
            5 => Ok(ReliLastQue::CgRealize),
            6 => Ok(ReliLastQue::StartWhenUpheld),
            7 => Ok(ReliLastQue::StopWhenBound),
            8 => Ok(ReliLastQue::StopWhenUnneeded),
            9 => Ok(ReliLastQue::Dbus),
            v => Err(format!("input {} is invalid", v)),
        }
    }
}

/* manager */
#[allow(dead_code)]
const RELI_DB_HMNG: &str = "manager";
/* job */
pub(in crate::core) const RELI_DB_HJOB_TRIGGER: &str = "jtrigger";
pub(in crate::core) const RELI_DB_HJOB_SUSPENDS: &str = "jsuspends";
/* unit */
pub(in crate::core) const RELI_DB_HUNIT_BASE: &str = "ubase";
pub(in crate::core) const RELI_DB_HUNIT_LOAD: &str = "uload";
pub(in crate::core) const RELI_DB_HUNIT_CONFIG: &str = "uconfig";
pub(in crate::core) const RELI_DB_HUNIT_CGROUP: &str = "ucgroup";
pub(in crate::core) const RELI_DB_HUNIT_CHILD: &str = "uchild";
pub(in crate::core) const RELI_DB_HUNIT_PPS: &str = "upps";
pub(in crate::core) const RELI_DB_HUNIT_DEP: &str = "udep";
pub(in crate::core) const RELI_DB_HUM_NOTIFY: &str = "um-notify";
/* service */
#[allow(dead_code)]
const RELI_DB_HSERVICE_CONF: &str = "svcconf";
#[allow(dead_code)]
const RELI_DB_HSERVICE_MNG: &str = "svcmng";
/* socket */
#[allow(dead_code)]
const RELI_DB_HSOCKET_CONF: &str = "sockconf";
#[allow(dead_code)]
const RELI_DB_HSOCKET_MNG: &str = "sockmng";
#[allow(dead_code)]
const RELI_DB_HSOCKETM_FRAME: &str = "sockm-frame";
/* mount */
#[allow(dead_code)]
const RELI_DB_HMOUNT_MNG: &str = "mntmng";
#[allow(dead_code)]
const RELI_DB_HMOUNTM_FRAME: &str = "mntm-frame";
#[allow(dead_code)]
/* target */
const RELI_DB_HTARGET_MNG: &str = "tarmng";

pub const RELI_HISTORY_MAX_DBS: u32 = 18;
#[allow(dead_code)]
static RELI_HISTORY_DB_NAME: [&str; RELI_HISTORY_MAX_DBS as usize] = [
    RELI_DB_HJOB_TRIGGER,
    RELI_DB_HJOB_SUSPENDS,
    RELI_DB_HUNIT_BASE,
    RELI_DB_HUNIT_LOAD,
    RELI_DB_HUNIT_CONFIG,
    RELI_DB_HUNIT_CGROUP,
    RELI_DB_HUNIT_CHILD,
    RELI_DB_HUNIT_PPS,
    RELI_DB_HUNIT_DEP,
    RELI_DB_HUM_NOTIFY,
    RELI_DB_HSERVICE_CONF,
    RELI_DB_HSERVICE_MNG,
    RELI_DB_HSOCKET_CONF,
    RELI_DB_HSOCKET_MNG,
    RELI_DB_HSOCKETM_FRAME,
    RELI_DB_HMOUNT_MNG,
    RELI_DB_HMOUNTM_FRAME,
    RELI_DB_HTARGET_MNG,
];
