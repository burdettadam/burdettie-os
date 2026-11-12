use crate::config::RuntimeConfig;
use crate::tracer::{EventKind, RuntimeTracer};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const FOUNDATION_SYNTHETIC_CLASS_COUNT: usize = 98;
const FOUNDATION_SELECTOR_RESERVE: usize = 512;
const FOUNDATION_PROTOCOL_RESERVE: usize = 16;
const FOUNDATION_CATEGORY_LOAD_RESERVE: usize = 16;

static FOUNDATION_RUNTIME_TEMPLATE: OnceLock<objc_runtime::ObjcRuntime> = OnceLock::new();

#[derive(Debug, Clone, Default)]
struct FoundationRegistrationStats {
    platform_setup_time: Duration,
    template_apply_time: Duration,
    host_isa_alias_time: Duration,
    guest_compat_time: Duration,
}

/// Aggregate timing + counter stats for the ObjC registration pipeline.
#[derive(Debug, Clone, Default)]
pub struct ObjcRegistrationStats {
    pub images_scanned: usize,
    pub objc_images: usize,
    pub class_refs_seen: usize,
    pub categories_seen: usize,
    pub category_instance_methods: usize,
    pub category_class_methods: usize,
    pub selrefs_fixed: usize,
    pub foundation_classes_registered: usize,
    pub metadata_parse_time: Duration,
    pub register_image_time: Duration,
    pub category_merge_time: Duration,
    pub selref_fixup_time: Duration,
    pub foundation_registration_time: Duration,
    pub foundation_platform_setup_time: Duration,
    pub foundation_template_apply_time: Duration,
    pub foundation_host_isa_alias_time: Duration,
    pub foundation_guest_compat_time: Duration,
    pub superclass_repair_time: Duration,
}

impl ObjcRegistrationStats {
    pub fn total_time(&self) -> Duration {
        self.metadata_parse_time
            + self.register_image_time
            + self.category_merge_time
            + self.selref_fixup_time
            + self.foundation_registration_time
            + self.superclass_repair_time
    }

    pub fn record_to(&self, tracer: &RuntimeTracer) {
        tracer.record_perf_timing("objc_reg", "metadata_parse_us", self.metadata_parse_time);
        tracer.record_perf_timing("objc_reg", "register_image_us", self.register_image_time);
        tracer.record_perf_timing("objc_reg", "category_merge_us", self.category_merge_time);
        tracer.record_perf_timing("objc_reg", "selref_fixup_us", self.selref_fixup_time);
        tracer.record_perf_timing(
            "objc_reg",
            "foundation_registration_us",
            self.foundation_registration_time,
        );
        tracer.record_perf_timing(
            "objc_reg",
            "foundation_platform_setup_us",
            self.foundation_platform_setup_time,
        );
        tracer.record_perf_timing(
            "objc_reg",
            "foundation_template_apply_us",
            self.foundation_template_apply_time,
        );
        tracer.record_perf_timing(
            "objc_reg",
            "foundation_host_isa_alias_us",
            self.foundation_host_isa_alias_time,
        );
        tracer.record_perf_timing(
            "objc_reg",
            "foundation_guest_compat_us",
            self.foundation_guest_compat_time,
        );
        tracer.record_perf_timing(
            "objc_reg",
            "superclass_repair_us",
            self.superclass_repair_time,
        );
        tracer.record_perf_timing("objc_reg", "total_us", self.total_time());
        tracer.record_perf_counter("objc_reg", "images_scanned", self.images_scanned as u64);
        tracer.record_perf_counter("objc_reg", "objc_images", self.objc_images as u64);
        tracer.record_perf_counter("objc_reg", "class_refs_seen", self.class_refs_seen as u64);
        tracer.record_perf_counter("objc_reg", "categories_seen", self.categories_seen as u64);
        tracer.record_perf_counter(
            "objc_reg",
            "category_instance_methods",
            self.category_instance_methods as u64,
        );
        tracer.record_perf_counter(
            "objc_reg",
            "category_class_methods",
            self.category_class_methods as u64,
        );
        tracer.record_perf_counter("objc_reg", "selrefs_fixed", self.selrefs_fixed as u64);
        tracer.record_perf_counter(
            "objc_reg",
            "foundation_classes_registered",
            self.foundation_classes_registered as u64,
        );
    }
}

