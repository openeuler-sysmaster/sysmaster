//! Convert the command request into the corresponding execution action
use super::{
    sys_comm, unit_comm, CommandRequest, CommandResponse, MngrComm, RequestData, SysComm, UnitComm,
    UnitFile,
};

use http::StatusCode;
use libutils::Result;
use std::io::Error;
use std::rc::Rc;

/// CMD error
pub enum ExecCmdErrno {
    /// invalid input
    Input,
    /// not existed
    NotExisted,
    /// Internal error
    Internal,
    /// not supported
    NotSupported,
}

impl From<ExecCmdErrno> for String {
    fn from(errno: ExecCmdErrno) -> Self {
        match errno {
            ExecCmdErrno::Input => "Invalid input".into(),
            ExecCmdErrno::NotExisted => "No such file or directory".into(),
            ExecCmdErrno::Internal => "Unexpected internal error".into(),
            ExecCmdErrno::NotSupported => "Unsupported action".into(),
        }
    }
}

pub(crate) trait Executer {
    /// deal Command，return Response
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse;
}

/// ExecuterAction
pub trait ExecuterAction {
    /// start the unit_name
    fn start(&self, unit_name: &str) -> Result<(), ExecCmdErrno>;
    /// stop the unit_name
    fn stop(&self, unit_name: &str) -> Result<(), ExecCmdErrno>;
    /// show the status of unit_name
    fn status(&self, unit_name: &str) -> Result<String, ExecCmdErrno>;
    /// suspend host
    fn suspend(&self) -> Result<i32>;
    /// poweroff host
    fn poweroff(&self) -> Result<i32>;
    /// reboot host
    fn reboot(&self) -> Result<i32>;
    /// halt host
    fn halt(&self) -> Result<i32>;
    /// disable unit_name
    fn disable(&self, unit_name: &str) -> Result<(), Error>;
    /// enable unit_name
    fn enable(&self, unit_name: &str) -> Result<(), Error>;
}

/// Depending on the type of request
pub(crate) fn dispatch<T>(cmd: CommandRequest, manager: Rc<T>) -> CommandResponse
where
    T: ExecuterAction,
{
    println!("commandRequest :{:?}", cmd);
    let res = match cmd.request_data {
        Some(RequestData::Ucomm(param)) => param.execute(manager),
        Some(RequestData::Mcomm(param)) => param.execute(manager),
        Some(RequestData::Syscomm(param)) => param.execute(manager),
        Some(RequestData::Ufile(param)) => param.execute(manager),
        _ => CommandResponse::default(),
    };
    println!("CommandResponse :{:?}", res);
    res
}

impl Executer for UnitComm {
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse {
        let ret = match self.action() {
            unit_comm::Action::Status => manager.status(&self.unitname),
            unit_comm::Action::Start => match manager.start(&self.unitname) {
                Ok(()) => Ok(String::new()),
                Err(e) => Err(e),
            },
            unit_comm::Action::Stop => match manager.stop(&self.unitname) {
                Ok(()) => Ok(String::new()),
                Err(e) => Err(e),
            },
            _ => todo!(),
        };
        match ret {
            Ok(m) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                message: m,
            },
            Err(e) => {
                let action_str = match self.action() {
                    unit_comm::Action::Status => String::from("get status of "),
                    unit_comm::Action::Start => String::from("start "),
                    unit_comm::Action::Stop => String::from("stop "),
                    _ => String::from("process"),
                };
                let error_message = String::from("Failed to ")
                    + &action_str
                    + &self.unitname
                    + ": "
                    + &String::from(e);
                CommandResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                    message: error_message,
                }
            }
        }
    }
}

impl Executer for MngrComm {
    fn execute(self, _manager: Rc<impl ExecuterAction>) -> CommandResponse {
        todo!()
    }
}

impl Executer for SysComm {
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse {
        let ret = match self.action() {
            sys_comm::Action::Hibernate => manager.suspend(),
            sys_comm::Action::Suspend => manager.suspend(),
            sys_comm::Action::Halt => manager.halt(),
            sys_comm::Action::Poweroff => manager.poweroff(),
            sys_comm::Action::Shutdown => manager.poweroff(),
            sys_comm::Action::Reboot => manager.reboot(),
        };
        match ret {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                ..Default::default()
            },
            Err(_e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                message: String::from("error."),
            },
        }
    }
}

impl Executer for UnitFile {
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse {
        let ret = match self.action() {
            super::unit_file::Action::Enable => manager.enable(&self.unitname),
            super::unit_file::Action::Disable => manager.disable(&self.unitname),
            _ => todo!(),
        };
        match ret {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                ..Default::default()
            },
            Err(_e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                message: String::from("error."),
            },
        }
    }
}
