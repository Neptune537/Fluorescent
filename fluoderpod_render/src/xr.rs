use openxr as xr;

pub struct OpenXrContext {
    pub instance: xr::Instance,
    pub system: xr::SystemId,
    pub session: xr::Session<xr::Vulkan>,
    pub space: xr::Space,
    pub frame_waiter: xr::FrameWaiter,
    pub frame_stream: xr::FrameStream<xr::Vulkan>,
}

impl OpenXrContext {
    /// Initializes OpenXR for Vulkan, targeting Android (e.g. Meta Quest 3).
    #[cfg(target_os = "android")]
    pub fn new_android(
        app_vm: *mut std::ffi::c_void,
        app_activity: *mut std::ffi::c_void,
    ) -> Result<Self, xr::sys::Result> {
        let entry = xr::Entry::load().expect("Failed to load OpenXR entry");

        // The OpenXR Android loader requires initialization with the VM and Activity.
        if let Err(e) = entry.initialize_android_loader() {
            log::warn!("Could not initialize Android loader, it might already be initialized or not required: {:?}", e);
        }

        let mut extensions = xr::ExtensionSet::default();
        extensions.khr_vulkan_enable2 = true;
        extensions.khr_android_create_instance = true;

        let instance = entry.create_instance(
            &xr::ApplicationInfo {
                application_name: "Fluorescent VR",
                application_version: 0,
                engine_name: "Fluorescent Engine",
                engine_version: 0,
            },
            &extensions,
            &["VK_LAYER_KHRONOS_validation"],
        )?;

        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;

        // Fetch Vulkan Graphics Requirements
        // OpenXR needs to approve the Vulkan version and dictates which Physical Device we must use
        let reqs = instance.vulkan_graphics_requirements2(system)?;
        log::info!("OpenXR Vulkan requirements: min={:?}, max={:?}", reqs.min_api_version_supported, reqs.max_api_version_supported);

        // At this point we would:
        // 1. Create an `ash::Instance` enabling extensions mandated by OpenXR via `instance.vulkan_instance_extensions(system)`.
        // 2. Fetch the `vk::PhysicalDevice` from OpenXR using `instance.vulkan_graphics_device(system, vk_instance)`.
        // 3. Create the `ash::Device` enabling extensions mandated by `instance.vulkan_device_extensions(system)`.
        // 4. Create the OpenXR session with the `ash::Instance`, `vk::PhysicalDevice`, and `ash::Device`.
        // 5. Pass those same `ash` handles to `unified_pipeline::PipelineManager::init_from_openxr_vulkan`.

        // Example Session Creation (stubbed because we need the ash bindings connected):
        // let session = instance.create_session::<xr::Vulkan>(
        //     system,
        //     &xr::vulkan::SessionCreateInfo {
        //         instance: vk_instance.handle().as_raw() as _,
        //         physical_device: vk_physical_device.as_raw() as _,
        //         device: vk_device.handle().as_raw() as _,
        //         queue_family_index,
        //         queue_index,
        //     },
        // )?;

        unimplemented!("Vulkan Device creation logic is partially drafted. Needs ash instantiation before session creation.")
    }

    /// Update XR state, handle events
    pub fn update(&mut self) {
        // Poll OpenXR events
        while let Some(event) = self.instance.poll_event(&mut Default::default()).unwrap() {
            match event {
                xr::Event::SessionStateChanged(e) => {
                    log::info!("XR Session state changed: {:?}", e.state());
                }
                _ => {}
            }
        }
    }
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod jni_bindings {
    use jni::objects::{JClass, JObject};
    use jni::sys::jboolean;
    use jni::JNIEnv;

    // The name of this function must match your Kotlin package structure.
    // For example, if your package is `com.example.hybrid_2d_3d`, the function is:
    // Java_com_example_hybrid_12d_13d_MainActivity_initNativeOpenXR
    // (Note: underscores in package names are escaped with _1)
    
    #[no_mangle]
    pub extern "system" fn Java_com_example_hybrid_12d_13d_MainActivity_initNativeOpenXR(
        mut env: JNIEnv,
        _class: JClass,
        activity: JObject,
    ) -> jboolean {
        let vm = env.get_java_vm().expect("Failed to get JavaVM");

        // Pass the raw pointers for the VM and Activity down to OpenXR
        let vm_ptr = vm.get_java_vm_pointer() as *mut std::ffi::c_void;
        // In jni >= 0.20, we need to extract the raw object pointer
        let activity_ptr = activity.as_raw() as *mut std::ffi::c_void;

        match super::OpenXrContext::new_android(vm_ptr, activity_ptr) {
            Ok(context) => {
                log::info!("Successfully initialized OpenXR on Android!");
                // Store the context globally or within the renderer
                1 // true
            }
            Err(e) => {
                log::error!("Failed to initialize OpenXR: {:?}", e);
                0 // false
            }
        }
    }
}