/// Run the full ObjC image-registration pipeline (phases A–E) for all images
/// in `registry`, then install the assembled runtime in the global slot.
///
/// Pass a disabled tracer to suppress event recording (e.g. on the
/// interpreted execution path where tracing is not needed).
pub fn run_registration_pipeline(
    registry: &macho_exec::ImageRegistry,
    config: &RuntimeConfig,
    tracer: &RuntimeTracer,
) -> ObjcRegistrationStats {
    let mut objc_rt = objc_runtime::ObjcRuntime::new();
    let mut image_metadata: Vec<(macho_objc::ObjcMetadata, i64)> = Vec::new();
    let mut stats = ObjcRegistrationStats::default();

    // Phase A: register classes + same-image categories
    for loaded in registry.images() {
        stats.images_scanned += 1;
        let metadata_parse_start = Instant::now();
        let metadata = macho_objc::parse_objc_metadata(&loaded.parsed, &loaded.data);
        stats.metadata_parse_time += metadata_parse_start.elapsed();
        if metadata.presence.has_objc {
            stats.objc_images += 1;
            stats.class_refs_seen += metadata.class_refs.len();
            stats.categories_seen += metadata.categories.len();
            stats.selrefs_fixed += metadata.selref_addrs.len();
            stats.category_instance_methods += metadata
                .categories
                .iter()
                .map(|cat| cat.instance_methods.len())
                .sum::<usize>();
            stats.category_class_methods += metadata
                .categories
                .iter()
                .map(|cat| cat.class_methods.len())
                .sum::<usize>();
            let register_start = Instant::now();
            objc_rt.register_image(&metadata, loaded.image.slide);
            stats.register_image_time += register_start.elapsed();
            for cls in &metadata.class_refs {
                tracer.record(
                    EventKind::ClassRegistration,
                    format!(
                        "'{}' from '{}'",
                        cls.name.as_deref().unwrap_or("<anon>"),
                        loaded.install_name,
                    ),
                );
            }
            log::info!(
                "ObjC: registered {} classes from '{}'",
                metadata.class_refs.len(),
                loaded.install_name,
            );
            image_metadata.push((metadata, loaded.image.slide));
        }
    }

    // Phase B: fix up __objc_selrefs to point at interned selectors
    for (metadata, slide) in &image_metadata {
        if !metadata.selref_addrs.is_empty() {
            tracer.record(
                EventKind::SelectorRegistration,
                format!("{} selrefs fixed up", metadata.selref_addrs.len()),
            );
            let fixup_start = Instant::now();
            objc_rt.fixup_selrefs(metadata, *slide);
            stats.selref_fixup_time += fixup_start.elapsed();
        }
    }

    register_guest_compat_classes_from_registry(&mut objc_rt, registry);

    // Phase C: register synthetic Foundation/AppKit classes
    if config.shim_set.foundation {
        let foundation_start = Instant::now();
        let foundation_stats = register_foundation_classes_with_stats(&mut objc_rt, config, tracer);
        stats.foundation_registration_time += foundation_start.elapsed();
        stats.foundation_platform_setup_time += foundation_stats.platform_setup_time;
        stats.foundation_template_apply_time += foundation_stats.template_apply_time;
        stats.foundation_host_isa_alias_time += foundation_stats.host_isa_alias_time;
        stats.foundation_guest_compat_time += foundation_stats.guest_compat_time;
        stats.foundation_classes_registered = FOUNDATION_SYNTHETIC_CLASS_COUNT;
    }

    // Phase D: merge cross-image categories from mapped memory (post-fixup).
    // Runs after Foundation registration so categories on Foundation classes
    // (e.g., NSString, NSMutableString) can find their targets.
    for (metadata, slide) in &image_metadata {
        if !metadata.categories.is_empty() {
            for cat in &metadata.categories {
                tracer.record(
                    EventKind::CategoryMerge,
                    format!(
                        "category '{}' ({} instance, {} class methods)",
                        cat.name.as_deref().unwrap_or("<anon>"),
                        cat.instance_methods.len(),
                        cat.class_methods.len(),
                    ),
                );
            }
            let merge_start = Instant::now();
            objc_rt.merge_categories_post_fixup(metadata, *slide);
            stats.category_merge_time += merge_start.elapsed();
        }
    }

    // Phase E: repair superclass addrs from mapped memory (post-fixup).
    // register_image reads superclass from on-disk data, where bind-encoded
    // refs are unresolved. After fixup the mapped memory has correct addresses.
    let repair_start = Instant::now();
    unsafe {
        objc_rt.repair_superclass_addrs();
    }
    stats.superclass_repair_time += repair_start.elapsed();

    // Store in global — shim functions will read from it
    *objc_runtime::OBJC_RUNTIME
        .write()
        .unwrap_or_else(|e| e.into_inner()) = Some(objc_rt);

    stats.record_to(tracer);
    stats
}

