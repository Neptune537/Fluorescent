import 'dart:ui';

enum PinDirection { input, output }

class NodePin {
  final String id;
  final String name;
  final PinDirection direction;
  final String type; // e.g., 'Float', 'Vector3', 'Particles', 'Texture'

  NodePin({
    required this.id,
    required this.name,
    required this.direction,
    required this.type,
  });
}

class NodeConnection {
  final String fromNodeId;
  final String fromPinId;
  final String toNodeId;
  final String toPinId;

  NodeConnection({
    required this.fromNodeId,
    required this.fromPinId,
    required this.toNodeId,
    required this.toPinId,
  });
}

class GraphNode {
  final String id;
  final String name;
  final String type;
  Offset position;
  Map<String, dynamic> properties;
  List<NodePin> inputs;
  List<NodePin> outputs;

  GraphNode({
    required this.id,
    required this.name,
    required this.type,
    this.position = Offset.zero,
    Map<String, dynamic>? properties,
    List<NodePin>? inputs,
    List<NodePin>? outputs,
  })  : properties = properties ?? {},
        inputs = inputs ?? [],
        outputs = outputs ?? [];
}

class NodeGraph {
  final List<GraphNode> nodes = [];
  final List<NodeConnection> connections = [];

  void addNode(GraphNode node) {
    nodes.add(node);
  }

  void removeNode(String nodeId) {
    nodes.removeWhere((n) => n.id == nodeId);
    connections
        .removeWhere((c) => c.fromNodeId == nodeId || c.toNodeId == nodeId);
  }

  void addConnection(NodeConnection connection) {
    connections.add(connection);
  }

  void removeConnection(
      String fromNode, String fromPin, String toNode, String toPin) {
    connections.removeWhere((c) =>
        c.fromNodeId == fromNode &&
        c.fromPinId == fromPin &&
        c.toNodeId == toNode &&
        c.toPinId == toPin);
  }
}

// Factories for specific nodes
abstract class NodeFactory {
  static GraphNode createComputeParticleEmitter(String id, Offset position) {
    return GraphNode(
      id: id,
      name: 'Compute Particle Emitter',
      type: 'ComputeParticleEmitter',
      position: position,
      properties: {
        'spawn_rate': 1000.0,
        'use_half_precision': true,
        'life_time': 2.0,
      },
      inputs: [
        NodePin(
            id: 'in_velocity',
            name: 'Velocity',
            direction: PinDirection.input,
            type: 'Vector3'),
      ],
      outputs: [
        NodePin(
            id: 'out_particles',
            name: 'Particles',
            direction: PinDirection.output,
            type: 'Particles'),
      ],
    );
  }

  static GraphNode createMobileOptimizedTerrainPCG(String id, Offset position) {
    return GraphNode(
      id: id,
      name: 'Mobile Optimized Terrain PCG',
      type: 'MobileOptimizedTerrainPCG',
      position: position,
      properties: {
        'biome_scale': 1.0,
        'density': 1.0,
        'seed': 0,
      },
      inputs: [
        NodePin(
            id: 'in_noise',
            name: 'Noise Map',
            direction: PinDirection.input,
            type: 'Texture'),
      ],
      outputs: [
        NodePin(
            id: 'out_terrain',
            name: 'Terrain',
            direction: PinDirection.output,
            type: 'Mesh'),
      ],
    );
  }

  static GraphNode createHalfPrecisionVFX(String id, Offset position) {
    return GraphNode(
      id: id,
      name: 'Half Precision VFX',
      type: 'HalfPrecisionVFX',
      position: position,
      properties: {
        'max_particles': 1000,
        'emit_rate': 100.0,
      },
      inputs: [
        NodePin(
            id: 'in_particles',
            name: 'Particles',
            direction: PinDirection.input,
            type: 'Particles'),
      ],
      outputs: [
        NodePin(
            id: 'out_gbuffer',
            name: 'GBuffer',
            direction: PinDirection.output,
            type: 'RenderPass'),
      ],
    );
  }
}
