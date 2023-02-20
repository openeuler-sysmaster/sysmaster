//! sysmaster-core bin
mod error;
mod job;
mod manager;
///

/// dependency:
///    [manager]
///       ↑
///[reliability] → [unit   job  plugin]
///            ↖  ↗
///            [butil]
///
// mount not to be here;
mod mount;
mod plugin;
mod unit;
mod utils;

#[macro_use]
extern crate lazy_static;
use crate::error::*;
use crate::manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};
use crate::mount::setup;
use libc::{c_int, prctl, PR_SET_CHILD_SUBREAPER};
use libutils::logger::{self};
use log::{self};
use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::unistd::{self};
use std::convert::TryFrom;
use std::env::{self};
use std::ffi::CString;
use sysmaster::rel;

fn main() -> Result<()> {
    logger::init_log_with_console("sysmaster", log::LevelFilter::Debug);
    log::info!("sysmaster running in system mode.");

    // temporary annotation for repeat mount

    // mount_setup::mount_setup_early().map_err(|e| {
    //     log::error!("failed to mount early mount point, errno: {}", e);
    //     format!("failed to mount early mount point, errno: {}", e)
    // })?;

    setup::mount_setup()?;

    rel::reli_dir_prepare().context(IoSnafu)?;
    let switch = rel::reli_debug_get_switch();
    log::info!("sysmaster initialize with switch: {}.", switch);

    initialize_runtime(switch)?;

    let args: Vec<String> = env::args().collect();
    let manager = Manager::new(Mode::System, Action::Run);

    // enable clear, mutex with install_crash_handler
    if !switch {
        manager.debug_clear_restore();
        log::info!("debug: clear data restored.");
    }

    manager.setup_cgroup()?;

    // startup
    manager.startup()?;

    // main loop
    let ret = manager.main_loop();
    log::info!("sysmaster end its main loop with result: {:?}", ret);

    // get result
    let reexec = ret.map_or(false, |ree| ree);

    // re-exec
    if reexec {
        do_reexecute(&args);
    }

    Ok(())
}

fn initialize_runtime(switch: bool) -> Result<()> {
    if switch {
        install_crash_handler();
        log::info!("install crash handler.");
    }

    #[cfg(feature = "linux")]
    setup::mount_cgroup_controllers().map_err(|_| Error::Other {
        msg: "mount cgroup controllers failed: {e}".to_string(),
    })?;

    set_child_reaper();

    Ok(())
}

fn set_child_reaper() {
    let ret = unsafe { prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };

    if ret < 0 {
        log::warn!("failed to set child reaper, errno: {}", ret);
    }
}

fn do_reexecute(args: &Vec<String>) {
    let args_size = args.len().max(MANAGER_ARGS_SIZE_MAX);

    // build default arg
    let (cmd, argv) = execarg_build_default();
    assert!(argv.len() <= args_size);

    // action
    if let Err(e) = unistd::execv(&cmd, &argv) {
        log::info!("execute failed, with arg{:?} result {:?}", argv, e);
    }
}

fn install_crash_handler() {
    let signals = vec![
        Signal::SIGSEGV,
        Signal::SIGILL,
        Signal::SIGFPE,
        Signal::SIGBUS,
        Signal::SIGQUIT,
        Signal::SIGABRT,
        Signal::SIGSYS,
    ];
    let handler = SigHandler::Handler(crash);
    let flags = SaFlags::SA_NODEFER;
    let action = SigAction::new(handler, flags, SigSet::empty());
    for &signal in signals.iter() {
        unsafe {
            signal::sigaction(signal, &action).expect("failed to set signal handler for crash")
        };
    }
}

extern "C" fn crash(signo: c_int) {
    let _signal = Signal::try_from(signo).unwrap(); // debug

    // default
    let (cmd, argv) = execarg_build_default();
    if let Err(_e) = unistd::execv(&cmd, &argv) {
        // debug
    }
}

fn execarg_build_default() -> (CString, Vec<CString>) {
    let mut argv = Vec::new();

    // current execute path
    let path = env::current_exe().unwrap();
    let cmd = CString::new(path.to_str().unwrap()).unwrap();
    argv.push(cmd.clone());

    // return
    (cmd, argv)
}