pub fn register_guest_compat_classes_from_registry(
    objc_rt: &mut objc_runtime::ObjcRuntime,
    registry: &macho_exec::ImageRegistry,
) {
    register_export_backed_compat_class(objc_rt, registry, "DVTSystemPrerequisitesPaths");
}

fn register_export_backed_compat_class(
    objc_rt: &mut objc_runtime::ObjcRuntime,
    registry: &macho_exec::ImageRegistry,
    class_name: &str,
) {
    if objc_rt.get_class(class_name).is_some() {
        return;
    }

    let class_symbol = format!("_OBJC_CLASS_$_{class_name}");
    let metaclass_symbol = format!("_OBJC_METACLASS_$_{class_name}");

    let find_export_addr = |symbol_name: &str| {
        registry.images().iter().find_map(|loaded| {
            loaded
                .exports
                .iter()
                .find(|export| export.name == symbol_name)
                .map(|export| (export.address as i64 + loaded.image.slide) as u64)
        })
    };

    let Some(class_addr) = find_export_addr(&class_symbol) else {
        return;
    };
    let Some(metaclass_addr) = find_export_addr(&metaclass_symbol) else {
        return;
    };

    objc_rt.register_synthetic_class(objc_runtime::ClassInfo {
        name: class_name.to_string(),
        mapped_addr: class_addr,
        methods: vec![],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: vec![],
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: None,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    objc_rt.register_metaclass_addr(metaclass_addr, class_name);
    log::debug!(
        "ObjC: registered export-backed compatibility class '{}' at {:#x} / meta {:#x}",
        class_name,
        class_addr,
        metaclass_addr,
    );
}

/// Register all synthetic Foundation classes in the ObjC runtime.
///
/// This registers NSObject, NSString, NSArray, NSDictionary, and all
/// other Foundation types that cf-lite provides shim implementations for.
pub fn register_foundation_classes(
    objc_rt: &mut objc_runtime::ObjcRuntime,
    config: &RuntimeConfig,
    tracer: &RuntimeTracer,
) {
    let _ = register_foundation_classes_with_stats(objc_rt, config, tracer);
}

fn register_foundation_classes_with_stats(
    objc_rt: &mut objc_runtime::ObjcRuntime,
    config: &RuntimeConfig,
    tracer: &RuntimeTracer,
) -> FoundationRegistrationStats {
    let mut stats = FoundationRegistrationStats::default();

    // Initialize cf-lite's platform provider for Foundation shims
    let platform_setup_start = Instant::now();
    cf_lite::set_platform(config.platform.clone());
    stats.platform_setup_time = platform_setup_start.elapsed();

    let template_apply_start = Instant::now();
    let foundation_template = foundation_runtime_template();
    if objc_rt.class_count() == 0 && objc_rt.selector_count() == 0 {
        *objc_rt = foundation_template.clone();
    } else {
        objc_rt.overlay_from(foundation_template);
    }
    stats.template_apply_time = template_apply_start.elapsed();

    tracer.record(
        EventKind::ClassRegistration,
        format!(
            "{} synthetic Foundation/AppKit classes registered",
            FOUNDATION_SYNTHETIC_CLASS_COUNT,
        ),
    );

    // Register host ISA aliases so constant objects
    // (@[], @{}, @(int)) dispatch through our shims.
    let host_isa_alias_start = Instant::now();
    cf_lite::FoundationShims::register_host_isa_aliases(objc_rt);
    stats.host_isa_alias_time = host_isa_alias_start.elapsed();

    // Install compatibility methods on guest-defined Xcode helper classes.
    let guest_compat_start = Instant::now();
    cf_lite::FoundationShims::register_guest_compat_methods(objc_rt);
    stats.guest_compat_time = guest_compat_start.elapsed();

    stats
}

fn foundation_runtime_template() -> &'static objc_runtime::ObjcRuntime {
    FOUNDATION_RUNTIME_TEMPLATE.get_or_init(|| {
        let mut objc_rt = objc_runtime::ObjcRuntime::new();
        objc_rt.reserve_registration_capacity(
            FOUNDATION_SYNTHETIC_CLASS_COUNT,
            FOUNDATION_PROTOCOL_RESERVE,
            FOUNDATION_SELECTOR_RESERVE,
            FOUNDATION_CATEGORY_LOAD_RESERVE,
        );

        populate_foundation_runtime(&mut objc_rt);
        objc_rt
    })
}

fn populate_foundation_runtime(objc_rt: &mut objc_runtime::ObjcRuntime) {
    cf_lite::FoundationShims::register_nsobject_class(objc_rt);
    cf_lite::FoundationShims::register_nsstring_proper_class(objc_rt);
    cf_lite::FoundationShims::register_nsstring_class(objc_rt);
    cf_lite::FoundationShims::register_nsarray_class(objc_rt);
    cf_lite::FoundationShims::register_nsnumber_class(objc_rt);
    cf_lite::FoundationShims::register_nsconstantintegernumber_class(objc_rt);
    cf_lite::FoundationShims::register_nsdictionary_class(objc_rt);
    cf_lite::FoundationShims::register_nsconstantdictionary_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutablearray_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutabledictionary_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutablestring_class(objc_rt);
    cf_lite::FoundationShims::register_nsdata_class(objc_rt);
    cf_lite::FoundationShims::register_nsprocessinfo_class(objc_rt);
    cf_lite::FoundationShims::register_nserror_class(objc_rt);
    cf_lite::FoundationShims::register_nsbundle_class(objc_rt);
    cf_lite::FoundationShims::register_nsfilemanager_class(objc_rt);
    cf_lite::FoundationShims::register_nsexception_class(objc_rt);
    cf_lite::FoundationShims::register_nsenumerator_class(objc_rt);
    cf_lite::FoundationShims::register_nsdate_class(objc_rt);
    cf_lite::FoundationShims::register_nsuserdefaults_class(objc_rt);
    cf_lite::FoundationShims::register_nsset_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableset_class(objc_rt);
    cf_lite::FoundationShims::register_nsurl_class(objc_rt);
    cf_lite::FoundationShims::register_nsnull_class(objc_rt);
    cf_lite::FoundationShims::register_nsdateformatter_class(objc_rt);
    cf_lite::FoundationShims::register_nscharacterset_class(objc_rt);
    cf_lite::FoundationShims::register_nsnotificationcenter_class(objc_rt);
    cf_lite::FoundationShims::register_nsnotification_class(objc_rt);
    cf_lite::FoundationShims::register_nsdistributednotificationcenter_class(objc_rt);
    cf_lite::FoundationShims::register_nsvalue_class(objc_rt);
    cf_lite::FoundationShims::register_nsuuid_class(objc_rt);
    cf_lite::FoundationShims::register_nslocale_class(objc_rt);
    cf_lite::FoundationShims::register_nstimezone_class(objc_rt);
    cf_lite::FoundationShims::register_nscalendar_class(objc_rt);
    cf_lite::FoundationShims::register_nsnumberformatter_class(objc_rt);
    cf_lite::FoundationShims::register_nsdecimalnumber_class(objc_rt);
    cf_lite::FoundationShims::register_nsindexset_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableindexset_class(objc_rt);
    cf_lite::FoundationShims::register_nssortdescriptor_class(objc_rt);
    cf_lite::FoundationShims::register_nstimer_class(objc_rt);
    cf_lite::FoundationShims::register_nsthread_class(objc_rt);
    cf_lite::FoundationShims::register_nslock_class(objc_rt);
    cf_lite::FoundationShims::register_nsrecursivelock_class(objc_rt);
    cf_lite::FoundationShims::register_nscondition_class(objc_rt);
    cf_lite::FoundationShims::register_nsconditionlock_class(objc_rt);
    cf_lite::FoundationShims::register_nsmethodsignature_class(objc_rt);
    cf_lite::FoundationShims::register_nsinvocation_class(objc_rt);
    cf_lite::FoundationShims::register_nsregularexpression_class(objc_rt);
    cf_lite::FoundationShims::register_nstextcheckingresult_class(objc_rt);
    cf_lite::FoundationShims::register_nspredicate_class(objc_rt);
    cf_lite::FoundationShims::register_nsattributedstring_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableattributedstring_class(objc_rt);
    cf_lite::FoundationShims::register_nscache_class(objc_rt);
    cf_lite::FoundationShims::register_nsmaptable_class(objc_rt);
    cf_lite::FoundationShims::register_nshashtable_class(objc_rt);
    cf_lite::FoundationShims::register_nsoperation_class(objc_rt);
    cf_lite::FoundationShims::register_nsblockoperation_class(objc_rt);
    cf_lite::FoundationShims::register_nsoperationqueue_class(objc_rt);
    cf_lite::FoundationShims::register_nsjsonserialization_class(objc_rt);
    cf_lite::FoundationShims::register_nspropertylistserialization_class(objc_rt);

    // AppKit classes (Phase C5)
    cf_lite::FoundationShims::register_nsresponder_class(objc_rt);
    cf_lite::FoundationShims::register_nsapplication_class(objc_rt);
    cf_lite::FoundationShims::register_nsrunloop_class(objc_rt);
    cf_lite::FoundationShims::register_nswindow_class(objc_rt);
    cf_lite::FoundationShims::register_nsview_class(objc_rt);
    cf_lite::FoundationShims::register_nsevent_class(objc_rt);
    cf_lite::FoundationShims::register_nsworkspace_class(objc_rt);
    cf_lite::FoundationShims::register_nsscreen_class(objc_rt);
    cf_lite::FoundationShims::register_nscolor_class(objc_rt);
    cf_lite::FoundationShims::register_nsfont_class(objc_rt);
    cf_lite::FoundationShims::register_nsfontmanager_class(objc_rt);
    cf_lite::FoundationShims::register_nsimage_class(objc_rt);
    cf_lite::FoundationShims::register_nsimagerep_class(objc_rt);
    cf_lite::FoundationShims::register_nsbitmapimagerep_class(objc_rt);

    // AppKit classes (Phase C7)
    cf_lite::FoundationShims::register_nsappearance_class(objc_rt);
    cf_lite::FoundationShims::register_nsusernotificationcenter_class(objc_rt);
    cf_lite::FoundationShims::register_nsmenu_class(objc_rt);
    cf_lite::FoundationShims::register_nsmenuitem_class(objc_rt);
    cf_lite::FoundationShims::register_nsnib_class(objc_rt);

    // AppKit GUI classes (Phase C8)
    cf_lite::FoundationShims::register_nsstoryboard_class(objc_rt);
    cf_lite::FoundationShims::register_nsviewcontroller_class(objc_rt);
    cf_lite::FoundationShims::register_nswindowcontroller_class(objc_rt);
    cf_lite::FoundationShims::register_nsdocument_class(objc_rt);
    cf_lite::FoundationShims::register_nsdocumentcontroller_class(objc_rt);
    cf_lite::FoundationShims::register_nscontrol_class(objc_rt);
    cf_lite::FoundationShims::register_nstextfield_class(objc_rt);

    // AppKit GUI classes (Phase C9 AppKit)
    cf_lite::FoundationShims::register_nsbutton_class(objc_rt);
    cf_lite::FoundationShims::register_nspopupbutton_class(objc_rt);
    cf_lite::FoundationShims::register_nsalert_class(objc_rt);
    cf_lite::FoundationShims::register_nspanel_class(objc_rt);
    cf_lite::FoundationShims::register_nsscrollview_class(objc_rt);
    cf_lite::FoundationShims::register_nsbox_class(objc_rt);
    cf_lite::FoundationShims::register_nssplitview_class(objc_rt);
    cf_lite::FoundationShims::register_nsstackview_class(objc_rt);
    cf_lite::FoundationShims::register_nstabview_class(objc_rt);
    cf_lite::FoundationShims::register_nstabviewitem_class(objc_rt);
    cf_lite::FoundationShims::register_nstoolbar_class(objc_rt);
    cf_lite::FoundationShims::register_nstoolbaritem_class(objc_rt);
    cf_lite::FoundationShims::register_nssavepanel_class(objc_rt);
    cf_lite::FoundationShims::register_nsopenpanel_class(objc_rt);
    cf_lite::FoundationShims::register_nsclipview_class(objc_rt);
    cf_lite::FoundationShims::register_nstextview_class(objc_rt);
    cf_lite::FoundationShims::register_nstrackingarea_class(objc_rt);
    cf_lite::FoundationShims::register_nscursor_class(objc_rt);
    cf_lite::FoundationShims::register_nscolorpanel_class(objc_rt);
    cf_lite::FoundationShims::register_nsfontpanel_class(objc_rt);
    cf_lite::FoundationShims::register_nsstepper_class(objc_rt);

    // AppKit widget classes (Phase C10 AppKit)
    cf_lite::FoundationShims::register_nsslider_class(objc_rt);
    cf_lite::FoundationShims::register_nsprogressindicator_class(objc_rt);
    cf_lite::FoundationShims::register_nsimageview_class(objc_rt);
    cf_lite::FoundationShims::register_nssegmentedcontrol_class(objc_rt);
    cf_lite::FoundationShims::register_nscombobox_class(objc_rt);
    cf_lite::FoundationShims::register_nscolorwell_class(objc_rt);
    cf_lite::FoundationShims::register_nspasteboard_class(objc_rt);
    cf_lite::FoundationShims::register_nslayoutconstraint_class(objc_rt);

    // Gap-analysis classes (Phase C8)
    cf_lite::FoundationShims::register_nsmutabledata_class(objc_rt);
    cf_lite::FoundationShims::register_nsfilehandle_class(objc_rt);
    cf_lite::FoundationShims::register_nsscanner_class(objc_rt);
    cf_lite::FoundationShims::register_nsorderedset_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableorderedset_class(objc_rt);
    cf_lite::FoundationShims::register_nscountedset_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutablecharacterset_class(objc_rt);

    // Gap-analysis Phase C9
    cf_lite::FoundationShims::register_nsurlcomponents_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlqueryitem_class(objc_rt);
    cf_lite::FoundationShims::register_nsbytecountformatter_class(objc_rt);
    cf_lite::FoundationShims::register_nsiso8601dateformatter_class(objc_rt);
    cf_lite::FoundationShims::register_nsprogress_class(objc_rt);
    cf_lite::FoundationShims::register_nshost_class(objc_rt);
    cf_lite::FoundationShims::register_nspipe_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlrequest_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableurlrequest_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlresponse_class(objc_rt);
    cf_lite::FoundationShims::register_nshttpurlresponse_class(objc_rt);
    cf_lite::FoundationShims::register_nstask_class(objc_rt);
    cf_lite::FoundationShims::register_pkbundlecomponentversion_class(objc_rt);

    // Phase C10 – remaining top-20 gaps
    cf_lite::FoundationShims::register_nsstream_class(objc_rt);
    cf_lite::FoundationShims::register_nsinputstream_class(objc_rt);
    cf_lite::FoundationShims::register_nsoutputstream_class(objc_rt);
    cf_lite::FoundationShims::register_nsxmlparser_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlsessionconfiguration_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlsession_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlsessiondatatask_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlsessionuploadtask_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlsessiondownloadtask_class(objc_rt);
    cf_lite::FoundationShims::register_nscoder_class(objc_rt);
    cf_lite::FoundationShims::register_nskeyedarchiver_class(objc_rt);
    cf_lite::FoundationShims::register_nskeyedunarchiver_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlcredential_class(objc_rt);
    cf_lite::FoundationShims::register_nsurlcredentialstorage_class(objc_rt);
    cf_lite::FoundationShims::register_nshttpcookie_class(objc_rt);
    cf_lite::FoundationShims::register_nshttpcookiestorage_class(objc_rt);
    cf_lite::FoundationShims::register_nsautoreleasepool_class(objc_rt);

    // NSUnit / NSMeasurement and concrete unit subclasses
    cf_lite::FoundationShims::register_nsunit_class(objc_rt);
    cf_lite::FoundationShims::register_nsdimension_class(objc_rt);
    cf_lite::FoundationShims::register_nsmeasurement_class(objc_rt);
    cf_lite::FoundationShims::register_nsunitlength_class(objc_rt);
    cf_lite::FoundationShims::register_nsunitmass_class(objc_rt);
    cf_lite::FoundationShims::register_nsunittemperature_class(objc_rt);
    cf_lite::FoundationShims::register_nsunitvolume_class(objc_rt);
    cf_lite::FoundationShims::register_nsunitangle_class(objc_rt);
    cf_lite::FoundationShims::register_nsunitspeed_class(objc_rt);
    cf_lite::FoundationShims::register_nsunitduration_class(objc_rt);
    cf_lite::FoundationShims::register_nsmeasurementformatter_class(objc_rt);

    // XCStringsParser.framework classes
    cf_lite::FoundationShims::register_xcstringsdataentry_class(objc_rt);
    cf_lite::FoundationShims::register_xcstringsdatatable_class(objc_rt);
    cf_lite::FoundationShims::register_xcstringsdatafile_class(objc_rt);

    // CoreData stub classes
    cf_lite::FoundationShims::register_nsmanagedobjectcontext_class(objc_rt);
    cf_lite::FoundationShims::register_nspersistentstorecoordinator_class(objc_rt);
    cf_lite::FoundationShims::register_nsfetchrequest_class(objc_rt);
    cf_lite::FoundationShims::register_nsmanagedobject_class(objc_rt);
    cf_lite::FoundationShims::register_nsentitydescription_class(objc_rt);
    cf_lite::FoundationShims::register_nsmanagedobjectmodel_class(objc_rt);

    // AVFoundation stub classes
    cf_lite::FoundationShims::register_avaudioplayer_class(objc_rt);
    cf_lite::FoundationShims::register_avaudiorecorder_class(objc_rt);
    cf_lite::FoundationShims::register_avaudioengine_class(objc_rt);
    cf_lite::FoundationShims::register_avcapturedevice_class(objc_rt);
    cf_lite::FoundationShims::register_avcapturesession_class(objc_rt);
    cf_lite::FoundationShims::register_avplayer_class(objc_rt);

    // CoreLocation + CoreBluetooth stub classes
    cf_lite::FoundationShims::register_cllocationmanager_class(objc_rt);
    cf_lite::FoundationShims::register_cllocation_class(objc_rt);
    cf_lite::FoundationShims::register_clgeocoder_class(objc_rt);
    cf_lite::FoundationShims::register_cbcentralmanager_class(objc_rt);
    cf_lite::FoundationShims::register_cbperipheralmanager_class(objc_rt);

    // Phase C11 — Layout Anchors, Layout Guide, VFL, intrinsic content size
    cf_lite::FoundationShims::register_nslayoutanchor_class(objc_rt);
    cf_lite::FoundationShims::register_nslayoutguide_class(objc_rt);
    cf_lite::FoundationShims::register_nsview_anchor_methods(objc_rt);
    cf_lite::FoundationShims::register_nswindow_layout_guide_method(objc_rt);
    cf_lite::FoundationShims::register_nslayoutconstraint_vfl_method(objc_rt);

    // Phase C12 — Safe area, NSTraitCollection, NSAnimationContext, NSTableView, NSUndoManager
    cf_lite::FoundationShims::register_nstraitcollection_class(objc_rt);
    cf_lite::FoundationShims::register_nsanimationcontext_class(objc_rt);
    cf_lite::FoundationShims::register_nstableview_class(objc_rt);
    cf_lite::FoundationShims::register_nsoutlineview_class(objc_rt);
    cf_lite::FoundationShims::register_nsundomanager_class(objc_rt);
    cf_lite::FoundationShims::register_nsview_safe_area_methods(objc_rt);

    // Phase C13 — CoreAnimation (CALayer, CAMetalLayer, CATransaction, CADisplayLink, CAMediaTimingFunction)
    cf_lite::FoundationShims::register_calayer_class(objc_rt);
    cf_lite::FoundationShims::register_cametallayer_class(objc_rt);
    cf_lite::FoundationShims::register_catransaction_class(objc_rt);
    cf_lite::FoundationShims::register_cadisplaylink_class(objc_rt);
    cf_lite::FoundationShims::register_camediatimingfunction_class(objc_rt);

    // Phase C14 — Drawing (NSOpenGLPixelFormat, NSOpenGLContext,
    // NSGraphicsContext, NSBezierPath)
    cf_lite::FoundationShims::register_nsopenglpixelformat_class(objc_rt);
    cf_lite::FoundationShims::register_nsopenglcontext_class(objc_rt);
    cf_lite::FoundationShims::register_nsgraphicscontext_class(objc_rt);
    cf_lite::FoundationShims::register_nsbezierpath_class(objc_rt);

    // Phase C15 — TextKit (NSTextStorage, NSLayoutManager, NSTextContainer)
    cf_lite::FoundationShims::register_nstextstorage_class(objc_rt);
    cf_lite::FoundationShims::register_nslayoutmanager_class(objc_rt);
    cf_lite::FoundationShims::register_nstextcontainer_class(objc_rt);

    // Register tagged pointer class table (NSNumber=2, NSString=3, NSDate=6).
    cf_lite::FoundationShims::register_tagged_classes(objc_rt);
}

/// Notify the host ObjC runtime about guest-mapped images via `_objc_map_images`.
///
/// Guest-defined ObjC/Swift classes have uninitialized method caches.
/// When host code (e.g., the Swift runtime) internally calls the real
/// `objc_msgSend` on objects with guest class ISAs, the NULL cache
/// causes a SEGV.  We fix this by writing the host's `_objc_empty_cache`
/// address into each guest class's cache field (offset +0x10).  This
/// lets `objc_msgSend` see an empty-but-valid cache, fall through to
/// the slow path, and resolve methods normally.
///
/// Must be called AFTER fixups but BEFORE `protect()` (DATA must be writable).
#[cfg(target_os = "macos")]
pub fn register_guest_images_with_host_runtime(registry: &macho_exec::ImageRegistry) {
    use std::ffi::c_void;

    extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const std::ffi::c_char) -> *mut c_void;
    }
    const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

    // Resolve _objc_empty_cache — the sentinel empty bucket used by the
    // host runtime for classes that have no cached methods yet.
    let empty_cache = unsafe { dlsym(RTLD_DEFAULT, c"_objc_empty_cache".as_ptr()) } as u64;
    if empty_cache == 0 {
        log::debug!("host ObjC: _objc_empty_cache not found — skipping cache init");
        return;
    }

    /// Offset of `cache_t._bucketsAndMaybeMask` within `objc_class`.
    const CACHE_OFFSET: usize = 0x10;

    let mut total_patched = 0usize;
    for loaded in registry.images() {
        // Find __objc_classlist section to get class pointers.
        let classlist_section = loaded
            .parsed
            .sections
            .iter()
            .find(|s| s.sectname == "__objc_classlist");
        let Some(section) = classlist_section else {
            continue;
        };

        // Each entry in __objc_classlist is a pointer to an objc_class struct.
        let slide = loaded.image.slide;
        let entry_count = (section.size / 8) as usize;
        let section_start = (section.addr as i64 + slide) as *const u64;

        for i in 0..entry_count {
            let class_ptr = unsafe { *section_start.add(i) };
            if class_ptr == 0 {
                continue;
            }

            // Read the current cache value at class + 0x10.
            let cache_slot = (class_ptr as usize + CACHE_OFFSET) as *mut u64;

            // Always overwrite: the binary's cache field may contain a
            // chained-fixup bind to a shared-cache preopt cache that was
            // resolved to an invalid address by our stub resolver.
            unsafe {
                *cache_slot = empty_cache;
            }
            // Also zero the second qword of cache_t
            unsafe {
                *((class_ptr as usize + CACHE_OFFSET + 8) as *mut u64) = 0;
            }
            total_patched += 1;

            // Also patch the metaclass cache (ISA of the class → metaclass).
            let raw_isa = unsafe { *(class_ptr as *const u64) };
            // ARM64 non-pointer ISA mask
            let metaclass = raw_isa & 0x007ffffffffffff8;
            if metaclass != 0 && metaclass != class_ptr {
                let meta_cache = (metaclass as usize + CACHE_OFFSET) as *mut u64;
                unsafe {
                    *meta_cache = empty_cache;
                }
                unsafe {
                    *((metaclass as usize + CACHE_OFFSET + 8) as *mut u64) = 0;
                }
                total_patched += 1;
            }
        }
    }

    if total_patched > 0 {
        log::info!(
            "host ObjC: initialized {total_patched} guest class caches with _objc_empty_cache"
        );
    }
}

#[cfg(not(target_os = "macos"))]
pub fn register_guest_images_with_host_runtime(_registry: &macho_exec::ImageRegistry) {}
