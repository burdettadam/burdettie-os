use crate::table::ShimTable;

macro_rules! shim_group {
    ($name:ident, $label:literal) => {
        pub struct $name;

        impl $name {
            pub fn install(_table: &mut ShimTable) {
                log::debug!(concat!(
                    $label,
                    ": Unix-backed shims unavailable on this host"
                ));
            }
        }
    };
}

shim_group!(DarwinCompatShims, "DarwinCompatShims");
shim_group!(DispatchShims, "DispatchShims");
shim_group!(MachShims, "MachShims");
shim_group!(PthreadShims, "PthreadShims");
shim_group!(SocketShims, "SocketShims");

pub struct LifecycleShims;

impl LifecycleShims {
    pub fn install(_table: &mut ShimTable) {
        log::debug!("LifecycleShims: Unix-backed shims unavailable on this host");
    }

    pub fn reset() {}
}

pub struct TlsShims;

impl TlsShims {
    pub fn install(_table: &mut ShimTable) {
        log::debug!("TlsShims: Unix-backed shims unavailable on this host");
    }

    pub fn reset() {}
}
