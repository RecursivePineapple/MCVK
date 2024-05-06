use bytemuck::{Pod, Zeroable};
use glfw::*;
use nalgebra_glm::{
    half_pi, identity, look_at, perspective, pi, rotate_normalized_axis, translate, vec3, TMat4,
};
use pipe_trait::Pipe;
use vulkano::buffer::cpu_pool::CpuBufferPoolSubbuffer;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageAccess, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, BlendOp, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    self, AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::{Version, VulkanLibrary, VulkanObject};

use std::ptr::null;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position, normal, color);

#[derive(Default, Debug, Clone)]
struct AmbientLight {
    color: [f32; 3],
    intensity: f32,
}

#[derive(Default, Debug, Clone)]
struct DirectionalLight {
    position: [f32; 4],
    color: [f32; 3],
}

#[derive(Debug, Clone)]
struct MVP {
    model: TMat4<f32>,
    view: TMat4<f32>,
    projection: TMat4<f32>,
}

impl MVP {
    fn new() -> MVP {
        MVP {
            model: identity(),
            view: identity(),
            projection: identity(),
        }
    }
}

mod deferred_vert {
    const _: &'static str = include_str!("shaders/deferred.vert");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/deferred.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod deferred_frag {
    const _: &'static str = include_str!("shaders/deferred.frag");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/deferred.frag",
        types_meta: {
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod directional_vert {
    const _: &'static str = include_str!("shaders/directional.vert");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/directional.vert",
        types_meta: {
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod directional_frag {
    const _: &'static str = include_str!("shaders/directional.frag");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/directional.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod ambient_vert {
    const _: &'static str = include_str!("shaders/ambient.vert");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/ambient.vert",
        types_meta: {
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod ambient_frag {
    const _: &'static str = include_str!("shaders/ambient.frag");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/ambient.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

#[derive(Default, Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DummyVertex {
    pub position: [f32; 2],
}

vulkano::impl_vertex!(DummyVertex, position);

impl DummyVertex {
    pub fn list() -> [DummyVertex; 6] {
        [
            DummyVertex {
                position: [-1.0, -1.0],
            },
            DummyVertex {
                position: [-1.0, 1.0],
            },
            DummyVertex {
                position: [1.0, 1.0],
            },
            DummyVertex {
                position: [-1.0, -1.0],
            },
            DummyVertex {
                position: [1.0, 1.0],
            },
            DummyVertex {
                position: [1.0, -1.0],
            },
        ]
    }
}

fn main() {
    let mut mvp = MVP::new();

    mvp.view = look_at(
        &vec3(0.0, 0.0, 0.1),
        &vec3(0.0, 0.0, 0.0),
        &vec3(0.0, 1.0, 0.0),
    );

    mvp.model = translate(&identity(), &vec3(0.0, 0.0, -5.0));

    let ambient_light = AmbientLight {
        color: [1.0, 1.0, 1.0],
        intensity: 0.1,
    };
    let directional_light = DirectionalLight {
        position: [-4.0, -4.0, 0.0, 1.0],
        color: [1.0, 1.0, 1.0],
    };

    let mut glfw = glfw::init_no_callbacks().unwrap();

    let instance = {
        let library = VulkanLibrary::new().unwrap();
        let extensions: InstanceExtensions =
            vulkano_win::required_extensions(&library).union(&InstanceExtensions::from_iter(
                glfw.get_required_instance_extensions()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str()),
            ));

        let mut layers = Vec::new();

        if cfg!(debug_assertions) {
            let validation = library
                .layer_properties()
                .unwrap()
                .find(|l| l.name() == "VK_LAYER_KHRONOS_validation");

            layers.push(
                validation
                    .expect("expected layer VK_LAYER_KHRONOS_validation to be present")
                    .name()
                    .to_owned(),
            );
        }

        Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: extensions,
                enumerate_portability: true, // required for MoltenVK on macOS
                max_api_version: Some(Version::V1_1),
                enabled_layers: layers,
                ..Default::default()
            },
        )
        .unwrap()
    };

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .unwrap()
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    // pick first queue_familiy_index that handles graphics and can draw on the surface created by winit
                    q.queue_flags.graphics
                        && glfw.get_physical_device_presentation_support_raw(
                            instance.handle(),
                            p.handle(),
                            i as u32,
                        )
                })
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| {
            // lower score for preferred device types
            match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            }
        })
        .expect("No suitable physical device found");

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: device_extensions,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .unwrap();

    let queue = queues.next().unwrap();

    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::NoApi));

    let (mut window, events) = glfw
        .create_window(640, 480, "MCVK test", glfw::WindowMode::Windowed)
        .unwrap();

    let mut surface = Default::default();

    window
        .create_window_surface(instance.handle(), null(), &mut surface)
        .result()
        .unwrap();

    let surface = unsafe {
        Surface::from_handle(instance.clone(), surface, swapchain::SurfaceApi::Xlib, None)
    };

    let surface = Arc::new(surface);

    let (mut swapchain, images) = {
        let caps = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();

        let image_format = Some(
            device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let image_extent: [u32; 2] = window.get_size().pipe(|(x, y)| [x as u32, y as u32]);

        let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
        mvp.projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count,
                image_format,
                image_extent,
                image_usage: usage,
                composite_alpha: alpha,
                ..Default::default()
            },
        )
        .unwrap()
    };

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());

    let deferred_vert = deferred_vert::load(device.clone()).unwrap();
    let deferred_frag = deferred_frag::load(device.clone()).unwrap();
    let directional_vert = directional_vert::load(device.clone()).unwrap();
    let directional_frag = directional_frag::load(device.clone()).unwrap();
    let ambient_vert = ambient_vert::load(device.clone()).unwrap();
    let ambient_frag = ambient_frag::load(device.clone()).unwrap();

    let render_pass = vulkano::ordered_passes_renderpass!(device.clone(),
        attachments: {
            final_color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            },
            color: {
                load: Clear,
                store: DontCare,
                format: Format::A2B10G10R10_UNORM_PACK32,
                samples: 1,
            },
            normals: {
                load: Clear,
                store: DontCare,
                format: Format::R16G16B16A16_SFLOAT,
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D16_UNORM,
                samples: 1,
            }
        },
        passes: [
            {
                color: [color, normals],
                depth_stencil: {depth},
                input: []
            },
            {
                color: [final_color],
                depth_stencil: {},
                input: [color, normals]
            }
        ]
    )
    .unwrap();

    let deferred_pass = Subpass::from(render_pass.clone(), 0).unwrap();
    let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

    let deferred_pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        .vertex_shader(deferred_vert.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .fragment_shader(deferred_frag.entry_point("main").unwrap(), ())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
        .render_pass(deferred_pass.clone())
        .build(device.clone())
        .unwrap();

    let directional_pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
        .vertex_shader(directional_vert.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .fragment_shader(directional_frag.entry_point("main").unwrap(), ())
        .color_blend_state(
            ColorBlendState::new(lighting_pass.num_color_attachments()).blend(AttachmentBlend {
                color_op: BlendOp::Add,
                color_source: BlendFactor::One,
                color_destination: BlendFactor::One,
                alpha_op: BlendOp::Max,
                alpha_source: BlendFactor::One,
                alpha_destination: BlendFactor::One,
            }),
        )
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
        .render_pass(lighting_pass.clone())
        .build(device.clone())
        .unwrap();

    let ambient_pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
        .vertex_shader(ambient_vert.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .fragment_shader(ambient_frag.entry_point("main").unwrap(), ())
        .color_blend_state(
            ColorBlendState::new(lighting_pass.num_color_attachments()).blend(AttachmentBlend {
                color_op: BlendOp::Add,
                color_source: BlendFactor::One,
                color_destination: BlendFactor::One,
                alpha_op: BlendOp::Max,
                alpha_source: BlendFactor::One,
                alpha_destination: BlendFactor::One,
            }),
        )
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
        .render_pass(lighting_pass.clone())
        .build(device.clone())
        .unwrap();

    let uniform_buffer: CpuBufferPool<deferred_vert::ty::MvpData> =
        CpuBufferPool::uniform_buffer(memory_allocator.clone());

    let ambient_buffer: CpuBufferPool<ambient_frag::ty::AmbientData> =
        CpuBufferPool::uniform_buffer(memory_allocator.clone());

    let directional_buffer: CpuBufferPool<directional_frag::ty::DirectionalLightData> =
        CpuBufferPool::uniform_buffer(memory_allocator.clone());

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        &memory_allocator,
        BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::empty()
        },
        false,
        VERTICES,
    )
    .unwrap();

    let dummy_verts = CpuAccessibleBuffer::from_iter(
        &memory_allocator,
        BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::empty()
        },
        false,
        DummyVertex::list().iter().cloned(),
    )
    .unwrap();

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    let (mut framebuffers, mut color_buffer, mut normal_buffer) = window_size_dependent_setup(
        &memory_allocator,
        &images,
        render_pass.clone(),
        &mut viewport,
    );

    let mut recreate_swapchain = false;

    let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

    let rotation_start = Instant::now();

    window.set_all_polling(true);

    while !window.should_close() {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            println!("{:?}", event);
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                glfw::WindowEvent::FramebufferSize(_, _) => {
                    recreate_swapchain = true;
                }
                glfw::WindowEvent::Key(key, scancode, action, modifiers) => {
                    dbg!(scancode);
                }
                _ => {}
            }
        }

        previous_frame_end
            .as_mut()
            .take()
            .unwrap()
            .cleanup_finished();

        if recreate_swapchain {
            let image_extent: [u32; 2] = window.get_size().pipe(|(x, y)| [x as u32, y as u32]);

            let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
            mvp.projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);

            let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                image_extent,
                ..swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            let (new_framebuffers, new_color_buffer, new_normal_buffer) =
                window_size_dependent_setup(
                    &memory_allocator,
                    &new_images,
                    render_pass.clone(),
                    &mut viewport,
                );

            swapchain = new_swapchain;
            framebuffers = new_framebuffers;
            color_buffer = new_color_buffer;
            normal_buffer = new_normal_buffer;

            recreate_swapchain = false;
        }

        let (image_index, suboptimal, acquire_future) = {
            match swapchain::acquire_next_image(swapchain.clone(), Some(Duration::from_millis(50)))
            {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            }
        };

        if suboptimal {
            recreate_swapchain = true;
        }

        let uniform_subbuffer = {
            let elapsed = rotation_start.elapsed().as_secs() as f64
                + rotation_start.elapsed().subsec_nanos() as f64 / 1_000_000_000.0;
            let elapsed_as_radians = elapsed * pi::<f64>() / 180.0;
            let mut model: TMat4<f32> = rotate_normalized_axis(
                &identity(),
                elapsed_as_radians as f32 * 50.0,
                &vec3(0.0, 0.0, 1.0),
            );
            model = rotate_normalized_axis(
                &model,
                elapsed_as_radians as f32 * 30.0,
                &vec3(0.0, 1.0, 0.0),
            );
            model = rotate_normalized_axis(
                &model,
                elapsed_as_radians as f32 * 20.0,
                &vec3(1.0, 0.0, 0.0),
            );
            model = mvp.model * model;

            let uniform_data = deferred_vert::ty::MvpData {
                model: model.into(),
                view: mvp.view.into(),
                projection: mvp.projection.into(),
            };

            uniform_buffer.from_data(uniform_data).unwrap()
        };

        let ambient_subbuffer = {
            let uniform_data = ambient_frag::ty::AmbientData {
                color: ambient_light.color.into(),
                intensity: ambient_light.intensity.into(),
            };

            ambient_buffer.from_data(uniform_data).unwrap()
        };

        let deferred_layout = deferred_pipeline.layout().set_layouts().get(0).unwrap();
        let deferred_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            deferred_layout.clone(),
            [WriteDescriptorSet::buffer(0, uniform_subbuffer.clone())],
        )
        .unwrap();

        let ambient_layout = ambient_pipeline.layout().set_layouts().get(0).unwrap();
        let ambient_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, color_buffer.clone()),
                WriteDescriptorSet::buffer(1, ambient_subbuffer.clone()),
            ],
        )
        .unwrap();

        let directional_subbuffer =
            generate_directional_buffer(&directional_buffer, &directional_light);

        let directional_layout = directional_pipeline.layout().set_layouts().get(0).unwrap();
        let directional_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            directional_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, color_buffer.clone()),
                WriteDescriptorSet::image_view(1, normal_buffer.clone()),
                WriteDescriptorSet::buffer(2, directional_subbuffer.clone()),
            ],
        )
        .unwrap();

        let mut commands = AutoCommandBufferBuilder::primary(
            &command_buffer_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let clear_values = vec![
            Some([0.0, 0.0, 0.0, 1.0].into()),
            Some([0.0, 0.0, 0.0, 1.0].into()),
            Some([0.0, 0.0, 0.0, 1.0].into()),
            Some(1.0.into()),
        ];

        commands
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(framebuffers[image_index as usize].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap()
            .set_viewport(0, [viewport.clone()])
            .bind_pipeline_graphics(deferred_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                deferred_pipeline.layout().clone(),
                0,
                deferred_set.clone(),
            )
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(directional_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                directional_pipeline.layout().clone(),
                0,
                directional_set.clone(),
            )
            .bind_vertex_buffers(0, dummy_verts.clone())
            .draw(dummy_verts.len() as u32, 1, 0, 0)
            .unwrap()
            .bind_pipeline_graphics(ambient_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                ambient_pipeline.layout().clone(),
                0,
                ambient_set.clone(),
            )
            .bind_vertex_buffers(0, dummy_verts.clone())
            .draw(dummy_verts.len() as u32, 1, 0, 0)
            .unwrap()
            .end_render_pass()
            .unwrap();

        let command_buffer = commands.build().unwrap();

        let future = previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
            }
        }
    }
}

