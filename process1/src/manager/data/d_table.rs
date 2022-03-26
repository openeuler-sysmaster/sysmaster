use std::rc::Rc;
use std::cell::RefCell;
use crate::manager::table::{Table, TableSubscribe};
use super::unit_config::{UnitConfig};




#[derive(Debug)]
pub struct DataManager {
    tables:(
        RefCell<Table<String, UnitConfig>>, // unit-config
    ),
}

impl DataManager {
    pub(in crate::manager) fn new() -> DataManager {
        DataManager{
            tables:(
                RefCell::new(Table::new()),
            )
        }
    }

    pub(in crate::manager) fn insert_unit_config(&self, u_name:String, u_config:UnitConfig) -> Option<UnitConfig> {
        let mut table = self.tables.0.borrow_mut();
        table.insert(u_name, u_config)
    }

    pub(in crate::manager) fn remove_unit_config(&mut self, u_name:&String) -> Option<UnitConfig> {
        let mut table = self.tables.0.borrow_mut();
        table.remove(u_name)
    }

    pub(in crate::manager) fn register_unit_config(&mut self, name:String, subscriber:Rc<dyn TableSubscribe<String, UnitConfig>>) -> Option<Rc<dyn TableSubscribe<String, UnitConfig>>> {
        let mut table = self.tables.0.borrow_mut();
        table.subscribe(name, subscriber)
    }
}
