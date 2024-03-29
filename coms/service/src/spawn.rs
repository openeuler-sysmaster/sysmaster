// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use crate::mng::RunningData;

use super::comm::ServiceUnitComm;
use super::config::ServiceConfig;
use super::pid::ServicePid;
use super::rentry::ServiceType;
use nix::unistd::Pid;
use std::env;
use std::rc::Rc;
use sysmaster::error::*;
use sysmaster::exec::{ExecCommand, ExecContext, ExecFlags, ExecParameters};

pub(super) struct ServiceSpawn {
    comm: Rc<ServiceUnitComm>,
    pid: Rc<ServicePid>,
    config: Rc<ServiceConfig>,
    exec_ctx: Rc<ExecContext>,
    rd: Rc<RunningData>,
}

impl ServiceSpawn {
    pub(super) fn new(
        commr: &Rc<ServiceUnitComm>,
        pidr: &Rc<ServicePid>,
        configr: &Rc<ServiceConfig>,
        exec_ctx: &Rc<ExecContext>,
        rd: &Rc<RunningData>,
    ) -> ServiceSpawn {
        ServiceSpawn {
            comm: Rc::clone(commr),
            pid: Rc::clone(pidr),
            config: configr.clone(),
            exec_ctx: exec_ctx.clone(),
            rd: rd.clone(),
        }
    }

    pub(super) fn start_service(
        &self,
        cmdline: &ExecCommand,
        time_out: u64,
        ec_flags: ExecFlags,
    ) -> Result<Pid> {
        let mut params = ExecParameters::new();
        params.set_exec_flags(ec_flags);
        params.set_nonblock(self.config.config_data().borrow().Service.NonBlocking);

        params.add_env(
            "PATH",
            env::var("PATH").unwrap_or_else(|_| {
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()
            }),
        );

        if let Some(pid) = self.pid.main() {
            params.add_env("MAINPID", format!("{pid}"));
        }
        let unit = match self.comm.owner() {
            None => {
                return Err("spawn exec return error".to_string().into());
            }
            Some(v) => v,
        };
        let um = self.comm.um();
        unit.prepare_exec()?;

        self.rd.enable_timer(time_out)?;

        if ec_flags.contains(ExecFlags::PASS_FDS) {
            params.insert_fds(self.collect_socket_fds());
        }

        if self.config.service_type() == ServiceType::Notify
            || self.config.config_data().borrow().Service.WatchdogSec > 0
        {
            let notify_sock = um.notify_socket().unwrap();
            log::debug!("add NOTIFY_SOCKET env: {}", notify_sock.to_str().unwrap());
            params.add_env("NOTIFY_SOCKET", notify_sock.to_str().unwrap().to_string());
            params.set_notify_sock(notify_sock);
        }

        if let Err(e) = params.add_user(self.config.config_data().borrow().Service.User.clone()) {
            log::error!(
                "Failed to add user to execute parameters: {}",
                e.to_string()
            );
            return Err(e);
        }

        if let Err(e) = params.add_group(self.config.config_data().borrow().Service.Group.clone()) {
            log::error!(
                "Failed to add group to execute parameters: {}",
                e.to_string()
            );
            return Err(e);
        }

        if let Err(e) = params.add_umask(self.config.config_data().borrow().Service.UMask.clone()) {
            log::error!(
                "Failed to add umask to execute parameters: {}",
                e.to_string()
            );
            return Err(e);
        }

        if let Err(e) = params.add_working_directory(
            self.config
                .config_data()
                .borrow()
                .Service
                .WorkingDirectory
                .clone(),
        ) {
            log::error!("Failed to add working directory: {}", e.to_string());
            return Err(e);
        }

        params.set_watchdog_usec(self.watchdog_timer());

        log::debug!("begin to exec spawn");
        match um.exec_spawn(unit.id(), cmdline, &params, self.exec_ctx.clone()) {
            Ok(pid) => {
                um.child_watch_pid(unit.id(), pid);
                Ok(pid)
            }
            Err(e) => {
                log::error!("failed to start service: {}, error:{:?}", unit.id(), e);
                Err("spawn exec return error".to_string().into())
            }
        }
    }

    fn collect_socket_fds(&self) -> Vec<i32> {
        self.comm.um().collect_socket_fds(&self.comm.get_owner_id())
    }

    fn watchdog_timer(&self) -> u64 {
        self.config.config_data().borrow().Service.WatchdogSec
    }
}
