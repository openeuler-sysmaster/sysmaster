use super::execute::{ExecCmdError, ExecCommand, ExecParameters, ExecSpawn};
use super::job::{JobAffect, JobConf, JobKind, JobManager};
use super::unit_base::{JobMode, UnitLoadState, UnitRelationAtom};
use super::unit_datastore::UnitDb;
use super::unit_entry::{Unit, UnitObj, UnitX};
use super::unit_runtime::UnitRT;
use super::{UnitActionError, UnitType};
use crate::manager::data::{DataManager, UnitState};
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::{MngErrno, UnitRelations};
use event::{EventState, Events, Source};
use nix::unistd::Pid;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;
use unit_load::UnitLoad;

//#[derive(Debug)]
pub(in crate::manager) struct UnitManagerX {
    sub_name: String, // key for table-subscriber: UnitState
    data: Rc<UnitManager>,
}

impl UnitManagerX {
    pub(in crate::manager) fn new(dmr: &Rc<DataManager>, eventr: &Rc<Events>) -> UnitManagerX {
        let umx = UnitManagerX {
            sub_name: String::from("UnitManagerX"),
            data: UnitManager::new(dmr, eventr),
        };
        umx.register(dmr);
        umx
    }

    pub(in crate::manager) fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    pub(in crate::manager) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    pub(in crate::manager) fn child_dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.data.db.child_dispatch_sigchld()
    }

    pub(in crate::manager) fn dispatch_load_queue(&self) {
        self.data.rt.dispatch_load_queue()
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        let register_result = dm.register_unit_state(&self.sub_name, subscriber);
        if let Some(_r) = register_result {
            log::info!("TableSubcribe for {} is already register", &self.sub_name);
        } else {
            log::info!("register  TableSubcribe for {}  sucessfull", &self.sub_name);
        }
    }
}

//#[derive(Debug)]
pub struct UnitManager {
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,
    load: UnitLoad,
    jm: JobManager,
    exec: ExecSpawn,
    events: Rc<Events>,
}

// the declaration "pub(self)" is for identification only.
impl UnitManager {
    pub fn child_watch_pid(&self, pid: Pid, id: &str) {
        self.db.child_add_watch_pid(pid, id)
    }

    pub fn child_watch_all_pids(&self, id: &str) {
        self.db.child_watch_all_pids(id)
    }

    pub fn child_unwatch_pid(&self, pid: Pid) {
        self.db.child_unwatch_pid(pid)
    }

    pub fn exec_spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &ExecParameters,
    ) -> Result<Pid, ExecCmdError> {
        self.exec.spawn(unit, cmdline, params)
    }

    // load the unit for reference name
    pub fn load_unit_success(&self, name: &str) -> bool {
        if let Some(_unit) = self.load_unit(name) {
            return true;
        }

        return false;
    }

    // load the unit of the dependency UnitType
    pub fn load_related_unit_success(&self, name: &str, unit_type: UnitType) -> bool {
        let stem_name = Path::new(name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{}.{}", stem_name, String::from(unit_type));

        if let Some(_unit) = self.load_unit(&relate_name) {
            return true;
        }

        return false;
    }

    // check the unit active state of of reference name
    pub fn unit_enabled(&self, name: &str) -> Result<(), UnitActionError> {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return Err(UnitActionError::UnitActionENoent);
        };

        if u.load_state() != UnitLoadState::UnitLoaded {
            log::error!("related service unit: {} is not loaded", name);
            return Err(UnitActionError::UnitActionENoent);
        }

        if u.activeted() {
            return Err(UnitActionError::UnitActionEBusy);
        }

        return Ok(());
    }

    pub fn register(&self, source: Rc<dyn Source>) {
        self.events.add_source(source).unwrap();
    }

    pub fn enable(&self, source: Rc<dyn Source>, state: EventState) {
        self.events.set_enabled(source, state).unwrap();
    }

    pub fn unregister(&self, source: Rc<dyn Source>) {
        self.events.del_source(source).unwrap();
    }

    // check if there is already a stop job in process
    pub fn has_stop_job(&self, name: &str) -> bool {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return false;
        };

        self.jm.has_stop_job(&u)
    }

    // return the fds that trigger the unit {name};
    pub fn collect_socket_fds(&self, name: &str) -> Vec<i32> {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggeredBy);
        let mut fds = Vec::new();
        for dep in deps.iter() {
            if dep.unit_type() != UnitType::UnitSocket {
                continue;
            }

            fds.extend(dep.collect_fds())
        }

        fds
    }

    // check the unit that will be triggerd by {name} is in active or activating state
    pub fn relation_active_or_pending(&self, name: &str) -> bool {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggers);
        let mut pending: bool = false;
        for dep in deps.iter() {
            if dep.active_or_activating() {
                pending = true;
                break;
            }
        }

        pending
    }

    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unit(name) {
            log::debug!("load unit success, send to job manager");
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::JobStart),
                JobMode::JobReplace,
                &mut JobAffect::new(false),
            )?;
            log::debug!("job exec success");
            Ok(())
        } else {
            return Err(MngErrno::MngErrInternel);
        }
    }

    pub(self) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unit(name) {
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::JobStop),
                JobMode::JobReplace,
                &mut JobAffect::new(false),
            )?;
            Ok(())
        } else {
            return Err(MngErrno::MngErrInternel);
        }
    }

    pub(self) fn new(dmr: &Rc<DataManager>, eventr: &Rc<Events>) -> Rc<UnitManager> {
        let _db = Rc::new(UnitDb::new());
        let _rt = Rc::new(UnitRT::new(&_db));
        let um = Rc::new(UnitManager {
            load: UnitLoad::new(dmr, &_db, &_rt),
            db: Rc::clone(&_db),
            rt: Rc::clone(&_rt),
            jm: JobManager::new(&_db, eventr),
            exec: ExecSpawn::new(),
            events: eventr.clone(),
        });
        um.load.set_um(&um);
        um
    }

    fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.load.load_unit(name)
    }
}

