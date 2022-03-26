use crate::manager::signals::ProcessExit;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::collections::{HashMap, VecDeque};
use utils::path_lookup::LookupPaths;
use crate::manager::data::*;
use super::{UnitX, unit_name_to_type};
use super::unit_new;
use std::fs;
use utils:: {time_util, path_lookup};
use nix::sys::signal::Signal;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use siphasher::sip::SipHasher24;
use walkdir::{WalkDir};
use std::hash::Hasher;
use super::unit_sets::{UnitSets};

use nix::unistd::Pid;

// #[macro_use]
// use crate::unit_name_to_type;
//unitManger composition of units with hash map

#[derive(Debug)]
pub struct UnitManager {
    dm:Rc<DataManager>,
    pub units: UnitSets,
    unit_id_map: HashMap<String, String>,
    unit_name_map: HashMap<String, String>,
    lookup_path: LookupPaths,
    last_updated_timestamp_hash: u64,
    load_queue: VecDeque<Rc<RefCell<Rc<UnitX>>>>,
    pub watch_pids: HashMap<Pid, Rc<RefCell<Rc<UnitX>>>>,
}


impl UnitManager{
    pub fn new(dm:Rc<DataManager>) -> Self{
        UnitManager {
            dm,
            units: UnitSets::new(),
            unit_id_map: HashMap::new(),
            unit_name_map: HashMap::new(),
            last_updated_timestamp_hash: 0,
            lookup_path: path_lookup::LookupPaths::new(),
            load_queue: VecDeque::new(),
            watch_pids: HashMap::new(),
        }
    }

    pub fn build_name_map(&mut self) -> bool {
	    let mut timestamp_hash_new: u64 = 0;
        if !self.lookup_paths_updated(&mut timestamp_hash_new) {
            return false;
        }

        for dir in &self.lookup_path.search_path {
            if !std::path::Path::new(&dir).exists() {
                log::warn!("dir {} is not exist", dir);
                continue;
	        }
            for entry in WalkDir::new(&dir)
                .min_depth(1)
	            .max_depth(1)
                .into_iter() {
                let entry = entry.unwrap();
                let filename = entry.file_name().to_str().unwrap().to_string();
                let file_path = entry.path().to_str().unwrap().to_string();
                if self.unit_id_map.contains_key(&filename) {
                    continue;
                }
                self.unit_id_map.insert(filename, file_path);
            }
        }
	    self.last_updated_timestamp_hash = timestamp_hash_new;
        return true;
    }

    pub fn get_unit_file_path(&self, unit_name: &str) -> Option<&String> {
        match self.unit_id_map.get(unit_name) {
            None => { return None; },
            Some(v) => {
                return Some(v);
            },
        }
    }

    fn lookup_paths_updated(&mut self, timestamp_new: &mut u64) -> bool {
        let updated: u64;
        let mut siphash24 = SipHasher24::new_with_keys(0, 0);
        for dir in &self.lookup_path.search_path {
	        match fs::metadata(&dir) {
                Ok(metadata) => match metadata.modified() {
                    Ok(time) => {
                        siphash24.write_u128(time_util::timespec_load(time));
                    },
                    _ => {
                        log::error!("failed to get mtime {}", dir);
                    },
                }
                Err(e) => {
                    log::error!("failed to get metadata of {}, err: {}", dir, e);
                }
            }
        }

        updated = siphash24.finish();
        *timestamp_new = updated;
        return updated != self.last_updated_timestamp_hash;
    }

    pub fn init_lookup_path(&mut self) {
        self.lookup_path.init_lookup_paths();
    }


    pub fn dispatch_load_queue(&mut self) {
        log::debug!("dispatch load queue");

        loop {
            match self.load_queue.pop_front() {
                None => {break},
                Some(unit) => {
                    /*match unit.borrow_mut().load(self) {
                        Ok(()) => {continue},
                        Err(e) => {
                            log::error!("load unit config failed: {}", e.to_string());
                            println!("load unit config failed: {}", e.to_string())
                        }
                    }*/
                }
            }
        }
    }

    pub fn push_load_queue(&mut self, unit: Rc<RefCell<Rc<UnitX>>>) {
        if unit.borrow().in_load_queue() {
            return;
        }
        self.load_queue.push_back(unit);
    }

    
    
