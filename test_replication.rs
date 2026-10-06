use fluorite_core::net::replication::*;
use fluorite_core::ecs::world::*;

struct MockMeshNode {}
impl MeshNode for MockMeshNode {
    fn broadcast(&self, _data: &[u8]) {}
}

fn main() {
    let mut world = EngineWorld::new();
    let mesh_node = Box::new(MockMeshNode {});
    let sys = ReplicationSystem::new(mesh_node);
}