impl TableSubscribe<String, UnitState> for UnitManager {
    fn notify(&self, op: &TableOp<String, UnitState>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_states(name, config),
            TableOp::TableRemove(name, _) => self.remove_states(name),
        }
    }
}

impl UnitManager {
    fn insert_states(&self, source: &str, state: &UnitState) {
        log::debug!("insert unit states source {}, state: {:?}", source, state);
        let unitx = if let Some(u) = self.db.units_get(source) {
            u
        } else {
            return;
        };

        if let Err(_e) = self.jm.try_finish(&unitx, state.os, state.ns, state.flags) {
            // debug
        }

        for other in self
            .db
            .dep_gets_atom(&unitx, UnitRelationAtom::UnitAtomTriggeredBy)
        {
            other.trigger(&unitx);
        }
    }

    fn remove_states(&self, _source: &str) {
        todo!();
    }
}

pub trait UnitMngUtil {
    fn attach(&self, um: Rc<UnitManager>);
}

pub trait UnitSubClass: UnitObj + UnitMngUtil {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj>;
}

// #[macro_use]
// use crate::unit_name_to_type;
//unitManger composition of units with hash map
#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
        // method for create the unit instance
        #[no_mangle]
        pub fn __unit_obj_create() -> *mut dyn $crate::manager::UnitSubClass {
            logger::init_log_with_default($name, $level);
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::UnitSubClass> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

mod unit_load {
    use super::UnitManager;
    use crate::manager::data::{DataManager, UnitDepConf};
    use crate::manager::table::{TableOp, TableSubscribe};
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_base::{self, UnitType};
    use crate::manager::unit::unit_datastore::UnitDb;
    use crate::manager::unit::unit_entry::UnitX;
    use crate::manager::unit::unit_runtime::UnitRT;
    use crate::plugin::Plugin;
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};

    //#[derive(Debug)]
    pub(super) struct UnitLoad {
        sub_name: String, // key for table-subscriber: UnitDepConf
        data: Rc<UnitLoadData>,
    }

    impl UnitLoad {
        pub(super) fn new(dmr: &Rc<DataManager>, dbr: &Rc<UnitDb>, rtr: &Rc<UnitRT>) -> UnitLoad {
            let load = UnitLoad {
                sub_name: String::from("UnitLoad"),
                data: Rc::new(UnitLoadData::new(dmr, dbr, rtr)),
            };
            load.register(dmr);
            load
        }

        pub(super) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            self.data.load_unit(name)
        }

        pub(super) fn set_um(&self, um: &Rc<UnitManager>) {
            self.data.set_um(um);
        }

        fn register(&self, dm: &DataManager) {
            let subscriber = Rc::clone(&self.data);
            let ret = dm.register_ud_config(&self.sub_name, subscriber);
            if let Some(_r) = ret {
                log::info!("TableSubcribe for {} is already register", &self.sub_name);
            } else {
                log::info!("register  TableSubcribe for {}  sucessfull", &self.sub_name);
            }
        }
    }

    //#[derive(Debug)]
    struct UnitLoadData {
        // associated objects
        dm: Rc<DataManager>,
        um: RefCell<Weak<UnitManager>>,
        db: Rc<UnitDb>,
        rt: Rc<UnitRT>,

        // owned objects
        file: Rc<UnitFile>,
    }

    // the declaration "pub(self)" is for identification only.
    impl UnitLoadData {
        pub(self) fn new(
            dmr: &Rc<DataManager>,
            dbr: &Rc<UnitDb>,
            rtr: &Rc<UnitRT>,
        ) -> UnitLoadData {
            log::debug!("UnitLoadData db count is {}", Rc::strong_count(dbr));
            let file = Rc::new(UnitFile::new());
            UnitLoadData {
                dm: Rc::clone(dmr),
                um: RefCell::new(Weak::new()),
                db: Rc::clone(dbr),
                rt: Rc::clone(rtr),
                file: Rc::clone(&file),
            }
        }

        pub(self) fn prepare_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            match self.try_new_unit(name) {
                Some(unit) => {
                    self.db.units_insert(name.to_string(), Rc::clone(&unit));
                    self.rt.push_load_queue(Rc::clone(&unit));
                    Some(Rc::clone(&unit))
                }
                None => {
                    log::error!(
                        "create unit obj failed,name is {},{}",
                        name,
                        Rc::strong_count(&self.db)
                    );
                    return None;
                }
            }
        }

        pub(self) fn push_dep_unit_into_load_queue(&self, name: &str) -> Option<Rc<UnitX>> {
            if let Some(unit) = self.db.units_get(name) {
                return Some(Rc::clone(&unit));
            };
            let unit = self.prepare_unit(name);
            unit
        }

        pub(self) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            if let Some(unit) = self.db.units_get(name) {
                return Some(Rc::clone(&unit));
            };
            let unit = self.prepare_unit(name);
            let u = if let Some(u) = unit {
                u
            } else {
                return None;
            };
            log::info!("begin dispatch unit in  load queue");
            self.rt.dispatch_load_queue();
            Some(Rc::clone(&u))
        }

        pub(self) fn set_um(&self, um: &Rc<UnitManager>) {
            self.um.replace(Rc::downgrade(um));
        }

        fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            let unit_type = unit_base::unit_name_to_type(name);
            if unit_type == UnitType::UnitTypeInvalid {
                return None;
            }

            log::info!(
                "begin create obj for  type {}, name {} by plugin",
                unit_type.to_string(),
                name
            );
            let plugins = Plugin::get_instance();
            let subclass = match plugins.create_unit_obj(unit_type) {
                Ok(sub) => sub,
                Err(_e) => return None,
            };

            subclass.attach(self.um.clone().into_inner().upgrade().unwrap());

            Some(Rc::new(UnitX::new(
                &self.dm,
                &self.file,
                unit_type,
                name,
                subclass.into_unitobj(),
            )))
        }
    }

    impl TableSubscribe<String, UnitDepConf> for UnitLoadData {
        fn notify(&self, op: &TableOp<String, UnitDepConf>) {
            match op {
                TableOp::TableInsert(name, config) => self.insert_udconf(name, config),
                TableOp::TableRemove(_, _) => {} // self.remove_udconf(name)
            }
        }
    }

    impl UnitLoadData {
        fn insert_udconf(&self, name: &str, config: &UnitDepConf) {
            //hash map insert return is old value,need reconstruct
            let unit = match self.db.units_get(name) {
                Some(u) => u,
                None => {
                    log::error!("create unit obj error in unit manger");
                    return;
                } // load
            };

            // dependency
            for (relation, list) in config.deps.iter() {
                for o_name in list {
                    let tmp_unit: Rc<UnitX>;
                    if let Some(o_unit) = self.push_dep_unit_into_load_queue(o_name) {
                        //此处不能直接调用unit_load，会嵌套
                        tmp_unit = Rc::clone(&o_unit);
                    } else {
                        log::error!("create unit obj error in unit manger");
                        return;
                    }

                    if let Err(_e) =
                        self.db
                            .dep_insert(Rc::clone(&unit), *relation, tmp_unit, true, 0)
                    //依赖关系插入，但是未判断是否load成功，如果unit无法load，是否应该记录依赖关系
                    {
                        // debug
                    }
                }
            }
        }

        fn remove_udconf(&self, _source: &str) {
            todo!();
        }
    }
}

