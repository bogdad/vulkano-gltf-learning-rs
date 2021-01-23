use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::buffer::BufferUsage;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::swapchain;
use vulkano::swapchain::AcquireError;

use vulkano::command_buffer::{AutoCommandBufferBuilder, SubpassContents};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};

use vulkano_text::{DrawTextTrait};

use winit::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;

use cgmath::prelude::*;
use cgmath::{Matrix3, Matrix4, Point3, Rad, Vector3};

use std::sync::Arc;
use std::fmt;

use crate::vs;
use crate::Graph;
use crate::Model;

use crate::terrain_generation;
use crate::primitives::PrimitiveCube;

#[derive(Debug)]
struct Camera {
  // where camera is looking at
  front: Vector3<f32>,
  // where camera is
  pos: Point3<f32>,
  // up is there
  up: Vector3<f32>,
  speed: f32,
}
impl Camera {

  fn adjust(&mut self, mode: Mode, by: Vector3<f32>) {
    match mode {
      Mode::MoveCameraPos => self.pos += by,
      Mode::MoveCameraFront => self.front += by,
      Mode::MoveCameraUp => self.up += by,
    }
  }

  pub fn react(self: &mut Camera, mode: Mode, input: &KeyboardInput) -> bool {
    if let KeyboardInput {
      virtual_keycode: Some(key_code),
      ..
    } = input
    {
      let camera_speed = self.speed;
      let zz = self.front.cross(self.up).normalize();
      match key_code {
        VirtualKeyCode::A => {
          self.adjust(mode, -zz * camera_speed);
          return true;
        }
        VirtualKeyCode::D => {
          self.adjust(mode, zz * camera_speed);
          return true;
        }
        VirtualKeyCode::W => {
          self.adjust(mode, camera_speed * self.front);
          return true;
        }
        VirtualKeyCode::S => {
          self.adjust(mode, -camera_speed * self.front);
          return true;
        }
        _ => {
          return false;
        }
      };
    }
    return false;
  }

  fn proj(&self, graph: &Graph) -> vs::ty::Data {
    //let _elapsed = self.rotation_start.elapsed();
    let rotation = 0;
    //elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
    let rotation = Matrix3::from_angle_y(Rad(rotation as f32));

    // note: this teapot was meant for OpenGL where the origin is at the lower left
    //       instead the origin is at the upper left in, Vulkan, so we reverse the Y axis
    let aspect_ratio = graph.dimensions[0] as f32 / graph.dimensions[1] as f32;
    let mut proj =
      cgmath::perspective(Rad(std::f32::consts::FRAC_PI_2), aspect_ratio, 0.01, 100.0);

    // flipping the "horizontal" projection bit
    proj[0][0] = -proj[0][0];

    let target = self.pos.to_vec() + self.front;

    let view = Matrix4::look_at(self.pos, Point3::from_vec(target), self.up);
    let scale = Matrix4::from_scale(0.1);
    /*
       mat4 worldview = uniforms.view * uniforms.world;
       v_normal = transpose(inverse(mat3(worldview))) * normal;
       gl_Position = uniforms.proj * worldview * vec4(position, 1.0);
    */
    let uniform_data = vs::ty::Data {
      //world: Matrix4::from(eye).into(),
      world: Matrix4::from(rotation).into(),
      //world: <Matrix4<f32> as One>::one().into(),
      view: (view * scale).into(),
      proj: proj.into(),
    };
    uniform_data
  }
}

impl fmt::Display for Camera {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pos: ({}, {}, {}) front: ({}, {}, {}), up: ({}, {}, {}) speed: {}",
          self.pos.x, self.pos.y, self.pos.z,
          self.front.x, self.front.y, self.front.z,
          self.up.x, self.up.y, self.up.z,
          self.speed)
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Mode {
  MoveCameraPos,
  MoveCameraFront,
  MoveCameraUp,
}

impl Mode {
  const VALUES: [Self; 3] = [Self::MoveCameraPos, Self::MoveCameraFront, Self::MoveCameraUp];
  fn next(&self) -> Mode {
    let mut prev = Self::MoveCameraUp;
    for mode in Mode::VALUES.iter().copied() {
        if prev == *self {
          return mode;
        }
        prev = mode;
    }
    return prev
  }
}

struct World {
  mode: Mode
}
impl World {
  pub fn new() -> Self {
    World {
      mode: Mode::MoveCameraPos,
    }
  }
  pub fn camera_entered(&mut self, pos: &Point3<f32>) {
    // entering
    if pos.x.rem_euclid(2.0) < f32::EPSILON && pos.z.rem_euclid(2.0) < f32::EPSILON {
      //println!(" entering x, y, z {:?} {:?} {:?}", pos.x, pos.y, pos.z);
    }
  }
  pub fn command(&mut self) {
    self.mode = self.mode.next();
  }
  pub fn react(&mut self, input: &KeyboardInput) {
    if let KeyboardInput {
      virtual_keycode: Some(key_code),
      ..
    } = input {
      match key_code {
        VirtualKeyCode::Escape => self.command(),
        _ => (),
      }
    }
  }
}

impl fmt::Display for World {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "mode {:?}", self.mode)
  }
}

