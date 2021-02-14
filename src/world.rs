use cgmath::Point3;
use winit::event::{KeyboardInput, VirtualKeyCode};

use std::fmt;

use crate::executor::Executor;
use crate::render::model::Model;
use crate::sign_post::SignPost;
use crate::sky::Sky;
use crate::Graph;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Mode {
  MoveCameraPos,
  MoveCameraFront,
  MoveCameraUp,
}

impl Mode {
  const VALUES: [Self; 3] = [
    Self::MoveCameraPos,
    Self::MoveCameraFront,
    Self::MoveCameraUp,
  ];
  fn next(&self) -> Mode {
    let mut prev = Self::MoveCameraUp;
    for mode in Mode::VALUES.iter().copied() {
      if prev == *self {
        return mode;
      }
      prev = mode;
    }
    prev
  }
}

pub struct World {
  executor: Executor,
  pub mode: Mode,
  sky: Sky,
  sign_posts: Vec<SignPost>,
}
impl World {
  pub fn new(executor: Executor, graph: &Graph, sign_posts: Vec<SignPost>) -> Self {
    let sky = Sky::new(&graph.device, 0.0, 0.0);
    World {
      executor,
      mode: Mode::MoveCameraPos,
      sky,
      sign_posts,
    }
  }

  pub fn tick(&mut self) {
    self.sky.tick(&self.executor);
  }

  pub fn camera_entered(&mut self, pos: &Point3<f32>) {
    // entering
    if pos.x.rem_euclid(2.0) < f32::EPSILON && pos.z.rem_euclid(2.0) < f32::EPSILON {
      //println!(" entering x, y, z {:?} {:?} {:?}", pos.x, pos.y, pos.z);
    }
    self.sky.camera_entered(pos);
  }

  pub fn command(&mut self) {
    self.mode = self.mode.next();
  }

  pub fn react(&mut self, input: &KeyboardInput) {
    if let KeyboardInput {
      virtual_keycode: Some(key_code),
      ..
    } = input
    {
      match key_code {
        VirtualKeyCode::Escape => self.command(),
        _ => (),
      }
    }
  }

  pub fn get_models(&self) -> Vec<Model> {
    let mut res = vec![];
    res.extend(self.sky.get_current());
    for sign_post in &self.sign_posts {
      res.push(sign_post.get_model().clone());
    }
    res
  }
}

impl fmt::Display for World {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "mode {:?}", self.mode)
  }
}