fn generate_directional_buffer(
    pool: &CpuBufferPool<directional_frag::ty::DirectionalLightData>,
    light: &DirectionalLight,
) -> Arc<CpuBufferPoolSubbuffer<directional_frag::ty::DirectionalLightData>> {
    let uniform_data = directional_frag::ty::DirectionalLightData {
        position: light.position.into(),
        color: light.color.into(),
    };

    pool.from_data(uniform_data).unwrap()
}

fn window_size_dependent_setup(
    allocator: &StandardMemoryAllocator,
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(allocator, dimensions, Format::D16_UNORM).unwrap(),
    )
    .unwrap();

    let color_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator,
            dimensions,
            Format::A2B10G10R10_UNORM_PACK32,
        )
        .unwrap(),
    )
    .unwrap();

    let normal_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator,
            dimensions,
            Format::R16G16B16A16_SFLOAT,
        )
        .unwrap(),
    )
    .unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view,
                        color_buffer.clone(),
                        normal_buffer.clone(),
                        depth_buffer.clone(),
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    (framebuffers, color_buffer.clone(), normal_buffer.clone())
}

const VERTICES: [Vertex; 36] = [
    // front face
    Vertex {
        position: [-1.000000, -1.000000, 1.000000],
        normal: [0.0000, 0.0000, 1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, 1.000000],
        normal: [0.0000, 0.0000, 1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, 1.000000],
        normal: [0.0000, 0.0000, 1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, -1.000000, 1.000000],
        normal: [0.0000, 0.0000, 1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, 1.000000],
        normal: [0.0000, 0.0000, 1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, 1.000000],
        normal: [0.0000, 0.0000, 1.0000],
        color: [1.0, 0.35, 0.137],
    },
    // back face
    Vertex {
        position: [1.000000, -1.000000, -1.000000],
        normal: [0.0000, 0.0000, -1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, -1.000000],
        normal: [0.0000, 0.0000, -1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, -1.000000],
        normal: [0.0000, 0.0000, -1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, -1.000000],
        normal: [0.0000, 0.0000, -1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, -1.000000],
        normal: [0.0000, 0.0000, -1.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, -1.000000, -1.000000],
        normal: [0.0000, 0.0000, -1.0000],
        color: [1.0, 0.35, 0.137],
    },
    // top face
    Vertex {
        position: [-1.000000, -1.000000, 1.000000],
        normal: [0.0000, -1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, 1.000000],
        normal: [0.0000, -1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, -1.000000],
        normal: [0.0000, -1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, -1.000000, 1.000000],
        normal: [0.0000, -1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, -1.000000],
        normal: [0.0000, -1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, -1.000000, -1.000000],
        normal: [0.0000, -1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    // bottom face
    Vertex {
        position: [1.000000, 1.000000, 1.000000],
        normal: [0.0000, 1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, 1.000000],
        normal: [0.0000, 1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, -1.000000],
        normal: [0.0000, 1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, 1.000000],
        normal: [0.0000, 1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, -1.000000],
        normal: [0.0000, 1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, -1.000000],
        normal: [0.0000, 1.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    // left face
    Vertex {
        position: [-1.000000, -1.000000, -1.000000],
        normal: [-1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, -1.000000],
        normal: [-1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, 1.000000],
        normal: [-1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, -1.000000, -1.000000],
        normal: [-1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, 1.000000, 1.000000],
        normal: [-1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [-1.000000, -1.000000, 1.000000],
        normal: [-1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    // right face
    Vertex {
        position: [1.000000, -1.000000, 1.000000],
        normal: [1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, 1.000000],
        normal: [1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, -1.000000],
        normal: [1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, 1.000000],
        normal: [1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, 1.000000, -1.000000],
        normal: [1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
    Vertex {
        position: [1.000000, -1.000000, -1.000000],
        normal: [1.0000, 0.0000, 0.0000],
        color: [1.0, 0.35, 0.137],
    },
];