    fn prepare_unit(&mut self, name: &str) -> Option<Rc<RefCell<Rc<UnitX>>>> { 
        let unit_type = unit_name_to_type(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return None;
        }

        match unit_new(Rc::clone(&self.dm), unit_type, name) {
            Ok(unit) => {
                let u = Rc::new(RefCell::new(unit));
                self.units.insert_unit(name.to_string(), u.clone());
                return Some(u.clone())
            },
            Err(_e) => {
                log::error!("create unit obj failed {:?}",_e);
                return None
            }
        };
    }

    pub fn load_unit(&mut self, name: &str) -> Option<Rc<RefCell<Rc<UnitX>>>> {
        
        if let Some(unit) = self.units.get_unit_on_name(name) {
             return Some(unit);
        }; 

        let unit = self.prepare_unit(name);
        let u = if let Some(u) = unit {
            u
        } else {
            return None;
        };
        log::info!("push new unit into load queue");
        self.push_load_queue(u.clone());
        self.dispatch_load_queue();
        Some(u.clone())
    }
    pub fn dispatch_sigchld(&mut self) ->  Result<(), Box<dyn Error>> {
        log::debug!("Dispatching sighandler waiting for pid");
        let wait_pid = Pid::from_raw(-1);
        let flags = WaitPidFlag::WNOHANG;
        let process_exit = {
            match nix::sys::wait::waitpid(wait_pid, Some(flags)) {
                Ok(wait_status) => match wait_status {
                    WaitStatus::Exited(pid, code) => {
                        ProcessExit::Status(pid, code, Signal::SIGCHLD)
                    }
                    WaitStatus::Signaled(pid, signal, _dumped_core) => {
                        ProcessExit::Status(pid, -1, signal)
                    }
                    _ => {
                        log::debug!("Ignored child signal: {:?}", wait_status);
                        return Err(format!("Ignored child signal: {:?}", wait_status).into())
                    }
                },
                Err(e) => {
                    log::error!("Error while waiting pid: {}", e);
                    return Err(format!("Error while waiting pid: {}", e).into())
                }
            }
        };

        match process_exit {
            ProcessExit::Status(pid, code, signal) => {
                match self.watch_pids.get(&pid) {
                    Some(unit) => {
                        //unit.clone().borrow_mut().sigchld_events(self, pid, code, signal);
                    }
                    None => {
                        log::debug!("not found unit obj of pid: {:?}", pid);
                        return Err(format!("not found unit obj of pid: {:?}", pid).into())
                    },
                }

                self.watch_pids.remove(&pid);
                Ok(())
            }
        }
    }

    pub fn add_watch_pid(&mut self, pid: Pid, id: &str) {
        let unit_obj = self.units.get_unit_on_name(id).unwrap();
        self.watch_pids.insert(pid, unit_obj.clone());
    }

    pub fn unwatch_pid(&mut self, pid: Pid) {
        self.watch_pids.remove(&pid);
    }
        
}



#[cfg(test)]
mod tests {
    // use services::service::ServiceUnit;

    use super::*;
    use utils::{logger};

    #[test]
    fn  test_unit_load(){
        logger::init_log_with_console("test",4);
        log::info!("test");
        let dm_manager = Rc::new(DataManager::new());
        let mut unit_manager = UnitManager::new(Rc::clone(&dm_manager));
        unit_manager.init_lookup_path();

        let unit_name = String::from("config.service");
        unit_manager.load_unit(&unit_name);

        //assert_ne!(unit_manager.units.borrow().len(), 0);

        match unit_manager.units.get_unit_on_name(&unit_name) {
            Some(_unit_obj) => println!("found unit obj {}", unit_name),
            None => println!("not fount unit: {}", unit_name),
        };

    }

    #[test]
    fn  test_unit_start(){
        let dm_manager = Rc::new(DataManager::new());
        let mut unit_manager = UnitManager::new(Rc::clone(&dm_manager));
        unit_manager.init_lookup_path();

        let unit_name = String::from("config.service");
        unit_manager.load_unit(&unit_name);

        match unit_manager.units.get_unit_on_name(&unit_name) {
            Some(_unit_obj) => println!("found unit obj {}", unit_name),
            None => println!("not fount unit: {}", unit_name),
        };
    }

}
