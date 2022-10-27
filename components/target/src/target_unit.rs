//! TargetUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.
use super::target_base::{LOG_LEVEL, PLUGIN_NAME};
use super::target_comm::TargetUnitComm;
use super::target_mng::TargetMng;
use process1::manager::{
    UnitActiveState, UnitDependencyMask, UnitManager, UnitMngUtil, UnitObj, UnitRelationAtom,
    UnitRelations, UnitSubClass,
};
use process1::{ReStation, Reliability};
use std::{path::PathBuf, rc::Rc};
use utils::logger;

struct Target {
    comm: Rc<TargetUnitComm>,
    mng: Rc<TargetMng>,
}

impl ReStation for Target {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.mng.db_map();
    }

    fn db_insert(&self) {
        self.mng.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        // do nothing now
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now
    }
}

impl Target {
    fn new() -> Target {
        let _comm = Rc::new(TargetUnitComm::new());
        Target {
            comm: Rc::clone(&_comm),
            mng: Rc::new(TargetMng::new(&_comm)),
        }
    }

    pub(self) fn add_default_dependencies(&self) {
        let unit = self.comm.unit();
        log::debug!("add default dependencies for target[{}]", unit.id());
        if !unit.default_dependencies() {
            return;
        }
        let um = self.comm.um();
        let deps = um.get_dependency_list(
            unit.id(),
            UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue,
        );
        for _u in deps {
            if !_u.default_dependencies() {
                continue;
            }

            if um.unit_has_dependecy(unit.id(), UnitRelationAtom::UnitAtomBefore, _u.id()) {
                continue;
            }

            let e = um.unit_add_dependency(
                unit.id(),
                UnitRelations::UnitAfter,
                _u.id(),
                true,
                UnitDependencyMask::UnitDependencyDefault,
            );
            if e.is_err() {
                log::error!("add default dependencies error {:?}", e);
                return;
            }
        }
    }
}

impl UnitObj for Target {
    fn load(&self, _conf_str: Vec<PathBuf>) -> utils::Result<(), Box<dyn std::error::Error>> {
        //todo add default dependency funnction need add
        log::debug!("load for target");
        self.add_default_dependencies();
        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.to_unit_state()
    }

    fn attach_unit(&self, unit: Rc<process1::manager::Unit>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        //if current state is not valid, just return.
        self.mng.start_check()?;

        self.mng.start_action(true);
        Ok(())
    }

    fn stop(&self, force: bool) -> utils::Result<(), process1::manager::UnitActionError> {
        if !force {
            self.mng.stop_check()?;
        }

        self.mng.stop_action(true);
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(
        &self,
        _pid: nix::unistd::Pid,
        _code: i32,
        _status: nix::sys::signal::Signal,
    ) {
    }

    fn reset_failed(&self) {}
}

impl UnitSubClass for Target {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
        Box::new(*self)
    }
}

impl UnitMngUtil for Target {
    fn attach_um(&self, _um: Rc<UnitManager>) {
        self.comm.attach_um(_um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for Target {
    fn default() -> Self {
        Target::new()
    }
}

use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(Target, Target::default, PLUGIN_NAME, LOG_LEVEL);
