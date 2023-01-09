use super::service_comm::ServiceUnitComm;
use libutils::process_util;
use nix::errno::Errno;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::Rc;
use sysmaster::unit::UnitActionError;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum MainState {
    Unknown,
    Known,
}

pub(super) struct ServicePid {
    comm: Rc<ServiceUnitComm>,
    data: RefCell<ServicePidData>,
}

impl ServicePid {
    pub(super) fn new(commr: &Rc<ServiceUnitComm>) -> ServicePid {
        ServicePid {
            comm: Rc::clone(commr),
            data: RefCell::new(ServicePidData::new()),
        }
    }

    pub(super) fn set_main(&self, pid: Pid) -> Result<(), Errno> {
        if let Some(p) = self.main() {
            if p == pid {
                return Ok(());
            }

            self.unwatch_main();
        }
        self.data.borrow_mut().set_main(pid)
    }

    pub(super) fn reset_main(&self) {
        self.data.borrow_mut().reset_main()
    }

    pub(super) fn update_main(&self, pid: Option<Pid>) {
        if let Some(id) = pid {
            let _ = self.set_main(id);
        } else {
            self.reset_main();
        }
    }

    pub(super) fn unwatch_main(&self) {
        if let Some(pid) = self.main() {
            if let Some(u) = self.comm.owner() {
                self.comm.um().child_unwatch_pid(u.id(), pid)
            }
            self.data.borrow_mut().reset_main();
        }
    }

    pub(super) fn set_control(&self, pid: Pid) {
        self.data.borrow_mut().set_control(pid)
    }

    pub(super) fn reset_control(&self) {
        self.data.borrow_mut().reset_control()
    }

    pub(super) fn update_control(&self, pid: Option<Pid>) {
        if let Some(id) = pid {
            self.set_control(id);
        } else {
            self.reset_control();
        }
    }

    pub(super) fn unwatch_control(&self) {
        if let Some(pid) = self.control() {
            if let Some(u) = self.comm.owner() {
                self.comm.um().child_unwatch_pid(u.id(), pid)
            }
            self.data.borrow_mut().reset_control();
        }
    }

    pub(super) fn main(&self) -> Option<Pid> {
        self.data.borrow().main()
    }

    pub(super) fn control(&self) -> Option<Pid> {
        self.data.borrow().control()
    }

    pub(super) fn main_alive(&self) -> Result<bool, UnitActionError> {
        self.data.borrow().main_alive()
    }
}

struct ServicePidData {
    main: Option<Pid>,
    state: MainState,
    control: Option<Pid>,
}

// the declaration "pub(self)" is for identification only.
impl ServicePidData {
    pub(self) fn new() -> ServicePidData {
        ServicePidData {
            main: None,
            state: MainState::Unknown,
            control: None,
        }
    }

    pub(self) fn set_main(&mut self, pid: Pid) -> Result<(), Errno> {
        if pid < Pid::from_raw(1) {
            return Err(Errno::EINVAL);
        }
        self.main = Some(pid);
        self.state = MainState::Known;
        Ok(())
    }

    pub(self) fn reset_main(&mut self) {
        self.main = None;
    }

    pub(self) fn set_control(&mut self, pid: Pid) {
        self.control = Some(pid);
    }

    pub(self) fn reset_control(&mut self) {
        self.control = None;
    }

    pub(self) fn main(&self) -> Option<Pid> {
        self.main.as_ref().cloned()
    }

    pub(self) fn control(&self) -> Option<Pid> {
        self.control.as_ref().cloned()
    }

    pub(self) fn main_alive(&self) -> Result<bool, UnitActionError> {
        match self.state {
            MainState::Unknown => Err(UnitActionError::UnitActionEAgain),
            MainState::Known => {
                if self.main.is_none() {
                    return Ok(false);
                }

                Ok(process_util::alive(self.main.unwrap()))
            }
        }
    }
}