#[cfg(test)]
mod tests {
    // use services::service::ServiceUnit;

    use super::*;
    use event::Events;
    use utils::logger;

    #[test]
    fn test_service_unit_load() {
        logger::init_log_with_console("test_service_unit_load", 4);
        log::info!("test");
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event);

        let unit_name = String::from("config.service");
        let unit = um.load_unit(&unit_name);

        match unit {
            Some(_unit_obj) => assert_eq!(_unit_obj.get_id(), "config.service"),
            None => println!("test unit load, not fount unit: {}", unit_name),
        };
    }

    #[test]
    fn test_service_unit_start() {
        logger::init_log_with_console("test_service_unit_start", 4);
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event);

        let unit_name = String::from("config.service");
        let unit = um.load_unit(&unit_name);

        match unit {
            Some(u) => {
                u.start();
                log::debug!("unit start end!");
                u.stop();
                log::debug!("unit stop end!");
            }
            None => println!("load unit failed"),
        }
    }

    #[test]
    fn test_units_load() {
        logger::init_log_with_console("test_units_load", 4);
        let mut unit_name_lists: Vec<String> = Vec::new();
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event);

        unit_name_lists.push("config.service".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = um.load_unit(u_name);
            match unit {
                Some(_unit_obj) => assert_eq!(_unit_obj.get_id(), u_name),
                None => println!("test unit load, not fount unit: {}", u_name),
            };
        }
    }
}
