use std::sync::Arc;

use glam::{Mat4, Vec3};

use crate::{
    assets::AssetRef,
    game::entities::{Asteroid, Bullet, Camera, EntityId, Spacecraft},
};

/// View data to use in rendering
pub struct View {
    matrix: Mat4,
}

impl From<&Camera> for View {
    fn from(value: &Camera) -> Self {
        Self {
            matrix: value.to_view_matrix(),
        }
    }
}

/// Model data to use in rendering
pub struct Model {
    matrix: Mat4,
    color: Vec3,
    mesh: AssetRef,
    pipeline: AssetRef,
}

impl From<&Spacecraft> for Model {
    fn from(value: &Spacecraft) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(0.1, 0.8, 0.1),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

impl From<&Asteroid> for Model {
    fn from(value: &Asteroid) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(0.6, 0.6, 0.6),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

impl From<&Bullet> for Model {
    fn from(value: &Bullet) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(1.0, 1.0, 1.0),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

/// Renderer
pub struct Renderer {
    // TODO
}

impl Renderer {
    /// Creates new instance of [Renderer]
    pub fn new() -> Arc<Renderer> {
        todo!()
    }

    /// Sets entity to be used as view data source
    pub fn set_view(&self, entity_id: Option<EntityId>) {
        todo!()
    }

    /// Dispatches view data to renderer
    pub fn dispatch_view(&self, entity_id: EntityId, view: View) {
        todo!()
    }

