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
            final biomeScale = (node.properties['biome_scale'] as num?)?.toDouble() ?? 1.0;
            final density = (node.properties['density'] as num?)?.toDouble() ?? 1.0;
            final seed = (node.properties['seed'] as num?)?.toInt() ?? 0;

            final pcgPayload = {
              'config': {
                'biome_scale': biomeScale,
                'density': density,
              },
              'seed': seed,
            };
            // Simulate emitting this payload via WebSocket or FFI to the Rust engine
            print('Emitting PCG payload: ${jsonEncode(pcgPayload)}');
            break;
          case 'HalfPrecisionVFX':
            final spawnRate = (node.properties['spawn_rate'] as num?)?.toDouble() ?? 100.0;
            final gravity = (node.properties['gravity'] as num?)?.toDouble() ?? -9.81;
            final lifetime = (node.properties['lifetime'] as num?)?.toDouble() ?? 5.0;
            final color = node.properties['color'] != null
                ? List<double>.from((node.properties['color'] as List).map((e) => (e as num).toDouble()))
                : [1.0, 1.0, 1.0, 1.0];

            final vfxPayload = {
              'spawn_rate': spawnRate,
              'gravity': gravity,
              'lifetime': lifetime,
              'color': color,
            };
            // Simulate emitting this payload to the GpuParticleSystem in the Rust rendering core
            print('Emitting VFX payload: ${jsonEncode(vfxPayload)}');
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
