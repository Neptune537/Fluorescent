use openxr as xr;
use std::sync::Arc;
use crate::FluoderpodRenderer;

pub struct OpenXrContext {
    pub instance: xr::Instance,
    pub system: xr::SystemId,
    pub session: xr::Session<xr::Vulkan>,
    pub space: xr::Space,
    pub frame_waiter: xr::FrameWaiter,
    pub frame_stream: xr::FrameStream<xr::Vulkan>,
    pub telemetry_sender: Option<XrTelemetrySender>,
}

impl OpenXrContext {
    #[cfg(target_os = "android")]
    pub fn new_android(
        app_vm: *mut std::ffi::c_void,
        app_activity: *mut std::ffi::c_void,
    ) -> Result<Self, xr::sys::Result> {
        let entry = xr::Entry::load().expect("Failed to load OpenXR entry");

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

        let reqs = instance.vulkan_graphics_requirements2(system)?;
        log::info!("OpenXR Vulkan requirements: min={:?}, max={:?}", reqs.min_api_version_supported, reqs.max_api_version_supported);

        // At this point we would create the vulkan instance using extensions, then ash::Device using requirements,
        // and finally create the xr::vulkan::SessionCreateInfo to initialize the session.
        // For now, this initialization pipeline stops here until wgpu_core visibility allows mapping it back down to wgpu.
        // Returning unimplemented error to prevent runtime crash on initialization until fully integrated.
        Err(xr::sys::Result::ERROR_RUNTIME_FAILURE)
    }

    pub fn update(&mut self) {
        while let Some(event) = self.instance.poll_event(&mut Default::default()).unwrap() {
            match event {
                xr::Event::SessionStateChanged(e) => {
                    log::info!("XR Session state changed: {:?}", e.state());
                }
                _ => {}
            }
        }
    }

    pub fn render_frame(
        &mut self,
        renderer: &mut FluoderpodRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_target_left: &wgpu::TextureView,
        color_target_right: &wgpu::TextureView,
    ) {
        let frame_state = self.frame_waiter.wait().unwrap();
        self.frame_stream.begin().unwrap();

        if !frame_state.should_render {
            self.frame_stream.end(frame_state.predicted_display_time, xr::EnvironmentBlendMode::OPAQUE, &[]).unwrap();
            return;
        }

        let (_flags, views) = self.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            frame_state.predicted_display_time,
            &self.space,
        ).unwrap();

        // Broadcast head pose telemetry
        if let Some(sender) = &self.telemetry_sender {
            let pose = views[0].pose;
            sender.send_head_pose(
                [pose.position.x, pose.position.y, pose.position.z],
                [pose.orientation.x, pose.orientation.y, pose.orientation.z, pose.orientation.w]
            );
        }

        // 1. Render Left Eye
        let _view_left = views[0];
        let mut encoder_left = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Left Eye Encoder") });
        // TODO: update camera uniform buffer with `_view_left.pose`
        // Workaround for rust wgpu borrow checker on encoder/render pass
        renderer.pipeline_manager.flush_staging_to_gpu();
        {
            let mut rpass_left = encoder_left.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Left Eye Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_target_left,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderer.pipeline_manager.submit_indirect_draws(&mut rpass_left);
        }
        queue.submit(Some(encoder_left.finish()));

        // 2. Render Right Eye
        let _view_right = views[1];
        let mut encoder_right = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Right Eye Encoder") });
        // TODO: update camera uniform buffer with `_view_right.pose`
        {
            let mut rpass_right = encoder_right.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Right Eye Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_target_right,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderer.pipeline_manager.submit_indirect_draws(&mut rpass_right);
        }
        queue.submit(Some(encoder_right.finish()));

        self.frame_stream.end(frame_state.predicted_display_time, xr::EnvironmentBlendMode::OPAQUE, &[]).unwrap();
    }
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod jni_bindings {
    use jni::objects::{JClass, JObject};
    use jni::sys::jboolean;
    use jni::JNIEnv;
    
    #[no_mangle]
    pub extern "system" fn Java_com_example_hybrid_12d_13d_MainActivity_initNativeOpenXR(
        mut env: JNIEnv,
        _class: JClass,
        activity: JObject,
    ) -> jboolean {
        let vm = env.get_java_vm().expect("Failed to get JavaVM");
        let vm_ptr = vm.get_java_vm_pointer() as *mut std::ffi::c_void;
        let activity_ptr = activity.as_raw() as *mut std::ffi::c_void;

        match super::OpenXrContext::new_android(vm_ptr, activity_ptr) {
            Ok(_) => {
                log::info!("Successfully initialized OpenXR on Android!");
                1 // true
            }
            Err(e) => {
                log::error!("Failed to initialize OpenXR: {:?}", e);
                0 // false
            }
        }
    }
}

use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::SinkExt;
use crate::nexus_client::RailMessage;
use tokio::sync::mpsc;

pub struct XrTelemetrySender {
    tx: mpsc::UnboundedSender<String>,
}

impl XrTelemetrySender {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            let url = "ws://127.0.0.1:9001";
            if let Ok((mut ws_stream, _)) = tokio_tungstenite::connect_async(url).await {
                while let Some(msg) = rx.recv().await {
                    let _ = ws_stream.send(Message::Text(msg)).await;
                }
            }
        });
        Self { tx }
    }

    pub fn send_head_pose(&self, pos: [f32; 3], quat: [f32; 4]) {
        let msg = RailMessage::HeadPose { position: pos, orientation: quat };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.tx.send(json);
        }
    }
}