    /// Dispatches model data to renderer
    pub fn dispatch_model(&self, entity_id: EntityId, model: Model) {
        todo!()
    }
}

// use std::{
//     collections::BTreeMap,
//     f32::consts::PI,
//     iter::once,
//     sync::{Arc, Mutex, atomic::Ordering},
// };

// use glam::{Mat4, Quat, Vec2, Vec3};
// use vulkano::{
//     buffer::{BufferUsage, Subbuffer},
//     command_buffer::{
//         AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
//         allocator::CommandBufferAllocator,
//     },
//     descriptor_set::{DescriptorSet, WriteDescriptorSet, allocator::DescriptorSetAllocator},
//     format::ClearValue,
//     pipeline::{
//         GraphicsPipeline, Pipeline, PipelineBindPoint,
//         graphics::{input_assembly::PrimitiveTopology, viewport::Viewport},
//     },
//     render_pass::{AttachmentLoadOp, AttachmentStoreOp},
// };

// use crate::{
//     dispatch::{Dispatcher, Event},
//     game::Game,
//     game::entities::{self, EntityId},
//     rendering::{
//         backend::{ShaderFactory, ShaderStage},
//         models::{
//             ASTEROID_INDICES, BULLET_INDICES, BULLET_VERTICES, SPACECRAFT_INDICES,
//             SPACECRAFT_VERTICES,
//         },
//         shaders::{Vertex, bullet_vs, entity_fs, entity_vs},
//     },
//     worker::Worker,
// };

// use super::{backend::Backend, shaders::Entity};

// struct RenderData {
//     pipeline: Arc<GraphicsPipeline>,
//     entity_buffer: Subbuffer<Entity>,
//     entity_buffer_descriptor_set: Arc<DescriptorSet>,
//     vertex_buffer: Subbuffer<[Vertex]>,
//     index_buffer: Subbuffer<[u32]>,
// }

// struct Inner {
//     game: Arc<Game>,
//     backend: Arc<Backend>,
//     entity_pipeline: Arc<GraphicsPipeline>,
//     bullet_pipeline: Arc<GraphicsPipeline>,
//     render_data: Mutex<BTreeMap<EntityId, RenderData>>,
//     spacecraft_vertex_buffer: Subbuffer<[Vertex]>,
//     spacecraft_index_buffer: Subbuffer<[u32]>,
//     asteroid_index_buffer: Subbuffer<[u32]>,
//     bullet_vertex_buffer: Subbuffer<[Vertex]>,
//     bullet_index_buffer: Subbuffer<[u32]>,
//     command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
//     descriptor_set_allocator: Arc<dyn DescriptorSetAllocator>,
// }

// impl Inner {
//     fn new(game: Arc<Game>, backend: Arc<Backend>) -> Arc<Inner> {
//         let inner = Inner {
//             game,
//             command_buffer_allocator: backend.create_command_buffer_allocator(),
//             descriptor_set_allocator: backend.create_descriptor_set_allocator(),
//             entity_pipeline: backend.create_pipeline(
//                 PrimitiveTopology::TriangleList,
//                 [
//                     (ShaderStage::Vertex, entity_vs::load as ShaderFactory),
//                     (ShaderStage::Fragment, entity_fs::load as ShaderFactory),
//                 ],
//             ),
//             bullet_pipeline: backend.create_pipeline(
//                 PrimitiveTopology::PointList,
//                 [
//                     (ShaderStage::Vertex, bullet_vs::load as ShaderFactory),
//                     (ShaderStage::Fragment, entity_fs::load as ShaderFactory),
//                 ],
//             ),
//             render_data: Default::default(),
//             spacecraft_vertex_buffer: backend
//                 .create_buffer_iter(BufferUsage::VERTEX_BUFFER, SPACECRAFT_VERTICES),
//             spacecraft_index_buffer: backend
//                 .create_buffer_iter(BufferUsage::INDEX_BUFFER, SPACECRAFT_INDICES),
//             asteroid_index_buffer: backend
//                 .create_buffer_iter(BufferUsage::INDEX_BUFFER, ASTEROID_INDICES),
//             bullet_vertex_buffer: backend
//                 .create_buffer_iter(BufferUsage::VERTEX_BUFFER, BULLET_VERTICES),
//             bullet_index_buffer: backend
//                 .create_buffer_iter(BufferUsage::INDEX_BUFFER, BULLET_INDICES),
//             backend,
//         };

//         Arc::new(inner)
//     }

//     fn render(&self) {
//         if let Some(frame) = self.backend.acquire_frame() {
//             let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
//                 self.command_buffer_allocator.clone(),
//                 self.backend.graphics_queue_family_index(),
//                 CommandBufferUsage::MultipleSubmit,
//             )
//             .expect("failed to create command buffer builder");

//             command_buffer_builder
//                 .begin_rendering(RenderingInfo {
//                     color_attachments: vec![Some(RenderingAttachmentInfo {
//                         load_op: AttachmentLoadOp::Clear,
//                         store_op: AttachmentStoreOp::Store,
//                         clear_value: Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),

//                         ..RenderingAttachmentInfo::image_view(frame.image_view())
//                     })],
//                     ..Default::default()
//                 })
//                 .expect("failed to begin rendering");

//             command_buffer_builder
//                 .set_viewport(
//                     0,
//                     vec![Viewport {
//                         offset: [0.0, 0.0],
//                         extent: frame.extent(),
//                         ..Default::default()
//                     }]
//                     .into(),
//                 )
//                 .expect("failed to set viewport");

//             // TODO
//             // let camera_matrix = entities
//             //     .get(self.game.camera_id())
//             //     .map(|entity| {
//             //         let transform = entity.transform();
//             //         let camera = entity.camera().unwrap();

//             //         let mut projection =
//             //             Mat4::perspective_infinite_lh(PI / 2.0, frame.aspect(), 0.001);
//             //         projection.col_mut(1)[1] *= -1.0;

//             //         let view = Mat4::look_at_lh(
//             //             Vec3::new(transform.position.x, transform.position.y, camera.distance),
//             //             Vec3::new(transform.position.x, transform.position.y, 0.0),
//             //             Vec3::new(0.0, 1.0, 0.0),
//             //         );

//             //         projection * view
//             //     })
//             //     .expect("there is not camera entity");

//             // entities
//             //     .iter()
//             //     .filter(|(_, entity)| !matches!(entity, game_entity::Entity::Camera(_)))
//             //     .for_each(|(entity_id, entity)| {
//             //         let render_data = render_data.entry(entity_id).or_insert_with(|| {
//             //             self.create_render_data(entity)
//             //                 .expect("entity is not renderable")
//             //         });

//             //         {
//             //             let mut entity_buffer = render_data.entity_buffer.write().unwrap();

//             //             let model = match entity {
//             //                 game_entity::Entity::Spacecraft(spacecraft) => {
//             //                     Mat4::from_scale_rotation_translation(
//             //                         Vec3::ONE,
//             //                         Quat::from_rotation_z(-spacecraft.transform.rotation),
//             //                         Vec3::new(
//             //                             spacecraft.transform.position.x,
//             //                             spacecraft.transform.position.y,
//             //                             0.0,
//             //                         ),
//             //                     )
//             //                 }

//             //                 game_entity::Entity::Asteroid(asteroid) => {
//             //                     Mat4::from_scale_rotation_translation(
//             //                         Vec3::ONE,
//             //                         Quat::from_rotation_z(-asteroid.transform.rotation),
//             //                         Vec3::new(
//             //                             asteroid.transform.position.x,
//             //                             asteroid.transform.position.y,
//             //                             0.0,
//             //                         ),
//             //                     )
//             //                 }

//             //                 game_entity::Entity::Bullet(bullet) => {
//             //                     Mat4::from_scale_rotation_translation(
//             //                         Vec3::ONE,
//             //                         Quat::default(),
//             //                         Vec3::new(
//             //                             bullet.transform.position.x,
//             //                             bullet.transform.position.y,
//             //                             0.0,
//             //                         ),
//             //                     )
//             //                 }

//             //                 _ => unreachable!(),
//             //             };

//             //             entity_buffer.matrix = camera_matrix * model
//             //         }

//             //         command_buffer_builder
//             //             .bind_pipeline_graphics(render_data.pipeline.clone())
//             //             .expect("failed to bind entity pipeline");

//             //         command_buffer_builder
//             //             .bind_vertex_buffers(0, render_data.vertex_buffer.clone())
//             //             .expect("failed to bind vertex buffer");

//             //         command_buffer_builder
//             //             .bind_index_buffer(render_data.index_buffer.clone())
//             //             .expect("failed to bind index buffer");

//             //         command_buffer_builder
//             //             .bind_descriptor_sets(
//             //                 PipelineBindPoint::Graphics,
//             //                 self.entity_pipeline.layout().clone(),
//             //                 0,
//             //                 vec![render_data.entity_buffer_descriptor_set.clone()],
//             //             )
//             //             .expect("failed to bind descriptor set");

//             //         unsafe {
//             //             command_buffer_builder
//             //                 .draw_indexed(render_data.index_buffer.len() as u32, 1, 0, 0, 0)
//             //                 .expect("failed to draw entity");
//             //         }
//             //     });

//             command_buffer_builder
//                 .end_rendering()
//                 .expect("failed to end rendering");

//             let command_buffer = command_buffer_builder
//                 .build()
//                 .expect("failed to build command buffer");

//             frame.submit(command_buffer);
//         }
//     }

//     fn create_render_data(&self, entity: &entities::Entity) -> Option<RenderData> {
//         let entity_buffer: Subbuffer<Entity> =
//             self.backend.create_buffer(BufferUsage::UNIFORM_BUFFER);

//         let pipeline = match entity {
//             entities::Entity::Spacecraft(_) | entities::Entity::Asteroid(_) => {
//                 self.entity_pipeline.clone()
//             }

//             entities::Entity::Bullet(_) => self.bullet_pipeline.clone(),

//             _ => return None,
//         };

//         let entity_buffer_descriptor_set = {
//             let layout = pipeline.layout().set_layouts().get(0).cloned().unwrap();

//             DescriptorSet::new(
//                 self.descriptor_set_allocator.clone(),
//                 layout,
//                 [WriteDescriptorSet::buffer(0, entity_buffer.clone())],
//                 [],
//             )
//             .expect("failed to create descriptor set for entity")
//         };

//         let render_data = match entity {
//             entities::Entity::Spacecraft(_) => {
//                 {
//                     entity_buffer.write().unwrap().color = Vec3::new(0.1, 0.8, 0.1);
//                 }

//                 RenderData {
//                     pipeline,
//                     entity_buffer,
//                     entity_buffer_descriptor_set,
//                     vertex_buffer: self.spacecraft_vertex_buffer.clone(),
//                     index_buffer: self.spacecraft_index_buffer.clone(),
//                 }
//             }

//             entities::Entity::Asteroid(asteroid) => {
//                 {
//                     entity_buffer.write().unwrap().color = Vec3::new(0.6, 0.6, 0.6);
//                 }

//                 let vertices = once(Vec2::ZERO)
//                     .chain(asteroid.asteroid.body.into_iter())
//                     .map(|position| Vertex { position })
//                     .collect::<Vec<_>>();

//                 RenderData {
//                     pipeline,
//                     entity_buffer,
//                     entity_buffer_descriptor_set,
//                     vertex_buffer: self
//                         .backend
//                         .create_buffer_iter(BufferUsage::VERTEX_BUFFER, vertices),
//                     index_buffer: self.asteroid_index_buffer.clone(),
//                 }
//             }

//             entities::Entity::Bullet(_) => {
//                 {
//                     entity_buffer.write().unwrap().color = Vec3::new(1.0, 1.0, 1.0);
//                 }

//                 RenderData {
//                     pipeline,
//                     entity_buffer,
//                     entity_buffer_descriptor_set,
//                     vertex_buffer: self.bullet_vertex_buffer.clone(),
//                     index_buffer: self.bullet_index_buffer.clone(),
//                 }
//             }

//             _ => unreachable!(),
//         };

//         Some(render_data)
//     }

//     fn dispatch_entity_created(&self, entity_id: EntityId) {}

//     fn dispatch_entity_destroyed(&self, entity_id: EntityId) {
//         self.render_data.lock().unwrap().remove(&entity_id);
//     }
// }

// pub struct Renderer {
//     _worker: Worker,
// }

// impl Renderer {
//     pub fn new(
//         event_dispatcher: &Dispatcher<Event>,
//         game: Arc<Game>,
//         backend: Arc<Backend>,
//     ) -> Renderer {
//         let inner = Inner::new(game, backend);

//         {
//             let inner = inner.clone();

//             event_dispatcher.add_handler(move |event| match event {
//                 Event::EntityCreated(entity_id) => inner.dispatch_entity_created(*entity_id),
//                 Event::EntityDestroyed(entity_id) => inner.dispatch_entity_destroyed(*entity_id),

//                 _ => {}
//             });
//         }

//         let renderer = Renderer {
//             _worker: Worker::spawn("Renderer", move |alive| {
//                 while alive.load(Ordering::Relaxed) {
//                     inner.render();
//                 }
//             }),
//         };

//         renderer
//     }
// }
