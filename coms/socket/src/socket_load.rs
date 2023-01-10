//! socket_load mod parse the field of section Socket and add the extra dependency。
//!

use crate::socket_comm::SocketUnitComm;
use crate::socket_config::SocketConfig;
use crate::socket_rentry::PortType;
use libutils::error::Error as SocketError;
use libutils::special::{SHUTDOWN_TARGET, SOCKETS_TARGET, SYSINIT_TARGET};
use std::path::Path;
use std::{error::Error, rc::Rc};
use sysmaster::unit::{UnitActionError, UnitDependencyMask, UnitRelations, UnitType};

pub(super) struct SocketLoad {
    config: Rc<SocketConfig>,
    comm: Rc<SocketUnitComm>,
}

impl SocketLoad {
    pub(super) fn new(configr: &Rc<SocketConfig>, commr: &Rc<SocketUnitComm>) -> Self {
        SocketLoad {
            config: configr.clone(),
            comm: commr.clone(),
        }
    }

    pub(super) fn socket_add_extras(&self) -> Result<(), Box<dyn Error>> {
        log::debug!("socket add extras");
        if self.can_accept() {
            if self.config.unit_ref_target().is_none() {
                self.load_related_unit(UnitType::UnitService)?;
            }

            self.comm.owner().map(|u| {
                u.insert_two_deps(
                    UnitRelations::UnitBefore,
                    UnitRelations::UnitTriggers,
                    self.config.unit_ref_target().unwrap(),
                )
            });
        }

        self.add_default_dependencies().map_err(|_e| {
            Box::new(SocketError::Other {
                msg: "add default dependency error",
            })
        })?;

        Ok(())
    }

    pub(super) fn socket_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn load_related_unit(&self, related_type: UnitType) -> Result<(), Box<dyn Error>> {
        let unit_name = self.comm.owner().map(|u| u.id().to_string());
        let suffix = String::from(related_type);
        if suffix.is_empty() {
            return Err(format!("failed to load related unit {suffix}").into());
        }
        if unit_name.is_none() {
            return Err(format!("failed to load related unit {suffix} unit name is none").into());
        }
        let u_name = unit_name.unwrap();
        let stem_name = Path::new(&u_name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{stem_name}.{suffix}");
        self.config.set_unit_ref(relate_name)?;
        Ok(())
    }

    fn can_accept(&self) -> bool {
        if !self.config.config_data().borrow().Socket.Accept {
            return true;
        };

        self.no_accept_socket()
    }

    fn no_accept_socket(&self) -> bool {
        for port in self.config.ports().iter() {
            if port.p_type() != PortType::Socket {
                return true;
            }

            if !port.sa().can_accept() {
                return true;
            }
        }

        false
    }

    pub(self) fn add_default_dependencies(&self) -> Result<(), UnitActionError> {
        if let Some(u) = self.comm.owner() {
            log::debug!("add default dependencies for socket [{}]", u.id());
            if !u.default_dependencies() {
                return Ok(());
            }

            let um = self.comm.um();
            um.unit_add_dependency(
                u.id(),
                UnitRelations::UnitAfter,
                SOCKETS_TARGET,
                true,
                UnitDependencyMask::UnitDependencyDefault,
            )?;

            um.unit_add_two_dependency(
                u.id(),
                UnitRelations::UnitAfter,
                UnitRelations::UnitRequires,
                SYSINIT_TARGET,
                true,
                UnitDependencyMask::UnitDependencyDefault,
            )?;

            um.unit_add_two_dependency(
                u.id(),
                UnitRelations::UnitBefore,
                UnitRelations::UnitConflicts,
                SHUTDOWN_TARGET,
                true,
                UnitDependencyMask::UnitDependencyDefault,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{socket_comm::SocketUnitComm, socket_config::SocketConfig};
    use libtests::get_project_root;
    use std::rc::Rc;

    #[test]
    fn test_socket_load_parse() {
        let comm = Rc::new(SocketUnitComm::new());
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/test.socket.toml");

        let paths = vec![file_path];

        let config = SocketConfig::new(&comm);
        assert!(config.load(paths, false).is_ok());
    }
}