pub struct Game {
  graph: Graph,
  camera: Camera,
  world: World,
  recreate_swapchain: bool,
  models: Vec<Model>,
  uniform_buffer: CpuBufferPool<vs::ty::Data>,
  previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Game {
  pub fn new(graph: Graph) -> Game {
    // gltf:
    // "and the default camera sits on the
    // -Z side looking toward the origin with +Y up"
    //                               x     y    z
    // y = up/down
    // x = left/right
    // z = close/far
    let camera = Camera {
      pos: Point3::new(0.0, 0.0, -1.0),
      front: Vector3::new(0.0, 0.0, 1.0),
      up: Vector3::new(0.0, 1.0, 0.0),
      speed: 0.1,
    };

    let world = World::new();

    let recreate_swapchain = false;
    let previous_frame_end = Some(sync::now(graph.device.clone()).boxed());

    let models = vec![
      //Model::from_gltf(Path::new("models/creature.glb"), &device),
      //Model::from_gltf(Path::new("models/creature2.glb"), &device),
      //Model::from_gltf(Path::new("models/creature3.glb"), &device),
      //Model::from_gltf(Path::new("models/landscape.glb"), &graph.device),
      //Model::from_gltf(Path::new("models/dog.glb"), &graph.device),
      //Model::from_gltf(Path::new("models/box.glb"), &device),
      //Model::from_gltf(Path::new("models/center.glb"), &device),
      terrain_generation::execute(128, 2).get_buffers(&graph.device),
      PrimitiveCube::new(2.0, 4.0, 8.0, (-4.0, 0.0, 0.0)).mesh.get_buffers(&graph.device),
    ];

    let uniform_buffer =
      CpuBufferPool::<vs::ty::Data>::new(graph.device.clone(), BufferUsage::all());

    Game {
      graph,
      camera,
      world,
      recreate_swapchain,
      models,
      uniform_buffer,
      previous_frame_end,
    }
  }

  fn draw(&mut self) {
    self.previous_frame_end.as_mut().unwrap().cleanup_finished();
    if self.recreate_swapchain {
      self.graph.recreate_swapchain();
      self.recreate_swapchain = false;
    }
    let uniform_buffer_subbuffer = {

      let uniform_data = self.camera.proj(&self.graph);
      self.uniform_buffer.next(uniform_data).unwrap()
    };
    let layout = self.graph.pipeline.descriptor_set_layout(0).unwrap();
    let set = Arc::new(
      PersistentDescriptorSet::start(layout.clone())
        .add_buffer(uniform_buffer_subbuffer)
        .unwrap()
        .build()
        .unwrap(),
    );

    let (image_num, suboptimal, acquire_future) =
      match swapchain::acquire_next_image(self.graph.swapchain.clone(), None) {
        Ok(r) => r,
        Err(AcquireError::OutOfDate) => {
          self.recreate_swapchain = true;
          return;
        }
        Err(e) => panic!("Failed to acquire next image: {:?}", e),
      };

    if suboptimal {
      self.recreate_swapchain = true;
    }

    let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
      self.graph.device.clone(),
      self.graph.queue.family(),
    )
    .unwrap();
    builder
      .begin_render_pass(
        self.graph.framebuffers[image_num].clone(),
        SubpassContents::Inline,
        vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()],
      )
      .unwrap();
    for model in &self.models {
      model.draw_indexed(&mut builder, self.graph.pipeline.clone(), set.clone())
    }

    let mut y = 50.0;
    let status = self.status_string();
    for line in status .split("\n") {
      self.graph.draw_text.queue_text(200.0, y, 40.0, [1.0, 1.0, 1.0, 1.0], line);
      y += 40.0;
    };

    builder.end_render_pass().unwrap();
    builder.draw_text(&mut self.graph.draw_text, image_num);

    let command_buffer = builder.build().unwrap();

    let future = self
      .previous_frame_end
      .take()
      .unwrap()
      .join(acquire_future)
      .then_execute(self.graph.queue.clone(), command_buffer)
      .unwrap()
      .then_swapchain_present(
        self.graph.queue.clone(),
        self.graph.swapchain.clone(),
        image_num,
      )
      .then_signal_fence_and_flush();

    match future {
      Ok(future) => {
        self.previous_frame_end = Some(future.boxed());
      }
      Err(FlushError::OutOfDate) => {
        self.recreate_swapchain = true;
        self.previous_frame_end = Some(sync::now(self.graph.device.clone()).boxed());
      }
      Err(e) => {
        println!("Failed to flush future: {:?}", e);
        self.previous_frame_end = Some(sync::now(self.graph.device.clone()).boxed());
      }
    }
  }

  pub fn gloop(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
    match event {
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::WindowEvent {
        event: WindowEvent::Resized(_),
        ..
      } => {
        self.recreate_swapchain = true;
      }
      Event::WindowEvent {
        event: WindowEvent::KeyboardInput { input, .. },
        ..
      } => {
        self.world.react(&input);
        let camera_moved = self.camera.react(self.world.mode, &input);
        if camera_moved {
          self.world.camera_entered(&self.camera.pos);
        }
      }
      Event::RedrawEventsCleared => {
        self.draw();
      }
      _ => (),
    }
  }

  fn status_string(&self) -> String {
    format!("world {}\ncamera {}", self.world, self.camera)
  }
}
