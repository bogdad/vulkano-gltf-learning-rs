use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::pool::standard::StandardCommandPoolBuilder;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::DescriptorSetsCollection;
use vulkano::device::Device;
use vulkano::pipeline::GraphicsPipelineAbstract;

use std::path::Path;
use std::sync::Arc;

use crate::utils::{Normal, Vertex};
use crate::render::mymesh::MyMesh;

#[derive(Clone, Debug)]
pub struct Model {
  vertex: Arc<CpuAccessibleBuffer<[Vertex]>>,
  normals: Arc<CpuAccessibleBuffer<[Normal]>>,
  index: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl Model {
  pub fn new(
    vertex: Arc<CpuAccessibleBuffer<[Vertex]>>,
    normals: Arc<CpuAccessibleBuffer<[Normal]>>,
    index: Arc<CpuAccessibleBuffer<[u32]>>,
  ) -> Model {
    Model {
      vertex,
      normals,
      index,
    }
  }

  pub fn draw_indexed<S>(
    &self,
    builder: &mut AutoCommandBufferBuilder<StandardCommandPoolBuilder>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    set: S,
  ) where
    S: DescriptorSetsCollection,
  {
    builder
      .draw_indexed(
        pipeline.clone(),
        &DynamicState::none(),
        vec![self.vertex.clone(), self.normals.clone()],
        self.index.clone(),
        set,
        (),
      )
      .unwrap();
  }

  pub fn from_gltf(path: &Path, device: &Arc<Device>) -> Model {
    MyMesh::from_gltf(path).get_buffers(device)
  }
}