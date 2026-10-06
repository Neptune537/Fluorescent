import 'dart:convert';
import 'package:flutter/material.dart';
import 'models.dart';

class GraphSerialization {
  static String toJson(NodeGraph graph) {
    final Map<String, dynamic> data = {
      'nodes': graph.nodes
          .map((node) => {
                'id': node.id,
                'name': node.name,
                'type': node.type,
                'position': {'x': node.position.dx, 'y': node.position.dy},
                'properties': node.properties,
              })
          .toList(),
      'connections': graph.connections
          .map((conn) => {
                'from': conn.fromNodeId,
                'from_pin': conn.fromPinId,
                'to': conn.toNodeId,
                'to_pin': conn.toPinId,
              })
          .toList(),
    };
    return jsonEncode(data);
  }

  static void fromJson(String jsonStr, NodeGraph graph) {
    final data = jsonDecode(jsonStr);

    graph.nodes.clear();
    graph.connections.clear();

    if (data['nodes'] != null) {
      for (var nodeData in data['nodes']) {
        final id = nodeData['id'];
        final type = nodeData['type'];
        final pos = Offset(
          (nodeData['position']['x'] as num).toDouble(),
          (nodeData['position']['y'] as num).toDouble(),
        );

        GraphNode node;
        switch (type) {
          case 'ComputeParticleEmitter':
            node = NodeFactory.createComputeParticleEmitter(id, pos);
            break;
          case 'MobileOptimizedTerrainPCG':
            node = NodeFactory.createMobileOptimizedTerrainPCG(id, pos);
            break;
          case 'HalfPrecisionVFX':
            node = NodeFactory.createHalfPrecisionVFX(id, pos);
            break;
          default:
            // Fallback for unknown nodes
            node = GraphNode(
              id: id,
              name: nodeData['name'] ?? 'Unknown',
              type: type,
              position: pos,
              properties:
                  Map<String, dynamic>.from(nodeData['properties'] ?? {}),
            );
        }

        // Overwrite properties if they exist
        if (nodeData['properties'] != null) {
          node.properties = Map<String, dynamic>.from(nodeData['properties']);
        }

        switch (type) {
        case 'MobileOptimizedTerrainPCG':
          // AGENT 6 TODO: Serialize PCG Node to Engine Graph
          // 1. Extract `biome_scale`, `density`, and `seed` properties from `node.properties`.
          // 2. Format a JSON payload matching the Rust `PcgConfig` struct definition.
          // 3. Emit this payload via WebSocket or FFI to the Rust engine to trigger world generation.
          break;
        case 'HalfPrecisionVFX':
          // AGENT 6 TODO: Serialize VFX Node to Engine Graph
          // 1. Extract particle properties (spawn rate, gravity, lifetime, color) from `node.properties`.
          // 2. Format a JSON payload matching the Rust `ParticleConfig` struct.
          // 3. Emit this payload to the `GpuParticleSystem` in the Rust rendering core.
          break;
        }

        graph.addNode(node);
      }
    }

    if (data['connections'] != null) {
      for (var connData in data['connections']) {
        graph.addConnection(NodeConnection(
          fromNodeId: connData['from'],
          fromPinId: connData['from_pin'],
          toNodeId: connData['to'],
          toPinId: connData['to_pin'],
        ));
      }
    }
  }
}
