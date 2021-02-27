use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice, PhysicalDeviceType};
use vulkano::pipeline::vertex::TwoBuffersDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};

use vulkano::swapchain::{
  ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain,
  SwapchainCreationError,
};

use vulkano_text::DrawText;
use vulkano_win::VkSurfaceBuild;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

extern crate futures;
extern crate itertools;
extern crate mint;
extern crate vulkano_text;

use futures::executor::ThreadPoolBuilder;

use std::iter;
use std::sync::Arc;

mod actor;
mod camera;
mod game;
mod sign_post;
mod sky;
mod world;

mod executor;
mod render;
mod shaders;
mod things;
mod utils;

use executor::Executor;
use game::Game;
use render::model::Model;
use shaders::main::fs;
use shaders::main::vs;
use utils::{Normal, Vertex};

pub struct Graph {
  surface: Arc<Surface<Window>>,
  dimensions: [u32; 2],
  device: Arc<Device>,
  queue: Arc<Queue>,
  swapchain: Arc<Swapchain<Window>>,
  render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
  vs: vs::Shader,
  fs: fs::Shader,
  pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
  framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
  draw_text: DrawText,
}

impl Graph {
  fn new(event_loop: &EventLoop<()>) -> Graph {
    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, &required_extensions, None).unwrap();

    for device in PhysicalDevice::enumerate(&instance) {
      println!(
        "possible device: {} (type: {:?})",
        device.name(),
        device.ty()
      );
    }
    let device_ext = DeviceExtensions {
      khr_swapchain: true,
      ..DeviceExtensions::none()
    };

    let surface = WindowBuilder::new()
      .build_vk_surface(&event_loop, instance.clone())
      .unwrap();
    let dimensions: [u32; 2] = surface.window().inner_size().into();
    let physical = PhysicalDevice::enumerate(&instance)
      .find(|device| device.ty() == PhysicalDeviceType::DiscreteGpu)
      .unwrap();
    println!(
      "Using device: {} (type: {:?})",
      physical.name(),
      physical.ty()
    );
    let queue_family = physical
      .queue_families()
      .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
      .unwrap();

    let (device, mut queues) = Device::new(
      physical,
      physical.supported_features(),
      &device_ext,
      [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();
    let queue = queues.next().unwrap();
    let (swapchain, images) = {
      let caps = surface.capabilities(physical).unwrap();
      let alpha = caps.supported_composite_alpha.iter().next().unwrap();
      let format = caps.supported_formats[0].0;
      let dimensions: [u32; 2] = surface.window().inner_size().into();

      Swapchain::new(
        device.clone(),
        surface.clone(),
        caps.min_image_count,
        format,
        dimensions,
        1,
        ImageUsage::color_attachment(),
        &queue,
        SurfaceTransform::Identity,
        alpha,
        PresentMode::Fifo,
        FullscreenExclusive::Default,
        true,
        ColorSpace::SrgbNonLinear,
      )
      .unwrap()
    };

    let render_pass = Arc::new(
      vulkano::single_pass_renderpass!(
          device.clone(),
          attachments: {
              color: {
                  load: Clear,
                  store: Store,
                  format: swapchain.format(),
                  samples: 1,
              },
              depth: {
                  load: Clear,
                  store: DontCare,
                  format: Format::D16Unorm,
                  samples: 1,
              }
          },
          pass: {
              color: [color],
              depth_stencil: {depth}
          }
      )
      .unwrap(),
    );
    let vs = vs::Shader::load(device.clone()).unwrap();
    //let tcs = tcs::Shader::load(device.clone()).unwrap();
    //let tes = tes::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

    let (pipeline, framebuffers) =
      window_size_dependent_setup(device.clone(), &vs, &fs, &images, render_pass.clone());

    let draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

    Graph {
      surface,
      dimensions,
      device,
      queue,
      swapchain,
      render_pass,
      vs,
      fs,
      pipeline,
      framebuffers,
      draw_text,
    }
  }

  pub fn recreate_swapchain(&mut self) {
    let dimensions: [u32; 2] = self.surface.window().inner_size().into();
    let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimensions(dimensions) {
      Ok(r) => r,
      Err(SwapchainCreationError::UnsupportedDimensions) => return,
      Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
    };
    self.swapchain = new_swapchain;
    let (new_pipeline, new_framebuffers) = window_size_dependent_setup(
      self.device.clone(),
      &self.vs,
      &self.fs,
      &new_images,
      self.render_pass.clone(),
    );
    self.pipeline = new_pipeline;
    self.framebuffers = new_framebuffers;

    self.draw_text = DrawText::new(
      self.device.clone(),
      self.queue.clone(),
      self.swapchain.clone(),
      &new_images,
    );
  }
}

fn main() {
  let mut thread_pool_builder = ThreadPoolBuilder::new();
  thread_pool_builder.name_prefix("background").pool_size(2);
  let thread_pool = thread_pool_builder.create().unwrap();

  let event_loop = EventLoop::new();
  let graph = Graph::new(&event_loop);

  /*let dynamic_state = DynamicState {
      line_width: None,
      viewports: None,
      scissors: None,
      compare_mask: None,
      write_mask: None,
      reference: None,
  };*/

  let executor = Executor::new(thread_pool);

  let mut game = Game::new(executor, graph);
  game.init();
  event_loop.run(move |event, _, mut control_flow| {
    game.tick();
    game.gloop(event, &mut control_flow)
  });
}

fn window_size_dependent_setup(
  device: Arc<Device>,
  vs: &vs::Shader,
  fs: &fs::Shader,
  images: &[Arc<SwapchainImage<Window>>],
  render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
) -> (
  Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
  Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
) {
  let dimensions = images[0].dimensions();

  let depth_buffer =
    AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap();

  let framebuffers = images
    .iter()
    .map(|image| {
      Arc::new(
        Framebuffer::start(render_pass.clone())
          .add(image.clone())
          .unwrap()
          .add(depth_buffer.clone())
          .unwrap()
          .build()
          .unwrap(),
      ) as Arc<dyn FramebufferAbstract + Send + Sync>
    })
    .collect::<Vec<_>>();

  // In the triangle example we use a dynamic viewport, as its a simple example.
  // However in the teapot example, we recreate the pipelines with a hardcoded viewport instead.
  // This allows the driver to optimize things, at the cost of slower window resizes.
  // https://computergraphics.stackexchange.com/questions/5742/vulkan-best-way-of-updating-pipeline-viewport
  let pipeline = Arc::new(
    GraphicsPipeline::start()
      .vertex_input(TwoBuffersDefinition::<Vertex, Normal>::new())
      .vertex_shader(vs.main_entry_point(), ())
      .triangle_list()
      .viewports_dynamic_scissors_irrelevant(1)
      .viewports(iter::once(Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
      }))
      .fragment_shader(fs.main_entry_point(), ())
      .depth_stencil_simple_depth()
      .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
      .build(device)
      .unwrap(),
  );

  (pipeline, framebuffers)
}
