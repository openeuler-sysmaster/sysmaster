use super::service_comm::ServiceComm;
use super::service_pid::ServicePid;
use nix::unistd::Pid;
use process1::manager::{ExecCommand, ExecFlags, ExecParameters};
use std::error::Error;
use std::rc::Rc;

pub(super) struct ServiceSpawn {
    comm: Rc<ServiceComm>,
    pid: Rc<ServicePid>,
}

impl ServiceSpawn {
    pub(super) fn new(commr: &Rc<ServiceComm>, pidr: &Rc<ServicePid>) -> ServiceSpawn {
        ServiceSpawn {
            comm: Rc::clone(commr),
            pid: Rc::clone(pidr),
        }
    }

    pub(super) fn start_service(
        &self,
        cmdline: &ExecCommand,
        _time_out: u64,
        ec_flags: ExecFlags,
    ) -> Result<Pid, Box<dyn Error>> {
        let mut params = ExecParameters::new();
        if let Some(pid) = self.pid.main() {
            params.add_env("MAINPID", format!("{}", pid));
        }

        let unit = self.comm.unit();
        let um = self.comm.um();
        unit.prepare_exec()?;

        if ec_flags.contains(ExecFlags::PASS_FDS) {
            params.insert_fds(self.collect_socket_fds());
        }
        log::debug!("begin to exec spawn");
        match um.exec_spawn(&unit, cmdline, &params) {
            Ok(pid) => {
                um.child_watch_pid(pid, unit.get_id());
                Ok(pid)
            }
            Err(e) => {
                log::error!("failed to start service: {}, error:{:?}", unit.get_id(), e);
                Err(format!("spawn exec return error").into())
            }
        }
    }

    fn collect_socket_fds(&self) -> Vec<i32> {
        self.comm.um().collect_socket_fds(self.comm.unit().get_id())
    }
}