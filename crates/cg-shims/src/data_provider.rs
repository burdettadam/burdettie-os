//! CGDataProvider / CGDataConsumer stubs.

use libsystem_shims::table::ShimTable;

static CG_DATA_PROVIDER_SENTINEL: u64 = 0x4347_4450_0001;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(
        table,
        (
            "_CGDataProviderCreateWithData",
            shim_cg_dp_create as *const () as u64,
            "CGDataProviderCreateWithData (stub → sentinel)"
        ),
        (
            "_CGDataProviderCreateWithCFData",
            shim_cg_dp_create_cf as *const () as u64,
            "CGDataProviderCreateWithCFData (stub → sentinel)"
        ),
        (
            "_CGDataProviderCreateWithURL",
            shim_cg_dp_create_cf as *const () as u64,
            "CGDataProviderCreateWithURL (stub → sentinel)"
        ),
        (
            "_CGDataProviderCopyData",
            shim_cg_dp_null as *const () as u64,
            "CGDataProviderCopyData (stub → NULL)"
        ),
        (
            "_CGDataProviderRetain",
            shim_cg_dp_retain as *const () as u64,
            "CGDataProviderRetain (stub → same)"
        ),
        (
            "_CGDataProviderRelease",
            shim_cg_dp_release as *const () as u64,
            "CGDataProviderRelease (stub → void)"
        )
    );
}

pub extern "C" fn shim_cg_dp_create(_data: u64, _size: u64, _release: u64, _info: u64) -> u64 {
    &CG_DATA_PROVIDER_SENTINEL as *const u64 as u64
}
pub extern "C" fn shim_cg_dp_create_cf(_a: u64) -> u64 {
    &CG_DATA_PROVIDER_SENTINEL as *const u64 as u64
}
pub extern "C" fn shim_cg_dp_null(_a: u64) -> u64 {
    0
}
pub extern "C" fn shim_cg_dp_retain(p: u64) -> u64 {
    p
}
pub extern "C" fn shim_cg_dp_release(_p: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn data_provider_stubs_registered() {
        let mut table = ShimTable::new();
        install(&mut table);
        assert!(table.resolve("_CGDataProviderCreateWithData").is_some());
        assert!(table.resolve("_CGDataProviderRelease").is_some());
    }
}
